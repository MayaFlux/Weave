#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
if command -v podman >/dev/null; then
    ENGINE=podman
elif command -v docker >/dev/null; then
    ENGINE=docker
else
    echo "Install Podman or Docker to build the Ubuntu 26.04 AppImage on this host." >&2
    exit 1
fi

"$ENGINE" build -f "$ROOT_DIR/linux/Containerfile" \
    -t weave-appimage-builder:ubuntu-26.04 "$ROOT_DIR/linux"

RUN_OPTIONS=(--rm -v "$ROOT_DIR:/workspace:z" -w /workspace \
    -e CARGO_HOME=/workspace/build/linux/cargo-home \
    -e CARGO_TARGET_DIR=/workspace/build/linux/target-ubuntu26)
if [[ "$ENGINE" == podman ]]; then
    RUN_OPTIONS+=(--userns=keep-id)
else
    RUN_OPTIONS+=(--user "$(id -u):$(id -g)")
fi

"$ENGINE" run "${RUN_OPTIONS[@]}" weave-appimage-builder:ubuntu-26.04 \
    bash linux/scripts/package_weave_linux.sh build/linux/rust-package/output
echo "AppImage: $ROOT_DIR/build/linux/rust-package/output/"
