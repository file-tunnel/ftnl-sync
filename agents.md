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

## Repository-local Git worktrees

- Create or use a Git worktree only when the human operator explicitly authorizes it for the current task. Concurrency or a dirty checkout is not permission by itself.
- Put every authorized worktree at `<repository-root>/tmp/worktrees/<name>`; from the repository root, use `./tmp/worktrees/<name>`. Never place worktrees beside repositories or organization directories.
- Keep `tmp`, `temp`, `tmp/worktrees`, and `temp/worktrees` ignored in the repository-root `.gitignore`. Do not commit files from those directories.
- Relocate or remove a worktree only when the operator explicitly requests it. Before removal, preserve and publish intended changes, verify its commit is represented on the target branch, and confirm there are no tracked, untracked, ignored-sensitive, or in-use files that must survive. Remove it with `git worktree remove <path>` without `--force`; never delete a worktree directory with `rm`.
