// Gridlock's browser plugin, part 1 of 9: the protocol constants shared with
// transport/src/web.rs and the networking tuning. Every part adds what it
// defines to window.gridlockWeb; ../gridlock_web.js, loaded last, registers
// the plugin (see the header there).
"use strict";

(function (gridlockWeb) {
    const PLUGIN_VERSION = 4;

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
    // Listed games (every LAN one, the first few online, the one pointed at)
    // are connected to ahead (see preconnect), so joining one only waits for
    // the game's own Join and Welcome. A connection nobody asked for in this
    // long is closed.
    const PRECONNECT_IDLE_MS = 10000;
    const PRECONNECT_MAX = 8;
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

    Object.assign(gridlockWeb, {
        PLUGIN_VERSION, EV_PEER_JOINED, EV_PEER_LEFT, EV_PACKET, EV_CLOSED, FORCE_RELAY, NO_DIRECT, STUN_FALLBACK,
        POLL_HOST_MS, POLL_CONNECTING_MS, POLL_CONNECTED_MS, HOST_KEEP_MS, HELLO_MS, LONG_POLL_HEADER, LONG_POLL_GAP_MS,
        POLL_RETRY_MS, HAVE_WINDOW, HAVE_MAX, GATHER_MS, SPARE_COUNT, SPARE_ROTATE_MS, SPARE_GRACE_MS, SPARE_TICK_MS,
        SPARE_GATHER_MS, PREWARM_MS, PRECONNECT_IDLE_MS, PRECONNECT_MAX, PRECONNECT_RETRY_MS, SEND_BATCH_MS,
        DISCONNECT_GRACE_MS, DIRECT_ATTEMPT_MS, ICE_CACHE_MS, RELIABLE_MIN_BYTES, RELIABLE_MAX_BUFFERED, CHANNEL,
        RELIABLE_CHANNEL,
    });
})((window.gridlockWeb = window.gridlockWeb || {}));
