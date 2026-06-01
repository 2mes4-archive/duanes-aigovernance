#!/usr/bin/env bash
set -euo pipefail

echo "=== Duanes Build Script ==="
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

TARGET="${1:-release}"
BUILD_DIR="${ROOT_DIR}/build"

echo "Target: ${TARGET}"
echo "Build dir: ${BUILD_DIR}"
echo ""

mkdir -p "${BUILD_DIR}"

echo "[1/5] Building duanes-server..."
cargo build --$TARGET --bin duanes-server
echo "  -> OK"

echo "[2/5] Building duanes-collector..."
cargo build --$TARGET --bin duanes-collector
echo "  -> OK"

echo "[3/5] Building C bindings..."
cargo build --$TARGET -p duanes-c
echo "  -> OK"

echo "[4/5] Copying binaries..."
if [ "$TARGET" = "release" ]; then
    cp target/release/duanes-server "${BUILD_DIR}/"
    cp target/release/duanes-collector "${BUILD_DIR}/"
    cp target/release/libduanes_c.a "${BUILD_DIR}/" 2>/dev/null || true
    cp target/release/libduanes_c.so "${BUILD_DIR}/" 2>/dev/null || true
else
    cp target/debug/duanes-server "${BUILD_DIR}/"
    cp target/debug/duanes-collector "${BUILD_DIR}/"
fi
echo "  -> OK"

echo "[5/5] Copying config..."
cp -r config "${BUILD_DIR}/"
echo "  -> OK"

echo ""
echo "=== Build complete ==="
echo "Output: ${BUILD_DIR}/"
ls -la "${BUILD_DIR}/"
