---
name: how-spawn
description: Pre-spawn check for a how explorer or how explainer. Rejects a general-purpose Task whose prompt lacks the literal readonly: true.
disable-model-invocation: true
---

# How spawn

Every how explorer and every how explainer is a general-purpose Task whose prompt contains the literal `readonly: true`. `--readonly` does not replace that literal. The Task tool's readonly field never reaches `support/check-spawn.py`.

Before the Task spawn, run:

```bash
python3 support/check-spawn.py --role how-explorer --subagent generalPurpose --readonly --prompt "<prompt that contains readonly: true>"
```

The explainer uses `--role how-explainer` and the same prompt literal.

A non-zero exit is a refusal. Do not spawn. A prompt that lacks `readonly: true` is refused. A different subagent type is refused, because these spawns stay on generalPurpose. Discard that explorer's harvest directory before any explainer or arena reads it.
