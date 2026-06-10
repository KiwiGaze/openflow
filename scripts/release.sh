#!/usr/bin/env bash
# Local release build: run every check, then produce the macOS bundle.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> JS checks"
pnpm lint
pnpm format:check
pnpm typecheck
pnpm -r test

echo "==> Rust checks"
(
  cd apps/desktop/src-tauri
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
)

echo "==> Building bundle (this compiles whisper.cpp in release mode; be patient)"
pnpm tauri build

BUNDLE_DIR="apps/desktop/src-tauri/target/release/bundle"
echo
echo "==> Artifacts"
ls -lh "$BUNDLE_DIR"/dmg/*.dmg "$BUNDLE_DIR"/macos/*.app 2>/dev/null || true
echo
echo "Note: the bundle is unsigned. Distribute via GitHub releases (tag v*) or"
echo "right-click → Open locally. Signing/notarization is tracked in ROADMAP.md."
