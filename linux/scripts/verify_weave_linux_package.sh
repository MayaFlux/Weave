#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 1 && -f "$1" ]] || { echo "Usage: $0 <Weave-<version>-Linux-x86_64.AppImage>" >&2; exit 2; }
IMAGE="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/weave-appimage-check.XXXXXX")"
GUI_PID=""
cleanup() {
    if [[ -n "$GUI_PID" ]]; then kill "$GUI_PID" 2>/dev/null || true; wait "$GUI_PID" 2>/dev/null || true; fi
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT
fail() { echo "Linux AppImage validation: $*" >&2; exit 1; }

file "$IMAGE" | grep -q 'ELF 64-bit LSB.*x86-64' || fail "AppImage runtime is not x86_64 ELF"
cd "$TEMP_DIR"
"$IMAGE" --appimage-extract >/dev/null || fail "AppImage extraction failed"
APPDIR="$TEMP_DIR/squashfs-root"
[[ -x "$APPDIR/AppRun" && -x "$APPDIR/usr/bin/weave" ]] || fail "executable or launcher missing"
for resource in \
    Weave.desktop weave.png .DirIcon \
    usr/share/weave/LICENSE.txt \
    usr/share/weave/templates/CMakeLists.txt \
    usr/share/weave/templates/community/community.json; do
    [[ -e "$APPDIR/$resource" ]] || fail "missing packaged resource: $resource"
done

BINARY="$APPDIR/usr/bin/weave"
file "$BINARY" | grep -q 'ELF 64-bit LSB.*x86-64' || fail "Weave is not x86_64 ELF"
for elf in "$IMAGE" "$BINARY" "$APPDIR"/usr/lib/*.so*; do
    [[ -f "$elf" ]] || continue
    file "$elf" | grep -q ELF || continue
    while IFS= read -r version; do
        if [[ "$(printf '%s\n%s\n' 2.43 "$version" | sort -V | tail -n 1)" != 2.43 ]]; then
            fail "$(basename "$elf") needs GLIBC_$version; package baseline is GLIBC_2.43"
        fi
    done < <(readelf --version-info "$elf" | sed -n 's/.*Name: GLIBC_\([0-9][0-9.]*\).*/\1/p' | sort -Vu)
done
for forbidden in libc.so.6 ld-linux-x86-64.so.2 libGL.so.1 libEGL.so.1 libvulkan.so.1; do
    [[ ! -e "$APPDIR/usr/lib/$forbidden" ]] || fail "host library bundled: $forbidden"
done
LD_LIBRARY_PATH="$APPDIR/usr/lib" ldd "$BINARY" > "$TEMP_DIR/ldd.txt"
if grep -q 'not found' "$TEMP_DIR/ldd.txt"; then cat "$TEMP_DIR/ldd.txt" >&2; fail "unresolved library"; fi

"$APPDIR/AppRun" --help > "$TEMP_DIR/help.txt"
grep -q 'weave community' "$TEMP_DIR/help.txt" || fail "packaged CLI help failed"
mkdir "$TEMP_DIR/projects"
"$APPDIR/AppRun" new PackageSmoke "$TEMP_DIR/projects"
[[ -s "$TEMP_DIR/projects/PackageSmoke/CMakeLists.txt" ]] || fail "packaged project creation failed"
[[ -s "$TEMP_DIR/projects/PackageSmoke/src/main.cpp" ]] || fail "packaged templates were not used"
"$APPDIR/AppRun" community package_smoke "$TEMP_DIR/modules"
[[ -s "$TEMP_DIR/modules/package_smoke/community.json" ]] || fail "packaged community creation failed"

# Device presence alone does not prove FUSE works: libfuse can be missing, or
# the device can be inaccessible. Probe the actual runtime in either case.
# Extraction above is only a package inspection technique.
if "$IMAGE" --help > "$TEMP_DIR/image-launch.txt" 2>&1; then
    grep -q 'weave community' "$TEMP_DIR/image-launch.txt" || fail "actual AppImage CLI output was wrong"
else
    if ! grep -iq 'fuse' "$TEMP_DIR/image-launch.txt"; then
        cat "$TEMP_DIR/image-launch.txt" >&2
        fail "AppImage launch failed without a clear FUSE prerequisite error"
    fi
    echo "AppImage runtime reported that host FUSE is unavailable."
fi

if [[ "${WEAVE_GUI_SMOKE:-0}" == 1 ]]; then
    [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]] || fail "GUI smoke requested without a display"
    "$APPDIR/AppRun" > "$TEMP_DIR/gui.out" 2>&1 &
    GUI_PID=$!
    sleep 4
    if ! kill -0 "$GUI_PID" 2>/dev/null; then
        cat "$TEMP_DIR/gui.out" >&2
        fail "packaged GUI exited during startup"
    fi
    kill "$GUI_PID" 2>/dev/null || true
    wait "$GUI_PID" 2>/dev/null || true
    GUI_PID=""
fi

echo "Linux AppImage validation passed: x86_64, GLIBC <= 2.43, resources, CLI, and FUSE behavior."
