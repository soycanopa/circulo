#!/usr/bin/env bash
# Build Circulo.app on ~/Desktop for manual testing (release binaries + bundled daemon).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/target"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

APP_NAME="Circulo"
DEST="${1:-$HOME/Desktop/${APP_NAME}.app}"
MACOS="$DEST/Contents/MacOS"
RESOURCES="$DEST/Contents/Resources"

echo "Building release binaries..."
cd "$ROOT"
cargo build --release -q -p circulo-daemon -p circulo-app

echo "Packaging $DEST ..."
rm -rf "$DEST"
mkdir -p "$MACOS" "$RESOURCES"

cp "$ROOT/target/release/circulo-app" "$MACOS/circulo-app"
cp "$ROOT/target/release/circulo-daemon" "$MACOS/circulo-daemon"
chmod +x "$MACOS/circulo-app" "$MACOS/circulo-daemon"

VERSION="$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')"

cat >"$DEST/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>circulo-app</string>
  <key>CFBundleIdentifier</key>
  <string>dev.circulo.app</string>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

echo "Done: $DEST"
echo "OpenCode adapter is default; ensure opencode is installed for real turns."
echo "Optional: CIRCULO_ADAPTER=fake for UI-only smoke tests."
