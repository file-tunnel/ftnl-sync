#!/usr/bin/env bash
# shellcheck shell=bash
set -euo pipefail

readonly MODE="${1:-all}"
readonly FMCTL_MANIFEST="opto-sync-clients/tools/fmctl/Cargo.toml"

git submodule status --recursive
cargo build --locked --release --manifest-path "$FMCTL_MANIFEST"

run_fmctl() {
  (
    cd opto-sync-clients
    tools/fmctl/target/release/fmctl "$@"
  )
}

case "$MODE" in
  check)
    run_fmctl validate
    run_fmctl check
    ;;
  simulate)
    run_fmctl simulate
    ;;
  verify)
    run_fmctl verify
    ;;
  all)
    run_fmctl validate
    run_fmctl check
    run_fmctl simulate
    run_fmctl verify
    ;;
  *)
    echo "usage: $0 {check|simulate|verify|all}" >&2
    exit 64
    ;;
esac
