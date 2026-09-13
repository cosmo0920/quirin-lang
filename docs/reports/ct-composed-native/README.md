# Wave-32 CT lane — the composed-epoch native runner (CT-COMPOSED-NATIVE)

Wave 32, card `CT-COMPOSED-NATIVE`, discharging the follow-on named by
`docs/reports/ct22-wire/README.md` §8 and repeated by
`docs/reports/ct-wire-lane-3/README.md` §8: *an admitted `composed_epoch`
module emits the task route's own C that no harness runner executes.* One
runner, on the `run_sequential_sanitized` shape, now compiles and runs it.

**The runner is real and it runs on every `cargo test` of this workspace.**
On Linux and macOS, for CT22 and CT25, `run_composed_native` compiles that
case's **own emitted C** with the platform C compiler under ASan/UBSan
(plus LSan on Linux), runs it, and compares its standard output **byte for
byte** with `render_task_observation` of the reference execution of that same
module's **own checked task half**. Both cases pass.

**No stage record moves in this card, and no CT number moves.** CT coverage
stays **43/50 mapped**, sanitized-native stays **7 of 43**, proof stays
**3 of 43**. The three rows the runner backs still read `unassessed` in
`docs/reevaluation/ct_manifest.json`, which now **understates** what this
harness executes — the safe direction, never the claiming one. §4 publishes
the exact coupled edit that moves them, and §4.1 says why this card does not
take it.

Base: `main` at `1c7d8e45`. Branch: `worktree-agent-a76813dcec69cbabe` in an
isolated worktree.

**Nothing is promoted, and nothing semantic changed.** `git diff` over
`crates/` and `compiler/src/` is empty for this card: the only Rust is
`compiler/tests/ct_corpus.rs`, and the only data is two fixtures'
`expected.json`. No checker rule, no diagnostic code (this card allocates zero
and widens zero conditions — the Q-space icebox needed none), no lowering, no
model, no reserved word, no crate-source change.

---

## 1. What runs, and where

| | |
|---|---|
| Runner | `compiler/tests/ct_corpus.rs::run_composed_native` |
| Driven by | `COMPOSED_NATIVE_CASES = ["CT22", "CT25"]`, pinned and fail-closed |
| Dispatched from | `run_epoch_credit_split`'s **accepted** arm, after every checked-object premise |
| Artifact | the case's **own** `case.qcore`, its own emitted `c_source` |
| Emission gate | **host-unconditional** — `c_source` is asserted before the platform is consulted |
| Compile-and-run gate | **`["linux", "macos"]`** — the `deep_gate` every sanitized runner in this file carries |
| Leak detection | **Linux only** — Apple's ASan ships no LSan, so the macOS arm runs the same binary with `detect_leaks=0` |
| Tool | the platform C compiler, `QCORE_CLANG` if set, else `cc` |
| Sanitizers | `-fsanitize=address,undefined`, `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1`, `UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1` |
| Invocations | one emission, one compile, one run, per case |

Measured on this tree, host Linux, `cc`:

| Case | Compile | Run | stdout vs. reference | stderr | LSan |
|---|---|---|---|---|---|
| **CT22** | ok | exit 0 | **byte-identical** (7 lines) | empty | clean |
| **CT25** | ok | exit 0 | **byte-identical** (9 lines) | empty | clean |
| **CT23** | — | — | — | — | — (§3) |

### 1.1 The comparison basis, and why it is the module's own

A composed module publishes **no reference observation of its own** — which is
exactly why the `composed_epoch` route vocabulary has no reference-execution
stage, and why a native runner had no obvious thing to compare against. It
does, however, carry the `CheckedTaskModule` that the **unedited**
`qcore_core::task_execution` interpreter takes
(`ComposedEpochModule::tasks`). So the reference side here is that
interpreter's own run of this module's own checked task half: no second
interpreter, nothing hand-built (reevaluation spec §5.1), and no evidence
imported from a neighbouring fixture.

The rendering is additionally **pinned line by line** in each fixture's
`composed_native.declared_observation`, so the native bytes are compared
against a *stated* transcript rather than against whatever the two sides
happen to agree on — the `sequential_sanitized` discipline, one route over.
The runner also pins the reference run's `root_outcome` and its task count, so
a reference run that silently lost a task could not still match a shorter
transcript.

CT25's transcript carries the scenario in the open:

```
QCORE_BUFFER_ALLOC 0 1 42     the parent's owner
QCORE_TASK_SPAWN 1 1
QCORE_BUFFER_LOAN 1 1         the counted read share, at the spawn
QCORE_BUFFER_READ 1 1 42
QCORE_TASK_RESULT 1 I32 42
QCORE_TASK_JOIN 1
QCORE_BUFFER_RETURN 1 1       the loan expires, at the join
QCORE_BUFFER_DROP 0 1         the discharge's destroy, after both
QCORE_TASK_RESULT 0 I32 1
```

## 2. Observed, not proved

The `STABILITY-REEVAL` discipline, applied to this card's own additions.

**What one successful run establishes.** On this host, with this C compiler,
under these sanitizer options, on the all-success script: the emitted
program's standard output was byte-identical to the checked task half's
reference transcript; it wrote nothing to standard error; it exited 0. On
**Linux**, LSan's silence additionally observes that what the program
allocated was released — for CT25 that is the one buffer the discharge's
erased `drop owner;` destroys, which is the sense in which *cleanup destroys
owner* is **executed** here rather than decided in a model.

**What it does not establish.**

* **Not a proof.** A single scheduling of one program is an observation of
  that scheduling. Nothing here is a theorem about the emission, the
  interpreter or their agreement, and CT22's and CT25's `proof` stages stay
  `unassessed` (T00.7, unchanged).
* **Not evidence about any quantity.** The region and the flat-epoch plan
  **erase from emission**, so no byte of this run bears on credit 5, effort 2,
  the abandoned remainder 3, the granted/restored/spent split or the epoch
  bound. Those are check-time folds, measured by the `epoch_credit_split` mode
  and by nothing else. The converse is the runner's value: a discharge that
  lost an adopted duty shows up here, because the destroy is a real emitted
  `qcore_buffer_drop`; a mis-stated credit does not.
* **Not a macOS reclamation claim.** The macOS arm runs with
  `detect_leaks=0`. A reader must not take a green macOS run as evidence that
  anything was reclaimed.
* **Not other hosts, inputs or schedules.** Windows reaches the emission and
  returns before the compile-and-run, printing that it did. No claim is made
  for any input the fixture does not write or any interleaving this run did
  not take.
* **Not conformance.** Two cases' emissions now execute. Full language
  conformance stays `false`, and a stage that reads `executed` is evidence
  about that case's artifact (constraint 23).

### 2.1 Every arm was falsified before it was believed

An arm no test drives is a stated boundary, not a verified rule — so each was
made to fail on this tree, then restored (transcripts in `measure/`,
worktree-private):

| Arm | Falsification | Result |
|---|---|---|
| The pinned transcript | CT25's `declared_observation[7]` changed to `QCORE_BUFFER_DROP 0 99` | **red** — *"CT25: the reference transcript drifted from the pinned rendering"* |
| The native byte comparison | the runner's expected stdout temporarily suffixed `TRIPWIRE\n` | **red** — *"CT22: the native run's standard output is compared byte for byte …"*, the actual bytes printed |
| LSan | a leaking `__attribute__((constructor))` appended to the instrumented source | **red** — `LeakSanitizer: detected memory leaks`, `Direct leak of 64 byte(s)` |

The compile-and-run therefore demonstrably happens, and the comparison and the
leak check are live rather than vacuous.

## 3. CT23 keeps its rows, and the reason is stated

CT23 is the `composed_epoch` route's **refusal**. `compile_files` never
reaches the backend on it, so there is no emission of its own for any runner
to compile: its `c_emission` and `native_execution` rows are `not_applicable`
for exactly that reason and this card does not touch them, and its
`native_sanitized` stays `unassessed` on the unchanged route rule. It is
deliberately **absent** from `COMPOSED_NATIVE_CASES`, and the harness asserts
that absence from the refused arm — a refused composed-epoch fixture in the
table fails the suite by name. That is a stated reason, not a sweep.

## 4. The proposed stage records, for the integrator

The runner's name is registered in `RUNNER_REGISTRY`, which is what makes it
**legal** for a manifest stage entry to cite. The rows themselves are one
coupled edit, published here in full.

### 4.1 Why this card does not take it

A stage status is checked against the manifest for **exact equality**
(`ct_corpus.rs`, *"stage `{stage}` claims `{}` but this harness derives
`{}`"*). The derivation and the manifest rows must therefore move in **one**
change. The manifest is outside this card's boundary — the CT numerators it
feeds are the integrator's wave-wide reconciliation — and writing the
`executed` arm into `derive_stage_plan` alone would either turn the suite red
or leave an arm no test drives, which is the very thing §2.1 exists to
prevent. So the card lands the runner, runs it, and hands over the edit.

### 4.2 The harness half

In `derive_stage_plan`, beside the existing `composed_epoch` block (the site
carries a comment pointing here):

```rust
if route == "composed_epoch" && !refused {
    if COMPOSED_NATIVE_CASES.contains(&case_id) {
        let runner = "compiler/tests/ct_corpus.rs::run_composed_native";
        plan.insert("c_emission", StagePlan::executed(runner, 1, Some(all_three.clone())));
        plan.insert("native_execution", StagePlan::executed(runner, 1, Some(vec!["linux", "macos"])));
    } else {
        plan.insert("c_emission", StagePlan::absent("unassessed"));
        plan.insert("native_execution", StagePlan::absent("unassessed"));
    }
}
```

and, for the sanitizer gate (T00.3), `SANITIZED_GATED_CASES` grows by
`"CT22"` and `"CT25"` — 7 → **9** entries. Its existing
`match (deep, sanitized_control)` arm must gain the composed case, since
`deep` is `None` for this mode; the minimal form is a third arm crediting
`run_composed_native` with `1` invocation on `["linux", "macos"]`. Each of
those is a pinned expectation edited deliberately (invariant 5) and belongs in
the commit message.

### 4.3 The manifest half

`docs/reevaluation/ct_manifest.json`, three rows per case:

**CT22** and **CT25**, identically:

```json
"c_emission": {
  "status": "executed",
  "executor": "compiler/tests/ct_corpus.rs::run_composed_native",
  "observations": 1,
  "host_gate": ["linux", "macos", "windows"]
},
"native_execution": {
  "status": "executed",
  "executor": "compiler/tests/ct_corpus.rs::run_composed_native",
  "observations": 1,
  "host_gate": ["linux", "macos"]
},
"native_sanitized": {
  "status": "executed",
  "executor": "compiler/tests/ct_corpus.rs::run_composed_native",
  "observations": 1,
  "host_gate": ["linux", "macos"]
}
```

**CT23**: **unchanged** — `c_emission` and `native_execution` stay
`not_applicable`, `native_sanitized` stays `unassessed` (§3).

`evidence_target` is already `linux` on all three cases and needs no edit.

### 4.4 What the numbers become

| Measure | Now | After the coupled edit |
|---|---|---|
| CT mapped | 43/50 | **43/50** (mapping does not move) |
| sanitized-native | 7 of 43 | **9 of 43** |
| proof | 3 of 43 | **3 of 43** |
| `depth_profile` (`ct_stage_summary.py`) | `native_sanitized 6`, `task_ledger_check 7` | `native_sanitized 8`, `task_ledger_check 5` — CT22 and CT25 deepen from `task_ledger_check` to `native_sanitized` |
| `stages_executed` in CT22/CT25 `expected.json` | 6 stages | **9** — `c_emission`, `native_execution`, `native_sanitized` appended |

That last row is part of the same coupled edit: `stages_executed` is asserted
equal to the derived plan. The `depth_profile` shift follows mechanically from
the deepest-executed-stage rule and is not a separate decision.

A lane taking this inherits one measured consequence: `acceptance.json` is
regenerated through `scripts/ct_stage_summary.py --write`, its hand-kept
`ct_evidence.note` current-state numbers refreshed (*7 of 43* → *9 of 43*),
and `derived_from` grown by this record — none of which this card touched.
The derived `newly_decidable_not_mapped_reasons` key set is **not** affected:
its keys stay `["CT24", "CT47"]`, since no case's `mapping_status` moves
(constraint 25).

## 5. Files changed

* `compiler/tests/ct_corpus.rs` — the runner `run_composed_native`; the
  pinned `COMPOSED_NATIVE_CASES` table; the `ComposedNative` fixture struct
  and its `ExpectedCase` field; the two-directional table/fixture agreement
  assertion and the refused-arm exclusion; `RUNNER_REGISTRY` grown by one
  (deliberate, stated in the commit message); the `derive_stage_plan` comment
  refreshed to say the runner now exists, that the rows understate it, and
  where the coupled edit is published.
* `compiler/tests/fixtures/ct/CT22/expected.json`,
  `compiler/tests/fixtures/ct/CT25/expected.json` — the `composed_native`
  block (sanitizer flags and options, the pinned transcript, the root outcome,
  the task count, and a `note` stating what the run observes and what it does
  not); each file's `notes` sentence claiming *no native or sanitizer evidence
  is claimed* corrected, since it is no longer true.
* `docs/reports/ct-composed-native/README.md` — this record.
* `docs/development/increments/wave-32.md` — the increment paragraph.

**Not** changed, and left for the integrator: `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json`, `docs/reviews/r0_3/acceptance_matrix.json`,
every `EVIDENCE_FILES` list, `AGENTS.md`'s headline numbers and `README.md`.
This card publishes **no** stage-evidence record and declares **no**
`retired_unmapped_reasons`: it maps no case and retires no row.

## 6. What this does not establish

* **Nothing is promoted.** The Cycle 19 corpus is untouched — no gateway,
  contract, audit or provider file is in this card's diff — so its rows are
  **carried**: raw units 2 Safe / 42 provisional Unsafe / 137 Unknown plus
  four data-only, contract corpus **76** modules, `qcore-schemas` 14.0.0.
  This wave runs **no arbiter** and this card did not run the corpus.
* **Coverage did not grow.** No case is mapped, no denominator moves, no
  unmapped row is retired. The card adds evidence under three existing rows.
* **The capability cards' boundaries carry through**, none narrowed and none
  discharged here: `REGION-JOINS-T07`, `REGION-JOINS-EPOCH-AXIS`,
  `REGION-JOINS-ERASURE-FIT`, `REGION-JOINS-MULTI-OWNER`,
  `REGION-JOINS-LOAN-ADOPT-ONLY`, `CT24-CLEANUP-TASK-COMPOSITION`.

## 7. Gate

Run in this worktree on `main` at `1c7d8e45` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2`, before the workspace run |
| `cargo test --workspace --no-fail-fast` | **4,510 passed / 1 failed / 8 ignored** across 223 result lines, exit 101. The one failure is a constraint-16 load row, green in isolation (§7.1) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | `Passed 125 Lean audits` (unchanged — this card adds no Lean), exit 0, read from the text line |
| `python3 -m unittest discover -s scripts/tests` | **200 tests, OK** (this card adds none) |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT, `mapped 43 / unmapped 7` |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed — the runner's two compile-and-run cycles inside them |

### 7.1 The workspace row, both facts

One test failed, and it is one of constraint 16's five named load-sensitive
rows — the confined-fixture write test:

```
---- a_confined_fixture_cannot_write_outside_the_admitted_paths stdout ----
crates/qcore-verifier-host/tests/confined_process_execution.rs:349:5:
assertion `left == right` failed: Broken pipe (os error 32)
  left: Io
 right: Terminated
```

That is the row's recorded signature exactly — `Io(Broken pipe)` under load —
and the load was real: three sibling delegated agents were running their own
full workspace suites concurrently in their own worktrees, with a **load
average of 37.8 on 16 cores** at the time. Re-verified in isolation on this
same tree:

```
cargo test -p qcore-verifier-host --test confined_process_execution
test result: ok. 12 passed; 0 failed; 0 ignored
```

**Both facts are recorded and neither is presented as the other**: the single
`--no-fail-fast` run reported 4,510 passed / 1 failed / 8 ignored with exit
101, and the failing row passes in isolation. No assertion was weakened and no
row was skipped to get there (constraint 16). The suite covered **4,519**
tests (4,510 + the one row, itself green in isolation) against the wave-31
close's 4,511 at its integration tip; this card adds no test *function* —
`run_composed_native` executes inside `ct_corpus.rs`'s existing five tests,
whose assertions grew — so the difference is the sibling cards' tests present
in this worktree's base, not this card's.

`--no-fail-fast` earned its place here (`GATE-NO-FAIL-FAST`): the failing row
sits in `qcore-verifier-host`, and cargo's default would have stopped there
and never reached `qcore-compiler`'s `ct_corpus` — this card's own evidence —
at all.

The `providers/` workspace is untouched by this card and its 122-test
checkpoint is carried, not rerun (constraint 18 owes nothing: no provider,
schema or template changed).

## 8. Named follow-ons

| Id | What |
|---|---|
| `CT-COMPOSED-NATIVE-ROWS` | **New, and this card's own.** The runner executes; the three stage rows it backs still read `unassessed`. The coupled edit is published verbatim in §4 — harness arm, `SANITIZED_GATED_CASES`, manifest rows, `stages_executed`, and the `acceptance.json` regeneration — and applying it moves sanitized-native 7 of 43 → **9 of 43**. Until it is applied, the manifest understates this harness. |
| `CT-COMPOSED-NATIVE-SCRIPT` | The runner executes the **all-success** script only. The composed route's failure edges — a refused spawn, a failed allocation — are checked facts with no native run, where the task route's own fixtures inject faults through a header (`native_fault_run`). A composed fixture doing the same would observe the region's erased release on an error path, which is where a lost duty would hurt most. |
| `CT-COMPOSED-NATIVE-APPLE` | The Linux arm is the only one measured. The macOS arm is written and gated but has not been run here, and it makes **no** reclamation claim by construction (no LSan). Its discharge condition is a macOS run of `cargo test -p qcore-compiler --test ct_corpus`. |
| `T00.7` | Unchanged and still open for both cases: no theorem is bound to either fixture, and a native run is an observation, never the proof leg. |
