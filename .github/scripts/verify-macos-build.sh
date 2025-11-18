#!/bin/bash

echo "=== Checking Weave.app ==="
if [ ! -d "build/macos/WeaveApp/Weave.app" ]; then
    echo "Error: Weave.app not built"
    exit 1
fi
echo "OK: Weave.app exists"

echo ""
echo "=== Checking component packages ==="
for pkg in Weave-core Weave-gui; do
    if [ ! -f "build/macos/${pkg}.pkg" ]; then
        echo "Error: ${pkg}.pkg not found"
        exit 1
    fi
    echo "OK: ${pkg}.pkg exists"
done

echo ""
echo "=== Checking final package ==="
PKG=$(find build/macos -maxdepth 1 \( -name "Weave.pkg" -o -name "Weave-*.pkg" \) | head -1)
if [ -z "$PKG" ]; then
    echo "Error: Final package not found in build/macos"
    exit 1
fi
echo "OK: Final package created: $PKG"
ls -lh "$PKG"

echo ""
echo "=== Package contents ==="
pkgutil --payload-files "$PKG" | head -20