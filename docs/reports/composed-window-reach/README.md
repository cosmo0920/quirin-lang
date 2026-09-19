# `COMPOSED-WINDOW-REACH` (wave 41) — the composed route's window function, measured

Base `463fa6e8` (`= origin/main` at the card's start, confirmed by fetch).
Report directory: `docs/reports/composed-window-reach/`.

This is wave 41's soundness-adjacent card. It works four findings
`STABILITY-REEVAL-4` (wave 40) recorded about the loan-window machinery card
`LOOPS-ON-COMPOSED-ROUTES` landed in wave 39: `COMPOSED-VIEW-WINDOW-INERT`,
`WINDOW-FIELD-ARM-UNREACHED`, `LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION` and
`LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY`. The brief's instruction was to measure
before repairing, and to make a wrongly-admitted program the headline if one
existed.

---

## 1. Headline: no program was wrongly admitted, and that is the finding

**No program constraint 15h says must refuse is admitted.** Twenty-one probe
programs were compiled through the real pipeline *before any repair*, and every
one is refused at the code and by the mechanism the constraint names, or is
admitted exactly where the constraint says it should be. The probe harness and
its captured verdicts are shipped
(`probes/composed_window_probe.rs.txt`, `probes/verdicts.log`,
`probes/verdicts.exit` = 0); §2 reads them.

What is true is narrower than `COMPOSED-VIEW-WINDOW-INERT` states, and worth
the card:

1. **The residual's title is wrong and its content is right.**
   `std_string::view_uses_at_any_depth` does **not** return the empty vector for
   every input. It returns a view name at every position a view is read from —
   a slice, a `len`, a text render, a `concat` piece — and **those names are
   what the composed route's window is computed from**. What is empty for every
   input is the inner `buffer_view_uses`, i.e. the **depth contribution**; so
   the function is extensionally its own shallow half, and the operative
   consequence is the narrow one: **a `for` contributes nothing to any view's
   window.** (The residual's `why` field says the right thing about
   `buffer_view_uses`; the title generalises it to the outer function. Both are
   quoted and corrected in §9.)

2. **On the two routes the window functions have different standing.** On the
   **buffer** route `last_uses` is the deciding rule and its "at any depth" is
   live. On the **composed string** route the window decides at **top level
   only**; its depth half is inert, and it is **defence in depth rather than
   the deciding rule**, the deciding rule being `check_string_program`'s
   `Repeat` arm. Measured in §4, not asserted.

3. **The inertness is safe because of three mechanisms, not one** — and the
   third is a *rendering* rule doing a window's work, unnamed before this card
   and removable by a card that never reads constraint 15h. Registered as
   `COMPOSED-BODY-BARE-RENDER-INCIDENTAL`.

4. **One arm was genuinely fail-open-shaped** and is now closed: the window
   function's four field-path arms silently contributed nothing, correct only
   because two other refusals run first. They now refuse by name at the window
   function's own gate.

---

## 2. What the route judges today, and by which code path

`probes/verdicts.log`, produced by `probes/composed_window_probe.rs.txt` run as
a `compiler/tests/` harness at `c4daf585` (the report skeleton, i.e. the base
tree's code). The harness printed a verdict rather than asserting one, so that
the measurement could not be shaped by an expectation.

### 2.1 The composed string route

| probe | program | verdict | code | by which path |
|---|---|---|---|---|
| P1 | bare `print head;` **inside** a `for` body, owner store into the loan's bytes beside it | REFUSED | `Q077` | **the buffer route's line reader** — "`print` names a `std.fmt` rendering: `dec` or `hex`" |
| P1b | the same with no owner store | REFUSED | `Q077` | same |
| P1c | the same, owner store after the loop | REFUSED | `Q077` | same |
| P2 | owner store into the loan's bytes in a body, view read after the loop | REFUSED | `Q078` | `loop_body_conflict` — the window is right and the body is judged under it |
| P3 | inline-slice render `print head[0:1];` in a body | REFUSED | `Q077` | `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY` |
| P4 | bare `log info head;` in a body | REFUSED | `Q077` | the buffer route's line reader, again |
| P5 | control: owner store then a top-level `print head;` | REFUSED | `Q078` | the top-level window |
| V1 | a view at a `dec(…)` `concat` piece | REFUSED | `Q072` | the crossing rule (`STRING-SLICE-IO`) |
| V2 | a view at a `print dec` operand | REFUSED | `Q072` | same |
| V3 | a slice of the view read after an owner write | REFUSED | `Q078` | the window, reaching through **one** derivation |
| V4 | a slice of a slice | REFUSED | `Q078` | the window, through **two** |
| V5 | a `len` of the view after an owner write | REFUSED | `Q078` | the window |
| V6 | control: `concat` launders, owner write after it | **ADMITTED**, `out: abx` | — | the window closed at the composition, correctly |
| V7 | a nested-loop body renders an inline slice | REFUSED | `Q077` | the vocabulary scan, descending |
| R3 | an owned-field record in a composed `for` body | REFUSED | `Q077` | `LOOP-WINDOW-BODY-CUSTODY` |

**The distinction the brief asked for.** P2 is "refused by `loop_body_conflict`
because the window is right": the view is read *after* the loop, so the loan is
live at the loop's own index, and the body store is judged against it before the
body runs. P1/P1b/P1c/P4 are the other kind — **refused by something else
entirely, with the window contributing nothing**: the window closed at the
borrow (nothing after the loop reads the view), and the refusal comes from the
buffer route's rendering rule.

The hole P1 probes is the one that matters, because it is the exact shape the
inert recursion would have to cover: *a view whose only use is inside a `for`
body*. It is not expressible. Two independent refusals close it, and they are
**not the same refusal**:

* a body statement that **constructs** a view — a quoted literal, a `len`, a
  `concat`, an inline-slice render, a `borrow_utf8` — is claimed by
  `source_std_string::body_string_use` and refused at `Q077`
  (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`, `LOOP-PER-ITERATION-LOAN`);
* a **bare name** at a rendering operand is **deliberately not** claimed
  (`StringUse::Render`'s own documentation: a bare name must not pull a module
  that never mentions a string onto this route, so `print bogus;` keeps
  refusing at `Q030`). It falls through to the buffer reader, which demands a
  `dec`/`hex` selector. **That is a rendering rule, not a window rule.**

### 2.2 The buffer route

| probe | program | verdict |
|---|---|---|
| B1 | body renders the loan binding as a scalar | REFUSED `Q055` |
| B2 | loan loaded in a body, owner writes its bytes **after** the loop | ADMITTED — correct: nothing reads the loan after the loop |
| B3 | loan used in a body, owner writes its bytes **inside** the body | REFUSED `Q054` (`cannot mutate owner z while it is borrowed`) |
| B4 | `extent(l)` read in a body | ADMITTED |
| R1 | a **plain** (all-`i32`) record beside a lend | **ADMITTED** |
| R2 | a plain record inside a `for` body of a lending entry | **ADMITTED** |
| S1 | rebinding a scalar at top level | REFUSED `Q012` (duplicate local binding) |
| S2 | a lend bound through a scalar rebound from a byte | REFUSED `Q012` |
| S3 | the same through an `extent` rebinding | REFUSED `Q012` |

S1–S3 were probing a specific suspicion: `lend_windows`' scalar environment has
arms for `Bind`, `Acquire`, `Extent` and `Lend` and **none** for `Load` or
`Field`, so a name rebound from a byte read would keep a stale constant and a
lend bound could resolve through it instead of refusing at
`WRITE-LEND-BOUND-FROM-BYTE`. Name resolution closes it first (`Q012`), so
there is no gap — recorded because the absence of the arms is real and the
safety comes from elsewhere.

R1 and R2 are the measurement that corrects `WINDOW-FIELD-ARM-UNREACHED`; see
§5.

---

## 3. The repaired window

### 3.1 `std_buffer.rs` — a name the key space cannot express is refused

The window map is keyed by **plain binding names**, because that is the only
shape a lend binding has: `let NAME = lend(BUFFER, LOW, HIGH);` binds one
identifier. Card `AGGREGATE-OWNED-FIELD`'s four field-path events address a
buffer as `RECORD.FIELD`, which that key space cannot hold. Before this card
they sat in the same arm as `Bind` and `Line` and contributed nothing.

`custody_names_at_any_depth` now **classifies** what it meets:

```rust
enum WindowName<'a> {
    Binding(&'a str),
    FieldPath { record: &'a str, field: &'a str },
}
```

and the new `pub fn window_unnameable_events(events) -> Vec<(usize, String)>`
reports every top-level statement whose events, **at any depth**, name a field
path. `lend_windows` turns each into a `BufferError::Lend { refusal:
LendRefusal::Shape(…) }` — `Q051`, a condition extension of the band constraint
15h already names for the lend's window shapes, **no code allocated** (§7).
`last_uses` still skips a `FieldPath`, but now on the stated ground that the
gate refuses the program first, not on the ground that the construct is
unreachable.

**Direction.** This refuses more and admits nothing new, so invariant 1 is
satisfied by construction; §6's census confirms it empirically (0 verdicts
moved). It is a second, **independent** line: `OWNED-FIELD-LEND-WINDOW` and
`OWNED-FIELD-LOOP-BODY` still stand and still fire first
(`the_two_owned_field_refusals_still_precede_the_window_gate`), so lifting
either can no longer shorten a window by accident.

### 3.2 `std_string.rs` — the window recorded as what it is

No behaviour changed. `view_uses_at_any_depth`'s documentation is rewritten to
say what §1 and §2 measured: that the depth contribution is empty for every
input; that the operative consequence is that a `for` contributes nothing; that
this is safe because of three named mechanisms, the third incidental; and that
the recursion is **defence in depth and not the deciding rule**. A new
`pub fn view_window_names(event) -> (Vec<&str>, Vec<&str>)` splits the shallow
and depth halves so the inertness is a *tested equation over every constructor*
(`the_depth_contribution_is_empty_for_every_event`) rather than a sentence.

### 3.3 Which verdicts moved

**None.** §6's emission census: 535 programs, 146 admitted by both the base and
the card binary, **byte-identical**, 0 differing, 0 admitted by base only, 0 by
card only. The brief allowed for either outcome; this is the second one, and
`COMPOSED-VIEW-WINDOW-INERT` is discharged on its first disjunct accordingly —
the window on the composed route is recorded as defence in depth, in constraint
15h and in the code.

---

## 4. Which rows the window actually anchors (wave 39, re-measured)

`wave39_anchoring.sh`, log `wave39-anchoring.log`. Method: apply wave 39's own
M1 — the window function's `Repeat` arm emptied, so a loop contributes no name
— and run each of that card's 24 test rows.

**7 rows red, 17 green.** Every red row is a **buffer**-route lend row:

```
a_loan_used_only_inside_the_loop_is_live_across_it_and_the_body_write_refuses
a_loop_filling_a_lend_is_admitted_and_its_window_runs_through_the_loop
an_owner_write_before_the_loop_is_inside_the_window_and_one_after_it_is_outside
two_loans_last_used_inside_one_loop_nest_and_their_disjointness_is_still_decided
a_nested_loop_is_projected_by_the_same_rule_at_every_depth
a_body_index_read_from_a_byte_conflicts_with_every_live_loan
the_projection_unrolls_the_body_inside_the_borrow_node
```

**No composed-route row is anchored by it** — including
`a_body_write_into_a_borrowed_range_refuses_on_the_composed_route`, that card's
composed flagship, which stays green because the composed route reads
`view_uses_at_any_depth` and not `custody_names_at_any_depth`, and the former's
depth half was already empty.

That is the measurement behind §1's point 2. It does **not** mean the composed
window is inert: `W41-M9-composed-window-emptied` empties `last_view_uses`
outright and reddens `every_view_reading_position_keeps_the_owner_out`, so the
composed window decides at top level. The precise statement — now in constraint
15h — is that on the composed route the window decides at top level and its
*depth* half is defence in depth.

A green row under M1 is not thereby a tautology: 17 of them are about other
mechanisms (the body-versus-live-loans judgment, the by-name refusals, the
native sanitized runs) and are anchored by those. What the measurement rules
out is citing the window as the evidence for them.

---

## 5. The six aggregate arms — the residual's premise corrected

`WINDOW-FIELD-ARM-UNREACHED` counts **six** arms and says all six are
unreachable. Measured:

* **Two are reachable today.** `Construct` and `Field` are reached by a record
  whose fields are all `i32` — `source.record.v1`'s, card `AGGREGATES-V1`'s,
  which predates the owned field. R1 and R2 in §2.2 show such a record admitted
  **beside a lend** and **inside a loop body of a lending entry**; both reach
  `lend_windows` → `last_uses`. They are **correctly** empty: a scalar
  construction and a scalar field read name no buffer and no loan at all.
  Pinned by `a_scalar_record_reaches_the_window_function_and_names_nothing`,
  at source level and at model level.
* **Four are unreachable, and that is what the residual is about.**
  `FieldStore`, `FieldLoad`, `FieldExtent`, `FieldRelease` — card
  `AGGREGATE-OWNED-FIELD`'s. The ordering the residual measured is confirmed:
  `check_buffer_program` returns `OWNED-FIELD-LEND-WINDOW` for a program that
  composes an owned field and lends, and `OWNED-FIELD-LOOP-BODY` for an
  owned-field event in a `Repeat` body, both **before** the block that calls
  `lend_windows`.

**Is lifting either refusal load-bearing, and does it shorten a window?** Yes,
and yes in the admitting direction — which is why this card does not leave the
pair as a measured boundary. The gate in §3.1 makes the shortening impossible
*without touching the two refusals*: whoever lifts one meets a second,
independent fail-closed gate at the window function itself, whose message names
`WINDOW-FIELD-ARM-UNREACHED` and whose repair is to give the four arms real
windows. That is the residual's own second disjunct ("the arms must be made an
explicit refusal rather than an empty vector"), met word for word.

What is **not** done — and is registered as
`WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED` — is giving a field path a window. That
needs a key space that holds a path, through `LendWindow`, the custody
projection's segment walk (which addresses owners by plain name, and is
`OWNED-FIELD-LEND-WINDOW`'s own stated ground) and `crossed_lend_windows`. The
card that lifts either refusal must land all of it together and retire the gate
deliberately.

---

## 6. Emission census

`measure/census.sh` (worktree-private) builds two `quirin` binaries — one from
`463fa6e8`'s `std_buffer.rs` and `std_string.rs`, one from this card's — and
runs wave 39's own shipped `emission_census.py` over every `.qcore` file in the
tree. Output: `emission-census.json`.

```json
{
 "programs": 535,
 "admitted_by_both_byte_identical": 146,
 "admitted_by_both_differing": 0,
 "admitted_by_base_only": 0,
 "admitted_by_card_only": 0,
 "refused_by_both": 389
}
```

Every previously admitted program is admitted, with byte-identical emitted C.
No verdict moved in either direction. That is the expected result of a gate
that is unreachable from source, and it is published as the measurement rather
than argued from the code.

---

## 7. Diagnostics: one condition widened, no code allocated

The `Q-NAMESPACE-DECISION` icebox stands and this card allocates nothing.

* **`Q051` widens.** A statement whose name the window's key space cannot
  express — a field path met by `custody_names_at_any_depth` in an entry that
  lends — is refused there. `LendRefusal::Shape` already maps to `Q051`, and
  `Q051` is the band constraint 15h names for the lend's window shapes; this is
  the same condition reached from the window function rather than from a source
  spelling, the `SOCKET-EVENT-COMPOSE` precedent.
* **Nothing else moves.** No other `Q051`–`Q057` condition, and `Q056` is again
  not widened.
* Stated in `docs/QCORE_SPEC_r0_3.md` §L21 (the `Q051`–`Q057` row's new
  amendment paragraph) and in `AGENTS.md`'s namespace table row.

**Constraint 14 census: +0 words, +0 tokens, and the question does not arise.**
This card adds no source syntax at all — no new spelling reaches the lexer, the
parser or any route selector. The one new public Rust symbol pair
(`window_unnameable_events`, `view_window_names`) is library API, not language
surface. `is_reserved_source_word`'s count is unchanged at **113**.

---

## 8. The Lean binding, and its stated gap

### 8.1 The tautology, superseded

`QCore.LoopWindows` §8 carried:

```lean
inductive AdmittedInBody where | access | plain
def admittedInBody : List AdmittedInBody := [.access, .plain]
theorem the_admitted_body_shapes_are_exactly_these (s : AdmittedInBody) :
    s ∈ admittedInBody := by cases s <;> simp [admittedInBody]
```

— true of *any* two-constructor type and any list of both its constructors,
decided by the shape of the declaration, silent about the route. That is
`LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY`, and it is **the one tautology this
card found in wave 39's work** (§4 audits the Rust rows; see §8.4).

It is replaced, not deleted, by a decision over the alphabet the implementation
produces.

### 8.2 `LendEvent` — the alphabet with a loan in it

`the_unrolling_is_loop_v1s`, the only theorem tying the model to `LOOP-V1`, is
stated at `QCore.StdBuffer.Event`, whose eight constructors are
`bind`/`acquire`/`store`/`load`/`extent`/`release`/`line`/`ret` — **no lend**.
The Rust `BufferEvent` has sixteen. New in §8 of the Lean module:

```lean
inductive LendEvent where
  | names (name : String)     -- Store/Load/Extent/Release/Lend
  | fieldPath                 -- the four Field* events
  | quiet                     -- Bind/Acquire/Construct/Field/Line/Return
```

with `usesLoan` (the Rust `WindowName::Binding` test), `unnameable` (the Rust
`window_unnameable_events` test) and `admittedInBody` as a *decision*, and:

| theorem | content |
|---|---|
| `every_admitted_body_event_is_windowed` | every event a body may hold is one the window can account for — falsified by admitting `fieldPath`, which is `WINDOW-FIELD-ARM-UNREACHED`'s shape |
| `the_unnameable_events_are_exactly_the_inadmissible_ones` | the two definitions cannot drift apart |
| `an_unnameable_event_uses_no_loan` | the precise sense in which forgetting one shortens a window |
| `the_lend_alphabets_unrolling_is_loop_v1s_recursion` | a loop's unrolling at `LendEvent` is its body's, flattened out of a replication |
| `loop_v1s_unrolling_is_the_same_flattening` | `LOOP-V1`'s own `repeatAppend` is that same right-hand side — so this is not a second unrolling that agrees on an alphabet with no loan |
| `every_lend_use_lies_inside_the_window` | §3's soundness theorem instantiated where a lend can be named |

### 8.3 The binding: a differential test over shared vectors

The link between `lastUse` and `last_uses` was a doc comment. It is now
mechanical, in both directions:

* `QCore.LoopWindows.agreement` holds **eight rows**, each a program, its
  one-line `spelling` and the window `lastUse` gives for the loan `l`. The
  grammar is four characters plus `[N…]` for a loop.
* `the_agreement_table_holds : tableHolds = true := by decide` proves, for
  every row, that the spelling **is that row's program's own rendering**
  (`renderList`) *and* that the number **is `lastUse`'s answer**.
* `compiler/tests/composed_window_reach.rs` reads
  `proof/QCore/LoopWindows.lean` **as text**, extracts every row's spelling and
  number, rebuilds the spelling as a `Vec<BufferEvent>`, and asserts
  `qcore_core::std_buffer::last_uses` returns the Lean number.
* `the_table_contains_a_body_only_use` (Lean) and the Rust test's tail both
  pin that the table keeps a row whose loan is used **only inside a loop body**
  — without one, every row would also agree with a window that ignores loops
  and the differential would be vacuous.
* `the_lean_module_names_its_rust_differential_test` keeps the two files
  pointing at each other.

Non-vacuity in both directions is measured, not claimed: `W41-M6` moves one
row's **number** in the Lean file and the Rust test goes red; `W41-M7` moves one
row's **spelling** and the Lean build fails.

### 8.4 The gap, stated

`LOOPWINDOWS-BINDING-IS-FINITE`. The binding is **eight points**, not an
extraction: nothing here says `last_uses` and `lastUse` agree on a ninth
program. The vectors were chosen to exercise a flat last use, a body-only use,
a nested-body use, a loop naming another binding, a use before *and* inside a
loop, and a program with no use — but a divergence on a shape outside that set
would be invisible. The Rust test guards the table's size (`>= 8`), so shrinking
it is a red row rather than a silent loss. **Proved:** properties of the Lean
model, at an alphabet the implementation produces. **Observed, on eight
points:** that the Rust function is that model.

---

## 9. Non-vacuity: ten mutations, and which wave-39 tests were tautologies

`mutations.sh`, log `mutations.log`. Each mutation edits one arm, runs the
named test, expects it RED, restores by `mv` + `touch` (never mtime-preserving,
`MUTATION-HARNESS-STALE-ARTIFACTS`) and checks sha256 equality. The Lean half
additionally deletes the module's `.olean` so `lake` cannot serve a stale one.
Both baselines — the Rust test file and the Lean build — are proved green
**before the first mutation and after the last restore**.

| mutation | what it undoes | named test | result |
|---|---|---|---|
| `W41-M1-field-arms-silent` | the four field-path arms back in the empty arm (the state the residual measured) | `a_field_path_is_refused_by_the_window_functions_own_gate` | RED |
| `W41-M2-gate-not-consulted` | the gate removed from `lend_windows` | same | RED |
| `W41-M3-shallow-window-emptied` | `view_uses`' `Slice`/`Len` arm emptied | `every_view_reading_position_keeps_the_owner_out` | RED |
| `W41-M3b-shallow-pin` | `view_uses`' `Text`/`Concat` arms emptied | `the_depth_contribution_is_empty_for_every_event` | RED |
| `W41-M4-depth-contributes` | the depth recursion made to contribute a name | same | RED |
| `W41-M9-composed-window-emptied` | `last_view_uses` returns an empty map | `every_view_reading_position_keeps_the_owner_out` | RED |
| `W41-M5-body-vocabulary-skips-render` | mechanism 2 no longer claims a rendering operand | `a_view_constructing_statement_in_a_composed_body_is_refused_by_name` | RED |
| `W41-M6-lean-vector-number` | one shared vector's number moved **in the Lean file** | `the_lean_window_model_and_the_rust_window_function_agree_on_shared_vectors` | RED |
| `W41-M7-lean-vector-spelling` | one shared vector's spelling moved | `lake build QCore.LoopWindows` | FAILS |
| `W41-M8-body-admits-field-path` | `admittedInBody` admits `fieldPath` | `lake build QCore.LoopWindows` | FAILS |

All ten non-vacuous; all four files restored sha256-equal.

**Which of wave 39's tests were tautologies.** One: the Lean theorem
`the_admitted_body_shapes_are_exactly_these` (§8.1), superseded here. The Rust
rows were audited and none is: §4's sweep separates the 7 rows the window
anchors from the 17 it does not, and the 17 are anchored by other measured
mechanisms — `the_projection_unrolls_the_body_inside_the_borrow_node` pins
`last_uses` to a literal `Some(2)` and the body's footprints to
`[(2,3),(3,4),(4,5)]`; the admitted-shape rows compare an observed stdout
(`a_loop_filling_a_buffer_before_it_is_borrowed_is_admitted` asserts `["aa"]`);
the by-name refusal rows assert a code **and** a residual name in the message.
What the sweep does establish is that none of the **composed-route** rows is
evidence about the window function, which is a claim wave 39's report and
constraint 15h both made and which is corrected here rather than in a test.

---

## 10. Gate

`measure/run-gate.sh`, detached (`setsid -f`, no `trap '' TERM`), every exit to
its own file and never through a pipe, polled by a bounded foreground loop.
Exits copied to `gate/` beside `gate.json`. `clang --version` was run **in this
worktree** with `QCORE_CLANG` unset; the recording is this card's own
(`CLANG-RECORDING-UNBOUND-TO-ITS-RUN` — the generator cannot tell, so the
honesty is stated here).

See `gate.json` for the figures and `gate/` for the exits.

---

## 11. Residuals: retirements and registrations

`register_followups.py`, idempotent. **`named_followups` 257 → 256** (four
retired, three registered). Each retirement's discharge condition was read in
full first and is quoted here.

### Retired

| name | its discharge condition | met by |
|---|---|---|
| `COMPOSED-VIEW-WINDOW-INERT` | "amend 15h to attribute the composed route's repair to `check_string_program`'s `Repeat` arm and record `view_uses_at_any_depth` as written-ahead-of-its-grammar" | constraint 15h's wave-41 amendment and the function's own rewritten documentation — plus more than was asked: the residual's mistaken **title** corrected (§1), the two routes' different standing measured (§4), and the closure's three mechanisms named |
| `WINDOW-FIELD-ARM-UNREACHED` | "…**or** the arms must be made an explicit refusal rather than an empty vector" | §3.1 and §5 — for the four field-path arms; the two the residual miscounted are measured reachable and correctly empty |
| `LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION` | "a correspondence mechanism (… a differential test over shared vectors …) binding `last_uses` to `lastUse`; **OR** an instantiation of the LOOP-V1 theorem at an alphabet carrying Lend" | **both** — §8.2 and §8.3 |
| `LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY` | "restate the theorem's entry … **OR** relate `AdmittedInBody` to the Rust admitted set by a checked correspondence" | the card took the stronger route and **superseded the theorem outright** (§8.1). The residual's stated concern — "so the theorem is not later cited as coverage of the route's admitted set" — is gone because the theorem is; what stands in its place is falsifiable (`W41-M8`) |

### Registered

| name | in one line |
|---|---|
| `COMPOSED-BODY-BARE-RENDER-INCIDENTAL` | a bare `print s;` in a composed body is kept out by a **rendering** rule, not a window rule; discharge is to claim it in `body_string_use`, or make the recursion live, or record the incidental closure as permanent |
| `WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED` | the gate **refuses** a field path rather than windowing one; the card that lifts either owned-field refusal must give the four arms real windows and retire the gate deliberately |
| `LOOPWINDOWS-BINDING-IS-FINITE` | the Lean↔Rust binding is eight shared vectors, not an extraction; discharge is an extraction, a generated digest, or a property-based differential |

---

## 12. Files, and the shared-file note

Owned by this card per the wave's file-ownership list, and all this card
touched in crate source:

* `crates/qcore-core/src/std_buffer.rs` — **shared with `OWNED-FIELD-ROUTE-REACH`
  by design**, which owns that file's *record/field arms*. This card touched
  only the **window functions**: `custody_names_at_any_depth` (its classification
  and the four field-path arms *as the window reads them*), `last_uses`, the new
  `window_unnameable_events`, and `lend_windows`' call to it. It did **not**
  touch `check_buffer_program`'s record handling, `owned_field_event`,
  `RecordType`, `OWNED-FIELD-LEND-WINDOW` or `OWNED-FIELD-LOOP-BODY` — those two
  refusals are read and pinned (§5, `the_two_owned_field_refusals_still_precede_the_window_gate`)
  and deliberately left where they are. **A note for that card:** if it lifts
  either refusal, §5's gate becomes reachable and
  `WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED` becomes its work.
* `crates/qcore-core/src/std_string.rs` — `view_uses_at_any_depth`'s
  documentation and the new `view_window_names`. No behaviour change.
* `proof/QCore/LoopWindows.lean`, `proof/AuditLoopWindows.lean`.
* `compiler/tests/composed_window_reach.rs` (new).

Not owned and not edited: `crates/qcore-frontend/src/source_std_string.rs` is
**read and mutated-and-restored** by `mutations.sh` (`W41-M5`) but is unchanged
in the committed tree.

---

## 13. For the integrator

Every item is an item; each is already applied in this card's own tree in the
words below unless marked otherwise.

1. **`AGENTS.md` constraint 15h** — a new amendment paragraph, card
   `COMPOSED-WINDOW-REACH`, appended after the `LOOPS-ON-COMPOSED-ROUTES`
   amendment's census sentence. It narrows a clause that amendment carries and
   adds a refusal; it supersedes nothing else. Its three parts: the two routes'
   window functions have **different standing** (7 of wave 39's 24 rows anchored
   on the buffer route, **none** on the composed one); the closure rests on
   **three** mechanisms, the third a rendering rule doing a window's work; and a
   name the window's key space cannot express is now **refused** at `Q051` by
   the window function's own gate. `composed-window-reach` added to that
   constraint's evidence list.
2. **`AGENTS.md` diagnostics table, `Q051`–`Q057` row** — one sentence: `Q051`
   also covers a statement whose name the window's key space cannot express,
   unreachable from source today, no other condition on the row moving.
3. **`AGENTS.md` `Q-NAMESPACE-DECISION` tally** — wave 41 adds
   `COMPOSED-WINDOW-REACH`'s one widening, taking the running count to
   **nineteen** on the same ten bands. The wave's other cards are counted at the
   close, which should **re-derive** the total rather than adding to that
   sentence.
4. **`AGENTS.md` `named_followups` count: 257 → 256.** Four retired, three
   registered, by this card. The sentence names this card and says the wave's
   remaining cards and its close move it again — please re-derive at the close.
5. **`docs/QCORE_SPEC_r0_3.md` §L21**, `Q051`–`Q057` row — a new amendment
   paragraph stating the `Q051` widening, its ground (the key space is plain
   bindings), the two refusals that still precede it, the 535-program census and
   the successor residual.
6. **`docs/development/increments/wave-41.md`** — created in wave-40.md's shape;
   `docs/development/increments/README.md`'s current-record pointer moved to it
   and a wave-41 row added at the top of the wave table. The integrator should
   union siblings' paragraphs into this file at the merge.
7. **Headline numbers this card moves:** only `named_followups` (item 4).
   **Nothing else.** No corpus count, unit status or function status moves; raw
   units stay unpromoted; CT counts untouched; full language conformance stays
   **false**. The Lean audit file count rises by **0** — `AuditLoopWindows.lean`
   already existed and was edited, not added.
8. **A cross-card note, not a ledger item.** `OWNED-FIELD-ROUTE-REACH` owns
   `std_buffer.rs`'s record/field arms and this card owns its window functions;
   §12 says exactly which lines each touched. If that card lifts
   `OWNED-FIELD-LEND-WINDOW` or `OWNED-FIELD-LOOP-BODY`, this card's gate becomes
   reachable and `WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED` is the work that must
   land with it.
9. **A finding to carry, not to file quietly.** `COMPOSED-VIEW-WINDOW-INERT`'s
   registered **title** is wrong (§1). It was registered by `STABILITY-REEVAL-4`
   and is retired by this card, so nothing needs editing — but the shape is worth
   the next `STABILITY-REEVAL`'s attention: a residual whose `why` is accurate
   and whose title over-generalises reads, at a glance, as a live admitting gap.
