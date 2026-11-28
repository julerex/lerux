#!/bin/bash
# Record the firmware and hardware a new x86 desk needs, from the Arch install medium.
#
# Loads no modules and mounts no internal disk. nomodeset on the kernel line
# leaves the EFI framebuffer in place, which is the buffer mb2-stub paints.
# A second boot writes a new directory under arch-probe-out.

set -euo pipefail

work=/run/arch-probe

die() {
  echo "probe: $*" >&2
  exit 1
}

need_root() {
  [[ $(id -u) -eq 0 ]] || die "run as root"
}

trim() {
  local s=$1
  s=${s#"${s%%[![:space:]]*}"}
  s=${s%"${s##*[![:space:]]}"}
  printf '%s\n' "$s"
}

write_host() {
  local key file value
  while read -r key file; do
    [[ -n $key ]] || continue
    [[ -r /sys/class/dmi/id/$file ]] || continue
    value=$(trim "$(tr -d '\n' < "/sys/class/dmi/id/$file")")
    [[ -n $value ]] || continue
    printf '%s: %s\n' "$key" "$value"
  done <<'EOF'
vendor sys_vendor
product product_name
board_vendor board_vendor
board_name board_name
bios_version bios_version
bios_date bios_date
EOF
} > "$work/host.txt"

write_cpu() {
  awk '
    BEGIN { n = 0 }
    /^$/ { if (n == 1) exit }
    /^processor[[:space:]]*:/ { n++; next }
    n != 1 { next }
    /^vendor_id[[:space:]]*:/ { sub(/^[^:]+:[[:space:]]*/, ""); vendor = $0; next }
    /^cpu family[[:space:]]*:/ { sub(/^[^:]+:[[:space:]]*/, ""); family = $0; next }
    /^model[[:space:]]*:/ { sub(/^[^:]+:[[:space:]]*/, ""); model = $0; next }
    /^model name[[:space:]]*:/ { sub(/^[^:]+:[[:space:]]*/, ""); name = $0; next }
    /^stepping[[:space:]]*:/ { sub(/^[^:]+:[[:space:]]*/, ""); stepping = $0; next }
    END {
      if (vendor != "") print "vendor_id: " vendor
      if (family != "") print "cpu family: " family
      if (model != "") print "model: " model
      if (name != "") print "model name: " name
      if (stepping != "") print "stepping: " stepping
    }
  ' /proc/cpuinfo > "$work/cpu.txt"
  local flags want hit
  flags=$(awk '/^flags[[:space:]]*:/ { print; exit }' /proc/cpuinfo)
  hit=
  for want in lm syscall nx constant_tsc nonstop_tsc tsc_known_freq x2apic hypervisor; do
    if [[ " $flags " == *" $want "* ]]; then
      hit="${hit:+$hit }$want"
    fi
  done
  if [[ -n $hit ]]; then
    printf 'flags: %s\n' "$hit" >> "$work/cpu.txt"
  fi
}

efi_last_byte() {
  local prefix=$1 file size
  local files=()
  shopt -s nullglob
  files=(/sys/firmware/efi/efivars/"${prefix}"-*)
  shopt -u nullglob
  [[ ${#files[@]} -ge 1 ]] || return 1
  file=${files[0]}
  [[ -r $file ]] || return 1
  size=$(stat -c %s "$file")
  ((size >= 5)) || return 1
  od -An -tu1 -j "$((size - 1))" -N 1 "$file" | awk '{ print $1; exit }'
}

write_secure() {
  local sb sm
  {
    if sb=$(efi_last_byte SecureBoot); then
      printf 'SecureBoot: %s\n' "$sb"
    else
      printf 'SecureBoot: unknown\n'
    fi
    if sm=$(efi_last_byte SetupMode); then
      printf 'SetupMode: %s\n' "$sm"
    fi
  } > "$work/secure-boot.txt"
}

write_e820() {
  local index start end type
  [[ -d /sys/firmware/memmap ]] || return 0
  : > "$work/e820.txt"
  while read -r index; do
    [[ -n $index ]] || continue
    [[ -r /sys/firmware/memmap/$index/start ]] || continue
    start=$(tr -d '[:space:]' < "/sys/firmware/memmap/$index/start")
    end=$(tr -d '[:space:]' < "/sys/firmware/memmap/$index/end")
    type=$(tr -d '\n' < "/sys/firmware/memmap/$index/type")
    printf '%s\t0x%x\t0x%x\n' "$type" "$((start))" "$((end))"
  done < <(find /sys/firmware/memmap -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -n)
}

write_framebuffer() {
  local name
  {
    if [[ -d /sys/class/graphics/fb0 ]]; then
      for name in name virtual_size stride bits_per_pixel; do
        if [[ -r /sys/class/graphics/fb0/$name ]]; then
          printf 'fb0_%s: %s\n' "$name" "$(tr -d '\n' < "/sys/class/graphics/fb0/$name")"
        fi
      done
    fi
    if [[ -r /proc/fb ]]; then
      echo "--- /proc/fb ---"
      cat /proc/fb
    fi
    if lsmod | awk '{ print $1 }' | grep -Eq '^(amdgpu|i915|nouveau|radeon)$'; then
      echo "modeset: yes"
    else
      echo "modeset: no"
    fi
    echo "--- dmesg ---"
    grep -E 'efifb|vesafb|simplefb' "$work/dmesg.txt" || true
    echo "--- iomem ---"
    grep -Ei 'framebuffer|efifb|vesafb' "$work/iomem.txt" || true
  } > "$work/framebuffer.txt"
}

write_pci() {
  local src bdf dest parent name
  shopt -s nullglob
  for src in /sys/bus/pci/devices/*; do
    bdf=$(basename "$src")
    [[ $bdf =~ ^[0-9a-fA-F]{4}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}\.[0-7]$ ]] || continue
    # CAPTURE is FAT. A colon is not a legal name there, so the two colons
    # in the bus address become hyphens. summarize.sh reads them back.
    dest="$work/pci/${bdf,,}"
    dest=${dest//:/-}
    mkdir -p "$dest"
    for name in vendor device class irq resource boot_vga enable; do
      if [[ -r $src/$name ]]; then
        cp "$src/$name" "$dest/$name"
      fi
    done
    if [[ -r $src/config ]]; then
      dd if="$src/config" of="$dest/config" bs=4096 count=1 status=none || true
    fi
    parent=$(basename "$(dirname "$(readlink -f "$src")")")
    if [[ $parent =~ ^[0-9a-fA-F]{4}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}\.[0-7]$ ]]; then
      printf '%s\n' "${parent,,}" > "$dest/parent"
    fi
  done
  shopt -u nullglob
}

write_input() {
  {
    if [[ -e /sys/bus/platform/devices/i8042 ]]; then
      echo "i8042: yes"
    else
      echo "i8042: no"
    fi
    if [[ -r /proc/bus/input/devices ]]; then
      echo "--- devices ---"
      cat /proc/bus/input/devices
    fi
  } > "$work/input.txt"
}

write_usb() {
  local dev base product
  : > "$work/usb.txt"
  shopt -s nullglob
  for dev in /sys/bus/usb/devices/*; do
    base=$(basename "$dev")
    if [[ -f $dev/idVendor && -f $dev/idProduct ]]; then
      product=
      if [[ -r $dev/product ]]; then
        product=$(tr -d '\n' < "$dev/product")
      fi
      printf 'device %s %s:%s %s\n' "$base" \
        "$(tr -d '[:space:]' < "$dev/idVendor")" \
        "$(tr -d '[:space:]' < "$dev/idProduct")" \
        "$product"
    fi
    if [[ -f $dev/bInterfaceClass ]]; then
      printf 'interface %s class %s subclass %s protocol %s\n' "$base" \
        "$(tr -d '[:space:]' < "$dev/bInterfaceClass")" \
        "$(if [[ -r $dev/bInterfaceSubClass ]]; then tr -d '[:space:]' < "$dev/bInterfaceSubClass"; fi)" \
        "$(if [[ -r $dev/bInterfaceProtocol ]]; then tr -d '[:space:]' < "$dev/bInterfaceProtocol"; fi)"
    fi
  done
  shopt -u nullglob
}

write_nvme() {
  local model
  : > "$work/nvme.txt"
  shopt -s nullglob
  for model in /sys/class/nvme/nvme*/model; do
    printf '%s %s\n' "$(basename "$(dirname "$model")")" "$(tr -d '\n' < "$model")"
  done
  shopt -u nullglob
}

write_about() {
  cat > "$work/about.txt" <<'EOF'
Desk probe. board.txt is the summary. The other files are the raw record.

host.txt is the DMI vendor, product, board, and BIOS. Serial numbers and UUIDs are not here.
cpu.txt is the first processor from /proc/cpuinfo.
secure-boot.txt is the SecureBoot and SetupMode bytes. The key databases are not here.
dmesg.txt, iomem.txt, and e820.txt are the firmware memory map. e820.txt comes from
/sys/firmware/memmap when the kernel exports it.
framebuffer.txt is fb0, the efifb or vesafb lines, and whether a modesetting driver
was loaded. nomodeset keeps the firmware buffer.
lspci-nnvvv.txt and lspci-xxxx.txt are the whole bus. pci/ is one directory per
device, with hyphens where the bus address has colons. Each directory holds
vendor, device, class, irq, resource, boot_vga, config (at most 4096 bytes),
and the parent bridge.
acpi/ is /sys/firmware/acpi/tables, including MADT, MCFG, HPET, FACP, and IVRS or
DMAR when the firmware publishes them.
irq.txt is the interrupt, timer, and IOMMU lines from dmesg. input.txt is i8042
and /proc/bus/input/devices. serial.txt is /proc/tty/driver/serial.
usb.txt is vendor, product, and interface class. block.txt is lsblk of whole
disks. nvme.txt is the controller model. Neither reads a filesystem.
firmware-names.txt is the firmware filenames dmesg requested. The blobs stay on
the Arch image.
iommu.txt is the group list and the AMD-Vi or DMAR lines.
EOF
}

dest_root_for() {
  local script_dir=$1 fstype source
  command -v findmnt >/dev/null 2>&1 || return 0
  fstype=$(findmnt -no FSTYPE -T "$script_dir" 2>/dev/null || true)
  source=$(findmnt -no SOURCE -T "$script_dir" 2>/dev/null || true)
  case "$fstype" in
    vfat | exfat | ext4 | ext3 | ext2 | btrfs | xfs | f2fs) ;;
    *) return 0 ;;
  esac
  [[ $source == /dev/* ]] || return 0
  printf '%s\n' "$script_dir/arch-probe-out"
}

publish() {
  local dest=$1
  mkdir -p "$dest"
  cp -r --no-preserve=ownership,mode "$work"/. "$dest"/
  sync
  echo "probe: wrote $dest"
}

main() {
  local script_dir root stamp dest
  need_root
  rm -rf "$work"
  mkdir -p "$work/pci"

  write_host
  write_cpu
  cp /proc/cmdline "$work/cmdline.txt"
  uname -a > "$work/uname.txt"
  write_secure
  dmesg --color=never > "$work/dmesg.txt" || true
  cp /proc/iomem "$work/iomem.txt" || true
  write_e820
  write_framebuffer
  if command -v lspci >/dev/null 2>&1; then
    lspci -Dnnvvv > "$work/lspci-nnvvv.txt" || true
    lspci -xxxx > "$work/lspci-xxxx.txt" || true
  else
    echo "lspci: absent" > "$work/lspci-nnvvv.txt"
  fi
  write_pci
  if [[ -d /sys/firmware/acpi/tables ]]; then
    mkdir -p "$work/acpi"
    cp -r --no-preserve=ownership,mode /sys/firmware/acpi/tables/. "$work/acpi/"
  fi
  grep -E 'IO-APIC|IOAPIC|INT_SRC_OVR|i8042|serio:|HPET|tsc:|MMCONFIG|AMD-Vi|DMAR:|efifb|vesafb' \
    "$work/dmesg.txt" > "$work/irq.txt" || true
  write_input
  if [[ -r /proc/tty/driver/serial ]]; then
    cat /proc/tty/driver/serial > "$work/serial.txt"
  else
    echo "absent" > "$work/serial.txt"
  fi
  write_usb
  lsblk -d -o NAME,SIZE,TYPE,TRAN,MODEL,VENDOR > "$work/block.txt" || true
  write_nvme
  grep -Ei 'firmware:|direct firmware load' "$work/dmesg.txt" > "$work/firmware-names.txt" || true
  {
    echo "--- groups ---"
    find /sys/kernel/iommu_groups -mindepth 3 -maxdepth 3 -type l -printf '%p -> %l\n' 2>/dev/null || true
    echo "--- dmesg ---"
    grep -E 'AMD-Vi|DMAR:' "$work/dmesg.txt" || true
  } > "$work/iommu.txt"
  lsmod > "$work/modules.txt" || true
  write_about

  script_dir=$(cd "$(dirname "$0")" && pwd)
  bash "$script_dir/summarize.sh" "$work"

  root=$(dest_root_for "$script_dir")
  stamp=$(date -u +%Y%m%dT%H%M%SZ)
  if [[ -z $root ]]; then
    echo "probe: results are only in $work"
    echo "probe: that directory is RAM. Copy it to the stick before rebooting:"
    echo "    mkdir -p /mnt/stick/arch-probe-out/$stamp"
    echo "    cp -a $work/. /mnt/stick/arch-probe-out/$stamp/"
    exit 0
  fi
  dest="$root/$stamp"
  if [[ -e $dest ]]; then
    dest="${dest}-2"
  fi
  publish "$dest"
}

main "$@"
