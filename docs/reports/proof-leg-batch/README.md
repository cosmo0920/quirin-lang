# Two proof rows withdrawn by measurement: CT12 and CT14 are CT01's artifact (PROOF-LEG-BATCH)

Wave 32, card `PROOF-LEG-BATCH`. The card set out to grow the proof axis
**3 of 43 → 5 of 43** the cheapest honest way: bind CT01's *already-proved*
decision procedure (`QCore.Ct01OwnedTransfer.dischargesOwnedTransfer`, card
`CT01-PROOF-LEG`) to CT12's and CT14's own artifacts through named per-case
workspace tests, writing no new Lean.

**Both rows are withdrawn.** The premise was measured rather than assumed, and
it holds — but for a stronger reason than the survey that stated it had
recorded, and that stronger reason is what withdraws the rows:
`compiler/tests/fixtures/ct/{CT01,CT12,CT14}/case.qcore` are **one
byte-identical file**. CT12 and CT14 do not have artifacts that *resemble*
CT01's; they have CT01's artifact under two more names. The proof axis stays
**3 of 43**.

What the card delivers instead is the measurement, pinned: one new
host-unconditional test file, `compiler/tests/ct12_ct14_proof_reach.rs`, whose
two tests hold the withdrawal's ground and reopen the question automatically
the day either half of it changes.

Base: `main` at `1c7d8e45` (the wave-31 close plus the `STABILITY-REEVAL`
cadence commit).

**Nothing semantic changed.** `git diff` over `crates/`, `compiler/src/` and
`proof/` is empty. No Lean module, no audit, no diagnostic code, no reserved
word, no route change, no pinned expectation moved. The additions are one
workspace test file, this report, one increment record and the increments
index pointer. `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json`, the acceptance matrix,
`compiler/tests/ct_corpus.rs` and every `EVIDENCE_FILES` list are untouched —
and, the flip being withdrawn, **no integrator flip is proposed** (§5).

---

## 1. The premise, and the measurement that overtook it

`docs/reports/ct-proof-leg-2/README.md` §1 rejected CT12 and CT14 for a *new*
theorem on this ground:

> **CT12, CT14**: measured — their default-script reference traces are
> event-for-event CT01's `roundTrip`. […] A proof on their default trace would
> restate CT01's theorem under a second name — the dishonest shape the brief
> warns against.

The card's premise was that this rejection is answered by reusing CT01's
existing procedure rather than restating it: constraint 23 asks only that
"a stage may read `executed` only where a named workspace test runs it on the
case's own artifact", and a per-case runner on each case's own fixture path
satisfies that literally.

The measurement was run first. It found the reason for the trace coincidence,
and the reason is not a coincidence.

**Measurement 1 — the fixture census.** Every `case.qcore` in the corpus,
hashed (`measure/proof-leg-batch-fixture-hashes.txt`):

```
131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c  compiler/tests/fixtures/ct/CT01/case.qcore
131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c  compiler/tests/fixtures/ct/CT12/case.qcore
131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c  compiler/tests/fixtures/ct/CT14/case.qcore
```

Forty case fixtures, thirty-eight distinct hashes: **this is the corpus's only
duplicate**, and it is a triple. CT01, CT12 and CT14 are three manifest rows
over one source file.

**Measurement 2 — the derived reference runs** (probe log
`measure/proof-leg-batch-survey.log`; the probe was a temporary test file,
deleted after the survey, on the `CT-PROOF-LEG-2` precedent). Each fixture
compiled through `compile_files`, its checked task module executed by
`execute_task_module` under the default script, the admitted causal order
derived by the production observer `AdmittedCausalOrder::derive`. All three:

```
Allocated { task: 0, buffer: 1, payload: 42 }
Spawned   { child: 1, worker: 0 }
Completed { task: 1, outcome: Buffer(1) }
Joined    { child: 1 }
Read      { task: 0, buffer: 1, value: 42 }
Dropped   { task: 0, buffer: 1 }
Completed { task: 0, outcome: I32(42) }
len=7  digest=13023433800416048247  declare=ok  relate=true
```

Identical to each other, identical to the recorded F02 trace fixture
`crates/qcore-core/tests/fixtures/f02/round_trip.trace` that
`QCore.TaskTraceRelation.roundTrip` transcribes, and carrying
`13023433800416048247` — which is
`QCore.Ct01OwnedTransfer.ct01_trace_digest`, the kernel-proved pin over CT01's
own term, recomputed here from CT12's and CT14's fixtures.

**Domain membership, the question the card was told to answer.** Both traces
are in `dischargesOwnedTransfer`'s domain, and the answer to "how does each
differ from CT01's, if at all" is: **not in any event, and not by
coincidence** — it is the same trace term, because it is the same program.
There is one trace term here, therefore one theorem, therefore one proof.

## 2. Why that withdraws the rows rather than licensing them

The proof stage, as all three landed proof legs define it, is a function of
**the fixture's bytes alone**: compile, reference-execute under the default
script, derive the admitted causal order, run the decision procedure. No
harness regime, verdict mode or scripted arm enters it. So for CT12 and CT14:

1. **The rows would carry no new proved content.** Not "little" — none. No new
   theorem, no new trace term, no new digest, no new property, no new refusal.
   Two more test names running the same kernel `decide` over the same seven
   events.
2. **Constraint 23's gate would be satisfied vacuously.** "A named test runs it
   on the case's own artifact" is a necessary condition written to stop paper
   claims; here it cannot distinguish CT12 from CT01 at all, because CT12's own
   artifact *is* CT01's. Passing a gate that cannot see the distinction is not
   evidence that the distinction exists.
3. **Invariant 2 identifies a published result by `(artifact, property, …)`,
   not by path.** Two rows naming different paths over identical bytes with an
   identical property are one result published twice. `proof 5 of 43` would
   report five rows over **three** distinct artifacts.
4. **It is the shape `CT-PROOF-LEG-2` rejected, strengthened.** That survey
   said such a proof "would restate CT01's theorem under a second name". The
   byte-identity measurement adds that the *artifact* would be under a second
   name too. Reusing the procedure instead of restating it removes the
   duplicated Lean; it does not add the missing evidence.

**The strongest case for landing them anyway, and why it still fails.** The run
is not *nothing* for these cases: CT12's profile names "a successful-create
control [that] transfers the same pointer through child return and creator
join", CT14's names an "accepted control [that] joins, consumes the owned
Result and drops the original generation", and the seven-event run above is
exactly that control in both. `dischargesOwnedTransfer` decides precisely its
property — join before drop, exactly-once drop, generation-preserving return.
So a row reading *"proof — the admitting control's property, the distinguishing
arm unproved"* would be a true sentence. It still fails, on the number rather
than the sentence: the row would land in a pinned denominator whose numerator
is published as `proof N of 43` and read case by case, and no per-row note
travels with that number into the ledger. A card may not unilaterally give a
pinned denominator a second, weaker row semantics; if the project wants a
control-arm proof tier, that is a manifest and stage-vocabulary decision taken
deliberately, with the denominator's meaning restated — not a side effect of
two test files.

The project's own standard across the three landed legs points the same way:
CT01's theorem is its scenario; `CT-PROOF-LEG-2` chose CT37 specifically
because "the default run itself exhibits the scenario"; `CT03-PROOF-ALPHABET`
grew the alphabet so CT03's scenario would be exhibited. A CT12 row whose
theorem is silent about creation failure would be the first proof row that
does not touch the case it is filed under.

## 3. What each case's own scenario would need — measured, not asserted

Neither case's distinguishing scenario is reachable through this relation
today, and the reason is a measured refusal rather than an omission.

CT12's scenario is *Create fails before any child starts*; its manifest profile
records an **injected native creation failure**, and its native stage carries
two observations (the failure arm plus a successful-create control). The one
scripted arm that would put a creation failure into a *reference* run is
`TaskExecutionScript { spawn_outcomes: vec![false], .. }`. Measured, for both
fixtures:

```
--- CT12 create-failure script
    REFUSED: derive refused: Violation(UnattributableEvent { index: 1 })
--- CT14 create-failure script
    REFUSED: derive refused: Violation(UnattributableEvent { index: 1 })
```

The run derives a `CreationFailed` event, whose `actor()` is `None`: the
variant carries the *worker function* that was never created, not a task
identity, so there is no actor to attribute the line to — and
`AdmittedCausalOrder` refuses an unattributable line rather than guessing one.
**No declared order can contain the event CT12 is about**, so there is nothing
for a procedure to decide. This is the same fact `CT-PROOF-LEG-2` recorded,
now re-measured directly.

CT14's scenario is *Child finishes but parent has not joined*, and its
distinguishing content is a property of the **native adapter** (which forces
the worker body and completion-record write before creation returns) together
with `Q600` rejection controls — a checker refusal and a native ordering, not a
trace the relation carries. Its accepted control is precisely the run above.

One adjacent measurement, recorded so it is not mistaken for a route:
CT12 under `allocation_outcomes: vec![false]` does derive, to a two-event trace
`[AllocationFailed { task: 0 }, Completed { task: 0, outcome: Error("AllocError") }]`.
It is not a candidate — the Lean `Outcome` deliberately omits typed errors
(`TaskTraceRelation.lean`'s own alphabet note), and an allocation failure is
CT02's scenario, not CT12's.

## 4. What the card delivers: the withdrawal, pinned

`compiler/tests/ct12_ct14_proof_reach.rs` — two host-unconditional tests, no
`cfg`, no Clang, no `host_gate`. They add **no proof-stage evidence for any
case**; they hold the ground of a withdrawal so the next agent inherits a
measurement rather than a sentence.

* **`ct12_and_ct14_are_ct01s_artifact_under_two_more_names`** — identity at
  three levels: the fixtures' bytes, each case's derived reference run against
  the recorded F02 trace event for event, and the Lean-proved digest
  `13023433800416048247` recomputed from each case's own artifact. The digest
  clause is what makes the point exact: identity of *trace terms*, not merely
  of files.
* **`the_creation_failure_arm_is_outside_the_relation`** — the creation-failure
  script's refusal, pinned by violation kind
  (`TraceDeriveError::Violation(TraceRelationViolation::UnattributableEvent)`).

**Both are tripwires in constraint 24's sense — meant to be flipped
deliberately, never silently.** Each failure mode is exactly a condition under
which the withdrawn rows genuinely reopen:

| If this test fails | Then |
|---|---|
| the fixture/trace/digest identity | one of the three fixtures diverged; CT12 or CT14 now has an artifact of its own, and its proof leg is a real question again |
| the unattributable-creation-failure pin | the F02 alphabet grew to attribute `CreationFailed` to a creator; CT12's own scenario became expressible, and CT13/CT15 with it |

The second is the good direction, and `CT03-PROOF-ALPHABET` is its precedent:
extend the alphabet deliberately, prove the extension conservative, re-pin
every digest. A card doing that should update this test, not delete it.

## 5. No integrator flip is proposed

The card's deliverable was "capability plus the proposed flip terms for each
case". With both rows withdrawn there are no flip terms to propose:
`docs/reevaluation/ct_manifest.json` keeps `"proof": {"status":
"unassessed"}` for CT12 and CT14, `PROOF_GATED_CASES` keeps its three entries,
no `ct_stage_evidence.json` is written, `EVIDENCE_FILES` stays at 18, and the
headline stays **proof 3 of 43** (CT01, CT37, CT03). `check_lean_audits.py`
stays at **125** because the card adds no audit — the first proof-family card
whose delta to that gate is zero, and `scripts/tests` is correspondingly green
rather than carrying the one expected `validation.lean_audits` failure every
audit-adding card's report has had to record.

One verification the brief asked for was performed and is recorded, because it
will be needed if the question ever reopens: **the record currently holding
CT12's and CT14's stages is card T00's frozen archive**,
`docs/reports/remaining-t00/coverage.json` — both appear in its
`mapped_case_stage_matrix` with `"proof": "unassessed"`, and a scan of all
eighteen `EVIDENCE_FILES` records finds them in no other. That archive is the
`supersedes` target a future flip would name, and it is never edited
(constraint 21) — the same footing CT37's flip stood on.

T00's own note on those two rows already drew the distinction this card
re-measured, and is worth quoting because it is the standard the withdrawal
applies: *"no Lean theorem is bound to this fixture … the task theorems
constrain the model, not this source artifact."* A theorem is bound to a
fixture by being about *that fixture's* run. CT01's is. Filed under CT12 or
CT14, the same theorem would still be about CT01's run — the identical bytes
being what makes "this source artifact" fail to pick out a case.

## 6. Proved versus observed, stated explicitly

The `STABILITY-REEVAL` discipline asks that the two be separated even when the
answer is uncomfortable. Here it is short, because this card proves nothing new:

* **Proved (by earlier cards, unchanged by this one).** CT01's scenario
  property holds of the *checked artifact's reference run and its admitted
  renderings* — by kernel `decide` for the reference trace, and by
  `every_admitted_rendering_discharges_the_owned_transfer` for every rendering
  the F02 relation admits against it. That statement is about a reference
  execution and a relation. It is not about what any binary did.
* **Observed, not proved.** Native execution of all three cases' emitted C
  remains observation only — `run_native_owned_transfer`'s byte comparison for
  CT01/CT12, `run_native_task_scenarios` for CT14, and CT14's
  `recorded_not_gated` sanitized row. The manifest profile's "native runtime
  refinement proofs remain open" sentence stands untouched.
* **Transcription, not extraction.** The Lean–Rust tie throughout this family
  is checked-digest transcription — a kernel-proved digest recomputed from the
  artifact plus event-for-event equality — not verified extraction. This card's
  new test participates in that same tie and does not strengthen it.
* **Measured, not proved, by this card.** The fixture identity, the trace
  identity and the creation-failure refusal are Rust-test observations of this
  checkout, now gate-checked. They are not Lean theorems and are not claimed as
  such.

Nothing here is an upstream defect claim. The shared fixture is deliberate
harness design — one source program exercised under three native regimes — and
it is only the *proof* stage, which has no regime, that collapses the three
rows into one.

## 7. Gate

Run at the card tip on Linux under default cargo configuration. No test, pin or
assertion was edited to obtain these results. **This wave ran no arbiter and
this card ran none**: it touches no gateway, contract, audit or provider file,
so no corpus measurement was taken and no corpus row moves.

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **224 suite summaries: 4,512 passed / 1 failed / 8 ignored.** The one failure is constraint 16's named `RLIMIT_NPROC` row — §7.1 records both facts |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | `Passed 125 Lean audits` — **unchanged**; this card adds none |
| `python3 -m unittest discover -s scripts/tests` | `Ran 200 tests … OK` — fully green, no expected failure to carry (§5) |

Raw logs: worktree-private `measure/proof-leg-batch-*.log`.

The host was under heavy external load throughout: five other worktrees were
running their own gates on the same machine, with the load average between the
mid-20s and **65** during the workspace run. That is disclosed because it is
the regime constraint 16 records, and it is what §7.1 is about.

### 7.1 The one tripped constraint-16 row, both facts recorded

`enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
— constraint 16's **first** named row, the `RLIMIT_NPROC` conformance test,
recorded there as sensitive precisely because "it counts every process of the
real UID". It failed inside the concurrent workspace run with the
`Io(Broken pipe)` shape its two listed siblings share:

```
crates/qcore-verifier-host/tests/enforced_process_limits.rs:389:
assertion `left == right` failed: Broken pipe (os error 32)
  left: Io
 right: Terminated
```

Both facts, as the constraint requires:

| Run | Host state | Result |
|---|---|---|
| inside `cargo test --workspace` | load mid-20s→**65**, five other worktrees' gates | **failed** |
| immediate isolated retry | load ~33.7 | failed again, identical shape |
| isolated re-verification | load ~27.5, **1,047** processes for the real UID | **passed** — `test result: ok. 1 passed`, same binary, nothing edited |

The row cannot be affected by this card in any case: the card's only code
change is a new test file under `compiler/tests/`, and this row lives in
`qcore-verifier-host`, which does not depend on it. `--no-fail-fast` did its
job — every suite after the failing binary still ran, so the 224/4,512 above is
a measurement rather than a lower bound (`GATE-NO-FAIL-FAST`). Raw logs:
`measure/proof-leg-batch-workspace-tests.log`,
`measure/proof-leg-batch-rlimit-isolated.log` (the failing retry) and
`measure/proof-leg-batch-rlimit-isolated-lowload.log` (the passing one, which
records the load and UID process count of its attempt).

## 8. Follow-ons

* **`CT-PROOF-CREATION-FAILURE-ALPHABET`** — the named, measured condition
  under which CT12 becomes a real proof candidate: attribute `CreationFailed`
  to a creator in the F02 alphabet, on `CT03-PROOF-ALPHABET`'s deliberate
  extension pattern (prove the extension conservative on creation-failure-free
  traces, re-pin every digest). It would reach CT13 and CT15 by the same
  token, whose scenarios `CT-PROOF-LEG-2` rejected on this identical ground.
  No claim is made here that it is worth the cost.
* **`CT-SHARED-FIXTURE-DISCLOSURE`** — the manifest presents CT01, CT12 and
  CT14 with three distinct `source_fixture` paths and three semantic profiles
  over one file, which is legitimate for the per-regime native stages and
  misleading for any stage that is a function of the bytes. A one-sentence
  disclosure in those rows (or in the stage vocabulary's note on the `proof`
  stage) would make this card's measurement visible where the rows are read.
  Integrator/spec work; this card edits no manifest.
* **`CT-PROOF-CONTROL-ARM-TIER`** — the deliberate version of what this card
  declined to do by side effect (§2): if a proof row about a case's *admitting
  control* is wanted, give the stage vocabulary a tier that says so and restate
  what the `proof N of 43` numerator counts, in the manifest rather than in a
  card report. CT12 and CT14 would be its first two rows, and this card's test
  file is the evidence already in place for them. Integrator/spec work.
* **`CT-PROOF-FOURTH-CASE`** — unchanged from `CT-PROOF-LEG-2` §9 and narrowed
  by nothing here. Within the current alphabet, CT50 remains the only mapped
  case whose default reference run both derives and exhibits its scenario, with
  that card's honesty caveat intact.
* **`CT-PROOF-NATIVE-REFINEMENT`** — unchanged; neither closed nor narrowed.
