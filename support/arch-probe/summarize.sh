#!/bin/bash
# Turn one probe directory into board.txt.
#
# The lines are the facts a desk image hardcodes: firmware framebuffer,
# COM port, and the PCI windows. Raw files in the same directory keep every
# value this summary does not lift. Pass a directory, or --check to diff the
# fixture.

set -euo pipefail

die() {
  echo "summarize: $*" >&2
  exit 1
}

hex_trim() {
  printf '0x%x\n' "$(($1))"
}

field() {
  local key=$1 file=$2
  [[ -f $file ]] || return 0
  awk -v key="$key" 'index($0, key ": ") == 1 { print substr($0, length(key) + 3); exit }' "$file"
}

join_known() {
  local out="" part
  for part in "$@"; do
    [[ -n $part ]] || continue
    if [[ -z $out ]]; then
      out=$part
    else
      out="$out $part"
    fi
  done
  if [[ -z $out ]]; then
    printf 'unknown\n'
  else
    printf '%s\n' "$out"
  fi
}

# A stick stores 0000-02-00.0 because FAT rejects colons. A directory that
# already uses colons is accepted too.
bdf_name() {
  local name=$1
  if [[ $name =~ ^[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]$ ]]; then
    printf '%s\n' "$name"
    return 0
  fi
  if [[ $name =~ ^[0-9a-f]{4}-[0-9a-f]{2}-[0-9a-f]{2}\.[0-7]$ ]]; then
    printf '%s\n' "${name//-/:}"
    return 0
  fi
  return 1
}

id4() {
  local v
  v=$(tr -d '[:space:]' < "$1")
  v=${v#0x}
  v=${v#0X}
  printf '%04x\n' "$((16#$v))"
}

emit_cpu() {
  local file=$1
  local vendor family model name out
  vendor=$(field vendor_id "$file")
  family=$(field "cpu family" "$file")
  model=$(field model "$file")
  name=$(field "model name" "$file")
  out=""
  [[ -n $vendor ]] && out=$vendor
  if [[ -n $family ]]; then
    out="${out:+$out }family $family"
  fi
  if [[ -n $model ]]; then
    out="${out:+$out }model $model"
  fi
  if [[ -n $name ]]; then
    out="${out:+$out }$name"
  fi
  if [[ -z $out ]]; then
    echo "cpu: unknown"
  else
    echo "cpu: $out"
  fi
}

emit_secure() {
  local value
  value=$(field SecureBoot "$1")
  case $value in
    0 | 1) echo "secure_boot: $value" ;;
    *) echo "secure_boot: unknown" ;;
  esac
}

emit_framebuffer() {
  local dir=$1
  local dmesg=$dir/dmesg.txt
  local line phys="" size_k="" w="" h="" bpp="" pitch="" rs="" gs="" bs="" source=""
  local re_at re_size re_mode re_color
  re_at='(efi|vesa)fb: framebuffer at (0x[0-9a-fA-F]+)'
  re_size='(efi|vesa)fb: framebuffer at .*, using [0-9]+k, total ([0-9]+)k'
  re_mode='(efi|vesa)fb: mode is ([0-9]+)x([0-9]+)x([0-9]+), linelength=([0-9]+)'
  re_color='(efi|vesa)fb: Truecolor: size=[0-9]+:[0-9]+:[0-9]+:[0-9]+, shift=([0-9]+):([0-9]+):([0-9]+):[0-9]+'
  if [[ -f $dmesg ]]; then
    while IFS= read -r line; do
      if [[ -z $phys && $line =~ $re_at ]]; then
        source=${BASH_REMATCH[1]}fb
        phys=$(hex_trim "${BASH_REMATCH[2]}")
      fi
      if [[ -z $size_k && $line =~ $re_size ]]; then
        size_k=${BASH_REMATCH[2]}
      fi
      if [[ -z $w && $line =~ $re_mode ]]; then
        w=${BASH_REMATCH[2]}
        h=${BASH_REMATCH[3]}
        bpp=${BASH_REMATCH[4]}
        pitch=${BASH_REMATCH[5]}
      fi
      if [[ -z $rs && $line =~ $re_color ]]; then
        rs=${BASH_REMATCH[2]}
        gs=${BASH_REMATCH[3]}
        bs=${BASH_REMATCH[4]}
      fi
    done < "$dmesg"
  fi
  if [[ -n $phys ]]; then
    local out="framebuffer: phys $phys" size_hex=""
    if [[ -n $size_k ]]; then
      size_hex=$(hex_trim $((size_k * 1024)))
      out="$out size $size_hex"
    fi
    [[ -n $pitch ]] && out="$out pitch $pitch"
    [[ -n $w && -n $h ]] && out="$out ${w}x${h}"
    [[ -n $bpp ]] && out="$out bpp $bpp"
    [[ -n $rs && -n $gs && -n $bs ]] && out="$out r${rs} g${gs} b${bs}"
    printf '%s\n' "$out"
    printf 'framebuffer_source: %s\n' "$source"
    return
  fi
  echo "framebuffer: unknown"
  if [[ -f $dir/framebuffer.txt ]] && grep -q '^modeset: yes$' "$dir/framebuffer.txt"; then
    echo "framebuffer_source: modeset"
  else
    echo "framebuffer_source: unknown"
  fi
}

kind_of() {
  local class=$(($1))
  local base=$((class & 0xffff00))
  if ((base == 0x010800)); then
    printf 'nvme\n'
  elif ((class == 0x010601)); then
    printf 'ahci\n'
  elif ((class == 0x0c0330)); then
    printf 'xhci\n'
  elif ((base == 0x028000)); then
    printf 'wifi\n'
  elif ((base == 0x020000)); then
    printf 'ethernet\n'
  else
    return 1
  fi
}

sort_key() {
  case $1 in
    nvme) printf '1\n' ;;
    ahci) printf '2\n' ;;
    xhci) printf '3\n' ;;
    wifi) printf '4\n' ;;
    ethernet) printf '5\n' ;;
    *) return 1 ;;
  esac
}

bar_of() {
  local file=$1
  [[ -f $file ]] || return 1
  local idx=0 start end flags size
  while read -r start end flags; do
    [[ -n ${start:-} && -n ${end:-} ]] || continue
    if ((start == 0 && end == 0)); then
      idx=$((idx + 1))
      continue
    fi
    if ((end < start)); then
      idx=$((idx + 1))
      continue
    fi
    size=$((end - start + 1))
    printf 'bar%s %s size %s\n' "$idx" "$(hex_trim "$start")" "$(hex_trim "$size")"
    return 0
  done < "$file"
  return 1
}

emit_boot_vga() {
  local pci=$1 dir value bdf
  shopt -s nullglob
  if [[ -d $pci ]]; then
    for dir in "$pci"/*; do
      [[ -f $dir/boot_vga && -f $dir/vendor && -f $dir/device ]] || continue
      bdf=$(bdf_name "$(basename "$dir")") || continue
      value=$(tr -d '[:space:]' < "$dir/boot_vga")
      if [[ $value == 1 ]]; then
        printf 'boot_vga: %s %s:%s\n' "$bdf" "$(id4 "$dir/vendor")" "$(id4 "$dir/device")"
        shopt -u nullglob
        return
      fi
    done
  fi
  shopt -u nullglob
  echo "boot_vga: none"
}

emit_devices() {
  local pci=$1
  [[ -d $pci ]] || return 0
  local list dir bdf class kind vendor device line key parent bar
  list=$(mktemp)
  shopt -s nullglob
  for dir in "$pci"/*; do
    [[ -d $dir && -f $dir/class && -f $dir/vendor && -f $dir/device ]] || continue
    bdf=$(bdf_name "$(basename "$dir")") || continue
    class=$(tr -d '[:space:]' < "$dir/class")
    kind=$(kind_of "$class") || continue
    vendor=$(id4 "$dir/vendor")
    device=$(id4 "$dir/device")
    line="$kind: $bdf $vendor:$device"
    if bar=$(bar_of "$dir/resource"); then
      line="$line $bar"
    fi
    if [[ -f $dir/parent ]]; then
      parent=$(tr -d '[:space:]' < "$dir/parent")
      if [[ $parent =~ ^[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]$ ]]; then
        line="$line parent $parent"
      fi
    fi
    key=$(sort_key "$kind")
    printf '%s\t%s\t%s\n' "$key" "$bdf" "$line" >> "$list"
  done
  shopt -u nullglob
  if [[ -s $list ]]; then
    sort -t $'\t' -k1,1 -k2,2 "$list" | cut -f3
  fi
  rm -f "$list"
}

emit_i8042() {
  local dir=$1
  if [[ -f $dir/input.txt ]] && grep -q '^i8042: yes$' "$dir/input.txt"; then
    echo "i8042: yes"
  elif [[ -f $dir/dmesg.txt ]] && grep -q 'i8042' "$dir/dmesg.txt"; then
    echo "i8042: yes"
  else
    echo "i8042: no"
  fi
}

emit_irq1() {
  local file=$1 line
  local re='INT_SRC_OVR \(bus ([0-9]+) bus_irq ([0-9]+) global_irq ([0-9]+)'
  if [[ -f $file ]]; then
    while IFS= read -r line; do
      [[ $line =~ $re ]] || continue
      if [[ ${BASH_REMATCH[2]} == 1 ]]; then
        printf 'irq1: bus_irq 1 global_irq %s\n' "${BASH_REMATCH[3]}"
        return
      fi
    done < "$file"
  fi
  echo "irq1: no override"
}

emit_serial() {
  local file=$1 found=0 line port irq
  local re_port='port:([0-9a-fA-F]+)'
  local re_irq='irq:([0-9]+)'
  if [[ ! -f $file ]]; then
    echo "serial: none"
    return
  fi
  while IFS= read -r line; do
    [[ $line =~ ^[0-9]+: ]] || continue
    [[ $line == *uart:unknown* ]] && continue
    [[ $line =~ $re_port ]] || continue
    port=${BASH_REMATCH[1]}
    [[ $line =~ $re_irq ]] || continue
    irq=${BASH_REMATCH[1]}
    printf 'serial: port 0x%x irq %s\n' "$((16#$port))" "$irq"
    found=1
  done < "$file"
  if [[ $found -eq 0 ]]; then
    echo "serial: none"
  fi
}

emit_iommu() {
  local file=$1 line
  local amd="" intel="" amd_base="" intel_base=""
  local re_amd='AMD-Vi:.*at[[:space:]]+(0x[0-9a-fA-F]+)'
  local re_intel='DMAR:.*base:?[[:space:]]*(0x[0-9a-fA-F]+)'
  if [[ -f $file ]]; then
    while IFS= read -r line; do
      if [[ $line =~ $re_amd ]]; then
        amd=1
        amd_base=$(hex_trim "${BASH_REMATCH[1]}")
      elif [[ $line == *AMD-Vi* ]]; then
        amd=1
      fi
      if [[ $line =~ $re_intel ]]; then
        intel=1
        intel_base=$(hex_trim "${BASH_REMATCH[1]}")
      elif [[ $line == *DMAR:* ]]; then
        intel=1
      fi
    done < "$file"
  fi
  if [[ -n $amd_base ]]; then
    echo "iommu: amd $amd_base"
  elif [[ -n $intel_base ]]; then
    echo "iommu: intel $intel_base"
  elif [[ -n $amd ]]; then
    echo "iommu: amd"
  elif [[ -n $intel ]]; then
    echo "iommu: intel"
  else
    echo "iommu: none"
  fi
}

emit_mcfg() {
  local file=$1 line
  local re_base='\[bus[[:space:]]+([0-9a-fA-F]+)-([0-9a-fA-F]+)\].*\(base[[:space:]]+(0x[0-9a-fA-F]+)\)'
  local re_mem='MMCONFIG at \[mem (0x[0-9a-fA-F]+)-'
  if [[ -f $file ]]; then
    while IFS= read -r line; do
      if [[ $line =~ $re_base ]]; then
        printf 'mcfg: %s buses %d-%d\n' "$(hex_trim "${BASH_REMATCH[3]}")" "$((16#${BASH_REMATCH[1]}))" "$((16#${BASH_REMATCH[2]}))"
        return
      fi
      if [[ $line =~ $re_mem ]]; then
        printf 'mcfg: %s buses unknown\n' "$(hex_trim "${BASH_REMATCH[1]}")"
        return
      fi
    done < "$file"
  fi
  echo "mcfg: unknown"
}

emit_e820() {
  local dir=$1
  local type start end line
  local re='BIOS-e820: \[mem (0x[0-9a-fA-F]+)-(0x[0-9a-fA-F]+)\] (.*)'
  if [[ -s $dir/e820.txt ]]; then
    while IFS=$'\t' read -r type start end; do
      [[ -n $type && -n ${start:-} && -n ${end:-} ]] || continue
      printf 'e820: %s %s-%s\n' "$type" "$(hex_trim "$start")" "$(hex_trim "$end")"
    done < "$dir/e820.txt"
    return
  fi
  [[ -f $dir/dmesg.txt ]] || return 0
  while IFS= read -r line; do
    [[ $line =~ $re ]] || continue
    printf 'e820: %s %s-%s\n' "${BASH_REMATCH[3]}" "$(hex_trim "${BASH_REMATCH[1]}")" "$(hex_trim "${BASH_REMATCH[2]}")"
  done < "$dir/dmesg.txt"
}

summarize() {
  local dir=$1
  [[ -d $dir ]] || die "not a directory: $dir"
  {
    printf 'identity: %s\n' "$(join_known "$(field vendor "$dir/host.txt")" "$(field product "$dir/host.txt")")"
    printf 'board: %s\n' "$(join_known "$(field board_vendor "$dir/host.txt")" "$(field board_name "$dir/host.txt")")"
    printf 'bios: %s\n' "$(join_known "$(field bios_version "$dir/host.txt")" "$(field bios_date "$dir/host.txt")")"
    emit_cpu "$dir/cpu.txt"
    emit_secure "$dir/secure-boot.txt"
    emit_framebuffer "$dir"
    emit_boot_vga "$dir/pci"
    emit_i8042 "$dir"
    emit_irq1 "$dir/dmesg.txt"
    emit_serial "$dir/serial.txt"
    emit_iommu "$dir/dmesg.txt"
    emit_mcfg "$dir/dmesg.txt"
    emit_devices "$dir/pci"
    emit_e820 "$dir"
  } > "$dir/board.txt"
}

check_fixture() {
  local here
  here=$(cd "$(dirname "$0")" && pwd)
  # The EXIT trap runs after this function returns, so tmp cannot be local.
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  mkdir -p "$tmp/src"
  cp -a "$here/fixture/." "$tmp/src/"
  rm -f "$tmp/src/expected-board.txt"
  summarize "$tmp/src"
  diff -u "$here/fixture/expected-board.txt" "$tmp/src/board.txt"
}

if [[ ${1:-} == --check ]]; then
  check_fixture
  exit 0
fi

[[ $# -eq 1 ]] || die "usage: summarize.sh <probe-dir> | summarize.sh --check"
summarize "$1"
