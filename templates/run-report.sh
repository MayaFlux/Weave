#!/bin/bash

project_name='@PROJECT_NAME@'
project_root=''
executable=''
mayaflux_root=''
configuration=''
unresolved_name='@PROJECT_''NAME@'

usage() {
    cat <<'EOF'
Usage: ./run-report.sh [options]

Runs the built project with the MayaFlux report configuration and saves the
console output to mayaflux_run.txt in the project folder.

Options:
  --configuration NAME   Debug, Release, RelWithDebInfo or MinSizeRel
  --executable PATH      Project executable for a custom build layout
  --mayaflux-root PATH   MayaFlux installation folder
  --project-name NAME    Executable name when it differs from the project
  --project-root PATH    Project folder (default: this script's folder)
  -h, --help             Show this help
EOF
}

console_log=''
application_log=''
code=1

fail() {
    local message="[ERROR] $1"
    printf '%s\n' "$message"
    if [ -n "$console_log" ] && [ -f "$console_log" ]; then
        printf '%s\n' "$message" >>"$console_log"
    fi
    finish 1
}

finish() {
    code="$1"
    if [ -n "$console_log" ] && [ -f "$console_log" ]; then
        printf 'Console log: %s\n' "$console_log"
    fi
    if [ -n "$application_log" ]; then
        if [ -f "$application_log" ]; then
            printf 'Application report: %s\n' "$application_log"
        else
            printf '[WARN] Application report was not created: %s. Attach mayaflux_run.txt; the application may have stopped before logging started.\n' "$application_log"
        fi
    fi
    exit "$code"
}

absolute_path() {
    local value="$1"
    local base="$2"
    case "$value" in
        "~") value="$HOME" ;;
        "~/"*) value="$HOME/${value#"~/"}" ;;
    esac
    case "$value" in
        /*) ;;
        *) value="$base/$value" ;;
    esac
    local directory
    directory="$(cd "$(dirname "$value")" 2>/dev/null && pwd -P)" || return 1
    printf '%s/%s\n' "${directory%/}" "$(basename "$value")"
}

save_previous() {
    local path="$1"
    if [ -f "$path" ]; then
        mv "$path" "$path.previous-$2" || fail "Could not preserve the previous log: $path"
        printf 'Previous log saved: %s.previous-%s\n' "$path" "$2"
    fi
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage; exit 0 ;;
        --configuration|--executable|--mayaflux-root|--project-name|--project-root)
            [ "$#" -ge 2 ] && [ -n "$2" ] || fail "Option $1 requires a value."
            case "$1" in
                --configuration) configuration="$2" ;;
                --executable) executable="$2" ;;
                --mayaflux-root) mayaflux_root="$2" ;;
                --project-name) project_name="$2" ;;
                --project-root) project_root="$2" ;;
            esac
            shift 2
            ;;
        *) fail "Unknown option: $1. Run ./run-report.sh --help." ;;
    esac
done

[ -n "$project_root" ] || project_root="$(cd "$(dirname "$0")" && pwd -P)"
project="$(absolute_path "$project_root" "$PWD")" && [ -d "$project" ] \
    || fail 'The project folder does not exist. Check --project-root.'

case "$configuration" in
    ''|Debug|Release|RelWithDebInfo|MinSizeRel) ;;
    *) fail 'Choose Debug, Release, RelWithDebInfo or MinSizeRel with --configuration, or use --executable.' ;;
esac

if [ -n "$executable" ]; then
    executable_path="$(absolute_path "$executable" "$project")" \
        && [ -f "$executable_path" ] && [ -x "$executable_path" ] \
        || fail 'The specified executable is missing or is not executable. Check --executable.'
else
    if [ -z "$project_name" ] || [ "$project_name" = "$unresolved_name" ]; then
        cache="$project/build/CMakeCache.txt"
        if [ -f "$cache" ]; then
            names="$(sed -n 's/^CMAKE_PROJECT_NAME:[^=]*=//p' "$cache")"
            if [ -n "$names" ] && [ "$(printf '%s\n' "$names" | wc -l | tr -d ' ')" -eq 1 ]; then
                project_name="$names"
            fi
        fi
    fi
    if [ -z "$project_name" ] || [ "$project_name" = "$unresolved_name" ]; then
        fail 'Build the project first, or supply --project-name or --executable for a custom build layout.'
    fi
    [[ "$project_name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] \
        || fail 'The project name is unsafe or unsupported. Use --executable for a custom target name.'
    if [ -n "$configuration" ]; then
        layouts=(build "build/$configuration")
    else
        layouts=(build build/Debug build/Release build/RelWithDebInfo build/MinSizeRel)
    fi
    candidates=()
    for layout in "${layouts[@]}"; do
        candidate="$project/$layout/$project_name"
        if [ -f "$candidate" ] && [ -x "$candidate" ]; then
            candidates+=("$candidate")
        fi
    done
    [ "${#candidates[@]}" -gt 0 ] \
        || fail 'No project executable was found. Build the project first, or provide --executable.'
    [ "${#candidates[@]}" -eq 1 ] \
        || fail 'Found multiple project executables. Select --configuration or provide --executable.'
    executable_path="${candidates[0]}"
fi

[ -n "$mayaflux_root" ] || mayaflux_root="${MAYAFLUX_ROOT:-}"
if [ -z "$mayaflux_root" ]; then
    for prefix in /usr /usr/local; do
        if [ -f "$prefix/share/MayaFlux/report/mayaflux.report.json" ]; then
            mayaflux_root="$prefix"
            break
        fi
    done
fi
[ -n "$mayaflux_root" ] \
    || fail 'MayaFlux was not found under /usr or /usr/local. Set MAYAFLUX_ROOT or pass --mayaflux-root.'
install="$(absolute_path "$mayaflux_root" "$project")" && [ -d "$install" ] \
    || fail "The MayaFlux installation folder does not exist: $mayaflux_root. Check --mayaflux-root."
config="$install/share/MayaFlux/report/mayaflux.report.json"
[ -f "$config" ] \
    || fail "The MayaFlux report configuration is missing: $config. Update MayaFlux or specify --mayaflux-root."
first_character="$(tr -d '[:space:]' <"$config" | head -c 1)"
if [ "$first_character" != '{' ] || { command -v python3 >/dev/null 2>&1 \
    && ! python3 -c 'import json, sys; sys.exit(0 if isinstance(json.load(open(sys.argv[1])), dict) else 1)' "$config" >/dev/null 2>&1; }; then
    fail "The MayaFlux report configuration is not a readable JSON object: $config"
fi

magic="$(od -An -tx1 -N4 "$executable_path" 2>/dev/null | tr -d ' \n')"
case "$magic" in
    2321*|7f454c46) ;;
    *) fail "The project executable could not be started: $executable_path is not a Linux program or script." ;;
esac

console_log="$project/mayaflux_run.txt"
application_log="$project/data/log/mayaflux_report.log"
suffix="$(date -u +%Y%m%d-%H%M%S)-$$-$RANDOM$RANDOM"
save_previous "$application_log" "$suffix"
save_previous "$console_log" "$suffix"
: >"$console_log" || fail "Could not write the console log: $console_log"

{
    printf 'Executable: %s\n' "$executable_path"
    printf 'Project folder: %s\n' "$project"
    printf 'Report configuration: %s\n' "$config"
} | tee -a "$console_log"
printf 'Reproduce the problem, then close the application to finish collecting logs.\n'

( cd "$project" && exec "$executable_path" --config "$config" ) 2>&1 | tee -a "$console_log"
code="${PIPESTATUS[0]}"
if [ "$code" -eq 126 ] || [ "$code" -eq 127 ]; then
    fail 'The project executable could not be started.'
fi
printf 'Application exit code: %s\n' "$code" | tee -a "$console_log"
finish "$code"
