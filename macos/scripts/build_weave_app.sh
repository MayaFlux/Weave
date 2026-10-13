#!/usr/bin/env zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MACOS_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$MACOS_DIR")"
BUILD_DIR="$ROOT_DIR/build/macos/WeaveApp"

echo "Building Weave.app..."

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

TEMP_PROJECT="$BUILD_DIR/Weave"
mkdir -p "$TEMP_PROJECT"

cp "$MACOS_DIR/WeaveGUI.swift" "$TEMP_PROJECT/"
cp "$MACOS_DIR/CommunityManifest.swift" "$TEMP_PROJECT/"
cp "$MACOS_DIR/CommunityUpdate.swift" "$TEMP_PROJECT/"
cp "$MACOS_DIR/InstallerWorkflow.swift" "$TEMP_PROJECT/"
cp "$MACOS_DIR/ShellEnvironment.swift" "$TEMP_PROJECT/"
cp "$MACOS_DIR/CreationOperations.swift" "$TEMP_PROJECT/"

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
    <string>26.0</string>
    <key>LSUIElement</key>
    <false/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024 MayaFlux. All rights reserved.</string>
    <key>NSMainStoryboardFile</key>
    <string></string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>CFBundleIconFile</key>
    <string>weave</string>
</dict>
</plist>
EOF

echo "Compiling Swift code..."
cd "$TEMP_PROJECT"

SDK_PATH=$(xcrun --show-sdk-path)

swiftc -o Weave \
    -parse-as-library \
    -target arm64-apple-macos26.0 \
    -sdk "$SDK_PATH" \
    -framework SwiftUI \
    -framework AppKit \
    WeaveGUI.swift CommunityManifest.swift CommunityUpdate.swift \
    InstallerWorkflow.swift ShellEnvironment.swift CreationOperations.swift

APP_BUNDLE="$BUILD_DIR/Weave.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp Weave "$APP_BUNDLE/Contents/MacOS/"
cp Info.plist "$APP_BUNDLE/Contents/"
cp "$MACOS_DIR/resources/weave.icns" "$APP_BUNDLE/Contents/Resources/"

echo "Copying Weave CLI into bundle resources..."
cp "$MACOS_DIR/Weave.sh" "$APP_BUNDLE/Contents/Resources/weave"
chmod +x "$APP_BUNDLE/Contents/Resources/weave"

echo "Copying templates into bundle resources..."
cp -R "$ROOT_DIR/templates" "$APP_BUNDLE/Contents/Resources/templates"

echo "Code signing app bundle..."
codesign --force --deep --sign - "$APP_BUNDLE"

echo "✅ Weave.app built successfully"
echo "Location: $APP_BUNDLE"

if [ -d "$APP_BUNDLE" ]; then
    echo "✅ App bundle created successfully"
else
    echo "❌ Failed to create app bundle"
    exit 1
fi
