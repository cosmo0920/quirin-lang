# T00 — Stage- and target-derived CT coverage audit

Date: 2026-09-07. Audit revision: `ea2af0f`. Implementation checkpoint:
`e3e7c2c` (every commit since is documentation-only; verified with
`git diff --name-only e3e7c2c..HEAD`). Machine-readable companion:
[`coverage.json`](coverage.json).

**Scope.** Read-only. This card modifies no tracked file, maps no CT case,
changes no manifest stage status, weakens no gate and promotes no verdict.
Everything in §5 is a proposal for an integrator, not an applied change.

**What counted as executed evidence.** A stage is called *executed* only when a
named workspace test runs it on the case's own artifact. A fixture file's
existence, a manifest string, a hand-written Rust event test and a transcript
pasted into a report do **not** count. Where a stage is host-gated, that is
recorded rather than absorbed.

Observations this card actually made, at `ea2af0f`:

| Command | Result |
|---|---|
| `cargo test -p qcore-compiler --test ct_corpus -- --nocapture` | 4 passed, 0 failed (1.99 s) |
| `QCORE_CLANG=/nonexistent cargo test -p qcore-compiler --test ct_corpus` | 1 **FAILED** at `ct_corpus.rs:858` |

The second run is the point: the native stage is load-bearing and fails closed.
Removing the C compiler breaks the harness rather than silently reducing it to a
source check. Source-grammar probes below used `target/debug/qcorec`, verified
current (`find crates compiler -name '*.rs' -newer target/debug/qcorec` is
empty).

---

## 1. The structural finding that reframes the matrix

**All 11 mapped cases are `concurrency checked;` task modules.** Every one takes
`compile_task_source`
(`crates/qcore-verification/src/driver.rs:1100-1212`), which is:

```
parse → elaborate → lower_source_tasks → check_task_module → emit_task_c11
```

It **never** calls `synthesize_semantics` or `check_whole_program`, and it
returns `reference_observation: None` (driver.rs:1211) — an assertion the
harness itself repeats for CT01/CT12/CT15. So:

- The **sequential resource CFG and its reference execution are not run on any
  mapped CT case's own fixture.** CT16 and CT37 do have executed sequential
  reference evidence, but on *different* fixtures under
  `fixtures/task_worker_allocations/`, in two separate tests.
- The manifest's `cfg: executed` and `reference_execution: executed` therefore
  name the **task** IR and the **task** reference interpreter, not the
  sequential machinery those names have meant everywhere else in this project.

A second consequence: the manifest's `type_checker` stage silently covers
elaboration, task lowering *and* the task ledger check. **CT02 and CT19 reject
at `Q600` inside `check_task_module`** — the deepest source stage there is —
yet the matrix renders them identically to a hypothetical case that died at
`Q013` during name resolution.

---

## 2. Verified stage matrix (11 mapped cases)

Legend: **E** executed · **R** recorded, not gated · **N/A** unreachable in this
case's mode · **—** unassessed. Full per-stage executor and artifact pointers
are in `coverage.json` → `mapped_case_stage_matrix`.

| Case | parse | elab | task lower | ledger check | seq. CFG | task ref exec | C emit | native run | sanitized | proof |
|---|---|---|---|---|---|---|---|---|---|---|
| CT01 | E | E | E | E | N/A | E ×1 | E | E ×1 | — | — |
| CT02 | E | E | E | **E (Q600)** | N/A | N/A | N/A | N/A | — | — |
| CT12 | E | E | E | E | N/A | E ×2 | E | E ×2 | — | — |
| CT13 | E | E | E | E | N/A | E ×3 | E | E ×3 | — | — |
| CT14 | E | E | E | E | N/A | E ×1 | E | E ×1 | **R** | — |
| CT15 | E | E | E | E | N/A | E ×1 | E | E ×2 | — | — |
| CT16 | E | E | E | E | N/A | E ×4 | E | E ×4 | **R** | — |
| CT19 | E | E | E | **E (Q600)** | N/A | N/A | N/A | N/A | — | — |
| CT20 | E | E | E | E | N/A | E ×4 | E | E ×4 | **R** | — |
| CT37 | E | E | E | E | N/A | E ×1 | E | E ×1 | **R** | — |
| CT50 | E | E | E | E | N/A | E ×1 | E | E ×1 | **R** | — |

Every `E` in the *task ref exec*, *C emit* and *native run* columns is gated on
`cfg!(target_os = "linux")`; on any other host the runner returns after
`unsupported_target()` and those three columns become unexecuted.

**Executed totals:** 18 task reference executions, 19 native compile-and-run
cycles, 11 in-case source rejection controls, **0** gated sanitized-native
cases, **0** cases with proof evidence, 2 cases stopping at source rejection.

### Executed evidence the manifest does not record

- **CT01** and **CT13** additionally run end-to-end as *processes*:
  `crates/qcorec/tests/cli.rs` invokes `qcorec check`, `qcorec emit`, then
  clang, then the binary, on the real fixture (Linux-gated). CT01 is also the
  `ROUND_TRIP` constant driving ~25 adversarial mutation checks in
  `crates/qcore-frontend/tests/owned_tasks.rs`. These are the only two cases
  with process-level evidence, and the manifest has no field for it.
- **CT19**'s positive control *does* execute a Linux-gated task reference run on
  `joined_sibling.qcore` (root outcome 42, two children, both joined). The
  manifest records `reference_execution: unassessed` and no `evidence_target`.

---

## 3. Claims exceeding executed stage evidence

Nine findings; full text in `coverage.json` → `claims_exceeding_executed_evidence`.
The four that matter most:

**O1 — `cfg: executed` is ambiguous and unevidenced (9 cases).** No assertion
targets a CFG artifact. The sequential CFG is *provably* not run (§1). The claim
survives only under an unstated reading of "cfg" as the task IR's
blocks/terminators, which is exercised transitively but never checked directly.

**O3 — stage statuses are host-unconditional strings.** The runners are
Linux-gated; the manifest field a summarizer consumes is not. On macOS or
Windows the manifest still reads `executed` for three stages that did not run.
The manifest `note` concedes this in prose; the machine-readable field does not.

**O4 — sanitizer evidence is recorded, not gated (CT14/16/20/37/50).** No
workspace test compiles any CT fixture with `-fsanitize`. `native_fault_run`
uses `-std=c11 -pthread -Wall -Wextra -Werror` only. The ASan/UBSan/LSan
evidence lives as transcripts in `docs/reports/w6-*/README.md`, re-run by
nothing. The only sanitized C in the workspace
(`crates/qcore-core/tests/unit/c_emit.rs`) runs a different, sequential corpus.
This is the clearest case of recorded evidence sitting flush against gated
evidence in the same sentence.

**O7 — `acceptance.json` has no per-CT rows at all.** It carries exactly
`ct_mapped: 11` and `ct_unmapped: 39`. That scalar treats CT02 (source
rejection, zero native runs) and CT16 (4 reference executions + 4 native runs)
as one identical unit of coverage, and it hides that 0 of 11 cases have proof or
gated sanitized evidence. Rows R02 and R04 state their boundaries accurately in
prose — the defect is that the *consumable numbers* are binary. **No row's prose
claim was found to exceed its evidence; the scalar is what overstates.**

Also recorded: **O2** (`type_checker` conflates three passes), **O5** (CT02's
"CT01 supplies the control" is narrative — `ct_corpus.rs:260-264` structurally
*forbids* CT02 from carrying a control), **O6** (CT19's control evidence
unrecorded), **O8** (proof `unassessed` everywhere — honest, and no Lean theorem
is bound to any fixture; the sole CT string in `proof/` is a comment at
`proof/QCore/TaskExecution.lean:539`), **O9** (CT14/CT37's
`completion_before_join.h` is a 66-byte `#include` stub of a shared adapter
outside the inventory-checked tree).

---

## 4. Missing-stage tasks

Each absent stage becomes a named follow-on, not a duplicate mapping. Full
detail in `coverage.json` → `missing_stage_tasks`.

| ID | Task | Cases | Size |
|---|---|---|---|
| T00.1 | CT02: native negative-control stage + harness-checked control link | CT02 | small |
| T00.2 | CT19: native control stage and `evidence_target` | CT19 | small |
| T00.3 | Sanitized-native stage as a real (opt-in, fail-closed) gate | 5 cases | medium |
| T00.4 | Split the `cfg` and `type_checker` stage names into the four real passes | all 50 rows | medium |
| T00.5 | `host_gate` as a first-class per-stage field | 9 cases | small |
| T00.6 | Record the executed CLI end-to-end stage | CT01, CT13 | small |
| T00.7 | Proof stage: bind theorems to fixtures, or stop reporting Lean counts adjacent to CT counts | all 11 | large |
| T00.8 | Declare transitively included native adapters in the fixture inventory | CT14, CT37 | small |
| T00.9 | Derive `acceptance.json` CT rows from stage evidence | acceptance | medium |

T00.1, T00.2, T00.5, T00.6 and T00.8 are harness/manifest work with no
language, checker or backend change.

---

## 5. Updated triage of the 39 unmapped cases

Baseline is `docs/reports/w6-ct-batch-2/README.md`, written when 5 of 50 were
mapped. Six of its rows have since been mapped (CT01, CT13, CT14, CT15, CT20,
CT50), and its statement of the *mappable surface* — the "Cycle 12 executable
subset" — is now substantially obsolete: the task grammar adds `thread scope`,
`spawn`/`join`, `Result` match arms, `raise`, `own<Buffer> use one`, effect
lists and nested blocks.

**20 of 39 rows have a changed bucket or a changed blocker.** Current
distribution:

| Bucket | Count | Cases |
|---|---|---|
| **decidable-now** | **3** | CT24, CT39, CT47 |
| blocked-on-unimplemented-syntax (no mutex value) | 11 | CT26–CT34, CT46, CT48 |
| blocked-on-unimplemented-syntax (other) | 7 | CT07–CT10, CT23, CT38, CT49 |
| blocked-on-harness-schema (C-twin stage) | 5 | CT40–CT43, CT45 |
| blocked-on-task-lowering (share captures) | 4 | CT03, CT04, CT05, CT11 |
| blocked-on-W3/W4 | 4 | CT21, CT22, CT35, CT36 |
| blocked-on-typed-error-payloads | 2 | CT17, CT18 |
| ambiguous | 3 | CT06, CT25, CT44 |

### 5.1 Newly decidable (3)

**CT39 — "Reject unsupported claim."** Batch 2 named its direct blocker as a
*silent-acceptance* path: contract clauses parsed and were never read. That is
repaired. Probed:

```
fn main() -> i32 contract { quant normal wall_clock <= 5; } { return 0; }
→ Q021: source function contract clauses are not yet enforced; refusing unchecked contract
```

Decidable now as a `Reject` at parse + elaborate. **Caveat that must go in its
`semantic_profile`:** `Q021` refuses *every* contract clause, so a fixture
records "an unenforced claim was refused", not "a concurrent-work bound was
conflated with a wall-clock bound". Map with that stated, or hold until
contracts are enforced.

**CT47 — "Reject automatic cross-thread acceptance."** Batch 2's reason was
"there is no cross-thread boundary to reject". There is one now. Probed:

```
fn worker(p: ptr<Buffer>) -> i32 { … }   captured across `spawn s`
→ Q020: task parameters require i32 or own<Buffer> use one
```

Decidable now as a `Reject`. **Caveat:** `Q020` refuses every unsupported task
parameter type, not specifically a raw pointer lacking a transfer/share
contract.

**CT24 — "Cleanup destroys an owner before child join → Reject."** Batch 2's
reason ("no borrow/join to be outstanding") is obsolete. Two *distinct*
mechanisms now exist:

- explicit drop before join → already executed as CT14's
  `drop_before_join.qcore` rejection control;
- **implicit scope-exit cleanup** with a live handle → probed, `Q600
  ScopeBusy(4)`.

**Do not map CT24 to the explicit drop** — that duplicates CT14's executed
control and inflates the count without adding evidence. A CT24 fixture is
justified only if it uses the scope-exit path, which is a different rejection
mechanism.

### 5.2 Blocker corrections that do not change decidability

**CT03/CT04/CT05/CT11 — share captures now parse.** Batch 2 said fractional
shares "have no executable source form". They do have a source form:
`read <place> share <expr>` parses and name-resolves, with a real share grammar
(`FULL`, names, integers, `+`, parens) at
`crates/qcore-frontend/src/frontend_tasks.rs:206-226,347-380`. The refusal is
one layer deeper, at task lowering:

```
worker(read buffer share FULL)
→ Q500: only copy i32 and move own<Buffer> captures are executable
```

Bucket moves from *unimplemented-syntax* to *blocked-on-task-lowering*. One
change unblocks four rows.

**CT26–CT34 (+CT46, CT48) — the blocker moved but the bucket did not.** Batch 2
blamed the guard name (`with lock m as g` → `Q013` on `g`); `w6-frontend-gates`
repaired guard binding. The residual blocker is that **no source form creates or
declares a mutex at all**:

```
with lock m as g { … }         → Q013: unknown name `m`
let m = qcore_mutex_new();     → Q013: unknown name `qcore_mutex_new`
mutex m;                       → Q000: expected a QCore declaration
```

**CT17/CT18 — reclassified** from *blocked-on-W3/W4* to
*blocked-on-typed-error-payloads*: the obstacle is the task error model
(payload-free errors), not obligation vocabulary. `raise WorkerError { … }`
fails at `Q013` because no such declared error type carries a payload.

**CT36 — the conditional half is resolved.** `Result` match arms are a
conditional, so batch 2's "the subset has no conditional" no longer holds. Byte
quantities and the accounting oracle remain; two simultaneously live allocations
currently reject with `Q600` on the second `?` edge.

**CT49 — the assumption batch 2 flagged got stronger, not weaker.** The native
task runners compare stdout **byte-for-byte** with
`render_task_observation(reference)` (`ct_corpus.rs:1140-1145`), which is the
exact-total-order commitment CT49 exists to challenge.

---

## 6. Proposed stage-derived schema (report-only)

Full specification in `coverage.json` → `proposed_manifest_schema`. Summary:

**Per-stage entries become objects, not strings:**

```json
"native_execution": {
  "status": "executed",
  "executor": "compiler/tests/ct_corpus.rs::run_native_task_scenarios",
  "artifact": "compiler/tests/fixtures/ct/CT16/case.qcore",
  "host_gate": ["linux"],
  "observations": 4
}
```

**Status vocabulary** — `executed` · `unexecuted` · `recorded_not_gated`
(requires a `record` pointer; this is where the ASan transcripts belong) ·
`not_applicable` (so an *unreachable* stage is distinguishable from a *missing*
one) · `unassessed`.

**Stage vocabulary** replaces `cfg` and the overloaded `type_checker` with the
passes that exist: `parse`, `elaborate`, `task_lowering`, `task_ledger_check`,
`sequential_whole_program`, `sequential_reference_execution`,
`task_reference_execution`, `c_emission`, `native_execution`,
`native_sanitized`, `cli_end_to_end`, `legacy_c_twin` (reserved for
CT40–CT43/CT45), `proof`.

**Case-level `controlled_by`** makes CT02→CT01 a harness-asserted edge instead
of a sentence (repairs O5).

**Fail-closed rules**, in the spirit of the existing `MAPPED_CASES` pin:
`executed` without an `executor` and `artifact` fails; `recorded_not_gated`
without a `record` fails; a mapped case with nothing executed past `parse`
fails; the executor set named across all cases must equal the harness's own
runner registry; unknown stage names or statuses fail.

**Summarizer derivation** — counts come from executed stage evidence, never from
`mapping_status` alone:

- `ct_stage_histogram` — per stage, cases by status, *evaluated for the
  reporting host*. Today: `native_execution: executed 9 (linux) / not_applicable
  2`; `proof: unassessed 11`.
- `ct_depth_profile` — each mapped case bucketed by its deepest executed stage.
  Today: `source_reject 2 · native 9 · native_sanitized 0 (5 recorded) · proved 0`.
- `ct_observation_counts` — summed `observations`. Today: 18 / 19 / 11.
- `ct_mapped` — retained, but emitted *only* beside the histogram and an
  explicit note that mapping is not conformance.
- `ct_host_scope` — so a non-Linux report reads `unexecuted` instead of
  inheriting `executed`.

The summarizer emits **no counts at all** if any fail-closed rule is violated.
`acceptance.json`'s two scalars become generated per-CT rows plus the histogram
and depth profile, and no row may claim depth its stage evidence does not carry.

---

## 7. What this card does not establish

- It re-ran the CT harness and one negative probe; it did not run the full
  standing gate, and quotes no suite counts as re-executed evidence.
- The three newly-decidable cases are *assessed as decidable from probes*, not
  mapped. Two of the three would record a coarser reason than the spec row
  states; that caveat is part of the finding, not a footnote to it.
- No fail-closed gate, pinned expectation, verdict, CT mapping or
  Safe/Unsafe/Unknown wording was changed. All 11 raw mappings and the
  185-unit audit results are untouched, and remain unpromoted.
