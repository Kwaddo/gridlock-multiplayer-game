// Gridlock's browser plugin, part 7 of 8: player sockets opened ahead of a
// join. (See ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const {
        EV_PEER_LEFT, EV_CLOSED, PRECONNECT_IDLE_MS, PRECONNECT_MAX, PRECONNECT_RETRY_MS, sockets, roomHttpUrl,
        RtcSocket,
    } = gridlockWeb;

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

    Object.assign(gridlockWeb, { preconnect, takePreconnected });
})((window.gridlockWeb = window.gridlockWeb || {}));
