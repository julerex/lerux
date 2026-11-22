---
name: poteto-mode
description: One owner for a numbered milestone. Work stays on a feature branch and is not merged from this skill.
disable-model-invocation: true
---

# Poteto mode

One owner. The milestone stays on one feature branch. Open a pull request. Do not commit the milestone onto main. Do not merge.

Do not start numbered-plan on the same milestone. `support/check-plan-owner.py` already treats `poteto-mode` as the owner. The design doc is `docs/plan-lionsos.md`.
