# CALLBACK-MEMBER-NON-C-UNITS prediction

Committed before any analyzer, Lean or test change. **Base side:
`baseline:c8c7edf2165a2065c234f3a63b4eed23e937ced6`**, the newest published
baseline in the main checkout's `private/baselines/` at launch (a republish
at `7b726587b` was in progress). The card's base is `7b726587b`.

**What changes.** Only rule 12n's member-facts pre-pass. Today an unread
unit (a unit naming a token of any invoked member's closure graph whose facts
were not read) opens **every** member of the run. After the card, a C++ unit
whose preprocessed text (its own recorded command, `-E`) is the whole unit
opens only the members whose own closure-graph tokens it names, which is the
reading the pre-pass already uses to skip a C unit that names no token
(`QCore.AppRkCallbacks.skipping_a_silent_unit_keeps_the_run_set`), applied
per member. A unit whose preprocessing failed, an assembly unit, a C unit
whose AST was not read, and a C++ unit whose command reads a precompiled
header or module (or whose text holds a module declaration or import) still
open every member. Nothing is promoted by the fold; a member closes only when
every unit's reading of it is closed.

**The two units.** In the user's `current` run at Fluent Bit `046accf6` the
pre-pass leaves exactly two units unread, both C++:
`lib/ripser-1.2.1/ripser.cpp` names only `event` (libstdc++'s
`std::ios_base::event`), and `src/simdutf/flb_simdutf_connector.cpp` names
`cmt`, `event` and `evl` (its includes `fluent-bit/flb_log.h` and
`flb_mem.h` reach cmetrics' `struct cmt` and Fluent Bit's `struct
mk_event_loop *evl` fields). `event`, `evl` and `flb_input_collector` are the
S3 lifetime protocol's tokens (`input_collector_registration.qv`:
`quant registered #coll: flb_input_collector.event |->
flb_input_collector.evl;`), not tokens of any callback member's graph; `cmt`
is `cmt.log_cb`'s record tag.

**The user's `current` run, predicted** (the integrator reruns it):

- `callback_members.members`: `rd_kafka_s.rk_conf.*` (the 11 rk_conf members),
  `rd_kafka_s.rk_share_consumer.share_acknowledgement_commit_cb` and
  `consume_ctx.consume_cb` close empty; `rd_kafka_op_s.rko_serve` closes over
  {`rd_kafka_cgrp_op_serve`, `rd_kafka_mock_cluster_op_serve`,
  `rd_kafka_poll_cb`, `rd_kafka_toppar_op_serve`} (each equal to the
  baseline's `members_over_recorded_units`). `cmt.log_cb` stays open on one
  unit (`flb_simdutf_connector.cpp`, naming `cmt`); `cio_options.log_cb`
  stays open ("a formal that is not a function pointer");
  `rd_kafka_op_s.rko_op_cb` and its two `rko_u` members stay open (a
  `CStyleCastExpr`, a `memmove` writing a `struct rd_kafka_group_member_s`).
- Refusal rows: the 205 `callback-member-open` rows that named "2 unit(s) of
  the run may name it but were not read" keep refusing, with these reasons:
  131 `rd_kafka_op_destroy` rows on `rko_op_cb` and the 23 `cio_log_print`
  rows now name their member's own C reason; the 43 `cmt_log_print` rows name
  "1 unit(s) ... flb_simdutf_connector.cpp"; the 8 rows that named
  `rk_conf.rebalance_cb`, `consume_ctx.consume_cb` or `rko_serve`
  (`rd_kafka_consume0`, `rd_kafka_consumer_close0`, `rd_kafka_offsets_for_times`,
  `rd_kafka_query_watermark_offsets`, `rd_kafka_consume_callback0`,
  `rd_kafka_poll_cb`, `rd_kafka_op_req`, `rd_kafka_op_req2`) now refuse on
  `rd_kafka_op_s.rko_op_cb`, which the same declarations also invoke
  (`rd_kafka_poll_cb`, `rd_kafka_q_serve`, `rd_kafka_op_req0`).
- No function changes status (every affected function either keeps another
  invoked member open or has other blockers: the seven `cmt_*_create` /
  `filter_context_label_key_value` rows hold unresolved identities and
  uncontracted calls), no callee leaves the uncontracted list, no function
  leaves the "only uncontracted" set, no unit status changes.

**Measured routes.** The corpus, system and slice routes' pre-passes leave 36
units unread at the baseline: the same 2 C++ units and 34 units whose
preprocessing failed (sources and headers the prepared builds never
generated: `lj_vm.S`, `lj_libdef.h` users, the flex/bison outputs). A failed
preprocessing still opens every member, so no closure closes and no row moves
on any route; the refusal leaves' unit count changes from "36 unit(s)" to
"34 unit(s)" (text only, first unit unchanged: `lj_vm.S`).

```measure-prediction
route corpus
none
route system
none
route slice
none
```
