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

Implementation and review are in progress. Final changes, measured deltas,
gate results and reconciliation decisions must be recorded before completion.

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
