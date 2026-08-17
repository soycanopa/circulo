#!/usr/bin/env bash
# Run Circulo from the workspace target (same UI + daemon the repo builds).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/target"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

cd "$ROOT"
cargo build -q -p circulo-daemon -p circulo-app

pkill -f 'circulo-daemon' 2>/dev/null || true
pkill -f 'circulo-app' 2>/dev/null || true
sleep 0.3

"$ROOT/target/debug/circulo-daemon" &
sleep 0.8
exec "$ROOT/target/debug/circulo-app"
