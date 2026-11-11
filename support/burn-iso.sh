#!/bin/bash
# Write the lerux ISO onto one USB stick.
#
# The default image is the Acer Swift X ISO from `just iso`:
# build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso
#
# With no device, the command lists USB sticks and asks for a number. Passing
# /dev/sdX uses that stick and skips the list. Either way the stick is erased.
#
# The disk that holds /, /boot, or /home is refused. A crypt root is followed
# through its sysfs slaves, because lsblk's parent of that mount source is empty.

set -euo pipefail

die() {
  echo "burn-iso: $*" >&2
  exit 1
}

usage() {
  echo "usage: just burn-iso" >&2
  echo "       just burn-iso /dev/sdX" >&2
  echo "With no device, USB sticks are listed and the stick you number is erased." >&2
  echo "The image is build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso from just iso." >&2
}

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)

image=$root/build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso
dev=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --image)
      [[ $# -ge 2 ]] || die "--image needs a path"
      image=$2
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    -*)
      die "unknown option $1"
      ;;
    *)
      [[ -z "$dev" ]] || die "only one device can be given"
      dev=$1
      shift
      ;;
  esac
done

declare -A host_disk_mounts=()
declare -A walk_seen=()
declare -a found_disks=()

require_tools() {
  local cmd
  for cmd in lsblk findmnt dd find awk; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing $cmd"
  done
}

reset_walk() {
  walk_seen=()
  found_disks=()
}

note_disk() {
  local name=$1 existing
  for existing in "${found_disks[@]+"${found_disks[@]}"}"; do
    [[ "$existing" == "$name" ]] && return 0
  done
  found_disks+=("$name")
}

walk_backing() {
  local name=$1 slave pk found=0
  [[ -z "${walk_seen[$name]+x}" ]] || return 0
  walk_seen[$name]=1
  if [[ -d "/sys/class/block/$name/slaves" ]]; then
    while read -r slave; do
      [[ -n "$slave" ]] || continue
      found=1
      walk_backing "$slave"
    done < <(find "/sys/class/block/$name/slaves" -mindepth 1 -maxdepth 1 -printf '%f\n')
  fi
  [[ "$found" -eq 0 ]] || return 0
  pk=$(lsblk -dn -o PKNAME "/dev/$name" 2>/dev/null | awk '{$1=$1; print}') || true
  if [[ -n "${pk:-}" && "$pk" != "$name" ]]; then
    walk_backing "$pk"
    return 0
  fi
  note_disk "$name"
}

backing_of() {
  local src=$1 name
  reset_walk
  src=${src%%\[*}
  [[ "$src" == /dev/* && -e "$src" ]] || return 0
  name=$(basename "$(readlink -f "$src")")
  walk_backing "$name"
}

remember_host_disk() {
  local name=$1 mount=$2
  if [[ -z "${host_disk_mounts[$name]+x}" ]]; then
    host_disk_mounts[$name]=$mount
    return 0
  fi
  [[ ", ${host_disk_mounts[$name]}, " == *", ${mount}, "* ]] && return 0
  host_disk_mounts[$name]="${host_disk_mounts[$name]}, ${mount}"
}

load_host_disks() {
  local target src disk
  host_disk_mounts=()
  while read -r target src; do
    [[ -n "$target" && -n "$src" ]] || continue
    case "$target" in
      / | /boot | /boot/* | /home | /home/*) ;;
      *) continue ;;
    esac
    backing_of "$src"
    for disk in "${found_disks[@]+"${found_disks[@]}"}"; do
      remember_host_disk "$disk" "$target"
    done
  done < <(findmnt -rno TARGET,SOURCE)
}

holds_host_os() {
  local disk
  for disk in "${found_disks[@]+"${found_disks[@]}"}"; do
    if [[ -n "${host_disk_mounts[$disk]+x}" ]]; then
      printf '%s\n' "${host_disk_mounts[$disk]}"
      return 0
    fi
  done
  return 1
}

validate_device() {
  local type tran mounts
  [[ -b "$dev" ]] || die "$dev is not a block device"
  dev=$(readlink -f "$dev")
  backing_of "$dev"
  [[ ${#found_disks[@]} -gt 0 ]] || die "$dev is not a block device"
  if mounts=$(holds_host_os); then
    die "refusing $dev because it holds the host OS ($mounts)"
  fi
  type=$(lsblk -dn -o TYPE "$dev" | awk '{$1=$1; print}')
  tran=$(lsblk -dn -o TRAN "$dev" | awk '{$1=$1; print}')
  [[ "$type" == "disk" ]] || die "pass the whole stick ($dev is ${type:-unknown})"
  [[ "$tran" == "usb" ]] || die "refusing $dev (transport is ${tran:-not usb})"
}

pair_value() {
  local line=$1 key=$2
  local re="(^| )${key}=\"([^\"]*)\""
  [[ "$line" =~ $re ]] || return 1
  printf '%s\n' "${BASH_REMATCH[2]}"
}

human_bytes() {
  awk -v b="$1" 'BEGIN {
    if (b >= 1024 * 1024 * 1024) printf "%.1fG", b / 1024 / 1024 / 1024
    else if (b >= 1024 * 1024) printf "%.0fM", b / 1024 / 1024
    else printf "%dB", b
  }'
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
    IFS= read -r line < /dev/tty || return 1
  elif [[ -t 0 ]]; then
    IFS= read -r line || return 1
  else
    return 1
  fi
  printf '%s\n' "$line"
}

collect_usb() {
  local line name tran type size model disk host
  usb_paths=()
  usb_sizes=()
  usb_models=()
  while read -r line; do
    [[ -n "$line" ]] || continue
    name=$(pair_value "$line" NAME) || continue
    tran=$(pair_value "$line" TRAN) || tran=
    type=$(pair_value "$line" TYPE) || continue
    size=$(pair_value "$line" SIZE) || continue
    [[ "$type" == "disk" && "$tran" == "usb" ]] || continue
    [[ "$size" =~ ^[0-9]+$ ]] || continue
    (( size > 0 )) || continue
    backing_of "/dev/$name"
    host=0
    for disk in "${found_disks[@]+"${found_disks[@]}"}"; do
      if [[ -n "${host_disk_mounts[$disk]+x}" ]]; then
        host=1
      fi
    done
    [[ "$host" -eq 0 ]] || continue
    model=$(lsblk -dbno MODEL "/dev/$name" 2>/dev/null | awk '{$1=$1; print}') || model=
    usb_paths+=("/dev/$name")
    usb_sizes+=("$size")
    usb_models+=("$model")
  done < <(lsblk -dbP -o NAME,TRAN,TYPE,SIZE)
}

choose_device() {
  local choice i index
  if ! have_terminal; then
    die "name the stick: just burn-iso /dev/sdX"
  fi
  collect_usb
  if [[ ${#usb_paths[@]} -eq 0 ]]; then
    echo "burn-iso: plug in the USB stick" >&2
    while [[ ${#usb_paths[@]} -eq 0 ]]; do
      sleep 1
      collect_usb
    done
  fi
  while true; do
    echo "burn-iso: USB sticks" >&2
    for i in "${!usb_paths[@]}"; do
      printf '  %d  %s  %s  %s\n' "$((i + 1))" "${usb_paths[$i]}" "$(human_bytes "${usb_sizes[$i]}")" "${usb_models[$i]}" >&2
    done
    echo "burn-iso: type a number to erase that stick, or q to stop" >&2
    choice=$(read_answer) || die "stopped"
    choice=$(trim_answer "$choice")
    [[ "$choice" == "q" ]] && die "stopped"
    if [[ "$choice" =~ ^[0-9]{1,4}$ ]]; then
      index=$((10#$choice))
      if (( index >= 1 && index <= ${#usb_paths[@]} )); then
        dev=${usb_paths[$((index - 1))]}
        echo "burn-iso: selected $dev" >&2
        return 0
      fi
    fi
    for i in "${!usb_paths[@]}"; do
      if [[ "$choice" == "${usb_paths[$i]}" ]]; then
        dev=$choice
        echo "burn-iso: selected $dev" >&2
        return 0
      fi
    done
    echo "burn-iso: type a number from the list" >&2
    collect_usb
    if [[ ${#usb_paths[@]} -eq 0 ]]; then
      echo "burn-iso: plug in the USB stick" >&2
      while [[ ${#usb_paths[@]} -eq 0 ]]; do
        sleep 1
        collect_usb
      done
    fi
  done
}

stick_bytes() {
  lsblk -dbno SIZE "$dev"
}

confirm_erase() {
  echo "burn-iso: this erases $dev:"
  lsblk -o NAME,SIZE,TRAN,MODEL,LABEL,MOUNTPOINTS "$dev"
  echo "burn-iso: type $dev to continue"
  local typed
  typed=$(read_answer) || die "stopped"
  typed=$(trim_answer "$typed")
  [[ "$typed" == "$dev" ]] || die "not confirmed"
}

unmount_disk() {
  local node mp
  while read -r node; do
    [[ -n "$node" ]] || continue
    while read -r mp; do
      [[ -n "$mp" ]] || continue
      umount "$mp"
    done < <(findmnt -rno TARGET "$node" 2>/dev/null || true)
  done < <(lsblk -lnpo NAME "$dev")
}

require_image() {
  [[ -f "$image" ]] || die "image not found: $image (run: just iso)"
  [[ -s "$image" ]] || die "image is empty: $image"
}

require_room() {
  local image_bytes device_bytes
  image_bytes=$(stat -c %s "$image")
  device_bytes=$(stick_bytes)
  if (( device_bytes < image_bytes )); then
    die "stick is too small: the image is $image_bytes bytes and $dev is $device_bytes bytes"
  fi
}

write_image() {
  echo "burn-iso: writing $image"
  unmount_disk
  dd if="$image" of="$dev" bs=4M conv=fsync status=progress
  sync
  blockdev --flushbufs "$dev" 2>/dev/null || true
  blockdev --rereadpt "$dev" 2>/dev/null || true
  echo "burn-iso: wrote $image to $dev"
}

require_tools
load_host_disks
if [[ -n "$dev" ]]; then
  validate_device
fi
require_image
if [[ -z "$dev" ]]; then
  choose_device
  validate_device
fi
require_room
if [[ "$(id -u)" -ne 0 ]]; then
  echo "burn-iso: asking for root to write $dev" >&2
  exec sudo bash "$0" --image "$image" "$dev"
fi
confirm_erase
write_image
