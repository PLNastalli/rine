#!/bin/bash
# Compila os fontes de teste Windows (MinGW). Binários NÃO vão para o git
# (vão para $OUT, default /tmp/rine-win). O probe RODA SÓ no Windows real.
set -e
OUT="${OUT:-/tmp/rine-win}"
mkdir -p "$OUT"
CC="${CC:-x86_64-w64-mingw32-gcc}"
SRC_DIR="$(dirname "$0")"
$CC -O2 -o "$OUT/oracle_probe.exe" "$SRC_DIR/oracle_probe.c"
$CC -O2 -o "$OUT/hello_mingw.exe" "$SRC_DIR/hello_mingw.c"
ls -la "$OUT"
