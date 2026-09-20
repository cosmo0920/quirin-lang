# Cycle 19 priority implementation

Requested 2026-09-20; implementation base
`f7b010debd018a3516f919224afcb3ca6682f3f9`. The integration branch is
`codex/cycle19-priority-wave`. Three implementation/measurement workers use
GPT-5.6 Sol at medium reasoning effort in isolated worktrees.

## Selected work

* W42-INDIRECT-ACCESS: bounded scalar-member stores through tracked live bases,
  paired with the already integrated B02 condition evaluation.
* W42-MONKEY-CALLBACK-COVERAGE: total accounting for the selected forced-Libevent
  callbacks, with explicit Unknown reasons where composed certificates are absent.
* W42-UNPROJECTED-DEFINITIONS: reproduce and classify the raw-definition versus
  projected-function difference and measure the unit reason distribution.

The [plan](../../plans/cycle19-custody-cards/README.md) additionally defines
W42-CHUNK-LIFECYCLE-CERT, including the nested ChunkIO teardown dependencies.
That certificate work and static-helper summaries remain queued. No dispatch
or syntax-admission count closes Cycle 19.

## Fresh baseline

The release audit rebuilt from the unchanged source and exited **0** against
the pinned source and forced-Libevent compilation database. All **five**
artifacts reproduce the B-series baseline **byte for byte**, including raw
contract keys. [Byte identities](baseline-byte-identity.json) and
[command, source commit and binary digest](baseline-run.json) bind the run.
Private logs and raw outputs are under
`/home/cosmo/Documents/GitHub/qcore-c/private/w42-priority/baseline/`;
the release build log is `private/w42-priority/baseline-build.log`.

This BEFORE measurement is not a gate or an AFTER result for the new work.
Raw units remain unpromoted; provisional Unsafe results are not validated
upstream defects. Lean gateway models retain the extraction and Rust-model
correspondence premises in evidence rule 28.

The [lifecycle evidence inventory](lifecycle-inventory.json), reproduced by
`python3 docs/reports/w42-priority-integration/scripts/lifecycle_inventory.py
docs/reports/b-series-integration/artifacts/chunk-contracts.json.gz`, separates
the 66 input-chunk functions (1 checked field contract / 65 Unknown) from the
30 `cio_chunk.c` functions (2 checked field contracts / 28 Unknown). It lists
the 21 existing backend-teardown records without treating their presence as a
complete lifecycle certificate. The script exited 0 and records the source
revision and uncompressed artifact digest.

## Integration status

All three reviewed cards are integrated. The combined pinned audit and full
standing gate passed at source `f3f99df5d2dcdf27e9b73ecd81dd7fe126419ca4`. The syntax
slice, callback accounting and definition census are complete at the boundaries
below; Cycle 19 and its remaining certificates are not complete.

## Implementation boundaries and review

The scalar-store slice admits only an exact scalar `BASE->FIELD = RHS`
assignment through a declared record pointer that this function may bind.
The RHS cannot contain a call, assignment, increment/decrement or another
memory access. Transfer retains the existing alias-read obligation and
requires one known, non-null, live identity without nullable or unresolved
markers. Both successful and flow-refused stores preserve the resource
projection frame; a refusal retains `UnknownAlias`. This is the new
[gateway rule 5g](../../agents/gateway.md#constraint-5g), not a general
indirect-access implementation.

Review caught an RHS mutation boundary and a later overbroad frame poison.
The latter suppressed four baseline leaks in an intermediate run; that run
is preserved in the card's private evidence and excluded from acceptance.
The corrected source regression requires an unrelated allocation's `Leak(q)`
to remain alongside the refused store's `UnknownAlias(p)`. The independent
[review record](review.json) found no additional concrete defects in the
corrected rule, tests and abstract Lean model. Review is not extraction proof.

Callback accounting derives its three selected callbacks from the loaded
forced-Libevent profile. Missing, duplicate, extra or malformed inventory and
evidence rows reject the operation. The normal audit now emits an explicit
Cycle 18 `Unknown` reason for every callback without requiring optional native
replay. The focused replay test accounts for all eight selected source paths,
but all three callbacks remain Unknown because no source-to-`EventProgram`
projection or composed certificate exists. This is tracked as
`MONKEY-CALLBACK-CYCLE18-COMPOSITION`.

## Independent coverage confirmation

The integrator independently replayed the census at worker checkpoint
`22d3056ebadfc5cdfe0a2c67bb8b68694e320a71`; the command exited **0** and its
result matches the worker's committed census **byte for byte**. The
[confirmation record](census-confirmation.json) records the command, input
identity and result digest. All **87** raw definitions absent from the
projection have positive evidence of exclusion by the pinned historical
build. The census establishes **0** projection defects and leaves **0**
unresolved classifications; it does not expand the analyzed build scope.

The gap spans **24 full-path units**, or **23 distinct basenames**. In
particular, the two Monkey `mk_utils.c` paths must not be counted as one unit.
This measurement resolves the build-exclusion question without promoting any
function or unit verdict.

## Combined corpus result

The release audit at `f3f99df5d2dcdf27e9b73ecd81dd7fe126419ca4` exited **0**. The final
[run identity](artifacts/run.json), [artifact digests](artifacts/manifest.json),
[attribution checks](attribution.json) and [exact leaf delta](artifacts/delta.json.gz)
record the result. All five raw artifacts are archived under `artifacts/`.

| Measure | BEFORE | Combined AFTER |
|---|---:|---:|
| Safe functions | 221 | 223 |
| provisional Unsafe functions | 84 | 84 |
| Unknown functions | 4,529 | 4,527 |
| unsupported diagnostics | 49,735 | 49,371 |
| unknown_alias diagnostics | 4,309 | 4,665 |
| leak diagnostics | 272 | 272 |
| summary_conflict diagnostics | 1 | 1 |

Units remain **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**.
The unit reasons remain **136 unsupported-syntax / 1 incomplete-obligation-coverage
/ 48 without a reason code**. The exact source inventory is unchanged:
**4,921 raw definitions / 4,834 projected functions / 87 build exclusions**.

Only `ctraces/ctr_span.c::ctr_span_start_ts` and
`ctraces/ctr_span.c::ctr_span_start` move Unknown to Safe. The diagnostic
multiset removes **365** leaves and adds **357**, a net reduction of **8**:
**362** indirect-access refusals and **2** downstream uncontracted-call
refusals are removed, alongside one prior unresolved-`ctx` alias reason.
There are **350 exact-coordinate exchanges**, **12** other lexical removals,
and **seven downstream alias additions** in `mk_http_parser`. This is a
measured prediction miss: the frozen forecast expected 349 lexical removals,
zero exchanges and one mover. It does not validate the historical proxy's
whole population; both indirect-census follow-ups remain registered.

The combined diagnostics match the corrected isolated scalar-store output
**byte for byte**. The ChunkIO contracts, selected contracts and source-effects
artifacts match the root BEFORE **byte for byte**, with no path normalization.
The Monkey source sidecar differs only in schema **2→3** and the new
`callback_coverage` field: **3/3 callbacks accounted**, **8 selected paths**,
**0 checked native paths** in this no-binding run, and **3 explicit Unknown**
reasons. The source/build/profile identity remains attached. Optional focused
replay covers 8/8 paths, but neither route produces an event-composition
certificate.

The same-checkout result matters: the isolated worker's absolute contract
source-map keys and derived boolean-field identities differ with its worktree
path. Those raw differences remain disclosed in the card report; they are not
silently normalized into a byte-parity claim.

Reproduction commands (exit **0** each):

```sh
python3 docs/reports/w42-priority-integration/scripts/compare.py private/w42-priority/baseline/resource-diagnostics.json private/w42-priority/final-after/resource-diagnostics.json
python3 docs/reports/w42-priority-integration/scripts/verify_integration.py private/w42-priority/baseline private/w42-priority/final-after /tmp/agent-w42-indirect/private/w42-priority/after-frame-final
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w42-priority-integration/artifacts
```

The fresh [blocker census](blocker-census.txt) sizes future syntax work. The
87 excluded definitions are not cheap additions to this pinned Linux build.
All 269 prior follow-ups are retained; the lifecycle and callback-composition
entries bring the ledger to **271**. No raw unit is promoted, provisional
Unsafe remains unvalidated, and full-language conformance remains false.

The matrix's hand-maintained `cycle19_raw_units` row still described the older
C-GATEWAY-WAVE checkpoint although current status had advanced to B-series
evidence. Its exact previous contents are preserved in
[previous-corpus-row.json](previous-corpus-row.json); the current row now uses
this integration's primary artifacts. No historical report or archive was
rewritten.

## Final standing gate

The [gate](gate.json) and [command facts](gate-facts.json) bind the full run to
source `f3f99df5d2dcdf27e9b73ecd81dd7fe126419ca4`. The [release build](release-build.json)
and pinned audit also exited **0**. The gate ran outside ptrace with
LeakSanitizer enabled, `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned submodules initialized |
| `clang --version` | 0 | actual Clang 21.1.8 recording in this checkout |
| `cargo test --workspace --no-fail-fast` | 0 | **4,863 passed / 0 failed / 10 ignored**, 250 strict summaries over 248 targets (239 running + 9 doc) |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | passed |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | passed |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | **149 audits**; no project axioms or placeholders |
| five isolated workspace host checks | 0 each | **1 passed / 0 failed each**, exact commands in facts |
| matrix regeneration using this `gate.json` | 0 | 149 audit files, 271 follow-ups |
| `python3 -m unittest discover -s scripts/tests` after final regeneration | 0 | **313 tests** |

Private logs and direct exits are under `/home/cosmo/Documents/GitHub/qcore-c/private/w42-priority/combined-gate/`.
The Python suite ran after the provisional regeneration and again after the
final gate record was regenerated into the matrix; both direct exits were 0.
The final Python facts include the validated matrix digest. **122 provider
workspace tests are carried Unix evidence, not remeasured**; the separate
provider mutation row is likewise carried. The five applicable core-workspace
host-sensitive rows were each rerun in isolation without deadline changes.
The inherited summary-count and Clang-recording mechanism limitations remain.
The 10 ignored tests retain their host/profile boundaries, including the known
Windows pin-agreement issue.

No combined gate failure was hidden or replaced. The card reports preserve
and disclose the sandboxed LSan-under-ptrace failures, the scalar-store probe
exhaustiveness build failure, and the excluded intermediate corpus runs. All
published final counts come from the corrected runs, not those attempts.

## Reconciliation and remaining work

[Reconciliation](reconciliation.json) records all **15** swept items and their
verified locations: the final check exited **0**, verifying **23 content claims**.
The initial check caught **11** omitted carried decisions; all remain explicitly
deferred in the corrected record. Both check logs are retained under the
private gate directory as `reconciliation-check.log` and
`reconciliation-check-final.log`. No existing follow-up is retired. The residual sweep
reports **54 historical residual-shaped candidates**, with no untracked name
introduced by these three card reports; the scanner classifies none of them.
The reports each receive an `integration-gate.json` with the combined count and
an explicit disclosure that their colocated Clang recording is borrowed from
their isolated card; this report's Clang recording is the actual combined one.

`W42-CHUNK-LIFECYCLE-CERT` remains queued with the input-chunk and nested
ChunkIO teardown conditions. `MONKEY-CALLBACK-CYCLE18-COMPOSITION` remains
open despite complete accounting. General indirect access and static-helper
summaries also remain open. This is a prioritized implementation increment,
not a whole-wave cadence close or Cycle 19 completion.
