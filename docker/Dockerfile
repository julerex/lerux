# Development/CI image: build Microkit SDK from source + run lerux smoke test.
FROM debian:bookworm

RUN apt-get update -q && apt-get install -y --no-install-recommends \
    bash build-essential curl git ca-certificates \
    cmake ninja-build device-tree-compiler libxml2-utils \
    python3 python3-venv python3-pip \
    qemu-system-arm qemu-system-x86 qemu-system-misc qemu-utils ipxe-qemu \
    ovmf xorriso clang grub-efi-amd64-bin \
    libglib2.0-dev libpixman-1-dev pkg-config \
    libclang-dev \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# ARM GNU bare-metal toolchain (Microkit SDK build requirement).
# Same archive and digest as `install_arm_toolchain`. developer.arm.com often
# answers HTTP 503 before it redirects at the file. The second URL is that
# redirect target.
RUN set -eux; \
    archive=arm-gnu-toolchain-12.2.rel1-x86_64-aarch64-none-elf.tar.xz; \
    sum=62d66e0ad7bd7f2a183d236ee301a5c73c737c886c7944aa4f39415aab528daf; \
    primary="https://developer.arm.com/-/media/Files/downloads/gnu/12.2.rel1/binrel/${archive}"; \
    mirror="https://armkeil.blob.core.windows.net/developer/Files/downloads/gnu/12.2.rel1/binrel/${archive}"; \
    ok=0; \
    for url in "$primary" "$mirror"; do \
      if curl -fsSL \
          --connect-timeout 20 \
          --speed-limit 1024 \
          --speed-time 30 \
          --retry 5 \
          --retry-delay 2 \
          --retry-all-errors \
          --retry-max-time 60 \
          -o /tmp/arm-gnu.tar.xz \
          "$url" \
        && echo "${sum}  /tmp/arm-gnu.tar.xz" | sha256sum -c -; then \
        ok=1; \
        break; \
      fi; \
      rm -f /tmp/arm-gnu.tar.xz; \
    done; \
    test "$ok" = 1; \
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
# Cargo reads every workspace member before it builds lerux-cli.
COPY tools/lerux-image /work/tools/lerux-image
COPY support/boards.toml /work/support/boards.toml
COPY userspace/pds /work/userspace/pds
COPY userspace/crates /work/userspace/crates
COPY lerux-ladybird/crates /work/lerux-ladybird/crates
COPY lerux-ladybird/pds /work/lerux-ladybird/pds
# lerux-cli builds lerux-ladybird-image. That crate embeds these two files.
COPY support/browser/lenna.html /work/support/browser/lenna.html
COPY support/browser/lenna.png /work/support/browser/lenna.png
COPY Cargo.toml /work/Cargo.toml
COPY Cargo.lock /work/Cargo.lock
WORKDIR /work
# Install under /opt so a bind-mounted /work checkout does not hide the toolchain in CI.
# The RISC-V archive is a GitHub release. Unauthenticated downloads return
# HTTP 504; the optional github_token secret is the job token.
RUN --mount=type=secret,id=github_token,required=false \
    if [ -s /run/secrets/github_token ]; then \
      export GITHUB_TOKEN="$$(tr -d '[:space:]' < /run/secrets/github_token)"; \
    fi; \
    cargo build -q -p lerux-cli && \
    cargo run -q -p lerux-cli -- install riscv-toolchain && \
    mv deps/toolchains /opt/lerux-toolchains
ENV PATH="/opt/lerux-toolchains/riscv64-unknown-elf/bin:${PATH}"
RUN rustup show