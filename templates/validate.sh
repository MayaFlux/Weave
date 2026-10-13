#!/usr/bin/env bash

issues=0

ok() {
    printf '[OK] %s\n' "$1"
}

fix() {
    printf '[FIX] %s\n' "$1"
    issues=$((issues + 1))
}

project_dir="${1:-$(dirname -- "${BASH_SOURCE[0]}")}"

printf 'MayaFlux project diagnostics\n'
printf 'Project: %s\n' "$project_dir"

if [ ! -d "$project_dir" ]; then
    fix "Project directory is missing: $project_dir"
    exit 1
fi

printf '\nMayaFlux package\n'
package=''
manager=''
if command -v pacman >/dev/null 2>&1; then
    manager='pacman'
    packages=(mayaflux mayaflux-dev-bin)
elif command -v dnf >/dev/null 2>&1 && command -v rpm >/dev/null 2>&1; then
    manager='dnf'
    packages=(mayaflux mayaflux-dev)
elif command -v apt-get >/dev/null 2>&1 && command -v dpkg-query >/dev/null 2>&1; then
    manager='apt'
    packages=(mayaflux mayaflux-edge)
else
    fix 'No supported MayaFlux package manager was found (pacman, dnf/rpm, or apt/dpkg).'
fi

for candidate in "${packages[@]}"; do
    case "$manager" in
        pacman) pacman -Q "$candidate" >/dev/null 2>&1 && package="$candidate" ;;
        dnf) rpm -q "$candidate" >/dev/null 2>&1 && package="$candidate" ;;
        apt) [[ "$(dpkg-query -W -f='${Status}' "$candidate" 2>/dev/null)" == 'install ok installed' ]] && package="$candidate" ;;
    esac
    if [ -n "$package" ]; then
        break
    fi
done

if [ -n "$manager" ]; then
    if [ -n "$package" ]; then
        ok "$package is installed through $manager."
        case "$manager" in
            pacman)
                if pacman -Dk >/dev/null 2>&1; then
                    ok 'Installed package dependencies are consistent.'
                else
                    fix 'pacman reports missing dependencies or an inconsistent package database. Run pacman -Dk for details.'
                fi
                ;;
            dnf)
                if dnf check >/dev/null 2>&1; then
                    ok 'Installed package dependencies are consistent.'
                else
                    fix 'dnf reports dependency problems. Run dnf check for details.'
                fi
                ;;
            apt)
                simulation="$(LC_ALL=C apt-get --simulate check 2>&1)"
                if [ "$?" -ne 0 ]; then
                    fix "apt reports dependency problems: $simulation"
                else
                    ok 'Installed package dependencies are consistent.'
                fi
                ;;
        esac
    else
        fix 'No MayaFlux package is installed. Install mayaflux or the development channel through Weave.'
    fi
fi

printf '\nMayaFlux installation files\n'
config=''
if [ -n "${MAYAFLUX_ROOT:-}" ]; then
    case "$MAYAFLUX_ROOT" in
        /*) ;;
        *) fix "MAYAFLUX_ROOT must be an absolute path: $MAYAFLUX_ROOT" ;;
    esac
    for candidate in "$MAYAFLUX_ROOT/lib/cmake/MayaFlux/MayaFluxConfig.cmake" "$MAYAFLUX_ROOT/lib64/cmake/MayaFlux/MayaFluxConfig.cmake"; do
        if [ -f "$candidate" ]; then
            config="$candidate"
            break
        fi
    done
else
    for candidate in /usr/lib/cmake/MayaFlux/MayaFluxConfig.cmake /usr/lib64/cmake/MayaFlux/MayaFluxConfig.cmake /usr/local/lib/cmake/MayaFlux/MayaFluxConfig.cmake; do
        if [ -f "$candidate" ]; then
            config="$candidate"
            break
        fi
    done
fi

if [ -n "$config" ]; then
    ok "MayaFlux CMake config: $config"
else
    fix "MayaFluxConfig.cmake is missing from the locations searched by this project's CMake template. Install MayaFlux, or set MAYAFLUX_ROOT to its installation prefix."
fi

printf '\nProject files\n'
for relative in CMakeLists.txt CMakePresets.json cmake/mayaflux.cmake cmake/shaders.cmake cmake/build_community.cmake src/main.cpp src/user_project.hpp community.cmake .gitignore; do
    if [ -f "$project_dir/$relative" ]; then
        ok "$relative"
    else
        fix "Missing project file: $relative"
    fi
done

if [ -d "$project_dir/data/shaders" ]; then
    ok 'data/shaders/'
else
    fix 'Missing project directory: data/shaders/'
fi

printf '\nSummary\n'
if (( issues == 0 )); then
    printf 'No problems found in these checks.\n'
else
    printf '%d item(s) need attention.\n' "$issues"
fi
printf 'Copy this report when requesting help.\n'

if (( issues > 0 )); then
    exit 1
fi
