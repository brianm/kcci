#!/bin/bash
# Full release workflow: bump version -> build -> notarize -> push tag -> GitHub release
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TAURI_DIR="$PROJECT_ROOT/src-tauri"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Validate release level argument
LEVEL="${1:-}"
if [[ ! "$LEVEL" =~ ^(patch|minor|major)$ ]]; then
    echo "Usage: $0 <patch|minor|major>"
    echo "  patch: 0.3.1 -> 0.3.2"
    echo "  minor: 0.3.1 -> 0.4.0"
    echo "  major: 0.3.1 -> 1.0.0"
    exit 1
fi

# Check required environment variables for notarization
check_notarization_env() {
    local missing=()
    [[ -z "${APPLE_ID:-}" ]] && missing+=("APPLE_ID")
    [[ -z "${APPLE_PASSWORD:-}" ]] && missing+=("APPLE_PASSWORD")
    [[ -z "${APPLE_TEAM_ID:-}" ]] && missing+=("APPLE_TEAM_ID")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing environment variables for notarization: ${missing[*]}"
        log_error "Set these in your shell profile or export before running"
        exit 1
    fi
    log_info "Notarization credentials found"
}

# Check for uncommitted changes
check_clean_working_copy() {
    if ! jj status 2>/dev/null | grep -q "The working copy is clean"; then
        log_error "Working copy has uncommitted changes. Commit or abandon them first."
        jj status
        exit 1
    fi
    log_info "Working copy is clean"
}

# Check gh CLI is authenticated
check_gh_auth() {
    if ! gh auth status &>/dev/null; then
        log_error "GitHub CLI not authenticated. Run 'gh auth login' first."
        exit 1
    fi
    log_info "GitHub CLI authenticated"
}

# Get version from Cargo.toml
get_current_version() {
    grep '^version = ' "$TAURI_DIR/Cargo.toml" | sed 's/version = "\(.*\)"/\1/'
}

# Step 1: Bump version with cargo-release
bump_version() {
    log_info "Bumping version ($LEVEL)..."
    local old_version=$(get_current_version)

    cd "$TAURI_DIR"
    if ! cargo release "$LEVEL" --execute --no-confirm; then
        log_error "Version bump failed"
        exit 1
    fi
    cd "$PROJECT_ROOT"

    NEW_VERSION=$(get_current_version)
    log_info "Version bumped: $old_version -> $NEW_VERSION"
}

# Step 2: Build and notarize
build_notarized() {
    log_info "Building and notarizing app (this may take several minutes)..."

    # Build frontend first
    cd "$PROJECT_ROOT/ui"
    if ! npm run build; then
        log_error "Frontend build failed"
        exit 1
    fi

    cd "$PROJECT_ROOT"
    if ! cargo tauri build 2>&1 | tee /tmp/tauri-build.log; then
        log_error "Tauri build failed. Check /tmp/tauri-build.log for details"
        exit 1
    fi

    # Find the DMG (architecture may vary)
    DMG_PATH=$(find "$TAURI_DIR/target/release/bundle/dmg" -name "*.dmg" | head -1)
    if [[ -z "$DMG_PATH" || ! -f "$DMG_PATH" ]]; then
        log_error "No DMG found in: $TAURI_DIR/target/release/bundle/dmg/"
        ls -la "$TAURI_DIR/target/release/bundle/dmg/" 2>/dev/null || true
        exit 1
    fi

    log_info "Build complete: $DMG_PATH"

    # Check if notarization succeeded (look for stapling message in log)
    if grep -q "Stapling" /tmp/tauri-build.log; then
        log_info "Notarization and stapling successful"
    else
        log_warn "Notarization may not have completed - check build log"
    fi
}

# Step 3: Push tag to GitHub
push_tag() {
    log_info "Pushing tag v$NEW_VERSION to GitHub..."

    # cargo-release creates git tags directly, push via git
    if ! git push origin "v$NEW_VERSION"; then
        log_error "Failed to push tag v$NEW_VERSION"
        log_warn "You may need to push manually: git push origin v$NEW_VERSION"
        exit 1
    fi

    # Also push the main bookmark
    if ! jj git push --bookmark main 2>/dev/null; then
        log_warn "Could not push main bookmark - may already be up to date"
    fi

    log_info "Tag pushed successfully"
}

# Step 4: Create GitHub release
create_github_release() {
    log_info "Creating GitHub release..."

    DMG_PATH=$(find "$TAURI_DIR/target/release/bundle/dmg" -name "*.dmg" | head -1)

    if ! gh release create "v$NEW_VERSION" \
        --title "v$NEW_VERSION" \
        --generate-notes \
        "$DMG_PATH"; then
        log_error "Failed to create GitHub release"
        log_warn "You can create it manually: gh release create v$NEW_VERSION $DMG_PATH"
        exit 1
    fi

    log_info "GitHub release created: https://github.com/brianm/ook/releases/tag/v$NEW_VERSION"
}

# Main workflow
main() {
    log_info "Starting release workflow ($LEVEL)"
    echo ""

    check_notarization_env
    check_clean_working_copy
    check_gh_auth

    echo ""
    log_info "=== Step 1/4: Version Bump ==="
    bump_version

    echo ""
    log_info "=== Step 2/4: Build & Notarize ==="
    build_notarized

    echo ""
    log_info "=== Step 3/4: Push Tag ==="
    push_tag

    echo ""
    log_info "=== Step 4/4: GitHub Release ==="
    create_github_release

    echo ""
    log_info "=========================================="
    log_info "Release v$NEW_VERSION complete!"
    log_info "=========================================="
    echo ""
    DMG_PATH=$(find "$TAURI_DIR/target/release/bundle/dmg" -name "*.dmg" | head -1)
    echo "DMG: $DMG_PATH"
    echo "Release: https://github.com/brianm/ook/releases/tag/v$NEW_VERSION"
}

main
