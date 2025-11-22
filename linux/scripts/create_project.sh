#!/bin/bash
# Weave - MayaFlux Project Creator for Linux
# Creates new MayaFlux projects from embedded templates

set -euo pipefail

# ============================================================================
# CONFIGURATION
# ============================================================================

MAYAFLUX_ROOT="${MAYAFLUX_ROOT:-/usr/local}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
TEMPLATES_DIR="$PROJECT_ROOT/templates"

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
                 (default: /usr/local)
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

# ============================================================================
# ARGUMENT PARSING
# ============================================================================

[ "$#" -eq 0 ] && usage

if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    usage
fi

if [ "$1" != "new" ]; then
    error "Unknown command: $1. Use 'weave new <name>'"
fi

shift
PROJECT_NAME="${1:-}"
DEST_DIR="${2:-.}"
WITH_LILA=false
WITH_VSCODE=true

shift 2 2>/dev/null || true

while [ "$#" -gt 0 ]; do
    case "$1" in
    --with-lila)
        WITH_LILA=true
        ;;
    --no-vscode)
        WITH_VSCODE=false
        ;;
    *)
        error "Unknown option: $1"
        ;;
    esac
    shift
done

# ============================================================================
# VALIDATION
# ============================================================================

[ -z "$PROJECT_NAME" ] && error "Project name required"

DEST_DIR="${DEST_DIR/#\~/$HOME}"

mkdir -p "$DEST_DIR" || error "Cannot create destination directory: $DEST_DIR"
DEST_DIR="$(cd "$DEST_DIR" && pwd)"

PROJECT_DIR="$DEST_DIR/$PROJECT_NAME"

if [ -d "$PROJECT_DIR" ]; then
    if [ -f "$PROJECT_DIR/CMakeLists.txt" ] && grep -q "MayaFlux" "$PROJECT_DIR/CMakeLists.txt" 2>/dev/null; then
        log "Project '$PROJECT_NAME' already exists and is a MayaFlux project."
        log "Nothing to do."
        exit 0
    else
        error "Directory already exists and is not a MayaFlux project: $PROJECT_DIR"
    fi
fi

# ============================================================================
# CHECK TEMPLATES
# ============================================================================

if [ ! -d "$TEMPLATES_DIR" ]; then
    error "Templates not found at $TEMPLATES_DIR"
fi

REQUIRED_TEMPLATES=(
    "CMakeLists.txt"
    "main.cpp"
    "user_project.hpp"
)

for template in "${REQUIRED_TEMPLATES[@]}"; do
    if [ ! -f "$TEMPLATES_DIR/$template" ]; then
        error "Required template missing: $template at $TEMPLATES_DIR/$template"
    fi
done

# ============================================================================
# CREATE PROJECT STRUCTURE
# ============================================================================

log "Creating project: $PROJECT_NAME"
mkdir -p "$PROJECT_DIR/src"

if [ "$WITH_VSCODE" = true ]; then
    mkdir -p "$PROJECT_DIR/.vscode"
    log "  ✓ Created .vscode directory"
fi

log "  ✓ Created project structure"

# ============================================================================
# DETERMINE MAYAFLUX CMAKE PATH
# ============================================================================

MAYAFLUX_CMAKE_PATH="$MAYAFLUX_ROOT/lib/cmake/MayaFlux"

if [ ! -d "$MAYAFLUX_CMAKE_PATH" ]; then
    log "  ⚠ Warning: CMake config not found at $MAYAFLUX_CMAKE_PATH"
    log "  ⚠ You may need to set MAYAFLUX_ROOT or install MayaFlux"
fi

# ============================================================================
# GENERATE CMakeLists.txt
# ============================================================================

log "Generating CMakeLists.txt"

if [ "$WITH_LILA" = true ]; then
    LILA_LINK_BLOCK='if(TARGET MayaFlux::Lila)
    target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::Lila)
    message(STATUS "Lila live coding enabled")
else()
    message(WARNING "Lila not found - live coding disabled")
endif()'

    LILA_DLL_COPY='if(EXISTS "$ENV{MAYAFLUX_ROOT}/bin/Lila.dll")
            add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "$ENV{MAYAFLUX_ROOT}/bin/Lila.dll"
                    $<TARGET_FILE_DIR:${PROJECT_NAME}>
            )
        endif()'
else
    LILA_LINK_BLOCK='# Lila live coding not enabled'
    LILA_DLL_COPY='# Lila DLL copy not needed'
fi

{
    sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" "$TEMPLATES_DIR/CMakeLists.txt" |
        sed "s|@MAYAFLUX_CMAKE_PATH@|$MAYAFLUX_CMAKE_PATH|g" |
        awk -v lila="$LILA_LINK_BLOCK" '{gsub(/@LILA_LINK_BLOCK@/, lila); print}' |
        awk -v lila_dll="$LILA_DLL_COPY" '{gsub(/@LILA_DLL_COPY@/, lila_dll); print}'
} >"$PROJECT_DIR/CMakeLists.txt"

log "  ✓ Generated CMakeLists.txt"

# ============================================================================
# COPY MAIN.CPP
# ============================================================================

log "Generating source files"

if [ ! -f "$TEMPLATES_DIR/main.cpp" ]; then
    error "main.cpp template not found"
fi

cp "$TEMPLATES_DIR/main.cpp" "$PROJECT_DIR/src/main.cpp"
log "  ✓ Generated main.cpp"

# ============================================================================
# COPY USER_PROJECT.HPP
# ============================================================================

if [ ! -f "$TEMPLATES_DIR/user_project.hpp" ]; then
    error "user_project.hpp template not found"
fi

cp "$TEMPLATES_DIR/user_project.hpp" "$PROJECT_DIR/src/user_project.hpp"
log "  ✓ Generated user_project.hpp"

# ============================================================================
# CREATE VS CODE CONFIGURATION (if enabled)
# ============================================================================

if [ "$WITH_VSCODE" = true ]; then
    if [ -d "$TEMPLATES_DIR/vscode" ]; then
        log "Generating VS Code configuration"

        for vscode_file in settings.json tasks.json launch.json; do
            if [ -f "$TEMPLATES_DIR/vscode/$vscode_file" ]; then
                sed "s|@PROJECT_NAME@|$PROJECT_NAME|g" \
                    "$TEMPLATES_DIR/vscode/$vscode_file" \
                    >"$PROJECT_DIR/.vscode/$vscode_file"
            fi
        done

        log "  ✓ Generated VS Code configuration"
    else
        log "  ⚠ VS Code templates not found, skipping"
    fi
fi

# ============================================================================
# CREATE .GITIGNORE
# ============================================================================

log "Generating .gitignore"

cat >"$PROJECT_DIR/.gitignore" <<'EOF'
# Build directories
/build/
/cmake-build-*/

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# Generated
compile_commands.json
CMakeCache.txt
CMakeFiles/
cmake_install.cmake

# Binaries
*.o
*.so
*.a
*.lib
*.exe

# macOS
.DS_Store
.AppleDouble/
.LSOverride/

# Python
__pycache__/
*.pyc
*.pyo

# Misc
.env
local_settings.py
EOF

log "  ✓ Generated .gitignore"

# ============================================================================
# CREATE README
# ============================================================================

log "Generating README.md"

cat >"$PROJECT_DIR/README.md" <<EOF
# $PROJECT_NAME

A MayaFlux multimedia DSP project.

## Quick Start

### Build

\`\`\`bash
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --parallel
\`\`\`

### Run

\`\`\`bash
./build/$PROJECT_NAME
\`\`\`

## Project Structure

\`\`\`
$PROJECT_NAME/
├── src/
│   ├── main.cpp           # Entry point
│   └── user_project.hpp   # Your code goes here
├── CMakeLists.txt         # Build configuration
└── README.md
\`\`\`

## Editing Code

### Using VS Code

\`\`\`bash
code .
\`\`\`

Then edit \`src/user_project.hpp\`:
- **\`settings()\`** - Configure sample rate, buffer size, graphics, etc.
- **\`compose()\`** - Create your nodes, buffers, and processing chains

### From Command Line

\`\`\`bash
nano src/user_project.hpp
\`\`\`

## Environment

Make sure \`MAYAFLUX_ROOT\` is set:

\`\`\`bash
echo \$MAYAFLUX_ROOT
# Should output: $HOME/MayaFlux (or your installation location)
\`\`\`

If not set, add to \`~/.bashrc\` or \`~/.zshrc\`:

\`\`\`bash
export MAYAFLUX_ROOT=$HOME/MayaFlux
export PATH=\$MAYAFLUX_ROOT/bin:\$PATH
export CMAKE_PREFIX_PATH=\$MAYAFLUX_ROOT:\$CMAKE_PREFIX_PATH
\`\`\`

Then reload:

\`\`\`bash
source ~/.bashrc   # or ~/.zshrc
\`\`\`

## Building with Different Configurations

### Release (Optimized)

\`\`\`bash
cmake .. -DCMAKE_BUILD_TYPE=Release
\`\`\`

### Debug (With debugging symbols)

\`\`\`bash
cmake .. -DCMAKE_BUILD_TYPE=Debug
\`\`\`

### Custom Compiler

\`\`\`bash
cmake .. -DCMAKE_CXX_COMPILER=clang++
\`\`\`

## Troubleshooting

### CMake can't find MayaFlux

Set the \`MAYAFLUX_ROOT\` environment variable:

\`\`\`bash
export MAYAFLUX_ROOT=/path/to/MayaFlux
cmake .. -DCMAKE_BUILD_TYPE=Release
\`\`\`

### Build errors with C++20 features

Ensure your compiler supports C++23:

\`\`\`bash
g++ --version    # Ensure >= 12.0
clang++ --version  # Ensure >= 15.0
\`\`\`

## Documentation

- [MayaFlux Documentation](https://github.com/MayaFlux/MayaFlux)
- [CMake Documentation](https://cmake.org/cmake/help/latest/)

## Next Steps

1. Edit \`src/user_project.hpp\` to start coding
2. Run \`cmake --build . --parallel\` to rebuild
3. Run \`./build/$PROJECT_NAME\` to test

Happy coding! 🎉
EOF

log "  ✓ Generated README.md"

# ============================================================================
# SUMMARY
# ============================================================================

echo ""
echo "=========================================="
echo "  ✓ Project '$PROJECT_NAME' created!"
echo "=========================================="
echo ""
echo "Location: $PROJECT_DIR"
echo ""
echo "Next steps:"
echo "  1. cd $PROJECT_DIR"

if [ "$WITH_VSCODE" = true ]; then
    echo "  2. code .                    # Open in VS Code"
    echo "  3. Edit src/user_project.hpp"
    echo "  4. mkdir build && cd build"
else
    echo "  2. Edit src/user_project.hpp"
    echo "  3. mkdir build && cd build"
fi

echo "  5. cmake .. && make"
echo "  6. ./$(basename "$PROJECT_DIR")"
echo ""
echo "Documentation: https://github.com/MayaFlux/MayaFlux"
echo ""

exit 0
