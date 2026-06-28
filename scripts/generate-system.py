#!/usr/bin/env python3
"""Render a board-specific Microkit system description from a template."""

import argparse
import sys
from pathlib import Path

BOARDS = {
    "qemu_virt_aarch64": {
        "serial_mmio_phys_addr": "0x9_000_000",
        "serial_irq": 33,
    },
}

DEFAULT_TEMPLATE = Path("userspace/systems/templates/serial-hello.system.template")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--board", required=True, choices=sorted(BOARDS))
    parser.add_argument(
        "--template",
        type=Path,
        default=DEFAULT_TEMPLATE,
    )
    parser.add_argument("-o", type=Path, required=True)
    args = parser.parse_args()

    board = BOARDS[args.board]
    template = args.template.read_text()
    rendered = template.format(**board)
    args.o.parent.mkdir(parents=True, exist_ok=True)
    args.o.write_text(rendered)


if __name__ == "__main__":
    main()