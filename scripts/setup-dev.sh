#!/usr/bin/env bash
set -euo pipefail

echo "=== Duanes Development Environment Setup ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

echo "[1/6] Installing Rust toolchain..."
rustup show active-toolchain || rustup install 1.85.0
rustup component add rustfmt clippy
echo "  -> OK"

echo "[2/6] Installing targets..."
TARGETS=(
    x86_64-unknown-linux-gnu
    aarch64-unknown-linux-gnu
    x86_64-apple-darwin
    aarch64-apple-darwin
    x86_64-pc-windows-msvc
    wasm32-unknown-unknown
)
for target in "${TARGETS[@]}"; do
    echo "  Adding ${target}..."
    rustup target add "$target" 2>/dev/null || true
done
echo "  -> OK"

echo "[3/6] Installing cargo tools..."
cargo install cargo-audit 2>/dev/null || true
cargo install cargo-deny 2>/dev/null || true
cargo install wasm-pack 2>/dev/null || true
echo "  -> OK"

echo "[4/6] Setting up dashboard..."
if command -v npm &>/dev/null; then
    cd services/dashboard && npm install && cd "$ROOT_DIR"
    echo "  -> OK"
else
    echo "  -> SKIP (npm not found)"
fi

echo "[5/6] Setting up TypeScript SDK..."
if command -v npm &>/dev/null; then
    cd sdks/typescript && npm install && cd "$ROOT_DIR"
    echo "  -> OK"
else
    echo "  -> SKIP (npm not found)"
fi

echo "[6/6] Setting up Python SDK..."
if command -v python3 &>/dev/null; then
    cd sdks/python && python3 -m pip install -e ".[dev]" 2>/dev/null && cd "$ROOT_DIR"
    echo "  -> OK"
else
    echo "  -> SKIP (python3 not found)"
fi

echo ""
echo "=== Setup complete ==="
echo "Run 'make build' to compile the project."
