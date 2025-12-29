#!/bin/bash
# Build the Python sidecar for Tauri

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Determine target triple
TARGET=$(rustc -Vv | grep host | awk '{print $2}')
echo "Building for target: $TARGET"

# Create output directory
mkdir -p src-tauri/binaries

# Run PyInstaller
echo "Running PyInstaller..."
uv run pyinstaller --clean --noconfirm kcci-server.spec

# Copy to Tauri binaries with correct name
echo "Copying to src-tauri/binaries/kcci-server-$TARGET"
cp dist/kcci-server "src-tauri/binaries/kcci-server-$TARGET"

echo "Done! Sidecar built at: src-tauri/binaries/kcci-server-$TARGET"
