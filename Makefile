.PHONY: all dev build clean clean-all export-model app check fmt ui release release-patch release-minor release-major version-patch version-minor version-major

all: build

# Build Svelte frontend
ui:
	cd ui && npm run build

# Run in development mode
dev:
	rm -rf ui/node_modules/.vite
	cd ui && npm run dev &
	cargo tauri dev

# Build production app (includes DMG)
build: ui
	cargo tauri build

# Build just the .app bundle
app: ui
	cargo tauri build --bundles app

# Type check and lint
check:
	cargo check
	cargo clippy

# Format code
fmt:
	cargo fmt
	cd ui && npm run format 2>/dev/null || true

# Clean build artifacts (preserves ONNX model)
clean:
	rm -rf dist build
	rm -rf src-tauri/target
	rm -rf ui/dist ui/node_modules/.vite
	@echo "Cleaned build artifacts"

# Deep clean including ONNX model
clean-all: clean
	rm -rf src-tauri/binaries/onnx-model
	@echo "Cleaned ONNX model - run 'make export-model' to regenerate"

# Export ONNX model from HuggingFace (requires network, ~500MB download)
# Requires transformers and optimum packages (one-time setup)
export-model:
	pip install transformers 'optimum[onnxruntime]'
	python scripts/export-onnx-model.py

# =============================================================================
# RELEASE TARGETS
# Full release workflow: bump version, build, notarize, push tag, create release
#
# Prerequisites:
#   - Clean working copy (no uncommitted changes)
#   - Environment variables set: APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID
#   - gh CLI authenticated (run 'gh auth login' if needed)
#
# Usage:
#   make release-patch  # 0.3.1 -> 0.3.2
#   make release-minor  # 0.3.1 -> 0.4.0
#   make release-major  # 0.3.1 -> 1.0.0
# =============================================================================

release: release-patch

release-patch:
	@./scripts/release.sh patch

release-minor:
	@./scripts/release.sh minor

release-major:
	@./scripts/release.sh major

# Version bump only (without build/release) - useful for testing
version-patch:
	cd src-tauri && cargo release patch --execute

version-minor:
	cd src-tauri && cargo release minor --execute

version-major:
	cd src-tauri && cargo release major --execute
