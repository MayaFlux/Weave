#!/bin/bash

set -e

detect_distro() {
    if command -v pacman &>/dev/null; then
        echo "arch"
    elif command -v dnf &>/dev/null; then
        echo "fedora"
    elif command -v apt-get &>/dev/null; then
        echo "ubuntu"
    elif command -v zypper &>/dev/null; then
        echo "opensuse"
    else echo "unknown"; fi
}

install_arch() {
    echo "Installing for Arch Linux..."

    if command -v yay &>/dev/null; then
        echo "Found yay, installing mayaflux-dev-bin from AUR..."
        yay -S --noconfirm mayaflux-dev-bin
    elif command -v paru &>/dev/null; then
        echo "Found paru, installing mayaflux-dev-bin from AUR..."
        paru -S --noconfirm mayaflux-dev-bin
    else
        echo "No AUR helper found. Building mayaflux-dev-bin from AUR manually..."
        BUILD_DIR=$(mktemp -d)
        cd "$BUILD_DIR"
        git clone https://aur.archlinux.org/mayaflux-dev-bin.git
        cd mayaflux-dev-bin
        makepkg -si --noconfirm
        cd /
        rm -rf "$BUILD_DIR"
    fi
}

install_fedora() {
    echo "Installing for Fedora..."
    sudo dnf install -y \
        @development-tools llvm llvm-libs clang cmake pkgconfig \
        rtaudio-devel glfw-devel glm-devel eigen3-devel \
        spirv-headers spirv-tools vulkan-headers vulkan-loader vulkan-tools \
        vulkan-validation-layers ffmpeg-devel stb-devel magic_enum-devel
}

install_ubuntu() {
    echo "Installing for Ubuntu/Debian..."
    sudo apt-get update
    sudo apt-get install -y \
        build-essential cmake git pkg-config llvm llvm-dev clang \
        librtaudio-dev libglfw3-dev libglm-dev libeigen3-dev \
        libvulkan-dev vulkan-tools spirv-tools \
        ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswscale-dev \
        libstb-dev
}

install_opensuse() {
    echo "Installing for openSUSE..."
    sudo zypper install -y \
        gcc gcc-c++ cmake git pkg-config llvm-devel clang \
        rtaudio-devel glfw-devel glm-devel eigen3-devel \
        vulkan-devel vulkan-tools spirv-tools \
        ffmpeg-4-libavcodec-devel ffmpeg-4-libavformat-devel ffmpeg-4-libavutil-devel ffmpeg-4-libswscale-devel
}

DISTRO=$(detect_distro)

case "$DISTRO" in
arch) install_arch ;;
fedora) install_fedora ;;
ubuntu) install_ubuntu ;;
opensuse) install_opensuse ;;
*)
    echo "Unsupported distro: $DISTRO"
    exit 1
    ;;
esac

echo "✓ Dependencies installed"
