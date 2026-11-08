// The local server's state, all in memory (registry/local/server.mjs).

import { randomBytes } from "node:crypto";

import { MESSAGE_TTL_MS } from "./config.mjs";

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

export { listings, rooms, SIGNAL_SECRET, prune };
