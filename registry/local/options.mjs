// Command line options of the local server (registry/local/server.mjs),
// parsed when this module loads.

import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

import { REPO } from "./config.mjs";

// ---------------------------------------------------------------- options

function parseArgs(argv) {
  const opts = {
    port: Number(process.env.PORT) || 8080,
    host: process.env.HOST || "0.0.0.0",
    dir: join(REPO, "dist", "web"),
    models: null,
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) fail(`${arg} needs a value`);
      return argv[++i];
    };
    if (arg === "--port" || arg === "-p") opts.port = Number(value());
    else if (arg === "--host") opts.host = value();
    else if (arg === "--dir") opts.dir = resolve(value());
    else if (arg === "--models") opts.models = resolve(value());
    else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: node registry/local/server.mjs [--port 8080] [--host 0.0.0.0] [--dir dist/web] [--models <dir>]\n\n" +
          "Serves the browser build and a local registry for offline / LAN play.\n" +
          "  --dir     the browser build to serve (default: dist/web, from ./build_web.sh)\n" +
          "  --models  where /models/* is read from when the build does not bundle them\n" +
          "            (default: <dir>/assets/models, then client/assets/models)",
      );
      process.exit(0);
    } else fail(`unknown option ${arg} (try --help)`);
  }
  if (!Number.isInteger(opts.port) || opts.port < 1 || opts.port > 65535) fail("--port must be 1-65535");
  return opts;
}

function fail(message) {
  console.error(`gridlock-local: ${message}`);
  process.exit(1);
}

const opts = parseArgs(process.argv.slice(2));

const SITE = existsSync(join(opts.dir, "index.html")) ? opts.dir : null;
const MODELS_DIR =
  opts.models ?? [join(opts.dir, "assets", "models"), join(REPO, "client", "assets", "models")].find((d) => existsSync(d)) ?? null;

export { opts, SITE, MODELS_DIR, fail };
