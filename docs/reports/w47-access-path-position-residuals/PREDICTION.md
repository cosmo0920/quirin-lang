# W47-ACCESS-PATH-POSITION-RESIDUALS: frozen prediction

Frozen 2026-09-28 on base `66be515f` (local `main`), after the BEFORE and
sizing runs and before any implementation commit. Later corrections go in the
[report](README.md), never here. Code for steps 3 and 4 below was drafted in
the working tree while the sizing run was in flight; none of it was built,
run or committed before this file.

## BEFORE

`scripts/measure.py` (the `run_pinned_audit.py` and `run_system.py`
commands with an explicit private binary), base binary, `--jobs 5`. Corpus
diagnostics, chunk contracts, Monkey source and the sorted parity stream and
contexts are byte-identical to the round-1 combined baseline
(`w47-callee-closure-r2/integration/artifacts/corpus/`,
[identity](artifacts/before/identity-corpus.json)); `compose3.py` BASE BASE
BASE RUN composes with 0 mismatches. The system slice is equal after masking
path-derived digests (45 differing leaves, every one a 64-hex digest), with
byte-identical chunk contracts and sorted parity
([identity](artifacts/before/identity-system.json)). Baseline figures hold:
corpus functions 510 / 10 / 4,314, unknown_alias 7,443, unsupported 36,209,
leak 36; system unique functions 33 / 2 / 199.

## Sizing (step 1 of the brief)

A parity-only record (`typed_position_shapes.rs`, commit `bc79f073`) writes,
beside every refused typed condition, condition-assignment, return and
call-actual row, the constructs of the refused Clang node the reader does not
read, whether a unique node matched, and whether the node is admitted with
every root premise granted. The sizing run with that build is verdict-inert:
its diagnostics, chunk contracts, Monkey source and parity stream (without the
new rows) are byte-identical to the baseline
([identity](artifacts/sizing/identity-corpus.json)).

Corpus refused rows, de-duplicated by (function, line, tokens, position):
8,969 (4,365 call actuals, 4,306 condition operands, 233 returns, 65
condition assignments). By leading cause
([rank-corpus.json](artifacts/sizing/rank-corpus.json)):

| Cause | Rows |
|---|---:|
| root premise alone (admitted with every root premise granted) | 2,919 |
| an address `&PATH` (member lock, `(&X)->f`, element address) | 2,300 |
| no unique Clang node (a macro repeats the operand on one line) | 2,040 |
| `T *const` value (W47-TYPED-CONST-POINTER-VALUE's) | 335 |
| union member | 331 |
| array decay | 208 |
| function-pointer value | 147 |
| nested call | 143 |
| `sizeof` operand | 130 |
| anonymous inline enum member | 120 |
| cast | 93 |
| `?:` | 55 |
| assignment | 54 |
| pointer arithmetic | 45 |
| other (aggregate value, array value, increment, `&&`/`||`, qualifier, unclassified) | 49 |

**Ranking by functions a shape unblocks alone** (every leaf of an Unknown
function is a COND, RET or IND leaf whose rows are all of that shape;
`with-alias` also allows `unknown_alias` leaves). The joined figures are
small: no shape frees more than 9 functions alone.

| Shape | Alone, strict | Alone, with alias | Rows | Decision |
|---|---:|---:|---:|---|
| `address` | 9 | 9 | 1,391 | **implement** (step 1) |
| `no-unique-node` | 7 | 9 | 2,040 | **implement** for controlling expressions and returns (step 2) |
| `const-pointer` | 6 | 7 | 154 | not here: W47-TYPED-CONST-POINTER-VALUE |
| `address+aggregate-value` | 4 | 7 | 423 | **implement** (step 1: `(&X)->f` and an address of an aggregate member) |
| union with a function-pointer member (`rko_u`) | 4 | 5 | 175 | not here: W46-UNION-MEMBER-PATH |
| `root-premise` | 2 | 5 | 2,919 | not here: the root premise stands (rule 10l, W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT) |
| `pointer-arithmetic` | 3 | 3 | 45 | keep refused (P2's refusal) |
| `conditional` | 0 | 3 | 52 | `?:` actual deferred: needs a branching call lowering (sized, not admitted) |
| `anonymous-enum` | 1 | 1 | 116 | **implement** (step 3) |
| `logical` (return) | 1 | 1 | 2 | **implement** (step 4, with scalar `?:` returns) |
| `sizeof-operand` | 1 | 2 | 130 | keep refused |
| nested call (`call+…`) | 0 | 0 | 143 | not admitted: frees nothing alone; sized for the follow-up |

System slice ([rank-system.json](artifacts/sizing/rank-system.json)): 196
refused rows; no planned step frees a function alone; 26 rows fall to step 1.

## Planned steps (each measured on its own, cumulatively)

1. **Address values.** `&PATH` (a member or subscript path of known fields)
   read with P2's per-hop obligations for every dereference in `PATH` and no
   read of the final place: as a pointer operand of a condition (a
   comparison or truthiness; the value is never null), and as a call actual
   at a `Borrowed` or declared data-tail position, pointer-valued, so every
   root must be never bound (rule 10k's pointer-actual premise). `(&X)->f`
   is read as `X.f`. Never in a return (rule 5j).
2. **Controlling-scope node matching.** When several Clang nodes on the line
   carry the operand's exact tokens, only those inside a controlling
   expression or a `return` value are candidates; a unique survivor is the
   operand. Call-actual rows keep their refusal.
3. **Anonymous inline enum member.** A member whose type is an unnamed
   `EnumDecl` defined immediately before it in the same struct is a scalar.
4. **Connective returns.** A call-free, assignment-free scalar `return` whose
   Clang node is a top-level `&&`, `||` or `?:` is desugared as rule 5h
   desugars a condition, both exits joining one `NoOp` return.

## Forecast (corpus)

Per function ([predict-corpus.json](artifacts/sizing/predict-corpus.json)):
**Safe** where strict, **possible** where only alias leaves remain besides.

| Step | Predicted Safe | Possible |
|---|---|---|
| 1 | `rd_kafka_broker_outbufs_space`, `rd_kafka_brokers_get_state_version`, `rd_kafka_mock_group_initial_rebalance_delay_ms`, `rd_kafka_mock_set_group_consumer_heartbeat_interval_ms`, `rd_kafka_mock_set_group_consumer_session_timeout_ms`, `rd_kafka_mock_sharegroup_set_heartbeat_interval`, `rd_kafka_mock_sharegroup_set_session_timeout`, `rd_kafka_timers_lock`, `rd_kafka_timers_unlock` | `mk_fifo_queue_destroy`, `mk_plugin_exit_stages`, `mk_sched_worker_cb_free` |
| 2 | `rd_kafkap_Fetch_reply_tags_destroy`, `rd_kafkap_Produce_reply_tags_destroy`, `calculate_consumer_fetch_latency_avg`, `calculate_queue_time_avg`, `calculate_share_fetch_latency_avg`, `calculate_share_fetch_size_avg`, `calculate_throttle_avg` | `rd_kafka_sw_str_is_safe`, `rd_kafka_ListOffsetRequest_parameters_destroy_free` |
| 3 | `rd_kafka_mock_cgrp_classic_check_state` | — |
| 4 | `is_incremental` | the three `rd_kafka_metadata_*_cmp` (their `?:` actual stays refused, so these stay Unknown) |

So **+18 Safe at most** (9 + 7 + 1 + 1), more likely fewer: the forecast
reads shape rows, not the step code, so an admitted address may still refuse
by its root premise (the premise answer was taken with the address refusal
in place), and a lock actual (`&ra->ra_lock`) may meet a callee whose
`Borrowed` contract is not in force. Expected range **+8 to +18**; the
leading uncertainty is step 1's root premise. Plus summary cascades through
the existing same-unit rule, not forecast.

- Leaves: COND, IND and RET fall by roughly the admitted rows' leaves
  (a few hundred); `unknown_alias` rises where an admitted read exposes an
  unresolved base (P3 added 2,680), by hundreds.
- **No Safe → non-Safe move**, no new provisional Unsafe and no added
  `leak`, `use_after_release`, `invalid_release`, `double_release`: every
  step only replaces an `unsupported` refusal by obligations C evaluates.
  Any added violation leaf would be classified by reading.
- Units unchanged. No Safe move may rest on a root in rule 10l's disputed
  set (the interlock); forecast: none does.

## Forecast (system slice)

No function moves (26 refused rows fall to step 1; none is a function's sole
blocker). Unsupported may fall by a handful.
