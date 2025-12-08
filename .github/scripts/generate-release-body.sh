#!/bin/bash
# Generate release body markdown with all platforms

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
    --version)
        VERSION="$2"
        shift 2
        ;;
    --macos-sha256)
        MACOS_SHA256="$2"
        shift 2
        ;;
    --linux-sha256)
        LINUX_SHA256="$2"
        shift 2
        ;;
    --windows-sha256)
        WINDOWS_SHA256="$2"
        shift 2
        ;;
    --output)
        OUTPUT_FILE="$2"
        shift 2
        ;;
    *)
        echo "Unknown option: $1"
        exit 1
        ;;
    esac
done

# Create templates directory if it doesn't exist
mkdir -p .github/release-templates

if [ ! -f ".github/release-templates/header.md" ]; then
    cat >".github/release-templates/header.md" <<'HEADER'
# Weave v{{VERSION}}

Project scaffolding tool for MayaFlux applications.
HEADER
fi

if [ ! -f ".github/release-templates/macos-section.md" ]; then
    cat >".github/release-templates/macos-section.md" <<'MACOS'
**macOS Installer Package** - Includes Weave.app GUI and all project templates.
MACOS
fi

if [ ! -f ".github/release-templates/linux-section.md" ]; then
    cat >".github/release-templates/linux-section.md" <<'LINUX'
LINUX
fi

if [ ! -f ".github/release-templates/windows-section.md" ]; then
    cat >".github/release-templates/windows-section.md" <<'WINDOWS'
**Windows ZIP Archive** - Contains Weave.exe and project templates.
WINDOWS
fi

if [ ! -f ".github/release-templates/footer.md" ]; then
    cat >".github/release-templates/footer.md" <<'FOOTER'
## Documentation

See the [Weave documentation](https://github.com/mayaflux/weave) for usage instructions.
FOOTER
fi

# Generate release body
cat >"$OUTPUT_FILE" <<EOF
$(cat .github/release-templates/header.md)

## macOS Installer

$(cat .github/release-templates/macos-section.md)

**SHA256:** \`$MACOS_SHA256\`

## Linux Distribution Package

$(cat .github/release-templates/linux-section.md)

**SHA256:** \`$LINUX_SHA256\`

## Windows Installer  

$(cat .github/release-templates/windows-section.md)

**SHA256:** \`$WINDOWS_SHA256\`

$(cat .github/release-templates/footer.md)
EOF

if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/{{VERSION}}/$VERSION/g" "$OUTPUT_FILE"
else
    sed -i "s/{{VERSION}}/$VERSION/g" "$OUTPUT_FILE"
fi
