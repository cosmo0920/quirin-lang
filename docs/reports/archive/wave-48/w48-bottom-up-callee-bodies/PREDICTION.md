# W48-BOTTOM-UP-CALLEE-BODIES: prediction (frozen before implementation)

Base: local `main` at `c85cb8806`. Written after the BEFORE runs and the
plan below, before any implementation code.

## BEFORE (reproduced)

| Route | Totals | Digest |
|---|---|---|
| corpus | 647 / 9 / 4,178 | `dde4837e…`, the brief's |
| system | 44 / 2 / 188 (merged rows) | normalised `dfc1a9c2…` |
| slice | 29 / 1 / 238 / 2 Pending (merged rows) | normalised `2c1170bc…` |

## The plan (task step 1)

Artifacts in `artifacts/before/` and `artifacts/ranking/*-before.*`.

- **Closure.** 2,539 Unknown corpus functions carry a call blocker; 490
  carry nothing else. Same-unit summary rule 0, a cross-unit route 5 (6 at
  its ceiling); every in-corpus body taken Safe frees 416 to 423 (one round).
  Of the 416 callers in that ceiling, 337 wait on exactly one Unknown body.
- **Per body** (`bottom_up_plan.py`): the largest single-body yields are
  `LZ4_decompress_generic` 12 (116 own leaves), `cJSON_New_Item` 10 (one own
  leaf: `hooks->allocate`, an indirect callback, decision class 1),
  `cmt_mpack_unpack_array` / `ctr_mpack_unpack_array` 5 each (cross-unit),
  `rd_kafka_error_new_v` 5, `sds_cat_safe` 4 (`cfl_sds_cat_safe`, decision
  class 2), `rd_kafka_admin_result_ret_groups` 4 (`_resources`, `_topics` 3).
  Per-body yields are small; 2,053 candidate bodies.
- **Per family** (`family_leverage.py`, cross-unit serving, baseline 6
  included): indirect memory access 587, **return expression 81**, branch
  condition 56, unresolved identities 45, braced initializer 17, assignment
  destination 13, loop condition 12.
- **Return family by reason** (`return_reasons.py`, `leaf_set_leverage.py`):
  75 functions beyond the baseline. "Typed reader refused" frees 29, of
  which 26 are the librdkafka admin accessors returning
  `(const T **) rko->rko_u.admin_result.results.rl_elems`, a pointer read
  through a formal root that the body also hands to a call
  (`rd_list_cnt(&rko->…)`), so rule 5b's `may_bind` premise refuses it.
  "No value form" frees 33, mostly pointer member returns.
- **Held calls** (`held_calls.py`): a cross-unit route that also serves calls
  held inside return and condition leaves frees 10 (calibration: 12 leaves
  still refuse with every held call a Safe same-unit body).

Classification of the top blockers:

| Blocker | Class |
|---|---|
| `hooks->allocate` (cJSON), `rmap_cmp`, mpack callbacks | decision class 1 (callbacks) |
| `cfl_sds_cat_safe` | decision class 2 |
| admin accessors' pointer return after `&rko->…` is passed to a call | existing rule's residual (5b/10k pointer-root premise); lifting it needs a read-only frame for the callee's actual, since a `Borrowed` callee may copy a tracked pointer into the pointee (e.g. `memcpy`) |
| `rd_clock`'s `return` of two members of an address-taken local aggregate | existing rule's residual (W48-RETURN-AGGREGATE-MEMBER-READ); the statement position already admits the same read (W43) |
| `LZ4_decompress_generic` (116 leaves) | several Clang-gap residuals |
| `__builtin_ctz*` returns (`LZ4_NbCommonBytes`, `log2_floor`) | decision class 6 |

No body blocker read so far is a defect candidate.

## What this card attempts

One soundly liftable residual, in rank order of what can be completed:

**Rule 12f, a scalar return value read as a statement value.** A `return E;`
every existing route refused, in a function whose declared result is a
scalar value, where `E` is a scalar value the W43 typed value walker admits
(no call, no `&`, no assignment, increment or comma, no pointer-valued name,
at least one typed access), is lowered as the statement `T tmp = E;` lowers
`E`: one `IndirectScalarAccess` with the walker's bases (their dereference
obligations) and no destination or frame, then an exit that escapes nothing
(C11 6.8.6.4p3: the value is converted as if by assignment to an object of
the result type). A `S.F` selection of an automatic struct carries no
obligation, exactly as in a statement.

Predicted movers:

- **Corpus:** `rd_clock` (librdkafka `rdtime.h`, a header body in W4's
  derivation scope, no reported row) validates; the header helpers it alone
  blocks follow (`rd_interval0`, `rd_interval_reset_to_now`,
  `rd_kafka_buf_set_abs_timeout0`, `rd_kafka_buf_set_timeout`,
  `rd_timeout_init`, `rd_timeout_init_us`, `rd_timeout_remains_us`, then
  `rd_timeout_remains`, `rd_timeout_remains_limit`), and
  **`rdkafka_timer.c::rd_kafka_timer_next` Unknown → Safe** (+1). About 96
  `call to uncontracted function` leaves on those 7 callees are removed from
  other Unknown functions, which stay Unknown. The scalar member returns of
  `rd_kafka_broker_ApiVersion_supported0` (`ret.MaxVer`),
  `rd_kafka_list_groups` / `rd_kafka_offsets_for_times` /
  `rd_kafka_query_watermark_offsets` (`state.err`) and
  `rd_kafka_mock_*_next_request_error` (`err_rtt.err`) lose their return leaf
  and stay Unknown on other leaves. Expected corpus: **648 / 9 / 4,177**,
  between +1 and +3 Safe; no Safe → non-Safe move.
- **System and slice:** no status move predicted (librdkafka is not in
  either route; no return leaf of either is in the candidate set).

Not attempted, registered as the residual plan: the admin pointer-return
cluster (26 functions), `cJSON_New_Item` (10, a decision), `sds_cat_safe`
(4, a decision), `LZ4_decompress_generic` (12, 116 own leaves), the mpack
unpackers (5 + 5, callbacks), and the cross-unit route.

## The carrier

Every body this card lifts is a same-unit or header body that W4 already
derives in each including unit, so the same-unit route carries it; the
cross-unit route is not built here. Measured value of the route alone:
6 functions (10 when held calls are counted), registered with that size.
