#!/usr/bin/env python3
"""Refuse a Task spawn that breaks the how or feature-delegate rule.

A how explorer and a how explainer stay on generalPurpose. Each prompt must
contain the literal ``readonly: true``. A ``--readonly`` flag does not replace
that literal. A feature implementation delegate must be a poteto-agent.
"""

import argparse
import sys

HOW_TYPES = {"generalPurpose", "general-purpose"}
HOW_ROLES = {
    "how-explorer": "how explorer",
    "how-explainer": "how explainer",
}
LITERAL_READ_ONLY = "readonly: true"


def judge(role: str, subagent: str, prompt: str, readonly: bool) -> tuple[int, str]:
    # `readonly` is accepted so a set flag cannot stand in for the literal.
    _ = readonly
    if role in HOW_ROLES:
        label = HOW_ROLES[role]
        if subagent not in HOW_TYPES:
            return (
                1,
                f"refusing {label}: stay on generalPurpose with {LITERAL_READ_ONLY}",
            )
        if LITERAL_READ_ONLY not in prompt:
            return (
                1,
                f"refusing {label}: prompt lacks {LITERAL_READ_ONLY}",
            )
        return 0, f"{label} ok"
    if role == "feature-impl":
        if subagent != "poteto-agent":
            return (
                1,
                f"refusing feature delegate: subagent {subagent} is not poteto-agent",
            )
        return 0, "feature delegate ok"
    return 1, f"unknown spawn role {role}"


def self_test() -> int:
    cases = [
        ("how-explorer", "generalPurpose", "explore the linker", False, 1),
        ("how-explorer", "generalPurpose", "readonly: true", False, 0),
        ("how-explorer", "generalPurpose", "read-only", False, 1),
        ("how-explorer", "generalPurpose", "readonly=true", False, 1),
        ("how-explorer", "generalPurpose", "explore the linker", True, 1),
        ("how-explorer", "poteto-agent", "readonly: true", True, 1),
        ("how-explainer", "generalPurpose", "readonly: true", False, 0),
        ("how-explainer", "generalPurpose", "read-only", False, 1),
        ("how-explainer", "generalPurpose", "readonly=true", False, 1),
        ("how-explainer", "generalPurpose", "turn the notes into constraints", True, 1),
        ("how-explainer", "poteto-agent", "readonly: true", True, 1),
        ("feature-impl", "generalPurpose", "", False, 1),
        ("feature-impl", "poteto-agent", "", False, 0),
    ]
    failed = False
    for role, subagent, prompt, readonly, expect in cases:
        code, message = judge(role, subagent, prompt, readonly)
        if code != expect:
            print(f"spawn self-test {role} {subagent} -> {code}, want {expect}: {message}")
            failed = True
    if failed:
        return 1
    print("spawn self-test ok")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--role",
        required=True,
        choices=("how-explorer", "how-explainer", "feature-impl"),
    )
    parser.add_argument("--subagent", required=True)
    parser.add_argument("--prompt", default="")
    parser.add_argument("--readonly", action="store_true")
    args = parser.parse_args()
    code, message = judge(args.role, args.subagent, args.prompt, args.readonly)
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
