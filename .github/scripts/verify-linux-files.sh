#!/bin/bash

set -euo pipefail

echo ""
echo "========================================"
echo "  Verifying Linux Source Files"
echo "========================================"
echo ""

all_present=true

required_files=(
    "linux/lib/main.py"
    "linux/lib/modes/installation.py"
    "linux/lib/modes/project.py"
    "linux/lib/ui/theme.py"
    "linux/lib/ui/dark.css"
    "linux/scripts/build_appimage.sh"
    "linux/pyproject.toml"
    "linux/weave-config.json"
    "linux/resources/weave.png"
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
