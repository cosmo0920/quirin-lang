# OBS-REORDERING — comparison up to a declared reordering, and `COMPOSE-WORKER-LINES` (wave 22)

Card brief: *"Build the **comparison-up-to-an-admitted-reordering-relation**
capability: a checked way to compare a native run's observation stream against
the reference stream when a bounded, DECLARED set of interleavings is
admissible — the capability that `COMPOSE-WORKER-LINES`, `STD-LOGGER-ASYNC`'s
ordering half, and future event nondeterminism all wait on."*

Base commit: main `ec5535a9`.

**What landed.** One new model, `qcore.obs-reordering.v1`
(`crates/qcore-core/src/obs_reordering.rs`), normatively stated in
`docs/design/OBS_REORDERING_V1.md` and proved in a new `QCore.ObsReordering`
(29 audited theorems). Its first consumer is `source.std-io-tasks.v1`: a source
line may now stand in a **spawned worker's** body, is performed by that worker's
own thread with no buffering and no replay, and that program's streams are
compared window by window and emitter by emitter instead of byte for byte.

**Standing constraint 15a is moved by name** (§4). Its own refusal ground —
*"admitting one requires the comparison-up-to-an-admitted-reordering capability
(`COMPOSE-WORKER-LINES`), not a gate change"* — is exactly what this card lands,
so the constraint's condition is met rather than waived; the parent-only rule
survives for every shape the relation does not cover.

**No byte of any existing comparison moved.** The four published
`examples/threads` programs, the F02 emission digests, the CT fixtures and every
pinned trace byte are untouched — not by care but by construction: a program that
places no worker line declares nothing, and a relation whose windows each have
one emitter admits exactly one stream, its own. That is a theorem
(`strict_iff_byte_equality`), not a code path.

**One diagnostic code allocated: none.** `Q050`'s placement rule is *restated*
through §L21; `Q051`+ stay free for `STD-BUFFER`/`STD-EVENT` (§8.1).

---

## 1. The relation, stated exactly

An **observation stream** is what one C stream of one run carries: a sequence of
complete lines. Each line is a **record** carrying

* its **emitter** — the thread of control that wrote the bytes;
* its **window** — the concurrency region it was written in;
* its **text**.

Two records **commute** exactly when they lie in the same window *and* their
emitters differ. Equivalently, and this is the form to remember: **two records of
one emitter never commute** (a thread performs its writes in its own program
order), and **two records never commute across a window boundary**.

A **window boundary** is a point of the run at which **no worker is live** — no
task created and not yet collected. At such a point the parent is the only thread
of control, so a record written there is ordered after everything before it and
before everything after it. A record written at such a point is a **barrier**: it
is alone in its window and commutes with nothing.

Closed form, which is what the comparator decides:

> `s ≡ t` iff the window sizes agree and, for every window `w` and every emitter
> `e`, the subsequence of `w` written by `e` is byte-identical in both.

### 1.1 Emitter is the **writer**, which is the opposite of F02's attribution

`qcore.task-trace-relation.v1` attributes an observed line to *the task whose
state change it reports* — its premise P2 — because it relates **causal** order.
This relation attributes a record to *the thread that performed the write*,
because it relates **byte** order, and bytes are ordered by writers.

The two disagree on exactly one family of records: a worker's buffered trace
events, replayed by the parent inside `qcore_task_collect`. F02 reads them as the
child's actions; this relation reads them as **parent** records. Each is right
for its own question, and the disagreement is recorded rather than reconciled.

### 1.2 What derives it — and what does not

| Ingredient | Read from | **Not** read from |
|---|---|---|
| the records and their texts | the composed reference execution (`ComposedTaskObservation`) | any native run |
| a trace record's emitter | the emitted runtime's structure: every trace line is written by the parent's thread, immediately or at collect | the task id inside the line |
| a source line's emitter | the task executing that line's function in the reference run | the line's text |
| the live set | `TaskEvent::Spawn` opens it; the recorded index at which each `qcore_task_collect` replay begins closes it | timing |

The relation is **the reference stream, annotated**. It is constructed only
through `AdmittedReordering::declare`, which re-checks every premise the
comparison will later rest on, and it is never derived from, adjusted to or
validated against a native run. A native run is only ever compared to it.

**The collect boundary is recorded by the reference runtime**, at the moment its
model performs the join, rather than recovered afterwards by scanning the
rendered events. A pattern scan was considered and rejected by measurement: a
worker that emits no buffer event renders as `SPAWN k`, `RESULT k`, `JOIN k`, and
the `SPAWN` record carries `k` in exactly the field a backwards scan would read,
so the scan would attribute the parent's own spawn record to the collect.

**Windows are numbered over the merged sequence across both streams, then
projected.** Numbering per stream would have merged two concurrency regions
separated by a quiescent point into one window and admitted reorderings the
program does not perform: a barrier written on `stdout` still separates two
`stderr` windows. This is the one place the two streams are read together, and it
is a *refinement* of each stream's own relation — no cross-stream order is
claimed anywhere (§7).

## 2. Soundness — what an accepted run has been shown to satisfy

Proved in `proof/QCore/ObsReordering.lean`, audited by
`proof/AuditObsReordering.lean`.

1. **Per-emitter order is preserved.** `equiv_preserves_emitter_projection`: for
   every emitter, the whole-stream projection is the same list. An accepted run
   performed each thread's writes in that thread's program order, so the strict
   statement about a single-threaded emitter survives intact.
2. **Barrier structure is preserved.** `barrier_is_carried_across`,
   `equiv_preserves_barriers`: a window holding one record is carried across
   unchanged, record for record and position for position. The conservative
   reading `COMPOSE-CONCURRENCY` proposed — "before any spawn, between joins,
   after all joins" — *is* the set of barrier positions, so that card's rule is
   the barrier part of this relation, unchanged.
3. **Per-window content is preserved.** `equiv_preserves_window_content`,
   `equiv_window_sizes`: nothing dropped, duplicated or invented.
4. **`≡` is an equivalence relation.** `equiv_refl`, `equiv_symm`,
   `equiv_trans`: "these two runs agree" is a property of the pair, not of which
   one was called the reference.
5. **The strict comparison is the empty-relation case.**
   `strict_iff_byte_equality`: when every window has at most one emitter,
   `s ≡ t ⟺ s = t`. `barriers_are_strict` adds that a program writing only
   barriers is of that kind. **This is why this card replaces no existing
   comparison** — it generalises it, and every route before this card is the
   special case.
6. **The generators agree with the closed form.** `distinct_emitters_commute`
   and `one_emitter_never_commutes`: transposing two adjacent records of distinct
   emitters inside a window lands inside `≡`; transposing two of one emitter with
   different bytes does not.
7. **The decision procedure is exact.** `decideEquiv_correct`: the transcription
   of the Rust comparator's per-window, per-emitter loop returns `true` on
   exactly the equivalent streams.

### 2.1 Fail-closed — every way out is a refusal

| Refusal | Class | When |
|---|---|---|
| `WindowsNotMonotone` | 1 | at **declaration**: a record sequence whose windows go backwards. This derivation cannot produce one; a hand-built one refuses. |
| `AmbiguousAlphabet` | 2 | at **declaration**, before any run exists: two distinct emitters of one window would write the same bytes, so a native line's writer is not a function of the bytes and per-emitter order is not checkable from the stream. **The relation refuses rather than weakening silently to a multiset comparison.** |
| `LengthMismatch` | 3 | at comparison: the observed stream is not the declared length. |
| `Residue` | 4 | at comparison: a line no emitter of its window writes. |
| `EmitterOrder` | 5 | at comparison: one emitter's lines are not its declared lines in its declared order. |

Nothing is normalised, reordered or repaired, on the F02 precedent.
`AmbiguousAlphabet` is a real restriction on which programs can be compared this
way, and it is stated rather than engineered around: §5's fixture gives its two
workers different numbers *on purpose*, and the refusal is pinned from source
(`two_workers_writing_the_same_line_refuse_the_declaration`).

## 3. What `Q050` admits now, and what it refuses

`Q050` keeps its code and its pass. Rule (1) **Placement** is restated in §L21
and rule (1a) is added; rules (2)–(4) are unchanged.

| Shape | Before | Now |
|---|---|---|
| a line anywhere in the **parent** body | admitted, compared byte for byte | **unchanged**, byte for byte |
| a line in a **spawned worker**'s body, that worker declaring `io` | `Q050` | **admitted**; placing one declares the relation, and that program's streams are compared up to it |
| a line in a spawned worker's body, that worker **not** declaring `io` | `Q050` (placement) | `Q050` (**`WorkerEffectNotDeclared`**) — the effect discipline is per function and a worker body is a function body |
| a line in a function that is neither the entry nor spawned | `Q050` | `Q050` (**`SiteOutsideProgram`**) — nothing makes such a function run, so its line has no thread to attribute it to and no window to place it in |
| a `borrow NAME` rendering | `Q050` | `Q050`, unchanged (`COMPOSE-TASK-BORROW`) |
| an operand the task checker will not read there | `Q050` | `Q050`, unchanged — and now asked of the **site's own function**, so the probe runs in the worker for a worker line |
| an asynchronous logger's **bounded queue** | refused (no such construct) | refused, and for the reason it always was: no queue exists on this route, so no window boundary can be derived for one. `STD-LOGGER-ASYNC` still needs its own declared drain point (§9) |
| two emitters of one window writing the same bytes | — | the program **checks**; the *comparison* refuses at `AmbiguousAlphabet`, before any native run is looked at |

The last row is deliberate and worth stating plainly: placement is `Q050`'s
question and that placement is admitted; whether the resulting streams are
comparable is the relation's question, and it is answered when the relation is
declared from the reference run — still before any bytes of a native run are
read. Making it a `Q050` refusal would require the checker to know statically
which numbers a program will render, which on this route it does not: that is the
same fact `COMPOSE-CONCURRENCY` §8.2 recorded when it left
`StdIoProgram::scalars` empty.

## 4. Standing constraint 15a, moved by name

Quoted in full from `AGENTS.md` as of `ec5535a9`:

> **15a. Source lines in a composed task module are parent-body-only.** The
> `source.std-io-tasks.v1` admission rests on a measurement, not a preference:
> every write an emitted task program performs is performed by the parent's
> thread (workers buffer events privately; the parent replays them at collect),
> so any parent-body point is join-ordered and the per-stream byte comparison
> stays well-defined. A worker-body line refuses at `Q050`; **admitting one
> requires the comparison-up-to-an-admitted-reordering capability
> (`COMPOSE-WORKER-LINES`), not a gate change.** `wave21-compose-concurrency`.

**The condition is met, not waived.** The constraint names the capability whose
absence was its ground, and this card lands that capability as a model with its
own normative document, its own fail-closed refusals and its own audited Lean
proof of the soundness statement. The gate was not widened; a new comparison was
built and the gate now admits exactly what that comparison can decide.

**What the constraint said that is still true, and is kept:**

* The measurement it rests on is unchanged and is now *load-bearing in two
  directions*: every **trace** write is still performed by the parent's thread,
  which is why a parent line's position is determined **and** why `stdout` on the
  new fixture is still compared strictly.
* A parent-body point is still join-ordered, and a parent-only program's
  comparison is still byte for byte — now as a theorem
  (`strict_iff_byte_equality`) rather than as a rule.
* A worker-body line still refuses whenever it is not covered: without `io` on
  that worker's own row, in a function the program never runs, or with a
  `borrow` operand.

**Proposed replacement text for the integrator** (this card may not edit
`AGENTS.md`):

> **15a. A source line in a composed task module stands in the parent's body or
> in a spawned worker's, and the two are compared differently.** A parent line's
> position is determined because every *trace* write an emitted task program
> performs is performed by the parent's thread (workers buffer events privately;
> the parent replays them at collect), so a parent-only program's streams are
> compared **byte for byte** and every published `examples/threads` program is of
> that kind. A **worker**-body line is written by that worker's own thread, and
> placing one **declares** `qcore.obs-reordering.v1`: that program's streams are
> then compared window by window and emitter by emitter, where a window boundary
> is a point at which no worker is live and an emitter is the thread that wrote
> the bytes — the opposite of F02's causal attribution, and right for the
> opposite reason. The strict comparison is that relation's own degenerate case
> (`strict_iff_byte_equality`), so nothing was replaced. Do **not** weaken the
> two premises the declaration rests on: two emitters of one window may not share
> a line's bytes (`AmbiguousAlphabet` refuses the declaration, never a multiset
> comparison), and a worker that writes declares `io` in its **own** effect row.
> `wave21-compose-concurrency`, `wave22-obs-reordering`.

The test that carried the old pin,
`thread_examples.rs::a_worker_body_cannot_carry_a_source_line`, **failed and was
rewritten, not deleted** — the precedent `COMPOSE-CONCURRENCY` itself set when it
moved `TRACE-STDIO`'s pin. It is now two tests:
`a_worker_body_carries_a_source_line_when_that_worker_declares_io` (the exact
module 15a refused now compiles, and its certificate says it declares the
relation) and
`a_worker_line_still_needs_its_own_io_effect_and_the_examples_declare_nothing`
(the same module without `io` still refuses at `Q050`, and all four published
examples still declare nothing).

## 5. The acceptance — `COMPOSE-WORKER-LINES`

`compiler/tests/obs_reordering.rs`. The fixture:

```qcore
fn worker(value: i32) -> i32 effects { io } {
    value |> log info dec;   value |> log warn dec;   value |> log fatal dec;
    value |> log info hex;   value |> log warn hex;   value |> log fatal hex;
    return value;
}

fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    let first_value = 42;   let second_value = 7;
    log info dec 0;                       // nothing live: a barrier
    thread scope workers {
        let first = spawn workers worker(copy first_value)?;
        match spawn workers worker(copy second_value) {
            Result::Ok { value: second } => {
                log info dec 9; log warn dec 9; log fatal dec 9;   // in the window
                let second_result = join move second?;
                let first_result = join move first?;
                first_result |> log info dec;   // both joined: a barrier again
                second_result |> print dec;
                return first_result;
            }
            /* … the refused-creation arm … */
        }
    }
}
```

The derived `stderr` relation, asserted in
`the_derived_relation_is_the_spawn_join_structure`:

| Window | Size | Records |
|---|---|---|
| barrier | 1 | parent `[INFO] 0` |
| concurrent | 15 | worker 1's six, worker 2's six, parent's three — three emitters, so `15!/(6!·6!·3!)` = 168,168 admitted streams |
| barrier | 1 | parent `[INFO] 42` |

The derived `stdout` relation has **one** emitter and admits exactly one stream,
its own (`the_standard_output_relation_is_strict`) — *"the parent's lines and the
synthesized trace keep strict order"*, as a fact about the derived relation
rather than as a convention. The acceptance test therefore compares `stdout`
with `assert_eq!` and `stderr` through the relation.

**It is not a new published example.** `examples/threads` stays at four. A fifth
published example would need a transcript in `README.md`, and this program's
`stderr` transcript is *not a function of the program* — which is the whole
point. It is a test fixture, and the four published programs keep their byte
comparison and their documented transcripts untouched.

## 6. The loop, and the count

`many_sanitized_runs_are_each_admitted_by_the_declared_relation` compiles the
emitted C once — `-std=c11 -pedantic -pthread -Wall -Wextra -Werror -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, `detect_leaks=1`,
`halt_on_error=1` — and runs it `RUNS` times, comparing `stdout` strictly and
`stderr` up to the relation on every run.

### 6.1 The measurement

Six sessions at each count, on this machine, with five sibling agent builds
running throughout (so these are loaded-machine numbers, not quiet ones):

| Runs | Distinct `stderr` orders observed, per session | Sessions exercising an interleaving |
|---|---|---|
| 64 | 1, 1, 2, 1, 2, 2 | 3 of 6 |
| 256 | 14, 13, 11, 1, 6, 2 | **5 of 6** |

Cost at 256: one C compile plus 256 executions of a two-thread program — 1.7 s on
a quiet machine, 8 s under sibling load. The existing `thread_examples`
sanitized suite compiles and runs four programs and costs more.

**`RUNS = 256`**: the smallest power of two at which most sessions observe more
than one order, at a cost that is a fraction of an existing suite.

### 6.2 The fixture shape was measured too

A worker writing **one** line wins or loses its race with the parent as a single
event, and the outcome was stable: an earlier shape showed 1 distinct order in
256 runs. Six worker lines each spans enough of the run for the parent's writes
to land *inside* a worker's stream, which is the interleaving the relation exists
for. One observed order, verbatim from a measurement session, splices worker 2's
six lines between the parent's `[INFO] 9` and `[WARN] 9`.

### 6.3 The relation is load-bearing, not decorative

**In every session, on essentially every run, the native `stderr` is not the
reference's own order.** The reference executes each worker eagerly at its spawn
(`reference.rs`: *"Eager reference evaluation selects a legal schedule"*), so its
declared stream is `0, [42×6], [7×6], 9,9,9, 42`; the native program reliably
produces `0, [42×6], 9,9,9, [7×6], 42` and variants. A strict byte comparison of
this fixture fails on the first run, every time. The counts above measure how
much *further* variety the scheduler supplies; they do not measure whether the
capability is needed.

### 6.4 No assertion depends on the count

The distinct-order count is **printed, never checked**. Observing one order is a
legal outcome of a legal scheduler, and asserting on it would be a flake. What is
asserted is that every run is `R`-equivalent to the reference. The
non-vacuousness of the relation is pinned **deterministically** instead, in
`a_reordering_is_admitted_and_a_reversal_within_an_emitter_is_not`: a stream
built by regrouping each window's records by emitter is admitted and is not the
declared order; a barrier moved into the concurrent window is refused; one
emitter's own two lines transposed is refused; a line the program never wrote is
residue.

## 7. The trust statement

In the premise vocabulary of standing constraint 28. These are premises and
boundaries, not proofs and not TODOs.

1. **This is not a memory model.** No claim about visibility, atomicity of any
   shared object, data races, or the ordering of anything other than complete
   stream records. **TSan is not run and no TSan-shaped claim is made anywhere.**
2. **No cross-emitter ordering claim.** Inside a window the relation asserts the
   *absence* of order between distinct emitters. It does not assert that any
   particular interleaving happens, is reachable, or was taken.
3. **No cross-stream order.** `stdout` and `stderr` remain two independent
   observations, as on every route that writes. The merged sequence is used only
   to *number windows*, which refines each stream's own relation and relates the
   two to nothing.
4. **Premise `C11StreamLock`.** A record is a whole line because one `fprintf`
   runs under the stream's own lock (C11 7.21.2p7–p8), so two threads writing
   through `QCORE_IO_WRITE` cannot tear one line into another. A stream without
   that lock is outside this relation and no attempt is made to detect one.
5. **Premise `C11ThreadJoinSynchronizes`.** A window boundary is a happens-before
   edge because `thrd_join` synchronizes with the joined thread's completion
   (C11 7.26.5.6). Everything §1 says about quiescent points rests on that and on
   the profile's leaf-worker rule, not on timing.
6. **An accepted run is one run.** Acceptance says that run's stream lies in the
   admitted set. It says nothing about the other admitted orders being reachable,
   nothing about a run not performed, and nothing about the scheduler. Repeating
   the comparison is not a coverage claim.
7. **Not a correctness proof under reordering.** The relation states which byte
   orders the checked model admits. Whether admitting them is right for a given
   program is that program's own question.
8. **The Lean side proves the relation, not the derivation.** That the Rust
   derivation reads the window structure faithfully off a run's spawn/collect
   events, and that the emitted C performs the writes at all, are tested Rust
   boundaries — the `ConcurrencyPrelude` precedent.

## 8. What did not move

### 8.1 The F02 digests — **pinned, not moved**

The relation is **comparison-side**, not emission-side. `task_trace_relation.rs`
is untouched; `compiler/tests/f02_trace_relation.rs` and
`compiler/tests/windows_threads.rs` are untouched; their fixtures import no
`std.io`, stay on the task route, and travel `emit_task_c11_with_source_lines`
with an empty site list, which is the same code path it always was. Pinned by
their own unchanged assertions passing in the gate (§10).

Neither is any diagnostic code allocated. `Q050`'s placement rule is a
*restatement* in §L21 — the condition of a rule that pass already owned changed;
its ownership did not — and rule (1a) is an addition inside the same rule's
subject. A new code would have split one placement rule across two codes for no
gain. **Assumption stated as the brief asks:** `Q051` and above remain free, so
`STD-BUFFER` and `STD-EVENT` may allocate there without consulting this card;
relocation per T07/T09 is the integrator's, and nothing here depends on `Q050`'s
numeric position.

### 8.2 The four published examples

Byte-identical, on both streams, and pinned three ways: the existing
`every_published_example_matches_its_reference_execution_under_sanitizers` and
`the_documented_transcript_is_the_transcript_the_example_prints` are **unedited**
and pass; `a_parent_only_program_declares_a_strict_relation_on_both_streams`
adds, beside them, that each of the four declares nothing, that both of its
derived relations are strict, and that the strict relation's declared text is
*exactly* the projection the existing comparison uses.

### 8.3 The corpus

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/qcore/` or `providers/` is touched. The
contract-corpus pin (69 modules), the 185-unit artifacts, the leak count of
**272** and the CT manifest are main's.

### 8.4 Reserved words — the constraint-14 census

**Words added: 0.** The worker body uses `log`, `print`, `dec`, `hex`, `info`,
`warn`, `fatal` — all reserved by `STD-MODULES` — and `io`, which is an effect
name inside an `effects { … }` row and was already read there by
`COMPOSE-CONCURRENCY`. What changed is only *which functions'* rows may carry
`io` on this route. No `.qcore` file anywhere can newly collide.

## 9. Named follow-ons

| Name | What it is, and what it needs |
|---|---|
| `STD-LOGGER-ASYNC` *(ordering half)* | **The blocker named in `COMPOSE-CONCURRENCY` §2.4 is now half-lifted.** The comparison capability exists, and §2.1's commuting rule carries over with a *queue* as an emitter. What does **not** carry over is the **derivation**: a window boundary here is a join, and an asynchronous logger's boundary is its declared drain point, which no route has. Its *other* half — the task ledger still having no post-creation transfer — is unchanged and was re-measured by `COMPOSE-CONCURRENCY`; this card does not touch it. |
| `COMPOSE-EVENT-LINES` *(the event-nondeterminism consumer)* | the same capability over an event model's own concurrency regions. The comparator is already model-independent — it consumes records, emitters and windows and knows nothing about tasks — so what a future `source.event.*` needs is §1.2's live-set walk generalised to that model's regions, and nothing of the comparator. **`STD-EVENT` is deliberately not depended on here**: its v1 is deterministic without this, and composing the two is that card's own work. |
| `OBS-REORDERING-WORKER-ALPHABET` | the `AmbiguousAlphabet` restriction, made a *checkable* property instead of a comparison-time one. Needs the checker to decide which bytes a worker can render, which on this route means a static value analysis the composition deliberately does not have (§3). |
| `COMPOSE-WORKER-BORROW` | `borrow NAME` in a worker body. Strictly inside `COMPOSE-TASK-BORROW`, which is unchanged. |
| `TRACE-STDIO-WIN-RUN` | *(unchanged)* `build-windows.ps1 -Run` checks exit codes only. It now has one more thing it could check, and this card's fixture is not among the published examples so it is unaffected. |
| `COMPOSE-WORKER-LINES` | **retired by this card.** |
| `COMPOSE-TASK-BORROW`, `PIPE-INTRINSIC-STAGE`, `COMPOSE-TASK-EXTERN`, `COMPOSE-EPOCH-LINES`, `COMPOSE-MUTEX-LINES` | *(unchanged)* |

## 10. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test --workspace --no-fail-fast` | see §10.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **91/91**, no new axiom (90 → 91 for this card's new audit file) |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — see §10.4 |
| `cargo test -p qcore-compiler --test obs_reordering` | 10 pass, 256 sanitized native runs plus one published-example run |
| `cargo test -p qcore-compiler --test thread_examples` | 9 pass, four sanitized native runs, transcripts byte-identical |

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

### 10.1 The workspace run

*(filled in below at the closing measurement.)*

### 10.2 Tests added

| Target | Count |
|---|---|
| `crates/qcore-core/tests/unit/obs_reordering.rs` | **23 new**, against the relation directly — every refusal arm, the equivalence laws, the enumeration/comparison agreement over all 720 permutations of a six-record stream, and the two soundness statements |
| `crates/qcore-core/tests/unit/composed_tasks.rs` | **14 → 20**, six new for the worker-line admission, the declaration, the recheck and the per-function effect rule |
| `compiler/tests/obs_reordering.rs` | **10 new**, including the 256-run sanitized loop |
| `compiler/tests/thread_examples.rs` | **8 → 9**, the 15a pin moved into two tests; the other seven unedited (the module header records the move) |

### 10.3 Lean

New `proof/QCore/ObsReordering.lean`, audited by a new
`proof/AuditObsReordering.lean`, registered in `proof/QCore.lean` so
`AuditAll.lean`'s whole-project axiom closure covers it. **29 audited theorems**,
no new axiom, no placeholder, no `native_decide`, no `sorry`. Every audited name
reports only `propext`, `Classical.choice` and `Quot.sound`. No existing
`QCore/*.lean` or `Audit*.lean` file was edited — the one-line import in
`proof/QCore.lean` is the only change to an existing proof file.

Two `private` recursion helpers (`zip_forward`, `zip_backward`) are not listed
individually, on the `QCore.ComposedStdIo` convention: they are covered by
`AuditAll.lean`'s closure and transitively by `decideEquiv_correct`.

### 10.4 One expected failure, left for the integrator

`scripts/tests` reports `test_the_derived_facts_are_recomputed_rather_than_carried`
failing with `lean_audits: 90 != 91`. That is the derived-value tripwire working
exactly as designed: this card adds one audited Lean module, and
`docs/reviews/r0_3/acceptance_matrix.json` — which this card's brief forbids
editing — carries the pre-card count. Its own `section_ownership` block records
`validation.lean_audits` and `validation.lean_audit_files` as **derived** from
`proof/Audit*.lean` on disk, so regenerating them is the integrator's one-line
reconciliation. Base commit `ec5535a9` was self-consistent (90 audit files,
matrix value 90), so the delta is entirely this card's and is not a pre-existing
failure. The other 192 tests pass.

### 10.5 Integrator reconciliation list

* **No diagnostic code allocated.** `Q050`'s rule (1) is restated and a rule (1a)
  added, both in `docs/QCORE_SPEC_r0_3.md` §L21;
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02j's **Placement** paragraph is
  rewritten in the same change. `Q051`+ stay free.
* **Standing constraint 15a**: replacement text in §4. This card may not edit
  `AGENTS.md`.
* `docs/reviews/r0_3/acceptance_matrix.json` needs its derived `lean_audits` and
  `lean_audit_files` regenerated for one new audit file. Nothing else in it moves
  for this card. This card may not edit it, so the derived-value tripwire in
  `scripts/tests` will report the stale count — the same handling every wave's
  derived sections receive.
* **Public API changes in `qcore-core`**, all in `composed_tasks`:
  `TaskIoSite` gains `function`; `ComposedTaskProgram` gains `worker_io`;
  `ComposedTaskCertificate::placements` gains the function id and the struct
  gains `admits_reordering`; `ComposedTaskRule::SiteOutsideEntry` becomes
  `SiteOutsideProgram` and `WorkerEffectNotDeclared` is added;
  `ComposedTaskRecheckFailure` gains `Reordering`; `readable_probe` takes the
  site's function; `ComposedTaskObservation::lines` becomes `Vec<ComposedLine>`
  and the struct gains `collects`; `spawned_workers`, `ComposedLine`,
  `stream_records`, `admitted_reordering` and `compare_stream` are new.
* **New module** `qcore_core::obs_reordering`, depending on nothing outside
  `std`.
* `qcore_frontend::source_tasks::LoweredComposedTaskModule` gains `worker_io`;
  the builder's `is_entry: bool` becomes `function: usize`.
* `crates/qcorec/src/main.rs` gains four words in the composed profile's CLI
  label ("and in spawned workers"). No test asserts that text.
* `emit_task_c11`, `execute_task_module_with_worker_allocations` and
  `lower_source_tasks` keep their signatures and are still the widened function
  with an empty site list.

## 11. Draft increment paragraph

> The comparison-up-to-an-admitted-reordering capability, and its first consumer
> (OBS-REORDERING): `qcore.obs-reordering.v1` compares a native run's observation
> stream with a reference stream when a bounded, **declared** set of
> interleavings is admissible. A stream is a sequence of records, each carrying
> the **emitter** that wrote the bytes and the **window** it was written in; two
> records commute exactly when they share a window and differ in emitter, so a
> thread's own program order is never reordered and a **barrier** — a record
> written while no worker is live, where the parent is the only thread of control
> — commutes with nothing. The relation is the *reference* stream annotated from
> the checked model's own spawn/collect structure, constructed only through a
> `declare` that re-checks every premise the comparison will rest on, and never
> derived from or adjusted to a run. Emitter is the **writer**, which is
> deliberately the opposite of F02's causal attribution: the two disagree on a
> worker's buffered trace events, replayed by the parent, and each is right for
> its own question. **The strict byte comparison is not replaced; it is this
> relation at the declaration that admits nothing** — a theorem
> (`strict_iff_byte_equality`), which is why the four published
> `examples/threads` programs, the F02 emission digests, the CT fixtures and
> every pinned trace byte are untouched and their tests unedited. The consumer is
> `COMPOSE-WORKER-LINES`: a source line may now stand in a **spawned worker's**
> body, is performed by that worker's own thread with no buffering and no replay,
> and such a program declares the relation in its certificate. **Standing
> constraint 15a is moved by name, its own condition met rather than waived** —
> it refused a worker line "until the comparison-up-to-an-admitted-reordering
> capability", and this is that capability; the parent-only rule survives for
> every uncovered shape, with `Q050` now refusing a worker that writes without
> `io` in its **own** effect row and a line in a function the program never runs.
> Fail-closed throughout: two emitters of one window that would write the same
> bytes refuse the *declaration*, before any run is looked at, rather than
> weakening to a multiset comparison. The acceptance fixture is compared over
> **256** sanitized ASan/UBSan/LSan runs — `stdout` strictly, because its derived
> relation has one emitter, and `stderr` up to the relation — a count measured
> rather than chosen (6 sessions: 14/13/11/1/6/2 distinct orders at 256 against
> 1/1/2/1/2/2 at 64), with **no assertion depending on it** and the relation's
> non-vacuousness pinned deterministically instead. The native order is *never*
> the reference's own, so a strict comparison of that fixture fails on the first
> run. No reserved word and **no diagnostic code** were added: `Q050`'s placement
> rule is restated through §L21 and `Q051`+ stay free. 29 audited Lean theorems
> in a new `QCore.ObsReordering` — the equivalence laws, per-emitter order
> preservation, barrier preservation, the degenerate-case iff, the commuting
> generators and an exactness proof for the comparator's decision procedure — no
> new axiom. This admits reorderings the model declares; it is **not** a memory
> model, claims no cross-emitter and no cross-stream order, and runs no TSan.
> Corpus-neutral by construction. Evidence:
> `docs/reports/wave22-obs-reordering/README.md`.
