# Development/CI image: build Microkit SDK from source + run lerux smoke test.
FROM debian:bookworm

RUN apt-get update -q && apt-get install -y --no-install-recommends \
    build-essential curl git ca-certificates \
    cmake ninja-build device-tree-compiler libxml2-utils \
    python3 python3-venv python3-pip \
    qemu-system-arm qemu-system-x86 qemu-system-misc qemu-utils ipxe-qemu \
    libclang-dev \
    python3-pexpect \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# ARM GNU bare-metal toolchain (Microkit SDK build requirement)
RUN set -eux; \
    curl -fsSL -o /tmp/arm-gnu.tar.xz \
      "https://developer.arm.com/-/media/Files/downloads/gnu/12.2.rel1/binrel/arm-gnu-toolchain-12.2.rel1-x86_64-aarch64-none-elf.tar.xz"; \
    tar -xf /tmp/arm-gnu.tar.xz -C /opt; \
    rm /tmp/arm-gnu.tar.xz
ENV PATH="/opt/arm-gnu-toolchain-12.2.rel1-x86_64-aarch64-none-elf/bin:${PATH}"

# Rust (toolchain pinned by rust-toolchain.toml in repo)
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# just
RUN cargo install just --locked

# Match rust-toolchain.toml before building lerux PDs
COPY rust-toolchain.toml /work/rust-toolchain.toml
COPY scripts/install-riscv-toolchain.sh /work/scripts/install-riscv-toolchain.sh
WORKDIR /work
RUN chmod +x scripts/install-riscv-toolchain.sh && scripts/install-riscv-toolchain.sh
ENV PATH="/work/deps/toolchains/riscv64-unknown-elf/bin:${PATH}"
RUN rustup show