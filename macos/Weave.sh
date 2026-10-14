#!/usr/bin/env zsh
# Weave - MayaFlux Project Creator
# This script creates new MayaFlux projects from templates

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

BREW_CMD="$(command -v brew 2>/dev/null || true)"
if [[ "$BREW_CMD" == /usr/local/* || ! -x "$BREW_CMD" ]]; then
    BREW_CMD=""
fi
if [ -z "$BREW_CMD" ]; then
    for brew_path in /opt/*/bin/brew(N); do
        if [ -x "$brew_path" ]; then
            BREW_CMD="$brew_path"
            break
        fi
    done
fi

if [ -z "${MAYAFLUX_ROOT:-}" ]; then
    if [ -x "$BREW_CMD" ]; then
        MAYAFLUX_ROOT=$("$BREW_CMD" --prefix mayaflux-dev)
    else
        echo "[Weave ERROR] MAYAFLUX_ROOT not set and Homebrew not found. Please set MAYAFLUX_ROOT environment variable to your MayaFlux installation location."
        exit 1
    fi
fi

REGISTRY_URL="https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json"

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
  --help         Show this help

Environment Variables:
  MAYAFLUX_ROOT  Override MayaFlux installation location
                 (default: determined via Homebrew)
EOF
    exit 0
}

error() {
    if [ "${WEAVE_NEW_ENTRY_CREATED:-false}" = true ]; then
        rm -rf -- "$WEAVE_NEW_ENTRY_DIR"
        WEAVE_NEW_ENTRY_CREATED=false
    fi
    echo "[Weave ERROR] $*" >&2
    exit 1
}

_cleanup_new_entry() {
    local exit_code=$?
    if [ "$exit_code" -ne 0 ] && [ "${WEAVE_NEW_ENTRY_CREATED:-false}" = true ]; then
        rm -rf -- "$WEAVE_NEW_ENTRY_DIR"
    fi
}

_version_gte() {
    local -a a=("${(@s:.:)1}") b=("${(@s:.:)2}")
    local i
    for i in 1 2 3; do
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
        PYTHONHOME="" PYTHONPATH="" "$py" - "$module" <<'PYEOF' <<<"$registry"
import json, sys
name = sys.argv[1]
reg = json.load(sys.stdin)
entry = next((e for e in reg if e['name'] == name), None)
if not entry:
    sys.stderr.write(f"[Weave ERROR] Module '{name}' not found in registry\n")
    sys.exit(1)
print(entry['repo'], entry['min_version'])
PYEOF
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

    setopt local_options extendedglob

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
    [[ "$PROJECT_NAME" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] ||
        error "Project name must start with a letter or underscore and contain only letters, digits, and underscores"

    [ -d "$DEST_DIR" ] && [ ! -L "$DEST_DIR" ] \
        || error "Project destination must be an existing directory and cannot be a symbolic link: $DEST_DIR"

    local PROJECT_DIR="$DEST_DIR/$PROJECT_NAME"

    # ============================================================================
    # CHECK IF PROJECT EXISTS
    # ============================================================================

    if [ -e "$PROJECT_DIR" ] || [ -L "$PROJECT_DIR" ]; then
        if [ -d "$PROJECT_DIR" ]; then
            if [ -f "$PROJECT_DIR/CMakeLists.txt" ] && grep -q "MayaFlux" "$PROJECT_DIR/CMakeLists.txt" 2>/dev/null; then
                echo "[Weave] Project '$PROJECT_NAME' already exists and is a MayaFlux project."
                echo "Nothing to do."
                exit 0
            else
                error "Directory '$PROJECT_DIR' already exists but is not a MayaFlux project"
            fi
        fi
        error "Path '$PROJECT_DIR' already exists"
    fi

    # ============================================================================
    # CHECK TEMPLATES DIRECTORY
    # ============================================================================

    [ ! -f "$TEMPLATES_DIR/CMakeLists.txt" ] && error "Required template missing: CMakeLists.txt"
    [ ! -f "$TEMPLATES_DIR/validate-macos.sh" ] && error "Required template missing: validate-macos.sh"
    [ ! -f "$TEMPLATES_DIR/run-report-macos.sh" ] && error "Required template missing: run-report-macos.sh"
    [ ! -f "$TEMPLATES_DIR/cmake/mayaflux.cmake" ] && error "Required template missing: cmake/mayaflux.cmake"
    [ ! -f "$TEMPLATES_DIR/cmake/shaders.cmake" ] && error "Required template missing: cmake/shaders.cmake"
    [ ! -f "$TEMPLATES_DIR/cmake/build_community.cmake" ] && error "Required template missing: cmake/build_community.cmake"
    [ ! -f "$TEMPLATES_DIR/main.cpp" ] && error "Required template missing: main.cpp"
    [ ! -f "$TEMPLATES_DIR/user_project.hpp" ] && error "Required template missing: user_project.hpp"
    [ ! -f "$TEMPLATES_DIR/.gitignore" ] && error "Required template missing: .gitignore"

    if [ "$COPY_SHADER_SOURCES" = true ]; then
        SHADER_SOURCE_DIR="$MAYAFLUX_ROOT/share/MayaFlux/src/shaders"
        [ ! -d "$SHADER_SOURCE_DIR" ] && error "MayaFlux shader sources were not found at $SHADER_SOURCE_DIR. Update MayaFlux or omit --copy-shader-sources."
        SHADER_SOURCE_DIR="$(cd "$SHADER_SOURCE_DIR" && pwd -P)" \
            || error "MayaFlux shader sources could not be accessed at $SHADER_SOURCE_DIR."
        local linked_shader
        linked_shader="$(find "$SHADER_SOURCE_DIR" -type l -print -quit)" \
            || error "MayaFlux shader sources could not be checked at $SHADER_SOURCE_DIR."
        [ -n "$linked_shader" ] && error "MayaFlux shader sources contain a symbolic link: $linked_shader"
        SHADER_SOURCES=("$SHADER_SOURCE_DIR"/**/(#i)*.(glsl|vert|frag|comp|geom|tesc|tese|mesh|task|rgen|rchit|rahit|rmiss|rint|rcall)(N.))
        [ "${#SHADER_SOURCES[@]}" -eq 0 ] && error "MayaFlux shader sources are empty at $SHADER_SOURCE_DIR. Update MayaFlux or omit --copy-shader-sources."
    fi

    # ============================================================================
    # CREATE PROJECT STRUCTURE
    # ============================================================================

    echo "[Weave] Creating project: $PROJECT_NAME"
    mkdir "$PROJECT_DIR" || error "Could not create project directory: $PROJECT_DIR"
    WEAVE_NEW_ENTRY_DIR="$PROJECT_DIR"
    WEAVE_NEW_ENTRY_CREATED=true
    trap '_cleanup_new_entry' EXIT
    mkdir -p "$PROJECT_DIR/src" || error "Could not create project source directory"
    mkdir -p "$PROJECT_DIR/cmake" || error "Could not create project CMake directory"

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

    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/CMakeLists.txt" >"$PROJECT_DIR/CMakeLists.txt" \
        || error "Failed to generate CMakeLists.txt"
    echo "✅ CMakeLists.txt generated"

    # ============================================================================
    # COPY cmake modules
    # ============================================================================

    sed "s|@LILA_LINK_BLOCK@|$LILA_LINK_BLOCK|g" "$TEMPLATES_DIR/cmake/mayaflux.cmake" |
        sed "s|@LILA_DEBUGGER_PATH@||g" |
        sed "s|@LILA_DLL_COPY@||g" \
            >"$PROJECT_DIR/cmake/mayaflux.cmake" \
        || error "Failed to generate cmake/mayaflux.cmake"
    echo "✅ cmake/mayaflux.cmake generated"
    cp "$TEMPLATES_DIR/cmake/shaders.cmake" "$PROJECT_DIR/cmake/shaders.cmake" \
        || error "Failed to copy cmake/shaders.cmake"
    echo "✅ cmake/shaders.cmake copied"
    cp "$TEMPLATES_DIR/cmake/build_community.cmake" "$PROJECT_DIR/cmake/build_community.cmake" \
        || error "Failed to copy cmake/build_community.cmake"
    echo "✅ cmake/build_community.cmake copied"

    cp "$TEMPLATES_DIR/CMakePresets.json" "$PROJECT_DIR/CMakePresets.json" \
        || error "Failed to copy CMakePresets.json"
    echo "  ✓ Copied CMakePresets.json"

    cp "$TEMPLATES_DIR/validate-macos.sh" "$PROJECT_DIR/validate.sh" \
        || error "Failed to copy validate-macos.sh"
    chmod +x "$PROJECT_DIR/validate.sh" || error "Could not make validate.sh executable"
    echo "  ✓ Copied validate.sh"
    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/run-report-macos.sh" >"$PROJECT_DIR/run-report.sh" \
        || error "Failed to generate run-report.sh"
    chmod +x "$PROJECT_DIR/run-report.sh" || error "Could not make run-report.sh executable"
    echo "  ✓ Generated run-report.sh"
    printf '\n' >"$PROJECT_DIR/community.cmake"

    # ============================================================================
    # COPY main.cpp
    # ============================================================================

    if [ ! -f "$TEMPLATES_DIR/main.cpp" ]; then
        error "main.cpp template not found at $TEMPLATES_DIR/main.cpp"
    fi

    cp "$TEMPLATES_DIR/main.cpp" "$PROJECT_DIR/src/main.cpp" || error "Failed to copy main.cpp"
    echo "✅ main.cpp created"

    # ============================================================================
    # COPY user_project.hpp
    # ============================================================================

    if [ ! -f "$TEMPLATES_DIR/user_project.hpp" ]; then
        error "user_project.hpp template not found at $TEMPLATES_DIR/user_project.hpp"
    fi

    cp "$TEMPLATES_DIR/user_project.hpp" "$PROJECT_DIR/src/user_project.hpp" \
        || error "Failed to copy user_project.hpp"
    echo "✅ user_project.hpp created"

    # ============================================================================
    # CREATE data/shaders
    # ============================================================================

    mkdir -p "$PROJECT_DIR/data/shaders" || error "Could not create data/shaders"
    echo "  ✓ Created data/shaders"

    if [ "$COPY_SHADER_SOURCES" = true ]; then
        local shader relative destination
        for shader in "${SHADER_SOURCES[@]}"; do
            relative="${shader#"$SHADER_SOURCE_DIR"/}"
            destination="$PROJECT_DIR/data/shaders/$relative"
            if [ -e "$destination" ]; then
                echo "  ✓ Preserved project shader: $relative"
                continue
            fi
            mkdir -p "$(dirname "$destination")" || error "Could not create shader directory: $relative"
            cp "$shader" "$destination" || error "Failed to copy MayaFlux shader source: $relative"
            echo "  ✓ Copied MayaFlux shader source: $relative"
        done
    fi

    # ============================================================================
    # CREATE VS CODE CONFIGURATION (if enabled)
    # ============================================================================

    if [ "$WITH_VSCODE" = true ]; then
            mkdir -p "$PROJECT_DIR/.vscode" || error "Could not create VS Code configuration directory"

        # Check if VS Code templates exist
        if [ -d "$TEMPLATES_DIR/vscode" ]; then
            # Use templates with substitution
            for vscode_file in settings.json tasks.json launch.json; do
                if [ -f "$TEMPLATES_DIR/vscode/${vscode_file}" ]; then
                    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" \
                        "$TEMPLATES_DIR/vscode/${vscode_file}" \
                        >"$PROJECT_DIR/.vscode/$vscode_file"
                    [ "$?" -eq 0 ] || error "Failed to generate VS Code $vscode_file"
                fi
            done
            echo "✅ VS Code configuration created"
        else
            echo "⚠️  VS Code templates not found, skipping"
        fi
    fi

    cp "$TEMPLATES_DIR/.gitignore" "$PROJECT_DIR/.gitignore" || error "Failed to copy .gitignore"
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

## Troubleshooting

If setup or a build fails, run \`./validate.sh\` to check the MayaFlux Homebrew setup, shell environment, build tools, and project files.

## Reporting a problem

Build the project, then run \`./run-report.sh\`. It runs the project with MayaFlux's report configuration. Reproduce the problem, then close the application. Attach \`mayaflux_run.txt\` and \`data/log/mayaflux_report.log\` to the issue, together with the output of \`./validate.sh\`.

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

    WEAVE_NEW_ENTRY_CREATED=false
    trap - EXIT
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

    local -a MODULE_NAMES=()
    local ARG REST PART KNOWN DUPLICATE
    for ARG in "$@"; do
        REST="$ARG,"
        while [ -n "$REST" ]; do
            PART="${REST%%,*}"
            REST="${REST#*,}"
            PART="${PART#"${PART%%[![:space:]]*}"}"
            PART="${PART%"${PART##*[![:space:]]}"}"
            [ -n "$PART" ] || error "Empty module name in: $ARG"
            printf '%s\n' "$PART" | grep -qE '^[a-z][a-z0-9_]*$' ||
                error "Module name must be snake_case (lowercase letters, digits, underscores, no leading digit): $PART"
            DUPLICATE=0
            for KNOWN in ${MODULE_NAMES[@]+"${MODULE_NAMES[@]}"}; do
                [ "$KNOWN" = "$PART" ] && DUPLICATE=1
            done
            [ "$DUPLICATE" -eq 1 ] || MODULE_NAMES+=("$PART")
        done
    done

    command -v curl >/dev/null 2>&1 || error "Required command not found: curl"
    command -v git >/dev/null 2>&1 || error "Required command not found: git"

    echo "[Weave] Fetching registry..."
    local REGISTRY
    REGISTRY="$(curl -fsSL "$REGISTRY_URL")" || error "Failed to fetch registry from $REGISTRY_URL"

    local COMMUNITY_DIR="$PROJECT_DIR/community"
    mkdir -p "$COMMUNITY_DIR"

    local COMMUNITY_CMAKE="$PROJECT_DIR/community.cmake"
    touch "$COMMUNITY_CMAKE"

    for MODULE_NAME in "${MODULE_NAMES[@]}"; do
        echo "[Weave] Looking up module: $MODULE_NAME"

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
            echo "[Weave]   $MODULE_NAME already present, skipping clone"
        else
            echo "[Weave]   Cloning $REPO..."
            git clone --depth=1 "$REPO" "$MODULE_DIR" >/dev/null 2>&1 || error "Failed to clone $REPO"
            echo "[Weave]   ✓ Cloned into community/$MODULE_NAME"
        fi

        [ ! -f "$MODULE_DIR/${MODULE_NAME}.cmake" ] && error "Module '$MODULE_NAME' is missing ${MODULE_NAME}.cmake"
        [ ! -d "$MODULE_DIR/src" ] && error "Module '$MODULE_NAME' is missing src/"
    done

    for MODULE_NAME in "${MODULE_NAMES[@]}"; do
        if ! grep -qxF "$MODULE_NAME" "$COMMUNITY_CMAKE"; then
            echo "$MODULE_NAME" >>"$COMMUNITY_CMAKE"
            echo "[Weave]   ✓ Added $MODULE_NAME to community.cmake"
        else
            echo "[Weave]   ✓ $MODULE_NAME already in community.cmake"
        fi
    done

    echo ""
    echo "[Weave] Done. Rebuild your project to include the new modules."
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
    [ ! -L "$DEST_DIR" ] \
        || error "Community destination cannot be a symbolic link: $DEST_DIR"
    mkdir -p "$DEST_DIR" || error "Cannot create destination directory: $DEST_DIR"
    [ -d "$DEST_DIR" ] || error "Community destination is not a directory: $DEST_DIR"
    DEST_DIR="$(cd "$DEST_DIR" && pwd)"

    local MODULE_DIR="$DEST_DIR/$MODULE_NAME"
    if [ -e "$MODULE_DIR" ] || [ -L "$MODULE_DIR" ]; then
        error "Directory already exists: $MODULE_DIR"
    fi

    [ ! -f "$TEMPLATES_DIR/community/module.cmake" ] && error "Required template missing: community/module.cmake"
    [ ! -f "$TEMPLATES_DIR/community/community.json" ] && error "Required template missing: community/community.json"
    [ ! -f "$TEMPLATES_DIR/community/test/CMakeLists.txt" ] && error "Required template missing: community/test/CMakeLists.txt"

    mkdir "$MODULE_DIR" || error "Could not create module directory: $MODULE_DIR"
    WEAVE_NEW_ENTRY_DIR="$MODULE_DIR"
    WEAVE_NEW_ENTRY_CREATED=true
    trap '_cleanup_new_entry' EXIT
    mkdir -p "$MODULE_DIR/src" || error "Could not create community source directory"
    mkdir -p "$MODULE_DIR/test" || error "Could not create community test directory"

    cp "$TEMPLATES_DIR/.gitignore" "$MODULE_DIR/.gitignore" \
        || error "Failed to copy community template: .gitignore"
    cp "$TEMPLATES_DIR/community/CMakePresets.json" "$MODULE_DIR/CMakePresets.json" \
        || error "Failed to copy community template: CMakePresets.json"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/module.cmake" \
        >"$MODULE_DIR/${MODULE_NAME}.cmake" \
        || error "Failed to generate community CMake file"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/community.json" \
        >"$MODULE_DIR/community.json" \
        || error "Failed to generate community manifest"

    sed "s|@MODULE_NAME@|$MODULE_NAME|g" "$TEMPLATES_DIR/community/test/CMakeLists.txt" \
        >"$MODULE_DIR/test/CMakeLists.txt" \
        || error "Failed to generate community test configuration"

    touch "$MODULE_DIR/test/test_${MODULE_NAME}.cpp" || error "Could not create community test source"
    git -C "$MODULE_DIR" init -q -b main || error "Failed to initialize community module repository"
    git -C "$MODULE_DIR" add -A || error "Failed to stage community module files"

    WEAVE_NEW_ENTRY_CREATED=false
    trap - EXIT

    echo ""
    echo "[Weave] Community module created: $MODULE_DIR"
    echo ""
    echo "  src/           <- put your sources here"
    echo "  test/CMakeLists.txt"
    echo "  test/test_${MODULE_NAME}.cpp"
    echo "  ${MODULE_NAME}.cmake"
    echo "  community.json"
    echo ""
    echo "Build test:"
    echo "  cmake -G Ninja -B test/build -S test/"
    echo "  cmake --build test/build --parallel"
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
