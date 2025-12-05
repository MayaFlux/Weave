#!/bin/bash
# File: .github/scripts/verify-linux-files.sh
# Verify Linux source files before build

set -euo pipefail

echo ""
echo "========================================"
echo "  Verifying Linux Source Files"
echo "========================================"
echo ""

all_present=true

required_files=(
    "linux/lib/main.py"
    "linux/lib/cli.py"
    "linux/lib/modes/installation.py"
    "linux/lib/modes/project.py"
    "linux/lib/ui/theme.py"
    "linux/lib/ui/dark.css"
    "linux/scripts/create_project.sh"
    "linux/scripts/install_deps.sh"
    "linux/scripts/build_distribution.sh"
    "linux/pyproject.toml"
    "linux/Weave"
    "templates"
)

echo "Checking required files..."
echo ""

for file in "${required_files[@]}"; do
    if [ -e "$file" ]; then
        if [ -d "$file" ]; then
            item_count=$(find "$file" -type f | wc -l)
            echo "✅ $file (directory, $item_count files)"
        else
            file_size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file")
            file_size_kb=$((file_size / 1024))
            echo "✅ $file ($file_size_kb KB)"
        fi
    else
        echo "❌ Missing: $file"
        all_present=false
    fi
done

echo ""

if [ "$all_present" = true ]; then
    echo "========================================"
    echo "  All Required Files Present"
    echo "========================================"
    echo ""
else
    echo "❌ Some required files are missing"
    exit 1
fi
