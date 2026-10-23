#!/usr/bin/env python3
"""Compare first-party `unsafe` tokens with support/unsafe-allowlist.txt.

The allowlist shrinks as call sites move into userspace/crates/lerux-gate/src/lib.rs.
A file that is not on the list fails the check. A listed file that no longer
contains the token fails too, so the list cannot go stale.
"""

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOW = ROOT / "support" / "unsafe-allowlist.txt"
SKIP = {".git", "target", "build", "deps", "node_modules"}


def rust_files():
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".")]
        for name in filenames:
            if name.endswith(".rs"):
                yield Path(dirpath) / name


def has_unsafe(path: Path) -> bool:
    # Word match. `UnsafeCell` does not count.
    return re.search(r"\bunsafe\b", path.read_text(errors="replace")) is not None


def current():
    found = []
    for path in rust_files():
        if has_unsafe(path):
            found.append(path.relative_to(ROOT).as_posix())
    return sorted(found)


def main() -> int:
    found = current()
    if len(sys.argv) > 1 and sys.argv[1] == "--write":
        ALLOW.write_text("\n".join(found) + "\n")
        print(f"wrote {len(found)} paths")
        return 0
    allowed = [line.strip() for line in ALLOW.read_text().splitlines() if line.strip()]
    extra = sorted(set(found) - set(allowed))
    stale = sorted(set(allowed) - set(found))
    if extra or stale:
        for path in extra:
            print(f"unsafe outside the allowlist: {path}")
        for path in stale:
            print(f"allowlist entry has no unsafe token: {path}")
        return 1
    print(f"unsafe allowlist matches {len(found)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
