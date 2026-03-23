# sbpf-ir

IR (intermediate representation) toolkit for [solana-sbpf](https://github.com/anza-xyz/sbpf). Provides assembly-to-IR translation, structured mutation, differential testing, and corpus generation for fuzzing the SBPF interpreter and JIT compiler.

## Architecture

```
                     ┌─────────────┐
  assembly text ───► │   sbpf2ir   │ ───► IrSeq (regions of IrNodes)
                     └─────────────┘             │
                            ▲                    ▼
                            │            ┌──────────────┐
                    gen_pqr / gen_corpus  │   mutator    │ splice-based mutation
                            │            └──────┬───────┘
                            ▼                   ▼
                     ┌─────────────┐     ┌─────────────┐
                     │  .ir files  │ ──► │  executor   │ interpreter/JIT diff
                     │  (bincode)  │     └─────────────┘
                     └─────────────┘
```

### Modules

| Module | Purpose |
|---|---|
| `ir` | `IrNode` and `IrSeq` types. `sbpf2ir()` translates SBPF assembly into a region-based IR, splitting on branches, jumps, and exits. |
| `mutator` | Splice-based mutation: extracts instructions or entire regions from donor IRs and inserts them into a base IR. Used by the `ir-jit-diff` fuzz target. |
| `executor` | Assembles IR back to bytecode, runs interpreter and JIT, compares results (return value, instruction count, memory). Panics on differential mismatches. |
| `gen_pqr` | Generates IR corpus for PQR (product/quotient/remainder) instructions from `tests/execution.rs` test cases. Covers V0 div/mod/mul and V2 udiv/urem/sdiv/srem/lmul/shmul/uhmul. |
| `gen_corpus` | Generates IR corpus for all remaining non-PQR inline-assembly tests from `tests/execution.rs`: ALU, logic, shifts, byte swaps, memory ops, control flow, stack/calls, instruction meter, programs (prime, subnet, TCP port 80, TCP SACK), and V0-specific instructions. |
| `gen_smart` | Generates semantic-aware IR seeds using `arbitrary`-driven `FuzzProgram` construction. Produces verified, disassembled programs across all SBPF versions. |
| `semantic_aware` | `FuzzProgram` and `make_program()` — structured instruction generation that produces valid SBPF programs via `arbitrary::Arbitrary`. |

### IR representation

An `IrSeq` contains:
- **version** -- SBPF version string (`"V0"`, `"V2"`, `"V3"`, `"V4"`)
- **memory** -- input memory bytes
- **regions** -- ordered list of `(region_id, Vec<IrNode>)` representing basic blocks

Each `IrNode` has an `op` (instruction mnemonic) and `operands` (registers, immediates, offsets).

Files are serialized with [bincode](https://crates.io/crates/bincode) (`.ir` extension).

## Building

```bash
cargo build --manifest-path tools/Cargo.toml
```

Requires the parent `solana-sbpf` crate and `test_utils` to be present (paths are relative in `Cargo.toml`).

## CLI usage

```
sbpf-ir                                     Demo mode (built-in examples)
sbpf-ir <input.json> [-o <out.ir>]           Translate JSON to IR
sbpf-ir --load <file.ir>                     Load and print saved IR
sbpf-ir --mutate <f1> <f2> ... [-o out.ir]   Mutate k IRs (.ir or .json)
sbpf-ir --seed <N>                           Set RNG seed (with --mutate)
sbpf-ir --exec <prog.json>                   Run interpreter/JIT diff test
sbpf-ir --triage <file.ir>                   Triage: assemble, disassemble, verify, execute
sbpf-ir --gen-pqr [output_dir]               Generate PQR IR corpus
sbpf-ir --gen-corpus [output_dir]            Generate full IR corpus (PQR + general)
sbpf-ir --gen-smart [output_dir]             Generate smart IR seeds (default: 1000)
    [--count N] [--seed N]                   Optional count and RNG seed
```

### Examples

Generate the full seed corpus:

```bash
cargo run --manifest-path tools/Cargo.toml -- --gen-corpus tools/input_corpus
```

Triage a specific corpus file:

```bash
cargo run --manifest-path tools/Cargo.toml -- --triage tools/input_corpus/081_mov32_imm_1.ir
```

Mutate two corpus files into a new IR:

```bash
cargo run --manifest-path tools/Cargo.toml -- --mutate tools/input_corpus/081_mov32_imm_1.ir tools/input_corpus/100_ldxb.ir -o mutated.ir
```

Translate a JSON test case to IR:

```bash
echo '{"version":"V0","memory":[],"asm":"mov r0, 42\nexit"}' > test.json
cargo run --manifest-path tools/Cargo.toml -- test.json -o test.ir
```

### JSON input format

For `sbpf-ir <input.json>`:
```json
{
  "version": "V0",
  "memory": [0, 1, 2, 3],
  "asm": "add64 r10, 0\nldxb r0, [r1]\nexit"
}
```

For `sbpf-ir --exec <prog.json>`:
```json
{
  "version": "V0",
  "memory": [],
  "prog": [/* raw bytecode bytes */]
}
```

## Corpus generation

`--gen-corpus` produces seed files for the `ir-jit-diff` fuzz target by converting inline-assembly tests from `tests/execution.rs` into serialized IR files.

**What's included (~313 files):**

| Category | Count | Description |
|---|---|---|
| PQR (V0) | ~42 | div/mod/mul with imm and reg variants |
| PQR (V2) | ~84 | udiv/urem/sdiv/srem/lmul/shmul/uhmul |
| PQR errors | ~16 | divide-by-zero, divide-overflow |
| ALU | 12 | mov32/64, bounce, add/sub, lmul128 |
| Logic | 2 | alu32_logic, alu64_logic |
| Shifts | 7 | arsh32/64, lsh64, rsh32/64 |
| Byte swaps | 5 | be16/32/64 |
| Memory (V0+V4) | 32 | ldx/st/stx for b/h/w/dw, each version |
| HOR64 (V2) | 1 | hor64 instruction |
| LDX/STX variants | 10 | same_reg, oob, nomem, all-register loads/stores, chains |
| Exits/Jumps | 9 | exit capped/without_value/early, ja |
| Stack/Calls | 12 | stack1, entrypoint_exit, call depth, scratch regs, callx |
| Instruction meter | 11 | infinite loops, recursion, capped tests |
| Far jumps | 1 | .fill 1024 with callx |
| Programs | 8 | lmul_loop, prime, subnet, TCP port 80, TCP SACK |
| Callx/Other | 3 | callx_unsupported, capped_after_callx |
| V0-specific | 58 | lddw, le, neg, callx_imm, mul, div, mod, stack_gaps |

**What's excluded:** syscall tests, ELF tests, raw bytecode tests, random generation (`test_total_chaos`), V1-only tests, parametric tests with config-dependent memory.

## Integration with fuzzing

The `ir-jit-diff` fuzz target in `fuzz/fuzz_targets/` uses this crate:

1. libfuzzer provides a `ConfigTemplate` and random seed
2. The seed selects and mutates corpus IR files via `mutator::mutate()`
3. The mutated IR is assembled back to bytecode
4. Interpreter and JIT execute the program
5. Results are compared; mismatches trigger a panic (crash = bug found)

### Quick start

```bash
cd tools
./setup.sh
cd ..
python3 fuzz_loop.py
```

`setup.sh` builds the tools crate, builds the fuzz target, generates an initial IR corpus, and symlinks `fuzz_loop.py` to the repo root.

### Autonomous fuzzing loop

`fuzz_loop.py` runs the fuzzer continuously, monitoring coverage and refreshing seeds on plateau:

```
┌──────────────────┐     ┌───────────────────────────────────────────┐
│  Generate smart  │────►│  Run fuzzer, stream output, track cov:   │
│  seeds (sbpf-ir) │     │                                           │
└──────────────────┘     │  On plateau: generate new seeds in-place  │◄─┐
                         │  (fuzzer hot-reloads corpus every ~10s)   │  │
                         └───────────────────────────────────────────┘  │
                                          │ plateau                     │
                                          └─────────────────────────────┘
```

The `ir-jit-diff` target hot-reloads `fuzz/ir-corpus/*.ir` from disk every 10 seconds. When `fuzz_loop.py` detects a coverage plateau, it writes new seeds into the corpus directory while the fuzzer keeps running — no restart, no lost coverage.

**Options:**

| Flag | Default | Description |
|---|---|---|
| `--corpus-dir` | `fuzz/ir-corpus` | IR corpus directory |
| `--smart-count` | 1000 | Smart seeds generated per refresh |
| `--plateau-secs` | 120 | Seconds without coverage increase to trigger seed refresh |
| `--max-cycle-secs` | 0 (unlimited) | Max seconds before fuzzer exits |

**Examples:**

```bash
# Quick test: short plateau window, small seed count
python3 fuzz_loop.py --plateau-secs 30 --max-cycle-secs 120 --smart-count 100

# Full autonomous run (Ctrl+C to stop)
python3 fuzz_loop.py
```

### Manual fuzzing

```bash
# Generate seed corpus
cargo run --manifest-path tools/Cargo.toml -- --gen-corpus fuzz/ir-corpus

# Run the fuzz target directly
cargo +nightly fuzz run ir-jit-diff
```

## License

Same as the parent solana-sbpf crate: Apache-2.0 / MIT dual licensed.
