#!/bin/bash

set -euo pipefail

DIST="build/linux/Weave"
WEAVE_ROOT="$DIST"

echo "Building Weave distribution with unified configuration..."
echo ""

rm -rf "$DIST"
mkdir -p "$DIST/lib/weave"

echo "Copying Python modules..."
cp -r linux/weave/* "$DIST/lib/weave/"

if [ ! -f "$DIST/lib/weave/config.py" ]; then
    echo "ERROR: config.py not found in linux/weave/" >&2
    exit 1
fi

echo "Copying scripts and templates..."
mkdir -p "$DIST/lib/weave/scripts"
mkdir -p "$DIST/lib/weave/templates"

cp -r templates/* "$DIST/lib/weave/templates/"
cp linux/scripts/{create_project.sh,install_deps.sh} "$DIST/lib/weave/scripts/"
chmod +x "$DIST/lib/weave/scripts/"*

echo "Generating weave-config.json..."
cat >"$DIST/weave-config.json" <<'EOF'
{
  "version": "0.1.2",
  "package_root": "${WEAVE_ROOT}",
  "paths": {
    "lib": "${WEAVE_ROOT}/lib",
    "scripts": "${WEAVE_ROOT}/lib/weave/scripts",
    "templates": "${WEAVE_ROOT}/lib/weave/templates",
    "python_path": "${WEAVE_ROOT}/lib"
  },
  "executables": {
    "create_project_sh": "${WEAVE_ROOT}/lib/weave/scripts/create_project.sh",
    "install_deps_sh": "${WEAVE_ROOT}/lib/weave/scripts/install_deps.sh"
  },
  "environment_variables": {
    "WEAVE_ROOT": "${WEAVE_ROOT}",
    "PYTHONPATH": "${WEAVE_ROOT}/lib:${PYTHONPATH}",
    "WEAVE_TEMPLATE_DIR": "${WEAVE_ROOT}/lib/weave/templates",
    "WEAVE_SCRIPT_DIR": "${WEAVE_ROOT}/lib/weave/scripts"
  }
}
EOF

echo "✓ Configuration file created"

echo "Creating launcher script..."
cat >"$DIST/Weave" <<'LAUNCHER'
#!/bin/bash
# Weave launcher - sets up environment from weave-config.json

set -euo pipefail

# Find this script's directory
WEAVE_LAUNCHER_DIR="$(cd "$(dirname "$0")" && pwd)"

# Detect Weave root
if [ -f "$WEAVE_LAUNCHER_DIR/weave-config.json" ]; then
    export WEAVE_ROOT="$WEAVE_LAUNCHER_DIR"
elif [ -f "$WEAVE_LAUNCHER_DIR/../weave-config.json" ]; then
    export WEAVE_ROOT="$(cd "$WEAVE_LAUNCHER_DIR/.." && pwd)"
else
    echo "Error: weave-config.json not found" >&2
    exit 1
fi

export WEAVE_LAUNCHER_DIR

# Set Python path
export PYTHONPATH="$WEAVE_ROOT/lib:${PYTHONPATH:-}"

exec python3 -m weave.main "$@"
LAUNCHER

chmod +x "$DIST/Weave"
echo "✓ Launcher script created"

echo ""
echo "Verifying distribution structure..."
test -f "$DIST/weave-config.json" && echo "  ✓ weave-config.json"
test -f "$DIST/Weave" && echo "  ✓ Weave launcher"
test -f "$DIST/lib/weave/config.py" && echo "  ✓ config.py"
test -f "$DIST/lib/weave/main.py" && echo "  ✓ main.py"
test -f "$DIST/lib/weave/cli.py" && echo "  ✓ cli.py"
test -d "$DIST/lib/weave/scripts" && echo "  ✓ scripts directory"
test -d "$DIST/lib/weave/templates" && echo "  ✓ templates directory"

echo ""
echo "Creating distribution tarball..."
tar czf "$DIST.tar.gz" -C build/linux "Weave"

TARBALL_SIZE=$(stat -f%z "$DIST.tar.gz" 2>/dev/null || stat -c%s "$DIST.tar.gz")
TARBALL_SIZE_MB=$((TARBALL_SIZE / 1024 / 1024))

echo "✓ Tarball created: $DIST.tar.gz (${TARBALL_SIZE_MB} MB)"

echo ""
echo "Generating hash..."
sha256sum "$DIST.tar.gz"

echo ""
echo "=========================================="
echo "  Weave Distribution Built Successfully!"
echo "=========================================="
echo ""
echo "Distribution: $DIST.tar.gz"
echo "Size: ${TARBALL_SIZE_MB} MB"
echo ""
echo "Installation method:"
echo "  tar -xzf $DIST.tar.gz -C ~/.local/"
echo "  ~/.local/Weave/Weave              # For GUI"
echo "  ~/.local/Weave/Weave new Project  # For CLI"
