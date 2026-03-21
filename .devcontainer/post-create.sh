#!/usr/bin/env bash
set -euo pipefail

echo "── post-create: setting up Rust devterm ──"

# Ensure cargo bin is on PATH
CARGO_BIN="$HOME/.cargo/bin"
if ! grep -q 'cargo/bin' "$HOME/.bashrc" 2>/dev/null; then
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$HOME/.bashrc"
fi

# Initialize a Cargo project if none exists yet
if [ ! -f Cargo.toml ]; then
  echo "No Cargo.toml found — skipping cargo init (create one with 'cargo init')"
fi

# Verify Wayland connectivity
if [ -S "${XDG_RUNTIME_DIR:-/run/user/1001}/${WAYLAND_DISPLAY:-wayland-0}" ]; then
  echo "Wayland socket: OK"
else
  echo "WARNING: Wayland socket not found — GUI apps may not work"
fi

# Warm sccache
sccache --start-server 2>/dev/null || true
# Only chown container-local dirs — avoid bind mounts like .claude/.codex
sudo chown -R dev:dev /home/dev/.cargo /home/dev/.cache 2>/dev/null || true
rustup default stable
echo "── post-create: done ──"
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
