# ADR-001: In-tree Rust system generation (not sdfgen)

## Status

Accepted (Phase 41 implemented 2026-07-12)

## Date

2026-07-12

## Context

lerux composes Microkit systems from hand-written `.system.template` XML under `userspace/systems/templates/` (28 templates, ~2k lines). `lerux-cli` only substitutes `{placeholders}` from `support/boards.toml` `system_vars`. Channel IDs are maintained in three places that can drift:

1. Template `<channel>` ends  
2. Free-text `channels` strings in `support/profiles/*.toml` and package fragments  
3. Rust `const …: Channel = Channel::new(N)` in protection domains  

Profiles (`lerux profile build`) select a board; they do not generate SDF. Phase 41 ([plan-au-ts.md](../plan-au-ts.md)) requires fewer hand-edited XML edges, validation before `microkit`, and a single source for channel IDs.

Upstream inspiration is Trustworthy Systems’ [microkit_sdf_gen](https://github.com/au-ts/microkit_sdf_gen) (Python/C/Zig programmatic SDF). Host tooling rules ([AGENTS.md](../../AGENTS.md)) prefer Rust `lerux-cli` with `anyhow` / clap; userspace remains Rust-only postcard RPC, not sDDF C.

Inventory of mechanical vs board-specific content: [system-generation.md](../system-generation.md).

## Decision

**Extend `lerux profile` / `lerux system` with an in-tree Rust SDF generator in `lerux-cli`.**

Do **not** depend on sdfgen Python (or other out-of-tree SDF tools) for the default composition path.

| Concern | Owner |
|---------|--------|
| Topology (PDs, channels, shared rings, maps between clients and drivers) | Structured profile / channel manifest + generator recipes |
| Hardware (MMIO phys, IRQs, x86 ioport/IOAPIC, PCI BAR/DMA phys) | `support/boards.toml` (and future device-binding keys) |
| Irreducible arch quirks | Small device-recipe modules in Rust (or tiny XML snippets only if a recipe is not worth coding yet) |

Default lean from Phase 41 plan: in-tree generator; optional later **bridge** to sdfgen only if we need sDDF-compatible layouts for interop experiments — not for day-to-day workstation builds.

## Alternatives considered

### Call out to sdfgen (Python)

- **Pros:** Mature programmatic SDF; shared vocabulary with au-ts; less original code for MR/channel emission.  
- **Cons:** New host language/runtime in the critical path; versioning/Nix-style deps conflict with `just` + Rust CLI UX; sDDF-shaped APIs push toward C component assumptions we explicitly reject; harder for agents and CI that already run `cargo` only.  
- **Rejected** as default. Revisit only for an optional interop tool, not as the source of truth for lerux profiles.

### Keep hand templates forever; improve docs only

- **Pros:** Zero migration risk; already works for 26+ smokes.  
- **Cons:** Workstation channel graph (~26 channels) and arch forks already show copy-paste cost; Phase 42–44 (serial/net virt, FS backends) will multiply edges; channel drift remains a class of bugs.  
- **Rejected** as the long-term model. Hand templates remain during incremental migration.

### Generate only Rust `Channel` constants; leave XML hand-written

- **Pros:** Smaller change.  
- **Cons:** Does not remove dual maintenance; Microkit still cannot validate topology before link without parsing XML.  
- **Rejected** as the sole fix; constant emission is a **later step** after structured manifests exist.

## Consequences

### Positive

- One language for host tooling and validation logic.  
- Channel IDs can be allocated or checked once and shared with PD builds.  
- `profile diff` can grow to show real SDF deltas.  
- Device recipes encode arch differences (MMIO UART vs COM1 vs virtio-pci) without forking entire system files.  
- Aligns with package fragments: structured channels instead of free-text strings.

### Trade-offs / work

- Must implement Microkit-correct emission (priorities, PPC, map attrs, setvar symbols) carefully; golden tests against current workstation XML.  
- Migration is incremental: structured manifest → channel XML → full SDF → constant check (see inventory).  
- Board entries stay numerous until device bindings factor common `system_vars` sets.  
- Contributors author TOML (+ recipes), not raw XML, for new profiles once golden path lands.

### Non-consequences

- Does not vendor sDDF or change the non-POSIX RPC model.  
- Does not replace `boards.toml` hardware constants.  
- Does not require Microkit or seL4 version bumps for Phase 41.

## Implementation notes

1. ✅ Workstation SDF composed from channel-free layout template + profile `[[channel]]` (unit tests).  
2. ✅ AGENTS.md: channel numbers come from the profile manifest.  
3. ✅ `generate_system` / `render_system` compose layout + channels; `lerux profile sdf|emit-channels|check-channels|diff`.  
4. ✅ Validate on profile load: unique `(pd, id)`, known PDs.  
5. Layout templates still hold MRs/PD maps until later phases shrink them into device recipes.

## References

- [docs/system-generation.md](../system-generation.md) — template inventory  
- [docs/plan-au-ts.md](../plan-au-ts.md) — Phase 41 scope  
- [docs/context.md](../context.md) — profiles, packages, non-POSIX direction  
- [microkit_sdf_gen](https://github.com/au-ts/microkit_sdf_gen) — inspiration only  
