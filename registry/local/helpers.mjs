// Small helpers of the local server (registry/local/server.mjs): request
// fields, replies and the log.

import { createHash } from "node:crypto";

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

// The counts a listing shows: `players` (bots included) and, when the host
// sent it, `bots`, how many of them are bots. A joiner takes a bot's place,
// so the list counts a game full only when its people fill it: the seats cap
// the people, and the bots are whatever is left of the capped players.
// Hosts from before the bot count send none, and list none.
function occupancy(players, bots, maxPlayers) {
  const listed = Math.min(players, maxPlayers);
  if (bots === null) return { players: listed };
  const people = Math.min(players - Math.min(bots, players), maxPlayers);
  return { players: listed, bots: listed - people };
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

export { clientIp, cleanName, int, occupancy, parseJson, sha256Hex, json, empty, log };
