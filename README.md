# ftnl-sync

Local-first upload intent and resumability for File Tunnel, built on a pinned
revision of [`opto-sync-clients`](https://github.com/opto-sync/opto-sync-clients).

This layer makes the UI optimistic without confusing “sync” with “copy every
file everywhere.” It persists job metadata and checkpoints in IndexedDB on the
web and SQLite on native platforms. File bytes, pairing secrets, phone/desktop
capabilities, event tickets, and presigned URLs are explicitly excluded.

## Why the boundary matters

File handles and native paths are device-local. Capabilities are secrets. Raw
photos can be huge and deeply personal. Putting any of them into a generic
replication envelope would create broken cross-device references, excessive
storage, and an avoidable security incident.

The synced record contains only:

- stable job, tunnel, and file IDs;
- display-safe file name, declared type, and byte count;
- lifecycle state and byte checkpoint;
- attempt count and redacted reason code;
- hybrid logical timestamps for deterministic merge.

The local SQLite/IndexedDB row additionally stores an opaque `local_ref` that a
platform adapter can resolve to a file handle. Capabilities stay in OS secure
storage or an in-memory browser session keyed by tunnel ID.

## Dependency model

`opto-sync-clients/` is a git submodule pinned to one reviewed commit. File
Tunnel adapters delegate ordering, offline mutation durability, deduplication,
rebase, and reconciliation to that exact core rather than reimplementing merge
semantics.

```bash
git clone --recurse-submodules https://github.com/file-tunnel/ftnl-sync.git
```

## State machine

```text
queued → declaring → uploading ⇄ paused → available → imported
   └──────────────→ failed ───────┘
   └──────────────────────────────→ cancelled
```

Transitions are monotonic except a retry from `failed` or `paused`. A server
snapshot always becomes the new base, then unconfirmed local mutations are
replayed using opto-sync's `localView` invariant so progress does not jump
backward on reconnect.

See [`docs/integration.md`](docs/integration.md) for browser/native integration
and [`schema/upload-job.schema.json`](schema/upload-job.schema.json) for the
replication-safe record.

## Validate

```bash
nix develop --command agent-check
```

The parent repository also runs the complete formal model pinned in the
`opto-sync-clients` submodule, plus Proptest and Kani checks for File Tunnel
upload persistence. See [`docs/formal-methods.md`](docs/formal-methods.md).

Clone with `--recurse-submodules` before entering the shell. The root Nix lock
pins Rust, Node.js, SQLite, and the repository automation tools; the
`opto-sync-clients` source remains an independently reviewed gitlink.

MIT licensed.
