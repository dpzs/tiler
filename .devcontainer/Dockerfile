# ── Rust DevTerm Container ──────────────────────────────────────────
# Multi-stage: toolchain image for fast, small builds
# Base: Debian bookworm for broad lib compat + Wayland support

FROM rust:1.94-bookworm AS rust-base

# ── System packages ─────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Build essentials
    build-essential pkg-config cmake ninja-build \
    # Common native deps for Rust crates
    libssl-dev libsqlite3-dev libz-dev \
    # Wayland / GUI libs
    libwayland-client0 libwayland-cursor0 libwayland-egl1 \
    libwayland-dev wayland-protocols libxkbcommon-dev \
    libvulkan-dev libvulkan1 mesa-vulkan-drivers mesa-utils \
    libegl1-mesa-dev libgles2-mesa-dev libgl1-mesa-dri \
    # GPU / DRI
    libdrm-dev libgbm-dev \
    # Font rendering (for egui / iced / slint / etc.)
    libfontconfig1-dev libfreetype-dev \
    # Cross-compilation (musl static builds)
    musl-tools \
    # Misc dev tools
    curl wget ripgrep fd-find bat jq htop strace gdb \
    sudo fish zsh \
    # Clean up
    && rm -rf /var/lib/apt/lists/*

# ── Node.js (required by Claude Code and Codex CLI) ─────────────────
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# ── AI CLIs ─────────────────────────────────────────────────────────
RUN npm install -g @anthropic-ai/claude-code @openai/codex

# ── Rust tooling ────────────────────────────────────────────────────
RUN rustup target add x86_64-unknown-linux-musl \
    && rustup component add \
      rust-src rust-analyzer clippy rustfmt \
    && cargo install --locked \
      cargo-watch cargo-edit cargo-expand cargo-nextest \
      cargo-deny cargo-outdated cargo-audit \
      sccache bacon

# ── sccache as default compiler wrapper ─────────────────────────────
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/home/dev/.cache/sccache

# ── Create non-root user matching host UID ──────────────────────────
ARG USER_UID=1001
ARG USER_GID=1000
RUN groupadd --gid ${USER_GID} dev 2>/dev/null || true \
    && useradd --uid ${USER_UID} --gid ${USER_GID} -m -s /bin/bash dev \
    && echo "dev ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/dev \
    && chmod 0440 /etc/sudoers.d/dev

# ── Runtime dir for Wayland ─────────────────────────────────────────
RUN mkdir -p /run/user/${USER_UID} \
    && chown dev:${USER_GID} /run/user/${USER_UID} \
    && chmod 700 /run/user/${USER_UID}

# ── Workspace ───────────────────────────────────────────────────────
WORKDIR /workspaces/rust-agent
RUN chown dev:${USER_GID} /workspaces/rust-agent

USER dev
