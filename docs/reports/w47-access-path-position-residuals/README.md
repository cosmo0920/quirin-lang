# W47-ACCESS-PATH-POSITION-RESIDUALS

Card of wave 47, round 2, on the P3 residuals of the
[unsupported-coverage queue](../../plans/unsupported-coverage/README.md).
Base: local `main` at `66be515f` ("Merge PROVIDER-PY-DUAL-STEM-FIXTURE-STALE
onto the round-1 integration"); `main` had not moved. Gateway rule
[10n](../../agents/gateway.md#constraint-10n); Lean
`QCore.TypedAccessPathPositionResiduals`. The ranking and the per-function
forecast were frozen in [PREDICTION.md](PREDICTION.md) (commit `f1c77afb`)
before any implementation commit.

## 1. Sizing

A parity-only record, `typed_position_shapes.rs` (commit `bc79f073`), writes a
`residual-shape` row beside every condition operand, condition assignment,
return and call actual that rule 10k's typed reader refuses. The row lists the
constructs of the refused Clang node that the reader does not read, whether a
unique node matched, and whether the node would be admitted with every root
premise granted. The sizing run with that build is verdict-inert: its
diagnostics, chunk contracts and Monkey source are byte-identical to the
baseline, and so is its parity stream once the new rows are dropped
([identity](artifacts/sizing/identity-corpus.json)).

`scripts/rank_shapes.py` joins each refused row to the COND, RET or IND leaf of
its function at or above its line (within 6 lines), then ranks shapes by the
Unknown functions a shape would free on its own:

- **strict:** every leaf of the function is covered by that shape;
- **with alias:** every leaf except its `unknown_alias` leaves is covered.

The largest single-shape figure is 9 functions. In the corpus there are 8,969
refused rows. The table lists the main causes; the sizing artifact
([ranking](artifacts/sizing/rank-corpus.json)) lists the full breakdown.

| Cause | Rows | Alone, strict / with alias |
|---|---:|---:|
| root premise alone | 2,919 | 2 / 5 |
| an address `&PATH` (`address`, `address+aggregate-value`) | 2,300 | 13 / 16 |
| no unique Clang node | 2,040 | 7 / 9 |
| `T *const` value | 335 | 6 / 7 |
| union member | 331 | 4 / 5 |
| nested call | 143 | 0 / 0 |
| anonymous inline enum member | 120 | 1 / 1 |
| `?:` | 55 | 0 / 3 |
| `&&` / `\|\|` in a return | 2 | 1 / 1 |

The system slice has 196 refused rows, and no shape frees a function alone
there.

## 2. Mechanism (rule 10n)

Each step is a fallback: it is asked only after every older route of its
position, and rule 10k's reader, have refused. The code sits in new modules:

- `typed_position_shapes.rs`: the address arm, the scope matcher, the
  anonymous-enum evidence and the parity record;
- `legacy_typed_residuals.rs`: connective returns.

The hooks into existing files are thin: `typed_access_path.rs` (the
`address_target` flag, `(&X)->f`, one `scalar` disjunct and the
`anonymous_members` collection), `typed_access_positions.rs`,
`legacy_typed_positions.rs` and `legacy_c.rs`.

1. **Address values.** `&PATH` is read with every per-hop obligation of reading
   `PATH`, but the place itself is not read, so only its value type is waived.
   - **In a condition:** only as an operand of a pointer comparison. It is
     never a truthiness operand, because C fixes that answer.
   - **Handed to a callee:** only at a `Borrowed` or declared data-tail
     position, and only when the place is exactly a member whose Clang type is
     not a pointer, array or function pointer, and is not `volatile` or
     `_Atomic`. This is O04-B09B10's disjointness premise: a callee that writes
     through the address reaches no linked member. Every root must also be never
     bound, which is rule 10k's pointer-actual premise.
   - `(&X)->f` is read as `X.f`.
   - An address is never returned (rule 5j).
2. **Controlling-scope matching.** Sometimes several Clang nodes carry an
   operand's line and tokens, as when a macro repeats it (`if (x) f(x);`). The
   matcher then keeps only the candidates inside a controlling expression or a
   `return` value. If several remain, it keeps the one in a Boolean leaf
   position. A unique match is unchanged, and several survivors refuse.
3. **Anonymous inline enum member.** A member is a scalar when its `FieldDecl`
   follows an unnamed `EnumDecl` in the same complete struct and its type spells
   that enumeration. A union member, a pointer to one, a bitfield or the
   spelling alone keeps the refusal.
4. **Connective returns.** A scalar `return` whose Clang node has `&&`, `||` or
   `?:` at the top is desugared exactly as rule 5h desugars a condition. The
   return must contain no call, assignment, increment or tracked descriptor.
   Both exits join one `return` with a `NoOp` exit.

Six shapes stay refused: a pointer-valued `?:` return, a call or assignment in a
return, an array-decay return, a `?:` actual and a nested-call actual.

**Negative controls** were written before the positive cases. They are in
[`tests/access_path_position_residuals.rs`](../../../crates/qcore-c-gateway/tests/access_path_position_residuals.rs)
(8 tests), and each one below keeps its refusal:

- an address at a custody position, at an uncontracted callee, through a
  tracked root, returned, of a pointer member, or of an element;
- an address through a root bound only by a nested condition assignment, which
  is the interlock: after release it is never `Safe`;
- a union place behind `(&X)->f`;
- two identical leaves in one condition;
- an anonymous enum in a union or behind a pointer;
- a tracked `||` guard;
- a return that calls, assigns or returns a pointer through `?:`;
- a released base read in a return.

The same file checks that `p && p->x` and `p || p->x` returns keep P3's
short-circuit scoping.

**Re-pinned deliberately (5 tests).** Each older route still refuses these
shapes. The typed fallback asked after it now reads them, and every test states
its reason:

- `address_of_member::a_name_typedefd_twice_is_read_on_clangs_one_type`: the
  lexical reader retires a doubly typedef'd name, but Clang has one
  `struct a_packer`, so the member's address certifies. The file's pointer,
  array, element, `volatile`, continued-selection and parenthesized refusals
  still pass.
- `container_root_reach::a_member_of_a_static_record_now_reaches_the_container_rule`:
  `&global_queue.items` now reaches rule 10d and discharges, as `&registry`
  does. A local record's member stays non-Safe with no leak.
- `intrusive_list_iteration::{a_body_that_writes_its_cursor_keeps_the_old_refusals,linking_a_tracked_object_into_the_iterated_list_declines_the_iteration}`:
  the declined iteration's loop condition `it != (&q->items)` is now read as a
  comparison with an address. The function stays Unknown on its body's
  refusal, and the second test stays not Safe.
- `return_expression_families::a_bound_base_in_a_skipped_position_is_read_only_where_c_reads_it`:
  - `return (*str == '+' || *str == '-');` is `Safe` (the corpus's
    `is_incremental`);
  - `return f ? b->n : 0;` after `k_free(b)` is the use-after-release C has on
    the `f` edge (provisional Unsafe), with no accusation on the other edge.

## 3. Measurement

**Runs.** [`scripts/measure.py`](scripts/measure.py) makes the same corpus and
system runs as `run_pinned_audit.py` and `run_system.py`, but with an explicit
private binary and `--jobs 5`. Every run exits 0.

- BEFORE uses the base binary (`6d9dc2b6…`).
- The step binaries are built from `f9c3fead` with
  [`scripts/step_flags.py`](scripts/step_flags.py); their digests are in
  [step-binaries.sha256](artifacts/steps/step-binaries.sha256).
- Step 4 is the committed code.

Private runs are under `private/pos/` of this worktree.

**BEFORE reproduces the baseline.**

- **Corpus:** `resource-diagnostics.json`, the chunk contracts, the Monkey source
  and the sorted parity stream and its contexts are byte-identical to
  `w47-callee-closure-r2/integration/artifacts/corpus/`. `compose3.py` BASE BASE
  BASE RUN reports COMPOSES with 0 mismatches.
- **System:** equal after masking path-derived digests. The 45 differing leaves
  are all 64-hex digests. The chunk contracts and the sorted parity stream are
  byte-identical.

The comparisons are in [artifacts/before/](artifacts/before/). They are made by
[`scripts/identity.py`](scripts/identity.py), because `baseline_identity.py`
cannot read the round-1 manifest.

| Measure | BEFORE | Step 1 | Step 2 | Step 3 | Step 4 = AFTER |
|---|---:|---:|---:|---:|---:|
| Corpus functions Safe / provisional Unsafe / Unknown | 510 / 10 / 4,314 | 520 / 10 / 4,304 | 520 / 10 / 4,304 | 521 / 10 / 4,303 | **522 / 10 / 4,302** |
| Units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | same | same | same | same |
| unsupported | 36,209 | 35,997 | 35,881 | 35,754 | **35,747** |
| unknown_alias | 7,443 | 7,464 | 7,510 | 7,623 | **7,627** |
| leak / summary_conflict | 36 / 1 | same | same | same | same |
| COND / IND / RET / UNC / OTHER | 6,331 / 15,037 / 284 / 8,883 / 5,674 | 6,278 / 14,910 / 284 / 8,846 / 5,679 | 6,162 / 14,910 / 284 / 8,846 / 5,679 | 6,053 / 14,893 / 284 / 8,845 / 5,679 | 6,053 / 14,892 / 278 / 8,845 / 5,679 |
| Newly admitted rows (step against the one before) | — | 219 actuals, 58 conditions, 3 assignments, 2 statements | 523 conditions | 148 conditions, 25 statements, 2 actuals | 6 connective returns, 6 conditions |

**System slice (step 4 against BEFORE).** The configuration rows stay
63 / 4 / 381, so the unique functions stay 33 / 2 / 199. Unsupported falls from
2,908 to **2,906** (2 COND leaves at one newly admitted condition row), and
unknown_alias stays at 985. No function moves.

**Every move** ([cumulative](artifacts/after/corpus-comparison/comparison.json),
per step under [artifacts/steps/](artifacts/steps/), by
[`scripts/compare_steps.py`](scripts/compare_steps.py)) is Unknown → Safe:

| Step | Function | Attribution |
|---|---|---|
| 1 | `rd_kafka_timers_lock`, `rd_kafka_timers_unlock` | direct: `&rkts->rkts_lock` actual |
| 1 | `rd_kafka_brokers_get_state_version` | direct: `&rk->rk_broker_state_change_lock` actuals |
| 1 | `rd_kafka_mock_group_initial_rebalance_delay_ms`, `rd_kafka_mock_set_group_consumer_heartbeat_interval_ms`, `rd_kafka_mock_set_group_consumer_session_timeout_ms`, `rd_kafka_mock_sharegroup_set_heartbeat_interval`, `rd_kafka_mock_sharegroup_set_session_timeout` | direct: `&mcluster->lock` actuals |
| 1 | `rd_kafka_toppar_needs_query` | direct: the P3 actual `rktp->rktp_broker` of `rd_kafka_broker_get_state`, now admitted because that callee's summary validates once its `&rkb->rkb_lock` is read (unforecast) |
| 1 | `rd_kafka_timer_is_started` | cascade: its two leaves were uncontracted calls to `rd_kafka_timers_lock`/`_unlock`, now Safe in the same unit, which the existing summary rule serves (unforecast) |
| 3 | `rd_kafka_mock_cgrp_classic_check_state` | direct: `mcgrp->state` conditions |
| 4 | `is_incremental` | direct: connective return |

Some things did not happen:

- **No Safe → non-Safe move**, so there is no correction.
- **No new provisional Unsafe.**
- **No added violation leaf:** no `leak`, `use_after_release`,
  `invalid_release`, `double_release` or `counted_user`.

So there is nothing to classify as real, artifact or premise gap.

**The interlock.** For all 12 moving functions, `compare_steps.py` finds no
newly admitted row whose root the AST binds while the lexical closure calls it
never bound. None of the 12 functions has such a name at all (`interlock_violations: 0`).

**Every changed leaf** ([changed-leaves](artifacts/after/corpus-comparison/changed-leaves.jsonl.gz))
is attributed:

- **Direct:** 276 COND, 144 IND, 6 RET and 2 UNC removals, and 182 added
  aliases, all on the line of a newly admitted row.
- **Callee became Safe:** 29 removed UNC leaves in 25 functions, whose
  callee became Safe in the same unit (`rd_kafka_timers_lock`/`_unlock`,
  `rd_kafka_brokers_get_state_version` and the others above).
- **Later obligations:** 2 added aliases.
- **Read by hand:** 14 leaves
  ([reviewed.json](artifacts/after/reviewed.json)).
  - 7 UNC and 2 COND removals follow callees the address step now models:
    `rd_kafka_curr_msgs_get` and `_cnt`, and `rd_kafka_broker_get_state` via
    `rd_kafka_broker_is_up`.
  - 3 exchanged `arguments do not match` leaves are at those calls.
  - 2 container refusals replace gate refusals on the same line:
    `mk_http_request_init` (`mk_list_add` of an unnamed object) and
    `mk_plugin_unregister` (a foreign `mk_list_del`).
  - 1 IND removal is `is_incremental`'s summary cascade in
    `decode_numerical_message`.

All of these stay Unknown.

The +184 aliases are the admitted rows' unresolved bases, and each one sits at
its row.

## 4. Prediction against outcome

The frozen forecast was **+8 to +18**, with 18 predicted Safe. The outcome is
**+12**, and 10 of the 12 moves were forecast.

- **Step 1: 8 of 9 predicted Safe hit.**
  - The miss is `rd_kafka_broker_outbufs_space`. Its actual
    `&rkb->rkb_outbufs.rkbq_cnt` is admitted, but the statement
    `int r = rkb->rkb_rk->rk_conf.queue_backpressure_thres - rd_atomic32_get(…)`
    still refuses, because it mixes the call with a second access path.
  - The three possible `mk_*` functions stay Unknown. Their `&q->_head` actual
    refuses by the root premise: the element is released later in the body.
  - There were 2 unforecast gains, listed in the moves table above.
- **Step 2: 0 of 7 predicted Safe.**
  - The 523 newly matched condition rows remove 116 COND leaves.
  - Each predicted function still refuses. The matched `rd_avg_rollover->ra_v.cnt`
    operands of the `calculate_*_avg` family refuse by the root premise. The
    `*_reply_tags_destroy` IND leaves belong to the same macro line's
    `RD_IF_FREE` call at a custody position, not to the matched condition.
  - The join by line credited them to the condition. That is a forecast
    artifact, and it is recorded here.
- **Step 3: 1 of 1 hit.**
- **Step 4: 1 of 1 hit.** The three `rd_kafka_metadata_*_cmp` stay Unknown, as
  forecast: their `?:` actual stays refused.
- The rest of the forecast held:
  - no correction;
  - no new violation;
  - units unchanged;
  - the system slice moves no function.
- The COND and IND forecast of "a few hundred" leaves held: 278 COND and 145 IND
  were removed.
- The `unknown_alias` forecast of "hundreds" was an overestimate: the rise is
  184.

## 5. Lean

`proof/QCore/TypedAccessPathPositionResiduals.lean` is registered in
`proof/AuditTypedAccessPathPositionResiduals.lean` (51 theorems). There are no
placeholders, and the only axioms are `propext`, `Classical.choice` and
`Quot.sound`. It imports `QCore.TypedAccessPathPositions` and proves, for each
shape:

- **Address values:**
  - refusal in a return, at custody or uncontracted positions, from a tracked
    root, and for union, cast or tracked-mixed places;
  - that the address waives only the value type
    (`an_address_waives_only_the_value_type`);
  - that every hop discharged adds no diagnostic;
  - that it preserves the frame, so P3's actual theorem holds;
  - that `(&X)->f` adds no dereference and normalises to a member selection.
- **Scope matching:**
  - a unique match is unchanged;
  - a selection is a candidate, and an ambiguous selection is in scope;
  - two leaves in scope refuse;
  - P3's refusals are kept.
- **Anonymous enum member:**
  - the arm only adds inline members;
  - a union member, a pointer and a spelling alone never become scalars;
  - earlier refusals are kept.
- **Connective returns:**
  - `a_connective_return_is_its_c_evaluation`;
  - `&&` and `||` guard discharge, with non-vacuity;
  - a `?:` return evaluates only the selected arm;
  - the return hands nothing back;
  - pointer, call, assignment, older-route and undecided-leaf refusals.
- **Unlisted shapes** (`?:` actual, nested-call actual, pointer `?:` return,
  call or assignment in a return, array-decay return) refuse.

The Rust-to-Lean correspondence is a review premise (constraint 28).

## 6. Gate

[gate.json](gate.json), exits in [gate/](gate/) ([`scripts/gate.sh`](scripts/gate.sh),
each exit in its own file, no pipe, logs under `private/pos/gate2/`), at
`1cc4e643`:

- submodule 0;
- `run_test_binaries.py` 0: **342 units, 355 summaries; 6,149 passed, 0
  failed, 11 ignored** (`private/test-binaries/20260928T040751Z-1878271/summary.json`);
- `cargo clean --profile dev` 0, fmt 0, clippy 0;
- `reevaluation_probe` 0 and `reevaluation_followup` 0;
- `check_lean_audits.py` 0: **218** audits (+1);
- `unittest discover -s scripts/tests` 0: **365** OK;
- `build_docs_index.py --check` 0;
- `clang --version` 0: 21.1.8, recorded in `gate/`.

No `providers/` path is touched, so the provider gate does not apply.

**Attempt 1** at `8d78f254` ([gate/attempt1/](gate/attempt1/)) failed only
clippy: `collapsible_match` in the parity-only shape walker. `1cc4e643`
rewrites that arm as a match guard, with the same answer. Every other step of
attempt 1 exited 0 with the same counts. The measured step binaries were built
at `f9c3fead`; `1cc4e643` changes only that parity-only arm.

## 7. Follow-ups

The ledger is applied by [`scripts/update_ledger.py`](scripts/update_ledger.py)
and then `build_matrix.py`. It stays at 536: none registered, none retired.

**Amended:**

- `W47-ACCESS-PATH-POSITION-RESIDUALS`: the sizing, the four discharged shapes
  with their measured moves, and the AFTER residual. The residual is 8,066
  refused rows:
  - root premise: 4,367;
  - no unique node: 1,460, of which 939 are call actuals of a callee called
    twice on one line;
  - an address kept refused: 618;
  - union: 367;
  - `T *const`: 332;
  - `?:` actual: 53;
  - nested call: 143;
  - the named pointer `?:` returns, call or assignment returns, and the rule 5j
    decay.

  It is not retired. The pointer `?:` return, the `?:` actual and the
  nested-call actual are neither admitted nor closed by a keep-refused
  decision. Rule 10n only states that each stays refused under this rule.
- `W47-TYPED-CONST-POINTER-VALUE`: its residual rows are sized (335 rows, 7
  functions alone with their alias leaves, 6 strict).
- `W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT`: the interlock result (none of the
  12 moves), and the root premise as the largest remaining cause (4,367 rows,
  19 / 8 functions alone).

## Files likely to conflict

These are also touched by `W47-TYPED-CONST-POINTER-VALUE` and
`W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT`:

- `crates/qcore-c-gateway/src/typed_access_path.rs`: the `FieldFacts` fields
  become `pub(crate)`, plus `EnumFacts.anonymous_members`, `Reader.address_target`,
  `path`'s address check and `(&X)->f`, and a disjunct in `Reader::scalar`;
- `typed_access_positions.rs`, `legacy_typed_positions.rs`, `legacy_c.rs` (the
  `return` hook and the module declaration), `lib.rs`;
- the five re-pinned tests;
- `proof/QCore.lean`;
- `docs/agents/{gateway.md,README.md}`, `docs/STATUS.md`,
  `docs/plans/unsupported-coverage/README.md`,
  `docs/development/increments/wave-47.md`,
  `docs/reviews/r0_3/acceptance_matrix.json` and `docs/reports/INDEX.md`.
