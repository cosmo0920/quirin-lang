# REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: returned references and result holders

Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (wave 48, class 3, card-high).
Base: local `main` at `e27f98d5b`, which it stayed at.
[PREDICTION.md](PREDICTION.md) was committed at `855687ad7`, before any
implementation code, contract or Lean. Authority: the ledger entries
`REFCOUNT-REFERENCE-RETURNING-DECLARATIONS`, `RETAINED-BY-RESULT-HOLDER`
(absorbed) and `REFCOUNT-INFERRED-REFERENCE-RESULT` (read; not needed), under
the user's decision of 2026-10-05 on class 3 (C04.17).

**Published property and scope.** Custody (`c.custody`) on the pinned corpus,
the system route and the tail-pause slice, as before. The card adds a body
check (rule 13d), a holder word (`result`, rule 13e) and declarations; it adds
no property.

## 1. BEFORE and the population

Reproduced (binary `private/rrrd/bin/before`, `5c324125…`, equal to
`run.json`): corpus **884 / 9 / 3,941** (`7a4595d6…`, the brief's), system
87 / 4 / 357, slice 45 / 1 / 354 with 4 Pending. `reference-drop-unheld`
through a plain name **131 leaves / 91 functions / 1 alone**
([refusals-before-corpus.json](artifacts/refusals-before-corpus.json)), and
522 / 365 in the uncontracted-call population: both reproduce the ledger.

[`scripts/origins.py`](scripts/origins.py) reads where each dropped name's
reference came from ([origins-before-corpus.json](artifacts/origins-before-corpus.json),
table in [PREDICTION.md](PREDICTION.md)): `rd_kafka_q_new` 29 leaves / 24
functions, `rd_kafka_toppar_get` 15 / 11, `ut_create_mock_toppar` 9 / 9,
`rd_kafka_topic_partition_get_toppar` 8 / 7, `rd_kafka_q_fwd_get` 7 / 7,
`rd_kafka_toppar_get2` 5 / 5, the topic lookups 10 / 9, list removals 9 / 8,
formals 4 / 4, others 26. **Ranking:** no function is cleared by any
declaration. The declarable origins touch 66 leaves in 43 functions, and every
one of the 43 carries another leaf family (42 an uncontracted call, 38 the
access gate, 33 an unresolved identity, 28 a branch condition); none of the
candidates decides a function alone in the uncontracted-call population. The
yield was predicted as zero Safe and re-attribution only.

## 2. Design

Full text: contract specification C04.17 ("Returned references checked
against bodies", "The transfer") and C04.14.2; grammar G05;
[gateway.md](../../agents/gateway.md#constraint-13d) rules 13d and 13e.

* **Rule 13d, the body check of a returned reference**
  (`crates/qcore-c-gateway/src/legacy_reference_result.rs`, chained first from
  rule 13c's `reference_route::settle`). In a unit defining the body: no
  `goto`, at least one `return`, and each `return` a null constant (only for a
  `nullable` declaration), a call of a declared keep (alias result) or of a
  declared returned reference, or a local, never a formal, assigned only from
  those or from a declared heap allocation (a constructor), never
  address-taken, never at a declared release, consumption, store or drop.
  Otherwise the declaration moves, in that unit, to the retaining table marked
  withdrawn: every call refuses (`reference-withdrawn`) and no inference reads
  the body. The fresh count and the declared non-null result stay declared
  assumptions pinned to the body.
* **Rule 13e, the result as holder.** `resource_retained_by(P, result)`, only
  beside `resource_return_owned(nonnull)`. Rule 12t's table decides it with the
  holder the one live identity the call bound its plain destination to. A
  record ends when the result is released or when the function drops its last
  reference on an **acquired** holder family (P-RESULT-HOLDER-LAST-DROP); a
  borrowed family's last drop ends nothing; a holder still holding at an exit
  refuses (`retention-undecided`), never `Safe`. `__func__`, `__FUNCTION__` and
  `__PRETTY_FUNCTION__` are static storage at a retained position
  (P-RETAINED-PREDEFINED, C11 6.4.2.2p1).
* **The transfer.** `ut_destroy_mock_queue`, which consumes its caller's
  reference, is from its caller's side a drop, `resource_reference_dropped`,
  validated by rule 13c as a forward to the declared
  `rd_kafka_q_destroy_owner`. The caller's reference leaves its family, and a
  later drop of an acquired family is an over-drop. No caller needs a separate
  clause, so none is spelled. The callee's own row reading its parameter as
  the reference it took over is not built (`REFCOUNT-DROP-OWN-BODY-TRANSFER`,
  1 function).

Shared items added (named in rules 13d and 13e): `RetentionHolder::Result`,
the loader's `result` holder word, `retained_by::PREDEFINED_IDENTIFIERS`,
`HolderSpelling::Result`, a `destination` argument on `retained_by_route::apply`
and `holder_class`, the acquired family's final drop calling
`holder_released`, `reference_route::settle` chaining
`reference_result_route::settle` (13c's old body is `settle_clauses`), and
`retaining_call` refusing a withdrawn declaration that has no reference
clause.

**Controls** ([tests/refcount_reference_returning.rs](../../../crates/qcore-c-gateway/tests/refcount_reference_returning.rs),
9 tests over [fixtures/contracts/refcount_results.qv](../../../crates/qcore-c-gateway/tests/fixtures/contracts/refcount_results.qv)):
a returned reference balanced (`Safe`), leaked (a leak candidate) and
over-dropped (`DoubleRelease`); a result holder dropped or released first
(`Safe`), returned (`retention-undecided`), handed a frame object's address
(refused at the call) or a member (refused by the access gate first), its
retained object released while held (`retention-released-while-held`) and
after the handle (`Safe`), and an extra reference kept (not `Safe`); the three
predefined identifiers (`Safe`) and a frame buffer (not); a returned reference
whose in-unit body validates (`Safe` caller) and one handing back its argument
(`reference-withdrawn`); a declared drop wrapper taking over the caller's
reference (`Safe`), then an over-drop. Unit tests of the body check in
`legacy_reference_result.rs` (the measured shapes and every refused one).

## 3. Contracts

Ten declarations appended to `qcore-provider-librdkafka` `librdkafka/client.qv`,
and `rd_kafka_queue_new0` re-declared, each body read at both revisions
(byte-identical at `66910c10` and `046accf6`, `scripts/body.py`; its two
`DIFFERS` lines for `rd_kafka_toppar_desired_get` are call sites in
`rdkafka.c`, not the definition) and pinned with `quirin-audit pin` at both
(logs `private/rrrd/pin-a1/`): `pin --check` of the crate 0 at both,
`--verify-contracts` 0, the whole-tree check 1 at both as on the base (its one
row the libevent crate's tip-mode `mk_vhost_set`).

| kind | declarations |
| --- | --- |
| returned reference, nullable | `rd_kafka_toppar_get0`, `_desired_get`, `_get2`, `_get_avail`, `ut_create_mock_toppar` |
| returned reference, non-null | `rd_kafka_toppar_new0` (`return rd_kafka_toppar_keep(rktp)` over a count of 0), `rd_kafka_toppar_desired_add` (its last source is non-null and it is dereferenced), `rd_kafka_q_new0` (`rd_malloc`, count 1 in `rd_kafka_q_init0`; retains `rk` and `func` by its result: `ENABLE_DEVEL` is off in the audited CMake cache, so `rkq->rkq_name = func`), `ut_create_mock_queue` |
| drop | `ut_destroy_mock_queue` |
| re-declared | `rd_kafka_queue_new0` retains `rk` by its result (it leaves the function map) |

`rd_malloc` and `rd_calloc` assert their result and the audited compile
commands define no `NDEBUG`, so they never return NULL in the audited build.
Every defining body validates rule 13d (no `reference-withdrawn` on any
route). Prediction deviation: `rd_kafka_toppar_desired_add` was predicted
nullable (a structural check that could not see its last source); the check as
built reads nullability only from an explicit `return NULL`, so it is declared
non-null, which its body shows. Not declared (bodies rule 13d cannot validate,
or not a reference on every path): `rd_kafka_q_fwd_get`,
`rd_kafka_topic_partition_get_toppar`, the topic lookups,
`rd_kafka_topic_proper` (`REFCOUNT-REFERENCE-RESULT-BODY-SHAPES`), and
`rd_kafka_toppar_get_by_id2` (a by-value `rd_kafka_Uuid_t` parameter).

## 4. AFTER, attributed

Binary `private/rrrd/bin/a2` (`a3398282…`, equal to `run.json`), built from
`ed953aefd`. The later commits change tests, records, Lean and formatting
only; the binary built from `d4fd27b8d` (`a3`, `bdfb161f…`, equal to its
`run.json`) reproduces the corpus report byte for byte (`79dc429d…`) and the
parity multiset (662,742 rows, `private/rrrd/parity-a3-vs-a2.json`).

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus | 884 / 9 / 3,941 | **880 / 9 / 3,945** | `7a4595d6…` → `79dc429d…` |
| system | 87 / 4 / 357 | unchanged, no row moves | `b5118a80…` → `3abf7e91…` (both embed run paths) |
| slice | 45 / 1 / 354, 4 Pending | unchanged, no row moves | `32c1b2cc…` → `94084ff3…` |

**No Unknown → Safe. 4 Safe → Unknown, each a named correction of
REFCOUNT-RESOURCE-KIND** ([comparison](artifacts/compare-a2-corpus.json)):
`rd_kafka_queue_get_main`, `rd_kafka_queue_get_consumer` and
`rd_kafka_queue_get_sasl` return a queue handle that retains their caller's
`rk`: the leaf reported is the access gate's on the member actual (`rk->rk_rep`,
`rk->rk_cgrp->rkcg_q`, `rk->rk_sasl.callback_q`), which the gate read for the
function-map declaration and does not read for a withheld one
(`RETAINING-CALL-MEMBER-ACTUAL`); behind it the returned holder refuses at the
exit, as the controls show for a plain actual. `rd_kafka_share_queue_get_sasl`
was Safe through `_get_sasl`'s summary, which is gone. The previous card had
read `rk` as borrowed with the gap recorded; these four were `Safe` only
through that reading. **Unsafe 9 → 9; no leak or violation candidate added**
(no `Leak` or `DoubleRelease` among the added leaves), so none needed a
hand reading.

59 more Unknown rows changed leaves only (63 moving rows; labels by the
declared callee whose leaf left: `rd_kafka_q_new0` 25, `rd_kafka_toppar_get0`
13, `ut_create_mock_toppar` 9, `ut_create_mock_queue` 7,
`ut_destroy_mock_queue` 7, `rd_kafka_toppar_desired_get` 5, `_get2` 4,
`_desired_add` 3, `_new0` 3, `_get_avail` 1; the other 6 read a declared
lookup in a branch condition or lost `_get_sasl`'s summary). Removed: 78
`call to uncontracted function` leaves, 12 unresolved identities, 5 branch
conditions, 4 unheld drops, 2 access-gate leaves. Added: 20 `reference-dropped`
(5 functions), 19 holder exits and 2 call-site `retention-undecided`, 18
access-gate leaves, 17 unresolved identities, 9 loop re-acquisitions, 7 unheld
drops, 6 `reference-unbalanced`, and single others.

**Not predicted:** plain-name `reference-drop-unheld` fell only 131 → 127
([refusals-a2-corpus.json](artifacts/refusals-a2-corpus.json); declared
origins 66 / 43 → 62 / 41, [origins-a2-corpus.json](artifacts/origins-a2-corpus.json)).
The families are formed (each acquiring call's leaf left, and the holder
records appear), but in most callers a refused statement reads the name before
the drop (`rd_kafka_q_wait_result(tmpq, …)`, `rd_kafka_op_set_replyq(rko, rkq,
…)`, an uncontracted unit-test helper): rule 13b's refused hand-on withdraws the
family, so its drop is still unheld. That is the rule, not a declaration gap.

Remaining ([ranking](artifacts/rank-a2-corpus.json), [tally](artifacts/tally-a2.txt)):
**523** Unknown functions blocked by uncontracted calls alone, **367** by one
callee alone (+1 `rd_kafka_share_queue_get_sasl`, now blocked by
`rd_kafka_queue_get_sasl` alone; +1 `ut_create_mock_toppar`, by
`rd_kafka_topic_new` alone); system 26 / 18, slice 38 / 27 unchanged. Class 3
stays **2 callees / 4 functions** (`rd_atomic32_add` 2,
`rd_kafka_txn_clear_partitions_flag` 2).

**Parity, compared in full** ([corpus](artifacts/parity-a2-corpus.json),
[system](artifacts/parity-a2-system.json), [slice](artifacts/parity-a2-slice.json)):
system (139,962 rows) and slice (50,407) multisets identical; corpus 663,033 →
662,742 rows, 71 functions differ. Outside the moving rows: the derivation rows
of the declared bodies (`rd_kafka_toppar_new0`, `rd_kafka_q_new0`,
`rd_kafka_queue_new0`, `ut_destroy_mock_queue`, `rd_kafka_toppar_desired_get`:
a declared or withheld body is not replaced by an inference) and of the
bodies that call them in derivation only (`rd_kafka_topic_new0`,
`rd_kafka_topic_partition_ensure_toppar`), which have no moving report row.

**`mk_vhost_set` at the tip** ([rows](artifacts/tip-rows-after.txt)):
byte-identical to REFCOUNT-RESOURCE-KIND's record.

## 5. Prediction against outcome

| item | predicted | measured |
| --- | --- | --- |
| Unknown → Safe | 0 (0 to 2) | **0** |
| Safe → non-Safe | 4 named corrections | **4**, the four named |
| corpus | 880 / 9 / 3,945 | **880 / 9 / 3,945** |
| new leak or over-drop candidates | 0 to 3 | **0** |
| plain-name unheld drops | 131 → about 65 | **127** (refused hand-on, above) |
| `call to uncontracted function` on the declared callees | lost in 25 / 10 / 9 / 4 / 3 functions | 78 leaves removed in all |
| re-attributed Unknown functions | 40 to 80 | **59** |
| system / slice | no row moves | **no row moves** |
| uncontracted-call population | about 520 / 365 | **523 / 367** |

## 6. Lean

`QCore.ReferenceResult` (new, 17 audited theorems, registered in
`AuditReferenceResult.lean` and imported by `QCore.lean`; no placeholder, no
project axiom; 297 → 298 audits). Rule 13d:
`a_validated_body_never_returns_a_formal`,
`a_validated_body_returns_only_obtained_references`,
`a_validated_non_null_body_never_returns_null`,
`a_withdrawn_declaration_is_never_safe`, `a_jump_withdraws`,
`body_vectors_hold` (the measured shapes and the refused ones). Rule 13e, the
result-holder reading the brief asks for:
`a_result_holder_released_first_is_safe`,
`a_result_holder_dropped_first_is_safe`,
`a_result_holder_handed_on_is_never_safe`,
`a_borrowed_familys_last_drop_keeps_its_records`,
`a_result_holder_without_a_tracked_result_is_undecided`,
`an_object_released_while_its_handle_holds_it_is_refused`,
`a_predefined_identifier_outlives_every_holder`. The transfer: a transferred
reference leaves the caller's family
(`a_transferred_reference_leaves_the_callers_family`), a later caller drop is
an over-drop candidate (`a_drop_after_a_transfer_is_an_over_drop`), a later
use is refused (`a_use_after_a_transfer_is_refused`).

## 7. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py) (follow-ups 727 →
728). Retired: `REFCOUNT-REFERENCE-RETURNING-DECLARATIONS` (every declarable
lookup and constructor declared after reading each body at both revisions; the
transfer is a drop and needs no clause; no new leak or over-drop candidate;
measured; the undeclarable shapes re-registered) and `RETAINED-BY-RESULT-HOLDER`
(the holder spelled, rule 12t's outlives decision for it, Lean, measured).
Amended: `REFCOUNT-INFERRED-REFERENCE-RESULT` (not needed by any member; 0),
`W48-CALLEE-DECISION-TABLE` (class 3 still 2 / 4) and
`W45-CALLEE-CASCADE-RESIDUAL` (523 / 367). Registered:
`REFCOUNT-REFERENCE-RESULT-BODY-SHAPES` (29 leaves / 25 functions / 0 alone),
`REFCOUNT-DROP-OWN-BODY-TRANSFER` (1 function alone),
`RETAINING-CALL-MEMBER-ACTUAL` (4 functions, 0 freeable).

## 8. Gate

Standing gate (`docs/agents/validation.md`), [`scripts/gate.sh`](scripts/gate.sh)
`g1`, every code step at `ee4640a6f` on base `e27f98d5b`; logs
`private/rrrd/gate-g1/`, exits in [gate/](gate/), summary [gate.json](gate.json).

| step | exit | result |
| --- | ---: | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update `--init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 457 units, 470 summaries; 7,703 passed, 0 failed, 11 ignored (`private/rrrd/tests-gate-g1`, 354.8 s inside the lock) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 298 audits passed (297 + `AuditReferenceResult`) |
| `python3 -m unittest discover -s scripts/tests` | 1, then 0 | the committed matrix carried 297 Lean audits (kept as `08-python-stale-matrix*`); after `build_matrix.py . --gate gate.json` (09a, 0), 365 tests OK (09b) |
| `build_docs_index.py --check` | 0 | and 0 again after the matrix (10b, 10c) |

**Provider gate** ([`scripts/provider-gate.sh`](scripts/provider-gate.sh) `g1`,
logs `private/rrrd/provider-gate-g1/`): `cargo check --locked` 0, `cargo test`
0 (129 passed), fmt 0, clippy 0, `providers/contracts.py check` 0, package
build 0 and `verify` 0 for all five packages, `python3 -m unittest discover -s
providers/tests` 0 (25 tests OK). Pins: section 3.

Disk: the suite's debug build took the shared disk to 45 GB free (above the
40 GB floor); `cargo clean --profile dev` (workspace and `providers/`) after
the gate restored 100 GB. The private release target was deleted after the
measurements; the saved binaries stay in `private/rrrd/bin/`.

## Files

* Rules 13d, 13e: `crates/qcore-c-gateway/src/legacy_reference_result.rs`
  (new); hooks in `legacy_reference_count.rs`, `legacy_retained_by.rs`,
  `legacy_transfer.rs`, `retained_by.rs`, `legacy_c.rs`; schema
  `crates/qcore-schemas/src/retained_by.rs`; loader
  `crates/qcore-contract/src/contract_retained_by.rs`.
* Lean: `proof/QCore/ReferenceResult.lean`, `proof/AuditReferenceResult.lean`,
  `proof/QCore.lean`.
* Tests: `crates/qcore-c-gateway/tests/refcount_reference_returning.rs`,
  `tests/fixtures/contracts/refcount_results.qv`; deliberate pin updates in
  `compiler/tests/contract_loader.rs` (349 → 359 v1 declarations, retaining
  25 → 28, the eight function-map returned references in the previous-modes
  table) and `compiler/tests/qcore_contract_corpus.rs` (ten clause rows;
  `rd_kafka_queue_new0` 4 → 5).
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`.
* Records: contract spec C04.14.2, C04.17; grammar G05;
  `docs/agents/gateway.md` (13d, 13e), `docs/agents/README.md`,
  `docs/STATUS.md`, `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: adapted from REFCOUNT-RESOURCE-KIND, plus `origins.py`,
  `safe_callers.py`, `check.sh` and `lean.sh`.
