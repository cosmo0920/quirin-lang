# W48-QR-SCALAR-TYPES-R2: frozen prediction

Frozen after the BEFORE fixture freeze and before any source, test or Lean
change. Base: local `main` at `7e723a96` (the worktree's HEAD).

## BEFORE (measured)

`quirin` debug binary built at the base (sha256 `169e3920d51ea902…`). The
first half's `freeze_fixtures.py` ran `quirin check FILE` and `quirin emit
FILE` over the 626 tracked `.qr`/`.qv`/`.qrlib` files
([BEFORE](artifacts/fixtures-before.json)): check exit 0 on 358, exit 1 on
268; emit exit 0 on 154, exit 1 on 472.

BEFORE probes of item 0 (`W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES`). A helper
function appended to every admitted `.qr` fixture under `examples/`,
`compiler/tests/fixtures/` and `docs/reports/` (169 fixtures, five variants
each) measures three behaviours:

* **Sequential route** (49 fixtures): parameter annotations are never read —
  `x: Widget`, `x: fn() -> i32`, `x: u8` and `x: usize` with no recorded target
  are all admitted; result and local annotations are checked (`Q020`).
* **Entry-only routes** (47 fixtures: `source.std-io.v1`, the buffer/record/loop
  route, string, socket, HTTP and event): a function other than the entry is
  ignored wholesale unless it carries one of the route's own constructs — its
  parameter, result and local annotations (`-> Widget`, `let q: usize = 7;`)
  are admitted unread, and only name resolution reads its body (`Q013`).
* **Every other route** refuses the helper or checks it (`Q020`, `Q500`,
  `Q600`, `Q026`, `Q401`); the scalar route admits `x: u8` and refuses `x:
  usize` without a record, as the first half built.

Two tracked fixtures are admitted today only because of this: the probes
`remaining-v03/probes/S13_unknown_parameter_type.qr` (`x: Widget`) and
`X14_function_typed_param_helper.qr` (`f: fn() -> i32`), recorded there as the
named boundary B-1. No admitted fixture on an entry-only route has a second
function. Uncalled `extern fn` declarations on the sequential route
(`P08`/`P10`'s `quant<…>` parameter) are a separate position.

## Mechanism (planned)

0. **Item 0.** The sequential route checks every parameter annotation against
   the §G02t table (`isize`/`usize` need a recorded target) and refuses every
   other type; each entry-only route refuses a function it does not model
   instead of admitting it unread. A module with `std.io` lines, scalar-call
   syntax and such a helper reaches the scalar route, which checks every
   function. Uncalled extern declarations stay as they are and become a named
   follow-up.
1. **Item 1.** A builtin, payload-free `ArithmeticError { Overflow,
   DivisionByZero, OutOfRange }` on `source.scalar-calls.v1`: a function whose
   row declares `throws<ArithmeticError>` turns a fault at an operation with a
   non-constant operand into that error; a call to such a function is consumed
   by `?` (the caller's row must permit it) or by `match CALL { Result::Ok {
   value: v } => { … } Result::Err { error: e } => { … } }`; `raise e;` and
   `raise ArithmeticError::VARIANT;` re-raise. A constant-folded fault is still
   refused at check time, and a fault in a function without the permission is
   still the closed evaluation's refusal. The emitted C of a throwing function
   returns the error instead of exiting 86; an entry that propagates one exits
   with a status naming the variant.
2. Items 2–6 in the brief's order as time allows; every position not finished
   keeps its refusal and its follow-up.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture AFTER vs BEFORE | exactly `S13` and `X14` move (admission → refusal); every other existing file byte-identical in both commands | 80% |
| item 0 moves no existing fixture on an entry-only route | none moves | 90% |
| item 1 moves no existing fixture | none moves (new syntax only) | 90% |
| existing tests needing a change for item 0 | at most a few unit tests that pin a parameterised sequential function | 60% |
| native boundary table (UBSan, `-O0`/`-O2`) | every dynamic fault returns the typed error, 0 sanitizer reports, no exit 86 on a dynamic position | 85% |
| Lean | new theorems audit with no placeholder or project axiom | 85% |
| corpus arbiter `resource-diagnostics.json` | byte-identical `ae17152b…` | 97% |
| standing gate | green; count rises by the new tests only | 85% |
| items completed | 0 and 1 complete; item 2 attempted | 60% |
