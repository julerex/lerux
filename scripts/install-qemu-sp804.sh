#!/usr/bin/env bash
# Build qemu-system-aarch64 with SP804 timers on the virt machine (rust-sel4 patch).
# Installs to deps/toolchains/qemu-sp804/ when stock QEMU lacks 0x90d0000 SP804.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
install_prefix="${toolchains_dir}/qemu-sp804"
qemu_bin="${install_prefix}/bin/qemu-system-aarch64"
patch="${root}/support/qemu/arm-virt-sp804.patch"
qemu_version="6.2.0"
src_dir="${toolchains_dir}/qemu-${qemu_version}-src"
tarball="${toolchains_dir}/qemu-${qemu_version}.tar.xz"

if [[ -x "${qemu_bin}" ]]; then
    echo "==> SP804 QEMU already installed at ${install_prefix}" >&2
    echo "${install_prefix}/bin"
    exit 0
fi

for tool in curl patch make; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "error: ${tool} required to build SP804 QEMU" >&2
        exit 1
    fi
done

if ! pkg-config --exists glib-2.0 2>/dev/null; then
    echo "error: libglib2.0-dev required to build SP804 QEMU (apt install libglib2.0-dev libpixman-1-dev)" >&2
    exit 1
fi

mkdir -p "${toolchains_dir}"
if [[ ! -f "${tarball}" ]]; then
    echo "==> Downloading QEMU ${qemu_version}" >&2
    curl -fsSL -o "${tarball}" "https://download.qemu.org/qemu-${qemu_version}.tar.xz"
fi

if [[ ! -d "${src_dir}" ]]; then
    echo "==> Extracting QEMU ${qemu_version}" >&2
    tar -xf "${tarball}" -C "${toolchains_dir}"
    mv "${toolchains_dir}/qemu-${qemu_version}" "${src_dir}"
fi

if ! grep -q VIRT_TIMER1 "${src_dir}/hw/arm/virt.c" 2>/dev/null; then
    echo "==> Applying arm-virt-sp804 patch" >&2
    patch -d "${src_dir}" -p1 < "${patch}"
fi

if [[ ! -f "${src_dir}/build/config.status" ]]; then
    echo "==> Configuring SP804 QEMU (aarch64-softmmu only)" >&2
    rm -rf "${src_dir}/build"
    (
        cd "${src_dir}"
        ./configure \
            --prefix="${install_prefix}" \
            --target-list=aarch64-softmmu \
            --disable-werror \
            --disable-docs \
            --disable-gtk \
            --disable-sdl \
            --disable-vnc \
            --disable-curses \
            --audio-drv-list= \
            --disable-capstone \
            --disable-libusb \
            --disable-usb-redir \
            --disable-vhost-user \
            --disable-vhost-vdpa
    ) >&2
else
    echo "==> Reusing existing SP804 QEMU build tree" >&2
fi

echo "==> Building SP804 QEMU" >&2
make -C "${src_dir}/build" -j"$(nproc)" >&2
make -C "${src_dir}/build" install >&2

if [[ ! -x "${qemu_bin}" ]]; then
    echo "error: SP804 QEMU build failed" >&2
    exit 1
fi

echo "==> SP804 QEMU installed at ${install_prefix}" >&2
echo "${install_prefix}/bin"