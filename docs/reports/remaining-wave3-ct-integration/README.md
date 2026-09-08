# wave3-CT — integrating the T05 and T06 CT mappings

Date: 2026-09-08. Base checkpoint: `8c62294` (the third delegated wave's tip,
containing T04, T05, T06, T08, O03 and P08). Direct precedent:
[T03-CT](../remaining-t03-ct-integration/README.md), whose harness discipline,
`generator_status` handling and fail-closed probe method this card follows.
Depends on [T05](../remaining-t05/README.md) and [T06](../remaining-t06/README.md)
(the lowerings being mapped), [T08](../remaining-t08/README.md) (whose four CT
cases stay unmapped on its own instruction), [T00](../remaining-t00/README.md)
(the stage-derivation rule) and the [W6 CT harness](../w6-ct-harness/README.md).

**Scope.** Harness, fixtures, manifest and one acceptance record. No language,
checker, lowering or backend semantics changed; no diagnostic code was added,
removed or altered; **no file under `crates/`, `compiler/src/`, `contract/`,
`proof/`, `providers/` or `scripts/` was modified**, and `AGENTS.md`,
`CLAUDE.md` and `docs/reviews/r0_3/acceptance_matrix.json` are integrator-owned
and untouched (§7 proposes their text).

**Result.** CT coverage moves **15/50 → 20/50 mapped, 30 unmapped**, by
executed-stage evidence only. All five cases T05 and T06 proposed are applied.
CT11 stays unmapped on T05's own caveat; CT35/CT36/CT38/CT39 stay unmapped on
T08's instruction. Nothing is promoted: raw Cycle 19 units stay 2 Safe / 42
provisional Unsafe / 137 Unknown plus four data-only units, the contract corpus
stays 59 modules, `qcore-schemas` stays 5.0.0 and full language conformance
stays false.

---

## 1. Per-case disposition

### 1.1 Mapped (5)

| Case | Scenario (spec §55) | Expected | Route | Verdict mode | Stages claimed `executed` |
|---|---|---|---|---|---|
| **CT03** | Parent and child share reads with complementary positive shares | Accept | `ScopedReadSharesV1` | `native_heap_scenarios` | parser, type_checker, reference_execution, cfg, native_backend |
| **CT04** | Parent writes while child retains a read share | Reject | `ScopedReadSharesV1` | `reject` | parser, type_checker |
| **CT05** | Two children each request FULL for the same memory | Reject | `ScopedReadSharesV1` | `reject` | parser, type_checker |
| **CT17** | Worker error payload contains owned result | Join transfers it exactly once | `ScopedErrorPayloadV1` | `native_heap_scenarios` | parser, type_checker, reference_execution, cfg, native_backend |
| **CT18** | Worker error payload contains scoped capture borrow | Reject | `ScopedErrorPayloadV1` | `reject` | parser, type_checker |

All five carry `evidence_target: "linux"` and `proof: "unassessed"`. Per-stage
executed evidence, with executor and artifact pointers, is in
[`ct_stage_evidence.json`](ct_stage_evidence.json).

**CT03 — measured.** `checked 2 function(s); entry main; scoped owned-buffer
tasks with counted read shares (Linux C11)`, exit 0. Three fixed Linux
schedules compile the case's *own* emitted C under
`-fsanitize=address,undefined` and run it, with a heap witness printing every
Buffer allocation and release by address:

| Schedule | Injection | Exit | Heap | Observation |
|---|---|---|---|---|
| `two_readers_share_one_allocation` | none | 0 | 1 allocated, 1 released | stdout equals the reference observation line for line; ordered `WRITE 42 < LOAN 1 < LOAN 2 < RETURN 1 < RETURN 2 < WRITE 7 < READ 7 < DROP` |
| `second_creation_refuses` | only the second `thrd_create` fails | 0 | 1 / 1 | stdout equals the reference for `[true, false]`; no second loan, and the failure arm never regains write authority |
| `join_refuses` | `thrd_join` fails | 77 | 1 / **0** | both loans opened, no `RETURN`, no `JOIN`, no second write; `forbidden_free` would have exited 91 |

**CT04 — measured.** Exact ordered cascade `["Q600"]`:
`LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2, 3] }` at the
write between the two creations.

**CT05 — measured.** Exact ordered cascade `["Q600"]`:
`ShareExceedsAuthority { resource: 1, outstanding: 1024, requested: 1024 }` at
the second `spawn`.

**CT17 — measured.** `checked 2 function(s); entry main; scoped owned-buffer
tasks with one typed error payload (Linux C11)`, exit 0. Five fixed Linux
schedules:

| Schedule | Injection | Exit | Heap | Observation |
|---|---|---|---|---|
| `success_releases_everything_inside_the_child` | none | 0 | 3 / 3 | `QCORE_TASK_RESULT 1 I32 42`, no `ERROR_BUFFER` |
| `payload_crosses_the_join_exactly_once` | worker allocation 1 fails | 0 | 1 / 1 | one `ERROR_BUFFER Recovery::Failed 1`; ordered `JOIN 1 < READ 0 1 42 < DROP 0 1` |
| `partial_acquisition_releases_the_local_it_does_not_carry` | worker allocation 2 fails | 0 | 2 / 2 | the local is acquired and released inside the child before the tagged completion; generation 1 is never dropped by the child |
| `creation_refuses_and_transfers_nothing` | `thrd_create` fails | 0 | 1 / 1 | `QCORE_TASK_CREATE_FAILED 1`, no `ERROR_BUFFER` |
| `join_refuses_and_reclaims_nothing` | worker allocation 1 fails, then `thrd_join` fails | 77 | 1 / **0** | spawn published, no `JOIN`, no `ERROR_BUFFER`, no parent read |

**CT18 — measured.** Exact ordered cascade `["Q500"]`: `an error payload
requires own<Buffer>` at the `raise` that writes it. This is the frontend code
`source_tasks::failure` already assigns at that site; nothing was relabelled.

### 1.2 Examined and deliberately left unmapped

**CT11 — the T05 caveat is the reason, and it stands.** T05 §9 proposed CT11
with the measured refusal `Q600 LoanOutstanding { resource: 1, outstanding:
512, borrowers: [3] }`, and flagged that the manifest wording is "full
permission reconstructed while local shared **aliases** survive" while
`ScopedReadSharesV1` has no local alias form at all — the party retaining the
share is an unjoined child *task*. The refusal direction is the one the case
asks for; the retaining party is not. Under a strict reading of the scenario
this is intra-frame aliasing, and mapping the case would claim alias evidence
the corpus does not have. **CT11 stays unmapped**, recorded in the manifest
`note`, in `acceptance.json`'s `newly_decidable_not_mapped_reasons` and in
`ct_stage_evidence.json` → `cases_examined_and_left_unmapped`. It unblocks when
an owned-pointer alias form exists — the same gap that keeps CT06 unmapped.

**CT35, CT36, CT38, CT39 — T08's instruction, applied verbatim.** T08 §8.1 asks
that these four not be marked mapped: their scenarios line up exactly with
`FlatEpochBoundV1`'s four rules and each is now checked at model level with a
named diagnostic, but there is **no epoch source syntax**, so there is no parser
stage, no source fixture and no reference execution. Mapping them would claim
stages the corpus cannot execute. They stay unmapped and are recorded as a new
bucket, `blocked-on-model-only-epoch-accounting (no source syntax)` (4).

**CT06 and CT35 (share half) — T05 declined both, and that stands.** CT06 needs
an owned-pointer copy form the grammar refuses outright; CT35's "count 4096
once" needs byte-size accounting this profile does not perform, discharged
instead as one allocation counted once by address.

**CT24 and CT25** are T04's own proposals, outside this card's five-case scope
and unchanged by it.

---

## 2. One deliberate deviation from T05's proposal: CT03's `cfg`

T05 §9 proposed `cfg: unassessed` for CT03, reasoning that the route emits from
the checked task IR and has no *sequential* CFG stage. **This card records
`cfg: executed` instead, deliberately, and this section is the record.**

The manifest's own `note` defines what `cfg` names per route: task-route cases
read it as the checked task IR. Every existing task-route case claims it —
CT01, CT12, CT13, CT14, CT15, CT16, CT19, CT20, CT37, CT50 — and none of them
builds a sequential CFG either; they construct exactly the same checked task IR
CT03 does. T06 §7.1 independently proposed `cfg: executed` for CT17 on that same
route. Choosing T05's reading for CT03 alone would have made it the single
task-route case answering a different question with the same field name, while
adding a verdict mode whose only purpose was to withhold a stage its sibling
cases claim.

The alternative is not hidden: T05's proposal is reproduced here, the manifest
note now states both route readings explicitly, and `ct_stage_evidence.json`
records `route_reads_cfg_as_the_task_ir` pointing back at this section. An
integrator who prefers T05's reading should change it for the whole task route,
not for CT03 alone. The mutex route is untouched and still never claims `cfg`
(§5 probe 3 shows the harness enforces both directions).

---

## 3. Fixture inventory

Eighteen new files under `compiler/tests/fixtures/ct/`, all executed. The
harness's pre-existing inventory assertion requires each directory's contents to
equal the set of files the expectations actually consume, so an unused fixture
fails the gate.

| Path | Role | Outcome |
|---|---|---|
| `CT03/case.qcore` | 512 + 512 over one allocation, writes outside the window | accept + 3 sanitized native runs |
| `CT03/track.h` | heap witness (`SHARE_ALLOC`/`SHARE_FREE`) | — |
| `CT03/refuse_second_create.h` | only the second `thrd_create` fails | — |
| `CT03/refuse_join.h` | `thrd_join` fails; every release exits 91 | — |
| `CT03/expected.json` | schedules, heap counts, ordering, sanitizer flags | — |
| `CT04/case.qcore` | write while both shares are out | reject `[Q600]` |
| `CT04/write_after_joins.qcore` | **control**: same write after both joins | accept |
| `CT04/expected.json` | cascade + control table | — |
| `CT05/case.qcore` | two `share FULL` requests | reject `[Q600]` |
| `CT05/remainder_fits.qcore` | **control**: `share 511` + `share 513` | accept |
| `CT05/expected.json` | cascade + control table | — |
| `CT17/case.qcore` | tagged payload carrying one `own<Buffer>` | accept + 5 sanitized native runs |
| `CT17/track.h` | heap witness (`PAYLOAD_ALLOC`/`PAYLOAD_FREE`), `-D`-selected failure | — |
| `CT17/refuse_create.h` | `thrd_create` fails | — |
| `CT17/refuse_join.h` | allocation 1 and then `thrd_join` fail; every release exits 91 | — |
| `CT17/undelivered_payload.qcore` | **control**: handler returns without taking | reject `Q600` |
| `CT17/unreleased_payload.qcore` | **control**: taken payload never released | reject `Q600` |
| `CT17/expected.json` | schedules, heap counts, ordering, controls | — |
| `CT18/case.qcore` | `borrow<Buffer>` in the payload position | reject `[Q500]` |
| `CT18/owned_payload.qcore` | **control**: same shape with `own<Buffer>` | accept |
| `CT18/expected.json` | cascade + control table | — |

`CT03/case.qcore`, `CT04/case.qcore` and `CT05/case.qcore` are **byte-identical**
to T05's archived candidates under `docs/reports/remaining-t05/fixtures/`,
verified with `diff`. `CT17/case.qcore` is byte-identical to T06's canonical
`PAYLOAD` constant in `compiler/tests/error_payload_tasks.rs`, verified by
extraction and comparison. The archived copies are left in place: an archived
evidence package is not rewritten. `compiler/tests/fixtures/ct/` is now the
authoritative, gate-executed copy.

### 3.1 Why every rejection carries a control

A rejection alone does not distinguish "the checker refused *this* construct"
from "the profile cannot express the construct at all". Each control isolates
the offending element, and each must select a **named execution profile**, so a
control cannot quietly pass through a different route:

* CT04's control shows `Q600 LoanOutstanding` is keyed to the outstanding
  shares, not to `qcore_buffer_write` — the identical program with the write
  moved after both joins checks.
* CT05's control shows `ShareExceedsAuthority` is a budget boundary, not a
  blanket refusal of two borrowers: `511 + 513 = 1024` checks, so the exact
  remainder always fits.
* CT18's control shows `Q500` is keyed to the borrowed capture, not to tagged
  error payloads: the same shape with `own<Buffer>` moved into the worker
  checks.
* CT17's two rejection controls are the negative half of its acceptance: an
  undelivered payload and an unreleased taken payload are refused separately,
  with the two distinct `Q600` messages.

---

## 4. Harness changes

All in `compiler/tests/ct_corpus.rs`; no production file was touched.

* `MAPPED_CASES` grows to twenty; two new pins, `SOURCE_REJECT_CASES`
  (CT04/CT05/CT18) and `HEAP_SCENARIO_CASES` (CT03/CT17), bind the new metadata
  to exactly its own cases.
* One new verdict mode, `native_heap_scenarios`, sharing the task route's
  five-stage allowed set with the existing native task modes (§2).
* `NativeHeapScenarios` carries the required `SourceExecutionProfile` label, the
  heap witness's allocation/release markers, the exact sanitizer flag vector,
  the UBSan option string, the rejection controls and the schedules.
  `HeapScenario` carries its header, `-D` options, the reference-execution
  script (spawn, root-allocation and per-worker-allocation outcomes),
  `detect_leaks`, the expected exit code, stdout containment/absence/ordering
  and the heap counts.
* **The sanitizer cannot be disabled by editing data.** The harness asserts the
  flag vector really requests `-fsanitize=address,undefined`, that UBSan really
  halts, that at least one schedule per case enables LeakSanitizer, and it
  *computes* `ASAN_OPTIONS` from `detect_leaks` rather than reading an option
  string. A schedule may only turn LeakSanitizer off when its exit code is
  nonzero — i.e. when the retained allocation is the observation.
* **A completing schedule must match the reference.** `exit_code == 0` is what
  makes stdout equal the independently computed reference observation line for
  line; a trapped abort is required to state what its truncated observation must
  *not* contain, so it cannot be a free pass.
* Released addresses are checked against allocated ones: a release of an address
  never allocated, or a second release of the same address, fails.
* `SourceReject` carries the exact **ordered diagnostic code vector** and the
  control table, matching `CheckedMutexReject`'s discipline on the task route.
  The mutex path is byte-identical; `run_source_controls` is a separate runner
  so the mutex route's assertions did not move.
* The `evidence_target` assertion is extended: a case carries it exactly when a
  host-gated runner **or control** executes for it, which now includes
  `native_heap_scenarios` and any `source_reject` with an accepted control.
* Non-Linux hosts take the existing `unsupported_target` path (`Q500`, message
  `requires a Linux C11 threads target`) and print an explicit "unexecuted on
  this host" line, for both heap-scenario cases and every accepted control. No
  silent skip. The CT17 rejection controls run on **every** host, before the
  Linux gate.

---

## 5. Fail-closed probes

Each mutation was applied, the harness run, then reverted.

| Probe | Result |
|---|---|
| CT03 schedule 1 `heap_allocations` 1 → 2 | **FAILED**: `CT03: two_readers_share_one_allocation allocated ["0x71730b9e0010"]` — a real address from a real sanitized run |
| `QCORE_CLANG=/nonexistent-compiler` | **FAILED** at `ct_corpus.rs:1814` (`NotFound`) — the native stage is load-bearing corpus-wide |
| CT03 manifest `cfg: executed` → `unassessed` | **FAILED**: "CT03: manifest stage `cfg` must be `executed` to match the executed fixture" |
| CT17 schedule 2 `worker_allocation_outcomes` `[false]` → `[true, false]` | **FAILED**: "CT17: payload_crosses_the_join_exactly_once native and reference observations must agree line for line" — the injected schedule really binds both sides |
| CT18 `diagnostic_sequence` `["Q500"]` → `["Q600"]` | **FAILED**: "CT18: exact diagnostic cascade mismatch" |
| CT05 control `execution_profile` → `ScopedOwnedTasksV1` | **FAILED**: "CT05 control remainder_fits.qcore must take the route it names" |

Pre-existing fail-closed behaviour retained and unchanged: a fixture directory
whose manifest row still reads `unmapped` fails; a mapped case without a fixture
fails; an unknown `mapping_status` or verdict vocabulary fails; the fixture
inventory must equal the consumed file set; and the mutex route still cannot
claim `cfg`.

---

## 6. Acceptance records

* `docs/reevaluation/ct_manifest.json` — the five rows move to `mapped` with
  their fixture path, semantic profile, per-stage statuses and
  `evidence_target`. The `note` now states both route readings of `cfg`
  explicitly and records the reasons CT11, CT06 and CT35/CT36/CT38/CT39 stay
  unmapped. **Schema stays version 1**; no field and no stage name was added, so
  the harness's `deny_unknown_fields` contract is unchanged. The manifest has no
  per-case reasons field, and one was deliberately not invented — adding it
  would be a schema change owned by the existing `T00-CT-SCHEMA` follow-up.
* `docs/reviews/r0_3/acceptance.json` — `ct_mapped: 20`, `ct_unmapped: 30`, and
  the `ct_evidence` block's histogram, depth profile, observation counts and
  bucket counts are re-derived. Its note keeps T00's O7 finding intact (the
  scalar is still binary) and corrects only what became false: **3 of 20** mapped
  cases now carry gated sanitized-native evidence, and **0 of 20** carry proof
  evidence. Three buckets are retired (`blocked-on-task-lowering (share
  captures)`, `blocked-on-typed-error-payloads`,
  `blocked-on-typed-error-payloads + no borrow captures`) and two are added
  (`blocked-on-scenario-mismatch (no local alias form)` for CT11,
  `blocked-on-model-only-epoch-accounting (no source syntax)` for
  CT35/CT36/CT38/CT39, moved out of `blocked-on-unimplemented-syntax`). The
  block's counting rule is now stated in the note, because this card re-derived
  the numbers by hand.
* `ct_stage_evidence.json` (new, this directory) — per-case, per-stage executed
  evidence for the five cases, in the shape T00's `coverage.json` uses, with
  executor and artifact pointers, host gates, per-schedule heap witnesses and
  observation counts. Controls are recorded under `aux:` so a control's
  execution is never counted as the case's own stage.

### 6.1 One stage is `not_applicable` rather than `executed`, on purpose

CT18 is refused during task **lowering** (`Q500`), so its program never reaches
the core task ledger. `task_ledger_check` is therefore `not_applicable` for
CT18, not `executed` and not `unassessed` — the closed T00 vocabulary
(`executed` / `not_applicable` / `recorded_not_gated` / `unassessed`) was not
extended. That is also why the depth profile gains a `task_lowering: 1` entry:
CT18's deepest executed stage is the lowering that refuses it.

### 6.2 The matrix generator is now further out of date — recorded, not worked around

`docs/reports/remaining-d02/build_matrix.py` derives the matrix's CT section
from T00's archived `coverage.json` alone and asserts its mapped set equals the
manifest's. T03-CT already recorded that this assertion no longer holds and set
`ct_conformance.generator_status` accordingly. This card adds five more mapped
cases to the divergence and does **not** regenerate, following the same route:
`ct_stage_evidence.json` exists so a future stage-derived summarizer can merge
the three files. `acceptance_matrix.json` is integrator-owned in this wave, so
§7.1 gives the exact replacement text rather than editing it. The work belongs
to the existing `T00-CT-SCHEMA` follow-up and is not silently absorbed here.

---

## 7. Integrator decisions

### 7.1 `docs/reviews/r0_3/acceptance_matrix.json` — proposed edits (not made here)

1. `ct_conformance.totals` → `{"cases": 50, "mapped": 20, "unmapped": 30}`.
2. `ct_conformance.derived_from` → add
   `docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json`.
3. `ct_conformance.stage_histogram`, `depth_profile`, `observation_counts` and
   `unmapped_bucket_counts` → copy the values now in
   `docs/reviews/r0_3/acceptance.json`'s `ct_evidence` block (they were derived
   together and agree by construction).
4. `ct_conformance.generator_status` → replace with:

   > `docs/reports/remaining-d02/build_matrix.py` derives this section from
   > T00's archived `coverage.json` alone, which covers only the original eleven
   > mapped cases and asserts its mapped set equals the manifest's. That
   > assertion has not held since T03-CT and now diverges by nine cases
   > (CT26/CT27/CT29/CT30 and CT03/CT04/CT05/CT17/CT18), so the section is
   > hand-maintained and the generator needs a merge step over the two
   > `ct_stage_evidence.json` files before it can regenerate. Recorded, not
   > silently worked around; `T00-CT-SCHEMA` already owns the stage-derived
   > summarizer work.
5. Add a `wave3-CT` row to `named_followups`/`updated` naming CT11 (blocked on
   an owned-pointer alias form) and CT35/CT36/CT38/CT39 (blocked on epoch source
   syntax) as this card's deliberately unmapped set.

### 7.2 Status-ledger text — proposed paragraph for AGENTS.md / CLAUDE.md

> **wave3 CT integration:** CT03, CT04, CT05, CT17 and CT18 are mapped with
> stage-derived evidence, moving CT coverage from 15/50 to **20/50 mapped**.
> CT03 and CT17 are the second and third harness-gated sanitized-native CT
> cases: eight fixed Linux schedules compile each case's own emitted C under
> ASan/UBSan/LSan with a heap witness recording every Buffer by address, and
> each completing schedule's stdout equals the independently computed reference
> observation line for line, while the two trapped-abort schedules pin that a
> refused join returns no share and reclaims nothing. CT04, CT05 and CT18 pin
> exact ordered diagnostic cascades, each with an accepted control that names
> the route it takes, so a refusal is never merely "the profile cannot express
> this". **CT11 stays unmapped** on T05's own caveat: the scenario asks about
> local shared *aliases* and this profile has none, the retaining party being an
> unjoined child task. **CT35, CT36, CT38 and CT39 stay unmapped** on T08's
> instruction: they are checked at model level by `FlatEpochBoundV1` but have no
> epoch source syntax, hence no parser stage and no reference execution. One
> deviation from a proposing card is recorded: CT03 claims `cfg: executed`,
> matching the manifest's task-route reading of `cfg` as the checked task IR
> that every other task-route case uses, rather than T05's proposed
> `unassessed`. No semantic or diagnostic-code change, no new Lean theorem, no
> promotion; the four pinned 185-unit artifacts were not re-measured and were
> not touched. Evidence:
> `docs/reports/remaining-wave3-ct-integration/README.md`.

---

## 8. Gate results

Linux, submodule initialised at the pinned `d82464a`, run at this card's tip in
this worktree.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1201 tests; 1200 passed, 1 failed, 1 ignored** — the single failure is the known P02 flake (§8.1), which passes on isolated re-run |
| `cargo test -p qcore-verifier-host --test enforced_process_limits` (re-run) | **12 passed, 0 failed** |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 -B -m unittest discover -s scripts/tests` | **81 tests, OK** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **58 passed, 0 failed** |
| `PATH="$HOME/.elan/bin:$PATH" (cd proof && lake build)` | **Build completed successfully (70 jobs)** |
| `PATH="$HOME/.elan/bin:$PATH" python3 -B scripts/check_lean_audits.py` | **Passed 26 Lean audits** (exit 0) |
| `cargo test -p qcore-compiler --test ct_corpus` | **4 passed, 0 failed** |

The workspace total is unchanged from the merged `8c62294` baseline, and that is
the expected result: **this card adds no `#[test]` function** (`git diff` on
`ct_corpus.rs` contains zero added `#[test]` lines). The five new cases execute
*inside* the existing `ct_manifest_and_fixture_set_agree_and_mapped_cases_execute`,
which is also why the count alone would not reveal whether they ran — the
fail-closed probes in §5 are what establish that.

**No new Lean theorem is required and none was written.** This lane is Rust
harness work over features that already carry their own Lean evidence
(`QCore.ReadShare`'s nineteen theorems for T05, `QCore.ErrorPayload`'s nineteen
for T06). The Lean gate was re-run above only to show it still passes; `proof/`
is untouched, and `proof` stays `unassessed` for all twenty mapped cases,
because no Lean theorem is bound to any CT fixture.

### 8.1 The one failure is a known pre-existing flake

`qcore-verifier-host`'s `a_declared_process_limit_stops_the_fixture_from_forking`
(card P02) is load-dependent because it installs `RLIMIT_NPROC = 1`, which
counts every process of the real UID; under a full parallel workspace run it
fails with `Broken pipe (os error 32)`. It passed on isolated re-run here. It is
recorded in [`remaining-p03` §9.1](../remaining-p03/README.md) with the
assertion deliberately not weakened, and nothing here touches it. This card
modifies no file under `crates/`.

---

## 9. Files changed

```
compiler/tests/ct_corpus.rs                              modified (harness only)
compiler/tests/fixtures/ct/CT03/{case.qcore,expected.json,track.h,
                                 refuse_second_create.h,refuse_join.h}
compiler/tests/fixtures/ct/CT04/{case.qcore,expected.json,write_after_joins.qcore}
compiler/tests/fixtures/ct/CT05/{case.qcore,expected.json,remainder_fits.qcore}
compiler/tests/fixtures/ct/CT17/{case.qcore,expected.json,track.h,refuse_create.h,
                                 refuse_join.h,undelivered_payload.qcore,
                                 unreleased_payload.qcore}
compiler/tests/fixtures/ct/CT18/{case.qcore,expected.json,owned_payload.qcore}
docs/reevaluation/ct_manifest.json                       5 rows mapped, note extended
docs/reviews/r0_3/acceptance.json                        CT scalars + ct_evidence
docs/reports/remaining-wave3-ct-integration/{README.md,ct_stage_evidence.json}
```

`AGENTS.md`, `CLAUDE.md` and `docs/reviews/r0_3/acceptance_matrix.json` are
integrator-owned in this wave and untouched (§7).

---

## 10. What this card does not establish

* **Mapping is not conformance.** Twenty of fifty CT cases are mapped; thirty
  are not. `proof` is `unassessed` for all twenty and no Lean theorem is bound
  to any CT fixture.
* **Eight fixed schedules are not a schedule space.** CT03 and CT17 run one
  Linux target, one compiler invocation and eight injected outcomes. Nothing
  here is evidence about arbitrary scheduling, fairness, publication or the C
  memory model.
* **No race detector covers this backend.** ThreadSanitizer does not intercept
  C11 `thrd_create` (measured for T04; a `-fsanitize=thread` build of any
  emitted task program dies in the child before the worker body runs). CT03's
  data-race freedom rests on the checker's no-write-while-shared rule and
  CT17's single-transfer property on the checker's rule, not on measurement.
* **CT03's "counted once" is by address, not by bytes.** This profile has no
  byte-size accounting, which is why CT35 is not mapped.
* **Source-to-trace refinement remains a premise.** That the lowering produces
  the trace the source means is unproved in both Rust and Lean; the Rust ledger
  and the Lean models were written to agree and are tested separately.
* **No promotion, no semantic change.** No verdict, no diagnostic code, no
  pinned artifact, no contract count and no raw Cycle 19 unit changed. The four
  pinned 185-unit artifacts were not re-measured and were not touched;
  `CORPUS-REPIN` remains open.
