# CT01 proof leg — the first proof-stage capability for a mapped CT case (CT01-PROOF-LEG)

Wave 29, card `CT01-PROOF-LEG`. The manifest's `proof` denominator has read
**0 since the manifest existed**. This card delivers the capability to move it
by one, honestly: a Lean theorem package about **CT01's own artifact** — the
reference run of `compiler/tests/fixtures/ct/CT01/case.qcore` through the real
pipeline — plus the named, host-unconditional workspace test that binds the
theorems to that artifact by running the same total decision procedure the
Lean definition defines.

**The manifest and the acceptance matrix are untouched.** CT01's `proof` row
still reads `unassessed` and proof still reads **0 of 41** — deliberately, per
the card brief, to avoid conflicting with a parallel mapping lane. The
integrator flips the row with the exact terms in §6; until then the honest
statement is *the capability exists and is gate-checked, the stage evidence is
proposed*.

Base: `main` at `5d8b2ff1` (the wave-28 close).

**Nothing semantic changed.** `git diff` over `crates/` and `compiler/src/` is
empty. No diagnostic code is allocated, no word is reserved, no route changes.
The additions are one Lean module, one Lean audit, one workspace test file,
one import line in `proof/QCore.lean`, and this report.

---

## 1. What already existed, and what was missing

Card F02 built the whole bridge except the theorem:

* `QCore.TaskTraceRelation.roundTrip` is declared as *CT01's owned round
  trip* — a transcription of the recorded trace
  `crates/qcore-core/tests/fixtures/f02/round_trip.trace`.
* `compiler/tests/f02_trace_relation.rs::the_declared_trace_fixtures_are_the_reference_runs`
  proves (Linux-gated) that this recorded trace **is** the reference run of
  CT01's own fixture through `compile_files` — event for event, via the
  production observer `AdmittedCausalOrder::derive`.
* The Lean/Rust transcription is held together by pinned digests over three
  enumerated domains, recomputed independently by the Lean kernel and by
  wrapping `u64` arithmetic.

What no card had done: prove anything about the **scenario's property** — the
manifest row's "move an owned buffer to child; child returns it; parent joins
and drops" — and connect that proof to the artifact through a runner
constraint 23 can accept ("a stage may read `executed` only where a named
workspace test runs it on the case's own artifact").

## 2. The Lean side — `proof/QCore/Ct01OwnedTransfer.lean`

New module (audit `proof/AuditCt01OwnedTransfer.lean`, Lean audits 115 →
**116**), importing `QCore.TaskTraceRelation` and adding:

* **`dischargesOwnedTransfer : List Event → Bool`** — a total decision
  procedure for the scenario property: seven facts; the parent's projection is
  exactly `[allocation, creation, collection, readBack, release, rootResult]`
  (allocate, spawn, join, read, drop, complete — in that order); the child's
  projection is exactly the generation-preserving return
  `.completed 1 (.buffer 1)`; and exactly one allocation of the buffer,
  exactly one release of the moved generation, exactly one child completion.
* **Meaning theorems**, each for an *arbitrary* trace the procedure accepts,
  so the procedure cannot drift from the property it is named after:
  * `discharged_parent_facts_are_exactly_the_scenario` — the parent's own
    program order is exactly the scenario's six facts;
  * `discharged_release_is_unique` — the moved generation is released
    exactly once (`countP (isReleaseOf 1) = 1`) and the one release is the
    parent's fact `release = .dropped 0 1`;
  * `discharged_join_precedes_the_release` — `[collection, release]` is a
    subtrace: the join happens before the drop;
  * `discharged_read_back_sits_between_join_and_release` — the payload
    read-back sits between them;
  * `discharged_child_return_preserves_the_generation` — the child's only
    completion carries `.buffer 1`, the very generation the parent's only
    allocation created (`bufferOf childReturn = bufferOf allocation`).
* **`ct01_reference_run_discharges_the_owned_transfer`** — CT01's reference
  trace is accepted (`decide`, kernel-checked).
* **`every_admitted_rendering_discharges_the_owned_transfer`** — the closure:
  for **arbitrary** traces, `relate roundTrip observed = 0 →
  dischargesOwnedTransfer observed = true`. Every rendering the F02 relation
  admits against CT01's reference run discharges the property — a symbolic
  proof over the relation's own conjuncts (attributability, distinctness,
  multiset agreement, per-actor projection agreement), not an enumeration.
  With F02's own `domain_one_admits_only_the_reference_order` this closes the
  property over the run's entire admitted rendering set.
* **Five named refusals** — a release before the join, a missing release
  (the leak shape), a release by the child, a double release, and a
  generation-changing return are each proved refused, so the procedure is not
  vacuous.
* **`ct01_trace_digest : mixTrace roundTrip = 13023433800416048247`** — the
  transcription pin the Rust runner recomputes from the artifact.

Two registered instances were added for the F02 alphabet:
`deriving instance ReflBEq, LawfulBEq for Outcome` / `for Event`. The derived
`BEq` was always structural equality; registering that fact is what lets the
relation's `==` conjuncts be read back as propositional equalities in the
closure proof. No definition in `TaskTraceRelation` moved; its file is
untouched.

Axiom surface: every theorem's closure is within `propext`,
`Classical.choice`, `Quot.sound` (the `AuditAll` allow-list); the audit prints
each. No `native_decide`, no placeholder, no project axiom.

## 3. The Rust side — `compiler/tests/ct01_proof_leg.rs`

The proposed executor,
**`compiler/tests/ct01_proof_leg.rs::ct01_artifact_is_the_lean_proved_owned_transfer_shape`**,
is host-unconditional (no `cfg`, no Clang, no `host_gate`): the artifact is
the checked task module's reference execution, produced in-process. It:

1. compiles CT01's own fixture through `compile_files`, executes
   `execute_task_module` on the checked module, derives the admitted causal
   order with the production observer, and asserts the events equal the
   recorded F02 fixture trace — the same event-for-event identity the
   Linux-gated F02 twin checks, here without the native half so the proof leg
   does not inherit a gate it does not need;
2. recomputes `mixTrace` over the artifact's events with wrapping `u64`
   arithmetic and asserts it equals the Lean-proved
   `ct01_trace_digest` constant — the artifact and the Lean term cannot drift
   apart silently;
3. runs `discharges_owned_transfer`, the clause-for-clause transcription of
   the Lean decision procedure, on the artifact's events and requires
   acceptance;
4. asserts the artifact's rendering is admitted by the production `relate`
   against its own declared order, so the Lean closure theorem covers it.

A second test,
`the_procedure_refuses_the_lean_named_mutations`, re-runs the transcription on
the same five mutants the Lean module proves refused and requires refusal —
the transcription is checked on both sides of the boundary.

## 4. Exactly what the theorem does and does not cover

**Covered.** The property is proved of CT01's *checked artifact*: the
reference execution's admitted causal order, and every rendering the F02
relation admits against it. Concretely: the buffer is dropped exactly once,
by the parent, after the join, after the moved payload is read back, and the
child's return is generation-preserving — for the reference trace by kernel
`decide`, and for arbitrary admitted renderings by the symbolic closure
theorem.

**Not covered — stated so the row cannot be over-read.**

* **No native refinement is claimed.** The emitted C's native execution is
  covered only by the existing observations: `run_native_owned_transfer`'s
  byte comparison and F02's relation checks (which themselves record that no
  shared trace clock exists across C11 threads). The theorem is about the
  reference execution and the admitted-rendering relation, not about what the
  binary did.
* **The Lean–Rust tie is transcription agreement, not verified extraction.**
  It is the F02 pattern: a kernel-proved digest recomputed from the artifact,
  plus event-for-event equality against the recorded fixture, plus
  mutant-for-mutant agreement on the refusing side. A transcription that
  drifted anywhere in the artifact's trace would be caught by the digest; it
  is not a proof about the Rust compiler.
* **Nothing outside this profile.** The theorems quantify over F02-alphabet
  traces; they say nothing about other programs, other cases, or the checker.

## 5. Measured results

| Check | Result |
|---|---|
| `proof/QCore/Ct01OwnedTransfer.lean` under `-DwarningAsError=true` | clean |
| `AuditCt01OwnedTransfer` axiom surface | `propext` / `Classical.choice` / `Quot.sound` only; three `decide` theorems axiom-free |
| `ct01_artifact_is_the_lean_proved_owned_transfer_shape` | green: artifact = fixture trace, digest = `13023433800416048247`, procedure accepts, relation admits |
| `the_procedure_refuses_the_lean_named_mutations` | green: all five mutants refused |

Raw gate logs: worktree-private `measure/ct01-proof-leg-{workspace-tests,clippy,probe,followup}.log`.
The full gate table is in §7.

## 6. Proposed proof-stage evidence (the integrator's flip)

The exact stage record for CT01's `proof` row in
`docs/reevaluation/ct_manifest.json`:

```json
"proof": {
  "status": "executed",
  "executor": "compiler/tests/ct01_proof_leg.rs::ct01_artifact_is_the_lean_proved_owned_transfer_shape",
  "artifact": "compiler/tests/fixtures/ct/CT01/case.qcore",
  "observations": 1
}
```

No `host_gate` (the runner is host-unconditional) and no `tool_gate` (no
external tool; the Lean theorems are checked by the standing
`check_lean_audits.py` gate, not by this runner).

The harness edits the flip requires, none of which this card makes:

* `compiler/tests/ct_corpus.rs`: add the executor to `RUNNER_REGISTRY`; give
  `derive_stage_plan` a proof arm in place of the unconditional
  `plan.insert("proof", StagePlan::absent("unassessed"))` (the line whose
  comment says advancing it is deliberate) — on the sanitizer-gate precedent,
  a pinned `PROOF_GATED_CASES: &[(&str, &str)]` mapping `CT01` to the executor
  keeps the gate fail-closed for every other case;
* the manifest row above, and the headline `proof 0 of 41 → 1 of 41` wherever
  the close reconciles it (`AGENTS.md` ledger, matrix regeneration through the
  standing generator with hand-kept sections carried).

Suggested reading of the flipped row, for the record: *proof — the scenario
property proved of the checked artifact's reference run and its admitted
renderings; native execution unproved, covered by observation only.* A grown
proof numerator is evidence about this case's artifact, not language
conformance (constraint 23's spirit applies to this denominator too).

## 7. Gate

Run at the card tip on Linux. One environmental fact is disclosed rather than
hidden: this worktree's host disk could not hold a full debug-info workspace
build (a first attempt died in `rust-lld` with signal 7 — a bus error in the
linker's memory-mapped output write — and a second in plain `No space left on
device`; both raw logs are in the worktree-private `measure/` directory). The
gate's workspace run was therefore executed after `cargo clean` with
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` —
debug-info emission off, which changes binary sizes and nothing about what any
test asserts or observes. No test, pin or assertion was edited to obtain the
results below.

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,394 passed / 0 failed / 8 ignored** (218 suite summaries, exit 0; wave-28 baseline 4,392 + this card's 2). None of constraint 16's five load-sensitive rows tripped — nothing needed re-verification in isolation |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | Passed 116 Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 192 of 193 green; the one expected failure is §7.1 |

### 7.1 The one expected failure

`scripts/tests`'
`test_the_derived_facts_are_recomputed_rather_than_carried` trips on
`validation.lean_audits`, which moves 115 → **116**
(`AssertionError: Lists differ: ['14.0.0', 115] != ['14.0.0', 116]`). This
card's brief forbids editing `docs/reviews/r0_3/acceptance_matrix.json`, so
the derived row is deliberately left stale for the wave integrator to
regenerate — the handling every audit-adding card's delta receives (the
`AGGREGATES-V1` and `LOOP-V1` reports record the same shape). The delta is
entirely this card's one audit file.

## 8. Follow-ons

* `CT01-PROOF-ROW` — the integrator flip of §6.
* `CT-PROOF-NATIVE-REFINEMENT` — already open in substance as the manifest
  profile's own "native runtime refinement proofs remain open"; this card
  neither closes nor narrows it.
* `CT-PROOF-SECOND-CASE` — the pattern is reusable wherever a case's artifact
  has a recorded, digest-pinned trace term; CT02's create-failure recovery is
  the nearest candidate, but no claim is made here.
