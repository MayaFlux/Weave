#!/usr/bin/env zsh

# Runs AFTER .pkg completes, in visible Terminal
set -euo pipefail

# Configuration
MAYAFLUX_INSTALL_DIR="/Library/MayaFlux"
WEAVE_LOCATION="/Library/Weave"
LOG_FILE="$HOME/.weave_install.log"

show_progress() {
    log "➤ $1..."
}

show_complete() {
    log "✅ $1"
}

log() {
    echo "$*"
    echo "$*" >>"$LOG_FILE"
}

error() {
    echo "ERROR: $*"
    echo "ERROR: $*" >>"$LOG_FILE"
}

clear
cat <<'BANNER'
╔════════════════════════════════════════════════════════════╗
║                                                            ║
║ 🎛️  Weave - MayaFlux Installation, Dependency Management   ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝

BANNER

log "Starting Installation..."
log ""

# Step 1: Homebrew
log "➤ Checking for Homebrew..."

BREW_CMD=""
for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
        BREW_CMD="$brew_path"
        break
    fi
done

if [ -z "$BREW_CMD" ]; then
    log "Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew; do
        if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
            BREW_CMD="$brew_path"
            break
        fi
    done

    if [ -z "$BREW_CMD" ]; then
        error "Failed to install Homebrew"
        exit 1
    fi
fi

log "✅ Homebrew ready"

# Step 2: Download MayaFlux
log "➤ Downloading latest MayaFlux release..."

if [ -f "$MAYAFLUX_INSTALL_DIR/lib/libMayaFluxLib.dylib" ]; then
    log "✅ MayaFlux already installed"
else
    sudo mkdir -p "$MAYAFLUX_INSTALL_DIR"

    TMPDIR_DOWNLOAD=$(mktemp -d)
    trap 'rm -rf "$TMPDIR_DOWNLOAD"' EXIT

    # Just download the known prerelease directly
    TAG="v0.1.0-dev"
    ASSET_NAME="MayaFlux-0.1.0-dev-macos-arm64.tar.gz"
    ASSET_URL="https://github.com/MayaFlux/MayaFlux/releases/download/${TAG}/${ASSET_NAME}"

    log "  Downloading $ASSET_NAME..."

    curl -fL --progress-bar "$ASSET_URL" -o "$TMPDIR_DOWNLOAD/release.tar.gz"

    DOWNLOADED_SIZE=$(stat -f%z "$TMPDIR_DOWNLOAD/release.tar.gz")
    DOWNLOADED_SIZE_MB=$((DOWNLOADED_SIZE / 1024 / 1024))
    log "  Downloaded: ${DOWNLOADED_SIZE_MB} MB"

    log "  Extracting..."
    tar -xzf "$TMPDIR_DOWNLOAD/release.tar.gz" -C "$TMPDIR_DOWNLOAD"

    # The archive extracts directly to bin/, lib/, etc. - no dist_staging
    # Just copy everything from the temp dir to the install dir
    log "  Installing to $MAYAFLUX_INSTALL_DIR..."
    sudo cp -R "$TMPDIR_DOWNLOAD"/bin "$MAYAFLUX_INSTALL_DIR/"
    sudo cp -R "$TMPDIR_DOWNLOAD"/lib "$MAYAFLUX_INSTALL_DIR/"
    sudo cp -R "$TMPDIR_DOWNLOAD"/include "$MAYAFLUX_INSTALL_DIR/"
    sudo cp -R "$TMPDIR_DOWNLOAD"/share "$MAYAFLUX_INSTALL_DIR/"

    if [ ! -f "$MAYAFLUX_INSTALL_DIR/lib/libMayaFluxLib.dylib" ]; then
        error "Verification failed"
        sudo ls -la "$MAYAFLUX_INSTALL_DIR"
        exit 1
    fi

    echo "$TAG" | sudo tee "$MAYAFLUX_INSTALL_DIR/.version" >/dev/null
    log "✅ MayaFlux $TAG installed"
fi

# Step 3: Install dependencies
log "➤ Installing dependencies..."
log "  This may take several minutes..."

DEPS=(pkgconfig llvm ffmpeg rtaudio glfw glm eigen fmt magic_enum onedpl googletest)
DEPS_TO_INSTALL=()

for dep in "${DEPS[@]}"; do
    if ! "$BREW_CMD" list "$dep" &>/dev/null; then
        DEPS_TO_INSTALL+=("$dep")
    fi
done

if [ ${#DEPS_TO_INSTALL[@]} -gt 0 ]; then
    log "  Installing: ${DEPS_TO_INSTALL[*]}"
    "$BREW_CMD" install "${DEPS_TO_INSTALL[@]}"
    log "✅ Dependencies installed"
else
    log "✅ All dependencies already installed"
fi

# Step 4: STB Headers
STB_INSTALL_DIR="$HOME/Libraries/stb"
STB_HEADER_CHECK="$STB_INSTALL_DIR/stb_image.h"

if [ ! -f "$STB_HEADER_CHECK" ]; then
    mkdir -p "$STB_INSTALL_DIR"

    STB_HEADERS=(
        "stb_image.h"
        "stb_image_write.h"
        "stb_image_resize.h"
        "stb_truetype.h"
        "stb_rect_pack.h"
    )

    for header in "${STB_HEADERS[@]}"; do
        header_url="https://raw.githubusercontent.com/nothings/stb/master/$header"
        header_path="$STB_INSTALL_DIR/$header"

        if ! curl -fL "$header_url" -o "$header_path" 2>/dev/null; then
            error "Failed to download STB header: $header"
        fi
    done

    log 'STB headers installed to %s\n' "$STB_INSTALL_DIR"
else
    log 'STB already installed at %s\n' "$STB_INSTALL_DIR"
fi

# Step 5: Vulkan SDK
show_progress "Setting up Vulkan SDK..."

VULKAN_SDK_ROOT="$HOME/VulkanSDK"
if [ -d "$VULKAN_SDK_ROOT" ] && [ -n "$(find "$VULKAN_SDK_ROOT" -mindepth 1 -maxdepth 1 -type d 2>/dev/null)" ]; then
    show_complete "Vulkan SDK already installed"
else
    SDK_VERSION=$(curl -fsSL https://vulkan.lunarg.com/sdk/latest/mac.txt)
    SDK_URL="https://sdk.lunarg.com/sdk/download/${SDK_VERSION}/mac/vulkan_sdk.zip"

    TMPDIR_VULKAN=$(mktemp -d)

    log "  Downloading Vulkan SDK v$SDK_VERSION..."
    curl -L --progress-bar "$SDK_URL" -o "$TMPDIR_VULKAN/vulkan_sdk.zip"

    log "  Extracting..."
    unzip -q "$TMPDIR_VULKAN/vulkan_sdk.zip" -d "$TMPDIR_VULKAN"

    INSTALLER_APP=$(find "$TMPDIR_VULKAN" -name "*.app" -type d | head -n1)
    if [ -n "$INSTALLER_APP" ]; then
        mkdir -p "$VULKAN_SDK_ROOT/$SDK_VERSION"
        log "  Running Vulkan installer..."
        sudo "$INSTALLER_APP/Contents/MacOS/$(basename "$INSTALLER_APP" .app)" \
            --root "$VULKAN_SDK_ROOT/$SDK_VERSION" \
            --accept-licenses --default-answer --confirm-command install \
            com.lunarg.vulkan.core com.lunarg.vulkan.usr
    fi

    rm -rf "$TMPDIR_VULKAN"
    show_complete "Vulkan SDK installed"
fi

# Step 6: Environment setup
show_progress "Configuring environment..."

ZSHENV="${ZDOTDIR:-$HOME}/.zshenv"
touch "$ZSHENV"

append_if_missing() {
    if ! grep -Fq "$1" "$ZSHENV" 2>/dev/null; then
        echo "$1" >>"$ZSHENV"
    fi
}

append_if_missing "# MayaFlux Environment (added by Weave installer)"
append_if_missing "export MAYAFLUX_ROOT=\"$MAYAFLUX_INSTALL_DIR\""
append_if_missing "export PATH=\"\$MAYAFLUX_ROOT/bin:\$HOME/.local/bin:\$PATH\""
append_if_missing "export CMAKE_PREFIX_PATH=\"\$MAYAFLUX_ROOT:\$CMAKE_PREFIX_PATH\""

VULKAN_SDK_PATH=$(find "$VULKAN_SDK_ROOT" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -n1)
if [ -n "$VULKAN_SDK_PATH" ]; then
    append_if_missing "export VULKAN_SDK=\"$VULKAN_SDK_PATH/macOS\""
    append_if_missing "export PATH=\"\$VULKAN_SDK/bin:\$PATH\""
fi

append_if_missing 'export STB_ROOT="$HOME/Libraries"'
append_if_missing 'export CMAKE_PREFIX_PATH="$STB_ROOT:$CMAKE_PREFIX_PATH"'
append_if_missing 'export CPATH="$STB_ROOT/:$CPATH"'

set +e
LLVM_PREFIX="$("$BREW_CMD" --prefix llvm 2>/dev/null)"
set -e

if [ -n "$LLVM_PREFIX" ]; then
    if ! grep -Fq "$LLVM_PREFIX/bin" "$ZSHENV" 2>/dev/null; then
        append_if_missing "# LLVM setup for CMake / llvm-config"
        append_if_missing "export PATH=\"$LLVM_PREFIX/bin:\$PATH\""
        append_if_missing "export CMAKE_PREFIX_PATH=\"$LLVM_PREFIX/lib/cmake:\$CMAKE_PREFIX_PATH\""
        append_if_missing "export LLVM_DIR=\"$LLVM_PREFIX/lib/cmake/llvm\""
        append_if_missing "export Clang_DIR=\"$LLVM_PREFIX/lib/cmake/clang\""
    fi
fi

show_complete "Environment configured"

# Step 7: Install Weave CLI and templates
show_progress "Installing Weave CLI and templates..."

# Create MayaFlux share directory structure
sudo mkdir -p "$MAYAFLUX_INSTALL_DIR/share/weave"

# Copy templates from package location to MayaFlux
if [ -d "/Library/Weave/templates" ]; then
    sudo cp -R "/Library/Weave/templates" "$MAYAFLUX_INSTALL_DIR/share/weave/"
    show_complete "Templates installed to MayaFlux"
else
    error "Templates not found in /Library/Weave - package installation may have failed"
    exit 1
fi

WEAVE_BIN="$HOME/.local/bin"
mkdir -p "$WEAVE_BIN"
cp "/Library/Weave/project_creator.sh" "$WEAVE_BIN/weave"

show_complete "Weave CLI installed"

log ""
log "=========================================="
log "✅ Installation Complete!"
log "=========================================="
log ""
log "Next steps:"
log "  1. Restart your terminal or run: source ~/.zshenv"
log "  2. Create a project: weave new MyProject ~/Projects/"
log "  3. Build and run: cd ~/Projects/MyProject && mkdir build && cd build && cmake .. && make && ./MyProject"
log ""
log "Installation log saved to: $LOG_FILE"
log ""

echo "Press any key to close this window..."
read -n 1
