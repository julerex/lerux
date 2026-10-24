#!/usr/bin/env python3
"""Require a sibling smoke when a shared filesystem config writer changes.

The plan exit passes when the new board sets ci = true, or the test plan
names a `just test-...` recipe. A diff that does not touch the shared
writer passes.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHARED = {
    "tools/lerux-cli/src/fs_sddf.rs",
    "tools/lerux-image/src/binding.rs",
    "userspace/pds/sddf-shell/Cargo.toml",
}
RECIPE = re.compile(r"just test-[A-Za-z0-9-]+")


def judge(touched: set[str], plan: str, ci_true: bool) -> tuple[int, str]:
    hit = touched & SHARED
    if not hit:
        return 0, "sibling smoke: shared config writer unchanged"
    if ci_true:
        return 0, "sibling smoke: new board has ci = true"
    match = RECIPE.search(plan)
    if match:
        return 0, f"sibling smoke: {match.group(0)}"
    names = ", ".join(sorted(hit))
    return (
        1,
        f"sibling smoke: {names} changed; set ci = true or name a just test- recipe "
        "in the test plan",
    )


def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )


def real() -> tuple[int, str]:
    for ref in ("origin/main", "main"):
        verify = git("rev-parse", "--verify", ref)
        if verify.returncode != 0:
            continue
        base = git("merge-base", "HEAD", ref)
        if base.returncode != 0:
            continue
        names = git("diff", "--name-only", base.stdout.strip(), "HEAD")
        touched = set(names.stdout.splitlines())
        diff = git("diff", base.stdout.strip(), "HEAD")
        text = diff.stdout
        ci_true = bool(re.search(r"(?m)^\+\s*ci = true\s*$", text))
        plan = text + "\n" + git("log", "--format=%B", f"{base.stdout.strip()}..HEAD").stdout
        return judge(touched, plan, ci_true)
    return 0, "sibling smoke: no merge base"


def self_test() -> int:
    code, _ = judge(set(), "", False)
    if code != 0:
        print("sibling self-test rejected an empty diff")
        return 1
    code, message = judge({"tools/lerux-cli/src/fs_sddf.rs"}, "", False)
    if code != 1:
        print(f"sibling self-test accepted a bare writer edit: {message}")
        return 1
    code, message = judge(
        {"tools/lerux-cli/src/fs_sddf.rs"},
        "sibling smoke: just test-fs-sddf-x86",
        False,
    )
    if code != 0 or "just test-fs-sddf-x86" not in message:
        print(f"sibling self-test missed the recipe: {message}")
        return 1
    code, _ = judge({"userspace/pds/sddf-shell/Cargo.toml"}, "", True)
    if code != 0:
        print("sibling self-test rejected ci = true")
        return 1
    print("sibling smoke self-test ok")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    code, message = real()
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
