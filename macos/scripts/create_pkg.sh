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
# COMPONENT 1: Weave Files
# ============================================================================

echo "Building Weave-files.pkg..."

mkdir -p "$BUILD_DIR/files_payload/Library/Weave"

cp "$MACOS_DIR/Weave.sh" "$BUILD_DIR/files_payload/Library/Weave/project_creator.sh"
chmod +x "$BUILD_DIR/files_payload/Library/Weave/project_creator.sh"

cp -R "$ROOT_DIR/templates" "$BUILD_DIR/files_payload/Library/Weave/templates"

# Copy complete_installation.sh
cp "$MACOS_DIR/scripts/complete_installation.sh" "$BUILD_DIR/files_payload/Library/Weave/complete_installation.sh"
chmod +x "$BUILD_DIR/files_payload/Library/Weave/complete_installation.sh"

pkgbuild --root "$BUILD_DIR/files_payload" \
    --identifier com.mayaflux.weave.files \
    --version "$VERSION" \
    "$BUILD_DIR/Weave-files.pkg"

# ============================================================================
# COMPONENT 2: Weave.app GUI
# ============================================================================

echo "Building Weave-gui.pkg..."

mkdir -p "$BUILD_DIR/gui_payload/Applications"
cp -R "$BUILD_DIR/WeaveApp/Weave.app" "$BUILD_DIR/gui_payload/Applications/Weave.app"

pkgbuild --root "$BUILD_DIR/gui_payload" \
    --identifier com.mayaflux.weave.gui \
    --version "$VERSION" \
    "$BUILD_DIR/Weave-gui.pkg"

# ============================================================================
# COMPONENT 3: Postinstall launcher (just launches complete_installation.sh)
# ============================================================================

echo "Building Weave-launcher.pkg..."

mkdir -p "$BUILD_DIR/launcher_scripts"

cat > "$BUILD_DIR/launcher_scripts/postinstall" << 'POSTINSTALL'
#!/usr/bin/env zsh
set -euo pipefail

# Get the actual user
get_actual_user() {
    if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; then
        echo "$SUDO_USER"
        return
    fi
    /usr/sbin/scutil <<< "show State:/Users/ConsoleUser" | awk '/Name :/ && ! /loginwindow/ { print $3 }'
}

ACTUAL_USER=$(get_actual_user)

# Launch the real installer in a new Terminal window as the actual user
sudo -u "$ACTUAL_USER" /usr/bin/open -a Terminal /Library/Weave/complete_installation.sh

exit 0
POSTINSTALL

chmod +x "$BUILD_DIR/launcher_scripts/postinstall"

pkgbuild --nopayload \
    --identifier com.mayaflux.weave.launcher \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/launcher_scripts" \
    "$BUILD_DIR/Weave-launcher.pkg"

# ============================================================================
# FINAL: Distribution package
# ============================================================================

echo "Creating distribution package..."

cat > "$BUILD_DIR/Distribution.xml" << 'DISTXML'
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>Weave - MayaFlux Installation</title>
    
    <welcome file="welcome.html"/>
    <conclusion file="conclusion.html"/>
    
    <options customize="never" require-scripts="true" hostArchitectures="arm64,x86_64"/>
    <domains enable_localSystem="true"/>
    
    <choices-outline>
        <line choice="weave.files"/>
        <line choice="weave.gui"/>
        <line choice="weave.launcher"/>
    </choices-outline>
    
    <choice id="weave.files" title="Weave Files" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.files"/>
    </choice>
    
    <choice id="weave.gui" title="Weave GUI Application" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.gui"/>
    </choice>
    
    <choice id="weave.launcher" title="Installation Launcher" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave.launcher"/>
    </choice>
    
    <pkg-ref id="com.mayaflux.weave.files" installKBytes="1000" version="1.0">Weave-files.pkg</pkg-ref>
    <pkg-ref id="com.mayaflux.weave.gui" installKBytes="10000" version="1.0">Weave-gui.pkg</pkg-ref>
    <pkg-ref id="com.mayaflux.weave.launcher" installKBytes="100" version="1.0">Weave-launcher.pkg</pkg-ref>
</installer-gui-script>
DISTXML

cp "$MACOS_DIR/resources/welcome.html" "$BUILD_DIR/welcome.html"
cp "$MACOS_DIR/resources/conclusion.html" "$BUILD_DIR/conclusion.html"

productbuild --distribution "$BUILD_DIR/Distribution.xml" \
    --resources "$BUILD_DIR" \
    --package-path "$BUILD_DIR" \
    "$BUILD_DIR/Weave-${VERSION}.pkg"

echo "✅ Installer created: $BUILD_DIR/Weave-${VERSION}.pkg"
echo ""
echo "Installation flow:"
echo "  1. Files copied to /Library/Weave"
echo "  2. Weave.app installed to /Applications"
echo "  3. Terminal opens with complete_installation.sh"
