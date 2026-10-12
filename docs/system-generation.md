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
| `tools/lerux-cli/src/net_sddf.rs` | write the network-image config bytes and check them against the rendered system description |

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

`qemu_virt_aarch64_fs_sddf` has no profile. `fs-sddf.system.template` owns the memory regions and the channel ends. Serial roles share this image. Block and filesystem virtual addresses start at `0x4_000_000` so the serial queues keep `0x3_000_000`. `lerux-sddf::blk_image` and `lerux-sddf::fs_image` fill the config structs with those virtual addresses and channel ids. The command-line interface writes the same bytes during the board build. Each protection domain embeds the bytes in its build script and copies them at start. `fatfs` embeds the block client config and then the filesystem server config in one file. The shell embeds the serial client config, `fs_image::shell_client_config()`, the timer client page, and the schedule page in one file. Every role has a budget, and the period equals that budget. `timer_driver` is passive, priority 5, budget 5000. The host test checks that the rendered system description matches those addresses and channel ids. `lerux disk-img` writes a 4 MiB raw `support/disk.img` with `55 AA` at byte 510. The guest formats that image. Agent rules for the shared data region, the format order, and the completion-slot zeroing are in [AGENTS.md](../AGENTS.md#filesystem-image-qemu_virt_aarch64_fs_sddf).

## Network image

`qemu_virt_aarch64_net_sddf` has no profile. `net-sddf.system.template` owns the memory regions and the channel ends. Serial roles share this image. Network queues start at `0x5_000_000`. `lerux-sddf::net_image` fills the config structs with those virtual addresses and channel ids. The command-line interface writes the same bytes during the board build. Each protection domain embeds the bytes in its build script and copies them at start. The client embeds the serial client config and then the network client config in one file. The host test checks that the rendered system description matches those addresses and channel ids. Virtio-net is the only virtio device, so it occupies `+0xe00` and interrupt 79. Agent rules for the client mappings, the buffer ownership, and the address pacing are in [AGENTS.md](../AGENTS.md#network-image-qemu_virt_aarch64_net_sddf).

## Fetch image

`qemu_virt_aarch64_fetch_sddf` has no profile. `fetch-sddf.system.template` owns the memory regions and the channel ends. It is the filesystem image plus the network roles. The shell is the one client of the serial queues, the filesystem queue, and the network queues. Its network channels are 3 and 4 because serial transmit, serial receive, and the filesystem server already use 0, 1, and 2. The shell embeds the serial client config, `fs_image::shell_client_config()`, and `net_image::shell_client_config()` in one file. The same shell runs `calc`, `backup`, `source`, `cert`, `edit`, and `chat`. `chat` uses the host echo on port 18080, which the board starts with `qemu.tcp_echo`. Virtio-net is the first virtio device at `+0xe00` and interrupt 79. Virtio-blk stays at `+0xc00` and interrupt 78. Agent rules for the socket calls, the fixture fetch, and the transmit-buffer init are in [AGENTS.md](../AGENTS.md#fetch-image-qemu_virt_aarch64_fetch_sddf).

## Web image

`qemu_virt_aarch64_web_sddf` has no profile. It uses `fetch-sddf.system.template` and the same protection-domain list as the fetch image. The shell reads `/config/hostname` and serves those bytes on guest port 8080. Hostfwd maps host port 18080 to that port. The board sets `curl_expect` to `lerux` and does not start the host echo or the https-one fixture. Agent rules are in [AGENTS.md](../AGENTS.md#web-image-qemu_virt_aarch64_web_sddf).

## Browser image

`qemu_virt_aarch64_browser_sddf` has no profile. `browser-sddf.system.template` is the fetch template plus `gpu_driver` and `gpu_virt`. Virtio-gpu is the third virtio-mmio device, at `+0xa00` and interrupt 77. Graphics regions start at `0x7_000_000`. The shell config stays the serial client page, the filesystem shell page, and the network shell page. The pinned graphics headers have no configuration page. Agent rules are in [AGENTS.md](../AGENTS.md#browser-image-qemu_virt_aarch64_browser_sddf).

## Agent image

`qemu_virt_aarch64_agent_sddf` has no profile. `agent-sddf.system.template` is the fetch template plus the tool page and the agent protection domain. The shell program is `sddf-grok.elf`. It maps the serial client regions and the tool page. The agent maps the filesystem client regions, the network client regions, and the tool page. The shell embeds the serial client config only. The agent embeds `fs_image::shell_client_config()` and then `net_image::shell_client_config()`. The tool region is `0xa_000_000` and its size is `0x1_000`. Shell channel 2 notifies agent channel 0. Shell channel 3 notifies agent channel 1. There is no graphics role. Agent rules are in [AGENTS.md](../AGENTS.md#agent-image-qemu_virt_aarch64_agent_sddf).

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
