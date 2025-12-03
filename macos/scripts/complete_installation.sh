#!/usr/bin/env zsh

# Runs AFTER .pkg completes, in visible Terminal
set -euo pipefail

#------------------------------------------------------------------------------------------
#                                       Step 0: Setup
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Step 1: Homebrew
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Step 2: Download MayaFlux
#------------------------------------------------------------------------------------------

log "➤ Downloading latest MayaFlux release..."

RELEASE_RESPONSE=$(curl -s -H "Accept: application/vnd.github.v3+json" \
    -H "User-Agent: Weave-Installer/1.0" \
    "https://api.github.com/repos/MayaFlux/MayaFlux/releases")

TAG=$(echo "$RELEASE_RESPONSE" |
    sed -n 's/.*"tag_name":\s*"\([^"]*\)".*/\1/p' |
    head -1)

if [ -z "$TAG" ]; then
    error "Failed to get latest release tag"
    exit 1
fi

log "✓ Latest release: $TAG"

# Look for asset URLs in the response and filter for macos-arm64
ASSET_URL=$(echo "$RELEASE_RESPONSE" |
    grep -o '"browser_download_url":\s*"[^"]*"' |
    sed 's/"browser_download_url":\s*"\([^"]*\)"/\1/' |
    grep -i "macos.*arm64.*\.tar\.gz" |
    head -1)

if [ -z "$ASSET_URL" ]; then
    # Try alternative approach - construct the URL
    log "⚠ Could not find asset URL, constructing from tag..."

    # Clean tag (remove 'v' prefix if present)
    CLEAN_TAG=${TAG#v}

    # Try different possible asset name patterns
    PATTERNS=(
        "MayaFlux-${CLEAN_TAG}-macos-arm64.tar.gz"
        "MayaFlux-${TAG}-macos-arm64.tar.gz"
        "${CLEAN_TAG}-macos-arm64.tar.gz"
        "${TAG}-macos-arm64.tar.gz"
    )

    for PATTERN in "${PATTERNS[@]}"; do
        TEST_URL="https://github.com/MayaFlux/MayaFlux/releases/download/${TAG}/${PATTERN}"
        if curl -s -I "$TEST_URL" 2>/dev/null | grep -q "200 OK"; then
            ASSET_URL="$TEST_URL"
            break
        fi
    done

    if [ -z "$ASSET_URL" ]; then
        error "Could not find or construct valid download URL"
        exit 1
    fi
fi

ASSET_NAME=$(basename "$ASSET_URL")
log "✓ Found asset: $ASSET_NAME"

sudo mkdir -p "$MAYAFLUX_INSTALL_DIR"
TMPDIR_DOWNLOAD=$(mktemp -d)
trap 'rm -rf "$TMPDIR_DOWNLOAD"' EXIT

log "  Downloading..."
curl -fL --progress-bar "$ASSET_URL" -o "$TMPDIR_DOWNLOAD/release.tar.gz"

if [ ! -s "$TMPDIR_DOWNLOAD/release.tar.gz" ]; then
    error "Download failed or empty"
    exit 1
fi

DOWNLOADED_SIZE=$(stat -f%z "$TMPDIR_DOWNLOAD/release.tar.gz" 2>/dev/null || echo "0")
DOWNLOADED_SIZE_MB=$((DOWNLOADED_SIZE / 1024 / 1024))
log "  Downloaded: ${DOWNLOADED_SIZE_MB} MB"

log "  Extracting..."
tar -xzf "$TMPDIR_DOWNLOAD/release.tar.gz" -C "$TMPDIR_DOWNLOAD"

log "  Installing to $MAYAFLUX_INSTALL_DIR..."
sudo cp -R "$TMPDIR_DOWNLOAD"/bin "$MAYAFLUX_INSTALL_DIR/" 2>/dev/null
sudo cp -R "$TMPDIR_DOWNLOAD"/lib "$MAYAFLUX_INSTALL_DIR/" 2>/dev/null
sudo cp -R "$TMPDIR_DOWNLOAD"/include "$MAYAFLUX_INSTALL_DIR/" 2>/dev/null
sudo cp -R "$TMPDIR_DOWNLOAD"/share "$MAYAFLUX_INSTALL_DIR/" 2>/dev/null

if [ ! -f "$MAYAFLUX_INSTALL_DIR/lib/libMayaFluxLib.dylib" ]; then
    error "Verification failed - libMayaFluxLib.dylib not found"
    exit 1
fi

echo "$TAG" | sudo tee "$MAYAFLUX_INSTALL_DIR/.version" >/dev/null
log "✅ MayaFlux $TAG installed"

#------------------------------------------------------------------------------------------
#                                       Step 3: Install dependencies
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Step 4: STB Headers
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Step 5: Vulkan SDK
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Step 6: Environment setup
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Step 7: Weave CLI and templates
#------------------------------------------------------------------------------------------

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

#------------------------------------------------------------------------------------------
#                                       Conclusion
#------------------------------------------------------------------------------------------

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
