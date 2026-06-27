#!/usr/bin/env python3
"""Validate SMP trampoline NASM sources assemble to the expected flat binaries."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
KERNEL_DIR = SCRIPT_DIR.parent.parent
ASM_ROOT = KERNEL_DIR / "src" / "asm"
OUT_DIR = SCRIPT_DIR / "out"

ARCHES = ("x86_64", "x86")
EXPECTED_SIZES = {"x86_64": 202, "x86": 175}


def assemble(arch: str) -> bytes:
    src = ASM_ROOT / arch / "trampoline.asm"
    if not src.is_file():
        raise FileNotFoundError(src)
    out = OUT_DIR / f"trampoline_{arch}.bin"
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["nasm", "-f", "bin", "-o", str(out), str(src)],
        check=True,
        capture_output=True,
        text=True,
    )
    return out.read_bytes()


def cmd_check() -> int:
    from shutil import which

    if not which("nasm"):
        print("error: nasm required", file=sys.stderr)
        return 2

    ok = True
    print("=== SMP trampoline validation ===")
    print(f"Sources: {ASM_ROOT}")
    print()

    for arch in ARCHES:
        print(f"--- {arch} ---")
        try:
            data = assemble(arch)
        except (FileNotFoundError, subprocess.CalledProcessError) as e:
            print(f"  FAIL: {e}", file=sys.stderr)
            ok = False
            continue

        expected_len = EXPECTED_SIZES[arch]
        if len(data) != expected_len:
            print(f"  length: got {len(data)}, expected {expected_len}")
            ok = False
        elif data[8:40] != bytes(32):
            print("  data patch area (bytes 8..40) is not zero-initialized")
            ok = False
        else:
            print(f"  OK ({len(data)} bytes, patch area zeroed)")
        print()

    if ok:
        print("PASS: trampoline asm sources assemble correctly.")
        return 0
    print("FAIL: trampoline validation failed.", file=sys.stderr)
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check", help="Assemble kernel asm sources and verify size/invariants")

    args = parser.parse_args()
    if args.cmd == "check":
        return cmd_check()
    return 2


if __name__ == "__main__":
    sys.exit(main())