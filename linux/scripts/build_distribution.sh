#!/bin/bash
DIST="build/linux/Weave"

rm -rf "$DIST"
mkdir -p "$DIST"/{lib/weave,templates,scripts}

cp -r linux/weave/* "$DIST/lib/weave/"
cp -r templates/* "$DIST/templates/"
cp linux/scripts/{create_project.sh,install_deps.sh} "$DIST/scripts/"
chmod +x "$DIST/scripts/"*

cat >"$DIST/Weave" <<'EOF'
#!/bin/bash
D="$(cd "$(dirname "$0")" && pwd)"
export PYTHONPATH="$D/lib:${PYTHONPATH:-}"
export WEAVE_TEMPLATE_DIR="$D/templates"
export WEAVE_SCRIPT_DIR="$D/scripts"
exec python3 -m weave.main "$@"
EOF
chmod +x "$DIST/Weave"

tar czf "$DIST.tar.gz" -C build/linux "Weave"
sha256sum "$DIST.tar.gz"
