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

## Build and try it locally

Deploy the Worker first (`registry/README.md`), or run it locally with
`npx wrangler dev` in `registry/`. Then:

```bash
GRIDLOCK_REGISTRY=https://gridlock-registry.<you>.workers.dev ./build_web.sh --serve
# or, against a local Worker:
GRIDLOCK_REGISTRY=http://127.0.0.1:8787 ./build_web.sh --serve
```

Open <http://localhost:8080> in two windows: host in one (HOST tab), join from
the other (BROWSE, or DIRECT with the room code the host is shown).

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
