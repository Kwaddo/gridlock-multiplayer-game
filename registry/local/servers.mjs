// The host list: GET/POST/PUT/DELETE /servers (registry/local/server.mjs).

import { randomUUID } from "node:crypto";

import { MAX_LISTINGS, MAX_PER_IP, PEER_ID, ROOM_CODE, TTL_SECONDS } from "./config.mjs";
import { cleanName, clientIp, empty, int, json, log, parseJson, sha256Hex } from "./helpers.mjs";
import { listings, prune } from "./state.mjs";

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

export { servers };
