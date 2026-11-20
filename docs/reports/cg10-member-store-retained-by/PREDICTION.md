# CG10-MEMBER-STORE-RETAINED-BY: prediction

Committed before any implementation commit. Base `0111ddf38`
(CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3's card tip, which merged main
`b7f86a0de`). USER DECISION 2026-11-20 on round 3's decision class 1,
question 2, verbatim: "Read it as retention, option 1 with creating the
relevant proof(s)."

## 1. Census (method step 1)

Source: round 3's tip census (`census-tip2/premises.jsonl.gz`, the opt-in
`quirin-audit current --monkey-linked` over Fluent Bit 046accf6, binary
`cdcbf730…` of `1379d031a`; the later round-3 commits up to `0111ddf38` are a
lint fix, fixture updates and the provider-only main merge), read by
[scripts/pointer_name_rows.py](scripts/pointer_name_rows.py), which imports
round 3's own classifier (`refine.py`'s `shape`). Rows:
[census/pointer-name-rows.jsonl](census/pointer-name-rows.jsonl). The 19
sole-blocker functions holding a `pointer-name-into-member` leaf (13 of them
blocked by it alone):

| function | alone | store(s) | holder | stored value | other blockers |
|---|---|---|---|---|---|
| `chunkio.c::cio_set_log_callback` | yes | `ctx->options.log_cb = log_cb` | member path of formal `ctx` | formal, `void *` (declared `void (*log_cb)`, a parenthesised object pointer) | none |
| `cJSON.c::suffix_object` | yes | `prev->next = item; item->prev = prev` | members of formals `prev`, `item` | formals | none |
| `rdkafka_buf.c::rd_kafka_buf_set_maker` | yes | `rkbuf->rkbuf_make_req_cb = make_cb; rkbuf->rkbuf_make_opaque = make_opaque` | members of formal `rkbuf` | formals: `make_cb` is `rd_kafka_make_req_cb_t *` (pointer to a function typedef), `make_opaque` is `void *`; beside them `free_make_opaque_cb`, a `R (*)(A)` formal 10v already admits | none |
| `rdkafka_mock.c::rd_kafka_mock_partition_set_leader0` | yes | `mpart->leader = mrkb` | member of formal `mpart` | formal | none |
| `cmt_decode_msgpack.c::create_{counter,gauge,histogram,exp_histogram,summary,untyped}_instance` (6) | yes | `map->parent = (void *) counter` (etc.) | member of formal `map` | **a local holding the body's own `calloc` result**: custody moves into the caller's object, not a retention | the local's escape (MEMBER-EFFECT-STORES' shape) |
| `rdkafka_cgrp.c::PartitionMemberInfo_new` | yes | `pmi->member = member` | member of **local `pmi`, the body's `rd_calloc` result, returned** | formal | the holder is the result (C04.14.2's `result` holder), a tracked owner |
| `rdkafka_op.c::rd_kafka_op_new_cb` | yes | `rko->rko_op_cb = cb; rko->rko_rk = rk` | member of **local `rko`, `rd_kafka_op_new`'s result, returned** | formals (`cb` a function pointer) | the holder is the result, a tracked owner |
| `rdkafka_request.c::rd_kafka_ListOffsetRequest_parameters_make` | yes | `params.rktpars = rktpars; params.errstr = errstr` | member of **a local record returned by value** | formals | an aggregate result holder (no C04 form) |
| `pb_encode.c::pb_ostream_from_buffer` | no | `stream.state = buf` | member of a local record returned by value | formal | `stream.callback = &buf_write` (10v other-store) |
| `rdkafka_interceptor.c::rd_kafka_interceptor_method_new` | no | `method->ic_opaque = ic_opaque` | member of a tracked local | formal | owned result into a local member; a union pointer store |
| `rdkafka_range_assignor.c::rd_kafka_member_assigned_partitions_pair_new` | no | `pair->member_id = member_id` | member of a tracked local | formal | owned result into a local member |
| `rdkafka_msg.c::rd_kafka_msgq_split` | no | `rightq->rkmq_msgs.tqh_first = first_right` | member path of formal `rightq` | formal | four 10v other-stores (`&…`, member reads), a cast |
| `rdkafka_telemetry_encode.c::set_attributes` | no | `attributes[attr_idx].value = value` | element of a formal array | a local holding a call result | an indirect call; another member read store |
| `mk_clock.c::mk_clock_headers_preset` | no | `server->clock_context->headers_preset.data = buffer` | a member two pointer hops below formal `server` | a local holding a call result | a sub-object address actual |

## 2. The reading (rule 70) as planned

A store `F->m[.n…] = q` read on rule 10v's typed statement route, where `F`
and `q` are formals of the function that the body never rebinds (no
assignment, increment or address-of), `F->m[.n…]` reads no pointer but `F`
(the member lies in `*F`), `F` and the member's object are untracked at the
node (rule 10v's owner premise and premise P-CG11-CALLER-SLOT), and `q` is
bound to no identity on any reaching path, is admitted as
`retained_by(q, F.m[.n…])`: the body takes no custody of `q`, the store moves
nothing, and the body publishes a **retention summary**, the existing
C04.14.3 path retention (`RetainingContract::path_retentions`, the form a
declared `resource_retained_by(P, FORMAL.FIELD)` takes), installed for the
unit's callers through `insert_retaining` and decided at each call by rule
12t's existing route-G table. The summary is published only when the
ordinary W4 validation of the body (with rule 10v's pointer-store gate
relaxed for exactly these stores) gives every formal `Borrowed` and a `None`
result; the body's other pointer stores (10v's neutral values into the same
kind of place) are stated as **overwritten paths** of the same summary. A
function pointer (a value 10v classifies as function-pointer typed) carries
no lifetime: it adds an overwritten path and no retention (the subclass). At
a call, every overwritten or retaining path must resolve to rule 12t's path
holder (the caller's own unrebound, untracked formal named only through
member accesses, premise P-HOLDER-PATH-STABLE); otherwise the call refuses as
`retention-undecided`, whatever the retained actual is. A local's pointer
stays refused: no census row stores a local that is a borrowed alias (each is
an owned allocation, a custody move, or a call result).

## 3. Predicted movements (opt-in route)

**Rows that move, Unknown -> Safe (4):** `cio_set_log_callback`,
`suffix_object`, `rd_kafka_buf_set_maker` (`make_cb`'s spelling `T *` for a
function typedef `T` is not read as a function pointer by 10v's classifier,
so it is read as an object pointer and retained: more obligation, not less),
`rd_kafka_mock_partition_set_leader0`. Range: 3–4 (the W4 validation of the
summary does not gate the body's own row, so a summary refusal would not stop
a move; a body-level surprise could).

**Callers that newly carry an obligation (each stays Unknown):**

* `cio_create` (chunkio.c:148, `cio_set_log_callback(ctx, options->log_cb)`):
  the holder root `ctx` is the body's own allocation, not a path holder, and
  the retained actual is a member read: `retention-undecided`.
* `add_item_to_array` (`suffix_object(child->prev, item)`) and the four
  `cJSON_Create*Array` (`suffix_object(p, n)`): holder roots a member read and
  tracked locals: `retention-undecided`.
* `rd_kafka_mock_partition_assign_replicas` (`…set_leader0(mpart, NULL)`
  decides: nothing retained, `mpart` the caller's formal; `…set_leader0(mpart,
  mpart->replicas[…])` refuses), and the mock command handler
  (`set_leader0(mpart, mrkb)`, `mpart` a lookup result): stay Unknown.
* `rd_kafka_buf_set_maker`'s caller is in `rdkafka_request.c`, another unit:
  no derived summary crosses units, so the call stays uncontracted.

Each of these callers' leaf for the call changes from `call to uncontracted
function` (or the access refusal of its actual) to the route-G refusal; none
moves. **No caller can move Safe -> non-Safe**: before this card each called
an uncontracted function (a body with a pointer store published no summary),
which is never `Safe`. A caller elsewhere in the corpus whose call to a newly
summarised function passes only null constants, string literals or function
designators (for function-pointer formals) into a path holder of its own
formal could move Unknown -> Safe; none is known from the census (0–2).

**Rows that do not move, and why:** the six cmetrics constructors (the stored
local carries the body's allocation: a custody move into the caller's object,
MEMBER-EFFECT-STORES, not a retention); `PartitionMemberInfo_new` and
`rd_kafka_op_new_cb` (the holder is the body's tracked result: a
`retained_by(p, result)` derivation, which needs the tracked-owner field
semantics rule 10v keeps refused; follow-up); `ListOffsetRequest_parameters_make`
and `pb_ostream_from_buffer` (a record returned by value is no C04 holder);
`rd_kafka_interceptor_method_new`, `rd_kafka_member_assigned_partitions_pair_new`
(tracked local holder, and an owned result store); `rd_kafka_msgq_split`,
`set_attributes`, `mk_clock_headers_preset` (other blockers; the last two
store a call result through a deeper place). Rows outside the sole set that
hold such a store lose that leaf and stay Unknown.

**Invariants:** no Safe -> non-Safe move, no new Unsafe, no Pending moved.
Access-refusal leaves fall by the stores read (about 5 in the four moved
rows, plus leaves on non-sole rows: 0–60).

```measure-prediction
route corpus
move cio_set_log_callback Unknown -> Safe
move suffix_object Unknown -> Safe
move rd_kafka_buf_set_maker Unknown -> Safe
move rd_kafka_mock_partition_set_leader0 Unknown -> Safe
route system
none
route slice
none
```

Corpus and twelve-home: the four rows where the route analyses `chunkio.c`,
`cJSON.c`, `rdkafka_buf.c` and `rdkafka_mock.c` (`quirin-audit measure
--declared` cannot run on this host: no prepared pinned-route inputs or
baselines; to be confirmed by the integrator); system and slice: none.
