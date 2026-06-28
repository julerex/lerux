#!/usr/bin/env bash
# Clone seL4 ecosystem repos into deps/workspace/ (not vendored into the tree).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
workspace="${root}/deps/workspace"
sel4_tag="15.0.0"
microkit_tag="2.2.0"

mkdir -p "${workspace}"

clone_or_checkout() {
    local name="$1"
    local url="$2"
    local tag="$3"
    local dest="${workspace}/${name}"

    if [[ -d "${dest}/.git" ]]; then
        echo "==> ${name}: already cloned, checking out ${tag}"
        git -C "${dest}" fetch --tags origin
        git -C "${dest}" checkout "${tag}"
    else
        if [[ -e "${dest}" ]]; then
            echo "==> ${name}: removing existing non-repository path at ${dest}"
            rm -rf "${dest}"
        fi
        echo "==> ${name}: cloning ${tag}"
        git clone --branch "${tag}" --depth 1 "${url}" "${dest}"
    fi
}

clone_or_checkout seL4 https://github.com/seL4/seL4.git "${sel4_tag}"
clone_or_checkout microkit https://github.com/seL4/microkit.git "${microkit_tag}"

echo "==> Dependencies ready under ${workspace}"