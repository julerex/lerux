# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`docs/guide/overview/context.md`**: domain language for this repo.
- **`docs/decisions/`**: architecture decision records. Read the ones that touch the area you are about to work in.
- **`GLOSSARY.md`** at the repo root, if it exists. If **`GLOSSARY-MAP.md`** exists, it points at one `GLOSSARY.md` per context; read each one relevant to the topic.

This repo is single-context. There is no `GLOSSARY-MAP.md`. `/domain-modeling` creates `GLOSSARY.md` lazily when a new term is resolved there. Architecture decision records live in `docs/decisions/`.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

Single-context:

```
/
├── docs/guide/overview/context.md
├── docs/decisions/
└── GLOSSARY.md              ← created lazily by /domain-modeling
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `docs/guide/overview/context.md`, and in `GLOSSARY.md` when that file exists. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't written down yet, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag conflicts with existing decisions

If your output contradicts an existing architecture decision record, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007, but worth reopening because…_
