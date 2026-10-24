---
name: feature-delegate
description: Pre-spawn check for a feature implementation delegate. Rejects a code-writing Task that is not poteto-agent.
disable-model-invocation: true
---

# Feature delegate

A code-writing feature delegate is a poteto-agent. The prompt carries the poteto-mode skill path.

Before the Task spawn, run:

```bash
python3 support/check-spawn.py --role feature-impl --subagent "<subagent_type>" --prompt "<prompt>"
```

A non-zero exit is a refusal. Do not spawn. How explorers are not this role. They stay on the subagent type the how skill names, and they use `--role how-explorer`.
