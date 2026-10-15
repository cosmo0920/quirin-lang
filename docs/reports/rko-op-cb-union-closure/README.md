# RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: rko_u's places by layout

Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (wave 48, card-high). It amends
gateway rule **12n** in place and contract specification **C04.19.3**'s
"Keys" bullet; no new rule id (13l stays free) and no new surface word. Lean:
`QCore.UnionArmLayout`. Base: local `main` `e8158562d`. The census and the
frozen [prediction](PREDICTION.md) were committed (`e84ef4a59`) before any
implementation code.

## 1. The defect and the census

`rd_kafka_op_s.rko_op_cb` is stored from `rd_kafka_op_new_cb`'s formal `cb`;
one caller passes `rko->rko_u.leaders.cb` (`rdkafka_partition.c:4136`). Q3
keyed every member of a union together, so that read took every
function-pointer store into any arm and every store into a record the union
holds by value, and the run's set read open through four stores. The census
([prediction](PREDICTION.md) section 2, [`census_sites.py`](scripts/census_sites.py))
classified each by reading:

| Store | Where | Target | In `rko_u`? |
| --- | --- | --- | --- |
| C-style cast | seven `rd_list_init` / `rd_list_new` free callbacks, e.g. `rdkafka.c:5576` `(void *)rd_kafka_partition_leader_destroy` | locals, `rd_kafka_conf_s` lists, the mock's lists, a heap list | no; it opens `rd_list_s.rl_free_cb`, which the union holds by value |
| `memmove` | `rdkafka_sticky_assignor.c:2878`, `:3007`, `:3089` (unit tests) | a local `rd_kafka_group_member_t members[N]` shifted by whole elements | no; `rkgm_eligible` is an `rd_list_t` |
| whole `rd_kafka_fetch_pos_s` | `rdkafka_request.c:1313`, `rdkafka_cgrp.c:4322`, `rdkafka_partition.c:1382` (`rktp->rktp_committed_pos = rd_kafka_fetch_pos_make(...)`), a local at `:2233` | toppar fields, a local | no |
| whole `rd_kafka_replyq_s` | `rdkafka.c:3410`, `:3647`, `:4187`, `:4470` (`rko->rko_replyq = RD_KAFKA_REPLYQ(...)`), three locals | `rd_kafka_op_s.rko_replyq` (outside `rko_u`), locals | no |

**A keying bug, found while implementing.** Clang's desugared type of an
anonymous member is `union rd_kafka_op_s::(unnamed at ...)`, which the keys
read as the record `rd_kafka_op_s`. So `rko_u`'s key was in fact
`rd_kafka_op_s.*`: the union key named the op's whole record, which is why
`rko->rko_replyq` (a field of the op, outside the union) counted as an arm.
The same reading dropped the path of any place below an anonymous structure
(`conf->sasl.oauthbearer.token_refresh_cb` keyed
`rd_kafka_conf_s.token_refresh_cb`, not the normal key the tables give a
declared member). Below a named root an anonymous record is now read under
its path (`anonymous_record`); a pointer-to-anonymous root keeps the old
reading (residual `ANONYMOUS-RECORD-POINTER-ROOT-KEYS`).

## 2. Mechanism and why it is sound

**Arm by discriminant was sized and not taken**
([`arm_sites.py`](scripts/arm_sites.py), [census](artifacts/census.json)):
204 sites can put a function-pointer value into `rko_u`'s bytes; 89 carry
constant local evidence of the op's type, 80 take it from a variable
(`rd_kafka_admin_request_op_new(rk, optype, ...)`) and 35 have none. It needs a
stated arm-to-type map (a surface word) and an interprocedural op-type
reading: residual `RKO-U-ARMS-BY-DISCRIMINANT`.

**Non-overlap by layout (the brief's option 2), taken.** It is weaker than a
discriminant and specific to the corpus's target (x86-64 Linux, Clang 21).

* **Keys.** A place under a union keeps its arm path:
  `rd_kafka_op_s.rko_u.*@leaders.cb` (`ARM`, `union_parts`). Without a layout
  such a key aliases every place of the union, exactly as Q3 did.
* **Layout.** `read_unit` (the pre-pass and data-only units) calls
  `MemberFacts::probe_layouts`: one enumerator probe over the unit's own text
  (`layout_extents::clang_constants`, Clang's `__builtin_offsetof` and
  `sizeof`) for every union its keys name: the range of each arm path, each
  function-pointer member and each record held by value, and, transitively
  through every held record, that record's placements and its
  function-pointer members' ranges (`UnionLayout`). A failed probe, a value
  Clang does not give, an unnamed member on the way and a field whose size is
  not its record's (an array) leave the entry unknown.
* **Aliasing** (`Tables::aliases`). Two places of one union alias when they
  are the same arm path, when either is the whole union, when a range is not
  given, or when the ranges overlap (`places_alias`). A store into a record
  the union holds (`rd_list_s.rl_free_cb`) reaches a place only where one of
  its placements meets it (`meets_held`); an opaque write of a held record
  notes the place only where a placement meets it (`meets_record`). Two units
  that give one place different bytes leave the union unplaced
  (`merge_layout`).
* **Escapes (fail-closed additions).** A `memcpy`-family call into a place the
  union holds, and an address of such a place converted to another pointer
  type (a `memset`-family filler excepted: it writes one byte value), write
  the whole union opaquely. An anonymous arm assigned whole from another type
  or a call writes its own bytes opaquely. None of these was seen before.

**Soundness.** Under rule 12n's premise `P-CG22-MEMBER-CLOSED-WORLD` (a
type-punned store of non-function data, a scalar or a byte pattern, is not
seen; that was already so for every scalar store into an arm), a
function-pointer value reaches a union's bytes only through a
function-pointer store, a store through a held record, an opaque copy, or one
of the escapes; each is placed by Clang's own layout or treated as covering
the whole union. A store over bytes disjoint from a read's never changes what
the read sees (`QCore.UnionArmLayout.a_disjoint_write_never_changes_the_read`,
`a_separated_store_never_changes_the_read`), anything ungiven or disagreeing
aliases (`an_ungiven_range_aliases`, `a_disagreeing_union_aliases_everything`),
and the layout's aliasing implies Q3's sharing, so it never adds a candidate
or a reason (`the_layout_takes_only_shared_stores`). The fold stays
demote-only.

**The layout read** ([census](artifacts/census.json), six librdkafka units
probing `rko_u`, one identical layout): `leaders.cb` at union bytes 168..176;
`offset_commit.cb`, `broker_monitor.cb`, `terminated.cb` at 8;
`admin_result.result_cb` 56; `rtmr_callback` at 56, 112, 704; `rl_free_cb` at
24, 88, 744, 872; `rd_kafka_fetch_pos_s` placed at 0..16, `rd_kafka_replyq_s`
at 136..152 and 808..824. It matches Clang's `-fdump-record-layouts` of
`rdkafka_partition.c` ([`layout.py`](scripts/layout.py)).

## 3. Tests

`crates/qcore-c-gateway/tests/rko_union_layout.rs` (the brief's five cases in
the layout's terms, and the edges):

| Brief's case | Test |
| --- | --- |
| a sibling-arm write on a different discriminant does not open | `a_sibling_arm_on_other_bytes_does_not_join` (a sibling's function pointer, a cast free callback into a held list, both on other bytes) |
| a sibling-arm write without a decided discriminant opens | `a_sibling_arm_on_the_same_bytes_joins_and_a_copy_opens` (same bytes join; a `memcpy` into the union opens) |
| a type-punning cast into the function-pointer arm opens | `a_type_punning_cast_into_the_union_opens` (`*(cb_t **)&w->u.early = fd`, a cast value, an address handed as `void *`; `memset` does not) |
| a store to a separate object of the arm's type does not key into the union | `a_separate_object_of_an_arm_type_does_not_key_into_the_union` (and where a placement meets the read it still opens) |
| an unclassified store opens | `an_unclassified_store_opens` |

Also `the_union_layout_is_read_and_the_read_keeps_its_arm`,
`a_store_below_an_anonymous_member_keeps_its_path` (the keying correction),
`an_unplaced_or_disagreeing_union_stays_shared`, and the module's
`layout_vectors_hold`. The existing `app_rk_callbacks`, `cg22_op_callbacks`
and `callback_member_data_only` suites pass unchanged.

## 4. Measurement

BEFORE `before` (`63548490…`, base code) and AFTER `a1` (`bb13eda5…`, the
card's code at `a1da6a3e6`; later commits only format and add records), each
`run.json` naming its binary, every route with the null property
([summary](artifacts/summary.json), [`measure.sh`](scripts/measure.sh)):

| Route | BEFORE | AFTER | Report sha256 | Wall (s) |
| --- | --- | --- | --- | --- |
| Corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `620fbd75…` → `e93a68bd…` | 673.9 → 652.0 |
| System | 93 / 4 / 351 | 93 / 4 / 351 | `8ecd9c63…` byte-identical | 77.3 → 77.1 |
| Slice | 52 / 1 / 347, 4 Pending | the same | `36c2dd06…` byte-identical | 38.8 → 38.9 |
| Null property | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | every row identical (4,921, 237, 271 rows) | | |

* **The run's sets** (corpus sidecar): `rd_kafka_op_s.rko_op_cb` **closed
  over its 27 candidates** (open before); `rko_serve` closed over 4, as
  before; the rk_conf members unchanged. The fold demoted nothing. The
  pre-pass read 845 of 1,444 units in 3 rounds, none unread.
* **Moving rows** ([compare](artifacts/compare-corpus.json)): 83 Unknown rows,
  no status move. Exactly 131 leaves `call to rd_kafka_op_destroy: the
  callbacks stored in rd_kafka_op_s.rko_op_cb are not a closed set ...
  (callback-member-open)` become `callback rd_kafka_DeleteRecords_leaders_queried_cb
  (from rd_kafka_op_s.rko_op_cb) has no contract in force
  (callback-uncontracted)`: the run's set now closes, and nine of its 27
  candidates have no contract.
* **No Safe gain, no Safe → non-Safe move**, no leak, violation or Unsafe
  added. So there is no Safe row to list with its clauses and premises; the
  rows that would gain rest on the nine undeclared candidates
  (`OP-CALLBACK-CANDIDATE-CONTRACTS`, `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`)
  and, for application callbacks, on `rk_no_plugin_library`.

**Prediction against outcome.** As predicted: `rko_op_cb` closes over the same
27, no status move, null and the other routes unchanged, and the 13 destroy
leaves do **not** decide again (each unit decides over the run's set since
R1, and nine candidates are undeclared); this corrects the brief's
expectation. Not foreseen: the anonymous-record keying bug (section 1), and
the first implementation's whole-object rule firing on scalar stores into
arms and on anonymous `enum` fields, and the escape rule firing on `memset`
into `rd_kafka_msg_s.rkm_u` (each found on the librdkafka census before the
measured build and corrected: [`member_facts_census`](../../../crates/qcore-audit/examples/member_facts_census.rs),
logs `private/rkou/census-after*.log`). Pre-pass wall time did not
measurably grow.

## 5. Lean

`proof/QCore/UnionArmLayout.lean`, audited by `proof/AuditUnionArmLayout.lean`:
11 theorems, standard axioms only, no placeholder, no project axiom
(`a_disjoint_write_never_changes_the_read`, `separated_places_are_disjoint`,
`a_separated_store_never_changes_the_read`, `an_unplaced_place_aliases`,
`an_ungiven_range_aliases`, `the_same_arm_aliases`,
`places_alias_is_symmetric`, `disagreeing_units_unplace`,
`a_disagreeing_union_aliases_everything`,
`the_layout_takes_only_shared_stores`, `layout_vectors_hold` against Rust's
`LAYOUT_VECTORS`). These are the layout analogues of the brief's two
discriminant theorems. Lean audits 321 → **322**.

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 776 → **779**.

* **Retired:** `RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION` (each opening store
  classified by reading, the union's closure closed, measured).
* **Registered:** `RKO-U-ARMS-BY-DISCRIMINANT`,
  `UNION-OPAQUE-RECORD-WRITES-BY-TYPE`, `ANONYMOUS-RECORD-POINTER-ROOT-KEYS`,
  `RL-FREE-CB-CAST-AND-ELEMENT-SHIFT`, each with its DISCHARGE.

## 7. Gate

See [gate.json](gate.json) and [gate/](gate/) (logs `private/rkou/gate-g1/`).
No provider gate: `providers/` is unchanged.

## Files

* `crates/qcore-c-gateway/src/callback_member_closure.rs`;
  `crates/qcore-c-gateway/tests/rko_union_layout.rs`.
* `crates/qcore-audit/src/member_prepass.rs` (`read_facts` public, for the
  census example); `crates/qcore-audit/examples/member_facts_census.rs`.
* `proof/QCore/UnionArmLayout.lean`, `proof/AuditUnionArmLayout.lean`,
  `proof/QCore.lean`.
* `docs/agents/gateway.md` (12n), `docs/agents/README.md`,
  `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.3), `docs/STATUS.md`,
  `docs/development/increments/wave-48-part7.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`.
