// Gridlock online services as a Dilmune (DCS) edge function. Same host list,
// ICE and models routes as ../src/index.js (the Cloudflare Worker), but a DCS
// function has no Durable Objects and no memory shared between requests, so:
//
// - State lives in a private bucket, reached through the S3 API (STATE_BUCKET).
//   Objects under servers/ and rooms/ are short-lived; the bucket's lifecycle
//   rules delete anything left behind after a day.
// - WebRTC signalling uses HTTP mailboxes instead of WebSocket rooms, since two
//   sockets can land in different isolates and never meet.
//
// Host list. Game hosts announce themselves with a heartbeat; clients fetch the list.
//   GET    /servers        -> [{ id, name, addr | room, players, max_players }]
//   POST   /servers        { name, port | room, players, max_players } -> { id, token, ttl, addr? }
//   PUT    /servers/:id    { players }   (Authorization: Bearer <token>)
//   DELETE /servers/:id                  (Authorization: Bearer <token>)
//   A native host's address is the IP the request came from, never a
//   client-supplied one, so a listing can only point at the machine that made it.
//   Browser hosts list a WebRTC room code instead.
//
// ICE servers for WebRTC.
//   GET    /ice            -> { iceServers: [...] }   STUN, plus a TURN relay when one is
//                             configured: your own (TURN_URLS, TURN_USERNAME, TURN_CREDENTIAL)
//                             or Cloudflare's (TURN_KEY_ID, TURN_KEY_API_TOKEN).
//
// WebRTC signalling (client-server topology; web/gridlock_web.js is the client).
//   POST   /room/:code/join?host   -> { id, token }   claim the room (409 if taken)
//   POST   /room/:code/join        -> { id, token }   join it (404 without a host)
//   POST   /room/:code/send        { id, token, signals: [{ receiver, data }] }
//   GET    /room/:code/poll?id&token -> [{ NewPeer } | { PeerLeft } | { Signal: { sender, data } }]
//   POST   /room/:code/leave       { id, token }
//   Polling keeps the host's claim alive; a room whose host stops polling for
//   HOST_TTL_SECONDS closes, and its players' polls get 410.
//   Players are only ever introduced to the host, never to each other.
//
// Game models, for the browser build.
//   GET    /models/<path>  Proxied from MODELS_ORIGIN (a public bucket that only
//                          serves plain HTTP) over HTTPS, with CORS and caching.
//
// Settings (secrets): S3_ENDPOINT, S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY,
// STATE_BUCKET, SIGNAL_SECRET (any long random string), MODELS_ORIGIN, and the
// optional TURN_* ones above.

const TTL_SECONDS = 90;
const HOST_TTL_SECONDS = 60;
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

// ---------------------------------------------------------------- host list

async function servers(request, env, ctx, id, extra) {
  if (extra !== undefined) return json({ error: "not found" }, 404);
  const store = new Store(env);

  if (!id && request.method === "GET") {
    const listings = await liveListings(store, ctx);
    return json(listings.map(({ id, listing }) => ({ id, ...listing.entry })));
  }
  if (!id && request.method === "POST") return createListing(request, store, ctx);
  if (!id || !PEER_ID.test(id)) return json({ error: "not found" }, 404);
  if (request.method === "PUT") return heartbeat(request, store, id);
  if (request.method === "DELETE") return removeListing(request, store, id);
  return json({ error: "method not allowed" }, 405);
}

// Every unexpired listing; expired ones are deleted on the way.
async function liveListings(store, ctx) {
  const keys = await store.list("servers/");
  const now = Date.now();
  const all = await Promise.all(keys.map(async (key) => ({ key, listing: await store.get(key) })));
  const live = [];
  for (const { key, listing } of all) {
    if (!listing) continue;
    if (listing.expires <= now) ctx.waitUntil(store.delete(key).catch(() => {}));
    else live.push({ id: key.slice("servers/".length, -".json".length), listing });
  }
  return live;
}

async function createListing(request, store, ctx) {
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
  const live = await liveListings(store, ctx);
  const fromIp = live.filter(({ listing }) => listing.ip === ip).length;
  if (fromIp >= MAX_PER_IP || live.length >= MAX_LISTINGS) {
    return json({ error: "too many listings" }, 429);
  }

  const id = crypto.randomUUID();
  const token = crypto.randomUUID();
  const entry = { name, players: Math.min(players, maxPlayers), max_players: maxPlayers };
  if (room) entry.room = room;
  else entry.addr = ip.includes(":") ? `[${ip}]:${port}` : `${ip}:${port}`;

  await store.put(`servers/${id}.json`, {
    tokenHash: await sha256Hex(token),
    ip,
    expires: Date.now() + TTL_SECONDS * 1000,
    entry,
  });
  // Echo the address back so a host can tell whether it is reachable as listed.
  return json({ id, token, ttl: TTL_SECONDS, ...(entry.addr ? { addr: entry.addr } : {}) }, 201);
}

async function heartbeat(request, store, id) {
  const listing = await authorizedListing(request, store, id);
  if (!listing) return json({ error: "unknown listing" }, 404);
  const players = int((await readJson(request))?.players, 0, 255);
  if (players !== null) listing.entry.players = Math.min(players, listing.entry.max_players);
  listing.expires = Date.now() + TTL_SECONDS * 1000;
  await store.put(`servers/${id}.json`, listing);
  return new Response(null, { status: 204 });
}

async function removeListing(request, store, id) {
  if (!(await authorizedListing(request, store, id))) return json({ error: "unknown listing" }, 404);
  await store.delete(`servers/${id}.json`);
  return new Response(null, { status: 204 });
}

async function authorizedListing(request, store, id) {
  const listing = await store.get(`servers/${id}.json`);
  if (!listing || listing.expires <= Date.now()) return null;
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

// Room layout in the state bucket:
//   rooms/<code>/host.json                  { id, expires }
//   rooms/<code>/box/<peer>/<time>-<n>.json [message, ...] waiting for <peer>
// A peer's token is an HMAC of the room and its id, so nothing per player is stored.
async function room(request, env, ctx, code, action) {
  const store = new Store(env);
  const url = new URL(request.url);
  const hostKey = `rooms/${code}/host.json`;
  const now = Date.now();
  const liveHost = async () => {
    const host = await store.get(hostKey);
    return host && host.expires > now ? host : null;
  };

  if (action === "join" && request.method === "POST") {
    const host = await liveHost();
    const id = crypto.randomUUID();
    const token = await peerToken(env, code, id);
    if (url.searchParams.has("host")) {
      if (host) return json({ error: "room already has a host" }, 409);
      await store.put(hostKey, { id, expires: now + HOST_TTL_SECONDS * 1000 });
      return json({ id, token });
    }
    if (!host) return json({ error: "no host in this room" }, 404);
    // The host is the offerer for every player, so only it hears about newcomers.
    await deliver(store, code, host.id, [{ NewPeer: id }]);
    return json({ id, token });
  }

  if (action === "poll" && request.method === "GET") {
    const id = url.searchParams.get("id") || "";
    if (!(await validPeer(env, code, id, url.searchParams.get("token")))) return json({ error: "bad token" }, 403);
    const [host, keys] = await Promise.all([liveHost(), store.list(`rooms/${code}/box/${id}/`)]);
    if (!host) return json({ error: "room closed" }, 410);
    // The host's polls keep its claim on the room alive.
    if (host.id === id && host.expires - now < (HOST_TTL_SECONDS * 1000) / 2) {
      ctx.waitUntil(store.put(hostKey, { id, expires: now + HOST_TTL_SECONDS * 1000 }).catch(() => {}));
    }
    const batches = await Promise.all(keys.map((key) => store.get(key)));
    ctx.waitUntil(Promise.all(keys.map((key) => store.delete(key))).catch(() => {}));
    return json(batches.flatMap((b) => (Array.isArray(b) ? b : [])));
  }

  if (action === "send" && request.method === "POST") {
    const body = await readJson(request);
    const id = typeof body?.id === "string" ? body.id : "";
    if (!(await validPeer(env, code, id, body?.token))) return json({ error: "bad token" }, 403);
    const signals = Array.isArray(body?.signals) ? body.signals.slice(0, MAX_SIGNALS) : [];
    const host = await liveHost();
    if (!host) return json({ error: "room closed" }, 410);
    const byReceiver = new Map();
    for (const s of signals) {
      if (typeof s?.receiver !== "string" || !PEER_ID.test(s.receiver)) continue;
      // Signals only flow between the host and a player.
      if ((id === host.id) === (s.receiver === host.id)) continue;
      if (!byReceiver.has(s.receiver)) byReceiver.set(s.receiver, []);
      byReceiver.get(s.receiver).push({ Signal: { sender: id, data: s.data } });
    }
    await Promise.all([...byReceiver].map(([receiver, messages]) => deliver(store, code, receiver, messages)));
    return new Response(null, { status: 204 });
  }

  if (action === "leave" && request.method === "POST") {
    const body = await readJson(request);
    const id = typeof body?.id === "string" ? body.id : "";
    if (!(await validPeer(env, code, id, body?.token))) return json({ error: "bad token" }, 403);
    const host = await liveHost();
    if (!host) return new Response(null, { status: 204 });
    // A leaving host closes the room; players find out from their next poll.
    if (host.id === id) await store.delete(hostKey);
    else await deliver(store, code, host.id, [{ PeerLeft: id }]);
    return new Response(null, { status: 204 });
  }

  return json({ error: "not found" }, 404);
}

async function deliver(store, code, receiver, messages) {
  // Zero-padded time first, so listing a mailbox returns its messages in order.
  const key = `rooms/${code}/box/${receiver}/${String(Date.now()).padStart(15, "0")}-${crypto.randomUUID()}.json`;
  await store.put(key, messages);
}

async function peerToken(env, code, id) {
  const key = await crypto.subtle.importKey("raw", utf8(env.SIGNAL_SECRET), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return hex(await crypto.subtle.sign("HMAC", key, utf8(`${code}:${id}`)));
}

async function validPeer(env, code, id, token) {
  if (!PEER_ID.test(id) || typeof token !== "string") return false;
  return token === (await peerToken(env, code, id));
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

  // Keys under a prefix, in lexical order (at most 1000, plenty here).
  async list(prefix) {
    const resp = await this.send("GET", "", { "list-type": "2", prefix });
    if (!resp.ok) throw new Error(`S3 LIST ${prefix}: ${resp.status}`);
    const xml = await resp.text();
    return [...xml.matchAll(/<Key>([^<]*)<\/Key>/g)].map((m) => unescapeXml(m[1]));
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

function unescapeXml(text) {
  return text.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&amp;/g, "&");
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
