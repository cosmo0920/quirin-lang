# Cycle 19: extracted source and wrapper contracts

Date: 2026-09-06. The latest source findings now live in three native QCore
modules: **26 declarations**, with distinct executable and specification roles.
Cycle 19 remains in progress.

| Module | Declarations | Consumer |
| --- | ---: | --- |
| `library/source_projection.qcore` | 10 | Executed by both typed source importers through the shared predicate interpreter |
| `chunkio/error_state.qcore` | 1 | Checked against the complete source-validated `cio_error_reset` body in the pinned audit |
| `monkey/wrapper_paths.qcore` | 15 | Syntax-preserved behavioral specifications; behavioral/custody replay remains open |

The source policies replace the corresponding Rust acceptance comparisons for
evaluated enums, scan effect and initial-state gates, type matching, symbol and
loop-index provenance, read-only regions, and frame writes/declarations/dependencies.
Rust still decodes typed C, computes facts, enforces representational limits and
executes abstract control/storage transitions. The extraction does not move
arbitrary C semantics or grammar into predicates.

The helper contract records the reset value, explicit write footprint and lock
frame. Its checker validates the mutable Chunk borrow, explicit use-one mode,
unit result, required live/exclusive premises and supported clause kinds before
evaluating source-derived field facts. Wrong constants, unmentioned writes,
missing premises, unsupported predicates and signature changes reject it.
The live/exclusive and cross-unit source/build/ABI requirements remain supplied
premises; no C linker/ABI or arbitrary-pointer proof is claimed.

The 15 wrapper declarations preserve socketpair forwarding, allocation and
registration failures, cleanup call order, missing return/null checks, timed
wait branches and structured injection control. They do not assert that a
close/free call successfully discharged custody. These opaque behavioral
predicates are currently specifications, not executable source-safety results.
They are explicitly labeled as such in the source report. All three modules
remain outside the resource-contract manifest, whose vocabulary cannot express
these policies and conditional behaviors.

Audit sidecars now retain the exact source-policy and helper-contract inputs,
as well as the wrapper specification text and its unverified status. The
existing `cio_chunk_unlock` contract remains in `chunkio/lock_state.qcore`.

## Validation

All **345 workspace tests** pass. The 25-module corpus parses, round-trips,
compiles and pins every declaration and clause. Tests mutate every extracted
policy to false and unsupported predicates, flip every supplied acceptance fact,
and check helper contract/value/footprint/signature mutations. Another regression
requires the wrapper-spec names to match all 15 selected source functions.
Formatting, warnings-denied Clippy, audit-example tests, Lean build and the
Cycle 19 dependency audit pass. The existing Lean models are unchanged; this
increment does not claim a new proof of the Rust contract interpreter.

The pinned resource inventory is byte-identical to the scan-loop baseline:
185 selected units, 4,688/4,775 projections, two Safe, 98 provisional Unsafe,
81 Unknown, four data-only and zero gateway failures. The Monkey callback and
wrapper results and the 96 native chunk-function results are unchanged.
`cio_error_reset` is additionally checked as a dependency with three native
postconditions; it is not added to the 96-function coverage numerator.
Projection and provisional Unsafe remain distinct from safety proofs and
confirmed upstream defects. Native observation instrumentation was unchanged
and was not rerun for this extraction.

Evidence: [parity and contract inventory](parity.json), [source sidecar](source.json.gz),
[native chunk sidecar](chunk-contracts.json.gz), validation logs and
[evidence hashes](evidence.json).

Reproduction:

```sh
cargo test --workspace
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
(cd proof && lake build && lake env lean AuditCycle19.lean)
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-extract-source.json /tmp/cycle19-extract-diagnostics.json
```

The remaining 19B custody mappings, 19C complete lifecycle/nested teardown,
19D loop safety and wrapper/map/failure/event composition, and 19E combined
formal audit are unchanged. Extracting their known contracts does not close them.
