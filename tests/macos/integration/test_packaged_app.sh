#!/bin/bash
set -euo pipefail

app_source="${1:?Usage: test_packaged_app.sh /path/to/Weave.app}"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/weave-macos-integration.XXXXXX")"
trap '/bin/rm -rf "$test_root"' EXIT

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

assert_file() {
    [ -f "$1" ] || fail "Expected packaged file: $1"
}

assert_directory() {
    [ -d "$1" ] || fail "Expected packaged directory: $1"
}

assert_output_contains() {
    local output="$1"
    local expected="$2"
    [[ "$output" == *"$expected"* ]] || fail "Expected output to contain: $expected; received: $output"
}

assert_failure_contains() {
    local expected="$1"
    shift
    local output
    if output="$("$@" 2>&1)"; then
        fail "Expected command to fail: $*"
    fi
    assert_output_contains "$output" "$expected"
}

app="$test_root/Relocated Weave.app"
ditto "$app_source" "$app"
resources="$app/Contents/Resources"
cli="$resources/weave"
assert_file "$app/Contents/MacOS/Weave"
assert_file "$app/Contents/Info.plist"
assert_file "$cli"
assert_directory "$resources/templates"
assert_file "$resources/templates/CMakeLists.txt"
assert_file "$resources/templates/community/community.json"
assert_file "$resources/templates/validate-macos.sh"

architectures="$(lipo -archs "$app/Contents/MacOS/Weave")"
[ "$architectures" = arm64 ] || fail "Expected arm64 app binary, found: $architectures"
minimum_version="$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$app/Contents/Info.plist")"
[ "$minimum_version" = 26.0 ] || fail "Expected macOS minimum 26.0, found: $minimum_version"

if codesign -dv --verbose=2 "$app" >/dev/null 2>&1; then
    codesign --verify --deep --strict "$app" || fail "Code-signing verification failed"
fi

fixture_root="$test_root/mayaflux"
mkdir -p "$fixture_root/lib/cmake/MayaFlux" "$test_root/shims"
printf 'set(PACKAGE_VERSION "0.5.0")\n' \
    >"$fixture_root/lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake"
printf 'include(CMakeFindDependencyMacro)\n' \
    >"$fixture_root/lib/cmake/MayaFlux/MayaFluxConfig.cmake"
printf 'export MAYAFLUX_ROOT=%s\n' "$fixture_root" >"$fixture_root/env.sh"
shader_source="$fixture_root/share/MayaFlux/src/shaders"
mkdir -p "$shader_source/includes"
printf 'vertex shader fixture\n' >"$shader_source/basic.vert"
printf 'include fixture\n' >"$shader_source/includes/common.glsl"

cat >"$test_root/shims/curl" <<'SH'
#!/bin/sh
cat "$WEAVE_TEST_REGISTRY"
SH

cat >"$test_root/shims/git" <<'SH'
#!/bin/sh
if [ "${1:-}" = clone ]; then
    if [ "${WEAVE_TEST_FAIL_CLONE:-0}" = 1 ]; then
        exit 1
    fi
    destination="${4:?Missing clone destination}"
    printf 'fixture=%s destination=%s\n' "$WEAVE_TEST_MODULE_FIXTURE" "$destination" >>"$WEAVE_TEST_GIT_LOG"
    mkdir -p "$destination"
    cp -R "$WEAVE_TEST_MODULE_FIXTURE/." "$destination/" >>"$WEAVE_TEST_GIT_LOG" 2>&1
    status=$?
    printf 'copy_status=%s\n' "$status" >>"$WEAVE_TEST_GIT_LOG"
    exit "$status"
fi
exec /usr/bin/git "$@"
SH
cat >"$test_root/shims/jq" <<'SH'
#!/bin/sh
printf 'https://invalid.example/%s 0.5.0\n' "$4"
SH
cat >"$test_root/shims/brew" <<'SH'
#!/bin/sh
case "${1:-}" in
    list) printf 'mayaflux-dev\n' ;;
    --prefix) printf '%s\n' "$WEAVE_TEST_MAYAFLUX_ROOT" ;;
    *) exit 1 ;;
esac
SH
cat >"$test_root/shims/cmake" <<'SH'
#!/bin/sh
printf 'cmake version 3.31.0\n'
SH
cat >"$test_root/shims/ninja" <<'SH'
#!/bin/sh
exit 0
SH
chmod +x "$test_root/shims/curl" "$test_root/shims/git" "$test_root/shims/jq" \
    "$test_root/shims/brew" "$test_root/shims/cmake" "$test_root/shims/ninja"
mkdir -p "$test_root/curl-only"
ln -s "$test_root/shims/curl" "$test_root/curl-only/curl"

mkdir -p "$test_root/home" "$test_root/zdotdir"
export HOME="$test_root/home"
export ZDOTDIR="$test_root/zdotdir"
tool_bin="$test_root/tools"
mkdir -p "$tool_bin"
for utility in zsh cat cp dirname find grep head ln ls mkdir mv pwd rm sed seq sleep touch tr wc python3; do
    utility_path="$(command -v "$utility" || true)"
    [ -n "$utility_path" ] && ln -s "$utility_path" "$tool_bin/$utility"
done
mkdir -p "$test_root/home/.local/bin"
export PATH="$HOME/.local/bin:$test_root/shims:$tool_bin"
export MAYAFLUX_ROOT="$fixture_root"
export WEAVE_TEST_MAYAFLUX_ROOT="$fixture_root"
export WEAVE_TEST_REGISTRY="$test_root/registry.json"
export WEAVE_TEST_MODULE_FIXTURE="$test_root/module-fixture"
export WEAVE_TEST_GIT_LOG="$test_root/git.log"
mkdir -p "$WEAVE_TEST_MODULE_FIXTURE/src"
printf 'set(MF_MIN_VERSION "0.5.0")\n' \
    >"$WEAVE_TEST_MODULE_FIXTURE/fixture_module.cmake"
printf '[]\n' >"$WEAVE_TEST_MODULE_FIXTURE/src/placeholder.cpp"
printf '[{"name":"fixture_module","repo":"https://invalid.example/fixture_module","min_version":"0.5.0"}]\n' \
    >"$WEAVE_TEST_REGISTRY"

[ "$(command -v git)" = "$test_root/shims/git" ] || fail "Fixture git was not selected from test PATH"
git clone --depth=1 https://invalid.example/preflight "$test_root/preflight-clone"
assert_file "$test_root/preflight-clone/fixture_module.cmake"

help_output="$("$cli" --help)"
assert_output_contains "$help_output" 'weave new'
assert_output_contains "$help_output" '--copy-shader-sources'

project_parent="$test_root/Projects With Spaces"
mkdir -p "$project_parent"
project_output="$("$cli" new DemoProject "$project_parent" --with-lila 2>&1)" \
    || fail "Packaged CLI could not create project: $project_output"
project="$project_parent/DemoProject"
assert_file "$project/CMakeLists.txt"
assert_file "$project/src/main.cpp"
assert_file "$project/cmake/build_community.cmake"
assert_file "$project/validate.sh"
[ -x "$project/validate.sh" ] || fail "Project validator is not executable"
assert_file "$project/community.cmake"
assert_file "$project/.vscode/settings.json"
assert_output_contains "$(cat "$project/CMakeLists.txt")" DemoProject
assert_output_contains "$(cat "$project/cmake/mayaflux.cmake")" MayaFlux::MayaFluxHost
printf "# MayaFlux (installed via Homebrew)\nsource '%s/env.sh'\nexport PATH=\"\$HOME/.local/bin:\$PATH\"\n" \
    "$fixture_root" >"$ZDOTDIR/.zshenv"
validator_output="$("$project/validate.sh" 2>&1)" \
    || fail "Generated project validator rejected the controlled environment: $validator_output"
assert_output_contains "$validator_output" "zsh startup file: $ZDOTDIR/.zshenv"
assert_output_contains "$validator_output" 'MAYAFLUX_ROOT:'
if missing_tools_output="$(PATH="$tool_bin" "$project/validate.sh" 2>&1)"; then
    fail "Validator unexpectedly succeeded without Homebrew, CMake, or Ninja"
fi
assert_output_contains "$missing_tools_output" 'Homebrew was not found'
assert_output_contains "$missing_tools_output" 'CMake was not found'
if invalid_root_output="$(MAYAFLUX_ROOT=relative "$project/validate.sh" 2>&1)"; then
    fail "Validator unexpectedly accepted a relative MAYAFLUX_ROOT"
fi
assert_output_contains "$invalid_root_output" 'MAYAFLUX_ROOT must be an absolute path'
if missing_config_output="$(MAYAFLUX_ROOT="$test_root/missing-mayaflux" "$project/validate.sh" 2>&1)"; then
    fail "Validator unexpectedly accepted MAYAFLUX_ROOT without MayaFluxConfig.cmake"
fi
assert_output_contains "$missing_config_output" 'MayaFluxConfig.cmake is missing under MAYAFLUX_ROOT'
cp "$project/CMakePresets.json" "$test_root/CMakePresets.json.saved"
printf '{bad json\n' >"$project/CMakePresets.json"
assert_failure_contains 'Project presets are invalid' "$project/validate.sh"
cp "$test_root/CMakePresets.json.saved" "$project/CMakePresets.json"
printf '../outside\n' >>"$project/community.cmake"
assert_failure_contains 'Unsafe or unsupported community module entry' "$project/validate.sh"
printf '\n' >"$project/community.cmake"
mv "$ZDOTDIR/.zshenv" "$ZDOTDIR/.zshenv.saved"
assert_failure_contains "zsh startup file is missing: $ZDOTDIR/.zshenv" "$project/validate.sh"
mv "$ZDOTDIR/.zshenv.saved" "$ZDOTDIR/.zshenv"

shader_project_parent="$test_root/Shader Projects"
mkdir -p "$shader_project_parent"
shader_project_output="$("$cli" new ShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1)" \
    || fail "Packaged CLI could not create a project with installed shaders: $shader_project_output"
shader_project="$shader_project_parent/ShaderProject"
assert_file "$shader_project/data/shaders/basic.vert"
assert_file "$shader_project/data/shaders/includes/common.glsl"
assert_output_contains "$(cat "$shader_project/data/shaders/includes/common.glsl")" 'include fixture'
(cd "$shader_project_parent" && "$cli" new DefaultShaderProject --copy-shader-sources --no-vscode >/dev/null) \
    || fail "CLI did not accept --copy-shader-sources when destination was omitted"
assert_file "$shader_project_parent/DefaultShaderProject/data/shaders/basic.vert"

if missing_shaders_output="$(MAYAFLUX_ROOT="$test_root/missing-mayaflux" "$cli" new MissingShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1)"; then
    fail "Project creation unexpectedly succeeded without installed shader sources"
fi
assert_output_contains "$missing_shaders_output" 'MayaFlux shader sources were not found'
[ ! -e "$shader_project_parent/MissingShaderProject" ] \
    || fail "Missing shader source preflight left a partial project"

ln -s "$shader_source/basic.vert" "$shader_source/linked.vert"
if linked_shaders_output="$("$cli" new LinkedShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1)"; then
    fail "Project creation unexpectedly accepted a linked shader source"
fi
assert_output_contains "$linked_shaders_output" 'MayaFlux shader sources contain a symbolic link'
[ ! -e "$shader_project_parent/LinkedShaderProject" ] \
    || fail "Linked shader source preflight left a partial project"
rm "$shader_source/linked.vert"

unsafe_parent="$test_root/Unsafe Projects"
mkdir -p "$unsafe_parent"
assert_failure_contains 'Project name must start' "$cli" new ../outside "$unsafe_parent"
[ ! -e "$test_root/outside" ] || fail "Unsafe project name escaped its destination"

module_parent="$test_root/Modules With Spaces"
mkdir -p "$module_parent"
module_output="$("$cli" community fixture_module "$module_parent" 2>&1)" \
    || fail "Packaged CLI could not create community module: $module_output"
module="$module_parent/fixture_module"
assert_file "$module/fixture_module.cmake"
assert_file "$module/community.json"
assert_file "$module/test/CMakeLists.txt"
assert_directory "$module/src"
assert_output_contains "$(cat "$module/community.json")" fixture_module

update_project="$test_root/Update Project"
mkdir -p "$update_project"
printf 'project(UpdateFixture)\n' >"$update_project/CMakeLists.txt"
update_output="$("$cli" update "$update_project" fixture_module 2>&1)" \
    || fail "Packaged CLI could not update a project: $update_output $(cat "$WEAVE_TEST_GIT_LOG" 2>/dev/null || true)"
assert_file "$update_project/community/fixture_module/fixture_module.cmake"
[ "$(cat "$update_project/community.cmake")" = fixture_module ] \
    || fail "Updated project did not register the module exactly once"
"$cli" update "$update_project" fixture_module >/dev/null
[ "$(wc -l <"$update_project/community.cmake" | tr -d ' ')" = 1 ] \
    || fail "Repeated update duplicated the module registration"

if missing_curl_output="$(PATH="$tool_bin" "$cli" update "$update_project" fixture_module 2>&1)"; then
    fail "Update unexpectedly succeeded without curl"
fi
assert_output_contains "$missing_curl_output" 'Required command not found: curl'

if missing_git_output="$(PATH="$test_root/curl-only:$tool_bin" "$cli" update "$update_project" fixture_module 2>&1)"; then
    fail "Update unexpectedly succeeded without git"
fi
assert_output_contains "$missing_git_output" 'Required command not found: git'

failed_project="$test_root/Failed Update Project"
mkdir -p "$failed_project"
printf 'project(FailedFixture)\n' >"$failed_project/CMakeLists.txt"
export WEAVE_TEST_FAIL_CLONE=1
assert_failure_contains 'Failed to clone' "$cli" update "$failed_project" fixture_module
unset WEAVE_TEST_FAIL_CLONE
[ ! -e "$failed_project/community/fixture_module" ] \
    || fail "Failed clone left a partial module directory"
[ ! -s "$failed_project/community.cmake" ] \
    || fail "Failed clone registered an incomplete module"

invalid_project="$test_root/Not a Project"
mkdir -p "$invalid_project"
assert_failure_contains 'Not a MayaFlux project' "$cli" update "$invalid_project" fixture_module

"$app/Contents/MacOS/Weave" >"$test_root/gui.log" 2>&1 &
gui_pid=$!
/bin/sleep 2
if ! kill -0 "$gui_pid" 2>/dev/null; then
    wait "$gui_pid" || true
    fail "Packaged GUI did not remain running after launch: $(cat "$test_root/gui.log")"
fi
kill -TERM "$gui_pid" 2>/dev/null || true
wait "$gui_pid" 2>/dev/null || true

printf 'PASS: packaged macOS artifact integration checks\n'
