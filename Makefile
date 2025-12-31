.PHONY: all dev build clean clean-all export-model app check fmt ui release release-patch release-minor release-major

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

# Release targets (bump version, commit, tag, push)
# Usage: make release-patch (0.2.2 -> 0.2.3)
#        make release-minor (0.2.2 -> 0.3.0)
#        make release-major (0.2.2 -> 1.0.0)
release: release-patch

release-patch:
	cd src-tauri && cargo release patch --execute

release-minor:
	cd src-tauri && cargo release minor --execute

release-major:
	cd src-tauri && cargo release major --execute
