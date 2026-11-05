# Gridlock online services on Dilmune (DCS)

`index.js` is a DCS edge function with the same job as the Cloudflare Worker
in `../src/`: the host list, ICE servers, WebRTC signalling for the browser
build, and HTTPS access to the game's models. It is what the live game uses:
`https://gridlock-registry.fn.dilmune.app`.

How it differs from the Worker:

- **State in a bucket.** DCS functions have no Durable Objects, and requests
  land in different isolates with no shared memory. Listings and rooms are
  small JSON objects in the private `gridlock-state` bucket, read and written
  through the S3 API. Lifecycle rules there delete anything under `servers/`
  or `rooms/` after a day, so abandoned rooms clean themselves up.
- **Signalling over HTTP.** Two WebSockets in different isolates can never
  meet, so a room is a set of mailboxes: peers `POST /room/:code/send` and
  `GET /room/:code/poll`. `web/gridlock_web.js` speaks this protocol only;
  the Worker's WebSocket rooms no longer match the browser client.
  Joining takes about 3 s.
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

Pushing to `main` deploys it: the Deploy workflow
(`.github/workflows/deploy.yml`) sends `index.js` to the Dilmune API whenever
it changed, before it publishes the browser build. To deploy by hand, paste
`index.js` into `deploy_app` (DCS MCP) or the portal editor. Then check it:

```bash
curl https://gridlock-registry.fn.dilmune.app/servers   # []
curl https://gridlock-registry.fn.dilmune.app/ice       # STUN servers
```

## Updating assets

`client/assets` is not in git; its copy lives in the `gridlock-assets` bucket
(models under `models/`, served by this function). After changing an asset:

```bash
AWS_ACCESS_KEY_ID=<key> AWS_SECRET_ACCESS_KEY=<secret> ./upload_assets.sh
```

It uploads the changes and runs the Deploy workflow, which stamps the models'
new version into the page so browsers fetch the new files instead of their
cached ones.
