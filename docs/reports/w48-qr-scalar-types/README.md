# W48-QR-SCALAR-TYPES: fixed-width scalar types in `.qr` sources

Card 0 of the `.qr` language track (base `d389700a`). The §G05 `scalar_abi`
table — `bool`, `i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`, `u32`, `u64`,
`usize` — is now a set of first-class value types on `source.scalar-calls.v1`,
rendered through `std.fmt`/`std.io`/`std.logger`. Grammar: implemented
grammars [§G02s](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-pure-scalar-functions-sourcescalar-callsv1)
and the new [§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types);
semantics: language specification §L14.0, §L20 and §L21. The prediction was
frozen before implementation ([PREDICTION.md](PREDICTION.md)).

## Mechanism

* `crates/qcore-core/src/scalar_types.rs` (new, route-independent): the table,
  value ranges, checked `+ - * / %` and negation, the one conversion,
  comparisons, `std.fmt` rendering, the C11 spellings, and the recorded target
  and its total-or-reject loader. Values travel as `i128`.
* `crates/qcore-core/src/scalar_calls.rs`: the route's IR gains types,
  `bool` literals, comparisons, `as` and the line statements; the checker is a
  bidirectional typing pass (a literal takes its context's type, default `i32`)
  producing a typed IR with folded constants, and the closed evaluator runs
  that IR.
* `crates/qcore-core/src/scalar_calls_c_emit.rs`: typed locals, temporaries and
  signatures; one helper per used (operation, type) and fallible conversion,
  each testing its operands against the type's limits before computing; the
  `i32` spellings unchanged. `size_t`/`ptrdiff_t` programs carry
  `_Static_assert`s on the recorded width.
* `crates/qcore-frontend/src/source_scalar_calls.rs`: the parser (types,
  comparisons, `as`, `true`/`false`, line statements read by
  `source_std_io::std_io_statement_tokens`, the std.io route's own reader).
  `frontend.rs`: a scalar type name after the reserved `as` is not a reference.
* `crates/qcore-verification/src/driver.rs`: the recorded target is threaded
  through `*_for_target` entry points to the route; the std.io/scalar route
  guard; `CompiledModule::target_assumption`. `crates/qcorec`: `quirin
  --recorded-target FILE` and the assumption line.
* `proof/QCore/ScalarTypes.lean` + `proof/AuditScalarTypes.lean` (23 theorems).

## Decisions as implemented

| Decision | Implementation |
|---|---|
| Checked overflow | Every operation of every type computes the exact result and faults outside the type, on a zero divisor, and on `MIN / -1`, `MIN % -1`. On this route every operand of an executable entry is a check-time constant of its closed evaluation, and every literal-only subexpression of every function is folded while typed, so each fault is **refused at check time** (`Q020`, "`T` arithmetic overflow or division by zero"). The emitted C still tests the operands before every operation (explicit range tests, no `__builtin_*`, so `cl` can build it) and exits 86 on a fault — the route's established fault status, which no admitted closed entry reaches. **Typed error:** no widened position has an operand undetermined at check time, so none needs the `throws<E>` form and no error type was added; that form is registered for the first positions that will (`W48-QR-TYPED-ARITHMETIC-ERROR`), not built unused. |
| Explicit conversions | One spelling, `EXPR as TYPE`: total (a plain cast) when every source value fits the target, else checked — the same number or the fault, never a truncation. Mixed-type operands, and a binding/argument/result of another type, refuse. **Census** (`census_spelling.py`, 625 tracked `.qr`/`.qv`/`.qrlib` files): 0 occurrences of `as` followed by a scalar type name; `as` already reserved (109 other uses in 85 files, all binders/clauses). No word newly reserved, no token added. |
| `bool` distinct | Comparisons (`== != < <= > >=`, non-associative) produce `bool`; `bool` has `==`/`!=`, no ordering, no arithmetic, no conversion. Branches are not on this route (§G02i's `if` is sequential-route only and already two-sorted). |
| `isize`/`usize` width | From the builder's recorded target (`--recorded-target FILE`: header `quirin-recorded-target v1`, keys `pointer_width` 32/64, `triple`, `compiler`, each once). Without one, any program naming either refuses. With one, the check line prints `recorded target: …` among the declared assumptions (§L20) and the C asserts `SIZE_MAX`/`PTRDIFF_MIN`/`PTRDIFF_MAX` against it, so a program recorded for 32 bits does not build on this 64-bit host (tested). **C types:** `size_t`/`ptrdiff_t`, not `uintptr_t`/`intptr_t`: they are mandatory in C11 (the pointer-integer types are optional, C11 7.20.1.4), they are what an index/extent is (the track's buffers and records), `%zu`/`%td` render them portably including MSVC, and their limits are in `<stdint.h>` for the width assertion. |
| Literals | Range-checked per context type, minimum included; `i32` stays the default, so `-2147483648` is still an `i32` literal. |
| Output | Only `print dec|hex X;`, `log LEVEL dec|hex X;`, `flush;`, read by §G02g's own reader; `print(EXPR)` stays `i32`-only and is refused in any module using the new surface. `dec`: the number (signed or unsigned by type). `hex`: `0x` + the type's two's-complement bit pattern, upper case, zero-padded to width/4 digits (`u8` `0xFF`, `i32` `0x000000FF` as before, `u64` 16 digits, `usize` W/4). `bool`: `dec` renders `true`/`false`; `hex` refuses. |

## Positions

| Position | Status |
|---|---|
| §G02s parameters, results, locals (annotated and inferred), literals, arithmetic, comparison, conversion | **widened** |
| §G02g line statements on the scalar route, with typed operands, in any function | **widened** (the std.io route itself is unchanged: a typed `let` is scalar-call syntax and reaches the scalar route) |
| §G02h pipe sinks (`x |> print dec;`, `x |> f |> log info hex;`) | **widened** by desugaring onto the above |
| Executable entry result other than `i32`/`unit` | refused (C `main` returns `int`; the value is published, never returned) |
| §G07.13 record fields | refused, `W48-QR-TYPES-RECORD-FIELDS` |
| §G07.12 loop bounds and index | refused, `W48-QR-TYPES-LOOP-BOUNDS` |
| §G02d extern results as values; §G02i extern + io operands | refused, `W48-QR-TYPES-EXTERN-VALUES` |
| §G02a worker parameters/results and `copy` captures; §G02j tasks + io operands | refused, `W48-QR-TYPES-TASK-CAPTURES` |
| §G02b `Buffer` and §G02c `protected N` payloads | refused, `W48-QR-TYPES-PAYLOADS` |

A route either admits the full table with the full rules or keeps refusing:
every position above marked refused keeps its existing refusal. One refusal
moves code: a conversion written on another route was refused at `Q013`
(name resolution collected the type name) and now reaches that route's own
reader, which refuses it at its own code; `a_conversion_on_another_route_stays_refused`
pins seven such programs (sequential, buffer, loop, std.io, `if`, record) and
none becomes an admission.

## Evidence

**Fixture identity.** `freeze_fixtures.py` runs `quirin check FILE` and
`quirin emit FILE` over every tracked `.qr`/`.qv`/`.qrlib` file and records
exits and stream digests ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json)); `compare_fixtures.py` compares them.
BEFORE: base binary `e171623b…`, 625 files — check 357 exit 0 / 268 exit 1,
emit 153 / 472. AFTER: final binary `2507792a…`, 626 files ([comparison](artifacts/fixtures-compare.txt)):
**every one of the 625 existing files is byte-identical** in exit, stdout and
stderr of both commands — every certificate line, every refusal and every
emitted C byte — and the one difference is the new example
`examples/scalar/fixed_width.qr` (check 0, emit 0). No existing refusal became
an admission. The two freezes are deterministic (a BEFORE rerun was
byte-identical). The four fixtures on the scalar route (`calls.qr`,
`printing.qr`, probes `S10`, `S32`) keep their check lines and messages.

**Boundary table** (`compiler/tests/scalar_types.rs`). Operands MIN, MIN+1,
−1, 0, 1, MAX−1, MAX (those in range) of every integer type, under every
operation, negation and conversion to every other integer type:

* kernel vs an independent `i128` reference: 3,250 operation cases at both
  recorded widths, plus negation and 10 × 10 conversions per value — equal;
* checker vs reference: 1,625 closed entries admitted exactly when the
  reference succeeds (the reference value rendered) and refused with the typed
  fault otherwise; the same 1,625 cases as a library's constant fold (never
  evaluated) refuse exactly the faulting ones — the executable instance of
  `static_fault_is_dynamic_fault`;
* native: per type, one harness of the program's own emitted helpers calls
  every case and compares with the reference inside the program (no output of
  its own), built with Clang 21.1.8 at `-O0` and `-O2` under
  `-fsanitize=undefined,integer -fno-sanitize-recover=all` and at plain `-O2`:
  **2,175 cases × 3 builds, 1,561 reference values and 614 faults per build, every
  fault exit 86, 0 sanitizer reports, 0 bytes on either stream**
  ([summary](artifacts/native-boundary.json), [Clang](artifacts/clang-version.txt));
* renderings: every boundary value of every type printed `dec` and `hex` and
  logged, plus `bool`, run natively under the same three builds; each stream
  equals the reference projection.

The Lean vectors (`QCore.ScalarTypes.agreement`, 34 rows proved by `decide`)
are replayed against the Rust kernel.

**Negative controls** (all refused, each with its own test row): mixed-type
arithmetic and comparison, implicit widening and narrowing at a binding,
out-of-range literals at both ends of every type, overflow at every boundary,
division and remainder by zero, `MIN / -1`, `MIN % -1`, `-MIN`, a constant
fault in an uncalled function, `bool` arithmetic/negation/ordering/conversion,
`hex` of `bool`, chained comparisons, failing conversions, `usize`/`isize`
without a recorded target (parameter, result, local, conversion), a 32-bit
record refusing a 64-bit-only value, a builtin `print(…)` beside the new
surface, a `u8` entry result, `log_level` (`Q030`) and `borrow` (`Q028`) on
the route, and a missing import (`Q015`).

**Lean** (`QCore.ScalarTypes`, `AuditScalarTypes.lean`, 23 theorems, no
placeholder, no project axiom): `checked_in_range` (checked arithmetic never
returns a value outside its type), `checked_exact` (success returns the exact
result), `total_conversion_preserves`, `convert_succeeds_iff` and
`convert_value` (a fallible conversion succeeds exactly when the value fits,
and returns it), `fold_sound` / `static_fault_is_dynamic_fault` (the checker's
static refusal implies the dynamic check fails in every environment, and a
static value is the dynamic one), plus `bool_has_no_arithmetic`,
`bool_has_no_ordering`, `no_bool_conversion`, `no_target_no_value`,
`no_target_no_arithmetic`, `division_by_zero_faults`,
`min_div_minus_one_faults`, `integer_comparison_total`, `pattern_in_width` and
the helper lemmas. The Rust/Lean binding is the shared vector table and the
boundary table, not an extraction theorem.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixture freeze AFTER vs BEFORE (625 files) | byte-identical, 85% | **byte-identical**; the only new entry is the new example |
| std.io-refused fixture moving routes | 12% | none |
| existing scalar-route tests unchanged | 80% | **21/21 pass unchanged** (messages kept) |
| conversion-spelling census | 0 hits, `as` reserved, 95% | **0 hits**, reserved |
| native boundary table, 3 builds, UBSan | agree / 86 / 0 reports, 85% | **2,175 cases × 3: all agree, every fault 86, 0 reports** |
| Lean module, no placeholder or project axiom | 85% | **225 audits pass** (23 new theorems) |
| corpus arbiter `resource-diagnostics.json` | byte-identical `ae17152b…`, 97% | **byte-identical** (`ae17152b…`) |
| standing gate | green, count + new tests only, 85% | **green**; 6,417 → 6,433 (+16, the new test file) |

Not predicted: the contextual `as` rule moves a conversion written on another
route from `Q013` to that route's own refusal (attributed above), and the
sequential route's unchecked parameter annotations
(`W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES`) were found, not changed.

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 359 units, 372 summaries; **6,433 passed, 0 failed, 11 ignored** (`private/test-binaries/20260929T075249Z-4024297/summary.json`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **225 audits** (224 + `AuditScalarTypes.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix regeneration) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| clean | `cargo clean --profile dev` | 0 | |

Exits in [gate/](gate/), record in [gate.json](gate.json); logs under
`private/w48-qr-scalar-types/gate/`. `providers/` is unchanged, so the
provider gate is not due. Clang 21.1.8 recorded in this worktree
([gate/clang-version.txt](gate/clang-version.txt)).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
tip's release `quirin-audit` (`c6d03107…`): exit 0. `resource-diagnostics.json`
sha256 `ae17152b05516be7…` is **byte-identical** to the archived baseline
(`docs/reports/w47-premises-instance/artifacts/arbiters/corpus.identity`, the
digest `w47-round9-integration` confirmed); `monkey-source.json` (`57acfc58…`)
and `resource-diagnostics.chunk-contracts.json` (`cbbcfd9c…`) are identical to
the archive too ([identity](artifacts/arbiter-corpus.identity)). No C-audit
path was touched, as predicted.

## Follow-ups

Registered in the acceptance matrix (580 → 588):

* `W48-QR-TYPES-RECORD-FIELDS`, `W48-QR-TYPES-LOOP-BOUNDS`,
  `W48-QR-TYPES-EXTERN-VALUES`, `W48-QR-TYPES-TASK-CAPTURES`,
  `W48-QR-TYPES-PAYLOADS` — the positions above, each with its size.
* `W48-QR-TYPED-ARITHMETIC-ERROR` — the `throws<E>` fault for operands not
  determined at check time.
* `W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES` — found here: the sequential
  route admits a parameter annotation it never checks
  (`fn g(x: u8) -> i32 { return 0; }`, and so `x: usize` without a recorded
  target), a pre-existing structural acceptance this card did not change.
* `W48-QR-TYPES-MSVC-NATIVE` — the new helpers, `%zu`/`%td`/`%zX` and
  `_Static_assert` are built natively here with Clang only.

## Files likely to conflict with cards 1–4

`crates/qcore-core/src/scalar_calls.rs`, `scalar_calls_c_emit.rs` and the new
`scalar_types.rs`; `crates/qcore-frontend/src/source_scalar_calls.rs`,
`frontend.rs` (the `lexical_references` contextual rule) and
`source_std_io.rs`; the driver's route selection and `compile_scalar_calls`
(`crates/qcore-verification/src/driver.rs`); `crates/qcorec/src/main.rs`;
implemented grammars §G02s/§G02t/§G02g and specification §L14.0/§L20/§L21;
`proof/QCore.lean`.

## Scripts

* `freeze_fixtures.py QUIRIN OUT.json [RAW_DIR]`, `compare_fixtures.py BEFORE AFTER`.
* `census_spelling.py` — the conversion-spelling census.
* The native summary: `QCORE_W48_NATIVE_SUMMARY=native-boundary.json cargo test
  -p qcore-compiler --test scalar_types native_boundary` (written under
  `compiler/`, then moved to `artifacts/`).
