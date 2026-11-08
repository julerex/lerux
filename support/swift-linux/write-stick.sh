#!/bin/bash
# Write the current Arch install image and capture.sh onto one USB stick.
#
# With no device, the command lists USB sticks and asks for a number. Passing
# /dev/sdX uses that stick and skips the list. Either way the stick is erased.
#
# The image is a DOS partition table: partition 1 is the ISO, partition 2 is
# the small EFI system partition. The rest of the stick becomes partition 3,
# a FAT volume labelled CAPTURE. sfdisk is told not to wipe signatures, because
# the ISO9660 signature is how the installer finds its own files.
#
# A later image with a GPT table is handled by moving the backup header to the
# end of the stick before the same extra partition is added.

set -euo pipefail

mirror=https://geo.mirror.pkgbuild.com/iso/latest
# One extra GiB on the stick. Alignment takes a little of that, so the
# formatted partition is accepted at 512 MiB, which covers the trace.
min_stick_extra=$((1024 * 1024 * 1024))
min_partition=$((512 * 1024 * 1024))

die() {
  echo "iso-arch-and-capture: $*" >&2
  exit 1
}

usage() {
  echo "usage: just iso-arch-and-capture" >&2
  echo "       just iso-arch-and-capture /dev/sdX" >&2
  echo "With no device, USB sticks are listed and the stick you number is erased." >&2
}

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

image=
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

layout_test=0
if [[ "${SWIFT_LINUX_LAYOUT_TEST:-}" == 1 ]]; then
  layout_test=1
fi

require_tools() {
  local cmd
  for cmd in curl sfdisk mkfs.vfat sha256sum gpg lsblk findmnt partx; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing $cmd"
  done
}

root_disk_name() {
  local src pk
  src=$(findmnt -no SOURCE /)
  pk=$(lsblk -no PKNAME "$src" 2>/dev/null || true)
  if [[ -n "$pk" ]]; then
    printf '%s\n' "$pk"
  else
    basename "$src"
  fi
}

validate_device() {
  local tran type name root_name
  [[ -b "$dev" ]] || die "$dev is not a block device"
  dev=$(readlink -f "$dev")
  type=$(lsblk -dn -o TYPE "$dev" | awk '{ print $1 }')
  tran=$(lsblk -dn -o TRAN "$dev" | awk '{ print $1 }')
  name=$(basename "$dev")
  root_name=$(root_disk_name)
  [[ "$name" != "$root_name" ]] || die "refusing $dev because / is mounted from it"
  if [[ "$layout_test" -eq 1 ]]; then
    [[ "$type" == "loop" ]] || die "the layout test only runs on a loop device"
    return 0
  fi
  [[ "$type" == "disk" ]] || die "pass the whole stick ($dev is $type)"
  [[ "$tran" == "usb" ]] || die "refusing $dev (transport is ${tran:-not usb})"
}

system_mountpoint() {
  local name=$1 src pk disk target
  while read -r target src; do
    [[ -n "$src" ]] || continue
    pk=$(lsblk -no PKNAME "$src" 2>/dev/null || true)
    disk=${pk:-$(basename "$src")}
    [[ "$disk" == "$name" ]] || continue
    case "$target" in
      / | /boot | /boot/* | /home | /home/*)
        printf '%s\n' "$target"
        return 0
        ;;
    esac
  done < <(findmnt -rno TARGET,SOURCE)
  return 1
}

holds_system_mount() {
  local name=$1 mp
  if mp=$(system_mountpoint "$name"); then
    die "refusing $dev because $mp is mounted from it"
  fi
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
  local line name tran type size model
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
    [[ "$name" != "$(root_disk_name)" ]] || continue
    if system_mountpoint "$name" >/dev/null; then
      continue
    fi
    model=$(lsblk -dbno MODEL "/dev/$name" 2>/dev/null | awk '{$1=$1; print}') || model=
    usb_paths+=("/dev/$name")
    usb_sizes+=("$size")
    usb_models+=("$model")
  done < <(lsblk -dbP -o NAME,TRAN,TYPE,SIZE)
}

choose_device() {
  local choice i index
  if ! have_terminal; then
    die "name the stick: just iso-arch-and-capture /dev/sdX"
  fi
  collect_usb
  if [[ ${#usb_paths[@]} -eq 0 ]]; then
    echo "iso-arch-and-capture: plug in the USB stick" >&2
    while [[ ${#usb_paths[@]} -eq 0 ]]; do
      sleep 1
      collect_usb
    done
  fi
  while true; do
    echo "iso-arch-and-capture: USB sticks" >&2
    for i in "${!usb_paths[@]}"; do
      printf '  %d  %s  %s  %s\n' "$((i + 1))" "${usb_paths[$i]}" "$(human_bytes "${usb_sizes[$i]}")" "${usb_models[$i]}" >&2
    done
    echo "iso-arch-and-capture: type a number to erase that stick, or q to stop" >&2
    choice=$(read_answer) || die "stopped"
    choice=$(trim_answer "$choice")
    [[ "$choice" == "q" ]] && die "stopped"
    if [[ "$choice" =~ ^[0-9]{1,4}$ ]]; then
      index=$((10#$choice))
      if (( index >= 1 && index <= ${#usb_paths[@]} )); then
        dev=${usb_paths[$((index - 1))]}
        echo "iso-arch-and-capture: selected $dev" >&2
        return 0
      fi
    fi
    for i in "${!usb_paths[@]}"; do
      if [[ "$choice" == "${usb_paths[$i]}" ]]; then
        dev=$choice
        echo "iso-arch-and-capture: selected $dev" >&2
        return 0
      fi
    done
    echo "iso-arch-and-capture: type a number from the list" >&2
    collect_usb
    if [[ ${#usb_paths[@]} -eq 0 ]]; then
      echo "iso-arch-and-capture: plug in the USB stick" >&2
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

curl_get() {
  local url=$1 dest=$2
  local tmp="${dest}.partial"
  local args=(-fL --retry 5 --retry-delay 2 --retry-all-errors --connect-timeout 20)
  if [[ -f "$tmp" ]]; then
    args+=(--continue-at -)
  fi
  curl "${args[@]}" -o "$tmp" "$url"
  mv "$tmp" "$dest"
}

dated_iso_name() {
  awk '$2 ~ /^archlinux-[0-9]{4}\.[0-9]{2}\.[0-9]{2}-x86_64\.iso$/ { print $2; exit }' "$1"
}

file_sha256() {
  sha256sum "$1" | awk '{ print $1 }'
}

fetch_iso() {
  local cache="$root/build/archiso"
  local sums name hash iso sig actual attempt
  mkdir -p "$cache"
  echo "iso-arch-and-capture: downloading the checksum list" >&2
  curl_get "$mirror/sha256sums.txt" "$cache/sha256sums.txt"
  name=$(dated_iso_name "$cache/sha256sums.txt")
  [[ -n "$name" ]] || die "sha256sums.txt has no dated Arch iso"
  hash=$(awk -v name="$name" '$2 == name { print $1; exit }' "$cache/sha256sums.txt")
  [[ -n "$hash" ]] || die "sha256sums.txt has no hash for $name"
  iso="$cache/$name"
  sig="$cache/$name.sig"
  if [[ -f "$iso" && "$(file_sha256 "$iso")" == "$hash" ]]; then
    echo "iso-arch-and-capture: using cached $iso" >&2
  else
    rm -f "$iso"
    for attempt in 1 2; do
      echo "iso-arch-and-capture: downloading $name" >&2
      curl_get "$mirror/$name" "$iso"
      actual=$(file_sha256 "$iso")
      if [[ "$actual" == "$hash" ]]; then
        break
      fi
      rm -f "$iso"
      [[ "$attempt" -eq 1 ]] || die "checksum mismatch for $name"
      echo "iso-arch-and-capture: checksum mismatch, downloading again" >&2
    done
  fi
  echo "iso-arch-and-capture: checking the signature" >&2
  curl_get "$mirror/$name.sig" "$sig"
  if ! gpg --batch --no-tty --trust-model always --homedir /etc/pacman.d/gnupg --verify "$sig" "$iso"; then
    die "signature check failed. Update archlinux-keyring and run this again"
  fi
  printf '%s\n' "$iso"
}

confirm_erase() {
  echo "iso-arch-and-capture: this erases $dev:"
  lsblk -o NAME,SIZE,TRAN,MODEL,LABEL,MOUNTPOINTS "$dev"
  echo "iso-arch-and-capture: type $dev to continue"
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

disk_label() {
  sfdisk -d "$1" | awk '/^label:/ { print $2; exit }'
}

partition_count() {
  local disk=$1
  sfdisk -d "$disk" | awk -v disk="$disk" 'index($0, disk) == 1 && / : start=/ { n++ } END { print n+0 }'
}

partition_node() {
  local disk=$1 num=$2
  if [[ "$disk" =~ [0-9]$ ]]; then
    printf '%sp%s\n' "$disk" "$num"
  else
    printf '%s%s\n' "$disk" "$num"
  fi
}

add_capture_partition() {
  local label count
  label=$(disk_label "$dev")
  count=$(partition_count "$dev")
  case "$label" in
    dos)
      [[ "$count" -lt 4 ]] || die "the image already has four DOS partitions"
      printf '%s\n' 'type=c' | sfdisk --append --wipe never "$dev" >&2
      ;;
    gpt)
      sfdisk --relocate gpt-bak-std "$dev" >&2
      printf '%s\n' 'type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7, name="CAPTURE"' |
        sfdisk --append --wipe never "$dev" >&2
      ;;
    *)
      die "unrecognised partition table ${label:-missing}"
      ;;
  esac
  partition_node "$dev" $((count + 1))
}

wait_for_node() {
  local node=$1 i
  for i in 1 2 3 4 5 6 7 8 9 10; do
    partx -u "$dev" 2>/dev/null || partx -a "$dev" 2>/dev/null || true
    udevadm settle 2>/dev/null || true
    [[ -b "$node" ]] && return 0
    sleep 0.2
  done
  die "partition $node did not appear"
}

write_boot_note() {
  local dest=$1
  cat > "$dest/BOOT.txt" <<'EOF'
Arch is on this stick. These files are on the partition labelled CAPTURE.
Leave the stick plugged in for the whole capture.

At the Arch menu, press e and add this to the end of the kernel line:

    modprobe.blacklist=mt7921e

Press Enter to boot. At the root prompt:

    mkdir -p /mnt/stick
    mount /dev/disk/by-label/CAPTURE /mnt/stick
    bash /mnt/stick/capture.sh

The last line is the verdict. The results are swift-linux-out on this
same partition. Then run:

    poweroff
EOF
}

write_stick() {
  local node bytes mnt
  echo "iso-arch-and-capture: writing the image"
  unmount_disk
  dd if="$image" of="$dev" bs=4M conv=fsync status=progress
  sync
  blockdev --flushbufs "$dev" 2>/dev/null || true
  node=$(add_capture_partition)
  wait_for_node "$node"
  bytes=$(lsblk -dbno SIZE "$node")
  [[ "$bytes" -ge "$min_partition" ]] || die "capture partition is only $bytes bytes; use a larger stick"
  echo "iso-arch-and-capture: formatting $node"
  mkfs.vfat -F 32 -n CAPTURE "$node"
  mnt=$(mktemp -d)
  mount -t vfat "$node" "$mnt"
  cp "$here/capture.sh" "$here/README.md" "$mnt/"
  write_boot_note "$mnt"
  sync
  umount "$mnt"
  rmdir "$mnt"
  echo "iso-arch-and-capture: wrote CAPTURE on $node"
  echo "iso-arch-and-capture: on the Acer, add modprobe.blacklist=mt7921e, then:"
  echo "    mount /dev/disk/by-label/CAPTURE /mnt/stick"
  echo "    bash /mnt/stick/capture.sh"
}

require_tools
if [[ -z "$dev" ]]; then
  if [[ "$layout_test" -eq 1 ]]; then
    die "the layout test needs a loop device"
  fi
  choose_device
fi
validate_device
holds_system_mount "$(basename "$dev")"

if [[ -z "$image" ]]; then
  if [[ "$layout_test" -eq 1 ]]; then
    die "the layout test needs --image"
  fi
  image=$(fetch_iso)
fi
[[ -f "$image" ]] || die "image not found: $image"

image_bytes=$(stat -c %s "$image")
device_bytes=$(stick_bytes)
if (( device_bytes < image_bytes + min_stick_extra )); then
  die "stick is too small: the image is $image_bytes bytes and the stick needs 1 GiB more"
fi

if [[ "$(id -u)" -ne 0 ]]; then
  echo "iso-arch-and-capture: asking for root to write $dev" >&2
  exec sudo --preserve-env=SWIFT_LINUX_LAYOUT_TEST bash "$0" --image "$image" "$dev"
fi

if [[ "$layout_test" -eq 0 ]]; then
  confirm_erase
fi
write_stick
