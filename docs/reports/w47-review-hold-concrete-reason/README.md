# W47-REVIEW-HOLD-CONCRETE-REASON: a held row names the act it awaits

Card of wave 47. Base: local `main` at `ec4c9d4a`. Gateway rule 10i amended
([gateway](../../agents/gateway.md#constraint-10i)); contract specification
§C06.1 and grammar G05 amended; Lean `QCore.ReviewHoldStanding.ConcreteReason`.
Frozen prediction: [PREDICTION.md](PREDICTION.md) (commit `6169bf42`, before any
implementation).

## 1. The decision

Follow-up `W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN` asked for a product decision
on a review-held clause whose own reason says it must not be accepted as
written. The user decided (2026-09-29): such a clause **stays `Pending`**, and
the published verdict carries the **concrete reason**, not only the generic
`awaiting-human-review`. The instance was the tail-pause slice's
`src/flb_lib.c::flb_input` (both configuration rows), `Pending` on a call to
`flb_input_new`, whose `resource_borrowed(config)` and `resource_borrowed(data)`
are held with the reason "retained, not borrowed: … so this clause must not be
unwrapped as written".

Before this card the row said only
``call to `flb_input_new` whose declared contract awaits human review
(awaiting-human-review)``; the reasons were in the contract sidecar's run-wide
`pending_human_review` section, with nothing saying which of them a reviewer
could accept as written.

## 2. Mechanism

**Category from structure, not prose.** The review marker gains one optional
trailing word, after the reason and the owner (§C06.1, G05):

```qcore
    ensures normal resource_review_pending(resource_borrowed(data), "retained, not borrowed: …", rewrite_before_acceptance);
```

`ReviewDeferral` records it as `ReviewDisposition` (`qcore-schemas`):
`must-be-rewritten-before-acceptance` with the word,
`acceptable-as-written-after-review` without it, which is the existing meaning
of a plain marker (the resolution it offers is a review and then unwrapping).
Reading the category from the reason's words would make a verdict annotation
depend on how a sentence is phrased; the marker vocabulary could not express
it, so it was extended minimally, as the brief allowed. A plain marker
serializes exactly as before (the field is omitted), so every environment
without the word is byte-identical. The loader (`review_marker`) refuses, and so
rejects the whole module: the word anywhere but last, twice, another word, the
word standing in for the reason, and a trailing comma. An inherited marker must
carry its parent's disposition (§C15).

**The row names the hold.** `crates/qcore-audit/src/review_hold_reasons.rs`
adds one member, `review_holds`, to every published function row that records a
held-call refusal (read only from the exact refusal the gateway mints, rule
10i): for every held declaration in name order, each of its markers in clause
order, with `declaration`, `module`, `scope`, `clause`, the `reason` verbatim,
`owner`, `category` and `reason_code`. It is hooked in `projection.rs` right
after `AuditReport::record`, which every audit route (`pinned`, `current`, the
configured route, `contracts`) goes through, so a per-unit checkpoint stores the
member with its row and a composition renders it unchanged. A held callee the
environment does not withhold, or withholds with no marker, is an error that
fails the run: a row is never published holding a call whose reason is
unknown. Additive only: no status, diagnostic or count changes, and a row
without a held call is byte-identical.

**Section, tally, checkpoint.** Each `pending_human_review` entry carries its
`category`, the section counts `rewrite_markers`, and the console tally ends
with `N marker(s) must be rewritten before acceptance`. The `.qo` checkpoint's
marker record carries the category, so the checkpoint schema moves 3 → 4 and
a schema-3 file is refused (then recomputed by `--resume`), never misread.

**The verdict algebra is unchanged.** `Pending` still stands only under the
standing rule of rule 10i's W45 amendment (every other reason absent with the
held declarations in force as written); the second analysis still puts every
held declaration in force as written, whatever its category; nothing becomes
`Safe`; reasons are additive.

## 3. The twelve markers

| Declaration (module) | Marker | Category | Why, from the marker's own reason |
|---|---|---|---|
| `flb_create` (`input_instance.qv`) | `return_owned(nullable)` | acceptable | the clause is true; v1 cannot state the lists' contents |
| `flb_input` | `borrowed(data)` | **rewrite** | "retained, not borrowed … must not be unwrapped as written" |
| `flb_input` | `return_none()` | acceptable | P-HEAD-INSTANCE is an observation the clause does not contradict |
| `flb_input_new` | `borrowed(config)` | **rewrite** | "retained, not borrowed … must not be unwrapped as written" |
| `flb_input_new` | `borrowed(data)` | **rewrite** | "retained, not borrowed … must not be unwrapped as written" |
| `flb_input_new` | `return_interior(config, nullable)` | acceptable | v1 states neither list membership nor the flag; the clause holds |
| `flb_service_set` | `variadic_data_tail()` | acceptable | the key-dependent effect is unread, not contradicted |
| `flb_start` (`lib_engine_thread.qv`) | contract | **rewrite** | `ctx` is "handed to the engine thread until that thread is joined": `resource_borrowed(ctx)` denies that retention |
| `flb_stop` | contract | acceptable | joins and a candidate double join; `borrowed(ctx)` custody holds |
| `flb_input_thread_instance_exit` | contract | acceptable | the join is not custody; `borrowed(ins)` holds |
| `flb_output_thread_pool_destroy` | contract | acceptable | the skipped join frees the worker's parameters, not `ins` |
| `flb_downstream_worker_runtime_stop` | contract | **rewrite** | "the runtime is freed on that path and not on the -1 return": `resource_released(runtime, nullable)` asserts a release on every return |

Only the word was added; every reason, owner, clause and body is unchanged. The
two `lib_engine_thread.qv` classifications are this card's reading of their
reasons, the conservative direction (a rewrite category claims less than an
acceptable one); `W47-LIB-ENGINE-THREAD-REVIEW` and
`W47-INSTANCE-FLOW-CONTRACT-REVIEW` now ask the reviewer to confirm or remove
the word.

**Re-pin.** The four changed declarations' contract digests moved, voiding
their identities, so they were re-pinned with
[`scripts/pin.py`](scripts/pin.py) (`quirin-audit pin --function …`) at
`66910c10` (the corpus source, read only), `046accf6` (the system clone) and the
`90978ac6` export (commit `34155955`). [`scripts/lock_diff.py`](scripts/lock_diff.py)
shows only the four `contract_sha256` values move: every entry keeps exactly its
reviewed identities (sites, declaration and body digests, revisions). `pin
--check`, restricted to the four and to all nine review-held declarations,
exits 0 at each tree (4 and 9 recorded, 0 not in force).

## 4. Negative controls

| Control | Where |
|---|---|
| A held clause with no reason, a blank reason, or the word standing in for it, refuses the module at load (fail-closed; no "missing reason" is ever published) | `contract_deferred_review.rs`: `a_missing_or_empty_reason_refuses_the_module`, `a_disposition_is_no_reason` |
| A misplaced, repeated or other word refuses the module | `a_misplaced_or_unknown_disposition_refuses_the_module` |
| The word changes no contract and no consumer's view; a plain marker serializes as before | `a_trailing_rewrite_word_marks_the_marker_and_nothing_else` |
| A child inherits the disposition | `a_child_inherits_the_rewrite_disposition` |
| Two holds on one row list both, in a deterministic order (name, then clause), one callee held twice listed once | `review_hold_reasons.rs`: `two_holds_on_one_row_list_every_marker_in_a_deterministic_order` |
| A held callee the environment does not withhold fails; a text merely resembling a hold is not one; a row without a hold is unchanged; annotation adds one member and nothing else | `review_hold_reasons.rs` tests |
| A `Pending` row never loses its reason through composition: per-unit checkpoints and `checkpoint compose` publish the same bytes as the whole run, every held row (Pending included) carrying its `review_holds` | `checkpoint_cli.rs`: `a_held_rows_concrete_reasons_survive_checkpoints_and_composition`; the report merge is `AuditReport::absorb`, which appends rows unchanged |
| End to end: the `contracts` route publishes the member on the Pending row, not on the quiet one; a rewrite marker keeps the row Pending and says so | `pending_review_report.rs` |
| Old checkpoints are refused | `checkpoint.rs`: `old_and_future_schema_versions_are_refused` (2, 3, 5) |

## 5. Lean

`QCore.ReviewHoldStanding.ConcreteReason`, 20 theorems registered in
`AuditReviewHoldStanding.lean` (the module: 22 → 42), no placeholder, no axiom
beyond the allowed three. The loader keeps a loaded reason verbatim and refuses
a blank one (`a_blank_reason_is_refused`,
`a_loaded_marker_keeps_its_reason_verbatim`); the category is the word
(`Marker.category_eq_rewrite_iff`). The row's holds contain every marker of
every held declaration (`every_hold_names_its_concrete_reason`,
`two_holds_on_one_row_list_both`) and nothing the environment does not declare
(`holdsOf_invents_nothing`, `every_published_reason_is_a_loaded_one`); an
unnamed hold publishes nothing (`an_unnamed_hold_is_not_published`). The
verdict is the fold of the row's values whatever its reasons
(`annotation_keeps_the_verdict`, `adding_reasons_never_changes_the_verdict`,
`the_category_never_changes_the_verdict`). End to end with the standing rule's
own model: a row `Row.published` makes `Pending`, annotated over what `settle`
records, stays `Pending` and names every hold, at least one when it holds a
call (`a_pending_row_stays_pending_and_names_every_hold`). Composition, as the
`Reasoned` join, keeps every row's reasons (`composition_never_loses_a_reason`,
`compose_append`, `a_merge_keeps_both_reports_reasons`). Premises stated, not
proved: the held callees are exactly those the minted refusal names (rule 10i),
and a checkpoint stores and renders a row's members verbatim (the checkpoint
test pins it).

## 6. Measurements

Three arbiters, the common brief's commands, [`scripts/measure.sh`](scripts/measure.sh)
(`--jobs 8`), BEFORE at this worktree's base build, AFTER at the tip build;
compared with [`scripts/compare.py`](scripts/compare.py) through
[`compare_all.sh`](scripts/compare_all.sh), units aligned by position.

**BEFORE reproduces the baseline** (`w47-loop-owner-instance-flow/integration/artifacts`):
corpus `resource-diagnostics.json` raw SHA-256 `ae17152b…77c8`, equal to the
baseline manifest, its sidecar identical after the root alias, the parity
stream equal as a multiset (153,414 records; the stream is written in
completion order under `--jobs`, so it is compared unordered by
[`parity_multiset.py`](scripts/parity_multiset.py)); system and slice: 0 unit,
function or leaf moves, reports differing only in each unit's
`resource_contract_sources` digest and `inventory_sha256` (the run's own
contract paths), sidecars identical after the alias. Tallies: corpus functions
526 Safe / 10 Unsafe / 4,298 Unknown, units 6 / 8 / 167 / 4 data-only; system
rows 60 / 4 / 384; slice rows 37 Safe / 2 Pending / 1 Unsafe / 364 Unknown.

**AFTER against BEFORE: no verdict moves anywhere** — 0 unit status, 0
function status and 0 diagnostic-leaf moves on each scope; `diagnostic_counts`
equal; corpus `resource-diagnostics.json` byte-identical (still the baseline
manifest's SHA-256), corpus parity equal as a multiset, `chunk-contracts`,
`source-effects` and `monkey-source` byte-identical.

| Scope | Rows changed | Only by `review_holds` | Rows |
|---|---:|---:|---|
| slice | 6 | 6 | `flb_lib.c::flb_input` ×2 (**Pending**): `flb_input_new` `borrowed(config)` rewrite, `borrowed(data)` rewrite, `return_interior(config, nullable)` acceptable; `flb_lib.c::flb_create` ×2 (Unknown): `flb_stop` contract acceptable; `flb_input.c::flb_input_instance_exit` ×2 (Unknown): `flb_input_thread_instance_exit` contract acceptable |
| system | 4 | 4 | `flb_storage.c::flb_storage_create` ×2 (Unknown): `flb_input_new`'s 3 markers; `flb_output.c::flb_output_exit` ×2 (Unknown): `flb_output_thread_pool_destroy` contract acceptable |
| corpus | 0 | 0 | no corpus function holds a call |

Sidecars, all three scopes: `pending_human_review` entries gain `category`
(12 markers, 5 rewrite), `rewrite_markers` 5, the meaning text, and the two
modules' `module_sha256`; the retained texts of `input_instance.qv` and
`lib_engine_thread.qv`; `contract_revision`'s four contract digests and the
Fluent Bit lock's digest. Held counts unchanged (slice 3 functions / 6 call
sites, system 2 / 4, corpus 0 / 0); every pin in force (131 of 131). Console:
the tally line gains `; 5 marker(s) must be rewritten before acceptance`; no
other line changes (besides the report path and the preparation timings).

**Prediction against outcome.** Every prediction held: no verdict move, BEFORE
reproduced the baseline, exactly the 10 predicted rows gained exactly the
predicted holds and categories, the sidecar and tally moved as predicted,
parity unchanged.

**Classification.** No leak, violation or provisional `Unsafe` was added or
removed: every leaf is unchanged. No `Safe → non-Safe` move, so no correction.
Nothing here is a verified defect: the rewrite category records that a
contract's author states its text is not what the body does, and the slice's
one provisional `Unsafe` (`flb_tail_file_budget_destroy`, an artifact) is
untouched.

The new baseline is archived under [`artifacts/`](artifacts/) (corpus, system,
slice; gzipped with a SHA-256 manifest by the loop-owner card's
`archive_arbiters.py`; the system run recorded no parity stream).

## 7. Gate

See [gate.json](gate.json) for commands, exits and counts, run by
[`scripts/gate.sh`](scripts/gate.sh) (standing gate, provider gate, records);
logs under this worktree's `private/cr/gate/`.

## 8. Follow-ups

Retired: `W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN` (user decision 2026-09-29;
discharged by the disposition, the row member and the measurement above).
Amended: `W47-INSTANCE-FLOW-CONTRACT-REVIEW`, `W47-LIB-ENGINE-THREAD-REVIEW`
(the reviewer confirms or removes the word, and re-pins). Registered
([followups.json](followups.json)): `W47-PIN-MISMATCH-ROW-REASON` (a
contract-pin-mismatch hold carries no marker and is not listed on its row; 0
rows at this tip, every pin in force) and `W47-EXTERN-ROUTE-REVIEW-CATEGORY`
(the checked extern-call refusal Q023 names the reasons but not the category;
no audit route). Follow-ups 561 → 562 ([`scripts/update_ledger.py`](scripts/update_ledger.py),
idempotent).

## Reproduce

```sh
bash docs/reports/w47-review-hold-concrete-reason/scripts/measure.sh before   # at the base
bash docs/reports/w47-review-hold-concrete-reason/scripts/measure.sh after    # at the tip
bash docs/reports/w47-review-hold-concrete-reason/scripts/compare_all.sh baseline before OUT
bash docs/reports/w47-review-hold-concrete-reason/scripts/compare_all.sh before after OUT
python3 docs/reports/w47-review-hold-concrete-reason/scripts/summarise.py OUT
bash docs/reports/w47-review-hold-concrete-reason/scripts/parity_all.sh OUT
python3 docs/reports/w47-review-hold-concrete-reason/scripts/pin.py OUT [--check] [--all]
python3 docs/reports/w47-review-hold-concrete-reason/scripts/lock_diff.py BEFORE.lock AFTER.lock
python3 docs/reports/w47-review-hold-concrete-reason/scripts/update_ledger.py
python3 docs/reports/remaining-d02/build_matrix.py .
bash docs/reports/w47-review-hold-concrete-reason/scripts/gate.sh LOGS rust|provider|records
```
