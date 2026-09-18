# STABILITY-REEVAL #4 — the cadence's fourth firing

**Card:** `STABILITY-REEVAL` #4, wave 40. **Base:** `6dc484ad` (`origin/main`
at branch time, confirmed equal by `git fetch` then `git rev-parse origin/main`).
**Method:** `CLAUDE.md`'s four steps, over **waves 38 and 39**. **Baselines:**
`docs/reports/stability-reeval-{1,2,3}/README.md`.

This card is an **audit**. It weakens no gate, edits no frozen archive, changes
no verdict rule and lands **no semantic change**; it moves **no Rust and no
Lean**. It registers findings as named residuals with discharge conditions and
retires **one** entry it measured discharged. Every claim below is labelled
**MEASURED** (a command was run or a line was read, and is cited) or
**INFERRED**; where a boundary is proved on one side and observed on the other,
this report says so in those words. Four read-only sub-audits ran as delegated
readers in this worktree with cargo forbidden while the gate ran; every
conclusion used here was re-verified against the cited line by this card, or is
attributed to the sub-audit in so many words.

The first act of this run was to commit a report skeleton (`bd5eb8fd`), on
firing #2's own lesson, kept by firings #3 and #4.

The sibling card this wave is `DOCS-COMPACT-8`, which **owns the three
top-level documents' prose**. Two findings below are defects *in that prose*
(§2.7, §3.5). This card reports them and edits only the one line it owns —
`AGENTS.md`'s `named_followups` count.

---

## 1. Tally over firing #3's fourteen residuals

**8 discharged · 2 partial · 4 open**, over the fourteen entries
`STABILITY-REEVAL-3` registered. Seven are still in the matrix under
`recorded_by: STABILITY-REEVAL-3` (MEASURED: a `json` read of
`docs/reviews/r0_3/acceptance_matrix.json` at this base returns exactly those
seven, out of 234 entries); the other seven are gone, and **every one of the
seven went with a recorded disposition** — none went quiet. That is the first
firing of this cadence at which that is true.

### 1.1 The seven that are gone — discharged by what, or retired by whom

Each row was found with `git log -S'<id>' -- docs/reviews/r0_3/acceptance_matrix.json`,
then the removing commit's message and the retiring document read.

| Name | Disposition |
|---|---|
| `LEAN-AUDIT-GATE-KEY-OPTIONAL` | **Discharged, then retired by the wave-38 close** (tracker edit `19c61952`, close `41797284`). Mechanism **exists**: `docs/reports/remaining-d02/build_matrix.py:234-239` refuses a `--gate` file lacking the key; test `scripts/tests/test_build_matrix_sections.py:313`. |
| `CLANG-GUARDED-GREEN-FAMILY-750` | **Discharged on its gate-recording arm, then retired by the wave-38 close**, with `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN` re-scoped to the family. Mechanism exists (`build_matrix.py:178` `CHECKED_GATE_KEYS`, refusals `:288-313`; five tests). **Boundary, stated:** the *defect* — 750 `if !clang_available()` guards reporting `ok` on a Clang-less host — is **not repaired**; only the recording half is, and the surviving entry carries the rest. |
| `WAVE-38-QUEUE-NAMES-UNTRACKED` | **Retired by the wave-38 close** after classifying its three names. The registration was then **worked**: `STRING-CONCAT-LOOP-WINDOW` retired at `6adaf1ef` (`LOOPS-ON-COMPOSED-ROUTES`), `AGGREGATE-OWNED-FIELD` at `95460d9d`. |
| `CT07-PROOF-LEG-PATTERN-UNAVAILABLE` | **Retired by card `CT07-TWO-THREAD-PROOF-LEG`** (tracker edit `81f783ce`, merge `0d9e922f`). Mechanism exists and is stronger than a sentence: `compiler/tests/ct07_proof_leg.rs:588-595` re-measures the unavailability at every run. |
| `CT42-CELL-FALLBACK-ARM-DIRECTION` | **Retired by card `CT42-CELL-ROUTE`** (`62ff7943`, merge `45a33285`). Mechanisms: both arms are now `Err` (`crates/qcore-core/src/socket_footprint.rs:419`, `:448`), and the note was **appended** to the frozen wave-31 report rather than edited (constraint 21). |
| `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES` | **Retired by card `TSAN-ROWS-2`** (`ae73b5f9`, merge `0150c904`) on its parenthetical arm; the CT07/CT46 rows now read the real fixtures (`compiler/tests/task_route_tsan.rs:103,106`). |
| `WLT-EXERCISE-TSAN-TEST-SILENT-ARMS` | **Retired by card `TSAN-ROWS-2`.** Mechanism exists: the three silent arms now `panic!` (`compiler/tests/write_lend_tasks_exercise.rs:790,795,799`). |

**No name went quiet without a discharge.** This is the finding of §1 and it is
a *change of sign* from firing #3.

### 1.2 The seven still registered — verdicts at this base

1. `MATRIX-RECORDED-BY-ABSENT-67` — **open.** MEASURED by this card: 67 of
   234 entries still carry no `recorded_by`; `build_matrix.py` still has no
   five-key check. Deferred as a decision at both closes.
2. `W36-W37-MUTATION-HARNESSES-UNSHIPPED` — **discharged, and RETIRED BY THIS
   CARD** (§6). MEASURED: all four named report directories carry the dated
   appended restatement in the discharge's own words —
   `joined-provenance-round-bound/README.md:453,458`, `ct-lane-5:580,587`,
   `write-lend-tasks-exercise:577,581`, `mutex-share-cross-emission:567,571`,
   each opening "Appended, not edited (constraint 21)" and each reading
   "measured once by the card, harness not shipped" — and the convention line
   the discharge also required is `CLAUDE.md:219-224`. **It is the mirror image
   of the three below:** a fully met discharge that no close document names, so
   it sat registered for two waves with nobody reading it.
3. `LEGACY-CFG-NODE-FIXTURES-SPELL-EVERY-FIELD` — **open, and untouched by
   anyone.** `LegacyCfgNode` (`crates/qcore-c-gateway/src/legacy_c.rs:456-474`)
   still has no constructor and no `Default`; no recorded decision either. The
   name occurs **only** in firing #3's report and the matrix — zero occurrences
   in any close.
4. `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` — **partial; the literals still
   disagree.** MEASURED by this card, both files re-read:
   `compiler/tests/f02_trace_relation.rs:96-97` = `bfa572914cb96413` /
   `9cbe67a70a420d52`; `compiler/tests/windows_threads.rs:31,36` =
   `6df369991465940e` / `0da423be2f16b8bb`. The Linux half of the discharge
   landed (`compiler/tests/f02_windows_pin_agreement.rs`, `#[ignore]`d and red
   by design); the re-pin half is deferred as `F02-WINDOWS-PIN-RUN` at both
   closes. **The Windows row is still red on every Windows host** — now for
   five waves.
5. `MAIN-GUARD-PACK-REFS-UNPINNED` — **partial.** The sentence arm landed (the
   wave-38 close amended `MAIN-GUARD-REFUSES-PACK-REFS`' `why` to cover both
   transactions and the main-checkout refusal); the pin arm is open —
   `scripts/tests/test_main_guard_hooks.py` still spells no pack-ref case.
6. `CALL-FRAMES-NO-SOURCE-FIXTURE` — **open.** No gateway test file has moved
   since the card; no C-text fixture drives a local through an intervening
   contracted call end to end through `analyze_unrestricted_c11`. Name appears
   in no close document.
7. `W37-SUBSTITUTION-SET-EQUALITY-UNARCHIVED` — **open.**
   `docs/reports/w37-corpus-contracts-4/scripts/attribution.py` is unchanged
   since the card's own commit and still classifies by per-position leaf
   counts; no appended section, no recomputation log. Name appears in no close
   document.

### 1.3 The pattern, restated with this firing's numbers — **the sign has changed, and the discriminator is not the class**

**MEASURED.** Of the fourteen, **11** needed a mechanism (a test, a control, a
script, a proof, a code change) and **3** needed a sentence. Mechanism: **5
discharged, 2 partial, 4 open**. Sentence: **3 discharged, 0 partial, 0 open**.

Firing #3 measured *zero* of eighteen mechanism-needing residuals discharged
across two waves and offered "the sweep discharges; the tracker does not" as
the process finding. This firing measures **5 of 11**. The variable that moved
is **not** the class. It is **brief-reach**:

* Every one of the eight discharged names, and both partials, appears either in
  the wave-38 close's own item table (`docs/reports/wave38-close/README.md:99-110`)
  or in that close's §14 lines written into the wave-39 cards' briefs
  (`:528-547`) — `CT42-CELL-ROUTE` briefed with `CT42-CELL-FALLBACK-ARM-DIRECTION`,
  `CT07-TWO-THREAD-PROOF-LEG` with `CT07-PROOF-LEG-PATTERN-UNAVAILABLE`,
  `TSAN-ROWS-2` with `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`.
* The three names that reach **no close document at all** —
  `LEGACY-CFG-NODE-FIXTURES-SPELL-EVERY-FIELD`, `CALL-FRAMES-NO-SOURCE-FIXTURE`,
  `W37-SUBSTITUTION-SET-EQUALITY-UNARCHIVED` — are **all open**, and two of the
  three have a *sentence*-satisfiable second arm (a recorded decision;
  publishing a recomputation) that nobody took.

So firing #3's finding holds with its sign intact but its **cause corrected**:
a residual is discharged when a close writes it into a card's brief, whether it
needs code or a sentence. A mechanism-needing residual is not harder to
discharge; it is harder to *reach*. Firing #3 inferred the class from the
correlation because in its two waves no residual of either kind had been
briefed. Recorded here as the cadence's own methodological correction rather
than as a new claim about difficulty.

**One honesty note carried forward:** "discharged" here means *the recorded
condition was met*. `CLANG-GUARDED-GREEN-FAMILY-750` was retired with its
750-guard defect intact.

---

## 2. Step 1 — vacuous greenness in waves 38–39's fail-closed arms

Reachability is measured (who calls the function; does any fixture produce the
refused shape), never inferred from the diff.

### 2.1 `AGGREGATE-OWNED-FIELD` (`e80b58d6`) — three of four rules are driven by real source programs; the fourth is reachable from **nothing**

**MEASURED.**

| Rule | Implementation | Driven by |
|---|---|---|
| partial move | `crates/qcore-core/src/ownership.rs:503` (`MoveOut` → `move_owned(Place::Field…)`), produced at `std_buffer.rs:1016` | **a source program through the real pipeline** — `examples/records/owned_field.qcore:63`, pinned `compiler/tests/aggregate_owned_field.rs:95`; the refusing side at `:193` |
| drop-through-aggregate | `ownership.rs:737` `drop_through`, entered at `:700` | **source programs** — `aggregate_owned_field.rs:247`, `:288` |
| leak at exit naming a path | `ownership.rs:1103` → `:1312` `NormalPathLeak` | **a source program** — `aggregate_owned_field.rs:219` asserts `p.buf` in the message |
| **non-copyability** | `ownership.rs:680` `copy_owned`, `Binding::Aggregate` arm `:682` | **nothing** |

The fourth is the finding, and it is larger than "unit-test only". MEASURED by
this card with `rg -n 'OwnershipOp::Copy' --include='*.rs' crates compiler`:
`OwnershipOp::Copy` is **constructed at exactly four places in the tree, all in
`crates/qcore-core/tests/unit/ownership.rs`** (`:80`, `:715`, `:727`, `:817`).
The single non-test occurrence, `crates/qcore-frontend/src/whole_program.rs:716`,
is a *consumer* — an arm of an exhaustive match. `copy_owned` has one caller
(`ownership.rs:466`). **No route on any pipeline emits `Copy`**, so the
aggregate copy rule and the older plain-owner copy rule alike are decided by
unit tests over hand-built operation lists. This is inherited rather than
introduced by the card — but the card counts it among the four rules it landed,
and a reader of "four IR rules, all green" would not learn it.
→ `OWNERSHIP-COPY-OP-UNPRODUCED`.

One level down, the same shape: `AggregateAtOwnerPosition` (eight construction
sites, `ownership.rs:617,647,697,825,920,969,1059,1073`) and `NotAnAggregate`
(`:1047`) are asserted only at `unit/ownership.rs:779` and `:800`; on the route,
`Q052` ("read the field you mean first") fires first. Yet `AGENTS.md`'s `Q054`
amendment states the condition "now carries … the IR's own
`AggregateAtOwnerPosition` and `NotAnAggregate`" — a standing-constraint
sentence about a condition no source program reaches.
→ `OWNED-FIELD-IR-REFUSALS-UNREACHED`.

**How large is `OWNED-FIELD-SEQUENTIAL-UNREACHED`?** Larger than its text
admits, and it is not about `Q103`'s *vocabulary*. `Q103` is emitted from one
place (`crates/qcore-frontend/src/frontend.rs:3285`); the sequential route's
`ownership_operations` is built at two sites (`source_extern.rs:1215`,
`driver.rs:1713`) which between them construct **four** variants — `Acquire`,
`TransferOut`, `Drop`, `Branch`. So the whole path-extended vocabulary is
unreachable from `Q103`, not a widening of it. All five aggregate variants plus
`Copy`, `LendRange`, `WriteRange`, `ReadRange`, the two `Borrow*`, `Move`,
`Read` and `Write` are in that class. The five `Q103` assertions in the tree
(`branch_arms.rs:336,482,534`, `branch_statement.rs:277`,
`extern_calls.rs:632`) are all non-aggregate.

**Owned-field fixture census (MEASURED, whole tree):**
`rg --glob '*.qcore' --glob '*.qc' -e ': *bytes'` finds **exactly one file** —
`examples/records/owned_field.qcore:31`. Every other owned-field program is an
inline string in a test.

### 2.2 The three named boundaries of `AGGREGATE-OWNED-FIELD` are **absence, not refusal**

**MEASURED.** This is the distinction the cadence exists to draw.

* `AGGREGATE-VIEW-FIELD` — a view spelling reaches
  `FieldType::from_spelling` (`crates/qcore-core/src/source_record.rs:88`; two
  constructors, `I32` and `Bytes`), returns `None`, and refuses as
  `UnknownFieldType` at `crates/qcore-frontend/src/source_record.rs:447` with a
  `Q051` message naming **`AGGREGATE-FIELD-TYPES`, not `AGGREGATE-VIEW-FIELD`**.
  The only tests on that arm use `u8` and `Buffer`. **No test writes a view
  field.**
* `AGGREGATE-NESTED` — `type A { b: B }` takes the identical generic arm and
  the identical message. **No nested-record declaration exists anywhere in the
  tree.**
* `AGGREGATE-TASK-CAPTURE` — **pure absence.** Route dispatch selects the task
  route at `crates/qcore-verification/src/driver.rs:1168`, the record/buffer
  route at `:1277`, so a module that spawns never reaches the record route. **No
  test combines `spawn` with a record declaration, and nothing refuses by this
  name.**

Of three registered "boundaries", **zero are refusals a test drives by name**;
two collapse into one generic message naming a *different* residual, and one is
unreachable by dispatch order. → `OWNED-FIELD-BOUNDARIES-ARE-ABSENCE`.

### 2.3 `LOOPS-ON-COMPOSED-ROUTES` (`b6d1f101`) — the Lean is a **transcription**, and one of the two named window functions is provably inert

**MEASURED, the `JOINED-PROVENANCE-BOUND-DESCENT-PREMISE` shape.** The Lean
`QCore.LoopWindows.lastUse` (`proof/QCore/LoopWindows.lean:122`) is defined over
its own `inductive Stmt α` (`:75`) with its own tagging (`:89`, `:96`, `:106`).
The Rust `std_buffer::last_uses` (`crates/qcore-core/src/std_buffer.rs:1219`)
is defined over `BufferEvent` with `custody_names_at_any_depth` (`:1169`). The
link between them is **a doc comment** (`std_buffer.rs:1213-1217`) and nothing
mechanical: no extraction, no differential evaluation, no digest, and no Rust
test references `QCore.LoopWindows`. **Proved:** properties of the Lean model.
**Observed:** that the Rust function is that model.
→ `LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`.

Two gaps the transcription hides, both measurable:

1. **The one theorem tying the model to `LOOP-V1` cannot spell the construct
   the card is about.** `the_unrolling_is_loop_v1s` (`LoopWindows.lean:345`) is
   stated at `QCore.StdBuffer.Event`, whose eight constructors
   (`proof/QCore/StdBuffer.lean:47`) are `bind/acquire/store/load/extent/release/line/ret`
   — `rg -n "lend" proof/QCore/StdBuffer.lean` returns **nothing**. The Rust
   `BufferEvent` has sixteen constructors including `Lend`, `Repeat`,
   `Construct` and the four `Field*`. So the theorem that says "this is
   `LOOP-V1`'s unrolling" is proved over an alphabet with **no loan in it**; the
   general theorems keep `uses` abstract, which is what lets the gap through.
2. **The non-vacuity witness is a toy.** `a_top_level_only_window_misses_a_body_use`
   (`:387`) is `decide`d over `[.leaf 0, .loop 2 [.leaf 1]]` with `uses = (· == 1)`
   — the model's own scenario, not the route's events.

Separately, `the_admitted_body_shapes_are_exactly_these` (`:445`) proves
`∀ s : AdmittedInBody, s ∈ [.access, .plain]` by `cases s <;> simp`: a
two-constructor enum's constructors are in the list of its own constructors. The
report's theorem table presents it as "the body shapes admitted beside a loan,
as one list". → `LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY` (low severity, recorded
so it is not cited as coverage).

### 2.4 `std_string::view_uses_at_any_depth` is **empty for every input**, and `AGENTS.md` names it as the composed route's window function

**MEASURED by this card**, reading `crates/qcore-core/src/std_string.rs:1130-1158`.
`buffer_view_uses` returns `Vec::new()` for every `BufferEvent` arm except
`Repeat`, whose arm `flat_map`s over a body that can only hold the same
constructors. **The function therefore returns the empty vector for every
possible input, by construction.** The code says so in its own comment
(`:1125-1129`: "no buffer event names a view … The recursion is written anyway,
so that a later widening of the body grammar … lengthens the window here") and
the card's report says so in its §2. **`AGENTS.md:1266-1268` does not:** it
reads "on **both** routes it runs from the loan's opening to the last top-level
statement naming the loan **at any depth** (`std_buffer::last_uses`;
`std_string::view_uses_at_any_depth`)". The composed route's real repair is
`loop_body_conflict` / the `Repeat` arm of `check_string_program`, which the
constraint *does* describe two sentences later — so the constraint is not
wrong, it is **misleading in the one clause that names a function**, and a
reader following it will believe the two routes have parallel window
computations when one is a no-op.
→ `COMPOSED-VIEW-WINDOW-INERT`. Reported, not edited: `DOCS-COMPACT-8` owns
that prose.

### 2.5 The loop window's six aggregate arms are unreachable — **and lifting either guarding refusal makes them load-bearing in the admitting direction**

**MEASURED.** `custody_names_at_any_depth` (`std_buffer.rs:1169`) has an
explicit arm returning nothing for `Construct`, `Field`, `FieldStore`,
`FieldLoad`, `FieldExtent`, `FieldRelease` (`:1181-1192`), justified in-comment
by the refusal `OWNED-FIELD-LEND-WINDOW`. The ordering was measured rather than
trusted: `check_buffer_program` returns `Err(OWNED-FIELD-LEND-WINDOW)` at
`std_buffer.rs:2677-2691` for any program that composes an owned field and
contains a `Lend`, and `Err(OWNED-FIELD-LOOP-BODY)` at `:2713-2733` for an
owned-field event in a `Repeat` body — **both before** the block at `:2737`
that calls `lend_windows` → `last_uses`. So those six arms are unreachable in
every program the checker admits that far.

This is the shape `CLAUDE.md`'s step 4 names: **a rule correct today because a
construct is unreachable, which a queued card makes load-bearing.** The day
`OWNED-FIELD-LEND-WINDOW` is lifted, the window silently forgets every
field-path name — a *shorter* window, which is the **admitting** direction
invariant 1 constrains. → `WINDOW-FIELD-ARM-UNREACHED`, naming the lifting
card in its discharge.

Related and measured: **there is no admitted program anywhere in the tree with
an owned field inside or beside a loop or a lend.** The pair test
`the_owned_field_refusals_survive_a_for_beside_a_loan`
(`compiler/tests/loops_on_composed_routes.rs:977`) exists and drives three
combinations — all three **refusals** (`:1006`, `:1030`, `:1048`). The two
cards' interaction is therefore pinned as *mutual refusal only*.

### 2.6 The three loop residual names, and a caveat on two mutations

**MEASURED.** `LOOP-PER-ITERATION-LOAN` refuses by name on **both** routes
(`loops_on_composed_routes.rs:856` `Q051`, arm `std_buffer.rs:2769-2775`;
`:878` `Q077`, arm `source_std_string.rs:1220`).
`LOOP-WINDOW-BODY-CUSTODY` is driven on both (`:903-917` the acquire case,
`:918-933` the release case) — though each route drives only one half of the
`matches!(Acquire | Release)` disjunction. `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`
is one route only (`:955`), correctly: there is no string vocabulary on the
buffer route.

Carried forward from the card's own `mutations.log` table (the card says so in
its §6, so this is not a hidden defect): **M4 and M6 are red on the message text
only** — under both mutations the program stays refused by a different arm and
only the residual *name* moves. Two of six mutations therefore show name
stability, not rule necessity.

### 2.7 `CT42-CELL-ROUTE` (`45a33285`) — both arms are `Err`, both are driven, and neither is reachable through the route

**MEASURED.** Both fallback arms are now `Err`:
`crates/qcore-core/src/socket_footprint.rs:419` (`UnpairedConnect`) and `:448`
(`UnpairedAccept`), each carrying `Q085` (`:215`) and the offending statement
index (`:226`). Firing #3 found them assigning a singleton cell in the
*admitting* direction; that is repaired.

Reachability, measured rather than taken from the card: the only production
path is `crates/qcore-verification/src/driver.rs:5476,5657` →
`check_socket_cells` (`socket_footprint.rs:507`) → `derive_socket_cells`
(`:361`), whose **first line** (`:362`) is
`check_socket_program(program).map_err(SocketCellError::Transport)?`. The
transport checker refuses both conditions first (`std_socket.rs:2022-2031` the
unqueued accept; `:1476-1484` the unresolved target). **So neither arm is
reachable from a source program** — which the card says, and which this card
confirms.

They are nevertheless **driven** — by a deliberately published bypass.
`crates/qcore-core/tests/unit/socket_footprint.rs:319` and `:355` call
`derive_cells_of_admitted` directly (published for exactly this,
`socket_footprint.rs:369-378`). So this is not an `Err` arm nothing verifies;
it is one verified **only** off-route.

**Proved and observed, in those words.** *Proved* (in Lean, of a Lean re-model):
the derivation's direction and its three refusals —
`proof/QCore/SocketFootprint.lean:234,239,245,255`, `:184`, the composition with
the unedited `decideDisjoint` at `:273,280`, and the T09 narrowing in
`proof/QCore/SocketCellRoute.lean:74`. *Observed*: (a) that the Rust deriver
**is** that Lean model — no correspondence test, no digest, no Rust test reads
`proof/`; (b) that the arms are unreachable from source — read off
`check_socket_program`, not proved; (c) the **coupling** that makes (b) true,
namely that the deriver's queueing walk agrees with the checker's. Both read the
same `resolve_addresses` (`std_socket.rs:1742`; `socket_footprint.rs:363`) and
queue connectors identically (`std_socket.rs:1981-1985`;
`socket_footprint.rs:424-427`) — **and nothing tests that agreement**.
→ `CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND` for (a),
`CT42-ARM-UNREACHABILITY-UNPINNED` for (c).

`SOCKET-CELL-COUPLING-UNPROVED` was re-read and **accurately names what it
names**: `proof/QCore/StdSocket.lean` carries ledger and stream theorems
(`:132`–`:349`) and no theorem about peer coupling. What it does **not** name
is (a).

`CT42-CONTROLS-NATIVE-UNRUN` confirmed: the three controls
(`compiler/tests/fixtures/ct/CT42/{listener_question,two_connections,datagram_pair}.qcore`)
dispatch through `ct_corpus.rs:5621` → `run_sequential_control_list`, whose
accept arm asserts only `module.c_source.is_some()` (`:5658-5663`). CT42 is in
neither `SEQUENTIAL_SANITIZED_CASES` (`:218`) nor `COMPOSED_NATIVE_CASES`
(`:288`). **Nothing compiles or runs their emitted C.**

### 2.8 `TSAN-ROWS-2` (`0150c904`) — the gate-closed row is genuinely pinned; the header-invisibility coupling is enforced only by coincidence

**MEASURED.** `TSAN-FORCED-GATE-OPENED-BY-THE-JOIN` is **pinned by a real test
in both directions**: `compiler/tests/task_route_tsan.rs:711`
`a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing` asserts
`races == 0` **and** `schedule.is_empty()` for each `Schedule::FORCED`
(`:719-729`), then `races >= 1` unforced (`:733-738`). The file is
`#![cfg(target_os = "linux")]` (`:72`) — a declared scope, not a silent skip.
This residual is honest and its row is not prose.

`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY` is the one that can rot. MEASURED:
`hook_visible_to_tsan` exists **only** at
`docs/reports/tsan-rows-2/artifacts/mutate.sh:139-141`; nothing in the workspace
runs it. **Nothing reads the header's text** — no test asserts that
`SCHEDULE_FORCING` (`tests/support/thread_sanitizer.rs:173`) contains
`QCORE_FORCED_UNINSTRUMENTED` (`:199`); no checksum, no generated-file pin.
`task_route_tsan.rs:507` checks the *emitted C* for harness symbols, the
opposite direction. The measured failure mode **is** caught behaviourally —
`overlapping_writers_are_reported_under_every_schedule` (`:624`) asserts
`races >= 1` over `Schedule::ALL` on two subjects, which is exactly the row the
v1 pthread header made go unreported
(`artifacts/catch-point-reverse-mutex-edge.log`). But that is **coverage by
coincidence, not by design**: nothing names that test as the header's
invisibility guard, and a visible-sync design that synchronised only the
gate-open (not between worker bodies) would hide other races while leaving that
row green. The residual's discharge — "keep the reproduction mutation red
whenever the header changes" — has **no mechanism**.
→ `TSAN-HEADER-INVISIBILITY-UNENFORCED`.

**Harness shipping:** `W36-W37-MUTATION-HARNESSES-UNSHIPPED`'s complaint does
**not** apply to any wave-39 card. All three shipped, tracked: `tsan-rows-2/artifacts/`
(seven files), `ct07-two-thread-proof-leg/{mutations.log,scripts/mutate_ct07_proof_leg.py}`,
`ct42-cell-route/{mutations.log,scripts/mutations.py}`. Wave 39 is the
standard-compliant wave, which is part of why that entry retires here.

### 2.9 `CT07-TWO-THREAD-PROOF-LEG` (`0d9e922f`) — the liveness check is non-vacuous for this case; the digest has a discrimination control and **no re-transcription control**

**MEASURED.** `PROOF_GATED_CASES` (`compiler/tests/ct_corpus.rs:228`, CT07 at
`:253-256`) **is** driven by wave 38's
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture` (`:7823`):
it reads the named file, finds `fn ct07_artifact_is_the_lean_proved_disjoint_halves_shape()`
(`ct07_proof_leg.rs:492`), checks the attribute block carries `#[test]` (`:491`)
and no `ignore`, and requires the literal
`include_str!("fixtures/ct/CT07/case.qcore")` — present at `:71` and actually
used by the named test at `:493`. **Not vacuous for CT07.**

Two shape gaps in the check itself, neither biting CT07 today: it asserts the
**file** contains the `include_str!`, never that the *named test* uses it (a
second test in the file satisfies it); and its attribute scan rejects only
`ignore`, so a `#[cfg(target_os = …)]` on a proof-leg test would leave the
proof numerator green while the stage ran nowhere off that host.
→ `PROOF-GATED-LIVENESS-CHECK-FILE-SCOPED`.

**The digest pin.** `ct07_proof_leg.rs:75`
`const CT07_LEAN_LEDGER_DIGEST: u64 = 8876998389998880787;`, checked at `:532`;
Lean twin `proof/QCore/Ct07LeaseLedger.lean:209`. A **discrimination control
exists**: `a_single_writer_mutant_…` (`:752`) ends `assert_ne!` (`:799`), and
the leg does not rest on the digest alone (`:527`, `:534-539`). A
**re-transcription control does not**: the two literals are independent copies,
**nothing compares them**, no test under `compiler/tests/` or `crates/` reads
any `.lean` file, and `proof/AuditCt07LeaseLedger.lean` only `#print axioms`
(`:15-40`) — it proves the Lean digest matches the *Lean* term, never the Rust
constant. Someone who re-derived the digest from new bytes and updated both
literals would be green in both gates.

**This is the whole family, not one case:** `ct01`(via `ct12_ct14_proof_reach.rs:63`),
`ct03_proof_leg.rs:55`, `ct37_proof_leg.rs:49`, `ct07_proof_leg.rs:75` — four
bare literals, zero cross-checks. And there is a sharp consequence for an
existing entry: `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`'s discharge prescribes
"bind the Lean term to the Rust pin by a generated digest **the way CT01's
proof leg binds its trace**" — measured, **CT01's leg does no such binding**. An
existing discharge condition names a mechanism that does not exist.
→ `PROOF-LEG-DIGESTS-UNCROSSCHECKED`, whose discharge includes correcting that
sentence.

**The F02 premise pin exists and is weaker than its own residual reads.**
`ct07_proof_leg.rs:588-595` asserts only
`AdmittedCausalOrder::derive(&artifact.observation).is_err()`. It pins neither
the error variant nor the refused index, so **any other** future refusal of this
run keeps the assertion green while the premise "the trace route is closed
*because of the lease*" has silently moved — even though constraint 15n's own
proved facts (`refusal_points_at_a_refused_event`, `refusal_is_the_first`) and
the sibling pin `index: 2` in `compiler/tests/trace_alphabet_lease_boundary.rs`
make the sharper assertion available for free.
→ `CT07-F02-PREMISE-PIN-IS-ERROR-BLIND`.

### 2.10 Wave 38's own machinery — still non-vacuous, with one arm red for the wrong reason and one check that cannot see what it claims

**MEASURED.** `docs/reports/remaining-d02/build_matrix.py` is byte-identical to
the sha recorded in `docs/reports/wave38-close/mutations.log:1`; all six
mutation target strings still occur exactly once. Arms M2–M6 are still checked
and would still redden by the test named for each. **M1 is the exception**:
with the `if missing_checked:` refusal neutered, `:244`
`GATE_CHECKED = {key: supplied[key] …}` raises `KeyError`, so the run still
exits nonzero — but the traceback carries neither residual name, and
`_refused` (`test_build_matrix_sections.py:305-311`) asserts both needles. The
arm is red **by an accidental crash rather than by the intended refusal**;
recorded as an observation, not a defect.

**The `clang` key cannot tell a borrowed recording from an own one.**
`clang_recording()` (`build_matrix.py:182-196`) checks only that a file named
`clang-version.txt` sits in the gate file's directory or a `gate/` subdirectory,
that the claim parses as `present:`/`absent: <banner>`, that the banner equals
the recording's **first line**, and — *only if `clang-version.exit` happens to
exist* — that the exit agrees with the presence. **No re-invocation, no host
identity, no timestamp, no tree or commit binding, no `QCORE_CLANG`
cross-check**, and the exit arm is optional. A `clang-version.txt` copied from
any other report directory passes every arm.
`INTEGRATION-GATE-CLANG-CLAIM-BORROWED` is therefore not merely a hazard about
three files; the check is **unfalsifiable as written**.
→ `CLANG-RECORDING-UNBOUND-TO-ITS-RUN`. (This card ran `clang --version` in its
own worktree, as its brief requires; `gate/clang-version.txt` and `.exit`.)

`proof/Audit*.lean` on disk today: **141** (`ls proof/Audit*.lean | wc -l`, run
by this card), agreeing with `AGENTS.md` and the wave-39 close's `gate.json`.

**The `#[ignore]`d Windows F02 agreement test still reproduces its red.** Test:
`the_windows_f02_pins_agree_with_the_linux_ones`,
`compiler/tests/f02_windows_pin_agreement.rs:101-111`. The literals still
disagree (§1.2 item 4), and the extraction anchors are intact
(`windows_threads.rs:28,34`; `f02_trace_relation.rs:96-97`), so its un-ignored
companion `both_f02_pin_sites_are_still_where_this_test_reads_them` (`:75-97`)
is measuring what it claims. **What would ever surface the red: nothing
automatic.** `rg -- '--ignored|--include-ignored'` over `.github/`, `scripts/`,
`AGENTS.md` and `CLAUDE.md` finds **no** invocation; CI runs plain
`cargo test --workspace`. The only surfaces are a person typing the documented
command, the two tracker entries, and prose in `AGENTS.md` and two close
reports. This is by design (un-ignoring it would redden the standing gate), but
it should be stated where the test is cited.
→ `F02-AGREEMENT-TEST-SURFACED-BY-PROSE-ONLY`.

**The reconciliation sweep's content `--check` genuinely opens the files.**
`_verify_claim` (`scripts/reconciliation_sweep.py:728-766`) resolves the path
under the root, rejects escapes (`:748-751`), reads the file and tests a
whitespace-normalised substring (`:758`); `contains` missing and `absent`
present each raise a complaint (`:759-766`), wired at `:820` and `:923`, exiting
1. Coverage is **100% in both waves**: wave 38, 6 of 6 `applied` items carry a
non-empty `verify`; wave 39, **31 of 31**, with the `unverifiable` escape hatch
(`:797-816`) unused in both. It is still red by mutation — demonstrated below
without needing one.

---

## 3. Step 3 — semantic edits made without a card

### 3.1 The three already registered — verdicts, and whether the registration suffices

* **`GATE-SUMMARY-PARSER-OK-ONLY`** — **verdict: accurate, and the registration
  suffices as a record but not as a guard.** MEASURED, `git log -1 --format=%B 0d9e922f`
  carries "workspace 4620 passed / **0 failed** / 9 ignored … **the 3 failures
  are** constraint 16's load-sensitive verifier-host rows …" in one sentence.
  The wave-39 close states the true totals (4,641 / 3 / 9) and records that no
  ledger figure carried the error. What the registration does not do is stop
  the next merge message being written by the same parser; §3.5 shows the
  adjacent counting method changing between merges within one wave.
* **`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`** — **verdict: accurate but
  under-scoped.** MEASURED: each of the three `integration-gate.json` files
  ends its `note` with "Clang claim taken from the card's own recording beside
  this file (gate/clang-version.txt, same host)", so the reuse is **disclosed**,
  not concealed; `loops-on-composed-routes` and `tsan-rows-2` even share a
  byte-identical recording. The entry reads as a fact about three files. §2.10
  measures it as a fact about the **check**: nothing ties any recording to any
  run. **The registration is insufficient**, and the sharpening is registered as
  `CLANG-RECORDING-UNBOUND-TO-ITS-RUN` rather than by editing the entry.
* **`MUTATION-HARNESS-STALE-ARTIFACTS`** — **verdict: accurate and sufficient**,
  and this card complied with it: §2 added no test, so no restore was needed;
  the one harness-shaped artifact shipped here is the gate runner.

### 3.2 `UNCARDED-BLIND-TO-MERGE-PROSE` — the sweep cannot see where the integrator's prose lives

**MEASURED.** `python3 scripts/reconciliation_sweep.py . --since e878b362 --uncarded`
returns 18 commits over `e878b362..HEAD`: **16** card-branch WIP commits, each
prefixed with its own card's name, plus two `WAVE-39-CLOSE:` WIP commits; and
**one** structural integrator commit, `f11ccbba` "Open wave 40" — this card's
own, in the established convention of `05bbd149` (wave 38) and `477b733d`
(wave 37). **Zero unexplained commits.**

But `uncarded_commits` (`scripts/reconciliation_sweep.py:547-587`) computes
`diff-tree … -m --first-parent <sha>` and **skips any commit whose file list
contains a `docs/reports/*/README.md`** (`:580-584`). Every wave-39 merge's
first-parent diff carries several (verified for `b6d1f101`: five report
READMEs), so **all six merges are excluded by construction** — and the merges
are exactly where the integrator's own prose lives. §3.5 is the proof that this
matters. The tool's docstring says a merge carrying a report "lands here"; it
does not.

### 3.3 `INTEGRATION-TIP-CITATION-UNREACHABLE` — five cited integration tips are not ancestors of `main`

**MEASURED by this card**, `git merge-base --is-ancestor <sha> HEAD` for each:
`4786ae4c` (exit 1), `3fa88fc6` (1), `601acde6` (1), `f1fcd14d` (1),
`ecfef946` (1). All five are the **pre-amendment** merges — e.g. `4786ae4c` and
the in-history `b6d1f101` have identical parents and differ in exactly three
files (`AGENTS.md` one line, the new `integration-gate.json`, the regenerated
matrix), so the *code* is identical and the test figures are honest. But the
citations point at commits `git log` will never show and `gc` will eventually
drop — **including `AGENTS.md`'s own merged-gate row**, which cites
"4,727 / 0 / 10 at `4786ae4c`".

Two of the five are worse: `0d9e922f` (CT07) and `45a33285` (CT42) publish
**no** `integration-gate.json` at all and state integration figures — 4620/138
and 4703/140 — only in their merge messages, each differing from that card's own
`gate.json` (4644/138, 4697/140). `AGENTS.md` describes the convention as
covering "the three integration gates", which leaves those two figures with no
record outside an unreachable commit's message.
→ `INTEGRATION-TIP-CITATION-UNREACHABLE`, `INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE`.

The three published files were checked against the tree at their merges and
**agree**: `loops-on-composed-routes` 4727/141 at `b6d1f101` (141 Audit files),
`aggregate-owned-field` 4677/139 at `e80b58d6` (139), `tsan-rows-2` 4683/139 at
`0150c904` (139), each matching its merge message.

### 3.4 `MERGED-GATE-ROW-LEAN-HAND-EDIT` — the drift the `lean_audits` key was created to end, three times, inside the wave that made the key required

**MEASURED by this card.** Using the combined diff (`git show --cc -U0 <merge> -- AGENTS.md`),
lines prefixed `++` are in neither parent and are therefore the integrator's own.
At `e80b58d6`, `0150c904`, `45a33285` and `b6d1f101` there are one or two such
lines, and at three of them the line is **`CT07-TWO-THREAD-PROOF-LEG`'s merged-gate
row verbatim with only the Lean number replaced**. Read at `45a33285`:

> `| Merged gate | **4,644 passed / 0 failed / 9 ignored, 235 result summaries over 233 test targets** … · **140** Lean audits · … Measured by card **`CT07-TWO-THREAD-PROOF-LEG`** at its own tip `81f783ce` …`

and `docs/reports/ct07-two-thread-proof-leg/gate.json` records
`"lean_audits": 138` (its workspace figure, 4644, matches). So a Lean figure
the named run did not produce wore that run's label at three consecutive
merges. **This is exactly `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`** — the defect
the `lean_audits` key was created for in wave 37 and made *required* in wave
38. The key does not reach here: it guards `build_matrix.py --gate`, not
`AGENTS.md`'s hand-maintained prose row.

**The wave-39 close repaired both prose lines at `24505b83`**, and the row today
reads "Measured by the **wave-39 close** at its own tip `11edf7eb`". The finding
is that the mechanism recurred three times within one wave and **nothing
prevents it recurring** — a close is the only catch, and a close is not a gate.

Also measured and **clean**: the follow-up *counts* are honest at every merge
(`AGENTS.md` 232 / 234 / 231 / 233 against `named_followups` lengths
232 / 234 / 231 / 233; the 234→231 dip at `45a33285` is CT42's five retirements
less two registrations). Constraint 14's keyword-census ledger was checked at
`b6d1f101` and is **clean**: both wave-39 census bullets came from their own
branches; there is **no** hand-written constraint-14 prose union at that merge.
What the merge *did* discard was the `LOOPS-ON-COMPOSED-ROUTES` branch's own
count sentence, replaced by one crediting `CT07-TWO-THREAD-PROOF-LEG`; all three
names it listed are registered, so nothing was lost but the attribution, and the
close repaired that too.

### 3.5 `GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT` — the ledger's headline row publishes an arithmetic that does not reproduce

**MEASURED by this card** on its own gate log, by a shipped script
(`scripts/count_summaries.py`):

* **236** test targets — 227 `Running` + 9 doc-test sections;
* **238** lines matching the strict `^test result: `;
* **240** lines matching the loose `^test result`;
* exactly **two** targets print more than one summary — `Doc-tests qcore_core`
  and `Doc-tests qcore_verifier_host` — so 236 + 2 = **238**, and the extra two
  in the loose count are the two tests whose *names* begin `result_`
  (`result_handlers_collect_siblings_before_rethrowing_exact_worker_errors`,
  `result_arm_bindings_neither_leak_nor_resolve_their_own_scrutinee`).

`AGENTS.md`'s merged-gate row publishes "**240 result summaries over 236 test
targets** (227 `Running` plus 9 doc-test sections; **four** of those sections
print two summaries each)". The count of doubled sections is wrong — it is two —
and 240 is the **loose grep's** number, the artifact `STABILITY-REEVAL-3`
explicitly named for its own run ("a loose `grep -c '^test result'` reads 235
for the two `result_` test names"). The parenthetical "four" makes 236 + 4 = 240
self-consistent, which is how the error survives reading.

The counting method also **changed within wave 39**: the row written at
`45a33285` published "235 result summaries over 233 test targets … **two** of
those sections print two summaries each" — 233 + 2 = 235, the **strict** count,
correct. Between that merge and the close the row switched to the loose count
and the parenthetical was adjusted to fit. No verdict, no ledger figure and no
test depends on the number; it is a *published measurement* that does not
reproduce, which is what this cadence exists to catch. **Not repaired here:**
`DOCS-COMPACT-8` owns that prose, and this card must not edit a figure into a
document it does not own.

### 3.6 `RECONCILIATION-VERIFY-CLAIM-PERISHABLE` — a frozen close's `--check` is red at this base, and this card's own conventional commit is what made it so

**MEASURED.** Run by this card:

```
python3 scripts/reconciliation_sweep.py . --since e878b362 \
    --check docs/reports/wave39-close/reconciliation.json     # exit 1
```

two complaints, one of which is

> `ct07-two-thread-proof-leg#for-the-integrator#4 verify[2]: docs/development/increments/README.md does NOT carry the asserted text 'Current record file: [wave-39.md](wave-39.md)' — the verdict says 'applied' and the named location disagrees`

The claim quotes a **mutable pointer**. `f11ccbba`, this card's wave-opening
commit — the same convention as `05bbd149` and `477b733d` — moved that pointer
to `wave-40.md`, so the wave-39 close's record is now permanently red through no
fault of the close, and the next wave-opening commit was certain to happen when
the claim was written. (The second complaint is this card's own report skeleton
appearing in the sweep with "(pending)"; that is the sweep working, and the
wave-40 close will verdict it.) This is the failure mode `_normalise`'s own
docstring (`:713-717`) names as the reason verification arms get switched off.

---

## 4. Step 4 — what a queued card would make load-bearing

(filled below)

---

## 5. Gate

(filled below)

---

## 6. Integrator reconciliation list

(filled below)
