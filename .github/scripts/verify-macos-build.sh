#!/bin/bash

echo "=== Checking Weave.app ==="
if [ ! -d "build/macos/WeaveApp/Weave.app" ]; then
    echo "❌ Weave.app not built"
    exit 1
fi
echo "✅ Weave.app exists"

echo ""
echo "=== Checking bundle structure ==="
if [ ! -f "build/macos/WeaveApp/Weave.app/Contents/MacOS/Weave" ]; then
    echo "❌ Weave executable not found"
    exit 1
fi
echo "✅ Weave executable exists"

if [ ! -f "build/macos/WeaveApp/Weave.app/Contents/Info.plist" ]; then
    echo "❌ Info.plist not found"
    exit 1
fi
echo "✅ Info.plist exists"

if [ ! -f "build/macos/WeaveApp/Weave.app/Contents/Resources/weave" ]; then
    echo "❌ Weave CLI resource not found"
    exit 1
fi
echo "✅ Weave CLI resource exists"

echo ""
echo "=== Checking universal binary ==="
BINARY="build/macos/WeaveApp/Weave.app/Contents/MacOS/Weave"
ARCHITECTURES=$(lipo -info "$BINARY")
echo "Architectures: $ARCHITECTURES"

if [[ "$ARCHITECTURES" != *"x86_64"* ]]; then
    echo "❌ Missing x86_64 (Intel) architecture"
    exit 1
fi
if [[ "$ARCHITECTURES" != *"arm64"* ]]; then
    echo "❌ Missing arm64 (Apple Silicon) architecture"
    exit 1
fi
echo "✅ Universal binary contains both x86_64 and arm64"

echo ""
echo "=== Bundle size ==="
du -sh "build/macos/WeaveApp/Weave.app"
