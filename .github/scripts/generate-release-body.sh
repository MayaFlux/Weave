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

## Windows Installer  

$(cat .github/release-templates/windows-section.md)

**SHA256:** \`$WINDOWS_SHA256\`

$(cat .github/release-templates/footer.md)
EOF

# Replace version placeholders
sed -i "s/{{VERSION}}/$VERSION/g" "$OUTPUT_FILE"
