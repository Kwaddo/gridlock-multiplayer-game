#!/usr/bin/env bash
set -e

INSTALL_DIR="$HOME/.local/share/gridlock"
BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Installing Gridlock to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"
mkdir -p "$DESKTOP_DIR"

cp "$SCRIPT_DIR/client" "$INSTALL_DIR/gridlock"
cp "$SCRIPT_DIR/server" "$INSTALL_DIR/server"
cp -r "$SCRIPT_DIR/assets" "$INSTALL_DIR/assets"
chmod +x "$INSTALL_DIR/gridlock" "$INSTALL_DIR/server"

# Symlink client into PATH
ln -sf "$INSTALL_DIR/gridlock" "$BIN_DIR/gridlock"

# Desktop entry
cat > "$DESKTOP_DIR/gridlock.desktop" <<EOF
[Desktop Entry]
Name=Gridlock
Exec=$INSTALL_DIR/gridlock
Type=Application
Categories=Game;
EOF

# Uninstaller
cat > "$INSTALL_DIR/uninstall.sh" <<EOF
#!/usr/bin/env bash
rm -rf "$INSTALL_DIR"
rm -f "$BIN_DIR/gridlock"
rm -f "$DESKTOP_DIR/gridlock.desktop"
echo "Gridlock uninstalled."
EOF
chmod +x "$INSTALL_DIR/uninstall.sh"

echo ""
echo "Done. Run 'gridlock' to launch, or find it in your application menu."
echo "To uninstall: $INSTALL_DIR/uninstall.sh"
