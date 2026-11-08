// Gridlock online services as a Dilmune (DCS) edge function. Same host list,
// ICE and models routes as ../src/index.js (the Cloudflare Worker), but a DCS
// function has no Durable Objects and no memory shared between requests, so:
//
// - State lives in a private bucket reached through the S3 API (STATE_BUCKET).
//   The host list is one object, state.json. The bucket has no conditional
//   writes, so two writes at once can overwrite each other: every change to
//   it is checked after a moment and applied again on top of the other one if
//   it went missing (commit). A host whose listing still went missing gets
//   404 on its next heartbeat and registers again.
// - WebRTC signalling never touches state.json, since it is on the path of
//   every join: each room's claim and each message are objects of their own
//   under rooms/ (see "signalling" below), written once and never read back,
//   so nothing can be lost to a write at the same moment and nothing waits.
//   Two sockets can land in different isolates and never meet, so messages
//   go through mailboxes instead of WebSocket rooms; polls are held open
//   until something arrives (long polling). Players stop polling once
//   connected.
// - The function cleans up after itself (see "cleanup" below): a poll
//   deletes the messages its caller already has, leaving deletes the
//   leaver's mailbox (a host's, the whole room), and now and then a sweep
//   deletes whatever a crashed tab left behind.
//
// Host list. Game hosts announce themselves with a heartbeat; clients fetch the list.
//   GET    /servers        -> [{ id, name, addr | room, players, max_players, lan? }]
//   (lan: the listing came from the asker's own network; its address is never shown.)
//   POST   /servers        { name, port | room, players, max_players } -> { id, token, ttl, addr? }
//   PUT    /servers/:id    { players }   (Authorization: Bearer <token>)
//   DELETE /servers/:id                  (Authorization: Bearer <token>)
//   A native host's address is the IP the request came from, never a
//   client-supplied one, so a listing can only point at the machine that made it.
//   Browser hosts list a WebRTC room code instead. A listing lives TTL_SECONDS
//   past its last heartbeat; a browser host leaving its room removes it at once.
//
// ICE servers for WebRTC.
//   GET    /ice            -> { iceServers: [...] }   STUN, plus a TURN relay when one is
//                             configured: your own (TURN_URLS, TURN_USERNAME, TURN_CREDENTIAL)
//                             or Cloudflare's (TURN_KEY_ID, TURN_KEY_API_TOKEN).
//
// WebRTC signalling (client-server topology; web/gridlock_web.js is the client).
//   POST   /room/:code/join?host   -> { id, token }   claim the room (409 if taken)
//   POST   /room/:code/join        { signals? }  -> { id, token, host, via }
//                                  join it (404 without a host); `signals` (the
//                                  player's offer) reach the host with the join
//   POST   /room/:code/hello       { id, token, to?, via?, signals? }  a player re-announcing itself
//   POST   /room/:code/send        { id, token, host?, to?, via?, signals: [{ receiver, data }] }
//   GET    /room/:code/poll?id&token[&host][&keep][&wait][&after=<seq>][&have=<ids>]
//                                  -> [{ id, seq, NewPeer | PeerLeft | Signal: { sender, data } }]
//   POST   /room/:code/leave       { id, token, host?, to?, via? }
//   POST   /room/:code/spare       { id, token, host: true, offers: [{ oid, data }] }
//                                  the host's offers made ahead of any join
//   GET    /room/:code/spare       -> { host, oid, data }   one of them, at random (404: none)
//   A player answers a spare offer and sends the answer (tagged with its
//   `oid`) with its join, so a join is one signalling trip; the host
//   republishes its spares as they are taken and every 30 s or so.
//   A poll returns the caller's messages from the last MESSAGE_TTL_MS, newer
//   than `after` (the newest seq it has) and not among `have` (ids it has
//   from around then); with `wait` it is held until there is one, for up to
//   LONG_POLL_MS, and the reply carries LONG_POLL_HEADER. The host's polls
//   carry ?host (its token is a host token) and, every so often, ?keep to
//   renew its claim; a room not renewed for HOST_TTL_SECONDS closes, and a
//   joining player's polls get 410. (`to` and `via` from a join are accepted
//   for older pages; a player's messages always go to the room's host.)
//   Players are only ever introduced to the host, never to each other.
//
// Game models, for the browser build.
//   GET    /models/<path>  Proxied from MODELS_ORIGIN (a public bucket that only
//                          serves plain HTTP) over HTTPS, with CORS and caching.
//
// Settings (secrets): S3_ENDPOINT, S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY,
// STATE_BUCKET, SIGNAL_SECRET (any long random string), MODELS_ORIGIN, and the
// optional TURN_* ones above.

const TTL_SECONDS = 60;
const HOST_TTL_SECONDS = 120;
const MESSAGE_TTL_MS = 60000;
// Long polling (see room): how long a poll is held, how often it looks for
// new messages, and how often it checks that the room is still open. A
// player's mailbox, or a host's that had a message in the last
// LONG_POLL_ACTIVE_MS, is in the middle of a handshake and is looked at more
// often; an idle host's (waiting for someone to join) less, since each look
// is an S3 request.
const LONG_POLL_MS = 6000; // the function itself times out at 10 s
const LONG_POLL_ACTIVE_MS = 10000;
// (Each look itself takes an S3 round trip, ~150 ms from the edge, on top.)
const LONG_POLL_CHECK_MS = 25;
const LONG_POLL_IDLE_CHECK_MS = 50;
const LONG_POLL_CLAIM_CHECK_MS = 2000;
const LONG_POLL_HEADER = "X-Gridlock-Long-Poll";
// How far apart two isolates' clocks may be, for a poll's `after`.
const CLOCK_SKEW_MS = 2000;
// How long a host claiming a room waits before checking its claim stuck.
const CLAIM_SETTLE_MS = 150;
// Sweeps (see sweep): at most this often per isolate, and only from requests
// of a hosting player; leftovers are deleted once this much older than they
// could still be used.
const SWEEP_EVERY_MS = 10 * 60 * 1000;
const SWEEP_MARGIN_MS = 60 * 1000;
const SWEEP_MAX_PAGES = 20;
const DELETE_PARALLEL = 16;
// A host rewrites its spares every 30 s; ones older than this are gone.
const SPARE_TTL_MS = 75000;
const SPARE_MAX_OFFERS = 4;
const SPARE_MAX_BYTES = 64 * 1024;
const SPARE_ID = /^[0-9a-f]{6,32}$/;
const STATE_KEY = "state.json";
const MAX_LISTINGS = 500;
const MAX_PER_IP = 4;
const MAX_SIGNALS = 64;
const ROOM_CODE = /^[A-Za-z0-9_-]{4,32}$/;
const PEER_ID = /^[0-9a-f-]{36}$/;
const TURN_CREDENTIAL_TTL = 86400;
const STUN_ONLY = { iceServers: [{ urls: ["stun:stun.cloudflare.com:3478", "stun:stun.l.google.com:19302"] }] };

export default {
  async fetch(request, env, ctx) {
    if (request.method === "OPTIONS") return cors(new Response(null, { status: 204 }));
    const [, root, arg, action] = new URL(request.url).pathname.split("/");
    try {
      if (root === "room" && arg && action) {
        if (!ROOM_CODE.test(arg)) return cors(json({ error: "bad room code" }, 400));
        return cors(await room(request, env, ctx, arg, action));
      }
      if (root === "ice" && request.method === "GET") return cors(json(await iceServers(env)));
      if (root === "models" && request.method === "GET" && env.MODELS_ORIGIN) return cors(await model(request, env));
      if (root === "servers") return cors(await servers(request, env, ctx, arg, action));
      return cors(json({ error: "not found" }, 404));
    } catch (e) {
      console.error("gridlock-registry:", e && e.stack ? e.stack : e);
      return cors(json({ error: "storage unavailable" }, 503));
    }
  },
};

// ---------------------------------------------------------------- state

// { listings: { id: { tokenHash, ip, expires, entry } } }, with anything
// expired already dropped. (Rooms live in objects of their own: see room.)
async function loadState(store) {
  const now = Date.now();
  const state = (await store.get(STATE_KEY)) || {};
  const listings = Object.fromEntries(Object.entries(state.listings || {}).filter(([, l]) => l?.expires > now));
  return { listings };
}

// Apply a change to state.json and make sure it stuck: a write made at the
// same moment can overwrite ours, so read back after a moment and apply the
// change again on top of what is there if it is missing. `apply` must be
// safe to repeat; `applied` tells whether a state has the change.
async function commit(store, apply, applied) {
  for (let attempt = 0; attempt < 4; attempt++) {
    const state = await loadState(store);
    if (attempt > 0 && applied(state)) return;
    apply(state);
    await store.put(STATE_KEY, state);
    await new Promise((r) => setTimeout(r, 100 + Math.random() * 200));
    if (applied(await loadState(store))) return;
  }
}

// ---------------------------------------------------------------- host list

async function servers(request, env, ctx, id, extra) {
  if (extra !== undefined) return json({ error: "not found" }, 404);
  const store = new Store(env);

  if (!id && request.method === "GET") {
    const { listings } = await loadState(store);
    const ip = request.headers.get("CF-Connecting-IP");
    return json(
      Object.entries(listings).map(([id, listing]) => ({
        id,
        ...listing.entry,
        ...(sameNetwork(listing.ip, ip) ? { lan: true } : {}),
      })),
    );
  }
  if (!id && request.method === "POST") return createListing(request, store);
  if (!id || !PEER_ID.test(id)) return json({ error: "not found" }, 404);
  if (request.method === "PUT") return heartbeat(request, store, id);
  if (request.method === "DELETE") return removeListing(request, store, id);
  return json({ error: "method not allowed" }, 405);
}

async function createListing(request, store) {
  const ip = request.headers.get("CF-Connecting-IP");
  const body = await readJson(request);
  const name = cleanName(body?.name);
  const maxPlayers = int(body?.max_players, 1, 255);
  const players = int(body?.players, 0, 255);
  const port = int(body?.port, 1, 65535);
  const room = typeof body?.room === "string" && ROOM_CODE.test(body.room) ? body.room : null;
  if (!ip || !name || maxPlayers === null || players === null || (port === null) === (room === null)) {
    return json({ error: "invalid listing" }, 400);
  }
  const entry = { name, players: Math.min(players, maxPlayers), max_players: maxPlayers };
  if (room) entry.room = room;
  else entry.addr = ip.includes(":") ? `[${ip}]:${port}` : `${ip}:${port}`;

  // One listing per game: a host registering again (its old listing lost a
  // heartbeat) replaces what it had, so the game is never listed twice.
  // Only the machine that listed a room may replace that listing: anyone else
  // could otherwise take over (or rename) another host's game.
  const { listings } = await loadState(store);
  for (const [other, l] of Object.entries(listings)) {
    if (room ? l.entry.room !== room : l.entry.addr !== entry.addr) continue;
    if (l.ip !== ip) return json({ error: "room already listed" }, 409);
    delete listings[other];
  }
  const fromIp = Object.values(listings).filter((l) => l.ip === ip).length;
  if (fromIp >= MAX_PER_IP || Object.keys(listings).length >= MAX_LISTINGS) {
    return json({ error: "too many listings" }, 429);
  }

  const id = crypto.randomUUID();
  const token = crypto.randomUUID();
  const listing = { tokenHash: await sha256Hex(token), ip, expires: Date.now() + TTL_SECONDS * 1000, entry };
  const replaced = (l) => (room ? l.entry.room === room : l.entry.addr === entry.addr) && l.ip === ip;
  await commit(
    store,
    (s) => {
      for (const [other, l] of Object.entries(s.listings)) if (replaced(l)) delete s.listings[other];
      s.listings[id] = listing;
    },
    (s) => Boolean(s.listings[id]),
  );
  // Echo the address back so a host can tell whether it is reachable as listed.
  return json({ id, token, ttl: TTL_SECONDS, ...(entry.addr ? { addr: entry.addr } : {}) }, 201);
}

async function heartbeat(request, store, id) {
  const { listings } = await loadState(store);
  if (!(await authorized(request, listings, id))) return json({ error: "unknown listing" }, 404);
  const players = int((await readJson(request))?.players, 0, 255);
  const expires = Date.now() + TTL_SECONDS * 1000;
  await commit(
    store,
    (s) => {
      const l = (s.listings[id] ??= listings[id]);
      if (players !== null) l.entry.players = Math.min(players, l.entry.max_players);
      l.expires = expires;
    },
    (s) => s.listings[id]?.expires >= expires,
  );
  return new Response(null, { status: 204 });
}

async function removeListing(request, store, id) {
  if (!(await authorized(request, (await loadState(store)).listings, id))) return json({ error: "unknown listing" }, 404);
  await commit(store, (s) => delete s.listings[id], (s) => !s.listings[id]);
  return new Response(null, { status: 204 });
}

async function authorized(request, listings, id) {
  const listing = listings[id];
  if (!listing) return null;
  const token = (request.headers.get("Authorization") || "").replace(/^Bearer /, "");
  return listing.tokenHash === (await sha256Hex(token)) ? listing : null;
}

// ---------------------------------------------------------------- ICE / TURN

async function iceServers(env) {
  // A TURN server of your own (e.g. coturn), configured as plain settings.
  if (env.TURN_URLS) {
    const turn = {
      urls: env.TURN_URLS.split(",").map((u) => u.trim()).filter(Boolean),
      username: env.TURN_USERNAME,
      credential: env.TURN_CREDENTIAL,
    };
    return { iceServers: [...STUN_ONLY.iceServers, turn] };
  }
  // Cloudflare's TURN service, with short-lived credentials minted per request.
  if (!env.TURN_KEY_ID || !env.TURN_KEY_API_TOKEN) return STUN_ONLY;
  const resp = await fetch(
    `https://rtc.live.cloudflare.com/v1/turn/keys/${env.TURN_KEY_ID}/credentials/generate-ice-servers`,
    {
      method: "POST",
      headers: { Authorization: `Bearer ${env.TURN_KEY_API_TOKEN}`, "Content-Type": "application/json" },
      body: JSON.stringify({ ttl: TURN_CREDENTIAL_TTL }),
    },
  );
  if (!resp.ok) return STUN_ONLY;
  const { iceServers } = await resp.json();
  // Port 53 URLs tend to time out in browsers; drop them.
  return {
    iceServers: iceServers.map((s) => ({ ...s, urls: [].concat(s.urls).filter((u) => !/:53\b/.test(u)) })),
  };
}

// ---------------------------------------------------------------- signalling

// Signalling stays out of state.json: a room's claim is its own object,
// rooms/<code>.json = { host, expires }, written only by its host, and every
// message is an object of its own in its receiver's mailbox, as
// <mailbox><at>-<mid>.json = { at, sender, bodies }, written once and never
// changed. Nothing is read, modified and written back, so no write can
// overwrite another and nothing has to be checked after a moment. A poll
// lists the caller's mailbox and reads only what is new.
//
// Mailboxes: the room's host has rooms/<code>/host/ (whoever hosts it, so a
// player can write there without reading the claim first), each player
// rooms/<code>/to/<player>/. Every S3 request costs ~150 ms from the edge,
// so the path of a join is kept to as few, and as parallel, requests as can be.
//
// A peer's token is an HMAC of the room, its id and its role, so nothing per
// player is stored. A joining player also gets `via`, an HMAC naming the host
// it joined (older pages send it back with their signals).
async function room(request, env, ctx, code, action) {
  const store = new Store(env);
  const url = new URL(request.url);
  const now = Date.now();
  const claimKey = `rooms/${code}.json`;
  const loadClaim = async () => {
    const claim = await store.get(claimKey);
    return claim?.expires > Date.now() ? claim : null;
  };

  if (action === "join" && request.method === "POST") {
    const asHost = url.searchParams.has("host");
    const id = crypto.randomUUID();
    const token = await peerToken(env, code, id, asHost);
    if (asHost) {
      if (await loadClaim()) return json({ error: "room already has a host" }, 409);
      await store.put(claimKey, { host: id, expires: now + HOST_TTL_SECONDS * 1000 });
      // Two hosts claiming one code at the same moment: the later write wins,
      // and the other is told the room is taken. Codes are random, so this
      // is rare, and it is only on the host's own start, never on a join.
      await delay(CLAIM_SETTLE_MS);
      if ((await loadClaim())?.host !== id) return json({ error: "room already has a host" }, 409);
      maybeSweep(store, ctx);
      return json({ id, token });
    }
    // A player sends its offer with the join (see web/gridlock_web.js), so the
    // host hears of it and gets the offer in one message. The message goes
    // out while the claim is read: a room without a host takes it back.
    const body = await readJson(request);
    const bodies = [{ NewPeer: id }, ...signalBodies(id, body?.signals)];
    const [claim, key] = await Promise.all([loadClaim(), deliver(store, hostMailbox(code), id, bodies)]);
    if (!claim) {
      ctx.waitUntil(store.delete(key).catch(() => {}));
      return json({ error: "no host in this room" }, 404);
    }
    return json({ id, token, host: claim.host, via: await hostVia(env, code, id, claim.host) });
  }

  if (action === "poll" && request.method === "GET") {
    const id = url.searchParams.get("id") || "";
    const asHost = url.searchParams.has("host");
    if (!(await validPeer(env, code, id, url.searchParams.get("token"), asHost))) {
      return json({ error: "bad token" }, 403);
    }
    const after = Number(url.searchParams.get("after")) || 0;
    // A page that sends no `have` (from before it) gets only what is newer
    // than `after`, or it would be handed the same messages on every poll.
    const have = url.searchParams.has("have")
      ? new Set(url.searchParams.get("have").split(",").filter(Boolean))
      : null;
    const wait = url.searchParams.has("wait");
    const open = (claim) => claim && (!asHost || claim.host === id);

    // Messages the caller has are deleted once (not on every look of a held poll).
    const deleting = new Set();
    const box = asHost ? hostMailbox(code) : mailbox(code, id);
    const look = () => unseen(store, ctx, box, after, have, deleting);
    let [claim, messages] = await Promise.all([loadClaim(), look()]);
    if (!open(claim)) return json({ error: "room closed" }, 410);
    // The host renews its claim now and then; it is the only one writing it.
    if (asHost && url.searchParams.has("keep")) {
      ctx.waitUntil(store.put(claimKey, { ...claim, expires: now + HOST_TTL_SECONDS * 1000 }).catch(() => {}));
      maybeSweep(store, ctx);
    }
    // Long poll: hold the request until something arrives, looking again
    // every LONG_POLL_CHECK_MS (or at once when a write in this same isolate
    // wakes us), and checking now and then that the room is still open.
    let lastClaimCheck = Date.now();
    // (A player only polls while it connects.)
    const active = !asHost || now - after < LONG_POLL_ACTIVE_MS;
    const check = active ? LONG_POLL_CHECK_MS : LONG_POLL_IDLE_CHECK_MS;
    while (wait && !messages.length && Date.now() - now < LONG_POLL_MS) {
      await nextMessage(box, check);
      // A look that failed (the store has the odd hiccup) is tried again.
      messages = await look().catch(() => []);
      if (!messages.length && Date.now() - lastClaimCheck >= LONG_POLL_CLAIM_CHECK_MS) {
        lastClaimCheck = Date.now();
        if (!open(await loadClaim())) return json({ error: "room closed" }, 410);
      }
    }
    const resp = json(messages);
    resp.headers.set(LONG_POLL_HEADER, "1");
    return resp;
  }

  // The host's spare offers: rooms/<code>/spare.json = { host, at, offers },
  // written only by the host, so a player can fetch one before it joins
  // (web/gridlock_web.js does so when the room is pointed at) and join with
  // its answer. One read, no claim check: a leaving host deletes the file
  // with its room, and a crashed host's file ages out (SPARE_TTL_MS).
  const spareKey = `rooms/${code}/spare.json`;
  if (action === "spare" && request.method === "GET") {
    const spare = await store.get(spareKey).catch(() => null);
    const offers = spare?.at > now - SPARE_TTL_MS && Array.isArray(spare.offers) ? spare.offers : [];
    if (!offers.length) return json({ error: "no spare offer" }, 404);
    const offer = offers[Math.floor(Math.random() * offers.length)];
    const resp = json({ host: spare.host, oid: offer.oid, data: offer.data });
    resp.headers.set("Cache-Control", "no-store");
    return resp;
  }
  if (action === "spare" && request.method === "POST") {
    const body = await readJson(request);
    const id = typeof body?.id === "string" ? body.id : "";
    if (!body?.host || !(await validPeer(env, code, id, body?.token, true))) return json({ error: "bad token" }, 403);
    const offers = (Array.isArray(body?.offers) ? body.offers.slice(0, SPARE_MAX_OFFERS) : [])
      .filter((o) => typeof o?.oid === "string" && SPARE_ID.test(o.oid) && o.data?.sdp?.type === "offer")
      .map((o) => ({ oid: o.oid, data: o.data }));
    if (JSON.stringify(offers).length > SPARE_MAX_BYTES) return json({ error: "too big" }, 413);
    if (offers.length) await store.put(spareKey, { host: id, at: now, offers });
    else await store.delete(spareKey).catch(() => {});
    return new Response(null, { status: 204 });
  }

  if (["hello", "send", "leave"].includes(action) && request.method === "POST") {
    const body = await readJson(request);
    const id = typeof body?.id === "string" ? body.id : "";
    const asHost = Boolean(body?.host);
    if (!(await validPeer(env, code, id, body?.token, asHost))) return json({ error: "bad token" }, 403);

    if (asHost) {
      if (action === "send") {
        // The host addresses players; one message per player.
        const byReceiver = new Map();
        for (const s of Array.isArray(body?.signals) ? body.signals.slice(0, MAX_SIGNALS) : []) {
          const receiver = s?.receiver;
          if (typeof receiver !== "string" || !PEER_ID.test(receiver) || receiver === id) continue;
          if (!byReceiver.has(receiver)) byReceiver.set(receiver, []);
          byReceiver.get(receiver).push({ Signal: { sender: id, data: s.data } });
        }
        await Promise.all([...byReceiver].map(([to, bodies]) => deliver(store, mailbox(code, to), id, bodies)));
      } else if (action === "leave") {
        // A leaving host closes the room, and takes its listing off the host list.
        const claim = await store.get(claimKey);
        if (claim?.host === id) {
          await store.delete(claimKey);
          // Everyone's mailboxes go with the room.
          ctx.waitUntil(deletePrefix(store, `rooms/${code}/`).catch(() => {}));
        }
        await commit(
          store,
          (s) => {
            for (const [lid, l] of Object.entries(s.listings)) if (l.entry.room === code) delete s.listings[lid];
          },
          (s) => !Object.values(s.listings).some((l) => l.entry.room === code),
        );
      }
      return new Response(null, { status: 204 });
    }

    // A player's messages only ever go to the room's host.
    let bodies = [];
    if (action === "hello") bodies = [{ NewPeer: id }, ...signalBodies(id, body?.signals)];
    else if (action === "send") bodies = signalBodies(id, body?.signals);
    else if (action === "leave") bodies = [{ PeerLeft: id }];
    if (bodies.length) await deliver(store, hostMailbox(code), id, bodies);
    // A player leaving needs its mailbox no more.
    if (action === "leave") ctx.waitUntil(deletePrefix(store, mailbox(code, id)).catch(() => {}));
    return new Response(null, { status: 204 });
  }

  return json({ error: "not found" }, 404);
}

function signalBodies(sender, signals) {
  return (Array.isArray(signals) ? signals.slice(0, MAX_SIGNALS) : [])
    .filter((s) => s && typeof s === "object")
    .map((s) => ({ Signal: { sender, data: s.data } }));
}

// Mailbox keys sort by time: a fixed-width ms timestamp, then a random id.
function mailbox(code, player) {
  return `rooms/${code}/to/${player}/`;
}

function hostMailbox(code) {
  return `rooms/${code}/host/`;
}

// Put a message in `box`; returns its key.
async function deliver(store, box, sender, bodies) {
  const at = Date.now();
  const mid = crypto.randomUUID().slice(0, 8);
  const key = `${box}${String(at).padStart(15, "0")}-${mid}.json`;
  await store.put(key, { at, sender, bodies });
  wakeLocal(box);
  return key;
}

// Messages in mailbox `box` its owner does not have yet, oldest first, as
// [{ id, seq, ...body }]: those from a little before `after` on, in case
// clocks of two isolates disagree by a moment, except the ids in `have`.
// Without `have`, only messages newer than `after`.
//
// The whole mailbox is listed (it holds only a handful of messages): those
// in `have` (the caller has them) and expired ones are deleted, in the
// background and once each (`deleting`).
async function unseen(store, ctx, prefix, after, have, deleting) {
  const now = Date.now();
  const from = Math.max(have ? after - CLOCK_SKEW_MS : after + 1, now - MESSAGE_TTL_MS, 0);
  const { items } = await store.list(prefix);
  const fresh = [];
  const done = [];
  for (const { key } of items) {
    const name = key.slice(prefix.length).replace(/\.json$/, "");
    if (!/^\d{15}-[0-9a-f]{8}$/.test(name)) continue;
    const at = Number(name.slice(0, 15));
    const had = have?.has(name.slice(16));
    if (had || at < now - MESSAGE_TTL_MS) {
      if (!deleting.has(key)) done.push(key);
    } else if (at >= from) {
      fresh.push({ key, name });
    }
  }
  if (done.length) {
    for (const key of done) deleting.add(key);
    ctx.waitUntil(deleteKeys(store, done).catch(() => {}));
  }
  const objects = await Promise.all(fresh.map(({ key }) => store.get(key).catch(() => null)));
  const out = [];
  fresh.forEach(({ name }, i) => {
    const object = objects[i];
    if (!object || !Array.isArray(object.bodies)) return;
    const seq = Number(name.slice(0, 15));
    const mid = name.slice(16);
    // One id per message; a message with several bodies numbers them.
    object.bodies.forEach((body, n) => out.push({ id: n ? `${mid}.${n}` : mid, seq, ...body }));
  });
  return out;
}

// ---------------------------------------------------------------- cleanup

// Messages are deleted as soon as their receiver has them (unseen) or leaves
// (a player its mailbox, a host the whole room). What a tab that crashed or
// lost its connection leaves behind goes in a sweep of everything under
// rooms/: messages past MESSAGE_TTL_MS, claims not renewed for
// HOST_TTL_SECONDS (a host renews every 20 s), and spare offers past SPARE_TTL_MS. Sweeps run in the background
// of a hosting player's requests, at most every SWEEP_EVERY_MS per isolate.
let lastSweep = 0;

function maybeSweep(store, ctx) {
  if (Date.now() - lastSweep < SWEEP_EVERY_MS) return;
  lastSweep = Date.now();
  ctx.waitUntil(sweep(store).catch((e) => console.error("gridlock-registry: sweep:", e && e.stack ? e.stack : e)));
}

async function sweep(store) {
  const now = Date.now();
  const stale = [];
  const claimsToCheck = [];
  let token;
  for (let page = 0; page < SWEEP_MAX_PAGES; page++) {
    const { items, next } = await store.list("rooms/", { token, maxKeys: 1000 });
    for (const { key, modified } of items) {
      const message = key.match(/^rooms\/[^/]+\/(?:host|to\/[^/]+)\/(\d{15})-[0-9a-f]{8}\.json$/);
      if (message) {
        if (Number(message[1]) < now - MESSAGE_TTL_MS - SWEEP_MARGIN_MS) stale.push(key);
      } else if (/^rooms\/[^/]+\/spare\.json$/.test(key)) {
        // A host rewrites its spares every 30 s or so.
        if (now - Date.parse(modified || "") > SPARE_TTL_MS + SWEEP_MARGIN_MS) stale.push(key);
      } else if (/^rooms\/[^/]+\.json$/.test(key)) {
        const age = now - Date.parse(modified || "");
        if (age > HOST_TTL_SECONDS * 1000 + SWEEP_MARGIN_MS) stale.push(key);
        else if (Number.isNaN(age)) claimsToCheck.push(key);
      }
    }
    if (!next) break;
    token = next;
  }
  // A store that gives no modification times: read the claim's own expiry.
  for (const key of claimsToCheck) {
    const claim = await store.get(key).catch(() => null);
    if (claim && !(claim.expires > now - SWEEP_MARGIN_MS)) stale.push(key);
  }
  await deleteKeys(store, stale);
}

async function deletePrefix(store, prefix) {
  let token;
  for (let page = 0; page < SWEEP_MAX_PAGES; page++) {
    const { items, next } = await store.list(prefix, { token, maxKeys: 1000 });
    await deleteKeys(store, items.map((i) => i.key));
    if (!next) return;
    token = next;
  }
}

async function deleteKeys(store, keys) {
  for (let i = 0; i < keys.length; i += DELETE_PARALLEL) {
    await Promise.all(keys.slice(i, i + DELETE_PARALLEL).map((key) => store.delete(key).catch(() => {})));
  }
}

// Held polls in this isolate, by mailbox. Requests can land in any isolate,
// so this only helps when sender and receiver happen to share one; otherwise
// the poll sees the message on its next look.
const localWaiters = new Map();

function nextMessage(key, ms) {
  return new Promise((resolve) => {
    const done = () => {
      clearTimeout(timer);
      localWaiters.get(key)?.delete(done);
      resolve();
    };
    const timer = setTimeout(done, ms);
    if (!localWaiters.has(key)) localWaiters.set(key, new Set());
    localWaiters.get(key).add(done);
  });
}

function wakeLocal(box) {
  const waiters = localWaiters.get(box);
  if (waiters) for (const done of [...waiters]) done();
}

function delay(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function peerToken(env, code, id, asHost) {
  return hmacHex(env.SIGNAL_SECRET, `${code}:${id}:${asHost ? "host" : "player"}`);
}

async function hostVia(env, code, id, host) {
  return hmacHex(env.SIGNAL_SECRET, `${code}:${id}>${host}`);
}

async function hmacHex(secret, message) {
  const key = await crypto.subtle.importKey("raw", utf8(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return hex(await crypto.subtle.sign("HMAC", key, utf8(message)));
}

async function validPeer(env, code, id, token, asHost) {
  if (!PEER_ID.test(id) || typeof token !== "string") return false;
  return token === (await peerToken(env, code, id, asHost));
}

// ---------------------------------------------------------------- models

// The page asks for /models/<path>?v=<version>, so a new version is a new URL
// and every cache below can keep a model for a long time.
async function model(request, env) {
  const { pathname, search } = new URL(request.url);
  const upstream = await fetch(env.MODELS_ORIGIN.replace(/\/$/, "") + pathname + search, {
    cf: { cacheEverything: true, cacheTtl: 30 * 86400 },
  });
  if (!upstream.ok) return json({ error: "not found" }, upstream.status === 404 ? 404 : 502);
  const headers = new Headers({
    "Content-Type": upstream.headers.get("Content-Type") || "application/octet-stream",
    "Cache-Control": "public, max-age=2592000, immutable",
  });
  const length = upstream.headers.get("Content-Length");
  if (length) headers.set("Content-Length", length);
  return new Response(upstream.body, { status: 200, headers });
}

// ---------------------------------------------------------------- state bucket

// JSON objects in STATE_BUCKET over the S3 API, signed with AWS Signature V4.
class Store {
  constructor(env) {
    this.env = env;
    this.base = `${env.S3_ENDPOINT.replace(/\/$/, "")}/${env.STATE_BUCKET}`;
  }

  async get(key) {
    const resp = await this.send("GET", `/${key}`);
    if (resp.status === 404) return null;
    if (!resp.ok) throw new Error(`S3 GET ${key}: ${resp.status}`);
    try {
      return await resp.json();
    } catch {
      return null;
    }
  }

  async put(key, value) {
    const resp = await this.send("PUT", `/${key}`, {}, JSON.stringify(value));
    if (!resp.ok) throw new Error(`S3 PUT ${key}: ${resp.status}`);
  }

  async delete(key) {
    const resp = await this.send("DELETE", `/${key}`);
    if (!resp.ok && resp.status !== 404) throw new Error(`S3 DELETE ${key}: ${resp.status}`);
  }

  // One page of the keys under `prefix`, in order (ListObjectsV2), as
  // { items: [{ key, modified }], next }: `next` is the token for the page
  // after, if there is one.
  async list(prefix, { token, maxKeys = 200 } = {}) {
    const query = { "list-type": "2", prefix, "max-keys": String(maxKeys) };
    if (token) query["continuation-token"] = token;
    const resp = await this.send("GET", "/", query);
    if (!resp.ok) throw new Error(`S3 LIST ${prefix}: ${resp.status}`);
    const xml = await resp.text();
    const items = [...xml.matchAll(/<Contents>([\s\S]*?)<\/Contents>/g)].map(([, c]) => ({
      key: xmlText(c.match(/<Key>([^<]*)<\/Key>/)?.[1] ?? ""),
      modified: c.match(/<LastModified>([^<]*)<\/LastModified>/)?.[1],
    }));
    const truncated = /<IsTruncated>\s*true\s*<\/IsTruncated>/.test(xml);
    const next = truncated ? xmlText(xml.match(/<NextContinuationToken>([^<]*)<\/NextContinuationToken>/)?.[1] ?? "") : "";
    return { items: items.filter((i) => i.key), next: next || null };
  }

  async send(method, path, query = {}, body = "") {
    const url = new URL(this.base + path);
    const params = Object.keys(query).sort().map((k) => `${rfc3986(k)}=${rfc3986(query[k])}`).join("&");
    url.search = params;

    const amzDate = new Date().toISOString().replace(/[-:]|\.\d{3}/g, "");
    const date = amzDate.slice(0, 8);
    const payloadHash = await sha256Hex(body);
    const headers = { host: url.host, "x-amz-content-sha256": payloadHash, "x-amz-date": amzDate };
    const signedHeaders = Object.keys(headers).sort().join(";");
    const canonicalHeaders = Object.keys(headers).sort().map((h) => `${h}:${headers[h]}\n`).join("");
    const canonicalRequest = [method, url.pathname, params, canonicalHeaders, signedHeaders, payloadHash].join("\n");
    const scope = `${date}/auto/s3/aws4_request`;
    const toSign = `AWS4-HMAC-SHA256\n${amzDate}\n${scope}\n${await sha256Hex(canonicalRequest)}`;

    let key = utf8(`AWS4${this.env.S3_SECRET_ACCESS_KEY}`);
    for (const part of [date, "auto", "s3", "aws4_request"]) key = await hmac(key, part);
    const signature = hex(await hmac(key, toSign));

    return fetch(url, {
      method,
      body: method === "PUT" ? body : undefined,
      headers: {
        "x-amz-content-sha256": payloadHash,
        "x-amz-date": amzDate,
        ...(method === "PUT" ? { "Content-Type": "application/json" } : {}),
        Authorization: `AWS4-HMAC-SHA256 Credential=${this.env.S3_ACCESS_KEY_ID}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`,
      },
    });
  }
}

function xmlText(text) {
  return text.replace(/&(lt|gt|quot|apos|amp);/g, (_, e) => ({ lt: "<", gt: ">", quot: '"', apos: "'", amp: "&" })[e]);
}

async function hmac(key, message) {
  const k = await crypto.subtle.importKey("raw", key, { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return new Uint8Array(await crypto.subtle.sign("HMAC", k, utf8(message)));
}

async function sha256Hex(text) {
  return hex(await crypto.subtle.digest("SHA-256", utf8(text)));
}

function hex(buffer) {
  return [...new Uint8Array(buffer)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function utf8(text) {
  return new TextEncoder().encode(text);
}

function rfc3986(value) {
  return encodeURIComponent(value).replace(/[!'()*]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());
}

// ---------------------------------------------------------------- helpers

// Whether two client addresses are on the same network, as far as the
// registry can tell: the same public IPv4 address (one NAT), or the same /64
// for IPv6 (one LAN segment). Used to flag games from the asker's network.
function sameNetwork(a, b) {
  const v4 = (ip) => (ip.startsWith("::ffff:") && ip.includes(".") ? ip.slice(7) : ip);
  a = v4(a || "");
  b = v4(b || "");
  if (!a || !b) return false;
  if (!a.includes(":") || !b.includes(":")) return a === b;
  const prefix = (ip) => {
    const [head, tail = ""] = ip.split("%")[0].toLowerCase().split("::");
    const left = head ? head.split(":") : [];
    const right = tail ? tail.split(":") : [];
    const groups = ip.includes("::") ? [...left, ...Array(8 - left.length - right.length).fill("0"), ...right] : left;
    return groups.slice(0, 4).map((g) => parseInt(g, 16)).join(":");
  };
  return prefix(a) === prefix(b);
}

function cleanName(value) {
  if (typeof value !== "string") return null;
  const name = value.replace(/[\u0000-\u001f\u007f]/g, "").trim().slice(0, 32);
  return name || null;
}

function int(value, min, max) {
  return Number.isInteger(value) && value >= min && value <= max ? value : null;
}

async function readJson(request) {
  try {
    return await request.json();
  } catch {
    return null;
  }
}

function json(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

// The browser build of the game calls these from another origin.
function cors(response) {
  const headers = new Headers(response.headers);
  headers.set("Access-Control-Allow-Origin", "*");
  headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
  headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization");
  // The page reads this one to know its polls are held (see room).
  headers.set("Access-Control-Expose-Headers", LONG_POLL_HEADER);
  return new Response(response.body, { status: response.status, headers });
}
