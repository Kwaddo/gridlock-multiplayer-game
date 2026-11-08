// The local server's limits and fixed tables (registry/local/server.mjs).
// The limits match registry/dcs/index.js unless noted.

import { dirname, resolve } from "node:path";
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

// Content types of the static files (static.mjs).
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

// On every reply (server.mjs).
const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization",
  "Access-Control-Expose-Headers": "X-Gridlock-Long-Poll",
};

export {
  TTL_SECONDS,
  HOST_TTL_SECONDS,
  MESSAGE_TTL_MS,
  MAX_LISTINGS,
  MAX_PER_IP,
  MAX_SIGNALS,
  MAX_BODY,
  ROOM_CODE,
  PEER_ID,
  LONG_POLL_MS,
  LONG_POLL_HEADER,
  REPO,
  TYPES,
  CORS,
};
