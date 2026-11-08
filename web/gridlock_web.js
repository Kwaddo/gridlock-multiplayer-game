// Browser networking for Gridlock, as a miniquad plugin (no wasm-bindgen).
//
// WebRTC: one data channel per peer, negotiated through the HTTP signalling
// mailboxes of the Gridlock registry (registry/dcs/index.js). The host
// publishes offers made ahead ("spares"); a player answers one and sends the
// answer with its join request (or, without a spare, sends its own offer
// with it). Players only ever connect to the host. ICE
// servers (STUN, plus TURN relay credentials when configured) come from the
// registry's /ice route, fetched ahead of time.
//
// HTTP: fetch() for the host list.
//
// Rust drives everything by polling (transport/src/web.rs). Load this file
// after mq_js_bundle.js and before calling load().
"use strict";

(function () {
    const PLUGIN_VERSION = 3;

    // Event kinds shared with transport/src/web.rs.
    const EV_PEER_JOINED = 1;
    const EV_PEER_LEFT = 2;
    const EV_PACKET = 3;
    const EV_CLOSED = 4;

    const FORCE_RELAY = new URLSearchParams(location.search).has("relay");
    // ?nodirect makes the direct attempt fail, to check the fallback to TURN.
    const NO_DIRECT = new URLSearchParams(location.search).has("nodirect");
    const STUN_FALLBACK = [{ urls: ["stun:stun.cloudflare.com:3478", "stun:stun.l.google.com:19302"] }];
    // Poll intervals for a registry that answers polls at once (without long
    // polling, see LONG_POLL_HEADER). An idle host polls this often to notice
    // newcomers: half the interval is the average wait before a join even starts.
    const POLL_HOST_MS = 1000;
    const POLL_CONNECTING_MS = 500;
    const POLL_CONNECTED_MS = 1000; // a connected player only checks whether to resume
    const HOST_KEEP_MS = 20000; // how often the host renews its claim on the room
    // A player not connected yet re-announces itself this often, so a
    // signalling message that went missing only costs a few seconds: the
    // host resends its answer (or offer), or offers anew if it has nothing.
    const HELLO_MS = 3000;
    // A registry that holds polls until something arrives (both registries
    // do) marks its replies with this header; the page then polls again at
    // once instead of waiting out an interval.
    const LONG_POLL_HEADER = "X-Gridlock-Long-Poll";
    // Least time between two long polls, so a registry answering at once
    // (nothing held) can't spin the loop.
    const LONG_POLL_GAP_MS = 10;
    // A poll that failed (the registry's host turns away the odd request) is
    // tried again after this, doubling with each failure in a row up to the
    // usual poll interval, instead of waiting out a whole interval at once.
    const POLL_RETRY_MS = 100;
    // A poll names the messages it already has from this far back of the
    // newest one, so a registry listing a little early (isolate clocks differ)
    // does not hand them out again.
    const HAVE_WINDOW = 2500;
    const HAVE_MAX = 40;
    // An offer or answer goes out once its ICE candidates are gathered (host
    // and STUN ones, which takes a STUN round trip), or after this long
    // regardless; later ones follow on their own (trickled). Each trip
    // through the registry costs far more than this, so candidates riding
    // along with the description beats sending them after it.
    // (It goes out sooner, once a server reflexive candidate is in: the
    // address the other side needs to punch through a NAT.)
    const GATHER_MS = 250;
    // A host keeps this many offers made ahead of any join ("spares") on the
    // registry. A player answers one and sends the answer with its join, so
    // the handshake is one signalling trip, and the host gathers nothing on it.
    // (Enough that players joining in quick succession rarely pick one
    // already taken, which costs them the slow way round.)
    const SPARE_COUNT = 4;
    // Spares are made anew this often (a NAT forgets an idle mapping after a
    // while); the ones they replace still take answers this much longer.
    const SPARE_ROTATE_MS = 30000;
    const SPARE_GRACE_MS = 30000;
    const SPARE_TICK_MS = 5000;
    // Nobody waits on a spare, so it goes out with every candidate (up to this long).
    const SPARE_GATHER_MS = 3000;
    // A player fetches a room's spare when it points at the room (before it
    // clicks join), and uses what it fetched for this long.
    const PREWARM_MS = 15000;
    // Games on the player's own network are connected to as soon as they are
    // listed (see preconnect), so joining one only waits for the game's own
    // Join and Welcome. A connection nobody asked for in this long is closed.
    const PRECONNECT_IDLE_MS = 10000;
    const PRECONNECT_MAX = 4;
    // A connection ahead that failed (the host just left, still listed) is
    // tried again after this, not on every frame.
    const PRECONNECT_RETRY_MS = 5000;
    // ICE candidates come in bursts; signals queued within this long go out
    // as one request.
    const SEND_BATCH_MS = 20;
    const DISCONNECT_GRACE_MS = 3000;
    // TURN is the last resort: a connection first tries direct paths only
    // (STUN), and the offerer starts over with the relay when no direct path
    // opened this long after the answer arrived, or the attempt failed.
    const DIRECT_ATTEMPT_MS = 2500;
    // ICE servers are fetched ahead of a join and reused this long (TURN
    // credentials are minted for a day).
    const ICE_CACHE_MS = 30 * 60 * 1000;
    // Packets bigger than this need several SCTP chunks, and losing any one
    // loses the whole packet on the unreliable channel; they go on a second,
    // reliable (but still unordered) channel instead: the maze, player lists.
    const RELIABLE_MIN_BYTES = 1100;
    // ...unless that channel is backed up this far, when they take their chances.
    const RELIABLE_MAX_BUFFERED = 256 * 1024;
    const CHANNEL = "gridlock";
    const RELIABLE_CHANNEL = "gridlock-r";

    const decoder = new TextDecoder();
    const sockets = new Map(); // handle -> RtcSocket
    const requests = new Map(); // id -> { done, status, body }
    const sounds = new Map(); // id -> AudioBuffer
    let nextHandle = 1;

    // Browsers keep audio suspended until the user interacts with the page.
    let audioContext = null;
    function audio() {
        if (!audioContext) {
            audioContext = new AudioContext();
            const resume = () => audioContext.resume();
            for (const ev of ["pointerdown", "keydown", "touchstart"]) {
                window.addEventListener(ev, resume, { capture: true });
            }
        }
        return audioContext;
    }

    function readString(ptr, len) {
        return decoder.decode(new Uint8Array(wasm_memory.buffer, ptr, len));
    }

    function readBytes(ptr, len) {
        return new Uint8Array(wasm_memory.buffer, ptr, len).slice();
    }

    function writeU32(ptr, value) {
        new Uint32Array(wasm_memory.buffer, ptr, 1)[0] = value >>> 0;
    }

    function writeBytes(ptr, bytes) {
        new Uint8Array(wasm_memory.buffer, ptr, bytes.length).set(bytes);
    }

    // ------------------------------------------------------------ WebRTC

    // Who offers: the player, which sends its offer with its join request, so
    // the host learns of the player and gets its offer in one message and the
    // whole handshake is two signalling trips (offer, answer). The host still
    // offers to a player that joined without one (a page from before this),
    // and a player takes the host's offer when one comes (an older host page).
    class RtcSocket {
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

        // ---- spares (host): offers made ahead of any join, published on the
        // registry; a player fetches one, answers it and joins with the
        // answer (answerSpare), and the host claims it (claimSpare). SPARE_COUNT
        // are kept live (published); each is replaced after SPARE_ROTATE_MS and
        // still takes answers for SPARE_GRACE_MS after that.
        async keepSpares() {
            while (!this.closed && !this.noSpares) {
                this.tendSpares();
                await sleep(SPARE_TICK_MS);
            }
            this.dropSpares();
        }

        tendSpares() {
            const now = Date.now();
            let changed = false;
            for (const [oid, spare] of this.spares) {
                if (now - spare.at > SPARE_ROTATE_MS + SPARE_GRACE_MS) {
                    this.spares.delete(oid);
                    try {
                        spare.peer.pc.close();
                    } catch {}
                } else if (spare.live && now - spare.at > SPARE_ROTATE_MS) {
                    spare.live = false;
                    changed = true;
                }
            }
            const live = [...this.spares.values()].filter((s) => s.live).length + this.makingSpares;
            for (let i = live; i < SPARE_COUNT; i++) this.makeSpare();
            if (changed) this.publishSpares();
        }

        async makeSpare() {
            this.makingSpares++;
            const oid = randomHex(6);
            const { id, peer } = this.newPeer(null, true, 0, oid);
            try {
                await peer.offered;
            } catch {
                try {
                    peer.pc.close();
                } catch {}
                this.makingSpares--;
                return;
            }
            this.makingSpares--;
            if (this.closed || this.noSpares) {
                try {
                    peer.pc.close();
                } catch {}
                return;
            }
            this.spares.set(oid, { id, peer, at: Date.now(), live: true });
            this.publishSpares();
        }

        // Tell the registry which spares are live. One request at a time;
        // changes meanwhile go out in one more after it.
        async publishSpares() {
            if (!this.me || this.closed || this.noSpares) return;
            if (this.publishing) {
                this.republish = true;
                return;
            }
            this.publishing = true;
            do {
                this.republish = false;
                const offers = [...this.spares]
                    .filter(([, s]) => s.live)
                    .map(([oid, s]) => ({ oid, data: this.describe(s.peer) }));
                try {
                    const resp = await fetch(this.room + "/spare", {
                        method: "POST",
                        body: JSON.stringify({ ...this.credentials(), offers }),
                    });
                    // A registry from before spares: players offer, as before.
                    if (resp.status === 404 || resp.status === 405) this.noSpares = true;
                } catch {}
            } while (this.republish && !this.closed && !this.noSpares);
            this.publishing = false;
        }

        // A player answered spare `oid`: it becomes that player's peer. Its
        // id, or undefined when that spare is gone (taken, or expired).
        claimSpare(oid, sender) {
            const spare = this.spares.get(oid);
            if (!spare) return undefined;
            this.spares.delete(oid);
            const { id, peer } = spare;
            peer.remoteId = sender;
            peer.t0 = performance.now();
            this.peers.set(id, peer);
            this.byRemote.set(sender, id);
            // Off the list before anyone else takes it, and a new one made.
            if (spare.live) this.publishSpares();
            this.tendSpares();
            return id;
        }

        dropSpares() {
            for (const { peer } of this.spares.values()) {
                try {
                    peer.pc.close();
                } catch {}
            }
            this.spares.clear();
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

        // A peer: { remoteId, pc, channel, reliable, pending, open, gen,
        // offerer, described, offered }. `remoteId` is the other side's
        // signalling id; a player's first offer goes out before it knows it
        // (null until the join answers). `gen` counts attempts for this peer:
        // 0 is direct only, 1 adds the TURN relay. Signals carry it, so ones
        // from an abandoned attempt are ignored. `spare`: one of the
        // host's spares (see keepSpares), tagged with its oid, kept out of
        // `peers` until a player claims it; a player's peer answering one
        // carries its oid too.
        connect(remoteId, offerer, gen = 0) {
            return this.newPeer(remoteId, offerer, gen).id;
        }

        newPeer(remoteId, offerer, gen = 0, spare = undefined) {
            const id = this.nextPeer++;
            const relay = gen > 0 || FORCE_RELAY;
            const pc = new RTCPeerConnection({
                iceServers: relay ? this.iceServers : this.iceServers.filter(isStunOnly),
                // ?relay in the page URL forces every connection through TURN (for testing the relay).
                iceTransportPolicy: FORCE_RELAY || (NO_DIRECT && !relay) ? "relay" : "all",
            });
            // `described`: our offer/answer went out, so candidates found from
            // now on are sent on their own
            const peer = { remoteId, pc, channel: null, reliable: null, pending: [], open: false, gen, offerer, described: false, spare };
            // when the host heard of this player, for the log
            peer.t0 = performance.now();
            if (spare === undefined) {
                this.peers.set(id, peer);
                if (remoteId !== null) this.byRemote.set(remoteId, id);
            }

            // Trickle ICE: a candidate gathered before the description went out
            // is already in it; later ones are sent as they come.
            pc.onicecandidate = (e) => {
                if (!e.candidate || !peer.described || this.peers.get(id) !== peer) return;
                this.signal({ Signal: { receiver: peer.remoteId, data: { candidate: e.candidate.toJSON(), gen } } });
            };

            pc.onconnectionstatechange = () => {
                const state = pc.connectionState;
                if (state === "connected" && !this.isHost && this.peers.get(id) === peer) this.mark("ice connected");
                if (state === "failed" && this.retryWithRelay(id)) return;
                if (state === "failed" || state === "closed") this.drop(id);
                // A closed or crashed browser shows up as "disconnected" within
                // seconds, but "failed" only after ~30 s; brief network blips
                // recover well within this grace.
                if (state === "disconnected") {
                    setTimeout(() => {
                        if (pc.connectionState === "disconnected") this.drop(id);
                    }, DISCONNECT_GRACE_MS);
                }
            };

            if (offerer) {
                // Game traffic is UDP-like: unordered, no retransmits. Big
                // packets get a reliable channel of their own (RELIABLE_MIN_BYTES).
                this.attach(id, peer, pc.createDataChannel(CHANNEL, { ordered: false, maxRetransmits: 0 }));
                this.attach(id, peer, pc.createDataChannel(RELIABLE_CHANNEL, { ordered: false }));
                peer.offered = pc
                    .createOffer()
                    .then((offer) => pc.setLocalDescription(offer))
                    .then(() => gathered(pc, spare === undefined ? { full: relay } : { full: true, ms: SPARE_GATHER_MS }));
                // A player's first offer goes with its join (start); every
                // other one is sent as a signal.
                peer.offered.then(() => remoteId !== null && this.sendDescription(peer)).catch(() => this.drop(id));
            } else {
                pc.ondatachannel = (e) => this.attach(id, peer, e.channel);
            }
            return { id, peer };
        }

        // The direct attempt failed: the offerer starts over with the TURN relay.
        retryWithRelay(id) {
            const peer = this.peers.get(id);
            if (!peer || !peer.offerer || peer.open || peer.gen > 0 || peer.remoteId === null || FORCE_RELAY || this.closed) {
                return false;
            }
            this.discard(id);
            this.connect(peer.remoteId, true, 1);
            return true;
        }

        // Forget a peer that never opened, without telling the game.
        discard(id) {
            const peer = this.peers.get(id);
            if (!peer) return;
            this.peers.delete(id);
            if (this.byRemote.get(peer.remoteId) === id) this.byRemote.delete(peer.remoteId);
            try {
                peer.pc.close();
            } catch {}
        }

        // Our offer or answer, with every ICE candidate gathered so far in it.
        // Candidates found after this are trickled.
        describe(peer) {
            peer.described = true;
            const data = { sdp: peer.pc.localDescription.toJSON(), gen: peer.gen };
            if (peer.spare !== undefined) data.spare = peer.spare;
            return data;
        }

        sendDescription(peer) {
            if (!peer.pc.localDescription || this.closed) return;
            this.signal({ Signal: { receiver: peer.remoteId, data: this.describe(peer) } });
        }

        async onPeerSignal(peer, data) {
            if (!peer || (data.gen ?? 0) !== peer.gen) return;
            const pc = peer.pc;
            try {
                if (data.sdp) {
                    // The same offer again: our answer went missing; resend it.
                    if (data.sdp.type === "offer" && pc.remoteDescription) {
                        if (pc.localDescription) this.sendDescription(peer);
                        return;
                    }
                    // An answer we already have (the offer was resent).
                    if (data.sdp.type === "answer" && pc.remoteDescription) return;
                    await pc.setRemoteDescription(data.sdp);
                    for (const c of peer.pending) await pc.addIceCandidate(c);
                    peer.pending = [];
                    if (data.sdp.type === "offer") {
                        await pc.setLocalDescription(await pc.createAnswer());
                        await gathered(pc, { full: peer.gen > 0 || FORCE_RELAY });
                        this.sendDescription(peer);
                    } else if (peer.offerer && peer.gen === 0) {
                        // The answer is in: give the direct attempt a moment.
                        const id = this.byRemote.get(peer.remoteId);
                        setTimeout(() => {
                            if (this.peers.get(id) === peer && !peer.open) this.retryWithRelay(id);
                        }, DIRECT_ATTEMPT_MS);
                    }
                } else if (data.candidate) {
                    // Candidates can arrive before the description they belong to.
                    if (pc.remoteDescription) await pc.addIceCandidate(data.candidate);
                    else peer.pending.push(data.candidate);
                }
            } catch (e) {
                console.warn("gridlock_web: signalling error", e);
            }
        }

        attach(id, peer, channel) {
            channel.binaryType = "arraybuffer";
            channel.onmessage = (e) => {
                if (e.data instanceof ArrayBuffer && this.peers.get(id) === peer) {
                    this.events.push({ kind: EV_PACKET, peer: id, data: new Uint8Array(e.data) });
                }
            };
            // The reliable channel only helps big packets along; the game's
            // link is the main one, and lives and dies with it.
            if (channel.label === RELIABLE_CHANNEL) {
                peer.reliable = channel;
                channel.onclose = () => {
                    if (peer.reliable === channel) peer.reliable = null;
                };
                return;
            }
            peer.channel = channel;
            channel.onopen = () => {
                if (this.peers.get(id) !== peer) return;
                peer.open = true;
                this.events.push({ kind: EV_PEER_JOINED, peer: id, data: null });
                const how = peer.spare !== undefined ? "spare" : peer.offerer ? "our offer" : "its offer";
                if (this.isHost) {
                    const ms = Math.round(performance.now() - peer.t0);
                    console.info(`gridlock_web: player connected ${ms} ms after its join reached us (${how}, gen ${peer.gen})`);
                } else {
                    this.mark("open");
                    const total = Math.round(performance.now() - this.t0);
                    console.info(`gridlock_web: connected to host in ${total} ms (${how}, gen ${peer.gen}): ${this.marks.join(", ")}`);
                }
            };
            channel.onclose = () => this.drop(id);
        }

        send(id, bytes) {
            const peer = this.peers.get(id);
            if (!peer || !peer.open) return;
            const reliable = peer.reliable;
            const channel =
                bytes.length > RELIABLE_MIN_BYTES &&
                reliable?.readyState === "open" &&
                reliable.bufferedAmount < RELIABLE_MAX_BUFFERED
                    ? reliable
                    : peer.channel;
            if (channel.readyState !== "open") return;
            try {
                channel.send(bytes);
            } catch {
                // send buffer full: drop the packet, like UDP would
            }
        }

        drop(id) {
            const peer = this.peers.get(id);
            if (!peer) return;
            this.peers.delete(id);
            if (this.byRemote.get(peer.remoteId) === id) this.byRemote.delete(peer.remoteId);
            try {
                peer.pc.close();
            } catch {}
            if (peer.open) this.events.push({ kind: EV_PEER_LEFT, peer: id, data: null });
        }

        shutdown() {
            if (this.closed) return;
            this.closed = true;
            for (const id of [...this.peers.keys()]) this.drop(id);
            this.dropSpares();
            if (this.onPageHide) window.removeEventListener("pagehide", this.onPageHide);
            this.leave();
            this.events.push({ kind: EV_CLOSED, peer: 0, data: null });
        }
    }

    // Timers in hidden tabs are throttled (to once a minute after a while),
    // which would leave a hidden host deaf to newcomers; a worker's timers are
    // not, so polling waits on those.
    let timerWorker = null;
    const timerCallbacks = new Map();
    let nextTimer = 1;
    function sleep(ms) {
        if (!timerWorker && typeof Worker !== "undefined") {
            try {
                const source = "onmessage = (e) => setTimeout(() => postMessage(e.data.id), e.data.ms);";
                timerWorker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
                timerWorker.onmessage = (e) => {
                    timerCallbacks.get(e.data)?.();
                    timerCallbacks.delete(e.data);
                };
            } catch {
                timerWorker = false;
            }
        }
        if (!timerWorker) return new Promise((r) => setTimeout(r, ms));
        return new Promise((r) => {
            const id = nextTimer++;
            timerCallbacks.set(id, r);
            timerWorker.postMessage({ id, ms });
        });
    }

    // The game passes wss://<registry>/room/<code>; signalling now runs over
    // https://<registry>/room/<code>/..., and ICE servers live at https://<registry>/ice.
    function roomHttpUrl(roomUrl) {
        const url = new URL(roomUrl);
        url.protocol = url.protocol === "wss:" ? "https:" : "http:";
        url.search = "";
        return url.toString().replace(/\/$/, "");
    }

    // Resolves once a connection has gathered its ICE candidates, or after
    // `ms`, whichever comes first; unless `full`, already once a server
    // reflexive candidate is in (the rest trickle). (A candidate is in the
    // local description by the time its event fires.)
    function gathered(pc, { full = false, ms = GATHER_MS } = {}) {
        if (pc.iceGatheringState === "complete") return Promise.resolve();
        return new Promise((resolve) => {
            const done = () => {
                clearTimeout(timer);
                pc.removeEventListener("icegatheringstatechange", check);
                pc.removeEventListener("icecandidate", candidate);
                resolve();
            };
            const check = () => pc.iceGatheringState === "complete" && done();
            const candidate = (e) => !full && isReflexive(e.candidate) && done();
            pc.addEventListener("icegatheringstatechange", check);
            pc.addEventListener("icecandidate", candidate);
            const timer = setTimeout(done, ms);
        });
    }

    function isReflexive(candidate) {
        return Boolean(candidate) && (candidate.type === "srflx" || / typ srflx /.test(candidate.candidate));
    }

    function randomHex(bytes) {
        // (getRandomValues, unlike randomUUID, works on a plain-http LAN page)
        return [...crypto.getRandomValues(new Uint8Array(bytes))].map((b) => b.toString(16).padStart(2, "0")).join("");
    }

    function isStunOnly(server) {
        return [].concat(server.urls).every((u) => u.startsWith("stun:"));
    }

    // ICE servers by registry origin: { at, servers: Promise }. Fetched as
    // soon as the game first talks to a registry (the host list, see
    // gridlock_http_start), so a join does not wait for them.
    const iceCache = new Map();

    function iceServersFor(anyUrl) {
        let origin;
        try {
            origin = new URL(anyUrl).origin;
        } catch {
            return Promise.resolve(STUN_FALLBACK);
        }
        const cached = iceCache.get(origin);
        if (cached && Date.now() - cached.at < ICE_CACHE_MS) return cached.servers;
        const servers = fetchIceServers(origin);
        iceCache.set(origin, { at: Date.now(), servers });
        // A failed fetch is not kept: the next join asks again.
        servers.then((s) => s === STUN_FALLBACK && iceCache.get(origin)?.servers === servers && iceCache.delete(origin));
        return servers;
    }

    // A room's spare offer, fetched ahead of a join: room http url ->
    // { at, spare: Promise<{ host, oid, data } | null> }.
    const prewarmed = new Map();

    // The player points at a room (hovers it in the list): have its ICE
    // servers and the host's spare offer ready for when it joins.
    function prewarm(room) {
        const hit = prewarmed.get(room);
        if (hit && Date.now() - hit.at < PREWARM_MS) return;
        iceServersFor(room);
        prewarmed.set(room, { at: Date.now(), spare: fetchSpare(room) });
    }

    // The spare to join with: the one fetched ahead (each is used once), or
    // one fetched now.
    function spareFor(room) {
        const hit = prewarmed.get(room);
        prewarmed.delete(room);
        return hit && Date.now() - hit.at < PREWARM_MS ? hit.spare : fetchSpare(room);
    }

    // Player sockets opened ahead of a join, by room http url:
    // { socket, wanted, at }. The host takes no notice of a peer until it sends
    // Join, so an open connection costs it nothing but the connection.
    const preconnected = new Map();
    let preconnectSweep = null;

    function preconnect(roomUrl) {
        const room = roomHttpUrl(roomUrl);
        // Already in that game (or joining it): nothing to get ready.
        for (const s of sockets.values()) if (!s.isHost && !s.closed && s.room === room) return;
        const hit = preconnected.get(room);
        if (hit && (!hit.socket.closed || Date.now() - hit.at < PRECONNECT_RETRY_MS)) {
            hit.wanted = Date.now();
            return;
        }
        if (hit) preconnected.delete(room);
        if (preconnected.size >= PRECONNECT_MAX) return;
        preconnected.set(room, { socket: new RtcSocket(roomUrl, false), wanted: Date.now(), at: Date.now() });
        preconnectSweep ??= setInterval(sweepPreconnected, 2000);
    }

    function sweepPreconnected() {
        for (const [room, p] of preconnected) {
            if (Date.now() - p.wanted > PRECONNECT_IDLE_MS) {
                p.socket.shutdown();
                preconnected.delete(room);
            }
        }
        if (!preconnected.size) {
            clearInterval(preconnectSweep);
            preconnectSweep = null;
        }
    }

    // The socket opened ahead for this room, if it is still good.
    function takePreconnected(roomUrl) {
        let room;
        try {
            room = roomHttpUrl(roomUrl);
        } catch {
            return null;
        }
        const hit = preconnected.get(room);
        preconnected.delete(room);
        // (Not one whose host went away meanwhile: a fresh join instead.)
        const lost = hit?.socket.events.some((e) => e.kind === EV_PEER_LEFT || e.kind === EV_CLOSED);
        if (!hit || hit.socket.closed || lost) {
            hit?.socket.shutdown();
            return null;
        }
        const open = [...hit.socket.peers.values()].some((p) => p.open);
        const ms = Math.round(performance.now() - hit.socket.t0);
        console.info(`gridlock_web: joining over a connection opened ${ms} ms ago (${open ? "open" : "still connecting"})`);
        return hit.socket;
    }

    async function fetchSpare(room) {
        try {
            const resp = await fetch(room + "/spare", { cache: "no-store" });
            if (!resp.ok) return null;
            const spare = await resp.json();
            const valid = typeof spare?.host === "string" && typeof spare.oid === "string" && spare.data?.sdp?.type === "offer";
            return valid ? spare : null;
        } catch {
            return null; // no spare (an older registry or host page): the player offers
        }
    }

    async function fetchIceServers(origin) {
        try {
            const resp = await fetch(origin + "/ice");
            const body = await resp.json();
            // An empty list is deliberate: the local server (registry/local/) has
            // no STUN to offer offline, and LAN peers connect without one.
            if (Array.isArray(body.iceServers)) return body.iceServers;
        } catch {}
        return STUN_FALLBACK;
    }

    // Models are too big to ship with the page, so build_web.sh can host them
    // elsewhere and set GRIDLOCK_MODELS_URL (plus a version) in index.html. They
    // are downloaded once and kept in Cache Storage; a new version starts a new
    // cache and drops the old one. Every other file loads from the page as usual.
    const MODELS_PREFIX = "assets/models/";
    const MODELS_URL = window.GRIDLOCK_MODELS_URL || "";
    const MODELS_CACHE = "gridlock-models-" + (window.GRIDLOCK_MODELS_VERSION || "dev");

    let modelsCache = null;
    function openModelsCache() {
        if (!modelsCache) {
            modelsCache = (async () => {
                if (!("caches" in window)) return null;
                try {
                    for (const name of await caches.keys()) {
                        if (name.startsWith("gridlock-models-") && name !== MODELS_CACHE) caches.delete(name);
                    }
                    navigator.storage?.persist?.();
                    return await caches.open(MODELS_CACHE);
                } catch {
                    return null; // private windows can refuse storage; download every time
                }
            })();
        }
        return modelsCache;
    }

    // Bytes received so far per model path, for the loading screen
    // (gridlock_download_received / gridlock_download_total).
    const downloads = new Map(); // path -> { received, total }
    // One download per model at a time: the game asking for a model that is
    // being preloaded waits for that download instead of starting another.
    const inflight = new Map(); // path -> Promise<Uint8Array | null>

    function modelUrl(path) {
        const file = path.slice(MODELS_PREFIX.length).split("/").map(encodeURIComponent).join("/");
        return MODELS_URL.replace(/\/$/, "") + "/" + file + "?v=" + (window.GRIDLOCK_MODELS_VERSION || "dev");
    }

    function loadModel(path) {
        if (!inflight.has(path)) {
            const download = fetchModel(path).finally(() => inflight.delete(path));
            inflight.set(path, download);
        }
        return inflight.get(path);
    }

    // ---- preloading: from the title screen on, download the models the game
    // will need into Cache Storage (gridlock_preload), so joining a game only
    // reads them from disk. Stops starting new downloads once the game loads
    // models itself, so the two do not compete for bandwidth.
    const PRELOAD_PARALLEL = 3;
    const preloadQueue = [];
    let preloadActive = 0;
    let preloadDone = 0;
    let preloadTotal = 0;
    let preloadPaused = false;

    function pumpPreload() {
        while (!preloadPaused && preloadActive < PRELOAD_PARALLEL && preloadQueue.length) {
            const path = preloadQueue.shift();
            preloadActive++;
            preloadModel(path)
                .catch(() => {})
                .finally(() => {
                    preloadActive--;
                    preloadDone++;
                    pumpPreload();
                });
        }
    }

    // Like loadModel, but a model already cached is not read back.
    async function preloadModel(path) {
        if (!inflight.has(path)) {
            const cache = await openModelsCache();
            if (cache && (await cache.match(modelUrl(path)))) return;
        }
        await loadModel(path);
    }

    async function fetchModel(path) {
        const url = modelUrl(path);
        const cache = await openModelsCache();
        const cached = cache && (await cache.match(url));
        if (cached) {
            const bytes = new Uint8Array(await cached.arrayBuffer());
            downloads.set(path, { received: bytes.length, total: bytes.length });
            return bytes;
        }
        const resp = await fetch(url);
        if (!resp.ok) return null;
        const progress = { received: 0, total: Number(resp.headers.get("Content-Length")) || 0 };
        downloads.set(path, progress);
        // Read in chunks so the loading screen can show how far along it is.
        const chunks = [];
        const reader = resp.body.getReader();
        for (;;) {
            const { done, value } = await reader.read();
            if (done) break;
            chunks.push(value);
            progress.received += value.length;
        }
        const bytes = new Uint8Array(progress.received);
        let at = 0;
        for (const c of chunks) {
            bytes.set(c, at);
            at += c.length;
        }
        progress.total = bytes.length;
        if (cache) {
            const headers = { "Content-Type": resp.headers.get("Content-Type") || "application/octet-stream" };
            try {
                await cache.put(url, new Response(bytes, { headers }));
            } catch {} // quota full: still use this copy
        }
        return bytes;
    }

    // ------------------------------------------------------------ hidden tabs

    // The game advances once per animation frame, and browsers stop animation
    // frames in hidden tabs, which would freeze a game hosted in one (players
    // could connect but nobody would answer). While hidden with a game open,
    // a worker's ticks run the frames instead: worker timers and the messages
    // they post keep going in the background. Those frames only keep the game
    // and its connections going, without drawing (see platform::page_hidden),
    // so 20 a second is plenty.
    const HIDDEN_FRAME_MS = 50;
    let hiddenTicker = null;

    function keepRunningWhenHidden() {
        if (hiddenTicker || typeof Worker === "undefined") return;
        const source = `setInterval(() => postMessage(0), ${HIDDEN_FRAME_MS});`;
        try {
            hiddenTicker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
        } catch {
            return; // no workers (e.g. a strict content policy): the game pauses when hidden
        }
        hiddenTicker.onmessage = () => {
            const inGame = [...sockets.values()].some((s) => !s.closed);
            if (document.hidden && inGame && wasm_exports) wasm_exports.frame();
        };
    }

    // ------------------------------------------------------------ imports

    function register(importObject) {
        const env = importObject.env;

        // ---- file loading: models from GRIDLOCK_MODELS_URL, through Cache Storage

        if (MODELS_URL) {
            const loadFromPage = env.fs_load_file;
            env.fs_load_file = function (ptr, len) {
                const path = UTF8ToString(ptr, len);
                if (!path.startsWith(MODELS_PREFIX)) return loadFromPage(ptr, len);
                const id = FS.unique_id++;
                if (!preloadPaused) {
                    preloadPaused = true;
                    // The queued preloads will never start now; drop them from the
                    // count so the title screen's progress note does not stay up.
                    preloadTotal -= preloadQueue.length;
                    preloadQueue.length = 0;
                }
                loadModel(path)
                    .catch(() => null)
                    .then((bytes) => {
                        FS.loaded_files[id] = bytes;
                        wasm_exports.file_loaded(id);
                    });
                return id;
            };
        }

        // ---- client: download progress of one model, for the loading screen
        // (client/src/app/asset_loader.rs). -1 when it is not being downloaded.

        env.gridlock_preload = function (ptr, len) {
            const path = UTF8ToString(ptr, len);
            if (!MODELS_URL || preloadPaused || !path.startsWith(MODELS_PREFIX) || preloadQueue.includes(path)) return;
            preloadQueue.push(path);
            preloadTotal++;
            pumpPreload();
        };
        // Preloaded models so far and in all, as two u32s at out_ptr.
        env.gridlock_preload_status = function (outPtr) {
            writeU32(outPtr, preloadDone);
            writeU32(outPtr + 4, preloadTotal);
        };

        env.gridlock_download_received = function (ptr, len) {
            return downloads.get(UTF8ToString(ptr, len))?.received ?? -1;
        };
        env.gridlock_download_total = function (ptr, len) {
            return downloads.get(UTF8ToString(ptr, len))?.total ?? -1;
        };

        // ---- platform crate: clocks and randomness (platform/src/lib.rs)

        env.gridlock_now_ms = () => performance.now();
        env.gridlock_page_hidden = () => (document.hidden ? 1 : 0);

        // miniquad's loader uploads only the first matrix of a uniform array
        // (it ignores `count`), which the GPU skinning shader's joint array
        // needs whole (client/src/types/animated_model.rs).
        env.glUniformMatrix4fv = function (location, count, transpose, value) {
            const data = new Float32Array(wasm_memory.buffer, value, 16 * count);
            gl.uniformMatrix4fv(GL.uniforms[location], !!transpose, data);
        };
        env.gridlock_force_cpu_skinning = () => (new URLSearchParams(location.search).has("cpuskin") ? 1 : 0);
        env.gridlock_unix_ms = () => Date.now();
        env.gridlock_random_fill = function (ptr, len) {
            // getRandomValues fills at most 65536 bytes per call.
            for (let off = 0; off < len; off += 65536) {
                crypto.getRandomValues(new Uint8Array(wasm_memory.buffer, ptr + off, Math.min(65536, len - off)));
            }
        };

        // ---- platform::storage: settings and presets in localStorage

        const STORAGE_PREFIX = "gridlock:";
        const encoder = new TextEncoder();
        let storageTaken = null;
        env.gridlock_storage_get = function (keyPtr, keyLen, lenOut) {
            let value = null;
            try {
                value = localStorage.getItem(STORAGE_PREFIX + readString(keyPtr, keyLen));
            } catch {} // storage disabled (e.g. some private modes)
            if (value === null) return 0;
            storageTaken = encoder.encode(value);
            writeU32(lenOut, storageTaken.length);
            return 1;
        };
        env.gridlock_storage_take = function (ptr) {
            if (storageTaken) writeBytes(ptr, storageTaken);
            storageTaken = null;
        };
        env.gridlock_storage_set = function (keyPtr, keyLen, valuePtr, valueLen) {
            try {
                localStorage.setItem(STORAGE_PREFIX + readString(keyPtr, keyLen), readString(valuePtr, valueLen));
                return 1;
            } catch {
                return 0; // quota exceeded or storage disabled
            }
        };

        // ---- platform::page_setting: window.GRIDLOCK_<name>, set by the page
        // (the local server sets GRIDLOCK_REGISTRY). Bytes via gridlock_storage_take.

        env.gridlock_page_setting = function (namePtr, nameLen, lenOut) {
            const value = window["GRIDLOCK_" + readString(namePtr, nameLen)];
            if (typeof value !== "string") return 0;
            storageTaken = encoder.encode(value);
            writeU32(lenOut, storageTaken.length);
            return 1;
        };

        // ---- pointer lock (replaces miniquad's version)
        // The game re-asserts the grab often. Browsers grant pointer lock only from
        // a user gesture and reject rapid repeats, so remember what the game wants,
        // try once when that changes, and otherwise lock on the next click.

        let wantLock = false;
        const lock = () => {
            if (wantLock && document.pointerLockElement !== canvas) {
                try {
                    const p = canvas.requestPointerLock();
                    if (p && p.catch) p.catch(() => {});
                } catch {}
            }
        };
        canvas.addEventListener("mousedown", lock);
        env.sapp_set_cursor_grab = function (grab) {
            const want = grab !== 0;
            if (want === wantLock) return;
            wantLock = want;
            if (want) lock();
            else if (document.pointerLockElement === canvas) document.exitPointerLock();
        };

        // ---- client audio (client/src/audio.rs): Web Audio, with pitch via playbackRate

        env.gridlock_audio_load = function (ptr, len) {
            const id = nextHandle++;
            const bytes = readBytes(ptr, len);
            audio()
                .decodeAudioData(bytes.buffer)
                .then((buffer) => sounds.set(id, buffer))
                .catch((e) => console.warn("gridlock_web: could not decode sound", e));
            return id;
        };

        // Plays nothing until decoding finishes (it is fast for short effects).
        env.gridlock_audio_play = function (id, speed, volume) {
            const buffer = sounds.get(id);
            if (!buffer) return;
            const ctx = audio();
            const source = ctx.createBufferSource();
            const gain = ctx.createGain();
            source.buffer = buffer;
            source.playbackRate.value = speed;
            gain.gain.value = volume;
            source.connect(gain).connect(ctx.destination);
            source.start();
        };

        // ---- transport crate: WebRTC and fetch (transport/src/web.rs)

        env.gridlock_rtc_open = function (urlPtr, urlLen, isHost) {
            const handle = nextHandle++;
            const url = readString(urlPtr, urlLen);
            const ahead = isHost === 0 ? takePreconnected(url) : null;
            sockets.set(handle, ahead || new RtcSocket(url, isHost !== 0));
            keepRunningWhenHidden();
            return handle;
        };

        env.gridlock_rtc_preconnect = function (urlPtr, urlLen) {
            try {
                preconnect(readString(urlPtr, urlLen));
            } catch {}
        };

        env.gridlock_rtc_prewarm = function (urlPtr, urlLen) {
            try {
                prewarm(roomHttpUrl(readString(urlPtr, urlLen)));
            } catch {}
        };

        env.gridlock_rtc_send = function (handle, peer, ptr, len) {
            const socket = sockets.get(handle);
            if (socket) socket.send(peer, readBytes(ptr, len));
        };

        // Returns the next event kind (0 = none) and writes its peer and payload
        // length; the payload is then copied out with gridlock_rtc_take.
        env.gridlock_rtc_next = function (handle, peerOut, lenOut) {
            const socket = sockets.get(handle);
            const ev = socket && socket.events.shift();
            if (!ev) return 0;
            socket.taken = ev.data;
            writeU32(peerOut, ev.peer);
            writeU32(lenOut, ev.data ? ev.data.length : 0);
            return ev.kind;
        };

        env.gridlock_rtc_take = function (handle, ptr) {
            const socket = sockets.get(handle);
            if (socket && socket.taken) writeBytes(ptr, socket.taken);
            if (socket) socket.taken = null;
        };

        env.gridlock_rtc_close = function (handle) {
            const socket = sockets.get(handle);
            if (socket) socket.shutdown();
            sockets.delete(handle);
        };

        env.gridlock_http_start = function (methodPtr, methodLen, urlPtr, urlLen, bodyPtr, bodyLen, authPtr, authLen) {
            const id = nextHandle++;
            const url = readString(urlPtr, urlLen);
            // The game asking a registry for its host list is about to join or
            // host through it: have its ICE servers ready by then.
            if (/\/servers\/?$/.test(url)) iceServersFor(url);
            const req = { done: false, status: 0, body: new Uint8Array(0) };
            requests.set(id, req);
            const headers = { "Content-Type": "application/json" };
            if (authLen > 0) headers.Authorization = readString(authPtr, authLen);
            const method = readString(methodPtr, methodLen);
            fetch(url, {
                method,
                headers,
                body: bodyLen > 0 ? readBytes(bodyPtr, bodyLen) : undefined,
            })
                .then(async (resp) => {
                    req.body = new Uint8Array(await resp.arrayBuffer());
                    req.status = resp.status;
                })
                .catch(() => {
                    req.status = 0; // network error
                })
                .finally(() => {
                    req.done = true;
                });
            return id;
        };

        // 0 while in flight; 1 when done, with status (0 = network error) and body length written.
        env.gridlock_http_poll = function (id, statusOut, lenOut) {
            const req = requests.get(id);
            if (!req || !req.done) return 0;
            writeU32(statusOut, req.status);
            writeU32(lenOut, req.body.length);
            return 1;
        };

        env.gridlock_http_take = function (id, ptr) {
            const req = requests.get(id);
            if (req) writeBytes(ptr, req.body);
            requests.delete(id);
        };

        env.gridlock_http_cancel = function (id) {
            requests.delete(id);
        };
    }

    // Debugging aid: how each WebRTC connection is routed. From the browser console:
    //   await gridlockConnections()  ->  [{ peer, role, path: "direct" | "stun" | "relay", remote }]
    window.gridlockConnections = async function () {
        const result = [];
        for (const socket of sockets.values()) {
            for (const [peer, p] of socket.peers) {
                const stats = await p.pc.getStats();
                let pair = null;
                stats.forEach((s) => {
                    if (s.type === "candidate-pair" && s.nominated && s.state === "succeeded") pair = s;
                });
                const local = pair && stats.get(pair.localCandidateId);
                const remote = pair && stats.get(pair.remoteCandidateId);
                const types = [local?.candidateType, remote?.candidateType];
                const path = !pair ? "none" : types.includes("relay") ? "relay" : types.includes("srflx") || types.includes("prflx") ? "stun" : "direct";
                result.push({ peer, role: socket.isHost ? "host" : "player", path, remote: remote ? `${remote.address}:${remote.port}` : null });
            }
        }
        return result;
    };

    miniquad_add_plugin({
        name: "gridlock_web",
        version: PLUGIN_VERSION,
        register_plugin: register,
    });
})();
