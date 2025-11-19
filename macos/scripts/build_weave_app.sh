#!/usr/bin/env zsh
# Build Weave.app (GUI for project creation)
# This should be placed in: macos/scripts/build_weave_app.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$MACOS_DIR")"
BUILD_DIR="$ROOT_DIR/build/macos/WeaveApp"

echo "Building Weave.app..."

# Clean
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Create temporary project structure
TEMP_PROJECT="$BUILD_DIR/Weave"
mkdir -p "$TEMP_PROJECT"

# Copy Swift source
cp "$MACOS_DIR/WeaveGUI.swift" "$TEMP_PROJECT/"

# Create Info.plist
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
    <string>14.0</string>
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

# Build the app using swiftc
echo "Compiling Swift code..."
cd "$TEMP_PROJECT"

swiftc -o Weave \
    -parse-as-library \
    -target arm64-apple-macos14.0 \
    -sdk "$(xcrun --show-sdk-path)" \
    -framework SwiftUI \
    -framework AppKit \
    WeaveGUI.swift

# Create .app bundle structure
APP_BUNDLE="$BUILD_DIR/Weave.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy executable and Info.plist
cp Weave "$APP_BUNDLE/Contents/MacOS/"
cp Info.plist "$APP_BUNDLE/Contents/"

# Code sign (ad-hoc for now)
codesign --force --deep --sign - "$APP_BUNDLE"

echo "✅ Weave.app built successfully"
echo "Location: $APP_BUNDLE"

# Verify
if [ -d "$APP_BUNDLE" ]; then
    echo "✅ App bundle created"
    echo "You can test it with: open $APP_BUNDLE"
else
    echo "❌ Failed to create app bundle"
    exit 1
fi
