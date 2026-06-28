#!/usr/bin/env python3
"""Smoke test: boot loader.img in QEMU and expect hello PD serial output."""

import argparse
import sys

import pexpect


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cmd", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    child = pexpect.spawn(args.cmd[0], args.cmd[1:], encoding="utf-8", timeout=30)
    child.logfile = sys.stdout
    child.expect("lerux: Hello from Rust on seL4 Microkit!", timeout=15)
    print("\n==> smoke test passed")


if __name__ == "__main__":
    main()