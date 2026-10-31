#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 1 && -f "$1" ]] || { echo "Usage: $0 <Weave-<version>-Linux-x86_64.AppImage>" >&2; exit 2; }
IMAGE="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
# A caller's extraction fallback must not turn the mounted launch probe into
# a false success when the host cannot mount AppImages.
unset APPIMAGE_EXTRACT_AND_RUN APPIMAGE_EXTRACT
TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/weave-appimage-check.XXXXXX")"
GUI_PID=""
MOUNTED_CLI=0
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

run_package_case() {
    local launcher="$1" case_name="$2" case_dir="$TEMP_DIR/$2"
    local shims="$case_dir/shims" module_fixture="$case_dir/module-fixture"
    local mayaflux_root="$case_dir/mayaflux-root" project="$case_dir/projects/PackageSmoke"
    mkdir -p "$case_dir/projects" "$shims" "$module_fixture/src" \
        "$mayaflux_root/lib/cmake/MayaFlux"
    "$launcher" --help > "$case_dir/help.txt"
    grep -q 'weave community' "$case_dir/help.txt" || fail "$case_name CLI help failed"
    "$launcher" new PackageSmoke "$case_dir/projects"
    [[ -s "$project/CMakeLists.txt" && -s "$project/src/main.cpp" ]] \
        || fail "$case_name project creation or templates failed"
    "$launcher" community package_smoke "$case_dir/modules"
    [[ -s "$case_dir/modules/package_smoke/community.json" ]] \
        || fail "$case_name community creation failed"

    # Fake host tools and an installed-version fixture keep this module addition offline.
    printf 'set(PACKAGE_VERSION "0.5.0")\n' \
        > "$mayaflux_root/lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake"
    printf 'set(MF_MIN_VERSION "0.5.0")\n' > "$module_fixture/package_smoke.cmake"
    printf '{"name":"package_smoke","min_version":"0.5.0"}\n' \
        > "$module_fixture/community.json"
    printf 'int placeholder;\n' > "$module_fixture/src/placeholder.cpp"
    printf '[{"name":"package_smoke","repo":"https://invalid.example/package_smoke","min_version":"0.5.0"}]\n' \
        > "$case_dir/registry.json"
    cat > "$shims/curl" <<'SH'
#!/bin/sh
printf 'curl\n' >> "$WEAVE_TEST_TOOL_LOG"
cat "$WEAVE_TEST_REGISTRY"
SH
    cat > "$shims/git" <<'SH'
#!/bin/sh
printf 'git %s\n' "${1:-}" >> "$WEAVE_TEST_TOOL_LOG"
if [ "${1:-}" != clone ]; then exit 2; fi
destination="${5:?Missing clone destination}"
mkdir -p "$destination"
cp -R "$WEAVE_TEST_MODULE_FIXTURE/." "$destination/"
SH
    chmod +x "$shims/curl" "$shims/git"
    env PATH="$shims:$PATH" MAYAFLUX_ROOT="$mayaflux_root" \
        WEAVE_TEST_REGISTRY="$case_dir/registry.json" \
        WEAVE_TEST_MODULE_FIXTURE="$module_fixture" \
        WEAVE_TEST_TOOL_LOG="$case_dir/tools.log" \
        "$launcher" manage "$project" package_smoke \
        || fail "$case_name packaged project module addition failed"
    [[ -s "$project/community/package_smoke/package_smoke.cmake" ]] \
        || fail "$case_name module addition did not install the module"
    grep -qx 'package_smoke' "$project/community.cmake" \
        || fail "$case_name module addition did not register the module"
    grep -qx 'curl' "$case_dir/tools.log" \
        || fail "$case_name did not discover the fake registry tool"
    grep -qx 'git clone' "$case_dir/tools.log" \
        || fail "$case_name did not discover the fake repository tool"

    # The packaged CLI must invoke the generated Linux validator and preserve
    # findings as diagnostics rather than a script-launch error.
    if env PATH="$shims:$PATH" MAYAFLUX_ROOT="$mayaflux_root" \
        "$launcher" manage "$project" --validate > "$case_dir/validation.txt" 2>&1; then
        :
    else
        grep -q 'item(s) need attention.' "$case_dir/validation.txt" \
            || fail "$case_name validation failed without project findings"
    fi
    grep -q 'MayaFlux project diagnostics' "$case_dir/validation.txt" \
        || fail "$case_name did not run the packaged Linux validation script"

    # A controlled built executable produces both report files without
    # installing MayaFlux or launching a real project.
    mkdir -p "$project/build" "$project/data/log" \
        "$mayaflux_root/share/MayaFlux/report"
    printf '{}\n' > "$mayaflux_root/share/MayaFlux/report/mayaflux.report.json"
    cat > "$project/build/PackageSmoke" <<'SH'
#!/bin/sh
printf 'packaged run fixture\n'
printf 'packaged application report\n' > data/log/mayaflux_report.log
SH
    chmod +x "$project/build/PackageSmoke"
    "$launcher" manage "$project" --run-report \
        --executable "$project/build/PackageSmoke" \
        --mayaflux-root "$mayaflux_root" > "$case_dir/run-report.txt" \
        || fail "$case_name packaged run/report failed"
    grep -q 'Application exit code: 0' "$case_dir/run-report.txt" \
        || fail "$case_name run/report did not finish the project"
    grep -Fq "Console log: $project/mayaflux_run.txt" "$case_dir/run-report.txt" \
        || fail "$case_name did not report the console log location"
    grep -Fq "Application report: $project/data/log/mayaflux_report.log" \
        "$case_dir/run-report.txt" \
        || fail "$case_name did not report the application log location"
    grep -q 'packaged run fixture' "$project/mayaflux_run.txt" \
        || fail "$case_name did not save the project console log"
    grep -q 'packaged application report' "$project/data/log/mayaflux_report.log" \
        || fail "$case_name did not save the application report"
}

run_package_case "$APPDIR/AppRun" extracted

# Device presence alone does not prove FUSE works: libfuse can be missing, or
# the device can be inaccessible. Probe the actual runtime in either case.
# Extraction above is only a package inspection technique.
if "$IMAGE" --help > "$TEMP_DIR/image-launch.txt" 2>&1; then
    grep -q 'weave community' "$TEMP_DIR/image-launch.txt" || fail "actual AppImage CLI output was wrong"
    run_package_case "$IMAGE" mounted
    MOUNTED_CLI=1
    echo "Mounted AppImage CLI creation, module addition, validation, run/report, and host-tool discovery passed."
else
    if ! grep -iq 'fuse' "$TEMP_DIR/image-launch.txt"; then
        cat "$TEMP_DIR/image-launch.txt" >&2
        fail "AppImage launch failed without a clear FUSE prerequisite error"
    fi
    echo "AppImage runtime reported that host FUSE is unavailable."
fi

if [[ "${WEAVE_GUI_SMOKE:-0}" == 1 ]]; then
    [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]] || fail "GUI smoke requested without a display"
    [[ "$MOUNTED_CLI" -eq 1 ]] || fail "GUI smoke requires a working FUSE-mounted AppImage"
    "$IMAGE" > "$TEMP_DIR/gui.out" 2>&1 &
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

if [[ "$MOUNTED_CLI" -eq 1 ]]; then
    echo "Linux AppImage validation passed: x86_64, GLIBC <= 2.43, resources, extracted and mounted CLI."
else
    echo "Linux AppImage package checks passed; mounted CLI and GUI validation remain unavailable without FUSE."
fi
