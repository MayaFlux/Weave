#!/bin/bash

set -euo pipefail

echo ""
echo "========================================"
echo "  Verifying Linux Build"
echo "========================================"
echo ""

BUILD_DIR="build/linux"

if [ ! -d "$BUILD_DIR" ]; then
    echo "❌ Build directory not found: $BUILD_DIR"
    exit 1
fi

echo "Checking tarball..."
TARBALL=$(find "$BUILD_DIR" -name "Weave.tar.gz" -type f | head -1)

if [ -z "$TARBALL" ]; then
    echo "❌ No tarball found in $BUILD_DIR"
    ls -la "$BUILD_DIR"
    exit 1
fi

echo "✅ Tarball found: $TARBALL"
TARBALL_SIZE=$(stat -f%z "$TARBALL" 2>/dev/null || stat -c%s "$TARBALL")
TARBALL_SIZE_MB=$((TARBALL_SIZE / 1024 / 1024))
echo "   Size: ${TARBALL_SIZE_MB} MB"

echo ""
echo "Verifying tarball contents..."

ENTRIES=$(tar -tzf "$TARBALL")
ENTRY_COUNT=$(echo "$ENTRIES" | wc -l)

echo "$ENTRIES" | head -20
echo "✅ Tarball contains $ENTRY_COUNT entries"

echo ""
echo "Required files in tarball:"

required_items=(
    "Weave/weave-config.json"
    "Weave/Weave"
    "Weave/lib/config.py"
    "Weave/lib/main.py"
    "Weave/lib/cli.py"
    "Weave/lib/modes"
    "Weave/lib/ui"
    "Weave/lib/templates/CMakeLists.txt"
    "Weave/lib/templates/shaders.cmake"
    "Weave/lib/templates/main.cpp"
    "Weave/lib/templates/user_project.hpp"
    "Weave/lib/templates/shaders"
    "Weave/lib/scripts/create_project.sh"
    "Weave/lib/scripts/install_deps.sh"
)

all_present=true
for item in "${required_items[@]}"; do
    if echo "$ENTRIES" | grep -q "^$item"; then
        echo "✅ $item"
    else
        echo "❌ Missing: $item"
        all_present=false
    fi
done

echo ""

if [ "$all_present" = true ]; then
    echo "========================================"
    echo "  Linux Build Verified ✅"
    echo "========================================"
    echo ""
    echo "✅ Ready for distribution: $TARBALL"
    exit 0
else
    echo "❌ Tarball is missing required items"
    exit 1
fi
