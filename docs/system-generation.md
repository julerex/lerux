# System generation (Phase 41) ✅

Last updated: 2026-10-04

Companion to [ADR-001](decisions/001-in-tree-system-generation.md) and [plan-au-ts.md](plan-au-ts.md) Phase 41.

## Composition model

```
support/boards.toml                 support/profiles/<name>.toml
  system_vars (MMIO, IRQ, …)          pds + [[channel]] (typed ends)
  template = "*.system.template"      default_board
                \                    /
                 \                  /
                  v                v
         layout body (MRs, PDs, maps, IRQs)
         + generated <channel> XML from [[channel]]
                  |
                  v
         system.system  (+ channel_consts.rs)
                  |
                  v
              microkit → loader.img
```

| Layer | Source of truth |
|-------|-----------------|
| Hardware phys / IRQ | `support/boards.toml` `system_vars` |
| Layout (MRs, PD maps, device IRQs) | `userspace/systems/templates/*.system.template` |
| IPC topology (channel ids, `pp`) | `support/profiles/*.toml` `[[channel]]` |
| PD `Channel::new(N)` drift | `lerux profile check-channels` |

Workstation and workstation-rpi4 templates are **channel-free**: channels exist only in the profile so they cannot drift from hand XML.

## CLI

| Command | Purpose |
|---------|---------|
| `lerux system --board B -o path` | Compose SDF; write `channel_consts.rs` if a profile owns `B` |
| `lerux profile sdf <name> [-o path]` | Same composition via profile |
| `lerux profile emit-channels <name>` | Print generated const module |
| `lerux profile check-channels [name]` | Verify PD named consts vs manifest |
| `lerux profile diff a b` | PD/channel TOML + composed SDF delta |
| `lerux profile validate` | Unique `(pd, id)`, known PDs |

## Implementation map

| Module | Role |
|--------|------|
| `tools/lerux-cli/src/channels.rs` | `ChannelSpec`, validate, `to_xml`, splice |
| `tools/lerux-cli/src/channel_consts.rs` | emit + check Rust constants |
| `tools/lerux-cli/src/system.rs` | `render_system` / `render_profile_system` / SDF diff |
| `tools/lerux-cli/src/profile.rs` | load profiles, `find_profile_for_board` |
| `tools/lerux-cli/src/serial_sddf.rs` | write the serial-role config bytes and check them against the rendered system description |
| `tools/lerux-cli/src/fs_sddf.rs` | write the filesystem-image config bytes and check them against the rendered system description |

## Template catalog (layout bodies)

Templates under `userspace/systems/templates/`. Most still embed hand channels for smoke boards that have **no** profile. Profile-owned boards (workstation, `browser`, …) splice channels on every `render_system`.

| Kind | Board-specific | Mechanical |
|------|----------------|------------|
| UART / virtio / genet MMIO + IRQ | `system_vars` | — |
| Shared ring MR sizes/vaddrs | — | template layout |
| Channel ends | — | profile `[[channel]]` |
| App-only PDs | — | template PD stanza |

## Serial role image

`qemu_virt_aarch64_serial_sddf` has no profile. `serial-sddf.system.template` owns the memory regions and the channel ends. `lerux-sddf::serial_image` fills the config structs with those virtual addresses and channel ids. The command-line interface writes the same bytes during the board build. The Microkit kit cannot prefill a memory region, so each protection domain embeds the bytes in its build script and copies them at start. The host test checks that the rendered system description matches those addresses and channel ids. Agent rules for the padding, the copy, and the notify path are in [AGENTS.md](../AGENTS.md#lionsos-queues-lerux-sddf).

## Filesystem image

`qemu_virt_aarch64_fs_sddf` has no profile. `fs-sddf.system.template` owns the memory regions and the channel ends. Serial roles share this image. Block and filesystem virtual addresses start at `0x4_000_000` so the serial queues keep `0x3_000_000`. `lerux-sddf::blk_image` and `lerux-sddf::fs_image` fill the config structs with those virtual addresses and channel ids. The command-line interface writes the same bytes during the board build. Each protection domain embeds the bytes in its build script and copies them at start. `fatfs` embeds the block client config and then the filesystem server config in one file. The shell embeds the serial client config and then `shell_client_config()` in one file. The host test checks that the rendered system description matches those addresses and channel ids. `lerux disk-img` writes a 4 MiB raw `support/disk.img` with `55 AA` at byte 510. The guest formats that image. Agent rules for the shared data region, the format order, and the completion-slot zeroing are in [AGENTS.md](../AGENTS.md#filesystem-image-qemu_virt_aarch64_fs_sddf).

## Incremental history

1. ✅ Structured `[[channel]]` + validation  
2. ✅ Channel XML splice + workstation golden  
3. ✅ Channel-free workstation templates + const emit/check  
4. ✅ `profile diff` SDF delta + `profile sdf` emit  
5. Later (42+): shrink layout templates into device recipes; serial/net virtualisers  

## Out of scope for Phase 41

- Full sDDF / C sdfgen dependency (see ADR-001)
- Generating every MR/map without a layout template
- Auto PCI BAR discovery
