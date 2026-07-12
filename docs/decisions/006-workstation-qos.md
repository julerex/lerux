# ADR-006: Workstation service classes and PD priorities

## Status

Accepted (Phase 48)

## Date

2026-07-12

## Context

Workstation systems share one core under Microkit fixed priorities. We want clearer isolation between shell (interactive), fs/net (services), and bulk apps (edit/chat/http-fs).

Microkit enforces: **PPC callees must have strictly higher priority than callers**. The shell uses PPC to reach `edit`, `chat`, `fs_server`, `net_server`, `supervisor`, `log_server`, `config_server`, and `serial_virt`. Therefore the shell **cannot** sit above those PDs.

## Decision

1. Publish **service classes** for workstation templates (platform / services / control / bulk / interactive) with documented priority bands in [`docs/qos.md`](../qos.md).
2. Keep the existing numeric layout that already satisfies PPC edges (shell = 1; bulk/control = 2–3; services = 4–5; platform = 6–10). Do **not** raise shell above apps while shell→app launch remains PPC.
3. Treat **single-flight** fs/net jobs as the v1 bulk throttle (no MCS budgets, no token buckets).
4. Expose a read-only shell `qos` command summarizing the policy; stress note uses `just test-workstation` concurrent boot as the smoke proxy.

## Alternatives considered

### Raise shell above bulk apps

Rejected under current channel graph: shell PPCs edit/chat; Microkit would reject the image (verified when trialing shell priority 6).

### Convert shell→edit/chat to notify-only

Deferred: enables higher shell priority later, but changes app launch IPC and is out of Phase 48 scope.

### MCS / time partitions

Deferred: larger kernel/config surface.

## Consequences

- Interactive “feel” depends on higher-priority PDs blocking (typical for drivers/services), not on shell outranking apps.
- Any new workstation channel with `pp = true` must re-check priority ordering (`microkit` fails the image otherwise).
- Future QoS work may move app launch off PPC if measured shell starvation appears.

## References

- Phase 48 in [plan-au-ts.md](../plan-au-ts.md)
- [docs/qos.md](../qos.md)
- Microkit system tool PPC priority check
