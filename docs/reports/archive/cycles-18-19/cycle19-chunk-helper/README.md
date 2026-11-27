# Cycle 19: source-validated ChunkIO frame helper

Date: 2026-09-06. Status: bounded 19C dependency/field-contract increment
complete; Cycle 19 remains in progress.

The pinned `cio_chunk_unlock` now has a checked native QCore contract, using
the independently imported body of `cio_error_reset`. The helper clears
`error_n` before both unlock exits. For entry lock=false, unlock returns -1
without a lock write; for lock=true it clears lock and returns 0. Busy remains
unchanged. Four busy/lock Boolean pre-states are checked under explicit live,
exclusive-access and Boolean-domain assumptions.

There are now **three checked native field contracts among 96 selected source
functions**: `cio_chunk_is_locked`, `cio_chunk_unlock`, and
`flb_input_chunk_release_lock`. The ChunkIO file has 2 checked and 28 Unknown;
the Fluent Bit wrapper has 1 checked and 65 Unknown. The helper is a separately
validated dependency, not an extra function counted in the 96-function scope.

## Evidence and dependency boundary

`import_chunk_frame_helper` checks the helper's entire typed body as constant
assignments to ordinary int metadata fields of its single borrowed ChunkIO
parameter. Lifecycle-field writes, reads of initial fields, aliases, calls,
branches, returns/hidden effects, wrong pointer bindings, qualified fields,
layout mismatches, duplicate dependencies, and local helper definitions reject
composition. It does not recognize helper behavior from its name. The external
audit routes the actual `cio_error.c` build entry into this checker.

Helper preprocessed source text travels with the privately constructed summary
and native contract certificate. Every written field must appear in the QCore
postconditions under `field_NAME`; absent/unknown values, wrong postconditions
and const borrows reject the result. Changing the helper's constant to 1 changes
the checked behavior and rejects the zero-reset contract.

The direct record field names/types must match between source units. This is
not a proof of complete cross-unit ABI equality, nested record layout or linker
resolution. The pinned audit supplies the revision/build/ABI boundary; arbitrary
clients must supply that evidence. Clang, its AST transport, the Rust bridge,
caller identity, exclusivity and live-chunk premises remain trusted boundaries.
This bounded nonrecursive helper check does not close W4's general SCC/fixed-point
custody inference.

Source metadata writes remain explicit in `frame_writes`. The lifecycle model
tracks lock/busy and resource obligations, not error storage. Instantiation emits
an Unlock at the supplied live identity; a regression composes it with the
existing model teardown. This is not an import of the caller's whole C path or
the native nested destructor chain. No whole-unit Safe claim is issued.

The audit now emits `RESOURCE_REPORT.chunk-contracts.json` with every checked
or Unknown result, exact native contracts and helper dependency sources. The
archived [gzip sidecar](chunk-contracts.json.gz) contains all 96 results.

## Validation

All 340 workspace tests, audit-example tests, formatting, warnings-denied
Clippy, Lean build and Cycle 19 dependency audit pass. Five new Lean theorems
cover abstract metadata-write retention, both unlock results and lock/busy/other
identity preservation. They do not prove the C/Rust bridge or general teardown.

The complete generic resource inventory remains byte-identical: 185 selected
units, 4,688/4,775 projections, two Safe, 98 provisional Unsafe, 81 Unknown,
four data-only and zero gateway failures. The Monkey callback/wrapper report is
also byte-identical to the preceding 14-wrapper/81-path run. Projection is not
proved coverage and provisional Unsafe is not a confirmed upstream defect.
Native instrumentation was unchanged and was not rerun in this increment.

Reproduction:

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-chunk-helper-source.json /tmp/cycle19-chunk-helper-diagnostics.json
cargo test --workspace
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
(cd proof && lake build && lake env lean AuditCycle19.lean)
```

Remaining Cycle 19 gates include 19B concrete custody mapping; 19C lock/sync,
residency, wrapper errors, nested teardown and complete lifecycle traces;
19D inject's variable-bound loop, map provenance, failure custody and full
Cycle 18 composition; and 19E their combined formal audit. None is closed by
this extra field contract.
