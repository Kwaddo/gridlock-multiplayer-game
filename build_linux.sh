#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST="$SCRIPT_DIR/dist/linux"

echo "==> Building client for Linux..."
cd "$SCRIPT_DIR"
cargo build --release --target x86_64-unknown-linux-gnu -p client

echo "==> Building server for Linux..."
cargo build --release --target x86_64-unknown-linux-gnu -p server

echo "==> Assembling distribution package..."
rm -rf "$DIST"
mkdir -p "$DIST"

cp "$SCRIPT_DIR/target/x86_64-unknown-linux-gnu/release/client" "$DIST/"
cp "$SCRIPT_DIR/target/x86_64-unknown-linux-gnu/release/server" "$DIST/"
cp -r "$SCRIPT_DIR/client/assets" "$DIST/assets"
cp -r "$SCRIPT_DIR/server/assets/." "$DIST/assets/"

echo "==> Bundling installer..."
cp "$SCRIPT_DIR/install_linux.sh" "$DIST/install.sh"
chmod +x "$DIST/install.sh"

cd "$SCRIPT_DIR/dist"
tar -czf "Gridlock-Linux.tar.gz" linux/

echo ""
echo "Done."
echo "  Installer: dist/Gridlock-Linux.tar.gz"
echo "  Raw files: dist/linux/"
