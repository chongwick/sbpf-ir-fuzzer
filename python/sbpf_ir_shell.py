#!/usr/bin/env python3
"""Interactive Python interface for sbpf-tool IR + disassembly demos."""

from __future__ import annotations

import shlex
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def run_bridge(
    generator: str,
    max_lines: int = 60,
    min_regions: int = 12,
    per_region: int = 24,
    seed: int = 1337,
) -> int:
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "sbpf-tool",
        "--example",
        "ir_disasm_bridge",
        "--",
        generator,
        str(max_lines),
    ]
    if generator == "complex":
        cmd.extend([str(min_regions), str(per_region), str(seed)])

    print(f"\n$ {' '.join(shlex.quote(c) for c in cmd)}\n")
    proc = subprocess.run(cmd, cwd=REPO_ROOT)
    print()
    return proc.returncode


def print_help() -> None:
    print(
        "Commands:\n"
        "  verifier [max_lines]\n"
        "  jit [max_lines]\n"
        "  complex [max_lines] [min_regions] [per_region] [seed]\n"
        "  help\n"
        "  quit\n"
    )


def main() -> int:
    print("sBPF IR Shell")
    print("Shows generated IR and lowered disassembly from sbpf-tool.")
    print_help()

    while True:
        try:
            raw = input("sbpf-ir> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return 0

        if not raw:
            continue

        parts = raw.split()
        cmd = parts[0].lower()

        if cmd in {"quit", "exit"}:
            return 0
        if cmd == "help":
            print_help()
            continue

        try:
            if cmd == "verifier":
                max_lines = int(parts[1]) if len(parts) > 1 else 60
                run_bridge("verifier", max_lines=max_lines)
            elif cmd == "jit":
                max_lines = int(parts[1]) if len(parts) > 1 else 60
                run_bridge("jit", max_lines=max_lines)
            elif cmd == "complex":
                max_lines = int(parts[1]) if len(parts) > 1 else 60
                min_regions = int(parts[2]) if len(parts) > 2 else 12
                per_region = int(parts[3]) if len(parts) > 3 else 24
                seed = int(parts[4]) if len(parts) > 4 else 1337
                run_bridge(
                    "complex",
                    max_lines=max_lines,
                    min_regions=min_regions,
                    per_region=per_region,
                    seed=seed,
                )
            else:
                print("unknown command; type 'help'")
        except ValueError:
            print("invalid numeric argument")


if __name__ == "__main__":
    sys.exit(main())
