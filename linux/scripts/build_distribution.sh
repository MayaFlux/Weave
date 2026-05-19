#!/bin/bash

set -euo pipefail

DIST="build/linux/Weave"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LINUX_DIR="$REPO_ROOT/linux"

echo "Building Weave distribution..."
echo ""

rm -rf "$DIST"
mkdir -p "$DIST/lib/"

echo "Copying Python modules..."
cp -r "$LINUX_DIR/lib" "$DIST/"

if [ ! -f "$DIST/lib/config.py" ]; then
    echo "ERROR: config.py not found in $LINUX_DIR/lib/" >&2
    exit 1
fi

echo "Copying scripts and templates..."
mkdir -p "$DIST/lib/scripts"
cp "$LINUX_DIR/scripts"/create_project.sh "$DIST/lib/scripts/"
chmod +x "$DIST/lib/scripts/"*

cp -r "$REPO_ROOT/templates" "$DIST/lib/templates"

echo "Copying configuration files..."
cp "$LINUX_DIR/weave-config.json" "$DIST/weave-config.json"
cp "$LINUX_DIR/Weave" "$DIST/Weave"
chmod +x "$DIST/Weave"

echo "✓ Configuration file created"
echo "✓ Launcher script created"

echo ""
echo "Verifying distribution structure..."
test -f "$DIST/weave-config.json" && echo "  ✓ weave-config.json"
test -f "$DIST/Weave" && echo "  ✓ Weave launcher"
test -f "$DIST/lib/config.py" && echo "  ✓ config.py"
test -f "$DIST/lib/main.py" && echo "  ✓ main.py"
test -f "$DIST/lib/cli.py" && echo "  ✓ cli.py"
test -d "$DIST/lib/scripts" && echo "  ✓ scripts directory"
test -d "$DIST/lib/templates" && echo "  ✓ templates directory"

echo ""
echo "Creating distribution tarball..."
tar czf "$DIST.tar.gz" -C build/linux "Weave"

TARBALL_SIZE=$(stat -f%z "$DIST.tar.gz" 2>/dev/null || stat -c%s "$DIST.tar.gz")
TARBALL_SIZE_MB=$((TARBALL_SIZE / 1024 / 1024))

echo "✓ Tarball created: $DIST.tar.gz (${TARBALL_SIZE_MB} MB)"

echo ""
echo "Generating hash..."
sha256sum "$DIST.tar.gz"

echo ""
echo "=========================================="
echo "  Weave Distribution Built!"
echo "=========================================="
echo ""
echo "Distribution: $DIST.tar.gz"
echo "Size: ${TARBALL_SIZE_MB} MB"
echo ""
echo "Installation:"
echo "  tar -xzf $DIST.tar.gz -C ~/.local/"
echo "  ~/.local/Weave/Weave              # GUI or CLI"
