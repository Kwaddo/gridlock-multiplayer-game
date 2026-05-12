#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST="$SCRIPT_DIR/dist/windows"

echo "==> Building client for Windows..."
cd "$SCRIPT_DIR"
cargo build --release --target x86_64-pc-windows-gnu -p client

echo "==> Building server for Windows..."
cargo build --release --target x86_64-pc-windows-gnu -p server

echo "==> Assembling distribution package..."
rm -rf "$DIST"
mkdir -p "$DIST"

cp "$SCRIPT_DIR/target/x86_64-pc-windows-gnu/release/client.exe" "$DIST/"
cp "$SCRIPT_DIR/target/x86_64-pc-windows-gnu/release/server.exe" "$DIST/"
cp -r "$SCRIPT_DIR/client/assets" "$DIST/assets"
cp -r "$SCRIPT_DIR/server/assets/." "$DIST/assets/"

echo "==> Building installer..."
cd "$SCRIPT_DIR"
if command -v makensis &>/dev/null; then
    makensis installer.nsi
    echo ""
    echo "Done."
    echo "  Installer: dist/Gridlock-Setup.exe"
    echo "  Raw files: dist/windows/"
else
    echo "Warning: makensis not found — skipping installer. Install with: sudo apt install nsis"
    echo ""
    echo "Done."
    echo "  Raw files: dist/windows/"
fi
