#!/bin/bash
# Weave - MayaFlux Project Creator for Linux

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

MAYAFLUX_ROOT_OVERRIDE="${MAYAFLUX_ROOT:-}"
MAYAFLUX_ROOT="${MAYAFLUX_ROOT:-/usr/local}"
REGISTRY_URL="https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json"

if [ -n "${WEAVE_TEMPLATE_DIR:-}" ] && [ -d "$WEAVE_TEMPLATE_DIR" ]; then
    TEMPLATES_DIR="$WEAVE_TEMPLATE_DIR"
elif [ -d "$HOME/.local/share/weave/templates" ]; then
    TEMPLATES_DIR="$HOME/.local/share/weave/templates"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    TEMPLATES_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")/templates"
fi

# ============================================================================
# UTILITIES
# ============================================================================

usage() {
    cat <<EOF
Weave - MayaFlux Project and Community Tool

Usage:
  weave new <project-name> [destination-dir] [options]
  weave update <project-dir> <module-name> [module-name ...]
  weave community <module-name> [destination-dir]
  weave --help

Commands:
  new        Create a new MayaFlux project
  update     Acquire and add community modules to an existing project
  community  Create a new community module template

Options (new):
  --with-lila    Enable live coding (links against Lila)
  --no-vscode    Skip VS Code configuration
  --copy-shader-sources  Copy installed MayaFlux shader sources into the project

Environment Variables:
  MAYAFLUX_ROOT  Override MayaFlux installation location (default: /usr/local)
EOF
    exit 0
}

error() {
    echo "[Weave ERROR] $*" >&2
    exit 1
}

log() {
    echo "[Weave] $*"
}

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || error "Required command not found: $1"
}

_version_gte() {
    local IFS=.
    local a=($1) b=($2)
    local i
    for i in 0 1 2; do
        local av=${a[$i]:-0} bv=${b[$i]:-0}
        if [ "$av" -gt "$bv" ]; then return 0; fi
        if [ "$av" -lt "$bv" ]; then return 1; fi
    done
    return 0
}

_registry_lookup() {
    local registry="$1" module="$2"
    if command -v jq >/dev/null 2>&1; then
        local entry
        entry="$(echo "$registry" | jq -r --arg n "$module" '.[] | select(.name==$n) | "\(.repo) \(.min_version)"')"
        if [ -z "$entry" ]; then
            echo "[Weave ERROR] Module '$module' not found in registry" >&2
            return 1
        fi
        echo "$entry"
    else
        local py
        py="$(command -v python3)"
        PYTHONHOME="" PYTHONPATH="" "$py" -c '
import json, sys
name = sys.argv[1]
reg = json.load(sys.stdin)
entry = next((e for e in reg if e["name"] == name), None)
if not entry:
    sys.stderr.write(f"[Weave ERROR] Module {name} not found in registry\n")
    sys.exit(1)
print(entry["repo"], entry["min_version"])
' "$module" <<<"$registry"
    fi
}

# ============================================================================
# CMD: new
# ============================================================================

cmd_new() {
    local PROJECT_NAME="${1:-}"
    local DEST_DIR='.'
    local WITH_LILA=false
    local WITH_VSCODE=true
    local COPY_SHADER_SOURCES=false
    local SHADER_SOURCE_DIR=''
    local -a SHADER_SOURCES=()

    shift || true
    if [ "$#" -gt 0 ] && [[ "$1" != --* ]]; then
        DEST_DIR="$1"
        shift
    fi

    while [ "$#" -gt 0 ]; do
        case "$1" in
        --with-lila) WITH_LILA=true ;;
        --no-vscode) WITH_VSCODE=false ;;
        --copy-shader-sources) COPY_SHADER_SOURCES=true ;;
        *) error "Unknown option: $1" ;;
        esac
        shift
    done

    [ -z "$PROJECT_NAME" ] && error "Project name required"
    [[ "$PROJECT_NAME" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || error "Project name must start with a letter or underscore and contain only letters, digits, or underscores"
    case "${PROJECT_NAME^^}" in
        CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9]) error "Project name is reserved on Windows: $PROJECT_NAME" ;;
    esac

    DEST_DIR="${DEST_DIR/#\~/$HOME}"
    [[ "$DEST_DIR" = /* ]] || DEST_DIR="$(pwd -P)/$DEST_DIR"
    [ -d "$DEST_DIR" ] || error "Project destination must be an existing directory: $DEST_DIR"
    local current='' component
    local -a components=()
    IFS='/' read -r -a components <<< "$DEST_DIR"
    for component in "${components[@]}"; do
        [ -z "$component" ] && continue
        current="$current/$component"
        [ -L "$current" ] && error "Project destination contains a symbolic link: $current"
    done
    DEST_DIR="$(cd -P "$DEST_DIR" && pwd)"

    local PROJECT_DIR="$DEST_DIR/$PROJECT_NAME"

    if [ -e "$PROJECT_DIR" ] || [ -L "$PROJECT_DIR" ]; then
        error "Project destination already exists: $PROJECT_DIR"
    fi

    [ ! -d "$TEMPLATES_DIR" ] && error "Templates not found at $TEMPLATES_DIR"

    for template in CMakeLists.txt CMakePresets.json main.cpp user_project.hpp validate.sh run-report.sh .gitignore; do
        [ ! -f "$TEMPLATES_DIR/$template" ] && error "Required template missing: $template"
    done

    [ ! -f "$TEMPLATES_DIR/cmake/mayaflux.cmake" ] && error "Required template missing: cmake/mayaflux.cmake"
    [ ! -f "$TEMPLATES_DIR/cmake/shaders.cmake" ] && error "Required template missing: cmake/shaders.cmake"
    [ ! -f "$TEMPLATES_DIR/cmake/build_community.cmake" ] && error "Required template missing: cmake/build_community.cmake"

    if [ "$COPY_SHADER_SOURCES" = true ]; then
        if [ -n "$MAYAFLUX_ROOT_OVERRIDE" ]; then
            SHADER_SOURCE_DIR="$MAYAFLUX_ROOT_OVERRIDE/share/MayaFlux/src/shaders"
        else
            for candidate in /usr/share/MayaFlux/src/shaders /usr/local/share/MayaFlux/src/shaders; do
                if [ -d "$candidate" ]; then
                    SHADER_SOURCE_DIR="$candidate"
                    break
                fi
            done
        fi
        [ -z "$SHADER_SOURCE_DIR" ] && error 'MayaFlux shader sources were not found under /usr or /usr/local. Set MAYAFLUX_ROOT to the installation prefix, or omit --copy-shader-sources.'
        [ ! -d "$SHADER_SOURCE_DIR" ] && error "MayaFlux shader sources were not found at $SHADER_SOURCE_DIR. Update MayaFlux or omit --copy-shader-sources."
        [ -L "$SHADER_SOURCE_DIR" ] && error "MayaFlux shader source directory is a symbolic link: $SHADER_SOURCE_DIR"
        [ -n "$(find "$SHADER_SOURCE_DIR" -type l -print -quit)" ] && error "MayaFlux shader sources contain a symbolic link: $SHADER_SOURCE_DIR"
        mapfile -d '' -t SHADER_SOURCES < <(
            find "$SHADER_SOURCE_DIR" -type f \( \
                -iname '*.glsl' -o -iname '*.vert' -o -iname '*.frag' -o -iname '*.comp' -o \
                -iname '*.geom' -o -iname '*.tesc' -o -iname '*.tese' -o -iname '*.mesh' -o \
                -iname '*.task' -o -iname '*.rgen' -o -iname '*.rchit' -o -iname '*.rahit' -o \
                -iname '*.rmiss' -o -iname '*.rint' -o -iname '*.rcall' \
            \) -print0
        )
        [ -z "${SHADER_SOURCES[0]-}" ] && error "MayaFlux shader sources are empty at $SHADER_SOURCE_DIR. Update MayaFlux or omit --copy-shader-sources."
    fi

    log "Creating project: $PROJECT_NAME"
    mkdir "$PROJECT_DIR"
    CREATED_PROJECT_DIR="$PROJECT_DIR"
    trap 'status=$?; if [ "$status" -ne 0 ] && [ -d "$CREATED_PROJECT_DIR" ]; then rm -rf -- "$CREATED_PROJECT_DIR" || printf "[Weave ERROR] Incomplete project could not be removed: %s\n" "$CREATED_PROJECT_DIR" >&2; fi' EXIT
    mkdir "$PROJECT_DIR/src"
    mkdir -p "$PROJECT_DIR/cmake"
    [ "$WITH_VSCODE" = true ] && mkdir -p "$PROJECT_DIR/.vscode"

    if [ "$WITH_LILA" = true ]; then
        LILA_LINK_BLOCK='target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::MayaFluxHost)'
        LILA_DEBUGGER_PATH='$<TARGET_FILE_DIR:MayaFlux::MayaFluxHost>;'
        LILA_DLL_COPY='if(EXISTS "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll")
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll"
                $<TARGET_FILE_DIR:${PROJECT_NAME}>
        )
    endif()'
    else
        LILA_LINK_BLOCK=''
        LILA_DEBUGGER_PATH=''
        LILA_DLL_COPY=''
    fi

    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/CMakeLists.txt" >"$PROJECT_DIR/CMakeLists.txt"
    log "  ✓ Generated CMakeLists.txt"

    {
        awk -v lila="$LILA_LINK_BLOCK" '{gsub(/@LILA_LINK_BLOCK@/, lila); print}' "$TEMPLATES_DIR/cmake/mayaflux.cmake" |
            awk -v v="$LILA_DEBUGGER_PATH" '{gsub(/@LILA_DEBUGGER_PATH@/, v); print}' |
            awk -v v="$LILA_DLL_COPY" '{gsub(/@LILA_DLL_COPY@/, v); print}'
    } >"$PROJECT_DIR/cmake/mayaflux.cmake"
    log "  ✓ Generated cmake/mayaflux.cmake"

    cp "$TEMPLATES_DIR/cmake/shaders.cmake" "$PROJECT_DIR/cmake/shaders.cmake"
    log "  ✓ Copied cmake/shaders.cmake"
    cp "$TEMPLATES_DIR/cmake/build_community.cmake" "$PROJECT_DIR/cmake/build_community.cmake"
    log "  ✓ Copied cmake/build_community.cmake"

    cp "$TEMPLATES_DIR/CMakePresets.json" "$PROJECT_DIR/CMakePresets.json"
    log "  ✓ Copied CMakePresets.json"

    cp "$TEMPLATES_DIR/validate.sh" "$PROJECT_DIR/validate.sh"
    chmod +x "$PROJECT_DIR/validate.sh"
    log "  ✓ Copied validate.sh"

    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/run-report.sh" >"$PROJECT_DIR/run-report.sh"
    chmod +x "$PROJECT_DIR/run-report.sh"
    log "  ✓ Generated run-report.sh"

    cp "$TEMPLATES_DIR/main.cpp" "$PROJECT_DIR/src/main.cpp"
    log "  ✓ Generated main.cpp"

    cp "$TEMPLATES_DIR/user_project.hpp" "$PROJECT_DIR/src/user_project.hpp"
    log "  ✓ Generated user_project.hpp"

    mkdir -p "$PROJECT_DIR/data/shaders"
    if [ -d "$TEMPLATES_DIR/shaders" ] && [ -n "$(ls -A "$TEMPLATES_DIR/shaders" 2>/dev/null)" ]; then
        cp "$TEMPLATES_DIR/shaders/"* "$PROJECT_DIR/data/shaders/"
        log "  ✓ Copied template shaders"
    fi

    if [ "$COPY_SHADER_SOURCES" = true ]; then
        local shader relative destination
        for shader in "${SHADER_SOURCES[@]}"; do
            relative="${shader#"$SHADER_SOURCE_DIR"/}"
            destination="$PROJECT_DIR/data/shaders/$relative"
            if [ -e "$destination" ]; then
                log "  ✓ Preserved project shader: $relative"
                continue
            fi
            mkdir -p "$(dirname "$destination")"
            cp "$shader" "$destination"
            log "  ✓ Copied MayaFlux shader source: $relative"
        done
    fi

    touch "$PROJECT_DIR/community.cmake"
    log "  ✓ Created community.cmake"

    if [ "$WITH_VSCODE" = true ] && [ -d "$TEMPLATES_DIR/vscode" ]; then
        for vscode_file in settings.json tasks.json launch.json; do
            if [ -f "$TEMPLATES_DIR/vscode/$vscode_file" ]; then
                sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" \
                    "$TEMPLATES_DIR/vscode/$vscode_file" \
                    >"$PROJECT_DIR/.vscode/$vscode_file"
            fi
        done
        log "  ✓ Generated VS Code configuration"
    fi

    cp "$TEMPLATES_DIR/.gitignore" "$PROJECT_DIR/.gitignore"
    log "  ✓ Copied .gitignore"

    cat >"$PROJECT_DIR/README.md" <<EOF
# $PROJECT_NAME

A MayaFlux multimedia DSP project.

## Troubleshooting

If setup or a build fails, run \`./validate.sh\` from this project folder to check the installed MayaFlux package, its dependencies, and project files. Copy its report when requesting help.

## Reporting a problem

Build the project, then run \`./run-report.sh\`. It runs the project with MayaFlux's report configuration. Reproduce the problem, then close the application. Attach \`mayaflux_run.txt\` and \`data/log/mayaflux_report.log\` to the issue, together with the output of \`./validate.sh\`.

## Building

\`\`\`bash
cmake --preset release
cmake --build --preset release --parallel
\`\`\`
EOF
    log "  ✓ Generated README.md"

    log ""
    log "Project created: $PROJECT_DIR"
    log "Build from the project directory with the release preset:"
    log "  cmake --preset release"
    log "  cmake --build --preset release --parallel"
    trap - EXIT
    unset CREATED_PROJECT_DIR
}

# ============================================================================
# CMD: update
# ============================================================================

cmd_update() {
    local PROJECT_DIR="${1:-}"
    shift || true

    [ -z "$PROJECT_DIR" ] && error "Project directory required: weave update <project-dir> <module-name> ..."
    [ "$#" -eq 0 ] && error "At least one module name required: weave update <project-dir> <module-name> ..."

    PROJECT_DIR="${PROJECT_DIR/#\~/$HOME}"
    PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"

    [ ! -f "$PROJECT_DIR/CMakeLists.txt" ] && error "Not a MayaFlux project: $PROJECT_DIR"

    require_cmd curl
    require_cmd git

    log "Fetching registry..."
    local REGISTRY
    REGISTRY="$(curl -fsSL "$REGISTRY_URL")" || error "Failed to fetch registry from $REGISTRY_URL"

    local COMMUNITY_DIR="$PROJECT_DIR/community"
    mkdir -p "$COMMUNITY_DIR"

    local COMMUNITY_CMAKE="$PROJECT_DIR/community.cmake"
    touch "$COMMUNITY_CMAKE"

    for MODULE_NAME in "$@"; do
        log "Looking up module: $MODULE_NAME"

        local REPO MIN_VERSION
        read -r REPO MIN_VERSION < <(_registry_lookup "$REGISTRY" "$MODULE_NAME") || exit 1

        local MF_VERSION_FILE="$MAYAFLUX_ROOT/lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake"
        local MF_VERSION=""
        if [ -f "$MF_VERSION_FILE" ]; then
            MF_VERSION="$(grep 'set(PACKAGE_VERSION ' "$MF_VERSION_FILE" | sed 's/.*"\(.*\)".*/\1/')"
        fi

        if [ -n "$MF_VERSION" ] && [ -n "$MIN_VERSION" ]; then
            if ! _version_gte "$MF_VERSION" "$MIN_VERSION"; then
                error "Module $MODULE_NAME requires MayaFlux >= $MIN_VERSION, found $MF_VERSION"
            fi
        fi

        local MODULE_DIR="$COMMUNITY_DIR/$MODULE_NAME"

        if [ -d "$MODULE_DIR" ]; then
            log "  $MODULE_NAME already present, skipping clone"
        else
            log "  Cloning $REPO..."
            git clone --depth=1 "$REPO" "$MODULE_DIR" >/dev/null 2>&1 || error "Failed to clone $REPO"
            log "  ✓ Cloned into community/$MODULE_NAME"
        fi

        [ ! -f "$MODULE_DIR/${MODULE_NAME}.cmake" ] && error "Module '$MODULE_NAME' is missing ${MODULE_NAME}.cmake"
        [ ! -d "$MODULE_DIR/src" ] && error "Module '$MODULE_NAME' is missing src/"

        if ! grep -qxF "$MODULE_NAME" "$COMMUNITY_CMAKE"; then
            echo "$MODULE_NAME" >>"$COMMUNITY_CMAKE"
            log "  ✓ Added $MODULE_NAME to community.cmake"
        else
            log "  ✓ $MODULE_NAME already in community.cmake"
        fi
    done

    log ""
    log "Done. Rebuild your project to include the new modules."
}

# ============================================================================
# CMD: community
# ============================================================================

cmd_community() {
    local MODULE_NAME="${1:-}"
    local DEST_DIR="${2:-.}"

    [ -z "$MODULE_NAME" ] && error "Module name required: weave community <module-name> [destination-dir]"

    echo "$MODULE_NAME" | grep -qE '^[a-z][a-z0-9_]*$' ||
        error "Module name must be snake_case (lowercase letters, digits, underscores, no leading digit)"

    DEST_DIR="${DEST_DIR/#\~/$HOME}"
    [[ "$DEST_DIR" = /* ]] || DEST_DIR="$(pwd -P)/$DEST_DIR"
    [ -d "$DEST_DIR" ] || error "Module destination must be an existing directory: $DEST_DIR"
    local current='' component
    local -a components=()
    IFS='/' read -r -a components <<< "$DEST_DIR"
    for component in "${components[@]}"; do
        [ -z "$component" ] && continue
        current="$current/$component"
        [ -L "$current" ] && error "Module destination contains a symbolic link: $current"
    done
    DEST_DIR="$(cd -P "$DEST_DIR" && pwd)"

    local MODULE_DIR="$DEST_DIR/$MODULE_NAME"
    if [ -e "$MODULE_DIR" ] || [ -L "$MODULE_DIR" ]; then
        error "Module destination already exists: $MODULE_DIR"
    fi

    [ ! -f "$TEMPLATES_DIR/.gitignore" ] && error "Required template missing: .gitignore"
    [ ! -f "$TEMPLATES_DIR/community/CMakePresets.json" ] && error "Required template missing: community/CMakePresets.json"
    [ ! -f "$TEMPLATES_DIR/community/module.cmake" ] && error "Required template missing: community/module.cmake"
    [ ! -f "$TEMPLATES_DIR/community/community.json" ] && error "Required template missing: community/community.json"
    [ ! -f "$TEMPLATES_DIR/community/test/CMakeLists.txt" ] && error "Required template missing: community/test/CMakeLists.txt"

    mkdir "$MODULE_DIR"
    CREATED_MODULE_DIR="$MODULE_DIR"
    trap 'status=$?; if [ "$status" -ne 0 ] && [ -d "$CREATED_MODULE_DIR" ]; then rm -rf -- "$CREATED_MODULE_DIR" || printf "[Weave ERROR] Incomplete module could not be removed: %s\n" "$CREATED_MODULE_DIR" >&2; fi' EXIT
    mkdir "$MODULE_DIR/src"
    mkdir "$MODULE_DIR/test"

    cp "$TEMPLATES_DIR/.gitignore" "$MODULE_DIR/.gitignore"
    cp "$TEMPLATES_DIR/community/CMakePresets.json" "$MODULE_DIR/CMakePresets.json"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/module.cmake" \
        >"$MODULE_DIR/${MODULE_NAME}.cmake"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/community.json" \
        >"$MODULE_DIR/community.json"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/test/CMakeLists.txt" \
        >"$MODULE_DIR/test/CMakeLists.txt"

    touch "$MODULE_DIR/test/test_${MODULE_NAME}.cpp"
    trap - EXIT
    unset CREATED_MODULE_DIR
    if command -v git >/dev/null 2>&1; then
        if git -C "$MODULE_DIR" init -q -b main; then
            git -C "$MODULE_DIR" add -A || log "  WARNING: Git could not stage module files. Run git add -A in $MODULE_DIR when ready."
        else
            log "  WARNING: Git could not initialize the module repository. Run git init in $MODULE_DIR when ready."
        fi
    else
        log "  WARNING: Git is unavailable. Initialize a repository in $MODULE_DIR when ready."
    fi

    log ""
    log "[Weave] Community module created: $MODULE_DIR"
    log ""
    log "  src/           <- put your sources here"
    log "  test/CMakeLists.txt"
    log "  test/test_${MODULE_NAME}.cpp"
    log "  ${MODULE_NAME}.cmake"
    log "  community.json"
    log ""
    log "Build test:"
    log "  cmake -G Ninja -B test/build -S test/"
    log "  cmake --build test/build --parallel"
}

# ============================================================================
# ENTRY POINT
# ============================================================================

[ "$#" -eq 0 ] && usage
[ "$1" = "--help" ] || [ "$1" = "-h" ] && usage

CMD="$1"
shift

case "$CMD" in
new) cmd_new "$@" ;;
update) cmd_update "$@" ;;
community) cmd_community "$@" ;;
*) error "Unknown command: $CMD. Use 'weave --help'" ;;
esac
