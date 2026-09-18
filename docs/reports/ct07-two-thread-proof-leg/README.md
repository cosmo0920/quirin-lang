# `CT07-TWO-THREAD-PROOF-LEG` (wave 39) — bind CT07's disjointness theorem to its artifact through the ledger

Base `e878b362` (`origin/main` at branch time, confirmed equal). Report
directory: this one. Evidence files beside this README:
`ct_stage_evidence.json` (the integrator's flip, superseding the wave-37
`ct-lane-5` record from its own record), `mutations.log` (the shipped harness
run), `scripts/mutate_ct07_proof_leg.py` (the harness),
`scripts/edit_named_followups.py` (the tracker edit, idempotent), `gate.json`
and `gate/` (the detached full standing gate's exits and logs).

**Headline.** CT07's `proof` stage moves `unassessed` → **executed** on a named
host-unconditional workspace test, and the proof numerator moves **3 of 46 →
4 of 46**. That is evidence about one case's artifact and never conformance
(constraint 23). No other count moves: mapped stays 46/50, sanitized-native
stays 10 of 46, no unit or function status, no corpus number. **The theorem
bound** is wave 31's `QCore.WriteLendTasks.no_byte_has_two_write_authorities`,
instantiated on CT07's own two-thread ledger state; **the binding** goes
through the checked `ScopedTasks` module's **lease set** — a proved replay of
the lease opens and closes the reference execution performs is the
`Reachable` witness — and **not** through the F02 trace relation, which
refuses this run by constraint 15n's decided boundary and was not touched.

## 1. Scope and what is true today (re-derived at `e878b362`, not inherited)

* **CT07's artifact** is `compiler/tests/fixtures/ct/CT07/case.qcore` since
  `CT07-TWO-THREAD-REMAP`/`CT-LANE-5` (wave 37): one allocation whose payload is
  a single `int32_t`, two children `thrd_create`'d with range write leases over
  `[0, 2)` and `[2, 4)` (`lend buffer 0..2`, `lend buffer 2..4`), each writing
  its relative byte 0 through its own lease, the owner reading both bytes back
  after both joins. Its manifest row (`docs/reevaluation/ct_manifest.json`)
  carried `"proof": { "status": "unassessed" }` and its semantic profile named
  `CT07-TWO-THREAD-PROOF-LEG` as the residual.
* **Two theorems are about the mechanism** and neither was bound to the
  artifact: `QCore.WriteLendTasks.no_byte_has_two_write_authorities`
  (`proof/QCore/WriteLendTasks.lean`, wave 31) — for every `Reachable` ledger
  state and every byte, `writeAuthorities s b ≤ 1` — and
  `QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties`
  (`proof/QCore/WriteLendTasksExercise.lean`, wave 36) — in a `Reachable`
  state two admitted writes to one byte are one writer's.
* **The CT01/CT37/CT03 pattern is unavailable by decision.** Each existing leg
  (`compiler/tests/ct0{1,3}_proof_leg.rs`, `ct37_proof_leg.rs`) binds through
  `AdmittedCausalOrder::derive` → `reference_trace`
  (`crates/qcore-core/src/task_trace_relation.rs`), which returns
  `Err(UnsupportedObservedEvent)` for `BufferLeased`/`BufferLeaseReturned`
  (`:563-565`) and `BufferByteWritten`/`BufferByteRead` (`:551-553`); CT07's
  run carries both kinds. Constraint 15n as amended by
  `WRITE-LEND-TASKS-EXERCISE` records that as **decided** (the alphabet does
  not grow). The leg re-measures this rather than inheriting it: its last
  assertion is that `derive` refuses CT07's own observation (§3, §8).
* **The residual's discharge as amended at the wave-38 merge** (verified in
  `named_followups` at base): "a proof-leg card binding CT07's checked
  `ScopedTasks` module's lease set to the Lean `Reachable` term of
  `QCore.WriteLendTasks` — a new binding pattern … with its own non-vacuity
  method, plus a `PROOF_GATED_CASES` entry. That would move the proof
  numerator from 3 to 4, which is evidence about one case's artifact and never
  conformance." No "unusually cheap" remains in the entry.
* `compiler/tests/write_lend.rs` no longer reads CT07's fixture (wave 37);
  `proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`
  (wave 38) requires the credited leg to `include_str!` the case's own fixture.

## 2. The theorem bound, and the artifact-side witness

**Primary binding — `no_byte_has_two_write_authorities` (wave 31).** The
theorem's hypothesis is `Reachable s`, an inductive relation over lease lists:
the empty ledger, an admitted `openLease`, a `closeLease`. The new module
`proof/QCore/Ct07LeaseLedger.lean` (imports `QCore.WriteLendTasksExercise`,
hence `QCore.WriteLendTasks` and `QCore.SlicePermissions`; every wave-31 and
wave-36 definition is used, none restated) adds:

| Lean | What it is |
|---|---|
| `Step` (`lease holder range` / `reclaim holder`), `step`, `replayFrom`, `replay` | A ledger transition as the reference execution performs it (`TaskEvent::BufferLeased` / `BufferLeaseReturned`), and the total-or-nothing fold from the empty ledger through the **unedited** `openLease`/`closeLease` |
| `step_reaches`, `replayFrom_reaches`, **`replay_reaches`** | **The equivalence the runner rests on**: `replay steps = some t → Reachable t`. Any Rust transcription of `replay` that returns `Some` has produced a witness for the theorem's hypothesis |
| `ct07Steps = [lease 1 [0,2), lease 2 [2,4), reclaim 1, reclaim 2]` | The artifact's steps, holders the children's own task identities, in the order the run performs them |
| `ct07Ledger = [{2, [2,4)}, {1, [0,2)}]` | The state after both opens — the two-thread point (`openLease` conses) |
| `ct07_both_leases_are_admitted : replay (ct07Steps.take 2) = some ct07Ledger` (`decide`) | The state the artifact reaches is this term |
| `ct07_the_run_returns_every_lease : replay ct07Steps = some []` (`decide`) | After both joins every lease is back; owner authority reconstructed exactly there |
| `ct07_ledger_is_reachable : Reachable ct07Ledger` | **The artifact-side witness**, by `replay_reaches` |
| **`ct07_no_byte_has_two_write_authorities (b) : writeAuthorities ct07Ledger b ≤ 1`** | **The bound theorem, instantiated on CT07's own state** |
| `ct07_ledger_digest : mixLedger ct07Ledger = 8876998389998880787` (`decide`) | The transcription pin (F02's fold constants over `encodeLease`, `% 2^64`) the Rust runner recomputes with wrapping `u64` arithmetic |
| `dischargesDisjointHalves`, `ct07_discharges_disjoint_halves`, `a_discharged_ledger_holds_every_byte_exactly_once`, `a_discharged_ledger_has_two_distinct_holders` | **The scenario shape**: exactly two leases, two distinct holders, every payload byte under **exactly one** write authority — the theorem's own `writeAuthorities` met with equality where the theorem proves `≤ 1` — and what a discharged state means (each byte held by exactly one lease; two leases with different holders) |

**Second theorem, instantiated on the same state — `no_byte_is_written_by_two_parties`
(wave 36).** `ct07Writes = [{writer 1, byte 0}, {writer 2, byte 2}]`,
`ct07_writes_are_admitted : ct07Writes.all (admitsWrite ct07Ledger)` (`decide`),
and `ct07_no_byte_is_written_by_two_parties` — two admitted writes to one byte
of `ct07Ledger` are the same writer's. This is a consequence checked on top of
the lease binding, not a second binding route: the hypothesis is still the
same `Reachable ct07Ledger`.

**The `Reachable` term ↔ ledger state correspondence**, precisely: the Lean
term is a `List Lease`; the ledger state is `ResourceState::WriteLeased {
leases: Vec<(ShareHolder, LeaseRange)> }` in `crates/qcore-core/src/scoped_tasks.rs`.
The runner reads the latter back through the public `lease_holders` /
`held_lease` after driving the production ledger through the same steps
(`reserve_with_transfers` → `commit` per open; `complete` → `join` per close),
and requires the `(holder, range)` set to equal the transcribed state after
**every** step; and it requires `LeaseRange::overlaps` — which calls
`ownership::axes_overlap` — to agree with the transcribed `overlaps` on the
artifact's ranges in both orders. Lean imports the same `overlaps` from
`QCore.SlicePermissions` rather than restating it, so the disjointness decision
is one rule in three places, checked against each other on this artifact.

## 3. The binding test — `compiler/tests/ct07_proof_leg.rs`

Host-unconditional (no `cfg`, no Clang, no native half). The credited executor
`ct07_artifact_is_the_lean_proved_disjoint_halves_shape`:

1. `compile_files` on `include_str!("fixtures/ct/CT07/case.qcore")`;
   `checked.as_scoped_tasks()`; reads every `ScalarOperand::Lease { low, high }`
   of every `TrySpawn` in the entry function out of the checked task IR — the
   two halves, each handed to `writer` — and requires the reference run to open
   exactly those ranges.
2. `execute_task_module` (the reference execution); transcribes
   `BufferLeased`/`BufferLeaseReturned` into `Step`s and requires them to be
   exactly `ct07Steps`.
3. The clause-for-clause transcription (`admits_lease`, `open_lease`,
   `close_lease`, `leases_holding`, `write_authorities`, `admits_write`,
   `step`, `replay_from`, `replay`, `distinct_holders`,
   `discharges_disjoint_halves`, `encode_lease`, `mix_ledger`) replays the
   first two steps to `ct07_ledger()` and all four to `[]` — the `Some` that
   `replay_reaches` turns into the `Reachable` witness.
4. `mix_ledger(state) == 8876998389998880787`, the kernel-decided pin.
5. `discharges_disjoint_halves(state)`, and `write_authorities(state, b) == 1`
   for every `b < 4`.
6. The production `ScopedTaskLedger` agreement after every step, and the
   overlap-decision agreement (§2).
7. The artifact's byte writes are `ct07_writes()`, each admitted against the
   state the replay has reached **at the write** and against the two-thread
   state; two writes to one byte would be one writer's (vacuous on this
   artifact, stated as the theorem's conclusion).
8. **The premise pin**: `AdmittedCausalOrder::derive(&observation).is_err()` —
   `CT07-PROOF-LEG-PATTERN-UNAVAILABLE` re-measured on the artifact itself.

A second test, `the_procedure_decides_the_lean_named_mutations_for_the_theorems_reason`,
runs the transcription on the three mutants' would-be steps with no checker in
between and requires each decided at the clause the Lean module names. Three
further tests measure where the pipeline catches each mutated **fixture** (§4).
Result at every run: `5 passed; 0 failed` (`measure/ct07-run3.log`, exit 0).

What is **not** established: anything about the emitted C's native execution
(that stays `run_native_heap_scenarios`' sanitized observation, with no
ThreadSanitizer evidence — `TASK-ROUTE-TSAN-SEGV`); a verified extraction
between Lean and Rust (the agreement is the checked-digest transcription
pattern F02 established, here over the ledger); anything about the native
run's interleaving (the theorem is about every reachable state; the binding is
about the states this artifact's reference execution reaches). The payload is
one `int32_t` (`WRITE-LEND-TASKS-EXTENT`, unchanged).

## 4. Non-vacuity: the three fixture mutations, with their catch points separated

The brief asked that the binding go red **for the reason the theorem gives,
not because the checker refused earlier**, and that the two be separated by
measurement. Each mutation is therefore measured twice: (a) where the pipeline
catches the mutated fixture — read off the panic line of the credited executor
when the fixture on disk is mutated (`mutations.log`, F1–F3) — and (b) what the
transcribed procedure says when fed the mutant's would-be lease steps directly
(the Rust test `the_procedure_decides_…` and the Lean theorems).

| Mutation | (a) pipeline catch point, measured | (b) the procedure's reason, measured |
|---|---|---|
| **F1** overlapping halves: `lend buffer 0..2` → `0..3` beside `2..4` | **The checker**, before any artifact exists: `ct07_proof_leg.rs:316` (the `compile_files` refusal inside `artifact_of`), `Q600`, the ledger's own `OverlappingLeases { resource: 1, requested: [2,4), held: [0,3) }` | `replay` = `None` at the second open; `overlaps([0,3), [2,4)) = true`, the nonemptiness and holder-bound conjuncts true — **the disjointness clause** (`overlapping_halves_are_refused`, `the_overlap_is_the_disjointness_clause`) |
| **F2** empty half: `lend buffer 2..4` → `2..2` | **The checker**, before the ledger is asked: `:316`, `Q600`, "a write lend must cover at least one byte: [2, 2) is empty" (the capture's own check) | `replay` = `None`; `overlaps` false, holder bound satisfied, `low < high` false — **the nonemptiness clause** (`an_empty_half_is_refused`, `the_empty_half_is_the_nonemptiness_clause` = `an_empty_range_is_not_a_lease` instantiated) |
| **F3** single writer: the inner `match spawn s writer(lend buffer 2..4) { … }` replaced by `let a = join move first?; let byte0 = …; drop buffer; return byte0;` | **The binding** — the checker **admits** it: `:499`, `assert_eq!(artifact.steps, ct07_steps())`, the run's steps being `[lease 1 [0,2), reclaim 1]` | `replay` = `Some([{1, [0,2)}])` — **reachable, so the theorem holds of it** (`write_authorities ≤ 1` on every byte) — and **not the scenario**: `dischargesDisjointHalves = false`, `writeAuthorities … 2 = 0`, `mix_ledger ≠` the pin (`a_single_writer_is_reachable_but_not_the_scenario`, `a_single_writer_leaves_the_other_half_unheld`) |

So the two refusing mutations never reach the binding — that is the checker
doing its job, and the procedure agrees with it clause for clause — while the
admitted mutation is exactly the one the binding must catch, and the reason it
gives is the theorem's own quantity failing the scenario's equality rather than
its bound.

**Rule mutations of the transcription** (R1–R6, each editing
`ct07_proof_leg.rs` in place, each reddening the named test, each restored):
R1 drop the overlap conjunct of `admits_lease` → the overlap tests red; R2
`low < high` → `low <= high` → the empty tests red; R3 the digest pin `+1` →
the executor red at `:532`; R4 the shape's `== 1` → `<= 1` (the theorem's
bound) → the single-writer tests red — **the shape is strictly stronger than
the theorem**, measured; R5 holders 1 → 3 → the executor red at `:521` (holders
are compared, not assumed); R6 skip the production ledger's `commit` → the
executor red (the ledger's lease set is read back and compared).

**Lean mutations** (L1–L3, each editing `Ct07LeaseLedger.lean`, `lake build`):
L1 `rightHalf` `[2,4)` → `[2,3)` → `decide` fails at `:209` (the digest) and
`:228` (the shape); L2 the digest constant `+1` → `decide` fails; L3 (edit the
imported `admitsLease`) is **refused by the harness** — the site is not in this
module, which is the point: the admission is imported, not restated, so the
Rust R1 is the transcription-side check of that clause.

Baseline green, every mutation red for its stated reason, L3 refused as
designed, restore **sha256 EQUAL**, cargo and lake green after restore, harness
exit 0 — `mutations.log`, reproducible with
`python3 docs/reports/ct07-two-thread-proof-leg/scripts/mutate_ct07_proof_leg.py`.

## 5. `PROOF_GATED_CASES`, the manifest and the stage-evidence record

The diff to `compiler/tests/ct_corpus.rs` (the pinned fail-closed table):

```diff
     (
         "CT03",
         "compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape",
     ),
+    // Card `CT07-TWO-THREAD-PROOF-LEG` (wave 39): CT07's disjointness
+    // procedure over the scoped-task ledger's lease set
+    // (QCore.Ct07LeaseLedger over QCore.WriteLendTasks,
+    // AuditCt07LeaseLedger.lean). The first leg bound **through the ledger**
+    // rather than the F02 trace relation, which refuses this run's lease
+    // events by constraint 15n's decided boundary; the alphabet did not grow.
+    (
+        "CT07",
+        "compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape",
+    ),
 ];
```

Landed in lockstep, each a fail-closed check of the others: the same string in
`RUNNER_REGISTRY`; the manifest's CT07 `proof` row → `executed` with that
executor, the case's own artifact, 1 observation; `expected.json`'s
`stages_executed` += `"proof"` (the harness refuses drift between the fixture,
the manifest and its derived plan); `docs/reviews/r0_3/acceptance.json`'s
generated fields rewritten by `scripts/ct_stage_summary.py --write` (proof
executed 3 → 4, CT07's `case_depth` `native_sanitized` → `proof`) and its
`derived_from` pin extended; both `EVIDENCE_FILES` copies
(`scripts/ct_stage_summary.py`, `ct_corpus.rs` `[&str; 24]`) extended with
`docs/reports/ct07-two-thread-proof-leg/ct_stage_evidence.json`, which declares
`"supersedes": "docs/reports/ct-lane-5/ct_stage_evidence.json"` and carries
every other CT07 stage row from that record **unchanged** (constraint 21 — the
wave-37 record keeps its bytes). The CT reasons map was **not** regenerated by
hand; `build_matrix.py --gate` regenerates the derived sections (§9).
`ct_corpus` green: `6 passed` (`measure/ct_corpus-2.log`), including
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture` over the
new row; `ct_stage_summary.py --check` exit 0.

**Lean audit**: `proof/AuditCt07LeaseLedger.lean`, registered by
`import QCore.Ct07LeaseLedger` in `proof/QCore.lean`; 24 `#print axioms`
declarations, every one `[propext]` or `[propext, Quot.sound]`
(`measure/audit-1.log`, exit 0). Lean audits **137 → 138** — the count moves
because this card adds an executable-term audit, not because any semantics
moved: no Rust production code changed, no checker rule, no emission.

## 6. Shared readers

`CT-FIXTURE-EXTERNAL-READERS` records eight CT directories read by
`include_str!` from outside `ct_corpus.rs`. This leg adds CT07 back to that set
**by design** — `compiler/tests/ct07_proof_leg.rs` reads
`fixtures/ct/CT07/case.qcore`, which is exactly what
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture` requires
of a credited leg (run green, §5). Measured at this tree
(`grep -rn "fixtures/ct/CT07" compiler/tests/*.rs`, `ct_corpus.rs` excluded):
CT07's **`case.qcore`** is read from outside the CT harness by
`ct07_proof_leg.rs` only; `write_lend.rs` reads two of CT07's *control*
fixtures (`sequential_write_halves.qcore`, `overlapping_lends.qcore`) and not
the case artifact — the wave-37 repair, still in place. So a lane re-pointing
CT07's case now moves a proof leg's artifact, which is exactly the situation
the wave-38 tripwire exists to catch. The residual itself (a declared-or-refused
check over every external `include_str!`) is a close-rule item and is not
closed here.

## 7. Keyword census and diagnostics

No new word, no new token, no lexer change: **+0**, and there was nothing to
census — the card adds a test, a Lean module and records. No `Q` code is
allocated and no condition is widened; `docs/QCORE_SPEC_r0_3.md` §L21 is
untouched. The `Q600` refusals the mutations meet are the existing
`OverlappingLeases` and empty-lease conditions, reached unchanged.

## 8. Residuals and retirements

**Retired** (each verified registered in `named_followups` at base, then
compared with its discharge word for word):

* `CT07-TWO-THREAD-PROOF-LEG` (recorded by `CT-LANE-5`). Discharge: "a
  proof-leg card binding CT07's checked `ScopedTasks` module's lease set to the
  Lean `Reachable` term of `QCore.WriteLendTasks` — a new binding pattern …
  with its own non-vacuity method, plus a `PROOF_GATED_CASES` entry. That
  would move the proof numerator from 3 to 4, which is evidence about one
  case's artifact and never conformance." Met: §2 (`replay_reaches`,
  `ct07_ledger_is_reachable`), §4, §5, and the numerator sentence in those
  words.
* `CT07-PROOF-LEG-PATTERN-UNAVAILABLE` (recorded by `STABILITY-REEVAL-3`).
  Discharge: "amend `CT07-TWO-THREAD-PROOF-LEG`'s discharge to a ledger-side
  binding with its own non-vacuity method, and drop 'unusually cheap'; the
  entry stays open." Met: the amendment was applied at the wave-38 merge (the
  entry at base carries the ledger-side discharge and no "unusually cheap"),
  the entry stayed open until this card landed that binding, and the
  unavailability is re-measured by the leg's own premise pin.

**Registered** (`recorded_by: CT07-TWO-THREAD-PROOF-LEG`):

* `CT07-PROOF-LEG-F02-PREMISE-PIN` — the leg's last assertion pins that
  `AdmittedCausalOrder::derive` refuses CT07's run, so a card that grows the
  F02 alphabet by CT03's method turns the leg red **by design**. Nothing in the
  ledger binding depends on the refusal; the pin exists so the change is
  visible at the leg. Discharge: that card removes the one assertion
  deliberately in the same change, records whether CT07's proof stage stays on
  the ledger binding or is additionally derived through the trace, and states
  the choice in constraint 15n's amendment.

`named_followups` **228 → 227** (+1 −2). Not registered, because it is a
standing boundary already recorded rather than a new one: the Lean ↔ Rust
transcription is unverified extraction (the F02 legs' pattern); the native
run's interleaving is observation (`run_native_heap_scenarios`).

## 9. Gate

One detached full standing gate (`measure/gate.sh`: `setsid -f`, no
`trap '' TERM`, every exit to its own file, no pipe), run at tip **`81f783ce`**
— the commit carrying every code change of this card; the commits after it are
documentation and the matrix regeneration only. Exits and the short logs are
copied into `gate/` beside this file; the facts below were extracted from them
by `measure/build_gate_json.py` (`measure/gate-facts.json`), nothing typed in.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 (`measure/submodule.log`, `d82464a2`) |
| `cargo test --workspace --no-fail-fast` | **exit 0 — 4,644 passed / 0 failed / 9 ignored** over 235 result summaries and 233 test targets (224 `Running` + 9 doc-test sections); `test result: FAILED` **0** times, so no constraint-16 isolated re-verification was needed and none is claimed; 18m16s (07:22:35Z → 07:40:51Z) with four sibling worktrees building; +5 against the wave-38 close's 4,639 (this leg's five tests), ignored +0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `scripts/check_lean_audits.py` | exit 0, **"Passed 138 Lean audits"** (137 + `AuditCt07LeaseLedger.lean`) |
| `python3 -m unittest discover -s scripts/tests` (detached run) | **exit 1, deliberately reported**: 303 ran, 2 failures, both regeneration tripwires against the tree with `named_followups` 228 → 227 and `Audit*.lean` 137 → 138 not yet regenerated — `test_each_derived_dimension_matches_the_section_it_counts` (`228 != 227`) and `test_the_derived_facts_are_recomputed_rather_than_carried` (`['14.0.0', 137] != ['14.0.0', 138]`); the wave-36/37/38 closes' shape |
| matrix regeneration | `build_matrix.py . --gate docs/reports/ct07-two-thread-proof-leg/gate.json` exit 0: named followups 227, lean audits 138, contract modules 88; the CT section's derived rows moved exactly as §5 describes (proof executed 4, CT07 `case_depth` → `proof`, `stage_evidence_record` → this card's) |
| Python **post**-regeneration | `gate/python-post.exit` **0**, `Ran 303 tests … OK`; `ct_stage_summary.py --check` exit 0 |
| `clang --version` | exit 0, `Ubuntu clang version 21.1.8 (6ubuntu1)` (`gate/clang-version.txt`), `QCORE_CLANG` unset |
| disk | 280 GB → 207 GB free across the run |
| provider 122 | **carried, not re-measured**: `git diff --name-only e878b362 HEAD -- providers/ crates/qcore-schemas/` is empty, `qcore-schemas` 14.0.0 (constraint 18 not engaged); a Unix figure |

`gate.json` carries the nine standard keys plus `lean_audits` (138) and `clang`
(the wave-38 close's shape). A gate figure is a measurement on one machine,
never a property; this one was taken under sibling load and is offered as the
card's own figure, to be replaced by the wave-39 close's integration-tip
measurement.

## 10. Files touched, and sibling ownership

Owned by this card per the wave-39 brief: `compiler/tests/ct_corpus.rs`
(`PROOF_GATED_CASES`, `RUNNER_REGISTRY`, `EVIDENCE_FILES`) and
`compiler/tests/ct07_proof_leg.rs`. Also touched, none owned by another wave-39
card: `proof/QCore/Ct07LeaseLedger.lean`, `proof/AuditCt07LeaseLedger.lean`,
`proof/QCore.lean` (one import), `compiler/tests/fixtures/ct/CT07/expected.json`
(`stages_executed`), `docs/reevaluation/ct_manifest.json` (CT07's proof row),
`docs/reviews/r0_3/acceptance.json` (generated fields, `derived_from`),
`scripts/ct_stage_summary.py` (`EVIDENCE_FILES`),
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`; derived
sections regenerated with `--gate`), `AGENTS.md` (headline numbers; constraint
15n appended), `docs/development/increments/{README.md,wave-39.md}`. The files
`CT42-CELL-ROUTE`, `AGGREGATE-OWNED-FIELD`, `TSAN-ROWS-2` and
`LOOPS-ON-COMPOSED-ROUTES` own are untouched. The integrator unions the
wave-39 record paragraphs and the matrix's `named_followups` at the merge.

## 11. For the integrator

1. **Headline numbers, edited in this tree**: AGENTS.md CT row `proof 3 of 46`
   → **`proof 4 of 46`**, with CT07 added to *The proof leg* sentence and the
   "each … through the F02 digest relation" sentence qualified to the first
   three (CT07's is bound through the ledger's own admission); `named_followups`
   **228 → 227**; Lean audits **137 → 138**; the merged-gate row is this card's
   `gate.json` figures (§9) — a per-card gate, to be replaced by the wave-39
   close's integration-tip measurement.
2. **Constraint 15n, appended in this tree** (amends nothing above it), quoted:
   "**Appended, card `CT07-TWO-THREAD-PROOF-LEG` (wave 39; it amends nothing
   above).** The refusal is now also what a proof leg rests on: CT07's `proof`
   stage is bound **through the scoped-task ledger's lease set** —
   `QCore.Ct07LeaseLedger.replay_reaches` turns the reference execution's lease
   opens and closes into a `Reachable` witness for wave 31's
   `no_byte_has_two_write_authorities` — precisely because the trace route is
   closed, and the leg's last assertion **pins** that closure
   (`AdmittedCausalOrder::derive` refuses CT07's own observation;
   `CT07-PROOF-LEG-F02-PREMISE-PIN`). A card that re-opens this boundary by
   CT03's method therefore removes that one assertion **deliberately in the
   same change**, states whether CT07's proof stage stays on the ledger binding
   or is additionally derived through the trace, and records the choice here —
   the leg must not be left to go red as a side effect, and the ledger binding
   must not be dropped on the ground that a trace route now exists: the two
   bind different hypotheses (a state the ledger admits; an order the alphabet
   admits), and neither implies the other." Pointer list gains
   `ct07-two-thread-proof-leg`.
3. **Matrix**: `named_followups` carried with the two retirements and one
   registration above; `dimensions` regenerated with
   `--gate docs/reports/ct07-two-thread-proof-leg/gate.json` (§9). At the
   union with sibling cards' `named_followups` edits, the count line in
   AGENTS.md must be re-summed.
4. **Records**: `docs/development/increments/wave-39.md` created in wave-38's
   shape with this card's paragraph; the index's pointer and table row moved to
   wave 39. Sibling paragraphs union at the merge.
5. **The CT reasons map** was not regenerated by hand; the manifest's CT07 row,
   the fixture's stage list and the acceptance record's generated fields moved
   by the generator and are checked in both directions by `ct_corpus` and
   `ct_stage_summary.py --check`.
6. **Decision the integrator may want to take, not taken here**: whether the
   `ct_corpus.rs` doc comment above `ct_mapping_progress_is_pinned_and_counted`
   — prose that has read "proof is 3 of 43" since wave 32 and which that
   comment itself records as having drifted twice — should be refreshed or
   deleted; it is prose no test asserts, and this card left it as the record
   it says it is.
7. **Not claimed**: any native, sanitizer, Windows or Apple observation for
   this card (none was run beyond the standing gate's own workspace tests);
   any conformance; any change to the F02 alphabet, `reference_trace`, the
   checker or the emitter.

---

## Published beside by `WAVE-40-CLOSE` (2026-09-19) — the merge's integration figure, and its in-history sha

Appended, not edited: no line above this heading is changed, and this note
archives no new run of this card's own gate (constraint 21).

`INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE` (measured by
`STABILITY-REEVAL-4`, wave 40) recorded that this card's merge states an
integration figure differing from this card's own `gate.json` and publishes no
`integration-gate.json`, so that figure lived only in a commit message. The
wave-40 close took the residual's **second** discharge arm — state the rule and
apply it retrospectively by a dated appended note — and the rule is now in
`CLAUDE.md`: a merge whose stated figure differs from the card's own gate owes
an `integration-gate.json`; one whose figure equals it does not.

| | |
|---|---|
| Merge, **in history** | `0d9e922f` |
| Tip the merge message names as measured | `f1fcd14d` — **a non-ancestor of `main`** (`git merge-base --is-ancestor f1fcd14d <main>` exits 1), the pre-amendment commit whose successor is the merge above |
| Figure the merge message states | 4,620 passed / 0 failed / 9 ignored, 138 Lean audits, 303 Python (detached run `gate-ct07proof`, under sibling load; the line then names three failed constraint-16 rows re-verified in isolation 12/12 and 12/12 green) |
| This card's own `gate.json` | 4,644 workspace tests, 138 Lean audits |
| Why they differ | the merged tree carries a sibling card's tests; the merge message's own line is the only place the integration figure was written |

Two boundaries travel with this note. The merge line's "0 failed" is itself
inaccurate while naming three failed constraint-16 rows — that is
`GATE-SUMMARY-PARSER-OK-ONLY`, a separate registered residual about the
integrator's summary parser, and it is **not** repaired here; the figures above
are transcribed from the merge message as written, not re-measured. And no
retrospective `integration-gate.json` is written for this merge: the run's log
is not in the tree, so a file would assert a measurement nobody can reproduce.
The note preserves the citation and its successor sha, which is what the
residual's arm asks for.
