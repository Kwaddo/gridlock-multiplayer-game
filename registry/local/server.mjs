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

//
// The pieces live next to this file: config.mjs (limits), options.mjs (the
// command line), state.mjs, servers.mjs (host list), signalling.mjs,
// static.mjs and helpers.mjs. This file is the HTTP server itself.

import { createServer } from "node:http";
import { existsSync } from "node:fs";
import { networkInterfaces } from "node:os";
import { join } from "node:path";

import { CORS, MAX_BODY, ROOM_CODE } from "./config.mjs";
import { fail, MODELS_DIR, opts, SITE } from "./options.mjs";
import { json } from "./helpers.mjs";
import { servers } from "./servers.mjs";
import { room } from "./signalling.mjs";
import { serveStatic } from "./static.mjs";

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
