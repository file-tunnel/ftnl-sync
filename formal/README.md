# File Tunnel bounded replay and AttoLang admission

Run `node --test tests/formal-upload-replay.test.mjs` or `npm test`.

The model exhaustively enumerates a **finite abstraction** of upload lifecycle,
stale server checkpoints, retries, imports, and cancellation. It reads the real
`schema/upload-job.schema.json` to enforce the replicated-field allowlist.
The key safety properties are monotonic byte checkpoints, bounded attempts,
and absorbing terminal states. Witnesses ensure these actions are reachable.

`upload_guards.atto` is an AttoLang (`.atto`) source-level policy for
checkpoint admission and metadata-only replication. It uses snake_case names.
**It is not yet compiled or executed**, and the JS abstract model is not
implementation-level proof. Before promoting AttoLang support to executable
conformance, compile it using an exact AttoLang backend commit, exercise the
negative/positive vectors against the compiled functions, and compare live
`ftnl-sync`/Opto-Sync storage traces. Do not copy capability-bearing fields into
the replication envelope, or substitute an unpinned submodule revision.
