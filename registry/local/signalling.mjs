// WebRTC signalling for the browser build: /room/:code/...
// (registry/local/server.mjs).

import { createHmac, randomUUID } from "node:crypto";

import { HOST_TTL_SECONDS, LONG_POLL_HEADER, LONG_POLL_MS, MAX_SIGNALS, PEER_ID } from "./config.mjs";
import { empty, json, log, parseJson } from "./helpers.mjs";
import { listings, prune, rooms, SIGNAL_SECRET } from "./state.mjs";

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

export { room };
