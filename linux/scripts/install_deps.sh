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
    else
        echo "unknown"
    fi
}

ensure_add_apt_repository() {
    if ! command -v add-apt-repository >/dev/null 2>&1; then
        echo "add-apt-repository not found. Installing software-properties-common..."
        run_with_sudo apt-get update
        run_with_sudo apt-get install -y software-properties-common
    fi
}

install_via_package_manager() {
    # This function handles distros where MayaFlux is available via native package managers
    # Currently: Arch Linux (AUR) and Fedora (COPR)

    local distro="$1"

    case "$distro" in
    arch)
        echo "Installing for Arch Linux..."
        show_gui_message "Arch Linux Detected" "Installing MayaFlux for Arch Linux.\n\nThis will install mayaflux-dev-bin from AUR."

        # Dummy sudo call to trigger GUI password prompt and cache it for AUR helper
        echo "Caching sudo password for AUR installation..."
        run_with_sudo true

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
        ;;

    fedora)
        echo "Installing for Fedora..."
        show_gui_message "Fedora Detected" "Installing MayaFlux for Fedora.\n\nThis will enable the COPR repository and install mayaflux-dev."

        echo "Enabling COPR repository..."
        run_with_sudo dnf copr enable -y ranjithshegde/spirv-cross
        run_with_sudo dnf copr enable -y ranjithshegde/mayaflux-dev

        echo "Installing mayaflux-dev..."
        run_with_sudo dnf install -y mayaflux-dev
        ;;

    *)
        echo "ERROR: Package manager installation requested for unsupported distro: $distro"
        return 1
        ;;

    ubuntu)
        echo "Installing for Ubuntu..."
        show_gui_message "Ubuntu Detected" \
            "Installing MayaFlux for Ubuntu.\n\nThis will enable the Launchpad PPA and install mayaflux-dev."

        echo "Enabling MayaFlux PPA..."
        ensure_add_apt_repository
        run_with_sudo add-apt-repository -y ppa:mayaflux/mayaflux-dev
        run_with_sudo apt-get update

        echo "Installing mayaflux-edge..."
        run_with_sudo apt-get install -y mayaflux-edge
        ;;
    esac

    return 0
}

install_manual_deps() {
    # This function installs dependencies manually for distros without native MayaFlux packages
    # Used for: Ubuntu/Debian, openSUSE, and other unsupported distros

    local distro="$1"

    case "$distro" in
    opensuse)
        echo "Installing dependencies for openSUSE..."
        run_with_sudo zypper install -y \
            gcc-c++ gcc \
            clang llvm llvm-devel clang-devel \
            cmake ninja pkg-config git \
            rtaudio-devel \
            glfw-devel \
            glm-devel \
            eigen3-devel \
            spirv-headers spirv-tools \
            vulkan-headers vulkan-loader vulkan-loader-devel vulkan-tools vulkan-validationlayers \
            ffmpeg-4-libavcodec-devel ffmpeg-4-libavformat-devel ffmpeg-4-libavutil-devel \
            ffmpeg-4-libswscale-devel ffmpeg-4-libavdevice-devel \
            stb-devel \
            magic_enum-devel \
            tbb-devel \
            gtest \
            shaderc-devel \
            wayland-devel
        ;;

    *)
        echo "WARNING: Unsupported distribution: $distro"
        echo "MayaFlux installation may require manual dependency setup."
        show_gui_message "Unsupported Distribution" "Sorry, Weave Installer doesn't fully support $distro yet.\n\nPlease check our documentation for manual installation instructions."
        return 1
        ;;
    esac

    return 0
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

show_sudo_warning

DISTRO=$(detect_distro)

echo "Detected distribution: $DISTRO"

case "$DISTRO" in
arch | fedora | ubuntu)
    # These distros have native MayaFlux packages - use package manager
    show_gui_message "Distribution Detected" "Detected: $DISTRO\n\nMayaFlux will be installed via package manager."

    if install_via_package_manager "$DISTRO"; then
        echo "✓ MayaFlux and dependencies installed successfully"
        show_gui_message "Installation Complete" "Weave installation complete! 🎉\n\nNext steps:\n1. Restart your terminal or run: source ~/.bashrc (or ~/.zshrc)\n2. Create a project: weave new MyProject ~/Projects/\n3. Build and run your project"
    else
        echo "✗ Installation failed"
        exit 1
    fi
    ;;

opensuse)
    # These distros don't have native MayaFlux packages - install deps manually
    show_gui_message "Distribution Detected" "Detected: $DISTRO\n\nInstalling dependencies manually.\n\nYou will need to download MayaFlux separately."

    if install_manual_deps "$DISTRO"; then
        echo "✓ Dependencies installed successfully"
        echo ""
        echo "NOTE: MayaFlux framework must be downloaded separately."
        echo "The installer will handle this in the next step."
        show_gui_message "Dependencies Installed" "Development dependencies installed.\n\nMayaFlux framework will be downloaded in the next step."
    else
        echo "✗ Dependency installation failed"
        exit 1
    fi
    ;;

unknown)
    echo "ERROR: Could not detect Linux distribution"
    show_gui_message "Unknown Distribution" "Could not detect your Linux distribution.\n\nPlease install dependencies manually."
    exit 1
    ;;
esac

unset SUDO_PASSWORD
