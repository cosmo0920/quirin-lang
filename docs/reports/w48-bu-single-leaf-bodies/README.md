# Literal reads and decayed scalar array members (rule 35)

Card W48-BU-SINGLE-LEAF-BODIES adds gateway rule
[35](../../agents/gateway.md#constraint-35). It has two arms:

- **Arm 1.** A `*` applied to a string literal that is only read is constant
  data to the lexical access gate.
- **Arm 2.** At an admissible W44 call actual, an array member that decays to a
  pointer is read as rule 14c's element address `&PATH[0]`. This holds only for
  a one-dimensional array of builtin scalars, unqualified, with every root
  premise unchanged.

The pinned corpus moves exactly **12 rows Unknown → Safe** when the TIP is
measured against its own arms-off control. On the comparable eight-home route,
the merged TIP reads **977 → 999 Safe, 3,848 → 3,826 Unknown, 9 provisional
Unsafe unchanged** against BASE `4dd8c8013`:

- **+12** from rule 35;
- **+10** from sibling rules 32 (8) and 34 (2).

No row leaves Safe. No Unsafe appears. Leaks stay at 40. The null analysis is
unchanged on every route, and the combined Safe count rises from 851 to 859
under rule 35. System (448 rows: 99 Safe / 345 Unknown / 4 provisional Unsafe)
and the tail-pause slice (404 rows: 57 Safe / 342 Unknown / 4 Pending / 1
provisional Unsafe) keep every status. Rule 35 changes only their leaves.

Of the twelve brief bodies, rule 35 frees exactly one,
`rd_kafka_mock_cluster_io_set_event`, as predicted. One prediction misses:
`XXH32_digest_endian` and `XXH64_digest_endian` keep their unchanged access leaf
(see [the twelve bodies](#the-twelve-brief-bodies)).

## Chronology (stated plainly)

[PREDICTION.md](PREDICTION.md) was **not** committed before the first semantic
change. The order was:

1. `0bf8f9f47`: the measurement driver and link map.
2. `b41a36512`: the rule-35 Rust and Lean. This is the first semantic commit,
   committed as WIP after a session restart.
3. `5e2d3c368`: the gateway text and its router row.
4. `81911ed3f`: merge of local integration main `0217006f2`.
5. `1482e0267`: the prediction. It came after all of the above and before any
   corpus, system or slice measurement.

The prediction file states this chronology itself. It has never been edited.
The human halt of 2026-10-23
([record](../w48-impl-round1-halt/README.md)) stopped the card before any
measurement. The resumed card then made these commits:

- `0a9c4c3c6`: merged main `8991b7b58`.
- `8879c2d89`: adopted the scheduler card's driver, so corpus figures compare
  across cards.
- `1a2ffed1d`: added the input re-spelling described below.

Analyzer code has not changed since `b41a36512`. The measured TIP binary is
built from `8879c2d89`. Every later commit changes only this report directory.

## What changed

**Arm 1** lives in `literal_dereference.rs`, with a thin hook in
`legacy_token_shapes::contains_unmodeled_access`.

- A `*` whose next token is a literal is not an access when the literal run is
  not stored into or incremented.
- A string literal is a static array (C11 6.4.5p6), so `*"..."` reads its first
  character. That reads no pointer the projection names and changes no binding,
  link or obligation.
- No constant is folded, so both edges of `rd_assert(!*"...")` stay reachable.
- Every other token of the predicate keeps its answer.

**Arm 2** lives in `array_member_decay.rs`, with a thin hook in
`typed_access_positions::StatementIndex::argument`.

- An `ArrayToPointerDecay` of a member place whose type is a one-dimensional
  array of builtin scalars, not `volatile` and not `_Atomic`, is read through
  rule 10n(1)'s address reading. Each hop keeps its obligation, and every root
  keeps the never-bound premise.
- Pointer, record, union, multi-dimensional, enumeration or opaque elements are
  refused. So are qualified members and non-member decays.
- Parity runs record each decision as a `typed-place` place (`rule: "35"`).

**Lean.** `QCore.LiteralDecayReadings` (14 theorems, audited in
`proof/AuditLiteralDecayReadings.lean`) binds both Rust `DECISION_VECTORS`
tables.

**Tests.** `crates/qcore-c-gateway/tests/literal_decay_readings.rs` holds the
fixtures, and the two unit modules hold the decision vectors.

**Premises (stated, not proved).**

- Faithful lexing of literals, and faithful extraction of Clang's cast kinds
  and member types (trust boundary 28).
- As for rule 14c, the callee writes within the array the address points into.
  Bounds are outside the resource property.

## Route

The corpus route is the conditional eight-home pinned corpus with the batch4
link map. It is the scheduler card's route: `scripts/measure.py` copies that
card's driver. Each side runs its own `git archive` snapshot with its own runner,
profiles and release binary. The system and slice routes are the configured
Fluent Bit routes; the slice reads the tail-pause preparation, as batch5 does.

**TIP-off control.** The arms-off control is the TIP snapshot with
`DECAY_ARM` and `LITERAL_ARM` set false; the diff is retained as
`private/slb/run-4/tipoff.diff`. Two comparisons follow from it:

- TIP against TIP-off isolates rule 35.
- BASE against TIP-off is the sibling effect of rules 32 and 34.

The two add up exactly: 22 = 12 + 10 status moves.

**Input re-spelling (new, forced by the checkout move).**

- After the folder migration, `/home/cosmo/Documents/GitHub/qcore-c` is a
  symlink to `/media/Data/GitHub/quirin-lang`.
- The pinned compile databases spell the old prefix.
- `run_pinned_audit.py` resolves its inputs root to the real path, and the
  audit selects units by comparing path strings (`pinned_route.rs`
  `plan_library`, `current.rs` `export_cross_unit_home`).
- So no unit matched, and every route of run 4 failed before any analysis.

`scripts/mirror_inputs.py` re-spells the inputs:

- For the corpus, it copies the pinned source, Monkey and the chunkio package
  byte for byte. Each copy's content digest equals its original's.
- It rewrites only the path prefix of the four compile databases. The mirror
  manifest records every original and rewritten sha256.

Clang therefore reads identical bytes. The BASE reproduces the scheduler card's
**977 Safe / 3,848 Unknown / 9 Unsafe** exactly, which confirms the route is
comparable.

## Attempts

[attempts.json](attempts.json) lists them.

- Two queued BASE attempts never acquired the lock.
- Run 3 was interrupted by the human halt while building.
- Run 4 failed on the path spelling.
- **Run 5 is the evidence.** All nine routes exit 0 with no identity error;
  each invocation records its binary and input digests.

[native-comparison.json](native-comparison.json) holds the per-route tallies,
every status move with its leaf trade, the null records and the outer
projection deltas. The full attributions are under `private/slb/cmp/` in the
card worktree.

## Rule 35's twelve moves (corpus, TIP-off → TIP)

| Row | Arm | Removed leaf | TIP dependencies |
|---|---|---|---|
| `rd_kafka_mock_cluster_io_set_event` | 1 | branch condition `rd_assert(!*"...")`, `rdkafka_mock.c:1153` | `__assert_fail` |
| `rd_kafka_mock_cluster_io_set_events` | 1 | same shape, `:1130` | `__assert_fail` |
| `rd_kafka_mock_cluster_io_clear_events` | 1 | same shape, `:1173` | `__assert_fail` |
| `rd_kafka_mock_connection_set_blocking` | 1, through a summary | `call to uncontracted function rd_kafka_mock_cluster_io_set_event`, `:1693` | summary `rd_kafka_mock_cluster_io_set_event` |
| `rd_kafka_log_syslog` | 1 | `rd_assert(!*"...")`, `rdkafka.c:404` | — |
| `rd_kafka_set_logger` | 1 | same, `rdkafka.c:415` | — |
| `rd_kafka_oidc_assertion_get_algo_label` | 1 | `rd_assert(!*"Unknown JOSE algorithm")` | — |
| `unit_test_telemetry_decode_error` | 1 | `rd_assert(!*"...")` | — |
| `rd_kafka_txn_state_transition_is_valid` | 1 | `RD_BUG(...)` | — |
| `rd_kafka_anyconf_set_modified` | 1 | `rd_assert(idx < MAX && *"...")`, split over two lines; its dependent `unresolved resource identity conf` leaf goes with it | summary `rd_kafka_prop2idx`; null premises on `confhdr` |
| `rd_kafka_log_print` | 2 | `fprintf(stderr, ..., rk ? rk->rk_name : "", buf)`: `char rk_name[]` decayed on rule 10z's arm | `fprintf`, `gettimeofday`, its own pre-existing body-checked declaration |
| `mk_config_signature` | 2 | three `snprintf(server->server_signature…, sizeof …)` on `char[16]`/`char[32]` members | `snprintf` |

**Soundness review.** Every removed leaf was read against its source window.

- 80 leaves sit on, or within three lines of, a literal dereference. Others sit
  on a macro line that expands to one (`RD_UT_FAIL`,
  `rd_kafka_anyconf_set_internal`).
- Arm 2 accounts for the decays (`rk_name`, `server_signature`, `d->scratch`,
  `name`, `rkhdr_name`, …).
- The remaining removals are callers whose freed callee is now summarized.

No removal comes from another shape.

**`rd_kafka_anyconf_set_modified`.** Its identity leaf on the alias
`confhdr = conf` is a dependent diagnostic. Two siblings of the same shape,
`rd_kafka_anyconf_clear_all_is_modified` and `rd_kafka_anyconf_is_modified`, are
already Safe at both BASE and TIP-off.

**`rd_kafka_log_print`.** It depends on its own `client.qv` declaration, which
existed at BASE. 60 BASE Safe rows already have the same self-dependency. Its
Safe comes from its body, which now has no diagnostic. That body-validates the
declaration rather than resting on it.

**Assumptions carried by the twelve verdicts.**

- The unchanged environment: assumption digest `d993ea5b…`, identical on both
  sides.
- Rule 35's stated premises above.
- `rd_kafka_anyconf_set_modified`'s two `confhdr` null premises, which the null
  side proves under its inferred `#conf` precondition.

No new hypothesis is discharged ("no new hypothesis discharge" in every
`hypothesis_context`).

**Combined and runtime views.**

- Combined Safe rises 851 → 859. The four mock rows stay combined Unknown
  because their null verdicts are Unknown.
- Under every loaded runtime scenario (`all`, `flb_external_plugin`,
  `rk_plugin_library`), none of the twelve moves: no row is added to any
  scenario's moved list.
- By contrast, all ten sibling rows move Safe → Unknown under every runtime
  scenario, so their Safe is static only.

## Elsewhere (rule 35, leaves only)

**Corpus.** 116 rows differ beyond environment digests. Rows with a gate leaf on
a literal line fall from 61 (76 leaves, 8 sole-blocked) to 7. The seven left are
other questions:

- `RD_BUG` with indirect arguments;
- `rd_kafka_assert(rkb->rkb_rk, !*"...")` identity leaves;
- one cJSON `sizeof` leaf.

New leaves name the callee or identity the old refusal hid:

- `rd_http_error_new`, `rd_kafka_ConfigEntry_new`,
  `rd_kafka_interceptors_on_request_sent`, `rd_strtup_new`;
- `conf`, `mconn`, `mrkb`, `names`.

Diagnostic counts (TIP-off → TIP): unsupported 27,593 → 27,465, unknown alias
11,004 → 11,002, leak 40 → 40.

**System.** Four rows lose one access leaf each and keep their status:
`flb_output_proxy_tls_ca_check` and `print_storage_info`, each in two
configurations.

**Slice.** Eleven access leaves go. Three re-attribute to their callees:

- `flb_sqldb_open` in `flb_tail_db_open`;
- `flb_pipe_create` in both `flb_input_channel_init` configurations.

The collector pause, resume and start rows lose theirs. No status moves.

**Null.** Every null verdict is unchanged on every route. Two kinds of field
change:

- `custody`/`combined` change on moved rows only.
- `custody_null_premises` appears on three unmoved rows, whose newly read
  actuals' roots are now listed: `rd_kafka_header_cmp_str` and
  `rd_kafka_mock_connection_write_out_tmr_cb` in the corpus, and
  `print_storage_info` in system. Custody stays Unknown, so these lists change
  nothing.

## The twelve brief bodies

| Body | Prediction | Measured |
|---|---|---|
| `rd_kafka_mock_cluster_io_set_event` | Unknown → Safe (arm 1) | **Unknown → Safe** |
| `XXH32_digest_endian`, `XXH64_digest_endian` | Unknown; the leaf moves to the callee `XXH*_finalize` | **Miss.** Unknown with the same `indirect memory access` leaf (`rdxxhash.c:542`/`:1002`). The decayed `state->mem32`/`mem64` sits in a `return XXH32_finalize(...)` whole-return call, and arm 2 (in `StatementIndex::argument`) did not decide that actual on this route. The route that refused it was not isolated further. |
| `rd_kafka_queue_get_sasl` | unchanged | unchanged |
| `rd_kafka_broker_ApiVersion_at_least` | unchanged | unchanged |
| `thrd_setname` | unchanged | unchanged |
| `rd_kafka_MemberDescription_copy`, `rd_kafka_group_result_copy` | unchanged | unchanged |
| `rd_kafka_topic_partition_list_grow` | unchanged | unchanged |
| `rd_map_init` | unchanged | unchanged |
| `mk_iov_free_marked` | unchanged (custody position) | unchanged |
| `rd_kafka_topic_partition_list_cmp` | unchanged (rule 11p open set) | unchanged |

**Against the rest of the prediction.**

- **Arm 1.** Predicted +3 to +15 Safe. Measured +10: all eight sole-leaf rows,
  plus `rd_kafka_anyconf_set_modified` (a two-line assert outside the
  single-line census) and the predicted in-unit caller
  `rd_kafka_mock_connection_set_blocking`. `rd_kafka_log_syslog` and
  `rd_kafka_set_logger` were in the census.
- **Arm 2.** Predicted +0 to +20. Measured +2.
- **System.** Predicted 0 to +5. Measured 0.
- **Slice.** Predicted 0. Measured 0.
- **"Never" clauses.** All hold: no Safe → non-Safe move, no new Unsafe, no leak
  removed, null unchanged.

## Sibling effects (BASE → TIP-off, rules 32 and 34)

These are attributed to their own cards and reported here only for the figures.

**Rule 32 (eight rows):**

- `rd_kafka_log0`;
- `rd_kafka_share_ack_commit_cb_register_op` and
  `rd_kafka_share_log_fetch_stall`;
- the three mock member-active rows;
- `rd_kafka_mock_cgrp_classic_set_state`;
- `rd_kafka_topic_set_state`.

**Rule 34 (two rows):** `sched_thread_cleanup` and
`mk_sched_threads_destroy_conn`.

**Totals.** 977 → 987 Safe. These match the halt record's lists exactly.

**Other sibling effects.**

- Combined Safe rises 849 → 851.
- Null verdicts unchanged; `custody_null_premises` lists change on ten unmoved
  rows.
- All ten rows move Safe → Unknown under every runtime scenario.
- The halt record's concern about `rd_kafka_log0` resting on its own declared
  contract belongs to rule 32's card.

## Planner (fresh BASE diagnostics)

`bottom_up_plan.py` was run on run 5's corpus diagnostics.

| Side | Candidate bodies | Positive | Ceiling | One-body waits | Same/cross closure |
|---|---|---|---|---|---|
| BASE | 2,214 | 232 | 360 | 253 | 4/4 |
| TIP-off | 2,203 | 229 | 358 | 248 | 6/6 |
| TIP | 2,200 | 230 | 383 | 249 | 6/6 |

These are census quantities, not promised gains. At BASE every brief body
scores 1.0 (one own leaf, one caller freed). At TIP eleven remain at 1.0;
`rd_kafka_mock_cluster_io_set_event` has left the table.

## Gate

The full standing gate ran on `d81a788d2`, the commit holding this report,
under the shared lock (2026-10-23 14:15–14:29Z). [gate.json](gate.json) records
it, and the logs are under `private/slb/gate-1` in the card worktree. Every step
exits 0:

- Rust: 545 units / 558 summaries, **8,587 passed, 0 failed, 15 ignored**;
- fmt and Clippy;
- both reevaluation examples;
- **369 Lean audits**;
- **374 Python tests**.

The tree was clean before and after. Clang was 21.1.8. `providers/` is
unchanged, so no provider gate applies.

## Residuals and follow-ups

- **W48-RETURN-CALL-DECAYED-MEMBER.** A decayed scalar array member as the
  actual of a whole-return call (`return F(..., S->arr, ...)`) is not read by
  arm 2. `XXH32_digest_endian` and `XXH64_digest_endian` keep their access leaf.
  - DISCHARGE: isolate the route that refuses that actual and extend the same
    decision to it, with matching Lean, a fixture and a fresh BASE/TIP
    re-measure.
- **PINNED-INPUTS-PATH-AFTER-MIGRATION.** Every pinned and configured route fails
  on path spelling since the checkout moved, until its inputs are re-spelled
  (`scripts/mirror_inputs.py`).
  - DISCHARGE: regenerate the pinned inputs under the real path, or compare
    canonical paths in the audit, with its own gate and BASE parity.
- The ten other brief bodies keep the questions the prediction named: callee
  contracts, `?:` actuals, reallocated or aggregate member stores, custody
  position and the rule 11p indirect call. These stay with
  CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, W43-INDIRECT-UNTYPED-SINGLE-ACCESS and
  W44-CONDITION-CALL-INDIRECT-ARGUMENT.
