# F02 — one source-to-native trace relation

Baseline `c60fd91`, 2026-09-08. Work card F02 (`bounded_proof`, W6, Proof lane).
Prerequisite F01 complete.

**Deliverable: a scoped relation for one admitted source worker/capsule/join
path, preserving identity, cleanup and the relevant partial order — with its
conjuncts, its enumerated domains, its tested/proved split, its bound native
identity and its premises all named.**

F01 tied `check_concurrent` to its Lean model and closed with "no source-to-trace
and no trace-to-native relation (that is F02)". This card supplies one, for one
path, and measures exactly how far it goes.

## 1. The selected path, and why

`ScopedOwnedTasksV1`, the owned-`Buffer` worker/capsule/join round trip: the root
allocates a Buffer, moves it into one leaf worker, the worker returns the same
generation, the creator joins, reads the returned Buffer and releases it.

Two reasons, and one that is a fact about this worktree rather than a
preference:

* It is the **simplest admitted path that carries all three things the card asks
  the relation to preserve**: a resource *identity* that crosses a task boundary
  and comes back (`w6-owned-source-tasks`), a *cleanup* obligation that the
  checker discharges on every exit, and a *partial order* with two actors
  (`w6-completed-child-custody`).
* **T06 is not in this branch's ancestry.** The card offers a choice between the
  T06 owned-result/error-payload path and the simpler pre-T06 owned round trip;
  this worktree is branched from `c60fd91`, where `docs/reports/remaining-t06/`
  does not exist and no error-payload profile is implemented. The choice was
  therefore made for us, and the pre-T06 round trip is what is related here.
  Typed error outcomes are consequently outside every declared domain below,
  and both the Rust digest and the Lean alphabet refuse them rather than
  guessing an encoding.

Two source programs on that path are used:

| Program | Source | Facts | Actors |
|---|---|---|---|
| round trip | `compiler/tests/fixtures/ct/CT01/case.qcore` (unchanged) | 7 | root + 1 child |
| concurrent round trip | `compiler/tests/fixtures/f02/concurrent_round_trip.qcore` (new) | 11 | root + 1 child |

The second is new **fixture** text, not new language surface: it is the G02b
grammar already accepted by the checker. It exists because the plain round trip
has no parent fact between creation and join, so its admitted causal order has
exactly one linearization (§5) and could not exhibit the schedule freedom the
card's first acceptance criterion is about. The concurrent program gives the
parent a second Buffer to read and release while the child runs.

## 2. The two sides, by name

| Side | Name | File |
|---|---|---|
| Relation (Rust, production) | `qcore_core::task_trace_relation::relate` | `crates/qcore-core/src/task_trace_relation.rs` |
| — its reference side | `AdmittedCausalOrder::declare` / `::derive` | same |
| — its observer | `parse_observed_trace` | same |
| — its alphabet | `ObservedEvent`, `ObservedOutcome` | same |
| — its refusals | `TraceRelationViolation` (20 classes, `code()`) | same |
| — its witness | `TraceLinearization` | same |
| — bounded enumerator | `admitted_linearizations` | same |
| Reference run | `qcore_core::task_execution::execute_task_module` | `crates/qcore-core/src/task_execution/reference.rs` |
| Native backend | `emit_task_c11` and its `qcore_task_start`/`qcore_task_collect` runtime | `crates/qcore-core/src/task_c_emit.rs` |
| Relation (Lean transcription) | `QCore.TaskTraceRelation.relate` | `proof/QCore/TaskTraceRelation.lean` (new) |
| Domain harness (Rust) | `crates/qcore-core/tests/task_trace_relation.rs` (new) | — |
| Native observer (Rust) | `compiler/tests/f02_trace_relation.rs` (new) | — |
| Audit | `proof/AuditTaskTraceRelation.lean` (new) | — |

No production behaviour of the checker, the reference interpreter or the C
backend changed. `task_execution.rs`, `task_c_emit.rs` and `scoped_tasks.rs` are
byte-identical; the whole production diff is one new `qcore-core` module and one
line in `crates/qcore-core/src/lib.rs`.

## 3. The relation

Let `O` be a reference run and `T` an independently observed trace of the same
run — for example the stdout of the generated C11 program, read by
`parse_observed_trace`. `relate(O, T)` succeeds when all nine conjuncts hold.
They are checked in this order, so a refusal's class is stable.

| # | Conjunct | Classes |
|---|---|---|
| R1 | **Attribution.** Every observed line names a task. | 1 |
| R2 | **Identity.** The occurrences are pairwise distinguishable, and `T` reports exactly `O`'s facts — same generations, payloads, task identities and outcomes. | 2, 3 |
| R3 | **Program order.** For each actor, `T` restricted to that actor equals `O` restricted to it. | 4 |
| R4 | **Allocation.** One allocation per generation, before every other fact about it, and every read reports the allocated payload. | 5, 6, 7, 8 |
| R5 | **Release.** One release per generation, after every other fact about it, and every allocated generation is released. | 9, 10, 11 |
| R6 | **Creation.** One creation per child, before every fact by or about it. | 12, 13, 14 |
| R7 | **Completion and join.** One join per child, preceded by exactly one completion; no fact by that child afterwards; every created child is joined. | 15, 16, 17, 18 |
| R8 | **Loan.** A loan precedes its borrower's reads and is closed only after the join. | 19 |
| R9 | **Termination.** The root's completion is the final fact. | 20 |

R2 and R4/R5's counting conjuncts are the **identity and cleanup** half; R3 and
R4–R9's ordering conjuncts are the **partial order** half.

What is deliberately *not* a constraint: cross-actor order. Nothing requires the
child's facts to sit where the reference put them relative to the parent's, only
that they sit after the creation and before the join. That is the whole point of
the card's first acceptance criterion, and §5 measures it.

### 3.1 The premises the relation rests on

* **P1 (sequential actors).** Each task in this profile is one sequential thread
  of control, so the reference run's order *restricted to one actor* is that
  actor's program order and is a genuine causal constraint. This is the only
  place the reference's order is used. Cross-actor order in the reference is not
  a constraint and is never consulted.
* **P2 (attribution).** An observed line is attributed to the task whose state
  change it reports, not to the thread that printed it. This matters: the
  shipped backend buffers a child's Buffer facts in the child's private context
  and prints them *from the parent* after a successful join
  (`task_c_emit.rs`, `qcore_task_collect`). The relation reads them as the
  child's.
* **P3 (flat scopes).** Every spawned child's parent is the root task. The
  checker enforces it — "workers must be nonrecursive task-free leaves",
  `task_execution.rs` — and `AdmittedCausalOrder::derive` re-checks it against
  the reference observation's own task table rather than assuming it.
* **The observed trace is a rendering, not a schedule.** `relate` says the
  rendering is *consistent with* the admitted causal order. It does not say the
  native execution took that order. No shared trace clock exists across the C11
  threads this backend uses, and ThreadSanitizer does not intercept
  `thrd_create` (measured in T04), so no measurement of the true interleaving is
  offered here or claimed.

### 3.2 Fail-closed behaviour

An observation that violates a constraint is refused with the constraint it
violated. It is never normalised, reordered or repaired. `relate` has no
fallback branch, no "closest match" and no tolerance parameter, and
`AdmittedCausalOrder` has no public constructor — a candidate reference order
must pass R1, R2's distinctness and R4–R9 before it can admit anything, which is
what stops a leaked or unjoined run from becoming a vacuous constraint set. Two
compile-fail doctests pin the constructor boundary.

## 4. The declared finite domains

A **schedule** of a run is any ordering of its facts that preserves each actor's
own program order — that is, any rendering a different observer of the same
execution could legitimately have produced.

| Domain | Contents | Size | Checked by |
|---|---:|---:|---|
| D1 | every schedule of the round trip | 7 | Lean `decide` **and** the Rust harness |
| D2 | every schedule of the concurrent round trip | 55 | Lean `decide` **and** the Rust harness |
| D3 | every single transposition of every D2 schedule | 3025 | Lean `decide` **and** the Rust harness |
| E1 | every one of the `7! = 5040` orderings of the round trip's facts, schedule-preserving or not | 5040 | the Rust harness only |

D3 exists because D1 and D2 cannot contain class 4 (an actor's own facts out of
order) by construction; perturbing an admitted schedule reintroduces it and
everything downstream of it. E1 is the Rust-only extension, in F01's sense: it
measures refusal against an arbitrary observer rather than a plausible one.

## 5. Results

### 5.1 The round trip has one schedule

Of the 7 schedules of the round trip — and of all 5040 orderings of its facts —
the relation admits exactly **one**, the reference order itself.

That is a measurement about *this program*, not a property of the relation: with
no parent fact between creation and join, the child's single completion fact has
exactly one admissible position. It is also the explanation for why byte-equal
stdout was an adequate check for CT01 (`w6-owned-source-tasks`,
`w6-completed-child-custody`) — and the reason a second program was needed.

| Domain | admitted | 4 | 7 | 10 | 14 | 17 |
|---|---:|---:|---:|---:|---:|---:|
| D1 (7) | 1 | — | 1 | 2 | 1 | 2 |
| E1 (5040) | 1 | 5033 | 1 | 2 | 1 | 2 |

Classes: 4 actor order, 7 allocation order, 10 release order, 14 creation order,
17 completion order.

### 5.2 The concurrent round trip has six

Of the 55 schedules of the concurrent round trip the relation admits **six**.
The shipped backend renders exactly one of them. Byte equality against the
reference would refuse the other five; the relation admits all six, which is the
card's first acceptance criterion discharged by measurement rather than by
assertion.

| Domain | admitted | 4 | 7 | 10 | 14 | 17 |
|---|---:|---:|---:|---:|---:|---:|
| D2 (55) | 6 | — | 10 | 17 | 13 | 9 |
| D3 (3025) | 18 | 2845 | 27 | 55 | 48 | 32 |

D3's 18 admitted perturbations are transpositions that carry one admitted
schedule onto another, so the admitted set is genuinely a partial order and not
the reference total order under a new name. Its 2845 class-4 refusals are the
per-actor conjunct doing the work D1 and D2 cannot exercise.

`admitted_linearizations`, the bounded enumerator, independently reproduces
exactly D2's six admitted schedules from the constraints alone, and returns
`None` rather than an answer of unknown cost when its search bound is exceeded.

### 5.3 The native runs relate

Both programs were compiled and executed; the generated program's stdout was
read by the observer and related to the reference run's admitted causal order.
Both are admitted. Today both render the reference order, because the backend
buffers child facts until join — recorded as a measurement about this backend,
not as a requirement.

## 6. Tested versus proved — the precise split

### 6.1 Proved in Lean (kernel-checked, no new axiom)

Twenty-six theorems in `QCore.TaskTraceRelation`, every one of them registered
in `AuditTaskTraceRelation.lean`. Measured axiom closure: `propext` on
twenty-three, nothing at all on three, and `Quot.sound` additionally on
`admitted_traces_report_the_same_facts` — inside the `AuditAll.lean` allowance
(`propext`, `Classical.choice`, `Quot.sound`), and `Classical.choice` is not
used. No `sorry`, no placeholder, no existing model changed.

Six are **general**: they quantify over arbitrary reference and observed traces,
not over a domain.

| Theorem | Statement |
|---|---|
| `unattributable_is_refused` | a trace with an unattributable fact is refused, class 1 |
| `duplicate_occurrences_are_refused` | an attributable trace with indistinguishable occurrences is refused, class 2 |
| `admitted_traces_are_attributable` | an admitted trace passed R1 |
| `admitted_traces_have_distinct_occurrences` | ... and R2's distinctness |
| `admitted_traces_report_the_same_facts` | ... and reports as many facts as the reference |
| `admitted_traces_satisfy_the_order_constraints` | ... and satisfies R4–R9 |

The remaining twenty are kernel evaluations over the declared domains: the two
reference runs are admitted causal orders; each domain's size and trace digest;
each domain's **verdict digest**, which folds the classification of every trace
in domain order; the admitted counts (1, 6, 18); that D1's single admitted trace
is the reference order; and that D2 admits the reference rendering and five
schedules that are not it. Three further non-vacuity statements — that every
domain refuses part of itself — are derived from the counts and sizes already
evaluated rather than costing a second traversal.

### 6.2 Checked by cross-evaluation (not premised, not proved)

The Rust relation is *production* code and the Lean side is its transcription.
That transcription is not asserted:

* **Domain agreement.** Both sides enumerate D1, D2 and D3 independently and
  both pin the same trace digests.
* **Classification agreement.** Both sides pin the same verdict digest for each
  domain. A verdict digest is a fold of the violation *class codes themselves*
  in domain order, so a transcription that classified a single trace of a single
  domain differently would fail here rather than be assumed away. All six
  digests matched on the first run; nothing was adjusted to make them agree.

This is cross-evaluation, not mechanised extraction. §8 names the residual.

### 6.3 Tested in Rust (executable, exhaustive over the declared domains)

Nineteen tests in `crates/qcore-core/tests/task_trace_relation.rs`, four in
`compiler/tests/f02_trace_relation.rs`.

| Test | What it establishes |
|---|---|
| `both_reference_orders_relate_to_themselves` | reflexivity, with the identity linearization |
| `the_declared_domains_have_their_pinned_shape_and_digest` | domain agreement (§6.2), plus E1 |
| `the_round_trip_admits_exactly_one_total_order` | §5.1 on D1, with its class census |
| `no_ordering_of_the_round_trip_outside_its_one_schedule_is_admitted` | §5.1 on E1 (Rust-only) |
| `the_concurrent_round_trip_admits_six_distinct_total_orders` | §5.2 on D2 |
| `the_bounded_enumerator_finds_the_same_six_schedules` | the enumerator agrees with the domain, and honours its bound |
| `single_transpositions_of_admitted_schedules_are_censused` | §5.2 on D3 |
| `the_admitted_order_reports_its_own_edges` | 16 and 31 admitted causal edges, pinned |
| nine named refusal tests | one per constraint family, each with its exact violation value |
| two observer tests | the alphabet round-trips; a partially readable line is refused, not skipped |
| `the_declared_trace_fixtures_are_the_reference_runs` | the domain fixtures are the real reference runs, not hand-written approximations |
| `the_native_rendering_relates_to_the_admitted_causal_order` | §5.3, with the generated-C digest pinned |
| `a_causally_impossible_native_rendering_would_be_refused` | fail-closed, on a real native trace |
| `the_native_evidence_binds_its_whole_identity` | §7 |

The refusal tests are split deliberately between `relate` and `declare`: a
mutation *within one actor's* facts is caught by R3 before any resource conjunct
can fire, so the resource conjuncts are exercised where they are actually
decided — on a candidate reference order.

## 7. The bound native identity

Acceptance 2. Every native record binds, in one place
(`docs/reports/remaining-f02/native-runs.json`, produced by the test itself):

| Field | Round trip | Concurrent |
|---|---|---|
| source digest | `1cc29437772ab764` | `051dfc88eb3ef08f` |
| generated C digest | `7d39f7c9157db610` | `3cb6a22168b481b3` |
| facts / admitted edges | 7 / 16 | 11 / 31 |
| reference positions | `[0..6]` | `[0..10]` |

Shared by both records: compiler `Ubuntu clang version 21.1.8 (6ubuntu1)`,
target `x86_64-pc-linux-gnu`, flags `-std=c11 -pthread -Wall -Wextra -Werror -g
-O1`, runtime `c11-threads/qcore_task_start+qcore_task_collect`, observer
`qcore_core::task_trace_relation::parse_observed_trace`, relation
`qcore.task-trace-relation.v1`.

`native-runs.json` also records an **executable digest** per run. It is *not*
reproducible and is deliberately not pinned: `-g` bakes the temporary build
directory into the debug info, so the same generated C yields a different
executable digest on every run. It identifies the binary that produced that
record and nothing more. The generated C digest is the reproducible identity,
and it is the one the test pins.

Following G01's discipline, the generated-C digests are **pinned in the test**.
A change to emission retires this record rather than silently reinterpreting it;
updating those two constants is a deliberate act that must be recorded here. The
relation carries its own identity constant for the same reason: a changed
relation must not be read as having admitted what the old one admitted.

### 7.1 Fixed sanitizer runs

Both programs, built from the same generated C with
`-fsanitize=address,undefined -fno-omit-frame-pointer` added to the flag vector
above and run with `ASAN_OPTIONS=detect_leaks=1
UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`:

| Program | generated C sha256 | exit | sanitizer output | stdout |
|---|---|---:|---|---|
| round trip | `f7061e7599901c6a…` | 0 | none | the 7 facts, admitted |
| concurrent | `44599187b5c0aa43…` | 0 | none | the 11 facts, admitted |

Full digests, the exact flag vector, the environment and the reproduction
commands are in `sanitizer-runs.json`.

No TSan run is offered: ThreadSanitizer does not intercept C11 `thrd_create`
(measured in T04, `docs/reports/remaining-t04/README.md`), so the concurrent
program's freedom from data races rests on the checker's rules and on this
profile having no shared mutable state, not on a measurement.

## 8. Premises — what remains asserted

1. **Transcription.** `QCore.TaskTraceRelation.relate` is a hand transcription
   of the Rust production relation. §6.2 checks it by whole-domain verdict
   digest rather than by inspection, but no mechanised extraction exists in this
   project, so *outside* the enumerated domains the transcription is asserted.
2. **Digest injectivity.** Domain and classification agreement are equalities of
   64-bit FNV-style folds. `encodeEvent` is injective for the identifiers the
   domains use (tasks below 8, generations below 64, non-negative payloads); the
   folds are not claimed collision-resistant against an adversary, only against
   accident.
3. **P1, P2, P3** of §3.1.
4. **Domain choice.** D1–D3 are *defined* to be all schedules of two runs and
   all single transpositions of one of them. That these two runs are the right
   ones to relate is a judgement, evidenced by the non-vacuity counts, not a
   theorem.
5. **One rendering per program.** §5.3 relates the rendering the shipped backend
   produced on this host, on these two runs. It is not a claim about all runs of
   these programs, all hosts, or any failure schedule.
6. **Everything under the checked profile.** The reference run, the checked task
   module and the C11 backend are the ones the existing W6 records describe, with
   their own premises unchanged.

## 9. Out of scope, and explicitly not claimed

* **Native trace replay is not general C refinement.** Nothing here says the
  generated C refines the reference interpreter, for these programs or any
  others. It says one rendering of one run is consistent with one admitted
  causal order.
* **No claim about the true native interleaving.** §3.1's fourth bullet.
* No arbitrary compiler optimisation level, architecture, allocator, failure
  schedule or full-language theorem. One compiler, one target, one flag vector,
  two runs, all recorded.
* No error-payload path (T06 is absent from this branch), no read loans in the
  measured domains — R8 is transcribed on both sides and exercised by neither
  domain, which is recorded as a non-vacuity gap in §11 — no shared memory, no
  mutexes, no nested workers, no non-Linux target.
* The relation refuses, rather than resolves, two indistinguishable occurrences.
  A program whose run reads the same generation twice from the same task with
  the same value cannot be related without occurrence indices. That is a real
  restriction on which runs this relation applies to, recorded in §11.
* **Nothing is promoted.** Raw Cycle 19 units remain 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units, CT coverage remains 15/50
  mapped, the contract corpus remains 59 modules, `qcore-schemas` remains 5.0.0
  and full language conformance remains false. The four pinned 185-unit
  artifacts are untouched by construction — no analysis code changed — and were
  not re-measured; CORPUS-REPIN remains open.

## 10. CT49 candidate — proposed, not mapped

The card names CT49 as a spec anchor. `docs/reevaluation/ct_manifest.json` is
**not** edited by this card, and CT coverage stays at 15/50. What follows is a
proposal for the integrator.

CT49 asks for observation of a concurrent execution that is compared by
admitted ordering rather than by a fixed schedule. The evidence this card can
offer for it is: parser, checker, reference and CFG stages on
`compiler/tests/fixtures/f02/concurrent_round_trip.qcore`, plus a Linux native
stage whose rendering is admitted by `relate` against the reference-derived
causal order, with the generated C, compiler, target, flags, runtime and
observer identity bound in `native-runs.json`, and with six admitted schedules
measured rather than one asserted. What it cannot offer, and what an integrator
should weigh before mapping: the native stage exercises **one** of those six
schedules, because the shipped backend renders child facts at join. A CT49
mapping that reads as "several schedules were observed natively" would
overstate this evidence; a mapping that reads as "the observed native rendering
was admitted by a partial order that admits six" is what the measurements
support.

Suggested AGENTS.md ledger sentence, for the integrator to place or discard:

> Source-to-native trace relation (F02): one admitted owned-Buffer
> worker/capsule/join path now has a scoped relation between its reference run
> and an observed native rendering, compared by admitted causal constraints
> rather than by an identical total schedule. Nine conjuncts cover identity,
> cleanup and the partial order; twenty violation classes are fail-closed. Three
> declared domains (7, 55 and 3025 traces) are classified identically by the
> Rust production relation and a Lean transcription, pinned by trace and verdict
> digests; the concurrent run has six admitted schedules and the shipped backend
> renders one of them. Generated C, compiler, target, flags, runtime and
> observer identity are bound into the record, with two clean fixed
> ASan/UBSan/LSan runs. Twenty-two Lean theorems, no new axiom. This is not
> general C refinement and no raw unit is promoted; CT coverage stays 15/50 with
> a CT49 candidate proposed in the report only.

## 11. Named follow-ups

* **F02-B1.** The relation is established for two runs of two programs. It is
  not an inductive statement over the profile, and there is no theorem relating
  *every* run of a checked task module to its native rendering.
* **F02-B2.** The transcription premise (§8.1) is the weakest link, exactly as
  in F01. Generating the Lean side from the Rust source, or both from one
  description, would close it.
* **F02-B3.** R8 (loans) is transcribed on both sides and exercised by neither
  declared domain, because the T04 read-loan program's parent reads the same
  generation twice and is therefore refused for ambiguous occurrences. Adding
  occurrence indices to the alphabet would let the loan path be related and
  would close both this gap and the restriction in §9.
* **F02-B4.** The shipped backend renders exactly one of the six admitted
  schedules, so the *other five* are admitted by construction and by domain
  enumeration, never by observation. A second native observer — one that emits
  a child's facts as they happen rather than at join — would turn five of them
  into measurements. That is a backend change and is deliberately not made here.
* **F02-B5.** The class-level census is the granularity at which Rust and Lean
  agree: the verdict digest folds violation *class codes*, not the identifiers
  the Rust violations carry. Two refusals of the same class with different
  buffers agree in Lean by construction.
* **F02-B6.** `CreationFailed` is the one alphabet line that names no task, so
  every trace containing one is refused as unattributable. Failure-arm runs of
  this profile therefore cannot be related at all. Adding the creator's identity
  to that line is a backend change; it is recorded, not made.

## 12. Changes

| File | Change |
|---|---|
| `crates/qcore-core/src/task_trace_relation.rs` | new, the relation |
| `crates/qcore-core/src/lib.rs` | +1 module line |
| `crates/qcore-core/tests/task_trace_relation.rs` | new, 19 tests |
| `crates/qcore-core/tests/fixtures/f02/round_trip.trace` | new |
| `crates/qcore-core/tests/fixtures/f02/concurrent_round_trip.trace` | new |
| `compiler/tests/f02_trace_relation.rs` | new, 4 tests |
| `compiler/tests/fixtures/f02/concurrent_round_trip.qcore` | new fixture |
| `proof/QCore/TaskTraceRelation.lean` | new, 26 theorems |
| `proof/AuditTaskTraceRelation.lean` | new |
| `proof/QCore.lean` | +1 import |
| `docs/reports/remaining-f02/` | this record, `native-runs.json`, `sanitizer-runs.json` |

`AGENTS.md`, `CLAUDE.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as the card requires.
No existing production file changed except the one module line.

## 13. Gate results

| Gate | Result |
|---|---|
| `cargo test --workspace` | exit 0 — **1165 passed**, 0 failed, 1 ignored (1141 at baseline; +19 core tests, +4 compiler tests, +1 compile-fail doctest) |
| `cargo fmt --all -- --check` | exit 0, clean |
| `cargo clippy --all-targets -- -D warnings` | exit 0, clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B -m unittest discover -s scripts/tests` | exit 0 — 43 passed |
| `cd proof && lake build` | exit 0 — 68 jobs (was 67) |
| `python3 -B scripts/check_lean_audits.py` | exit 0 — **24 audits** (was 23) |
| `cargo test --manifest-path providers/Cargo.toml` | exit 0 — 56 passed |

Native runs used `clang` 21.1.8 on `x86_64-pc-linux-gnu`;
`git submodule update --init --recursive` was run once in this worktree, as
AGENTS.md documents.

### 13.1 Cost, recorded rather than hidden

`QCore.TaskTraceRelation` takes **336 s** to elaborate and peaks near **13 GB**
of resident memory, which is far more than any other module in `proof/`. The
cost is D3: 3025 traces of eleven facts, each classified by a transcription of a
twenty-class relation, in four kernel traversals. Two decisions follow from it
and are deliberate:

* The 5040-ordering domain (E1) is **Rust-only**. An earlier version put it in
  Lean as well and reached 17 GB without finishing.
* The per-class census is **Rust-only**. An earlier version folded a
  21-element list accumulator through the domain in Lean; because the kernel
  substitutes a `let` rather than sharing it, the accumulator term grew
  without bound. The verdict digest — a `Nat` fold of the class codes — pins
  strictly more (the class of every trace in order, not just the totals) at
  linear cost, and the readable census is decoded on the Rust side.

Anyone raising a domain bound here should re-measure before committing: this
module is the workspace's Lean cost outlier.
