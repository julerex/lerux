#!/usr/bin/env bash
# Apply lerux-specific patches to the vendored rust fork after fetch.
#
# Rust excludes src/bootstrap from its workspace; when bootstrap runs cargo inside
# the lerux repo, cargo walks past vendor/sources/rust and inherits lerux's root
# workspace. Give bootstrap its own workspace root (upstream workaround).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_DIR="$ROOT/vendor/sources/rust"
BOOTSTRAP_CARGO="$RUST_DIR/src/bootstrap/Cargo.toml"

if [[ ! -f "$BOOTSTRAP_CARGO" ]]; then
    echo "patch-vendor-rust: $BOOTSTRAP_CARGO not found (run just fetch-vendor-sources)" >&2
    exit 1
fi

if grep -q '# lerux: standalone workspace for bootstrap' "$BOOTSTRAP_CARGO"; then
    echo "patch-vendor-rust: bootstrap already patched"
else
    cat >> "$BOOTSTRAP_CARGO" <<'EOF'

# lerux: standalone workspace for bootstrap (see scripts/patch-vendor-rust.sh)
[workspace]
members = []
EOF
    echo "patch-vendor-rust: patched src/bootstrap/Cargo.toml"
fi

if [[ -d "$RUST_DIR/.git" ]]; then
    cd "$RUST_DIR"
    # Shallow clone omits submodules; std/backtrace and cargo are required to build.
    if [[ ! -f library/backtrace/Cargo.toml ]]; then
        echo "patch-vendor-rust: initializing rust submodules (backtrace, cargo)..."
        git submodule update --init --depth 1 library/backtrace src/tools/cargo
    else
        echo "patch-vendor-rust: rust submodules OK"
    fi
fi
