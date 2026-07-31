# File Tunnel sync agent instructions

These instructions apply to this repository and every directory beneath it.

## Repository role

- This repository owns local-first upload intent and resumability metadata.
- Keep file bytes, capabilities, pairing secrets, event tickets, presigned
  URLs, native paths, and browser file handles out of replicated envelopes.
- Keep `local_ref` device-local and opaque. Replicate only the documented
  display-safe metadata and lifecycle/checkpoint fields.
- Preserve monotonic transitions and replay unconfirmed mutations over a
  server snapshot without allowing progress to move backward.
- Treat `opto-sync-clients` and its nested `syncer.c` dependency as reviewed,
  immutable gitlinks; do not silently substitute branch heads.

## Validation

- Initialize submodules recursively, then run
  `nix develop --command agent-check` before completing a change.
- Run the formal boundary checks for persistence, reconciliation, or state
  transition changes.
- Never commit credentials, local database contents, file handles, user
  content, generated build trees, or dirty submodules.

## Git workflow

- Keep changes focused and reviewable.
- Pull and merge remote work before pushing; avoid git rebase in favor of git merge.
- Never discard unrelated or uncommitted user work.
