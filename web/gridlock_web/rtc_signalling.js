// Gridlock's browser plugin, part 4 of 9: RtcSocket's signalling, through
// the registry's HTTP mailboxes. RtcSocket is built up over three parts:
// RtcSignalling (here), RtcSpares (rtc_spares.js) and RtcSocket itself
// (rtc_peers.js). (See ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const {
        FORCE_RELAY, POLL_HOST_MS, POLL_CONNECTING_MS, POLL_CONNECTED_MS, HOST_KEEP_MS, HELLO_MS, LONG_POLL_HEADER,
        LONG_POLL_GAP_MS, POLL_RETRY_MS, HAVE_WINDOW, HAVE_MAX, SEND_BATCH_MS, sleep, roomHttpUrl, gathered,
        iceServersFor, spareFor,
    } = gridlockWeb;

    // ------------------------------------------------------------ WebRTC

    // Who offers: the player, which sends its offer with its join request, so
    // the host learns of the player and gets its offer in one message and the
    // whole handshake is two signalling trips (offer, answer). The host still
    // offers to a player that joined without one (a page from before this),
    // and a player takes the host's offer when one comes (an older host page).
    class RtcSignalling {
        constructor(roomUrl, isHost) {
            this.isHost = isHost;
            this.events = [];
            this.taken = null; // payload of the event last returned by next()
            this.peers = new Map(); // small id -> peer (see connect)
            this.byRemote = new Map(); // signalling id -> small id
            this.nextPeer = 1;
            this.closed = false;
            this.me = null; // { id, token, host?, via? } from the room's join
            this.outbox = []; // signals waiting for the next send
            this.flushing = false;
            // Host: offers made ahead of any join, oid -> { id, peer, at, live }
            // (see keepSpares). `live` ones are published.
            this.spares = new Map();
            this.makingSpares = 0;
            // How long each step of a join took, logged once it connects.
            this.t0 = performance.now();
            this.marks = [];
            this.start(roomUrl);
        }

        mark(step) {
            this.marks.push(`${step} +${Math.round(performance.now() - this.t0)}`);
        }

        // Signalling goes through HTTP mailboxes on the registry (registry/dcs/index.js):
        // join the room, then poll it for messages and post signals to it.
        async start(roomUrl) {
            this.room = roomHttpUrl(roomUrl);
            this.iceServers = await iceServersFor(this.room);
            this.mark("ice servers");
            if (this.closed) return;
            let signals = [];
            if (!this.isHost) {
                // Answer the host's spare offer if it has one (fetched ahead
                // when the player pointed at the room); offer otherwise.
                const spare = await spareFor(this.room);
                if (this.closed) return;
                let peer = spare && (await this.answerSpare(spare));
                if (this.closed) return;
                this.mark(peer ? "answered spare" : spare ? "spare failed" : "no spare");
                if (!peer) {
                    peer = this.peers.get(this.connect(null, true));
                    try {
                        await peer.offered;
                    } catch (e) {
                        console.warn("gridlock_web: could not make an offer", e);
                        return this.shutdown();
                    }
                    if (this.closed) return;
                    this.mark("offered");
                }
                signals = [{ data: this.describe(peer) }];
            }
            try {
                const resp = await fetch(this.room + "/join" + (this.isHost ? "?host" : ""), {
                    method: "POST",
                    body: JSON.stringify({ signals }),
                });
                if (!resp.ok) throw new Error("join refused: " + resp.status);
                this.me = await resp.json();
            } catch (e) {
                console.warn("gridlock_web: could not join room", e);
                return this.shutdown();
            }
            if (this.closed) return this.leave();
            this.mark("joined");
            if (!this.isHost && this.me.host) this.adoptHost(this.me.host);
            this.onPageHide = () => this.leave();
            window.addEventListener("pagehide", this.onPageHide);
            // candidates found while the join was on its way
            if (this.outbox.length && !this.flushing) {
                this.flushing = true;
                this.flush();
            }
            if (this.isHost) this.keepSpares();
            this.poll();
        }

        // A player: take the host's spare offer and answer it. The peer is
        // filed under the host named with the spare; null if that failed.
        async answerSpare(spare) {
            const id = this.connect(spare.host, false, spare.data.gen ?? 0);
            const peer = this.peers.get(id);
            peer.spare = spare.oid;
            try {
                await peer.pc.setRemoteDescription(spare.data.sdp);
                await peer.pc.setLocalDescription(await peer.pc.createAnswer());
                await gathered(peer.pc, { full: peer.gen > 0 || FORCE_RELAY });
                return peer;
            } catch (e) {
                console.warn("gridlock_web: could not answer the spare offer", e);
                this.discard(id);
                return null;
            }
        }

        // The player's offer went out before it knew its host's signalling
        // id: file that peer under it. (A spare names its host, but the room
        // may have changed hands since it was fetched: the new host offers
        // anew, and the peer must be filed under it to take that offer.)
        adoptHost(remoteId) {
            for (const [id, peer] of this.peers) {
                if (peer.spare !== undefined && peer.remoteId !== remoteId && !peer.open) {
                    if (this.byRemote.get(peer.remoteId) === id) this.byRemote.delete(peer.remoteId);
                    peer.remoteId = remoteId;
                    this.byRemote.set(remoteId, id);
                    return id;
                }
                if (peer.remoteId !== null) continue;
                peer.remoteId = remoteId;
                this.byRemote.set(remoteId, id);
                return id;
            }
            return undefined;
        }

        async poll() {
            let lastKeep = Date.now();
            let lastHello = Date.now();
            const seen = new Map(); // id -> seq of messages already handled
            let longPoll = false; // the registry holds polls (see LONG_POLL_HEADER)
            let lastSeq = 0; // newest message seq seen, for a long poll's `after`
            let failures = 0; // polls failed in a row
            while (!this.closed) {
                // A connected player needs no signalling: if the host goes,
                // the WebRTC connection itself reports it.
                const connected = !this.isHost && [...this.peers.values()].some((p) => p.open);
                if (!connected) {
                    try {
                        let q = `?id=${encodeURIComponent(this.me.id)}&token=${encodeURIComponent(this.me.token)}`;
                        if (this.isHost) {
                            q += "&host";
                            if (Date.now() - lastKeep >= HOST_KEEP_MS) {
                                q += "&keep";
                                lastKeep = Date.now();
                            }
                        }
                        // Ask to be held until something new arrives; a registry
                        // without long polling ignores this.
                        const have = [...seen].filter(([, seq]) => seq >= lastSeq - HAVE_WINDOW).slice(-HAVE_MAX);
                        q += `&wait&after=${lastSeq}&have=${have.map(([id]) => encodeURIComponent(id)).join(",")}`;
                        const resp = await fetch(this.room + "/poll" + q, { cache: "no-store" });
                        longPoll = resp.headers.has(LONG_POLL_HEADER);
                        failures = 0;
                        // A host whose claim was lost claims the room again;
                        // players already connected are not affected.
                        if (resp.status === 410 && this.isHost && (await this.reclaim())) {
                            this.publishSpares(); // they name the host by its new id
                            continue;
                        }
                        // 410: the host left; 403/404: this room is gone for us.
                        if (resp.status === 410 || resp.status === 403 || resp.status === 404) return this.shutdown();
                        if (resp.ok) {
                            const batch = await resp.json();
                            // Players that sent their offer (or their answer to a
                            // spare) along with their join: the host answers
                            // those, or takes their answer, instead of offering.
                            const offering = new Set(
                                batch
                                    .filter((m) => {
                                        const data = m.Signal?.data;
                                        return data?.sdp?.type === "offer" || (data?.sdp?.type === "answer" && data.spare !== undefined);
                                    })
                                    .map((m) => m.Signal.sender),
                            );
                            for (const msg of batch) {
                                // Messages carry ids (repeated until they expire);
                                // a registry without ids sends each one once.
                                // A message with several parts numbers them
                                // "<id>.<n>"; the registry knows it by <id>.
                                const base = msg.id === undefined ? undefined : String(msg.id).split(".")[0];
                                if (typeof msg.seq === "number") lastSeq = Math.max(lastSeq, msg.seq);
                                if (this.closed || (msg.id !== undefined && seen.has(String(msg.id)))) continue;
                                if (msg.id !== undefined) {
                                    seen.set(String(msg.id), msg.seq ?? 0);
                                    seen.set(base, msg.seq ?? 0);
                                }
                                this.onSignal(msg, offering);
                            }
                        }
                        if (!this.isHost && Date.now() - lastHello >= HELLO_MS) {
                            lastHello = Date.now();
                            this.post("/hello", {});
                        }
                    } catch {
                        failures++; // network blip: try again shortly
                    }
                }
                const delay = failures
                    ? Math.min(POLL_RETRY_MS * 2 ** (failures - 1), this.pollDelay(connected))
                    : longPoll && !connected
                      ? LONG_POLL_GAP_MS
                      : this.pollDelay(connected);
                await sleep(delay);
            }
        }

        // Poll fast while a connection is being negotiated; an idle host polls
        // just often enough to notice newcomers.
        pollDelay(connected) {
            if ([...this.peers.values()].some((p) => !p.open)) return POLL_CONNECTING_MS;
            if (this.isHost) return POLL_HOST_MS;
            return connected ? POLL_CONNECTED_MS : POLL_CONNECTING_MS;
        }

        async reclaim() {
            try {
                const resp = await fetch(this.room + "/join?host", { method: "POST", body: "{}" });
                if (!resp.ok) return false;
                this.me = await resp.json();
                return true;
            } catch {
                return false;
            }
        }

        // Who we are, and for a player which host it joined (saves the
        // registry a read on every send).
        credentials() {
            const { id, token, host, via } = this.me;
            return { id, token, host: this.isHost, ...(!this.isHost && host && via ? { to: host, via } : {}) };
        }

        post(path, fields) {
            const body = JSON.stringify({ ...this.credentials(), ...fields });
            return fetch(this.room + path, { method: "POST", body }).catch(() => {});
        }

        signal(msg) {
            if (!msg.Signal || this.closed) return;
            this.outbox.push(msg.Signal);
            // ICE candidates come in bursts; send each burst as one request.
            // Before the join is answered there is nowhere to send them yet:
            // start() sends what piled up.
            if (!this.flushing && this.me) {
                this.flushing = true;
                setTimeout(() => this.flush(), SEND_BATCH_MS);
            }
        }

        async flush() {
            const signals = this.outbox.splice(0);
            try {
                if (signals.length && this.me && !this.closed) {
                    await fetch(this.room + "/send", {
                        method: "POST",
                        body: JSON.stringify({ ...this.credentials(), signals }),
                    });
                }
            } catch {}
            this.flushing = this.outbox.length > 0;
            if (this.flushing) setTimeout(() => this.flush(), SEND_BATCH_MS);
        }

        leave() {
            if (!this.me || this.left) return;
            this.left = true;
            const body = JSON.stringify(this.credentials());
            // sendBeacon still goes out while the tab is closing.
            if (!navigator.sendBeacon(this.room + "/leave", body)) {
                fetch(this.room + "/leave", { method: "POST", body, keepalive: true }).catch(() => {});
            }
        }

        // `offering`: senders whose offer came in the same poll.
        onSignal(msg, offering = new Set()) {
            if (msg.NewPeer) {
                if (!this.isHost) return;
                const known = this.peers.get(this.byRemote.get(msg.NewPeer));
                // Heard again from a player not connected yet: what we sent it
                // (offer or answer) may have gone missing, so send it again.
                // A player we know nothing of (its attempt failed, or it sent
                // no offer with its join) gets an offer from us, and takes it.
                if (known) {
                    if (!known.open) this.sendDescription(known);
                } else if (!offering.has(msg.NewPeer)) {
                    this.connect(msg.NewPeer, true);
                }
            } else if (msg.PeerLeft) {
                const id = this.byRemote.get(msg.PeerLeft);
                if (id !== undefined) this.drop(id);
            } else if (msg.Signal) {
                const { sender, data } = msg.Signal;
                // A player's answer to one of our spares: that spare becomes
                // its connection. Taken by someone else meanwhile, or gone:
                // offer to the player instead (it takes our offer).
                if (this.isHost && data?.sdp?.type === "answer" && data.spare !== undefined) {
                    const known = this.peers.get(this.byRemote.get(sender));
                    if (known) {
                        if (known.spare === data.spare) this.onPeerSignal(known, data);
                        return;
                    }
                    const claimed = this.claimSpare(data.spare, sender);
                    if (claimed !== undefined) this.onPeerSignal(this.peers.get(claimed), data);
                    else this.connect(sender, true);
                    return;
                }
                let id = this.byRemote.get(sender);
                // A registry that did not name the host at join: the first
                // signal for a player comes from it.
                if (id === undefined && !this.isHost) id = this.adoptHost(sender);
                const isOffer = data?.sdp?.type === "offer";
                const gen = data?.gen ?? 0;
                if (id !== undefined && isOffer) {
                    const peer = this.peers.get(id);
                    // Both sides offered (the host heard of the player before
                    // its offer): the player takes the host's, so the host
                    // keeps its own.
                    if (this.isHost && peer.offerer && gen <= peer.gen) return;
                    // An offer replaces an attempt that never opened when it
                    // is newer (a retry through the relay), or when a player
                    // that offered hears an offer from its host (an older
                    // host page, which always offers): take the host's. A
                    // player that answered a spare takes any offer but that
                    // spare's own (resent: it is answered again): the spare
                    // went to someone else, or the host retries.
                    const replaces =
                        gen > peer.gen ||
                        (!this.isHost && peer.offerer && gen >= peer.gen) ||
                        (!this.isHost && peer.spare !== undefined && data.spare !== peer.spare);
                    if (!peer.open && replaces) {
                        this.discard(id);
                        id = undefined;
                    }
                }
                // Anything else from someone unknown is a leftover of a dropped peer.
                if (id === undefined && !isOffer) return;
                id ??= this.connect(sender, false, gen);
                this.onPeerSignal(this.peers.get(id), data);
            }
            // IdAssigned needs no action: we never address ourselves.
        }
    }

    Object.assign(gridlockWeb, { RtcSignalling });
})((window.gridlockWeb = window.gridlockWeb || {}));
