#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST="$SCRIPT_DIR/dist/mac"
APP="$DIST/Gridlock.app"

# Detect host arch; build native target by default
ARCH="$(uname -m)"
if [ "$ARCH" = "arm64" ]; then
    TARGET="aarch64-apple-darwin"
else
    TARGET="x86_64-apple-darwin"
fi

echo "==> Building client for macOS ($TARGET)..."
cd "$SCRIPT_DIR"
cargo build --release --target "$TARGET" -p client

echo "==> Building server for macOS ($TARGET)..."
cargo build --release --target "$TARGET" -p server

echo "==> Assembling .app bundle..."
rm -rf "$DIST"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp "$SCRIPT_DIR/target/$TARGET/release/client" "$APP/Contents/MacOS/Gridlock"
cp "$SCRIPT_DIR/target/$TARGET/release/server" "$APP/Contents/MacOS/server"
cp -r "$SCRIPT_DIR/client/assets" "$APP/Contents/Resources/assets"
cp -r "$SCRIPT_DIR/server/assets/." "$APP/Contents/Resources/assets/"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Gridlock</string>
    <key>CFBundleDisplayName</key>
    <string>Gridlock</string>
    <key>CFBundleIdentifier</key>
    <string>com.kwaddo.gridlock</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleExecutable</key>
    <string>Gridlock</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
</dict>
</plist>
EOF

echo "==> Building DMG..."
if command -v hdiutil &>/dev/null; then
    hdiutil create -volname "Gridlock" \
        -srcfolder "$DIST" \
        -ov -format UDZO \
        "$SCRIPT_DIR/dist/Gridlock-Mac.dmg"
    echo ""
    echo "Done."
    echo "  Installer: dist/Gridlock-Mac.dmg"
    echo "  Raw .app:  dist/mac/Gridlock.app"
else
    echo "Warning: hdiutil not found — skipping DMG. Run this script on macOS to produce a DMG."
    echo ""
    echo "Done."
    echo "  Raw .app: dist/mac/Gridlock.app"
fi
