# W48-QR-CLOSURES: closures that capture into concurrent workers

Card 4 of the `.qr` language track, and the track's goal: `.qr` whose emitted
C runs concurrent workers written as **closures** that take variables from the
enclosing scope. Base `f3969316` (= local `main` = origin/main; the worktree's
HEAD was already that commit, so no reset or fast-forward was needed).
Grammar: implemented grammars §G02v (new) and §G02a; semantics: specification
§L16 and §L21. The [prediction](PREDICTION.md) was frozen before any source
change, after the BEFORE fixture freeze and the spelling census.

## The syntax as implemented

The user's decision of 2026-09-29 — an explicit capture list, lifted into a
hidden named worker, no implicit capture — with the brief's proposed detail
implemented as proposed:

```ebnf
spawn    = "spawn", SCOPE, captures, "->", TYPE, [ "effects", "{", EFFECTS, "}" ], block ;
captures = "||" | "|", capture, { ",", capture }, "|" ;
capture  = ( "copy" | "move" ), PLACE            (* PLACE = NAME { "." FIELD } *)
         | "read", PLACE, "share", SHARE
         | "lend", PLACE, LOW, "..", HIGH
         | "write", PLACE                         (* refused by the judgment, by name *)
         , [ "as", NAME ] ;
```

* The capture spellings are exactly a named worker's (card 0's and card 3's);
  `share NAME under M` is **not** one — on the mutex route it is a clause of a
  lexical `thread scope` block, not a capture a named worker's list admits (see
  item 4).
* The result type is **required**; the effect row follows the worker rules.
* Inside the body each capture is bound under its place's root name, a field
  capture's last segment, or an explicit `as NAME` (implemented, since two field
  captures ending in the same name — `copy a.v, copy b.v` — need it).
* The empty list is `||`, one token (`| |` refuses, naming `||`).
* A closure is a spawn target in both spawn forms: `let h = spawn s |…| -> T { … }?;`
  and `match spawn s |…| -> T { … } { Result::Ok … Result::Err … }`.

**Census** ([script](census_closure_spelling.py), [BEFORE](artifacts/census-before.json),
[AFTER](artifacts/census-after.json)): over the 697 tracked
`.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` files at the base, **zero** `spawn NAME |`,
`spawn NAME ||`, `spawn NAME [|`, `|` before a capture-mode word, `|` or `||`
before `->`, and `as NAME` inside a spawn's capture list; 2 `|`, 2 `||`, 4 `[|`,
4 `|]` and 118 `|>` tokens in all. Reserved words: 118, **+0**. The capture
list never touches `[`/`]`: card 1's `[|`/`|]` stay the quant delimiters, and a
capture list adjacent to a bracket is refused (`Q000`). The `|>` family is
untouched: `|>` still wins over `|` by width, and a `|` or `||` claims a closure
reading only directly after `spawn SCOPE`.

## The mechanism

**Parse and lift** (`crates/qcore-frontend/src/frontend_tasks.rs`,
`find_closures`/`parse_closure`, called from `SourceParser::parse` before the
task and match inventories). Each closure yields a **hidden function
declaration** named `ENCLOSING#closureN` — the enclosing function's name and
the closure's ordinal there in source order; `#` is no identifier character,
so no source declaration can take it — inserted immediately before the
enclosing function (a closure's own closures before it), with one parameter
per capture (the binder; `use one` for a `move`), the written result type and
effect row, and the closure's body. The spawn becomes an ordinary spawn of
that worker with the written captures (`SourceTaskExpressionKind::Spawn`, the
worker located at the capture list). `SourceProgram::closures()` records each
closure's extents.

**The tokens stay where they are.** A statement that spawns a closure contains
the closure's bytes, so every reader of a byte window — the task inventory, the
elaborator's references, the task route's statement readers — skips a
closure's header and body when the window contains the closure without lying
inside it (`SourceProgram::outside_closure_window`); the enclosing statement
therefore reads `spawn SCOPE |CAPTURES|`, its twin's shape, and references its
hidden worker as the twin's statement references the named one. Two
end-of-expression checks read the visible end (`visible_end`): the closing bar,
where the twin's `)` is.

**Types from the spawn site** (`crates/qcore-frontend/src/source_tasks.rs`).
A closure's signature reads its result and effect row with the named worker's
code (`result_and_effects`, split out of `signature` byte for byte); its
parameters wait. `lower` first lowers each function that spawns an undecided
closure once, in **discovery** mode, where the spawn decides each parameter's
type from the captured place in the environment there
(`Builder::closure_parameter`): a record's record type, a scalar's type, an
owned Buffer's `own<Buffer<T>>`, and for `read … share`/`lend …` of an owned
Buffer its `borrow<Buffer<T>>`/`lend<Buffer<T>>` — the one type the
transferability judgment could admit (`QCore.Closures`). A place that is not a
value (a handle, a Result, a scope, an error, `unit`) refuses there, naming the
capture. The discovery lowering's outcome is discarded unless it refused before
every closure it spawns was decided. Then every function is lowered in
declaration order with complete signatures — exactly as the hand-written twin
is — so the judgment, the ledger, the checker and the emitter see a named
worker. There is no second checker and no closure node past the parser.

**No implicit capture** is the elaborator's: the hidden worker's scope is its
parameters (the capture binders) and the module's globals. A name the body
uses that the enclosing scope holds at the spawn site is refused (`Q013`) with
a message naming it and saying the closure does not capture it; a place
captured twice or two captures bound to one name refuse (`Q011`).

**Everything else is refused by name.** A capture-list shape anywhere but after
`spawn SCOPE` (`Q038`), a closure in a named worker's argument list (`Q000`),
and a closure on any route that does not lower it — FFI, field-footprint,
epoch, mutex, or none — at the driver before that route reads it (`Q600`).

## Items

| Item | Status |
|---|---|
| 1. Parse and lift: every capture mode, scalar and record, `ArithmeticError` through `join`; hidden name; twin identity; Lean | **done** |
| 2. The `match spawn s \|…\| … { Result::Ok … Result::Err … }` recovery form; sequential and concurrent sibling closures | **done**: `closures.qr` runs two concurrent closures under nested `match` recovery, `closure_errors.qr` two sequential siblings in one scope |
| 3. Diagnostics at the closure's span, naming the capture or free variable | **done**: the hidden worker's span *is* the closure's (the opening bar through the body's `}`), so a whole-worker refusal (the leaf rule, a missing return) names it; a capture's refusal names the capture; an uncaptured outer name is named as such |
| 4. Stretch: closures on the epoch route and the mutex route | **not done**, refused by name and registered |

Item 4 was measured, not only skipped. On the composed epoch route the plan
names every worker it accounts for (`thread scope NAME { copy; peak …; }`), and
a hidden worker's name cannot be spelled, so accounting for a closure needs a
plan spelling — a language decision (`W48-QR-CLOSURE-EPOCH`, M). On the mutex
route a child is a lexical `thread scope NAME share PLACE under MUTEX { … }`
block, not a spawned worker, and `share … under M` is that scope's clause, not
a capture any named worker's list admits, so the brief's `share NAME under M`
capture has no hand-written twin to lift to (`W48-QR-CLOSURE-MUTEX`, M). Both
keep a by-name `Q600` refusal at the driver.

## Twin identity

Each published example has a hand-written twin beside it — each closure written
as a named worker declared before `main`, its parameters at the captured
places' types, the spawn naming it with the same captures:
[`closures.qr`](../../../examples/threads/closures.qr) /
[`closures_hand_written.qr`](../../../examples/threads/closures_hand_written.qr)
and [`closure_errors.qr`](../../../examples/threads/closure_errors.qr) /
[`closure_errors_hand_written.qr`](../../../examples/threads/closure_errors_hand_written.qr).
In the AFTER fixture freeze each pair's `check` and `emit` exit, stdout and
stderr digests are **equal** (emitted C `f7add095…` and `7d0228a9…`; the
certificate line `7321f3c6…` for both). `compiler/tests/closures.rs` also
compares, for those two and for seven inline pairs covering every capture mode
(`copy` of a scalar field and of a whole record of scalars, `move` of an owned
Buffer beside a `read … share` of a field, a `lend` of a field, a whole record
that owns moved and returned, an owned field moved out under `as`, the empty
list, two captures bound under `as`): the emitted C, the execution profile, the
declaration count, the entry, the result profile (through each name's function
identity, since a hidden worker's name is its closure's), the checked task
module with names and source positions erased, and the reference run's two
streams — all equal.

## Corrections

| What | Before | After | Why |
|---|---|---|---|
| The prediction's base arbiter archive | `c012e785…`, 586 / 10 / 4,238 (card 3's) | `2fc3af8c…`, **602 / 10 / 4,222** (CG08's AFTER) | CG08-CASTS-AND-TYPE-QUERIES merged into the base after card 3's archive; the prediction cited the older archive. The predicted *identity* is what was measured |
| `signature` (`source_tasks.rs`) | result and effect-row reading inline | `result_and_effects`, split out byte for byte | a closure's header is read by the same code |
| `lower`'s function loop | one loop | a per-function `lower_function`, a discovery pass for modules with closures, then the same loop | parameter types come from the spawn site; a module without closures takes exactly the old path |
| `lexical_references` | read the whole statement window | skips a contained closure's header and body; `lend` after `\|`; an `as` binder in a capture list; a closure's hidden worker as a reference | the enclosing statement resolves what its twin's does |
| the operator gate's bare `\|` | refused everywhere outside a condition (`Q038`) | a closure's two bars admitted; a capture-list shape elsewhere refused with a closure message (`Q038`) | the census found no existing `\|` in either position |
| a named worker's capture parser | `\|` first: "capture requires copy, move, …" | a closure in the list: "a closure is a spawn target only" (`Q000`) | same code, a naming message; no existing file spells it |

No existing test changed; no existing file's check or emit changed (below).

## Evidence

**Negative controls** (`compiler/tests/closures.rs`, first, in the brief's
order): an unlisted outer name — `Q013` naming it ("belongs to the scope
enclosing this closure, and the closure does not capture it"; the same body
with it listed is admitted, and a name the enclosing scope does not have keeps
"unknown name"); refused capture modes — `copy` of a record that owns (`Q600`,
the aggregate judgment naming the leaf `job.data`), `write` (refused by name),
`move` of a scalar, and a handle (`Q600`, "names a task handle, which no worker
parameter can receive"); a duplicate capture — `Q011` for a place captured
twice and for two captures bound to one name; overlapping lends by two
closures — the ledger's `OverlappingLeases` (disjoint ranges admitted); a
result type that does not match the returns — `Q020`, and a missing `-> TYPE`
— `Q000`; a closure at a non-spawn position — stored (`Q038`) and passed to a
worker (`Q000`), both naming `W48-QR-CLOSURE-VALUES`; a capture list adjacent to
`[`/`]` — `spawn s [|copy x|] …` and `spawn s |copy x|] …`, `Q000`. Beside
them: a closure that spawns (the first-order leaf rule, `Q600`, at the
closure's span), the outer scope used inside a body (`Q013`), and closures on
the epoch and mutex routes (`Q600`, naming the residuals).

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`f5b46672…`, 697 files)
and at the tip (`c9a6808c…`, 701 files) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**697 of 697 existing files byte-identical** in exit, stdout and stderr of
`check` and `emit`; the 4 new entries are the examples and their twins.

**Native on real threads** ([summary](artifacts/native-closures.json)): both
examples, Clang 21.1.8, `-std=c11 -pthread -Wall -Wextra -Werror -pedantic`, at
`-O0` and `-O2` under `-fsanitize=undefined,integer -fno-sanitize-recover=all`
and at plain `-O2`, plus `closures.qr` with every creation refused through the
runtime's `QCORE_TASK_CREATE` seam (the record stays the parent's, its `count`
printed): **9 runs**, exit 0, both streams equal to the reference, **0**
reports; under **ThreadSanitizer** through the existing pthread binding: each
example unforced, and `closures.qr` forced with both closures live together
and forced in reverse, each forced row asserting both workers ran — **4 rows**,
clean, **0** races, stdout equal to the reference. `closures.qr` is the goal
example: an outer scalar, an outer record with an owned field, and two
disjoint lends captured by two concurrent closures, joined, the results
rendered through `std.io` (`41`, `64`, `41`, `0x44332211`). The page
transcripts are read out of `examples/threads/README.md` and compared with the
reference.

**Lean** ([module](../../../proof/QCore/Closures.lean),
[audit](../../../proof/AuditClosures.lean), 19 theorems, no placeholder, only
`propext`, `Classical.choice`, `Quot.sound`): capture admission equals the
transfer judgment — the parameter a closure's capture binds is the only one the
judgment admits (`theClosureParameterIsTheOnlyAdmissibleOne`,
`aRecordCaptureAdmitsOnlyItsOwnRecordType`), so a closure capture is admitted
exactly when some twin's is (`aClosureCaptureIsAdmittedExactlyWhenSomeTwinIs`),
by the kind judgment itself (`aClosureCaptureIsDecidedByTheKindJudgment`), and
refused with the judgment's own reason (`aClosureRefusalIsTheJudgmentsOwnReason`,
`aClosureWriteCaptureIsRefusedByName`); no implicit capture — the body's scope
ignores the enclosing scope (`theBodyScopeIgnoresTheEnclosingScope`), a body
that resolves uses no outer name but a binder
(`theBodysFreeOuterNamesAreExactlyTheCaptureBinders`), an uncaptured outer name
is refused (`anUncapturedOuterNameIsRefused`); the desugaring preserves the
task IR — a checking twin's parameter list is the discovered one
(`aCheckingParameterListIsTheDiscoveredOne`), so the lifted module lowers to the
twin's transfers and worker IR (`desugaringPreservesTheTaskIR`) and a refused
closure has no checking twin (`aRefusedClosureHasNoCheckingTwin`); the hidden
name is no identifier (`aHiddenNameIsNoSourceIdentifier`); non-vacuity on the
goal example's captures (`theExampleCapturesBindWhatTheTwinDeclares`). The
models are transcriptions of the Rust, bound to it by the test rows; the
sharing mode `share … under M` is outside the modelled closure modes, as it is
outside the implementation.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixture freeze | every existing file byte-identical, 85% | **as predicted**: 697 / 697 |
| existing tests changed | 0–2 files, 70% | **0** |
| twin identity | byte-identical C and certificate, 80% | **as predicted**, both examples and seven inline pairs |
| negative controls | each refused, naming the capture or name, 85% | **as predicted** |
| native incl. TSan | clean, 75% | **clean**: 9 sanitizer runs, 4 TSan rows, 0 reports |
| Lean | one module, ~8–15 theorems, 75% | one module, **19** theorems |
| corpus arbiter | identical to the base's archive, 95% | **identical** — to `2fc3af8c…` (the prediction cited the older archive; a correction above) |
| standing gate first run | green, 45% | **not green**: clippy 101 (two lints in new code); every other step 0 on the first run, rerun green |
| items | 1 (75%), 2 (70%), 3 (70%), 4 (15% / 10%) | **1–3 done; 4 not done** |

Not predicted: the parameter types could not come from the parser (a capture
list spells no type), so the lift is split between the parser (the hidden
declaration) and the task route (its types, by a discovery lowering of the
enclosing function); and that split leaves one narrow order difference from a
twin, for a module refused both in a closure body and before its spawn site
(`W48-QR-CLOSURE-FIRST-REFUSAL-ORDER`).

## Gate

[`gate.json`](gate.json), exits in [`gate/`](gate/), logs private to the
worktree (`private/w48-c4/gate`, `private/w48-c4/gate-rerun`), run by
`private/w48-c4/gate.sh` at `c408056a` and, after the clippy fix,
`private/w48-c4/gate-rerun.sh` at `4351007f` (each exit to a file, no step
piped):

| Step | Exit | Count |
|---|---|---|
| submodule (`--init --recursive`) | 0 | |
| `cargo fmt --all -- --check` | 0, rerun **0** | |
| `scripts/run_test_binaries.py` | 0, rerun **0** | 391 units, 404 summaries: **6,898** passed / 0 failed / 11 ignored (both runs) |
| `cargo clippy --all-targets -- -D warnings` | first run 101, rerun **0** | a capture-and-binder pair type; a needless borrow |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 (both runs) | |
| `scripts/check_lean_audits.py` | 0 | **249** audits (248 + `AuditClosures.lean`) |
| provider suite, `--no-fail-fast` (providers unchanged, run anyway) | 0 | 129 passed / 0 failed / 1 ignored, 43 summaries |
| matrix (`build_matrix.py --gate gate.json`) | 0 | `named_followups` 643 |
| docs index | 0 | |
| Python (`unittest discover -s scripts/tests`) | 0 | 365 tests |
| Clang | present | Ubuntu clang 21.1.8 ([record](gate/clang-version.txt)) |

## Corpus arbiter

[Identity record](artifacts/arbiter-corpus.identity). `scripts/run_pinned_audit.py`
at pin `66910c10` over the recorded Cycle-19 inputs, `--jobs 8`, release
`quirin-audit` (`a83a746f…`) built at `4351007f`: exit 0;
`resource-diagnostics.json` `2fc3af8c39f0cf04…`, **byte-identical** to the base
archive (CG08-CASTS-AND-TYPE-QUERIES's AFTER, merged into the base), so 602
Safe / 10 Unsafe / 4,222 Unknown; `resource-diagnostics.chunk-contracts.json`
and `monkey-source.json` byte-identical to every archive since card 0. No C
analysis path changed: the card's code is the source parser, the elaborator's
name resolution, the operator gate, the scoped-task route and the driver's
route gate.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py); a second run changes nothing): amends
`W48-QR-TASK-FFI-STRUCT-WORKER`; registers `W48-QR-CLOSURE-VALUES` (L),
`W48-QR-CLOSURE-EPOCH` (M), `W48-QR-CLOSURE-MUTEX` (M), `W48-QR-CLOSURE-ROUTES`
(S) and `W48-QR-CLOSURE-FIRST-REFUSAL-ORDER` (S): 638 → 643.

## Scripts

* Card 0's [`freeze_fixtures.py`](../w48-qr-scalar-types/freeze_fixtures.py)
  and [`compare_fixtures.py`](../w48-qr-scalar-types/compare_fixtures.py),
  unchanged.
* [`census_closure_spelling.py`](census_closure_spelling.py) — the
  constraint-14 census, run at the base and the tip.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
* The native summary: `QCORE_W48_CLOSURES_NATIVE_SUMMARY=<absolute path>
  cargo test -p qcore-compiler --test closures
  the_published_examples_run_clean_under_three_builds`.

