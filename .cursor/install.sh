#!/usr/bin/env bash
# Cloud Agent bootstrap for Lantern (Rust + Tauri 2 + React).
#
# Idempotent: safe to run repeatedly and against a warm snapshot. The heavy,
# stable system packages (WebKitGTK etc.) are only installed when missing, so
# booting from a snapshot that already has them needs no network access.
set -euo pipefail

# --- System dependencies -----------------------------------------------------
# Building the Tauri desktop shell (`lantern-app`) on Linux requires the
# WebKitGTK / GTK / libsoup development headers. The default Cursor image does
# not ship them. rust + node are already provided by the base image.
if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
  echo ">> Installing Tauri Linux system dependencies via apt..."
  export DEBIAN_FRONTEND=noninteractive
  sudo apt-get update -y
  sudo apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    pkg-config \
    patchelf
else
  echo ">> Tauri system dependencies already present; skipping apt."
fi

# --- Rust dependencies -------------------------------------------------------
# Toolchain is pinned by rust-toolchain.toml (stable); rustup selects it
# automatically. Pre-fetch the workspace's crates into the shared cargo cache.
echo ">> Fetching Rust dependencies (cargo fetch --locked)..."
cargo fetch --locked

# --- JavaScript dependencies -------------------------------------------------
# Installs the root + `ui` npm workspaces reproducibly from package-lock.json.
echo ">> Installing npm dependencies (npm ci)..."
npm ci

echo ">> Lantern environment bootstrap complete."
