#!/usr/bin/env bash
# Fetch one GitHub release asset.
#
# Unauthenticated releases/download requests are answered with HTTP 504 or a
# stall. A job token makes that URL redirect, and the API asset URL is the
# backup. Authorization is sent only to github.com and api.github.com. curl
# does not forward it to the release blob host.

github_release_curl() {
  local dest="$1"
  local url="$2"
  if github_curl "${dest}" "${url}"; then
    return 0
  fi
  echo "==> ${url} failed; fetching the asset through the GitHub API" >&2
  github_release_api "${dest}" "${url}"
}

github_curl() {
  local dest="$1"
  local url="$2"
  shift 2
  local -a args=()
  local token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
  token="${token//[$' \t\r\n']/}"
  case "${url}" in
    https://github.com/* | https://api.github.com/*)
      if [[ -n "${token}" ]]; then
        args+=(-H "Authorization: Bearer ${token}")
      fi
      ;;
    *) ;;
  esac
  curl -fsSL \
    --connect-timeout 20 \
    --speed-limit 1024 \
    --speed-time 30 \
    --retry 5 \
    --retry-delay 2 \
    --retry-all-errors \
    "${args[@]}" \
    "$@" \
    -o "${dest}" \
    "${url}"
}

github_release_api() {
  local dest="$1"
  local url="$2"
  local rest owner repo tag name tag_enc api body asset_url
  if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 is required when a GitHub release URL fails" >&2
    return 1
  fi
  rest="${url#https://github.com/}"
  if [[ "${rest}" == "${url}" ]]; then
    echo "not a GitHub release URL: ${url}" >&2
    return 1
  fi
  owner="${rest%%/*}"
  rest="${rest#*/}"
  repo="${rest%%/*}"
  rest="${rest#*/releases/download/}"
  name="${rest##*/}"
  tag="${rest%"${name}"}"
  tag="${tag%/}"
  if [[ -z "${owner}" || -z "${repo}" || -z "${tag}" || -z "${name}" ]]; then
    echo "not a GitHub release URL: ${url}" >&2
    return 1
  fi
  tag_enc="${tag//\//%2F}"
  api="https://api.github.com/repos/${owner}/${repo}/releases/tags/${tag_enc}"
  body="$(mktemp)"
  if ! github_curl "${body}" "${api}" \
    -H "Accept: application/vnd.github+json" \
    -H "User-Agent: lerux"; then
    rm -f "${body}"
    return 1
  fi
  if ! asset_url="$(python3 - "${body}" "${name}" <<'PY'
import json
import sys

path, name = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as handle:
    payload = json.load(handle)
for asset in payload.get("assets", []):
    if asset.get("name") == name:
        print(asset["url"])
        raise SystemExit(0)
raise SystemExit(1)
PY
)"; then
    rm -f "${body}"
    echo "release has no asset named ${name}" >&2
    return 1
  fi
  rm -f "${body}"
  github_curl "${dest}" "${asset_url}" \
    -L \
    -H "Accept: application/octet-stream" \
    -H "User-Agent: lerux"
}
