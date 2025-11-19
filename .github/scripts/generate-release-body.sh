#!/bin/bash

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

# Generate release body
cat <<EOF >"$OUTPUT_FILE"
$(cat .github/release-templates/header.md)

## macOS Installer

$(cat .github/release-templates/macos-section.md)

**SHA256:** \`$MACOS_SHA256\`

**Installation:**
1. Download and open \`Weave-${VERSION}.pkg\`
2. Follow the installer prompts (installs Weave.app and files)
3. Terminal will open automatically with installation progress
4. Watch real-time progress for MayaFlux download and dependency installation

**What's installed:**
- Weave.app in \`/Applications\` (GUI project creator)
- Project templates and CLI tool in \`/Library/Weave\`
- MayaFlux framework to \`/Library/MayaFlux\` (via post-install script)
- Dependencies via Homebrew (via post-install script)

## Windows Installer  

$(cat .github/release-templates/windows-section.md)

**SHA256:** \`$WINDOWS_SHA256\`

$(cat .github/release-templates/footer.md)
EOF

# Replace version placeholders
sed -i '' "s/{{VERSION}}/$VERSION/g" "$OUTPUT_FILE"
