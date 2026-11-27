# Cycle 19: source-bound teardown traversal

Both `cio_chunk_close_stream` and `cio_stream_destroy_all` match complete typed
reference bodies in the pinned build. The check retains initialization, the
sentinel comparison, saved successor, container recovery, child argument,
non-deleting chunk call and loop increment. The null context return is retained;
chunk traversal requires a live stream. Empty, singleton and longer finite rings
share the same loop skeleton rather than an arbitrary unrolling limit.

The generic matcher compiles a reviewed, fully expanded reference **after** input
preprocessing. Changing an input loop/container macro cannot change the reference.
It compares the complete typed AST, normalizing local declaration identities and
source locations only (plus non-execution definition usage/storage metadata).
It preserves offsetof operands from that same preprocessed source because Clang
JSON omits them. Extra operations, changed conditions/calls, wrong members,
pointer identities, attributes and volatile/atomic access reject. This bounded
route may reject semantically equivalent C. Clang, JSON normalization and the
reviewed reference interpretation are explicit trusted boundaries.

The 11-clause native QCore policy states finite exclusive reciprocal rings, live
sentinels, distinct container provenance, and child call frames. A child must
unlink only its current node and preserve the saved next node and parent. Source
matching alone discharges no child custody and does not establish these premises.
Six Lean theorems prove order conservation, decreasing remaining length, exact
completion and no duplicate calls for the corresponding finite-list abstraction.
They do not prove C memory safety or the Rust matcher.

All 388 workspace tests, 40 contract modules, audit example tests, formatting,
Clippy with warnings denied, both reevaluation probes, Lean build/dependency
audit and the pinned external audit pass. An initial workspace run hit the
existing process-provider test's transient `Text file busy` spawn error; the
complete rerun passes. The generic diagnostic inventory, Monkey source sidecar,
scalar results and all 13 preceding teardown entries are unchanged. Two new
traversal entries are archived in `chunk-contracts.json.gz`. Aggregate verdicts
remain two Safe, 98 provisional Unsafe and 81 Unknown across 185 units, with four
data-only units and zero gateway failures. No new runtime execution is claimed.

Reproduction:

```sh
cargo test --workspace
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p qcore-compiler --example reevaluation_probe
cargo run --quiet -p qcore-compiler --example reevaluation_followup
(cd proof && lake build && lake env lean AuditCycle19.lean)
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-traversal-source.json /tmp/cycle19-traversal-diagnostics.json
```

Cycle 19 remains in progress. Enclosing stream/context allocation release,
per-child dependency and identity composition, full sync/caller/OS guarantees,
constructor provenance, aggregate custody and event composition remain open.
