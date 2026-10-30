#!/usr/bin/env bash
set -euo pipefail
# Assemble the existing packaged template layout from the shared source catalog.
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
DESTINATION="${1:?Usage: stage-resources.sh <weave-resource-directory>}"
mkdir -p "$DESTINATION/templates"
cp -R "$ROOT_DIR/resources/templates/project/." "$DESTINATION/templates/"
mkdir -p "$DESTINATION/templates/community"
cp -R "$ROOT_DIR/resources/templates/community/." "$DESTINATION/templates/community/"
for scripts in "$ROOT_DIR"/resources/templates/scripts/*; do
    cp -R "$scripts/." "$DESTINATION/templates/"
done
