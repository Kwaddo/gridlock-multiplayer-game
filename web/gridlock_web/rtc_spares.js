// Gridlock's browser plugin, part 5 of 9: RtcSocket's spare offers (host).
// (See rtc_signalling.js and ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const {
        SPARE_COUNT, SPARE_ROTATE_MS, SPARE_GRACE_MS, SPARE_TICK_MS, sleep, randomHex, RtcSignalling,
    } = gridlockWeb;

    class RtcSpares extends RtcSignalling {
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
    }

    Object.assign(gridlockWeb, { RtcSpares });
})((window.gridlockWeb = window.gridlockWeb || {}));
