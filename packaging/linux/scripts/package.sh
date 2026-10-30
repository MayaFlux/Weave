#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../../.." && pwd)"
BUILD_DIR="$ROOT_DIR/target/packaging/linux"
APPDIR="$BUILD_DIR/Weave.AppDir"
TOOLS_DIR="$BUILD_DIR/tools"
OUTPUT_DIR="${1:-$BUILD_DIR}"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)"
IMAGE="$OUTPUT_DIR/Weave-${VERSION}-Linux-x86_64.AppImage"
BASELINE=2.43

fail() { echo "Linux AppImage packaging: $*" >&2; exit 1; }
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || fail "requires x86_64 Linux"
[[ -n "$VERSION" ]] || fail "could not read version from Cargo.toml"
for command in cargo curl file readelf ldd; do
    command -v "$command" >/dev/null || fail "missing build tool: $command"
done

# Build on Ubuntu 26.04 (or another system with glibc <= 2.43). The build
# machine's libc is encoded into the Rust executable and cannot be repaired
# by copying a newer executable into an older AppDir.
cd "$ROOT_DIR"
cargo build --release --locked
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
BINARY="$TARGET_DIR/release/weave"
[[ -x "$BINARY" ]] || fail "Cargo did not produce $BINARY"
file "$BINARY" | grep -q 'ELF 64-bit LSB.*x86-64' || fail "Cargo output is not x86_64 ELF"

check_abi() {
    local binary="$1" version
    while IFS= read -r version; do
        if [[ "$(printf '%s\n%s\n' "$BASELINE" "$version" | sort -V | tail -n 1)" != "$BASELINE" ]]; then
            fail "$binary requires GLIBC_$version (maximum supported is GLIBC_$BASELINE); build on Ubuntu 26.04"
        fi
    done < <(readelf --version-info "$binary" | sed -n 's/.*Name: GLIBC_\([0-9][0-9.]*\).*/\1/p' | sort -Vu)
}
check_abi "$BINARY"

mkdir -p "$TOOLS_DIR" "$OUTPUT_DIR"
download_tool() {
    local url="$1" path="$2"
    if [[ ! -x "$path" ]]; then
        curl -fsSL "$url" -o "$path.tmp"
        mv "$path.tmp" "$path"
        chmod +x "$path"
    fi
}
download_tool \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
    "$TOOLS_DIR/linuxdeploy-x86_64.AppImage"
download_tool \
    https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage \
    "$TOOLS_DIR/appimagetool-x86_64.AppImage"

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/weave" "$APPDIR/usr/share/icons/hicolor/512x512/apps"
cp "$BINARY" "$APPDIR/usr/bin/weave"
bash "$ROOT_DIR/packaging/scripts/stage-resources.sh" "$APPDIR/usr/share/weave"
cp "$ROOT_DIR/LICENSE.txt" "$APPDIR/usr/share/weave/LICENSE.txt"
cp "$ROOT_DIR/resources/icons/weave.png" "$APPDIR/weave.png"
cp "$ROOT_DIR/resources/icons/weave.png" "$APPDIR/usr/share/icons/hicolor/512x512/apps/weave.png"
cp "$ROOT_DIR/packaging/linux/assets/Weave.desktop" "$APPDIR/Weave.desktop"
cp "$ROOT_DIR/packaging/linux/assets/AppRun" "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"
ln -s weave.png "$APPDIR/.DirIcon"

# linuxdeploy finds the ELF dependency closure and keeps its normal exclusion
# list for operating-system and graphics-driver libraries.
APPIMAGE_EXTRACT_AND_RUN=1 \
    "$TOOLS_DIR/linuxdeploy-x86_64.AppImage" \
    --appdir "$APPDIR" --executable "$BINARY" \
    --desktop-file "$ROOT_DIR/packaging/linux/assets/Weave.desktop" \
    --icon-file "$ROOT_DIR/resources/icons/weave.png"

# Preserve the launcher: linuxdeploy may replace AppRun with its own launcher.
cp "$ROOT_DIR/packaging/linux/assets/AppRun" "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"

# linuxdeploy intentionally blacklists several common libraries. Weave ships
# its non-glibc ELF dependencies so users do not have to install fontconfig,
# freetype, libpng, or similar runtime packages separately. The kernel,
# libc, display server, and graphics driver remain host provided.
LINKED_LIBRARIES="$(ldd "$BINARY")"
if grep -q 'not found' <<< "$LINKED_LIBRARIES"; then
    fail "build executable has unresolved shared libraries"
fi
while IFS= read -r dependency; do
    name="$(basename "$dependency")"
    case "$name" in
        libc.so.*|libm.so.*|ld-linux*.so.*|libpthread.so.*|libdl.so.*|libresolv.so.*|libnss_*.so.*|libGL*.so.*|libEGL*.so.*|libvulkan.so.*) continue ;;
    esac
    cp -L "$dependency" "$APPDIR/usr/lib/$name"
    if command -v dpkg-query >/dev/null; then
        package="$(dpkg-query -S "$(readlink -f "$dependency")" 2>/dev/null | head -n 1 | cut -d: -f1 || true)"
        if [[ -n "$package" && -f "/usr/share/doc/$package/copyright" ]]; then
            mkdir -p "$APPDIR/usr/share/weave/third-party-licenses"
            cp "/usr/share/doc/$package/copyright" \
                "$APPDIR/usr/share/weave/third-party-licenses/$package.txt"
        fi
    fi
done < <(sed -n 's/.*=> \(\/[^ ]*\).*/\1/p' <<< "$LINKED_LIBRARIES")

for elf in "$APPDIR/usr/bin/weave" "$APPDIR"/usr/lib/*.so*; do
    [[ -f "$elf" ]] || continue
    if file "$elf" | grep -q ELF; then check_abi "$elf"; fi
done
for forbidden in libc.so.6 ld-linux-x86-64.so.2 libGL.so.1 libEGL.so.1 libvulkan.so.1; do
    [[ ! -e "$APPDIR/usr/lib/$forbidden" ]] || fail "host system library was bundled: $forbidden"
done

rm -f "$IMAGE"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 \
    "$TOOLS_DIR/appimagetool-x86_64.AppImage" "$APPDIR" "$IMAGE"
[[ -s "$IMAGE" ]] || fail "appimagetool did not create $IMAGE"
check_abi "$IMAGE"
echo "Created $IMAGE (x86_64, GLIBC <= $BASELINE)"
