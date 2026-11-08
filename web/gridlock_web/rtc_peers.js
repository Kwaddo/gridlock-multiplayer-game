// Gridlock's browser plugin, part 6 of 9: RtcSocket's peers, one
// RTCPeerConnection and its data channels each. (See rtc_signalling.js
// and ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const {
        EV_PEER_JOINED, EV_PEER_LEFT, EV_PACKET, EV_CLOSED, FORCE_RELAY, NO_DIRECT, SPARE_GATHER_MS,
        DISCONNECT_GRACE_MS, DIRECT_ATTEMPT_MS, RELIABLE_MIN_BYTES, RELIABLE_MAX_BUFFERED, CHANNEL, RELIABLE_CHANNEL,
        gathered, isStunOnly, RtcSpares,
    } = gridlockWeb;

    class RtcSocket extends RtcSpares {
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

    Object.assign(gridlockWeb, { RtcSocket });
})((window.gridlockWeb = window.gridlockWeb || {}));
