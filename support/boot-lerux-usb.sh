#!/bin/bash
# One-shot UEFI boot of the Lerux USB stick already plugged into this machine.
#
# Sets BootNext only. BootOrder is left alone (or restored after creating an
# entry), so the next power-on after Lerux returns to the host OS (Omarchy /
# Limine on this desk).
#
# With no device, lists USB sticks and asks for a number. Pass /dev/sdX to skip
# the list. Pass --no-reboot to set BootNext and stop.

set -euo pipefail

die() {
  echo "boot-lerux-usb: $*" >&2
  exit 1
}

usage() {
  echo "usage: just boot-lerux-usb" >&2
  echo "       just boot-lerux-usb /dev/sdX" >&2
  echo "       just boot-lerux-usb --no-reboot" >&2
  echo "       just boot-lerux-usb /dev/sdX --no-reboot" >&2
  echo "Sets UEFI BootNext to the Lerux stick for one reboot, then reboots." >&2
  echo "Does not change the lasting BootOrder, so the host OS stays the default." >&2
}

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)

dev=
do_reboot=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    --no-reboot)
      do_reboot=0
      shift
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
declare -a usb_paths=()
declare -a usb_sizes=()
declare -a usb_models=()

LERUX_LABEL="Lerux ISO"
EFI_LOADER='\EFI\BOOT\BOOTX64.EFI'
# EFI System Partition GPT type GUID.
ESP_TYPE=c12a7328-f81f-11d2-ba4b-00a0c93ec93b

require_tools() {
  local cmd
  for cmd in lsblk findmnt efibootmgr awk find; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing $cmd (on Arch: sudo pacman -S --needed efibootmgr util-linux)"
  done
}

require_uefi() {
  [[ -d /sys/firmware/efi ]] || die "this machine is not in UEFI mode"
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
    IFS= read -r line </dev/tty || return 1
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
    die "name the stick: just boot-lerux-usb /dev/sdX"
  fi
  collect_usb
  if [[ ${#usb_paths[@]} -eq 0 ]]; then
    echo "boot-lerux-usb: plug in the USB stick" >&2
    while [[ ${#usb_paths[@]} -eq 0 ]]; do
      sleep 1
      collect_usb
    done
  fi
  while true; do
    echo "boot-lerux-usb: USB sticks" >&2
    for i in "${!usb_paths[@]}"; do
      printf '  %d  %s  %s  %s\n' "$((i + 1))" "${usb_paths[$i]}" "$(human_bytes "${usb_sizes[$i]}")" "${usb_models[$i]}" >&2
    done
    echo "boot-lerux-usb: type a number to boot that stick once, or q to stop" >&2
    choice=$(read_answer) || die "stopped"
    choice=$(trim_answer "$choice")
    [[ "$choice" == "q" ]] && die "stopped"
    if [[ "$choice" =~ ^[0-9]{1,4}$ ]]; then
      index=$((10#$choice))
      if (( index >= 1 && index <= ${#usb_paths[@]} )); then
        dev=${usb_paths[$((index - 1))]}
        echo "boot-lerux-usb: selected $dev" >&2
        return 0
      fi
    fi
    for i in "${!usb_paths[@]}"; do
      if [[ "$choice" == "${usb_paths[$i]}" ]]; then
        dev=$choice
        echo "boot-lerux-usb: selected $dev" >&2
        return 0
      fi
    done
    echo "boot-lerux-usb: type a number from the list" >&2
    collect_usb
    if [[ ${#usb_paths[@]} -eq 0 ]]; then
      echo "boot-lerux-usb: plug in the USB stick" >&2
      while [[ ${#usb_paths[@]} -eq 0 ]]; do
        sleep 1
        collect_usb
      done
    fi
  done
}

# Print PARTUUID values for a partition (kernel by-partuuid and blkid can differ
# on an isohybrid ISO).
part_uuids() {
  local part=$1 uuid
  uuid=$(lsblk -no PARTUUID "$part" 2>/dev/null | awk 'NF{print; exit}') || true
  [[ -n "${uuid:-}" ]] && printf '%s\n' "${uuid,,}"
  if command -v blkid >/dev/null 2>&1; then
    uuid=$(blkid -s PARTUUID -o value "$part" 2>/dev/null | awk 'NF{print; exit}') || true
    [[ -n "${uuid:-}" ]] && printf '%s\n' "${uuid,,}"
  fi
}

find_esp() {
  local line name type parttype fstype label part
  esp=
  esp_part=
  while read -r line; do
    [[ -n "$line" ]] || continue
    name=$(pair_value "$line" NAME) || continue
    type=$(pair_value "$line" TYPE) || continue
    [[ "$type" == "part" ]] || continue
    part="/dev/$name"
    parttype=$(pair_value "$line" PARTTYPE) || parttype=
    fstype=$(pair_value "$line" FSTYPE) || fstype=
    label=$(pair_value "$line" LABEL) || label=
    if [[ "${parttype,,}" == "$ESP_TYPE" ]]; then
      esp=$part
      break
    fi
    if [[ "${fstype,,}" == "vfat" && ("${label,,}" == "lerux" || "${label,,}" == "efi" || "${label,,}" == "esp") ]]; then
      esp=$part
      break
    fi
  done < <(lsblk -bP -o NAME,TYPE,PARTTYPE,FSTYPE,LABEL "$dev")
  [[ -n "${esp:-}" ]] || die "no EFI system partition on $dev (is this a Lerux ISO stick?)"
  if [[ "$esp" =~ ^${dev}p([0-9]+)$ ]]; then
    esp_part=${BASH_REMATCH[1]}
  elif [[ "$esp" =~ ^${dev}([0-9]+)$ ]]; then
    esp_part=${BASH_REMATCH[1]}
  else
    die "cannot read partition number of $esp"
  fi
}

# Print "unset" when firmware has no BootOrder. Print the comma list when it
# does. Fail when the dump has neither, so a missed line cannot become a guess.
parse_boot_order_dump() {
  local line value
  line=$(awk '/^BootOrder:/{print; exit} /^No BootOrder is set/{print; exit}')
  if [[ "$line" == "No BootOrder is set"* ]]; then
    printf 'unset\n'
    return 0
  fi
  if [[ "$line" == BootOrder:* ]]; then
    value=${line#BootOrder:}
    value=${value#"${value%%[![:space:]]*}"}
    value=${value%"${value##*[![:space:]]}"}
    if [[ -z "$value" ]]; then
      printf 'unset\n'
    else
      printf '%s\n' "$value"
    fi
    return 0
  fi
  return 2
}

boot_order_state() {
  local dump
  dump=$(efibootmgr) || return 2
  printf '%s\n' "$dump" | parse_boot_order_dump
}

# Print the BootOrder to write, with NEU last. Print nothing when the firmware
# had no BootOrder. An empty snapshot must not become a one-entry order.
restored_boot_order() {
  local saved=$1 neu=$2
  if [[ "$saved" == unset || -z "$saved" ]]; then
    return 0
  fi
  awk -v keep="$saved" -v neu="$neu" 'BEGIN {
    n = split(keep, a, ",")
    out = ""
    for (i = 1; i <= n; i++) {
      gsub(/^[ \t]+|[ \t]+$/, "", a[i])
      if (a[i] == "" || tolower(a[i]) == tolower(neu)) continue
      out = (out == "" ? a[i] : out "," a[i])
    }
    if (out == "") out = neu
    else out = out "," neu
    print out
  }'
}

# Print Boot#### numbers whose verbose path mentions any of the given PARTUUIDs
# and preferably loads BOOTX64.EFI.
find_entry_for_uuids() {
  local uuid line num path prefer=() any=()
  local -a uuids=("$@")
  while IFS= read -r line; do
    [[ "$line" =~ ^Boot([0-9A-Fa-f]+) ]] || continue
    num=${BASH_REMATCH[1]}
    path=${line#*$'\t'}
    [[ "$path" == "$line" ]] && path=${line#*[[:space:]]}
    for uuid in "${uuids[@]}"; do
      [[ -n "$uuid" ]] || continue
      if [[ "${path,,}" == *"${uuid,,}"* ]]; then
        if [[ "${path,,}" == *bootx64.efi* ]]; then
          prefer+=("$num")
        else
          any+=("$num")
        fi
        break
      fi
    done
  done < <(efibootmgr -v 2>/dev/null || efibootmgr)
  if [[ ${#prefer[@]} -gt 0 ]]; then
    printf '%s\n' "${prefer[0]}"
    return 0
  fi
  if [[ ${#any[@]} -gt 0 ]]; then
    printf '%s\n' "${any[0]}"
    return 0
  fi
  return 1
}

entries_named_lerux() {
  efibootmgr | sed -nE 's/^Boot([0-9A-Fa-f]+)\*?[[:space:]]+Lerux ISO[[:space:]]*$/\1/p'
}

as_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  else
    echo "boot-lerux-usb: asking for root for $*" >&2
    sudo "$@"
  fi
}

ensure_entry() {
  local -a uuids=()
  local uuid bootnum created order restored num
  mapfile -t uuids < <(part_uuids "$esp" | awk 'NF' | sort -u)
  [[ ${#uuids[@]} -gt 0 ]] || die "no PARTUUID for $esp"

  if ! order=$(boot_order_state); then
    die "cannot read BootOrder"
  fi

  if bootnum=$(find_entry_for_uuids "${uuids[@]}"); then
    echo "boot-lerux-usb: using existing Boot${bootnum}" >&2
  else
    echo "boot-lerux-usb: creating EFI entry for $dev partition $esp_part" >&2
    # Drop stale label-only entries that no longer match this stick.
    while read -r num; do
      [[ -n "$num" ]] || continue
      as_root efibootmgr --bootnum "$num" --delete-bootnum >/dev/null
    done < <(entries_named_lerux)

    # --create inserts the new number at the front of BootOrder. --create-only
    # leaves an unset BootOrder unset.
    if [[ "$order" == unset ]]; then
      as_root efibootmgr --create-only \
        --disk "$dev" \
        --part "$esp_part" \
        --label "$LERUX_LABEL" \
        --loader "$EFI_LOADER" >/dev/null
    else
      as_root efibootmgr --create \
        --disk "$dev" \
        --part "$esp_part" \
        --label "$LERUX_LABEL" \
        --loader "$EFI_LOADER" >/dev/null
    fi
    bootnum=$(entries_named_lerux | tail -n1)
    [[ -n "$bootnum" ]] || die "efibootmgr did not report a new Boot number"
  fi

  # Put the entry last when a snapshot exists, so Omarchy or Limine stays the
  # lasting default. An unset snapshot must not become a one-entry BootOrder.
  restored=$(restored_boot_order "$order" "$bootnum")
  if [[ -n "$restored" ]]; then
    as_root efibootmgr --bootorder "$restored" >/dev/null
    echo "boot-lerux-usb: restored BootOrder ($restored)" >&2
  else
    echo "boot-lerux-usb: BootOrder stays unset" >&2
  fi

  printf '%s\n' "$bootnum"
}

confirm_reboot() {
  local typed
  echo "boot-lerux-usb: next reboot is Lerux on $dev once"
  echo "boot-lerux-usb: the lasting BootOrder stays this host OS (Omarchy)"
  echo "boot-lerux-usb: type reboot to continue"
  typed=$(read_answer) || die "stopped"
  typed=$(trim_answer "$typed")
  [[ "$typed" == "reboot" ]] || die "not confirmed"
}

require_tools
require_uefi
load_host_disks
if [[ -n "$dev" ]]; then
  validate_device
else
  choose_device
  validate_device
fi
find_esp
echo "boot-lerux-usb: ESP is $esp (partition $esp_part)" >&2

bootnum=$(ensure_entry)

if [[ "$do_reboot" -eq 0 ]]; then
  echo "boot-lerux-usb: BootNext -> Boot${bootnum} ($LERUX_LABEL)" >&2
  as_root efibootmgr --bootnext "$bootnum" >/dev/null
  echo "boot-lerux-usb: BootNext set; reboot when ready (stick stays default only for that one boot)"
  exit 0
fi

if ! have_terminal; then
  die "no terminal to confirm reboot (pass --no-reboot, then reboot yourself)"
fi
confirm_reboot

echo "boot-lerux-usb: BootNext -> Boot${bootnum} ($LERUX_LABEL)" >&2
as_root efibootmgr --bootnext "$bootnum" >/dev/null
echo "boot-lerux-usb: rebooting" >&2
as_root systemctl reboot
