# W48-QR-SCALAR-TYPES-R4: fixed-width values across a task boundary

The last positions of card 0 of the `.qr` language track (base `0691a1cc` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). It completes the brief's item **1**
(`W48-QR-TYPES-TASK-CAPTURES`) and stops there: items 2
(`W48-QR-TYPES-LOOP-BOUNDS`) and 3 (`W48-QR-TYPES-PAYLOADS`) keep their
refusals, and `W48-QR-BUFFER-ROUTE-ARITHMETIC` stays open (below). Card 0's
decisions, output rule and evidence requirements apply unchanged, and the
earlier halves' machinery is reused, not forked — the extern route's value
statement is now shared rather than copied. Grammar: implemented grammars
[§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
(the new "Task workers and `copy` captures" paragraph), §G02a and §G02j;
semantics: specification §L14.0 ("The table across a task boundary"), §L20 and
§L21; diagnostics rows `Q020`, `Q050`, `Q300`, `Q600`. The prediction was
frozen before any source change ([PREDICTION.md](PREDICTION.md)).

## Mechanism

**Shared, not forked.** The extern route's value-statement decision moved out
of `extern_call.rs`, unchanged, into `crates/qcore-core/src/scalar_values.rs`
(`Decider::decide`, `ValueRefusal`, `raisable`); `extern_call.rs` resolves its
operands and calls it. Its reader moved out of `source_extern.rs`, unchanged in
what it accepts and in every message, into
`crates/qcore-frontend/src/source_value_statement.rs`
(`parse_value_expression`, generic over the caller's namespace). The task route
uses both, so one value statement has one reader and one decision on the two
routes that have it. The extern route's fixtures and tests are byte-identical
(below).

**The task IR** (`task_execution.rs`): `ScalarType::Fixed(ty)` for the ten
types other than `i32` (which keeps its variant, so one type has one
representation; `Fixed(i32)` is refused as malformed), typed literals
`ScalarOperand::Fixed`, `TaskOutcome::Fixed` in the ledger's capsule, and the
value statement in two forms: `Compute` (an instruction; it cannot raise) and
`TryCompute` (a terminator with a success edge and an error edge binding the
classified `ArithmeticError::VARIANT` in an error slot, exactly as `TryOutcome`
binds a joined error). The checker tracks, per path, the number of each slot
known at check time and re-decides every statement with the shared decision:
the wrong form for what the decision says refuses, an operation over known
values that faults refuses, and a raising operation needs the function's
`throws<ArithmeticError>`. Types and literal ranges are validated for the
whole function before any path is explored; `isize`/`usize` need
`TaskModule::target`. `CheckedTaskModule` records one type per value slot
(a disagreement between paths refuses) and publishes the recorded target as
the named assumption exactly when the module names `isize` or `usize`.

**The error edge is an exit.** The lowering makes `TryCompute`'s error block
the route's ordinary error exit — the scope exits, then `raise` of the bound
error — so a raise is checked by the existing exit rules: a function that holds an
owned Buffer across a raising statement is refused (`Q600`, tested), as is
anything else those rules account for at an exit. No ledger rule changed: custody,
handles, spawn failure and "no automatic join" are exactly as they were.

**`ArithmeticError`** is a builtin, payload-free error type of the task route
beside `SpawnError` and `AllocError` (its three variants as tags), and like them
may not be redeclared (`Q013`). A worker's `ArithmeticError` is an ordinary
typed worker error: the worker's summary carries its declared error
alternatives, `join` collects the outcome, and the creator consumes it with `?`
(under its own permission) or `match`.

**The reference execution** (`task_execution/reference.rs`) runs each
statement with §G02t's own `checked_error`/`compare`/`negate_error`/
`convert_error`, and a `TryCompute` fault binds the variant and takes the error
edge. A fixed-width outcome renders `QCORE_TASK_RESULT TASK TYPE NUMBER`
(`true`/`false` for `bool`).

**Emission** (`task_c_emit.rs`): in a module that names a non-`i32` type the
task value gains `union qcore_task_typed { … }` with one member per type the
module names, positional initializers name it explicitly, a value statement
calls §G02t's shared `q_scalar_*` helper (a witness the checker proved never
fires) or `q_scalar_try_*` helper (its tag mapped onto the task route's error
tag, then the error edge), a fixed-width result logs under the name `=TYPE`
(an unsigned number in the event's `related`, a signed one in `value`, both
lossless), and `<stdbool.h>`/`<stddef.h>` and the target's `_Static_assert`s
are added. The maintained stdout sink gains the matching arm in
`diagnostic_stdout_adapter_definition_with_fixed_results`, installed only in a
program with a fixed-width worker result; the historical adapter keeps its
bytes because the two F02 C digests pin them (a first gate-less run of the
suite moved them, and the harness was split rather than the pins).

**Frontend** (`source_tasks.rs`): parameter, result and local types read the
§G02t table; a literal after a fixed-width annotation or in a fixed-width
worker's `return` takes that type, range-checked; `true`/`false`; the value
statement read by the shared reader and decided by the shared decision, with
the frontend's own known-value tracking so its refusals carry the source codes
(`Q020` typing and static faults, `Q300` the missing permission, `Q500` a
statement of more than one operation); `copy` captures of any scalar; the
§G02j composed operands of any type (`Q050` for `hex` of a `bool`); the
recorded target. `transfer.rs` gains `TransferKind::FixedScalar`, duplicated
(`QCore.Transferability` grows by the same constructor, 17 x 6 x 17 decided
whole). The driver threads the recorded target to the task route and publishes
its assumption. The composed epoch route, which lowers its task half through
the same route with no target, keeps refusing a module using any of this, by
name (`W48-QR-TYPES-EPOCH-TASKS`).

## Positions

| Item | Position | Status |
|---|---|---|
| 1 | §G02a worker parameters and results, local annotations, `copy` captures of every scalar; value statements in every task function with the typed `ArithmeticError` through join; §G02j operands in the parent and in workers | **done**; `W48-QR-TYPES-TASK-CAPTURES` retired |
| 2 | §G07.12 loop bound and index | refused, `W48-QR-TYPES-LOOP-BOUNDS` (open, amended with its measured size) |
| 3 | `Buffer` and `protected N` payloads | refused, `W48-QR-TYPES-PAYLOADS` (open, amended) |
| — | `W48-QR-BUFFER-ROUTE-ARITHMETIC` | open: it did not fall out of item 1 — the buffer route has its own IR and certificate — but its reader and decision are now the shared modules above |
| — | the composed epoch route's task half; a fixed-width outcome in the F02 alphabet | refused, new `W48-QR-TYPES-EPOCH-TASKS`, `W48-QR-F02-FIXED-OUTCOMES` |
| — | a task entry's result other than `i32`/`unit` | refused, card 0's rule (`Q020`) |

**Why the stop is after item 1.** Item 2 needs the buffer route's lowering to
carry the check-time number of a conversion result or a typed field read —
both decided today only in the core — before a loop header could read a typed
bound, plus a typed `BufferEvent::Repeat` and a typed C `for`; item 3 changes
the owned Buffer's payload representation shared by every task and mutex
program. Neither fitted after item 1, and a half-widened route is not allowed,
so both keep their refusals.

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| `crates/qcore-frontend/tests/source_tasks.rs::whole_statement_consumption_rejects_ignored_suffixes_and_multiple_tasks` | listed `let y = x + 1; return y;` (over the entry's literal `x`) as refused | that row is now `let y = x + 1 + 2;` (still refused, one operation per statement); the widened shape is asserted admitted as a folded `Compute`, and the same statement over a worker's parameter refused without `throws<ArithmeticError>` | the deliberate widening of item 1 |
| `crates/qcore-core/tests/unit/transfer.rs` (5 pinned tables) | 16 kinds; 6 admitted cells of 96 and of 1,536 | 17 kinds; 7 admitted cells of 102 and of 1,734 (`FixedScalar, copy` → `FixedScalar`) | the new duplicated kind, deliberately |
| `proof/QCore/Transferability.lean` | `kindInventoryHasSixteenEntries`, the five decided tables, `duplicationCarriesNoCustody = [.i32]` | `kindInventoryHasSeventeenEntries` (audit renamed), the tables with the new kind, `= [.i32, .fixedScalar]` | Lean parity with the Rust judgment |
| an untracked task module declaring its own `error ArithmeticError { … }` | admitted (probe `p3`) | `Q013`, reserved | `ArithmeticError` is now the route's builtin; the constraint-14 census ([r2's script](../w48-qr-scalar-types-r2/census_error_name.py), rerun: 630 files) finds the word only in the two card-0 examples, so no tracked file moves |

Test constructions of `TaskModule` gained `target: None` with no behaviour
change.

## Evidence

**Fixture identity.** The first half's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`fe2acd81…`, 629 files)
and at the tip ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**all 629 existing files are byte-identical** in the exit, stdout and stderr of
both `check` and `emit`, the extern route's included; the one difference is the
new example `examples/scalar/typed_workers.qr` (check 0, emit 0). No existing
refusal became an admission and no admission a refusal.

**Native** (`compiler/tests/scalar_tasks.rs`; Clang 21.1.8, each program at
`-O0` and `-O2` under `-fsanitize=undefined,integer -fno-sanitize-recover=all`
and at plain `-O2`, all with `-Wall -Wextra -Werror -pedantic`, on the real C11
thread backend; [summary](artifacts/native-boundary.json)):

| Table | Per build | Result |
|---|---|---|
| worker boundary table: every integer type, the five operations, negation and every conversion over MIN, MIN+1, −1, 0, 1, MAX−1, MAX — each case one worker computing on its two `copy` captures, joined by the entry | **2,175** cases in **226** programs: **1,561** reference values, **614** typed raises through join (**325** `Overflow`, **110** `DivisionByZero`, **179** `OutOfRange`) | every stream byte-identical to the reference execution; every worker outcome the trace prints equal to an independent reference, value for value and variant for variant; root result 0 everywhere; **0** sanitizer reports |
| end to end | a worker's `Overflow` propagated out of the entry by `?`; handled by `match`; every type (MIN and MAX, `bool` both) rendered by the parent `dec`/`hex` and logged; the spawn-failure control (a failing `QCORE_TASK_CREATE`) | stdout and stderr equal the reference |
| worker lines | every type rendered by the worker that received it (`log info dec`, `log warn hex`) | stdout byte-identical; the workers' stderr admitted by the declared reordering relation (`compare_stream`) |

The table is card 0's 2,175 cases, case for case, now computed inside worker
threads on values that crossed a `copy` capture. Each value case's program
chains its cases through the expected `match` arm; raising cases run five to a
program, since each raising join's checked error alternatives multiply the
paths the checker explores.

**Checker and reference.** Every type crosses as a capture and returns through
join (reference trace `QCORE_TASK_RESULT 1 U8 255`, …); the named assumption is
present exactly for `isize`/`usize`; known statements fold to `Compute` with
exiting helpers only, run-time ones to `TryCompute` with try-helpers only; the
Lean vectors replay through the task route's reference run
(`lean_task_vectors_match_the_reference_run`); every raise of the boundary
table is on its statement's error edge (`every_boundary_raise_is_on_its_error_edge`).

**Negative controls** (each refused, each a test row): mixed-type arithmetic
and comparison, a return and a capture of another type, literals out of range
at both ends (`let`, `return`), check-time faults (`255 + 1` at `u8`, `7 / 0`,
`300 as u8`) whatever the row permits, run-time arithmetic, negation and a
non-total conversion without `throws<ArithmeticError>`, `bool` arithmetic,
ordering and conversion, `true` at a `u8`, a chained expression, a `u8` entry
result, `usize` without a recorded target, a redeclared `ArithmeticError`,
`hex` of a `bool`, the builtin `print(…)`, and a raising statement while a
Buffer is held.

**Lean** (`QCore.ScalarTasks`, [`AuditScalarTasks.lean`](../../../proof/AuditScalarTasks.lean),
15 theorems, no placeholder, no project axiom; building on `QCore.ScalarTypes`,
`QCore.ScalarTypedErrors`, `QCore.ScalarPositions`, `QCore.Transferability`):
`capture_admitted_iff`, `capture_refuses_another_type`,
`capture_preserves_value`, `fixedScalar_crosses_only_by_copy`,
`captured_operand_raises_under_permission`,
`captured_operand_refused_without_permission`, `raised_variant_is_raisable`,
`conversion_raise_is_raisable`, `join_returns_the_completion`,
`join_consumes_the_completion`, `join_leaves_other_tasks`,
**`worker_error_reaches_creator_unchanged`** (the joined outcome is the very
variant the worker raised; `?` raises it out of the creator; `match` runs the
`Err` arm with it), `worker_conversion_error_reaches_creator_unchanged`,
`worker_value_reaches_creator` (well-typed and exact), `taskAgreement_holds`
(10 vectors by `decide`). `QCore.Transferability` gains the kind and its
decided tables. The binding to Rust is the vector tables and the native
boundary tables, not an extraction theorem.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixtures AFTER vs BEFORE (629) | byte-identical, 80% | **byte-identical**; the new example only |
| existing tests changed | only pins of a widened refusal, 60% | **one** frontend row (the widened entry arithmetic) and the transfer pins |
| transferability pins | counts 16 → 17 only, 75% | **as predicted** (five Rust tables, the Lean module and one audit name) |
| native tables under UBSan | all agree, 0 reports, 75% | **2,175 × 3 through threads, all agree, 614 raises through join, 0 reports** |
| Lean | 80% | **229 audits** (15 new theorems) |
| corpus arbiter | byte-identical `ae17152b…`, 97% | **byte-identical** (`ae17152b…`) |
| standing gate | green, new tests only, 70% | **green** after one load-sensitive rerun and one clippy fix; 6,581 → 6,594 (+13, the new test file) |
| items completed | 1 (70%), 2 (20%), 3 (5%), buffer arithmetic (10%) | **1**; 2, 3 and the buffer arithmetic not done |

Not predicted: the first full-suite run moved the two F02-pinned C digests,
because those tests instrument their C with the shared harness sink, which
this card had widened; the sink was split (`instrument` keeps the historical
adapter, `instrument_fixed` adds the arm) instead of re-pinning. And the
checker's per-join error alternatives multiply the paths of a program that
chains raising joins, which shaped the native table (raising cases five per
program).

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | (rerun after the clippy fix: 0) |
| tests | `python3 scripts/run_test_binaries.py` | first 101 | 370 units, 383 summaries; **6,593 passed, 1 failed, 11 ignored** (`private/test-binaries/20260929T140022Z-2844237/summary.json`); the failure is constraint 16's load-sensitive `cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (`elapsed < 500ms`) under a load average of ~40 from other sessions: rerun alone while the gate's clippy step still ran, 19/20 again (exit 101), and once after it, **20/20** (exit 0) — **6,594** pass |
| clippy | `cargo clippy --all-targets -- -D warnings` | first 101, rerun 0 | first run: dead code on the shared sink's historical `instrument()` in the one test binary that uses only the fixed-width variant, and `type_complexity` on a test-local tuple; fixed (`#[allow(dead_code)]`, a type alias), `scalar_tasks` rerun 13 passed |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **229 audits** (228 + `AuditScalarTasks.lean`) |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qr-scalar-types-r4/gate.json` | 0 | after the ledger, 622 → 623 |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix regeneration) |
| clean | `cargo clean --profile dev` | 0 | |

Exits in [gate/](gate/) (first runs as `*.first.exit`, reruns as
`*.rerun*.exit`), record in [gate.json](gate.json); logs under
`private/w48-r4/gate/`. The test count rose 6,581 (the R3 integration's) →
6,594: `scalar_tasks.rs`'s 13 tests. `providers/` is unchanged, so the provider
gate is not due. Clang 21.1.8 recorded in this worktree
([gate/clang-version.txt](gate/clang-version.txt)). The AFTER fixture freeze
was taken with the tip's debug `quirin` (`3070b9cd…`).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
card's release `quirin-audit` (`916292d7…`, built at `3d4d9945`; later commits
touch tests and records only): exit 0. `resource-diagnostics.json` sha256
`ae17152b05516be7…` is **byte-identical** to the archived baseline, and so are
`resource-diagnostics.chunk-contracts.json` (`cbbcfd9c…`) and
`monkey-source.json` (`57acfc58…`) ([identity](artifacts/arbiter-corpus.identity)).
No C-audit path was touched.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py)): retires `W48-QR-TYPES-TASK-CAPTURES`;
registers `W48-QR-TYPES-EPOCH-TASKS` and `W48-QR-F02-FIXED-OUTCOMES`; amends
`W48-QR-TYPES-LOOP-BOUNDS`, `W48-QR-TYPES-PAYLOADS`,
`W48-QR-BUFFER-ROUTE-ARITHMETIC`, `W48-QR-TYPES-MSVC-NATIVE`,
`W48-QR-VALUE-STATEMENT-EXPRESSIONS` and `W48-QR-ERROR-VARIANT-INSPECTION`.
622 → 623.

## Files likely to conflict with cards 1–4

`crates/qcore-core/src/task_execution.rs` (IR types, the checker's known map,
slot types, `validate_fixed_width`), `task_execution/reference.rs`,
`task_c_emit.rs` (the typed runtime, `self_value`, `source_lines`),
`scoped_tasks.rs` (`TaskOutcome::Fixed`), `composed_tasks.rs`,
`task_trace_relation.rs`, `transfer.rs` (`FixedScalar`),
`diagnostic_log_c_emit.rs`, the new `scalar_values.rs` and `extern_call.rs`
(its `ValueTyping` now delegates); `crates/qcore-frontend/src/source_tasks.rs`
(the `Scalar` enum, `Builder::value_statement`, the error table),
`source_extern.rs` and the new `source_value_statement.rs`,
`source_epoch_tasks.rs`; `crates/qcore-verification/src/driver.rs`
(`task_route` takes the target); `tests/support/diagnostic_sink.rs`; implemented
grammars §G02t/§G02a/§G02j and specification §L14.0/§L20/§L21;
`docs/agents/diagnostics.md`; `proof/QCore.lean`,
`proof/QCore/Transferability.lean`. Card 3 (task record captures) builds on the
task IR's value types and the transfer kind.

## Scripts

* The first half's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
* The native summary: `QCORE_W48_R4_NATIVE_SUMMARY=PATH cargo test -p
  qcore-compiler --test scalar_tasks native_worker_boundary_table`.
