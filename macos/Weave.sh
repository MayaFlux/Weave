#!/usr/bin/env zsh
# Weave - MayaFlux Project Creator
# This script creates new MayaFlux projects from templates

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

BREW_CMD=""
for brew_path in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    if [ -f "$brew_path" ] && [ -x "$brew_path" ]; then
        BREW_CMD="$brew_path"
        break
    fi
done

if [ -z "${MAYAFLUX_ROOT:-}" ]; then
    if [ -n "$BREW_CMD" ]; then
        MAYAFLUX_ROOT=$("$BREW_CMD" --prefix mayaflux-dev)
    else
        echo "[Weave ERROR] MAYAFLUX_ROOT not set and Homebrew not found. Please set MAYAFLUX_ROOT environment variable to your MayaFlux installation location."
        exit 1
    fi
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -d "$SCRIPT_DIR/templates" ]; then
    TEMPLATES_DIR="$SCRIPT_DIR/templates"
elif [ -d "$HOME/.local/share/weave/templates" ]; then
    TEMPLATES_DIR="$HOME/.local/share/weave/templates"
else
    echo "[Weave ERROR] Templates not found. Please reinstall MayaFlux." >&2
    exit 1
fi

# ============================================================================
# UTILITIES
# ============================================================================

usage() {
    cat <<EOF
Weave - MayaFlux Project Creator

Usage:
  weave new <project-name> [destination-dir] [options]
  weave community <module-name> [destination-dir]
  weave --help

Commands:
  new        Create a new MayaFlux project
  community  Create a new community module template

Options (new):
  --with-lila    Enable live coding (links against Lila)
  --no-vscode    Skip VS Code configuration
  --help         Show this help

Environment Variables:
  MAYAFLUX_ROOT  Override MayaFlux installation location
                 (default: determined via Homebrew)
EOF
    exit 0
}

error() {
    echo "[Weave ERROR] $*" >&2
    exit 1
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

    local PROJECT_DIR="$DEST_DIR/$PROJECT_NAME"

    # ============================================================================
    # CHECK IF PROJECT EXISTS
    # ============================================================================

    if [ -d "$PROJECT_DIR" ]; then
        if [ -f "$PROJECT_DIR/CMakeLists.txt" ] && grep -q "MayaFlux" "$PROJECT_DIR/CMakeLists.txt" 2>/dev/null; then
            echo "[Weave] Project '$PROJECT_NAME' already exists and is a MayaFlux project."
            echo "Nothing to do."
            exit 0
        else
            error "Directory '$PROJECT_DIR' already exists but is not a MayaFlux project"
        fi
    fi

    # ============================================================================
    # CHECK TEMPLATES DIRECTORY
    # ============================================================================

    if [ ! -f "$TEMPLATES_DIR/CMakeLists.txt" ]; then
        error "CMakeLists.txt template not found"
    fi
    if [ ! -f "$TEMPLATES_DIR/cmake/shaders.cmake" ]; then
        error "cmake/shaders.cmake template not found"
    fi
    if [ ! -f "$TEMPLATES_DIR/cmake/build_community.cmake" ]; then
        error "cmake/build_community.cmake template not found"
    fi

    # ============================================================================
    # CREATE PROJECT STRUCTURE
    # ============================================================================

    echo "[Weave] Creating project: $PROJECT_NAME"
    mkdir -p "$PROJECT_DIR/src"
    mkdir -p "$PROJECT_DIR/cmake"

    # ============================================================================
    # GENERATE CMakeLists.txt
    # ============================================================================

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

    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/CMakeLists.txt" |
        sed "s|@LILA_LINK_BLOCK@|$LILA_LINK_BLOCK|g" |
        sed "s|@LILA_DEBUGGER_PATH@||g" |
        sed "s|@LILA_DLL_COPY@||g" \
            >"$PROJECT_DIR/CMakeLists.txt"
    echo "✅ CMakeLists.txt generated"

    # ============================================================================
    # COPY cmake modules
    # ============================================================================

    cp "$TEMPLATES_DIR/cmake/shaders.cmake" "$PROJECT_DIR/cmake/shaders.cmake"
    echo "✅ cmake/shaders.cmake copied"
    cp "$TEMPLATES_DIR/cmake/build_community.cmake" "$PROJECT_DIR/cmake/build_community.cmake"
    echo "✅ cmake/build_community.cmake copied"

    # ============================================================================
    # COPY main.cpp
    # ============================================================================

    if [ ! -f "$TEMPLATES_DIR/main.cpp" ]; then
        error "main.cpp template not found at $TEMPLATES_DIR/main.cpp"
    fi

    cp "$TEMPLATES_DIR/main.cpp" "$PROJECT_DIR/src/main.cpp"
    echo "✅ main.cpp created"

    # ============================================================================
    # COPY user_project.hpp
    # ============================================================================

    if [ ! -f "$TEMPLATES_DIR/user_project.hpp" ]; then
        error "user_project.hpp template not found at $TEMPLATES_DIR/user_project.hpp"
    fi

    cp "$TEMPLATES_DIR/user_project.hpp" "$PROJECT_DIR/src/user_project.hpp"
    echo "✅ user_project.hpp created"

    # ============================================================================
    # CREATE data/shaders AND COPY TEMPLATE SHADERS
    # ============================================================================

    mkdir -p "$PROJECT_DIR/data/shaders"

    if [ -d "$TEMPLATES_DIR/shaders" ] && [ -n "$(ls -A "$TEMPLATES_DIR/shaders" 2>/dev/null)" ]; then
        cp "$TEMPLATES_DIR/shaders/"* "$PROJECT_DIR/data/shaders/"
        echo "  ✓ Copied template shaders"
    else
        echo "  ✓ Created empty data/shaders (no template shaders)"
    fi

    # ============================================================================
    # CREATE VS CODE CONFIGURATION (if enabled)
    # ============================================================================

    if [ "$WITH_VSCODE" = true ]; then
        mkdir -p "$PROJECT_DIR/.vscode"

        # Check if VS Code templates exist
        if [ -d "$TEMPLATES_DIR/vscode" ]; then
            # Use templates with substitution
            for vscode_file in settings.json tasks.json launch.json; do
                if [ -f "$TEMPLATES_DIR/vscode/${vscode_file}" ]; then
                    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" \
                        "$TEMPLATES_DIR/vscode/${vscode_file}" \
                        >"$PROJECT_DIR/.vscode/$vscode_file"
                fi
            done
            echo "✅ VS Code configuration created"
        else
            echo "⚠️  VS Code templates not found, skipping"
        fi
    fi

    cp "$TEMPLATES_DIR/.gitignore" "$PROJECT_DIR/.gitignore"
    echo "✅ .gitignore copied"

    # ============================================================================
    # CREATE README
    # ============================================================================

    cat >"$PROJECT_DIR/README.md" <<EOF
# $PROJECT_NAME

A MayaFlux multimedia DSP project.

## Building

\`\`\`bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
\`\`\`

## Running

\`\`\`bash
./build/$PROJECT_NAME
\`\`\`

## Editing

Open in VS Code:
\`\`\`bash
code .
\`\`\`

Edit your code in \`src/user_project.hpp\`:
- \`settings()\`: Configure sample rate, buffer size, graphics
- \`compose()\`: Create your nodes, buffers, and processing chains

## Documentation

See [MayaFlux Documentation](https://github.com/MayaFlux/MayaFlux)
EOF

    echo "✅ README.md created"

    # ============================================================================
    # SUMMARY
    # ============================================================================

    echo ""
    echo "=========================================="
    echo "  Project '$PROJECT_NAME' created!"
    echo "=========================================="
    echo ""
    echo "Location: $PROJECT_DIR"
    echo ""
    echo "Next steps:"
    echo "  cd $PROJECT_DIR"
    if [ "$WITH_VSCODE" = true ]; then
        echo "  code .                              # Open in VS Code"
    fi
    echo "  mkdir build && cd build"
    echo "  cmake .. && make"
    echo "  ./$PROJECT_NAME"
    echo ""
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

    [ ! -f "$TEMPLATES_DIR/community/module.cmake" ] && error "Required template missing: community/module.cmake"
    [ ! -f "$TEMPLATES_DIR/community/community.json" ] && error "Required template missing: community/community.json"
    [ ! -f "$TEMPLATES_DIR/community/test/CMakeLists.txt" ] && error "Required template missing: community/test/CMakeLists.txt"

    mkdir -p "$MODULE_DIR/src"
    mkdir -p "$MODULE_DIR/test"

    cp "$TEMPLATES_DIR/.gitignore" "$MODULE_DIR/.gitignore"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/module.cmake" \
        >"$MODULE_DIR/${MODULE_NAME}.cmake"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/community.json" \
        >"$MODULE_DIR/community.json"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/test/CMakeLists.txt" \
        >"$MODULE_DIR/test/CMakeLists.txt"

    echo ""
    echo "[Weave] Community module created: $MODULE_DIR"
    echo ""
    echo "  src/           <- put your sources here"
    echo "  test/CMakeLists.txt"
    echo "  ${MODULE_NAME}.cmake"
    echo "  community.json"
    echo ""
    echo "Test with:"
    echo "  test/test_${MODULE_NAME}.cpp  <- create this to build the test"
    echo "  cd $MODULE_DIR/test && mkdir build && cd build"
    echo "  cmake .. && cmake --build . --parallel"
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
community) cmd_community "$@" ;;
*) error "Unknown command: $CMD. Use 'weave --help'" ;;
esac
