# W48 bottom-up batch 4 — checked call-free predicate composition

Base `2d8f6d34257db14031a2578ce14f941bee25fc19`; branch
`codex/w48-bottom-up-batch4`. Prediction commit `5cdf4b594` precedes semantic
implementation `b931ed61d`. The [prediction](PREDICTION.md) remains frozen.
The exact-base release analyzer was preserved before implementation; its
build HEAD `35aaecd3e` changes report text only. [Build provenance](base-binary.json)
discloses the HEAD and immutable binary digest.

## Selection and checked boundary

The prior ranked residual puts `rd_kafka_log0` first (score 3). Actual pinned
source confirms `rd_kafka_log_buf` allocates an op and log string, writes op
fields and either enqueues or calls `conf->log_cb`. `rd_kafka_q_enq1` recursively
forwards, stores serving callbacks and either transfers or destroys/replies to
the op. The neutral prefix-closed recursion theorem cannot justify that
positive effect. No neutral callback, consumed-op declaration or assumed
acyclic forwarding graph is introduced. Scheduler cleanup (score 2) waits on
TLS-retained `libco_params`, list unlink, `mk_thread_destroy`'s `cb_destroy`
dispatch and `co_delete` before frame release. These remain real dependencies,
not cheap one-leaf closures. The next score 1 cJSON object-detach wrappers return
a Removed result after a string-keyed getter. The getter traverses child/next
and string comparisons; the existing bounded array-lookup frame supplies
neither string lookup membership nor richer Removed return transport.

The next ready score 1 body is `cio_file_is_up`. Its actual Linux source expands
to `(cf->fd != -1) && (cf->map != NULL)`. Both leaves already have checked
readers, but the call-free plan was discarded when neither leaf needed the
newer Clang access-path reader or elaborator. A baseline synthetic control
reproduced Unknown with the exact branch refusal; production conformance is
measured separately under its own actual declarations. All readiness source
hashes are [recorded](readiness-sources.json).

The new Rust rule retains a complete, otherwise refused call-free condition
plan when one node is an existing `IndirectScalarAccess`. Each original leaf
instruction, alias/lifetime/per-hop obligation and branch decision remains at
its existing short-circuit/loop position. No leaf syntax, pointer identity,
scalar value, source summary or contract authority is assumed by retention.
An incomplete leaf still refuses the complete plan and restores the probe.
The descriptor and switch guards remain. Existing admissions keep their route.
The report-owned link map adds only `cio_file_is_up` as an explicit
selected-program home. Link resolution, type/calling-convention agreement and
non-interposition remain stated assumptions.

`QCore.CallFreeConditionReads` binds the 8 Boolean retention vectors and reuses
the established Boolean-tree desugaring theorem for checked-read events. For
an arbitrary read checker, the complete state and error trace agree; an
error emitted by an evaluated read is retained. The model proves no C
extraction or Rust correspondence theorem (boundary 28). Six focused controls
cover Boolean compositions, loops, leaks, double release, released bases and
refused volatile/atomic/write/uncontracted leaves. Every Safe fixture replays
its retained certificate. The literal 0 compound control stays conservative:
both scalar outcomes are represented, so it cannot erase a released read.

## Evidence and results

All six final audit commands exited 0, using the same explicit eight-home map
on both corpus sides (batch 3's seven homes, including `mk_fifo_send`, plus
`cio_file_is_up`). Corpus selection is 4,834 custody rows: Safe **967 → 973**,
Unknown **3,858 → 3,852**, and provisional Unsafe **9 → 9**. These are
conditional custody results under the recorded declarations and external-home
assumptions. None is presented as a complete C proof or an upstream defect.
The six gains are:

- `cmt_metric_name_from_labels` and `compute_flat_scope_index`;
- `cmt_summary_quantile_get_value` and `cmt_summary_quantile_set`;
- `cio_file_is_up` and `cio_file_native_delete`.

System's 448 custody rows remain 99 Safe / 345 Unknown / 4 provisional Unsafe.
Slice's 404 remain 53 Safe / 346 Unknown / 4 Pending / 1 provisional Unsafe.
Every prior Safe and provisional Unsafe is preserved. All **5,781 complete null
row dictionaries** are identical, as are all runtime dictionaries and their
`all`, `flb_external_plugin` and `rk_plugin_library` scenario views. Combined
custody/null corpus Safe is 843 → 847; system remains 65 and slice 31. Property
projections retain null-only rows and their denominators separately. See
[results](results.json) and the route-owned `projection-comparison.json` files.

There are 48 corpus status/diagnostic changes, of which 42 stay Unknown. All
73 unique changed diagnostic origins resolve to the pinned source and retain
excerpts and hashes in [corpus attribution](artifacts/corpus/attribution.json).
The important newly visible failures are unresolved aliases, not assumed
neutral effects. System changes 33 diagnostic rows and slice 17; their
verdicts do not change. Unsupported compound guards previously poisoned the
bindings `in`, `ins` and `config`; retaining the checked guards removes those
specific later alias diagnostics. Newly evaluated checked reads expose other
aliases. [Configured source review](configured-source-review.json) binds every
such origin to the actual raw source digest. Two system caller sessions now
consult existing `mk_list_entry_is_orphan` conformance; this is caller evidence,
not two new distinct native definitions.

One existing declaration gains native body standing:
`cmt_summary_quantile_get_value` changes from declared assumption to validated
for `resource_borrowed(#0)`, `resource_borrowed(#1)` and
`resource_return_none()`, with both `W4` and `entry` evidence. Its actual checked
body guards the metric/quantile buffer before the existing atomic scalar
accessor. No declared mode, source pin, contract text, graph disposition or
named premise is changed. The exact promotion and all usage changes are kept
in [authority review](artifacts/corpus/authority-review.json).

All five raw artifact families are archived for each route and side. Every
changed function field is retained in duplicate-preserving comparisons;
complete source, chunk-contract and source-effect payloads agree after the
explicit archive-root substitution, with **zero source-effect identity
aliases**. Contract dependency tables are freshly replayed by the production
Rust producer (`metadata_replay.rs`); negative controls reject wrong verdict,
build and unsupported method evidence. Full environment bytes and every
external-home hash are reconstructed with the production environment helper.
Build/source-assumption identities and canonical context-set permutations are
checked against their actual producer inputs, rather than deleting identity
fields. Empty Monkey summary keys containing `__TIME__` are reproduced from
the actual preprocessing input and each measured UTC window; their values are
empty on both sides and wrong-time controls fail. These are metadata identity
proofs, not safety premises. Per-route context/time receipts, full raw payloads,
[complete contract review](artifacts/corpus/complete-contract-review.json) and
[complete diagnostic review](artifacts/corpus/complete-diagnostics-review.json.gz)
retain the derivations. Review scripts live in `scripts/` beside this report.

The fresh ranked graph has 2,215 candidate bodies (before 2,220), ceiling 362
(before 360), 254 one-body waits and six same-/cross-unit baseline closures.
The ceiling is hypothetical graph closure, not promised Safe coverage. Logger
still ranks first at score 3; its `log_buf` dependency still has eight leaves.
Scheduler cleanup remains score 2 with five `mk_http_thread_destroy` leaves.
The refreshed source readiness is [recorded](artifacts/ranking-readiness.json).

## Validation

The standing gate completed: **8,377 Rust tests passed**, zero failed, 15
ignored across 522 binaries / 535 summaries; **347 Lean audits passed**.
Submodule initialization, local Clang, fmt, Clippy with warnings denied and
both reevaluation examples exited 0. The final Python evidence suite passed **374 tests** (33.500 seconds); its command is
recorded in [gate.json](gate.json). No provider files, profiles, schema, SDK or
lock were changed, so the conditional full provider gate is inapplicable; no
provider-test count is borrowed from another card. The Rust/Lean model and
semantic implementation were unchanged after these final runs; subsequent
changes are evidence tooling, formatting and documentation.

[Command receipts](command-receipts.json), [Rust attempts](rust-attempts.json),
[Lean receipt](lean-receipt.json), and [preparation attempts](preparation-attempts.json)
record actual commands, exits, counts and private logs. The private root is
`private/w48-bottom-up-batch4/`; heavy gates and all six fresh audits used the
shared host lock. Configured pairs used eight workers per route together,
then each corpus side alone used 16. No corpus worker ran beside cargo tests.
The exact BASE and TIP analyzer SHA256 values are
`1fbfe1bb8f3d1a2bfd4d8323489b1431e0ba917941645f7ce09898f8051396b7` and
`d3f41cf7ae6aa3c7a04fef249d6ae4f713963382e3906026dcdbbdd8d304bca4`.
Their actual evidence-version HEADs are respectively `35aaecd3e` and
`b931ed61d`, explicitly distinguished from the exact Rust base and report tip.

## Retained preparation and validation attempts

The first full Rust run finished with 8,376 passing tests, one failure and 15
ignored tests across 522 binaries/535 summaries. Its sole failure was an
explicit old compound-read refusal pin for `n->buf && n->n`. The checked
composition intentionally supersedes that refusal; the replacement requires
Safe, empty diagnostics and both `IndirectScalarAccess` obligations. Existing
cast and chain refusal controls remain. The full Lean attempt also caught an
aggregate import placed after the final module-doc command. Moving that import
among the other imports leaves the individually checked model unchanged.
Both failed attempts are retained for comparison with the final reruns. The final Rust rerun is 8,377/0/15 and the final full Lean rerun is 347/0.

All six initial audit commands exited 1 before program analysis because the
runtime archives omitted the tracked root `cycle19-resource-contracts.json`.
Those original 306-file snapshots remain unchanged. Fresh, separate 307-file
snapshots include the registry, whose filesystem and provider-qualified modules
all resolve. [The final input manifest](runtime-inputs-final.json) records each
file and the distinct analyzer binary on either side. Both contract file sets
are identical. No verdict or coverage result comes from the failed preparations.

Two replacement collectors were cancelled while still waiting for the shared
lock, before any child process or output route existed. Their exit 130 and
captured process-state checks are retained privately. The final schedule uses
at most 16 analysis workers: configured system/slice run together at eight each,
then each corpus side runs alone at 16. Each before/after route uses the same
worker count. This follows the documented worker cap and deterministic-output
contract; it changes neither selection nor assumptions. Original audit-driver
bytes and the complete raw artifacts remain retained.

Metadata review guards also retained failures: initial strict configured checks
rejected changed null carriers, usage reasons and context-set ordering until
they were explicitly attributed. The initial corpus authority check rejected
the new native body standing until its exact three clauses and `W4`/`entry`
evidence were verified. The six program audits themselves all exited 0;
that collector exited 1 for this metadata guard, and its later exact retry
exited 0. A supplementary diagnostic check initially assumed a row's own null
carriers equalled its transitive dependency set; the producer explicitly
includes consulted in-unit summaries, so the corrected check requires unique
dependency names and containment of every own carrier while retaining both
complete payloads. No raw output is rewritten or failed attempt hidden.

## Follow-ons

`W48-BOTTOM-UP-RESIDUAL-PLAN` remains open. Logger ownership/callback closure,
scheduler TLS/list/thread destruction, string-keyed lookup membership and
richer Removed return transport require their own checked boundaries. The
configured slice's unresolved `end` identities remain independently tracked.
`cio_chunk_is_up` remains Unknown: `else if (type == CIO_STORE_FS)` at
`chunkio/cio_chunk.c:518` still has an unresolved scalar-local `type` resource
identity. The new conditional home closes its previous uncontracted
`cio_file_is_up` call; it does not justify this independent scalar identity.

## For the integrator

Integrate serially from this isolated branch; it never advances main. Preserve
prediction chronology, the exact build/analyzer/input/source identities and
all failed attempts. Keep conditional external homes separate from source
facts and caller-session replay occurrences separate from distinct C-body
conformance. Shared STATUS, part10 increment, acceptance generator/matrix and
report index need serial reconciliation with the premise cards. No provider
contract edits are made, so the conditional provider gate is inapplicable.

The exact proposed STATUS paragraph, one part10 increment paragraph and residual-plan amendment are in [integration-amendments.json](integration-amendments.json). Apply those records during integration; this branch deliberately leaves the shared STATUS and increment files for the integrator. The shared gate matrix retains preceding supplied gate figures and derives only the current 347-audit count; regenerate with the measured merged gate after serial integration.
