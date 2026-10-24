#!/usr/bin/env python3
"""Warn when a new public item has no caller outside its own tests.

The warning names the symbol and the crates that were searched. It does
not fail the host check. A host-only library that nothing on the current
image calls stays a warning.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ITEM = re.compile(
    r"^\+\s*pub\s+(?:fn|struct|enum|trait|type|const|static|union)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
SKIP = {".git", "target", "build", "deps", "node_modules"}


def merge_base() -> str | None:
    for ref in ("origin/main", "main"):
        verify = subprocess.run(
            ["git", "rev-parse", "--verify", ref],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        if verify.returncode != 0:
            continue
        base = subprocess.run(
            ["git", "merge-base", "HEAD", ref],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        if base.returncode == 0:
            return base.stdout.strip()
    return None


def added_items(base: str) -> list[tuple[str, str]]:
    diff = subprocess.run(
        ["git", "diff", "-U0", base, "--", "*.rs"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if diff.returncode != 0:
        return []
    items = []
    path = ""
    for line in diff.stdout.splitlines():
        if line.startswith("+++ b/"):
            path = line[6:]
            continue
        match = ITEM.match(line)
        if not match or "/tests/" in path or not path.endswith(".rs"):
            continue
        if "/src/" not in path:
            continue
        items.append((path, match.group(1)))
    return items


def crate_of(path: str) -> str:
    parts = Path(path).parts
    for index, part in enumerate(parts):
        if part in {"crates", "pds", "tools"} and index + 1 < len(parts):
            return parts[index + 1]
    return parts[0]


def searched_crates() -> list[str]:
    names = []
    for folder in ("userspace/crates", "userspace/pds", "tools"):
        root = ROOT / folder
        if not root.is_dir():
            continue
        for path in sorted(root.iterdir()):
            if (path / "Cargo.toml").is_file():
                names.append(path.name)
    return names


def referenced_outside(symbol: str, crate: str) -> bool:
    pattern = re.compile(rf"\b{re.escape(symbol)}\b")
    for dirpath, dirnames, filenames in __import__("os").walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".")]
        rel = Path(dirpath).relative_to(ROOT).as_posix()
        if rel == ".":
            rel = ""
        if crate in Path(rel).parts:
            continue
        for name in filenames:
            if not name.endswith(".rs"):
                continue
            text = (Path(dirpath) / name).read_text(errors="replace")
            if pattern.search(text):
                return True
    return False


def main() -> int:
    base = merge_base()
    if base is None:
        print("public api: no merge base, skipping")
        return 0
    crates = searched_crates()
    seen = set()
    for path, symbol in added_items(base):
        key = (path, symbol)
        if key in seen:
            continue
        seen.add(key)
        crate = crate_of(path)
        if referenced_outside(symbol, crate):
            continue
        print(
            f"warning: {crate}::{symbol} has no reference outside that crate's tests; "
            f"searched: {', '.join(crates)}"
        )
    if not seen:
        print("public api: no new exported items")
    return 0


if __name__ == "__main__":
    sys.exit(main())
