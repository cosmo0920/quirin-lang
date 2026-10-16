# UNION-OPAQUE-RECORD-WRITES-BY-TYPE: census and prediction

Frozen before any implementation code. Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE
(wave 48, card-high). Base: local `main` `1c7333db1` (corpus 938 / 9 / 3,887
`1e76a508…`, system 93 / 4 / 351, slice 52 / 1 / 347 / 4 Pending, null
1,113 / 1,435 / 2,355 / 18; 84 / 125 / 245; 71 / 103 / 232; ledger 787; Lean
329). Inputs: the pinned corpus `66910c10`
(`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`, librdkafka
2.15.0). Rule 12n and C04.19.3 amended in place; no new rule id and no new
surface word expected (the keying is internal; no clause changes).

## 1. The defect reproduced, and a second keying defect found

[`member_facts_census`](../../../crates/qcore-audit/examples/member_facts_census.rs)
(base binary, the 94 librdkafka units of the production compile database, all
read; log `private/uorw/census-base.log`) gives the same four reasons for the
two declared members (and `broker_monitor.cb`) as the corpus sidecar
(`docs/reports/handoff-direct-call-reading/artifacts/callback-members-corpus.json`):

| Member | Closure (base) |
| --- | --- |
| `rd_kafka_op_s.rko_u.offset_commit.cb` | open: "a `CStyleCastExpr`", "a `memmove` into it writes a `struct rd_kafka_group_member_s`", "a whole-object assignment writes a `struct rd_kafka_fetch_pos_s`", "a whole-object assignment writes a `struct rd_kafka_replyq_s`" |
| `rd_kafka_op_s.rko_u.terminated.cb` | the same four |

**Not as the ledger entry states.** The entry names only the
`rd_kafka_fetch_pos_s` writes (placed at union bytes 0..16, meeting the two
members at 8). The sidecar also names the cast free callbacks
(`rd_list_s.rl_free_cb`, placed at 24, 88, 744, 872), the unit tests'
`memmove` of `rd_kafka_group_member_s` (reaching `rl_free_cb`) and the
`rd_kafka_replyq_s` writes (placed at 136 and 808), none of which meets byte
8. Reading `Tables::normalise`: a **declared** member spelled into a union
arm (`rd_kafka_op_s.rko_u.offset_commit.cb`, the clause's `TAG.FIELD`) is
normalised to the whole union `rd_kafka_op_s.rko_u.*`, dropping the arm
path, so the layout route (rule 12n, RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION)
never applies to it: it aliases every place and every held record of
`rko_u`. Only a read keyed by `callback_key` (a `Flow::Member`, as
`rko->rko_u.leaders.cb`) kept its arm. Both defects must be repaired for the
declared expected result; the second is the same keying rule (a place under
a union keeps its arm path) applied to the declared spelling.

## 2. Census of every opaque write that opens the two members today

[`opaque_sites.py`](scripts/opaque_sites.py) over all 94 librdkafka units
(Clang's AST of each unit's own command), for every record `rko_u` holds by
value plus the op itself and the record that reaches `rl_free_cb`
(output `private/uorw/opaque-sites.json`); the layout from the units' own
probes ([`union_places.py`](scripts/union_places.py), six units, one
agreeing layout): `offset_commit.cb`, `terminated.cb` and
`broker_monitor.cb` at union bytes **8..16**.

| # | File:line (function) | Write | Target expression | Targets `rko_u`? | Byte range (probe) | Overlap with 8..16 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `rdkafka_request.c:1313` (`rd_kafka_handle_OffsetFetch`) | whole `rd_kafka_fetch_pos_s` = call | `rktp->rktp_committed_pos` (root: pointer to `rd_kafka_toppar_s`) | no: no union holds `rd_kafka_toppar_s` by value | none in `rko_u` (the type is placed at 0..16 by `fetch_start.pos` / `offset_reset.pos`) | none (separate object) |
| 2 | `rdkafka_cgrp.c:4322` (`rd_kafka_cgrp_update_committed_offsets`) | the same | `rktp->rktp_committed_pos` | no | none | none |
| 3 | `rdkafka_partition.c:1382` (`rd_kafka_toppar_offset_commit_result`) | the same | `rktp->rktp_committed_pos` (`rktp` a formal) | no | none | none |
| 4 | `rdkafka_partition.c:2233` (`rd_kafka_toppar_op_serve`) | the same | local `pos` | no | none | none |
| 5 | `rdkafka.c:3410` (`rd_kafka_share_fetch_fanout`), `:3647` (`rd_kafka_share_enqueue_fetch_op`), `:4187` (`rd_kafka_share_enqueue_sync_ack_op`), `:4470` (`rd_kafka_share_commit_sync`) | whole `rd_kafka_replyq_s` = `RD_KAFKA_REPLYQ(...)` | `rko->rko_replyq` (root: pointer to `rd_kafka_op_s`, path outside `rko_u`) | no: `rko_replyq` is the op's own field (op offset 64); no union holds `rd_kafka_op_s` | none in `rko_u` (the type is placed at 136..152, 808..824) | none |
| 6 | `rdkafka.c:4752` (`rd_kafka_seek`), `rdkafka_offset.c:400` (`rd_kafka_commit`), `rdkafka_request.c:3110` (`rd_kafka_MetadataRequest0`) | the same | locals `replyq`, `rq`, `use_replyq` | no | none | none |
| 7 | `rdkafka_sticky_assignor.c:2878`, `:3007`, `:3089` (unit tests) | `memmove` of `rd_kafka_group_member_s` | `&members[i]`, a local array's element (an untargeted pointer to the writer) | no | n/a: reaches `rd_list_s.rl_free_cb` (the record holds `rkgm_eligible` by value), placed at 24, 88, 744, 872 | none |
| 8 | the seven cast free callbacks (RKO report section 1, row 1) | not an opaque write: a `CStyleCastExpr` store into `rd_list_s.rl_free_cb` | lists outside `rko_u` | no | `rl_free_cb` at 24, 88, 744, 872 | none |

No other opaque write of a record `rko_u` holds at bytes meeting 8..16
(`rd_kafka_AdminOptions_s`, `rd_kafka_confval_s`, `rd_kafka_message_s`,
`rd_kafka_msg_s`, `rd_kafka_msgq_s`, `rd_kafka_msgs_head_s`, `rd_list_s`)
and no opaque write of `rd_kafka_op_s` exists in the 94 units; no record
holds `rd_kafka_toppar_s` or `rd_kafka_op_s` by value. **Every opening write
is shown not to overlap the members' bytes**: rows 1–6 by their target, rows
7–8 by the layout once the declared member keeps its arm.

## 3. Mechanism to be built

1. **Declared spelling keeps its arm** (`Tables::walk`): a declared path
   that reaches a union keys as `U.*@rest` (the rest of the path, as
   `callback_key` keys a read), the whole union only when nothing follows.
   An arm path the layout does not give stays unplaced (aliases everything).
2. **Opaque writes keyed by target** (`MemberFacts::walk`): every
   type-keyed opaque write (`opened`) also records its target class
   (new `MemberFacts` field, `#[serde(default)]`): a place inside a union
   (its union key and arm path), a separate object (a local, global or
   by-value formal, or a member path rooted at one), a member path below a
   pointer to a named record, or untargeted (a dereference, an array
   element, a pointer handed to a copier). A write into a union place also
   becomes an open store at that place, keyed with its arm path, so its own
   bytes decide (as the anonymous-arm assignment already does).
3. **Closure** (`closure_prepared`, the held-record path): for a union
   target, a held record's opaque write opens the place only if a placement
   meets it **and** some write of the record may lie in this union: an
   untargeted write, a write below a pointer to a record the union holds by
   value (transitively), a write into another union, or facts without
   targets (older facts: fail-closed, type-keyed as before). A separate
   object, a member path below a record no union holds, and a write into
   this union (decided by its store) do not. The non-union readings (a
   member keyed by its record, containers) stay type-keyed. What stays
   open: an overlapping or unknown-range write into the union, a
   `memcpy`/`memset`-family whole-union write, an address escape (all
   unchanged). The fold stays demote-only.

## 4. Predicted measurement

* **Closures.** `rko_u.offset_commit.cb` and `rko_u.terminated.cb` lose all
  four reasons. Their sets become the stores on bytes 8..16:
  `rd_kafka_decommissioned_broker_thread_join` (`rdkafka_broker.c`), the
  closure of `rd_kafka_broker_monitor_s.rkbmon_cb`, the closure of
  `rd_kafka_conf_s.offset_commit_cb`, and the actuals of
  `rd_kafka_commit0`'s formal `cb` (`rd_kafka_offset_broker_commit_cb`,
  null, and `rd_kafka_commit_queue`'s formal). They close if those chains
  close; any reason they carry is named in the report.
* **Corpus:** 938 / 9 / 3,887, **no status move** (the four rows carry many
  other leaves). The four rows trade their union leaf:
  `rd_kafka_offsets_for_times`, `rd_kafka_query_watermark_offsets`
  (`callback-own-invocation-refused` at `rd_kafka_poll_cb`'s own
  invocation), `rd_kafka_consume0`, `rd_kafka_consumer_close0`
  (`callback-member-open`) each move to their next blocker: most likely
  `callback-uncontracted` naming a candidate of the closed set (none of the
  candidates above has a contract), or an open reason of a subsidiary chain.
  No other row is predicted to move: no other declared member lies in a
  union, and the only union read by a `Flow::Member` (`leaders.cb`, 168..176)
  meets none of the census writes.
* **No Safe gain, no Safe → non-Safe move** predicted; any would be read by
  hand and listed with its clauses, premises and the x86-64 layout reading.
* **System, slice:** byte-identical (no librdkafka member invocation decides
  differently there). **Null property:** rows identical on all three routes.
* **Lean:** `QCore.UnionArmLayout` extended (a non-overlapping opaque write
  never changes the read; an unknown range aliases; a separate object never
  keys into the union); audits 329 → 329 (the module is already audited) or
  330 if a new audit file is needed.
