#!/usr/bin/env python3
"""Render a board-specific Microkit system description from a template."""

import argparse
import importlib.util
import sys
from pathlib import Path


def load_boards():
    root = Path(__file__).resolve().parent
    spec = importlib.util.spec_from_file_location("board_config", root / "board_config.py")
    if spec is None or spec.loader is None:
        raise RuntimeError("failed to load board_config.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.BOARDS


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--board", required=True, choices=sorted(load_boards()))
    parser.add_argument("-o", type=Path, required=True)
    args = parser.parse_args()

    boards = load_boards()
    board = boards[args.board]
    template_path = Path("userspace/systems/templates") / board["template"]
    template = template_path.read_text()
    rendered = template.format(**board.get("system_vars", {}))
    args.o.parent.mkdir(parents=True, exist_ok=True)
    args.o.write_text(rendered)


if __name__ == "__main__":
    main()