#!/bin/bash
# Sync version to ui/package.json
# Called by cargo-release as a pre-release hook

set -e

VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PACKAGE_JSON="$SCRIPT_DIR/../ui/package.json"

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

# Update package.json version using sed (works on macOS and Linux)
sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$PACKAGE_JSON"
rm -f "$PACKAGE_JSON.bak"

# Stage the change so it's included in the release commit
git add "$PACKAGE_JSON"

echo "Updated ui/package.json to version $VERSION"
