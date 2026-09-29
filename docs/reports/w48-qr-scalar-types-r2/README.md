# W48-QR-SCALAR-TYPES-R2: unread annotations and the typed arithmetic error

The second half of card 0 of the `.qr` language track (base `7e723a96`). It
completes the brief's items **0** (`W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES`)
and **1** (`W48-QR-TYPED-ARITHMETIC-ERROR`) and stops there: items 2–6 keep
their refusals and their follow-ups (below). Card 0's decisions, output rule and
evidence requirements apply unchanged; the first half's report is
[w48-qr-scalar-types](../w48-qr-scalar-types/README.md), whose machinery
(`scalar_types.rs`, the renderings, `--recorded-target`, the freeze and
boundary-table scripts) is reused, not forked. Grammar: implemented grammars
[§G02](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02-source-execution-profile),
[§G02s](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)
and [§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types);
semantics: specification §L14.0 and §L21. The prediction was frozen before any
source change ([PREDICTION.md](PREDICTION.md)).

## Item 0: every route checks every annotation it admits

**Measured gap.** A helper appended to every admitted `.qr` fixture under
`examples/`, `compiler/tests/fixtures/` and `docs/reports/` (169 fixtures, ten
helper shapes) found three behaviours at the base:

* the **sequential route** (49 fixtures) read no parameter annotation —
  `x: Widget`, `f: fn() -> i32`, `x: own<Widget>`, `x: u8`, and `x: usize` with
  no recorded target were all admitted — while it did check results and locals;
* six **entry-only routes** (47 fixtures: `source.std-io.v1`, `source.buffer.v1`
  with its record and loop forms, `source.string.v1`, `source.socket.v1`,
  `source.http.v1`, `source.event.v1`) admitted any function other than the entry
  **unread** unless it carried one of the route's own constructs: its parameter,
  result and local annotations (`-> Widget`, `let q: usize = 7;`) and its body,
  which only name resolution saw;
* every other route checks or refuses the helper (`Q020`, `Q500`, `Q600`,
  `Q026`, `Q401`). The extern and composed extern + io routes, which need a
  contract selection and so were probed separately through the compiler API,
  already refuse any helper at `Q026` ("models exactly one entry function") —
  the precedent the entry-only fix follows.

**Mechanism** (`crates/qcore-verification/src/driver.rs`):

* `check_sequential_parameters` runs after the sequential route's own passes:
  a parameter's type must be one of the §G02t table, `isize`/`usize` only under
  a recorded target; anything else refuses at `Q020`, naming the parameter and
  the written type. The route supplies no argument to any parameter (a direct
  call names a zero-parameter function and the entry has none), so no custody
  form was ever read either; `own<T>`/`borrow<T>` parameters refuse too. The
  sequential tail of `compile_files_with_registry_inner` moved unchanged into
  `compile_sequential_source` so the check can follow it.
* `refuse_unmodeled_functions` widens each entry-only route's existing
  "models the entry only" condition, at that route's own code (`Q030`, `Q051`,
  `Q058`, `Q068`, `Q080`, `Q090`), to every function other than the entry.
* Both run on an **admitted** module only, so every refusal given before keeps
  its bytes; only an admission can become a refusal. A module with `std.io`
  lines, scalar-call syntax and a helper now reaches the scalar route (the
  first half's door gains `has_unmodeled_function`), which checks every
  function; no fixture takes that door.

**Corrections (admission → refusal).** Exactly two tracked files move, both the
`remaining-v03` probes that recorded this gap as the named boundary B-1:

| File | Before | After |
|---|---|---|
| `docs/reports/remaining-v03/probes/S13_unknown_parameter_type.qr` | check 0, emit 0 | check/emit 1: `Q020 parameter \`x\` is \`Widget\`: …` |
| `docs/reports/remaining-v03/probes/X14_function_typed_param_helper.qr` | check 0, emit 0 | check/emit 1: `Q020 parameter \`f\` is \`fn() -> i32\`: …` |

Two tests pinned the old admissions and are corrected with them:
`compiler/tests/std_string.rs::a_length_query_without_the_import_refuses` (its
module carried an unread `helper()`; it now asserts `Q068` for it and measures
the import gate on the entry alone) and
`compiler/tests/scalar_types.rs::modules_the_std_io_route_admits_keep_that_route`
(an std.io entry beside `fn h(x: i32) -> i32 { return x + 1; }`, admitted by
`source.std-io.v1` unread; it now reaches `source.scalar-calls.v1`, which checks
`h`). After the change the probe finds **0** fixtures admitting an unread
`Widget`/`fn`/target-less `usize` parameter (88 before) and **0** admitting an
unread result or local annotation (47 before); a `u8` or `i32` parameter on the
sequential route is still admitted because it is now checked.

**Not covered: uncalled extern declarations.** `extern fn f(x: Widget) -> Widget;`
beside a sequential entry is still admitted with its signature unread. Two
frozen probes rely on that shape (`remaining-v04/probes/P08`, `P10`, which
declare `quant<n where n := 5>` parameters to measure quantity-name
resolution), so fixing it moves them and changes what they measure; it is
registered as `W48-INERT-EXTERN-DECLARATION-TYPES` rather than folded in.

## Item 1: the typed arithmetic error

**Shape (as proposed by the first half).** A builtin, payload-free
`ArithmeticError { Overflow, DivisionByZero, OutOfRange }`
(`qcore_core::scalar_types::ArithmeticError`): `Overflow` for an exact result
outside the type and for `MIN / -1`, `MIN % -1`; `DivisionByZero` for a zero
divisor; `OutOfRange` for a conversion that does not fit. No existing error type
fits — the task profile's `SpawnError`/`AllocError` name resource failures — so
one was added, spelled like them.

**Semantics on `source.scalar-calls.v1`**, the task profile's discipline:

* A function raises it only when its row lists `throws<ArithmeticError>`
  (never inferred; `effects { io, throws<ArithmeticError> }` in either order).
* In such a function an operation whose operands are not all literals raises
  its fault's variant — at check time in the closed evaluation, and at run time
  in the emitted C. A fault the checker decides from literals alone (the first
  half's fold) is **still refused** whatever the row permits, and a function
  without the permission keeps the closed evaluation's refusal: the message and
  the emitted C of such a function are byte-identical to the first half's.
* A call of a function that may raise must be consumed: `CALL?` propagates it
  unchanged under the caller's own permission (`CALL?;` for a `unit` callee), or
  `match CALL { Result::Ok { value: v } => { … } Result::Err { error: e } => { … } }`
  runs exactly one arm; `raise e;` re-raises and `raise ArithmeticError::VARIANT;`
  raises explicitly, both under the permission. Every block ends in `return`,
  `raise` or a `match`, so arms never fall through (`W48-QR-TYPED-ERROR-CONTINUING-ARMS`).
* An entry may declare the permission: a propagated error is its published
  outcome (`ExecutionOutcome::Error { name: "ArithmeticError::Overflow", tag: 1 }`)
  and the emitted program exits `80 + tag` (81, 82, 83) after the lines it wrote
  — distinct from the untyped fault status 86 and the extern route's 70.

**Mechanism.** `scalar_calls.rs`: the IR's function body became a `Block`
(`statements` + `End::{Return, Raise, Match}`), `Expr::Try`, error slots and
scoped local slots; the checker is a per-function `FunctionCheck` over blocks;
the closed evaluator returns `Outcome::{Value, Raised}` and turns a fault into
the classified variant only in a permitted function (`st::checked_error`,
`negate_error`, `convert_error`). `scalar_calls_c_emit.rs`: a permitted
function returns `int` (the tag) and writes its value through `q_result`; every
operation in it — `i32` included — calls a `q_scalar_try_*` helper that tests
the operands first and returns the tag, and the function returns it at once;
`match` is an `if`/`else` on the callee's tag. `source_scalar_calls.rs` reads
the row, `?`, blocks, arms and `raise`; `frontend.rs` skips `ArithmeticError`
before `::` in name resolution (contextual; the census,
[`census_error_name.py`](census_error_name.py), finds the word in no tracked
source file but this card's example and the qualified shape in none), so no
word is reserved. `driver.rs` publishes the entry's error outcome.

**Diagnostics.** No code allocated: every typed-error refusal is a further
condition of `Q020` (unconsumed call, `?`/`raise` without the permission, `?`
or `match` on a callee that cannot raise, arms other than one `Ok` and one
`Err`, a `unit` scrutinee, another error name, an unknown variant); an unbound
`raise NAME` and an escaped arm binding are name resolution's own `Q013`.

## Evidence

**Fixture identity.** The first half's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`169e3920…`, 626 files)
and at the tip (`1c102d36…`, 627 files) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**624 of the 626 existing files are byte-identical** in the exit, stdout and
stderr of both `check` and `emit`; the two differences are the corrections
above (S13, X14), and the one new file is the example
`examples/scalar/typed_errors.qr` (check 0, emit 0). The intermediate freezes
after item 0 alone and after item 1 showed item 1 moves nothing.

**Native boundary table on the dynamic path** (`compiler/tests/scalar_typed_errors.rs`,
[summary](artifacts/native-dynamic-boundary.json)). Per integer type, one
program of **throwing** functions (the five operations, negation, a conversion
to every other integer type), called on every boundary case (MIN, MIN+1, −1, 0,
1, MAX−1, MAX) by a harness that compares the returned tag and value with an
independent classifying reference inside the program; Clang 21.1.8 at `-O0` and
`-O2` under `-fsanitize=undefined,integer -fno-sanitize-recover=all` and at
plain `-O2`: **2,175 cases × 3 builds; per build 1,561 reference values and 614 raises (325 `Overflow`, 110 `DivisionByZero`, 179 `OutOfRange`), every raise the reference's variant, 0 exits 86, 0 sanitizer reports, 0 bytes on either stream**. These are the first half's 1,561 values and 614 faults, case for case, now returned as typed errors. The harness also asserts no exiting helper
(`q_scalar_add_…`, `q_scalar_neg_…`) is emitted in a throwing function.

**Checker on the dynamic path.** The same 1,625 operation cases as closed
programs whose operation takes parameters through `?`: each is admitted with
the reference's rendered value or raises exactly the reference's variant as
the entry's outcome; the literal-only form of each case, under the permission,
is refused exactly when it faults — the executable instance of
`static_fault_is_dynamic_raise`. Negation and every conversion likewise.

**End to end.** Seven programs (handled Err arm, Ok arm, overflow and division
by zero and out-of-range propagated out of the entry, `MIN / -1` in `i32` on the
typed path, re-raise after a line) built under the three builds: every stream
equals the reference per stream and every exit status is `0` or `80 + tag` as
the reference outcome says. The example's page transcript is checked the same
way.

**Negative controls** (each refused): an unconsumed call (bound and bare), `?`
without the permission (absent and `io`-only rows), `raise` without it, `?` and
`match` on a callee that cannot raise, two `Ok` arms, `throws<AllocError>`, a
`unit` match scrutinee, a constant fault under the permission (`255 + 1` at
`u8`, `7 / 0`, `300 as u8`), a closed-evaluation fault without the permission,
an unbound `raise` name, an unknown variant, an arm binding used outside its arm.

**Lean** (`QCore.ScalarTypedErrors`, [`AuditScalarTypedErrors.lean`](../../../proof/AuditScalarTypedErrors.lean),
20 theorems, no placeholder, no project axiom): `checkedE_ok_iff` and
`checkedE_raises_iff` (the typed path returns exactly `checked`'s value and
raises exactly when it faults), `checkedE_raises_classified`,
`checkedE_in_range`, `checkedE_exact`, `division_by_zero_raises`,
`min_div_minus_one_raises_overflow`, `convertE_raises_out_of_range`,
`convertE_ok_iff_fits`, `total_conversion_never_raises`, `evalE_agrees` (the
typed dynamic evaluation agrees with the first half's fault semantics on every
expression and environment), **`static_fault_is_dynamic_raise`**,
`static_value_is_dynamic_value` and **`static_fault_variant`** (the dynamic path
agrees with the static refusal, naming the same variant), `propagate_raise`,
`propagate_value`, `handle_raise`, `handle_value`, `handled_never_raises`, and
`typed_agreement_holds` (19 vectors by `decide`, replayed against
`st::checked_error` by `lean_typed_error_vectors_match_the_rust_kernel`). The
binding to Rust is the vector table and the boundary tables, not an extraction
theorem.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixtures AFTER vs BEFORE | exactly S13, X14 move, 80% | **exactly S13 and X14**; the new example only after |
| item 0 moves no entry-only-route fixture | 90% | **none** |
| item 1 moves no fixture | 90% | **none** (the item-0 and item-1 freezes are identical) |
| existing tests changed for item 0 | a few sequential ones, 60% | **two**, neither sequential: a string-route and an std.io-route test pinned an unread helper |
| native table under UBSan | typed error, 0 reports, no 86, 85% | **2,175 × 3: all agree, 614 typed raises per build, 0 exit 86, 0 reports** |
| Lean | new theorems audit, 85% | **226 audits pass** (20 new theorems) |
| corpus arbiter | byte-identical `ae17152b…`, 97% | **byte-identical** (`ae17152b…`) |
| standing gate | green, new tests only, 85% | **green** after one clippy fix; 6,433 → 6,445 (+12, the two new test files) |
| items completed | 0 and 1; 2 attempted, 60% | **0 and 1**; item 2 not attempted (below) |

Not predicted: the six entry-only routes admitted helpers unread in their
result and local annotations and bodies, not only parameters — which widened
item 0 from the sequential route to those six; and uncalled extern
declarations are a separate unread position.

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | (rerun after the clippy fix) |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 361 units, 374 summaries; **6,445 passed, 0 failed, 11 ignored** (`private/test-binaries/20260929T084824Z-191140/summary.json`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | first run 101 (`type_complexity` on one test-local tuple type in `scalar_typed_errors.rs`); fixed with a type alias, rerun 0, and that test binary rerun: 8 passed |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **226 audits** (225 + `AuditScalarTypedErrors.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix regeneration) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| clean | `cargo clean --profile dev` | 0 | |

Exits in [gate/](gate/), record in [gate.json](gate.json); logs under
`private/w48-r2/gate/`. `providers/` is unchanged, so the provider gate is not
due. Clang 21.1.8 recorded in this worktree ([gate/clang-version.txt](gate/clang-version.txt)).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
tip's release `quirin-audit` (`17127660…`, built at `fea82fdc`): exit 0.
`resource-diagnostics.json` sha256 `ae17152b05516be7…` is **byte-identical** to
the archived baseline, and so are `resource-diagnostics.chunk-contracts.json`
(`cbbcfd9c…`) and `monkey-source.json` (`57acfc58…`)
([identity](artifacts/arbiter-corpus.identity)). No C-audit path was touched.

## Positions and follow-ups

| Item | Position | Status |
|---|---|---|
| 0 | parameter annotations on the sequential route; functions the entry-only routes do not model | **done**; `W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES` retired |
| 1 | the typed arithmetic error through `throws<E>`, `?`, `match … Result::Err` | **done** on `source.scalar-calls.v1`; `W48-QR-TYPED-ARITHMETIC-ERROR` retired |
| 2 | §G07.13 record fields | refused, `W48-QR-TYPES-RECORD-FIELDS` (open, size unchanged) |
| 3 | extern results as values, §G02i operands | refused, `W48-QR-TYPES-EXTERN-VALUES` (open; the typed error now exists for its undeclared results) |
| 4 | task workers, `copy` captures, §G02j operands | refused, `W48-QR-TYPES-TASK-CAPTURES` (open) |
| 5 | §G07.12 loop bounds | refused, `W48-QR-TYPES-LOOP-BOUNDS` (open) |
| 6 | `Buffer` and `protected N` payloads | refused, `W48-QR-TYPES-PAYLOADS` (open) |

**Why the stop is after item 1.** Item 2 widens the record fields of
`source.buffer.v1`, whose scalar namespace is an untyped `i32` map read by every
buffer, record, loop and line position of that route (`std_buffer.rs`,
`source_std_buffer.rs`, `std_buffer_c_emit.rs`); a typed field value has to be
typed at each of them or refused at each, which is a whole route's worth of
change. It did not fit after items 0 and 1, and a half-widened route is not
allowed, so every position of items 2–6 keeps its existing refusal.

Ledger ([followups.json](followups.json)): retires
`W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES` and `W48-QR-TYPED-ARITHMETIC-ERROR`;
registers `W48-INERT-EXTERN-DECLARATION-TYPES`,
`W48-QR-TYPED-ERROR-CONTINUING-ARMS` and `W48-QR-ERROR-VARIANT-INSPECTION`;
amends `W48-QR-TYPES-MSVC-NATIVE` (the try-helpers and the `80 + tag` exit are
not built under MSVC either). 588 → 589.

## Files likely to conflict with cards 1–4

`crates/qcore-core/src/scalar_calls.rs` (the IR is now block-structured),
`scalar_calls_c_emit.rs`, `scalar_types.rs`; `crates/qcore-frontend/src/source_scalar_calls.rs`
and `frontend.rs` (`lexical_references`); `crates/qcore-verification/src/driver.rs`
(the sequential tail is now `compile_sequential_source`, and eight route calls
are wrapped by `refuse_unmodeled_functions`); implemented grammars §G02, §G02s,
§G02t; specification §L14.0, §L21; `docs/agents/source-routes.md` (constraint
14); `proof/QCore.lean`.

## Scripts

* The first half's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`census_error_name.py`](census_error_name.py) — the constraint-14 census.
* The native summary: `QCORE_W48_R2_NATIVE_SUMMARY=native-dynamic-boundary.json
  cargo test -p qcore-compiler --test scalar_typed_errors native_dynamic` (written
  under `compiler/`, then moved to `artifacts/`).
