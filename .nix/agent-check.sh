# shellcheck shell=bash
set -euo pipefail

git submodule status --recursive
npm test

(
  cd adapters/typescript
  npm ci --legacy-peer-deps
  npm test
)

(
  cd adapters/rust
  cargo fmt --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo test --locked --all-targets --all-features
)
