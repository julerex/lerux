#!/usr/bin/env python3
"""Fail when a crate under userspace/crates or lerux-ladybird/crates is missing from a lint list."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE_TREES = (
    "userspace/crates",
    "lerux-ladybird/crates",
)
WORKSPACE = ROOT / "Cargo.toml"
HOST = ROOT / "support" / "host-check.sh"
CLIPPY = ROOT / "tools" / "lerux-cli" / "src" / "clippy.rs"


def crate_manifests() -> list[tuple[str, str, Path]]:
    found = []
    for prefix in CRATE_TREES:
        root = ROOT / prefix
        if not root.is_dir():
            raise SystemExit(f"{prefix} is not a directory")
        for path in sorted(root.iterdir()):
            manifest = path / "Cargo.toml"
            if not manifest.is_file():
                continue
            match = re.search(r'(?m)^name = "([^"]+)"', manifest.read_text())
            if not match:
                raise SystemExit(f"{manifest} has no package name")
            found.append((match.group(1), prefix, manifest))
    return found


def needs_sel4(manifest: Path) -> bool:
    text = manifest.read_text()
    deps = text.split("[dependencies]", 1)[-1].split("\n[", 1)[0]
    return re.search(r"(?m)^\s*sel4[\w-]*\s*=", deps) is not None


def dash_p(script: str, tool: str) -> set[str]:
    names = set()
    for line in script.splitlines():
        if f"cargo {tool}" not in line:
            continue
        parts = line.split()
        for index, part in enumerate(parts):
            if part == "-p" and index + 1 < len(parts):
                names.add(parts[index + 1])
    return names


def shared_crates() -> set[str]:
    text = CLIPPY.read_text()
    match = re.search(r"const SHARED_CRATES: &\[&str\] = &\[(.*?)\];", text, re.S)
    if not match:
        raise SystemExit("SHARED_CRATES is missing from clippy.rs")
    return set(re.findall(r'"([^"]+)"', match.group(1)))


def main() -> int:
    host = HOST.read_text()
    clippy = dash_p(host, "clippy")
    tests = dash_p(host, "test")
    shared = shared_crates()
    workspace = WORKSPACE.read_text()
    errors = []
    host_checked = 0
    cross_only = 0
    for name, prefix, manifest in crate_manifests():
        if manifest.parent.name != name:
            errors.append(
                f"{prefix}/{manifest.parent.name} package name is {name}"
            )
        quoted = f'"{prefix}/{name}"'
        if quoted not in workspace:
            errors.append(f"{name} is not a workspace member")
        if name not in shared:
            errors.append(f"{name} is missing from SHARED_CRATES")
        if needs_sel4(manifest):
            cross_only += 1
            continue
        host_checked += 1
        if name not in clippy:
            errors.append(f"{name} is missing from the host clippy list")
        if name not in tests:
            errors.append(f"{name} is missing from the host test list")
    if errors:
        print("\n".join(errors))
        return 1
    print(f"shared crates: {host_checked} host-checked, {cross_only} cross-only")
    return 0


if __name__ == "__main__":
    sys.exit(main())
