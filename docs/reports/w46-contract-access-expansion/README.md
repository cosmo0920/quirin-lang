# W46 contract and typed-statement expansion

Integration in progress from `207cebe4`. The [prediction](PREDICTION.md)
precedes merging the three independent Sol 6 medium implementation cards.
Astra owns gap selection, source-boundary decisions and combined measurements.

## Baseline and selected gaps

The fresh corpus baseline at `29b799f9` (documentation changes only since
`207cebe4`) ran with eight jobs and exited 0. All five artifacts reproduce
the preceding expansion's AFTER byte for byte: diagnostics, source,
contracts, chunk contracts and source effects. The fresh system slice at
`046accf6`, four jobs, also exited 0 with diagnostics, contracts and both
effect artifacts byte-identical. It retains two refutations and ten held
caller sites. Identity records are in `census/baseline-*-identity.json`;
the [archive](artifacts/baseline/manifest.json) records file digests and
commands. Private logs are `private/w46-next/baseline/`. The system run
records `e8052e39`, another documentation-only commit made after the corpus
started; no analyzer, provider or profile bytes changed during either run.

The [selected-call census](census/selected-before.json) finds 33 named
diagnostic leaves in the corpus (32 for the six CMetrics getters, one for
the chunk magic-byte helper) and two in the system slice (the magic-byte
helper in two configurations). These are call-site observations, not an API
completeness claim or predicted safety coverage. The remaining four Fluent
Bit query declarations have focused caller controls even though this
particular baseline does not name them in diagnostics.

## Contract integration

| Card | New declarations | Branch tip | Main merge |
|---|---:|---|---|
| [Metric aggregate getters](../w46-metric-aggregate-contracts/README.md) | 6 | `4767d3b1` | `4d73ef16` |
| [Fluent routing queries](../w46-fluent-routing-query-contracts/README.md) | 5 | `65d717b4` | `bfa510b7` |

The [tracked declaration diff](census/added-contracts.json) records all eleven
names. Merge conflicts in the two corpus test files were resolved by taking
both sets of entries and deliberately changing the declaration pin to 230
(219 + 6 + 5). Modules remain 118, selected modules 54, rejected legacy
declarations 216. There are 125 pinned declarations and 105 flexible libc
premises. Inventory synchronization and checking both exited 0 after merge.
The metric active-profile digest is the complete SHA-256 of its manifest.
The contracts add no analyzer admission rule.

The metric card's first full gate had three Rust failures: two stale count/list
expectations and one truncated manifest digest. Those card errors were repaired;
the affected suites passed 10/10 and 12/12. The routing card's first full gate
had one Rust failure from `plugin-spawn: Text file busy (os error 26)` and a
stale documentation index. The exact plugin test passed 1/1 in isolation under
its captured Cargo environment, and the regenerated index passed. The original
fixture is preserved in `private/w46-next/routing-environment-original-fixture/`;
the retry command and logs are in `private/w46-next/routing-environment-isolated/`.
All original nonzero exits remain recorded in the individual reports. These
targeted repairs do not stand in for the final merged gate.

## Contract-only measurements

Both release builds and audits at `bfa510b7` exit 0. Corpus and system each
use four workers; the independent statement-card audit also uses four, and
no standing tests run during these measurements. Commands, binary digests,
Clang selections and timestamps are in the [archive](artifacts/contracts-only/manifest.json).
The corpus source and chunk artifacts remain byte-identical. In all 15 system
compile configurations, only resource-contract source dependencies and their
derived inventory digests change ([comparison](census/contracts-system-configuration-identity.json)).
Private logs are `private/w46-next/contracts-only/`.

| Measure | Baseline | Contracts only |
|---|---:|---:|
| Corpus functions: Safe / provisional Unsafe / Unknown | 411 / 9 / 4,414 | unchanged |
| Corpus units: Safe / provisional Unsafe / Unknown / data-only | 5 / 8 / 168 / 4 | unchanged |
| Corpus unsupported / unknown_alias / leak | 41,086 / 3,573 / 34 | 41,045 / 3,575 / 34 |
| Corpus uncontracted leaves on Unknown functions | 9,022 | 8,998 |
| System configuration rows: Safe / provisional Unsafe / Unknown | 54 / 4 / 390 | unchanged |
| System unique projected functions: Safe / provisional Unsafe / Unknown | 28 / 2 / 204 | unchanged; 3 excluded |
| System unsupported / unknown_alias / counted_user | 3,303 / 713 / 4 | 3,291 / 712 / 4 |
| System uncontracted leaves on Unknown functions | 717 | 715 |

Corpus changes affect 22 functions: 42 leaves removed, three added. The system
changes affect 12 function/configuration rows: 15 leaves removed, two added.
Neither scope has a new leak/violation or a Safe correction. The new refusals
are an embedded formal expression in `pack_histogram_metric`, unresolved
`summary` in `pack_summary_metric`, and unresolved `in` in
`flb_input_chunk_set_limits` (also in both system configurations). Declaring
a query does not discharge those caller obligations.

The preceding expansion's `scripts/attribute.py`, given this card's added-name
inventory and the exact source root, exits 1 in each scope because its
automatic matcher leaves two corpus and three system differences unclassified.
They are all source-reviewed in
[reviewed-contract-attribution.json](census/reviewed-contract-attribution.json):
two multiline `snprintf` calls whose new histogram getter is below the
diagnostic's first line, and three downstream `ctx` identity refusals in
`cb_sb_init` removed when the mask-slot query no longer invalidates aliases.
The [identity checker](scripts/check_contract_attribution.py) exits 0 for each
scope with exact review coverage and verified source excerpts. It checks
record identity, not the semantic correctness of the written review. The
automatic results and summaries are in `census/contracts-{corpus,system}-*.json`.
The [refutation comparison](census/contracts-refutation-preservation.json)
is exactly equal as a JSON value: two declarations, three held
functions and ten held sites. No lifecycle result is promoted.

## Worktree sweep

The user requested outdated worktrees be swept while the cards run. Seven
clean, merged and unlocked worktrees were removed using `git worktree remove`:
`fb-system-audit-cards`, `agent-a31872259c7e9850d`, `hungry-thompson-1740da`,
`w46-access-path-census-launch-8ba0f2`, and the previous expansion's
`qcore-w46-fluent-events`, `qcore-w46-libc-events`, `qcore-w46-metric-trace`.
All branch refs remain. Eight locked worktrees, one worktree containing an
untracked fault-injection report, the main checkout and all three active cards
are preserved.

Before removal, ignored private evidence was archived and every preserved file
was checked by SHA-256 (symlink targets checked separately). Only disposable
`target/`, `providers/target/`, `proof/.lake/` and `__pycache__` caches were
excluded. Each removal rechecked a clean status, unchanged head, ancestry in
main, unlocked state and archive digest. All seven removals exit 0. Inventory,
per-file manifests, tar archives and removal results live in
`private/w46-next/worktree-sweep/`. This directory is the relocation index for
old worktree-relative private log paths; frozen reports are unchanged.
Observed free space on the main filesystem rose from 97 GiB to 157 GiB during
the sweep (other builds were running, so this is not exact per-file accounting).

## Typed-statement integration boundary

The statement card adds a Clang-typed fallback after all existing shape routes.
It binds the exact analyzed source, dialect, frontend context, statement tokens
and line; missing or ambiguous facts retain Unknown. Field declarations must
have a known complete record owner. Union fields, pointer-valued stores,
bitfields, volatile/atomic paths, and unsupported casts or calls remain refused.
Tracked arrow chains use the existing per-hop obligations; mixed embedded or
indexed paths require a root that the static custody closure proves never
bound. A pointer-valued read introduces a borrowed alias without a tracked
target, so later uses can still be unresolved. No array-bounds or whole-C
correctness claim follows from this resource projection.

Review found that testing type spellings for `union` missed a typedef-hidden
union. The corrected reader indexes actual field-declaration owners; negative
controls cover typedef and nested unions. A separate Lean review required the
borrowed-read lowering to refuse a write frame, matching the reader's reachable
plans. The earlier corpus child completed with exit 0 just before an attempted
interruption reached its wrapper, which exited 130. Its artifacts are retained
as superseded evidence from the defective reader, not final validation. Both
fixed audits complete at implementation tip `753cde34`, exit 0, four workers
each. The corrected corpus has 451 Safe / 9 provisional Unsafe / 4,374 Unknown
functions: 40 gains. `rdcrc32.c` moves Unknown to Safe, giving 6 Safe / 8
provisional Unsafe / 167 Unknown / 4 data-only units. Unsupported leaves are
38,729, unknown_alias 5,584, and leaks remain 34. The system gains one unique
function (`flb_engine_get_chunk_backpressure_percent`) in two configurations.
These are isolated statement-card measurements, not the final merged figures.

The union-owner correction withdraws five apparent gains from the defective
attempt: `LZ4_favorDecompressionSpeed`, `LZ4_setCompressionLevel`, and the
telemetry helpers `calculate_consumer_commit_latency_max`,
`calculate_consumer_rebalance_latency_max` and
`calculate_share_time_between_poll_max`. All five remain Unknown in the fixed
measurement. There is no Safe correction relative to the actual BEFORE.

The later full Rust gate found another extraction defect: an anonymous
aggregate typedef can have only a typedef spelling in Clang's member type,
so a negative spelling filter mistook it for a scalar. The repair requires
positive scalar-type recognition and retains the aggregate-store refusal.
Consequently the `753cde34` isolated counts above are a superseded checkpoint,
not validation of the final implementation. The final statement comparison
uses the measured contract-only tree as BEFORE and the corrected combined tree
as AFTER, with complete parity recording and 16 audit workers. This keeps a
single attributable implementation change between those two measurements.

Old routes are retained. Complete side-by-side plans and frontend-context
preimages are archived by the statement card; differing plan vocabularies are
unresolved parity items, not evidence for deleting a route. Conditions, returns
and call arguments remain the separate P3 work. The frozen P1 forecast is
compared per function, including misses and gains outside its candidate set.
