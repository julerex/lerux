#!/bin/bash
# Record one mt7921e probe on the Acer Swift, from the Arch install medium.
#
# The medium has no network until this driver is up, so this uses the kernel
# tracer instead of trace-cmd. mt76:reg_rr and mt76:reg_wr are the events
# trace-cmd would record. They fire in mt76_mmio_rr/wr, after the driver's
# remap, so the addresses are BAR offsets. map_reg in lerux-swift translates
# those back to the logical addresses device.rs uses.
#
# Firmware load runs from a workqueue after modprobe returns, and a semaphore
# timeout is retried past "chip reset failed", so the trace stays on until
# "Firmware init done" or two minutes.

set -euo pipefail

work=/run/swift-linux
tracefs=/sys/kernel/tracing
trace_pid=
wait_secs=120

die() {
  echo "capture: $*" >&2
  exit 1
}

need_root() {
  [[ "$(id -u)" -eq 0 ]] || die "run as root"
}

# `blacklist` stops the automatic alias load. An explicit modprobe still works.
# `module_blacklist=` on the kernel line does not: the kernel refuses the load.
hold_autoload() {
  local line
  line=$(cat /proc/cmdline)
  if [[ "$line" =~ module_blacklist=([^[:space:]]*) ]]; then
    case ",${BASH_REMATCH[1]}," in
      *,mt7921e,*)
        die "kernel line has module_blacklist=mt7921e, which blocks this script too. Reboot with modprobe.blacklist=mt7921e"
        ;;
    esac
  fi
  mkdir -p /run/modprobe.d
  echo 'blacklist mt7921e' > /run/modprobe.d/swift-linux.conf
}

wifi_bdf() {
  lspci -Dnn | awk '/\[14c3:7961\]/ { print $1; exit }'
}

port_of() {
  local bdf=$1 path parent
  path=$(readlink -f "/sys/bus/pci/devices/$bdf")
  parent=$(basename "$(dirname "$path")")
  if [[ "$parent" =~ ^[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]$ ]]; then
    printf '%s\n' "$parent"
  fi
}

nvme_bdf() {
  lspci -Dnn | awk '/\[0108\]/ { print $1; exit }'
}

firmware_ok() {
  local dir name found_patch=0 found_ram=0
  for dir in /usr/lib/firmware /lib/firmware; do
    [[ -d "$dir/mediatek" ]] || continue
    for name in WIFI_MT7961_patch_mcu_1_2_hdr.bin WIFI_MT7961_patch_mcu_1a_2_hdr.bin; do
      if [[ -f "$dir/mediatek/$name" || -f "$dir/mediatek/$name.zst" || -f "$dir/mediatek/$name.xz" ]]; then
        found_patch=1
      fi
    done
    for name in WIFI_RAM_CODE_MT7961_1.bin WIFI_RAM_CODE_MT7961_1a.bin; do
      if [[ -f "$dir/mediatek/$name" || -f "$dir/mediatek/$name.zst" || -f "$dir/mediatek/$name.xz" ]]; then
        found_ram=1
      fi
    done
  done
  [[ "$found_patch" -eq 1 && "$found_ram" -eq 1 ]]
}

dump_dev() {
  local tag=$1 bdf=$2 when=$3
  [[ -n "$bdf" ]] || return 0
  lspci -nnvvv -s "$bdf" > "$work/lspci-${tag}-${when}.txt"
  lspci -xxxx -s "$bdf" > "$work/lspci-${tag}-xxxx-${when}.txt"
}

dump_tree() {
  local when=$1
  dump_dev wifi "$wifi" "$when"
  dump_dev wifi-port "$wifi_port" "$when"
  dump_dev nvme "$nvme" "$when"
  dump_dev nvme-port "$nvme_port" "$when"
  lspci -Dnnvvv > "$work/lspci-all-${when}.txt"
}

write_devices() {
  {
    printf 'wifi %s\n' "$wifi"
    printf 'wifi_port %s\n' "${wifi_port:-}"
    printf 'nvme %s\n' "${nvme:-}"
    printf 'nvme_port %s\n' "${nvme_port:-}"
  } > "$work/devices.txt"
}

write_host() {
  {
    echo "uname: $(uname -a)"
    echo "cmdline: $(cat /proc/cmdline)"
    echo "vendor: $(cat /sys/class/dmi/id/sys_vendor 2>/dev/null || true)"
    echo "product: $(cat /sys/class/dmi/id/product_name 2>/dev/null || true)"
    echo "bios: $(cat /sys/class/dmi/id/bios_version 2>/dev/null || true)"
  } > "$work/host.txt"
  lsmod > "$work/lsmod.txt"
}

write_dmesg() {
  local tag=$1
  dmesg --color=never > "$work/dmesg-${tag}.txt"
  grep -E 'mt7921|AMD-Vi' "$work/dmesg-${tag}.txt" > "$work/dmesg-${tag}-mt7921.txt" || true
  grep -iE 'mt7921|mt76|AMD-Vi|amd_iommu|iommu' "$work/dmesg-${tag}.txt" > "$work/dmesg-${tag}-bus.txt" || true
}

write_iommu() {
  {
    echo "== cmdline =="
    cat /proc/cmdline
    echo
    echo "== iommu groups =="
    find /sys/kernel/iommu_groups -mindepth 3 -maxdepth 3 -type l -printf '%p -> %l\n' 2>/dev/null || true
    echo
    echo "== debugfs =="
    find /sys/kernel/debug -iname '*iommu*' -o -iname '*amd*vi*' 2>/dev/null || true
  } > "$work/iommu.txt"
}

list_firmware() {
  local dir
  {
    for dir in /usr/lib/firmware /lib/firmware; do
      if [[ -d "$dir/mediatek" ]]; then
        echo "== $dir/mediatek =="
        ls -l "$dir/mediatek"/WIFI_MT7961* "$dir/mediatek"/WIFI_RAM_CODE_MT7961* 2>/dev/null || true
      fi
    done
  } > "$work/firmware.txt"
}

stop_trace() {
  if [[ -z "$trace_pid" ]]; then
    return 0
  fi
  echo 0 > "$tracefs/tracing_on" 2>/dev/null || true
  sleep 0.5
  kill "$trace_pid" 2>/dev/null || true
  wait "$trace_pid" 2>/dev/null || true
  trace_pid=
  echo 0 > "$tracefs/events/mt76/reg_rr/enable" 2>/dev/null || true
  echo 0 > "$tracefs/events/mt76/reg_wr/enable" 2>/dev/null || true
}

trap 'stop_trace || true' EXIT

start_trace() {
  echo nop > "$tracefs/current_tracer"
  echo 16384 > "$tracefs/buffer_size_kb" || true
  echo > "$tracefs/trace"
  echo 1 > "$tracefs/events/mt76/reg_rr/enable"
  echo 1 > "$tracefs/events/mt76/reg_wr/enable"
  echo 1 > "$tracefs/tracing_on"
  cat "$tracefs/trace_pipe" > "$work/trace-report.txt" &
  trace_pid=$!
  sleep 0.3
}

probe_settled() {
  grep -q 'Firmware init done' "$1"
}

wait_for_probe() {
  local i size
  echo "capture: waiting up to ${wait_secs}s for firmware init"
  for ((i = 1; i <= wait_secs * 2; i++)); do
    dmesg --color=never > "$work/dmesg-probe.txt"
    if probe_settled "$work/dmesg-probe.txt"; then
      sleep 1
      dmesg --color=never > "$work/dmesg-probe.txt"
      return 0
    fi
    if [[ -f "$work/trace-report.txt" ]]; then
      size=$(stat -c %s "$work/trace-report.txt")
      if (( size > 268435456 )); then
        echo "capture: trace reached 256MB, stopping the wait" | tee "$work/trace-note.txt"
        return 0
      fi
    fi
    if (( i % 10 == 0 )); then
      grep -E 'mt7921e|Firmware init|patch semaphore' "$work/dmesg-probe.txt" | tail -n 1 || true
    fi
    sleep 0.5
  done
  echo "capture: no firmware result after ${wait_secs}s" | tee "$work/trace-note.txt"
}

compact_trace() {
  awk '
    function hex8(s, n, i, d, v) {
      v = 0
      for (i = 1; i <= length(s); i++) {
        d = index("0123456789abcdef", tolower(substr(s, i, 1)))
        if (d == 0) return "????????"
        v = v * 16 + (d - 1)
      }
      return sprintf("%08x", v)
    }
    {
      kind = ""
      if ($0 ~ /reg_rr:/) kind = "rr"
      else if ($0 ~ /reg_wr:/) kind = "wr"
      if (kind == "") next
      if (match($0, /[0-9a-fA-F]+=[0-9a-fA-F]+/)) {
        pair = substr($0, RSTART, RLENGTH)
        split(pair, a, "=")
        printf "%s %s %s\n", kind, hex8(a[1]), hex8(a[2])
      }
    }
  ' "$work/trace-report.txt" > "$work/regs.txt"
  {
    echo "# mt76:reg_rr / mt76:reg_wr, one event per line: rr|wr <bar offset> <value>"
    echo "# Addresses are BAR offsets. mt76 traces the access after remap."
    echo "# map_reg in userspace/crates/lerux-swift/src/mt7921.rs is the logical address."
    if [[ -f "$work/trace-report.txt" ]]; then
      echo "# lost-event lines: $(grep -c 'LOST' "$work/trace-report.txt" || true)"
    fi
  } > "$work/regs-note.txt"
}

write_verdict() {
  local log=$1 verdict
  if grep -q 'Firmware init done' "$log"; then
    verdict='probe ok'
  elif grep -q 'Failed to get patch semaphore' "$log"; then
    verdict='probe failed: patch semaphore'
  elif grep -q 'chip reset failed' "$log"; then
    verdict='probe failed: chip reset'
  else
    verdict='probe unfinished'
  fi
  printf '%s\n' "$verdict" | tee "$work/verdict.txt"
}

write_about() {
  cat > "$work/about.txt" <<'EOF'
Linux mt7921e probe captured on the Acer Swift.

trace-report.txt is the ftrace report of mt76:reg_rr and mt76:reg_wr, the
events `trace-cmd record -e mt76:reg_rr -e mt76:reg_wr` would write. regs.txt
is those events as `rr|wr <bar offset> <value>`. The offset is the address
mt76_mmio used after __mt7921_reg_addr, not the logical address in device.rs.

dmesg-after-mt7921.txt is `dmesg | grep -E 'mt7921|AMD-Vi'`.
lspci-wifi-*.txt is `lspci -vvv` for the MT7921. lspci-wifi-port-*.txt is its
root port. The nvme files are the disk endpoint and its root port, for the
configuration-space comparison the panel photograph could not resolve.
EOF
}

publish() {
  local dest=$1
  write_about
  if [[ -z "$dest" ]]; then
    echo "capture: results are only in $work"
    echo "capture: that directory is RAM. Copy it to the stick before rebooting:"
    echo "    cp -a $work /mnt/stick/swift-linux-out"
    return 0
  fi
  mkdir -p "$dest"
  # vfat rejects ownership and mode preservation.
  cp -r --no-preserve=ownership,mode "$work"/. "$dest"/
  sync
  echo "capture: wrote $dest"
}

dest_for_script() {
  local script_dir=$1 fstype source
  fstype=$(findmnt -no FSTYPE -T "$script_dir" 2>/dev/null || true)
  source=$(findmnt -no SOURCE -T "$script_dir" 2>/dev/null || true)
  case "$fstype" in
    vfat|exfat|ext4|ext3|ext2|btrfs|xfs|f2fs) ;;
    *) return 0 ;;
  esac
  [[ "$source" == /dev/* ]] || return 0
  printf '%s\n' "$script_dir/swift-linux-out"
}

main() {
  local script_dir dest wifi wifi_port nvme nvme_port probe_rc
  need_root
  hold_autoload
  mkdir -p "$work"

  wifi=$(wifi_bdf || true)
  [[ -n "$wifi" ]] || die "no MT7921 [14c3:7961] on this machine"
  wifi_port=$(port_of "$wifi" || true)
  nvme=$(nvme_bdf || true)
  nvme_port=
  if [[ -n "$nvme" ]]; then
    nvme_port=$(port_of "$nvme" || true)
  fi
  write_devices
  write_host
  list_firmware

  script_dir=$(cd "$(dirname "$0")" && pwd)
  dest=${SWIFT_LINUX_OUT:-$(dest_for_script "$script_dir")}

  if [[ -d /sys/module/mt7921e ]]; then
    echo "capture: mt7921e is already loaded, so this boot is not a traced first probe."
    echo "capture: saving lspci and dmesg, then stopping."
    echo "capture: shut down, power the machine off, and boot the stick with"
    echo "capture:     modprobe.blacklist=mt7921e"
    echo "capture: appended to the kernel line. Unloading here can skip the download,"
    echo "capture: because the ROM then reports the patch as already present."
    dump_tree boot
    write_dmesg boot
    write_iommu
    printf '%s\n' 'already loaded, no trace' | tee "$work/verdict.txt"
    publish "$dest"
    exit 2
  fi

  firmware_ok || die "MT7961 firmware is not on this image. Use the official Arch releng ISO."

  if [[ ! -d "$tracefs/events" ]]; then
    mount -t tracefs tracefs "$tracefs"
  fi

  dump_tree before
  write_dmesg boot
  write_iommu

  modprobe mt76
  [[ -e "$tracefs/events/mt76/reg_rr/enable" ]] || die "mt76 has no reg_rr tracepoint"

  start_trace
  set +e
  modprobe mt7921e
  probe_rc=$?
  set -e
  echo "modprobe_exit=$probe_rc" > "$work/modprobe-status.txt"
  wait_for_probe
  stop_trace

  dump_tree after
  write_dmesg after
  compact_trace
  echo
  write_verdict "$work/dmesg-after.txt"
  publish "$dest"
}

# Host check for the trace compactor. The laptop run has no arguments.
if [[ "${1:-}" == "--check-regs" ]]; then
  work=${2:?}
  compact_trace
  exit 0
fi

main "$@"
