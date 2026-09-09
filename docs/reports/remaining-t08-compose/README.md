# T08-COMPOSE — a flat-epoch bound over a real checked task region

Date: 2026-09-09. Baseline: `695f0e1` (wave-10 consolidation).
Branch: `claude/t08-compose` (isolated worktree). Wave 11, relaunch of a wave-10
card stopped for a fleet trim; the prior uncommitted work was gone and this was
started fresh.

**What landed:** `source.epoch.tasks.v1` — an epoch *plan* in front of an
ordinary checked scoped-task region, with an **agreement relation** between the
plan's declared rows and what the checked task ledger actually does
(`crates/qcore-frontend/src/source_epoch_tasks.rs`, 1,073 lines), plus the driver
route that runs both checkers and publishes both halves
(`crates/qcore-verification/src/driver.rs`). 21 new Rust tests, 30 new audited
Lean theorems in the existing `QCore.EpochSourceLowering` namespace.

**Nothing is promoted.** No diagnostic code is added, moved or downgraded; no
`qcore-schemas` change; no new reserved word; no CT manifest edit; no core
checker edited. The Lean audit count is unchanged at **51** (the new theorems
extend the existing `AuditEpochSourceLowering.lean`), so the acceptance-matrix
derived-facts test is **not** disturbed by this card.

T08-EPOCH-SRC recorded `T08-SRC-COMPOSE` as an open follow-on: *"An epoch
declaration composed with a real task, share, guard or region-credit module.
Every such combination is currently outside every checked profile."* This card
closes the task half of that, at the boundaries recorded in §8 and §10, and records the
guard half as measured-and-refused.

---

## 1. What the composition is

`source.epoch.bound.v1`'s workers are *declarations*: a worker block declares
quantities and has no body, no captures and no completion capsule. That was the
profile's own first named boundary. Here the entry's first statement is a plan
and the rest of the entry is a task region the **unedited**
`check_task_module` accepts on its own:

```qcore
import std.concurrency;
concurrency checked;
fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            move;                       // ordered captures
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);                 // optional: derived when elided
        }
    }
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(buffer);
                drop buffer;
                return value;
            }
        }
    }
}
```

The grammar is in `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02f.

### Why this shape, and what it costs

* **The plan is the entry's first statement, and it is removed by *blanking*,
  not by deletion.** The plan's bytes are replaced with spaces and newlines, so
  the task half is byte-for-byte the module the scoped-task route would have
  compiled and every byte offset, line and span it reports still points at the
  original file. This is the same device `compile_files_with_registry` already
  uses for the module-mode marker. A test asserts length, line count, the two
  byte-identical outside regions, and that nothing but blanks is left inside.
* **No new reserved word.** Capture clauses are spelled `move;` and `copy;`,
  reusing two words the language already reserves. `move;` arrives as the
  parser's own `StatementKind::Move` with no place; `copy;` is an expression
  statement. The fixture-collision audit is therefore vacuous — zero words are
  reserved by this card — and the whole workspace suite (2055 tests) executes
  every fixture and inline program in the repository unchanged.
* **A capture clause names no place.** What the creator captured is the
  region's fact; repeating its name in the plan would be a second spelling of
  the same thing, and it would also have to resolve against bindings the plan
  precedes. `move buffer;` is refused with that reason.
* **Rows are matched by worker name and then by order.** Rows naming `W`, in
  declaration order, correspond one-to-one with `W`'s `TrySpawn` sites in the
  entry function, in source order. Task identities are then numbered in
  *creation* order, so the epoch's numbering follows the region rather than the
  plan's text order — pinned by a test that agrees the same two rows with the
  same two creations written in either order.

---

## 2. The agreement relation

`agrees(rows, ledger)` is decided in two halves, kept apart on purpose.

**Matching** (per worker, both directions):

| Condition | Outcome |
|---|---|
| worker `W` has `k` rows and `m` creations, `k ≠ m` | refused |
| a creation of `W` with no row (`k = 0`, `m > 0`) | refused (*undeclared worker*) |
| a row for `W` with no creation (`k > 0`, `m = 0`) | refused (*over-declared worker*) |

Every worker named by *either* side is checked, so an over-declared row for one
worker cannot be paid for by an undeclared creation of another.

**Per matched pair:**

| Fact | Source | Rule |
|---|---|---|
| ordered capture list | derived from `TrySpawn.arguments` | must equal the row's ordered `move;`/`copy;` clauses |
| retains a result | derived from the worker's checked `result` type | `retained BYTES;` required exactly when the result is `own<Buffer>` |
| disposal | **derived** from the checked IR | a written `join(...)` must equal it; an elided one is filled from it |
| joined | **derived** (always true) | no source form declares it |
| task count | **derived** | no source form declares it |

The disposal derivation walks the checked IR: every `Join` of the creation's
handle slot, every `TryOutcome`/`UnwrapInfallible` of the resulting outcome
slots, and then, per bound value, whether a `DropBuffer` releases it and whether
any `Move` operand hands it on. A creation joined on several paths — a spawn
inside a `Result` match is joined in both arms — must yield the *same* disposal
on all of them; disagreement is refused rather than resolved by picking one.
This was found by measurement: the first implementation refused the real CT20
shape with "joined more than once", and the fix was to derive over the sets, not
to relax the check.

---

## 3. DERIVE — disposition

The card asked for elided declarations wherever every figure of a row is
derivable from checked structure, giving *capture sizes from declared Buffer
extents* as the example.

**That example is not honestly reachable, and the reason is structural.** This
backend's `Buffer` has **no extent**: `TaskTerminatorKind::TryAllocateBuffer`
takes an `initial: ScalarOperand` — a scalar *value*, not a size —
`qcore_buffer_read` returns one scalar, and no source form declares a size
anywhere. There is also no cost model, so `effort` is not derivable either.
Consequently `peak`, `retained`, `credit`, `bound` and `overhead` stay declared
premises of `FlatEpochBoundV1`, exactly as on the pure route.

**What is derivable is the epoch's shape, and all of it is derived:**

* the task count (`declared_task_count` comes from the ledger, and no source
  form can state it),
* each task's identity and its ordering (creation order),
* `joined` (the scoped-task ledger requires every creation to be joined before
  its scope exits, so a checked module has no unjoined creation to record),
* `shared_entry` (empty by construction — see `T08-COMPOSE-SHARED`),
* the result *kind* (retained versus released-in-worker), and
* the **disposal**, which is the one figure a plan may still spell.

So DERIVE lands on exactly one elidable declaration — the `join(...)` clause —
with declared-but-divergent still refusing in both wrong directions
(`join(move)` and `join(close)` against a derived `join(drop)`), and with
eliding proved to produce the same program a correct declaration produces. The
result kind is *checked* rather than elidable, because its footprint is not
derivable even though its presence is.

### A measured consequence: `TransferredToCreator` is unreachable

`join(move)` — a result still live in the creator at the epoch exit — cannot be
derived from this profile at all. An owned Buffer leaves the creator only by
`drop`, by a move into another creation, by a return, or as a raise payload; the
task entry's result type is `i32` or `unit`, so it cannot be returned. A module
that joins and neither drops nor moves is refused by the **unedited** task
checker with `return/error discards a handle, unconsumed Result or owned
buffer` — measured, with the plain (no-plan) control giving the identical
refusal. A composed epoch's `exit_residency` is therefore always the epoch
overhead alone. Recorded as `T08-COMPOSE-EXIT`, not repaired: repairing it means
a task-profile change, not a composition change.

---

## 4. Guard and rank interaction — measured, then refused deliberately

Wave 10's `CT33-RANKS` moved the mutex band to `Q610`-`Q625` and added
`let m = mutex rank N guards r;`; wave 9's `T03-GUARD-JOIN` added
`thread scope c under g`. The card asked what happens when an epoch wraps a
`with lock` body.

| Module | Route reached | Measured outcome |
|---|---|---|
| plan + `let m = mutex guards cell; with lock m as g { … }` + spawn region | composed | `["Q600"]`, message names `T08-COMPOSE-GUARD` |
| the same body with **no plan** | mutex (`source.mutex.guard.v1`, selected before tasks) | `["Q600"]`, message: *"the scoped task profile's `spawn`/`join` expressions are outside source.mutex.guard.v1"* |
| plan + `let m = mutex rank 1 guards cell; …` + spawn region | composed | `["Q600"]`, `T08-COMPOSE-GUARD` |
| the same ranked body with no plan | mutex | `["Q600"]`, the same mutex-route refusal |

Both are refused with or without the plan, so **nothing that was admitted stops
being admitted and nothing that was refused becomes admitted**. The composed
refusal is deliberate and explicit rather than incidental: the epoch profile
models no guard and the mutex profile models no accounting, so publishing the
pair would assert a composition neither checker was given. It is raised in
`split_composed_epoch`, before either lowering runs, so the message names the
composition instead of leaving the user with another profile's complaint.

A rank does not change this: a ranked mutex is still a mutex generation, and
`T03-GUARD-JOIN`'s `thread scope c under g` is the *mutex* route's own
child-task form, untouched here. Both are pinned by
`a_mutex_generation_is_refused_deliberately_with_its_plain_control`, which
asserts the composed refusal, the plain control's refusal, and that the plain
control's message comes from the mutex route rather than from this card.

---

## 5. Where the work is done, and where it is not

| Component | Change |
|---|---|
| `crates/qcore-frontend/src/source_epoch_tasks.rs` | **new** (1,073 lines): route detection, the split, the ledger derivation, the agreement relation, the plan grammar |
| `crates/qcore-frontend/src/source_epoch.rs` | nine private helpers become `pub(crate)`; **no behavioural line changed** |
| `crates/qcore-frontend/src/lib.rs` | one `pub mod` |
| `crates/qcore-verification/src/driver.rs` | `compile_composed_epoch_source`, `ComposedEpochModule`, one `CheckedSourceModule` variant, one `SourceExecutionProfile` variant, route dispatch before the pure epoch route |
| `compiler/src/lib.rs` | one re-export |
| `crates/qcorec/src/main.rs` | one profile label on the existing match |
| `qcore_core::epoch_accounting` | **untouched** |
| `qcore_core::task_execution` | **untouched** |
| `qcore_frontend::source_tasks` | **untouched** |
| `qcore_frontend::source_mutex` | **untouched** |

The driver's order is: split → parse/elaborate the blanked text →
`compile_task_source` (the whole scoped-task route, called as-is) → derive the
ledger from the resulting `CheckedTaskModule` → reconcile → `check_flat_epoch` →
`recheck_epoch_bound` → the latency claim. A certificate is never published
without being independently re-derived, exactly as on the pure route.

`CheckedSourceModule::as_scoped_tasks` and `as_checked_epoch_bound` both answer
`None` for a composed module, deliberately: a composed module's task evidence is
one half of a pair, and a consumer that wants it must ask for the composition
and see the accounting half beside it. `as_composed_epoch_tasks` is the new
accessor, and `ComposedEpochModule` also carries the scoped-task sub-profile
label (`ScopedOwnedTasksV1`, `ScopedCopyTasksV1`, …) so the composed label does
not hide what the region does.

**Reference execution and emission flow through the existing task path.**
`the_emitted_c_is_the_task_routes_own_output_byte_for_byte` compiles the
composed module and the blanked text separately and asserts `c_source`,
`source_map`, `entry` and the reference-observation shape are equal. The epoch
adds a checked accounting statement on top and emits nothing of its own.

---

## 6. Evidence

### Rust — 21 new tests, all green

`crates/qcore-frontend/tests/source_epoch_tasks.rs` (12), on the composition
itself:

| Test | Covers |
|---|---|
| `route_detection_needs_both_halves_and_claims_neither_alone` | an epoch alone keeps G02e; a task module alone keeps the task route; only both select this one |
| `the_split_blanks_the_plan_and_preserves_every_byte_offset` | length, lines, both outside regions byte-identical, blanks only inside, and the blanked text is a scoped-task module |
| `the_ledger_is_read_out_of_the_checked_task_ir` | captures, result kind and disposal derived from the checked module; `derive_ledger` alone reproduces the lowering's ledger |
| `every_declared_figure_arrives_in_its_own_slot_and_the_shape_is_derived` | full transcription; empty shared storage; `joined` derived; the row names the creation it agreed with |
| `an_elided_join_is_derived_and_a_declared_one_is_checked` | elided ⇒ `Derived` and the same program; declared-and-agreeing ⇒ `Declared`; both wrong declarations refuse |
| `rows_and_creations_must_correspond_in_both_directions` | undeclared worker; over-declared worker |
| `captures_must_correspond_element_by_element` | missing, extra, wrong mode, and a capture naming a place |
| `the_result_kind_must_correspond_to_the_checked_worker_result` | retained missing for an owned result; retained declared for a scalar result; a disposal on a resultless row |
| `a_copy_only_region_composes_with_a_resultless_row` | the copy/scalar shape end to end |
| `the_grammar_refuses_what_this_composition_cannot_represent` | seven refusal shapes plus the three mandatory row figures and the two mandatory headers |
| `the_plan_is_the_first_statement_and_the_only_epoch_scope` | a misplaced plan; a plan accounting for nothing |
| `a_mutex_generation_in_the_composed_body_is_refused_deliberately` | both the plain and the ranked guard spellings |

`compiler/tests/epoch_tasks_source.rs` (9), end to end through `compile_files`,
every refusal asserting the **exact ordered code vector**:

| Test | Covers |
|---|---|
| `the_composed_route_publishes_both_halves_and_neither_is_the_other` | the profile label, both `None` accessors, every certificate figure, all four premises, the task module's own function count |
| `the_emitted_c_is_the_task_routes_own_output_byte_for_byte` | emission parity against the blanked text compiled alone |
| `two_workers_with_different_capture_modes_compose` | the CT20 shape: two workers, mixed modes, task identities in creation order, summed peaks (1280), summed work (14) |
| `the_two_existing_routes_are_unchanged` | a plain task module and a plain epoch module keep their routes and their evidence |
| `agreement_failures_refuse_at_the_structural_code_in_both_directions` | eight agreement failures, each `["Q600"]` |
| `an_elided_join_is_derived_from_the_checked_region` | derivation end to end, and program equality with the declared form |
| `the_profiles_own_refusals_still_come_from_the_cores_band` | `["Q633"]`, `["Q637","Q638"]`, `["Q639"]`, `["Q640"]` |
| `shapes_this_composition_cannot_represent_are_named_refusals` | `T08-COMPOSE-SHARED`; the measured `Q013`-first ordering; the misplaced plan |
| `a_mutex_generation_is_refused_deliberately_with_its_plain_control` | the guard/rank matrix of §4 with its controls |

### Measured diagnostic vectors

| Source | Cascade | Message (abridged) |
|---|---|---|
| creation with no row | `["Q600"]` | `the plan declares 0 row(s) for worker \`worker\` and the checked task region creates 1 task(s) with it` |
| row with no creation | `["Q600"]` | `the plan declares 1 row(s) for worker \`ghost\`, which the checked task region never creates` |
| missing capture | `["Q600"]` | `the plan declares captures [] for \`worker\` and the checked creation passes [move]` |
| extra capture | `["Q600"]` | `… declares captures [move, move] …` |
| wrong capture mode | `["Q600"]` | `… declares captures [copy] … passes [move]` |
| capture naming a place | `["Q600"]` | `a composed capture clause is exactly \`move;\`` |
| diverging disposal | `["Q600"]` | `the plan declares \`join(move)\` for \`worker\` and the checked task region performs \`join(drop)\`` |
| `retained` for a scalar result | `["Q600"]` | `… whose checked result type is not an owned Buffer` |
| no `retained` for an owned result | `["Q600"]` | `… returns an owned Buffer … so its row declares \`retained BYTES;\`` |
| disposal on a resultless row | `["Q600"]` | `… which retains no result to dispose of` |
| shared cell in a composed plan | `["Q600"]` | `T08-COMPOSE-SHARED` |
| mutex generation in the body | `["Q600"]` | `T08-COMPOSE-GUARD` |
| plan not first | `["Q600"]` | `the epoch plan is the entry's first statement` |
| `retained 64;` with `peak 32;` | `["Q633"]` | `RetainedResultExceedsTaskPeak { task: 1, axis: "memory", result: 64, task_peak: 32 }` |
| `bound 1024 effort 10;` | `["Q637", "Q638"]` | `CreditsExceedAllocation` then `EpochExceedsAllocation` |
| `bound effort as latency;` | `["Q639"]` | `WallClockUnavailable` |
| `overhead … measured 30;` | `["Q640"]` | `MeasuredOverheadIsNotADeclaredBound` |
| `read page;` with `page` undeclared | `["Q013"]` | name resolution refuses the name before the composition sees the clause |

The last row is a measurement, recorded rather than worked around: end to end,
a `read` clause naming an undeclared cell never reaches the composition. Its own
refusal is reachable at the lowering (covered by the frontend test), and both
orders refuse, so nothing is admitted either way.

### Lean — 30 new audited theorems, no new axiom

`proof/QCore/EpochSourceLowering.lean` gains a composition section; the audit
list in `proof/AuditEpochSourceLowering.lean` grows from 37 to 67 names.
**The audit *file* count is unchanged at 51**, so the acceptance matrix's
derived-facts test is not disturbed. Every new theorem depends only on `propext`
and `Quot.sound`; no `sorry`, no placeholder.

* *The matching half, in general* — `agrees_implies_the_counts_match`,
  `a_creation_names_its_worker`, `a_row_names_its_worker`, and the two
  direction theorems `a_creation_with_no_row_is_not_agreed` /
  `a_row_with_no_creation_is_not_agreed`, proved for arbitrary row and ledger
  lists, not just for the worked examples (5).
* *The per-pair half, by `decide` on the worked compositions* — a missing,
  extra and wrongly-moded capture; a declared result the creation does not
  retain and a retaining creation with no declared result; a diverging
  disposal, a completion disposal, and a disposal on a row that retains
  nothing; a second row for one creation and a second creation for one row (10).
* *Non-vacuity* — four theorems assert concrete compositions **are** agreed
  (owned, copy-only, two-worker, and the two-worker case with the rows written
  in the other order), so the refusals are not proved against a relation that
  refuses everything. A fifth checks the worked ledger entries are themselves
  wellformed (5).
* *The derivation* — `an_elided_join_agrees_and_is_the_regions_disposal`: an
  elided join agrees, agrees with exactly the region's disposal, and every other
  disposal is refused (universally quantified over `Disposal`) (1).
* *Composition into the epoch* — the composed worker preserves each declared
  figure; the composed task count is the region's; a composed epoch has no
  entry allocation; a composed worker is admitted; **an agreed composition is
  admitted**, and therefore
  `every_configuration_of_a_composed_epoch_is_bounded` follows from the file's
  existing `every_configuration_of_an_admitted_declaration_is_bounded` with no
  new machinery. A concrete owned composition is admitted by `decide` (7).

That last chain is the payoff: agreement plus the existing admission theorem
gives the model bound for a composed epoch, so composition adds a relation to
the region rather than a way past the accounting.

The Lean transcription of the Rust derivation is **by inspection**, as
everywhere else in this namespace; there is no digest-checked correspondence of
the F01 kind.

---

## 7. Standing per-change gate

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok |
| `cargo test --workspace` | **2055 passed, 0 failed, 4 ignored** (21 new; baseline 2034 at `695f0e1`) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **Passed 51 Lean audits** (unchanged) |
| `python3 -B -m unittest discover -s scripts/tests` | 125 tests, OK |
| `cargo test` in `providers/` | 76 passed, 0 failed |

One load-flake was observed and re-run, as the card brief anticipated: P03's
`a_confined_fixture_cannot_write_outside_the_admitted_paths` failed once under a
parallel workspace run and passed both in isolation (12/12) and on the next full
run (2055/0/4). Nothing in this card touches the verifier host; the assertion
was not weakened. `P02 RLIMIT_NPROC` and `vp04` passed every time.

---

## 8. What this does not establish

* **No quantity is imported from anything.** Peaks, results, credits, the
  allocation and the overhead remain declared figures, and all four
  `FlatEpochBoundV1` premises (`DeclaredTaskFootprints`,
  `DeclaredRuntimeOverhead`, `DisjointSharedStorageCells`, `NoScheduleModel`)
  are carried unchanged on every certificate this route publishes. Agreement
  makes the bound be *about* the region that runs; it does not make any figure
  in it a measurement.
* **The bound is still not a time.** `epoch_wall_clock_bound` still always
  refuses.
* **No native or sanitizer evidence is added.** The emitted C is the
  scoped-task route's own, and this card runs no new native fixture, so any
  claim about what the composed program does at run time is the task route's,
  not this one's.
* **The composition is with one profile.** Counted read shares (T05), typed
  owned error payloads (T06), region task credit (T07), disjoint footprints
  (T09) and guards (T03) are all outside it, each with a named follow-on below.
* **Rust/Lean correspondence is by inspection.**
* **Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; the contract corpus stays 60
  modules; CT coverage stays 30/50 mapped; `qcore-schemas` stays 8.0.0. This
  card touches no audit path and no pinned artifact.

---

## 9. CT proposal — recorded only, no manifest edit

`docs/reevaluation/ct_manifest.json` and `docs/reviews/r0_3/acceptance.json` are
**untouched**; CT coverage stays **30/50**.

The four epoch cases CT35/CT36/CT38/CT39 are already mapped (wave 8) on the
check-only `epoch` route, where `native_sanitized` is `not_applicable` by the
route rule. A composed fixture would change that for them: a composed module
*does* emit C and *does* have a scoped-task execution stage, so a CT lane could
in principle strengthen those four from `epoch` to a composed route carrying
both the `epoch_bound_figures` verdict mode and the task route's execution
stages.

**This card does not propose that, and the reason is measured.** Restating
CT35/CT36 over a composed fixture would change what those cases are about: their
evidence is the *shared-cell union* (CT35) and the *summed private peak with
declared overhead* (CT36), and this composition has **no shared storage at all**
(`T08-COMPOSE-SHARED`) and derives no peak. CT38's credit split and CT39's
latency refusal do carry over unchanged — both are pinned here by
`the_profiles_own_refusals_still_come_from_the_cores_band` — but re-mapping them
would grow no evidence class, only move a fixture.

What a CT lane could honestly add is a **new** case for the composition itself,
if the corpus has an unmapped case about a quantitative bound over a real
concurrent region. No such case was identified in the current CT01–CT50
inventory, so the recommendation is: **no CT change**. Recorded as
`T08-COMPOSE-CT`.

---

## 10. Open, named

| Id | Item |
|---|---|
| `T08-COMPOSE-SHARED` | The composition has no shared storage: `let NAME = share N;`, `read NAME;` and `write NAME;` are refused, because the copy and owned task forms have no shared cell for them to name. A shared-cell source form for the *task* profile would have to land first. |
| `T08-COMPOSE-LOAN` | Read loans (T04) and counted shares (T05) are refused as composed captures: a loan's residency is shared with the lender, which this flat accounting does not represent. |
| `T08-COMPOSE-GUARD` | A mutex generation anywhere in the composed body is refused, ranked or not (§4). Composing the accounting profile with the guard profile needs a model that has both, which neither checker is. |
| `T08-COMPOSE-RESULT-REUSE` | A joined owned result the creator hands on — into another creation, out of a return, or as a raise payload — is refused: it is neither released at the join nor live in the creator at the epoch exit, and the flat epoch has no residency for it. |
| `T08-COMPOSE-EXIT` | `ResultDisposal::TransferredToCreator` is unreachable from this profile (§3), so a composed epoch's exit residency is always the epoch overhead alone. Lifting it is a task-profile change, not a composition change. |
| `T08-COMPOSE-PAYLOAD` | Typed owned error payloads (T06) and region task credit (T07) are untouched: a payload carried out along an error edge has no composed row spelling. |
| `T08-COMPOSE-DERIVE` | No byte or work figure is derivable, because `Buffer` has no extent and the profile has no cost model (§3). Deriving a peak would need an extent on the allocation form first. |
| `T08-COMPOSE-CT` | The CT recommendation of §9: no manifest change, with the reason recorded. |
