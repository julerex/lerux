#!/bin/bash
# Copy arch-probe-out off a CAPTURE partition into support/arch-probe/out/.
#
# The partition is mounted read-only. The stick is not erased. With no device,
# USB sticks that have a CAPTURE partition are listed when more than one is
# present.

set -euo pipefail

die() {
  echo "read-arch-probe: $*" >&2
  exit 1
}

usage() {
  echo "usage: just read-arch-probe" >&2
  echo "       just read-arch-probe /dev/sdX" >&2
  echo "With no device, the USB stick labelled CAPTURE is mounted read-only." >&2
}

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

dev=
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    -*)
      die "unknown option $1"
      ;;
    *)
      [[ -z $dev ]] || die "only one device can be given"
      dev=$1
      shift
      ;;
  esac
done

mnt=
mounted=0
cleanup() {
  if [[ $mounted -eq 1 && -n $mnt ]]; then
    if [[ $(id -u) -eq 0 ]]; then
      umount "$mnt" || true
    else
      sudo umount "$mnt" || true
    fi
  fi
  if [[ -n $mnt ]]; then
    rmdir "$mnt" 2>/dev/null || true
  fi
}
trap cleanup EXIT

run_root() {
  if [[ $(id -u) -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

pair_value() {
  local line=$1 key=$2
  local re="(^| )${key}=\"([^\"]*)\""
  [[ $line =~ $re ]] || return 1
  printf '%s\n' "${BASH_REMATCH[2]}"
}

trim_answer() {
  local s=$1
  s=${s#"${s%%[![:space:]]*}"}
  s=${s%"${s##*[![:space:]]}"}
  printf '%s\n' "$s"
}

have_terminal() {
  if (: </dev/tty) 2>/dev/null; then
    return 0
  fi
  [[ -t 0 ]]
}

read_answer() {
  local line
  if (: </dev/tty) 2>/dev/null; then
    IFS= read -r line </dev/tty || return 1
  elif [[ -t 0 ]]; then
    IFS= read -r line || return 1
  else
    return 1
  fi
  printf '%s\n' "$line"
}

root_disk_name() {
  local src pk
  src=$(findmnt -no SOURCE /)
  pk=$(lsblk -no PKNAME "$src" 2>/dev/null || true)
  if [[ -n $pk ]]; then
    printf '%s\n' "$pk"
  else
    basename "$src"
  fi
}

system_mountpoint() {
  local name=$1 src pk disk target
  while read -r target src; do
    [[ -n $src ]] || continue
    pk=$(lsblk -no PKNAME "$src" 2>/dev/null || true)
    disk=${pk:-$(basename "$src")}
    [[ $disk == "$name" ]] || continue
    case "$target" in
      / | /boot | /boot/* | /home | /home/*)
        printf '%s\n' "$target"
        return 0
        ;;
    esac
  done < <(findmnt -rno TARGET,SOURCE)
  return 1
}

field() {
  local key=$1 file=$2
  [[ -f $file ]] || return 0
  awk -v key="$key" 'index($0, key ": ") == 1 { print substr($0, length(key) + 3); exit }' "$file"
}

slugify() {
  local s=${1,,}
  s=$(printf '%s' "$s" | tr -c 'a-z0-9._-' '_')
  while [[ $s == *__* ]]; do
    s=${s//__/_}
  done
  s=${s#_}
  s=${s%_}
  [[ -n $s ]] || s=desk
  printf '%s\n' "$s"
}

collect_captures() {
  local line name label pk type tran
  capture_nodes=()
  while read -r line; do
    [[ -n $line ]] || continue
    label=$(pair_value "$line" LABEL) || label=
    [[ $label == CAPTURE ]] || continue
    type=$(pair_value "$line" TYPE) || continue
    [[ $type == part ]] || continue
    pk=$(pair_value "$line" PKNAME) || continue
    name=$(pair_value "$line" NAME) || continue
    tran=$(lsblk -dn -o TRAN "/dev/$pk" | awk '{ print $1 }')
    [[ $tran == usb ]] || continue
    [[ $pk != "$(root_disk_name)" ]] || continue
    if system_mountpoint "$pk" >/dev/null; then
      continue
    fi
    capture_nodes+=("/dev/$name")
  done < <(lsblk -P -o NAME,LABEL,PKNAME,TYPE)
}

choose_capture() {
  local choice i index
  if ! have_terminal; then
    die "name the stick: just read-arch-probe /dev/sdX"
  fi
  collect_captures
  if [[ ${#capture_nodes[@]} -eq 0 ]]; then
    echo "read-arch-probe: plug in the USB stick" >&2
    while [[ ${#capture_nodes[@]} -eq 0 ]]; do
      sleep 1
      collect_captures
    done
  fi
  if [[ ${#capture_nodes[@]} -eq 1 ]]; then
    node=${capture_nodes[0]}
    echo "read-arch-probe: using $node" >&2
    return 0
  fi
  while true; do
    echo "read-arch-probe: CAPTURE partitions" >&2
    for i in "${!capture_nodes[@]}"; do
      printf '  %d  %s\n' "$((i + 1))" "${capture_nodes[$i]}" >&2
    done
    echo "read-arch-probe: type a number, or q to stop" >&2
    choice=$(read_answer) || die "stopped"
    choice=$(trim_answer "$choice")
    [[ $choice == q ]] && die "stopped"
    if [[ $choice =~ ^[0-9]{1,4}$ ]]; then
      index=$((10#$choice))
      if ((index >= 1 && index <= ${#capture_nodes[@]})); then
        node=${capture_nodes[$((index - 1))]}
        echo "read-arch-probe: using $node" >&2
        return 0
      fi
    fi
    echo "read-arch-probe: type a number from the list" >&2
    collect_captures
  done
}

resolve_node() {
  local given=$1 type label pk tran disk name
  given=$(readlink -f "$given")
  [[ -b $given ]] || die "$given is not a block device"
  type=$(lsblk -dn -o TYPE "$given" | awk '{ print $1 }')
  if [[ $type == disk ]]; then
    disk=$given
    node=$(lsblk -lnpo NAME,LABEL "$disk" | awk '$2 == "CAPTURE" { print $1; exit }')
    [[ -n $node ]] || die "$disk has no CAPTURE partition"
  elif [[ $type == part ]]; then
    label=$(lsblk -dn -o LABEL "$given" | awk '{ print $1 }')
    [[ $label == CAPTURE ]] || die "$given is not labelled CAPTURE"
    node=$given
    pk=$(lsblk -dn -o PKNAME "$given" | awk '{ print $1 }')
    [[ -n $pk ]] || die "$given has no parent disk"
    disk=/dev/$pk
  else
    die "pass the stick or its CAPTURE partition ($given is $type)"
  fi
  tran=$(lsblk -dn -o TRAN "$disk" | awk '{ print $1 }')
  [[ $tran == usb ]] || die "refusing $disk (transport is ${tran:-not usb})"
  name=$(basename "$disk")
  [[ $name != "$(root_disk_name)" ]] || die "refusing $disk because / is mounted from it"
  if mp=$(system_mountpoint "$name"); then
    die "refusing $disk because $mp is mounted from it"
  fi
}

copy_out() {
  local stamp_dir stamp product slug dest out found=0
  out=$root/support/arch-probe/out
  mkdir -p "$out"
  shopt -s nullglob
  for stamp_dir in "$mnt"/arch-probe-out/*; do
    [[ -d $stamp_dir ]] || continue
    stamp=$(basename "$stamp_dir")
    product=$(field product "$stamp_dir/host.txt")
    if [[ -z $product ]]; then
      product=$(field vendor "$stamp_dir/host.txt")
    fi
    slug=$(slugify "$product")
    dest=$out/${slug}_${stamp}
    case $dest in
      "$out"/*) ;;
      *) die "refusing to replace $dest" ;;
    esac
    rm -rf "$dest"
    mkdir -p "$dest"
    cp -r --no-preserve=ownership,mode "$stamp_dir"/. "$dest"/
    bash "$here/summarize.sh" "$dest"
    echo "read-arch-probe: $dest/board.txt"
    found=1
  done
  shopt -u nullglob
  [[ $found -eq 1 ]] || die "arch-probe-out on $node is empty"
}

command -v lsblk >/dev/null 2>&1 || die "missing lsblk"
command -v findmnt >/dev/null 2>&1 || die "missing findmnt"

node=
if [[ -n $dev ]]; then
  resolve_node "$dev"
else
  choose_capture
fi

mnt=$(mktemp -d)
run_root mount -o ro "$node" "$mnt"
mounted=1
if [[ ! -d $mnt/arch-probe-out ]]; then
  die "no arch-probe-out on $node. Run probe.sh on the desk first"
fi
copy_out
