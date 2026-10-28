#!/bin/bash

set -euo pipefail

echo ""
echo "========================================"
echo "  Verifying Linux Source Files"
echo "========================================"
echo ""

all_present=true

required_files=(
    "legacy/linux/lib/main.py"
    "legacy/linux/lib/modes/installation.py"
    "legacy/linux/lib/modes/project.py"
    "legacy/linux/lib/ui/theme.py"
    "legacy/linux/lib/ui/dark.css"
    "legacy/linux/scripts/build_appimage.sh"
    "legacy/linux/pyproject.toml"
    "legacy/linux/weave-config.json"
    "legacy/linux/resources/weave.png"
    "templates"
)

echo "Checking required files..."
echo ""

for file in "${required_files[@]}"; do
    if [ -e "$file" ]; then
        if [ -d "$file" ]; then
            item_count=$(find "$file" -type f | wc -l)
            echo "OK $file (directory, $item_count files)"
        else
            file_size=$(stat -c%s "$file")
            file_size_kb=$((file_size / 1024))
            echo "OK $file ($file_size_kb KB)"
        fi
    else
        echo "MISSING: $file"
        all_present=false
    fi
done

echo ""

if [ "$all_present" = true ]; then
    echo "========================================"
    echo "  All Required Files Present"
    echo "========================================"
else
    echo "ERROR: Some required files are missing"
    exit 1
fi
