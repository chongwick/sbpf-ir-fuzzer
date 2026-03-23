#!/usr/bin/env python3
"""Autonomous fuzzing loop with plateau detection for ir-jit-diff.

On coverage plateau: pauses the fuzzer (SIGSTOP), generates fresh smart seeds,
then resumes (SIGCONT). The fuzz target periodically reloads its corpus from
disk, so new seeds are picked up without losing accumulated coverage.

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


def generate_seeds(args, rng_seed):
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
        log(f"WARNING: seed generation failed (exit {result.returncode})")
        if result.stderr:
            for line in result.stderr.strip().splitlines()[-5:]:
                log(f"  {line}")
    else:
        for line in result.stdout.strip().splitlines():
            log(line)


def run_fuzzer_cycle(args, cycle_num, initial_seed):
    """Run one fuzzer cycle. Returns (final_cov, refresh_count)."""
    generate_seeds(args, initial_seed)

    cmd = [
        "cargo", "+nightly", "fuzz", "run", "ir-jit-diff", "--",
        f"-max_total_time={args.max_cycle_secs}",
    ]
    log("Starting fuzzer...")

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
        cwd=REPO_ROOT,
        start_new_session=True,  # own process group for SIGSTOP/SIGCONT
    )

    pgid = os.getpgid(proc.pid)
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
                    refreshes += 1
                    log(f"Plateau at cov={last_cov}, refreshing seeds (#{refreshes})...")
                    os.killpg(pgid, signal.SIGSTOP)
                    try:
                        rng_seed = random.randrange(2**64)
                        generate_seeds(args, rng_seed)
                    finally:
                        os.killpg(pgid, signal.SIGCONT)
                    last_cov_time = time.time()
                    log("Fuzzer resumed with new seeds")
    except KeyboardInterrupt:
        os.killpg(pgid, signal.SIGTERM)
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
    parser.add_argument("--plateau-secs", type=int, default=120,
                        help="Seconds without cov increase to declare plateau (default: 120)")
    parser.add_argument("--max-cycle-secs", type=int, default=600,
                        help="Max seconds per fuzzer cycle (default: 600)")
    parser.add_argument("--cycles", type=int, default=0,
                        help="Number of cycles to run, 0=unlimited (default: 0)")
    args = parser.parse_args()

    log(f"Repo root: {REPO_ROOT}")
    log(f"Config: corpus_dir={args.corpus_dir}, smart_count={args.smart_count}, "
        f"plateau_secs={args.plateau_secs}, max_cycle_secs={args.max_cycle_secs}, "
        f"cycles={'unlimited' if args.cycles == 0 else args.cycles}")

    cycle = 0
    try:
        while args.cycles == 0 or cycle < args.cycles:
            cycle += 1
            rng_seed = random.randrange(2**64)
            log(f"=== Cycle {cycle} (seed={rng_seed}) ===")
            start = time.time()

            final_cov, refreshes = run_fuzzer_cycle(args, cycle, rng_seed)

            elapsed = time.time() - start
            log(f"Cycle {cycle} complete: cov={final_cov}, "
                f"refreshes={refreshes}, elapsed={elapsed:.0f}s")
    except KeyboardInterrupt:
        log(f"Interrupted after {cycle} cycle(s)")
        sys.exit(0)

    log(f"Finished {cycle} cycle(s)")


if __name__ == "__main__":
    main()
