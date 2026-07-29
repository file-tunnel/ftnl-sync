# Formal verification

File Tunnel deliberately reuses the complete opto-sync proof instead of
maintaining a weaker copy. The pinned `opto-sync-clients` submodule contains:

- a Quint model of queue allocation, ambiguous commits, exact retry,
  acknowledgement, compaction, and crash-safe snapshot reset;
- exhaustive TLC verification and reachability witnesses;
- deterministic ITF traces; and
- Rust and TypeScript adapters that replay model traces against production
  client implementations.

The top-level formal-methods workflow initializes the recursive submodule and
runs its schema-v1 `fmctl` manifest, including simulation and exhaustive
verification. This is important because GitHub does not execute workflow files
that exist only inside a submodule.

The File Tunnel Rust adapter adds its own implementation-level obligations:
persisted upload progress cannot exceed declared size, required metadata remains
present, and replication JSON excludes capability secrets and file content.
Proptest explores the progress boundary and Kani proves it for every `u64`
input.

The parent workflow also executes the exact nested revisions' language-specific
proofs. CBMC checks the production opto-sync C timestamp comparator and malformed
FFI-strategy guard. Kani checks both the Rust protocol queue's allocation,
batch, and watermark bounds and the Rust/C binding's ABI and NUL preconditions.
This duplication is intentional: GitHub does not execute workflows stored only
inside either level of a nested submodule.

Run the inherited model with the same entry point used by CI:

```bash
nix develop --command bash scripts/formal-check.sh all

# Optional code-level bounded proof:
cargo install --locked kani-verifier --version 0.67.0
cargo kani setup
(cd adapters/rust && cargo kani)
```

The opto-sync model intentionally proves replication and crash semantics. The
File Tunnel capability/lifecycle model remains canonical in
`ftnl-backend-api.rs/formal` so the two models have disjoint ownership.
