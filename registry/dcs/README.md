# Gridlock online services on Dilmune (DCS)

`index.js` is a DCS edge function with the same job as the Cloudflare Worker
in `../src/`: the host list, ICE servers, WebRTC signalling for the browser
build, and HTTPS access to the game's models. It is what the live game uses:
`https://gridlock-registry.fn.dilmune.app`.

How it differs from the Worker:

- **State in one object.** DCS functions have no Durable Objects, and requests
  land in different isolates with no shared memory. All state (the host list
  and each open room with its waiting signalling messages) is one object,
  `state.json`, in the private `gridlock-state` bucket, read and written
  through the S3 API. Polls only read; joins, offers, answers, leaves and
  heartbeats write. Nothing else is created there.
- **Signalling over HTTP.** Two WebSockets in different isolates can never
  meet, so a room is a set of mailboxes in `state.json`: peers
  `POST /room/:code/send` and `GET /room/:code/poll`. Each side sends one
  complete offer or answer, and a player that has not connected re-announces
  itself (`/hello`) every few seconds, which heals a write lost to two writes
  at once (the bucket has no atomic updates). `web/gridlock_web.js` speaks
  this protocol only; the Worker's WebSocket rooms no longer match it.
- **Models.** The models (about 300 MB) are too big for a DCS app bundle
  (100 MiB). They sit in the public `gridlock-assets` bucket, whose own URL
  only works over plain HTTP, so `/models/*` relays them over HTTPS with CORS
  and long cache headers. Browsers keep them in Cache Storage after the
  first download.

## Settings

Secrets on the function (`set_app_secrets`, then redeploy):

| Name | Value |
|---|---|
| `S3_ENDPOINT` | `https://api.dilmune.com/s3` |
| `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY` | An S3 key that can read and write `gridlock-state` |
| `STATE_BUCKET` | `gridlock-state` (must be private: it holds room tokens) |
| `SIGNAL_SECRET` | Any long random string (`openssl rand -hex 32`); signs peer tokens |
| `MODELS_ORIGIN` | The `gridlock-assets` bucket's public URL, `http://<bucket-id>.pub.dilmune.app` |
| `TURN_*` (optional) | As in `../README.md`, "Optional: TURN relay" |

## Deploying

Pushing to `main` deploys it when the repository has a `DCS_API_KEY` secret:
the Deploy workflow (`.github/workflows/deploy.yml`) sends `index.js` to the
Dilmune API whenever it changed, before it publishes the browser build.
Without the key the workflow only warns. To deploy by hand, paste
`index.js` into `deploy_app` (DCS MCP) or the portal editor. Then check it:

```bash
curl https://gridlock-registry.fn.dilmune.app/servers   # []
curl https://gridlock-registry.fn.dilmune.app/ice       # STUN servers
```

## Updating assets

`client/assets` is not in git; its copy lives in the `gridlock-assets` bucket:
the models under `models/` (served by this function) and the other assets,
plus `manifest.json`, the one list of what every game loads
(`{ world: { key: path } }`). Every client loads from that manifest; hosts do
not send their own. After changing an asset or the manifest:

```bash
AWS_ACCESS_KEY_ID=<key> AWS_SECRET_ACCESS_KEY=<secret> ./upload_assets.sh
```

It uploads the changes and runs the Deploy workflow, which builds the page
with the bucket's manifest and stamps the models' new version into it, so
browsers fetch new files instead of their cached ones.
