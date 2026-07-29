# Integration guide

## Web

Use the pinned `@opto-sync/client` IndexedDB queue. Keep the browser `File`
object in memory during an active upload. If the host opts into resumability,
store a `FileSystemFileHandle` in a separate local-only IndexedDB store and
re-request permission after reload; never put it in the mutation payload.

The adapter queues `ftnl_upload_job` records. Render opto-sync's `localView`,
not the raw server reconciliation result, so an optimistic progress transition
does not disappear while its mutation is still pending.

## Native

Use the pinned Rust or Dart opto-sync SQLite client. Resolve `local_ref` through
the platform's security-scoped bookmark/content URI abstraction. Copy content
into app-owned temporary storage only when the OS grant is not durable, and
delete it on import, cancellation, or expiry.

Store phone/desktop capabilities separately:

- iOS/macOS: Keychain with the narrowest accessibility class;
- Android: Keystore-backed encrypted storage;
- desktop Rust: OS credential store;
- browser: in-memory or `sessionStorage` for the current tab only.

## Reconnect loop

1. Load local jobs and render `localView`.
2. Reacquire a usable local file reference.
3. Fetch a tunnel snapshot using the scoped capability.
4. Rebase pending local mutations over that snapshot.
5. Resume upload from the server checkpoint when multipart support is active.
6. On WebSocket sequence gaps, refetch rather than inventing missed state.
7. Mark imported only after the host application durably accepts the file.

At-least-once push requires stable `(clientId, mutationId)` identities. Never
regenerate them when a request times out.
