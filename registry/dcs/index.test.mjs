// Tests for the DCS registry (index.js), against an in-memory S3 bucket that
// stands in for STATE_BUCKET behind a stubbed fetch().
//
//   node --test registry/dcs/index.test.mjs
//
// REGISTRY_UNDER_TEST=<path> runs them against another copy of index.js
// (an older one, to see a test fail without its fix).
import assert from "node:assert/strict";
import { test } from "node:test";
import { pathToFileURL } from "node:url";

const modulePath = process.env.REGISTRY_UNDER_TEST
  ? pathToFileURL(process.env.REGISTRY_UNDER_TEST).href
  : new URL("./index.js", import.meta.url).href;
const { default: registry } = await import(modulePath);

const ENDPOINT = "https://s3.test";
const BUCKET = "state";
const env = {
  S3_ENDPOINT: ENDPOINT,
  S3_ACCESS_KEY_ID: "key",
  S3_SECRET_ACCESS_KEY: "secret",
  STATE_BUCKET: BUCKET,
  SIGNAL_SECRET: "signal-secret",
};

// ---------------------------------------------------------------- the bucket

// key -> { body, modified }
const objects = new Map();
// Counts of S3 requests by method ("LIST" for a listing).
const calls = {};
const realFetch = globalThis.fetch;

globalThis.fetch = async (input, init = {}) => {
  const url = new URL(String(input));
  if (url.origin !== ENDPOINT) return realFetch(input, init);
  const method = init.method || "GET";
  const key = decodeURIComponent(url.pathname.slice(`/${BUCKET}/`.length));
  if (method === "GET" && key === "") return listObjects(url.searchParams);
  calls[method] = (calls[method] || 0) + 1;
  if (method === "PUT") {
    objects.set(key, { body: String(init.body), modified: new Date().toISOString() });
    return new Response(null, { status: 200 });
  }
  if (method === "DELETE") {
    objects.delete(key);
    return new Response(null, { status: 204 });
  }
  const object = objects.get(key);
  if (!object) return new Response("<Error/>", { status: 404 });
  return new Response(object.body, { status: 200 });
};

// ListObjectsV2: prefix, max-keys, start-after and continuation-token.
function listObjects(params) {
  calls.LIST = (calls.LIST || 0) + 1;
  const prefix = params.get("prefix") || "";
  const max = Number(params.get("max-keys") || 1000);
  const after = params.get("continuation-token") || params.get("start-after") || "";
  const keys = [...objects.keys()].filter((k) => k.startsWith(prefix) && k > after).sort();
  const page = keys.slice(0, max);
  const truncated = keys.length > max;
  const contents = page
    .map((k) => `<Contents><Key>${k}</Key><LastModified>${objects.get(k).modified}</LastModified></Contents>`)
    .join("");
  const next = truncated ? `<NextContinuationToken>${page.at(-1)}</NextContinuationToken>` : "";
  return new Response(`<ListBucketResult><IsTruncated>${truncated}</IsTruncated>${next}${contents}</ListBucketResult>`);
}

function readObject(key) {
  const object = objects.get(key);
  return object && JSON.parse(object.body);
}

function putObject(key, value) {
  objects.set(key, { body: JSON.stringify(value), modified: new Date().toISOString() });
}

// ---------------------------------------------------------------- requests

// Work a request left for after its response (ctx.waitUntil).
const background = [];
const ctx = { waitUntil: (p) => background.push(p) };

async function settled() {
  while (background.length) await Promise.allSettled(background.splice(0));
}

async function call(method, path, { body, headers = {}, ip = "203.0.113.7" } = {}) {
  const request = new Request(`https://registry.test${path}`, {
    method,
    headers: { "CF-Connecting-IP": ip, ...headers },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return registry.fetch(request, env, ctx);
}

// A request whose body arrives only when `release` is called: the request
// is held there, as by a slow client.
function heldRequest(method, path, headers, body) {
  let release;
  const arrived = new Promise((r) => (release = r));
  const request = {
    method,
    url: `https://registry.test${path}`,
    headers: new Headers({ "CF-Connecting-IP": "203.0.113.7", ...headers }),
    json: async () => {
      await arrived;
      return body;
    },
  };
  return { request, release };
}

async function list(name, ip) {
  const resp = await call("POST", "/servers", { body: { name, room: name, players: 1, max_players: 16 }, ip });
  assert.equal(resp.status, 201);
  return resp.json();
}

function listings() {
  return readObject("state.json")?.listings || {};
}

// ---------------------------------------------------------------- tests

test("a heartbeat does not write back the host list it read before its body came in", async () => {
  objects.clear();
  const a = await list("ROOMAA", "203.0.113.1");
  await settled();

  const { request, release } = heldRequest("PUT", `/servers/${a.id}`, { Authorization: `Bearer ${a.token}` }, { players: 3 });
  const beating = registry.fetch(request, env, ctx);
  // Let the heartbeat get as far as it can without its body.
  await new Promise((r) => setTimeout(r, 50));

  // Another host lists itself meanwhile, and its check after a moment is done.
  const b = await list("ROOMBB", "203.0.113.2");
  await settled();
  assert.ok(listings()[b.id]);

  release();
  assert.equal((await beating).status, 204);
  await settled();
  const after = listings();
  assert.ok(after[b.id], "the other host's listing survives the heartbeat");
  assert.equal(after[a.id].entry.players, 3);
});

test("a heartbeat with the wrong token, or for an unknown listing, is 404", async () => {
  objects.clear();
  const a = await list("ROOMCC", "203.0.113.3");
  await settled();
  const wrong = await call("PUT", `/servers/${a.id}`, { body: { players: 2 }, headers: { Authorization: "Bearer nope" } });
  assert.equal(wrong.status, 404);
  const unknown = await call("PUT", `/servers/${crypto.randomUUID()}`, {
    body: { players: 2 },
    headers: { Authorization: `Bearer ${a.token}` },
  });
  assert.equal(unknown.status, 404);
  assert.equal(listings()[a.id].entry.players, 1);
});

test("a listing keeps the bot count a host sends, and a heartbeat updates it", async () => {
  objects.clear();
  const body = { name: "ROOMBT", room: "ROOMBT", players: 8, bots: 5, max_players: 8 };
  const created = await call("POST", "/servers", { body, ip: "203.0.113.8" });
  assert.equal(created.status, 201);
  const a = await created.json();
  await settled();
  assert.deepEqual([listings()[a.id].entry.players, listings()[a.id].entry.bots], [8, 5]);

  const auth = { Authorization: `Bearer ${a.token}` };
  // A beat that renews nothing writes nothing: keep them a few ms apart.
  const beat = async (body) => {
    await new Promise((r) => setTimeout(r, 5));
    return call("PUT", `/servers/${a.id}`, { body, headers: auth });
  };
  assert.equal((await beat({ players: 6, bots: 2 })).status, 204);
  assert.deepEqual([listings()[a.id].entry.players, listings()[a.id].entry.bots], [6, 2]);
  // never more bots than players
  assert.equal((await beat({ players: 3, bots: 9 })).status, 204);
  assert.deepEqual([listings()[a.id].entry.players, listings()[a.id].entry.bots], [3, 3]);
  // more players than seats (a lobby beside a bot-filled match): the people
  // are capped at the seats, the bots are what is left
  assert.equal((await beat({ players: 10, bots: 7 })).status, 204);
  assert.deepEqual([listings()[a.id].entry.players, listings()[a.id].entry.bots], [8, 5]);
  assert.equal((await beat({ players: 13, bots: 4 })).status, 204);
  assert.deepEqual([listings()[a.id].entry.players, listings()[a.id].entry.bots], [8, 0]);
  // a heartbeat without a count (as from an older host) drops it
  assert.equal((await beat({ players: 4 })).status, 204);
  assert.equal(listings()[a.id].entry.players, 4);
  assert.equal("bots" in listings()[a.id].entry, false);
});

test("a host from before the bot count lists none, and a bad count is ignored", async () => {
  objects.clear();
  const a = await list("ROOMOB", "203.0.113.9");
  await settled();
  assert.equal("bots" in listings()[a.id].entry, false);
  const beat = await call("PUT", `/servers/${a.id}`, {
    body: { players: 5 },
    headers: { Authorization: `Bearer ${a.token}` },
  });
  assert.equal(beat.status, 204);
  assert.equal(listings()[a.id].entry.players, 5);
  assert.equal("bots" in listings()[a.id].entry, false);

  const body = { name: "ROOMBD", room: "ROOMBD", players: 2, bots: -1, max_players: 8 };
  const created = await call("POST", "/servers", { body, ip: "203.0.113.10" });
  assert.equal(created.status, 201);
  const b = await created.json();
  await settled();
  assert.equal(listings()[b.id].entry.players, 2);
  assert.equal("bots" in listings()[b.id].entry, false);
});

test("a listing is removed only with its token", async () => {
  objects.clear();
  const a = await list("ROOMDD", "203.0.113.4");
  await settled();
  assert.equal((await call("DELETE", `/servers/${a.id}`, { headers: { Authorization: "Bearer nope" } })).status, 404);
  assert.ok(listings()[a.id]);
  assert.equal((await call("DELETE", `/servers/${a.id}`, { headers: { Authorization: `Bearer ${a.token}` } })).status, 204);
  await settled();
  assert.equal(listings()[a.id], undefined);
});

test("a room listed from another address is refused", async () => {
  objects.clear();
  await list("ROOMEE", "203.0.113.5");
  await settled();
  const resp = await call("POST", "/servers", {
    body: { name: "x", room: "ROOMEE", players: 1, max_players: 16 },
    ip: "198.51.100.9",
  });
  assert.equal(resp.status, 409);
});

// ---------------------------------------------------------------- signalling

async function hostRoom(code) {
  const resp = await call("POST", `/room/${code}/join?host`, { body: {} });
  assert.equal(resp.status, 200);
  return resp.json();
}

test("an idle host's held poll looks about once a second and reads the claim once", async () => {
  const host = await hostRoom("IDLE1");
  const before = { LIST: calls.LIST || 0, GET: calls.GET || 0 };
  const started = Date.now();
  const resp = await call("GET", `/room/IDLE1/poll?id=${host.id}&token=${host.token}&host&idle&wait&after=0&have=`);
  assert.equal(resp.status, 200);
  assert.deepEqual(await resp.json(), []);
  assert.ok(Date.now() - started >= 5000, "held");
  const lists = calls.LIST - before.LIST;
  // one look at the start, then one a second for LONG_POLL_MS (6 s)
  assert.ok(lists >= 2 && lists <= 9, `looks: ${lists}`);
  // the claim, once, at the start: not again while held
  assert.equal(calls.GET - before.GET, 1);
  await settled();
});

test("a busy host's held poll, or an idle one a player writes to, answers at once", async () => {
  const host = await hostRoom("BUSY1");
  const join = await call("POST", "/room/BUSY1/join", { body: { signals: [] } });
  assert.equal(join.status, 200);
  const started = Date.now();
  const resp = await call("GET", `/room/BUSY1/poll?id=${host.id}&token=${host.token}&host&busy&wait&after=0&have=`);
  const batch = await resp.json();
  assert.ok(batch.some((m) => m.NewPeer), "the join is there");
  assert.ok(Date.now() - started < 1000);
  // An idle host's held poll wakes when a player writes in the same isolate.
  const seq = Math.max(...batch.map((m) => m.seq));
  const have = batch.map((m) => m.id).join(",");
  const held = call("GET", `/room/BUSY1/poll?id=${host.id}&token=${host.token}&host&idle&wait&after=${seq}&have=${have}`);
  await new Promise((r) => setTimeout(r, 300));
  const woke = Date.now();
  const player = await join.json();
  await call("POST", "/room/BUSY1/hello", { body: { id: player.id, token: player.token } });
  const next = await (await held).json();
  assert.ok(next.some((m) => m.NewPeer === player.id), "the hello is there");
  assert.ok(Date.now() - woke < 1500);
  await settled();
});

test("a poll finds a new message behind more than a page of older ones", async () => {
  const host = await hostRoom("MANY1");
  const now = Date.now();
  // 250 messages from 50 s ago (not expired), all already handled.
  for (let i = 0; i < 250; i++) {
    const at = now - 50000 + i;
    putObject(`rooms/MANY1/host/${String(at).padStart(15, "0")}-${i.toString(16).padStart(8, "0")}.json`, {
      at,
      sender: "old",
      bodies: [{ NewPeer: "old" }],
    });
  }
  const join = await call("POST", "/room/MANY1/join", { body: { signals: [] } });
  const player = await join.json();
  const after = now - 10000;
  const resp = await call("GET", `/room/MANY1/poll?id=${host.id}&token=${host.token}&host&after=${after}&have=`);
  const batch = await resp.json();
  assert.deepEqual(
    batch.map((m) => m.NewPeer),
    [player.id],
  );
  await settled();
});

test("a poll lists a flood of new messages past the first page", async () => {
  const host = await hostRoom("FLOOD1");
  const now = Date.now();
  for (let i = 0; i < 250; i++) {
    const at = now - 5000 + i;
    putObject(`rooms/FLOOD1/host/${String(at).padStart(15, "0")}-${i.toString(16).padStart(8, "0")}.json`, {
      at,
      sender: `p${i}`,
      bodies: [{ NewPeer: `p${i}` }],
    });
  }
  const resp = await call("GET", `/room/FLOOD1/poll?id=${host.id}&token=${host.token}&host&after=0&have=`);
  assert.equal((await resp.json()).length, 250);
  await settled();
});
