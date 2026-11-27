# CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT: prediction

Committed before any implementation commit. Base `8e7e8d509` (integrate/r48:
main `0057bac00` plus CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4). USER
DECISION 2026-11-21 (through the integrator's brief): launch this card.

**What was known when this was written.** Round 4's tip census
(`docs/reports/cg10-indirect-access-sole-residuals-r4/census/rank-tip.json`)
names the 13 rows; every caller and callee body was read at Fluent Bit
`046accf6` (section 1). One held-out probe (an uncommitted test target, not
evidence) showed that with the callee declared, `return R(reader, &opts->ns);`
keeps the gate's refusal whether `R`'s slot is declared `resource_out`, a
return-code disposition or even `resource_borrowed`: rule W44 does not read the
address of a pointer-typed member at any position, so the reading has to be
this card's own. The base measurement had not finished.

## 1. The 13 rows and their callees (step a)

| shape | rows | slot actual | callee body (`046accf6`) |
|---|---:|---|---|
| cmetrics `unpack_opts_{ns,ss,name,desc}` | 4 | `&opts->F`, `opts = map->opts`, `map = (struct cmt_map *) context` (a formal) | `cmt_mpack_consume_string_tag` (`cmt_mpack_utils.c:115-175`): `*output_buffer = cfl_sds_create_size(len)`; 0 on success with a non-NULL sds, nonzero with the slot unwritten or written NULL |
| cmetrics `unpack_static_label_component` | 1 | `&label->key`, `&label->val`, `label = (struct cmt_label *) context` | same |
| ctraces `unpack_instrumentation_scope_{name,version}`, `unpack_link_trace_state`, `unpack_span_schema_url`, `unpack_span_status_message` | 5 | `&context->scope_span->instrumentation_scope->name`, `&context->link->trace_state`, `&context->span->status.message`; `struct … *context = ctx` (a formal) | `ctr_mpack_consume_string_or_nil_tag` (`ctr_mpack_utils.c:25-42`, through `ctr_mpack_consume_string_tag`): a string tag gives a fresh sds, a nil tag writes NULL; 0 on success (the slot a fresh sds or NULL), nonzero with the slot unwritten or NULL |
| librdkafka `rd_kafka_msgset_writer_compress_{gzip,lz4,snappy}` | 3 | `&ciov->iov_base`, `ciov` a formal | `rd_kafka_gzip_compress`, `rd_kafka_lz4_compress`, `rd_kafka_snappy_compress_slice`: `RD_KAFKA_RESP_ERR_NO_ERROR` (0) with a fresh `rd_malloc` buffer, any other code with the slot unwritten or NULL |

In all 13 the member lives in an object the caller reaches from one of its own
formals: none holds a frame-owned record, so no field link exists to rewrite.

## 2. Planned changes

* **Contract form (step b): no new grammar.** The callees are declared with
  the existing out slot under C04.13's return-code disposition:
  `resource_on_return(eq, 0, resource_out(slot, nonnull|nullable))` and
  `resource_on_return(ne, 0, resource_borrowed(slot))` (the class leaves the
  slot unwritten), every other formal `resource_borrowed`, result
  `resource_return_none()`. A new consumer section of the contract
  specification states what a member-path actual at an out slot means.
* **Gateway rule 72** (the reading, step c): at the gate's refusal of a
  statement or whole-call return whose callee has an out slot (plain, or a
  disposition whose classes differ only there) handed `&R->F…` with `R` a
  caller-supplied root (an unrebound formal, or a local every write of which is
  such a root, a cast of one or a member read through one): the call is read
  with the slot as data, the slot's chain obligation is kept, and the store is
  decided on the flow as rule 10v's owner premise decides it. An untracked
  owner takes the delivered custody (it escapes to the caller that supplied
  the object; premise P-CG11-CALLER-SLOT for the old value); a tracked,
  unresolved or rooted-binding owner, any other root or slot spelling refuses
  with the gate's text. A body with such a store publishes no summary (rule
  10v's gate), so no caller reads the delivery as `Borrowed`.
* **Declarations (step d)** in their own `.qv` files (cmetrics, ctraces,
  librdkafka), pinned at `66910c10` and `046accf6`.
* Lean `QCore.OwnedOutMemberSlot`, audited.

## 3. Predicted movements (opt-in `current --monkey-linked`, Fluent Bit `046accf6`)

**Unknown -> Safe:** the 13 rows of section 1, range **9-13**. Risks that
would keep some Unknown: rule 12s may forward a disposition through the ten
`return f(…)` callbacks and decide them per class (whose out slot then meets
C04.13's `&NAME` refusal); the lz4 caller's `?:` actual may stay unread; a body
check may withdraw a disposition where the analysed unit defines the callee
(`cmt_mpack_utils.c`, `ctr_mpack_utils.c`, `rdkafka_msgset_writer.c`,
`rdkafka_lz4.c`).

**Other callers** of the five callees (`&value`/`&key_name` locals in
`cmt_mpack_unpack_map`, `unpack_opts_unit`, five ctraces callbacks,
`rdkafka_telemetry.c`) meet a declared disposition where they met an
uncontracted callee: 0-8 further rows may change; re-attributions are
expected, an exposed leak or violation would be reported per row as
provisional, never as an upstream defect. Callers that release the member
first (`cfl_sds_destroy(context->span->name)`, rule 5k) and `new_label->name`
(a tracked owner) stay Unknown.

**Invariants:** no Safe -> non-Safe move; no new Unsafe expected; Pending
unchanged; access-alone 218 -> 205-209.

```measure-prediction
route corpus
move rd_kafka_msgset_writer_compress_gzip Unknown -> Safe
move rd_kafka_msgset_writer_compress_lz4 Unknown -> Safe
move rd_kafka_msgset_writer_compress_snappy Unknown -> Safe
route system
none
route slice
none
```

Pinned routes (`quirin-audit measure --declared`) cannot run on this host (no
`build/cycle19` inputs or baselines). The corpus route analyses
`rdkafka_msgset_writer.c` (its three compressor callers are in the
CG10-INDIRECT-ACCESS-SOLE-BLOCKERS census at `66910c10`, lz4's `?:` there a
literal `1`), not the cmetrics or ctraces decoders; system and slice analyse
none of these units.
