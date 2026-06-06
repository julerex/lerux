#!/usr/bin/env bash
# Shallow-clone large Redox toolchain sources into vendor/sources/ (gitignored).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCES="$ROOT/vendor/sources"
MANIFEST="$ROOT/vendor/manifest.toml"

clone_if_missing() {
    local name="$1" git_url="$2" branch="$3" dest="$4"
    if [[ -d "$dest/.git" ]]; then
        echo "== $name: already at $dest"
        return 0
    fi
    echo "== $name: cloning $branch -> $dest"
    mkdir -p "$(dirname "$dest")"
    git clone --depth 1 --branch "$branch" "$git_url" "$dest"
}

read_manifest() {
    local section="$1" key="$2"
    awk -v s="[$section]" -v k="$key" '
        $0 == s { in_s=1; next }
        /^\[/ { in_s=0 }
        in_s && $1 == k { sub(/^[^=]*= *"?/, ""); sub(/"$/, ""); print; exit }
    ' "$MANIFEST"
}

mkdir -p "$SOURCES"

clone_if_missing rust \
    "$(read_manifest rust git)" \
    "$(read_manifest rust branch)" \
    "$ROOT/$(read_manifest rust path)"

clone_if_missing llvm-project \
    "$(read_manifest llvm-project git)" \
    "$(read_manifest llvm-project branch)" \
    "$ROOT/$(read_manifest llvm-project path)"

clone_if_missing rustc_codegen_cranelift \
    "$(read_manifest rustc_codegen_cranelift git)" \
    "$(read_manifest rustc_codegen_cranelift branch)" \
    "$ROOT/$(read_manifest rustc_codegen_cranelift path)"

clone_if_missing cookbook \
    "$(read_manifest cookbook git)" \
    "$(read_manifest cookbook branch)" \
    "$ROOT/$(read_manifest cookbook path)"

echo "fetch-vendor-sources: done"
