// The browser build and the models, served as static files
// (registry/local/server.mjs).

import { createReadStream, readFileSync, statSync } from "node:fs";
import { extname, join, normalize, sep } from "node:path";

import { TYPES } from "./config.mjs";
import { MODELS_DIR, SITE } from "./options.mjs";

// ---------------------------------------------------------------- static files

// The page, told to use this server for everything online. Read on every
// request so a rebuild (./build_web.sh) shows up on reload.
function indexHtml() {
  const html = readFileSync(join(SITE, "index.html"), "utf8");
  // The first of the plugin's scripts: gridlock_web/models.js reads
  // GRIDLOCK_MODELS_URL as it loads, so the override must come before it.
  const marker = '<script src="gridlock_web/constants.js"></script>';
  const inject =
    "<script>\n" +
    "        // gridlock local server: registry and models from this machine (registry/local/server.mjs)\n" +
    "        window.GRIDLOCK_REGISTRY = location.origin;\n" +
    '        if (window.GRIDLOCK_MODELS_URL) window.GRIDLOCK_MODELS_URL = location.origin + "/models";\n' +
    "    </script>\n    ";
  return html.includes(marker) ? html.replace(marker, inject + marker) : null;
}

// Path under `root` for a URL path, or null if it would leave `root`.
function within(root, urlPath) {
  let decoded;
  try {
    decoded = decodeURIComponent(urlPath);
  } catch {
    return null;
  }
  if (decoded.includes("\0")) return null;
  const file = normalize(join(root, decoded));
  return file === root || file.startsWith(root + sep) ? file : null;
}

function sendFile(req, res, file, cache) {
  let stat;
  try {
    stat = statSync(file);
  } catch {
    return false;
  }
  if (!stat.isFile()) return false;
  const headers = {
    "Content-Type": TYPES[extname(file).toLowerCase()] || "application/octet-stream",
    "Content-Length": stat.size,
    "Cache-Control": cache,
  };
  res.writeHead(200, headers);
  if (req.method === "HEAD") res.end();
  else createReadStream(file).pipe(res);
  return true;
}

function serveStatic(req, res, pathname) {
  if (req.method !== "GET" && req.method !== "HEAD") return false;

  if (pathname.startsWith("/models/") && MODELS_DIR) {
    // The page asks for /models/<path>?v=<version>; the query is ignored here.
    const file = within(MODELS_DIR, pathname.slice("/models".length));
    return file ? sendFile(req, res, file, "public, max-age=86400") : false;
  }
  if (!SITE) return false;

  if (pathname === "/" || pathname === "/index.html") {
    const html = indexHtml();
    if (html === null) {
      res.writeHead(500, { "Content-Type": "text/plain" });
      res.end("index.html has no gridlock_web/constants.js script tag; is this a Gridlock browser build?");
      return true;
    }
    res.writeHead(200, { "Content-Type": TYPES[".html"], "Cache-Control": "no-cache" });
    res.end(req.method === "HEAD" ? undefined : html);
    return true;
  }
  const file = within(SITE, pathname);
  // no-cache: the browser revalidates, so a rebuilt wasm is picked up on reload.
  return file ? sendFile(req, res, file, "no-cache") : false;
}

export { serveStatic };
