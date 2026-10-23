# Prediction: rule 35 (literal reads and decayed scalar array members)

Card W48-BU-SINGLE-LEAF-BODIES, written 2026-10-23. Later results are compared
against this file and never edit it.

## Chronology (stated plainly)

This prediction was **not** committed before the first semantic change:

1. `0bf8f9f47`: the measurement driver and link map (byte copies), no semantics.
2. `b41a36512`: the rule-35 implementation, Rust and Lean, committed as WIP at
   the integrator's request after a session restart. The prediction did not
   exist yet: it was waiting on a fresh BASE corpus run, which never acquired
   the shared lock (two queued attempts, both retained as failed and not
   evidence).
3. `5e2d3c368`: the rule-35 text in `docs/agents/gateway.md` and its router row.
4. `81911ed3f`: merge of the local integration main `0217006f2` (rules 32 and 34
   and AUDIT-IR-RETAINED-PRODUCTS).
5. This file, committed after all of the above and before any corpus, system or
   slice measurement of BASE `4dd8c8013` or of the TIP.

Before writing it, the implementation had been run only on synthetic fixtures
(`tests/literal_decay_readings.rs` and a development probe). No pinned-corpus,
system or slice run of the rule existed. The brief asks for the planner to be
re-run on fresh BASE diagnostics first. That was not possible before this
commit, so the orientation figures below come from the retained batch5 TIP
corpus diagnostics (`w48-batch5/analysis-inputs/corpus/tip`, an older main).
The fresh BASE planner run is reported in the README beside this prediction.

## Planned semantics (as implemented in `b41a36512`)

- **Arm 1.** A `*` token directly followed by a literal token, with the literal
  run not stored into or incremented, is not an access to the lexical gate
  (`contains_unmodeled_access`). This covers `rd_assert(!*"...")`,
  `RD_NOTREACHED()` and `RD_BUG(...)`.
- **Arm 2.** At a W44 typed call actual, an `ArrayToPointerDecay` of a member
  place of a one-dimensional array of builtin scalars, unqualified, is read as
  rule 14c's `&PATH[0]`, with every root premise unchanged.

## Predicted measurements (pinned corpus, BASE `4dd8c8013` against the TIP)

The TIP also carries rules 32 and 34. Their moves are sibling effects,
attributed to them and not predicted here.

For the twelve brief bodies, the leaf named in the brief hides a callee or a
custody question in ten of them. Fixtures show the access reading itself is
already admitted (or is admitted by arm 2), so the refusal moves to, or stays
at, that other question:

| Body | Predicted |
|---|---|
| `rd_kafka_mock_cluster_io_set_event` | **Unknown → Safe** (arm 1) |
| `XXH32_digest_endian`, `XXH64_digest_endian` | Unknown. Arm 2 admits `state->mem32`/`mem64`, and the leaf moves to the callee `XXH32_finalize`/`XXH64_finalize`, whose own rows are Unknown |
| `rd_kafka_queue_get_sasl` | Unknown, unchanged. `rd_kafka_queue_new0` is a retaining declaration (rule 12t, withheld from the function map). The mixed place `rk->rk_sasl.callback_q` is already read |
| `rd_kafka_broker_ApiVersion_at_least` | Unknown, unchanged. `return f(...) != -1` is already admitted. `rd_kafka_broker_ApiVersion_supported0` is Unknown and uncontracted |
| `thrd_setname` | Unknown, unchanged. The nested-call condition is already admitted, but `pthread_setname_np` has no libc contract |
| `rd_kafka_MemberDescription_copy`, `rd_kafka_group_result_copy` | Unknown, unchanged. A `?:` call actual (two-hop arm or nested owned call) is not lowered, and both callees are Unknown |
| `rd_kafka_topic_partition_list_grow` | Unknown, unchanged. A reallocated result is stored into a member |
| `rd_map_init` | Unknown, unchanged. An aggregate call result with an owned member is stored into a member, and the callee is Unknown |
| `mk_iov_free_marked` | Unknown, unchanged. It releases a caller's array element (custody position, by design) |
| `rd_kafka_topic_partition_list_cmp` | Unknown, unchanged. It is an indirect call through a function-pointer formal (rule 11p's open set) with record-element addresses |

Elsewhere in the corpus:

- **Arm 1.** The orientation census (`scripts/literal_candidates.py`) finds 60
  Unknown rows with a gate leaf on a literal-dereference line (71 leaves). Eight
  rows have no other leaf:
  - `rd_kafka_mock_cluster_io_set_event`, `rd_kafka_mock_cluster_io_clear_events`
    and `rd_kafka_mock_cluster_io_set_events`;
  - `rd_kafka_log_syslog` and `rd_kafka_set_logger`;
  - `rd_kafka_oidc_assertion_get_algo_label`;
  - `unit_test_telemetry_decode_error`;
  - `rd_kafka_txn_state_transition_is_valid`.

  Predicted: 3 to 8 of them become Safe directly. A macro line can hold another
  access, and a freed leaf can expose a later one. A few in-unit callers may
  follow through summaries (for example `rd_kafka_mock_connection_set_blocking`).
  The total is **+3 to +15 Safe from arm 1**.
- **Arm 2.** The number of decayed scalar-array member actuals in the corpus is
  not censused. Predicted: **+0 to +20 Safe**, together with leaf
  re-attributions to the callee (`call to uncontracted function`).
- **Never.** No Safe → non-Safe move, no new provisional Unsafe and no leak or
  violation removed by rule 35. A leak beside a literal read is still reported
  (fixture).
- **Null dictionaries.** Rule 35 does not touch the null analysis. Only the
  `custody`/`combined` fields of rows rule 35 moves may change.
- **System and tail-pause slice.** No brief body is on them. Arm 2 may move a
  few Fluent Bit rows (`char` array members handed to `strlen`/`memcpy`-like
  borrowed callees). Predicted 0 to +5 Safe on system, 0 on the slice, beside
  whatever rules 32 and 34 move.

A measured outcome that differs is reported as such and attributed. It is not
tuned toward this prediction.
