# UNION-OPAQUE-RECORD-WRITES-BY-TYPE: opaque record writes keyed by target

Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE (wave 48, card-high). It amends
gateway rule **12n** in place and contract specification **C04.19.3**'s
"Keys" bullet; no new rule id and no new surface word (the keying is
internal; no clause changes). Lean: `QCore.UnionArmLayout`, extended. Base:
local `main` `1c7333db1`. The census and the frozen
[prediction](PREDICTION.md) were committed (`5271df756`) before any
implementation code.

## 1. The census, and a second keying defect

The two members `rd_kafka_poll_cb` invokes, `rd_kafka_op_s.rko_u.offset_commit.cb`
and `rko_u.terminated.cb`, read open over librdkafka's 94 units at the base
([`member_facts_census`](../../../crates/qcore-audit/examples/member_facts_census.rs),
[census](artifacts/census.json)) with the same four reasons as the corpus
sidecar: a cast free callback, the unit tests' `memmove` of
`rd_kafka_group_member_s`, and whole-object writes of `rd_kafka_fetch_pos_s`
and of `rd_kafka_replyq_s`.

**Correction to the ledger entry.** The entry named only the
`rd_kafka_fetch_pos_s` writes (placed in `rko_u` at bytes 0..16, meeting the
members at 8). The other three reasons reach the members because
`Tables::normalise` keyed a **declared** path into a union
(`rd_kafka_op_s.rko_u.offset_commit.cb`) as the whole union
`rd_kafka_op_s.rko_u.*`, dropping its arm, so the layout route of
RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION never applied to a declared member:
it aliased every place and held record of `rko_u`. Only a read keyed by
`callback_key` (`rko->rko_u.leaders.cb`) kept its arm.

Every opaque write that opened the members, read by hand
([`opaque_sites.py`](scripts/opaque_sites.py) over the 94 units, Clang's AST
of each unit's own command; layout from the six units that probe `rko_u`, one
agreeing layout, the three members at union bytes **8..16**):

| Write | File:line (function) | Target | Targets `rko_u`? | Byte range | Overlap with 8..16 |
| --- | --- | --- | --- | --- | --- |
| whole `rd_kafka_fetch_pos_s` = call | `rdkafka_request.c:1313` (`rd_kafka_handle_OffsetFetch`), `rdkafka_cgrp.c:4322` (`rd_kafka_cgrp_update_committed_offsets`), `rdkafka_partition.c:1382` (`rd_kafka_toppar_offset_commit_result`) | `rktp->rktp_committed_pos`, below a pointer to `rd_kafka_toppar_s` | no: no record holds `rd_kafka_toppar_s` by value | none in `rko_u` (the type's placement there is 0..16) | none |
| the same | `rdkafka_partition.c:2233` (`rd_kafka_toppar_op_serve`) | local `pos` | no | none | none |
| whole `rd_kafka_replyq_s` = `RD_KAFKA_REPLYQ(...)` | `rdkafka.c:3410`, `:3647`, `:4187`, `:4470` | `rko->rko_replyq`, the op's own field outside `rko_u` | no | none (the type's placements: 136..152, 808..824) | none |
| the same | `rdkafka.c:4752`, `rdkafka_offset.c:400`, `rdkafka_request.c:3110` | locals | no | none | none |
| `memmove` of `rd_kafka_group_member_s` | `rdkafka_sticky_assignor.c:2878`, `:3007`, `:3089` (unit tests) | `&members[i]`, an element of a local array | no | reaches `rd_list_s.rl_free_cb` (24, 88, 744, 872) | none |
| cast store (not an opaque write) | the seven `rd_list_init` / `rd_list_new` free callbacks | `rd_list_s.rl_free_cb` of lists outside `rko_u` | no | 24, 88, 744, 872 | none |

No opaque write of any other record `rko_u` holds at bytes meeting 8..16,
and none of `rd_kafka_op_s`, exists in the 94 units; no record holds
`rd_kafka_toppar_s` or `rd_kafka_op_s` by value. **No opening write
overlaps the members' bytes.**

## 2. Mechanism and why it is sound

* **Declared spelling keeps its arm** (`Tables::walk`): a declared path that
  reaches a union keys `U.*@rest`, as a read of the same place does; the
  whole union only when nothing follows. A rest the layout does not give is
  unplaced and aliases everything, as before.
* **Opaque writes keep their target** (`MemberFacts::opened_targets`,
  `OpaqueTarget`, `MemberFacts::opaque_target`, read as `callback_key` reads
  an assignment's place): a place a union holds (`U.*@arm.path`; an
  assignment there is also an open store at that place, so `places_alias`
  decides it by its own bytes), a separate object (a local, global or
  by-value formal, or a member path in one), a member path below a pointer
  to a named record, or untargeted (a dereference, an array element, a
  pointer handed to a copier, an initializer list).
* **Closure** (`closure_prepared`): for a union target, a held record's
  opaque write is read by the record's placements (`meets_record`) only
  where the write may lie in that union (`Tables::may_lie_in`,
  `opaque_write_may_lie`): untargeted, into another union, below a record
  the union holds by value, or on a path the tables do not read as meeting
  no union (`path_avoids_unions`: every reading of every record on the way,
  an unread record, an anonymous member or a non-record step fails it).
  Facts without targets are read by type. The non-union readings (a member
  keyed by its record, its containers) stay type-keyed. A whole-union
  `memcpy`/`memset`-family write and an address escape are unchanged. The
  fold stays demote-only.

**Soundness.** The members' run sets are read under rule 12n's premise
`P-CG22-MEMBER-CLOSED-WORLD`. A write whose target is a separate object
writes bytes of that object only: it is not inside any union object
(`a_separate_write_never_changes_the_read`). A member path below a pointer
to record R lies in a union's bytes only if the union holds R by value, or
if the path itself passes through a union; the second is decided by the
tables and fails closed on anything they do not read. A write into a union
place is an open store at that place, decided by its own range, and an
unknown range aliases (`an_unknown_range_opaque_write_aliases`). The target
reading only narrows the type reading
(`the_target_reading_only_narrows_the_type_reading`): it never adds a
candidate or a reason. The new open store at a union place can add a
reason to a member of another record placed on those bytes, which is
fail-closed; none was measured.

## 3. Tests

`crates/qcore-c-gateway/tests/union_opaque_writes.rs` (Clang-backed):

| Brief's case | Test |
| --- | --- |
| a separate-object write does not key into the union | `a_separate_object_write_does_not_key_into_the_union` (below an unheld record, a local, a by-value formal, a global's member, a `memcpy` into one; with the targets dropped the same writes open by type) |
| a union-arm write that does not overlap does not open | `a_union_arm_write_that_does_not_overlap_does_not_open` |
| an overlapping write opens | `an_overlapping_write_opens` (the arm on the read's bytes, and a path below a record the union holds) |
| an unknown range opens | `an_unknown_range_opens` (a union place the layout does not give, `*p = ...`, a `memcpy` through a pointer) |

Also `the_layout_places_the_record_on_and_off_the_read` and
`a_declared_member_in_a_union_keeps_its_arm`. Two existing expectations
change deliberately: `rko_union_layout.rs`'s case 4 (a separate
`other_s->pos` write no longer opens; dropped targets still do) and
`app_rk_callbacks.rs`'s `a_unions_members_share_their_stores`
(`holder_s.slot.b` normalises to `slot_u.*@b`, not `slot_u.*`; the closure
assertion is unchanged). `opaque_vectors_hold` checks `OPAQUE_VECTORS`.

## 4. Measurement

BEFORE `before` (`a063a291…`, built at `1c7333db1`) and AFTER `a1`
(`44412766…`, built at `95f2b42bc`; later commits apply rustfmt and change records only),
each `run.json` naming its binary, every route with the null property
([summary](artifacts/summary.json), [`measure.sh`](scripts/measure.sh)):

| Route | BEFORE | AFTER | Report sha256 | Wall (s) |
| --- | --- | --- | --- | --- |
| Corpus | 938 / 9 / 3,887 | **938 / 9 / 3,887** | `1e76a508…` → `9076ae48…` | 702.4 → 706.3 |
| System | 93 / 4 / 351 | the same | `ba0eace3…` byte-identical | 197.4 → 197.1 |
| Slice | 52 / 1 / 347, 4 Pending | the same | `f71f5d30…` byte-identical | 153.5 → 154.8 |
| Null property | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | every row identical (4,921, 237, 271 rows) | | |

The BEFORE corpus report equals the one HANDOFF-DIRECT-CALL-READING
published for main (`1e76a508…`). Wall times measured once each on a shared
host; not a performance claim.

* **The run's sets** ([corpus sidecar](artifacts/callback-members-corpus.json)):
  `rko_u.offset_commit.cb` and `rko_u.terminated.cb` **close over four**:
  `rd_kafka_offset_broker_commit_cb` (`rd_kafka_commit0`'s formal `cb`),
  `rd_kafka_decommissioned_broker_thread_join` (`terminated.cb`), and
  `rd_kafka_coord_rkb_monitor_cb`, `rd_kafka_txn_coord_monitor_cb` (through
  `broker_monitor.cb` from `rd_kafka_broker_monitor_s.rkbmon_cb`); the
  application's `rd_kafka_conf_s.offset_commit_cb` closes empty. Three arms
  share bytes 8..16, so the layout cannot separate them (a discriminant
  would, `RKO-U-ARMS-BY-DISCRIMINANT`). `rko_op_cb` (27) and `rko_serve` (4)
  unchanged; `rd_list_s.rl_free_cb` still open on its own two reasons. The
  fold demoted nothing.
* **Moving rows** ([compare](artifacts/compare-corpus.json)): exactly the
  four predicted, all Unknown before and after, one leaf each, no status
  move, no leak, violation or Unsafe added:

| Row | Leaf before | Leaf after |
| --- | --- | --- |
| `rd_kafka_consume0` | `callback-member-open` at `rko_u.offset_commit.cb` (the three reasons) | `callback-uncontracted`: `rd_kafka_coord_rkb_monitor_cb` (from `rko_u.offset_commit.cb`) |
| `rd_kafka_consumer_close0` | the same | the same |
| `rd_kafka_offsets_for_times` | `callback-own-invocation-refused` (`rd_kafka_poll_cb`'s own invocation, `callback-member-open`) | `callback-own-invocation-refused` naming `rd_kafka_coord_rkb_monitor_cb` (`callback-uncontracted`) |
| `rd_kafka_query_watermark_offsets` | the same | the same |

* **No Safe gain, no Safe → non-Safe move**, so no Safe row to list with
  its clauses, premises or the x86-64 layout reading. The dependency counts
  and the Safe roll-up are identical.

**Next blocker per row** ([grouping](artifacts/blockers-corpus.json)):

* `rd_kafka_consume0`: the four uncontracted candidates at the call
  (`RKO-U-SHARED-BYTES-CANDIDATES`); then five uncontracted callees
  (`rd_kafka_app_poll_start`, `rd_kafka_app_polled`,
  `rd_kafka_fetch_op_app_prepare`, `rd_kafka_message_get`,
  `rd_kafka_set_last_error`), three unresolved identities and the loop
  condition.
* `rd_kafka_consumer_close0`: the same candidates; then uncontracted
  `rd_kafka_consumer_close_q`, two `reference-drop-unheld`, two
  `retention-undecided`, unresolved identities and indirect accesses.
* `rd_kafka_offsets_for_times`, `rd_kafka_query_watermark_offsets`: the same
  candidates through `rd_kafka_poll_cb`'s own invocation; then
  `callback-registry-undecided` at `rd_list_destroy(&leaders)`, uncontracted
  `rd_kafka_topic_partition_list_query_leaders` and
  `rd_kafka_ListOffsetsRequest`, `reference-drop-unheld`,
  `retention-undecided` and indirect accesses (14 and 24 other leaves).

**Prediction against outcome.** As predicted: both members lose all four
reasons and close (over the four candidates the prediction named, the
`rd_kafka_commit_queue` formal adding none), no status move, exactly the
four rows trade one leaf each for `callback-uncontracted`, system, slice
and null unchanged. Not foreseen at the brief: the declared-spelling
defect (section 1), found in the census before implementation.

## 5. Lean

`proof/QCore/UnionArmLayout.lean`, audited by `proof/AuditUnionArmLayout.lean`
(an existing audit, extended): nine theorems added, standard axioms only,
no placeholder, no project axiom:
`the_target_reading_only_narrows_the_type_reading`,
`an_untargeted_write_is_read_by_type`,
`a_separate_object_never_keys_into_the_union`,
`a_write_below_an_unheld_record_never_keys_into_the_union`,
`a_union_write_is_decided_by_its_place`,
`a_separate_write_never_changes_the_read`,
`a_non_overlapping_opaque_write_never_changes_the_read`,
`an_unknown_range_opaque_write_aliases`, `opaque_vectors_hold` (against
Rust's `OPAQUE_VECTORS`). Lean audits stay **329**.

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 787 → **787**.

* **Retired:** `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` (each opening write
  classified by its target, the type reading kept for an untargeted write,
  Lean parity, measured).
* **Amended:** `RKO-U-ARMS-BY-DISCRIMINANT` (the three arms on bytes 8..16
  and what a discriminant would narrow), `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`
  (`rko_u.offset_commit.cb` no longer read as the whole union).
* **Registered:** `RKO-U-SHARED-BYTES-CANDIDATES` (the four uncontracted
  candidates, 4 rows).

## 7. Gate

Gate **g2** ([gate.json](gate.json), exits in [gate/](gate/), logs
`private/uorw/gate-g2/`): the full standing gate, with the provider gate run
as well (not required: `providers/` is unchanged; run so the regenerated
matrix carries a measured provider count). Every code step ran at
**`dfc4e26b7`** on `1c7333db1`; later commits change records only. One
exclusive flock on the shared heavy lock covered the whole gate
(02:20:22Z–02:48:27Z, [`gate-run.sh`](scripts/gate-run.sh),
[`gate-whole.sh`](scripts/gate-whole.sh), [`gate.sh`](scripts/gate.sh),
[`finish-gate.sh`](scripts/finish-gate.sh)). Lean 4.35.0.

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets`, submodule, `fmt --check`, `clippy --locked -D warnings` | 0, 0, 0, 0 | |
| `run_test_binaries.py` | 0 | 504 units, 517 summaries, **8,151 passed, 0 failed, 15 ignored** (`private/uorw/tests-g2/`), no rerun |
| both reevaluation probes | 0, 0 | |
| `check_lean_audits.py` | 0 | **329** audits passed |
| provider gate | 0 each | `package.py` build, verify `qcore.libevent` and `qcore.librdkafka`; `cargo check --locked`; `contracts.py check`; `cargo test` 133 passed / 0 failed; providers Python 25 OK; fmt; clippy |
| Python suite (08), docs index check (09) | 0, 0 | 374 tests |
| `build_matrix.py . --gate gate.json` (09a), Python suite (09b, 374 tests), docs index (10b, 10c) | 0, 0, 0, 0 | |

The gate's own `cargo clean --profile dev` (07z) ran after the gate under the lock and exited 0 ([gate/](gate/)); on the integrator's disk notice the worktree's `target/` (59.8 GiB) and `providers/target` (8.9 GiB) were also cleaned in full. The bold rule-id duplicate
check is empty. Clang: [`gate/clang-version.txt`](gate/clang-version.txt).

**Discarded attempt:** gate **g1** at `c763227c5` (logs
`private/uorw/gate-g1/`) failed `cargo fmt --check` and the provider
workspace's `fmt --check` (exit 1 each, the same rustfmt diff in
`callback_member_closure.rs`); `dfc4e26b7` applies rustfmt and changes
nothing else. Its other steps were 0 (8,151 passed, 329 Lean audits).

## Files

* `crates/qcore-c-gateway/src/callback_member_closure.rs`;
  `crates/qcore-c-gateway/tests/union_opaque_writes.rs`,
  `tests/rko_union_layout.rs`, `tests/app_rk_callbacks.rs`.
* `proof/QCore/UnionArmLayout.lean`, `proof/AuditUnionArmLayout.lean`.
* `docs/agents/gateway.md` (12n), `docs/agents/README.md`,
  `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.3), `docs/STATUS.md`,
  `docs/development/increments/wave-48-part8.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`.
