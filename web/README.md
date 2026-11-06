# Gridlock in the browser

The same client, built for `wasm32-unknown-unknown` and loaded by macroquad's
JS loader. Players host and join through WebRTC, so nobody needs port
forwarding: the host's tab runs the server, and the Cloudflare Worker in
`registry/` introduces players to it (and relays traffic through TURN for the
few networks that block direct connections, once a TURN key is set).

| File | Role |
|---|---|
| `index.html` | The page: a full-window canvas and the three scripts below. |
| `mq_js_bundle.js` | macroquad's loader (vendored from `macroquad-0.4.14/js/`). Update it together with the macroquad version. |
| `gridlock_web.js` | Gridlock's plugin for that loader: WebRTC + signalling, fetch, Web Audio, clocks, randomness, localStorage, pointer lock. |

Why a plugin instead of wasm-bindgen: miniquad's loader instantiates the wasm
module itself and cannot provide wasm-bindgen's imports, so crates that need
them (matchbox, web-time, getrandom's `js` backend, rodio's browser backend)
are replaced by small `extern "C"` imports that `gridlock_web.js` supplies.
The Rust side of those imports lives in `platform/`, `transport/src/web.rs` and
`client/src/audio.rs`. If you add one, add it to the plugin too; the plugin and
the wasm must agree, and `PLUGIN_VERSION` / `gridlock_web_crate_version` let
miniquad warn when they do not.

## Games on your network

On <https://gridlock.kwaddo.dev>, games hosted from your own network are at
the top of BROWSE with a green LAN badge. The registry compares addresses:
a game whose host reached it from the same public address (IPv4) or the same
/64 (IPv6) is marked `lan` in its reply; the address itself is never shown.
Game traffic between players always goes directly from browser to browser
over WebRTC (on a LAN, inside it); only the brief introduction goes through
the registry. A page on HTTPS cannot reach a local server
(`http://192.168…`), so games hosted on one (below) are listed only there.

## Play offline / on your network

```bash
./play_local.sh            # builds dist/web the first time; --rebuild to build again
# or, after any build:
./build_web.sh --serve
```

Both run `registry/local/server.mjs` (Node.js 18+, no packages). It serves the
page and the registry's API from your machine, kept in memory, and prints
the addresses to open:

```
Gridlock local server
  Play on this machine:   http://localhost:8080
  Play from your network: http://192.168.1.20:8080
```

No internet is needed. Anyone on the same network opens the second address
and can host (HOST tab), see the games in BROWSE, or join one by its room code
(DIRECT). Open two windows on one machine to try it alone.

How it works: the server adds `window.GRIDLOCK_REGISTRY = location.origin` to
the page it serves, and the game uses that at run time instead of the registry
baked into the build (`platform::page_setting`). The same build therefore
works online and offline. `/ice` returns no ICE servers, since peers on one
network connect directly, and `/models/*` is served from the build (or from
`client/assets/models` when the build left them out).

Options: `--port`, `--host` (default `0.0.0.0`, every interface; use
`127.0.0.1` to keep it to this machine), `--dir`, `--models`. Native clients
can use the same server: `GRIDLOCK_REGISTRY=http://<that address> ./client`.

If players on other machines see the game list but cannot connect:

- Check that both machines are on the same network, and that the network does
  not isolate clients from each other. Many guest Wi-Fi networks do.
- Check that the firewall lets Node.js accept connections. macOS asks the
  first time the server starts.
- Browsers hide local IP addresses behind mDNS names (`<id>.local`), and the
  network must pass mDNS (multicast) for peers to find each other.

The page is plain HTTP on a LAN address, so the browser does not keep models
in Cache Storage there (it needs HTTPS or localhost). Each load downloads them
again from the local server, which is fast on a LAN.

## Publish on Dilmune

Pushing to `main` publishes it: the Deploy workflow
(`.github/workflows/deploy.yml`) builds the browser version and pushes
`dist/web` to the `Kwaddo/gridlock-web` repository, which the Dilmune app
rebuilds from, serving <https://gridlock.kwaddo.dev>. The workflow can also be
started by hand (Actions tab, or `gh workflow run deploy.yml`).

DCS apps are capped at 100 MiB, so the models (about 300 MB) are not in the
build; the registry function serves them (`registry/dcs/README.md`, which also
covers uploading changed assets). Without `GRIDLOCK_MODELS_URL`, `build_web.sh`
bundles the models as before, which suits local testing with `--serve`.

## Differences from the native build

- Browser players and native players cannot join each other: browsers can only
  use WebRTC, and native builds use UDP. The online list shows each build only
  the games it can join.
- Joining by address becomes joining by room code (DIRECT tab).
- The mouse locks on the first click into the game, as browsers require.
- Settings and crosshair presets are saved in the browser's localStorage (per
  site), instead of files. The local asset cache is skipped; the browser's HTTP
  cache does that job.

## Debugging

- `?relay` on the page URL forces every WebRTC connection through the TURN
  relay; `await gridlockConnections()` in the console shows how each connection
  is routed. See `registry/README.md`, "Checking the relay works".
- `localStorage` keys starting with `gridlock:` hold the saved settings and presets.
