#!/bin/bash

set -euo pipefail

echo ""
echo "========================================"
echo "  Verifying Linux Build"
echo "========================================"
echo ""

BUILD_DIR="build/linux"

if [ ! -d "$BUILD_DIR" ]; then
    echo "ERROR: Build directory not found: $BUILD_DIR"
    exit 1
fi

APPIMAGE=$(find "$BUILD_DIR" -name "Weave-*.AppImage" -type f | head -1)
if [ -z "$APPIMAGE" ]; then
    echo "ERROR: No AppImage found in $BUILD_DIR"
    ls -la "$BUILD_DIR"
    exit 1
fi

echo "OK AppImage: $APPIMAGE"
APPIMAGE_SIZE=$(stat -c%s "$APPIMAGE")
APPIMAGE_SIZE_MB=$((APPIMAGE_SIZE / 1024 / 1024))
echo "   Size: ${APPIMAGE_SIZE_MB} MB"

if [ "$APPIMAGE_SIZE_MB" -lt 50 ]; then
    echo "ERROR: AppImage is suspiciously small (${APPIMAGE_SIZE_MB} MB)"
    exit 1
fi

echo ""
echo "========================================"
echo "  Linux Build Verified"
echo "========================================"
echo ""
echo "Ready for distribution: $APPIMAGE"
