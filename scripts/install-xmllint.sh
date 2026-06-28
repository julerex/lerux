#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
deb_file="$(apt-cache show libxml2-utils 2>/dev/null | awk '/^Filename:/{print $2; exit}')"
deb_url="http://archive.ubuntu.com/ubuntu/${deb_file:-pool/main/libx/libxml2/libxml2-utils_2.9.13+dfsg-1ubuntu2.11_amd64.deb}"
bash "${root}/scripts/install-deb-tool.sh" xmllint "${deb_url}" xmllint