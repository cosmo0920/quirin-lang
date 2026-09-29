# W48-QR-SCALAR-TYPES-R3: frozen prediction

Frozen after the BEFORE fixture freeze and before any source, test or Lean
change. Base: local `main` at `d3508e10` (= origin/main), one records-only
commit (`CLANG-GAP-CARD-QUEUE`, documents and the acceptance matrix, no Rust)
past the brief's `535c327f`; the worktree's HEAD is that commit.

## BEFORE (measured)

`quirin` debug binary built at the base (sha256 `9342a7c9174b363e…`). The
first half's `freeze_fixtures.py` ran `quirin check FILE` and `quirin emit
FILE` over the 627 tracked `.qr`/`.qv`/`.qrlib` files
([BEFORE](artifacts/fixtures-before.json)): check exit 0 on 357, exit 1 on
270; emit exit 0 on 153, exit 1 on 474.

Probes at the base (`private/`, not tracked):

* `type P { x: u8, y: i32, ok: bool }` on the buffer route refuses at `Q051`
  naming `AGGREGATE-FIELD-TYPES`;
* `let b = a as u8;` on the buffer route refuses at `Q051` (the literal-bind
  shape message);
* `remaining-v04/probes/P08-valid-named-use.qr` and `P10-decl-after-use-site.qr`
  check `checked 1 function(s); entry \`main\`` with `extern fn f(q: quant<n
  where n := 5>) -> unit;` unread.

## Mechanism (planned)

1. **`W48-QR-TYPES-RECORD-FIELDS`.** `FieldType` becomes `Scalar(§G02t type) |
   Bytes`; the buffer route's value environment becomes typed: every name has
   exactly one type — the existing sources (literal `let`, byte read, `extent`,
   loop index) stay `i32` — and a field read binds its field's type. Every
   position of the route checks the type it takes: index, stored byte, lend
   bounds and loop bound take `i32`; a field value takes its field's type (a
   literal range-checked against it, `true`/`false` for `bool`, a name of
   exactly that type); a rendering operand takes any type and renders it at
   that type through §G02t's `std.fmt` rules; a copy bind keeps its type.
   The conversion `let NAME = OPERAND as TYPE;` is admitted on this route,
   decided at check time (every value here is a check-time constant) and
   refused when it does not fit. No arithmetic is added: the route has none.
   Each record emits one C struct with `<stdint.h>`/`<stdbool.h>` member
   types; `isize`/`usize` fields need the recorded target, which the route now
   receives and publishes as the named assumption.
2. **`W48-QR-TYPES-EXTERN-VALUES`** and **`W48-INERT-EXTERN-DECLARATION-TYPES`**,
   then 3 (`W48-QR-TYPES-TASK-CAPTURES`), 4 (`W48-QR-TYPES-LOOP-BOUNDS`) and 5
   (`W48-QR-TYPES-PAYLOADS`) in order as time allows; a position not finished
   keeps its refusal and its follow-up.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| item 1 moves no tracked fixture | every existing file byte-identical in both commands | 85% |
| item 1 existing tests changed | only `a_conversion_on_another_route_stays_refused`'s buffer and record programs (a deliberate widening) | 70% |
| item 2's inert-declaration check moves exactly `P08` and `P10` (admission → refusal) | exactly those two | 65% |
| native boundary table, UBSan `-O0`/`-O2` + plain `-O2`, extended to each widened position | every stream equals the reference, 0 sanitizer reports | 80% |
| Lean | new theorems audit with no placeholder or project axiom | 85% |
| corpus arbiter `resource-diagnostics.json` | byte-identical `ae17152b…` | 97% |
| standing gate | green; test count rises by new tests only | 80% |
| items completed | 1 complete (85%); 2 complete (45%); 3 (25%); 4 (20%); 5 (10%) | as stated |
