#!/usr/bin/env node
// Gridlock's online services, run on your own machine for offline and LAN
// play. One process, Node.js 18+ and nothing to install:
//
//   node registry/local/server.mjs [--port 8080] [--host 0.0.0.0] [--dir dist/web]
//
// It serves the browser build (dist/web, from ./build_web.sh) and the same
// API as the live registry (registry/dcs/index.js), kept in memory:
//
//   GET/POST/PUT/DELETE /servers   host list (browser rooms and native UDP hosts)
//   GET  /ice                      no ICE servers: LAN peers connect directly
//   POST /room/:code/{join,hello,send,leave,spare}, GET /room/:code/{poll,spare}
//                                  WebRTC signalling for the browser build
//   GET  /models/<path>            the models, when the build does not bundle them
//
// The page it serves is told to use this server as its registry
// (window.GRIDLOCK_REGISTRY), so anyone on the network who opens the printed
// address can host, browse and join, with no internet connection. Native
// clients can use it too: GRIDLOCK_REGISTRY=http://<this machine>:8080 ./client
//
// The routes, limits and replies match registry/dcs/index.js; only the
// storage differs. Everything is lost when the server stops, which is fine:
// hosts re-list themselves on their next heartbeat (404 -> register again).

import { createServer } from "node:http";
import { createReadStream, existsSync, readFileSync, statSync } from "node:fs";
import { createHmac, randomBytes, randomUUID, createHash } from "node:crypto";
import { networkInterfaces } from "node:os";
import { dirname, extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const TTL_SECONDS = 60;
const HOST_TTL_SECONDS = 120;
const MESSAGE_TTL_MS = 60000;
const MAX_LISTINGS = 500;
// Higher than the live registry's 4: on a LAN many games can come from one
// machine (several tabs, or every player behind the same address).
const MAX_PER_IP = 32;
const MAX_SIGNALS = 64;
const MAX_BODY = 256 * 1024;
const ROOM_CODE = /^[A-Za-z0-9_-]{4,32}$/;
const PEER_ID = /^[0-9a-f-]{36}$/;
// Long polling: a poll with `wait` and nothing new for the caller is held
// this long, answered the moment a message for it arrives. Signalling then
// costs one LAN round trip instead of a poll interval. The reply carries
// LONG_POLL_HEADER so the page knows it may poll again at once (the live
// registry holds polls too, looking for new messages every few ms).
const LONG_POLL_MS = 15000;
const LONG_POLL_HEADER = "X-Gridlock-Long-Poll";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");

// ---------------------------------------------------------------- options

function parseArgs(argv) {
  const opts = {
    port: Number(process.env.PORT) || 8080,
    host: process.env.HOST || "0.0.0.0",
    dir: join(REPO, "dist", "web"),
    models: null,
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) fail(`${arg} needs a value`);
      return argv[++i];
    };
    if (arg === "--port" || arg === "-p") opts.port = Number(value());
    else if (arg === "--host") opts.host = value();
    else if (arg === "--dir") opts.dir = resolve(value());
    else if (arg === "--models") opts.models = resolve(value());
    else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: node registry/local/server.mjs [--port 8080] [--host 0.0.0.0] [--dir dist/web] [--models <dir>]\n\n" +
          "Serves the browser build and a local registry for offline / LAN play.\n" +
          "  --dir     the browser build to serve (default: dist/web, from ./build_web.sh)\n" +
          "  --models  where /models/* is read from when the build does not bundle them\n" +
          "            (default: <dir>/assets/models, then client/assets/models)",
      );
      process.exit(0);
    } else fail(`unknown option ${arg} (try --help)`);
  }
  if (!Number.isInteger(opts.port) || opts.port < 1 || opts.port > 65535) fail("--port must be 1-65535");
  return opts;
}

function fail(message) {
  console.error(`gridlock-local: ${message}`);
  process.exit(1);
}

const opts = parseArgs(process.argv.slice(2));

const SITE = existsSync(join(opts.dir, "index.html")) ? opts.dir : null;
const MODELS_DIR =
  opts.models ?? [join(opts.dir, "assets", "models"), join(REPO, "client", "assets", "models")].find((d) => existsSync(d)) ?? null;

// ---------------------------------------------------------------- state

// listings: id -> { tokenHash, ip, expires, entry }
// rooms:    code -> { host, expires, messages: [{ id, to, at, body }] }
const listings = new Map();
const rooms = new Map();
// Peer tokens are an HMAC of the room, the peer and its role, as on the live
// registry, so nothing per player is stored. A fresh key each run.
const SIGNAL_SECRET = randomBytes(32);

function prune() {
  const now = Date.now();
  for (const [id, l] of listings) if (!(l.expires > now)) listings.delete(id);
  for (const [code, r] of rooms) {
    if (!(r.expires > now)) {
      rooms.delete(code);
      continue;
    }
    r.messages = r.messages.filter((m) => now - m.at < MESSAGE_TTL_MS);
  }
}
setInterval(prune, 5000).unref();

// ---------------------------------------------------------------- host list

function servers(req, body, id, extra) {
  if (extra !== undefined) return json({ error: "not found" }, 404);
  prune();
  if (!id && req.method === "GET") {
    // Everyone reaching this server is on its network, so every game is.
    return json([...listings].map(([id, l]) => ({ id, ...l.entry, lan: true })));
  }
  if (!id && req.method === "POST") return createListing(req, body);
  if (!id || !PEER_ID.test(id)) return json({ error: "not found" }, 404);
  if (req.method === "PUT") return heartbeat(req, body, id);
  if (req.method === "DELETE") return removeListing(req, id);
  return json({ error: "method not allowed" }, 405);
}

function createListing(req, body) {
  const ip = clientIp(req);
  const data = parseJson(body);
  const name = cleanName(data?.name);
  const maxPlayers = int(data?.max_players, 1, 255);
  const players = int(data?.players, 0, 255);
  const port = int(data?.port, 1, 65535);
  const room = typeof data?.room === "string" && ROOM_CODE.test(data.room) ? data.room : null;
  if (!ip || !name || maxPlayers === null || players === null || (port === null) === (room === null)) {
    return json({ error: "invalid listing" }, 400);
  }
  const entry = { name, players: Math.min(players, maxPlayers), max_players: maxPlayers };
  if (room) entry.room = room;
  else entry.addr = ip.includes(":") ? `[${ip}]:${port}` : `${ip}:${port}`;

  // One listing per game; only the machine that listed a room may replace it.
  for (const [other, l] of listings) {
    if (room ? l.entry.room !== room : l.entry.addr !== entry.addr) continue;
    if (l.ip !== ip) return json({ error: "room already listed" }, 409);
    listings.delete(other);
  }
  const fromIp = [...listings.values()].filter((l) => l.ip === ip).length;
  if (fromIp >= MAX_PER_IP || listings.size >= MAX_LISTINGS) return json({ error: "too many listings" }, 429);

  const id = randomUUID();
  const token = randomUUID();
  listings.set(id, { tokenHash: sha256Hex(token), ip, expires: Date.now() + TTL_SECONDS * 1000, entry });
  log(`listed "${name}" (${room ? `room ${room}` : entry.addr})`);
  return json({ id, token, ttl: TTL_SECONDS, ...(entry.addr ? { addr: entry.addr } : {}) }, 201);
}

function heartbeat(req, body, id) {
  const listing = authorized(req, id);
  if (!listing) return json({ error: "unknown listing" }, 404);
  const players = int(parseJson(body)?.players, 0, 255);
  if (players !== null) listing.entry.players = Math.min(players, listing.entry.max_players);
  listing.expires = Date.now() + TTL_SECONDS * 1000;
  return empty();
}

function removeListing(req, id) {
  const listing = authorized(req, id);
  if (!listing) return json({ error: "unknown listing" }, 404);
  listings.delete(id);
  log(`unlisted "${listing.entry.name}"`);
  return empty();
}

function authorized(req, id) {
  const listing = listings.get(id);
  if (!listing) return null;
  const token = (req.headers.authorization || "").replace(/^Bearer /, "");
  return listing.tokenHash === sha256Hex(token) ? listing : null;
}

// ---------------------------------------------------------------- signalling

// Peer id -> callbacks of its held polls.
const waiting = new Map();
let nextSeq = 1;

function wake(id) {
  const callbacks = waiting.get(id);
  if (!callbacks) return;
  waiting.delete(id);
  for (const cb of callbacks) cb();
}

function room(req, res, body, url, code, action) {
  prune();
  const now = Date.now();
  // Several bodies for one receiver land together, before its held poll is
  // answered: a join's NewPeer and offer must reach the host in one reply.
  const deliver = (to, ...bodies) => {
    const r = rooms.get(code);
    if (!r) return;
    for (const msgBody of bodies) r.messages.push({ id: randomUUID(), seq: nextSeq++, to, at: now, body: msgBody });
    wake(to);
  };
  const signalBodies = (sender, data) => signalsOf(data).map((s) => ({ Signal: { sender, data: s.data } }));

  if (action === "join" && req.method === "POST") {
    const asHost = url.searchParams.has("host");
    const r = rooms.get(code);
    const id = randomUUID();
    const token = peerToken(code, id, asHost);
    if (asHost) {
      if (r) return json({ error: "room already has a host" }, 409);
      rooms.set(code, { host: id, expires: now + HOST_TTL_SECONDS * 1000, messages: [] });
      log(`room ${code} opened`);
    } else {
      if (!r) return json({ error: "no host in this room" }, 404);
      // Only the host hears about newcomers. A player sends its offer with
      // the join, so the host gets both in the same poll.
      deliver(r.host, { NewPeer: id }, ...signalBodies(id, parseJson(body)));
      return json({ id, token, host: r.host, via: hostVia(code, id, r.host) });
    }
    return json({ id, token });
  }

  if (action === "poll" && req.method === "GET") {
    const id = url.searchParams.get("id") || "";
    const asHost = url.searchParams.has("host");
    if (!validPeer(code, id, url.searchParams.get("token"), asHost)) return json({ error: "bad token" }, 403);
    const r = rooms.get(code);
    if (!r || (asHost && r.host !== id)) return json({ error: "room closed" }, 410);
    if (asHost && url.searchParams.has("keep")) r.expires = now + HOST_TTL_SECONDS * 1000;
    // Every message for the caller from the last MESSAGE_TTL_MS, with ids; the
    // client skips the ones it has seen (same contract as the live registry).
    // `seq` lets a long poll say which messages it already has.
    const reply = () => {
      const current = rooms.get(code);
      if (!current || (asHost && current.host !== id)) return json({ error: "room closed" }, 410);
      const out = json(current.messages.filter((m) => m.to === id).map((m) => ({ id: m.id, seq: m.seq, ...m.body })));
      out.headers[LONG_POLL_HEADER] = "1";
      return out;
    };
    if (!url.searchParams.has("wait")) return reply();
    const after = Number(url.searchParams.get("after")) || 0;
    if (r.messages.some((m) => m.to === id && m.seq > after)) return reply();
    return new Promise((resolve) => {
      let done = false;
      const finish = () => {
        if (done) return;
        done = true;
        clearTimeout(timer);
        waiting.get(id)?.delete(finish);
        resolve(reply());
      };
      const timer = setTimeout(finish, LONG_POLL_MS);
      if (!waiting.has(id)) waiting.set(id, new Set());
      waiting.get(id).add(finish);
      // the page went away mid-poll: stop holding it (the request itself
      // reports "close" as soon as its body is read, so watch the response)
      res.on("close", finish);
    });
  }

  // The host's offers made ahead of any join (same contract as the live registry).
  if (action === "spare" && req.method === "GET") {
    const offers = rooms.get(code)?.spare?.offers || [];
    if (!offers.length) return json({ error: "no spare offer" }, 404);
    const offer = offers[Math.floor(Math.random() * offers.length)];
    return json({ host: rooms.get(code).spare.host, oid: offer.oid, data: offer.data });
  }
  if (action === "spare" && req.method === "POST") {
    const data = parseJson(body);
    const id = typeof data?.id === "string" ? data.id : "";
    if (!data?.host || !validPeer(code, id, data?.token, true)) return json({ error: "bad token" }, 403);
    const r = rooms.get(code);
    if (!r || r.host !== id) return json({ error: "room closed" }, 410);
    const offers = (Array.isArray(data?.offers) ? data.offers.slice(0, 4) : [])
      .filter((o) => typeof o?.oid === "string" && /^[0-9a-f]{6,32}$/.test(o.oid) && o.data?.sdp?.type === "offer")
      .map((o) => ({ oid: o.oid, data: o.data }));
    r.spare = { host: id, offers };
    return empty();
  }

  if (["hello", "send", "leave"].includes(action) && req.method === "POST") {
    const data = parseJson(body);
    const id = typeof data?.id === "string" ? data.id : "";
    const asHost = Boolean(data?.host);
    if (!validPeer(code, id, data?.token, asHost)) return json({ error: "bad token" }, 403);
    const r = rooms.get(code);
    if (!r) return action === "leave" ? empty() : json({ error: "room closed" }, 410);

    if (action === "hello" && !asHost) {
      deliver(r.host, { NewPeer: id }, ...signalBodies(id, data));
    } else if (action === "send") {
      // Signals only flow between the host and a player.
      const signals = Array.isArray(data?.signals) ? data.signals.slice(0, MAX_SIGNALS) : [];
      for (const s of signals) {
        const receiver = asHost ? s?.receiver : r.host;
        if (typeof receiver !== "string" || !PEER_ID.test(receiver) || receiver === id) continue;
        deliver(receiver, { Signal: { sender: id, data: s.data } });
      }
    } else if (action === "leave") {
      if (asHost && r.host === id) {
        // A leaving host closes the room and takes its listing off the host list.
        rooms.delete(code);
        for (const [lid, l] of listings) if (l.entry.room === code) listings.delete(lid);
        log(`room ${code} closed`);
        // held polls hear that the room closed
        for (const peer of [...waiting.keys()]) wake(peer);
      } else if (!asHost) {
        deliver(r.host, { PeerLeft: id });
      }
    }
    return empty();
  }

  return json({ error: "not found" }, 404);
}

function signalsOf(data) {
  return (Array.isArray(data?.signals) ? data.signals.slice(0, MAX_SIGNALS) : []).filter((s) => s && typeof s === "object");
}

// Names the host a player joined, as on the live registry (where it saves a
// read on each send; here the room's host is always at hand).
function hostVia(code, id, host) {
  return createHmac("sha256", SIGNAL_SECRET).update(`${code}:${id}>${host}`).digest("hex");
}

function peerToken(code, id, asHost) {
  return createHmac("sha256", SIGNAL_SECRET).update(`${code}:${id}:${asHost ? "host" : "player"}`).digest("hex");
}

function validPeer(code, id, token, asHost) {
  return PEER_ID.test(id) && typeof token === "string" && token === peerToken(code, id, asHost);
}

// ---------------------------------------------------------------- static files

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".otf": "font/otf",
  ".ttf": "font/ttf",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".glb": "model/gltf-binary",
  ".gltf": "model/gltf+json",
  ".bin": "application/octet-stream",
  ".wav": "audio/wav",
  ".ogg": "audio/ogg",
  ".mp3": "audio/mpeg",
};

// The page, told to use this server for everything online. Read on every
// request so a rebuild (./build_web.sh) shows up on reload.
function indexHtml() {
  const html = readFileSync(join(SITE, "index.html"), "utf8");
  const marker = '<script src="gridlock_web.js"></script>';
  const inject =
    "<script>\n" +
    "        // gridlock local server: registry and models from this machine (registry/local/server.mjs)\n" +
    "        window.GRIDLOCK_REGISTRY = location.origin;\n" +
    '        if (window.GRIDLOCK_MODELS_URL) window.GRIDLOCK_MODELS_URL = location.origin + "/models";\n' +
    "    </script>\n    ";
  return html.includes(marker) ? html.replace(marker, inject + marker) : null;
}

// Path under `root` for a URL path, or null if it would leave `root`.
function within(root, urlPath) {
  let decoded;
  try {
    decoded = decodeURIComponent(urlPath);
  } catch {
    return null;
  }
  if (decoded.includes("\0")) return null;
  const file = normalize(join(root, decoded));
  return file === root || file.startsWith(root + sep) ? file : null;
}

function sendFile(req, res, file, cache) {
  let stat;
  try {
    stat = statSync(file);
  } catch {
    return false;
  }
  if (!stat.isFile()) return false;
  const headers = {
    "Content-Type": TYPES[extname(file).toLowerCase()] || "application/octet-stream",
    "Content-Length": stat.size,
    "Cache-Control": cache,
  };
  res.writeHead(200, headers);
  if (req.method === "HEAD") res.end();
  else createReadStream(file).pipe(res);
  return true;
}

function serveStatic(req, res, pathname) {
  if (req.method !== "GET" && req.method !== "HEAD") return false;

  if (pathname.startsWith("/models/") && MODELS_DIR) {
    // The page asks for /models/<path>?v=<version>; the query is ignored here.
    const file = within(MODELS_DIR, pathname.slice("/models".length));
    return file ? sendFile(req, res, file, "public, max-age=86400") : false;
  }
  if (!SITE) return false;

  if (pathname === "/" || pathname === "/index.html") {
    const html = indexHtml();
    if (html === null) {
      res.writeHead(500, { "Content-Type": "text/plain" });
      res.end("index.html has no gridlock_web.js script tag; is this a Gridlock browser build?");
      return true;
    }
    res.writeHead(200, { "Content-Type": TYPES[".html"], "Cache-Control": "no-cache" });
    res.end(req.method === "HEAD" ? undefined : html);
    return true;
  }
  const file = within(SITE, pathname);
  // no-cache: the browser revalidates, so a rebuilt wasm is picked up on reload.
  return file ? sendFile(req, res, file, "no-cache") : false;
}

// ---------------------------------------------------------------- server

const server = createServer((req, res) => {
  const url = new URL(req.url, "http://local");
  const [, root, arg, action] = url.pathname.split("/");

  const reply = ({ status, body, headers }) => {
    res.writeHead(status, { ...CORS, ...headers });
    res.end(body);
  };

  if (req.method === "OPTIONS") return reply({ status: 204, headers: {} });

  const api = root === "servers" || root === "ice" || (root === "room" && arg && action);
  if (!api) {
    try {
      if (serveStatic(req, res, url.pathname)) return;
    } catch (e) {
      console.error("gridlock-local:", e);
      if (!res.headersSent) return reply(json({ error: "server error" }, 500));
      return res.destroy();
    }
    return reply(json({ error: "not found" }, 404));
  }

  readBody(req)
    .then((body) => {
      if (root === "ice") return req.method === "GET" ? json({ iceServers: [] }) : json({ error: "not found" }, 404);
      if (root === "servers") return servers(req, body, arg, action);
      if (!ROOM_CODE.test(arg)) return json({ error: "bad room code" }, 400);
      return room(req, res, body, url, arg, action);
    })
    .catch((e) => (e === TOO_BIG ? json({ error: "too large" }, 413) : json({ error: "server error" }, 500)))
    .then(reply);
});

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization",
  "Access-Control-Expose-Headers": "X-Gridlock-Long-Poll",
};

const TOO_BIG = new Error("body too large");

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on("data", (c) => {
      size += c.length;
      if (size > MAX_BODY) {
        reject(TOO_BIG);
        req.destroy();
      } else chunks.push(c);
    });
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });
}

server.on("error", (e) => {
  if (e.code === "EADDRINUSE") fail(`port ${opts.port} is in use; pick another with --port`);
  fail(e.message);
});

server.listen(opts.port, opts.host, () => {
  const port = server.address().port;
  const lan = Object.values(networkInterfaces())
    .flat()
    .filter((a) => a && a.family === "IPv4" && !a.internal)
    .map((a) => a.address);
  console.log("Gridlock local server");
  if (SITE) {
    console.log(`  Play on this machine:   http://localhost:${port}`);
    for (const ip of opts.host === "0.0.0.0" ? lan : []) console.log(`  Play from your network: http://${ip}:${port}`);
  } else {
    console.log(`  No browser build in ${opts.dir}: run ./build_web.sh first (registry only for now).`);
  }
  const registryHost = opts.host === "0.0.0.0" ? lan[0] || "localhost" : opts.host;
  console.log(`  Native clients:         GRIDLOCK_REGISTRY=http://${registryHost}:${port} ./client`);
  if (SITE && !MODELS_DIR && !existsSync(join(SITE, "assets", "models"))) {
    console.log("  Warning: no models found; build with ./build_web.sh (no GRIDLOCK_MODELS_URL) or pass --models.");
  }
  console.log("  Ctrl+C to stop.\n");
});

// Nothing to save (state is in memory), so stop at once instead of waiting
// for open keep-alive connections to close.
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => process.exit(0));

// ---------------------------------------------------------------- helpers

function clientIp(req) {
  const ip = req.socket.remoteAddress || "";
  return ip.startsWith("::ffff:") ? ip.slice(7) : ip;
}

function cleanName(value) {
  if (typeof value !== "string") return null;
  const name = value.replace(/[\u0000-\u001f\u007f]/g, "").trim().slice(0, 32);
  return name || null;
}

function int(value, min, max) {
  return Number.isInteger(value) && value >= min && value <= max ? value : null;
}

function parseJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function sha256Hex(text) {
  return createHash("sha256").update(text).digest("hex");
}

function json(value, status = 200) {
  return { status, body: JSON.stringify(value), headers: { "Content-Type": "application/json" } };
}

function empty() {
  return { status: 204, body: undefined, headers: {} };
}

function log(message) {
  console.log(`[${new Date().toLocaleTimeString()}] ${message}`);
}
