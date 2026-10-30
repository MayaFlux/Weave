#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../../.." && pwd)"
BUILD_DIR="$ROOT_DIR/target/packaging/macos"
APP_BUNDLE="$BUILD_DIR/Weave.app"
STAGE_DIR="$BUILD_DIR/dmg-root"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"
OUTPUT_DIR="${1:-$BUILD_DIR}"
DMG_PATH="$OUTPUT_DIR/Weave-${VERSION}-macOS-arm64.dmg"
TARGET="aarch64-apple-darwin"

if [[ "$(uname -m)" != "arm64" ]]; then
    echo "macOS packaging must run on Apple Silicon (arm64)." >&2
    exit 1
fi
if ! rustup target list --installed | grep -Fxq "$TARGET"; then
    echo "Rust target $TARGET is missing. Install it with: rustup target add $TARGET" >&2
    exit 1
fi
if [[ -z "$VERSION" ]]; then
    echo "Could not read the Weave version from Cargo.toml." >&2
    exit 1
fi

mkdir -p "$BUILD_DIR" "$OUTPUT_DIR"
rm -rf "$APP_BUNDLE" "$STAGE_DIR"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources" "$STAGE_DIR"

echo "Building Weave $VERSION for $TARGET with macOS minimum 26.0..."
cd "$ROOT_DIR"
MACOSX_DEPLOYMENT_TARGET=26.0 cargo build --release --locked --target "$TARGET"

BINARY="$ROOT_DIR/target/$TARGET/release/weave"
if [[ ! -x "$BINARY" ]]; then
    echo "Expected executable was not produced: $BINARY" >&2
    exit 1
fi
cp "$BINARY" "$APP_BUNDLE/Contents/MacOS/weave"
cp "$ROOT_DIR/packaging/macos/assets/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$APP_BUNDLE/Contents/Info.plist"
printf 'APPL????' > "$APP_BUNDLE/Contents/PkgInfo"

# The same self-contained executable provides the GUI and the standalone CLI.
ln -s ../MacOS/weave "$APP_BUNDLE/Contents/Resources/weave"
bash "$ROOT_DIR/packaging/scripts/stage-resources.sh" "$APP_BUNDLE/Contents/Resources"
cp "$ROOT_DIR/LICENSE.txt" "$APP_BUNDLE/Contents/Resources/LICENSE.txt"
cp "$ROOT_DIR/resources/icons/weave.icns" "$APP_BUNDLE/Contents/Resources/weave.icns"

if [[ -z "${WEAVE_MACOS_SIGNING_IDENTITY:-}" || "$WEAVE_MACOS_SIGNING_IDENTITY" == "-" ]]; then
    # Ad-hoc signing seals the completed bundle for integrity checks; it is not
    # Developer ID signing and does not satisfy Gatekeeper for downloaded apps.
    codesign --force --deep --sign - "$APP_BUNDLE"
else
    codesign --force --deep --options runtime --timestamp \
        --sign "$WEAVE_MACOS_SIGNING_IDENTITY" "$APP_BUNDLE"
fi

cp -R "$APP_BUNDLE" "$STAGE_DIR/Weave.app"
ln -s /Applications "$STAGE_DIR/Applications"
hdiutil create -volname "Weave $VERSION" -srcfolder "$STAGE_DIR" \
    -ov -format UDZO "$DMG_PATH"

echo "Created: $DMG_PATH"
