#!/bin/bash

set -e

get_sudo_password() {
    local password=""

    if command -v zenity &>/dev/null; then
        password=$(zenity --password --title="Weave Installer" --text="Weave Installer requires administrator privileges.\n\nPlease enter your password:" 2>/dev/null)
    elif command -v kdialog &>/dev/null; then
        password=$(kdialog --title "Weave Installer" --password "Weave Installer requires administrator privileges.\n\nPlease enter your password:" 2>/dev/null)
    elif command -v yad &>/dev/null; then
        password=$(yad --title="Weave Installer" --text="Weave Installer requires administrator privileges.\n\nPlease enter your password:" --entry --hide-text 2>/dev/null)
    elif command -v python3 &>/dev/null; then
        # Python Tkinter fallback
        password=$(python3 -c "
import tkinter as tk
from tkinter import simpledialog
root = tk.Tk()
root.withdraw()
root.attributes('-topmost', True)
password = simpledialog.askstring('Weave Installer', 
    'Weave Installer requires administrator privileges.\\n\\nPlease enter your password:', 
    show='*')
print(password if password else '')
" 2>/dev/null)
    fi

    if [[ -z "$password" ]]; then
        echo "Weave Installer requires administrator privileges." >&2
        echo "Please enter your password (typing is hidden):" >&2
        read -rsp "" password
        echo "" >&2
    fi

    echo "$password"
}

run_with_sudo() {
    if [[ $EUID -eq 0 ]]; then
        "$@"
    else
        if [[ -z "${SUDO_PASSWORD+x}" ]]; then
            SUDO_PASSWORD=$(get_sudo_password)

            if ! echo "$SUDO_PASSWORD" | sudo -S -v 2>/dev/null; then
                echo "ERROR: Incorrect password. Please run the installer again." >&2
                exit 1
            fi
            export SUDO_PASSWORD
        fi

        echo "$SUDO_PASSWORD" | sudo -S "$@"
    fi
}

show_gui_message() {
    local title="$1"
    local message="$2"

    if command -v zenity &>/dev/null; then
        zenity --info --title="$title" --text="$message" --width=400 2>/dev/null || true
    elif command -v kdialog &>/dev/null; then
        kdialog --title "$title" --msgbox "$message" 2>/dev/null || true
    elif command -v yad &>/dev/null; then
        yad --title="$title" --text="$message" --button="OK" 2>/dev/null || true
    else
        echo ""
        echo "=== $title ==="
        echo "$message"
        echo ""
    fi
}

show_sudo_warning() {
    show_gui_message "Weave Installer" "Welcome to Weave Installer!\n\nThis installer will set up MayaFlux development environment.\n\nYou may be asked for your password to install system packages."
}

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

    show_gui_message "Arch Linux Detected" "Installing MayaFlux for Arch Linux.\n\nThis will install mayaflux-dev-bin from AUR."

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
    run_with_sudo dnf install -y \
        @development-tools llvm llvm-libs clang cmake pkgconfig \
        rtaudio-devel glfw-devel glm-devel eigen3-devel \
        spirv-headers spirv-tools vulkan-headers vulkan-loader vulkan-tools \
        vulkan-validation-layers ffmpeg-devel stb-devel magic_enum-devel
}

install_ubuntu() {
    echo "Installing for Ubuntu/Debian..."
    run_with_sudo apt-get update
    run_with_sudo apt-get install -y \
        build-essential cmake git pkg-config llvm llvm-dev clang \
        librtaudio-dev libglfw3-dev libglm-dev libeigen3-dev \
        libvulkan-dev vulkan-tools spirv-tools \
        ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswscale-dev \
        libstb-dev
}

install_opensuse() {
    echo "Installing for openSUSE..."
    run_with_sudo zypper install -y \
        gcc gcc-c++ cmake git pkg-config llvm-devel clang \
        rtaudio-devel glfw-devel glm-devel eigen3-devel \
        vulkan-devel vulkan-tools spirv-tools \
        ffmpeg-4-libavcodec-devel ffmpeg-4-libavformat-devel ffmpeg-4-libavutil-devel ffmpeg-4-libswscale-devel
}

show_sudo_warning

DISTRO=$(detect_distro)

show_gui_message "Distribution Detected" "Detected: $DISTRO\n\nProceeding with installation."

case "$DISTRO" in
arch) install_arch ;;
fedora) install_fedora ;;
ubuntu) install_ubuntu ;;
opensuse) install_opensuse ;;
*)
    echo "Unsupported distro: $DISTRO"
    show_gui_message "Unsupported Distribution" "Sorry, Weave Installer doesn't support $DISTRO yet.\n\nPlease check our documentation for manual installation instructions."
    exit 1
    ;;
esac

unset SUDO_PASSWORD

echo "✓ Dependencies installed"
show_gui_message "Installation Complete" "Weave installation complete! 🎉\n\nNext steps:\n1. Restart your terminal or run: source ~/.bashrc (or ~/.zshrc)\n2. Create a project: weave new MyProject ~/Projects/\n3. Build and run your project"
