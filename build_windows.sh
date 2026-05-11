#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST="$SCRIPT_DIR/dist/windows"

echo "==> Building client for Windows..."
cd "$SCRIPT_DIR/client"
cargo build --release --target x86_64-pc-windows-gnu

echo "==> Building server for Windows..."
cd "$SCRIPT_DIR/server"
cargo build --release --target x86_64-pc-windows-gnu

echo "==> Assembling distribution package..."
rm -rf "$DIST"
mkdir -p "$DIST"

cp "$SCRIPT_DIR/client/target/x86_64-pc-windows-gnu/release/client.exe" "$DIST/"
cp -r "$SCRIPT_DIR/client/assets" "$DIST/assets"
cp "$SCRIPT_DIR/server/target/x86_64-pc-windows-gnu/release/server.exe" "$DIST/"
cp -r "$SCRIPT_DIR/server/assets" "$DIST/server_assets"

echo ""
echo "Done. Distribution package at: dist/windows/"
echo "  client.exe"
echo "  assets/"
echo "  server.exe"
echo "  server_assets/"
