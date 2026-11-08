#!/usr/bin/env python3
"""Require sibling smokes for shared filesystem writers and the ECAM window.

The filesystem plan passes when the new board sets ci = true, or the test
plan names a `just test-...` recipe. An added or removed line that names
`X86_PCI_ECAM_` or `push_x86_pci_ecam` must also name a guest smoke for every
image whose rendered XML contains `pci_ecam`.
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
ECAM_TOKENS = ("X86_PCI_ECAM_", "push_x86_pci_ecam")
ECAM_SMOKES = (
    "just test-fetch-sddf-x86",
    "just test-web-sddf-x86",
    "just test-agent-sddf-x86",
    "just test-program-sddf-x86",
    "just test-x86-fetch-native",
)


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


def ecam_touched(diff: str) -> bool:
    for line in diff.splitlines():
        if line.startswith(("+++", "---")) or not line.startswith(("+", "-")):
            continue
        if any(token in line for token in ECAM_TOKENS):
            return True
    return False


def judge_ecam(diff: str, plan: str) -> tuple[int, str]:
    if not ecam_touched(diff):
        return 0, "sibling smoke: ecam constant unchanged"
    missing = [recipe for recipe in ECAM_SMOKES if recipe not in plan]
    if missing:
        names = ", ".join(missing)
        return 1, f"sibling smoke: pci_ecam change needs {names}"
    return 0, "sibling smoke: pci_ecam images named"


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
        code, message = judge(touched, plan, ci_true)
        if code != 0:
            return code, message
        ecam_code, ecam_message = judge_ecam(text, plan)
        if ecam_code != 0 or ecam_touched(text):
            return ecam_code, ecam_message
        return code, message
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
    if ecam_self_test() != 0:
        return 1
    print("sibling smoke self-test ok")
    return 0


def ecam_self_test() -> int:
    code, message = judge_ecam(" context X86_PCI_ECAM_PHYS\n", "")
    if code != 0:
        print(f"sibling self-test treated a context line as a change: {message}")
        return 1
    header = "+++ b/userspace/crates/lerux-machine/src/table.rs\n"
    code, message = judge_ecam(header, "")
    if code != 0:
        print(f"sibling self-test treated a diff header as a change: {message}")
        return 1
    bare = "+pub const X86_PCI_ECAM_PHYS: u64 = 1;\n"
    code, message = judge_ecam(bare, "just test-fetch-sddf-x86")
    if code != 1:
        print(f"sibling self-test accepted a partial ecam plan: {message}")
        return 1
    code, message = judge_ecam(bare, "\n".join(ECAM_SMOKES))
    if code != 0:
        print(f"sibling self-test rejected a full ecam plan: {message}")
        return 1
    removed = "-fn push_x86_pci_ecam() {}\n"
    code, message = judge_ecam(removed, "")
    if code != 1 or "just test-web-sddf-x86" not in message:
        print(f"sibling self-test missed push_x86_pci_ecam: {message}")
        return 1
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    code, message = real()
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
