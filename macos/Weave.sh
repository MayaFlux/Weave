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
WEAVE_ROOT="/Library/Weave"
TEMPLATES_DIR="$WEAVE_ROOT/templates"

# ============================================================================
# UTILITIES
# ============================================================================

usage() {
    cat <<EOF
Weave - MayaFlux Project Creator

Usage:
  weave new <project-name> [destination-dir]
  weave --help

Examples:
  weave new AudioViz ~/Projects/
  weave new MyApp .

Options:
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
# ARGUMENT PARSING
# ============================================================================

[ "$#" -eq 0 ] && usage
[ "$1" = "--help" ] && usage
[ "$1" != "new" ] && error "Unknown command: $1. Use 'weave new <name>'"

shift
PROJECT_NAME="${1:-}"
DEST_DIR="${2:-.}"
WITH_LILA=false
WITH_VSCODE=true

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

PROJECT_DIR="$DEST_DIR/$PROJECT_NAME"

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

if [ ! -d "$TEMPLATES_DIR" ]; then
    error "Templates not found at $TEMPLATES_DIR. Is MayaFlux properly installed?"
fi

# ============================================================================
# CREATE PROJECT STRUCTURE
# ============================================================================

echo "[Weave] Creating project: $PROJECT_NAME"
mkdir -p "$PROJECT_DIR/src"

# ============================================================================
# GENERATE CMakeLists.txt
# ============================================================================

MAYAFLUX_CMAKE_PATH="$MAYAFLUX_ROOT/lib/cmake/MayaFlux"

if [ "$WITH_LILA" = true ]; then
    LILA_LINK_BLOCK='if(TARGET MayaFlux::Lila)
    target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::Lila)
    message(STATUS "Lila live coding enabled")
else()
    message(WARNING "Lila not found - live coding disabled")
endif()'
else
    LILA_LINK_BLOCK='# Lila live coding not enabled'
fi

LILA_DLL_COPY='# Lila DLL copy not needed'

sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/CMakeLists.txt" |
    sed "s|@MAYAFLUX_CMAKE_PATH@|$MAYAFLUX_CMAKE_PATH|g" |
    sed "s|@LILA_LINK_BLOCK@|${LILA_LINK_BLOCK}|g" |
    sed "s|@LILA_DLL_COPY@|${LILA_DLL_COPY}|g" \
        >"$PROJECT_DIR/CMakeLists.txt"

echo "✅ CMakeLists.txt generated"

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

exit 0
