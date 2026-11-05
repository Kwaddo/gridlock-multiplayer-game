#!/usr/bin/env bash
# Builds the browser version into dist/web, ready to serve as static files
# (locally with --serve, or on Dilmune; see web/README.md).
#
#   [GRIDLOCK_REGISTRY=<registry URL>] [GRIDLOCK_MODELS_URL=<registry URL>/models] ./build_web.sh [--serve]
#
# With GRIDLOCK_MODELS_URL the models stay out of dist/web (upload_assets.sh
# puts them where that URL serves them from). GRIDLOCK_MODELS_VERSION names the
# models' current version; without it, it is hashed from client/assets/models.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST="$SCRIPT_DIR/dist/web"
TARGET="wasm32-unknown-unknown"

# GRIDLOCK_REGISTRY defaults to the live registry (.cargo/config.toml).

rustup target add "$TARGET" >/dev/null 2>&1 || true

echo "==> Building client for the browser ($TARGET)..."
cd "$SCRIPT_DIR"
cargo build --release --target "$TARGET" -p client

echo "==> Assembling $DIST..."
mkdir -p "$DIST"
cp "$SCRIPT_DIR/target/$TARGET/release/client.wasm" "$DIST/gridlock.wasm"
cp "$SCRIPT_DIR/web/index.html" "$SCRIPT_DIR/web/mq_js_bundle.js" "$SCRIPT_DIR/web/gridlock_web.js" \
    "$SCRIPT_DIR/web/favicon.png" "$SCRIPT_DIR/web/apple-touch-icon.png" "$DIST/"
# Only what the game loads at run time. --delete keeps removed assets from lingering.
if [ -n "$GRIDLOCK_MODELS_URL" ]; then
    # Models are hosted separately (they exceed most static hosts' size limits),
    # and the page caches each model in the browser after its first download.
    rsync -a --delete --exclude '.DS_Store' --exclude '/models/' "$SCRIPT_DIR/client/assets/" "$DIST/assets/"
    rm -rf "$DIST/assets/models" # left over from a build that bundled them
    # Changes whenever a model changes, so browsers drop their cached copies.
    VERSION="${GRIDLOCK_MODELS_VERSION:-$(cd "$SCRIPT_DIR/client/assets/models" && find . -type f ! -name .DS_Store -print0 | sort -z | xargs -0 shasum | shasum | cut -c1-12)}"
    sed -i.bak "s|GRIDLOCK_MODELS_URL = \"\"|GRIDLOCK_MODELS_URL = \"$GRIDLOCK_MODELS_URL\"|; s|GRIDLOCK_MODELS_VERSION = \"\"|GRIDLOCK_MODELS_VERSION = \"$VERSION\"|" "$DIST/index.html"
    rm "$DIST/index.html.bak"
    echo "==> Models: served from $GRIDLOCK_MODELS_URL, version $VERSION"
else
    rsync -a --delete --exclude '.DS_Store' "$SCRIPT_DIR/client/assets/" "$DIST/assets/"
fi

echo "==> Done: $(du -sh "$DIST" | cut -f1) in $DIST"

if [ "$1" = "--serve" ]; then
    echo "==> Serving on http://localhost:8080 (Ctrl+C to stop)"
    cd "$DIST" && python3 -m http.server 8080
fi
