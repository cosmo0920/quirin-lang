# RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: census and prediction

Frozen before any implementation code. Base: local `main` at `e8158562d`.
Inputs: the pinned corpus `66910c10`
(`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`, librdkafka 2.15.0).

## 1. The defect reproduced

[`member_facts_census`](../../../crates/qcore-audit/examples/member_facts_census.rs)
reads every librdkafka unit of the production compile database exactly as the
pre-pass reads it (`member_prepass::read_facts`; 94 of 106 read, the 12
unread are the `src-cpp` C++ wrapper, which the scope file puts outside the
program). Over those units (log `private/rkou/census-base.log`):

| Member | Closure |
| --- | --- |
| `rd_kafka_op_s.rko_op_cb` | open: the four reasons below |
| `rd_kafka_op_s.rko_u.*` | open: the same four |
| `rd_list_s.rl_free_cb` | open: the cast and the `memmove` |
| `rd_kafka_timer_s.rtmr_callback` | closed over 40 timer callbacks |

The four reasons, as the corpus sidecar names them: "a `CStyleCastExpr`",
"a `memmove` into it writes a `struct rd_kafka_group_member_s`", "a
whole-object assignment writes a `struct rd_kafka_fetch_pos_s`", "a
whole-object assignment writes a `struct rd_kafka_replyq_s`".

**How they reach `rko_op_cb`.** `rko_op_cb` is stored from the formal `cb` of
`rd_kafka_op_new_cb`; one caller passes `rko->rko_u.leaders.cb`
(`rdkafka_partition.c:4136`), a read keyed `rd_kafka_op_s.rko_u.*` (Q3: the
union's members share one key). That key takes every function-pointer store
into any arm, and, because a union key aliases every record the union holds
by value, every store into `rd_list_s.rl_free_cb` (the union holds four
`rd_list_t`s), `rd_kafka_timer_s.rtmr_callback` (three timers) and every
opaque write of a record it holds (`rd_kafka_fetch_pos_s` twice,
`rd_kafka_replyq_s` twice, `rd_kafka_msg_s`, ...).

## 2. Census of the four opening stores

Read by hand ([`census_sites.py`](scripts/census_sites.py), Clang's AST of each
unit's own command; [`context.py`](scripts/context.py) prints the lines).

| # | Store | File:line (function) | Target expression | In `rko_u`? | How it reaches the union key |
| --- | --- | --- | --- | --- | --- |
| 1 | cast | `rdkafka.c:5576` (`rd_kafka_query_watermark_offsets`), `:5756` (`rd_kafka_offsets_for_times`): `(void *)rd_kafka_partition_leader_destroy`; `rdkafka_interceptor.c:235`, `rdkafka_sasl_oauthbearer.c:898`, `:1308`: `(void (*)(void *))rd_strtup_destroy`; `rdkafka_mock_sharegrp.c:579`: `(void *)rd_kafka_topic_partition_list_destroy`; `rdkafka_plugin.c:151`: `(void *)&rd_kafka_plugin_destroy`; `rdkafka_fetcher.c:3455`: `(void *)rd_kafka_toppar_ver_destroy` (through `rd_list_new`) | the free-callback actual of `rd_list_init` / `rd_list_new`, stored in `rdlist.c` by `rl->rl_free_cb = free_cb` | no: the lists are locals (`leaders`, `parsed.extensions`), `rd_kafka_conf_s` fields (`interceptors.config`, `plugins`), the mock's `target_assignment->assignment`, `handle->extensions` and a heap list in `rkbuf->rkbuf_rktp_vers` | `rd_list_s.rl_free_cb` (a C-style cast of a function designator to `void *` is an open flow); the union key aliases `rd_list_s` |
| 2 | `memmove` | `rdkafka_sticky_assignor.c:2878` (`ut_testReassignmentAfterOneConsumerLeaves`), `:3007` (`ut_testSameSubscriptions`), `:3089` (`ut_testLargeAssignmentWithMultipleConsumersLeaving`) | `&members[i]`, a local `rd_kafka_group_member_t members[N]` of a unit test, shifted down by whole elements | no | `rd_kafka_group_member_s` holds `rd_list_t rkgm_eligible` by value, so an opaque write of it opens `rd_list_s.rl_free_cb`, which the union key reads through `rd_list_init`'s formal (`src->rl_free_cb` actuals) |
| 3 | whole-object `rd_kafka_fetch_pos_s` | `rdkafka_request.c:1313` (`rd_kafka_handle_OffsetFetch`), `rdkafka_cgrp.c:4322` (`rd_kafka_cgrp_update_committed_offsets`), `rdkafka_partition.c:1382` (`rd_kafka_toppar_offset_commit_result`): `rktp->rktp_committed_pos = rd_kafka_fetch_pos_make(...)`; `rdkafka_partition.c:2233` (`rd_kafka_toppar_op_serve`): local `pos = ...` | toppar fields and a local | no | keyed by type: the union holds `rd_kafka_fetch_pos_t` by value (`offset_reset.pos`, `fetch_start.pos`) |
| 4 | whole-object `rd_kafka_replyq_s` | `rdkafka.c:3410`, `:3647`, `:4187`, `:4470`: `rko->rko_replyq = RD_KAFKA_REPLYQ(...)` (a call of `rd_kafka_replyq_make`); `rdkafka.c:4752`, `rdkafka_offset.c:400`, `rdkafka_request.c:3110`: locals | `rd_kafka_op_s.rko_replyq`, outside `rko_u` (offset 64), and locals | no | keyed by type: the union holds `rd_kafka_replyq_t` by value (`admin_request.replyq`, `leaders.replyq`) |

Types: `rd_kafka_fetch_pos_s` is `{int64_t offset; int32_t leader_epoch;
rd_bool_t validated}`, `rd_kafka_replyq_s` is `{rd_kafka_q_t *q; int32_t
version}` (no `ENABLE_DEVEL`); neither holds a function pointer.
`rd_kafka_group_member_s` holds one through `rkgm_eligible.rl_free_cb`.

**None of the four stores targets `rko_u`.** Each reaches it only because the
union key is shared and aliases the records the union holds, by type.

## 3. Which evidence separates the arms

**Arm by discriminant (preferred) does not fit this card.** Sizing
([`arm_sites.py`](scripts/arm_sites.py), [`arm_sites_summary.py`](scripts/arm_sites_summary.py),
22 units naming `rko_u.`): 204 sites can put a function-pointer value into
`rko_u`'s bytes (a function-pointer member store, an address of a `rd_list_t`
or timer in an arm, a whole-arm write). Local type evidence (an enclosing
`switch` on `rko_type`, a constructor with a constant type in the same
function, an `RD_KAFKA_OP_TYPE_ASSERT`, read as any earlier assertion in the function) covers 89; 80 take the type from a
variable (`rd_kafka_admin_request_op_new(rk, optype, ...)`), and 35 have none.
The leaders read itself (`rdkafka_partition.c:4136`) is reached through a
formal and rests on `RD_KAFKA_OP_TYPE_ASSERT`. A discriminant route would need
an interprocedural op-type analysis over those sites and a stated arm map,
which is a new surface word. It is registered as a residual, not taken.

**Non-overlap by layout (option 2) closes it with no new surface word.**
Clang's record layout of `struct rd_kafka_op_s`
([`layout.py`](scripts/layout.py), `rdkafka_partition.c`'s own command,
x86-64 Linux, `sizeof = 1024`): `rko_u` starts at byte 112. Offsets within the
union:

| Function-pointer position | Union offset |
| --- | --- |
| `leaders.cb` | **168..176** |
| `offset_commit.cb`, `broker_monitor.cb`, `terminated.cb` | 8 |
| `admin_result.args.rl_free_cb`, `admin_result.result_cb`, `admin_result.results.rl_free_cb` | 24, 56, 88 |
| `leaders.query_tmr.rtmr_callback`, `leaders.timeout_tmr.rtmr_callback` | 56, 112 |
| `admin_request.tmr.rtmr_callback`, `admin_request.args.rl_free_cb`, `admin_request.fanout.results.rl_free_cb` | 704, 744, 872 |

| Record held by value | Placements (offset, size) |
| --- | --- |
| `rd_kafka_fetch_pos_s` | (0, 16) twice |
| `rd_kafka_replyq_s` | (136, 16) in `leaders`, (808, 16) in `admin_request` |

No other function-pointer position, and no placement of the two opened
records, meets bytes 168..176. This is weaker than a discriminant and
arch-specific: it is Clang's layout for the corpus's target, read per unit.

## 4. The mechanism (to be implemented)

* **Keys.** A function-pointer member read or stored through a union keeps its
  arm path beside the union key: `rd_kafka_op_s.rko_u.*@leaders.cb`.
* **Layout.** The pre-pass's `read_unit` probes, for each union its keys name,
  Clang's `__builtin_offsetof` / `sizeof` of every arm path, every
  function-pointer member and every record held by value (transitively) in one
  enumerator probe over the unit's own preprocessed text
  (`layout_extents::clang_constants`). A probe that fails, an anonymous member
  without a name, an array of records or two units that disagree leave the
  union **unplaced**.
* **Aliasing.** Within one union: the same arm path aliases; an unplaced side
  aliases everything (today's sharing); otherwise only overlapping byte
  ranges alias. A store into a record the union holds (`rd_list_s.rl_free_cb`)
  aliases a union member only where one of its placements overlaps it (or is
  unknown); an opaque write of a held record opens it only where a placement
  overlaps (or is unknown). An opaque write of the union's own record
  (`rd_kafka_op_s`) still opens every member.
* **Escapes (new, fail-closed).** A `memcpy`-family call into a place inside
  a union, and an address of such a place converted to another pointer type,
  open the whole union (unplaced). Today neither is seen.
* Every route without the pre-pass keeps unplaced keys: unchanged.

## 5. Prediction

* **`rko_op_cb`'s run set closes again**, over the same 27 candidates as
  before Q3 (`rd_kafka_DeleteRecords_leaders_queried_cb`, ...,
  `rd_kafka_txn_op_send_offsets_to_transaction`), unless an escape site of
  section 4 meets `leaders.cb` (none is expected: no cast of an address into
  `rko_u` in librdkafka's text). `rko_u.*@leaders.cb` closes over the two
  `query_leaders_async` callers' actuals.
* **The 13 destroy leaves do not decide again.** Since R1 every unit decides
  `rd_kafka_op_destroy`'s member invocation over the run's set, and 9 of the
  27 candidates have no contract (`rd_kafka_DeleteRecords_leaders_queried_cb`,
  `rd_kafka_ListOffsets_leaders_queried_cb`, `rd_kafka_admin_fanout_worker`,
  `rd_kafka_admin_worker`, the three share fan-out ops,
  `..._query_leaders_async_worker_op_cb`,
  `rd_kafka_txn_op_send_offsets_to_transaction`). So the 131
  `callback-member-open` leaves of `rd_kafka_op_destroy` become
  `callback-uncontracted` leaves naming them. This corrects the brief's
  expectation: the leaves decided before only on in-unit sets.
* **No status move, no Safe gain, no Safe → non-Safe move.** Corpus
  936 / 9 / 3,889, system 93 / 4 / 351, slice 52 / 1 / 347 / 4 Pending, null
  tallies unchanged. Only Unknown rows calling `rd_kafka_op_destroy` (and
  callers of other `rko_op_cb` invokers) trade leaves.
* `rd_kafka_op_s.rko_serve` and the rk_conf members are unchanged.
* Pre-pass wall time grows by one Clang probe per unit whose keys name a
  union (a few seconds).

## 6. Residuals to register

* Arm by discriminant for `rko_u` (the 115 sites without constant local
  evidence, and the arm-map surface word).
* Opaque record writes still keyed by type, not target: they open a union
  member wherever a placement of the record overlaps it (for example
  `offset_commit.cb` at union offset 8 against `fetch_pos` at 0..16).
* A C-style cast of a function designator is an open flow
  (`rd_list_s.rl_free_cb` stays open).
