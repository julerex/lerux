#!/usr/bin/env python3
"""Refuse a Task spawn that breaks the how-explorer or feature-delegate rule.

A how explorer whose subagent is general-purpose must carry a read-only
requirement in the prompt. A feature implementation delegate must be a
poteto-agent. How explorers stay on generalPurpose, which is the type the
how skill names.
"""

import argparse
import sys

HOW_TYPES = {"generalPurpose", "general-purpose"}
READ_ONLY = ("readonly: true", "read-only", "readonly=true")


def judge(role: str, subagent: str, prompt: str, readonly: bool) -> tuple[int, str]:
    prompt_ro = readonly or any(token in prompt for token in READ_ONLY)
    if role == "how-explorer":
        if subagent in HOW_TYPES and not prompt_ro:
            return (
                1,
                "refusing how explorer: general-purpose Task prompt does not require read-only",
            )
        if subagent not in HOW_TYPES:
            return (
                1,
                "refusing how explorer: stay on generalPurpose with readonly: true",
            )
        return 0, "how explorer ok"
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
        ("how-explorer", "poteto-agent", "readonly: true", True, 1),
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
    parser.add_argument("--role", required=True, choices=("how-explorer", "feature-impl"))
    parser.add_argument("--subagent", required=True)
    parser.add_argument("--prompt", default="")
    parser.add_argument("--readonly", action="store_true")
    args = parser.parse_args()
    code, message = judge(args.role, args.subagent, args.prompt, args.readonly)
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
