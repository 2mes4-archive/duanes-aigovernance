#!/usr/bin/env bash
set -euo pipefail

echo "=== Duanes Cross-Compilation Script ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

TARGET="${1:-}"
if [ -z "$TARGET" ]; then
    echo "Usage: $0 <target>"
    echo ""
    echo "Available targets:"
    echo "  x86_64-unknown-linux-gnu"
    echo "  aarch64-unknown-linux-gnu"
    echo "  x86_64-apple-darwin"
    echo "  aarch64-apple-darwin"
    echo "  x86_64-pc-windows-msvc"
    echo "  aarch64-linux-android"
    echo "  x86_64-linux-android"
    echo "  aarch64-apple-ios"
    echo "  wasm32-unknown-unknown"
    exit 1
fi

echo "Target: ${TARGET}"

echo "Installing target..."
rustup target add "$TARGET" 2>/dev/null || true

echo "Building for ${TARGET}..."
cargo build --release --bin duanes-server --target "$TARGET"

echo ""
echo "=== Cross-compilation complete ==="
echo "Output: target/${TARGET}/release/duanes-server"
