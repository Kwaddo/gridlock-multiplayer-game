// Gridlock online services, all on one Cloudflare Worker:
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
// WebRTC signalling (matchbox protocol, client-server topology).
//   GET    /room/:code?host   WebSocket for the host (one per room)
//   GET    /room/:code        WebSocket for a player joining that host
//   Players are only ever introduced to the host, never to each other.

const TTL_SECONDS = 90;
const MAX_LISTINGS = 500;
const MAX_PER_IP = 4;
const ROOM_CODE = /^[A-Za-z0-9_-]{4,32}$/;
const TURN_CREDENTIAL_TTL = 86400;
const STUN_ONLY = { iceServers: [{ urls: ["stun:stun.cloudflare.com:3478", "stun:stun.l.google.com:19302"] }] };

export default {
  async fetch(request, env) {
    if (request.method === "OPTIONS") return cors(new Response(null, { status: 204 }));
    const [, root, arg] = new URL(request.url).pathname.split("/");

    if (root === "room" && arg) {
      if (!ROOM_CODE.test(arg)) return cors(json({ error: "bad room code" }, 400));
      // WebSocket responses must pass through untouched.
      return env.ROOMS.get(env.ROOMS.idFromName(arg)).fetch(request);
    }
    if (root === "ice" && request.method === "GET") return cors(json(await iceServers(env)));
    if (root === "servers") {
      return cors(await env.REGISTRY.get(env.REGISTRY.idFromName("global")).fetch(request));
    }
    return cors(json({ error: "not found" }, 404));
  },
};

// ---------------------------------------------------------------- host list

export class Registry {
  constructor() {
    this.listings = new Map(); // id -> { token, ip, expires, entry }
  }

  async fetch(request) {
    this.prune();
    const [, , id, extra] = new URL(request.url).pathname.split("/");
    if (extra !== undefined) return json({ error: "not found" }, 404);

    if (!id && request.method === "GET") {
      return json([...this.listings.entries()].map(([id, l]) => ({ id, ...l.entry })));
    }
    if (!id && request.method === "POST") return this.create(request);
    if (id && request.method === "PUT") return this.heartbeat(request, id);
    if (id && request.method === "DELETE") return this.remove(request, id);
    return json({ error: "method not allowed" }, 405);
  }

  async create(request) {
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
    const fromIp = [...this.listings.values()].filter((l) => l.ip === ip).length;
    if (fromIp >= MAX_PER_IP || this.listings.size >= MAX_LISTINGS) {
      return json({ error: "too many listings" }, 429);
    }

    const id = crypto.randomUUID();
    const token = crypto.randomUUID();
    const entry = { name, players: Math.min(players, maxPlayers), max_players: maxPlayers };
    if (room) entry.room = room;
    else entry.addr = ip.includes(":") ? `[${ip}]:${port}` : `${ip}:${port}`;

    this.listings.set(id, { token, ip, expires: Date.now() + TTL_SECONDS * 1000, entry });
    // Echo the address back so a host can tell whether it is reachable as listed.
    return json({ id, token, ttl: TTL_SECONDS, ...(entry.addr ? { addr: entry.addr } : {}) }, 201);
  }

  async heartbeat(request, id) {
    const listing = this.authorized(request, id);
    if (!listing) return json({ error: "unknown listing" }, 404);
    const players = int((await readJson(request))?.players, 0, 255);
    if (players !== null) listing.entry.players = Math.min(players, listing.entry.max_players);
    listing.expires = Date.now() + TTL_SECONDS * 1000;
    return new Response(null, { status: 204 });
  }

  remove(request, id) {
    if (!this.authorized(request, id)) return json({ error: "unknown listing" }, 404);
    this.listings.delete(id);
    return new Response(null, { status: 204 });
  }

  authorized(request, id) {
    const listing = this.listings.get(id);
    const token = (request.headers.get("Authorization") || "").replace(/^Bearer /, "");
    return listing && listing.token === token ? listing : null;
  }

  prune() {
    const now = Date.now();
    for (const [id, l] of this.listings) if (l.expires <= now) this.listings.delete(id);
  }
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

// One Durable Object per room. Uses the hibernation API, so idle rooms cost
// nothing; each socket's peer id and role live in its attachment.
export class Room {
  constructor(ctx) {
    this.ctx = ctx;
  }

  peers() {
    return this.ctx.getWebSockets().map((ws) => ({ ws, ...ws.deserializeAttachment() }));
  }

  async fetch(request) {
    if (request.headers.get("Upgrade") !== "websocket") return json({ error: "expected websocket" }, 426);
    const wantsHost = new URL(request.url).searchParams.has("host");
    const host = this.peers().find((p) => p.host);
    if (wantsHost && host) return json({ error: "room already has a host" }, 409);
    if (!wantsHost && !host) return json({ error: "no host in this room" }, 404);

    const [client, server] = Object.values(new WebSocketPair());
    const id = crypto.randomUUID();
    this.ctx.acceptWebSocket(server);
    server.serializeAttachment({ id, host: wantsHost });
    send(server, { IdAssigned: id });
    // The host is the offerer for every player, so only it hears about newcomers.
    if (!wantsHost) send(host.ws, { NewPeer: id });
    return new Response(null, { status: 101, webSocket: client });
  }

  webSocketMessage(ws, message) {
    const me = ws.deserializeAttachment();
    let msg;
    try {
      msg = JSON.parse(message);
    } catch {
      return;
    }
    if (msg === "KeepAlive" || !msg?.Signal) return;
    const { receiver, data } = msg.Signal;
    const target = this.peers().find((p) => p.id === receiver);
    // Signals only flow between the host and a player.
    if (!target || target.host === me.host) return;
    send(target.ws, { Signal: { sender: me.id, data } });
  }

  webSocketClose(ws) {
    this.leave(ws);
  }

  webSocketError(ws) {
    this.leave(ws);
  }

  leave(ws) {
    const me = ws.deserializeAttachment();
    const others = this.peers().filter((p) => p.id !== me.id);
    if (me.host) {
      for (const p of others) {
        send(p.ws, { PeerLeft: me.id });
        p.ws.close(1000, "host left");
      }
    } else {
      const host = others.find((p) => p.host);
      if (host) send(host.ws, { PeerLeft: me.id });
    }
  }
}

function send(ws, value) {
  try {
    ws.send(JSON.stringify(value));
  } catch {
    // socket already closing
  }
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
