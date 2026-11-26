# RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT: prediction

This file was committed before the implementation commit. The base is
`497c859fc`, the tip of RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (rule
74t, approved as r74). That card sits on rule 74r (r72) and rule 74m (r68),
on main `3ca85c7a6`. USER DECISION 2026-11-24: "launch both cards".

## What was known when this was written

The base was measured, but no implementation was.

* **Reachability.** An end-to-end probe at the base
  (`private/rsm/reach/census-base.log`, not committed) confirms the gap and
  corrects the route the finding card named. The strict `Returned` rule and
  its `Decides` widening both **refuse** `p->n` here, because the root is
  written (`= 0`), which `FunctionTypes::may_bind` treats as bound. A debug
  run (`private/rsm/reach/dbg2.log`) shows that the admitting route is
  **card B01's opaque member read** in `return_expression_instruction`
  (`legacy_return_routes.rs`), which runs before family S. The classifier
  turns the synthetic `__qcore_returned_value = p -> n` into
  `OpaqueValue { alias_read: None }` (C02-P's opaque field read over a root
  `UntrackedBases::never_bound` admits). B01 takes it when the expression has
  exactly one member selection. The node carries no obligation for `p`.
  Three probe rows are `Safe` at the base:
  * `long w(void) { const struct s *p = 0; return p->n; }`;
  * `struct s *w(void) { const struct s *p = 0; return p->next; }`;
  * `void *w(void) { … return (void *)p->next; }`, which `cast_return`
    strips first.

  Every other spelling the brief lists is already `Unknown`
  (`unknown_alias:p`): `p[0]`, `*p`, `((T *)p)->n`, `(p)->n`, `p->a.b`,
  `p->q->n`, `(p->n)` and `(long)p->n`. `&p->n` keeps the return refusal.
* **Census.** [scripts/census.py](scripts/census.py) reads the 181 units
  the base run reports (`private/rsm/run-base/resource-diagnostics.json`).
  It lists every `return R -> F ;`, optionally under one pointer cast, which
  is B01's spelling, and joins each to its row's base status. Its output
  is in `private/rsm/census-route.txt` and is not committed. There are 169
  hits:
  * **135 are on an unwritten formal** (95 on `Safe` rows). An unbound
    formal discharges family S's base obligation, and the base and tip
    probes pin `long w(const struct s *p) { return p->n; }` as `Safe`.
    These cannot move.
  * **34 are on a written formal or a local.** Of these, 26 rows are
    already `Unknown` and cannot move toward `Safe`, and a move inside an
    `Unknown` row changes no status. **8 rows are `Safe`.** These are the
    only candidates:
    * `cmt_encode_opentelemetry.c::get_context_arena`:
      `encoder = (T *) context;`, a cast of a formal;
    * `rdkafka_partition.c::rd_kafka_topic_partition_partition`:
      `const rd_kafka_toppar_t *rktp = (const rd_kafka_toppar_t *)rktpar;`,
      a cast of a formal;
    * `flb_input_chunk.c::flb_input_chunk_get_name`:
      `ch = (struct cio_chunk *) ic->chunk;`, a member of a formal;
    * `rdkafka_msg.c::rd_kafka_message_timestamp`, `…_broker_id` and
      `…_status`: `rkm = rd_kafka_message2msg(…)`, the result of a static
      inline call;
    * `mk_rconf.c::mk_rconf_meta_get`:
      `meta = mk_list_entry(head, …)`, a `container_of` computation inside
      a list loop.

    None of the 8 binds its root to a null constant. The probes show that
    a copy or a member read of a formal is a non-null untracked binding
    that discharges the obligation. So the first three should stay `Safe`.
    The other five bind their root from a call result or from pointer
    arithmetic. If the flow check holds that binding as unknown, it raises
    `unknown_alias` and the row moves `Safe -> Unknown`.

## Planned change

B01's admitted opaque member read is lowered by what it dereferences:

* `ROOT -> FIELD` lowers to family S's read node, `IndirectScalarAccess`,
  with the root as its only base and its C02-P-ALIAS reading;
* a `.` read of a record value keeps the `OpaqueValue`;
* any other spelling declines to the later routes.

The exit stays a `NoOp`. B01 admitted every expression it read, so nothing
admits more than at the base. A base only adds an obligation.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe -> Unknown` (`unknown_alias:ROOT` at the `return`
line), on a row whose bare `return R -> F ;` reaches B01 through a root
the flow check cannot settle. **Expected: 0 to 5 rows**, among
`rd_kafka_message_timestamp`, `rd_kafka_message_broker_id`,
`rd_kafka_message_status`, `mk_rconf_meta_get` and, less likely,
`get_context_arena`, `rd_kafka_topic_partition_partition` and
`flb_input_chunk_get_name` (at most 8). Each would be a refusal of a
root whose non-nullness the analysis does not know, not an observed null.
The report will account for each moved row. A row outside the 8 is an
unpredicted move. No `Unknown -> Safe` move is possible.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The block's `none` is for the pinned routes, which this host cannot run.
The opt-in route's expectation is the paragraph above.

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have. They are recorded as refused and not run.
