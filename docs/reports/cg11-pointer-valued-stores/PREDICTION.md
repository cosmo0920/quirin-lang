# CG11-POINTER-VALUED-STORES: frozen prediction

Frozen before any AFTER run, on base `f19204fa` (local `main` = `origin/main`).
It re-freezes CG00's [prediction](../cg00-clang-gap-baseline/predictions/CG11.md)
for this card, amended where the reasons below say so. CG00's figures were upper
bounds on the older `ae17152b` baseline; the design below is narrower than the
card's full scope, and its figures are upper bounds on it.

## BEFORE

The release binary built at `f19204fa`
(`private/cg11/before.binary.sha256`: `919508e3…`) ran the corpus arbiter into
`private/cg11/before` (exit 0). `resource-diagnostics.json` is `963d7c9f…`,
byte-identical to CG02's archived AFTER; 543 Safe / 10 provisional Unsafe /
4,281 Unknown; `monkey-source.json` `57acfc58…` and
`resource-diagnostics.chunk-contracts.json` `cbbcfd9c…` as CG02 recorded them.

All 37 functions CG00 listed as freed alone are still Unknown on this base
(`scripts/cg00_list.py private/cg11/before`): none has moved since
`ae17152b`.

## The rule this card implements (rule 10v)

A refused statement `P = V` whose place `P` has pointer type, read from the
Clang AST on the typed statement route (W46/CG10), is admitted only when:

- **Destination.** `P` is a CG10 place. Its root is never bound (a caller's
  object) or a never-bound record object of this frame, and at the node both
  the root and the object the member lives in are untracked. A tracked root
  or owner (a local acquisition, or a formal bound to its caller's site while
  a summary is derived) refuses: the member may own its old value.
- **Value.** `V` carries no identity: the null constant, a string literal, a
  function designator, a name of function-pointer type bound to no identity,
  or the address of the destination's own object (the root itself, or `&` of a
  place read through no pointer but the root's).
- **Old value.** No binding of a place rooted at the destination carries a
  target.
- **Summary.** A body with such a store publishes no derived summary.

Everything else keeps the gate's refusal byte for byte. That includes a formal's
object pointer (example C), a local pointer, a pointer read, a call,
arithmetic, another object's address, `*slot = …` stores (the out-slot refusal
is not consulted), and elaborator pieces.

## Predictions

1. **No Safe → non-Safe move.** Every refusal is the old refusal, and the
   summary gate withholds only summaries of bodies that were Unknown before.
2. **Unknown → Safe: at most 5 functions visible to a token estimate, plus a
   few the estimate cannot see.**
   - `scripts/estimate.py` is a static upper bound over BEFORE's refused
     statements. It spells the null constant, a string literal, the root
     itself, or `&` from the same root. It names 5 functions whose every
     leaf is such a store:
     - `rd_kafka_bufq_init`, `rd_kafka_coord_cache_init`,
       `rd_kafka_coord_reqs_init` and `rd_kafka_mock_sharegrps_init`, all
       `TAILQ_INIT` on a formal's member and all on CG00's list;
     - `cio_options_init`, null stores into a record that has a
       function-pointer member, which W45's reader refuses as a whole. It is
       not on CG00's alone-list.
   - Function designators and function-pointer names are invisible to tokens.
     At most 5 more functions are predicted from them.
3. **The other 33 of CG00's 37 stay Unknown**, for these reasons:
   - **A formal's object pointer stored**, the W45-SUB-OBJECT-STORE-FORMAL-VALUE
     shape: `suffix_object`, `otel_span_set_name`, `otel_span_set_trace_state`,
     the four `append_attribute_to_*` functions, `append_metric_to_scope_metrics`,
     `init_snappy_decompressor`, `rd_kafka_confval_disable`,
     `rd_kafka_pattern_add` and `rd_segment_init`.
   - **`cio_set_log_callback`** declares `void (*log_cb)`, which is a `void *`
     formal, not a function pointer.
   - **An owned result stored into a formal's record**, the OWNED-VALUE shape:
     the six `create_*_instance` functions, `ctr_link_set_attributes`,
     `ctr_link_set_trace_state`, `rd_kafka_assignment_init`,
     `rd_kafka_topic_partition_set_metadata_from_rktp_stored`,
     `rd_kafkap_Fetch_reply_tags_set_topic_cnt` and
     `rd_kafka_mock_partition_push_leader_response0`.
   - **A pointer read or arithmetic as the value**: `rd_kafka_timer_unschedule`
     (`TAILQ_REMOVE`), `mk_iov_consume`, `request_set`,
     `writer_set_expected_length` and `rd_kafka_snappy_init_env`.
   - **An out slot or other refusal**: `rd_map_iter_begin` (`*elem = …`),
     `rd_kafka_message_timestamp` (a scalar `*tstype` store, not a pointer
     store: a correction to CG00's attribution), and `decode_pointer_field`
     (a return-position refusal).
4. **No Unknown → Unsafe, and no added leak or violation.** An admitted value
   adds no identity, and the store changes no status, marker, field link or
   containment.
5. **Leaves.**
   - Unsupported store refusals fall by the admitted statements.
   - `unknown_alias` does not rise, because an admitted store's owner is
     untracked, so its chain resolves.
   - None of `CG10-BOUND-UNTRACKED-ROOTS`' leaves is removed, since a bound
     untracked root is an unresolved owner and refuses.
6. **No cascade.** Callers of newly Safe functions stay as they were, because
   no summary is published.
7. **System slice and tail-pause slice:** 0 status moves.
8. **Sidecars byte-identical** (`monkey-source.json`,
   `…chunk-contracts.json`, `…contracts.json`, `…source-effects.json`).
9. **Tests.**
   - CG00's M04 control `s->cb = f` flips to Safe.
   - The M02 and M03 controls keep their refusals:
     - formal and local values;
     - the tracked owner;
     - `slots[i] = p`;
     - `*slot = p`.
   - The Q36 fixture (`access_path_statements.rs`,
     `pointer_store_and_union_still_refuse`) keeps its refusal.
   - Example C stays Unknown. Its missing premise is an owner identity for
     `*s`, from a contract or tracked state, that bounds `*q`'s lifetime by
     its owner, or a derived Stored summary mode.
10. **Ledger.**
    - `CG11-POINTER-VALUED-STORES` is amended, not retired: example C, the
      formal-value and owned-value stores and the named-value stores remain.
    - The absorbed follow-ups are re-measured and amended. None closes in
      full.
