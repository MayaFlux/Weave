#!/bin/bash
# File: .github/scripts/verify-linux-build.sh
# Verify Linux build artifacts

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
TARBALL=$(find "$BUILD_DIR" -name "Weave-*.tar.gz" -type f | head -1)

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

# Save all entries to a variable to avoid pipe issues
ENTRIES=$(tar -tzf "$TARBALL")
ENTRY_COUNT=$(echo "$ENTRIES" | wc -l)

# Show first 20 entries
echo "$ENTRIES" | head -20
echo "✅ Tarball contains $ENTRY_COUNT entries"

echo ""
echo "Required files in tarball:"

required_items=(
    "Weave-0.1.0/lib/weave/main.py"
    "Weave-0.1.0/templates/CMakeLists.txt"
    "Weave-0.1.0/templates/main.cpp"
    "Weave-0.1.0/scripts/create_project.sh"
    "Weave-0.1.0/Weave"
)

all_present=true
for item in "${required_items[@]}"; do
    if echo "$ENTRIES" | grep -q "^$item$"; then
        echo "✅ $item"
    else
        echo "❌ Missing: $item"
        all_present=false
    fi
done

echo ""

if [ "$all_present" = true ]; then
    echo "========================================"
    echo "  Linux Build Verified"
    echo "========================================"
    echo ""
    echo "✅ Ready for distribution: $TARBALL"
else
    echo "❌ Tarball is missing required items"
    exit 1
fi
