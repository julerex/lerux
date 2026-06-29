#!/usr/bin/env python3
"""Smoke test: boot loader.img in QEMU and expect serial output."""

import argparse
import sys
import time

import pexpect


def expect_ordered(child: pexpect.spawn, patterns: list[str], per_pattern_timeout: int) -> None:
    for pattern in patterns:
        child.expect(pattern, timeout=per_pattern_timeout)


def expect_unordered(
    child: pexpect.spawn, patterns: list[str], total_timeout: int
) -> None:
    remaining = list(patterns)
    deadline = time.time() + total_timeout
    while remaining:
        left = deadline - time.time()
        if left <= 0:
            missing = ", ".join(repr(p) for p in remaining)
            raise pexpect.exceptions.TIMEOUT(
                f"Timed out waiting for: {missing}", child
            )
        idx = child.expect(remaining, timeout=min(left, 5))
        remaining.pop(idx)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--expect",
        action="append",
        default=[],
        help="Substring to wait for in QEMU output (repeatable)",
    )
    parser.add_argument(
        "--unordered",
        action="store_true",
        help="Match --expect patterns in any order (for concurrent PD output)",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=60,
        help="Total timeout in seconds (default: 60)",
    )
    parser.add_argument("cmd", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    patterns = args.expect or ["lerux: Hello from Rust on seL4 Microkit!"]

    child = pexpect.spawn(args.cmd[0], args.cmd[1:], encoding="utf-8", timeout=args.timeout)
    child.logfile = sys.stdout
    if args.unordered:
        expect_unordered(child, patterns, args.timeout)
    else:
        per_pattern = max(30, args.timeout // max(len(patterns), 1))
        expect_ordered(child, patterns, per_pattern)
    print("\n==> smoke test passed")


if __name__ == "__main__":
    main()