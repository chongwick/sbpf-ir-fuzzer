#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FUZZ_CARGO="$REPO_ROOT/fuzz/Cargo.toml"
FUZZ_TARGETS="$REPO_ROOT/fuzz/fuzz_targets"

# --- Create ir_jit_diff.rs fuzz target if missing ---
if [ ! -f "$FUZZ_TARGETS/ir_jit_diff.rs" ]; then
    echo "[setup] Creating fuzz target ir_jit_diff.rs..."
    cat > "$FUZZ_TARGETS/ir_jit_diff.rs" << 'RUSTEOF'
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use sbpf_ir::executor::{parse_sbpf_version, run_diff_ir};
use sbpf_ir::ir::IrSeq;
use sbpf_ir::mutator::mutate;

use crate::common::ConfigTemplate;

mod common;

const K: usize = 5;

static IR_CORPUS: OnceLock<Vec<IrSeq>> = OnceLock::new();

fn load_corpus() -> Vec<IrSeq> {
    let corpus_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ir-corpus");
    let mut irs = Vec::new();
    let entries = match std::fs::read_dir(&corpus_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("warning: cannot read ir-corpus at {}: {}", corpus_dir.display(), e);
            return irs;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "ir") {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(ir) = bincode::deserialize::<IrSeq>(&bytes) {
                    irs.push(ir);
                }
            }
        }
    }
    irs
}

#[derive(arbitrary::Arbitrary, Debug)]
struct FuzzData {
    template: ConfigTemplate,
    mutation_seed: u64,
}

fuzz_target!(|data: FuzzData| {
    let corpus = IR_CORPUS.get_or_init(load_corpus);
    if corpus.is_empty() {
        return;
    }

    let mut rng = StdRng::seed_from_u64(data.mutation_seed);

    // Select K IRs from the corpus
    let k = K.min(corpus.len());
    let selected: Vec<IrSeq> = (0..k)
        .map(|_| corpus[rng.gen_range(0..corpus.len())].clone())
        .collect();

    // Mutate: pick a base, splice from donors
    let mutated = mutate(&selected, &mut rng);

    let sbpf_version = match parse_sbpf_version(&mutated.version) {
        Some(v) => v,
        None => return,
    };

    // Use libfuzzer-provided config, but override version to match the IR
    let mut template = data.template;
    template.sbpf_version = sbpf_version;
    let config = template.into();

    let _ = run_diff_ir(&mutated, config, sbpf_version);
});
RUSTEOF
fi

# --- Patch fuzz/Cargo.toml as needed ---

# Ensure ir-jit-diff bin entry exists
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

# Ensure sbpf-ir dependency exists
if ! grep -q 'sbpf-ir' "$FUZZ_CARGO"; then
    echo "[setup] Adding sbpf-ir dependency to fuzz/Cargo.toml..."
    sed -i.bak '/\[dependencies\]/a\
sbpf-ir = { path = "../tools/" }
' "$FUZZ_CARGO"
    rm -f "$FUZZ_CARGO.bak"
fi

# Ensure rand dependency exists
if ! grep -q 'rand' "$FUZZ_CARGO"; then
    echo "[setup] Adding rand dependency to fuzz/Cargo.toml..."
    sed -i.bak '/\[dependencies\]/a\
rand = "0.8"
' "$FUZZ_CARGO"
    rm -f "$FUZZ_CARGO.bak"
fi

# Ensure bincode dependency exists
if ! grep -q 'bincode' "$FUZZ_CARGO"; then
    echo "[setup] Adding bincode dependency to fuzz/Cargo.toml..."
    sed -i.bak '/\[dependencies\]/a\
bincode = "1"
' "$FUZZ_CARGO"
    rm -f "$FUZZ_CARGO.bak"
fi

# --- Build ---

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
