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
[ -x "$cli" ] || fail "Packaged CLI is not executable"
assert_directory "$resources/templates"
assert_file "$resources/templates/CMakeLists.txt"
assert_file "$resources/templates/community/community.json"
assert_file "$resources/templates/validate-macos.sh"
assert_file "$resources/templates/run-report-macos.sh"

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
printf 'compiled shader fixture\n' >"$shader_source/includes/compiled.comp.spv"
printf 'fragment shader fixture\n' >"$shader_source/basic.frag"

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
for utility in zsh cat chmod cp date dirname basename find grep head ln ls mkdir mv od pwd rm sed seq sleep tee touch tr wc python3; do
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
assert_failure_contains 'Unknown option' "$cli" new UnknownOption "$test_root" --unsupported-option
[ ! -e "$test_root/UnknownOption" ] || fail "Unknown option left a partial project"
assert_failure_contains 'destination' "$cli" new MissingParent "$test_root/missing parent" --no-vscode
[ ! -e "$test_root/missing parent" ] || fail "Missing destination was unexpectedly created"

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
assert_file "$project/run-report.sh"
[ -x "$project/run-report.sh" ] || fail "Project report runner is not executable"
assert_output_contains "$(cat "$project/run-report.sh")" "project_name='DemoProject'"
assert_file "$project/community.cmake"
assert_file "$project/.vscode/settings.json"
assert_output_contains "$(cat "$project/CMakeLists.txt")" DemoProject
assert_output_contains "$(cat "$project/cmake/mayaflux.cmake")" MayaFlux::MayaFluxHost

packaged_templates="$resources/templates"
mv "$packaged_templates/user_project.hpp" "$packaged_templates/user_project.hpp.saved"
if broken_template_output="$("$cli" new MissingTemplateProject "$project_parent" --no-vscode 2>&1)"; then
    fail "CLI unexpectedly created a project without a required packaged template"
fi
assert_output_contains "$broken_template_output" 'Required template missing: user_project.hpp'
[ ! -e "$project_parent/MissingTemplateProject" ] \
    || fail "Missing packaged template left a partial project"
mv "$packaged_templates/user_project.hpp.saved" "$packaged_templates/user_project.hpp"
"$cli" new MissingTemplateProject "$project_parent" --no-vscode >/dev/null \
    || fail "Project creation could not be retried after restoring a packaged template"
assert_file "$project_parent/MissingTemplateProject/src/user_project.hpp"
printf "# MayaFlux (installed via Homebrew)\nsource '%s/env.sh'\nexport PATH=\"\$HOME/.local/bin:\$PATH\"\n" \
    "$fixture_root" >"$ZDOTDIR/.zshenv"
validator_output="$("$project/validate.sh" 2>&1)" \
    || fail "Generated project validator rejected the controlled environment: $validator_output"
assert_output_contains "$validator_output" "zsh startup file: $ZDOTDIR/.zshenv"
assert_output_contains "$validator_output" 'MAYAFLUX_ROOT:'
if missing_tools_output="$(PATH="$tool_bin" "$project/validate.sh" 2>&1)"; then
    fail "Validator unexpectedly succeeded without Homebrew, CMake, or Ninja"
fi
if [[ "$missing_tools_output" != *'Homebrew was not found'* ]]; then
    assert_output_contains "$missing_tools_output" 'Homebrew:'
fi
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

report_config="$fixture_root/share/MayaFlux/report/mayaflux.report.json"
physical_project="$(cd "$project" && pwd -P)"
physical_report_config="$(cd "$test_root" && pwd -P)/mayaflux/share/MayaFlux/report/mayaflux.report.json"
mkdir -p "$(dirname "$report_config")" "$project/build" "$test_root/no-report-root"
printf '{"journal":{}}\n' >"$report_config"
cat >"$project/build/DemoProject" <<'SH'
#!/bin/bash
echo "cwd=$(pwd -P)"
echo "config=${2:-INVALID} flag=${1:-INVALID}"
echo "environment=${WEAVE_TEST_REPORT_VALUE:-unset}"
echo "stdout: diagnostic output"
echo "stderr: crash details" >&2
if [ "${WEAVE_TEST_REPORT_NO_LOG:-0}" != 1 ]; then
    mkdir -p data/log
    printf 'fresh application report' >data/log/mayaflux_report.log
fi
exit "${WEAVE_TEST_REPORT_EXIT:-0}"
SH
chmod +x "$project/build/DemoProject"
export WEAVE_TEST_REPORT_VALUE='terminal value'
report_output="$("$project/run-report.sh" 2>&1)" \
    || fail "Report runner failed with a controlled application: $report_output"
console_capture="$(cat "$project/mayaflux_run.txt")"
assert_output_contains "$console_capture" "cwd=$physical_project"
assert_output_contains "$console_capture" "config=$physical_report_config flag=--config"
assert_output_contains "$console_capture" 'environment=terminal value'
assert_output_contains "$console_capture" 'stdout: diagnostic output'
assert_output_contains "$console_capture" 'stderr: crash details'
assert_output_contains "$console_capture" 'Application exit code: 0'
assert_output_contains "$report_output" "$physical_project/mayaflux_run.txt"
assert_output_contains "$report_output" "$physical_project/data/log/mayaflux_report.log"
[ "$(cat "$project/data/log/mayaflux_report.log")" = 'fresh application report' ] \
    || fail "Report runner did not leave the application report in place"

export WEAVE_TEST_REPORT_EXIT=37
set +e
report_output="$("$project/run-report.sh" 2>&1)"
report_status=$?
set -e
[ "$report_status" -eq 37 ] || fail "Report runner returned $report_status instead of the application's exit code"
assert_output_contains "$report_output" 'Application exit code: 37'
[ "$(ls "$project"/mayaflux_run.txt.previous-* | wc -l | tr -d ' ')" = 1 ] \
    || fail "Report runner did not preserve exactly one previous console log"
[ "$(ls "$project"/data/log/mayaflux_report.log.previous-* | wc -l | tr -d ' ')" = 1 ] \
    || fail "Report runner did not preserve exactly one previous application report"
unset WEAVE_TEST_REPORT_EXIT

export WEAVE_TEST_REPORT_EXIT=23 WEAVE_TEST_REPORT_NO_LOG=1
rm -f "$project/data/log/mayaflux_report.log"
set +e
report_output="$("$project/run-report.sh" 2>&1)"
report_status=$?
set -e
[ "$report_status" -eq 23 ] || fail "Early crash returned $report_status instead of 23"
assert_output_contains "$report_output" 'Application report was not created'
assert_output_contains "$report_output" 'mayaflux_run.txt'
unset WEAVE_TEST_REPORT_EXIT WEAVE_TEST_REPORT_NO_LOG

printf 'previous console\n' >"$project/mayaflux_run.txt"
assert_failure_contains 'Unknown option' "$project/run-report.sh" --unsupported
assert_failure_contains 'requires a value' "$project/run-report.sh" --executable
assert_failure_contains 'Choose Debug, Release' "$project/run-report.sh" --configuration Fast
assert_failure_contains 'project name is unsafe' "$project/run-report.sh" --project-name '../outside'
assert_failure_contains 'missing or is not executable' "$project/run-report.sh" --executable missing
assert_failure_contains 'MayaFlux installation folder does not exist' "$project/run-report.sh" --mayaflux-root "$test_root/absent"
assert_failure_contains 'report configuration is missing' "$project/run-report.sh" --mayaflux-root "$test_root/no-report-root"
assert_failure_contains 'project folder does not exist' "$project/run-report.sh" --project-root "$test_root/absent"
printf '{broken\n' >"$report_config"
assert_failure_contains 'not a readable JSON object' "$project/run-report.sh"
printf '[1]\n' >"$report_config"
assert_failure_contains 'not a readable JSON object' "$project/run-report.sh"
printf '{"journal":{}}\n' >"$report_config"
[ "$(cat "$project/mayaflux_run.txt")" = 'previous console' ] \
    || fail "A rejected report run altered the previous console log"

mkdir -p "$project/build/Release"
cp "$project/build/DemoProject" "$project/build/Release/DemoProject"
assert_failure_contains 'multiple project executables' "$project/run-report.sh"
mv "$project/build/DemoProject" "$test_root/DemoProject.saved"
"$project/run-report.sh" --configuration Release >/dev/null 2>&1 \
    || fail "Report runner rejected an explicit configuration"
mv "$test_root/DemoProject.saved" "$project/build/DemoProject"
rm -rf "$project/build/Release"

mkdir -p "$project/custom output"
cp "$project/build/DemoProject" "$project/custom output/Renamed"
rm "$project/build/DemoProject"
assert_failure_contains 'No project executable was found' "$project/run-report.sh"
"$project/run-report.sh" --executable 'custom output/Renamed' --mayaflux-root "$fixture_root" >/dev/null 2>&1 \
    || fail "Report runner rejected a custom executable layout"
printf '\000\001\002\003binary' >"$project/custom output/Broken"
chmod +x "$project/custom output/Broken"
assert_failure_contains 'could not be started' "$project/run-report.sh" --executable 'custom output/Broken'
if no_root_output="$(PATH="$tool_bin" MAYAFLUX_ROOT= "$project/run-report.sh" --executable 'custom output/Renamed' 2>&1)"; then
    fail "Report runner unexpectedly found MayaFlux without a root or Homebrew"
fi
assert_output_contains "$no_root_output" 'MayaFlux was not found'

shader_project_parent="$test_root/Shader Projects"
mkdir -p "$shader_project_parent"
shader_project_output="$("$cli" new ShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1)" \
    || fail "Packaged CLI could not create a project with installed shaders: $shader_project_output"
shader_project="$shader_project_parent/ShaderProject"
assert_file "$shader_project/data/shaders/basic.vert"
assert_file "$shader_project/data/shaders/includes/common.glsl"
assert_file "$shader_project/data/shaders/basic.frag"
assert_output_contains "$(cat "$shader_project/data/shaders/includes/common.glsl")" 'include fixture'
assert_output_contains "$(cat "$shader_project/data/shaders/basic.frag")" 'fragment shader fixture'
[ ! -e "$shader_project/data/shaders/includes/compiled.comp.spv" ] \
    || fail "Shader copy included an unsupported compiled shader"
(cd "$shader_project_parent" && "$cli" new DefaultShaderProject --copy-shader-sources --no-vscode >/dev/null) \
    || fail "CLI did not accept --copy-shader-sources when destination was omitted"
assert_file "$shader_project_parent/DefaultShaderProject/data/shaders/basic.vert"

mkdir -p "$test_root/isolated-zdotdir"
if missing_shaders_output="$(
    ZDOTDIR="$test_root/isolated-zdotdir" MAYAFLUX_ROOT="$test_root/missing-mayaflux" \
        "$cli" new MissingShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1
)"; then
    fail "Project creation unexpectedly succeeded without installed shader sources"
fi
assert_output_contains "$missing_shaders_output" 'MayaFlux shader sources were not found'
[ ! -e "$shader_project_parent/MissingShaderProject" ] \
    || fail "Missing shader source preflight left a partial project"

mkdir -p "$test_root/empty-mayaflux/share/MayaFlux/src/shaders"
if empty_shaders_output="$(
    ZDOTDIR="$test_root/isolated-zdotdir" MAYAFLUX_ROOT="$test_root/empty-mayaflux" \
        "$cli" new EmptyShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1
)"; then
    fail "Project creation unexpectedly succeeded with an empty installed shader directory"
fi
assert_output_contains "$empty_shaders_output" 'MayaFlux shader sources are empty'
[ ! -e "$shader_project_parent/EmptyShaderProject" ] \
    || fail "Empty shader source preflight left a partial project"

ln -s "$shader_source/basic.vert" "$shader_source/linked.vert"
if linked_shaders_output="$("$cli" new LinkedShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode 2>&1)"; then
    fail "Project creation unexpectedly accepted a linked shader source"
fi
assert_output_contains "$linked_shaders_output" 'MayaFlux shader sources contain a symbolic link'
[ ! -e "$shader_project_parent/LinkedShaderProject" ] \
    || fail "Linked shader source preflight left a partial project"
rm "$shader_source/linked.vert"

existing_project="$shader_project_parent/ExistingProject"
mkdir -p "$existing_project"
printf 'preserve this project\n' >"$existing_project/user-data.txt"
assert_failure_contains 'already exists' "$cli" new ExistingProject "$shader_project_parent" --no-vscode
assert_output_contains "$(cat "$existing_project/user-data.txt")" 'preserve this project'
printf 'not a project\n' >"$shader_project_parent/OccupiedPath"
assert_failure_contains 'already exists' "$cli" new OccupiedPath "$shader_project_parent" --no-vscode
assert_output_contains "$(cat "$shader_project_parent/OccupiedPath")" 'not a project'

ln -s "$shader_project_parent" "$test_root/Linked Projects"
assert_failure_contains 'destination' "$cli" new LinkedParent "$test_root/Linked Projects" --no-vscode
[ ! -e "$shader_project_parent/LinkedParent" ] \
    || fail "Project creation followed a linked destination directory"

mkdir -p "$test_root/failing-copy"
cat >"$test_root/failing-copy/cp" <<'SH'
#!/bin/sh
if [ "${WEAVE_TEST_FAIL_ANY_COPY:-0}" = 1 ]; then
    exit 7
fi
if [ "${1:-}" = "$WEAVE_TEST_SHADER_FAILURE_TARGET" ]; then
    /bin/cp "$@"
    exit 7
fi
exec /bin/cp "$@"
SH
chmod +x "$test_root/failing-copy/cp"
export WEAVE_TEST_SHADER_FAILURE_TARGET="$(cd "$shader_source/includes" && pwd -P)/common.glsl"
if PATH="$test_root/failing-copy:$PATH" "$cli" new RetryShaderProject "$shader_project_parent" \
    --copy-shader-sources --no-vscode >/dev/null 2>&1; then
    fail "Project creation unexpectedly succeeded after shader copy failed"
fi
[ ! -e "$shader_project_parent/RetryShaderProject" ] \
    || fail "Failed shader copy left a partial project"
unset WEAVE_TEST_SHADER_FAILURE_TARGET
"$cli" new RetryShaderProject "$shader_project_parent" --copy-shader-sources --no-vscode >/dev/null \
    || fail "Project creation could not be retried after shader copy failure"
assert_file "$shader_project_parent/RetryShaderProject/data/shaders/includes/common.glsl"

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
assert_failure_contains 'snake_case' "$cli" community '../outside' "$module_parent"
[ ! -e "$test_root/outside" ] || fail "Unsafe community module name escaped its destination"
ln -s "$module_parent" "$test_root/Linked Modules"
assert_failure_contains 'destination' "$cli" community linked_module "$test_root/Linked Modules"
[ ! -e "$module_parent/linked_module" ] || fail "Community creation followed a linked destination"
printf 'preserve module\n' >"$module_parent/fixture_module/user-data.txt"
assert_failure_contains 'already exists' "$cli" community fixture_module "$module_parent"
assert_output_contains "$(cat "$module_parent/fixture_module/user-data.txt")" 'preserve module'
mkdir "$module_parent/SiblingModule"
printf 'keep sibling\n' >"$module_parent/SiblingModule/keep.txt"
export WEAVE_TEST_FAIL_ANY_COPY=1
if PATH="$test_root/failing-copy:$PATH" "$cli" community retry_module "$module_parent" >/dev/null 2>&1; then
    fail "Community creation unexpectedly succeeded after a template copy failure"
fi
unset WEAVE_TEST_FAIL_ANY_COPY
[ ! -e "$module_parent/retry_module" ] || fail "Failed community creation left a partial module"
assert_output_contains "$(cat "$module_parent/SiblingModule/keep.txt")" 'keep sibling'
"$cli" community retry_module "$module_parent" >/dev/null \
    || fail "Community creation could not be retried after template copy failure"
assert_file "$module_parent/retry_module/retry_module.cmake"

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

comma_project="$test_root/Comma Update Project"
mkdir -p "$comma_project"
printf 'project(CommaFixture)\n' >"$comma_project/CMakeLists.txt"
"$cli" update "$comma_project" "fixture_module, fixture_module" fixture_module >/dev/null \
    || fail "Packaged CLI could not update a project from a comma separated list"
[ "$(cat "$comma_project/community.cmake")" = fixture_module ] \
    || fail "Comma separated duplicates were not registered once"

batch_project="$test_root/Batch Update Project"
mkdir -p "$batch_project"
printf 'project(BatchFixture)\n' >"$batch_project/CMakeLists.txt"
if "$cli" update "$batch_project" "fixture_module,absent_module" >/dev/null 2>&1; then
    fail "Update unexpectedly succeeded with an unknown module in the batch"
fi
[ ! -s "$batch_project/community.cmake" ] \
    || fail "A failed batch registered a module before every module validated"

for bad_names in "" "fixture_module," ",fixture_module" "a,,b"; do
    assert_failure_contains 'Empty module name' "$cli" update "$batch_project" "$bad_names"
done
for bad_names in "../outside" "Fixture"; do
    assert_failure_contains 'snake_case' "$cli" update "$batch_project" "$bad_names"
done
[ ! -e "$batch_project/community" ] || fail "Rejected module names reached the registry or clone steps"

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
