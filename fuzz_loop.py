#!/usr/bin/env python3
"""Autonomous fuzzing loop with plateau detection for ir-jit-diff.

On coverage plateau: generates fresh seeds (smart or JIT-stress) into the
corpus directory. The fuzz target periodically reloads its corpus from disk
(every 10s), so new seeds are picked up without restarting the fuzzer or
losing coverage.

Run from the repo root:  python3 fuzz_loop.py
"""

import argparse
import os
import random
import re
import signal
import subprocess
import sys
import time

# Resolve repo root: fuzz_loop.py lives in tools/, symlinked to repo root.
# Either way, repo root is where fuzz/ and tools/ directories exist.
SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR) if os.path.basename(SCRIPT_DIR) == "tools" else SCRIPT_DIR


def log(msg):
    print(f"[fuzz_loop] {msg}", flush=True)


def generate_smart_seeds(args, rng_seed):
    """Generate smart IR seeds via sbpf-ir --gen-smart."""
    cmd = [
        "cargo", "run", "--manifest-path", os.path.join(REPO_ROOT, "tools/Cargo.toml"),
        "--release", "--",
        "--gen-smart", args.corpus_dir,
        "--seed", str(rng_seed),
        "--count", str(args.smart_count),
    ]
    log(f"Generating {args.smart_count} smart seeds (seed={rng_seed})...")
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT)
    if result.returncode != 0:
        log(f"WARNING: smart seed generation failed (exit {result.returncode})")
        if result.stderr:
            for line in result.stderr.strip().splitlines()[-5:]:
                log(f"  {line}")
    else:
        for line in result.stdout.strip().splitlines():
            log(line)


def generate_jit_stress_seeds(args, rng_seed):
    """Generate JIT-stress IR seeds via sbpf-ir --gen-jit-stress."""
    cmd = [
        "cargo", "run", "--manifest-path", os.path.join(REPO_ROOT, "tools/Cargo.toml"),
        "--release", "--",
        "--gen-jit-stress", args.corpus_dir,
        "--seed", str(rng_seed),
        "--count", str(args.jit_stress_count),
    ]
    log(f"Generating {args.jit_stress_count} JIT-stress seeds (seed={rng_seed})...")
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT)
    if result.returncode != 0:
        log(f"WARNING: JIT-stress seed generation failed (exit {result.returncode})")
        if result.stderr:
            for line in result.stderr.strip().splitlines()[-5:]:
                log(f"  {line}")
    else:
        for line in result.stdout.strip().splitlines():
            log(line)


def run_fuzzer(args, initial_seed):
    """Run the fuzzer, refreshing seeds on plateau. Returns (final_cov, refresh_count)."""
    # Initial seed generation: both smart + JIT-stress
    generate_smart_seeds(args, initial_seed)
    generate_jit_stress_seeds(args, initial_seed ^ 0xdeadbeef)

    cmd = ["cargo", "+nightly", "fuzz", "run", "ir-jit-diff"]
    if args.max_cycle_secs > 0:
        cmd += ["--", f"-max_total_time={args.max_cycle_secs}"]
    log("Starting fuzzer...")

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
        cwd=REPO_ROOT,
    )

    last_cov = 0
    last_cov_time = time.time()
    refreshes = 0
    cov_pattern = re.compile(r"cov:\s+(\d+)")

    try:
        for line in proc.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()

            m = cov_pattern.search(line)
            if m:
                cov = int(m.group(1))
                if cov > last_cov:
                    last_cov = cov
                    last_cov_time = time.time()
                elif time.time() - last_cov_time > args.plateau_secs:
                    # Plateau detected — generate new seeds while fuzzer
                    # keeps running. The fuzz target hot-reloads its corpus
                    # from disk every 10s, so it will pick these up.
                    refreshes += 1
                    rng_seed = random.randrange(2**64)
                    # Every 3rd refresh is JIT-stress-only, others are smart-only
                    if refreshes % 3 == 0:
                        log(f"Plateau at cov={last_cov}, refreshing JIT-stress seeds (#{refreshes})...")
                        generate_jit_stress_seeds(args, rng_seed)
                    else:
                        log(f"Plateau at cov={last_cov}, refreshing smart seeds (#{refreshes})...")
                        generate_smart_seeds(args, rng_seed)
                    last_cov_time = time.time()
                    log(f"New seeds written, fuzzer will reload within ~10s")
    except KeyboardInterrupt:
        proc.terminate()
        proc.wait()
        raise

    proc.wait()
    return last_cov, refreshes


def main():
    parser = argparse.ArgumentParser(
        description="Autonomous fuzzing loop with plateau detection for ir-jit-diff")
    parser.add_argument("--corpus-dir", default=os.path.join(REPO_ROOT, "fuzz/ir-corpus"),
                        help="IR corpus directory (default: fuzz/ir-corpus)")
    parser.add_argument("--smart-count", type=int, default=1000,
                        help="Number of smart seeds per cycle (default: 1000)")
    parser.add_argument("--jit-stress-count", type=int, default=300,
                        help="Number of JIT-stress seeds per cycle (default: 300)")
    parser.add_argument("--plateau-secs", type=int, default=120,
                        help="Seconds without cov increase to declare plateau (default: 120)")
    parser.add_argument("--max-cycle-secs", type=int, default=0,
                        help="Max seconds per fuzzer run, 0=unlimited (default: 0)")
    args = parser.parse_args()

    log(f"Repo root: {REPO_ROOT}")
    log(f"Config: corpus_dir={args.corpus_dir}, smart_count={args.smart_count}, "
        f"jit_stress_count={args.jit_stress_count}, plateau_secs={args.plateau_secs}, "
        f"max_cycle_secs={'unlimited' if args.max_cycle_secs == 0 else args.max_cycle_secs}")

    rng_seed = random.randrange(2**64)
    log(f"=== Starting (seed={rng_seed}) ===")
    start = time.time()

    try:
        final_cov, refreshes = run_fuzzer(args, rng_seed)
    except KeyboardInterrupt:
        elapsed = time.time() - start
        log(f"Interrupted: elapsed={elapsed:.0f}s")
        sys.exit(0)

    elapsed = time.time() - start
    log(f"Done: cov={final_cov}, refreshes={refreshes}, elapsed={elapsed:.0f}s")


if __name__ == "__main__":
    main()
