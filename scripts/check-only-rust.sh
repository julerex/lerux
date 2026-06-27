#!/usr/bin/env bash
# Only Rust enforcement checks (see docs/plan.md).
# C sources are forbidden outside the relibc debt allowlist; asm (.asm/.S/.s) is allowed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

RUN_SMOKE=0
for arg in "$@"; do
    case "$arg" in
        --smoke) RUN_SMOKE=1 ;;
        --no-smoke) RUN_SMOKE=0 ;;
        *)
            echo "Unknown argument: $arg (try --smoke)" >&2
            exit 2
            ;;
    esac
done

echo "== Only Rust: C source policy =="
violations=()
while IFS= read -r -d '' path; do
    case "$path" in
        vendor/relibc/*) continue ;;
        *)
            violations+=("$path")
            ;;
    esac
done < <(find lerux-kernel userspace vendor -type f -name '*.c' -print0 2>/dev/null || true)

if ((${#violations[@]} > 0)); then
    echo "Disallowed C sources outside allowlist:" >&2
    printf '  %s\n' "${violations[@]}" >&2
    exit 1
fi
echo "OK: no disallowed C outside allowlist"

echo "== Only Rust: ELF audit (initfs staging + bootstrap) =="
audit_elf() {
    local bin="$1"
    if readelf -d "$bin" 2>/dev/null | grep -q 'Shared library:'; then
        echo "Dynamic NEEDED entries in $bin:" >&2
        readelf -d "$bin" | grep 'Shared library:' >&2 || true
        return 1
    fi
    return 0
}

elf_ok=1
if [[ -f build/bootstrap.elf ]]; then
    audit_elf build/bootstrap.elf || elf_ok=0
fi
for bin in userspace/initfs-staging/bin/*; do
    [[ -f "$bin" ]] || continue
    audit_elf "$bin" || elf_ok=0
done

if [[ "$elf_ok" -ne 1 ]]; then
    echo "ELF audit failed" >&2
    exit 1
fi
echo "OK: staged ELFs have no dynamic NEEDED entries"

if [[ "$RUN_SMOKE" -eq 1 ]]; then
    echo "== Only Rust: smoke-userspace =="
    just smoke-userspace
fi

echo "check-only-rust: all checks passed"