# Gridlock online services

> The live game now runs these services as a Dilmune edge function instead;
> see `dcs/README.md`. The browser build's signalling there is HTTP polling,
> so the Worker below no longer works for browser games (native hosts and the
> host list still do).

One Cloudflare Worker provides everything the game needs online:

| Route | Purpose |
|---|---|
| `GET/POST/PUT/DELETE /servers` | Public host list. Hosts heartbeat every 30 s; a listing expires 90 s after its last one. |
| `GET /ice` | STUN servers, plus short-lived TURN relay credentials once a TURN key is set. |
| `GET /room/:code` (WebSocket) | WebRTC signalling for the browser build (matchbox protocol). Players are only introduced to the host. |

It fits in Cloudflare's free plan. State is kept in Durable Objects, not KV,
because KV's free tier allows only 1,000 writes a day.

## First-time setup

You need a Cloudflare account and Node.js 18 or newer (`node --version`).

1. **Create an account.** Sign up at <https://dash.cloudflare.com/sign-up> and
   verify your email. The free plan is enough; no domain is needed.

2. **Log wrangler in.** Wrangler is Cloudflare's CLI; `npx` downloads it on first use.
   ```bash
   cd registry
   npx wrangler login
   ```
   A browser tab opens; approve the access request.

3. **Deploy.**
   ```bash
   npx wrangler deploy
   ```
   On the first deploy wrangler asks you to choose a `workers.dev` subdomain
   (once per account). It then prints the Worker's URL, for example
   `https://gridlock-registry.<your-subdomain>.workers.dev`.

4. **Check it works.**
   ```bash
   curl https://gridlock-registry.<your-subdomain>.workers.dev/servers   # []
   curl https://gridlock-registry.<your-subdomain>.workers.dev/ice       # STUN servers
   ```

5. **Point the game at it.** The URL is read from `GRIDLOCK_REGISTRY`, either baked
   in at build time (what you want for builds you hand out) or set at run time:
   ```bash
   GRIDLOCK_REGISTRY=https://gridlock-registry.<your-subdomain>.workers.dev cargo build -p client --release
   GRIDLOCK_REGISTRY=https://gridlock-registry.<your-subdomain>.workers.dev ./client
   ```
   The build scripts (`build_mac.sh` etc.) pick it up the same way if it is exported.
   Without it the game still hosts and finds games on the local network; the
   online list is turned off.

To redeploy after changing `src/index.js`, run `npx wrangler deploy` again.
`npx wrangler tail` streams the Worker's live logs.

## Optional: TURN relay

Some players (strict corporate or mobile networks) cannot connect to anyone
directly over WebRTC. A TURN server relays their traffic. Cloudflare's includes
1,000 GB a month free, then $0.05/GB.

1. In the Cloudflare dashboard open **Realtime** (formerly **Calls**) → **TURN Server**
   and create a TURN key. Copy its **Key ID** and **API token**.
2. Store both as Worker secrets (wrangler prompts for each value):
   ```bash
   npx wrangler secret put TURN_KEY_ID
   npx wrangler secret put TURN_KEY_API_TOKEN
   ```
3. `curl .../ice` should now also list `turn:` and `turns:` URLs with a username
   and credential. The credentials are minted per request and last a day.

Any other TURN server (e.g. your own coturn) works too; set these instead:
```bash
npx wrangler secret put TURN_URLS         # e.g. turn:turn.example.com:3478,turns:turn.example.com:5349
npx wrangler secret put TURN_USERNAME
npx wrangler secret put TURN_CREDENTIAL
```

The relay only matters for the browser build; the native build uses UDP.

### Checking the relay works

Most players connect directly and never touch the relay, so force it:

1. Open the game with `?relay` on the URL in two windows (for example
   `https://gridlock.pages.dev/?relay`). That forbids direct connections, so a
   join only succeeds through TURN.
2. Host in one window, join from the other.
3. In either window's developer console run `await gridlockConnections()`. Each
   connection should report `path: "relay"`, with `remote` pointing at the TURN
   server. Without `?relay` you will normally see `"direct"` or `"stun"`.
4. For a real-world check, put the two players on different networks (one on a
   phone hotspot) without `?relay`; if they cannot connect directly, the path
   falls back to `"relay"` by itself.

The same works locally against a throwaway TURN server, which is how it was
tested: run any TURN server on `127.0.0.1:3478`, then
`npx wrangler dev --var TURN_URLS:turn:127.0.0.1:3478 --var TURN_USERNAME:<user> --var TURN_CREDENTIAL:<pass>`.
With the TURN server stopped, `?relay` joins must fail, which shows the relay
was really carrying the traffic.

## Local testing

```bash
cd registry
npx wrangler dev --port 8787                      # local Workers runtime, no account needed
GRIDLOCK_REGISTRY=http://127.0.0.1:8787 cargo run -p client
GRIDLOCK_REGISTRY=http://127.0.0.1:8787 cargo test -p client --test registry_live -- --ignored
```

## How native hosts become reachable

When a host ticks "List on the online server browser", the game asks the home
router to forward UDP 7878 (UPnP, falling back to NAT-PMP), lists the port the
router actually opened, and closes it again when the game ends. The host gets a
message saying whether it worked. It cannot work behind carrier-grade NAT (the
ISP shares one public address between customers); the game detects that by
comparing the router's external address with the one this Worker sees, and
tells the host. Those hosts need manual port forwarding upstream, or players
can use the browser build, where WebRTC (with the TURN relay) gets through.

## Limits

The free Workers plan allows 100,000 requests a day. A host makes one request
every 30 s (2,880 a day), so about 30 hosts online around the clock, plus
browsing, fits. Raise the heartbeat interval in `client/src/net/registry.rs`
(keeping it under the 90 s TTL in `src/index.js`) if you get close.
