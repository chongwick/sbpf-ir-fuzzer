#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FUZZ_CARGO="$REPO_ROOT/fuzz/Cargo.toml"

# Ensure ir-jit-diff bin entry exists in fuzz/Cargo.toml
if ! grep -q 'name = "ir-jit-diff"' "$FUZZ_CARGO"; then
    echo "[setup] Adding ir-jit-diff target to fuzz/Cargo.toml..."
    cat >> "$FUZZ_CARGO" << 'EOF'

[[bin]]
name = "ir-jit-diff"
path = "fuzz_targets/ir_jit_diff.rs"
test = false
doc = false
EOF
fi

# Ensure sbpf-ir dependency exists in fuzz/Cargo.toml
if ! grep -q 'sbpf-ir' "$FUZZ_CARGO"; then
    echo "[setup] Adding sbpf-ir dependency to fuzz/Cargo.toml..."
    sed -i.bak '/\[dependencies\]/a\
sbpf-ir = { path = "../tools/" }
' "$FUZZ_CARGO"
    rm -f "$FUZZ_CARGO.bak"
fi

# Ensure rand dependency exists in fuzz/Cargo.toml
if ! grep -q '^rand' "$FUZZ_CARGO" && ! grep -q '^rand ' "$FUZZ_CARGO"; then
    if ! grep -q 'rand = ' "$FUZZ_CARGO"; then
        echo "[setup] Adding rand dependency to fuzz/Cargo.toml..."
        sed -i.bak '/\[dependencies\]/a\
rand = "0.8"
' "$FUZZ_CARGO"
        rm -f "$FUZZ_CARGO.bak"
    fi
fi

# Ensure bincode dependency exists in fuzz/Cargo.toml
if ! grep -q 'bincode' "$FUZZ_CARGO"; then
    echo "[setup] Adding bincode dependency to fuzz/Cargo.toml..."
    sed -i.bak '/\[dependencies\]/a\
bincode = "1"
' "$FUZZ_CARGO"
    rm -f "$FUZZ_CARGO.bak"
fi

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
