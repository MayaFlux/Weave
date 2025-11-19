#!/usr/bin/env zsh
# This should be placed in: macos/scripts/build_weave_app.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$MACOS_DIR")"
BUILD_DIR="$ROOT_DIR/build/macos/WeaveApp"

echo "Building Weave.app as Universal Binary (x86_64 + arm64)..."

# Clean
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Create temporary project structure
TEMP_PROJECT="$BUILD_DIR/Weave"
mkdir -p "$TEMP_PROJECT"

# Copy Swift source
cp "$MACOS_DIR/WeaveGUI.swift" "$TEMP_PROJECT/"

# Create Info.plist - UPDATED TO macOS 12.0
cat >"$TEMP_PROJECT/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>Weave</string>
    <key>CFBundleIdentifier</key>
    <string>com.mayaflux.weave</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Weave</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>LSUIElement</key>
    <false/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024 MayaFlux. All rights reserved.</string>
    <key>NSMainStoryboardFile</key>
    <string></string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF

# Build the app as universal binary
echo "Compiling Swift code as Universal Binary..."
cd "$TEMP_PROJECT"

# Get SDK path
SDK_PATH=$(xcrun --show-sdk-path)

# Build for arm64 (Apple Silicon) - UPDATED TO macOS 12.0
echo "Building for arm64 (Apple Silicon)..."
swiftc -o Weave-arm64 \
    -parse-as-library \
    -target arm64-apple-macos12.0 \
    -sdk "$SDK_PATH" \
    -framework SwiftUI \
    -framework AppKit \
    WeaveGUI.swift

# Build for x86_64 (Intel) - UPDATED TO macOS 12.0
echo "Building for x86_64 (Intel)..."
swiftc -o Weave-x86_64 \
    -parse-as-library \
    -target x86_64-apple-macos12.0 \
    -sdk "$SDK_PATH" \
    -framework SwiftUI \
    -framework AppKit \
    WeaveGUI.swift

# Create universal binary using lipo
echo "Creating universal binary..."
lipo -create -output Weave Weave-arm64 Weave-x86_64

# Verify architectures
echo "Verifying universal binary..."
file Weave
lipo -info Weave

# Clean up architecture-specific binaries
rm -f Weave-arm64 Weave-x86_64

# Create .app bundle structure
APP_BUNDLE="$BUILD_DIR/Weave.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy universal executable and Info.plist
cp Weave "$APP_BUNDLE/Contents/MacOS/"
cp Info.plist "$APP_BUNDLE/Contents/"

# Code sign (ad-hoc for now)
echo "Code signing universal binary..."
codesign --force --deep --sign - "$APP_BUNDLE"

echo "✅ Universal Weave.app built successfully"
echo "Location: $APP_BUNDLE"

# Final verification
echo "Final verification:"
echo "Architectures in binary:"
lipo -info "$APP_BUNDLE/Contents/MacOS/Weave"

if [ -d "$APP_BUNDLE" ]; then
    echo "✅ Universal app bundle created successfully"
    echo "You can test it with: open $APP_BUNDLE"
else
    echo "❌ Failed to create app bundle"
    exit 1
fi
