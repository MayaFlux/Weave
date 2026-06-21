#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LINUX_DIR="$REPO_ROOT/linux"
BUILD_DIR="$REPO_ROOT/build/linux"
APPDIR="$BUILD_DIR/Weave.AppDir"
TOOLS_DIR="$BUILD_DIR/tools"

PYTHON_VERSION="3.11.9"
PYTHON_STANDALONE_TAG="20240814"
PYTHON_ARCHIVE="cpython-${PYTHON_VERSION}+${PYTHON_STANDALONE_TAG}-x86_64-unknown-linux-gnu-install_only.tar.gz"
PYTHON_URL="https://github.com/indygreg/python-build-standalone/releases/download/${PYTHON_STANDALONE_TAG}/${PYTHON_ARCHIVE}"

LINUXDEPLOY_URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
LINUXDEPLOY_GTK_URL="https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
APPIMAGETOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"

VERSION=$(grep '^version' "$LINUX_DIR/pyproject.toml" | sed 's/version = "\(.*\)"/\1/')
OUTPUT="$BUILD_DIR/Weave-${VERSION}-x86_64.AppImage"

echo "Building Weave AppImage v${VERSION}..."
echo ""

# ==============================================================================
# Re-exec inside Ubuntu 25.10 container for correct glibc/GTK baseline
# ==============================================================================

if [ "${WEAVE_APPIMAGE_CONTAINER:-0}" != "1" ]; then
    if command -v podman &>/dev/null; then
        echo "Re-running inside Ubuntu 25.10 container for glibc 2.35 baseline..."
        exec podman run --rm \
            -v "$REPO_ROOT:/workspace:z" \
            -w /workspace \
            -e WEAVE_APPIMAGE_CONTAINER=1 \
            ubuntu:25.10 \
            bash linux/scripts/build_appimage.sh
    fi
fi

# ==============================================================================
# Build dependencies (container or CI)
# ==============================================================================

if [ "${WEAVE_APPIMAGE_CONTAINER:-0}" = "1" ]; then
    echo "Installing build dependencies..."
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y -qq \
        curl git file patchelf binutils \
        libgtk-4-dev \
        gtk-4-examples \
        libadwaita-1-dev libadwaita-1-0 \
        libgirepository-2.0-dev \
        libglib2.0-dev \
        libcairo2-dev libcairo2 \
        libffi-dev \
        gir1.2-gtk-4.0 gir1.2-adw-1 \
        pkg-config \
        python3-pip \
        fuse \
        libgdk-pixbuf2.0-bin
    echo "Build dependencies installed."
    echo ""
fi

# ==============================================================================
# Toolchain
# ==============================================================================

mkdir -p "$TOOLS_DIR"

download_tool() {
    local url="$1"
    local dest="$2"
    if [ ! -f "$dest" ]; then
        echo "Downloading $(basename "$dest")..."
        curl -fsSL "$url" -o "$dest"
        chmod +x "$dest"
    else
        echo "$(basename "$dest") already cached."
    fi
}

download_tool "$LINUXDEPLOY_URL" "$TOOLS_DIR/linuxdeploy-x86_64.AppImage"
download_tool "$LINUXDEPLOY_GTK_URL" "$TOOLS_DIR/linuxdeploy-plugin-gtk.sh"
download_tool "$APPIMAGETOOL_URL" "$TOOLS_DIR/appimagetool-x86_64.AppImage"

if [ ! -d "$TOOLS_DIR/linuxdeploy-squashfs" ]; then
    echo "Extracting linuxdeploy..."
    cd "$TOOLS_DIR"
    APPIMAGE_EXTRACT_AND_RUN=1 ./linuxdeploy-x86_64.AppImage --appimage-extract
    mv squashfs-root linuxdeploy-squashfs
    cd "$REPO_ROOT"
fi

STRIP_BIN="$(which strip 2>/dev/null || which x86_64-linux-gnu-strip 2>/dev/null || true)"
if [ -n "$STRIP_BIN" ]; then
    ln -sf "$STRIP_BIN" "$TOOLS_DIR/linuxdeploy-squashfs/usr/bin/strip"
else
    echo "WARNING: strip not found, linuxdeploy will use its own" >&2
fi

LINUXDEPLOY="$TOOLS_DIR/linuxdeploy-squashfs/AppRun"
export LINUXDEPLOY
export APPIMAGE_EXTRACT_AND_RUN=1
export PATH="$TOOLS_DIR:$PATH"

# ==============================================================================
# Python standalone
# ==============================================================================

PYROOT="$BUILD_DIR/python-standalone"

if [ ! -d "$PYROOT" ]; then
    echo "Downloading python-build-standalone ${PYTHON_VERSION}..."
    curl -fsSL "$PYTHON_URL" -o "$BUILD_DIR/${PYTHON_ARCHIVE}"
    echo "Extracting Python..."
    tar -xf "$BUILD_DIR/${PYTHON_ARCHIVE}" -C "$BUILD_DIR"
    mv "$BUILD_DIR/python" "$PYROOT"
    rm "$BUILD_DIR/${PYTHON_ARCHIVE}"
else
    echo "python-build-standalone already present."
fi

PYTHON="$PYROOT/bin/python3"
PIP="$PYROOT/bin/pip3"
echo "Python: $($PYTHON --version)"

# ==============================================================================
# PyGObject + pycairo into standalone Python
# ==============================================================================

echo ""
echo "Installing PyGObject and pycairo..."
"$PIP" install --quiet pycairo PyGObject

echo "Verifying PyGObject links against standalone Python..."
GI_SO=$(find "$PYROOT" -name "_gi.cpython-*.so" | head -1)
if [ -z "$GI_SO" ]; then
    echo "ERROR: _gi.cpython-*.so not found" >&2
    exit 1
fi
if ldd "$GI_SO" | grep libpython | grep -q "=> /usr/lib"; then
    echo "ERROR: _gi.so links against system libpython, not standalone" >&2
    ldd "$GI_SO" | grep libpython >&2
    exit 1
fi
echo "OK: $GI_SO"

# ==============================================================================
# Assemble AppDir — always from scratch
# ==============================================================================

echo ""
echo "Assembling AppDir..."
rm -rf "$APPDIR"

mkdir -p \
    "$APPDIR/usr/bin" \
    "$APPDIR/usr/lib" \
    "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
    "$APPDIR/usr/share/glib-2.0/schemas" \
    "$APPDIR/lib"

echo "Copying Weave application..."
cp -r "$LINUX_DIR/lib/"* "$APPDIR/lib/"
cp -r "$REPO_ROOT/templates" "$APPDIR/lib/templates"
cp "$LINUX_DIR/weave-config.json" "$APPDIR/weave-config.json"

echo "Copying scripts..."
mkdir -p "$APPDIR/lib/scripts"
cp "$LINUX_DIR/scripts/create_project.sh" "$APPDIR/lib/scripts/"
chmod +x "$APPDIR/lib/scripts/create_project.sh"

echo "Copying Python runtime..."
cp -r "$PYROOT/lib/python3.11" "$APPDIR/usr/lib/python3.11"
find "$PYROOT/lib" -maxdepth 1 -name "*.so*" -exec cp -P {} "$APPDIR/usr/lib/" \;
cp "$PYROOT/bin/python3.11" "$APPDIR/usr/bin/python3"

cp "$LINUX_DIR/resources/weave.png" "$APPDIR/Weave.png"
cp "$LINUX_DIR/resources/weave.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/Weave.png"

cat >"$APPDIR/Weave.desktop" <<EOF
[Desktop Entry]
Name=Weave
Exec=Weave
Icon=Weave
Type=Application
Categories=Utility;Development;
EOF

cat >"$APPDIR/usr/bin/weave-launcher" <<'EOF'
#!/bin/bash
exec "$(dirname "$(readlink -f "$0")")/python3" \
    "$(dirname "$(readlink -f "$0")")/../../lib/main.py" "$@"
EOF
chmod +x "$APPDIR/usr/bin/weave-launcher"
mv "$APPDIR/usr/bin/weave-launcher" "$APPDIR/usr/bin/Weave"

# ==============================================================================
# linuxdeploy — bundle GTK4, libadwaita, typelibs, schemas
# ==============================================================================

GTK_LIB_DIR="$(pkg-config --variable=libdir gtk4)"

echo "Running linuxdeploy (no GTK plugin)..."

export DISABLE_COPYRIGHT_FILES_DEPLOYMENT=1

"$LINUXDEPLOY" \
    --appdir "$APPDIR" \
    --library "${GTK_LIB_DIR}/libgtk-4.so.1" \
    --library "$(pkg-config --variable=libdir libadwaita-1)/libadwaita-1.so.0" \
    --library "$(pkg-config --variable=libdir harfbuzz)/libharfbuzz.so.0" \
    --executable "/usr/bin/gtk4-demo" \
    --executable "$APPDIR/usr/bin/python3" \
    --desktop-file "$APPDIR/Weave.desktop" \
    --icon-file "$LINUX_DIR/resources/weave.png"

cd "$REPO_ROOT"

echo "Manually deploying GTK4 modules and typelibs..."

GTK_MOD_SRC="${GTK_LIB_DIR}/gtk-4.0"
GTK_MOD_DST="$APPDIR/usr/lib/gtk-4.0"
if [ -d "$GTK_MOD_SRC" ]; then
    mkdir -p "$GTK_MOD_DST"
    cp -r "$GTK_MOD_SRC/." "$GTK_MOD_DST/"
fi

TYPELIB_SRC="/usr/lib/x86_64-linux-gnu/girepository-1.0"
TYPELIB_DST="$APPDIR/usr/lib/girepository-1.0"
mkdir -p "$TYPELIB_DST"
echo "Copying all typelibs from $TYPELIB_SRC..."
cp "$TYPELIB_SRC"/*.typelib "$TYPELIB_DST/"

SCHEMA_SRC="/usr/share/glib-2.0/schemas"
SCHEMA_DST="$APPDIR/usr/share/glib-2.0/schemas"
mkdir -p "$SCHEMA_DST"
cp "$SCHEMA_SRC/gschemas.compiled" "$SCHEMA_DST/" 2>/dev/null || glib-compile-schemas "$SCHEMA_DST"

cat >"$APPDIR/AppRun" <<'EOF'
#!/bin/bash
HERE="${APPDIR:-$(dirname "$(readlink -f "$0")")}"

export PYTHONHOME="$HERE/usr"
export PYTHONPATH="$HERE/usr/lib/python3.11:$HERE/usr/lib/python3.11/lib-dynload:$HERE/usr/lib/python3.11/site-packages:$HERE/lib"
export GI_TYPELIB_PATH="$HERE/usr/lib/girepository-1.0"
export LD_LIBRARY_PATH="$HERE/usr/lib:${LD_LIBRARY_PATH:-}"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export GSETTINGS_SCHEMA_DIR="$HERE/usr/share/glib-2.0/schemas"
export GDK_BACKEND="wayland"
export WEAVE_ROOT="$HERE"

exec "$HERE/usr/bin/python3" "$HERE/lib/main.py" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# ==============================================================================
# Rewrite gdk-pixbuf loaders cache to use AppDir paths
# ==============================================================================

LOADERS_CACHE="$APPDIR/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
if [ -d "$(dirname "$LOADERS_CACHE")" ]; then
    if command -v gdk-pixbuf-query-loaders &>/dev/null; then
        echo "Rewriting gdk-pixbuf loaders cache..."
        GDK_PIXBUF_MODULEDIR="$APPDIR/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders" \
            gdk-pixbuf-query-loaders >"$LOADERS_CACHE"
    else
        echo "gdk-pixbuf-query-loaders not found, skipping cache rewrite."
    fi
fi

# ==============================================================================
# Verify
# ==============================================================================

echo ""
echo "Verifying bundled libraries and typelibs..."

MISSING=0
check() {
    if find "$APPDIR" -name "$1" 2>/dev/null | grep -q .; then
        echo "  OK  $2"
    else
        echo "  MISSING  $2" >&2
        MISSING=1
    fi
}

check "libgtk-4.so*" "libgtk-4"
check "libadwaita-1.so*" "libadwaita"
check "Gtk-4.0.typelib" "Gtk-4.0 typelib"
check "Adw-1.typelib" "Adw-1 typelib"

if [ "$MISSING" -ne 0 ]; then
    echo "ERROR: Required files missing from AppDir. Aborting." >&2
    exit 1
fi

# ==============================================================================
# Compile GSettings schemas
# ==============================================================================

SCHEMA_DIR="$APPDIR/usr/share/glib-2.0/schemas"
if ls "$SCHEMA_DIR"/*.xml &>/dev/null; then
    echo "Compiling GSettings schemas..."
    glib-compile-schemas "$SCHEMA_DIR"
fi

# ==============================================================================
# Pack AppImage
# ==============================================================================

echo ""
echo "Packing AppImage..."

"$TOOLS_DIR/appimagetool-x86_64.AppImage" "$APPDIR" "$OUTPUT"

OUTPUT_SIZE=$(stat -c%s "$OUTPUT")
OUTPUT_SIZE_MB=$((OUTPUT_SIZE / 1024 / 1024))

echo ""
sha256sum "$OUTPUT"

echo ""
echo "=========================================="
echo "  Weave AppImage Built!"
echo "=========================================="
echo "Output: $OUTPUT"
echo "Size:   ${OUTPUT_SIZE_MB} MB"
