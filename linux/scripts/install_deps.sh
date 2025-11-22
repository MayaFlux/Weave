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
    sudo pacman -Syu --noconfirm \
        base-devel cmake git pkg-config llvm clang \
        rtaudio glfw glm eigen ffmpeg vulkan-devel
}

install_fedora() {
    echo "Installing for Fedora..."
    sudo dnf install -y \
        @development-tools cmake git pkg-config llvm-devel clang \
        rtaudio-devel glfw-devel eigen3-devel ffmpeg-devel vulkan-devel
}

install_ubuntu() {
    sudo apt-get update
    echo "Installing for Ubuntu/Debian..."
    sudo apt-get install -y \
        build-essential cmake git pkg-config llvm llvm-dev clang \
        librtaudio-dev libglfw3-dev libeigen3-dev ffmpeg libvulkan-dev
}

install_opensuse() {
    echo "Installing for openSUSE..."
    sudo zypper install -y \
        gcc gcc-c++ cmake git pkg-config llvm-devel clang \
        rtaudio-devel glfw-devel eigen3-devel ffmpeg-devel vulkan-devel
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
