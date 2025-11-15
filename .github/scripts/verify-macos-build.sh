#!/bin/bash
VERSION=$1

echo "=== Checking Weave.app ==="
if [ ! -d "build/macos/WeaveApp/Weave.app" ]; then
    echo "❌ Weave.app not built"
    exit 1
fi
echo "✅ Weave.app exists"

echo ""
echo "=== Checking component packages ==="
for pkg in Weave-core Weave-gui; do
    if [ ! -f "build/macos/${pkg}.pkg" ]; then
        echo "❌ ${pkg}.pkg not found"
        exit 1
    fi
    echo "✅ ${pkg}.pkg exists"
done

echo ""
echo "=== Checking final package ==="
FINAL_PKG="build/macos/Weave-${VERSION}.pkg"
if [ ! -f "$FINAL_PKG" ]; then
    echo "❌ Final package not found at $FINAL_PKG"
    exit 1
fi
echo "✅ Final package created: $FINAL_PKG"
ls -lh "$FINAL_PKG"

echo ""
echo "=== Package contents ==="
pkgutil --payload-files "$FINAL_PKG" | head -20
