#!/usr/bin/env python3
"""Smoke test: boot loader.img in QEMU and expect serial output."""

import argparse
import subprocess
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


def curl_check(url: str, expect_substr: str, timeout: int) -> None:
    deadline = time.time() + timeout
    last_error = ""
    while time.time() < deadline:
        try:
            result = subprocess.run(
                ["curl", "-sf", "--connect-timeout", "2", url],
                capture_output=True,
                text=True,
                timeout=5,
                check=False,
            )
            if result.returncode == 0 and expect_substr in result.stdout:
                print(f"\n==> curl {url} ok")
                return
            last_error = result.stdout or result.stderr or f"exit {result.returncode}"
        except subprocess.TimeoutExpired:
            last_error = "curl timed out"
        time.sleep(0.5)
    raise RuntimeError(f"curl {url} failed: expected {expect_substr!r}, last={last_error!r}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--expect",
        action="append",
        default=[],
        help="Substring to wait for in QEMU output (repeatable)",
    )
    parser.add_argument(
        "--curl",
        nargs=2,
        action="append",
        metavar=("URL", "EXPECT"),
        default=[],
        help="After --expect patterns match, curl URL until body contains EXPECT",
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

    if args.cmd and args.cmd[0] == "--":
        args.cmd = args.cmd[1:]

    patterns = args.expect or ["lerux: Hello from Rust on seL4 Microkit!"]

    child = pexpect.spawn(args.cmd[0], args.cmd[1:], encoding="utf-8", timeout=args.timeout)
    child.logfile = sys.stdout
    try:
        if args.unordered:
            expect_unordered(child, patterns, args.timeout)
        else:
            per_pattern = max(30, args.timeout // max(len(patterns), 1))
            expect_ordered(child, patterns, per_pattern)

        for url, expect_substr in args.curl:
            curl_check(url, expect_substr, timeout=30)

        print("\n==> smoke test passed")
    finally:
        child.terminate(force=True)


if __name__ == "__main__":
    main()