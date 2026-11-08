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

export { clientIp, cleanName, int, parseJson, sha256Hex, json, empty, log };
