#!/usr/bin/env python3
"""Board metadata for lerux build, QEMU, and system description generation."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

BOARDS: dict[str, dict[str, Any]] = {
    "qemu_virt_aarch64": {
        "arch": "aarch64",
        "microkit_board": "qemu_virt_aarch64",
        "target": "aarch64-sel4-microkit",
        "target_triple": "aarch64-sel4-microkit",
        "template": "serial-hello.system.template",
        "pds": ["hello", "serial-driver"],
        "qemu": "aarch64",
        "system_vars": {
            "serial_mmio_phys_addr": "0x9_000_000",
            "serial_irq": 33,
        },
    },
    "qemu_virt_aarch64_virtio": {
        "arch": "aarch64",
        "microkit_board": "qemu_virt_aarch64",
        "target": "aarch64-sel4-microkit",
        "target_triple": "aarch64-sel4-microkit",
        "template": "virtio-hello.system.template",
        "pds": ["hello", "serial-driver", "virtio-blk-driver", "virtio-net-driver"],
        "qemu": "aarch64_virtio",
        "system_vars": {
            "serial_mmio_phys_addr": "0x9_000_000",
            "serial_irq": 33,
            "virtio_mmio_phys_addr": "0xa003000",
            "virtio_blk_irq": 78,
            "virtio_net_irq": 79,
        },
    },
    "x86_64_generic": {
        "arch": "x86_64",
        "microkit_board": "x86_64_generic",
        "target": "x86_64-sel4-microkit",
        "target_triple": "x86_64-sel4-microkit",
        "template": "serial-hello-x86.system.template",
        "pds": ["hello", "serial-driver"],
        "qemu": "x86_64",
        "system_vars": {
            "serial_ioport_addr": "0x3f8",
            # COM1 ISA IRQ 4 → IOAPIC pin 4; vector 48 is the first user IRQ slot on pc99.
            "serial_ioapic_pin": 4,
            "serial_ioapic_vector": 48,
        },
    },
}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("board", choices=sorted(BOARDS))
    parser.add_argument(
        "field",
        nargs="?",
        help="Field to print (arch, target, template, pds, qemu, system_vars). Omit for JSON.",
    )
    args = parser.parse_args()

    board = BOARDS[args.board]
    if args.field is None:
        print(json.dumps(board))
        return

    if args.field not in board:
        print(f"error: unknown field {args.field!r}", file=sys.stderr)
        sys.exit(1)

    value = board[args.field]
    if isinstance(value, list):
        print(" ".join(value))
    elif isinstance(value, dict):
        print(json.dumps(value))
    else:
        print(value)


if __name__ == "__main__":
    main()