# Development/CI image: build Microkit SDK from source + run lerux smoke test.
FROM debian:bookworm

RUN apt-get update -q && apt-get install -y --no-install-recommends \
    bash build-essential curl git ca-certificates \
    cmake ninja-build device-tree-compiler libxml2-utils \
    python3 python3-venv python3-pip \
    qemu-system-arm qemu-system-x86 qemu-system-misc qemu-utils ipxe-qemu \
    libglib2.0-dev libpixman-1-dev pkg-config \
    libclang-dev \
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
COPY tools/lerux-cli /work/tools/lerux-cli
COPY support/boards.toml /work/support/boards.toml
COPY userspace/pds /work/userspace/pds
COPY userspace/crates /work/userspace/crates
COPY Cargo.toml /work/Cargo.toml
COPY Cargo.lock /work/Cargo.lock
WORKDIR /work
# Install under /opt so a bind-mounted /work checkout does not hide the toolchain in CI.
RUN cargo build -q -p lerux-cli && \
    cargo run -q -p lerux-cli -- install riscv-toolchain && \
    mv deps/toolchains /opt/lerux-toolchains
ENV PATH="/opt/lerux-toolchains/riscv64-unknown-elf/bin:${PATH}"
RUN rustup show