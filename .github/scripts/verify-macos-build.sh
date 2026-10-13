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
echo "=== Checking binary architecture ==="
BINARY="build/macos/WeaveApp/Weave.app/Contents/MacOS/Weave"
ARCHITECTURES=$(lipo -archs "$BINARY")
echo "Architectures: $ARCHITECTURES"

if [[ "$ARCHITECTURES" != "arm64" ]]; then
    echo "❌ Unexpected binary architecture: $ARCHITECTURES"
    exit 1
fi
echo "✅ Binary architecture verified"

MINIMUM_VERSION=$(/usr/libexec/PlistBuddy -c "Print :LSMinimumSystemVersion" "build/macos/WeaveApp/Weave.app/Contents/Info.plist")
if [[ "$MINIMUM_VERSION" != "26.0" ]]; then
    echo "❌ Expected macOS minimum version 26.0, found $MINIMUM_VERSION"
    exit 1
fi
echo "✅ Minimum macOS version is 26.0"

echo ""
echo "=== Bundle size ==="
du -sh "build/macos/WeaveApp/Weave.app"
