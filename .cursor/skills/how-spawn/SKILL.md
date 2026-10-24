---
name: how-spawn
description: Pre-spawn check for a how explorer. Rejects a general-purpose Task whose prompt does not require read-only.
disable-model-invocation: true
---

# How spawn

Every how explorer is a general-purpose Task with `readonly: true`. The prompt states that requirement.

Before the Task spawn, run:

```bash
python3 support/check-spawn.py --role how-explorer --subagent generalPurpose --readonly --prompt "<prompt that contains readonly: true>"
```

A non-zero exit is a refusal. Do not spawn. A general-purpose explorer whose prompt does not require read-only is refused. A different subagent type is refused, because how explorers stay on generalPurpose.
