# Config schema (Phase 54)

Configuration is FS-backed under **`/config/`** via `config-server` (`ConfigRequest` / `ConfigResponse`). Changing policy is a **config write + reboot** (or re-read on next service init), not an image rebuild.

## Path layout

| Path | Content |
|------|---------|
| `/config/<key>` | Ordinary policy keys (dot names, e.g. `net.ip`) |
| `/config/secrets/<name>` | Secret material for key `secret.<name>` |
| `/boot.log` | Last boot log ring dump (supervisor) |
| `/boot.log.1` | Previous boot log when `log.rotate=1` |

Keys must be printable ASCII without `/` or NUL. Max lengths: key 32, value 64 (`MAX_CONFIG_KEY_LEN` / `MAX_CONFIG_VAL_LEN`).

## Well-known keys

| Key | Values | Default (QEMU / RPi4) | Notes |
|-----|--------|------------------------|-------|
| `hostname` | short name | `lerux` / `lerux-rpi4` | Logged at boot; not applied to DNS yet |
| `net.mode` | `dhcp` \| `static` | `dhcp` | Informational for now; stack still tries DHCP then static fallback |
| `net.ip` | dotted IPv4 | `10.0.2.15` / `192.168.1.10` | Static fallback address |
| `net.gateway` | dotted IPv4 | `10.0.2.2` / `192.168.1.1` | Default route |
| `net.dns` | dotted IPv4 | `10.0.2.3` / `192.168.1.1` | DNS server |
| `net.prefix` | `1`–`32` | `24` | Prefix length |
| `log.level` | `error` \| `warn` \| `info` \| `debug` | `info` | Applied at boot via `LogRequest::SetMinLevel` (Phase 57) |
| `log.rotate` | `0` \| `1` | `1` | When `1`, rename `/boot.log` → `/boot.log.1` before rewrite |
| `boot.seeded` | `1` | set after first successful seed | Prevents overwriting operator edits on reboot |
| `secret.*` | opaque | (none) | Stored under `/config/secrets/`; listed as keys only |

Constants live in `lerux-interface-types` (`CFG_*`).

## Boot policy (supervisor)

1. Mount/format FS (`fs up`).
2. Ensure `/config` and `/config/secrets` directories exist.
3. **Seed missing keys only** (if `boot.seeded` is absent or incomplete). Never overwrite existing values.
4. Set `boot.seeded=1` and log `lerux-supervisor: first-boot seed ok` (or `config already seeded`).
5. **Read and log policy:** `lerux-supervisor: config hostname=… net.mode=… log.level=…`.
6. Bring up net; rotate/write `/boot.log` according to `log.rotate`.

## Shell

```text
config list              # list keys (secrets shown without values)
config get <key>
config set <key> <value>
config del <key>
get / set / list         # aliases when first arg looks like a key or `config`
hostname                 # print hostname from config (fallback: lerux)
```

## Host tooling

```bash
lerux config schema          # print this schema summary
lerux config seed-disk       # format LERUXFS2 on support/disk.img + default QEMU keys
```

Guest first-boot still seeds if the image is empty; `seed-disk` is for reproducible QEMU disks with known config before boot.

## Secrets

- Logical key `secret.<name>` maps to file `/config/secrets/<name>`.
- `config list` includes `secret.api_token` but not the value.
- `config get secret.api_token` returns the value (shell may read for operator inspection).
- **Phase 60 ACL:** shell (and any non-supervisor client) **cannot** `Set`/`Delete` `secret.*` keys. Writes return `ConfigResponse::Denied` (shell prints `config set: denied (secret.* write is supervisor-only)`). Supervisor seeds secrets at first boot / control path only.
- Ordinary keys (`hostname`, `net.*`, `log.*`) remain shell-writable.
- No encryption at rest yet; prefer config IPC over direct FS open of `/config/secrets/` from untrusted apps.

## Exit criteria (Phase 54)

Operator can change hostname / net.* / log.* via shell, reboot, and see the new values applied in supervisor config log without rebuilding `loader.img`.
