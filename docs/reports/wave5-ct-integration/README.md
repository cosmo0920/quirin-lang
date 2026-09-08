# Wave-5 CT integration — CT28, CT31, CT32 and CT48 mapped with stage-derived evidence

Date: 2026-09-08. Base checkpoint: `1fc273a` (the merged wave-5 tree).
Source of the proposals: [card T03-F](../remaining-t03-f/README.md) §6, which
measured all four and deliberately left `docs/reevaluation/ct_manifest.json`
untouched for the integrator.
Precedents followed exactly: [the T03 CT integration](../remaining-t03-ct-integration/README.md)
and [the wave-3 CT integration](../remaining-wave3-ct-integration/README.md).

**Scope.** Four conformance cases move from `unmapped` to `mapped`. No
production Rust changed: the whole diff outside `docs/` is nine new fixture
files and three pinned-table edits in `compiler/tests/ct_corpus.rs`. No
diagnostic code was added, removed or changed; no checker, lowering, backend or
Lean file was touched. **CT coverage moves 20/50 → 24/50 mapped, 30 → 26
unmapped.** Nothing else is promoted.

---

## 1. What was mapped, and what was measured

Every vector below was measured through `compile_files` on the fixture's own
bytes before the manifest was edited, and reproduces card T03-F §6 exactly —
not one code, one position or one cascade length differs.

| Case | Scenario (spec §55) | Fixture | Measured cascade |
|---|---|---|---|
| **CT28** | Guard is moved to a child or unlocked by another thread → Reject | `compiler/tests/fixtures/ct/CT28/case.qcore` | `["Q612","Q600"]` — `WrongGuardAffinity { event: 3, guard: 1, task: 1 }` then `JoinWhileGuardHeld { event: 4, task: 0 }` |
| **CT31** | Mutex destroyed with an outstanding handle borrower/waiter → Reject | `…/CT31/case.qcore` | `["Q619","Q600","Q600","Q600"]` — `NonquiescentMutexDestruction { event: 4, mutex: 1 }`, `NonquiescentDestruction { event: 5, resource: 1 }`, `OutstandingMutex { mutex: 1 }`, `OutstandingWaiter { mutex: 1, task: 1 }` |
| **CT32** | Join while a guard is held → Reject in initial profile | `…/CT32/case.qcore` | `["Q600"]` — `JoinWhileGuardHeld { event: 4, task: 0 }` |
| **CT48** | Stale heap refinement reused after mutex release/reacquire → invalidate/re-establish, never assume | `…/CT48/case.qcore` | `["Q616"]` — `StaleProtectedFact { event: 5, fact: 2 }` |

All four are **rejection** cases on the `source.mutex.guard.v1` route. The
harness pins the *whole ordered vector*, not its head, for the reason the T03
integration recorded: a refusal that also silently released the guard or
destroyed the generation would still satisfy a head-only assertion.

### 1.1 Why each cascade has the length it has

This is the part worth reading, because cascade length is evidence, not noise.

* **CT28** — the second entry is load-bearing. `Q612` refuses the child's act;
  `Q600 JoinWhileGuardHeld` then fires because the creator *still holds* the
  guard where the child's join lands. Had the refusal quietly released the
  guard, the vector would have been one entry long.
* **CT31** — three entries follow `Q619`, and each names a different thing the
  refused destruction left alive: the protected resource (`NonquiescentDestruction`),
  then the mutex and the waiter in the end-of-trace sweep. A cascade ending at
  `Q619` would mean the model had destroyed something anyway.
* **CT32** — exactly one entry, and *that* is the observation. The child body is
  empty and well formed, so nothing else about the program is wrong; a longer
  cascade would mean the refusal had disturbed the guard or the generation.
* **CT48** — exactly one entry, because the release that dropped the fact was
  itself well formed.

### 1.2 Blockers confirmed lifted, not assumed lifted

The T03 CT integration recorded why these stayed unmapped. Each blocker was
re-read against T03-F's landed features and confirmed against the measured
behaviour above:

| Case | Recorded blocker | What lifted it |
|---|---|---|
| CT28 | "This profile refuses task creation outright, so a borrow cannot travel to a child at all here; the model-level `Q612` evidence is T02/T03 model evidence, not source evidence" | T03-F's nested `thread scope NAME { … }` is one child task created and joined by the mutex ledger. The child can now *lexically name* the creator's guard, which is what makes `Q612` reachable **from source** rather than only from hand-built events. |
| CT31 | `RegisterWaiter` existed in the checker with **no source form** | T03-F's `wait NAME;` announces an acquisition the task has not performed. Note this case's `Q619` previously appeared only as a *cascade entry* on CT30; here it is the leading diagnostic of its own case. |
| CT32 | Same as CT28 — no child task form | Same as CT28. T03-F explicitly declined to widen `JoinWhileGuardHeld`, so "created and joined strictly inside a guard" is a refusal in this profile, which is precisely what CT32 asks for. |
| CT48 | Needed guard-scoped refinement with a **non-lexical** failure; a body-local binder gives the weaker `Q013 unknown name` the T03 integration §3.1 already recorded as insufficient | T03-F's fact binders are lexical and scope-level while the fact identity is guard-scoped. The name resolves in the second section and the *model* refuses the use at `Q616`. |

---

## 2. Controls — six of them, none vacuous

Every case carries an accepted control differing from the case by one
statement, so no refusal reads as "the profile cannot express this at all".
Two cases additionally carry a refused control that isolates a *different*
mechanism from the case's own.

| Case | Control | Expectation | Measured | What it isolates |
|---|---|---|---|---|
| CT28 | `own_acquisition.qcore` | accept | accepted, `CheckedMutexGuardsV1` | The same child takes its **own** acquisition of the same generation, writes under its own guard, and the creator reads after the join. So `Q612` is keyed to acting through a guard the child does not hold — not to child tasks, and not to touching a protected footprint. |
| CT28 | `child_unlocks.qcore` | reject `["Q612","Q600"]` | identical cascade | The scenario name's **second half**: the child *unlocks* (`close g`) rather than reads. Identical refusal ⇒ affinity is checked per acting task, not per operation kind. |
| CT31 | `performed.qcore` | accept | accepted | The announcing task performs the acquisition; the core's `Lock` withdraws the waiter and destruction is quiescent. So `Q619` is keyed to the outstanding announcement. |
| CT32 | `joined_outside.qcore` | accept | accepted | The identical child block moved one statement past the section's end. So the refusal is keyed to the join landing while a guard is held. |
| CT48 | `reasserted.qcore` | accept | accepted | Re-establishing the fact under the second guard mints a fresh fact and checks — the spec's "**re-establish**" half, so `Q616` demands re-establishment rather than poisoning the program. |
| CT48 | `never_asserted.qcore` | reject `["Q600"]` | `UnknownFact { event: 2, fact: 1 }` | A declared-but-never-asserted fact gets a **different** code, so the case's `Q616` is staleness rather than absence. |

CT48 is the only one of the four whose spec expectation has two directions
("invalidate / re-establish"); both execute, on separate artifacts.

---

## 3. Stage evidence — what is claimed, and what is not

Machine-readable rows: `docs/reports/wave5-ct-integration/ct_stage_evidence.json`,
in the same schema as the T03-CT and wave-3 files so a future summarizer can
merge all three.

All four cases claim exactly:

```
parser              executed
type_checker        executed
reference_execution unassessed
cfg                 unassessed
native_backend      unassessed
proof               unassessed
```

which the harness enforces: `verdict: "reject"` admits exactly
`{parser, type_checker}` as `stages_executed`, and each manifest stage must
read `executed` iff it is in that set.

### 3.1 `cfg` is `unassessed`, and CT03's precedent does not apply

The task said to follow CT03's `cfg: executed` precedent **only if the harness
actually executes it**. It does not, and cannot. `cfg` is `executed` on the
*task* route because that route reads it as the checked task IR it really
constructs. `source.mutex.guard.v1` emits directly from its checked event list
and **has no CFG construction at all** — the harness encodes this as a hard
rule (`allowed_stages` for every mutex verdict mode omits `cfg`), and the
overclaim is fail-closed (§5). Recording `cfg: executed` here would claim a
stage that does not exist in the route.

### 3.2 No native evidence is added by this card

All four cases are rejections, so none reaches C emission or native execution.
The controls do not repair that:

* **CT28, CT31 and CT32's accepted controls are multi-task modules.** The guard
  backend emits **one thread**, so `compile_mutex_source` publishes such a
  module with `c_source: None` — checked, with no emitted C (card T03-F §3.5).
  Measured here: all three report `c_source=false`. There is no C to compile
  and nothing to run.
* **CT48's accepted control is single-task and does emit C** (`c_source=true`),
  the only one of the four that does. Even so, the CT harness's
  `run_mutex_controls` **route-checks** every control — it asserts
  `as_checked_mutex_guards().is_some()` — and does not compile or run it.
* `wait m;` (`RegisterWaiter`) is deliberately in the emission-refused set: an
  announced acquisition only another thread could satisfy has no honest
  single-thread rendering.

**What `evidence_target: "linux"` means on these four cases.** It records the
harness's host gate and nothing more. `run_mutex_controls` compiles an accepted
control through a path where the non-Linux/macOS/Windows `Q500` target refusal
is evaluated *before* the emission decision, so on another host the control
arrives as `Q500` rather than being silently skipped — which is exactly the
condition the harness's `evidence_target` assertion tests. It is **not** a
claim that any C was compiled or any binary ran. The three
sanitized-native CT cases remain CT03, CT17 and CT26; this card adds none.

Also not claimed, carried over from card T03-F unchanged: no composition with
the scoped task ledger (captures, owned values and completion capsules are
refused in this profile); no schedule, no contention, no C memory-model claim;
no source-to-trace refinement; and a fact has no predicate, so CT48 is
refinement *invalidation*, never refinement *content* — nothing checks what a
fact says.

---

## 4. Deliberate pin updates

| Pin | Before | After | Why |
|---|---|---|---|
| `compiler/tests/ct_corpus.rs::MAPPED_CASES` | 20 ids | 24 ids (+CT28, CT31, CT32, CT48, in ascending order) | The harness compares the manifest's mapped set to this table element-wise. |
| `compiler/tests/ct_corpus.rs::MUTEX_REJECT_CASES` | `CT27, CT29, CT30` | `CT27, CT28, CT29, CT30, CT31, CT32, CT48` | The harness asserts `checked_mutex_reject` metadata is present **iff** the id is in this set — a fixture cannot carry mutex-rejection metadata without being listed, and cannot be listed without carrying it. |
| `ct_corpus.rs` module doc + `ct_mapping_progress_is_pinned_and_counted` doc | "20 of 50 … 30 remain"; mutex route "CT26/CT27/CT29/CT30" | "24 of 50 … 26 remain"; the eight-case mutex route | Prose that contradicts the table is a defect; the count assertion itself is derived from `MAPPED_CASES`. |
| `docs/reevaluation/ct_manifest.json` | 4 cases `unmapped` | 4 cases `mapped` with fixture path, semantic profile, per-stage statuses and `evidence_target` | The mapping itself. The `note` also grows the mutex-route case list and states the no-native-evidence boundary. |
| `docs/reviews/r0_3/acceptance.json` | `ct_mapped: 20`, `ct_unmapped: 30` | `24` / `26` | Following the wave-3 precedent, which updated the same scalars in the same shape. The `ct_evidence` block's note, `derived_from`, `stage_histogram`, `depth_profile`, `observation_counts`, `unmapped_bucket_counts` and `cause` are updated consistently; every histogram column still sums to the mapped count, and the depth profile still sums to 24. |

The acceptance-record deltas are all "+4 at the shallow end": `parse` and
`elaborate` executed 20 → 24; `mutex_lowering` and `mutex_invariant_check`
executed 4 → 8; nine deeper stages gain `not_applicable` 4; `native_sanitized`
gains `unassessed` 4; `proof` gains `unassessed` 4. `depth_profile`'s
`mutex_invariant_check` moves 3 → 7. Sanitized-native stays **3 of 24** and
proof stays **0 of 24** — unchanged in absolute terms while the denominator
grew, which is stated in the note so the ratio cannot be read as progress it
is not. The `blocked-on-guard-profile-composition-or-unrepresented-form`
unmapped bucket moves 7 → 3 (the remainder being CT33, CT34, CT46).

### 4.1 Not edited, and flagged for the integrator

`docs/reviews/r0_3/acceptance_matrix.json` is owned by the integrator and was
**not** touched. Its `generator_status` field states that
`docs/reports/remaining-d02/build_matrix.py` cannot regenerate the section
because its mapped set diverges from the manifest's — recorded then as **nine
cases**; after this card it is **thirteen** (the nine plus CT28/CT31/CT32/CT48).
Following the wave-3 convention, the generator was **not** silently re-run: the
divergence is recorded here for the integrator to carry into
`generator_status`, and `T00-CT-SCHEMA` still owns the stage-derived
summarizer work. `AGENTS.md`, `CLAUDE.md`, `README.md` and `DEVELOPMENT.md`
were likewise not edited.

---

## 5. Fail-closed verification

Each mutation was applied to a scratch copy of the fixture, the harness run,
and the fixture restored. **None was committed**; `git status` was verified
clean of mutations afterwards.

| Mutation | Result |
|---|---|
| CT28: drop the trailing `Q600` from `diagnostic_sequence` | harness **FAILED** |
| CT31: reorder the cascade to `["Q600","Q619","Q600","Q600"]` | harness **FAILED** |
| CT32: substitute `Q612` for `Q600` in both the sequence and `diagnostics` | harness **FAILED** |
| CT48: flip the accepted `reasserted.qcore` control to `expectation: "reject"` | harness **FAILED** |
| CT32: claim `cfg: "executed"` in the manifest | harness **FAILED**: `CT32: manifest stage 'cfg' must be 'unassessed' to match the executed fixture` |
| all restored | harness **passes** |

The last row is the one that matters for §3.1: the stage overclaim this card
declined to make is not merely a convention, it is enforced.

The harness's own structural gates were relied on rather than re-implemented: a
fixture directory whose case still reads `unmapped` fails; a mapped case with
no fixture fails; the fixture file inventory must equal the set of files the
metadata actually names (so an unreferenced fixture file cannot sit in a case
directory); and `MUTEX_REJECT_CASES` membership and `checked_mutex_reject`
presence must agree in both directions.

---

## 6. Gate results

Run at this card's tip, Linux x86_64, submodule initialised at the pinned
`d82464a`.

| Gate | Result |
|---|---|
| `cargo test -p qcore-compiler --test ct_corpus` | 4 passed, 0 failed |
| `cargo test --workspace` | **1624 passed, 0 failed, 1 ignored** |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |

The workspace total is unchanged from the merged wave-5 baseline of **1624**:
this card adds fixture *data* and pinned-table entries, not new `#[test]`
functions, so the four new cases execute inside the existing
`ct_manifest_and_fixture_set_agree_and_mapped_cases_execute` test.

Not re-run and not claimed: the four pinned 185-unit Cycle 19 artifacts
(`CORPUS-REPIN` remains open — this card touches no gateway, contract, profile,
schema, provider or audit file), the Lean audits (no proof file changed), the
provider workspace and the Python suites.

---

## 7. Nothing declined

All four proposed cases mapped honestly on measured evidence; none had to be
forced and none was dropped. Three mutex cases remain unmapped with their
reasons unchanged from card T03-F:

* **CT33** (lock-order cycle) — rank mode is a represented refusal, not a
  feature; there is no source form declaring a lock rank, so a cycle cannot be
  written down.
* **CT34** (protected graph grows after creation) — `CreateMutex` fixes its
  declared footprints at creation; there is no growth form.
* **CT46** — needs `Send`/`Sync` derivation, outside L09 and outside this
  profile.

CT11, CT24, CT35, CT36, CT38, CT39 and CT47 keep the reasons the wave-3
integration recorded; this card did not re-examine them.

**Mapping is not conformance.** 24 of 50 CT cases now execute as source-level
tests; 26 do not, and full language conformance remains false.
