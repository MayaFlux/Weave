#!/usr/bin/env zsh
# Build Weave installer package with integrated GUI project creator

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$MACOS_DIR")"
BUILD_DIR="$ROOT_DIR/build/macos"
VERSION="${1:-0.1.0}"

echo "Building Weave installer v$VERSION..."

# ============================================================================
# STEP 1: Build Weave.app (GUI)
# ============================================================================

echo "Step 1: Building Weave.app..."
"$MACOS_DIR/scripts/build_weave_app.sh"

# ============================================================================
# STEP 2: Prepare Weave-core package payload
# ============================================================================

echo "Step 2: Preparing Weave-core payload..."

# Clean build dir (preserve WeaveApp from step 1)
mkdir -p "$BUILD_DIR/payload/Library/Weave" "$BUILD_DIR/scripts"

# Copy scripts
cp "$MACOS_DIR/scripts/postinstall" "$BUILD_DIR/scripts/"
chmod +x "$BUILD_DIR/scripts/postinstall"

# Copy CLI tool (Weave.sh)
cp "$MACOS_DIR/Weave.sh" "$BUILD_DIR/payload/Library/Weave/project_creator.sh"
chmod +x "$BUILD_DIR/payload/Library/Weave/project_creator.sh"

# Copy templates directory
cp -R "$ROOT_DIR/templates" "$BUILD_DIR/payload/Library/Weave/templates"

echo "Weave-core payload contents:"
find "$BUILD_DIR/payload" -type f

# Build Weave-core component package
pkgbuild --root "$BUILD_DIR/payload" \
    --identifier com.mayaflux.weave.core \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/scripts" \
    "$BUILD_DIR/Weave-core.pkg"

# ============================================================================
# STEP 3: Prepare Weave.app package
# ============================================================================

echo "Step 3: Packaging Weave.app..."

mkdir -p "$BUILD_DIR/app_payload/Applications"
cp -R "$BUILD_DIR/WeaveApp/Weave.app" \
    "$BUILD_DIR/app_payload/Applications/Weave.app"

# Build Weave GUI component package
pkgbuild --root "$BUILD_DIR/app_payload" \
    --identifier com.mayaflux.weave.gui \
    --version "$VERSION" \
    "$BUILD_DIR/Weave-gui.pkg"

# ============================================================================
# STEP 4: Build distribution package (combines both)
# ============================================================================

echo "Step 4: Building distribution package..."

productbuild --distribution "$MACOS_DIR/resources/Distribution.xml" \
    --resources "$MACOS_DIR/resources" \
    --package-path "$BUILD_DIR" \
    "$BUILD_DIR/Weave-${VERSION}.pkg"

echo "✅ Installer created: $BUILD_DIR/Weave-${VERSION}.pkg"
echo ""
echo "Components included:"
echo "  1. Weave-core.pkg - MayaFlux installer, dependencies, CLI tool"
echo "  2. Weave-gui.pkg - GUI application for project creation"
echo ""
echo "Test installation (user-level):"
echo "  sudo installer -pkg $BUILD_DIR/Weave-${VERSION}.pkg -target CurrentUserHomeDirectory"
echo ""
echo "Test installation (system-wide):"
echo "  sudo installer -pkg $BUILD_DIR/Weave-${VERSION}.pkg -target /"
echo ""
echo "After installation, Weave.app will launch automatically."
