# SLOT-MOVE-AND-INTERIOR-BORROW: slot moves, borrowed results, derived out slots

Card SLOT-MOVE-AND-INTERIOR-BORROW (wave 48, card-high). Base: local `main` at
`6cbcc4e9c`, which it stayed at. [PREDICTION.md](PREDICTION.md) was committed
at `073a267a9`, before any implementation code or contract. Authority:
`W48-CALLEE-DECISION-TABLE` classes 2 and 8 and `INTERIOR-OUT-SLOT-MODE`.

**Published property and scope.** Custody (`c.custody`) on the pinned corpus,
the system route and the tail-pause slice, as before. The card adds three
clauses to resource vocabulary version 1 and one structural body check; it
adds no property.

## 1. The decision

**USER DECISION 2026-10-05**, recorded verbatim in the contract specification
(C04.3.4, which C04.15 and C04.16 cite) and in `W48-CALLEE-DECISION-TABLE`:
class 2 is a slot move (the slot still owns an object, possibly a new one;
every alias of the old pointer is invalid; on failure the original stays
owned; it composes with rule 11r where success and failure split by return
code); class 8 is an interior borrow of the parent (no release obligation;
valid only while the parent lives and is not mutated through another path;
a write through it is a write to the parent; a lazily created child is owned
by the parent; not contradicting the interim arena reading); and
`INTERIOR-OUT-SLOT-MODE` is absorbed as a derived out-slot mode under
C04.3.1's rules.

## 2. BEFORE and the members

Reproduced (binary `private/smib/bin/before`, `03a961a7…`, equal to
`run.json`): corpus **846 / 9 / 3,979** (`686fca16…`, the brief's), system
87 / 4 / 357, slice 45 / 1 / 354 with 4 Pending. Ranking population 571, 403
decided alone. Every member's size reproduces the ledger (class 2: 7 callees /
18 functions alone; class 8: 6 / 16; out slot: `rd_slice_reader0` 2,
`cJSON_ParseWithLengthOpts` 2, the `strto*` 9 leaves / 9 functions, none
alone): [size-before-corpus.json](artifacts/size-before-corpus.json) and the
table in [PREDICTION.md](PREDICTION.md).

**Reading the bodies** (`scripts/body.py`, every one byte-identical at
`66910c10` and `046accf6`):

* pure slot moves: `cfl_sds_cat_safe` (grows `*buf` through `cfl_sds_cat`,
  whose `realloc` may move it and whose failure leaves it), `sds_cat_safe`
  (forwards its slot), and the four `format_metric` bodies (forward their slot
  to `cfl_sds_cat_safe` **and** to undeclared static helpers);
* **not** slot moves: `mk_header_prepare` (an in-place lifecycle of `sr`'s
  iov with free-on-release flags), `rd_kafka_parse_Metadata0` (fresh custody
  through an out slot on success), `setupRackAwareAssignment0` and
  `ut_init_member_internal` (unit-test constructors into caller storage);
* `flb_config_task_map_grow` reallocates `config->task_map`, a field reached
  through a borrowed parameter, not a `T **` slot: the caller cannot enumerate
  the aliases of the old field value, so the slot reading does not cover it
  honestly; recorded, not declared (`SLOT-MOVE-FIELD-SLOT`);
* class 8: the admin result arrays (`rko->…results.rl_elems`), a cJSON child
  (`object->child` along `->next`), a lazily created `_private`, and
  `get_or_create_external_metadata_kvlist`, whose child comes from
  `cfl_kvlist_fetch` or a create-and-insert (not declared,
  `BORROW-CREATED-INSERTED-CHILD`);
* out slots: `rd_slice_reader0` writes `seg->seg_p + rof` (reached from
  `slice`); `strtol`, `strtoll`, `strtoul`, `strtoull`, `strtod` write into
  `nptr` through a nullable `endptr`; `cJSON_ParseWithLengthOpts` writes into
  `value` through `return_parse_end` and keeps `value` in `global_error`.

## 3. The rules

The full text is in [gateway.md](../../agents/gateway.md#constraint-12x)
(rules 12x, 12y, 12z, 13a) and the contract specification (C04.3.4, C04.15,
C04.16; grammar G05). In short:

* **12x, slot move** (`resource_slot_moved(s, n)`, withheld from the function
  map with the retaining declarations). `&L` holding exactly one live identity
  that no structure but names holds: `L` keeps it and its one obligation, and
  every other name bound to it is invalidated (its binding dropped; a later
  read refuses, `slot-move-alias`, never accused, since the object may not
  have moved). A caller's formal slot named only at slot-moving positions is
  passed through; NULL under `nullable` moves nothing; everything else refuses
  (`slot-move-undecided`). The failure arm is the slot still owing its object.
  The return-code composition is stated and not admitted: no member returns a
  code (`SLOT-MOVE-RETURN-CODE-SPLIT`).
* **12y, borrowed result** (`resource_return_borrowed_from(p, n)`, the derived
  result at a new `Borrowed` extent, in the function map). It owns nothing; a
  release of it is refused; a call or store ends it unless every name it may
  write is in the borrow's family (its aliases and the borrows taken from them)
  or a declared scalar; an actual the reader does not name, the parent's
  release and a rebound parent end it; a later read refuses
  (`interior-borrow-ended`) and the settled-state reports that would have
  accused a use after the parent's release refuse instead; a store through a
  non-null borrow is a write to the parent (it keeps the borrow, ends its
  siblings). Every consumer of `DerivedExtent` was audited: arithmetic and
  re-derivation keep `Borrowed`, the derived-result inference publishes
  `Borrowed` when any exit is borrowed, and only a `Borrowed` declaration
  covers a borrowed exit, so a wrapper's callers keep the ending rules. The
  parent may be spelled through a pointer cast of a place (for this extent
  only, `DERIVED-BASE-CAST-READING`). The interim `cfl_arena_calloc` interior
  reading is unchanged (C04.3.4 relates the two: a borrow is the stricter
  reading of the same relation; no arena result is read as borrowed).
* **12z, derived out slot** (`resource_out_derived(s, p, n)`, withheld). `&L`
  holding no tracked identity becomes a pointer derived from `p`, read at the
  `Borrowed` extent with no borrow record (the slot may keep its old value, so
  its release and its use after `p`'s release are refused, never accused); a
  tracked `L` refuses (the write could lose it).
* **13a, body checks.** In a defining unit, each declaration stands only when
  the body validates it structurally (slot mentions only at slot-moving
  positions; stores through the out slot reached from the base; every return
  NULL or reached from the parent by member reads, through so-assigned
  locals, or a created child linked into a member of the parent); otherwise it
  is withdrawn there and an inferred summary of the body governs.

Shared items added (named in the rules): `DerivedExtent::Borrowed`,
`RetainingContract::{slot_moves, derived_outs, retains}`,
`ResourceContracts::withdraw_declaration`, `AbstractState::{invalidated,
borrows}`, `retained_by_route::Operands::slots`, `Parser::slot_address`, the
`transfer` wrapper's `before`/`after` hooks, `retention_guard::definition`
made crate-visible.

**Controls** ([tests/slot_move_interior_borrow.rs](../../../crates/qcore-c-gateway/tests/slot_move_interior_borrow.rs)):
a use and a release of the old alias after a slot move are refused (never a
double release); the failure arm (the slot's object still owed: one release
is Safe, none is a leak); a caller's slot passed through is Safe; an interior
use after the parent's release and after an invalidating mutation are refused;
a write through one borrow ends its sibling; a release of an interior pointer
is refused (never `invalid_release`); a lazily created child needs no release
and its parent's release last is Safe; a forwarded borrow keeps its ending
rules through the wrapper's summary; the derived out slot owns nothing and
never overwrites a tracked identity; a declaration its body does not validate
is withdrawn in that unit. Loader controls in
[crates/qcore-contract/tests/slot_move_clause.rs](../../../crates/qcore-contract/tests/slot_move_clause.rs).

## 4. Contracts

Each body read at both revisions; the pinned ones pinned with `quirin-audit
pin` at `66910c10` and `046accf6` (logs `private/smib/pin-a1/`), the libc ones
flexible; every changed crate's `pin --check` 0 at both, `--verify-contracts`
0, the whole-tree check 1 at both as on the base (its one row the libevent
crate's tip-mode `mk_vhost_set`) (`private/smib/pin-a1b/`).

| crate, module | declaration | clause |
| --- | --- | --- |
| `quirin-provider-cfl` `sds_lifetime.qv` | `cfl_sds_cat_safe` | `resource_slot_moved(buf, nullable)` (withdrawn by 13a in `cfl_sds.c`, which stores through the slot) |
| `qcore-provider-ctraces` `context_lifetime.qv` | `sds_cat_safe` | `resource_slot_moved(buf, nullable)` (13a validates it in `ctr_encode_text.c`) |
| `qcore-provider-librdkafka` `cjson.qv` | `get_object_item` | `resource_return_borrowed_from(object, nullable)` |
| | `cJSON_ParseWithLengthOpts` | `resource_retained_by(value, global)`, `resource_out_derived(return_parse_end, value, nullable)`, owned nullable result (withdrawn by 13a in `cJSON.c`, its only unit) |
| `qcore-provider-librdkafka` `client.qv` | `rd_kafka_admin_result_ret_topics`, `_resources`, `_groups` | `resource_return_borrowed_from(rko, nullable)`, result `ptr<mut, ptr<const, u8>>` |
| | `rd_kafka_topic_partition_get_private` | `resource_return_borrowed_from(rktpar, nonnull)` (a lazily created child) |
| | `rd_slice_reader0` | `resource_out_derived(p, slice, nonnull)` |
| `quirin-provider-libc` `stdlib.qv` (flexible) | `strtol`, `strtoll`, `strtoul`, `strtoull`, `strtod` | `resource_out_derived(endptr, nptr, nullable)` |

Not declared: `format_metric`, `mk_header_prepare`, `rd_kafka_parse_Metadata0`,
`setupRackAwareAssignment0`, `ut_init_member_internal`,
`flb_config_task_map_grow`, `get_or_create_external_metadata_kvlist` (§2).

## 5. AFTER, attributed

Binary `private/smib/bin/a2` (`8f811155…`, equal to `run.json`); the gate's
code differs from it only by `cargo fmt`, a clippy auto-deref and test edits.

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus | 846 / 9 / 3,979 | **872 / 9 / 3,953** | `686fca16…` → `52d14db1…` |
| system | 87 / 4 / 357 | unchanged, no row moves | `95df8068…` → `ac78c496…` (both embed this worktree's paths) |
| slice | 45 / 1 / 354, 4 Pending | unchanged, leaves only | `490bca64…` → `0e4b84e0…` |

Corpus **+26 Unknown → Safe**, no other status move
([comparison](artifacts/compare-a2-corpus.json)): over `sds_cat_safe` the
four `ctr_encode_text.c` formatters; over `cfl_sds_cat_safe`
`format_metric_type`, `append_bytes_value` and `sds_cat_safe` itself; the ten
`rd_kafka_*_result_{topics,resources,groups}` wrappers over the three admin
accessors (each base a cast, read by `cast_place`); `cJSON_GetObjectItem` and
`_CaseSensitive` over `get_object_item`; `rd_kafka_topic_partition_set_leader_epoch`,
`_set_current_leader_epoch` and `_set_topic_id` over the lazily created
`_private` (each a store through the borrow); `rd_slice_reader` and
`rd_slice_peeker` (caller slots passed through); and two through the new
summaries: `cJSON_HasObjectItem` (the borrowed summary of `cJSON_GetObjectItem`)
and `rd_kafka_topic_partition_set_from_fetch_pos`. **No Safe → non-Safe move;
`Unsafe` 9 → 9; no leak or violation added.**

66 more Unknown rows changed leaves only, each family read: 146 leaves
of slot refusals where `call to uncontracted function` stood, in the cmetrics
and ctraces encoders (a formal slot also handed to an undeclared static
helper, or a member slot `&…->buf`; `SLOT-MOVE-SUMMARY-INFERENCE`) and 8
`slot-move-undecided` (`ctr_encode_text_create`, `bucket_value_to_string`,
`double_to_string`, `format_metric_name`: the slot's local holds no single
live identity there); 35 `interior-borrow-ended` leaves in four
`rdkafka_partition.c` functions that use `_private` after another call
(`BORROW-ENDING-PRECISION`); two `abi-unreadable-declaration` withdrawals of
`strtoll`/`strtoull` (glibc's leading `__extension__`, amended into
`W43-LIBC-ABI-READER-SHAPES`); later leaves the uncontracted call hid
(unresolved identities, the access gate); and rows that only lost leaves
(three `cJSON.c`, five `rdbuf.c`, several encoder helpers). Slice: `configured_limit` and
`flb_input_set_property` (both units) trade the uncontracted `strto*` call
for nothing or the `strtoull` withdrawal.

Remaining ([tally](artifacts/tally-a2.txt), [ranking](artifacts/rank-a2-corpus.json)):
**545** Unknown functions blocked by uncontracted calls alone, 384 by one
callee alone (571 / 403 before; system 26 / 18, slice 38 / 27 unchanged).
Classes after the card ([size-after-corpus.json](artifacts/size-after-corpus.json)):
class 2 **5 callees / 11** (`format_metric` 3 and the four non-slot shapes),
class 8 **1 / 2** (`get_or_create_external_metadata_kvlist`); a new head,
`cJSON_DetachItemViaPointer` 3, surfaced where `get_object_item`'s leaf
cleared.

**Parity, compared in full** ([corpus](artifacts/parity-corpus.json),
[system](artifacts/parity-system.json), [slice](artifacts/parity-slice.json)):
system multiset identical (139,962 rows); slice 50,426 → 50,407 rows, two
functions (the `strto*` callers); corpus 668,005 → 667,321 rows, 91 functions
differ. Outside the moving set: `rd_slice_reader0`'s own body (61 rows; a
withheld declaration is never replaced by an inference), the declared
borrowed accessors' bodies (`get_object_item` 22, each `ret_*` 21 rows, now
checked against their declarations) and `replace_item_in_object` (25, which
nests `get_object_item`), and the function-less summary-premises graph (7
rows each side).

**`mk_vhost_set` at the tip** ([rows](artifacts/tip-rows-after.txt)):
byte-identical to W43-ABI-ENUM-AND-CONST-PP's record.

## 6. Prediction against outcome

| item | predicted | measured |
| --- | --- | --- |
| class 8 | +10 to +14 | **+15** (one above: `_set_topic_id` also stores through `_private`) |
| class 2 | +4 to +7 | **+7** (the ceiling) |
| out slot | +2 | **+2** |
| cascade | 0 to +6 | **+2** |
| corpus total | +16 to +29 (central +23) | **+26** |
| Safe → non-Safe, leaks, violations | none | none |
| re-attributed Unknown functions | 20 to 50 | **66** (above: the encoders' slot refusals) |
| system / slice | 0 to 2 / none | none / leaves only |

## 7. Lean

Three modules, registered in `AuditSlotMove.lean`, `AuditInteriorBorrow.lean`
and `AuditDerivedOutSlot.lean` and imported by `QCore.lean`; no placeholder,
no project axiom (293 → 296 audits):

* **`QCore.SlotMove`** (14 audited): the call-site table
  (`a_move_needs_one_identity_held_nowhere_else`,
  `an_identity_held_elsewhere_is_refused`); the replay:
  `the_slot_keeps_its_identity_and_its_obligation` (the slot owns exactly one
  object before and after), `every_old_alias_is_invalid_after_a_move`,
  `a_use_of_an_old_alias_is_refused`, `the_failure_arm_keeps_the_original`,
  `a_safe_path_never_uses_an_invalid_name`; `slot_vectors_hold` and
  `slot_trace_vectors_hold` bind `SLOT_VECTORS` and `SLOT_TRACE_VECTORS`.
* **`QCore.InteriorBorrow`** (17 audited): `a_borrow_release_is_refused` (no
  release obligation), `a_parent_release_ends_its_borrows`,
  `an_invalidating_mutation_ends_a_borrow`, `a_use_after_the_end_is_refused`,
  `a_write_through_a_borrow_keeps_it`, `a_write_through_a_borrow_ends_its_sibling`,
  `a_lazily_created_child_is_owned_by_the_parent`,
  `a_safe_path_never_uses_an_ended_borrow`; `borrow_trace_vectors_hold` binds
  `BORROW_TRACE_VECTORS`.
* **`QCore.DerivedOutSlot`** (9 audited): `a_tracked_slot_is_never_overwritten`,
  `a_bind_needs_an_untracked_slot`, `the_slot_value_is_never_released`,
  `the_base_release_ends_the_slot_value`,
  `a_safe_path_never_uses_a_value_its_base_outlived`; `out_vectors_hold` binds
  `OUT_VECTORS`.

## 8. Ledger

`scripts/update_ledger.py` (follow-ups 715 → 720): `W48-CALLEE-DECISION-TABLE`
amended with the decision verbatim and classes 2 and 8 re-sized (7 / 18 → 5 /
11, 6 / 16 → 1 / 2); `W45-CALLEE-CASCADE-RESIDUAL` (545 / 384),
`CLASS1-UNDECLARABLE-RETAINERS` (`cJSON_ParseWithLengthOpts` declared,
withdrawn in its own unit), `W43-LIBC-ABI-READER-SHAPES` (the `__extension__`
prototypes) and `RETAINED-BY-BODY-CHECK` amended; **`INTERIOR-OUT-SLOT-MODE`
retired** (its discharge, a derived out-slot mode with Lean parity, holds; its
undischarged member is rehomed in `CLASS1-UNDECLARABLE-RETAINERS`). Registered:
`SLOT-MOVE-SUMMARY-INFERENCE`, `SLOT-MOVE-FIELD-SLOT`,
`SLOT-MOVE-RETURN-CODE-SPLIT`, `BORROW-ENDING-PRECISION`,
`BORROW-CREATED-INSERTED-CHILD`, `DERIVED-BASE-CAST-READING`.

## 9. Gate

GATE-SECTION

## Files

* Rules 12x, 12y, 12z, 13a: `crates/qcore-c-gateway/src/slot_move_borrow.rs`,
  `legacy_slot_move_borrow.rs` (new); hooks in `legacy_transfer.rs`
  (`AbstractState`, `transfer`, `derive_pointer`, the settled-state reports,
  `scalar_member_store_base_resolves`, `Derivation::step`),
  `legacy_retained_by.rs`, `legacy_classify.rs`, `legacy_token_shapes.rs`
  (`cast_place`, `interior_extent`), `legacy_derived_summary.rs`,
  `legacy_summary.rs`, `legacy_retention_guard.rs`, `legacy_c.rs`, `lib.rs`;
  schema `crates/qcore-schemas/src/{slot_move.rs,retained_by.rs,contract.rs,lib.rs}`;
  loader `crates/qcore-contract/src/{contract_slot_move.rs,contract_loader.rs,contract_retained_by.rs}`.
* Lean: `proof/QCore/{SlotMove,InteriorBorrow,DerivedOutSlot}.lean`, their
  `Audit*.lean`, `proof/QCore.lean`.
* Tests: `crates/qcore-c-gateway/tests/slot_move_interior_borrow.rs`,
  `tests/fixtures/contracts/slot_move_borrow.qv`,
  `crates/qcore-contract/tests/slot_move_clause.rs`; deliberate pin updates in
  `compiler/tests/{contract_loader,qcore_contract_corpus}.rs` and
  `crates/qcore-c-gateway/tests/clang_gap_phase4.rs` (the extent set).
* Contracts: the five modules of §4, their `contracts.lock`, the
  `qcore-provider-*` `contracts.json`.
* Records: contract spec C04.3, C04.3.4, C04.14, C04.15, C04.16; grammar G05;
  `docs/agents/gateway.md` (12x–12z, 13a), `docs/agents/README.md`,
  `docs/STATUS.md`, `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: adapted from W43-ABI-ENUM-AND-CONST-PP (build, measure, rank,
  tally, compare, parity, pin, gate, provider gate, finish, size, tip probe),
  plus `targeted.sh` and `update_ledger.py`. Rule id `13b` was not needed.
