# Cycle 19: source-derived ChunkIO counter updates

The pinned audit now checks all four `cio_chunk_counter_total_{add,sub}` and
`cio_chunk_counter_total_up_{add,sub}` helpers against the four native contracts
in `contract/qcore/chunkio/counters.qcore`. The source-derived certificate binds
the root parameter, selected unsigned field, exactly one increment/decrement,
and a return of the updated field. Extra writes, wrong fields, stale/constant
returns, unsupported branches/effects, qualified fields and signed types reject.
The complete typed paths and exact QCore inputs accompany each certificate.

The shared source importer retains typed unsigned increment/decrement and
resolves scalar return typedefs through the same Clang input. Existing int
increment and Monkey projections are unchanged. This is syntax/operational
projection; unsigned width is not inferred from the checker host.

Contracts require live/exclusive storage, a correct C-unsigned/usize ABI
binding and entry range. Increment additionally requires room below the maximum;
decrement requires a positive value. The C helpers themselves have no bounds
checks. The generic certificate's `instantiate(value, maximum)` checks these
bounds before computing the exact result and leaves its input untouched on
failure. The caller must bind `maximum` to the selected C ABI. No theorem of
arbitrary callers or automatic constructor/close counter balance is claimed.

All **351 workspace tests**, formatting, warnings-denied Clippy, audit example
tests, both reevaluation probes, Lean build and Cycle 19 dependency audit pass.
Tests cover zero/max boundaries including u128 overflow, changed update and
return identities, extra/unreachable effects, signed/qualified/bitfield types,
missing domain/ABI premises, signature changes and false/unsupported predicates.
The native corpus now contains 27 modules. Six Lean theorems prove boundary
rejection, exact nonwrapping arithmetic and increment/decrement conservation
for the abstract counter model; C/Clang and Rust translation remain trusted.

The pinned generic inventory is byte-identical to the memory-backend baseline.
The complete Monkey sidecar, all 96 scalar chunk-function results and the
nine-state memory-backend certificate are identical. Four counter certificates
are separately retained under `backend_teardown` with `kind: unsigned_counters`.
No whole-unit safety classification changes: 185 units, 4,688/4,775 projected,
two Safe, 98 provisional Unsafe, 81 Unknown, four data-only, no gateway failures.
Projection and provisional Unsafe are not proof coverage or confirmed defects.

Evidence is in `parity.json`, `chunk-contracts.json.gz`, the validation logs and
`evidence.json`. Reproduction:

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
  /tmp/cycle19-counter-source.json /tmp/cycle19-counter-diagnostics.json
```

Cycle 19 remains in progress. Connecting these updates to the concrete close
call's saved context identity, both list unlinks, filesystem backend outcomes,
stream/context iteration and higher-level lifecycle/event composition remains
open. Native observation instrumentation was unchanged.
