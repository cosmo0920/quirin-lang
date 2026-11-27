# CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION: prediction

Committed before the semantic change. Base `110a69c8e`. The card tip before
this file is `a8d3ed6c5` (the base probe and the measurement scripts).

## What was known when this was written

**Base probe.** Commit `a8d3ed6c5`, `tests/call_with_deref_actual.rs`, run at
base. Each of these was Unknown on "local declaration may shadow a formal
resource identity" alone:
- `q_observe(*out);`
- `cg_pair(*out, x);`
- `q_observe(**out);`
- `q_store(*list, item);`
- `undeclared_sink(*out);`

The real declarators (`int (*out);`, `void (*out)(void);`, `T (*out)[4];`,
`T (*out);`) carry the same refusal.

**Base opt-in `current` run** (`private/cwd/run-base2`, release binary of a
`git archive` of `110a69c8e`, Fluent Bit 046accf6, `--monkey-linked`):
- totals: 1,198 Safe, 2 Pending, 3,596 Unknown, 8 Unsafe;
- 27 shadow leaves in 17 rows.

**Census** (`scripts/shadow_census.py`, plus a by-hand reading of each of
those 17 function bodies in the checkout). Thirteen of the 17 rows contain a
statement that opens `IDENT ( *` with a formal behind the `*`. Those 13 rows:
- `unpack_basic_type`
- `cmt_mpack_consume_string_tag`
- `ctr_mpack_consume_binary_tag`
- `ctr_mpack_consume_string_tag`
- `rd_kafka_broker_share_session_add_remove_toppar`
- `rd_kafka_cgrp_consumer_assignment_with_metadata`
- `rd_kafka_ApiVersions_copy`
- `rd_kafka_gzip_compress`
- `rd_kafka_handle_ApiVersion`
- `serialize_Metric`
- `mk_utils_gmt_cache_get`
- `mk_utils_utime2gmt`
- `cio_file_native_lookup_group`, possibly; its statement may be in a macro.

The other rows' shadow leaves come from declarations:
- `rd_kafka_CreateAclsResponse_parse`
- `rd_kafka_DeleteAclsResponse_parse`
- `rd_kafka_timers_run` (`void (*callback)(…);`, a real declarator)
- `rd_map_copy`

**Every one of the 17 rows has other leaves**, so none of them rests on the
shadow leaf alone.

## Planned change

New gateway rule 89. In `classify_simple_base`'s declaration-likeness test,
the `IDENT ( *` alternative is dropped only when the statement's unique Clang
node is a `CallExpr`.

## Predicted

- **No status move on `current`.** Every candidate row keeps other refusals.
  Examples:
  - add_remove_toppar keeps:
    - the `rd_list_new` registry holder;
    - the store through `*toppars_add_list`;
    - the uncontracted `rd_list_remove`;
    - the reference drop it does not hold;
    - the unresolved `toppars_remove_list`.
  - `mk_utils_gmt_cache_get` keeps the unresolved `gcache`.
  - `rd_kafka_ApiVersions_copy` keeps the unmodeled borrowed expression.
- **Leaf changes on about 12-13 rows.** Up to 20 shadow leaves disappear.
  Each admitted call is replaced by its own reading:
  - nothing, for a call the borrowed-actual reading admits;
  - an access refusal;
  - an uncontracted-callee refusal;
  - a contract-specific refusal.

  Rows whose refusal used to poison names may also show different
  downstream leaves.
- **No leaf change** on the four rows whose shadow leaves are declarations.
- **No Safe -> non-Safe move.** An Unknown -> Unsafe move is not expected;
  any one would be a provisional Unsafe that the removed refusal had hidden.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host:
there are no `build/cycle19` inputs or baselines. The expected result there
is no status move.
