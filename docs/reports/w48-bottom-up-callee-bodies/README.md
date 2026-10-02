# W48-BOTTOM-UP-CALLEE-BODIES: the bottom-up plan, and one body family lifted

Card W48-BOTTOM-UP-CALLEE-BODIES, wave 48 round 14, 2026-10-02, base local
`main` `c85cb8806`. Plan P4, round 4, in the
[unsupported-coverage queue](../../plans/unsupported-coverage/README.md).
The [prediction](PREDICTION.md) was committed (`16324b7ed`) after the BEFORE
runs and before any implementation code (`97c4da505`). Gateway rule
[12f](../../agents/gateway.md#constraint-12f); Lean `QCore.ScalarReturnValue`
(new `AuditScalarReturnValue.lean`).

## Headline

- **BEFORE reproduces the base.** Corpus 647 / 9 / 4,178, `dde4837e…`, the
  brief's digest. System 44 / 2 / 188 and slice 29 / 1 / 238 / 2 Pending
  (rows merged per unit and function).
- **The yield is spread thin.** Taking every Unknown in-corpus callee body
  Safe frees 416 corpus functions in one round, and 338 of them wait on
  exactly one body; but 2,053 bodies are involved and no single body frees
  more than 12 (section 2). The bottom-up yield is a sequence of small
  lifts, each sound on its own.
- **One rule residual lifted (gateway 12f).** A refused scalar `return`
  value is read as the value of a statement, with the statement position's
  obligations. `rd_clock` and `rd_uclock` (librdkafka header bodies) now
  validate in every unit that includes them.
- **Movement, as predicted:** corpus **648 / 9 / 4,177** (`bcc98d2d…`),
  `rd_kafka_timer_next` Unknown → Safe. 112 call leaves to the two clocks and
  the helpers they alone blocked leave the corpus. No Safe → non-Safe move,
  no leak or violation added. System and slice byte-identical.
- **Cross-unit route:** re-measured and not built (section 5). Alone it
  frees 5 (6 at its ceiling), and 10 when calls held inside return and
  condition leaves are counted. Every body this card lifted is carried by
  the same-unit route.
- **Ledger** 695 → 696: one retired (`W48-RETURN-AGGREGATE-MEMBER-READ`),
  four amended, two registered (section 7).

## 1. BEFORE

[`scripts/measure.sh`](scripts/measure.sh) (W48-CALLEE-CLOSURE-R3's, with
paths under `private/w48bu/`) runs the corpus arbiter (`run_pinned_audit.py
… 66910c10 --jobs 4`), the system route and the tail-pause slice. Every run
exits 0. [`scripts/totals.py`](scripts/totals.py) records the digests
([artifacts/after/totals.txt](artifacts/after/totals.txt)). The binaries are
in `artifacts/before/binary.sha256` and `artifacts/after/binary.sha256`.

## 2. The plan (task step 1)

The scripts read the BEFORE diagnostics and parity stream. Their outputs are
in [artifacts/before/](artifacts/before/) and `artifacts/ranking/`:

- [`closure_r3.py`](scripts/closure_r3.py) (round 3's) gives the closure and
  the classes. 2,539 Unknown functions carry a call leaf and 490 carry
  nothing else. The same-unit rule frees 0, a cross-unit route 5 (6 at its
  ceiling), every in-corpus body taken Safe 423.
- [`bottom_up_plan.py`](scripts/bottom_up_plan.py) takes one Unknown body
  Safe at a time, under same-unit and cross-unit serving, beyond the
  baseline closure. It names each body's own blockers and ranks by callers
  freed per own leaf. The ceiling set is 416 callers: 338 wait on one body,
  42 on two and 36 on more.
- [`family_leverage.py`](scripts/family_leverage.py) removes one leaf family
  everywhere and runs the closure. It is the reachable question, since a
  family is what a rule lifts.
- [`return_reasons.py`](scripts/return_reasons.py),
  [`leaf_set_leverage.py`](scripts/leaf_set_leverage.py) and
  [`predict_12f.py`](scripts/predict_12f.py) split the return family by the
  elaborator's refusal.
- [`held_calls.py`](scripts/held_calls.py) recovers the calls held inside
  position leaves (section 5).

**Top bodies, with each own blocker classified:**

| Body | Frees | Own leaves | Blocker | Class |
|---|---:|---:|---|---|
| `lz4.c::LZ4_decompress_generic` | 12 | 116 | builtins, assignments, aliases | several Clang-gap residuals |
| `cJSON.c::cJSON_New_Item` | 10 | 1 | `hooks->allocate(…)`, an indirect callback | decision class 1 |
| `cmt_/ctr_mpack_unpack_array` | 5 + 5 | 8 | callback dispatch, cross-unit | decision class 1 |
| `rd_kafka_error_new_v` | 5 | 11 | `va_list` not a plain local, aliases | rule 12c/11w residual |
| `sds_cat_safe` | 4 | 1 | `cfl_sds_cat_safe` | decision class 2 |
| `rd_kafka_admin_result_ret_*` (6) | 4, 3, 3, … | 1 each | pointer return after `&rko->…` reaches a call | rule 10k/5b residual (section 6) |
| `rd_clock`, `rd_uclock` (header) | 1 reported, 112 leaves | 1 | scalar member return of an address-taken local | rule residual, **lifted (12f)** |
| `LZ4_NbCommonBytes`, `log2_floor` | 1 each | 1–4 | `__builtin_ctz*`/`clz*` returns | decision class 6 |

**Top families** (cross-unit serving, baseline 6 included): indirect memory
access 587, **return expression 81**, branch condition 56, unresolved
identity 45, braced initializer 17, assignment destination 13, loop
condition 12.

**Return family by reason:**

- "the typed access-path reader refused it" frees 29. Of these, 24 are the
  librdkafka admin accessors (section 6).
- "no value form of this card" frees 33, mostly pointer member returns.

No body blocker read here is a defect candidate. The one candidate in reach
of this card's routes is Monkey's `DocumentRoot` overwrite in
`mk_vhost_set_property`. It is already SUP08's, stays refused (section 4),
and is not suppressed.

## 3. Rule 12f (task step 2)

[`legacy_scalar_return_value.rs`](../../../crates/qcore-c-gateway/src/legacy_scalar_return_value.rs)
adds one hook. It is asked last in the `return` lowering, and only for a
return every route refused.

**The reading.** By C11 6.8.6.4p3, `return E;` in a function with a scalar
result evaluates what `T tmp = E; return tmp;` evaluates. The statement
position decides that value with W43's typed value walker
(`typed_scalar_value`). When the walker admits `E`, the return lowers it as
the statement does: the walker's bases with their dereference obligations
and alias reads, no destination and no frame. The exit then escapes nothing.
The walker refuses a call, `&`, an assignment, an increment, a comma and a
pointer-valued name, and requires at least one typed access.

**Why `rd_clock` was refused.** A member `S.F` of an automatic aggregate
dereferences nothing. The typed reader still asks for a never-bound root on
every `.` path, because it also serves stores and pointer reads. That is why
`rd_clock`'s `return ts.tv_sec * … + ts.tv_nsec / …` was refused after
`clock_gettime(&ts)`, while the same value assigned in a statement was
admitted.

**What stays refused, by name:**

- a pointer result;
- a value naming a tracked descriptor or an opaque callee spelling;
- every value the walker refuses.

**Controls.**
[`tests/w48_bottom_up_callee_bodies.rs`](../../../crates/qcore-c-gateway/tests/w48_bottom_up_callee_bodies.rs)
loads the production libc modules:

- each accepting return is `Safe`, and so is its statement form;
- an `rd_clock`-shaped static helper's summary serves its caller;
- a read through a released base reports the use after release;
- an owned object dropped at a scalar return still leaks;
- a pointer result, a pointer converted to an integer and a value holding a
  call keep the return refusal (beside a control whose scalar member return
  is read).

`tests/unit/legacy_scalar_return_value.rs` binds the Rust `DECISION_VECTORS`
to the Lean `decisionVectors`.

**Deliberate pin update.** `tests/w48_callee_closure_r3.rs` pinned
`rd_clock` and `return ts.tv_sec` as `Unknown`, which was round 3's
finding. Both are now `Safe`, and the comments name rule 12f.

**Lean.** `QCore.ScalarReturnValue` proves:

- the walker (an automatic member carries no base; `rd_clock`'s value is
  admitted with none; a call, an address and a pointer are refused);
- definedness under the obligations (`admitted_value_is_defined`,
  `a_released_base_fails_its_obligation`);
- that the return and statement lowerings coincide, escape nothing and keep
  every owed site owed;
- the decision (`decide_iff`, `decision_vectors_hold`);
- `a_declined_return_is_never_safe`.

The audit depends only on `propext` and `Quot.sound`.

## 4. AFTER (task step 4)

The same commands ran at `97c4da505`. Every run exits 0. The attribution is
by [`scripts/attribute.py`](scripts/attribute.py)
([artifacts/after/](artifacts/after/)).

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus | 647 / 9 / 4,178 `dde4837e…` | **648 / 9 / 4,177** `bcc98d2d…` | `rdkafka_timer.c::rd_kafka_timer_next` Unknown → Safe (its one leaf, `call to uncontracted function rd_clock`) |
| system | 44 / 2 / 188 | identical (`660295ac…`, raw bytes) | none |
| slice | 29 / 1 / 238 / 2 Pending | identical (`218a6819…`, raw bytes) | none |

**Rounds.** Round 1 is same-unit, and it runs inside one arbiter run. The
hook admits 40 returns in the corpus, all of them `rd_clock`, `rd_uclock`
(each in every librdkafka unit that includes `rdtime.h`) and
`rd_kafka_broker_ApiVersion_supported0`'s `ret.MaxVer`.

W4 then validates the two clocks, and with them header helpers they alone
blocked. The helpers whose call leaves left the corpus are:

- `rd_timeout_init`, `rd_timeout_init_us`, `rd_timeout_remains`,
  `rd_timeout_remains_limit`;
- `rd_interval0`;
- `rd_kafka_buf_set_abs_timeout0`.

Their callers lose 112 call leaves (`rd_clock` 62 plus one nested,
`rd_timeout_init` 19, `rd_kafka_buf_set_abs_timeout0` 16,
`rd_timeout_remains` 7, `rd_uclock` 4, and one each to the other three).
`rd_kafka_timer_next` is freed.

Round 2 would be cross-unit, and it moves nothing. The AFTER plan
([plan-after.txt](artifacts/after/plan-after.txt)) has the same top bodies,
and the cross-unit closure frees 5 (10 with held calls), as before.

**Every changed row** (104, all Unknown apart from the one move):

- 90 only lose leaves: call leaves to the helpers above, plus a branch
  condition (`rd_kafka_wait_destroyed`) and the `ApiVersion_supported0`
  return.
- In 14 functions, 15 statements that held a call to a helper are now read
  further and refuse on the same line as an unresolved identity
  (`rkb`, `rk`, `rko`, `rkmce`, `timeout_msp`,
  `rd_kafka_yield_thread`). Before, they refused as an indirect access or a
  condition. This is refusal to refusal, and no leak or violation is added.

**Against the prediction:**

- Totals and the move were predicted exactly.
- The predicted list of helpers was right. It missed `rd_uclock`, a second
  header body with the same shape over `gettimeofday`.
- Five of the six scalar member returns named in the prediction
  (`state.err` ×3, `err_rtt.err` ×2) are not admitted: the walker reads
  only `S.F` with `F` a scalar member of a completely read record, and they
  stay refused. `ret.MaxVer` is admitted.

**The `mk_vhost_set` probe** (`tip-contracts.sh`, the card's binary;
[`scripts/probe_rows.py`](scripts/probe_rows.py)):

- at the tip, `mk_vhost_set` is Unknown on one leaf, `call to uncontracted
  function mk_vhost_set_property`;
- `mk_vhost_set_property` is Unknown on the two `DocumentRoot` statements
  (`vh->documentroot.data = mk_string_dup(v)`, an overwrite without a
  release, SUP08's candidate). Those stay refused;
- master keeps its `va_end` protocol refusals.

Rule 12f decides no return there. The base binary was not run on the probe,
so its rows before the card are main's as recorded by the earlier cards.

## 5. The carrier (task step 3)

The brief's route, a summary exported keyed by the home unit's digests and
applied across units under an external-linkage premise, was not built.

- **What the plan shows.** Everything this card frees is a header body that
  W4 derives in each including unit. The same-unit route carries it.
- **The measured value.** The route alone frees 5 corpus functions, and 6
  at its ceiling.

The earlier closures read callee names only from call leaves.
`held_calls.py` also reads the calls held inside return and condition
leaves, from their parity texts. For calibration, 12 position leaves still
refuse although every held call is a Safe same-unit body.

Counting held calls, the route frees 10:

- `cio_file_real_size`, `cmt_initialize`, `map_lock`,
  `cmt_metric_exp_hist_lock`, `cmt_metric_has_start_timestamp`;
- `ctxTypeID_to_size`, `LZ4_createStreamHC`;
- `rd_kafka_Uuid_hash`, `rd_kafka_msg_partitioner_fnv1a`, and then
  `rd_kafka_Uuid_map_hash`.

The route needs the following, none of which this card's yield required:

- a two-phase pinned run;
- an answer to rule 4a's interposition and C99 `inline` objections;
- certificate re-derivation of both halves;
- a Lean module.

`W44-CALLEE-CROSS-UNIT-SUMMARY` is amended with these sizes.

## 6. Not attempted, and why

The largest coherent shape left is
**`W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT`**, the admin accessors that
return `(const T **) rko->rko_u.admin_result.results.rl_elems` after
`rd_list_cnt(&rko->…)`. The fixtures show the refusal:

- without the call the shape is `Safe`;
- with it, the AST reader binds `rko` (it binds the root of every `&` path
  and every name in a call's actuals), and rule 10k/5b's pointer-root
  premise refuses the return.

Lifting it frees 24, an upper bound.

It was not lifted because a `Borrowed` contract does not say the callee
writes nothing through the pointer. A `Borrowed` `memcpy` could copy a
tracked pointer into the pointee, and the return would then hand it out
untracked. A read-only frame for the actual is the sound premise (rule
11e's `bounds_frame_reads`, declared and pinned).

The rest of the plan, with each body's yield, is registered as
`W48-BOTTOM-UP-RESIDUAL-PLAN`. It includes the decision-class bodies
(`cJSON_New_Item`, `sds_cat_safe`, the mpack unpackers), which this card
does not decide.

## 7. Follow-ups

The changes are applied by [`scripts/update_ledger.py`](scripts/update_ledger.py),
695 → 696.

**Retired:** `W48-RETURN-AGGREGATE-MEMBER-READ`. Every clause of its
discharge holds:

- the aggregate-value return read is admitted with the statement position's
  obligations;
- it has Lean parity;
- it has a measured re-pin;
- `rd_clock` was re-measured and validates.

**Amended:**

- `W44-CALLEE-CROSS-UNIT-SUMMARY`: re-measured as 5, 6 at its ceiling, 10
  with held calls; not built;
- `W44-CALLEE-BODY-DOES-NOT-VALIDATE`: 1,439 callees, 3,511 leaves, per-body
  and per-family leverage;
- `W45-CALLEE-CASCADE-RESIDUAL`: 2,525 functions with a call leaf, 489
  call-only, ceiling 425;
- `W46-HEADER-INLINE-DECLARED-UNCHECKED`: `rd_clock`'s checked route is
  complete.

**Registered:**

- `W48-BOTTOM-UP-RESIDUAL-PLAN`;
- `W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT`.

**Not changed:** `W44-CALLEE-DESTRUCTOR-SUMMARY`,
`W44-CALLEE-NO-CORPUS-BODY`, `W44-CALLEE-EXTERNAL-INLINE-DEFINITION`,
`CG05-ELABORATOR-FOOTPRINTLESS-CALLS`, `CG10-BOUND-ROOT-PATH-REFUSED` and
`CG23-EXIT-SPECIFIC-BLOCKED-HELPERS`.

- None of them is discharged.
- Their sizes move only by this card's 112 removed call leaves. Those leaves
  were class (iii), on header helpers with no reported row: 1,386 → 1,274
  external leaves in the AFTER ranking.

## 8. Gate

[`scripts/gate.sh`](scripts/gate.sh) ran the code steps at `0362c86e0` and
the docs steps at `b922a265a`. The exits are in [gate/](gate/) and the
figures in [gate.json](gate.json); the logs are under `private/w48bu/gate-g2`.
A first attempt (`gate-g1`) stopped at clippy (`type_complexity` on the
test-only decision table) and was stopped; `0362c86e0` names the row type.

**Standing gate:**

- check, submodule, fmt and clippy `--locked` exit 0;
- `run_test_binaries.py`: 435 units, 448 summaries, **7,501 passed, 1
  failed**, 11 ignored, exit 101
  (`private/test-binaries/20261002T145951Z-1399376`);
- the failure is `qcore-verifier-host`'s `execution_quota`
  (`the_same_fixture_without_a_declared_quota_installs_none`,
  `Io("Broken pipe")` from its spawned fixture), unrelated to this card.
  Constraint 16 was applied: the binary re-run alone passed 13 of 13
  (`04b`, exit 0). Nothing was weakened;
- both reevaluation probes exit 0;
- **282** Lean audits pass (281 + `AuditScalarReturnValue.lean`);
- Python: 365, with the expected `lean_audits` 281 → 282 failure before the
  matrix regeneration (`08`, exit 1), then OK after `build_matrix.py . --gate`
  (`09a`, `09b`, exit 0);
- the docs index exits 0 (`09`, `10b`, `10c`).

**Provider gate.** `providers/` is unchanged. The provider gate was run
anyway, because `build_matrix.py` merges only a gate with `provider_tests`:

- check, fmt, clippy and `contracts.py check` exit 0;
- `cargo test`: 129 passed, exit 0;
- the package build and all five verifies exit 0.

The AFTER binary was built at `97c4da505`. The later commits change no
lowering (the decision table, a type alias, one parity field), and the
corpus was not re-run at the final head.

## 9. Reproduction

```sh
docs/reports/w48-bottom-up-callee-bodies/scripts/measure.sh NAME [corpus,system,slice]
docs/reports/w48-bottom-up-callee-bodies/scripts/artifacts.sh BEFORE AFTER
docs/reports/w48-bottom-up-callee-bodies/scripts/bottom_up_plan.py DIAGNOSTICS OUT.json
docs/reports/w48-bottom-up-callee-bodies/scripts/family_leverage.py DIAGNOSTICS OUT.json
docs/reports/w48-bottom-up-callee-bodies/scripts/return_reasons.py DIAGNOSTICS PARITY.jsonl FAMILY.json OUT.json
docs/reports/w48-bottom-up-callee-bodies/scripts/held_calls.py DIAGNOSTICS PARITY.jsonl OUT.json
docs/reports/w48-bottom-up-callee-bodies/scripts/gate.sh NAME [code|docs|all]
docs/reports/w48-bottom-up-callee-bodies/scripts/finish-gate.sh NAME
```
