#!/usr/bin/env bash
# Install one pinned just binary.
#
# extractions/setup-just lists every GitHub release and fails with
# "no release for just matching version specifier" when that lookup is empty.
set -euo pipefail

version=1.59.0
case "$(uname -m)" in
  x86_64)
    target=x86_64-unknown-linux-musl
    sha256=409044617b6f06517c1035e55ca38c8c856ff8b650e41bc2aa1ef82d5903a23e
    ;;
  aarch64 | arm64)
    target=aarch64-unknown-linux-musl
    sha256=23871b1702bfd550fb5f901778fd4290ecf2ee034a45a889fbe96d89b188086d
    ;;
  *)
    echo "unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

archive="just-${version}-${target}.tar.gz"
url="https://github.com/casey/just/releases/download/${version}/${archive}"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT
curl -fsSL -o "${tmp}/${archive}" "${url}"
echo "${sha256}  ${tmp}/${archive}" | sha256sum -c -
tar -xzf "${tmp}/${archive}" -C "${tmp}" just
dest="${HOME}/.local/bin"
mkdir -p "${dest}"
install -m 755 "${tmp}/just" "${dest}/just"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  echo "${dest}" >> "${GITHUB_PATH}"
fi
echo "installed ${dest}/just ($("${dest}/just" --version))"
