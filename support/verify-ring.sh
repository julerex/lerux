#!/usr/bin/env bash
# Prove userspace/crates/lerux-ring with Verus 0.2026.10.18.7d82994.
#
# The release binary is built for Ubuntu 24.04. A host with an older C library
# re-runs this script in that image. Artifacts land in deps/verus, which is
# gitignored. The proof target directory is build/verus on the host and /tmp
# inside the container, so it does not share build/host or build/target.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
release="0.2026.10.18.7d82994"
verus_sha256="e9af03732d4bdd32b93a8fcf471706739c24c1473fa5086618d96c47237845dd"
fmt_version="0.7.4"
verus_dir="${root}/deps/verus/verus-x86-linux"
fmt_dir="${root}/deps/verus/verusfmt-${fmt_version}"

download_verus() {
  if [[ -x "${verus_dir}/verus" && -x "${verus_dir}/cargo-verus" ]]; then
    return
  fi
  local archive tmp
  archive="verus-${release}-x86-linux.zip"
  tmp="$(mktemp -d)"
  curl -fsSL -o "${tmp}/${archive}" \
    "https://github.com/verus-lang/verus/releases/download/release/${release}/${archive}"
  echo "${verus_sha256}  ${tmp}/${archive}" | sha256sum -c -
  mkdir -p "${root}/deps/verus"
  unzip -q -d "${root}/deps/verus" "${tmp}/${archive}"
  rm -rf "${tmp}"
}

download_verusfmt() {
  if [[ -x "${fmt_dir}/verusfmt" ]]; then
    return
  fi
  local name tmp
  name="verusfmt-x86_64-unknown-linux-gnu.tar.xz"
  tmp="$(mktemp -d)"
  curl -fsSL -o "${tmp}/${name}" \
    "https://github.com/verus-lang/verusfmt/releases/download/v${fmt_version}/${name}"
  curl -fsSL -o "${tmp}/${name}.sha256" \
    "https://github.com/verus-lang/verusfmt/releases/download/v${fmt_version}/${name}.sha256"
  (cd "${tmp}" && sha256sum -c "${name}.sha256")
  mkdir -p "${fmt_dir}"
  tar -xJf "${tmp}/${name}" -C "${fmt_dir}"
  rm -rf "${tmp}"
  if [[ ! -x "${fmt_dir}/verusfmt" ]]; then
    # The archive may contain a single binary at the top, or one directory.
    local found
    found="$(find "${fmt_dir}" -type f -name verusfmt -print -quit)"
    if [[ -z "${found}" ]]; then
      echo "verusfmt binary not found in ${name}" >&2
      exit 1
    fi
    if [[ "${found}" != "${fmt_dir}/verusfmt" ]]; then
      install -m 755 "${found}" "${fmt_dir}/verusfmt"
    fi
  fi
}

glibc_too_old() {
  local out
  out="$("${verus_dir}/verus" --version 2>&1 || true)"
  [[ "${out}" == *GLIBC_* ]]
}

run_in_docker() {
  exec docker run --rm \
    -e VERIFY_RING_IN_DOCKER=1 \
    -e DEBIAN_FRONTEND=noninteractive \
    -v "${root}:/lerux" \
    -v lerux-verus-rustup:/root/.rustup \
    -v lerux-verus-cargo:/root/.cargo \
    -w /lerux \
    ubuntu:24.04 \
    bash /lerux/support/verify-ring.sh
}

ensure_rustup() {
  if command -v rustup >/dev/null 2>&1; then
    return
  fi
  if [[ "$(id -u)" -ne 0 ]]; then
    echo "rustup is required to run Verus" >&2
    exit 1
  fi
  apt-get update
  apt-get install -y --no-install-recommends \
    ca-certificates curl build-essential gcc pkg-config xz-utils unzip
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain none
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
}

ensure_toolchains() {
  # Verus 0.2026.10.18 invokes this toolchain by name. The repository pin
  # nightly-2026-04-04 is what `cargo verus` uses to drive Cargo.
  rustup toolchain install 1.99.1 --profile minimal
  rustup toolchain install nightly-2026-04-04 --profile minimal --component rustfmt
}

download_verus

if [[ "${VERIFY_RING_IN_DOCKER:-}" != 1 ]] && glibc_too_old; then
  if ! command -v docker >/dev/null 2>&1; then
    echo "Verus ${release} needs glibc 2.39 or newer, and docker is not installed" >&2
    exit 1
  fi
  run_in_docker
fi

# The Ubuntu image is new on every run. Its certificate store has to be
# installed before Cargo talks to crates.io, even when the toolchain volume
# already holds rustup.
if [[ "${VERIFY_RING_IN_DOCKER:-}" == 1 ]]; then
  apt-get update
  apt-get install -y --no-install-recommends \
    ca-certificates curl build-essential gcc pkg-config xz-utils unzip
fi

if ! command -v rustup >/dev/null 2>&1 && [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi
ensure_rustup
ensure_toolchains
download_verusfmt

export PATH="${verus_dir}:${fmt_dir}:${HOME}/.cargo/bin:${PATH}"

target_dir="${root}/build/verus"
if [[ "${VERIFY_RING_IN_DOCKER:-}" == 1 ]]; then
  target_dir="/tmp/verus-target"
fi

cd "${root}"
# `focus` checks this crate and builds `vstd` without re-proving it.
cargo verus focus -p lerux-ring --features verus --locked --target-dir "${target_dir}"
verusfmt --check "${root}/userspace/crates/lerux-ring/src/lib.rs"
