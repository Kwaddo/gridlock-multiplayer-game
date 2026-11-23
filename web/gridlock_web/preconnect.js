// Gridlock's browser plugin, part 7 of 9: player sockets opened ahead of a
// join. (See ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const {
        EV_PEER_LEFT, EV_CLOSED, PRECONNECT_IDLE_MS, PRECONNECT_MAX, PRECONNECT_RETRY_MS, PRECONNECT_RETRY_MAX_MS,
        sockets, roomHttpUrl,
        RtcSocket,
    } = gridlockWeb;

    // Player sockets opened ahead of a join, by room http url:
    // { socket, wanted, failures }: `failures`, sockets for that room that
    // closed in a row before this one (each retry waits longer). The host
    // takes no notice of a peer until it sends Join, so an open connection
    // costs it nothing but the connection.
    const preconnected = new Map();
    let preconnectSweep = null;

    function preconnect(roomUrl) {
        const room = roomHttpUrl(roomUrl);
        // Already in that game (or joining it): nothing to get ready.
        for (const s of sockets.values()) if (!s.isHost && !s.closed && s.room === room) return;
        const hit = preconnected.get(room);
        if (hit && (!hit.socket.closed || Date.now() - hit.socket.closedAt < retryDelay(hit.failures))) {
            hit.wanted = Date.now();
            return;
        }
        if (hit) preconnected.delete(room);
        if (preconnected.size >= PRECONNECT_MAX) return;
        const failures = hit ? hit.failures + 1 : 0;
        preconnected.set(room, { socket: new RtcSocket(roomUrl, false), wanted: Date.now(), failures });
        preconnectSweep ??= setInterval(sweepPreconnected, 2000);
    }

    // How long after a socket closed the next one for its room is opened,
    // `failures` closed before it.
    function retryDelay(failures) {
        return Math.min(PRECONNECT_RETRY_MS * 2 ** failures, PRECONNECT_RETRY_MAX_MS);
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
        // The game's own connect starts now: so does the socket's time to
        // give up (it may have waited a while already, unopened).
        if (!open) hit.socket.waitingSince = Date.now();
        const ms = Math.round(performance.now() - hit.socket.t0);
        console.info(`gridlock_web: joining over a connection opened ${ms} ms ago (${open ? "open" : "still connecting"})`);
        return hit.socket;
    }

    Object.assign(gridlockWeb, { preconnect, takePreconnected });
})((window.gridlockWeb = window.gridlockWeb || {}));
