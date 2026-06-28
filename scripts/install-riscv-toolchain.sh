#!/usr/bin/env bash
# Install riscv64-unknown-elf GCC 13.2+ into deps/toolchains/ (no sudo).
# Uses xPack (self-contained) plus riscv64-unknown-elf wrappers — Microkit expects
# that prefix; bare riscv-none-elf defaults to 32-bit linking.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
xpack_url="https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v13.2.0-2/xpack-riscv-none-elf-gcc-13.2.0-2-linux-x64.tar.gz"
xpack_dir="${toolchains_dir}/xpack-riscv-none-elf-gcc-13.2.0-2"
wrapper_dir="${toolchains_dir}/riscv64-unknown-elf/bin"
wrapper_gcc="${wrapper_dir}/riscv64-unknown-elf-gcc"

if command -v riscv64-unknown-elf-gcc >/dev/null 2>&1; then
    version="$(riscv64-unknown-elf-gcc -dumpversion | cut -d. -f1)"
    if [[ "${version}" -ge 13 ]]; then
        echo "==> riscv64-unknown-elf-gcc already on PATH: $(command -v riscv64-unknown-elf-gcc)" >&2
        dirname "$(command -v riscv64-unknown-elf-gcc)"
        exit 0
    fi
fi

if [[ -x "${wrapper_gcc}" ]]; then
    echo "==> RISC-V toolchain already installed under ${toolchains_dir}" >&2
    echo "${wrapper_dir}"
    exit 0
fi

echo "==> Downloading xPack RISC-V GNU toolchain 13.2.0" >&2
mkdir -p "${toolchains_dir}"
tmp="$(mktemp)"
curl -fsSL -o "${tmp}" "${xpack_url}"
tar -xf "${tmp}" -C "${toolchains_dir}"
rm -f "${tmp}"

if [[ ! -x "${xpack_dir}/bin/riscv-none-elf-gcc" ]]; then
    echo "error: xPack RISC-V toolchain install failed" >&2
    exit 1
fi

mkdir -p "${wrapper_dir}"
for tool in gcc g++ cpp as ld ar nm objcopy objdump ranlib strip; do
    case "${tool}" in
        gcc|g++|cpp)
            extra_flags="-march=rv64imafdc -mabi=lp64d"
            ;;
        as)
            extra_flags="-march=rv64imafdc -mabi=lp64d"
            ;;
        ld)
            extra_flags="-m elf64lriscv"
            ;;
        *)
            extra_flags=""
            ;;
    esac
    cat > "${wrapper_dir}/riscv64-unknown-elf-${tool}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
exec "${xpack_dir}/bin/riscv-none-elf-${tool}" ${extra_flags} "\$@"
EOF
    chmod +x "${wrapper_dir}/riscv64-unknown-elf-${tool}"
done

echo "${wrapper_dir}"