#!/usr/bin/env python3
"""Refuse the numbered plan skill when poteto-mode is already attached.

Prints one line pointing at the milestone design doc and exits non-zero.
"""

import argparse
import sys

DESIGN = "docs/plan-lionsos.md"


def judge(skills: list[str]) -> tuple[int, str]:
    names = {skill.strip() for skill in skills if skill.strip()}
    if "poteto-mode" in names and "numbered-plan" in names:
        return (
            1,
            f"poteto-mode owns this milestone; the design doc is {DESIGN}",
        )
    return 0, "numbered plan owner ok"


def self_test() -> int:
    code, message = judge(["poteto-mode", "numbered-plan"])
    if code != 1 or DESIGN not in message or not message.startswith("poteto-mode owns"):
        print(f"plan owner self-test failed: {code} {message}")
        return 1
    code, message = judge(["numbered-plan"])
    if code != 0:
        print(f"plan owner self-test allowed a lone numbered plan: {message}")
        return 1
    print("plan owner self-test ok")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    parser = argparse.ArgumentParser()
    parser.add_argument("--skills", default="", help="Comma-separated attached skill names")
    args = parser.parse_args()
    code, message = judge(args.skills.split(","))
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
