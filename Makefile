.PHONY: all sidecar dev build clean app test

TARGET := $(shell rustc -Vv | grep host | awk '{print $$2}')
SIDECAR := src-tauri/binaries/kcci-server-$(TARGET)

all: build

# Build the Python sidecar with PyInstaller
sidecar: $(SIDECAR)

$(SIDECAR): kcci-server.py kcci-server.spec src/kcci/*.py
	@echo "Building sidecar for $(TARGET)..."
	@mkdir -p src-tauri/binaries
	uv run pyinstaller --clean --noconfirm kcci-server.spec
	cp dist/kcci-server $(SIDECAR)
	@echo "Sidecar built: $(SIDECAR)"

# Run in development mode
dev: $(SIDECAR)
	cargo tauri dev

# Build production app (includes DMG)
build: $(SIDECAR)
	cargo tauri build

# Build just the .app bundle
app: $(SIDECAR)
	cargo tauri build --bundles app

# Run Python tests
test:
	uv run pytest

# Clean all build artifacts
clean:
	rm -rf dist build
	rm -rf src-tauri/binaries
	rm -rf src-tauri/target
	@echo "Cleaned build artifacts"
