# `CT-PROOF-LEG-3` (wave 41) — survey every unproved mapped case, then land the leg the survey picks

Branched from `463fa6e8` (`origin/main` at branch time, confirmed equal) and
**rebased onto `96142e49`**, the documentation restructure, before merge — §13
records exactly what that changed and what it did not. Report directory: this
one. Evidence files beside this README:

| File | What it is |
|---|---|
| `probes/survey_probe.rs` | The survey probe, verbatim. It was a temporary `compiler/tests/` file, run, then removed from the tree and kept here. |
| `probes/survey.tsv` | Its raw output: 44 default-run rows and 60 scripted-arm rows. |
| `survey.json` | The survey as a record, one row per mapped case, built from the two by `scripts/build_survey.py`. |
| `scripts/build_survey.py` | Builds `survey.json`. |
| `scripts/build_stage_evidence.py` | Builds `ct_stage_evidence.json`; every non-`proof` row is carried, only `proof` is written. |
| `scripts/edit_named_followups.py` | The tracker edit — two retirements, three amendments, four registrations. Idempotent. |
| `scripts/mutate_ct50_proof_leg.py` | The non-vacuity harness. |
| `mutations.log` | Its shipped log: 20 mutations, three tiers, every restore sha256-verified. |
| `l3-lean-full.log` | The whole `lake build` output of the L3 mutation, kept because Lean reports the first error only in the harness line. |
| `ct_stage_evidence.json` | CT50's stage-evidence record. |
| `gate.json`, `gate/` | The detached full standing gate's exits and logs. |

**Headline.** CT50's `proof` stage moves `unassessed` → **executed** on a named
host-unconditional workspace test, and the proof numerator moves **4 of 47 →
5 of 47**. That is evidence about one case's artifact and never conformance
(constraint 23). No other count moves: mapped stays 47/50, sanitized-native
stays 10 of 47, no unit or function status, no corpus number, no `Q` code, no
reserved word. Beside it, `compiler/tests/proof_leg_digests.rs` closes
`PROOF-LEG-DIGESTS-UNCROSSCHECKED`, and the survey that chose CT50 also
measures that **it was the last F02-shaped candidate**, which is registered
rather than left in a report.

Two findings of this card's own harness are in §5 and are not cosmetic: the
harness caught two overclaims in this card's first draft, and both were
repaired in the artifact rather than in the prose.

---

## 1. Scope and what is true today (re-derived at `463fa6e8`, not inherited)

* CT coverage at base: **47/50 mapped**, sanitized-native 10 of 47, **proof 4
  of 47** — CT01, CT37, CT03 (bound through `AdmittedCausalOrder::derive` and
  the F02 digest relation) and CT07 (bound through the scoped-task ledger's
  lease set, because `reference_trace` refuses its run by constraint 15n's
  decided boundary). Read from `docs/reevaluation/ct_manifest.json` at this
  tree, not from the ledger row.
* The F02 observable alphabet at this tree, read from
  `crates/qcore-core/src/task_trace_relation.rs`: `Allocated`,
  `AllocationFailed`, `Spawned`, `CreationFailed`, `Read`, `Written` (added by
  `CT03-PROOF-ALPHABET`), `Dropped`, `Loaned`, `Returned`, `Completed`,
  `Joined`. **Refused**: `BufferByteWritten`/`BufferByteRead`,
  `BufferLeased`/`BufferLeaseReturned`, and the `ErrorWithBuffer` outcome. The
  Lean side (`QCore.TaskTraceRelation.Event`) carries the same constructors
  minus `written`, which lives in the conservative extension
  `QCore.TaskTraceWrites`; **the Lean `Outcome` has `unit`, `i32` and `buffer`
  and no error constructor**, which turns out to decide three survey rows (§2).
* `proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`
  (wave 38) requires a credited leg to `include_str!` the case's own fixture;
  `compiler/tests/ct50_proof_leg.rs` does.

## 2. The survey — `CT-PROOF-LEG-2`'s method, re-run over all 43 unproved mapped cases

**Method, measured rather than read off the code.** Each mapped case's own
`source_fixture` was compiled through `qcore_compiler::compile_files`; where the
module lowers to a checked task module, it was executed by
`qcore_core::task_execution::execute_task_module` and the observation handed to
`AdmittedCausalOrder::derive` — which is `reference_trace` (the encoding)
followed by `declare` (the relation's own admission), so a row that says
`DERIVES` has passed both. Then five failure scripts were run against every case
that compiles, because a scenario's defining arm is often not the default run.
Probe: `probes/survey_probe.rs`; raw output: `probes/survey.tsv`; the record:
`survey.json`.

### 2.1 The whole corpus in one table

| Default-run outcome | Cases | Why it rules the case out as an F02 leg |
|---|---|---|
| `DERIVES` | **11** — CT01, CT03, CT12, CT13, CT14, CT15, CT16, CT17, CT20, CT37, CT50 | Candidates; broken out in §2.2 |
| `COMPILE-REFUSED` | **24** — CT02, CT04, CT05, CT06, CT08, CT11, CT18, CT19, CT21, CT23, CT27–CT34, CT38, CT39, CT42, CT44, CT47, CT48 | The case *is* a refusal. There is no admitted artifact, so there is no reference run to prove anything about. Each row in `survey.tsv` carries the exact diagnostic code the checker gave |
| `NO-TASK-MODULE` | **8** — CT09, CT22, CT25, CT26, CT35, CT36, CT46, CT49 | Sequential, composed-epoch, mutex-guard and flat-epoch routes. The F02 relation is over the **task** route's reference execution; these publish no such observation (`as_scoped_tasks()` answers `None`, and the epoch route publishes `reference_observation: None` by construction) |
| `DERIVE-REFUSED` | **1** — CT07 | Already proved, through the ledger. Re-measured here: `Unsupported(UnsupportedObservedEvent { index: 2 })`, the `BufferLeased` at position 2 — constraint 15n's boundary, unmoved |
| not probed | **3** — CT40, CT43, CT45 | The `legacy_c` route: their artifact is a `case.c` C twin, not a QCore program. Their stage vocabulary is `parse` / `legacy_c_twin` / `proof` and contains no reference execution at all, so the F02 pattern has nothing to attach to |

44 + 3 = 47 mapped. The one-line justification per case, with the exact
diagnostic or refusal, is `survey.json`'s `rows`.

### 2.2 The eleven that derive, case by case

| Case | Default run | Binding pattern that could carry it | Cost |
|---|---|---|---|
| CT01, CT03, CT37 | derives | F02 — **already proved** | — |
| **CT12** | derives, 7 events | F02 in principle. But the trace is `[Allocated 0 1 42, Spawned 1 0, Completed 1 Buffer(1), Joined 1, Read 0 1 42, Dropped 0 1, Completed 0 I32(42)]` — **event for event CT01's `roundTrip`**. Its scenario is *Create fails before any child starts*, and the scripted arm is refused: `Violation(UnattributableEvent { index: 1 })` | A proof row would restate CT01's theorem under a second name. Needs `CT-PROOF-CONTROL-ARM-TIER` first |
| **CT14** | derives, 7 events | Identical trace to CT12 and CT01. Scenario *Child finishes but parent has not joined* is a **refusal** (Q600); the accepted control is the roundtrip | Same as CT12 |
| **CT13** | derives, 7 events | Scenario *Second create fails after first succeeds*. Scripted: `Violation(UnattributableEvent { index: 1 })` | Needs `CreationFailed` attributed in the relation |
| **CT15** | derives, 4 events | Scenario *Backend join failure* — **no event in the alphabet at all**; the scripted creation-failure arm is `UnattributableEvent { index: 0 }` | Needs a join-failure event *and* attribution |
| **CT16** | derives in Rust, 8 events | Scenario *Worker typed error after partial local acquisition*. The default trace carries `Completed _ Error("WorkerError::Failed")` twice — **and `QCore.TaskTraceRelation.Outcome` has no error constructor**, so the Lean side cannot express it | Needs the `Outcome` alphabet to grow, with every digest re-pinned |
| **CT17** | derives, 11 events | The default run is the **success** path. The scenario is *Worker error payload contains owned result*, whose outcome is `ErrorWithBuffer` — refused by `reference_trace` itself | Needs a second alphabet growth (the payload-carrying outcome) |
| **CT20** | derives, 10 events | The default run is the **no-error** path: the in-scope handler and the rethrow the scenario names never run. Both scripted creation-failure arms are `UnattributableEvent` (index 1 and index 2) | Needs attribution *and* error outcomes |
| **CT50** | derives, 6 events, **and exhibits its scenario** | F02, single-actor | **Chosen** — §3 |

The scripted arms were run for completeness and confirm the pattern in one
direction and extend it in another. **Every** creation-failure arm in the corpus
is refused by the relation, not by the encoding: `UnattributableEvent` at index 1
for CT12, CT14, CT17, CT20 and at index 0 for CT13, CT15, CT16. And every
**allocation**-failure arm that runs derives in Rust but produces
`[AllocationFailed 0, Completed 0 Error("AllocError")]` — an error outcome the
Lean alphabet cannot express. So the condition
`CT-PROOF-CREATION-FAILURE-ALPHABET` records names **one** extension where the
cases it lists need **two**; the entry is amended accordingly (§6).

### 2.3 The claim `STABILITY-REEVAL-4` asked to be verified

> CT50 is the only remaining F02-shaped candidate.

**It holds**, and it is now verified rather than inherited: of the eleven cases
whose default reference run derives, three are proved already, two (CT12, CT14)
reproduce CT01's own trace event for event, and five (CT13, CT15, CT16, CT17,
CT20) derive without exhibiting their scenarios, whose defining arms need one or
both of the two alphabet extensions above. CT50 is the one whose default run
both derives *and* exhibits its scenario.

Because this card **used** it, the fact to register is stronger than the one the
audit named: the candidate set is now **empty**. Registered as
`CT-PROOF-F02-CANDIDATES-EXHAUSTED` (§8), with the consequence stated — a sixth
leg must grow the alphabet on `CT03-PROOF-ALPHABET`'s method, invent a binding
pattern as CT07's did, or wait for a case the mapping lanes have not mapped.
`CT-PROOF-FOURTH-CASE-UNTRACKED` is retired by that registration.

## 3. The leg: CT50

CT50's artifact is a ten-line program: allocate a Buffer carrying `7`, release
it, allocate a second carrying `42`, read that one back, release it, return.
Its reference run, derived by the production observer, is six events, all the
root task's:

```
Allocated { task: 0, buffer: 1, payload: 7 }
Dropped   { task: 0, buffer: 1 }
Allocated { task: 0, buffer: 2, payload: 42 }
Read      { task: 0, buffer: 2, value: 42 }
Dropped   { task: 0, buffer: 2 }
Completed { task: 0, outcome: I32(42) }
```

### 3.1 The Lean side — `proof/QCore/Ct50LifetimeIdentity.lean`

New module (audit `proof/AuditCt50LifetimeIdentity.lean`, Lean audits **141 →
142**), importing `QCore.Ct01OwnedTransfer` **only** for the registered
`ReflBEq`/`LawfulBEq` instances on the F02 alphabet — no CT01 definition is
reused, and `QCore.TaskTraceRelation` is untouched. The alphabet did **not**
grow: every event of this run is in the base alphabet, so no existing digest
moved.

* **`referenceRun`** — the six events above, proved a declared admitted causal
  order (`ct50_reference_run_is_an_admitted_causal_order`, `declare referenceRun
  = 0` by `decide`), with `root_projection_of_the_reference` recording that the
  run has one actor. `ct50_trace_digest : mixTrace referenceRun =
  1699395409980170611` is the transcription pin the Rust runner recomputes.
* **`dischargesLifetimeIdentity : List Event → Bool`** — a total decision
  procedure, eleven clauses: six facts, all the root's; each of the two
  generations allocated once and released once; **`t.countP isStaleUse = 0`**,
  where a *stale use* is any fact naming generation 1 that is neither its own
  allocation nor its own release; exactly one read, which is of generation 2
  carrying `42`; and two order chains as `List.Sublist` — `[staleAllocation,
  staleRelease, freshAllocation]` (the old lifetime closes before the new one
  opens) and `[freshAllocation, freshRead, freshRelease]` (the read is inside
  the new lifetime).
* **Meaning theorems**, each for an *arbitrary* accepted trace:
  `discharged_each_generation_is_allocated_and_released_once`,
  `discharged_the_old_lifetime_closes_before_the_new_one_opens`,
  `discharged_no_fact_uses_the_old_capability`,
  `discharged_the_old_generation_is_never_read` (derived from the stale-use
  clause),
  `discharged_the_one_read_names_the_new_generation`,
  `discharged_the_read_sits_inside_the_new_lifetime`,
  `discharged_every_fact_is_the_root_tasks`.
* **`ct50_reference_run_discharges_the_lifetime_identity`** — the reference
  trace is accepted (`decide`, kernel-checked).
* **`an_admitted_rendering_is_the_reference_run`** — the closure, and the reason
  this leg's closure is worth stating separately. CT01's and CT37's closures are
  symbolic arguments over per-actor clauses; CT50's run has **one actor**, and
  `relate` demands agreement with the reference on *every* actor's projection
  over the same multiset of facts, which for one actor forces equality. So the
  admitted set is a singleton — proved here, not assumed, and
  `every_admitted_rendering_discharges_the_lifetime_identity` follows from it in
  two lines.
* **Nine named refusals**, four of them proved to fail **exactly one** clause
  with every other clause still holding (§5.2).

Axiom surface: `propext` / `Classical.choice` / `Quot.sound` only; the digest
pin and the root projection are **axiom-free**
(`measure/lean-audits-2.log`; 31 `#print axioms` declarations). No
`native_decide`, no placeholder, no project axiom.

### 3.2 The Rust side — `compiler/tests/ct50_proof_leg.rs`

The credited executor,
**`ct50_artifact_is_the_lean_proved_lifetime_identity_shape`**, is
host-unconditional (no `cfg`, no Clang, no native half). It:

1. compiles CT50's own fixture (`include_str!`) through `compile_files`,
   executes `execute_task_module` on the checked task module, derives the
   admitted causal order with the production observer, and asserts the events
   equal the Lean `referenceRun` transcription, event for event;
2. recomputes `mixTrace` over the artifact's events with wrapping `u64`
   arithmetic and asserts it equals `CT50_LEAN_TRACE_DIGEST`;
3. runs `discharges_lifetime_identity`, the clause-for-clause transcription,
   and requires acceptance;
4. asserts the artifact's rendering is admitted by the production `relate`;
5. **enumerates** `admitted_linearizations(&order, 100_000)` and requires the
   admitted set to be exactly `[reference_run()]` — the artifact-side
   measurement of `an_admitted_rendering_is_the_reference_run`. This is the one
   step no earlier leg has: the closure theorem is not only proved in Lean but
   checked by exhaustive search on this case's own order.

Four further tests: `the_procedure_refuses_the_lean_named_mutations` (nine
mutants), `the_isolating_mutants_fail_only_their_named_clause` (the four
isolating ones, re-measured on the transcription),
`the_measured_non_isolating_mutant_and_the_implied_clause_are_stated` (§5.2) and
`the_length_changing_mutants_are_named_as_such`. **5 passed; 0 failed.**

### 3.3 What is covered, and what is not

**Covered.** Proved of CT50's *checked artifact*: the two generation lifetimes
are disjoint — the old generation's allocation and release both precede the new
generation's allocation, as a subtrace; **no fact of the run uses the old
generation** (no read of it, no loan, no return, no completion carrying it out);
the one read names the new generation and carries the new allocation's own
payload, inside the new generation's own lifetime; each generation is allocated
once and released once; and every fact is the root task's. For the reference
trace by kernel `decide`, and for every admitted rendering by the singleton
theorem.

**Not covered — stated so the row cannot be over-read.**

* **The numeric address reuse is not proved.** CT50's scenario is *New lifetime
  reuses old numeric address/descriptor*, and its native evidence really does
  place the second mapping at the first one's address with
  `MAP_FIXED_NOREPLACE` (the pinned stderr transcript `MAP 1 / UNMAP 1 / MAP 2
  / REUSED_ADDRESS / UNMAP 2 / RUN_CLEAN 2`). The F02 alphabet carries
  **generations, not addresses**, so every theorem above would hold equally if
  the second mapping landed somewhere else. The address half stays
  `run_native_task_scenarios`' observation. Registered as
  `CT50-PROOF-ADDRESS-REUSE-UNPROVED`.
* **`native_sanitized` stays `recorded_not_gated`** for CT50 — a transcript in
  `docs/reports/w6-reused-address/README.md` that no workspace test re-runs.
  This card does not touch it and the sanitized-native count does not move.
* **The Lean ↔ Rust tie is transcription agreement, not verified extraction** —
  the F02 pattern, now with the digest cross-check of §4 beside it.

## 4. The digest cross-check — `compiler/tests/proof_leg_digests.rs`

`PROOF-LEG-DIGESTS-UNCROSSCHECKED` (measured by `STABILITY-REEVAL-4`): every leg
pins a Lean-decided digest as a bare Rust literal that nothing compares with its
Lean twin, and no test under `compiler/tests/` or `crates/` read any `.lean`
file, so someone who re-derived a digest from new bytes and updated both
literals would be green in both gates.

The new test reads both sides and compares them, **total or reject in both
directions**: every `theorem ct<N>…digest : … = <numeral> :=` under
`proof/QCore/` must have a Rust constant for the same case carrying the same
numeral; every `const CT<N>_LEAN_…DIGEST: u64 = <numeral>;` under
`compiler/tests/` must have a Lean theorem; and the case set is itself pinned in
`CASES_WITH_A_DIGEST_PIN`, so a leg added with neither side registered trips
here.

**Measured inventory at this tip** (`the_digest_inventory_is_reported`, run with
`--nocapture`):

| | Case | Symbol | Value | Site |
|---|---|---|---|---|
| LEAN | CT01 | `ct01_trace_digest` | 13023433800416048247 | `proof/QCore/Ct01OwnedTransfer.lean:79` |
| LEAN | CT03 | `ct03_trace_digest` | 9638775376944872586 | `proof/QCore/Ct03ReadShares.lean:126` |
| LEAN | CT07 | `ct07_ledger_digest` | 8876998389998880787 | `proof/QCore/Ct07LeaseLedger.lean:209` |
| LEAN | CT37 | `ct37_trace_digest` | 17239871955803798867 | `proof/QCore/Ct37CompletionCapsule.lean:106` |
| LEAN | CT50 | `ct50_trace_digest` | 1699395409980170611 | `proof/QCore/Ct50LifetimeIdentity.lean:93` |
| RUST | CT01 | `CT01_LEAN_TRACE_DIGEST` | 13023433800416048247 | `compiler/tests/ct01_proof_leg.rs:52` |
| RUST | CT01 | `CT01_LEAN_TRACE_DIGEST` | 13023433800416048247 | `compiler/tests/ct12_ct14_proof_reach.rs:63` |
| RUST | CT03 | `CT03_LEAN_TRACE_DIGEST` | 9638775376944872586 | `compiler/tests/ct03_proof_leg.rs:55` |
| RUST | CT07 | `CT07_LEAN_LEDGER_DIGEST` | 8876998389998880787 | `compiler/tests/ct07_proof_leg.rs:75` |
| RUST | CT37 | `CT37_LEAN_TRACE_DIGEST` | 17239871955803798867 | `compiler/tests/ct37_proof_leg.rs:49` |
| RUST | CT50 | `CT50_LEAN_TRACE_DIGEST` | 1699395409980170611 | `compiler/tests/ct50_proof_leg.rs:64` |

Five Lean theorems, **six** Rust constants. The residual's own inventory named
four sites and gave CT01's as `ct12_ct14_proof_reach.rs:63`; there is a **second
CT01 site**, `ct01_proof_leg.rs:52`, which the inventory missed. The
cross-check covers both, and mutation `D3` is on the site the residual named.

**Deviation from the discharge, stated.** The entry asks for "one script in the
standing gate". This is a **workspace test** instead. `cargo test --workspace
--no-fail-fast` is the standing gate's first step, so it runs in the gate as
asked; a test also gets the harness's own failure reporting and its line
numbers, and cannot be forgotten by a separate invocation list. The second half
of the entry's discharge — correcting `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`'s
sentence — is §6.

**Its own premise, registered rather than hidden**: the test compares the
numeral in the `.lean` *source* with the Rust literal, and what makes the Lean
numeral the kernel's is the standing `scripts/check_lean_audits.py` step, not
this test. `PROOF-LEG-DIGEST-CROSSCHECK-TEXTUAL` records that, and the parser's
one fail-open direction (a digest theorem split across lines would not be seen),
which is why `CASES_WITH_A_DIGEST_PIN` is pinned beside it as the fail-closed
half.

## 5. Non-vacuity

Harness: `scripts/mutate_ct50_proof_leg.py`; log: `mutations.log`. Baseline
green before the first mutation and after the last; every restore rewrites the
file's bytes and touches its mtime (never mtime-preserving —
`MUTATION-HARNESS-STALE-ARTIFACTS`) and is verified **sha256 EQUAL**. Harness
exit 0.

### 5.1 Tier F — the fixture mutations, with their catch points separated

The brief asks that the binding go red **for the theorem's reason, not because a
checker refused earlier**. Each fixture mutation is therefore measured at the
place the pipeline actually catches it:

| Mutation | Catch point, measured |
|---|---|
| **F1** `drop old;` removed (the leak) | **The checker**, before any artifact exists: `ct50_proof_leg.rs:304` (the `compile_files` refusal), `Q600` at `case.qcore:8` |
| **F2** the read redirected to the released binding, `qcore_buffer_read(old)` | **The checker**: `:304`, `Q600` at `case.qcore:7`. This is the scenario's own crux, and the checker refuses it — which is the right answer and is *not* evidence about the leg |
| **F3** the second allocation's payload `42` → `43` | **The binding**: the checker **admits** it; `:324`, `assert_eq!(events, reference_run())` — the artifact is no longer the Lean term |
| **F4** the first lifetime removed entirely | **The binding**: admitted by the checker; `:324` again — a four-event trace with one generation, which is *reachable and not the scenario* |

So the two refusing mutations never reach the binding, and the two the checker
admits are exactly the ones the binding must catch. That is `CT07`'s separation,
applied here.

### 5.2 Tier R — the rule mutations, and the two defects they found in this card's own claims

Eight mutations of the transcription. **Six reddened as designed** (R1 the
stale-use clause, R2 the old-lifetime chain, R3 the read-window chain, R6 the
stale generation's identity, R7 the enumeration's bound — proving the
enumeration is really performed — and R8 the expected admitted set, proving it
is really compared). **Two did not, and both were real defects in the first
draft of this card, repaired in the artifact rather than in the prose:**

* **R4 — dropping the single-actor clause reddened nothing.** The reason: the
  mutant this card had written to isolate that clause moved a *read* to actor 1,
  which also moves it out of the read-window chain, so two clauses caught it and
  neither alone was load-bearing on it. The Lean theorem asserting the isolation
  was true only because it did not list that chain among the clauses that still
  hold. **Repair:** a new mutant `resultClaimedByASecondActor` — the task's own
  result attributed to a child — which genuinely isolates: every one of the other
  ten clauses is proved still to hold
  (`a_result_claimed_by_a_second_actor_fails_only_the_single_actor_clause`). The
  old mutant is **kept** and honestly re-labelled
  (`a_read_by_a_second_actor_fails_two_clauses`), because it is still a refusal —
  it just is not an isolating one. R4 now reddens.
* **R5 — weakening the payload clause reddened nothing, and no mutant can make
  it redden.** The reason is not thin mutants: the clause `t.countP (readsBack
  fresh 42) = 1` is **implied** by the read-window chain together with `t.countP
  isRead = 1`, because the chain puts `freshRead` in the trace and the count says
  there is only one read. **Repair:** the implication is now the Lean theorem
  `the_payload_clause_is_implied_by_the_read_window_and_the_read_count`, and the
  clause is kept — it is the sentence the scenario is about and the meaning
  theorem reads it directly. R5's row in `mutations.log` is labelled *GREEN (as
  measured)* with that ground, and the harness treats it as an expected-green row
  that would be a **failure if it reddened**.

Clause isolation is measured on both sides: four Lean theorems prove that four
mutants fail exactly one clause with every other clause holding, and the Rust
test `the_isolating_mutants_fail_only_their_named_clause` re-measures the same
isolation on the transcription. The two length-changing mutants are **named as
such** rather than presented as isolating.

### 5.3 Tier L — the Lean terms

| Mutation | Result |
|---|---|
| **L1** the digest numeral `+1` | `lake build` fails at the `decide` (`:94`) |
| **L2** `freshRead`'s payload `42` → `41` | fails at the reference-run `decide` (`:85`) |
| **L3** `isStaleUse`'s `mentions stale` blinded to `mentions 99` | fails at three sites, kept in full in `l3-lean-full.log`: `an_escaping_stale_capability_is_refused` (`:397`), the `countP isStaleUse = 1` conjunct of its isolation theorem (`:414`), and the derived `discharged_the_old_generation_is_never_read` (`:204`). The harness's log line shows Lean's first error, which is the derived theorem; all three are the clause being load-bearing |

### 5.4 Tier D — the digest cross-check

| Mutation | Result |
|---|---|
| **D1** one digit of the Rust CT50 literal | `proof_leg_digests.rs` red, naming both sites and both values |
| **D2** one digit of the **Lean** CT50 numeral, with no `lake` run | red — the test really reads the `.lean` file |
| **D3** one digit of **CT01's** literal in `ct12_ct14_proof_reach.rs` | red — the cross-check covers the existing legs, not only the new one |
| **D4** the Lean `ct50_trace_digest` theorem deleted | red — a pin removed is refused, not silently ignored |

## 6. The three carried entries

**`LEASE-BOUNDARY-TRANSCRIPTION-PREMISE` — discharge text repaired; entry stays
open.** The entry's second discharge arm read "bind the Lean term to the Rust
pin by a generated digest the way CT01's proof leg binds its trace", and
`STABILITY-REEVAL-4` measured that CT01's leg does no such thing. That mechanism
now exists, so the arm is rewritten **in the entry's own terms** to name it: give
`QCore.TaskTraceLeaseBoundary`'s `flagshipRun` a kernel-decided digest theorem
named `ct<N>…digest`, pin the same numeral in a `compiler/tests/` constant named
`CT<N>_LEAN_…DIGEST`, and register the case in `CASES_WITH_A_DIGEST_PIN` so the
total-or-reject cross-check covers it. **The entry stays open**, deliberately:
this card built the mechanism and did not apply it to the lease-boundary module,
which is another card's artifact and outside this card's file ownership. The
first arm — carry the module's own transcription sentence into constraint 15n —
is unchanged and remains the cheaper discharge.

**`CT-PROOF-CONTROL-ARM-TIER` — restated as a measured boundary, not landed,
with the ground stated.** The tier is a **stage-vocabulary and manifest** change
(a new stage tier plus a restatement of what the numerator counts), which is spec
work above a card; and the numerator sentence it asks for already stands in the
CT ledger row ("the proof leg has two shapes, not one"). What this card adds is
the measurement the tier exists for, re-run rather than inherited: CT12's and
CT14's default reference runs are **event for event CT01's** (§2.2). So without
the tier a proof row for either would credit CT01's artifact under a second name
— the tier is load-bearing, not a formality. The entry stays open with that
measurement in it.

**`CT-PROOF-CREATION-FAILURE-ALPHABET` — restated and sharpened.** The entry
names one alphabet extension (attribute `CreationFailed` to a creator). The
survey measured that the cases it lists need **two**: every creation-failure arm
is refused as `UnattributableEvent`, as the entry says — but every
allocation-failure arm that derives carries an `Error` outcome that
`QCore.TaskTraceRelation.Outcome` has no constructor for, and CT16's **default**
run carries one with no script at all. Both measurements are in the entry now,
with the raw output cited. The entry stays open; no claim is made that either
extension is worth the cost.

## 7. `PROOF_GATED_CASES`, the manifest and the stage-evidence record

The diff to `compiler/tests/ct_corpus.rs` (the pinned fail-closed table):

```diff
     (
         "CT07",
         "compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape",
     ),
+    // Card `CT-PROOF-LEG-3` (wave 41): CT50's lifetime-identity procedure over
+    // the F02 observation alphabet (QCore.Ct50LifetimeIdentity,
+    // AuditCt50LifetimeIdentity.lean). The fourth leg bound through the trace
+    // relation, and the first whose closure over admitted renderings is the
+    // **degenerate** case — the run is single-actor, so the relation admits
+    // nothing but the run itself, proved as
+    // `an_admitted_rendering_is_the_reference_run` and measured on the
+    // artifact by enumerating `admitted_linearizations`. The alphabet did not
+    // grow. Scope: the capability half of the scenario — the numeric address
+    // reuse it names is invisible to this alphabet and stays with
+    // `run_native_task_scenarios`' observation, unproved.
+    (
+        "CT50",
+        "compiler/tests/ct50_proof_leg.rs::ct50_artifact_is_the_lean_proved_lifetime_identity_shape",
+    ),
 ];
```

Landed in lockstep, each a fail-closed check of the others: the same string in
`RUNNER_REGISTRY`; the manifest's CT50 `proof` row → `executed` with that
executor, the case's own artifact, 1 observation; `expected.json`'s
`stages_executed` += `"proof"`; `docs/reviews/r0_3/acceptance.json`'s generated
fields rewritten by `scripts/ct_stage_summary.py --write` (proof executed 4 → 5,
CT50's `case_depth` `native_execution` → `proof`) and its `derived_from` pin
extended; both `EVIDENCE_FILES` copies (`scripts/ct_stage_summary.py`,
`ct_corpus.rs`'s `[&str; 25]` → `[&str; 26]`) extended with
`docs/reports/ct-proof-leg-3/ct_stage_evidence.json`.

That record declares `"supersedes": "docs/reports/remaining-t00/coverage.json"`
— card T00's frozen archive was still the record holding CT50's stages — and
carries every non-`proof` row from the manifest's own CT50 rows. **Measured**
(`measure/cmp_t00.py`): those agree with the archive in status and observation
count for all seventeen stages, and differ only in the `host_gate` of
`task_reference_execution` and `c_emission`, which later lanes widened from
`["linux"]` to `["linux","macos","windows"]`. The archive keeps its bytes
(constraint 21). `ct_stage_summary.py --check` exit 0; `ct_corpus` green,
**6 passed**, including
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture` over the
new row.

## 8. Registrations and retirements

**Retired** (each verified registered by id before removal):

* **`PROOF-LEG-DIGESTS-UNCROSSCHECKED`** (recorded by `STABILITY-REEVAL-4`).
  Discharge: "one script in the standing gate that extracts each `theorem
  ct*_digest : ... = N` from `proof/QCore/` and compares it with the
  corresponding const in `compiler/tests/`, shown red by mutation; AND correct
  `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`'s discharge sentence". Met: §4 (a
  workspace test rather than a standalone script — the deviation is stated
  there), D1–D4, and §6.
* **`CT-PROOF-FOURTH-CASE-UNTRACKED`** (recorded by `STABILITY-REEVAL-4`).
  Discharge: "register the survey's conclusion (or a decision that a fourth
  pattern is to be invented instead), and state in the CT ledger row that the
  proof numerator counts more than one binding pattern". Met: §2.3 and the
  registration below; the ledger row's "two shapes, not one" sentence was
  verified present at base and is kept and extended.

**Registered** (`recorded_by: CT-PROOF-LEG-3`, each with a discharge
condition):

* **`CT-PROOF-F02-CANDIDATES-EXHAUSTED`** — the survey's conclusion as a fact.
  Discharge: the next proof-leg card records which of the three remaining routes
  it took, and retires this entry.
* **`CT50-PROOF-ADDRESS-REUSE-UNPROVED`** — §3.3. Discharge: an alphabet that
  carries a mapping identity beside the generation (on `CT03-PROOF-ALPHABET`'s
  method, with the same conservativity obligation), or an explicit decision that
  address identity stays native evidence, recorded in the manifest's CT50
  semantic profile.
* **`PROOF-LEG-DIGEST-CROSSCHECK-TEXTUAL`** — §4's own premise. Discharge: emit
  the digests from Lean itself so the compared value comes from the kernel's
  elaboration rather than a line of source, or state the premise in the file and
  accept it deliberately.
* **`STATUS-SOURCES-CHECKPOINT-DIVERGES`** — added at the rebase (§13): the
  restructure's `rederive_status.py --check` asserts equality with the live
  tree, and this card is the first to move inputs it snapshots. Nothing goes
  red (the script is in no gate, verified by grep). Discharge: declare `--check`
  a checkpoint re-derivation tool rather than a gate, or publish a regenerated
  snapshot beside the frozen one.

**Amended, entries kept open**: `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`,
`CT-PROOF-CONTROL-ARM-TIER`, `CT-PROOF-CREATION-FAILURE-ALPHABET` (§6).

`named_followups` **257 → 259** (+4 −2; the fourth registration is §13's).

## 9. Keyword census and diagnostics

**+0 words, +0 tokens, and there was nothing to census**: the card adds a Lean
module, a Lean audit, two test files and records. No lexer change, no route
selection change, no checker or emitter change — `git diff --stat` over
`crates/` and `compiler/src/` is empty. **No `Q` code is allocated and no
condition is widened**; `docs/QCORE_SPEC_r0_3.md` §L21 is untouched. The `Q600`
refusals the F1 and F2 fixture mutations meet are the task route's existing
custody conditions, reached unchanged. `is_reserved_source_word` is unmoved, so
the reserved-word count stays **113**.

## 10. Gate

One detached full standing gate (`measure/gate.sh`: `setsid -f`, no
`trap '' TERM`, every step's exit to its own file, no pipe), **polled in a
foreground bounded loop**, run at tip **`0f8a5323`** — the commit carrying
every code change of this card plus its records; the commits after it are this
gate's own evidence and the matrix regeneration. Exits and the short logs are
copied into `gate/` beside this file, and the figures below were extracted from
them by `measure/build_gate_json.py` (`measure/gate-facts.json`) — **nothing
typed in**.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0, `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | **exit 0 — 4,735 passed / 0 failed / 10 ignored** over 240 result summaries and 238 test targets (229 `Running` + 9 doc-test sections; two of those print two summaries each). `test result: FAILED` **0** times, so no constraint-16 isolated re-verification was required and **none is claimed**. 21m47s wall (06:59:43Z → 07:21:30Z) with sibling worktrees building |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `python3 scripts/check_lean_audits.py` | exit 0, **"Passed 142 Lean audits"** |
| `python3 -m unittest discover -s scripts/tests` | **exit 1, deliberately reported** — see below |
| matrix regeneration | `build_matrix.py . --gate docs/reports/ct-proof-leg-3/gate.json` exit 0: named followups 258, lean audits 142, contract modules 88, CT depth `proof: 5` |
| Python **post**-regeneration | `gate/python-post.exit` **0**, `Ran 303 tests … OK`; `ct_stage_summary.py --check` exit 0 |
| `clang --version` | exit 0, `Ubuntu clang version 21.1.8 (6ubuntu1)` (`gate/clang-version.txt`), `QCORE_CLANG` unset, **run in this worktree by this card's own gate script** |
| disk | 199 GB → 116 GB free across the run |
| provider 122 | **carried, not re-measured**: nothing under `providers/` or `crates/qcore-schemas/` moved and `qcore-schemas` is 14.0.0, so constraint 18 is not engaged; a Unix figure |

**Deltas against the wave-40 close's 4,727 / 303 / 141**: workspace **+8** —
exactly `ct50_proof_leg.rs`'s five tests and `proof_leg_digests.rs`'s three;
targets +2, summaries +2, ignored +0, Python +0, Lean **+1**
(`AuditCt50LifetimeIdentity.lean`).

**The Python step's exit 1, in full.** Three failures, all regeneration
tripwires against the tree in which this card had edited `named_followups`
(257 → 258) and added an `Audit*.lean` (141 → 142) and not yet regenerated:
`test_each_derived_dimension_matches_the_section_it_counts` (`257 != 258`),
`test_the_derived_facts_are_recomputed_rather_than_carried` (the Lean count),
and `test_the_committed_matrix_keeps_the_generators_byte_format` — the third
because this card's own tracker script had serialised with `ensure_ascii`
defaulted on. That was a real defect in the script, **repaired in the script**
(it now writes `indent=1, ensure_ascii=False`, the committed format) rather than
by hand-fixing the file, and then regenerated. Post-regeneration Python is
green: `Ran 303 tests … OK`.

**A gate figure is a measurement on one machine, never a property.** This one
was taken under sibling load and is offered as this card's own figure, to be
replaced by the wave-41 close's integration-tip measurement.

## 11. Files touched, and sibling ownership

Owned by this card per the wave-41 brief: `compiler/tests/ct_corpus.rs`
(`PROOF_GATED_CASES`, `RUNNER_REGISTRY`, `EVIDENCE_FILES`) and this card's own
new leg file. Also touched, **none owned by another wave-41 card**:
`compiler/tests/ct50_proof_leg.rs`, `compiler/tests/proof_leg_digests.rs`,
`proof/QCore/Ct50LifetimeIdentity.lean`, `proof/AuditCt50LifetimeIdentity.lean`,
`proof/QCore.lean` (one import),
`compiler/tests/fixtures/ct/CT50/expected.json` (`stages_executed`),
`docs/reevaluation/ct_manifest.json` (CT50's proof row),
`docs/reviews/r0_3/acceptance.json` (generated fields, `derived_from`),
`scripts/ct_stage_summary.py` (`EVIDENCE_FILES`),
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`; derived sections
regenerated with `--gate`), **`docs/STATUS.md`** (headline numbers — see §13),
`docs/development/increments/{README.md,wave-41.md}`. **`AGENTS.md` and every
`docs/agents/*.md` page are untouched**, and no `<!-- preserved:… -->` block is
edited anywhere. The files
`OWNED-FIELD-ROUTE-REACH`, `COMPOSED-WINDOW-REACH` and `TSAN-FAILURE-ARMS` own
— `crates/qcore-core/src/{ownership,source_record,std_buffer,std_string}.rs`,
`proof/QCore/LoopWindows.lean`, `tests/support/thread_sanitizer.rs`,
`compiler/tests/task_route_tsan.rs`, `compiler/tests/write_lend_tasks_exercise.rs`
— are **untouched**. One shared-file note: `compiler/tests/proof_leg_digests.rs`
reads every `.rs` under `compiler/tests/` and every `.lean` under
`proof/QCore/` at run time, so a sibling card that adds a Lean digest theorem or
a `CT<N>_LEAN_…DIGEST` constant without registering its case in
`CASES_WITH_A_DIGEST_PIN` will turn this test red at the merge — deliberately,
and the fix is the one-line registration.

## 12. For the integrator

1. **Headline numbers, edited in this tree — all in `docs/STATUS.md`** (this
   card was rebased onto the documentation restructure; §13): the
   *CT case-bound proof* row `**4/47**: CT01, CT03, CT07, CT37` →
   `**5/47**: CT01, CT03, CT07, CT37, CT50`; the *Registered follow-ups* row
   **257 → 258**; and the "Mapping is not conformance" paragraph, whose F02/ledger
   sentence gains CT50 on the F02 side and which now also carries CT50's
   capability-half scope, the exhaustion fact and the digest-cross-check clause.
   Lean audits move **141 → 142** and are derived by the matrix generator, not
   written in `STATUS.md`. The *Last integrated gate* section is **not** edited
   by this card — it is the wave-40 close's measurement, and this card's own
   figures are in `gate.json`.
2. **No standing constraint is landed or amended.** No `docs/agents/*.md` page
   is touched and no `<!-- preserved:… -->` block is edited: no route, checker,
   emitter, alphabet or diagnostic changes. If a future card grows the F02
   alphabet, constraint 15n's amendment discipline applies to it and not to this
   one.
3. **Matrix**: `named_followups` carried with the two retirements, three
   amendments and three registrations above; `dimensions` regenerated with
   `--gate docs/reports/ct-proof-leg-3/gate.json` (§10). At the union with
   sibling cards' `named_followups` edits, the *Registered follow-ups* row in
   `docs/STATUS.md` must be re-summed.
4. **Records**: `docs/development/increments/wave-41.md` created in wave-40's
   shape with this card's paragraph; the index's pointer and table row moved to
   wave 41. Sibling paragraphs union at the merge.
5. **Decision this card did not take, and deliberately**:
   `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE` now has a discharge arm that names a
   real mechanism, and applying it to `QCore.TaskTraceLeaseBoundary` is a
   one-theorem, one-constant, one-list-entry change. It belongs to whoever owns
   that module; this card repaired the text only.
6. **Not claimed**: any native, sanitizer, Windows or Apple observation (none
   was run beyond the standing gate's own workspace tests); any conformance; any
   change to the F02 alphabet, `reference_trace`, the checker or the emitter;
   and, specifically, **no claim about CT50's numeric address reuse**, which
   stays native observation (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`).

## 13. Rebase onto the documentation restructure

This card was branched from `463fa6e8` and **rebased onto `96142e49`**
("docs: restructure guides into routed rules and a unified hub") before merge.
The restructure replaced the ~173 KB `AGENTS.md` ledger with a 5.4 KB router,
moved headline numbers to `docs/STATUS.md` and standing constraints to
`docs/agents/*.md` behind `<!-- preserved:… -->` markers that a reconstruction
check reads.

**What the rebase changed, and nothing else.** One commit conflicted
(`the survey, the tracker edits, the ledger and the record`), in two files:
`AGENTS.md`, resolved **entirely in favour of the restructure** — my ledger
hunks were dropped there and re-applied to `docs/STATUS.md` — and
`docs/development/increments/README.md`, which auto-merged correctly (the
restructure's rewording plus this card's wave-41 pointer and table row). The
wave-41 record's header was then routed through `AGENTS.md` → rule reference and
status page, matching the restructured index, and its paragraph records where
the ledger edits landed. The report's own §11 and §12 were re-routed to name
`docs/STATUS.md`. **Nothing else moved**: the survey, the CT50 leg, the Lean
module and audit, the digest cross-check, the mutation harness and its log, the
`PROOF_GATED_CASES` / `RUNNER_REGISTRY` / `EVIDENCE_FILES` edits, the manifest,
`expected.json`, `acceptance.json`, the stage-evidence record and the three
carried entries' dispositions are byte-identical to the pre-rebase commits.

**No preserved block is edited.** This card lands and amends no standing
constraint, so no `docs/agents/*.md` page is touched — which also means the
restructure's reconstruction check is unaffected by it. `docs/STATUS.md` carries
no preservation markers.

**One consequence worth naming, not repaired here.**
`docs/reports/docs-structure-compression/rederive_status.py --check` asserts
that its `status-sources.json` still equals the live tree, and this card moves
three inputs it snapshots (the manifest's `proof` ids, `named_followups`, the
`Audit*.lean` count). That script is **not** in the standing gate — it is not
invoked by `scripts/tests`, by `check_lean_audits.py` or by any CI step here —
so nothing goes red; and its file is that card's own frozen checkpoint, which
constraint 21 says to leave alone. Registered as
`STATUS-SOURCES-CHECKPOINT-DIVERGES` so the next card that touches the
restructure's own evidence can decide whether the checkpoint should be re-derived
or explicitly frozen.

The gate in §10 is the **re-run on the rebased tree**; the pre-rebase run's
figures are superseded by it and are not carried.
