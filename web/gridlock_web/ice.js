// Gridlock's browser plugin, part 3 of 9: ICE servers and the host's spare
// offers, fetched ahead of a join. (See ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const { STUN_FALLBACK, PREWARM_MS, ICE_CACHE_MS } = gridlockWeb;

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

    Object.assign(gridlockWeb, { iceServersFor, prewarm, spareFor, fetchSpare, fetchIceServers });
})((window.gridlockWeb = window.gridlockWeb || {}));
