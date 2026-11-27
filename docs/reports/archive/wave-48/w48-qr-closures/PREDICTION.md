# W48-QR-CLOSURES: prediction

Frozen before any source change, at base `f3969316` (= local `main` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). Taken first: the BEFORE fixture freeze
(`artifacts/fixtures-before.json`, card 0's `freeze_fixtures.py`, debug
`quirin` built at the base, 697 files — check 390 exit 0 / 307 exit 1, emit
181 / 516) and the spelling census (`artifacts/census-before.json`): **zero**
`spawn NAME |`, `spawn NAME ||`, `spawn NAME [|`, `|` before a capture-mode
word, `|`/`||` before `->`, and `as NAME` inside a spawn's capture list, over
697 tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` files; 2 `|`, 2 `||`, 4 `[|`,
4 `|]` and 118 `|>` tokens in total; 118 reserved words.

## Where the base stands

A worker is always a named top-level `fn`, and `spawn s w(CAPTURES)` names it.
A `|` outside an `if` condition is refused by the operator gate (`Q038`), so
every closure spelling is refused before name resolution today.

## Plan the prediction is about

1. **Parse and lift.** `spawn SCOPE |CAPTURES| -> TYPE [effects { … }] { BODY }`
   is recognised by the parser, which synthesizes a **hidden function
   declaration** — its name derived from the enclosing function and the
   closure's ordinal there, spelled with `#` so no source identifier can equal
   it — placed immediately before the enclosing function, with one parameter
   per capture (the place's root name, or a field capture's last segment, or an
   explicit `as NAME`), the written result and effect row, and the closure body
   as its body. The spawn becomes an ordinary spawn of that function with the
   written captures. The closure's tokens stay where they are; windows that
   contain a closure but are not inside it skip its header and body, so the
   parent's statement reads exactly as its twin's `spawn s w(CAPTURES)`.
2. **Types from the spawn site.** A capture list spells modes, not types; the
   task route decides each parameter's type from the enclosing function's
   environment at the spawn site (the place's own type; the loan or lease type
   for `read`/`lend` of an owned Buffer), by lowering the enclosing function
   once to discover them, then lowers every function in declaration order —
   so the lifted program checks and emits exactly like its hand-written twin.
   The transferability judgment decides each capture unchanged.
3. **No implicit capture** is the elaborator's: the hidden body's scope holds
   only the capture binders (and module globals), so an outer name refuses,
   named, with a closure-specific message.
4. Closures anywhere but after `spawn SCOPE`, bracket-adjacent capture lists,
   duplicate captures and closures on non-task routes refuse by name.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze AFTER vs BEFORE (697 files, `check` and `emit`, exit + both streams) | byte-identical for every existing file (no existing file spells a closure) | 85% |
| existing tests changed | 0–2 files (a pin of the `|` refusal's wording, if one exists) | 70% |
| twin identity | every closure example's emitted C and `check` output byte-identical to its hand-written twin | 80% |
| negative controls (unlisted outer name; refused capture mode; duplicate capture; overlapping lends; result type mismatch; non-spawn position; capture list adjacent to `[`/`]`) | each refused, naming the capture or name | 85% |
| native runs on real threads (`-O0`/`-O2` UBSan, plain `-O2`, `-Wall -Wextra -Werror -pedantic`, TSan on the concurrent examples) | clean, streams equal the reference, 0 reports — the C is the twin's | 75% |
| Lean | one new module and audit, ~8–15 theorems, no placeholder or axiom | 75% |
| corpus arbiter `resource-diagnostics.json` | identical to the base's archive (`c012e785…`, 586 / 10 / 4,238) — no C-audit path is touched | 95% |
| standing gate first run | green | 45% |
| items completed | 1 (75%), 2 (70%), 3 (70%), 4 (15% epoch route, 10% mutex route) | |
