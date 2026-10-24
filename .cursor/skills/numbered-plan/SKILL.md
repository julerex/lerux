---
name: numbered-plan
description: Continue a numbered milestone from docs/plan-lionsos.md. Refuses to start when poteto-mode is already attached.
disable-model-invocation: true
---

# Numbered plan

This skill continues a numbered milestone on main. It does not open a second owner for a milestone poteto-mode already holds.

## Invocation check

Before any other step, run:

```bash
python3 support/check-plan-owner.py --skills "<comma-separated attached skill names>"
```

Pass every attached skill name, including `poteto-mode` and `numbered-plan` when they are attached. A non-zero exit is a refusal. The script prints one line pointing at `docs/plan-lionsos.md`. Stop. Do not implement on main, and do not open a feature branch for that milestone.

## After the check exits zero

Read the milestone in `docs/plan-lionsos.md` and continue that milestone. The design doc is the plan, not a scratch issue.
