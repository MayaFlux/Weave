#!/bin/bash
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" ]]; then
    echo "Usage: $0 <Weave-<version>-macOS-arm64.dmg>" >&2
    exit 2
fi
DMG_PATH="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/weave-package-check.XXXXXX")"
MOUNT_POINT="$TEMP_DIR/mount"
mkdir -p "$MOUNT_POINT"
MOUNTED=0
GUI_OPEN_PID=""
cleanup() {
    if [[ -n "$GUI_OPEN_PID" ]] && kill -0 "$GUI_OPEN_PID" 2>/dev/null; then
        kill "$GUI_OPEN_PID" 2>/dev/null || true
        wait "$GUI_OPEN_PID" 2>/dev/null || true
    fi
    if [[ "$MOUNTED" -eq 1 ]]; then
        hdiutil detach "$MOUNT_POINT" -quiet \
            || hdiutil detach -force "$MOUNT_POINT" -quiet
        MOUNTED=0
    fi
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

hdiutil verify "$DMG_PATH"
hdiutil attach "$DMG_PATH" -readonly -nobrowse -mountpoint "$MOUNT_POINT" -quiet
MOUNTED=1
APP_BUNDLE="$MOUNT_POINT/Weave.app"
EXECUTABLE="$APP_BUNDLE/Contents/MacOS/weave"
CLI="$APP_BUNDLE/Contents/Resources/weave"
PLIST="$APP_BUNDLE/Contents/Info.plist"

[[ -d "$APP_BUNDLE" && -x "$EXECUTABLE" && -x "$CLI" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST")" == "com.mayaflux.weave" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$PLIST")" == "26.0" ]]
[[ "$(lipo -archs "$EXECUTABLE")" == "arm64" ]]
MIN_OS="$(otool -l "$EXECUTABLE" | awk '/cmd LC_BUILD_VERSION/ {build=1; next} build && /minos / {print $2; exit}')"
[[ "$MIN_OS" == "26.0" ]]
[[ -s "$APP_BUNDLE/Contents/Resources/templates/CMakeLists.txt" ]]
[[ -s "$APP_BUNDLE/Contents/Resources/templates/community/community.json" ]]
[[ -s "$APP_BUNDLE/Contents/Resources/LICENSE.txt" ]]

if codesign -dv "$APP_BUNDLE" >/dev/null 2>&1; then
    codesign --verify --deep --strict "$APP_BUNDLE"
else
    echo "App is unsigned; signature verification skipped."
fi

# Exercise the packaged CLI from an unrelated working directory. Templates are
# embedded in the executable and the bundle also carries the resource snapshot.
cd "$TEMP_DIR"
"$CLI" --help > "$TEMP_DIR/help.txt"
grep -q 'weave community' "$TEMP_DIR/help.txt"
"$CLI" community package_smoke "$TEMP_DIR/modules"
[[ -s "$TEMP_DIR/modules/package_smoke/community.json" ]]
[[ -s "$TEMP_DIR/modules/package_smoke/CMakeLists.txt" ]]
mkdir -p "$TEMP_DIR/projects"
"$CLI" new PackageSmoke "$TEMP_DIR/projects"
[[ -s "$TEMP_DIR/projects/PackageSmoke/CMakeLists.txt" ]]

# Ask Launch Services to launch the actual GUI app from the mounted DMG.
open -n -W "$APP_BUNDLE" &
GUI_OPEN_PID=$!
sleep 3
osascript -e 'tell application id "com.mayaflux.weave" to quit' >/dev/null
wait "$GUI_OPEN_PID"
GUI_OPEN_PID=""

echo "macOS DMG package validation passed: arm64, minimum macOS $MIN_OS, CLI/templates, and GUI launch."
