# T03-CT — integrating the `source.mutex.guard.v1` CT mappings

Date: 2026-09-08. Base checkpoint: `9478b94` (the merged delegated increment).
Depends on [T03](../remaining-t03/README.md) (the lowering being mapped),
[T02](../remaining-t02/README.md) (the checker it lowers onto),
[T00](../remaining-t00/README.md) (the stage-derivation rule this card obeys)
and the [W6 CT harness](../w6-ct-harness/README.md).

**Scope.** Harness, fixtures, manifest and the two acceptance records. No
language, checker, lowering or backend semantics changed; no diagnostic code was
added, removed or altered. Nothing is promoted: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, the contract
corpus stays 59 modules, `qcore-schemas` stays 4.0.0 and full language
conformance stays false.

**Result.** CT coverage moves **11/50 → 15/50 mapped, 35 unmapped**. Three of
T03's four proposed mappings are applied as proposed; the fourth (CT26) is
applied with **more** executed evidence than proposed, because its sanitized
native run is now gated by the harness rather than recorded as a transcript.

---

## 1. Per-case disposition

### 1.1 Mapped (4)

| Case | Scenario (spec §55) | Expected | Fixture | Verdict mode | Stages claimed `executed` |
|---|---|---|---|---|---|
| **CT26** | Two critical sections use the same mutex for the same payload | Accept | `compiler/tests/fixtures/ct/CT26/case.qcore` | `checked_mutex_sanitized` | parser, type_checker, reference_execution, native_backend |
| **CT27** | Two different mutex identities allegedly guard the same payload | Reject without a valid composite protocol | `.../CT27/case.qcore` | `reject` | parser, type_checker |
| **CT29** | Typed error leaves a temporarily broken mutex invariant | Reject unless restored before unlock | `.../CT29/case.qcore` | `reject` | parser, type_checker |
| **CT30** | Guard-derived borrow escapes critical section | Reject | `.../CT30/case.qcore` | `reject` | parser, type_checker |

All four carry `evidence_target: "linux"` and `cfg: "unassessed"`.

**CT26 — measured.** `checked 1 function(s); entry main; checked mutex guards
(Linux C11)`, guards 1 and 2 minted by their own sections. The harness asserts
`SourceExecutionProfile::CheckedMutexGuardsV1` (another route cannot satisfy the
case), `ConcurrencyProfile::CheckedMutexInvariantV1` on the published
certificate, and the **absence** of `GuardFootprintAuthority` and
`SynchronizedCustodyEdges` — so a `TrustedExplicitTrace` certificate can never
discharge this case. It then pins the complete 14-line reference observation
computed by `interpret_mutex_program` over the checked event list, and requires
the sanitized native run's stdout to equal it line for line:

```
resource 1 4 · create 1 · lock 1 1 · open 1 · write 1 0 4 · close 1 · unlock 1 ·
lock 1 2 · borrow 1 · read 1 0 4 16 · end-borrow 1 · unlock 2 ·
destroy-mutex 1 · destroy 1
```

**CT27 — measured.** Exact ordered cascade `["Q617","Q600"]`:
`Q617 FootprintAlreadyProtected { event: 1, mutex: 2 }` at the second
`mutex guards`, then `Q600 UnknownMutex { event: 2, mutex: 2 }` at scope exit,
because the refused generation is unknown to the destruction sweep.

**CT29 — measured.** Exact ordered cascade
`["Q614","Q619","Q600","Q600","Q600"]`: `InvariantOpenAtErrorExit`,
`NonquiescentMutexDestruction`, `NonquiescentDestruction`, `OutstandingGuard`,
`OutstandingMutex`.

**CT30 — measured.** Exact ordered cascade
`["Q615","Q619","Q600","Q600","Q600"]`: `GuardBorrowEscape` leading the same
fail-closed tail.

### 1.2 Examined and deliberately left unmapped (7)

T03 declined CT28, CT31, CT32, CT33, CT34, CT46 and CT48. Each was re-checked
against its scenario definition here and the decision stands. The reasons are
recorded per case in
[`ct_stage_evidence.json`](ct_stage_evidence.json) →
`cases_examined_and_left_unmapped`; the two that were closest to admissible:

* **CT28** (guard moved to a child / unlocked by another thread). The source
  refusal is `Q600 CompletionCapsuleComposition` — "this module creates a task,
  which this profile does not admit". That is a *route* refusal, strictly weaker
  than the scenario, which asks that a guard specifically not travel. The
  `Q612 WrongGuardAffinity` evidence exists but is model-level: T03 reaches it by
  splicing `Spawn` plus a child `Access` into the **lowered** program, not from
  source. Mapping CT28 would claim source evidence the corpus does not have.
* **CT31** (mutex destroyed with an outstanding borrower/waiter).
  `Q619 NonquiescentMutexDestruction` *is* executed — but only as the second
  entry of the CT29 and CT30 cascades, consequential to a different refusal
  rather than as the case's own mechanism, and no source form produces a waiter
  at all. Mapping it would double-count CT30's evidence.

**A blocker label is now stale, and is corrected here.** T00's triage put all
eleven of CT26–CT34/CT46/CT48 in the bucket `blocked-on-unimplemented-syntax
(no mutex value)`. A mutex source form now exists, so that reason is false for
the seven that remain. The bucket is renamed
`blocked-on-guard-profile-composition-or-unrepresented-form` (7) in both
acceptance records, and the real blockers are: task composition inside the guard
profile (CT28, CT31's waiter half, CT32), unrepresented forms (CT33 lock ranks,
CT34 protection growth, CT48 guard-scoped facts) and `Send`/`Sync` derivation
(CT46).

---

## 2. Why CT26 does not claim `cfg`

T03 proposed `cfg: unassessed` for CT26 and that is applied unchanged. This
route runs

```
parse → elaborate → lower_source_mutex → check_mutex_invariant_program
      → recheck_mutex_invariant_certificate → emit_mutex_c11
```

and emits **directly from the checked event list**. There is no CFG construction
anywhere on it, so `cfg: executed` would not be a generous reading — it would be
false. The harness enforces this structurally: the `checked_mutex_sanitized`
verdict mode's allowed stage set omits `cfg`, and the manifest's per-stage
statuses must equal the runner's set exactly. Flipping CT26's `cfg` to
`executed` in the manifest was probed and fails the harness (§5).

This is also why the stage-derived acceptance rows gain three names
(`mutex_lowering`, `mutex_invariant_check`, `mutex_reference_execution`) rather
than reusing `task_lowering` / `task_ledger_check` / `task_reference_execution`,
which name a different IR and a different interpreter. The **manifest schema is
unchanged at version 1** with its six stage names; the richer vocabulary lives
only in the stage-derived summaries, exactly where T00 put it.

---

## 3. Fixture inventory

Ten new files under `compiler/tests/fixtures/ct/`, all executed. The harness's
pre-existing inventory assertion requires the directory contents to equal the
set of files the expectations actually consume, so an unused fixture fails the
gate.

| Path | Role | Outcome |
|---|---|---|
| `CT26/case.qcore` | two sections, one generation, one payload | accept + sanitized native run |
| `CT26/expected.json` | pinned observation, guards, sanitizer flags, emitted-C substrings | — |
| `CT27/case.qcore` | second `mutex guards` over one payload | reject `[Q617, Q600]` |
| `CT27/disjoint_payloads.qcore` | **control**: two generations, two disjoint payloads | accept |
| `CT27/expected.json` | cascade + control table | — |
| `CT29/case.qcore` | typed error with the invariant open | reject `[Q614, Q619, Q600, Q600, Q600]` |
| `CT29/restored.qcore` | **control**: same error raised after `close g;` | accept |
| `CT29/expected.json` | cascade + control table | — |
| `CT30/case.qcore` | borrow live at the section end | reject `[Q615, Q619, Q600, Q600, Q600]` |
| `CT30/released.qcore` | **control**: same borrow dropped inside its section | accept |
| `CT30/escaped_borrow.qcore` | **control**: borrow binder named after its block | reject `[Q013]` |
| `CT30/expected.json` | cascade + control table | — |

Six of the `.qcore` files (`CT26/case`, `CT27/case`, `CT29/case`,
`CT29/restored`, `CT30/case`, `CT30/released`) are **byte-identical** to T03's
archived candidates under `docs/reports/remaining-t03/fixtures/`, verified with
`diff`. Those archived copies are left in place: an archived evidence package is
not rewritten. `compiler/tests/fixtures/ct/` is now the authoritative,
gate-executed copy. Two controls are new here
(`CT27/disjoint_payloads.qcore`, `CT30/escaped_borrow.qcore`).

### 3.1 Why every rejection carries a control

A rejection alone does not distinguish "the checker refused *this* construct"
from "the profile cannot express the construct at all". Each control isolates
the offending element:

* CT27's control shows `Q617` is keyed to the **shared payload**, not to
  declaring a second mutex — two generations over disjoint payloads check.
* CT29's control is the scenario's own `unless restored` half.
* CT30's `released.qcore` shows `Q615` is keyed to the **escape**, not to
  borrowing under a guard.
* CT30's `escaped_borrow.qcore` exercises the second, lexical escape route and
  is recorded honestly as the **weaker** mechanism: it fails at `Q013 unknown
  name` because the binder is body-local. `Q013` is name resolution and is
  deliberately **not** relabelled as a concurrency verdict — that separation is
  T03's, and this card preserves it rather than improving the apparent result.

---

## 4. Harness changes

All in `compiler/tests/ct_corpus.rs`; no production file was touched.

* `MAPPED_CASES` grows to fifteen; two new pins, `MUTEX_REJECT_CASES` and
  `MUTEX_SANITIZED_CASE`, bind the new metadata to exactly its own cases.
* One new verdict mode, `checked_mutex_sanitized`, with its own allowed stage
  set (no `cfg`).
* `CheckedMutexSanitized` carries the pinned reference observation, the guard
  ids, the exact sanitizer flag vector, both sanitizer option strings and the
  emitted-C substrings. The harness asserts the flag vector really requests
  `-fsanitize=address,undefined` and that `ASAN_OPTIONS` really keeps
  `detect_leaks=1`, so the sanitizer cannot be quietly disabled by editing data.
* `CheckedMutexReject` carries the exact **ordered diagnostic code vector** and
  the control table. The reject arm now compares the full vector, not only a
  containment check on selected entries. This is the point: a refused release
  that silently dropped the guard would still satisfy a head-only assertion,
  while the tail (`Q619`, `NonquiescentDestruction`, `OutstandingGuard`,
  `OutstandingMutex`) is exactly the evidence that it did not.
* The `evidence_target` assertion is restructured. It previously read "only the
  native fault mode carries target-scoped evidence"; it now reads "a case
  carries `evidence_target` exactly when a host-gated runner **or control**
  executes for it". CT27/CT29/CT30's accepted controls compile through the
  Linux-only emission path, so they are host-gated even though their own
  rejections are not. Recording that in the machine-readable field is a direct
  answer to T00's finding **O6**, which was that CT19's Linux-gated control
  evidence appears nowhere in the manifest.
* Non-Linux hosts take the existing `unsupported_target` path (`Q500`, message
  `requires a Linux C11 threads target`) and print an explicit "unexecuted on
  this host" line, for the sanitized case and for every accepted control. No
  silent skip.

---

## 5. Fail-closed probes

Each mutation was applied, the harness run, then reverted.

| Probe | Result |
|---|---|
| `QCORE_CLANG=/nonexistent` | **FAILED** at `ct_corpus.rs:1241` — the native stage is load-bearing corpus-wide (this reproduces T00's own probe) |
| CT26 `reference_observation` last line `destroy 1` → `destroy 2` | **FAILED**: "CT26: reference observation drifted from the pinned expectation" |
| CT26 `sanitizer_flags` + `--qcore-probe-invalid-flag` | **FAILED** at the compile: "C compiler rejected the emitted program" — the sanitized native compile really runs |
| CT26 manifest `cfg: unassessed` → `executed` | **FAILED**: "CT26: manifest stage `cfg` must be `unassessed` to match the executed fixture" |
| CT29 manifest `evidence_target` deleted | **FAILED**: "CT29: target metadata mismatch" |

Pre-existing fail-closed behaviour retained and unchanged: a fixture directory
whose manifest row still reads `unmapped` fails; a mapped case without a fixture
fails; an unknown `mapping_status` or verdict vocabulary fails; the fixture
inventory must equal the consumed file set.

---

## 6. Acceptance records

* `docs/reevaluation/ct_manifest.json` — the four rows move to `mapped` with
  their fixture path, semantic profile, per-stage statuses and
  `evidence_target`. The manifest `note` now states that two source routes are
  represented and that `cfg` is meaningless on the mutex route. Schema stays
  version 1; no stage name was added to the manifest.
* `docs/reviews/r0_3/acceptance.json` — `ct_mapped: 15`, `ct_unmapped: 35`, and
  the `ct_evidence` block's histogram/depth profile/observation counts/bucket
  counts are updated. Its note keeps T00's O7 finding intact (the scalar is
  still binary) and corrects only the sentence that was made false: **1 of 15**
  mapped cases now carries gated sanitized-native evidence, and **0 of 15**
  carry proof evidence.
* `docs/reviews/r0_3/acceptance_matrix.json` — the same aggregates, plus a
  retired `T03-CT` follow-up row and a new `T03-CT-REST` row naming the seven
  cases left unmapped and their corrected blockers.
* `ct_stage_evidence.json` (new, this directory) — per-case, per-stage executed
  evidence for the four cases in the shape T00's `coverage.json` uses, with
  executor and artifact pointers, host gates and observation counts, and
  controls recorded under `aux:` so a control's execution is never counted as
  the case's own stage.

### 6.1 One generator is now out of date — recorded, not worked around

`docs/reports/remaining-d02/build_matrix.py` derives the matrix's CT section
from T00's archived `coverage.json` alone and asserts that its mapped set equals
the manifest's. That assertion no longer holds. The matrix was already
hand-edited at `9478b94` (provider tests 47 → 51, retired follow-up rows), so
this card follows the same hand-maintained route and does **not** regenerate,
which would discard the integrator's edits. `ct_stage_evidence.json` exists so
that a future stage-derived summarizer can merge the two files instead of
hand-maintaining the aggregate. The matrix now carries a `generator_status`
field saying exactly this. The work itself belongs to the existing
`T00-CT-SCHEMA` follow-up and is not silently absorbed here.

---

## 7. Gate results

Run at this card's tip, on Linux. See §7.1 for a material caveat about the
worktree.

| Gate | Result |
|---|---|
| `cargo test --workspace` | see §7.1 |
| `cargo fmt --all -- --check` | see §7.1 |
| `cargo clippy --all-targets -- -D warnings` | see §7.1 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | see §7.1 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | see §7.1 |
| `python3 -B -m unittest discover -s scripts/tests` | see §7.1 |
| `cd proof && lake build` | see §7.1 |
| `python3 -B scripts/check_lean_audits.py` | see §7.1 |
| `cargo test -p qcore-compiler --test ct_corpus` | **4 passed, 0 failed** (11.26 s with a cold sanitized compile) |

### 7.1 The worktree was shared with another in-flight agent

This card's worktree (`.claude/worktrees/cycle-19-task-delegation-68c24d`,
branch `claude/cycle-19-task-delegation-68c24d`) was observed carrying another
agent's uncommitted, in-progress work during this session:
`crates/qcore-c-gateway/src/{legacy_c.rs,lib.rs,typed_field_access.rs}`, a new
untracked `crates/qcore-c-gateway/src/borrowed_actual.rs`, an untracked
`docs/reports/remaining-p07/` and `docs/design/W5_LIBRDKAFKA_CAPABILITY.md`. At
one point that work did not compile
(`error[E0433]: use of undeclared type UntrackedBases` and two related errors in
`legacy_c.rs`), which blocked the workspace gate for reasons unrelated to this
card. Those files are **not** part of this card's commit — only the paths listed
in §8 were staged. The full-gate numbers recorded above were taken with that
foreign work present in the working tree, and are therefore *not* a measurement
of this card's change in isolation; the CT harness result is, since
`ct_corpus.rs` and the CT fixtures are disjoint from the C gateway. This is
recorded as an environment condition, not repaired here.

A second, unrelated pre-existing flake stands: `qcore-verifier-host`'s
`a_declared_process_limit_stops_the_fixture_from_forking` (P02) is
load-dependent because it installs `RLIMIT_NPROC = 1`, which counts every
process of the real UID. It is recorded in
[`remaining-p03` §9.1](../remaining-p03/README.md) with the assertion
deliberately not weakened, and nothing here touches it.

---

## 8. Files changed

```
compiler/tests/ct_corpus.rs                          modified (harness only)
compiler/tests/fixtures/ct/CT26/{case.qcore,expected.json}
compiler/tests/fixtures/ct/CT27/{case.qcore,disjoint_payloads.qcore,expected.json}
compiler/tests/fixtures/ct/CT29/{case.qcore,restored.qcore,expected.json}
compiler/tests/fixtures/ct/CT30/{case.qcore,released.qcore,escaped_borrow.qcore,expected.json}
docs/reevaluation/ct_manifest.json                   4 rows mapped, note extended
docs/reviews/r0_3/acceptance.json                    CT scalars + ct_evidence
docs/reviews/r0_3/acceptance_matrix.json             ct_conformance + follow-ups
docs/reports/remaining-t03-ct-integration/{README.md,ct_stage_evidence.json}
```

No file under `crates/`, `compiler/src/`, `contract/`, `proof/`, `providers/` or
`scripts/` was modified. `AGENTS.md` and `CLAUDE.md` are integrator-owned and
untouched.

---

## 9. What this card does not establish

* **Mapping is not conformance.** Fifteen of fifty CT cases are mapped; thirty-
  five are not. `proof` is `unassessed` for all fifteen and no Lean theorem is
  bound to any CT fixture — T03's fourteen theorems constrain the Lean lowering
  functions, and source extraction is an explicit premise there.
* **CT26 is one thread, one Linux target, one compiler invocation.** `mtx_lock`
  is never contended. Nothing here is evidence about scheduling, fairness,
  publication or the C memory model.
* **Source-to-trace refinement remains a premise.** That `lower_source_mutex`
  produces the trace the source means is unproved in both Rust and Lean. Every
  certificate this route issues carries `DeclaredFootprintShape` and
  `InvariantPredicateDeclared`; `close g;` is what puts the second one there,
  and the invariant predicate has no runtime representation in the sanitized
  run.
* **CT30 makes no claim that a guard is unmovable.** This profile refuses task
  creation outright, so a borrow cannot travel to a child here at all. That is
  why CT28 stays unmapped.
* **No promotion, no semantic change.** No verdict, no diagnostic code, no
  pinned artifact, no contract count, and no raw Cycle 19 unit changed. The four
  pinned 185-unit artifacts were not re-measured and were not touched;
  `CORPUS-REPIN` remains open.
