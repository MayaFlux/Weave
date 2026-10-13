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
        || fix "The MayaFlux setup block is missing from $zshenv. Run environment setup in Weave."
    grep -Fq 'export PATH="$HOME/.local/bin:$PATH"' "$zshenv" \
        || fix "The zsh startup file does not add $HOME/.local/bin to PATH. Run environment setup in Weave."
else
    fix "zsh startup file is missing: $zshenv. Run environment setup in Weave."
fi

if [[ ":${PATH:-}:" == *":$HOME/.local/bin:"* ]]; then
    ok 'The current PATH includes $HOME/.local/bin.'
else
    fix 'The current PATH does not include $HOME/.local/bin. Restart the terminal after running environment setup in Weave.'
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
                fix "The zsh startup file does not source $expected_env. Run environment setup in Weave."
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
    fix 'MAYAFLUX_ROOT is not set in this terminal. Restart the terminal after running environment setup in Weave.'
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
