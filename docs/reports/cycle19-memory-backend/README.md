# Cycle 19: source-derived memory backend custody

The pinned `cio_memfs_close` body now has an executable native contract in
`contract/qcore/chunkio/memory_backend.qcore`. Its complete typed source paths
are replayed over **nine realizable nullable-field states**: a null backend,
or a nonnull backend with each of its three child allocations independently
null or owned. All 13 postconditions hold in every state. The checker retains
release order and rejects duplicate release, missing release, parent release
before a live child, reads through null/released storage, root release, writes,
and unsupported expressions/calls, including unsupported unreachable effects.

The backend's name, content buffer, metadata buffer and object are discharged.
The borrowed chunk stays live and its fields are unchanged: in particular,
`ch->backend` can retain a dangling pointer after this function. This result is
not permission to close the same backend again. Caller provenance, exclusive
root access, pairwise-distinct owned-or-null fields, the C/QCore root binding,
and resolving `free` to libc are explicit assumptions. Constructors have not
yet established these premises automatically. Undeclared allocations are not
part of the bounded ownership inventory.

The symbolic C importer now lives in `compiler/src/typed_source.rs`; Monkey is
a profile facade retaining its existing public names. Record selection,
fixed-size type selection and restricted function signatures/call vocabulary
are supplied as syntax-profile data. Calls do not gain semantics from that
profile. `field_destructor.rs` supplies generic nullable-field replay, and the
ChunkIO plugin selects the actual backend source and contract. The source
summary's construction remains private. This reuse avoids duplicating Clang
expression, alias, branch and producer-provenance logic.

The certificate records the source projection, exact native contract, root
binding and all nine checked release traces. The audit stores it separately
under `backend_teardown` in the native chunk sidecar. Existing scalar results
still contain three checked and 93 Unknown functions; this new backend result
does not silently change that denominator or any whole-unit verdict.

## Validation and evidence

All **348 workspace tests** pass, including contract signature/premise mutations,
every postcondition changed to false/unsupported, alias renaming, alternate
null-test spelling and teardown source mutations. The 26-module native corpus
parses, round-trips and compiles. Formatting, warnings-denied Clippy, audit
example tests, both reevaluation probes and the pinned external audit pass.

Seven new audited Lean theorems cover null release, required live custody,
discharge, other-identity preservation, duplicate-release rejection, parent
release after children, and the accepted terminal field inventory. `lake build`
and `AuditCycle19.lean` pass without placeholders or project axioms. They prove
the abstract identity algebra; C/Clang import, the Rust replay and libc binding
remain explicit trusted boundaries.

The generic diagnostic inventory is byte-identical to the extraction baseline
(SHA-256 `d06fd972d9d328de794196a50a380b3ce7513a79767de9ec746fc788a4a07035`).
The complete Monkey source sidecar and all 96 native scalar-function results
are identical. The pinned inventory remains 185 units, 4,688/4,775 projected
functions, two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero
gateway failures. Projection is not safety coverage; provisional Unsafe is not
a confirmed upstream defect. Native observation code was unchanged.

See `parity.json`, `chunk-contracts.json.gz`, validation logs and `evidence.json`.

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo run --quiet -p qcore-compiler --example reevaluation_probe
cargo run --quiet -p qcore-compiler --example reevaluation_followup
(cd proof && lake build && lake env lean AuditCycle19.lean)
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-backend-source.json /tmp/cycle19-backend-diagnostics.json
```

Cycle 19 remains in progress. Full chunk close still needs both list unlinks,
context counter discharge, enclosing allocation release and filesystem backend
outcomes; stream/context iteration, constructor provenance, wrapper lifecycle
and Monkey event composition remain open. This result closes only the bounded
memory-backend field-custody part of the nested source teardown chain.
