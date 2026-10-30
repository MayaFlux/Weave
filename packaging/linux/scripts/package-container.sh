#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../../.." && pwd)"
if command -v podman >/dev/null; then
    ENGINE=podman
elif command -v docker >/dev/null; then
    ENGINE=docker
else
    echo "Install Podman or Docker to build the Ubuntu 26.04 AppImage on this host." >&2
    exit 1
fi

"$ENGINE" build -f "$ROOT_DIR/packaging/linux/Containerfile" \
    -t weave-appimage-builder:ubuntu-26.04 "$ROOT_DIR/packaging/linux"

RUN_OPTIONS=(--rm -v "$ROOT_DIR:/workspace:z" -w /workspace \
    -e CARGO_HOME=/workspace/target/packaging/linux/cargo-home \
    -e CARGO_TARGET_DIR=/workspace/target/packaging/linux/cargo-ubuntu26)
if [[ "$ENGINE" == podman ]]; then
    RUN_OPTIONS+=(--userns=keep-id)
else
    RUN_OPTIONS+=(--user "$(id -u):$(id -g)")
fi

"$ENGINE" run "${RUN_OPTIONS[@]}" weave-appimage-builder:ubuntu-26.04 \
    bash packaging/linux/scripts/package.sh target/packaging/linux/output
echo "AppImage: $ROOT_DIR/target/packaging/linux/output/"
