# W48-QR-SCALAR-TYPES-R5: typed loops and buffer-route arithmetic

The final positions of card 0 of the `.qr` language track, loop bounds first
(base `6883623c` = origin/main; the worktree's HEAD was already that commit, so
no reset or fast-forward was needed). It completes the brief's items **1**
(`W48-QR-TYPES-LOOP-BOUNDS`) and **2** (`W48-QR-BUFFER-ROUTE-ARITHMETIC`) and
stops there: item 3 (`W48-QR-TYPES-PAYLOADS`) and item 4
(`W48-QR-TYPES-EPOCH-TASKS`, `W48-QR-F02-FIXED-OUTCOMES`) keep their refusals
and their follow-ups. Card 0's decisions, output rule and evidence apply
unchanged, and the machinery of R3 and R4 is reused, not forked: the shared
decision `scalar_values::Decider`, the shared reader
`source_value_statement::parse_value_expression`, §G02t's `q_scalar_*` helpers,
the buffer route's typed environment. Grammar: implemented grammars
[§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
("Typed loops and the buffer route's value statement") and §G07.12;
semantics: specification §L14.0, §L20 and §L21; diagnostics rows `Q051`–`Q057`
and `Q097`–`Q099`. The prediction was frozen before any source change
([PREDICTION.md](PREDICTION.md)).

## Item 1: typed loop bounds and index

**Rules.** On `source.buffer.v1` a `for NAME in 0..BOUND` bound may be a value
of any integer type, and the index has the bound's type. The low end is the
literal `0`, which takes its context's type as every literal does, so a range
has **one** type: a mixed-type range is not expressible. A literal bound is an
`i32`, so a bound of another type is always a name of the lowering's constant
tier. A bound that is not known at check time stays the existing `Q098`
refusal; no step can overflow (below), so no loop needs the `ArithmeticError`
path.

**Mechanism.**

* **The constant tier** (`source_std_buffer::LoweringConstants`, the one place
  a bound is decided) gains the conversions and — item 2 — value statements
  over its own values, folded by the shared `Decider` under the recorded target,
  which the driver now threads into the lowering
  (`lower_source_buffer_for_target`). It was a literal, a literal `let` and an
  `extent`; a conversion or statement over a byte, a field or a copy is still
  outside it, as before. A derived name of `isize`/`usize` that could not be
  folded for want of a target is remembered, so its bound refuses for the
  width rather than as unknown.
* **The decision** is the core's `source_loop::typed_bound`, total over the
  table and in `LOOP-V1`'s order: `bool` (`BoundRefusal::NotAnInteger`), a
  target width with no record (`NeedsRecordedTarget`), negative, zero, above
  4096 (the existing refusals, their values widened to `i128` so `u64::MAX`
  and `i64::MIN` are named exactly), then `index_admits` (the count is a value
  of the index type). The three-layer budget is applied unchanged. An `i32`
  bound of the old tier takes exactly the path it always took.
* **The IR.** `BufferEvent::Repeat` gains `index: LoopIndex` — `I32`, every
  loop before the card, or `Typed { ty, bound }`. The checker
  (`BufferRun::repeat_typed`) re-checks `index_admits`, **re-derives the
  count** from the bound's value in its own typed environment
  (`BoundRefusal::BoundDisagrees` refuses a disagreement: the count is decided
  in the lowering and re-derived, not trusted), binds the index at its type per
  iteration, and scopes it out after the loop; the recorded target is the
  named assumption when the index is `isize`/`usize`.
* **Emission.** A typed loop is `for (T s_i = 0; s_i < s_BOUND; s_i += 1)` at
  the type, against the bound's own C object, so a native run compares the
  index against the number the emitted program computed — a check of the
  lowering's fold. An `I32` loop is byte-identical.
* **The composed string/buffer route** embeds this route's loop and lowers its
  body with this route's reader. It binds only `i32`s and its emitter keeps no
  conversion helper, so it now refuses the typed tier at any depth by name
  (`Q077`, `W48-QR-TYPES-COMPOSED-STRING`) — a correction, below.

**Why the loop is sound at every width** (`QCore.ScalarLoopBounds`). An
admitted count `c` satisfies `1 ≤ c ≤ 4096` and is a value of the index type,
whose range is an interval containing `0`; so every index value `0 … c-1` and
the final step's value `c` are values of the type
(`every_index_is_a_value`), the step is the checked addition and never faults
(`the_step_never_faults`), and the loop run with that checked step terminates
after **exactly `c` iterations**, visiting `0, 1, …, c-1`
(`the_loop_runs_exactly_count_times`, `the_count_is_the_iteration_count`). A
`u64` at or near its maximum is refused by the ceiling and a signed bound near
`MIN` by the sign rule before any index exists
(`u64_near_its_maximum_is_refused`, `signed_bounds_near_min_and_max_are_refused`);
`u8`'s maximum 255 is admitted and its last step reaches 255 without fault
(`u8_reaches_its_maximum_without_fault`); on `i32` the decision is exactly
`LOOP-V1`'s (`i32_is_loop_v1`).

## Item 2: the buffer route's value statement

**Rules.** `let NAME = A OP B;`, `A CMP B` or `-NAME` — one operation, the
grammar the extern and task routes read, by the same reader; its `as` form is
the route's R3 conversion, read as before. Every value on this route is known
at check time — the loop index once per iteration — so the shared decision
always folds: a fault refuses at `Q053` naming the variant the typed path would
raise, and nothing raises at run time (no `throws<ArithmeticError>` is needed or
read). Mixed types and `bool` arithmetic, negation and ordering refuse at
`Q052`, a literal outside its type at `Q053`, a target width with no record and
a chained expression at `Q051`. The result is a typed value: rendered,
converted, copied, a loop bound when its operands are of the constant tier,
and refused at an `i32` position unless it is an `i32`.

**Mechanism.** `BufferEvent::Compute { binding, operation }` with
`ValueOperation::{Binary, Compare, Negate}` over `ValueOperand::{Name, Integer,
Bool}`; `BufferEvaluation::compute` resolves the names in the typed environment
and calls `Decider::decide` with no permission; `TypedRefusal::Value` carries
the shared `ValueRefusal` at its code. A value statement derives no effect, is
not a `buffer_use` (it selects no route), names its operands for the lend
windows, and is body-local in a loop. The emitter calls §G02t's shared
`q_scalar_OP_T` / `q_scalar_neg_T` helpers — their `exit(86)` a witness the
checker proved never taken — and makes each comparison through a two-parameter
function (`qcore_buffer_lt_u8` …): an inline `x == x` is a `-Werror` failure
(`-Wtautological-compare`), found by this
card's native table and measured on the scalar route too
(`W48-QR-INLINE-COMPARISON-WERROR`).

**Lean** (`QCore.ScalarBufferValues`): a statement is admitted exactly when the
checked operation succeeds, in range and exact (`bufferBinary_iff_checked`,
`bufferBinary_in_range`, `bufferBinary_exact`); the decision never takes its
raising arm (`buffer_statement_never_raises`); a refusal is the classified raise
(`buffer_refusal_is_the_raise`); negation is `0 - a`
(`negate_is_zero_minus`, `unsigned_negation_only_at_zero`); inside a loop the
unrolled decision admits exactly when every index value succeeds
(`unrolled_admits_iff`), an admitted body statement never faults at run time
(`admitted_body_statement_never_faults`), and one faulting iteration refuses
the loop (`one_faulting_iteration_refuses_the_loop`, `i + 100` at `u8` over
`0..156` admitted, `0..200` refused).

## Positions

| Item | Position | Status |
|---|---|---|
| 1 | §G07.12 loop bound and index of every integer type on `source.buffer.v1` | **done**; `W48-QR-TYPES-LOOP-BOUNDS` retired |
| 2 | the value statement on `source.buffer.v1` | **done**; `W48-QR-BUFFER-ROUTE-ARITHMETIC` retired |
| 3 | `Buffer` and `protected N` payloads | refused, `W48-QR-TYPES-PAYLOADS` (open, amended with its measured size) |
| 4 | the composed epoch route's task half; a fixed-width outcome in the F02 alphabet | refused, `W48-QR-TYPES-EPOCH-TASKS`, `W48-QR-F02-FIXED-OUTCOMES` (open, unchanged) |
| — | the typed tier on the composed string/buffer route | refused by name (`Q077`), new `W48-QR-TYPES-COMPOSED-STRING` |
| — | a bound read out of a byte, a field or a copy | refused, as before (`Q098`) |

**Why the stop is after item 2.** Item 3 changes the owned Buffer's payload
representation shared by every task and mutex program: the payload is one
`int32_t` (`BUFFER_PAYLOAD_BYTES = 4`) and a range write lease's extent is its
four bytes, so a typed payload moves the lease footprint arithmetic, the
mutex invariant's cell (`mutex_invariant.rs`, 42 sites) and the `protected N`
declaration (`source_mutex.rs`, 27), with the F02-pinned runtime traces. It
did not fit after items 1 and 2 as a whole item, and a half-widened route is
not allowed; item 4 comes after it in the brief's order.

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| a conversion inside a `for` body on the composed string/buffer route (probe `p4`, untracked) | `check` 0, `emit` 0; the emitted C called `qcore_buffer_as_i32_u8`, never defined, and did not compile | `Q077`, refused by name | the route binds only `i32`s and its loop emitter keeps no helper; R3 recorded it as "lowers none", true only outside a loop body; no tracked file has the shape |
| `BoundRefusal::Negative { value }`, `TooLarge { value }` | `i32`, `i64` | `i128` | a typed bound's number at its own width; the `i32` path clamps exactly as before, so every message is unchanged |
| test constructions of `BufferEvent::Repeat` (`source_loop.rs` ×4, `loops_on_composed_routes.rs` ×3, `composed_window_reach.rs` ×5, `string_concat.rs` ×1) | three fields | `index: LoopIndex::I32` added | the new field; no behaviour change |

## Evidence

**Fixture identity.** The first half's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`681a37fe…`, 630 files)
and at the tip (`8abbb6e3…`) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**all 630 existing files are byte-identical** in the exit, stdout and stderr of
both `check` and `emit`; the one difference is the new example
`examples/scalar/typed_loops.qr` (check 0, emit 0). No existing refusal became
an admission and no admission a refusal among tracked files.

**Native** (`compiler/tests/scalar_loop_bounds.rs`, `scalar_buffer_values.rs`;
Clang 21.1.8, each program at `-O0` and `-O2` under
`-fsanitize=undefined,integer -fno-sanitize-recover=all` and at plain `-O2`,
all with `-Wall -Wextra -Werror -pedantic`; [summary](artifacts/native-boundary.json),
folded by [`summarize_native.py`](summarize_native.py)):

| Table | Per build | Result |
|---|---|---|
| every integer type's loop at 3 iterations and at `min(MAX, 4000)` (`i8` 127, `u8` 255, the rest 4000), the index rendered `dec` to stdout and `log info hex` to stderr | **20** programs, **64,824** lines | both streams byte-identical to an independent reference, exit 0, **0** sanitizer reports — the step at the top of `u8` and `i8` included |
| every admitted boundary case of the checker table (**1,245** values) and every comparison over every boundary pair (**1,950**), one program per type; an index-arithmetic loop per type (`*`, `/`, `%`, `+`, `<` on the index, 60 iterations) | **20** programs, **3,245** statement sites, **4,395** lines | every line equal to the reference, exit 0 — so no helper's `exit(86)` witness fired — **0** sanitizer reports |
| the published example `typed_loops.qr` | 1 program | the page's transcript, per stream |

**Checker table.** Every integer type × every pair of boundary values × the
five operations, and every negation: **1,245** admitted with the reference
value and **435** refused at `Q053` naming the reference variant (1,680 cases;
a signed type has seven boundary values and an unsigned one four). Every
comparison is the reference `bool`.

**Checker against the reference** (`compiler/tests/scalar_loop_bounds.rs`,
`scalar_buffer_values.rs`). Every type's loop at 1, 3 and 127 iterations renders
the index `dec` and `hex` at its width and names its bound in the C; the 32
Lean rows of `loopAgreement` replayed through `typed_bound`, and the 14 an
`i32` conversion reaches replayed through whole programs (admitted with the
row's count or refused at `Q098`); the checker's re-derivation refuses a count
the bound disagrees with, a count above a `u8` index and a `bool` index
(hand-built programs); the 17 rows of `bufferAgreement` replayed through whole
programs; the published example's transcript.

**Negative controls** (each refused, each a test row): a negative, zero,
above-ceiling, unlicensed and licence-violating typed bound; a `usize` and an
`isize` bound with no recorded target; a bound read out of a field and out of
a byte; a nested bound spelled through an outer typed index; a typed index at
an `i32` position (`Q052`); a conversion of the index that does not fit at the
iteration reaching it (`Q053`); a `bool` bound (`Q098`); mixed operand types,
`bool` arithmetic, ordering and negation, a literal outside its type, a
chained expression, a value-statement result at an index, a target width with
no record, the builtin `print(…)`; `i + 100` at `u8` over `0..157`; a
conversion and a typed loop on the composed string route (`Q077`).

**Lean** ([`AuditScalarLoopBounds.lean`](../../../proof/AuditScalarLoopBounds.lean),
17 theorems; [`AuditScalarBufferValues.lean`](../../../proof/AuditScalarBufferValues.lean),
11 theorems; no placeholder, no project axiom), named above. The binding to
Rust is the vector tables and the native tables, not an extraction theorem.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixtures AFTER vs BEFORE (630) | byte-identical except new examples, 80% | **byte-identical**; the new example only |
| existing tests changed | only `Repeat` constructions and pins of a widened refusal, 65% | **only the `Repeat` constructions** (13 sites, a new field); no pinned refusal moved — R3's typed-bound row reads a field, still outside the tier |
| native tables under UBSan | all agree, 0 reports, 80% | **all agree, 0 reports** (after the comparison form below) |
| Lean | new audit file, 85% | **two** audit files, **28** theorems |
| corpus arbiter | byte-identical `ae17152b…`, 97% | **byte-identical** to the base's own archive, `7393fd21…` (CG03's AFTER; `ae17152b…` was superseded on main by the Clang-gap cards) |
| standing gate | green, new tests only, 65% | **green** on the first run, the provider suite included; +16 tests |
| items completed | 1 (85%), 2 (60%), 3 (15%), 4 (5%) | **1 and 2**; 3 and 4 not attempted |

Not predicted: the first native run of the value table failed to build —
an inline comparison of a name with itself is a `-Werror` failure — which is
why comparisons go through functions, and the same shape on the scalar route
was then measured at the base (`W48-QR-INLINE-COMPARISON-WERROR`); an `i32`
loop bound named only by the loop emits an unused C variable at the base
(`W48-QR-LOOP-BOUND-NAME-UNUSED-C`), which the typed loops avoid by comparing
against the bound's C object; and a typed bound whose `isize`/`usize` number
the lowering cannot fold without a target needed its own remembered state, so
it refuses for the width rather than as unknown.

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 376 units, 389 summaries; **6,681 passed, 0 failed, 11 ignored** (`private/test-binaries/20260929T190052Z-1819959/summary.json`); no load-sensitive rerun was needed |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **236 audits** (234 + `AuditScalarLoopBounds.lean`, `AuditScalarBufferValues.lean`) |
| provider tests | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **129 passed, 0 failed, 1 ignored** (43 summaries) — run as the brief asks; `providers/` is unchanged |
| provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | |
| provider clippy | `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qr-scalar-types-r5/gate.json` | 0 | after the ledger, 630 → 631 |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix regeneration) |
| clean | `cargo clean --profile dev` | 0 | |

Exits in [gate/](gate/), record in [gate.json](gate.json); logs under
`private/w48-r5/gate/`. The gate ran at `e96cc3de`; the commits after it change
one doc comment in `std_buffer_c_emit.rs`, the ledger's wording and records
only. This card's two test files add 16 tests; the base's full count was not
remeasured here. Clang 21.1.8 recorded in this worktree
([gate/clang-version.txt](gate/clang-version.txt)). The AFTER fixture freeze was
taken with the tip's debug `quirin` (`8abbb6e3…`).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
card's release `quirin-audit` (`df228714…`, built from the sources of
`cf902a63`; later commits touch tests, comments and records only): exit 0.
`resource-diagnostics.json` sha256 `7393fd21cf2d5f3f…` is **byte-identical** to
the base's own archive — CG03-CALL-SUMMARY-INSTANTIATION's AFTER on main (564 /
10 / 4,260), which superseded the brief's `ae17152b…` (the w47-round9 archive)
when the Clang-gap cards moved corpus figures on main — and so are
`resource-diagnostics.chunk-contracts.json` (`cbbcfd9c…`) and
`monkey-source.json` (`57acfc58…`) ([identity](artifacts/arbiter-corpus.identity)).
No C-audit path was touched.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py)): retires `W48-QR-TYPES-LOOP-BOUNDS` and
`W48-QR-BUFFER-ROUTE-ARITHMETIC`; registers `W48-QR-TYPES-COMPOSED-STRING`
(the typed tier on the composed string route), `W48-QR-LOOP-BOUND-NAME-UNUSED-C`
(measured at the base: `let n = 3; for i in 0..n { … }` with `n` named nowhere
else emits C that `-Werror` refuses, an unused `s_n`; the typed loop names its
bound and has no such defect) and `W48-QR-INLINE-COMPARISON-WERROR` (measured at
the base: `let c = x == x;` on the scalar route emits C that `-Werror` refuses);
amends `W48-QR-TYPES-PAYLOADS`, `W48-QR-TYPES-MSVC-NATIVE` and
`W48-QR-VALUE-STATEMENT-EXPRESSIONS`. 630 → 631.

## Files likely to conflict with cards 1–4

`crates/qcore-core/src/std_buffer.rs` (`LoopIndex`, `ValueOperand`,
`ValueOperation`, `BufferEvent::{Repeat, Compute}`, `BufferEvaluation::compute`,
`BufferRun::repeat_typed`, `TypedRefusal::Value`), `std_buffer_c_emit.rs` (the
typed loop header, `value_statement_expression`, the comparison functions,
`RecordEnvironment::{helpers, comparisons}`), `source_loop.rs` (`typed_bound`,
`index_admits`, the widened `BoundRefusal`), `std_string.rs` and
`std_string_c_emit.rs` (the `Q077` refusal and the new arms);
`crates/qcore-frontend/src/source_std_buffer.rs` (`LoweringConstants`, the
value-statement branch of `lower_let`, `lower_source_buffer_for_target`),
`source_loop.rs` (`BoundValue`, `read_loop_header`'s signature);
`crates/qcore-verification/src/driver.rs` (the buffer route's lowering takes the
target); implemented grammars §G02t/§G07.12, specification §L14.0/§L20/§L21,
`docs/agents/diagnostics.md`, `docs/agents/buffers-records.md`;
`proof/QCore.lean`. Card 2 (usable records) meets this card in the buffer
route's typed environment and value statement.

## Scripts

* The first half's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
* [`summarize_native.py`](summarize_native.py) — folds the native summaries:
  `QCORE_W48_R5_NATIVE_SUMMARY=../DIR cargo test -p qcore-compiler --test
  scalar_loop_bounds --test scalar_buffer_values`, then
  `summarize_native.py DIR artifacts/native-boundary.json`.
