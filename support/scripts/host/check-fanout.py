#!/usr/bin/env python3
"""Fail when a directory below the repo root has more than six children."""

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
LIMIT = 6
SKIP_PREFIXES = (
    ".git",
    ".cursor",
    ".scratch",
    "deps",
    "build",
    "target",
    "support/arch-probe/out",
    "support/swift-linux/out",
)
SKIP_NAMES = {"__pycache__", ".git"}


def skipped(rel: str) -> bool:
    return any(rel == prefix or rel.startswith(prefix + "/") for prefix in SKIP_PREFIXES)


def main() -> int:
    offenders = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        rel = Path(dirpath).relative_to(ROOT).as_posix()
        if rel == ".":
            dirnames[:] = [name for name in dirnames if not skipped(name)]
            continue
        if skipped(rel):
            dirnames[:] = []
            continue
        dirnames[:] = [name for name in dirnames if name not in SKIP_NAMES and not skipped(f"{rel}/{name}")]
        kids = [name for name in dirnames if name not in SKIP_NAMES]
        kids.extend(name for name in filenames if name not in SKIP_NAMES)
        if len(kids) > LIMIT:
            offenders.append((len(kids), rel, sorted(kids)))
    if not offenders:
        print("fanout ok")
        return 0
    for count, rel, kids in sorted(offenders, reverse=True):
        print(f"{count} {rel}")
        for name in kids:
            print(f"  {name}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
