# V06 — R07 signed literal boundaries: adversarial verification

Work card **V06 (verify_or_repair, P1)** against review requirement **R07,
"Signed decimal minimum literals"** (`docs/reviews/r0_3/acceptance.json`,
status `repaired_consumers`; prior evidence
`docs/reports/r0-3-source-validation/README.md`).

B02 classified R07 verified-fixed from the two cited test names. This package
does not restate that classification: it re-derives it by executing an
adversarial boundary matrix against **both** signed literal parsers through
live, external routes, and it records which rows an existing pinned test
already covers and which rows only this probe covers.

**Conclusion: fully covered for the stated boundary. 128 automated probe rows
(35 + 9 + 26 + 52 + 6), plus two hand-transcribed emit checks, 0 mismatches, no
genuine failure found. Nothing was repaired; no tracked file was modified.** Type acceptance was deliberately not broadened (out of scope):
every probe below either exercises an already-admitted spelling or asserts a
refusal.

## Parsers located

R07 names two signed literal profiles. Locating them found **three** signed
routes, not two, plus one adjacent unsigned family. All four are probed.

| Route | Location | Target type | Reached from |
|---|---|---|---|
| (a) source `return` literal | `crates/qcore-verification/src/driver.rs:1007` `parse_return`, called at `crates/qcore-verification/src/driver.rs:703` (`StatementKind::Return`) | `i32` | `qcorec check` / `emit` on a `.qcore` file |
| (a2) emitted constant | `crates/qcore-core/src/c_emit.rs:295` `c_i32`; `crates/qcore-core/src/task_c_emit.rs:29` | `i32` | `qcorec emit` |
| (a3) checked-task scalar operand | `crates/qcore-frontend/src/source_tasks.rs:858` `scalar_operand` (call sites at `:788`, `:1490`, `:1563`) | `i32` | `concurrency checked;` source: `let x = <lit>;`, task `return <lit>;`, buffer constructor argument |
| (b) observation predicate literal | `crates/qcore-contract/src/contract_observation.rs:177` `evaluate` + `:205` `Predicate::expression` | `i128` | public `check_contract_observation` / `check_contract_facts` over a `.qcore` extern contract |
| (c) adjacent, unsigned | `crates/qcore-frontend/src/frontend.rs:958` `usage_grade` (`u64`), `crates/qcore-frontend/src/quant_index.rs:283` (`u128`) | `u64` / `u128` | `use` annotations, index expressions |

Route (a3) is a **second i32 literal parser with an independent range rule**
(`parse::<u64>()` filtered at `<= 2147483648`, then negated in `i64` before the
`i32` cast) and is not exercised by either test B02 cited. It is probed here.

The two shipped range rules are both "parse the final signed spelling", never
"parse the magnitude as the positive signed type and negate":

- `parse_return`: `format!("-{value}").parse().ok()` — the minimum's magnitude
  `2147483648` is never materialised as an `i32`.
- `Predicate::expression`: `format!("-{number}").parse()` when a `Minus` is
  immediately followed by an `Integer`; otherwise `checked_neg()`.

## Reproduction

```sh
cargo build -p qcorec
bash docs/reports/remaining-v06/probes/probe_i32.sh "$PWD"
bash docs/reports/remaining-v06/probes/probe_i32_value.sh "$PWD"
bash docs/reports/remaining-v06/probes/probe_i32_tasks.sh "$PWD"
bash docs/reports/remaining-v06/probes/probe_adjacent_naturals.sh "$PWD"
# route (b): substitute the absolute path, then run OUTSIDE the workspace
sed -i "s#QCORE_CONTRACT_PATH#$PWD/crates/qcore-contract#" \
  docs/reports/remaining-v06/probes/probe_i128/Cargo.toml
cargo run --manifest-path docs/reports/remaining-v06/probes/probe_i128/Cargo.toml
```

Recorded output is under `logs/`. The `probe_i128` crate declares an empty
`[workspace]` and depends on `qcore-contract` by path, so it links the shipped
crate from **outside** the workspace and can only use the public API. It was
built into a scratchpad `CARGO_TARGET_DIR`; no workspace file, manifest, lock
file or test was added or modified (`git status --porcelain` shows only this
untracked report directory).

## Cited existing tests

| Key | Test |
|---|---|
| T-drv | `compiler/tests/unit/driver.rs:345` `review_r07_driver_signed_literal_boundaries` (runs as `driver::tests::…`) |
| T-obs | `crates/qcore-contract/tests/unit/contract_observation.rs:116` `review_r07_observation_signed_literal_boundaries` (included into the lib at `crates/qcore-contract/src/contract_observation.rs:354` as `cleanup_observation_tests`) |
| T-fact | `crates/qcore-contract/tests/unit/contract_observation.rs:154` `review_r07_public_observer_checks_minimum_facts` |
| T-fc | `crates/qcore-contract/src/contract_observation.rs:333` `tests::unsupported_and_overflowing_predicates_fail_closed` |
| — | no existing test pins this row; probe-only evidence |

Re-run for this card (`logs/cited-tests.log`): `qcore-contract --lib review_r07`
2 passed; `qcore-compiler --lib review_` 3 passed.

---

## Table A — route (a), i32 source `return` literal (`logs/i32-source-literals.log`)

Body is `fn main() -> i32 { return <spelling>; }` unless noted. `EXIT 0` =
accepted by `qcorec check`; `EXIT 1` = rejected.

| ID | Spelling | Expect | Exit | Result | Diagnostic | Existing test |
|---|---|---|---|---|---|---|
| A01 | `2147483647` | accept | 0 | MATCH | — | T-drv |
| A02 | `-2147483648` | accept | 0 | MATCH | — | T-drv |
| A03 | `2147483648` | reject | 1 | MATCH | Q020 | T-drv |
| A04 | `-2147483649` | reject | 1 | MATCH | Q020 | T-drv |
| A05 | `0` | accept | 0 | MATCH | — | T-drv |
| A06 | `-0` | accept | 0 | MATCH | — | T-drv |
| A07 | `- 2147483648` (space) | accept | 0 | MATCH | — | — |
| A08 | `-` newline `2147483648` | accept | 0 | MATCH | — | — |
| A09 | `--2147483648` | reject | 1 | MATCH | Q020 | — |
| A10 | `-(-1)` | reject | 1 | MATCH | Q020 | — |
| A11 | `(1)` | reject | 1 | MATCH | Q020 | — |
| A12 | `00000000002147483647` | accept | 0 | MATCH | — | — |
| A13 | `-00000000002147483648` | accept | 0 | MATCH | — | — |
| A14 | `00000000002147483648` | reject | 1 | MATCH | Q020 | — |
| A15 | `-00000000002147483649` | reject | 1 | MATCH | Q020 | — |
| A16 | `999999999999999999999999999999999999999999` | reject | 1 | MATCH | Q020 | T-drv |
| A17 | `-9999…9999` (42 digits) | reject | 1 | MATCH | Q020 | — |
| A18 | `170141183460469231731687303715884105727` (i128 max) | reject | 1 | MATCH | Q020 | — |
| A19 | `-170141183460469231731687303715884105728` (i128 min) | reject | 1 | MATCH | Q020 | — |
| A20 | `+1` | reject | 1 | MATCH | Q020 | — |
| A21 | `2_147_483_647` | reject | 1 | MATCH | Q013 unknown name `_147_483_647` | — |
| A22 | `0x7fffffff` | reject | 1 | MATCH | Q013 unknown name `x7fffffff` | — |
| A23 | `2147483647 + 1` | reject | 1 | MATCH | Q020 | — |
| A24 | `-2147483648 - 1` | reject | 1 | MATCH | Q020 | — |
| A25 | `1 + 1` | reject | 1 | MATCH | Q020 | — |
| A26 | `2147483647.0` | reject | 1 | MATCH | Q020 | — |
| A27 | `4294967296` (2^32) | reject | 1 | MATCH | Q020 | — |
| A28 | `4294967295` (u32 max) | reject | 1 | MATCH | Q020 | — |
| A29 | `-4294967296` | reject | 1 | MATCH | Q020 | — |
| A30 | `-1` | accept | 0 | MATCH | — | — |
| A31 | `-18446744073709551616` (−2^64) | reject | 1 | MATCH | Q020 | — |
| A32 | `9223372036854775808` (i64 max+1) | reject | 1 | MATCH | Q020 | — |
| A33 | `-9223372036854775808` (i64 min) | reject | 1 | MATCH | Q020 | — |
| A34 | `return 0;` in a `-> unit` body | reject | 1 | MATCH | Q020 bare return required | — |
| A35 | `return;` in a `-> unit` body | accept | 0 | MATCH | — | — |

A23–A25 record that **there is no constant arithmetic in the source return
route at all**: `parse_return` matches only the two- and three-token shapes, so
`2147483647 + 1` cannot fold and is refused as a non-literal return rather than
wrapping. Constant folding exists only in route (b), Table B.

## Table A2 — route (a2), value fidelity of accepted spellings (`logs/i32-value-fidelity.log`)

Acceptance alone would not exclude a wrapping bug, so every accepted spelling
was emitted and the emitted constant read back.

| ID | Spelling | Expected value | Emitted constant | Result | Existing test |
|---|---|---|---|---|---|
| V01 | `2147483647` | 2147483647 | `INT32_C(2147483647)` | MATCH | T-drv (value pinned) |
| V02 | `-2147483648` | −2147483648 | `INT32_MIN` | MATCH | T-drv (value pinned) |
| V03 | `0` | 0 | `INT32_C(0)` | MATCH | T-drv |
| V04 | `-0` | 0 | `INT32_C(0)` | MATCH | T-drv |
| V05 | `- 2147483648` | −2147483648 | `INT32_MIN` | MATCH | — |
| V06 | `00000000002147483647` | 2147483647 | `INT32_C(2147483647)` | MATCH | — |
| V07 | `-00000000002147483648` | −2147483648 | `INT32_MIN` | MATCH | — |
| V08 | `-1` | −1 | `(-INT32_C(1))` | MATCH | — |
| V09 | `000001` | 1 | `INT32_C(1)` | MATCH | — |
| V10 | task route, `let x = -2147483648;` | −2147483648 | `(-INT32_C(2147483647) - INT32_C(1))` | MATCH | — |
| V11 | task route, `let x = -0;` | 0 | `INT32_C(0)` | MATCH | — |

V02/V05/V07 and V10 confirm the minimum is emitted as `INT32_MIN` /
`(-INT32_C(2147483647) - INT32_C(1))`, never as a negated `INT32_C(2147483648)`
(which would be an out-of-range C constant). V10/V11 were taken by hand from
`qcorec emit` on the task fixture and are transcribed here; V01–V09 are in the
log.

## Table A3 — route (a3), checked-task scalar operand (`logs/i32-task-operands.log`)

`T…` rows put the spelling in `let x = <lit>;` inside a `concurrency checked;`
program; `R…` rows put it in a task function's `return <lit>;`. This route has
its own range rule and **is not covered by any existing R07 test**; every row
here is probe-only evidence.

| ID | Route | Spelling | Expect | Exit | Result | Diagnostic |
|---|---|---|---|---|---|---|
| T01 | bind | `2147483647` | accept | 0 | MATCH | — |
| T02 | bind | `-2147483648` | accept | 0 | MATCH | — |
| T03 | bind | `2147483648` | reject | 1 | MATCH | Q500 outside i32 |
| T04 | bind | `-2147483649` | reject | 1 | MATCH | Q500 negative outside i32 |
| T05 | bind | `0` | accept | 0 | MATCH | — |
| T06 | bind | `-0` | accept | 0 | MATCH | — |
| T07 | bind | `- 2147483648` (space) | accept | 0 | MATCH | — |
| T08 | bind | `00000000002147483647` | accept | 0 | MATCH | — |
| T09 | bind | `-00000000002147483648` | accept | 0 | MATCH | — |
| T10 | bind | `00000000002147483648` | reject | 1 | MATCH | Q500 |
| T11 | bind | `-00000000002147483649` | reject | 1 | MATCH | Q500 |
| T12 | bind | `4294967296` | reject | 1 | MATCH | Q500 |
| T13 | bind | `-4294967296` | reject | 1 | MATCH | Q500 |
| T14 | bind | `18446744073709551615` (u64 max) | reject | 1 | MATCH | Q500 |
| T15 | bind | `-18446744073709551615` | reject | 1 | MATCH | Q500 |
| T16 | bind | `18446744073709551616` (u64 max+1) | reject | 1 | MATCH | Q500 |
| T17 | bind | `-18446744073709551616` | reject | 1 | MATCH | Q500 |
| T18 | bind | `-170141183460469231731687303715884105728` | reject | 1 | MATCH | Q500 |
| T19 | bind | `--2147483648` | reject | 1 | MATCH | Q500 literal or immutable binding |
| T20 | bind | `-(-1)` | reject | 1 | MATCH | Q500 literal or immutable binding |
| R01 | return | `2147483647` | accept | 0 | MATCH | — |
| R02 | return | `-2147483648` | accept | 0 | MATCH | — |
| R03 | return | `2147483648` | reject | 1 | MATCH | Q500 |
| R04 | return | `-2147483649` | reject | 1 | MATCH | Q500 |
| R05 | return | `-0` | accept | 0 | MATCH | — |
| R06 | return | `-18446744073709551616` | reject | 1 | MATCH | Q500 |

T14–T17 are the rows that matter for this route's specific shape: the magnitude
is parsed as `u64`, so `u64::MAX` and `2^64` are the spellings that would
distinguish an unfiltered `as i32` cast from the shipped `<= 2147483648` filter.
Both reject.

## Table B — route (b), i128 observation predicate literal (`logs/i128-observation-literals.log`)

Every row is evaluated through the public `check_contract_observation` on a real
`.qcore` extern contract
(`extern fn observed() -> unit contract { ensures normal <predicate>; };`)
with supplied facts `minimum = i128::MIN`, `maximum = i128::MAX`. Outcomes are
disjoint: **Checked** (parsed, true), **Violation** (parsed, false — accepted
spelling, refused observation), **Rejected** (parse/range/overflow refusal).

| ID | Predicate | Expect | Actual | Result | Message | Existing test |
|---|---|---|---|---|---|---|
| B01 | `maximum == 170141183460469231731687303715884105727` | Checked | Checked | MATCH | — | T-obs |
| B02 | `minimum == -170141183460469231731687303715884105728` | Checked | Checked | MATCH | — | T-obs |
| B03 | `0 == 170141183460469231731687303715884105728` (max+1) | Rejected | Rejected | MATCH | invalid predicate integer | T-obs |
| B04 | `0 == -170141183460469231731687303715884105729` (min−1) | Rejected | Rejected | MATCH | invalid predicate integer | T-obs |
| B05 | `0 == 0` | Checked | Checked | MATCH | — | T-obs |
| B06 | `-0 == 0` | Checked | Checked | MATCH | — | T-obs |
| B07 | `-0 == -0` | Checked | Checked | MATCH | — | — |
| B08 | `- 170141183460469231731687303715884105728 == minimum` (space) | Checked | Checked | MATCH | — | — |
| B09 | `--170141183460469231731687303715884105728 == 0` | Rejected | Rejected | MATCH | predicate overflow | T-obs |
| B10 | `-(-170141183460469231731687303715884105727) == maximum` | Checked | Checked | MATCH | — | — |
| B11 | `-(-170141183460469231731687303715884105728) == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B12 | `+170141183460469231731687303715884105727 == maximum` | Rejected | Rejected | MATCH | unsupported predicate term | — |
| B13 | `000170141183460469231731687303715884105727 == maximum` | Checked | Checked | MATCH | — | — |
| B14 | `000170141183460469231731687303715884105728 == 0` | Rejected | Rejected | MATCH | invalid predicate integer | — |
| B15 | `-000170141183460469231731687303715884105728 == minimum` | Checked | Checked | MATCH | — | — |
| B16 | `-000170141183460469231731687303715884105729 == 0` | Rejected | Rejected | MATCH | invalid predicate integer | — |
| B17 | `0000 == 0` | Checked | Checked | MATCH | — | — |
| B18 | `-minimum == maximum` | Rejected | Rejected | MATCH | predicate overflow | T-obs |
| B19 | `minimum - 1 == 0` | Rejected | Rejected | MATCH | predicate overflow | T-obs |
| B20 | `0 - minimum == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B21 | `minimum * -1 == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B22 | `minimum + minimum == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B23 | `minimum + 1 == -170141183460469231731687303715884105727` | Checked | Checked | MATCH | — | T-obs |
| B24 | `minimum - minimum == 0` | Checked | Checked | MATCH | — | T-obs |
| B25 | `-(minimum + 1) == maximum` | Checked | Checked | MATCH | — | T-obs |
| B26 | `maximum + 1 == 0` | Rejected | Rejected | MATCH | predicate overflow | T-fc |
| B27 | `maximum * 2 == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B28 | `maximum + maximum == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B29 | `maximum - maximum == 0` | Checked | Checked | MATCH | — | — |
| B30 | `-maximum == minimum + 1` | Checked | Checked | MATCH | — | — |
| B31 | `<max> * <max> == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B32 | `-170141183460469231731687303715884105728 - 1 == 0` | Rejected | Rejected | MATCH | predicate overflow | — |
| B33 | `170141183460469231731687303715884105727 + 1 == 0` | Rejected | Rejected | MATCH | predicate overflow | T-fc |
| B34 | `minimum / -1 == 0` | Rejected | Rejected | MATCH | unsupported predicate syntax | T-obs |
| B35 | `minimum % -1 == 0` | Rejected | Rejected | MATCH | unsupported predicate syntax | T-obs |
| B36 | `minimum < minimum + 1` | Checked | Checked | MATCH | — | — |
| B37 | `minimum <= -170141183460469231731687303715884105728` | Checked | Checked | MATCH | — | — |
| B38 | `maximum >= 170141183460469231731687303715884105727` | Checked | Checked | MATCH | — | — |
| B39 | `minimum != 0` | Checked | Checked | MATCH | — | — |
| B40 | `minimum < maximum` | Checked | Checked | MATCH | — | — |
| B41 | `9223372036854775808 == 9223372036854775808` (i64 max+1) | Checked | Checked | MATCH | — | — |
| B42 | `-9223372036854775809 < 0` (i64 min−1) | Checked | Checked | MATCH | — | — |
| B43 | `minimum == 170141183460469231731687303715884105727` | Violation | Violation | MATCH | observed contract violation | — |
| B44 | `maximum == 0` | Violation | Violation | MATCH | observed contract violation | — |
| B45 | `0x7fffffff == 0` | Rejected | Rejected | MATCH | unsupported predicate syntax | — |
| B46 | `170_141 == 0` | Rejected | Rejected | MATCH | unsupported predicate syntax | — |
| B47 | `1e3 == 0` | Rejected | Rejected | MATCH | unsupported predicate syntax | — |
| F01 | supplied `minimum = i128::MIN`, `minimum == <min>` | Checked | Checked | MATCH | — | T-fact |
| F02 | supplied `minimum = i128::MIN + 1` | Violation | Violation | MATCH | observed contract violation | T-fact |
| F03 | supplied `minimum = i128::MAX` | Violation | Violation | MATCH | observed contract violation | — |
| F04 | supplied `minimum = 0` | Violation | Violation | MATCH | observed contract violation | — |
| F05 | no facts supplied | Rejected | Rejected | MATCH | missing observation: `minimum` | T-fact |

Constant arithmetic in this route is checked at both extrema and stays checked
when a **supplied minimum-valued fact** is the operand: B18 (negation of the
minimum), B19/B32 (`min − 1`), B20 (`0 − min`), B21 (`min × −1`), B22
(`min + min`) all reject with `predicate overflow`. Folding that stays in range
is admitted and exact (B23, B24, B25, B30, B36). B41/B42 record that the `i64`
boundary is not this parser's boundary — only the `i128` extrema are.

The distinction between **Violation** (B43/B44, F02–F04) and **Rejected** is
load-bearing: a well-formed extremum predicate that the observation contradicts
must be refused as an observed violation, not as a parse error, and neither is
ever a silent skip (F05).

## Table C — adjacent unsigned literal families (`logs/adjacent-natural-literals.log`)

Recorded so the audit does not leave an unchecked parse next to the signed ones.
These are a different literal profile from R07's signed extrema; this table
claims only that they are range-checked and fail closed.

| ID | Spelling | Exit | Diagnostic |
|---|---|---|---|
| U01 | `use one` | 1 | Q100 usage demand (literal admitted; rejected downstream) |
| U02 | `use 18446744073709551615` (u64 max) | 1 | Q100 usage demand (literal admitted) |
| U03 | `use 18446744073709551616` (u64 max+1) | 1 | Q000 usage bound exceeds the supported u64 range |
| U04 | `use [0, 18446744073709551616]` | 1 | Q000 usage bound exceeds the supported u64 range |
| U05 | `use -1` | 1 | Q000 named grade / natural literal / interval required |
| U06 | `use 0` | 0 | accepted |

## What this does and does not establish

Established, at the stated boundary:

- Both signed extrema are accepted in every reachable signed literal route, at
  the correct value, with the minimum emitted as `INT32_MIN` rather than a
  negated out-of-range C constant.
- Both extrema ±1 reject in every route, including with leading zeros, with an
  intervening space or newline between the sign and the digits, and at the
  neighbouring `u32`/`i64`/`u64`/`i128` widths.
- Zero and negative zero accept and evaluate to zero everywhere.
- No route parses the minimum's magnitude as the positive signed type.
- Constant arithmetic exists only in the observation route and stays checked at
  the extrema, including when the operand is a supplied minimum-valued fact.
- Non-decimal and separator spellings (`0x…`, `1_2`, `1e3`, `+1`, `1.0`) reject
  rather than being reinterpreted.
- The i128 route was exercised **through its public loadable API from outside
  the workspace**, not through the private `evaluate` entry point the two
  existing tests use.

Not established by this package (unchanged open boundaries, consistent with the
R07 acceptance entry's `remaining_boundary`):

- Arbitrary expression and result semantics are still not added; the source
  return route admits literals only, and the observation grammar remains the
  existing bounded subset (no division or modulo).
- No parser/Rust/C refinement proof is claimed. Table A2 reads back emitted
  constants; it does not execute the emitted C, and it is not a proof that the
  C compiler's constant is the model's value.
- Route (a3)'s acceptance rows were checked for value only at the minimum and
  zero (V10/V11), not across the whole accepted range.
- Table C's unsigned families were probed for fail-closed range checking only;
  no claim about their semantics is made here.
- No verdict, CT mapping, contract-corpus count, pinned expectation or
  acceptance record was changed by this card. `docs/reviews/r0_3/acceptance.json`
  is untouched: this is confirming evidence for its existing
  `repaired_consumers` status, not a status change.

## Files

```
docs/reports/remaining-v06/
  README.md
  probes/probe_i32.sh                  route (a)  — 35 rows
  probes/probe_i32_value.sh            route (a2) — 9 rows
  probes/probe_i32_tasks.sh            route (a3) — 26 rows
  probes/probe_adjacent_naturals.sh    route (c)  — 6 rows
  probes/probe_i128/                   route (b)  — 52 rows, external crate
  logs/i32-source-literals.log
  logs/i32-value-fidelity.log
  logs/i32-task-operands.log
  logs/i128-observation-literals.log
  logs/adjacent-natural-literals.log
  logs/cited-tests.log
```
