# create_pkg.sh - simplified to only install files

#!/usr/bin/env zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$MACOS_DIR")"
BUILD_DIR="$ROOT_DIR/build/macos"
VERSION="${1:-0.1.0}"

echo "Building Weave installer v$VERSION (file-only approach)..."

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# ============================================================================
# PAYLOAD: Just files, no scripts
# ============================================================================

mkdir -p "$BUILD_DIR/payload/Library/Weave"

# Copy CLI tool
cp "$MACOS_DIR/Weave.sh" "$BUILD_DIR/payload/Library/Weave/project_creator.sh"
chmod +x "$BUILD_DIR/payload/Library/Weave/project_creator.sh"

# Copy templates
cp -R "$ROOT_DIR/templates" "$BUILD_DIR/payload/Library/Weave/templates"

# Copy the REAL installer script (runs after .pkg completes)
cp "$MACOS_DIR/scripts/complete_installation.sh" "$BUILD_DIR/payload/Library/Weave/complete_installation.sh"
chmod +x "$BUILD_DIR/payload/Library/Weave/complete_installation.sh"

# ============================================================================
# POSTINSTALL: Just launch the real installer
# ============================================================================

mkdir -p "$BUILD_DIR/scripts"

cat > "$BUILD_DIR/scripts/postinstall" << 'POSTINSTALL'
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
# This runs AFTER the .pkg completes, in a visible terminal
sudo -u "$ACTUAL_USER" /usr/bin/open -a Terminal /Library/Weave/complete_installation.sh

exit 0
POSTINSTALL

chmod +x "$BUILD_DIR/scripts/postinstall"

# ============================================================================
# BUILD PACKAGE
# ============================================================================

pkgbuild --root "$BUILD_DIR/payload" \
    --identifier com.mayaflux.weave \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/scripts" \
    "$BUILD_DIR/Weave-core.pkg"

# Create distribution package with nice UI
cat > "$BUILD_DIR/Distribution.xml" << 'DISTXML'
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>Weave - MayaFlux Installation</title>
    
    <welcome file="welcome.html"/>
    <conclusion file="conclusion.html"/>
    
    <options customize="never" require-scripts="true" hostArchitectures="arm64,x86_64"/>
    <domains enable_localSystem="true"/>
    
    <choices-outline>
        <line choice="weave.install"/>
    </choices-outline>
    
    <choice id="weave.install" title="Weave Installation" start_enabled="true">
        <pkg-ref id="com.mayaflux.weave"/>
    </choice>
    
    <pkg-ref id="com.mayaflux.weave">Weave-core.pkg</pkg-ref>
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
echo "  1. User runs .pkg (installs files only, ~5 seconds)"
echo "  2. Terminal window opens automatically"
echo "  3. complete_installation.sh runs with visible progress"
echo "  4. User sees real-time output of downloads/installations"
