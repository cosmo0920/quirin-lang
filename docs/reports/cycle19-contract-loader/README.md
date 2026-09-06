# Cycle 19: declarative resource-contract loader

Date: 2026-09-06. Status: W1 resource-audit migration validated; Cycle 19 remains
in progress. The [generalization proposal](../../GENERALIZATION_PLAN.md) is
preserved verbatim; the [roadmap](../../ROADMAP_AFTER_CYCLE_9.md) records the
cross-cycle dependency order and open workstreams.

The pinned resource inventory is **byte-identical** before and after loading
contracts from native QCore files, and byte-identical to the archived
[resource-triage inventory](../cycle19-resource-triage/diagnostics-after.json.gz).
The callback-source reports are also byte-identical. See [parity.json](parity.json).

| Measure | Before | After |
| --- | ---: | ---: |
| Selected units | 185 | 185 |
| Raw source functions | 4,775 | 4,775 |
| Projected functions | 4,688 | 4,688 |
| Safe units | 2 | 2 |
| Provisional Unsafe units | 98 | 98 |
| Unknown units | 81 | 81 |
| Data-only units | 4 | 4 |
| Gateway failures | 0 | 0 |

Projection is not safety coverage. Provisional Unsafe classifications retain
unresolved diagnostics and are not confirmed upstream defects.

## Implemented boundary

`compiler/src/contract_loader.rs` consumes the existing source front end,
requires nonempty `module contract` inputs, and maps a versioned bounded
vocabulary into `ResourceFunctionContract`. Borrowed, Owned, Consumed, Released,
Out, Stored and Removed parameter modes and None, Owned and Reallocated result
modes are explicit. Every parameter and result must be specified exactly once;
aggregate owner and replacement references resolve to declared parameter indices.
The module header and [contract corpus guide](../../../contract/qcore/README.md)
define the complete mapping and its rejection rules.

`contract/cycle19-resource-contracts.json` selects five native modules containing
19 resource API assumptions. The Cycle 19 audit loads this selection at runtime;
it no longer constructs its production contract table in Rust. Existing
observation/lifecycle modules continue through their bounded semantic
interpreters. In particular `fluent_bit/resource_memory.qcore` is the resource
projection, whereas `fluent_bit/memory.qcore` retains its richer recorded
allocator postconditions. Neither interpreter silently discards the other's
predicates. Historical reevaluation examples remain regression probes with
explicit Rust inputs.

The new declarations make formerly implicit borrowed arguments explicit. Those
trailing Borrowed entries are semantically neutral in the existing resource
checker, so the serialized Rust table shape differs while all pinned verdicts
and diagnostics remain identical. A regression compares every previous mode
modulo these trailing entries. `rd_realloc` retains its owned first parameter,
borrowed size, and nullable replacement behavior. `cio_chunk_close` retains the
previous non-null release assumption; its detailed nested teardown is still a
separate gate. Signatures use opaque pointer projections rather than verified
C layouts or full ABI checking.

The loader rejects missing files, missing modes, duplicate inputs/functions
(including identical duplicates), unsupported versions, effects rows,
quantitative clauses, error/any-exit clauses, additional preconditions, arbitrary
predicates, non-null error sentinels and invalid parameter bindings. It returns
no partial environment. A failed load terminates the audit with an error rather
than producing results from a default table. Sentinel/conditional semantics
unrepresentable by `ResourceFunctionContract` require future interface work;
this increment does not approximate them.

Exact source texts and their paths are recorded as trusted assumptions in
`ResourceContracts`, which already travels into `LegacySafetyCertificate`.
Independent certificate checking reloads the recorded texts and requires exact
agreement with the contract table before rechecking the C projection. It does
not reread mutable files. Table mutation without matching recorded assumptions
is rejected. The optional inventory has a `.contracts.json` sidecar recording
the same selected sources for Safe, provisional Unsafe and Unknown results.
Moving these assumptions into data does not establish that the C implements them.

## Evidence and validation

- [Pinned audit output](source-audit.log) and [exact contract input sidecar](contract-inputs.json).
- [Parity comparison](parity.json): identical complete inventories and callback
  reports. The unchanged inventory is reused from the triage report rather than
  duplicated; the callback report is retained as [gzip JSON](callbacks.json.gz).
- [Workspace tests](tests.log): 331 pass, including eight loader tests.
- [Audit-report tests](report-tests.log): two pass.
- [Formatting](fmt.log) and [Clippy with warnings denied](clippy.log): pass.
- [Reevaluation probes](reevaluation.json) and [follow-up probes](followup.json):
  19 expectations pass; VP05 remains explicit Q701 and LC01 remains Unknown.
- [Lean build](lean-build.log) and [Cycle 19 dependency audit](lean-audit.log):
  pass, with no placeholders or project axioms. Existing resource transfer
  semantics and Lean definitions are unchanged. This is not a mechanized proof
  of the new Rust loader or C ABI bridge.
- [Artifact and input hashes](evidence.json) retain reproduction identity.

An initial workspace attempt encountered `Text file busy (os error 26)` in the
existing process-lifecycle test; the repeated workspace run passed. Native
allocator/socket harnesses were not rerun for this contract-table migration;
the pinned audit reran the existing source imports. Prior native observation
claims and limitations remain in the [native-rule report](../cycle19-native-rules/README.md).

## Reproduction and remaining work

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-loader-after-callbacks.json \
  /tmp/cycle19-loader-after-diagnostics.json

cargo test --workspace
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p qcore-compiler --example reevaluation_probe
cargo run --quiet -p qcore-compiler --example reevaluation_followup
(cd proof && lake build && lake env lean AuditCycle19.lean)
```

The default manifest is resolved from the compiler crate's repository root.
The sixth positional audit argument can select another resource manifest when
using all existing report/observation arguments. Missing external build inputs
can be recreated using the Cycle 19 validation instructions. A pre-migration
baseline was executed with the same source/build selection before replacing
the table; the archived triage inventory supplies a durable identical baseline.

The next work uses loaded contracts for 19B concrete source-to-identity custody
mapping and W4's validated internal-function summaries. 19C still requires
source-derived wrapper/ChunkIO lifecycle, error and nested teardown traces.
19D still requires unsupported wrapper imports, complete map provenance,
failure-path custody and full Cycle 18 event composition. 19E must compose
those completed checks. No new static event/lifecycle/plugin certificate was
issued. W2 profiles, W3 semantic dispatch, W5 provider extraction/parity, and W6
source-level CT mapping remain open; no CT case is marked complete by these
loader regressions. W7's roadmap update is recorded separately from its pending
namespace, model naming and Lean CI work.
