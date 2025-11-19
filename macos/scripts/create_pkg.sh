#!/usr/bin/env zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$MACOS_DIR")"
BUILD_DIR="$ROOT_DIR/build/macos"
VERSION="${1:-0.1.0}"

echo "Building Weave installer v$VERSION with sequential package steps..."

# Clean
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# ============================================================================
# BUILD WEAVE.APP GUI
# ============================================================================

echo "Building Weave.app GUI..."
"$MACOS_DIR/scripts/build_weave_app.sh"

# ============================================================================
# COMPONENT 1: Weave Files (no scripts, just payload)
# ============================================================================

echo "Building Weave-files.pkg..."

mkdir -p "$BUILD_DIR/files_payload/Library/Weave"

cp "$MACOS_DIR/Weave.sh" "$BUILD_DIR/files_payload/Library/Weave/project_creator.sh"
chmod +x "$BUILD_DIR/files_payload/Library/Weave/project_creator.sh"

cp -R "$ROOT_DIR/templates" "$BUILD_DIR/files_payload/Library/Weave/templates"

pkgbuild --root "$BUILD_DIR/files_payload" \
    --identifier com.mayaflux.weave.files \
    --version "$VERSION" \
    "$BUILD_DIR/Weave-files.pkg"

# ============================================================================
# COMPONENT 1B: Weave.app GUI
# ============================================================================

echo "Building Weave-gui.pkg..."

mkdir -p "$BUILD_DIR/gui_payload/Applications"
cp -R "$BUILD_DIR/WeaveApp/Weave.app" "$BUILD_DIR/gui_payload/Applications/Weave.app"

pkgbuild --root "$BUILD_DIR/gui_payload" \
    --identifier com.mayaflux.weave.gui \
    --version "$VERSION" \
    "$BUILD_DIR/Weave-gui.pkg"

# ============================================================================
# COMPONENT 2: Homebrew Check & Install (postinstall handles it, BLOCKS)
# ============================================================================

echo "Building Weave-homebrew.pkg..."

mkdir -p "$BUILD_DIR/homebrew_scripts"

cat > "$BUILD_DIR/homebrew_scripts/postinstall" << 'HOMEBREW_SCRIPT'
#!/usr/bin/env zsh
set -euo pipefail

INSTALLER_LOG="/var/tmp/weave_install.log"

log() {
    echo "[Weave-Homebrew] $*" | tee -a "$INSTALLER_LOG"
}

error() {
    echo "[Weave-Homebrew ERROR] $*" | tee -a "$INSTALLER_LOG" >&2
    exit 1
}

log "====== HOMEBREW INSTALLATION STEP ======"

BREW_PATHS=(
    "/opt/homebrew/bin/brew"
    "/usr/local/bin/brew"
)

BREW_CMD=""
for brew_path in "${BREW_PATHS[@]}"; do
    if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
        BREW_CMD="$brew_path"
        log "Found Homebrew at: $BREW_CMD"
        break
    fi
done

if [ -z "$BREW_CMD" ]; then
    log "Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" 2>&1 | tee -a "$INSTALLER_LOG"
    sleep 5
    
    for brew_path in "${BREW_PATHS[@]}"; do
        if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
            BREW_CMD="$brew_path"
            log "Homebrew installed at: $BREW_CMD"
            break
        fi
    done
    
    [ -z "$BREW_CMD" ] && error "Homebrew installation failed"
fi

log "✅ Homebrew verified"
exit 0
HOMEBREW_SCRIPT

chmod 755 "$BUILD_DIR/homebrew_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.homebrew \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/homebrew_scripts" \
    "$BUILD_DIR/Weave-homebrew.pkg"

# ============================================================================
# COMPONENT 3: JQ Installation (postinstall, BLOCKS)
# ============================================================================

echo "Building Weave-jq.pkg..."

mkdir -p "$BUILD_DIR/jq_scripts"

cat > "$BUILD_DIR/jq_scripts/postinstall" << 'JQ_SCRIPT'
#!/usr/bin/env zsh
set -euo pipefail

INSTALLER_LOG="/var/tmp/weave_install.log"

log() {
    echo "[Weave-JQ] $*" | tee -a "$INSTALLER_LOG"
}

error() {
    echo "[Weave-JQ ERROR] $*" | tee -a "$INSTALLER_LOG" >&2
    exit 1
}

log "====== JQ INSTALLATION STEP ======"

# Find brew (should exist from previous step)
BREW_CMD=""
for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew /usr/local/Homebrew/bin/brew; do
    if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
        BREW_CMD="$brew_path"
        break
    fi
done

[ -z "$BREW_CMD" ] && error "Homebrew not found"

log "Installing jq via: $BREW_CMD"

# Get non-root user
ACTUAL_USER="${SUDO_USER:-nobody}"
if [ "$ACTUAL_USER" = "nobody" ]; then
    ACTUAL_USER=$(ps aux | grep -v root | grep -v "^_" | head -1 | awk '{print $1}')
fi

log "Running as user: $ACTUAL_USER"

sudo -u "$ACTUAL_USER" -H "$BREW_CMD" install jq 2>&1 | tee -a "$INSTALLER_LOG" || error "jq installation failed"

log "✅ jq installed"
exit 0
JQ_SCRIPT

chmod 755 "$BUILD_DIR/jq_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.jq \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/jq_scripts" \
    "$BUILD_DIR/Weave-jq.pkg"

# ============================================================================
# COMPONENT 4: MayaFlux Download (postinstall, BLOCKS)
# ============================================================================

echo "Building Weave-mayaflux.pkg..."

mkdir -p "$BUILD_DIR/mayaflux_scripts"

cat > "$BUILD_DIR/mayaflux_scripts/postinstall" << 'MAYAFLUX_SCRIPT'
#!/usr/bin/env zsh
set -euo pipefail

INSTALLER_LOG="/var/tmp/weave_install.log"

log() {
    echo "[Weave-MayaFlux] $*" | tee -a "$INSTALLER_LOG"
}

error() {
    echo "[Weave-MayaFlux ERROR] $*" | tee -a "$INSTALLER_LOG" >&2
    exit 1
}

log "====== MAYAFLUX DOWNLOAD STEP ======"

MAYAFLUX_INSTALL_DIR="/Library/MayaFlux"

if [ -f "$MAYAFLUX_INSTALL_DIR/lib/libMayaFluxLib.dylib" ]; then
    log "MayaFlux already installed, skipping download"
    exit 0
fi

mkdir -p "$MAYAFLUX_INSTALL_DIR"

TMPDIR_DOWNLOAD=$(mktemp -d)
trap 'rm -rf "$TMPDIR_DOWNLOAD"' EXIT

log "Fetching latest release from GitHub..."
RELEASES=$(curl -fsSL "https://api.github.com/repos/MayaFlux/MayaFlux/releases") || error "Failed to fetch releases"

TAG_NAME=$(echo "$RELEASES" | jq -r '.[0].tag_name') || error "Failed to parse tag"
ASSET_NAME=$(echo "$RELEASES" | jq -r ".[] | .assets[] | select(.name | test(\"macos.*\\.tar\\.gz\")) | .name" | head -1) || error "Failed to find asset"
ASSET_URL=$(echo "$RELEASES" | jq -r ".[] | .assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url") || error "Failed to parse URL"

log "Release: $TAG_NAME"
log "Asset: $ASSET_NAME"
log "Downloading..."

curl -fL --progress-bar "$ASSET_URL" -o "$TMPDIR_DOWNLOAD/release.tar.gz" || error "Download failed"

log "Extracting..."
tar -xzf "$TMPDIR_DOWNLOAD/release.tar.gz" -C "$TMPDIR_DOWNLOAD" || error "Extraction failed"

DIST_STAGING=$(find "$TMPDIR_DOWNLOAD" -type d -name "dist_staging" | head -1)
[ -z "$DIST_STAGING" ] && error "dist_staging not found"

cp -R "$DIST_STAGING"/* "$MAYAFLUX_INSTALL_DIR/" || error "Copy failed"

[ ! -f "$MAYAFLUX_INSTALL_DIR/lib/libMayaFluxLib.dylib" ] && error "Verification failed"

echo "$TAG_NAME" > "$MAYAFLUX_INSTALL_DIR/.version"

log "✅ MayaFlux $TAG_NAME installed"
exit 0
MAYAFLUX_SCRIPT

chmod 755 "$BUILD_DIR/mayaflux_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.mayaflux \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/mayaflux_scripts" \
    "$BUILD_DIR/Weave-mayaflux.pkg"

# ============================================================================
# COMPONENT 5: Dependencies Installation
# ============================================================================

echo "Building Weave-dependencies.pkg..."

mkdir -p "$BUILD_DIR/dependencies_scripts"

cat > "$BUILD_DIR/dependencies_scripts/postinstall" << 'DEPS_SCRIPT'
#!/usr/bin/env zsh
set -euo pipefail

INSTALLER_LOG="/var/tmp/weave_install.log"

log() {
    echo "[Weave-Dependencies] $*" | tee -a "$INSTALLER_LOG"
}

error() {
    echo "[Weave-Dependencies ERROR] $*" | tee -a "$INSTALLER_LOG" >&2
    exit 1
}

log "====== DEPENDENCIES INSTALLATION STEP ======"

BREW_CMD=""
for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew /usr/local/Homebrew/bin/brew; do
    if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
        BREW_CMD="$brew_path"
        break
    fi
done

[ -z "$BREW_CMD" ] && error "Homebrew not found"

ACTUAL_USER="${SUDO_USER:-nobody}"
if [ "$ACTUAL_USER" = "nobody" ]; then
    ACTUAL_USER=$(ps aux | grep -v root | grep -v "^_" | head -1 | awk '{print $1}')
fi

log "Checking dependencies..."

DEPS_NEEDED=()
for dep in ffmpeg rtaudio glfw glm eigen fmt magic_enum onedpl googletest; do
    if ! sudo -u "$ACTUAL_USER" -H "$BREW_CMD" list "$dep" &>/dev/null 2>&1; then
        DEPS_NEEDED+=("$dep")
    fi
done

if [ ${#DEPS_NEEDED[@]} -eq 0 ]; then
    log "✅ All dependencies already installed"
else
    log "Installing ${#DEPS_NEEDED[@]} dependencies..."
    sudo -u "$ACTUAL_USER" -H "$BREW_CMD" install "${DEPS_NEEDED[@]}" 2>&1 | tee -a "$INSTALLER_LOG" || log "⚠️  Some dependencies may have failed"
    log "✅ Dependency installation step complete"
fi

exit 0
DEPS_SCRIPT

chmod 755 "$BUILD_DIR/dependencies_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.dependencies \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/dependencies_scripts" \
    "$BUILD_DIR/Weave-dependencies.pkg"

# ============================================================================
# COMPONENT 6: Vulkan SDK Installation
# ============================================================================

echo "Building Weave-vulkan.pkg..."

mkdir -p "$BUILD_DIR/vulkan_scripts"

cat > "$BUILD_DIR/vulkan_scripts/postinstall" << 'VULKAN_SCRIPT'
#!/usr/bin/env zsh
set -euo pipefail

INSTALLER_LOG="/var/tmp/weave_install.log"

log() {
    echo "[Weave-Vulkan] $*" | tee -a "$INSTALLER_LOG"
}

log "====== VULKAN SDK INSTALLATION STEP ======"

ACTUAL_USER="${SUDO_USER:-$(logname 2>/dev/null || echo 'nobody')}"
VULKAN_SDK_ROOT="$HOME/VulkanSDK"

if [ -d "$VULKAN_SDK_ROOT" ] && [ -n "$(find "$VULKAN_SDK_ROOT" -mindepth 1 -maxdepth 1 -type d 2>/dev/null)" ]; then
    log "✅ Vulkan SDK already installed"
    exit 0
fi

log "Fetching Vulkan SDK version..."
SDK_VERSION=$(curl -fsSL https://vulkan.lunarg.com/sdk/latest/mac.txt 2>&1) || {
    log "⚠️  Could not fetch Vulkan SDK version, skipping"
    exit 0
}

[ -z "$SDK_VERSION" ] && exit 0

SDK_URL="https://sdk.lunarg.com/sdk/download/${SDK_VERSION}/mac/vulkan_sdk.zip"

TMPDIR_VULKAN=$(mktemp -d)
trap 'rm -rf "$TMPDIR_VULKAN"' EXIT

log "Downloading Vulkan SDK v$SDK_VERSION..."
curl -fL "$SDK_URL" -o "$TMPDIR_VULKAN/vulkan_sdk.zip" 2>&1 | tee -a "$INSTALLER_LOG" || log "Vulkan download failed, continuing"

if [ -f "$TMPDIR_VULKAN/vulkan_sdk.zip" ]; then
    unzip -q "$TMPDIR_VULKAN/vulkan_sdk.zip" -d "$TMPDIR_VULKAN" 2>&1 || log "Vulkan extraction failed"
    INSTALLER_APP=$(find "$TMPDIR_VULKAN" -name "*.app" -type d | head -n1)
    if [ -n "$INSTALLER_APP" ]; then
        mkdir -p "$VULKAN_SDK_ROOT/$SDK_VERSION"
        log "Running Vulkan installer..."
        sudo "$INSTALLER_APP/Contents/MacOS/$(basename "$INSTALLER_APP" .app)" \
            --root "$VULKAN_SDK_ROOT/$SDK_VERSION" \
            --accept-licenses --default-answer --confirm-command install \
            com.lunarg.vulkan.core com.lunarg.vulkan.usr 2>&1 | tee -a "$INSTALLER_LOG" || true
        log "✅ Vulkan SDK configured"
    fi
fi

exit 0
VULKAN_SCRIPT

chmod 755 "$BUILD_DIR/vulkan_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.vulkan \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/vulkan_scripts" \
    "$BUILD_DIR/Weave-vulkan.pkg"

# ============================================================================
# COMPONENT 7: Environment & Templates Setup
# ============================================================================

echo "Building Weave-environment.pkg..."

mkdir -p "$BUILD_DIR/environment_scripts"

cat > "$BUILD_DIR/environment_scripts/postinstall" << 'ENV_SCRIPT'
#!/usr/bin/env zsh
set -euo pipefail

INSTALLER_LOG="/var/tmp/weave_install.log"

log() {
    echo "[Weave-Environment] $*" | tee -a "$INSTALLER_LOG"
}

error() {
    echo "[Weave-Environment ERROR] $*" | tee -a "$INSTALLER_LOG" >&2
    exit 1
}

log "====== ENVIRONMENT & TEMPLATES SETUP STEP ======"

ACTUAL_USER="${SUDO_USER:-$(logname 2>/dev/null || echo 'nobody')}"
ACTUAL_HOME=$(eval echo ~$ACTUAL_USER)
MAYAFLUX_INSTALL_DIR="/Library/MayaFlux"
WEAVE_LOCATION="/Library/Weave"
WEAVE_BIN="$ACTUAL_HOME/.local/bin"

log "Setting up environment for user: $ACTUAL_USER"

# Setup .zshenv
ZSHENV="${ACTUAL_HOME}/.zshenv"
touch "$ZSHENV" || error "Failed to create $ZSHENV"

append_if_missing() {
    if ! grep -Fq "$1" "$ZSHENV" 2>/dev/null; then
        echo "$1" >> "$ZSHENV" || error "Failed to update $ZSHENV"
    fi
}

append_if_missing "# MayaFlux Environment (added by Weave installer)"
append_if_missing "export MAYAFLUX_ROOT=\"$MAYAFLUX_INSTALL_DIR\""
append_if_missing "export PATH=\"\$MAYAFLUX_ROOT/bin:\$HOME/.local/bin:\$PATH\""
append_if_missing "export CMAKE_PREFIX_PATH=\"\$MAYAFLUX_ROOT:\$CMAKE_PREFIX_PATH\""

# Vulkan setup
VULKAN_SDK_PATH=$(find "$ACTUAL_HOME/VulkanSDK" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -n1)
if [ -n "$VULKAN_SDK_PATH" ]; then
    append_if_missing "export VULKAN_SDK=\"$VULKAN_SDK_PATH/macOS\""
    append_if_missing "export PATH=\"\$VULKAN_SDK/bin:\$PATH\""
    append_if_missing "export DYLD_LIBRARY_PATH=\"\$VULKAN_SDK/lib:\$DYLD_LIBRARY_PATH\""
    append_if_missing "export VK_ICD_FILENAMES=\"\$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json\""
    append_if_missing "export VK_LAYER_PATH=\"\$VULKAN_SDK/etc/vulkan/explicit_layer.d\""
fi

# STB headers
log "Installing STB headers..."
STB_INSTALL_DIR="$ACTUAL_HOME/Libraries/stb"
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
        curl -fL "https://raw.githubusercontent.com/nothings/stb/master/$header" -o "$STB_INSTALL_DIR/$header" 2>&1 | tee -a "$INSTALLER_LOG" || log "Warning: failed to download $header"
    done
fi

append_if_missing "export STB_ROOT=\"$ACTUAL_HOME/Libraries\""
append_if_missing 'export CMAKE_PREFIX_PATH="$STB_ROOT:$CMAKE_PREFIX_PATH"'
append_if_missing 'export CPATH="$STB_ROOT/:$CPATH"'

# LLVM setup
log "Configuring LLVM..."
BREW_CMD=""
for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew /usr/local/Homebrew/bin/brew; do
    if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
        BREW_CMD="$brew_path"
        break
    fi
done

if [ -n "$BREW_CMD" ]; then
    LLVM_PREFIX=$(sudo -u "$ACTUAL_USER" -H "$BREW_CMD" --prefix llvm 2>/dev/null || true)
    if [ -n "$LLVM_PREFIX" ]; then
        if ! grep -Fq "$LLVM_PREFIX/bin" "$ZSHENV" 2>/dev/null; then
            {
                echo ''
                echo '# LLVM setup for CMake / llvm-config'
                echo "export PATH=\"$LLVM_PREFIX/bin:\$PATH\""
                echo "export CMAKE_PREFIX_PATH=\"$LLVM_PREFIX/lib/cmake:\$CMAKE_PREFIX_PATH\""
                echo "export LLVM_DIR=\"$LLVM_PREFIX/lib/cmake/llvm\""
                echo "export Clang_DIR=\"$LLVM_PREFIX/lib/cmake/clang\""
            } >> "$ZSHENV"
        fi
    fi
fi

append_if_missing "# END: MayaFlux Environment (added by Weave installer)"

# Install templates
log "Installing Weave templates..."
WEAVE_SHARE="$MAYAFLUX_INSTALL_DIR/share/weave"
mkdir -p "$WEAVE_SHARE/templates" || error "Failed to create templates directory"

if [ ! -d "$WEAVE_LOCATION/templates" ]; then
    error "Templates not found at $WEAVE_LOCATION/templates"
fi

cp -R "$WEAVE_LOCATION/templates/"* "$WEAVE_SHARE/templates/" 2>&1 | tee -a "$INSTALLER_LOG" || error "Failed to copy templates"

log "✅ Templates installed"

# Install weave CLI tool
log "Installing weave CLI tool..."
mkdir -p "$WEAVE_BIN" || error "Failed to create $WEAVE_BIN"

if [ ! -f "$WEAVE_LOCATION/project_creator.sh" ]; then
    error "Weave tool not found at $WEAVE_LOCATION/project_creator.sh"
fi

cp "$WEAVE_LOCATION/project_creator.sh" "$WEAVE_BIN/weave" 2>&1 | tee -a "$INSTALLER_LOG" || error "Failed to copy weave tool"
chmod +x "$WEAVE_BIN/weave" || error "Failed to make weave executable"

log "✅ Weave CLI tool installed"

log "✅ Environment and templates setup complete"
exit 0
ENV_SCRIPT

chmod 755 "$BUILD_DIR/environment_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.environment \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/environment_scripts" \
    "$BUILD_DIR/Weave-environment.pkg"

# ============================================================================
# FINAL: Combine all packages with Distribution.xml
# ============================================================================

echo "Creating distribution package..."
cat > "$BUILD_DIR/Distribution.xml" << 'DIST_XML'
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>Weave - MayaFlux Installation</title>
    
    <welcome file="welcome.html"/>
    <conclusion file="conclusion.html"/>
    
    <options customize="never" require-scripts="true" hostArchitectures="arm64,x86_64"/>
    
    <domains enable_localSystem="true" enable_anywhere="false" enable_currentUserHome="false"/>
    
    <choices-outline>
        <line choice="weave.files"/>
        <line choice="weave.homebrew"/>
        <line choice="weave.jq"/>
        <line choice="weave.mayaflux"/>
        <line choice="weave.dependencies"/>
        <line choice="weave.vulkan"/>
        <line choice="weave.environment"/>
        <line choice="weave.gui"/>
    </choices-outline>
    
    <choice id="weave.files" title="Weave Files" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.files"/>
    </choice>
    
    <choice id="weave.homebrew" title="Homebrew Setup" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.homebrew"/>
    </choice>
    
    <choice id="weave.jq" title="JQ Installation" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.jq"/>
    </choice>
    
    <choice id="weave.mayaflux" title="MayaFlux Installation" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.mayaflux"/>
    </choice>
    
    <choice id="weave.dependencies" title="Dependencies Installation" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.dependencies"/>
    </choice>
    
    <choice id="weave.vulkan" title="Vulkan SDK Installation" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.vulkan"/>
    </choice>
    
    <choice id="weave.environment" title="Environment & Templates Setup" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.environment"/>
    </choice>

    <choice id="weave.gui" title="Weave GUI Application" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.gui"/>
    </choice>

    <pkg-ref id="com.mayaflux.weave.gui" installKBytes="10000" version="1.0">
        Weave-gui.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.files" installKBytes="1000" version="1.0">
        Weave-files.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.homebrew" installKBytes="100" version="1.0" auth="Admin">
        Weave-homebrew.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.jq" installKBytes="500" version="1.0" auth="Admin">
        Weave-jq.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.mayaflux" installKBytes="2000000" version="1.0" auth="Admin">
        Weave-mayaflux.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.dependencies" installKBytes="500000" version="1.0" auth="Admin">
        Weave-dependencies.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.vulkan" installKBytes="1000000" version="1.0" auth="Admin">
        Weave-vulkan.pkg
    </pkg-ref>
    
    <pkg-ref id="com.mayaflux.weave.environment" installKBytes="5000" version="1.0" auth="Admin">
        Weave-environment.pkg
    </pkg-ref>
</installer-gui-script>
DIST_XML

cp "$MACOS_DIR/resources/welcome.html" "$BUILD_DIR/welcome.html"
cp "$MACOS_DIR/resources/conclusion.html" "$BUILD_DIR/conclusion.html"

productbuild --distribution "$BUILD_DIR/Distribution.xml" \
    --resources "$BUILD_DIR" \
    --package-path "$BUILD_DIR" \
    "$BUILD_DIR/Weave-${VERSION}.pkg"

echo "✅ Installer created: $BUILD_DIR/Weave-${VERSION}.pkg"
