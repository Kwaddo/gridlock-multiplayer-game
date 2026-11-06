#!/usr/bin/env bash
# Play Gridlock in the browser with no internet: serves the browser build and
# a local registry from this machine (registry/local/server.mjs). Everyone on
# the same network opens the address it prints and can host, browse and join.
#
#   ./play_local.sh [--rebuild] [--port 8080]
#
# Builds dist/web first if there is no build yet (or with --rebuild). Needs
# Node.js 18 or newer.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v node >/dev/null 2>&1; then
    echo "Node.js 18 or newer is needed: https://nodejs.org" >&2
    exit 1
fi
if [ "$(node -p 'process.versions.node.split(".")[0]')" -lt 18 ]; then
    echo "Node.js 18 or newer is needed (found $(node --version))." >&2
    exit 1
fi

REBUILD=0
ARGS=()
for arg in "$@"; do
    if [ "$arg" = "--rebuild" ]; then REBUILD=1; else ARGS+=("$arg"); fi
done

# The models must be next to the page offline: build without GRIDLOCK_MODELS_URL.
if [ "$REBUILD" = 1 ] || [ ! -f "$SCRIPT_DIR/dist/web/gridlock.wasm" ]; then
    GRIDLOCK_MODELS_URL= "$SCRIPT_DIR/build_web.sh"
fi

exec node "$SCRIPT_DIR/registry/local/server.mjs" --dir "$SCRIPT_DIR/dist/web" "${ARGS[@]}"
