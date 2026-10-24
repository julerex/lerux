#!/usr/bin/env python3
"""Fail when a crate under userspace/crates is missing from a lint list.

Every such crate is a workspace member and is named in SHARED_CRATES.
A crate whose Cargo.toml does not depend on sel4 is also named in the
host clippy list and the host test list inside support/host-check.sh.
A sel4 crate stays off that host list: `just check` has no SDK. Forgetting
the host list for a crate that does compile on the host, or forgetting
SHARED_CRATES, is a failure.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "userspace" / "crates"
WORKSPACE = ROOT / "Cargo.toml"
HOST = ROOT / "support" / "host-check.sh"
CLIPPY = ROOT / "tools" / "lerux-cli" / "src" / "clippy.rs"


def crate_names() -> list[str]:
    names = []
    for path in sorted(CRATES.iterdir()):
        manifest = path / "Cargo.toml"
        if not manifest.is_file():
            continue
        match = re.search(r'(?m)^name = "([^"]+)"', manifest.read_text())
        if not match:
            raise SystemExit(f"{manifest} has no package name")
        names.append(match.group(1))
    return names


def needs_sel4(name: str) -> bool:
    text = (CRATES / name / "Cargo.toml").read_text()
    deps = text.split("[dependencies]", 1)[-1].split("\n[", 1)[0]
    return re.search(r"(?m)^\s*sel4[\w-]*\s*=", deps) is not None


def workspace_members() -> set[str]:
    text = WORKSPACE.read_text()
    return set(re.findall(r'"userspace/crates/([^"]+)"', text))


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
    members = workspace_members()
    errors = []
    host_checked = 0
    cross_only = 0
    for name in crate_names():
        if name not in members:
            errors.append(f"{name} is not a workspace member")
        if name not in shared:
            errors.append(f"{name} is missing from SHARED_CRATES")
        if needs_sel4(name):
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
