#!/bin/bash
# Weave - MayaFlux Project Creator for Linux

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

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

# ============================================================================
# CMD: new
# ============================================================================

cmd_new() {
    local PROJECT_NAME="${1:-}"
    local DEST_DIR="${2:-.}"
    local WITH_LILA=false
    local WITH_VSCODE=true

    shift 2 2>/dev/null || true

    while [ "$#" -gt 0 ]; do
        case "$1" in
        --with-lila) WITH_LILA=true ;;
        --no-vscode) WITH_VSCODE=false ;;
        *) error "Unknown option: $1" ;;
        esac
        shift
    done

    [ -z "$PROJECT_NAME" ] && error "Project name required"

    DEST_DIR="${DEST_DIR/#\~/$HOME}"
    mkdir -p "$DEST_DIR" || error "Cannot create destination directory: $DEST_DIR"
    DEST_DIR="$(cd "$DEST_DIR" && pwd)"

    local PROJECT_DIR="$DEST_DIR/$PROJECT_NAME"

    if [ -d "$PROJECT_DIR" ]; then
        if [ -f "$PROJECT_DIR/CMakeLists.txt" ] && grep -q "MayaFlux" "$PROJECT_DIR/CMakeLists.txt" 2>/dev/null; then
            log "Project '$PROJECT_NAME' already exists and is a MayaFlux project."
            log "Nothing to do."
            exit 0
        else
            error "Directory already exists and is not a MayaFlux project: $PROJECT_DIR"
        fi
    fi

    [ ! -d "$TEMPLATES_DIR" ] && error "Templates not found at $TEMPLATES_DIR"

    for template in CMakeLists.txt main.cpp user_project.hpp; do
        [ ! -f "$TEMPLATES_DIR/$template" ] && error "Required template missing: $template"
    done

    [ ! -f "$TEMPLATES_DIR/cmake/shaders.cmake" ] && error "Required template missing: cmake/shaders.cmake"
    [ ! -f "$TEMPLATES_DIR/cmake/build_community.cmake" ] && error "Required template missing: cmake/build_community.cmake"

    log "Creating project: $PROJECT_NAME"
    mkdir -p "$PROJECT_DIR/src"
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

    {
        sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/CMakeLists.txt" |
            awk -v lila="$LILA_LINK_BLOCK" '{gsub(/@LILA_LINK_BLOCK@/, lila); print}' |
            awk -v v="$LILA_DEBUGGER_PATH" '{gsub(/@LILA_DEBUGGER_PATH@/, v); print}' |
            awk -v v="$LILA_DLL_COPY" '{gsub(/@LILA_DLL_COPY@/, v); print}'
    } >"$PROJECT_DIR/CMakeLists.txt"
    log "  ✓ Generated CMakeLists.txt"

    cp "$TEMPLATES_DIR/cmake/shaders.cmake" "$PROJECT_DIR/cmake/shaders.cmake"
    log "  ✓ Copied cmake/shaders.cmake"
    cp "$TEMPLATES_DIR/cmake/build_community.cmake" "$PROJECT_DIR/cmake/build_community.cmake"
    log "  ✓ Copied cmake/build_community.cmake"

    cp "$TEMPLATES_DIR/main.cpp" "$PROJECT_DIR/src/main.cpp"
    log "  ✓ Generated main.cpp"

    cp "$TEMPLATES_DIR/user_project.hpp" "$PROJECT_DIR/src/user_project.hpp"
    log "  ✓ Generated user_project.hpp"

    mkdir -p "$PROJECT_DIR/data/shaders"
    if [ -d "$TEMPLATES_DIR/shaders" ] && [ -n "$(ls -A "$TEMPLATES_DIR/shaders" 2>/dev/null)" ]; then
        cp "$TEMPLATES_DIR/shaders/"* "$PROJECT_DIR/data/shaders/"
        log "  ✓ Copied template shaders"
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

    cat >"$PROJECT_DIR/.gitignore" <<'EOF'
/build/
/cmake-build-*/
.vscode/
.idea/
*.swp
*.swo
compile_commands.json
CMakeCache.txt
CMakeFiles/
cmake_install.cmake
*.o
*.so
*.a
*.lib
*.exe
.DS_Store
__pycache__/
*.pyc
EOF
    log "  ✓ Generated .gitignore"

    cat >"$PROJECT_DIR/README.md" <<EOF
# $PROJECT_NAME

A MayaFlux multimedia DSP project.

## Building

\`\`\`bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
\`\`\`
EOF
    log "  ✓ Generated README.md"

    log ""
    log "Project created: $PROJECT_DIR"
    log "Build with:"
    log "  cd $PROJECT_DIR && mkdir build && cd build"
    log "  cmake .. -DCMAKE_BUILD_TYPE=Release && cmake --build . --parallel"
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

    for MODULE_NAME in "$@"; do
        log "Acquiring module: $MODULE_NAME"

        local REPO
        REPO="$(echo "$REGISTRY" | python3 -c "
import json, sys
reg = json.load(sys.stdin)
entry = next((e for e in reg if e['name'] == '${MODULE_NAME}'), None)
if not entry:
    sys.stderr.write(\"Module '${MODULE_NAME}' not found in registry\n\")
    sys.exit(1)
print(entry['repo'])
")" || error "Module '$MODULE_NAME' not found in registry"

        local MIN_VERSION
        MIN_VERSION="$(echo "$REGISTRY" | python3 -c "
import json, sys
reg = json.load(sys.stdin)
entry = next((e for e in reg if e['name'] == '${MODULE_NAME}'), None)
print(entry['min_version'])
")"

        if [ -f "$MAYAFLUX_ROOT/lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake" ]; then
            local MF_VERSION
            MF_VERSION="$(grep 'set(PACKAGE_VERSION ' "$MAYAFLUX_ROOT/lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake" | sed 's/.*"\(.*\)".*/\1/')"
            python3 -c "
import sys
def parse(v):
    return tuple(int(x) for x in v.split('.'))
if parse('$MF_VERSION') < parse('$MIN_VERSION'):
    sys.stderr.write('[Weave ERROR] Module $MODULE_NAME requires MayaFlux >= $MIN_VERSION, found $MF_VERSION\n')
    sys.exit(1)
" || exit 1
        fi

        local TMP_DIR
        TMP_DIR="$(mktemp -d)"

        log "  Cloning $REPO..."
        git clone --depth=1 "$REPO" "$TMP_DIR" >/dev/null 2>&1 || error "Failed to clone $REPO"

        [ ! -f "$TMP_DIR/${MODULE_NAME}.cmake" ] && error "Module '$MODULE_NAME' is missing ${MODULE_NAME}.cmake"
        [ ! -d "$TMP_DIR/src" ] && error "Module '$MODULE_NAME' is missing src/"

        local MODULE_DIR="$COMMUNITY_DIR/$MODULE_NAME"
        rm -rf "$MODULE_DIR"
        mkdir -p "$MODULE_DIR"
        cp "$TMP_DIR/${MODULE_NAME}.cmake" "$MODULE_DIR/"
        cp -r "$TMP_DIR/src" "$MODULE_DIR/"
        rm -rf "$TMP_DIR"

        log "  ✓ Acquired $MODULE_NAME"

        local COMMUNITY_CMAKE="$PROJECT_DIR/community.cmake"
        touch "$COMMUNITY_CMAKE"
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
    mkdir -p "$DEST_DIR" || error "Cannot create destination directory: $DEST_DIR"
    DEST_DIR="$(cd "$DEST_DIR" && pwd)"

    local MODULE_DIR="$DEST_DIR/$MODULE_NAME"
    [ -d "$MODULE_DIR" ] && error "Directory already exists: $MODULE_DIR"

    local CLASS_NAME
    CLASS_NAME="$(echo "$MODULE_NAME" | python3 -c "
import sys
parts = sys.stdin.read().strip().split('_')
print(''.join(p.capitalize() for p in parts))
")"

    mkdir -p "$MODULE_DIR/src"
    mkdir -p "$MODULE_DIR/test"

    cat >"$MODULE_DIR/src/${CLASS_NAME}.hpp" <<EOF
#pragma once

#include "MayaFlux/MayaFlux.hpp"

namespace MayaFlux {

class ${CLASS_NAME} {
public:
    ${CLASS_NAME}() = default;
};

} // namespace MayaFlux
EOF

    cat >"$MODULE_DIR/src/${CLASS_NAME}.cpp" <<EOF
#include "${CLASS_NAME}.hpp"

namespace MayaFlux {

} // namespace MayaFlux
EOF

    cat >"$MODULE_DIR/${MODULE_NAME}.cmake" <<EOF
set(MF_MIN_VERSION "0.3.0")
set(MF_NEEDS_LILA OFF)

file(GLOB_RECURSE _${MODULE_NAME}_sources CONFIGURE_DEPENDS
    "\${CMAKE_SOURCE_DIR}/community/${MODULE_NAME}/src/*.cpp"
)

add_library(${MODULE_NAME} OBJECT
    \${_${MODULE_NAME}_sources}
)

target_include_directories(${MODULE_NAME} PUBLIC
    "\${CMAKE_SOURCE_DIR}/community/${MODULE_NAME}/src"
)

set_target_properties(${MODULE_NAME} PROPERTIES
    CXX_STANDARD 23
    CXX_STANDARD_REQUIRED ON
)

target_link_libraries(${MODULE_NAME} PUBLIC MayaFlux::MayaFluxLib)

if(MF_NEEDS_LILA)
    target_link_libraries(${MODULE_NAME} PUBLIC MayaFlux::Lila MayaFlux::MayaFluxHost)
endif()
EOF

    cat >"$MODULE_DIR/community.json" <<EOF
{
  "name": "${MODULE_NAME}",
  "min_version": "0.3.0",
  "needs_lila": false,
  "licence": "MIT",
  "description": ""
}
EOF

    cat >"$MODULE_DIR/test/CMakeLists.txt" <<EOF
cmake_minimum_required(VERSION 3.25)
project(${MODULE_NAME}_test LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_compile_definitions(MAYAFLUX_PROJECT)

set(MAYAFLUX_SEARCH_PATHS "")
if(DEFINED ENV{MAYAFLUX_ROOT})
    list(APPEND MAYAFLUX_SEARCH_PATHS "\$ENV{MAYAFLUX_ROOT}/lib/cmake/MayaFlux")
    list(APPEND MAYAFLUX_SEARCH_PATHS "\$ENV{MAYAFLUX_ROOT}/lib64/cmake/MayaFlux")
endif()
if(APPLE)
    list(APPEND MAYAFLUX_SEARCH_PATHS "/Library/MayaFlux/lib/cmake/MayaFlux" "\$ENV{HOME}/MayaFlux/lib/cmake/MayaFlux")
elseif(UNIX)
    list(APPEND MAYAFLUX_SEARCH_PATHS "/usr/local/lib/cmake/MayaFlux" "\$ENV{HOME}/MayaFlux/lib/cmake/MayaFlux" "\$ENV{HOME}/.local/lib/cmake/MayaFlux")
elseif(WIN32)
    list(APPEND MAYAFLUX_SEARCH_PATHS "C:/MayaFlux/lib/cmake/MayaFlux" "\$ENV{USERPROFILE}/MayaFlux/lib/cmake/MayaFlux")
endif()

find_package(MayaFlux REQUIRED PATHS \${MAYAFLUX_SEARCH_PATHS} NO_DEFAULT_PATH)

file(GLOB_RECURSE _${MODULE_NAME}_sources CONFIGURE_DEPENDS
    "\${CMAKE_CURRENT_SOURCE_DIR}/../src/*.cpp"
)

add_library(${MODULE_NAME} OBJECT \${_${MODULE_NAME}_sources})
target_include_directories(${MODULE_NAME} PUBLIC "\${CMAKE_CURRENT_SOURCE_DIR}/../src")
set_target_properties(${MODULE_NAME} PROPERTIES CXX_STANDARD 23 CXX_STANDARD_REQUIRED ON)
target_link_libraries(${MODULE_NAME} PUBLIC MayaFlux::MayaFluxLib)

add_executable(test_${MODULE_NAME} test_${MODULE_NAME}.cpp)
target_link_libraries(test_${MODULE_NAME} PRIVATE ${MODULE_NAME})

if(UNIX)
    execute_process(
        COMMAND \${CMAKE_COMMAND} -E create_symlink
        \${CMAKE_BINARY_DIR}/compile_commands.json
        \${CMAKE_CURRENT_SOURCE_DIR}/../compile_commands.json
    )
endif()
EOF

    cat >"$MODULE_DIR/test/test_${MODULE_NAME}.cpp" <<EOF
#include "${CLASS_NAME}.hpp"

int main()
{
    return 0;
}
EOF

    cat >"$MODULE_DIR/.gitignore" <<'EOF'
build/
compile_commands.json
EOF

    log ""
    log "Community module created: $MODULE_DIR"
    log ""
    log "  src/${CLASS_NAME}.hpp"
    log "  src/${CLASS_NAME}.cpp"
    log "  test/CMakeLists.txt"
    log "  test/test_${MODULE_NAME}.cpp"
    log "  ${MODULE_NAME}.cmake"
    log "  community.json"
    log ""
    log "Test with:"
    log "  cd $MODULE_DIR/test && mkdir build && cd build"
    log "  cmake .. && cmake --build . --parallel"
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
