# `OWNED-FIELD-ROUTE-REACH` (wave 41) — the reach of wave 39's owned-field rules, measured, then closed or named

**Card:** `OWNED-FIELD-ROUTE-REACH` · **Wave:** 41 · **Base:** `463fa6e8`
(`origin/main` at branch time, confirmed equal by `git rev-parse`) ·
**Date:** 2026-09-19.

**Headline.** The five owned-field rules were measured for reach **before
anything changed**, by a script this report ships and by the same script run
against a `git archive` of the base: four are driven by real programs through
the real pipeline, and the fifth — **non-copyability** — was reached by
*nothing*, `OwnershipOp::Copy` being constructed at four places in the tree,
every one a unit test. The card gives the question a source spelling. **The copy
bind**, `let NAME = NAME;` on `source.buffer.v1`, builds `Copy` for every name
the ownership ledger holds a binding for and **no operation at all** for every
other: an owned buffer, an owning record hollow or not, a moved binding, a
dropped one and a write loan refuse at `Q054` with the inherited checker's own
words; a scalar and a record of scalars are **admitted**, which is the card's one
new admission and the only place invariant 1 is engaged — the argument for it is
proved, not asserted (`QCore.OwnedFieldCopyReach`). The three boundaries
`STABILITY-REEVAL-4` measured as **absence rather than refusal** now refuse
**by their own names**, each at the code, span and refused set it already had.
**Zero words reserved, zero tokens, zero codes**; the census found a collision —
`source.socket.v1`'s `let listener = tcp;` is the identical token shape, 29 times
in the tree — and the card kept the collision and pinned the separation rather
than reshaping either grammar. 19 end-to-end tests, 14 mutations, **and the
harness earned its keep: its first run reported one VACUOUS**, which a second
test turned into a real measurement. Three residuals retired by exact name, five
registered, four amended because this card made their text false.

Sections: 1 the reach table · 2 what became reachable, and by which spelling ·
3 the boundaries · 4 non-vacuity · 5 what did not move, and the census ·
6 Lean · 7 retirements and residuals · 8 gate · 9 for the integrator ·
10 file ownership.

---

## 1. The reach table, measured before anything changed

`scripts/reach.py` writes **29 whole `.qcore` programs** into an output
directory and hands each to the built `quirin check`, recording exit code,
first diagnostic code and first message to files — never through a pipe. It was
run twice: against this tree, and against a `git archive 463fa6e8` extraction
built in its own scratch directory, whose results `scripts/reach_delta.py`
pairs. **The scratch tree was removed immediately after the run**
(`measure/base-tree`, gitignored — wave 39 lost a gate row to one it left in
place; `find measure -name '*.qcore'` was re-run afterwards to confirm nothing
of that tree remained).

Reproduce:

```sh
cargo build -p qcorec
python3 docs/reports/owned-field-route-reach/scripts/reach.py . --out measure/reach
python3 docs/reports/owned-field-route-reach/scripts/reach_delta.py \
    measure/reach-base/reach.json measure/reach/reach.json
```

`reach.txt` and `reach-delta.txt` beside this file are the run's own output.
**7 of 29 probes moved**; every one of the seven is in the copy family, and
every other row — the four rules that already worked, the boundaries, the
IR-only refusals, the collision probes — is unchanged.

### 1.1 The five rules, and what drives each

| Rule | The `.qcore` shape that produces the operation | Admitted at the base? | Driven by, at the base | Driven by, now |
|---|---|---|---|---|
| **1. construction moves in** (`Compose`) | `let f = Frame { body: b };` over an acquired `b` | yes | a source program: `examples/records/owned_field.qcore`, pinned in `aggregate_owned_field.rs`; probe `r1-*` | unchanged |
| **2. partial move leaves a hole** (`MoveOut`) | `let taken = f.body;` at a `bytes` field | yes | a source program; probe `r2-*` | unchanged |
| **3. drop-through in declaration order** (`Drop` on an aggregate) | `drop f;` | yes | source programs; probe `r3-*` | unchanged |
| **4. leak at exit names the path** (`NormalPathLeak`) | a construction with no `drop` | n/a (refusal) | a source program; probe `r4-refused-leak` | unchanged |
| **5. non-copyability** (`Copy`) | **nothing** — no route built the operation | — | **four unit tests in `crates/qcore-core/tests/unit/ownership.rs` and nothing else** | **`let g = f;` through the real pipeline** (probes `r5-copy-owning-record`, `-hollow-record`, `-buffer`, `-moved-record`, `-loan`) |

Row 5 is `STABILITY-REEVAL-4`'s `OWNERSHIP-COPY-OP-UNPRODUCED`, re-verified by
this card before being relied on: `rg -n 'OwnershipOp::Copy' --type rust crates
compiler` finds four construction sites, all in that unit file, plus one
*consumer* in `whole_program.rs`'s exhaustive no-identity match.

### 1.2 The seven rows that moved

| Probe | Base | Now | What it means |
|---|---|---|---|
| `r5-copy-owning-record` | `Q051` | `Q054` | `owned resource `f` cannot be copied` — the rule, reached |
| `r5-copy-hollow-record` | `Q051` | `Q054` | same message after a field moved out: "hollow or not", reached |
| `r5-copy-buffer` | `Q051` | `Q054` | the **older plain-owner** copy rule, also reached for the first time |
| `r5-copy-moved-record` | `Q051` | `Q054` | `use of `f` after drop` — the IR's *state* rules answer too |
| `r5-copy-loan` | `Q051` | `Q054` | `borrow `l` cannot be moved as an owner` |
| `r5-copy-scalar-record` | `Q051` | **ADMITTED** | constraint 15k's "a plain copyable value", made checkable |
| `r5-copy-scalar` | `Q051` | **ADMITTED** | a scalar copy on the same route |

Five refusals became **better answers to the same question** (a shape refusal
becoming a custody refusal, the wave-25 precedent); two became admissions, and
§2.3 is their invariant-1 argument.

### 1.3 The rows that did not move, and why that is the measurement

| Probe | Verdict, unchanged | Reading |
|---|---|---|
| `r5-copy-shadowing` | `Q012` | name resolution refuses a duplicate `let` **before** this route; the IR's `BindingAlreadyExists` arm stays model-level |
| `r5-copy-self` | `Q013` | `let q = q;` is an unknown name, also before this route |
| `r5-copy-unbound` | `Q013` | so the value half's "nothing this route bound" arm is a backstop, not a reachable message |
| `r5-copy-composed-string` | `Q068` | the composed string route lowers no copy bind at all (§5.1) |
| `collide-socket-creation` | ADMITTED | a socket program is untouched (§5.2) |
| `collide-buffer-route-transport-word` | `Q015` | a transport word at a copy source on the buffer route refuses at the import gate, before the route |
| `b-view-field`, `b-nested-*`, `b-unknown-field-type` | `Q051` | the boundary work is **message-only**: same code, same refused set (§3) |
| `b-task-capture` | `Q500` | likewise; `b-task-capture-baseline` is the same module minus the record and is ADMITTED, so the refusal is attributable |
| `ir-aggregate-at-owner-lend`/`-index`, `ir-not-an-aggregate` | `Q051`/`Q052`/`Q052` | the IR refusals stay unreached, deliberately (§3.4) |

## 2. What became reachable, and by which spelling

### 2.1 The statement

```
stmt ::= … | "let" NAME "=" NAME ";"          -- the copy bind
```

One statement asking one question: **may a second binding hold what this name
holds?** It is read by `source_std_buffer::lower_let` as the last arm before
that function's shape refusal, so it claims nothing another form wants —
`bytes`, `extent` and `lend` take a `(`, a byte read a `[`, the two record forms
a `{` or a `.`, a literal bind a digit or a `-`.

### 2.2 The decision, and where each half lives

* **The custody walk** (`std_buffer::custody_operations`, and the
  lend-structured projection beside it) pushes `OwnershipOp::Copy(source)`
  **exactly when the source is a name the ledger holds** — an acquired buffer, a
  binding a partial move gave a buffer to, a record composed with an owned
  field, or (in the lend projection) a loan. For every other name it pushes
  **nothing**.
* **The ownership IR** answers, and always with a refusal: `CopyOfOwned` for an
  owner and for an aggregate, `MoveOfBorrow` for a loan, `UseAfterMove` and
  `UseAfterDrop` for a binding already moved or dropped. `copy_owned` is
  unedited by this card.
* **The value half** copies a scalar into the scalar environment and a record
  value into the record environment, and **says nothing at all** about a
  ledger-held name. That is deliberate and was corrected during the card: an
  earlier draft also refused a loan at `Q055`, and the measurement showed the
  program receiving **two diagnostics for one fault** (`Q054` from the IR and
  `Q055` from the value half). The custody refusals of a copy arrive from the
  IR exactly as a lend's do (standing constraint 15h), so the `Q055` arm was
  removed.
* **Emission** writes `int32_t s_q = s_p;` for a scalar and
  `struct qcore_record_Point r_q = r_p;` for a record — one C struct
  assignment, by value, so the two bindings are two C objects and a later read
  names its own member. The record half lives in `emit_events`, which carries
  the record environment, exactly as `Construct` does; `buffer_event_statement`
  refuses it as `UnemittableRecord` for that same reason.

Measured end to end (`measure/reach-after/*.c`, produced by
`scripts/reach.py` plus `quirin emit`): both admitted probes compile under
`cc -std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined` at exit 0 and run at exit 0 with empty stderr
under `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1`, printing what the
reference observation predicts.

### 2.3 Invariant 1: the one thing this card admits that was refused before

`let NAME = NAME;` was a `Q051` shape refusal at the base for **every** source.
It is now admitted when the source is a check-time-constant scalar or a record
of a type declaring no owned field. Stated plainly, and argued:

* **What refuses the bad cases** is the ownership IR, not a route rule, and the
  route asks it about every name the ledger holds. There is no third class: a
  name is in the bindings or it is not (`the_decision_is_total`,
  `nothing_is_both_admitted_and_refused`).
* **Why the admitted class is safe**: a place the ledger holds no binding for
  **owns no resource**, so an admitted copy receives no duty and the step is the
  identity on the ledger. Proved in Lean, under the Rust checker's own
  ledger-coherence invariant named as a premise
  (`an_admitted_copy_owns_no_resource`,
  `an_admitted_step_is_the_identity_on_the_ledger`,
  `a_copy_either_refuses_or_moves_no_duty`; §6).
* **Nothing else widened.** Route selection did not move — the statement is
  deliberately not a `buffer_use` (§5.1). The lowering's constant tier did not
  move: a loop bound or a lend footprint spelled through a copy is refused at
  its own condition (`Q098`, `Q053`), the `LOOP-BOUND-FROM-BYTE` shape, pinned
  by `a_bound_spelled_through_a_copy_is_refused_rather_than_followed` and
  registered as `COPY-BIND-BOUND-TIER`. The *checker's* tier does follow a copy —
  both custody projections carry a copied scalar's value forward, so an **index**
  written through a copy resolves to the constant it is rather than falling back
  to the rangeless form, which is strictly a sharpening (the fallback refuses).
* **Effects**: a copy bind derives nothing, since an admitted copy binds a value
  that already exists (`a_copy_bind_derives_no_effect`).

## 3. The boundaries: three now refuse by name, two are recorded as absence

### 3.1 `AGGREGATE-VIEW-FIELD` — refuses by name

`type Frame { s: string }` reached `FieldType::from_spelling`, returned `None`,
and refused as `UnknownFieldType` with a message naming **`AGGREGATE-FIELD-TYPES`,
a different residual**. A new `RecordRefusal::ViewFieldType` now carries its own
sentence — a view is `{storage, offset, len}` borrowed for an exact lexical
window, and an aggregate has nowhere to carry that window's end — at the **same
code** `Q051`. Pinned by `a_view_field_refuses_by_its_own_name`, which asserts
both that the boundary names itself and that it names *only* itself.

### 3.2 `AGGREGATE-NESTED` — refuses by name, in either declaration order

`type Outer { inner: Inner }` takes `RecordRefusal::NestedFieldType`, detected
against the module's **declared record names** rather than against the type
table being built — so a forward reference (`Outer` written above `Inner`) is
the same refusal as a backward one, pinned in both orders by
`a_nested_record_field_refuses_by_its_own_name_in_both_declaration_orders`.
Every other spelling (`u8`, `Buffer`, `i64`) keeps the generic message and the
residual it always named
(`every_other_unknown_field_type_keeps_the_message_it_had`). The classification
lives in **one** function, `source_record::field_type_refusal`, so a reader and
a later route cannot disagree about which residual a spelling meets.

### 3.3 `AGGREGATE-TASK-CAPTURE` — refuses by name, and the refusal already existed

`STABILITY-REEVAL-4` recorded this one as "pure absence". The measurement here
is finer: a record type declared in a checked-concurrency module **already
refused**, at `Q500`, in the task route's declaration walk — the refusal existed
and named no boundary. It now names `AGGREGATE-TASK-CAPTURE`, at the **same
code, the same span and over the same refused set** (the
`CT24-CLEANUP-PREEMPTED` shape: not an instance of the Q-space rule at all,
because a message is not a code). The probe pair is what makes it attributable:
`b-task-capture-baseline`, the identical module with the record declaration
removed, is **admitted**. Pinned by
`a_record_in_a_task_module_refuses_by_its_own_name`, over an owning record and a
scalar one.

### 3.4 What is recorded as absence, in those words

`AggregateAtOwnerPosition` (eight construction sites) and `NotAnAggregate` are
**unreachable from any source program**, and this card does not make them
reachable. Measured through the real pipeline rather than argued:

| Probe | What it writes | Verdict | Which gate answered |
|---|---|---|---|
| `ir-aggregate-at-owner-lend` | `lend(f, 0, 2)` on an owning record | `Q051` | the route's own `OWNED-FIELD-LEND-WINDOW` refusal |
| `ir-aggregate-at-owner-index` | `f[0] = 1;` on a record | `Q052` | "`f` is not a buffer this entry acquired" |
| `ir-not-an-aggregate` | `b.field` on a plain buffer | `Q052` | "`b` is not a record this entry constructed" |

The route's gates fire first **by design**, each giving the better-localised
message, and the IR variants remain the fail-closed backstop for a future route
that reaches the IR without such a gate. The two things that would reach them
are named rather than left open — `AGGREGATE-ROUTE-WIDENING` (a record on the
sequential route, where `whole_program` builds operations from names with no
kind gate) and `OWNED-FIELD-LEND-WINDOW`'s discharge (the lend projection
generalised to field paths) — in AGENTS.md's `Q054` sentence, in §L21's, and in
the successor residual `AGGREGATE-OWNER-POSITION-UNREACHABLE`.

`OWNED-FIELD-SEQUENTIAL-UNREACHED` likewise **stands**, re-scoped: the
sequential route's `ownership_operations` is built at two sites
(`source_extern.rs:1215`, `driver.rs:1713`) which between them construct four
variants — `Acquire`, `TransferOut`, `Drop`, `Branch` — so `Copy`, the five
aggregate operations, the range and borrow operations, `Move`, `Read` and
`Write` are all unreachable from any sequential source program, not only the
aggregate ones. Giving `Copy` a spelling on `source.buffer.v1` moves `Q103` not
at all, because that route inherits the checker and reports at its own `Q054`.

## 4. Non-vacuity — the mutations, and the one the harness caught

`scripts/mutations.py` (driver `measure/run_mutations.sh`). Each mutation is an
exact, **unique** text replacement of one arm; the named test must go **red**;
the file is then restored by rewriting it from a byte copy taken before the edit
**and bumping its mtime** (`os.utime`, never an mtime-preserving copy —
`MUTATION-HARNESS-STALE-ARTIFACTS`), and shown sha256-equal. The baseline is
proved green before and after the whole run. A green mutation is reported
**VACUOUS** and fails the harness.

**The harness earned its keep on the first run, and both logs are shipped.**
`mutations-first-run.log` records **13 of 14 red, 1 VACUOUS**: M12, the
`body_bindings` arm, whose test (`a_copy_bind_in_a_loop_body_is_the_same_statement`)
measured the value half's body-local scoping and not that function at all. The
arm *is* observable — `body_bindings` has a second caller, the loop's
**shadowing** refusal — so the fix was a test that reaches it
(`a_copy_bind_in_a_body_that_shadows_an_outer_name_refuses`, `Q097`), not a
weakened claim. `mutations.log` is the corrected run.

| Mutation | File | Arm falsified | Red test |
|---|---|---|---|
| M1 | `std_buffer.rs` | the custody walk asks the IR about a ledger-held name | `copying_a_record_with_an_owned_field_refuses_as_the_checkers_own_copy_rule` |
| M2 | `source_std_buffer.rs` | the reader recognises `let NAME = NAME;` at all | `a_record_of_scalars_is_copyable_and_the_copy_is_by_value` |
| M3 | `std_buffer.rs` | the value half binds the copied record value | same |
| M4 | `std_buffer.rs` | the value half carries a copied scalar's value | `a_scalar_is_copyable_and_the_copy_carries_its_value` |
| M5 | `std_buffer_c_emit.rs` | the emitted C copies the record rather than zeroing it | `a_record_of_scalars_is_copyable_and_the_copy_is_by_value` |
| M6 | `std_buffer_c_emit.rs` | the emitted C copies the scalar | `a_scalar_is_copyable_and_the_copy_carries_its_value` |
| M7 | `source_record.rs` | `AGGREGATE-VIEW-FIELD` refuses by its own name | `a_view_field_refuses_by_its_own_name` |
| M8 | `source_record.rs` | `AGGREGATE-NESTED` refuses by its own name | `a_nested_record_field_refuses_by_its_own_name_in_both_declaration_orders` |
| M9 | `source_record.rs` (frontend) | the nested classification sees the module's declared record names | same |
| M10 | `source_tasks.rs` | `AGGREGATE-TASK-CAPTURE` refuses by its own name | `a_record_in_a_task_module_refuses_by_its_own_name` |
| M11 | `std_buffer.rs` | the lend-structured projection asks the IR about a copy too | `a_loan_at_a_copy_position_refuses_once_from_the_ownership_ir` |
| M12 | `std_buffer.rs` | a copy bind's binding counts as a name the body binds | `a_copy_bind_in_a_body_that_shadows_an_outer_name_refuses` |
| L1 | `OwnedFieldCopyReach.lean` | every bound place refuses — the **aggregate** arm | `copy_refuses_every_bound_place` (`lake build` red) |
| L2 | `OwnedFieldCopyReach.lean` | every bound place refuses — the **dropped** arm | same |

**Not mutated, and said so.** `custody_names_at_any_depth`'s copy-bind arm is a
totality arm in the refusing direction: it lengthens a loan's window through a
statement naming the loan, and a copy of a loan is refused anyway, so no test
can separate it from its absence today. It is written because the alternative —
a window that ended before a statement naming the loan — is the one mistake that
function exists to make impossible. The Lean theorems beyond L1/L2 are not
mutated: a proof is its own falsification.

## 5. What did not move, and the census that found a collision

### 5.1 Route selection, and the composed string route

The copy bind is **deliberately not** added to
`source_std_buffer::buffer_use`. That function decides route selection *and* is
how the composed string/buffer route reads this route's statements
(`buffer_statement` returns `None` for anything `buffer_use` does not claim), so
claiming the shape there would have moved both. Consequences, pinned rather than
argued: a module whose only extra statement is a copy bind does not become a
buffer module, and `let t = s;` in a composed string module keeps the `Q068` it
always had (`the_composed_string_route_keeps_its_own_refusal`). Registered as
`COPY-BIND-ROUTE-SCOPE`: the ownership IR's copy rule is now reached by **one**
route, so "no program can copy an owner" is a statement about
`source.buffer.v1` and not about the language.

### 5.2 Constraint 14 — the census, run, and the collision it found

`scripts/census.py`, **535 `.qcore`/`.qc` files**, comments stripped
(`census.txt` beside this file). **+0 words, +0 tokens, no lexer rule changed**:
`let`, `=`, `;` and a name are all things the lexer already produces. The
question a census has to answer for a token *shape* is not "is a word free" but
"does any program spell it":

| Counted | Occurrences |
|---|---|
| `let NAME = NAME ;` — the claimed shape | **29** |
| `: string` at a field type (`AGGREGATE-VIEW-FIELD`) | 0 |
| a record field whose type is a record the file declares (`AGGREGATE-NESTED`) | 0 |

All 29 are `source.socket.v1`'s creation form — `let listener = tcp;`,
`let client = udp;` — in `examples/{sockets,http,events}` and
`compiler/tests/fixtures/ct/CT42`. The two constructs are separated by **route
selection and by nothing else**. The card kept the collision (renaming either
construct would edit a published example family or a pinned CT fixture) and
pinned the separation: `the_socket_routes_creation_form_is_untouched` asserts a
socket program still takes `StandardSocketV1`, the `examples_suite` covers the
published ones structurally, and `COPY-BIND-SOCKET-SHAPE-COLLISION` records that
the guarantee is a property of dispatch order rather than of either grammar,
with a discharge that would make it structural.

### 5.3 Emission census

No emission census over the tree was needed and none is claimed, because the
census above measures the premise a census would establish: **no program in the
tree spells the claimed shape on this route**, so no previously admitted program
reaches the new arm and none of the exhaustive-match arms this card added
changes a byte for an existing program. The tests that would catch it if that
reasoning were wrong were run and are green — `examples_suite` (every published
family, byte-compared against its reference execution), `std_buffer`,
`source_record`, `aggregate_owned_field`, `std_string`, `string_concat`,
`string_slice_io`, `slice_permissions`, `loops_on_composed_routes`,
`source_loop`, `loop_unroll_budget` — and the full workspace gate is §8.

### 5.4 One property measured in passing, and registered rather than fixed

An admitted `source.buffer.v1` program whose `let` binding is never read emits C
that the suite's own flags refuse: `let n = 5;` unused emits
`int32_t s_n = INT32_C(5);`, `quirin emit` exits 0 and `clang -Wall -Wextra
-Werror` exits 1 with `-Werror,-Wunused-variable` (measured; logs in
`measure/unused.*`). This is **inherited, not introduced** — it applies to the
acquisition, the extent read and the byte read exactly as to the copy bind — it
is fail-closed, and no shipped program is affected. Registered as
`BUFFER-UNUSED-BIND-WERROR` with the composed string route's existing `(void)`
discard scan named as one of its two discharges.

## 6. Lean — `proof/QCore/OwnedFieldCopyReach.lean`

Registered in `proof/AuditOwnedFieldCopyReach.lean`, imported by
`proof/QCore.lean`; placeholder-free, no project axiom, warnings as errors.
`scripts/check_lean_audits.py` exits 0 at **142** audits
(141 + `AuditOwnedFieldCopyReach.lean`).

The route's *admitted set* changed, so the question §5 of the brief asks is
which theorem covers the new shapes. The answer is that wave 39's theorems still
hold of the type-level rule unchanged — `an_aggregate_is_never_copied` and
`a_hollow_aggregate_is_still_not_copied` in `QCore.AggregateOwnedField`, neither
edited — and that what needed stating is the **route's decision**, which wave 39
had no reason to model because no route made it:

| Statement | Theorem |
|---|---|
| every bound place refuses, whatever it is bound to | `copy_refuses_every_bound_place` |
| an aggregate refuses full, hollow, or declaring no field | `an_aggregate_binding_is_refused_however_hollow` |
| an owner refuses by the same arm (one statement reaches both rules) | `an_owner_binding_is_refused_by_the_same_arm` |
| the route refuses exactly the bound names | `the_route_refuses_exactly_the_bound_names` |
| the decision is total; nothing is both | `the_decision_is_total`, `nothing_is_both_admitted_and_refused` |
| **the admitting side owns no resource** | `an_admitted_copy_owns_no_resource` |
| **an admitted copy is the identity on the ledger** | `an_admitted_step_is_the_identity_on_the_ledger` |
| both halves as one claim | `a_copy_either_refuses_or_moves_no_duty` |

`copyOwned`'s result type carries a success case **precisely so that "it never
succeeds" is a theorem with content** rather than a fact about the type; L1 and
L2 (§4) turn one arm into a success each and redden it.

**Named as a premise, not proved:** `LedgerCoherent` — the Rust
`Ledger::validate_invariant` clause "every resource entry's owner has a binding",
which that checker maintains after every step and unit-tests there. This module
is about the route's decision, not the checker's maintenance of its state.
**Not modelled at all:** the token reading (a tested Rust boundary), the emitted
C struct assignment (the sanitized run in §2.2), and the value half's
environment.

## 7. Retirements and residuals

**Retired (3), by exact name, after `scripts/register.py --show` printed each
entry in full** (`register-show.log`), with the discharge read clause by clause:

| Retired | Its discharge | How it is met, word for word |
|---|---|---|
| `OWNERSHIP-COPY-OP-UNPRODUCED` | "emit Copy from a route (which is a semantic card, not an audit's edit), OR record the variant as model-only…" | **first arm**: `std_buffer::custody_operations` and the lend-structured projection both build `OwnershipOp::Copy` from a source statement; §1.2, §2.2, M1 and M11 |
| `OWNED-FIELD-IR-REFUSALS-UNREACHED` | "a source fixture that reaches either refusal…, OR an amended `Q054` sentence saying the two IR errors are carried but unreached today and naming what would reach them" | **second arm**: the `Q054` sentence is amended in AGENTS.md's `Q051`–`Q057` row and in §L21's, saying "carried but unreached — unreachable from any source program" and naming `AGGREGATE-ROUTE-WIDENING` and `OWNED-FIELD-LEND-WINDOW`'s discharge; the measurement survives as `AGGREGATE-OWNER-POSITION-UNREACHABLE` |
| `OWNED-FIELD-BOUNDARIES-ARE-ABSENCE` | "per boundary, a test that drives the refusal and asserts the boundary's own name in the message, OR a restatement…" | **first arm, all three**: §3.1–§3.3, each with its own test asserting its own name, each shown non-vacuous (M7, M8, M9, M10) |

**Not retired, and why.** `OWNED-FIELD-SEQUENTIAL-UNREACHED`'s discharge asks
for a sequential-route construct that builds a `Compose` reaching `Q103`, or a
restatement when the namespace decision is taken. Neither happened. It **stands**,
with its `why` re-scoped in place (§3.4) and the amendment attributed.

**Registered (5), `recorded_by: OWNED-FIELD-ROUTE-REACH`, each with a discharge
condition** (`register.log`): `AGGREGATE-OWNER-POSITION-UNREACHABLE` (the
"unreachable from any source program" measurement, carried forward),
`COPY-BIND-SOCKET-SHAPE-COLLISION`, `BUFFER-UNUSED-BIND-WERROR`,
`COPY-BIND-BOUND-TIER`, `COPY-BIND-ROUTE-SCOPE`.

**Amended in place (4)**, because this card made their text false, each
amendment appended and attributed rather than rewriting what a previous card
measured: `AGGREGATE-VIEW-FIELD`, `AGGREGATE-NESTED`, `AGGREGATE-TASK-CAPTURE`
(all three said "no refusal names it" or equivalent, which is no longer true)
and `OWNED-FIELD-SEQUENTIAL-UNREACHED` (re-scoped).

`named_followups` **257 → 259**.

## 8. Gate

See `gate.json` and `gate/` beside this file.

## 9. For the integrator

Every item is addressable on its own.

1. **Headline numbers moved in this tree.** `AGENTS.md` `named_followups`
   **257 → 259** (the sentence after `named_followups`, re-worded to name this
   card; three retired, five registered, four amended). Lean audits
   **141 → 142** (`AuditOwnedFieldCopyReach.lean`). Workspace tests: **+19**
   (`compiler/tests/owned_field_route_reach.rs`; no other test file gains or
   loses a function) — the figure to carry into the "Merged gate" row at the
   merge is §8's, measured at this card's own tip, and it is a card gate, not
   an integration one.
2. **Standing constraint 15k is amended in this tree, deliberately**, by an
   **appended** paragraph that supersedes nothing above it: it records that two
   of the constraint's own sentences (the copy rule, and "a record of scalars
   is a plain copyable value") were decided by four unit tests and by nothing
   else, and states the copy bind, the three rules that must survive refactors
   (not a `buffer_use`; the value half says nothing about a ledger-held name;
   the admitting side's safety is proved), the three boundaries that now name
   themselves, and what stays absence. Quoted here in the card's own words —
   read it at 15k in `AGENTS.md`, not from this list.
3. **Constraint 14's ledger** gains this card's bullet: +0 words, +0 tokens,
   with the **29-occurrence collision** with `source.socket.v1`'s creation form
   recorded as the census's finding, and the two zero counts behind the
   by-name boundary refusals.
4. **The Q-space note** counts the **nineteenth** instance of the icebox's
   consequence (`Q051`–`Q057`, allocating nothing) and states that this card's
   two other movements are **not** instances, a message not being a code.
5. **§L21** (`docs/QCORE_SPEC_r0_3.md`): three rows amended — `Q051`–`Q057`
   (the copy bind; the `Q051` narrowing; the by-message split of the
   unknown-field-type condition), `Q1xx` (re-measured, both codes unmoved,
   `OWNED-FIELD-SEQUENTIAL-UNREACHED` re-scoped), `Q5xx`
   (`AGGREGATE-TASK-CAPTURE` by name, no condition widened). The `Q054`
   sentence in the wave-39 amendment is amended in the same place, which is
   `OWNED-FIELD-IR-REFUSALS-UNREACHED`'s discharge word for word.
6. **`AGENTS.md`'s diagnostics table**: the `Q051`–`Q057`, `Q100`/`Q103` and
   `Q500` rows carry the same three amendments in summary form.
7. **Increment record:** `docs/development/increments/wave-41.md` **created**
   in wave-40's shape, one paragraph, and `docs/development/increments/README.md`
   has its current-record pointer moved and its wave-table row added. Union
   with siblings' paragraphs at the merge. `DEVELOPMENT.md` needs no edit —
   it names the index, not a wave file.
8. **Matrix:** `docs/reviews/r0_3/acceptance_matrix.json`'s `named_followups`
   edited by `scripts/register.py` (idempotent; `--show` first, output shipped
   as `register-show.log`) and regenerated with
   `--gate docs/reports/owned-field-route-reach/gate.json` — never without.
9. **A decision I could not take.** `COPY-BIND-SOCKET-SHAPE-COLLISION` is the
   one thing in this card an integrator might want handled differently. The
   copy bind and `let listener = tcp;` are the same token shape; I kept the
   collision because the alternatives each edit something pinned (renaming the
   socket route's creation form would touch a published example family and CT42
   fixtures; renaming the copy bind means introducing a word, which this card's
   whole census says is unnecessary). The separation is real and pinned, but it
   is a property of **dispatch order**. If the integrator prefers a structural
   guard, the residual's discharge names one.
10. **A second one.** `BUFFER-UNUSED-BIND-WERROR` is inherited, not introduced,
    and I did not fix it inside this card's scope. The fail-closed direction is
    a route-level refusal of an unused binding, which is a `Q051` condition
    extension and someone's card; the cheap direction is the composed string
    route's existing `(void)` discard scan, applied to the plain route.

## 10. File ownership and overlaps (wave-41 scope)

**Owned by this card and edited:** `crates/qcore-core/src/ownership.rs` — in
fact **not edited at all**, which is worth saying: the copy rule this card makes
reachable is `copy_owned` exactly as wave 39 left it, and that is what "the
route asks, the IR answers" means here. `crates/qcore-core/src/source_record.rs`
(two `RecordRefusal` variants, their `code` arms, their `Display` arms, the
`field_type_refusal` classifier and `VIEW_FIELD_SPELLING`).
`crates/qcore-core/src/std_buffer.rs` **record/field and copy arms only**: the
`CopyBind` variant, its arm in the custody walk, in the lend-structured
projection's flat segment and repeat walk, in the value half, in
`derived_effects`, in `body_bindings`, in `run`'s value-bearing list, and in
`custody_names_at_any_depth`.

**Shared with `COMPOSED-WINDOW-REACH` by design, and coordinated here rather
than by editing their functions.** Two of the functions that card owns gained a
**one-arm** addition each, forced by an exhaustive `match` on a grown enum and
by nothing else:

* `std_buffer::custody_names_at_any_depth` (feeds `last_uses`) — a copy bind
  **names its source**, so it is pushed, in the refusing direction: a window
  that ended before a statement naming a loan is the one mistake that function
  exists to prevent. No other line of the function is touched, and
  `last_uses`, `lend_windows`, `crossed_lend_windows` and the nested-acquire
  scan are **not** edited.
* `std_string::view_uses_at_any_depth` and `loop_body_conflict` — the copy bind
  is listed in the "names no view" arm and in the leaves-the-constant-tier arm
  respectively. On that route the variant cannot exist at all (`buffer_use`
  does not claim the shape), so both arms are total rather than reachable and
  say so in a comment.

**Edited with no other card's ownership, and noted:**
`crates/qcore-frontend/src/source_std_buffer.rs` (the reader's last `let` arm,
`LoweringConstants::observe`'s explicit do-nothing arm, the module header's
grammar and one new header section), `crates/qcore-frontend/src/source_record.rs`
(the declared-record-name set and the classifier call),
`crates/qcore-frontend/src/source_tasks.rs` (**one** `if` before the existing
declaration-kind refusal; that refusal's own arm is unedited),
`crates/qcore-core/src/std_buffer_c_emit.rs` (the scalar arm in
`buffer_event_statement`, the record arm in `emit_events`),
`crates/qcore-core/src/std_string_c_emit.rs` (one `buffer_consumers` arm),
`proof/QCore/OwnedFieldCopyReach.lean` and `proof/AuditOwnedFieldCopyReach.lean`
(new), `proof/QCore.lean` (one import), and the ledger documents.

**Not touched:** `compiler/tests/ct_corpus.rs` (`CT-PROOF-LEG-3`),
`tests/support/thread_sanitizer.rs`, `compiler/tests/task_route_tsan.rs`,
`compiler/tests/write_lend_tasks_exercise.rs` (`TSAN-FAILURE-ARMS`),
`proof/QCore/LoopWindows.lean` (`COMPOSED-WINDOW-REACH`), and every frozen
archive under `docs/reports/`.
