#!/bin/bash
# Empacota o sidecar `rine` para o bundle do Manager.
# Uso: crates/manager-tauri/pack-sidecar.sh [--debug]
# Gera: crates/manager-tauri/binaries/rine-<target-triple>
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MODE="release"
if [ "${1:-}" = "--debug" ]; then
  MODE="debug"
fi

TRIPLE="$(rustc -vV | grep '^host:' | cut -d' ' -f2)"
if [ -z "$TRIPLE" ]; then
  echo "pack-sidecar: não foi possível detectar o target triple" >&2
  exit 2
fi

echo "pack-sidecar: building launcher ($MODE, $TRIPLE)..."
if [ "$MODE" = "release" ]; then
  cargo build --release -p launcher --manifest-path "$ROOT/Cargo.toml"
  SRC="$ROOT/target/release/rine"
else
  cargo build -p launcher --manifest-path "$ROOT/Cargo.toml"
  SRC="$ROOT/target/debug/rine"
fi

if [ ! -x "$SRC" ]; then
  echo "pack-sidecar: binário ausente após build: $SRC" >&2
  exit 2
fi

DEST="$ROOT/crates/manager-tauri/binaries/rine-$TRIPLE"
cp "$SRC" "$DEST"
chmod +x "$DEST"
"$DEST" --version
echo "pack-sidecar: sidecar pronto: $DEST"
