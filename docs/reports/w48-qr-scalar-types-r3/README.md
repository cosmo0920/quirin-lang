# W48-QR-SCALAR-TYPES-R3: typed record fields and extern values

The remaining positions of card 0 of the `.qr` language track (base
`d3508e10`, one records-only commit past the brief's `535c327f`). It completes
the brief's items **1** (`W48-QR-TYPES-RECORD-FIELDS`) and **2**
(`W48-QR-TYPES-EXTERN-VALUES` with `W48-INERT-EXTERN-DECLARATION-TYPES`) and
stops there: items 3–5 keep their refusals and their follow-ups. Card 0's
decisions, output rule and evidence requirements apply unchanged, and its
machinery is reused, not forked: `scalar_types.rs` (ranges, `convert`,
`render`, `c_rendering`, now also the shared `c_conversion_test` and
`c_target_assertions`), the scalar route's helpers (`q_scalar_*`,
`q_scalar_try_*`, now public through `scalar_calls_c_emit::Helper`),
`ArithmeticError` and its `80 + tag` exit, `--recorded-target`, and the
first half's `freeze_fixtures.py`/`compare_fixtures.py`. Grammar: implemented
grammars [§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
(the new "buffer route's typed environment", "extern results as values" and
"every extern declaration is read" paragraphs), §G07.13, §G02k, §G02d, §G02i
and §G02; semantics: specification §L14.0, §L20 and §L21. The prediction was
frozen before any source change ([PREDICTION.md](PREDICTION.md)).

## Item 1: typed record fields and the buffer route's typed environment

**Grammar.** `field = NAME ":" (SCALAR | "bytes")`, construct-then-read, as
before; a field value is a literal, `true`/`false` at a `bool` field, or a
bound name.

**Mechanism.**

* `source_record.rs` (core): `FieldType::Scalar(ScalarType) | Bytes`
  (`FieldType::I32` is kept as an associated constant), `FieldValue::Typed
  { ty, value }` beside the `i32` `FieldValue::Scalar`, so one type has one
  representation and `construct` checks type equality; new refusals
  `FieldTakesAType` (`Q052`), `FieldLiteralOutOfRange` (`Q053`),
  `FieldNeedsRecordedTarget` (`Q051`).
* `std_buffer.rs`: the value environment is **typed**. `BufferEvaluation::scalars`
  stays the `i32` map — the one every composed route reads, since they bind
  only `i32`s and refuse the record and conversion statements that are the only
  sources of another type — and `BufferEvaluation::typed` holds every other
  type; a name is in at most one, and `scalar_value` is the typing function.
  `operand` (every `i32` position: index, stored byte, lend bound, field
  index) refuses a name of another type (`TypedRefusal::NotAnI32`, `Q052`); a
  field value is checked against its field's type; a rendering reads any type
  and renders it with `scalar_types::render`, an `i32` taking exactly the
  path it always took; a copy bind keeps its type; loop scoping and shadowing
  cover both maps. The route's one new statement is `BufferEvent::Convert`
  (`let NAME = OPERAND as TYPE;`, not a `buffer_use`, so it selects no route),
  decided at check time because every value on this route is a check-time
  constant (`ConversionOutOfRange` `Q053`, `NoConversion` `Q052`,
  `NoRecordedTarget` `Q051`). `check_buffer_program_for_target` and
  `recheck_buffer_certificate_for_target` take the builder's recorded target;
  the certificate publishes `typed_scalars`, `typed_projections`,
  `RecordConstruction::typed` and `target_assumption` (present exactly when a
  field or conversion names `isize`/`usize`), all rechecked.
* `std_buffer_c_emit.rs`: each member at its type's C11 spelling (`int32_t`
  for `i32`, unchanged), a construction's typed literals as the type's C
  constant, a typed field read and a copy at their type, a conversion as a plain
  cast when total and otherwise through `qcore_buffer_as_FROM_TO`, whose test is
  the shared `c_conversion_test` and which **aborts** — this route's witness
  discipline, the checker having proved it never taken; `<stdbool.h>` and
  `<stddef.h>` and the target's `_Static_assert`s only for a program naming a
  non-`i32` type; a paragraph appended to the record prelude only for a module
  declaring such a field, so every module of `i32` records emits the bytes it
  always did.
* `source_std_buffer.rs` (frontend): the declared record types are installed in
  `LoweringConstants` so a construction reads each field value at its declared
  type (an `i32` field with the route's own `i32` reader, as before); the
  conversion statement's reader. `driver.rs` passes the recorded target and
  publishes the buffer certificate's target assumption.

**Positions.** Widened: record fields of every §G02t type; every scalar
position of the route typed; the conversion statement. The route has **no
arithmetic** and never had an expression, so none was added; that capability
is registered (`W48-QR-BUFFER-ROUTE-ARITHMETIC`). A loop bound from a field
read or a conversion stays the lowering's existing `BOUND-FROM-FIELD` refusal
(the loop bound position is item 4).

## Item 2: extern results as values, and every extern declaration read

**Mechanism.**

* `extern_call.rs`: the value namespace is typed (`AbiType::scalar`); a bound
  result has its declared result's type, a literal binding is an `i32`, and
  `ExternEvent::Compute(ValueStatement)` performs one §G02t operation
  (`ValueOperation::{Binary, Compare, Negate, Convert}`) over `ValueOperand::
  {Value, Integer, Bool}` — a literal typed by the other operand, `i32` when
  both are literals. `ValueTyping` decides each statement: over **known**
  values (a literal, a declared outcome, a statement over known values) it
  folds with `checked_error`/`compare`/`negate_error`/`convert_error` and a
  fault refuses (`ValueRefusal::StaticFault`, naming the variant); over a
  run-time value an operation that can fault (arithmetic, negation, a
  conversion that is not total) needs `ExternProgram::raises` — the row's
  `throws<ArithmeticError>` — else `NeedsPermission`. A declared outcome is now
  an `i128` range-checked against the result's type (`OutcomeOutOfRange`), and
  an argument at a declared scalar parameter must be a value of exactly that
  type (`ArgumentType`) or a literal in its range (`LiteralOutOfRange`). All are
  `ExternDiagnostic::Value` at `Q024`, the namespace's existing code. The
  certificate publishes `value_types`, `computations` (type, operand type,
  known number, whether it can raise) and `target_assumption`.
* `extern_c_emit.rs`: a statement over known values calls the shared checked
  helper (`q_scalar_OP_T`, whose `exit(86)` the checker proved never taken); one
  that can raise declares its value, calls `q_scalar_try_OP_T`, and on a nonzero
  tag logs `raise ArithmeticError` and returns `80 + tag`; a comparison and a
  total conversion are plain C. `scalar_fragments` contributes the headers, the
  width assertions (also binding the ABI's `intptr_t`/`uintptr_t` to
  `ptrdiff_t`/`size_t`) and each helper once — nothing for a program without a
  statement. An outcome outside `int32_t` is spelled as its type's C constant;
  every other outcome as before. A value statement is observed as nothing in
  the call trace; a run that raises exits `80 + tag` after the lines before it,
  the partial-correctness reading the route's `exit 70` already has.
* The composed extern + io route (`source_composed.rs`, `composed_std_io.rs`,
  `std_io.rs`, `std_io_c_emit.rs`, `composed_std_io_c_emit.rs`): a rendering
  reads a **known** value of every §G02t type at that type (before, an `i32`
  only) through `StdIoProgram::typed_scalars`, checked, re-interpreted and
  emitted with §G02t's rendering over the extern variable cast to the type's C
  spelling; an undetermined value still refuses (`Q043`), a target-width value
  needs the record (`Q043`), and the entry's one row may carry
  `throws<ArithmeticError>`. `source_extern.rs` parses the value statements,
  the widened outcomes and the row permission; `driver.rs` threads the target.
* **Every extern declaration is read** (`driver.rs`,
  `check_extern_declaration_signatures`): the route dispatch moved unchanged
  into `compile_elaborated_source`, and after whichever route admits an
  executable or library module, every `extern fn` declaration's parameters and
  result are read against the §C04.1 vocabulary (scalars, the three custody
  pointers, `unit` as a result); anything else refuses at `Q020`. A contract
  module is exempt: its declarations are the contract surface, in the loader's
  wider vocabulary (`f64`, quantity types), read by the contract loader —
  checking them here moved 41 provider `.qv` files in a first measurement,
  which is why the exemption is stated rather than assumed.

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| `docs/reports/remaining-v04/probes/P08-valid-named-use.qr` | check/emit 0 | check/emit 1, `Q020 parameter \`q\` of \`extern fn f\` is \`quant<n where n := 5>\`` | the uncalled declaration is now read; recorded beside the frozen report in [`remaining-v04/CORRECTIONS.md`](../remaining-v04/CORRECTIONS.md) |
| `docs/reports/remaining-v04/probes/P10-decl-after-use-site.qr` | check/emit 0 | check/emit 1, the same `Q020` | the same |
| `compiler/tests/quant_index_contracts.rs` (2 tests), `logical_connectives.rs` (1) | compiled a module with an uncalled `quant` extern declaration | assert its elaboration and that the one refusal is this `Q020` | the P08/P10 shape |
| `compiler/tests/std_string.rs::the_contextual_len_rule_still_leaves_a_contract_parameter_resolvable` | admitted an unbraced clause list parsed into the result annotation | asserts no `Q013` and the one `Q020`; the braced `contract { … }` form is admitted on the sequential route | the unread annotation is now read |
| `compiler/tests/unsafe_linkage.rs::every_admitted_scalar_crosses_…` | passed `2` at a `bool` parameter | passes `1` | a literal at a declared parameter is range-checked (an implicit integer-to-`bool` conversion before) |
| `compiler/tests/scalar_types.rs::a_conversion_on_another_route_stays_refused` | listed two buffer-route conversions as refused | they moved to `a_conversion_statement_is_admitted_on_the_buffer_route` | the deliberate widening of item 1 |
| `compiler/tests/std_io_compose.rs::rendering_a_wider_scalar_refuses_at_q043` | a declared `i64` rendered refused at `Q043` | renamed `…_renders_it_at_its_own_type`: `out: 21`, `out: 0x0000000000000015` | the deliberate widening of item 2's composed rendering |
| `source_record.rs`/`owned_field_route_reach.rs` field-type tests | used `u8`/`i64` as unmodelled field types | use `f64`, `f32`, `char` | the deliberate widening of item 1 |

Test constructions of `BufferEvent::Construct` and `ExternProgram` were
adapted to the new field and fields (`.into()`, `raises`, `target`) with no
behaviour change.

## Evidence

**Fixture identity.** `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`9342a7c9…`, 627 files)
and at the tip ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**625 of the 627 existing files are byte-identical** in the exit, stdout and
stderr of both `check` and `emit`; the two differences are the corrections
P08 and P10; the one new file is the example `examples/scalar/record_fields.qr`
(check 0, emit 0). Intermediate freezes after item 1 alone (627/627
identical), after the declaration read, and after item 2's argument and
outcome checks moved nothing else.

**Native boundary tables** (Clang 21.1.8, each program built at `-O0` and `-O2`
under `-fsanitize=undefined,integer -fno-sanitize-recover=all` and at plain
`-O2` with `-Wall -Wextra -Werror -pedantic`; [summary](artifacts/native-boundary.json),
folded by [`summarize_native.py`](summarize_native.py)):

| Position | Per build | Result |
|---|---|---|
| record fields of every type (boundary values, `dec` + `hex`, `print` + `log`) | 1 program, 112 lines | every stream equals an independent reference; exit 0 |
| the buffer-route conversion over every integer pair and boundary value | 10 programs, 371 admitted conversions (`dec` + `hex`) | equal to the reference; the 0-exit run means no witness fired |
| extern values, known path (declared outcomes, every operation and boundary pair, comparisons), composed route | 10 programs, 2,475 lines (1,210 admitted operations, 55 comparisons) | equal to the reference; the 415 faulting cases are refused at check time with the reference's variant |
| extern values, dynamic path (undeclared results under `throws<ArithmeticError>`: every operation, negation, every conversion) | 150 binaries, 2,175 runs: **1,561** reference values, **614** typed raises | every value checked by a callee, every raise exits `80 + tag` with the reference's variant, **0** exits 86, **0** sanitizer reports, no output |

The dynamic table is card 0's 2,175 cases (1,561 values, 614 faults), case for
case, now reached from extern results linked against real C callees.

**Checker against the reference.** Every conversion case is admitted exactly
when the reference says it fits and refused at `Q053` otherwise; every field
literal at both ends of every type likewise (`Q053`, or the `i32` reader's
`Q051`); every known extern operation is admitted with the reference value or
refused at `Q024` with the reference variant.

**Negative controls** (each refused, each a test row): a field literal out of
range at both ends, a number at a `bool` field and `true` at an integer field,
a name of another type at a field in both directions, a `u8` at an index, a
stored byte, a byte read's index, a lend bound and a loop bound, `hex` of a
`bool`, a conversion out of range, from a literal, and to or from `bool`,
`isize`/`usize` fields and conversions without a record; on the extern route
mixed operand types, a literal outside its operand's type, a declared outcome
outside its result, `bool` arithmetic, ordering and conversion, a run-time
operation and a non-total run-time conversion without the permission, a value
and a literal of another type at a declared parameter, an undetermined value
rendered, `hex` of a `bool` rendered, a `usize` statement without a record;
extern declarations with `Widget`, `own<Widget>` and `quant<…>` annotations on
the sequential and buffer routes.

**Lean** (`QCore.ScalarPositions`, [`AuditScalarPositions.lean`](../../../proof/AuditScalarPositions.lean),
22 theorems, no placeholder, no project axiom): item 1 —
`fieldValue_lit_iff` (a field literal is admitted exactly when it is a value of
the field's type), `fieldValue_name_same_type` (no implicit conversion at a
field), `fieldValue_bool_iff`, `fieldValue_in_range`, `construct_well_typed`,
`project_well_typed` (a field read binds a well-typed value),
`i32Operand_type` and `i32Operand_refuses_other_types`,
`convertStmt_well_typed` and `convertStmt_succeeds_iff`, `no_target_no_field`,
`fieldAgreement_holds` (29 vectors by `decide`, replayed through the checker);
item 2 — `decideBinary_known_iff`, **`decideBinary_refusal_is_raise`** (a
statement refused over known operands is one whose typed path raises the
classified variant), **`decideBinary_raises_needs_permission`** and
`decideBinary_without_permission`, `decideBinary_known_in_range`,
`argumentAdmitted_type`, `decideCompare_bool_is_bool`, `externAgreement_holds`
(10 vectors, replayed through the checker). It builds on `QCore.ScalarTypes`
and `QCore.ScalarTypedErrors`. The binding to Rust is the vector tables and the
boundary tables, not an extraction theorem.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| item 1 moves no tracked fixture | 85% | **none** (627/627 identical after item 1) |
| item 1 existing tests changed | only the conversion-route test, 70% | that test **and** two field-type tests that used `u8`/`i64` as unmodelled types |
| the declaration read moves exactly P08 and P10 | 65% | **exactly P08 and P10** among `.qr` files — after exempting contract modules, whose first measurement moved 41 `.qv` files |
| native tables under UBSan | agree, 0 reports, 80% | **all agree, 0 reports** (table above) |
| Lean | 85% | **227 audits** (22 new theorems) |
| corpus arbiter | byte-identical `ae17152b…`, 97% | **byte-identical** (`ae17152b…`) |
| standing gate | green, new tests only, 80% | **green** after one test correction; 6,445 → 6,465 |
| items completed | 1 (85%), 2 (45%), 3 (25%), 4 (20%), 5 (10%) | **1 and 2**; 3–5 not attempted |

Not predicted: four more tests pinned the unread-declaration shape
(`quant_index_contracts.rs` ×2, `logical_connectives.rs`, `std_string.rs`),
the extern route passed a literal at a declared parameter unchecked
(`unsafe_linkage.rs`'s `2` at `bool`), now refused, and one composed test
pinned the `i32`-only rendering (`std_io_compose.rs`), found by the gate's
first run.

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | (rerun after the test correction) |
| tests | `python3 scripts/run_test_binaries.py` | first 101, rerun 0 | first run 363 units, 6,464 passed, **1 failed**: `std_io_compose::rendering_a_wider_scalar_refuses_at_q043` pinned the `i32`-only composed rendering this card widened (corrected above); rerun 363 units, 376 summaries, **6,465 passed, 0 failed, 11 ignored** (`private/test-binaries/20260929T120147Z-1393506/summary.json`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | (rerun after the test correction) |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **227 audits** (226 + `AuditScalarPositions.lean`) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix regeneration) |
| clean | `cargo clean --profile dev` | 0 | |

Exits in [gate/](gate/) (the first test run's as `03-tests.first.exit`),
record in [gate.json](gate.json); logs under `private/w48-r3/gate/`. The test
count rose 6,445 → 6,465 (+20: the two new test files' 19 tests and the moved
buffer-route conversion test). `providers/` is unchanged, so the provider gate
is not due. Clang 21.1.8 recorded in this worktree
([gate/clang-version.txt](gate/clang-version.txt)). The AFTER fixture freeze
was taken with the tip's debug `quirin` (`00689550…`).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
card's release `quirin-audit` (`c940ff49…`, built at `28ffe82f`; later
commits touch tests and records only): exit 0. `resource-diagnostics.json`
sha256 `ae17152b05516be7…` is **byte-identical** to the archived baseline, and
so are `resource-diagnostics.chunk-contracts.json` (`cbbcfd9c…`) and
`monkey-source.json` (`57acfc58…`) ([identity](artifacts/arbiter-corpus.identity)).
No C-audit path was touched, as predicted.

## Positions and follow-ups

| Item | Position | Status |
|---|---|---|
| 1 | §G07.13 record fields; the buffer route's value environment | **done**; `W48-QR-TYPES-RECORD-FIELDS` retired |
| 2 | extern results as values, the §G02i operands; uncalled extern declarations | **done**; `W48-QR-TYPES-EXTERN-VALUES` and `W48-INERT-EXTERN-DECLARATION-TYPES` retired |
| 3 | task workers, `copy` captures, §G02j operands | refused, `W48-QR-TYPES-TASK-CAPTURES` (open, amended with its size) |
| 4 | §G07.12 loop bounds | refused, `W48-QR-TYPES-LOOP-BOUNDS` (open, amended) |
| 5 | `Buffer` and `protected N` payloads | refused, `W48-QR-TYPES-PAYLOADS` (open) |

**Why the stop is after item 2.** Item 3 is the task route's own typed IR —
worker parameters, results, locals and captures in `task_execution.rs`,
`scoped_tasks.rs`, `task_c_emit.rs` and the composed tasks + io route — and the
task route has no arithmetic or conversion statement in a worker at all, so a
worker's `ArithmeticError` first needs a value statement there. That is a whole
route's worth of change; a half-widened route is not allowed, and items 4 and 5
come after it in the brief's priority order, so every position of items 3–5
keeps its existing refusal.

Ledger ([followups.json](followups.json), applied by
[`apply_ledger.py`](apply_ledger.py)): retires `W48-QR-TYPES-RECORD-FIELDS`,
`W48-QR-TYPES-EXTERN-VALUES` and `W48-INERT-EXTERN-DECLARATION-TYPES`;
registers `W48-QR-BUFFER-ROUTE-ARITHMETIC`, `W48-QR-VALUE-STATEMENT-EXPRESSIONS`,
`W48-QR-EXTERN-VALUE-BRANCHES` and `W48-QR-EXTERN-TARGET-WIDTH-OUTCOME`; amends
`W48-QR-TYPES-MSVC-NATIVE`, `W48-QR-TYPES-LOOP-BOUNDS` and
`W48-QR-TYPES-TASK-CAPTURES`. 620 → 621.

## Files likely to conflict with cards 1–4

`crates/qcore-core/src/std_buffer.rs` (typed environment, `Convert`,
`FieldOperand`, certificate fields), `std_buffer_c_emit.rs`, `source_record.rs`
(`FieldType::Scalar`, `FieldValue::Typed`), `extern_call.rs` (typed value
namespace, `ValueStatement`, certificate fields), `extern_c_emit.rs`,
`std_io.rs`/`std_io_c_emit.rs` (`typed_scalars`), `composed_std_io.rs` and its
emitter, `scalar_calls_c_emit.rs` (public `Helper`), `scalar_types.rs`;
`crates/qcore-frontend/src/source_std_buffer.rs`, `source_extern.rs`,
`source_composed.rs`, `source_std_io.rs` (`ComposedValues`);
`crates/qcore-verification/src/driver.rs` (the route dispatch is now
`compile_elaborated_source`, followed by the extern-declaration read);
implemented grammars §G02, §G02t, §G02d, §G02k, §G07.13 and specification
§L14.0/§L20/§L21; `docs/agents/source-routes.md` (constraint 14) and
`buffers-records.md` (15k); `proof/QCore.lean`.

## Scripts

* The first half's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`summarize_native.py`](summarize_native.py) — folds the native summaries:
  `QCORE_W48_R3_NATIVE_SUMMARY=../DIR cargo test -p qcore-compiler --test
  scalar_positions --test scalar_extern_values`, then
  `summarize_native.py DIR artifacts/native-boundary.json`.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json` to the matrix.
