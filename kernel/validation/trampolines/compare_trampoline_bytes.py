#!/usr/bin/env python3
"""Compare SMP trampoline bytes: NASM output vs trampoline.rs vs golden files."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
KERNEL_DIR = SCRIPT_DIR.parent.parent
TRAMPOLINE_RS = KERNEL_DIR / "src" / "arch" / "x86_shared" / "trampoline.rs"
ASM_DIR = SCRIPT_DIR / "asm"
OUT_DIR = SCRIPT_DIR / "out"
EXPECTED_DIR = SCRIPT_DIR / "expected"

ARCHES = ("x86_64", "x86")

CFG_MARKERS = {
    "x86_64": ('#[cfg(target_arch = "x86_64")]', "/// 32-bit x86"),
    "x86": ('#[cfg(target_arch = "x86")]', "#[cfg(not(any(target_arch"),
}


def extract_rust_bytes(arch: str) -> bytes:
    """Bytes embedded in trampoline.rs (via include_bytes! or inline array)."""
    text = TRAMPOLINE_RS.read_text()
    include = re.search(
        rf'#\[cfg\(target_arch = "{arch}"\)\][\s\S]*?include_bytes!\("([^"]+)"\)',
        text,
    )
    if include:
        rel = include.group(1)
        path = (TRAMPOLINE_RS.parent / rel).resolve()
        return path.read_bytes()

    start_marker, end_marker = CFG_MARKERS[arch]
    start = text.index(start_marker)
    end = text.index(end_marker, start)
    block = text[start:end]

    nums: list[int] = []
    in_array = False
    for line in block.splitlines():
        if "&[" in line:
            in_array = True
            continue
        if not in_array:
            continue
        chunk = line.split("];")[0]
        nums.extend(int(x, 16) for x in re.findall(r"0x[0-9a-fA-F]{1,2}\b", chunk))
        if "];" in line:
            break
    return bytes(nums)


def assemble(arch: str) -> bytes:
    src = ASM_DIR / f"trampoline_{arch}.asm"
    out = OUT_DIR / f"trampoline_{arch}.bin"
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    import subprocess

    subprocess.run(
        ["nasm", "-f", "bin", "-o", str(out), str(src)],
        check=True,
        capture_output=True,
        text=True,
    )
    return out.read_bytes()


def diff_bytes(a: bytes, b: bytes, label_a: str, label_b: str) -> list[str]:
    lines: list[str] = []
    if len(a) != len(b):
        lines.append(f"  length: {label_a}={len(a)} {label_b}={len(b)}")
    limit = max(len(a), len(b))
    count = 0
    for i in range(limit):
        av = a[i] if i < len(a) else None
        bv = b[i] if i < len(b) else None
        if av != bv:
            count += 1
            if count <= 8:
                av_s = f"{av:#04x}" if av is not None else "---"
                bv_s = f"{bv:#04x}" if bv is not None else "---"
                lines.append(f"  @{i:3d}: {label_a}={av_s} {label_b}={bv_s}")
    if count > 8:
        lines.append(f"  ... and {count - 8} more differing bytes")
    elif count == 0 and len(a) == len(b):
        lines.append("  OK (byte-for-byte match)")
    return lines


def format_rust_array(data: bytes, width: int = 12) -> str:
    rows: list[str] = []
    for i in range(0, len(data), width):
        chunk = data[i : i + width]
        rows.append("    " + ", ".join(f"0x{b:02x}" for b in chunk) + ",")
    return "\n".join(rows)


def cmd_check() -> int:
    missing = [tool for tool in ("nasm",) if not shutil_which(tool)]
    if missing:
        print(f"error: missing tools: {', '.join(missing)}", file=sys.stderr)
        return 2

    ok = True
    print("=== SMP trampoline validation ===")
    print(f"Sources: {ASM_DIR}")
    print(f"Rust:    {TRAMPOLINE_RS}")
    print()

    for arch in ARCHES:
        print(f"--- {arch} ---")
        nasm = assemble(arch)
        rust = extract_rust_bytes(arch)
        golden_path = EXPECTED_DIR / f"trampoline_{arch}.bin"
        golden = golden_path.read_bytes() if golden_path.exists() else None

        for left, right, la, lb in (
            (nasm, rust, "nasm", "trampoline.rs"),
            (nasm, golden, "nasm", "expected") if golden is not None else (None, None, "", ""),
            (rust, golden, "trampoline.rs", "expected") if golden is not None else (None, None, "", ""),
        ):
            if left is None:
                continue
            print(f"  {la} vs {lb}:")
            lines = diff_bytes(left, right, la, lb)
            print("\n".join(lines))
            if not any("OK" in line for line in lines):
                ok = False
        print()

    if ok:
        print("PASS: all trampoline bytes match NASM output and golden files.")
        return 0
    print("FAIL: trampoline byte mismatch (see above).", file=sys.stderr)
    return 1


def cmd_refresh_expected() -> int:
    if not shutil_which("nasm"):
        print("error: nasm required", file=sys.stderr)
        return 2
    EXPECTED_DIR.mkdir(parents=True, exist_ok=True)
    for arch in ARCHES:
        data = assemble(arch)
        path = EXPECTED_DIR / f"trampoline_{arch}.bin"
        path.write_bytes(data)
        print(f"wrote {path} ({len(data)} bytes)")
    return 0


def cmd_print_rust() -> int:
    if not shutil_which("nasm"):
        print("error: nasm required", file=sys.stderr)
        return 2
    for arch in ARCHES:
        data = assemble(arch)
        print(f"// {arch}: {len(data)} bytes (from {ASM_DIR}/trampoline_{arch}.asm)")
        print(format_rust_array(data))
        print()
    return 0


def shutil_which(name: str) -> str | None:
    from shutil import which

    return which(name)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("check", help="Assemble NASM, compare to trampoline.rs and expected/*.bin")
    sub.add_parser(
        "refresh-expected",
        help="Regenerate expected/*.bin from asm/ (run after intentional asm changes)",
    )
    sub.add_parser("print-rust", help="Print Rust byte arrays from asm/ for manual paste")

    args = parser.parse_args()
    if args.cmd == "check":
        return cmd_check()
    if args.cmd == "refresh-expected":
        return cmd_refresh_expected()
    if args.cmd == "print-rust":
        return cmd_print_rust()
    return 2


if __name__ == "__main__":
    sys.exit(main())
