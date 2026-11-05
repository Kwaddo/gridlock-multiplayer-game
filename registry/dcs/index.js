// Gridlock online services as a Dilmune (DCS) edge function. Same host list,
// ICE and models routes as ../src/index.js (the Cloudflare Worker), but a DCS
// function has no Durable Objects and no memory shared between requests, so:
//
// - All state is one object, state.json, in a private bucket reached through
//   the S3 API (STATE_BUCKET): the host list, and each open room with the
//   signalling messages waiting in it. Nothing else is ever created there.
//   Reading is one GET; polls only read. Writes happen on joins, offers and
//   answers, leaves, heartbeats and a host's claim renewal.
// - The bucket has no conditional writes, so two writes at once can overwrite
//   each other. Every change is checked after a moment and applied again on
//   top of the other one if it went missing (commit). Few writes happen at
//   all (one complete offer and one complete answer per connection, in turn),
//   and the clients heal what still slips through: a player not connected
//   yet re-announces itself every few seconds, and a host whose listing went
//   missing gets 404 on its next heartbeat and registers again.
// - WebRTC signalling uses these mailboxes instead of WebSocket rooms, since
//   two sockets can land in different isolates and never meet. Players stop
//   polling once connected; the host polls every few seconds when idle.
//
// Host list. Game hosts announce themselves with a heartbeat; clients fetch the list.
//   GET    /servers        -> [{ id, name, addr | room, players, max_players }]
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
//   POST   /room/:code/join        -> { id, token }   join it (404 without a host)
//   POST   /room/:code/hello       { id, token }      a player re-announcing itself
//   POST   /room/:code/send        { id, token, host?, signals: [{ receiver, data }] }
//   GET    /room/:code/poll?id&token[&host][&keep]
//                                  -> [{ id, NewPeer | PeerLeft | Signal: { sender, data } }]
//   POST   /room/:code/leave       { id, token, host? }
//   A poll returns every message for the caller from the last MESSAGE_TTL_MS,
//   each with an id; clients skip the ones they have seen. The host's polls
//   carry ?host (its token is a host token) and, every so often, ?keep to
//   renew its claim; a room not renewed for HOST_TTL_SECONDS closes, and a
//   joining player's polls get 410.
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

// { listings: { id: { tokenHash, ip, expires, entry } },
//   rooms: { code: { host, expires, messages: [{ id, to, at, body }] } } }
// with anything expired already dropped.
async function loadState(store) {
  const now = Date.now();
  const state = (await store.get(STATE_KEY)) || {};
  const listings = Object.fromEntries(Object.entries(state.listings || {}).filter(([, l]) => l?.expires > now));
  const rooms = {};
  for (const [code, r] of Object.entries(state.rooms || {})) {
    if (!(r?.expires > now)) continue;
    rooms[code] = { ...r, messages: (r.messages || []).filter((m) => now - m.at < MESSAGE_TTL_MS) };
  }
  return { listings, rooms };
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
    return json(Object.entries(listings).map(([id, listing]) => ({ id, ...listing.entry })));
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

// A room in state.json: { host: <host's peer id>, expires, messages }. A
// peer's token is an HMAC of the room, its id and its role, so nothing per
// player is stored, and the host is known without reading anything.
async function room(request, env, ctx, code, action) {
  const store = new Store(env);
  const url = new URL(request.url);
  const now = Date.now();
  // A message is made once, so re-applying it after a lost write adds no copy.
  const message = (to, body) => ({ id: crypto.randomUUID(), to, at: now, body });
  const deliver = (messages) =>
    commit(
      store,
      (s) => {
        const r = s.rooms[code];
        if (r) for (const m of messages) if (!r.messages.some((x) => x.id === m.id)) r.messages.push(m);
      },
      (s) => !s.rooms[code] || messages.every((m) => s.rooms[code].messages.some((x) => x.id === m.id)),
    );

  if (action === "join" && request.method === "POST") {
    const asHost = url.searchParams.has("host");
    const state = await loadState(store);
    const room = state.rooms[code];
    const id = crypto.randomUUID();
    const token = await peerToken(env, code, id, asHost);
    if (asHost) {
      if (room) return json({ error: "room already has a host" }, 409);
      const claim = { host: id, expires: now + HOST_TTL_SECONDS * 1000, messages: [] };
      await commit(store, (s) => (s.rooms[code] ??= claim), (s) => s.rooms[code]?.host === id);
    } else {
      if (!room) return json({ error: "no host in this room" }, 404);
      // The host is the offerer for every player, so only it hears about newcomers.
      await deliver([message(room.host, { NewPeer: id })]);
    }
    return json({ id, token });
  }

  if (action === "poll" && request.method === "GET") {
    const id = url.searchParams.get("id") || "";
    const asHost = url.searchParams.has("host");
    if (!(await validPeer(env, code, id, url.searchParams.get("token"), asHost))) {
      return json({ error: "bad token" }, 403);
    }
    const state = await loadState(store);
    const room = state.rooms[code];
    if (!room || (asHost && room.host !== id)) return json({ error: "room closed" }, 410);
    // The host renews its claim now and then; every other poll only reads.
    if (asHost && url.searchParams.has("keep")) {
      const expires = now + HOST_TTL_SECONDS * 1000;
      const renew = (s) => s.rooms[code]?.host === id && (s.rooms[code].expires = expires);
      ctx.waitUntil(commit(store, renew, (s) => s.rooms[code]?.expires >= expires).catch(() => {}));
    }
    return json(room.messages.filter((m) => m.to === id).map((m) => ({ id: m.id, ...m.body })));
  }

  if (["hello", "send", "leave"].includes(action) && request.method === "POST") {
    const body = await readJson(request);
    const id = typeof body?.id === "string" ? body.id : "";
    const asHost = Boolean(body?.host);
    if (!(await validPeer(env, code, id, body?.token, asHost))) return json({ error: "bad token" }, 403);
    const room = (await loadState(store)).rooms[code];
    if (!room) return action === "leave" ? new Response(null, { status: 204 }) : json({ error: "room closed" }, 410);

    if (action === "hello" && !asHost) {
      await deliver([message(room.host, { NewPeer: id })]);
    } else if (action === "send") {
      // Signals only flow between the host and a player: the host addresses
      // players; a player's signals always go to the host.
      const signals = Array.isArray(body?.signals) ? body.signals.slice(0, MAX_SIGNALS) : [];
      const messages = [];
      for (const s of signals) {
        const receiver = asHost ? s?.receiver : room.host;
        if (typeof receiver !== "string" || !PEER_ID.test(receiver) || receiver === id) continue;
        messages.push(message(receiver, { Signal: { sender: id, data: s.data } }));
      }
      if (messages.length) await deliver(messages);
    } else if (action === "leave") {
      if (asHost && room.host === id) {
        // A leaving host closes the room, and takes its listing off the host list.
        const close = (s) => {
          if (s.rooms[code]?.host === id) delete s.rooms[code];
          for (const [lid, l] of Object.entries(s.listings)) if (l.entry.room === code) delete s.listings[lid];
        };
        const closed = (s) => s.rooms[code]?.host !== id && !Object.values(s.listings).some((l) => l.entry.room === code);
        await commit(store, close, closed);
      } else if (!asHost) {
        await deliver([message(room.host, { PeerLeft: id })]);
      }
    }
    return new Response(null, { status: 204 });
  }

  return json({ error: "not found" }, 404);
}

async function peerToken(env, code, id, asHost) {
  const key = await crypto.subtle.importKey("raw", utf8(env.SIGNAL_SECRET), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return hex(await crypto.subtle.sign("HMAC", key, utf8(`${code}:${id}:${asHost ? "host" : "player"}`)));
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
  return new Response(response.body, { status: response.status, headers });
}
