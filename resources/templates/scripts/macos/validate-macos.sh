#!/bin/bash

issues=0

ok() {
    printf '[OK] %s\n' "$1"
}

fix() {
    printf '[FIX] %s\n' "$1"
    issues=$((issues + 1))
}

project_dir="${1:-$(cd "$(dirname "$0")" && pwd)}"

printf 'MayaFlux project diagnostics (macOS)\n'
printf 'Project: %s\n' "$project_dir"

if [ ! -d "$project_dir" ]; then
    fix "Project directory is missing: $project_dir"
    exit 1
fi

brew_cmd="$(command -v brew 2>/dev/null || true)"
case "$brew_cmd" in
    /usr/local/*) brew_cmd='' ;;
esac
if [ -z "$brew_cmd" ]; then
    for candidate in /opt/*/bin/brew; do
        if [ -x "$candidate" ]; then
            brew_cmd="$candidate"
            break
        fi
    done
fi

if [ -z "$brew_cmd" ] || [ ! -x "$brew_cmd" ]; then
    fix 'Homebrew was not found. Add its bin directory to PATH, then open a new terminal.'
else
    ok "Homebrew: $brew_cmd"
fi

printf '\nMayaFlux environment\n'
zshenv="$HOME/.zshenv"
if [ -n "${ZDOTDIR:-}" ] && [ -d "$ZDOTDIR" ]; then
    zshenv="$ZDOTDIR/.zshenv"
fi

if [ -f "$zshenv" ]; then
    ok "zsh startup file: $zshenv"
    grep -Fq '# MayaFlux (installed via Homebrew)' "$zshenv" \
        || fix "The MayaFlux setup block is missing from $zshenv. Use Install MayaFlux in Weave to configure it."
else
    fix "zsh startup file is missing: $zshenv. Use Install MayaFlux in Weave to configure it."
fi

if [ -n "$brew_cmd" ] && [ -x "$brew_cmd" ]; then
    formula=''
    prefix=''
    for candidate in mayaflux-dev mayaflux; do
        if "$brew_cmd" list --formula 2>/dev/null | grep -qx "$candidate"; then
            formula="$candidate"
            prefix="$("$brew_cmd" --prefix "$candidate" 2>/dev/null || true)"
            break
        fi
    done
    if [ -n "$formula" ]; then
        ok "Homebrew formula: $formula"
    else
        fix 'Neither mayaflux-dev nor mayaflux is installed through Homebrew. Install MayaFlux in Weave.'
    fi
    if [ -n "$prefix" ]; then
        expected_env="$prefix/env.sh"
        if [ -f "$expected_env" ]; then
            ok "MayaFlux environment file: $expected_env"
            if [ -f "$zshenv" ] && grep -Fq "source '$expected_env'" "$zshenv"; then
                ok 'The zsh startup file sources the installed MayaFlux environment.'
            else
                fix "The zsh startup file does not source $expected_env. Use Weave's Install MayaFlux operation to restore the shell configuration."
            fi
        else
            fix "Homebrew reported a MayaFlux prefix without env.sh: $prefix"
        fi
    elif [ -n "$formula" ]; then
        fix "Could not determine the Homebrew prefix for $formula."
    fi
fi

if [ -n "${MAYAFLUX_ROOT:-}" ]; then
    case "$MAYAFLUX_ROOT" in
        /*)
            ok "MAYAFLUX_ROOT: $MAYAFLUX_ROOT"
            config="$MAYAFLUX_ROOT/lib/cmake/MayaFlux/MayaFluxConfig.cmake"
            if [ -f "$config" ]; then
                ok "MayaFlux CMake config: $config"
            else
                fix "MayaFluxConfig.cmake is missing under MAYAFLUX_ROOT: $MAYAFLUX_ROOT"
            fi
            ;;
        *)
            fix "MAYAFLUX_ROOT must be an absolute path: $MAYAFLUX_ROOT"
            ;;
    esac
else
    fix 'MAYAFLUX_ROOT is not set in this terminal. Open a new terminal after installing MayaFlux with Weave.'
fi

printf '\nBuild tools\n'
if command -v cmake >/dev/null 2>&1; then
    cmake_version="$(cmake --version 2>/dev/null | sed -n '1s/[^0-9]*\([0-9][0-9.]*\).*/\1/p')"
    cmake_major="${cmake_version%%.*}"
    cmake_minor="${cmake_version#*.}"
    cmake_minor="${cmake_minor%%.*}"
    if [ -n "$cmake_version" ] && { [ "$cmake_major" -gt 3 ] || { [ "$cmake_major" -eq 3 ] && [ "$cmake_minor" -ge 31 ]; }; }; then
        ok "CMake $cmake_version"
    else
        fix "CMake 3.31 or newer is required; found ${cmake_version:-an unrecognized version}. Install or update CMake with Homebrew."
    fi
else
    fix 'CMake was not found. Install CMake with Homebrew.'
fi

if command -v ninja >/dev/null 2>&1; then
    ok "Ninja: $(command -v ninja)"
else
    fix 'Ninja was not found. Install Ninja with Homebrew.'
fi

printf '\nProject files\n'
for relative in CMakeLists.txt CMakePresets.json cmake/mayaflux.cmake cmake/shaders.cmake cmake/build_community.cmake src/main.cpp src/user_project.hpp community.cmake .gitignore; do
    if [ -s "$project_dir/$relative" ]; then
        ok "$relative"
    else
        fix "Missing or empty project file: $relative"
    fi
done

if [ -d "$project_dir/data/shaders" ]; then
    ok 'data/shaders/'
else
    fix 'Missing project directory: data/shaders/'
fi

for relative in CMakeLists.txt cmake/mayaflux.cmake; do
    if [ -f "$project_dir/$relative" ] && grep -Eq '@[A-Z][A-Z_]*@' "$project_dir/$relative"; then
        fix "Project template substitution is unfinished in $relative."
    fi
done

if [ -f "$project_dir/CMakePresets.json" ]; then
    if command -v python3 >/dev/null 2>&1; then
        if python3 - "$project_dir/CMakePresets.json" <<'PY'
import json
import sys

try:
    with open(sys.argv[1], encoding="utf-8") as source:
        presets = json.load(source)
    configure = {item["name"] for item in presets["configurePresets"]}
    builds = {item["name"]: item["configurePreset"] for item in presets["buildPresets"]}
    if any(name not in configure or builds.get(name) != name for name in ("debug", "release")):
        raise ValueError("debug and release configure/build preset pairs are required")
except (OSError, ValueError, KeyError, TypeError) as error:
    print(error)
    sys.exit(1)
PY
        then
            ok 'Project presets include debug and release configure/build pairs.'
        else
            fix 'Project presets are invalid or missing debug and release configure/build pairs.'
        fi
    else
        fix 'Project presets could not be checked because python3 is unavailable.'
    fi
fi

if [ -f "$project_dir/community.cmake" ]; then
    while IFS= read -r module || [ -n "$module" ]; do
        module="$(printf '%s' "$module" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
        [ -z "$module" ] && continue
        case "$module" in \#*) continue ;; esac
        if ! [[ "$module" =~ ^[a-z][a-z0-9_]*$ ]]; then
            fix "Unsafe or unsupported community module entry: $module"
        elif [ ! -f "$project_dir/community/$module/$module.cmake" ]; then
            fix "Community module $module is listed but its CMake file is missing."
        fi
    done < "$project_dir/community.cmake"
fi

printf '\nProject configuration\n'
if [ -f "$project_dir/mayaflux.json" ]; then
    if command -v python3 >/dev/null 2>&1; then
        if config_error="$(python3 - "$project_dir/mayaflux.json" <<'PY'
import json
import sys

known = {"stream", "graphics", "input", "network", "journal"}
try:
    with open(sys.argv[1], encoding="utf-8") as source:
        data = json.load(source)
    if not isinstance(data, dict):
        raise ValueError("the top level must be a JSON object")
    unknown = sorted(set(data) - known)
    if unknown:
        raise ValueError("unknown top-level section(s): " + ", ".join(unknown))
except (OSError, ValueError) as error:
    print(error)
    sys.exit(1)
PY
)"; then
            ok 'mayaflux.json is valid JSON with known sections.'
        else
            fix "mayaflux.json could not be validated: $config_error"
        fi
    else
        fix 'mayaflux.json could not be checked because python3 is unavailable.'
    fi
else
    printf '[INFO] No mayaflux.json in the project. Engine defaults apply.\n'
fi

printf '\nSystem\n'
printf '[INFO] macOS: %s (%s)\n' "$(sw_vers -productVersion 2>/dev/null || echo unknown)" "$(uname -m 2>/dev/null || echo unknown)"
if [ -n "${config:-}" ]; then
    version_file="$(dirname "$config")/MayaFluxConfigVersion.cmake"
    if [ -f "$version_file" ]; then
        installed_version="$(sed -n 's/^set(PACKAGE_VERSION "\(.*\)")/\1/p' "$version_file" | head -n 1)"
        [ -n "$installed_version" ] && printf '[INFO] MayaFlux version: %s\n' "$installed_version"
    fi
fi
if [ -n "${MAYAFLUX_ROOT:-}" ]; then
    report_config="$MAYAFLUX_ROOT/share/MayaFlux/report/mayaflux.report.json"
    if [ -f "$report_config" ]; then
        ok "Report configuration: $report_config"
    else
        printf '[INFO] Report configuration not found at %s. It ships with MayaFlux 0.5 and later.\n' "$report_config"
    fi
fi
if [ -f "$project_dir/build/CMakeCache.txt" ]; then
    build_type="$(sed -n 's/^CMAKE_BUILD_TYPE:[A-Z]*=//p' "$project_dir/build/CMakeCache.txt" | head -n 1)"
    printf '[INFO] Project build type: %s\n' "${build_type:-unset}"
fi
for variable in MAYAFLUX_ROOT MAYAFLUX_GPU VK_ICD_FILENAMES VK_LAYER_PATH DYLD_LIBRARY_PATH; do
    if [ -n "${!variable:-}" ]; then
        printf '[INFO] %s=%s\n' "$variable" "${!variable}"
    fi
done
if command -v system_profiler >/dev/null 2>&1; then
    while IFS= read -r line; do
        printf '[INFO] GPU: %s\n' "$line"
    done < <(system_profiler SPDisplaysDataType 2>/dev/null | grep -E 'Chipset Model|Metal' | sed 's/^[[:space:]]*//')
fi
if command -v vulkaninfo >/dev/null 2>&1; then
    while IFS= read -r line; do
        printf '[INFO] Vulkan: %s\n' "$line"
    done < <(vulkaninfo --summary 2>/dev/null | grep -E 'deviceName|deviceType|driverName|driverInfo|apiVersion' | sed 's/^[[:space:]]*//')
else
    printf '[INFO] vulkaninfo was not found. Install vulkan-tools with Homebrew to include Vulkan details.\n'
fi
layer_manifest=''
for directory in "${VK_LAYER_PATH:-}" "$([ -n "$brew_cmd" ] && "$brew_cmd" --prefix 2>/dev/null)/share/vulkan/explicit_layer.d"; do
    if [ -n "$directory" ] && [ -f "$directory/VkLayer_khronos_validation.json" ]; then
        layer_manifest="$directory/VkLayer_khronos_validation.json"
        break
    fi
done
if [ -n "$layer_manifest" ]; then
    printf '[INFO] Vulkan validation layer: %s\n' "$layer_manifest"
else
    printf '[INFO] Vulkan validation layer not found. Install vulkan-validationlayers with Homebrew to use the report configuration.\n'
fi

printf '\nSummary\n'
if [ "$issues" -eq 0 ]; then
    printf 'No problems found in these checks.\n'
else
    printf '%d item(s) need attention.\n' "$issues"
fi
printf 'Copy this report when requesting help.\n'

if [ "$issues" -gt 0 ]; then
    exit 1
fi
