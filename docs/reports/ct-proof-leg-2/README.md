# CT37 proof leg — the second executed proof row's capability, on the CT01 precedent (CT-PROOF-LEG-2)

Wave 30, card `CT-PROOF-LEG-2`. Wave 29 landed the manifest's first executed
`proof` row (CT01, card `CT01-PROOF-LEG`, integrator-flipped at the wave-29
close). This card delivers the second, by the same pattern and to the same
standard: a Lean theorem package about **CT37's own artifact** — the reference
run of `compiler/tests/fixtures/ct/CT37/case.qcore` through the real
pipeline — plus the named, host-unconditional workspace test that binds the
theorems to that artifact by running the same total decision procedure the
Lean definition defines.

**The manifest, the acceptance matrix, `acceptance.json`, `ct_corpus.rs` and
every EVIDENCE_FILES list are untouched.** CT37's `proof` row still reads
`unassessed` and proof still reads **1 of 42** — deliberately, per the card
brief. The integrator flips the row with the exact terms in §7; until then the
honest statement is *the capability exists and is gate-checked, the stage
evidence is proposed*.

Base: `main` at `30aee0b2` (the wave-29 close).

**Nothing semantic changed.** `git diff` over `crates/` and `compiler/src/` is
empty. No diagnostic code is allocated, no word is reserved, no route changes,
no pinned expectation moves. The additions are one Lean module, one Lean
audit, one workspace test file, one import line in `proof/QCore.lean`, this
report and one `DEVELOPMENT.md` increment paragraph.

---

## 1. Case selection: the survey, the choice, the runners-up

The brief asks for the mapped **Accept** case whose proof leg is *cheapest and
most honest* on the CT01 pattern — a case whose route records a
reference-trace relation the Lean core already models. The selection was
**measured, not assumed**: every plausible candidate's fixture was compiled
through `compile_files`, executed by `execute_task_module` under the default
script, and handed to the production observer `AdmittedCausalOrder::derive`
(the probe's derived traces are reproduced below where they decide the
choice; the probe itself was a temporary test file, deleted after the survey).

**Chosen: CT37** — scenario *Child returns allocation and finishes early*,
expected *Allocation remains live in completion capsule*, route `task`,
verdict mode `native_task_scenarios`. Its default reference run derives
cleanly into the F02 alphabet, and — decisively — **the default run itself
exhibits the scenario**, with no scripted failure arm:

```
Spawned   { child: 1, worker: 0 }
Allocated { task: 1, buffer: 4294967297, payload: 42 }   (the child's own allocation)
Completed { task: 1, outcome: Buffer(4294967297) }        (finishes early, capsule loaded)
Joined    { child: 1 }
Read      { task: 0, buffer: 4294967297, value: 42 }      (the capsule held it live)
Dropped   { task: 0, buffer: 4294967297 }
Completed { task: 0, outcome: I32(42) }
```

The worker allocates the Buffer in its own body (the worker-local generation
`4294967297`), returns it inside its completion record, and completes before
the parent joins; the parent joins, reads the allocation's own payload back,
releases exactly once. That is the manifest row's semantic profile sentence —
"Parent join transfers generation 4294967297, which must be consumed and
dropped" — verbatim in trace form. One genuinely new proof obligation
distinguishes it from CT01: the scenario's defining fact (*the child's
completion precedes the parent's join*) **crosses actors**, so it cannot be
read off any single actor's projection; the closure theorem extracts it from
the relation's own order admission (§2).

**Runners-up, each rejected on a measured ground:**

* **CT03** (the only other literal-"Accept" task case, loans in the
  alphabet): its reference execution contains `BufferWritten` events
  (`qcore_buffer_write` at two sites), which the F02 alphabet deliberately
  refuses to encode — measured: `derive` refuses with
  `Unsupported(UnsupportedObservedEvent { index: 1 })`. No admitted rendering
  exists to prove anything about without extending the pinned alphabet, which
  would touch `TaskTraceRelation` and every digest downstream.
* **CT12, CT14**: measured — their default-script reference traces are
  **event-for-event CT01's `roundTrip`**. Their distinguishing scenarios live
  only in scripted failure arms, and the failure events (`CreationFailed`) are
  unattributable in the relation (`actorOf = none`), so no declared order can
  contain them. A proof on their default trace would restate CT01's theorem
  under a second name — the dishonest shape the brief warns against.
* **CT13, CT15**: the scenario-defining runs need scripted create/join
  failures; `CreationFailed` is unattributable and a join failure has no
  event in the alphabet at all.
* **CT16, CT20**: their scenario paths carry typed error outcomes; the Lean
  `Outcome` deliberately omits errors ("the Rust digest refuses exactly the
  same cases" — `TaskTraceRelation.lean`'s own alphabet note), and CT16's
  trace carries `Error(..)` outcomes even on the default run. CT20's default
  10-event trace is in-alphabet but is the *no-error* path — the handler and
  the rethrow the scenario names never run in it.
* **CT17**: the scenario is the error-payload transfer; `ErrorWithBuffer`
  refuses `reference_trace`, and the default run is a success path that does
  not exhibit the scenario.
* **CT22, CT26, CT35, CT36**: epoch-credit, mutex and epoch-figure verdict
  modes — no reference-trace relation in the Lean core models their evidence.
* **CT49**: has a Lean sibling (`QCore.ObsReordering`), but that module is a
  general model of the stream comparator with **no digest-pinned transcription
  of CT49's own recorded run** — the bridge CT01's pattern presupposes would
  have to be built first, from native byte streams rather than a checked
  artifact term. Much more expensive, different pattern.
* **CT50**: viable and strictly cheapest — its 6-event trace is single-actor,
  so the closure collapses to strict identity — but thinner: the scenario's
  crux (the numeric address reuse under `MAP_FIXED_NOREPLACE`) is invisible to
  the reference alphabet, so the provable statement (two disjoint generation
  lifetimes) leaves the case's distinguishing risk entirely to native
  observation. CT37's scenario is fully expressed by its trace and relation;
  CT50's is not. Honesty outweighed the cost difference.

## 2. The Lean side — `proof/QCore/Ct37CompletionCapsule.lean`

New module (audit `proof/AuditCt37CompletionCapsule.lean`, Lean audits 118 →
**119**), importing `QCore.Ct01OwnedTransfer` (for the registered
`ReflBEq`/`LawfulBEq` instances on the F02 alphabet — nothing else is reused;
CT01's helpers are private) and adding:

* **`referenceRun`** — the transcription of CT37's reference trace (the seven
  events above), proved a declared admitted causal order
  (`ct37_reference_run_is_an_admitted_causal_order`, `declare referenceRun =
  0` by kernel `decide`). Unlike CT01, whose trace term F02 had already
  recorded, this term is new — the named Rust runner is what binds it to the
  artifact, and `ct37_trace_digest : mixTrace referenceRun =
  17239871955803798867` is the transcription pin the runner recomputes.
* **`dischargesCompletionCapsule : List Event → Bool`** — a total decision
  procedure for the scenario property: seven facts; the parent's projection is
  exactly `[creation, collection, readBack, release, rootResult]` (spawn,
  join, read, drop, complete — the parent allocates nothing); the child's
  projection is exactly `[allocation, earlyReturn]` (allocate, finish carrying
  the allocation — the child neither reads nor releases); exactly one
  allocation and one release of the moved generation, exactly one child
  completion and one join; and **`returnPrecedesTheJoin`** — the first
  completion of the child sits strictly before the first join, stated
  positionally because no single actor's projection can carry it.
* **Meaning theorems**, each for an *arbitrary* trace the procedure accepts:
  * `discharged_parent_facts_are_exactly_the_scenario` and
    `discharged_child_facts_are_exactly_the_scenario` — the two exact
    program orders;
  * `discharged_release_is_unique` — the moved generation is released exactly
    once and the one release is the parent's fact (`.dropped 0 4294967297`);
  * `discharged_join_precedes_the_release` and
    `discharged_read_back_sits_between_join_and_release` — the capsule's
    content is unreclaimed until the parent has joined and read the
    allocation's own payload back;
  * `discharged_return_preserves_the_generation` — the child's only
    completion carries `.buffer 4294967297`, the very generation the child's
    only allocation created (`bufferOf earlyReturn = bufferOf allocation`);
  * `discharged_return_precedes_the_join` — the early finish as a subtrace:
    `[earlyReturn, collection].Sublist t`, derived from the positional clause
    through new general lemmas (`positions_length`, `positions_spec`,
    `eq_of_countP_eq_one`, `sublist_pair_of_getElem?`).
* **`ct37_reference_run_discharges_the_completion_capsule`** — the reference
  trace is accepted (`decide`, kernel-checked).
* **`every_admitted_rendering_discharges_the_completion_capsule`** — the
  closure: for **arbitrary** traces, `relate referenceRun observed = 0 →
  dischargesCompletionCapsule observed = true`. The per-actor clauses and the
  counts follow CT01's closure verbatim (attributability, distinctness,
  multiset agreement, per-actor projection agreement, unique-witness counting).
  The cross-actor clause is the new work: `checkOrder`'s
  completion-before-join conjunct is extracted from the relation's own order
  admission. To address that conjunct **without editing
  `TaskTraceRelation`**, the module restates `checkOrder`'s sixteen-flag list
  definitionally — `checkOrder events = firstFailure (orderChecks events)` is
  proved by `rfl`, the kernel's own zeta/beta reduction witnessing that
  nothing changed — and then proves that an admitted rendering's unique child
  completion sits at or before its unique join (`firstFailure_ne_zero`,
  `mem_sortNats`, flag extraction), with strictness from the fact that no
  event is both a completion and a join.
* **Six named refusals** — a join moved before the return (every projection
  and count intact: only the positional clause refuses it, so the new clause
  is load-bearing), a missing release (the leak shape), a child that releases
  instead of returning (the emptied capsule), a double release, a
  generation-changing return, and a capsule that loses the payload (the
  parent reads back a value the allocation never carried).

Axiom surface: every theorem's closure is within `propext`,
`Classical.choice`, `Quot.sound` (the `AuditAll` allow-list); the audit prints
each, and the three `decide` reference theorems plus the digest pin are
`propext`-only or axiom-free. No `native_decide`, no placeholder, no project
axiom. `TaskTraceRelation.lean` and `Ct01OwnedTransfer.lean` are untouched.

## 3. The Rust side — `compiler/tests/ct37_proof_leg.rs`

The proposed executor,
**`compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape`**,
is host-unconditional (no `cfg`, no Clang, no `host_gate`): the artifact is
the checked task module's reference execution, produced in-process. It:

1. compiles CT37's own fixture through `compile_files`, executes
   `execute_task_module` on the checked module, derives the admitted causal
   order with the production observer, and asserts the events equal the trace
   the Lean `referenceRun` term transcribes, event for event;
2. recomputes `mixTrace` over the artifact's events with wrapping `u64`
   arithmetic and asserts it equals the Lean-proved `ct37_trace_digest`
   constant — the artifact and the Lean term cannot drift apart silently;
3. runs `discharges_completion_capsule`, the clause-for-clause transcription
   of the Lean decision procedure (including `return_precedes_the_join`, the
   positional cross-actor clause), on the artifact's events and requires
   acceptance;
4. asserts the artifact's rendering is admitted by the production `relate`
   against its own declared order, so the Lean closure theorem covers it.

A second test, `the_procedure_refuses_the_lean_named_mutations`, re-runs the
transcription on the same six mutants the Lean module proves refused and
requires refusal — the transcription is checked on both sides of the boundary.

One deliberate difference from CT01's runner: CT01 compared the artifact
against a recorded F02 `.trace` fixture file; CT37 has no such recorded file,
so the runner compares against the named-fact event list that mirrors the Lean
term constructor for constructor, and the kernel-proved digest pin is the
guard against a transcription drifting on either side. The binding is the
same: artifact ↔ named events ↔ digest ↔ Lean term.

## 4. Exactly what the theorem does and does not cover

**Covered.** The property is proved of CT37's *checked artifact*: the
reference execution's admitted causal order, and every rendering the F02
relation admits against it. Concretely: the child's whole program order is
allocate-then-finish, carrying the very generation its only allocation
created; the child's completion precedes the parent's join in every admitted
rendering; the moved generation is released exactly once, by the parent, after
the join and after the allocation's own payload is read back out of the
completion capsule — for the reference trace by kernel `decide`, and for
arbitrary admitted renderings by the symbolic closure theorem with the
cross-actor conjunct extracted from `checkOrder`.

**Not covered — stated so the row cannot be over-read.**

* **No native refinement is claimed.** The emitted C's native execution is
  covered only by the existing `native_task_scenarios` observations (which
  force worker-body completion before creation returns and byte-compare the
  streams, per the manifest's semantic profile). The theorem is about the
  reference execution and the admitted-rendering relation, not about what the
  binary did. In particular the profile's own "native refinement remains
  open" sentence stands.
* **The Lean–Rust tie is transcription agreement, not verified extraction.**
  It is the F02 pattern: a kernel-proved digest recomputed from the artifact,
  plus event-for-event equality against the named transcription, plus
  mutant-for-mutant agreement on the refusing side.
* **Nothing outside this profile.** The theorems quantify over F02-alphabet
  traces; they say nothing about other programs, other cases, the checker, or
  general memory-peak accounting (the profile's other named open items).

## 5. Measured results

| Check | Result |
|---|---|
| `proof/QCore/Ct37CompletionCapsule.lean` under `-DwarningAsError=true` | clean |
| `AuditCt37CompletionCapsule` axiom surface | `propext` / `Classical.choice` / `Quot.sound` only; the digest pin and both projections axiom-free |
| `ct37_artifact_is_the_lean_proved_completion_capsule_shape` | green: artifact = `referenceRun` transcription, digest = `17239871955803798867`, procedure accepts, relation admits |
| `the_procedure_refuses_the_lean_named_mutations` | green: all six mutants refused |

Raw gate logs: worktree-private `measure/`. The full gate table is in §8.

## 6. New general Lean lemmas, and why they are private

The closure needed six small general facts the core modules did not export:
`positionsFrom_length`/`positions_length` (a position list's length is the
predicate's count), `positionsFrom_spec`/`positions_spec` (a listed index
holds a matching event), `eq_of_countP_eq_one` (a predicate counted once names
one event), `sublist_pair_of_getElem?` (two indexed events in order form a
two-element subtrace), `mem_insertNat_of`/`mem_foldl_insertNat`/`mem_sortNats`
(membership survives the relation's insertion sort), and
`firstFailure_ne_zero` (a true flag with nonzero index defeats
`firstFailure`). All are `private` in the new module, on CT01's own
convention (its `countP` helpers are private too): they are proof
infrastructure for this discharge, not published surface, and a future card
that wants them shared should promote them into `TaskTraceRelation`
deliberately rather than inherit them by accident.

## 7. Proposed proof-stage evidence (the integrator's flip)

On the exact terms of CT01's wave-29 flip, which the integrator executes; this
card edits none of these files.

**(a) The manifest record** — CT37's `proof` row in
`docs/reevaluation/ct_manifest.json`:

```json
"proof": {
  "status": "executed",
  "executor": "compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape",
  "artifact": "compiler/tests/fixtures/ct/CT37/case.qcore",
  "observations": 1
}
```

No `host_gate` (the runner is host-unconditional) and no `tool_gate` (no
external tool; the Lean theorems are checked by the standing
`check_lean_audits.py` gate, not by this runner).

**(b) The `PROOF_GATED_CASES` row** in `compiler/tests/ct_corpus.rs` — the
pinned fail-closed table grows by exactly one entry:

```rust
(
    "CT37",
    "compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape",
),
```

Every case outside the table keeps `unassessed`; the existing `derive_stage_plan`
proof arm needs no edit — it already reads the table.

**(c) The stage-evidence record** — a new
`docs/reports/ct-proof-leg-2/ct_stage_evidence.json` on the exact shape of
`docs/reports/ct01-proof-leg/ct_stage_evidence.json`, written by the
integrator at the flip: `mapped_case_stage_matrix.CT37` with
`"supersedes": "docs/reports/remaining-t00/coverage.json"` — **the record
currently holding CT37's stages** (verified: CT37's `verified_stages` live in
card T00's frozen archive, which is superseded from the new record and never
edited, constraint 21) — every non-proof stage row carried from the T00 record
unchanged, and the `proof` row:

```json
"proof": {
  "status": "executed",
  "executor": "compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape",
  "artifact": "compiler/tests/fixtures/ct/CT37/case.qcore",
  "observations": 1,
  "note": "the artifact checked against the Lean-proved total decision procedure dischargesCompletionCapsule (QCore.Ct37CompletionCapsule, AuditCt37CompletionCapsule.lean); reference-run discharge and admitted renderings covered, including the cross-actor early-finish conjunct extracted from the relation's own order admission; native execution deliberately not (observation only)"
}
```

plus the matrix regeneration through the standing generator with hand-kept
sections carried (constraint 25) and the headline `proof 1 of 42 → 2 of 42`
wherever the close reconciles it (`AGENTS.md` ledger).

Suggested reading of the flipped row, for the record: *proof — the scenario
property proved of the checked artifact's reference run and its admitted
renderings; native execution unproved, covered by observation only.* A grown
proof numerator is evidence about this case's artifact, not language
conformance (constraint 23's spirit applies to this denominator too).

## 8. Gate

Run at the card tip on Linux, after the same disk-bound configuration the
CT01 card recorded for this worktree host
(`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`,
debug-info emission off; no test, pin or assertion edited to obtain the
results).

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,447 passed / 0 failed / 8 ignored** (exit 0; wave-29 baseline 4,445 + this card's 2). Constraint-16 rows: see the disclosure under the table |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | Passed 119 Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 192 of 193 green; the one expected failure is §8.1 |

### 8.1 The one expected failure

`scripts/tests`'
`test_the_derived_facts_are_recomputed_rather_than_carried` trips on
`validation.lean_audits`, which moves 118 → **119**. This card's brief forbids
editing `docs/reviews/r0_3/acceptance_matrix.json`, so the derived row is
deliberately left stale for the wave integrator to regenerate — the handling
every audit-adding card's delta receives (the `CT01-PROOF-LEG`,
`AGGREGATES-V1` and `LOOP-V1` reports record the same shape). The delta is
entirely this card's one audit file.

## 9. Follow-ons

* `CT37-PROOF-ROW` — the integrator flip of §7.
* `CT-PROOF-NATIVE-REFINEMENT` — unchanged; this card neither closes nor
  narrows the manifest profile's "native refinement remains open".
* `CT-PROOF-THIRD-CASE` — the pattern now has two instances and one measured
  boundary: within the F02 alphabet, CT50 is the only remaining mapped case
  whose default reference run both derives and exhibits its scenario
  (single-actor, so its closure would be the strict-identity degenerate case,
  and its address-reuse crux stays native-only — the honesty caveat in §1
  applies and should be weighed by whoever takes it). Every other mapped task
  case needs either an alphabet extension (writes, error outcomes — a
  deliberate `TaskTraceRelation` change with every digest re-pinned) or a
  scripted-run trace term whose failure events the relation cannot attribute
  today.
