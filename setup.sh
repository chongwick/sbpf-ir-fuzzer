#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "[setup] Building sbpf-ir tools (release)..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

echo "[setup] Building ir-jit-diff fuzz target..."
cd "$REPO_ROOT"
cargo +nightly fuzz build ir-jit-diff

echo "[setup] Generating initial IR corpus..."
mkdir -p "$REPO_ROOT/fuzz/ir-corpus"
cargo run --manifest-path "$SCRIPT_DIR/Cargo.toml" --release -- \
    --gen-smart "$REPO_ROOT/fuzz/ir-corpus"

echo "[setup] Symlinking fuzz_loop.py to repo root..."
ln -sf tools/fuzz_loop.py "$REPO_ROOT/fuzz_loop.py"

echo ""
echo "[setup] Done! To run the fuzzer:"
echo "  cd $REPO_ROOT"
echo "  python3 fuzz_loop.py"
