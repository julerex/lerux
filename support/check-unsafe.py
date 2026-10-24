#!/usr/bin/env python3
"""Compare first-party unsafe code with support/unsafe-allowlist.txt.

Rust 2024: an `unsafe fn` body is not an unsafe block. A block is the
keyword `unsafe` followed by `{`. Comments and string literals do not count.

The allowlist is the set of files that contain an `unsafe` token outside
comments and strings. On main that set must match the file. On a branch,
a file that already contained the token at the merge base with main does
not have to be appended by hand. A file this change introduces does.

support/unsafe-baseline.txt is the approved unsafe-block count. The check
fails when this change increases that count without editing the baseline.
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOW = ROOT / "support" / "unsafe-allowlist.txt"
BASELINE = ROOT / "support" / "unsafe-baseline.txt"
SKIP = {".git", "target", "build", "deps", "node_modules"}


def rust_files():
    for dirpath, dirnames, filenames in walk_tree(ROOT):
        for name in filenames:
            if name.endswith(".rs"):
                yield Path(dirpath) / name


def walk_tree(root: Path):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".")]
        yield dirpath, dirnames, filenames


def scan(text: str) -> tuple[bool, int]:
    """Return (has an unsafe token, unsafe-block count)."""
    i = 0
    n = len(text)
    has = False
    blocks = 0
    while i < n:
        ch = text[i]
        if ch.isspace():
            i += 1
            continue
        if text.startswith("//", i):
            i = text.find("\n", i)
            i = n if i < 0 else i + 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
            continue
        if starts_literal(text, i):
            i = skip_literal(text, i)
            continue
        if is_ident_start(ch):
            start = i
            i += 1
            while i < n and is_ident_continue(text[i]):
                i += 1
            if text[start:i] == "unsafe":
                has = True
                j = skip_gap(text, i)
                if j < n and text[j] == "{":
                    blocks += 1
            continue
        i += 1
    return has, blocks


def skip_gap(text: str, i: int) -> int:
    n = len(text)
    while i < n:
        if text[i].isspace():
            i += 1
            continue
        if text.startswith("//", i):
            nl = text.find("\n", i)
            i = n if nl < 0 else nl + 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
            continue
        break
    return i


def starts_literal(text: str, i: int) -> bool:
    n = len(text)
    if text[i] in "'\"":
        return True
    if text.startswith(("b'", 'b"', "c'", 'c"'), i):
        return True
    if text.startswith(("br", "cr"), i):
        return i + 2 < n and text[i + 2] in '#"'
    return text[i] == "r" and i + 1 < n and text[i + 1] in '#"'


def skip_literal(text: str, i: int) -> int:
    n = len(text)
    if text.startswith(("br", "cr"), i) or (
        text[i] == "r" and i + 1 < n and text[i + 1] in '#"'
    ):
        hashes = 0
        j = i
        if text.startswith(("br", "cr"), i):
            j += 2
        else:
            j += 1
        while j < n and text[j] == "#":
            hashes += 1
            j += 1
        if j < n and text[j] == '"':
            j += 1
            closer = '"' + ("#" * hashes)
            end = text.find(closer, j)
            return n if end < 0 else end + len(closer)
        return i + 1
    if text.startswith(("b'", 'b"', "c'", 'c"'), i):
        return skip_quoted(text, i + 1)
    if text[i] == "'":
        if i + 1 < n and is_ident_start(text[i + 1]):
            j = i + 1
            while j < n and is_ident_continue(text[j]):
                j += 1
            return j
        return skip_quoted(text, i)
    if text[i] == '"':
        return skip_quoted(text, i)
    return i + 1


def skip_quoted(text: str, i: int) -> int:
    quote = text[i]
    j = i + 1
    n = len(text)
    while j < n:
        if text[j] == "\\":
            j += 2
            continue
        if text[j] == quote:
            return j + 1
        if quote == "'" and text[j] == "\n":
            return j
        j += 1
    return n


def is_ident_start(ch: str) -> bool:
    return ch.isascii() and (ch.isalpha() or ch == "_")


def is_ident_continue(ch: str) -> bool:
    return ch.isascii() and (ch.isalnum() or ch == "_")


def scan_path(path: Path) -> tuple[bool, int]:
    return scan(path.read_text(errors="replace"))


def current():
    rows = []
    for path in rust_files():
        has, blocks = scan_path(path)
        if has:
            rows.append((path.relative_to(ROOT).as_posix(), blocks))
    rows.sort()
    return rows


def read_allow() -> list[str]:
    return [line.strip() for line in ALLOW.read_text().splitlines() if line.strip()]


def read_baseline_text(text: str) -> int:
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] == "blocks":
            return int(parts[1])
    raise SystemExit("unsafe baseline is missing a blocks count")


def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )


def merge_base() -> str | None:
    ref = git("rev-parse", "--verify", "origin/main")
    if ref.returncode != 0:
        ref = git("rev-parse", "--verify", "main")
    if ref.returncode != 0:
        return None
    base = git("merge-base", "HEAD", ref.stdout.strip())
    if base.returncode != 0:
        return None
    head = git("rev-parse", "HEAD")
    if head.returncode != 0:
        return None
    if base.stdout.strip() == head.stdout.strip():
        return None
    return base.stdout.strip()


def scan_rev(rev: str) -> tuple[list[str], int]:
    listed = git("ls-tree", "-r", "--name-only", rev)
    if listed.returncode != 0:
        raise SystemExit(listed.stderr.strip() or "git ls-tree failed")
    found = []
    blocks = 0
    for path in listed.stdout.splitlines():
        if not path.endswith(".rs"):
            continue
        if any(part in SKIP or part.startswith(".") for part in path.split("/")):
            continue
        shown = git("show", f"{rev}:{path}")
        if shown.returncode != 0:
            continue
        has, count = scan(shown.stdout)
        if has:
            found.append(path)
            blocks += count
    return sorted(found), blocks


def self_test() -> int:
    has, blocks = scan(
        "\n".join(
            [
                "unsafe fn demo() { let _n = 1; }",
                "unsafe { let _n = 2; }",
                "// unsafe { hidden }",
                'let s = "unsafe { text }";',
                "#[unsafe(no_mangle)]",
                "unsafe impl Send for Demo {}",
            ]
        )
    )
    if not has or blocks != 1:
        print(f"unsafe self-test expected one block, got has={has} blocks={blocks}")
        return 1
    print("unsafe self-test ok")
    return 0


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1] == "--self-test":
        return self_test()
    rows = current()
    found = [path for path, _ in rows]
    blocks = sum(count for _, count in rows)
    print(f"unsafe blocks: {blocks}")
    print(f"unsafe files: {len(found)}")
    if len(sys.argv) > 1 and sys.argv[1] == "--write":
        ALLOW.write_text("\n".join(found) + "\n")
        BASELINE.write_text(f"blocks {blocks}\n")
        print(f"wrote {len(found)} paths and baseline {blocks}")
        return 0
    allowed = read_allow()
    baseline = read_baseline_text(BASELINE.read_text())
    print(f"allowlist length: {len(allowed)}")
    print(f"unsafe baseline: {baseline}")
    base_rev = merge_base()
    if base_rev is None:
        extra = sorted(set(found) - set(allowed))
        introduced_blocks = blocks
        base_blocks = 0
        base_baseline = baseline
    else:
        base_found, base_blocks = scan_rev(base_rev)
        introduced = set(found) - set(base_found)
        extra = sorted(introduced - set(allowed))
        introduced_blocks = blocks - base_blocks
        base_text = git("show", f"{base_rev}:support/unsafe-baseline.txt")
        base_baseline = (
            read_baseline_text(base_text.stdout)
            if base_text.returncode == 0
            else baseline
        )
    stale = sorted(set(allowed) - set(found))
    failed = False
    for path in extra:
        print(f"unsafe outside the allowlist: {path}")
        failed = True
    for path in stale:
        print(f"allowlist entry has no unsafe token: {path}")
        failed = True
    if base_rev is None:
        if blocks > baseline:
            print(
                f"unsafe block count {blocks} rose above baseline {baseline} without a baseline bump"
            )
            failed = True
    elif introduced_blocks > 0 and baseline < base_blocks + introduced_blocks:
        print(
            f"unsafe block count rose by {introduced_blocks} without a baseline bump "
            f"(baseline {baseline}, was {base_baseline}, count {blocks})"
        )
        failed = True
    if failed:
        return 1
    print(f"unsafe allowlist matches {len(found)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
