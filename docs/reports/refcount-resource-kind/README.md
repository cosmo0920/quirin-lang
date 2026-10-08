# REFCOUNT-RESOURCE-KIND: the reference-count resource kind

Card REFCOUNT-RESOURCE-KIND (wave 48, class 3, card-high). Base: local `main`
at `9258289c9`, which it stayed at. [PREDICTION.md](PREDICTION.md) was
committed at `107952c68`, before any implementation code or contract.
Authority: `W48-CALLEE-DECISION-TABLE` class 3.

**Published property and scope.** Custody (`c.custody`) on the pinned corpus,
the system route and the tail-pause slice, as before. The card adds a resource
kind, two withheld clauses, one result spelling and a structural body check to
resource vocabulary version 1; it adds no property.

## 1. The decision

**USER DECISION 2026-10-05**, recorded verbatim in the contract specification
(C04.17) and in `W48-CALLEE-DECISION-TABLE`: class 3, reference counts, is a new
reference-count resource kind. A keep is +1 owned reference, a drop −1; each
function balances the references it takes or hands them off (return, store into
a tracked owner, a declared transfer), exactly as with other owned resources; a
plain counter that is not a reference needs its own declaration and never
inherits the reference reading; the custody-neutral reading of keep and drop is
rejected and never fallen back to.

## 2. BEFORE and the members

Reproduced (binary `private/rrk/bin/before`, `86110fdc…`, equal to
`run.json`): corpus **872 / 9 / 3,953** (`52d14db1…`, the brief's), system 87 /
4 / 357, slice 45 / 1 / 354 with 4 Pending. Ranking population 545, 384 decided
alone; class 3 **9 callees / 19 functions alone**, the ledger's figure. The
member census ([census-before-corpus.json](artifacts/census-before-corpus.json))
and every body read are in [PREDICTION.md](PREDICTION.md); every body is
byte-identical at `66910c10` and `046accf6` (`scripts/body.py`; its two
`DIFFERS` lines for `rd_kafka_toppar_keep0` pair macro lines, not the
definition). No `Safe` row calls a keep or drop wrapper (226 call sites, all in
`Unknown` functions), and system and slice carry no member.

**The hazard.** In the audited build `rd_refcnt_t` is a typedef of
`rd_atomic32_t` (`rd.h:298`) and `rd_refcnt_add0(R)` a macro for
`rd_atomic32_add(R, 1)` (`rd.h:351`); `rd_kafka_q_keep` counts in a plain `int`.
No single counter type covers every family, and after preprocessing a reference
keep and a statistics counter are the same call.

## 3. Design

The full text is in the contract specification (C04.17, C04.6), grammar G05 and
[gateway.md](../../agents/gateway.md#constraint-13b) (rules 13b, 13c).

* **The kind.** `ResourceKind::Reference`, pointer-carried. A reference is
  taken by `resource_reference_kept(p)` and dropped by
  `resource_reference_dropped(p)` (on a `resource_borrowed` depth-one pointer;
  a keep returns `resource_return_alias(p, nonnull)` or nothing), both withheld
  from the function map with the retaining declarations, so no consumer that
  knows nothing of references ever reads them as borrowed. A **returned
  reference** is `resource_return_owned(n)` with
  `resource_return_kind(reference)`: how a caller owns a reference a callee
  hands back. `resource_kind(p, reference)` on a parameter rejects the module.
* **Which object.** Counted per object, because a keep returns its own
  argument: a keep of a name bound to no tracked identity mints a *family*
  (held once, bound to the name and the result), a keep of a live family adds
  one; a returned reference starts an *acquired* family.
* **Balance.** At an exit a family owes its count, less one if it escaped
  (alone or with the owner it is linked into), was stored or was published: a
  leak candidate on a path with no refusal, `reference-unbalanced` (Unknown) on
  one with a refusal; a refused statement that may have handed it on withdraws
  it as for any resource.
* **The last drop.** The drop that reaches zero is possibly final: the family
  is discharged and every name bound to it invalidated, so any later read is
  refused (`reference-dropped`), never accused, unless the function still holds
  another reference. A drop the function does not hold refuses
  (`reference-drop-unheld`); a drop past an acquired family's last drop is an
  **over-drop**, a `DoubleRelease` candidate on a path with no refusal.
* **Plain counters, resolved at the wrapper level.** No `rd_atomic32_*`
  primitive is declared. A reference operation is declared only on a named
  wrapper whose body names the object, and that body is checked in every unit
  that defines it (rule 13c), so a statistics counter is never read as a
  reference and a reference spelled through the primitive is never read as
  neutral (it stays `call to uncontracted function`). Why this cannot misread a
  counter: the reading is keyed on a callee whose declaration is pinned to its
  body and whose body the route checks, never on the primitive's name, and no
  counter code calls a keep wrapper.
* **Interactions.** A slot move of a family is refused (12x's held-elsewhere
  test now includes families); a borrowed result or derived out slot is never a
  family (12y, 12z); a retained or field-linked family counts as held elsewhere
  for a drop (12t), and a field link hands the reference off only when its owner
  escapes; no reference clause stands beside a return-code disposition (11r,
  12s).
* **Rule 13c.** In a defining unit the referenced formal is never assigned,
  address-taken, stored or returned (except an alias keep's `return p;`), is
  mentioned only through member accesses, whole call actuals and null tests,
  the body has no `goto` and touches the object; otherwise every call there
  refuses (`reference-withdrawn`) and the declaration stays in the table so no
  inference reads the body in its place.

Shared items added (named in rule 13b): `ResourceKind::Reference` (the kind
table, `observed_kind`, the use-after-close candidate, the Lean kind list),
`RetainingContract::{references, references_withdrawn}`,
`AbstractState::references`, `retained_by_route::Operands::references`,
`Parser::reserve_reference_site`, `slot_move_route::Invalid::{ReferenceDropped,
ReferenceFreed}` (`defined_body` made `pub(super)`), and hooks in
`apply_contract_call`, `remove_null_site`, `report_leaks`, the exit report, the
loop generations' `rename`, the slot move's held-elsewhere test and the kind
table.

**Controls** ([tests/refcount_resource_kind.rs](../../../crates/qcore-c-gateway/tests/refcount_resource_kind.rs),
14 tests): balanced keep and drop (also through the result, a void keep, two
references, a branch); a missing drop (leak candidate; refused, not accused, on
a path with a refusal); a double drop (refused; a drop never taken refused); a
keep returned to the caller (also through a pointer cast; two keeps one returned
leak); a keep stored into a tracked owner (handed off; an owner freed still
owes it; a drop of the owner's reference refused; a keep and drop in a loop
balanced); a statistics counter beside a reference count on the same primitive
(the declared wrapper is a reference, the primitive on either field is refused,
never a leak, never Safe); a use after a possibly-final drop (refused, also
through an alias, and a keep after it); a returned reference owned by the
caller; an over-drop of an acquired reference (a `DoubleRelease` candidate; on a
refused path refused); the withheld declaration; a body that does not validate
(withdrawn, refusing); a heap release of a reference (a kind mismatch). Loader
controls in [crates/qcore-contract/tests/reference_clause.rs](../../../crates/qcore-contract/tests/reference_clause.rs).

## 4. Contracts

Sixteen declarations appended to `qcore-provider-librdkafka`
`librdkafka/client.qv` (the placement rules: librdkafka in its own crate), each
body read at both revisions and pinned with `quirin-audit pin` at `66910c10` and
`046accf6` (logs `private/rrk/pin-a1/`); `pin --check` of the crate 0 at both,
`--verify-contracts` 0, the whole-tree check 1 at both as on the base (its one
row the libevent crate's tip-mode `mk_vhost_set`); rechecked after the last
comment edit (`private/rrk/pin-a3/`).

| kind | declarations |
| --- | --- |
| keep (`resource_reference_kept`, alias result) | `rd_kafka_toppar_keep0`, `rd_kafka_q_keep`, `rd_kafka_q_keep_nolock`, `rd_kafka_topic_keep` |
| drop (`resource_reference_dropped`) | `rd_kafka_toppar_destroy0`, `rd_kafka_q_destroy0`, `rd_kafka_q_destroy_owner`, `rd_kafka_topic_destroy0`, `rd_kafka_enq_once_trigger` |
| owner, existing vocabulary | `rd_kafka_queue_new0` (owned non-null handle; the reference on `rkq` is the handle's), `rd_kafka_topic_partition_list_destroy`, `rd_kafka_topic_partition_destroy0` (literal premise `do_free = 1`), `rd_kafka_AlterConsumerGroupOffsets_destroy`, `_DeleteConsumerGroupOffsets_destroy`, `_ListConsumerGroupOffsets_destroy`, `rd_kafka_topic_partition_result_destroy` (released, non-null) |

Every keep and drop body validates rule 13c in every unit that defines it (no
`reference-withdrawn` on any route). `rd_kafka_topic_partition_destroy0` was
deliberately left undeclared by `topic_partition.qv` on two grounds; the
literal premise answers the conditional release, and the other (W4 is the
lever) does not hold because its body drops the reference `_private` holds
through a member read (`REFCOUNT-MEMBER-REFERENCE-DROP`). `rd_kafka_queue_new0`
also keeps `rk` in its handle; no retention clause names the result as holder,
so `rk` is borrowed with the gap recorded (`RETAINED-BY-RESULT-HOLDER`).
`memory.qv`'s note on the undeclared `rd_atomic32_add`/`_set` records the
decision. Not declared: `rd_kafka_txn_clear_partitions_flag` (a release of the
references a caller's list holds), every `rd_atomic32_*` primitive, the
reference-returning lookups and constructors (§6).

## 5. AFTER, attributed

Binary `private/rrk/bin/a3` (`bfd36e2a…`, equal to `run.json`), built from
`454de4bb8`; the later commits change tests, records and comments only.

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus | 872 / 9 / 3,953 | **884 / 9 / 3,941** | `52d14db1…` → `7a4595d6…` |
| system | 87 / 4 / 357 | unchanged, no row moves | `66091702…` → `42dfd80f…` (both embed this worktree's paths) |
| slice | 45 / 1 / 354, 4 Pending | unchanged, no row moves | `bf3a615c…` → `e9478f68…` |

Corpus **+12 Unknown → Safe**, no other status move
([comparison](artifacts/compare-a3-corpus.json)): over the destructors, the five
`_free` wrappers (`rd_kafka_{Alter,Delete,List}ConsumerGroupOffsets_free`,
`rd_kafka_topic_partition_result_free`, `rd_kafka_topic_partition_list_destroy_free`)
and `rd_kafka_topic_partition_destroy`/`_destroy_free`; over the queue handle
constructor `rd_kafka_queue_get_consumer`, `_get_main`, `_get_sasl` and,
through `_get_sasl`'s new summary, `rd_kafka_share_queue_get_sasl`; over the
route, `rd_kafka_toppar_list_copy` (a keep returned). **No Safe → non-Safe move;
`Unsafe` 9 → 9; no leak or violation added.**

232 more Unknown rows changed leaves only: 138 `reference-drop-unheld` leaves
where `call to uncontracted function` stood (131 through a plain name holding a
reference from an undeclared reference-returning call or the caller's reference,
3 member reads, 4 others: [refusals-a3-corpus.json](artifacts/refusals-a3-corpus.json)),
7 `reference-undecided` (keeps of operands the route cannot name), 5
`reference-unbalanced` (refusals on paths that refused something; one is
`rd_kafka_queue_new0`'s own body, whose store precedes its keep), 90
`unresolved resource identity` leaves in 57 functions at a declared destructor's
own line (the list it releases came from an undeclared constructor or copy,
`scripts/reattribution.py`), and access-gate leaves where a declared call's
member or element actual is now read (the four `_destroy_array` loops and
`rd_kafka_assignment_destroy`, W44-CALL-ARGUMENT-CUSTODY-POSITION).

**The first measurement (`a1`, recorded, not used).** Built before two fixes,
it showed one new `Unsafe`, `rd_kafka_purge_ua_toppar_queues`: a balanced keep
and drop in a loop read as a leak because the loop generations renamed the
family's site without its count. Read by hand, it was a false candidate; the
rename now carries the count, and a field link now hands a reference off only
when its owner escapes. The second (`a2`, `cab4a7121`) and final (`a3`) runs are
byte-identical (`7a4595d6…`); `a3` adds the returned-reference spelling, which
no corpus declaration uses.

Remaining ([ranking](artifacts/rank-a3-corpus.json), [tally](artifacts/tally-a3.txt)):
**522** Unknown functions blocked by uncontracted calls alone, 365 by one callee
alone (545 / 384 before; system 26 / 18, slice 38 / 27 unchanged). Class 3
**9 callees / 19 → 2 / 4**: `rd_atomic32_add` 2 (`rd_kafka_cert_dup`, a keep
spelled through the primitive; `rd_kafka_idemp_inflight_toppar_add`, a counter)
and `rd_kafka_txn_clear_partitions_flag` 2.

**Parity, compared in full** ([corpus](artifacts/parity-corpus.json),
[system](artifacts/parity-system.json), [slice](artifacts/parity-slice.json)):
system (139,962 rows) and slice (50,407) multisets identical; corpus 667,321 →
663,033 rows, 281 functions differ. Outside the moving rows: the bodies of the
nine withheld keep and drop declarations (a withheld declaration is never
replaced by an inference, so their derivation rows go), and header helpers
whose bodies call a keep or drop (`rd_kafka_q_fwd_get`, `rd_kafka_replyq_make`,
`_copy`, `_destroy`, `rd_kafka_set_replyq`, the admin op constructors, unit
tests), which have no report rows.

**`mk_vhost_set` at the tip** ([rows](artifacts/tip-rows-after.txt)):
byte-identical to SLOT-MOVE-AND-INTERIOR-BORROW's record. The BEFORE binary
cannot load this tree's `client.qv` (`unsupported predicate
resource_reference_kept`, the versioning rule: an older loader rejects the new
clauses rather than misreading them), so the previous card's record is the
BEFORE.

## 6. Prediction against outcome

| item | predicted | measured |
| --- | --- | --- |
| queue handle callers | +3 | **+3**, and +1 through a new summary |
| `_free` wrappers | +5 | **+5** |
| `topic_partition_destroy`, `_destroy_free` | +2 | **+2** |
| `_destroy_array` loops | 0 to +4 | **0** (element actuals at a non-null release) |
| `rd_kafka_assignment_destroy` | 0 | **0** |
| `rd_kafka_toppar_list_copy` | +1 | **+1** |
| `rd_kafka_msgbatch_destroy`, `ut_destroy_mock_queue` | 0 | **0** |
| cascade | 0 to +3 | **+1** |
| corpus total | +11 to +18 (central +13) | **+12** |
| primitives | 0 | **0** |
| Safe → non-Safe | none | **none** |
| new leak candidates | 0 to 3 | **0** (one in `a1`, a defect of the card, fixed) |
| re-attributed Unknown functions | 150 to 260 | **232** |
| system / slice | no row moves | **no row moves** |
| class 3 after | 2 to 4 callees / 3 to 8 | **2 / 4** |

## 7. Lean

`QCore.ReferenceCount` (new, 25 audited theorems, registered in
`AuditReferenceCount.lean` and imported by `QCore.lean`; no placeholder, no
project axiom; 296 → 297 audits), and the kind added to `QCore.KindedCustody`
(the use-after-close candidate excludes it, as the heap) and
`QCore.ResourceKinds` (`kindVectors`, bound to `ResourceKind::ALL` by
`tests/resource_kinds_lean.rs`). The brief's five:

* balance soundness: `a_safe_path_is_balanced`;
* a missing drop is never Safe: `a_missing_drop_is_never_safe`,
  `a_kept_reference_never_dropped_leaks`;
* a statistics counter is never a reference:
  `a_statistics_counter_is_never_read_as_a_reference` (a keep or drop is read
  only through a declared reference clause) and `a_counter_changes_no_family`;
* the neutral reading is not admitted: `the_neutral_reading_is_not_admitted`
  (withheld from the function map; the neutral reading would certify the
  missing drop the route reports; a withdrawn declaration refuses);
* an alias use after a possibly-final drop is refused:
  `an_alias_use_after_a_possibly_final_drop_is_refused`,
  `a_use_after_the_last_drop_is_refused_never_accused`.

Also: `an_over_drop_of_an_acquired_family_is_a_violation`,
`an_over_drop_on_a_refused_path_is_not_accused`,
`a_drop_past_a_borrowed_familys_last_drop_is_refused`,
`an_unmodeled_exit_never_leaks`, `a_refused_path_is_never_safe`, the call-site
tables (`a_mint_needs_an_unbound_name`, `a_final_drop_is_the_last_reference`),
and `reference_vectors_hold`, `reference_trace_vectors_hold`, whose rows the
Rust test reads out of the Lean text and compares with `REFERENCE_VECTORS` and
`REFERENCE_TRACE_VECTORS` (9 and 21 rows).

## 8. Ledger

`scripts/update_ledger.py` (follow-ups 720 → 727): `W48-CALLEE-DECISION-TABLE`
amended with the decision verbatim and class 3 re-sized (9 / 19 → 2 / 4);
`W45-CALLEE-CASCADE-RESIDUAL` (522 / 365) and `CG22-CALLBACK-REGISTRY-CONTRACT`
(its reference drops now have vocabulary; nothing declared, its refcount part
not discharged) amended. Registered: `REFCOUNT-REFERENCE-RETURNING-DECLARATIONS`
(131 leaves / 91 functions / 1 alone), `REFCOUNT-PLAIN-COUNTER-DECLARATION` (21
leaves / 20 functions / 4 alone), `REFCOUNT-MEMBER-REFERENCE-DROP` (6 leaves, 1
alone), `REFCOUNT-KEEP-INTO-PLACE` (7 / 7 / 0), `REFCOUNT-CONTAINER-REFERENCE-SET`
(2 alone), `REFCOUNT-INFERRED-REFERENCE-RESULT` (0 measured),
`RETAINED-BY-RESULT-HOLDER` (0 measured).

## 9. Gate

See [gate.json](gate.json) and [gate/](gate/) once run.

## Files

* Rules 13b, 13c: `crates/qcore-c-gateway/src/reference_count.rs`,
  `legacy_reference_count.rs` (new); hooks in `legacy_transfer.rs`,
  `legacy_retained_by.rs`, `legacy_slot_move_borrow.rs`,
  `legacy_loop_generations.rs`, `legacy_descriptor_custody.rs`, `legacy_c.rs`,
  `lib.rs`; schema `crates/qcore-schemas/src/{reference_count.rs,retained_by.rs,contract.rs,lib.rs}`;
  loader `crates/qcore-contract/src/{contract_reference_count.rs,contract_loader.rs,contract_retained_by.rs,descriptor_contract_binding.rs}`.
* Lean: `proof/QCore/ReferenceCount.lean`, `proof/AuditReferenceCount.lean`,
  `proof/QCore/{KindedCustody,ResourceKinds}.lean`, `proof/QCore.lean`.
* Tests: `crates/qcore-c-gateway/tests/refcount_resource_kind.rs`,
  `tests/fixtures/contracts/refcount.qv`,
  `crates/qcore-contract/tests/reference_clause.rs`; deliberate pin updates in
  `compiler/tests/{contract_loader,qcore_contract_corpus}.rs` (333 → 349 v1
  declarations, retaining 16 → 25, sixteen clause rows),
  `crates/qcore-c-gateway/tests/clang_gap_phase4.rs` (six kinds → seven) and
  `crates/qcore-contract/tests/resource_kinds_cg25.rs`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/{client,memory}.qv`,
  `contracts.lock`, `contracts.json`.
* Records: contract spec C04.6, C04.17; grammar G05; `docs/agents/gateway.md`
  (13b, 13c), `docs/agents/README.md`, `docs/STATUS.md`,
  `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: adapted from SLOT-MOVE-AND-INTERIOR-BORROW, plus `census.py`,
  `refusals.py`, `reattribution.py` and `build-measure.sh`. Rule ids `13d`–`13f`
  were not needed.
