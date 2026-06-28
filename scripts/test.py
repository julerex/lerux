#!/usr/bin/env python3
"""Smoke test: boot loader.img in QEMU and expect serial output."""

import argparse
import sys

import pexpect


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--expect",
        action="append",
        default=[],
        help="Substring to wait for in QEMU output (repeatable)",
    )
    parser.add_argument("cmd", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    patterns = args.expect or ["lerux: Hello from Rust on seL4 Microkit!"]

    child = pexpect.spawn(args.cmd[0], args.cmd[1:], encoding="utf-8", timeout=60)
    child.logfile = sys.stdout
    for pattern in patterns:
        child.expect(pattern, timeout=30)
    print("\n==> smoke test passed")


if __name__ == "__main__":
    main()