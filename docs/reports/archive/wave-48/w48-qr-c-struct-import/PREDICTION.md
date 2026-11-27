# W48-QR-C-STRUCT-IMPORT: prediction

Frozen before any source change, at base `96764798` (= origin/main; the
worktree's HEAD was already that commit). The BEFORE fixture freeze
(`artifacts/fixtures-before.json`, debug `quirin` `a1583969…`, 632 files —
check 360 exit 0 / 272 exit 1, emit 154 / 478) and the base census
(`artifacts/census-base.json`) were taken first.

## Plan the prediction is about

* A new route, `source.ffi.v1`, selected before every other route by
  `#import std.ffi.unsafe;` or a `struct!` declaration, so no existing program
  can reach it and no FFI program can reach another route.
* `struct! { type T = c"struct tag" from "h.h"; fields { … } }` read by the
  frontend at declaration position; the layout read by a new gateway module
  running the recorded compiler (the recorded target's `compiler`/`triple`
  compared with the running Clang) over the header, from Clang's AST and
  record-layout dump; the emitted C `#include`s the header and
  `_Static_assert`s `sizeof`, each listed field's `offsetof` and its type.
* `[| quant #c_ffi |]` read by the frontend parser as a declaration attribute
  and by the contract loader, which withholds a marked declaration from the
  ordinary function map and rederives it from the retained texts.
* No new diagnostic code (the `Q-NAMESPACE-DECISION` icebox): refusals are
  condition extensions of `Q032`–`Q035`, `Q063`–`Q064`, `Q051`–`Q053`,
  `Q022`–`Q027` and `Q028`–`Q031`, each stated in §L21.
* A verdict carrying assumptions (`AssumedVerdict`), whose join keeps them;
  an FFI module publishes `Safe` only with the unsafe-FFI assumption.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| census: `struct !`, `c"…"`, `[|`, `|]`, `quant # c_ffi`, `c_ffi`, `std.ffi`, `| copy` in 632 tracked files | 0 each (measured before the change: 0 each); no card word reserved | 99% |
| fixture freeze AFTER vs BEFORE (632 files, `check` and `emit`, exit + both streams) | byte-identical; the only differences are new files | 80% |
| existing tests changed | none, beyond constructing a frontend/loader value that gained a field | 70% |
| real-library example natively, Clang, `-Wall -Wextra -Werror -pedantic`: `-O0` and `-O2` under UBSan, plain `-O2` | builds, stdout/stderr equal the reference, exit 0, 0 sanitizer reports | 75% |
| layout drift after checking (header edited, C rebuilt) | the `_Static_assert` build refuses | 90% |
| negative controls (no import, type/offset mismatch, unknown tag, opaque field, unmarked contract, unresolvable header, plain-Safe claim) | each refused, none admitted | 85% |
| Lean | one new module and audit, ~10–15 theorems, no placeholder or axiom | 80% |
| corpus arbiter `resource-diagnostics.json` | identical to the base's current archive, CG06's AFTER `c012e785…` (586 / 10 / 4,238) | 95% |
| standing gate | green, count grows by the new tests only | 60% (first run) |
| items completed | 1 (90%), 2 (75%), 3 (65%), 4 (55%), 5 (60%) | |
