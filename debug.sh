#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$ROOT_DIR/target/debug/debug_packets"

cd "$ROOT_DIR"

echo "Building debug_packets..." >&2
cargo build --bin debug_packets

if ! command -v setcap >/dev/null 2>&1; then
  echo "error: setcap is required to grant packet capture permissions" >&2
  echo "install libcap tools, or run the binary with elevated privileges instead" >&2
  exit 1
fi

echo "Granting packet capture capabilities to $BIN..." >&2
sudo setcap cap_net_raw,cap_net_admin=eip "$BIN"

echo "Running debug_packets $*" >&2
exec "$BIN" --event-exclude 3,20,21,19,141,74,45,91,14,6,22,11,267,275,160,211,29,96,212,545,270,1,113,18,8,92 "$@"
