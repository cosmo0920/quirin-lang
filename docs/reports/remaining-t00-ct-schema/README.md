# T00-CT-SCHEMA — a stage-derived CT manifest and summarizer

Card: T00-CT-SCHEMA, wave 6. Date: 2026-09-08. Base checkpoint: `cf2e5ee`
(merged wave-5 state; CT coverage 24/50 with the guard route's no-CFG
convention). Scope: schema and bookkeeping only.

**Nothing about the evidence changed.** CT coverage stays **24/50 mapped**, no
fixture was added, removed or edited except for its `stages_executed` array, no
diagnostic code moved, no checker or backend semantics changed, no verdict was
promoted, and every derived number below reproduces the value the previous
hand-kept record already carried. What changed is that those numbers are now
*computed from named executed evidence and refused when they drift*, instead of
being words a human typed into a manifest.

---

## 1. What T00 recorded, and what each item became

`docs/reports/remaining-t00/README.md` §4 lists nine schema tasks. §6 proposes
the shape. This card implements the schema half.

| ID | Task | Disposition |
|---|---|---|
| T00.1 | CT02: native negative-control stage + harness-checked control link | **Not done — out of scope.** This is a *fixture* task (add a control), not a schema one. The schema now has the field it would need (`aux:` rows in the evidence records, `positive_control` in the fixture); CT02 still executes exactly what it executed. Named open. |
| T00.2 | CT19: native control stage and `evidence_target` | **Partly.** CT19's control execution is recorded in T00's `aux:reference_control` row and now flows into the generated matrix's `auxiliary_executed_stages`. It is still not a *stage* of CT19 itself, deliberately: a control's execution is not the case's own evidence. Named open. |
| T00.3 | Sanitized-native stage as a real, fail-closed gate | **Done.** §4. |
| T00.4 | Split `cfg` and `type_checker` into the real passes | **Done.** §2, §3. |
| T00.5 | `host_gate` as a first-class per-stage field | **Done.** §5. |
| T00.6 | Record the executed CLI end-to-end stage (CT01, CT13) | **Partly.** The CLI runs are carried as `aux:cli_end_to_end` rows in T00's evidence record and reach the generated matrix. They are not a manifest stage: `cli_end_to_end` is not a pass of any route, and adding it to the stage vocabulary would put a process-level run in the same column as a compiler pass. Named open with that reason. |
| T00.7 | Proof stage: bind theorems to fixtures, or stop reporting Lean counts beside CT counts | **Not done — out of scope.** `proof` is `unassessed` for all 24 cases, and the harness now *pins* that: `derive_stage_plan` writes `unassessed` unconditionally, so advancing it is a deliberate edit to one line. Named open. |
| T00.8 | Declare transitively included native adapters in the fixture inventory | **Not done — out of scope.** CT14/CT37's `completion_before_join.h` `#include` stub is unchanged. Named open. |
| T00.9 | Derive `acceptance.json` CT rows from stage evidence | **Done.** §6, §7. |

---

## 2. Schema before and after

### Before (schema 1)

```json
"stages": {
  "parser": "executed",
  "type_checker": "executed",
  "reference_execution": "executed",
  "cfg": "executed",
  "native_backend": "executed",
  "proof": "unassessed"
}
```

Six words. Four recorded defects, all from T00 §3:

* **O1/O2** — `cfg` and `type_checker` conflated four real passes. `type_checker`
  covered elaboration, lowering *and* the model check, so CT02 (refused by the
  task ledger at `Q600`) and a hypothetical case dying at `Q013` in name
  resolution rendered identically. `cfg` had no assertion behind it at all and
  had to be *read differently per route*: the manifest note carried two readings,
  one for the task route and one saying the mutex route has no such stage.
* **O3** — statuses were host-unconditional. The runners are gated on
  `cfg!(target_os = …)`; the field a summarizer consumes was not, so on Windows
  the manifest still read `executed` for stages that did not run.
* **O4** — sanitizer evidence was indistinguishable from gated evidence. Five
  cases' ASan/UBSan/LSan runs live as transcripts in `docs/reports/w6-*/README.md`
  and are re-run by nothing; the manifest had no way to say so.

### After (schema 2)

```json
"route": "task",
"harness_verdict_mode": "native_owned_transfer",
"stages": {
  "parse":            { "status": "executed",
                        "executor": "compiler/tests/ct_corpus.rs::run_mapped_case",
                        "artifact": "compiler/tests/fixtures/ct/CT01/case.qcore",
                        "observations": 1 },
  "task_ledger_check":{ "status": "executed", "executor": "…::run_mapped_case",
                        "artifact": "…/CT01/case.qcore", "observations": 1 },
  "mutex_lowering":   { "status": "not_applicable" },
  "native_execution": { "status": "executed",
                        "executor": "compiler/tests/ct_corpus.rs::run_native_owned_transfer",
                        "artifact": "…/CT01/case.qcore", "observations": 1,
                        "host_gate": ["linux", "macos", "windows"] },
  "native_sanitized": { "status": "unassessed" },
  "proof":            { "status": "unassessed" }
}
```

The manifest also carries its own `stage_vocabulary` (stages, statuses, routes),
which `ct_corpus.rs` pins against its own constants, so the data documents the
words while the code owns them.

**Status vocabulary:**

| Status | Means | Requires |
|---|---|---|
| `executed` | a named harness runner ran this pass on this case's own fixture | `executor` (from a pinned registry), `artifact` (this case's `case.qcore`, and it must exist), `observations` ≥ 1, and `host_gate` exactly where the runner is gated |
| `recorded_not_gated` | an archived transcript that no workspace test re-runs | a `record` pointer to an existing file, and **no** executor/artifact/count/gate |
| `not_applicable` | the route has no such pass, or this case's own refusal provably never reaches it | no evidence fields |
| `unassessed` | genuinely open | no evidence fields |

`native_sanitized` and `proof` are never `not_applicable`: a gated sanitized
re-run or a bound theorem could be added for any case, so calling them
structurally absent would be false. Every pipeline stage a refused program never
reaches *is* `not_applicable` — this is the pre-existing counting rule, now
enforced rather than described.

---

## 3. Per-route stage vocabularies

Fifteen stages, in pipeline order:
`parse`, `elaborate`, `task_lowering`, `task_ledger_check`, `mutex_lowering`,
`mutex_invariant_check`, `sequential_whole_program`,
`sequential_reference_execution`, `task_reference_execution`,
`mutex_reference_execution`, `c_emission`, `native_execution`,
`native_sanitized`, `legacy_c_twin`, `proof`.

This is the vocabulary the three per-card `ct_stage_evidence.json` files already
used; adopting it means the manifest and those records speak one language.

| Route | Stages it can execute | Cases | What the schema-1 `cfg` meant here |
|---|---|---|---|
| `task` | parse, elaborate, task_lowering, task_ledger_check, task_reference_execution, c_emission, native_execution, native_sanitized, proof | CT01, CT02, CT03, CT04, CT05, CT12–CT20, CT37, CT50 (16) | the checked task IR |
| `mutex_guard` | parse, elaborate, mutex_lowering, mutex_invariant_check, mutex_reference_execution, c_emission, native_execution, native_sanitized, proof | CT26–CT32, CT48 (8) | **nothing** — this route emits directly from a checked event list and has no CFG stage, which schema 1 could only spell as an indefinite `cfg: unassessed` |
| `sequential` | parse, elaborate, sequential_whole_program, sequential_reference_execution, c_emission, native_execution, native_sanitized, proof | none yet (reserved; the harness's plain `accept` verdict takes it) | — |
| `legacy_c` | parse, legacy_c_twin, proof | none yet (reserved for CT40–CT43/CT45) | — |

**The wave-5 rule is preserved and strengthened.** Wave 5 established that the
guard route must never claim `cfg: executed`, and recorded that as a convention
in prose. It is now structural: `mutex_guard` has no CFG-shaped stage in its
vocabulary at all, and a mutex case's `task_lowering`/`task_ledger_check` entries
must read `not_applicable` — the harness refuses `unassessed` there, because
"unassessed" would mean "someone might assess it later", which is false. The
same holds in the other direction for a task case's `mutex_*` entries.

Declaring `sequential` and `legacy_c` with no member is deliberate: it is what
makes "this route has no such stage" a statement about a defined set rather than
about the two routes that happen to exist.

---

## 4. The sanitizer gate (T00.3)

T00 finding O4 was the clearest overclaim available: five cases' sanitizer
evidence sat "flush against gated evidence in the same sentence".

The harness now pins two disjoint sets:

* `SANITIZED_GATED_CASES = ["CT03", "CT17", "CT26"]` — a runner really compiles
  the case's emitted C with `-fsanitize=address,undefined` and runs it. Only
  these may read `native_sanitized: executed`.
* `SANITIZER_TRANSCRIPT_CASES = ["CT14", "CT16", "CT20", "CT37", "CT50"]` — must
  read `recorded_not_gated` with a `record` pointer to an existing file
  (`docs/reports/w6-*/README.md`).

Every other mapped case must read `unassessed`. All three directions are
enforced and all three were mutation-tested (§8): promoting CT50 to `executed`,
dropping CT50's `record`, pointing it at a nonexistent file, and claiming
`recorded_not_gated` for CT02 all fail closed.

This extends the existing convention rather than inventing one: `CT26` was
already the first harness-gated sanitized-native case, and `CT03`/`CT17` the
second and third. What is new is that the manifest can now say which is which,
and cannot lie about it.

---

## 5. Host gates (T00.5, repairing O3)

Every `executed` stage whose runner is host-gated must record the exact set of
`target_os` values on which it runs, and the harness recomputes that set from
its own dispatch:

| Mode | reference | c_emission | native_execution | native_sanitized |
|---|---|---|---|---|
| `native_owned_transfer`, `native_create_recovery`, `native_join_failure`, `native_task_scenarios` | linux, macos, windows | linux, macos, windows | linux, macos, windows (**CT50: linux only**) | — |
| `native_heap_scenarios` (CT03, CT17) | linux, macos | linux, macos, windows | linux, macos | linux, macos |
| `checked_mutex_sanitized` (CT26) | linux, macos | linux, macos, windows | linux, macos | linux, macos |
| source stages (parse … model check) | *no gate* — they run on every host | | | |

Two asymmetries are real and now visible. The two sanitized runners return on
Windows *after* asserting the emitted C exists, so emission is reached on all
three hosts while the sanitized compile-and-run is Unix-only. CT50's native
observation needs Linux `MAP_FIXED_NOREPLACE`, so its `native_execution` gate is
`["linux"]` while its reference execution and emission are not. Both were
previously invisible: schema 1 said `native_backend: executed` for both.

The derived `host_scope` in the summary reports `any` for an ungated stage and
the exact list otherwise, so a report produced on a host outside a gate cannot
inherit `executed`.

---

## 6. Derivation, not declaration

`compiler/tests/ct_corpus.rs::derive_stage_plan` computes the whole 15-stage plan
for a case from three things, none of which is the manifest:

1. **this harness's own dispatch** — the verdict mode selects the runner, and the
   runner determines the executor name, the host gate and the deep-stage set;
2. **the fixture's own scenario data** — observation counts are read from what
   each runner loops over (`native_task_scenarios.scenarios.len()`,
   `native_owned_transfer.fail_create` → 2 else 1, `native_join_failure` → one
   reference execution and two compile-and-run cycles, `native_create_recovery` →
   three fixed rows), so a count is a fact about the runner rather than a number
   someone typed;
3. **the diagnostic codes the compiler really produced on the fixture** — the
   lowering stage is executed unless every emitted code is below `Q500`, and the
   model-check stage is executed only if some emitted code is `Q600` or above.
   This is what distinguishes CT18 (refused at `Q500` during task lowering, so
   `task_ledger_check` is `not_applicable`) from CT04/CT05 (refused at `Q600` by
   the ledger). Under schema 1 all three read `type_checker: executed`.

`run_mapped_case` then requires the manifest to equal that plan, entry for entry,
field for field. The route must be the one the verdict mode really takes, the
executor must be in a pinned registry of this file's own functions, the artifact
must be the case's own `case.qcore` and must exist on disk, and the recorded
`harness_verdict_mode` must be the mode that produced the evidence.

The fixture's `expected.json` keeps its `stages_executed` array — migrated to the
new vocabulary, not deleted — and must also equal the derived executed set, so
the same fact is stated in two independent places and checked against a third.

---

## 7. The generated summary (T00.9, repairing O7)

T00 finding O7: `acceptance.json` carried `ct_mapped` and `ct_unmapped` and
nothing else, so "the scalar treats CT02 (source rejection, zero native runs) and
CT16 (4 reference executions + 4 native runs) as one identical unit of coverage."

`docs/reviews/r0_3/acceptance.json`'s `ct_evidence` block now names exactly which
of its fields are generated:

```
"generated_fields": ["ct_mapped", "ct_unmapped", "stage_histogram",
                     "observation_counts", "host_scope", "depth_profile",
                     "case_depth"]
```

and a matching `hand_maintained_fields` list covers the rest (the triage rows,
which are analysis rather than stage evidence). A test asserts the two lists are
disjoint and together cover every key of the block, so a field cannot quietly
migrate from generated to hand-kept.

`ct_mapped`/`ct_unmapped` are **retained** for existing readers, beside the
derived rows. **`case_depth` is the O7 repair**: every mapped case's deepest
executed stage, so CT18 (`task_lowering`) and CT17 (`native_sanitized`) are no
longer the same number.

Two independent derivations, both compared to the committed file:

* **Rust** — `ct_corpus.rs::ct_stage_summary` + the new test
  `ct_acceptance_summary_is_derived_from_the_manifest`. This runs inside
  `cargo test --workspace`, so the gate does not depend on Python.
* **Python** — `scripts/ct_stage_summary.py`, covered by
  `scripts/tests/test_ct_stage_summary.py` (13 tests), and consumed by
  `build_matrix.py`.

Derived values at this checkpoint (all identical to the values the block already
carried — this card changed how they are produced, not what they are):

```
mapped 24 / unmapped 26
depth_profile   native_execution 9 · mutex_invariant_check 7 · task_ledger_check 4
                native_sanitized 3 · task_lowering 1
observations    parse 24 · elaborate 24 · task_lowering 16 · task_ledger_check 15
                mutex_lowering 8 · mutex_invariant_check 8
                task_reference_execution 26 · mutex_reference_execution 1
                c_emission 28 · native_execution 28 · native_sanitized 9
native_sanitized  executed 3 · recorded_not_gated 5 · unassessed 16
proof             unassessed 24
```

### The generator is back in truth (card item 4)

`docs/reports/remaining-d02/build_matrix.py` read T00's eleven-case
`coverage.json` alone and asserted its mapped set equalled the manifest's. That
assertion stopped holding at T03-CT; the script has **crashed on that assertion**
ever since, and `acceptance_matrix.json`'s `ct_conformance` recorded the
thirteen-case divergence in a `generator_status` field rather than pretending
otherwise.

It now reads the manifest through `ct_stage_summary`, merges the archived
per-card records for the `aux:` rows the manifest has no field for (controls, CLI
end-to-end runs, the CT01 frontend mutation matrix), and asserts that the
manifest, all four archived records and `acceptance.json` agree before it writes
anything. It also gained a `--out PATH` option so it can be verified without
overwriting the committed matrix, and it now *emits* the per-CT rows it
previously built and only counted.

`scripts/tests/test_ct_stage_summary.py::test_build_matrix_regenerates_the_ct_section_from_the_manifest`
runs the generator into a temporary file and checks its 50 rows against the
manifest case for case — route, per-stage record and deepest executed stage — and
its histogram, depth profile, observation counts and `case_depth` against the
summarizer. `acceptance_matrix.json` is not touched by this card.

Provenance is checked in both directions: `check_provenance` compares every
manifest stage status and observation count against the archived per-card record
that established it, and reports any mapped case with no archived record. It
returns clean for all 24.

---

## 8. Fail-closed verification (card item 5)

Seventeen scratch mutations, each applied to the committed manifest, run, then
reverted. **Every one is refused.** No mutation was committed.

| Mutation | Refused with |
|---|---|
| mutex-route case claims a task stage | `CT26: 'task_ledger_check' is not a stage of route 'mutex_guard', so it is structurally absent, never unassessed` |
| `executed` with no executor/artifact/count | `an executed stage counts its runs` |
| `executed` naming another case's fixture | `CT02: 'parse' must be executed on the case's own fixture` |
| `executed` naming an unregistered runner | `CT02: 'parse' names executor '…::invented_runner', which is not a runner in this harness` |
| sanitized `executed` without the gate (CT50) | `CT50: stage 'native_sanitized' claims 'executed' but this harness derives 'recorded_not_gated'` |
| `recorded_not_gated` with no `record` | `CT50: 'native_sanitized' is recorded_not_gated but names no record` |
| `record` pointing at a nonexistent file | `CT50: 'native_sanitized' names record 'docs/reports/does-not-exist.md', which does not exist` |
| `recorded_not_gated` on an ungated case (CT02) | `CT02: stage 'native_sanitized' claims 'recorded_not_gated' but this harness derives 'unassessed'` |
| inflated observation count | `CT16: 'native_execution' observation count must equal the runner's invocations` |
| host gate widened (CT50 → +macos/windows) | `CT50: 'native_execution' must record the exact host gate of its runner, so the status cannot be read as host-unconditional` |
| host gate dropped | same assertion, plus `acceptance.json 'host_scope' is not what the manifest's stage evidence derives` |
| wrong route declared | `CT26: the declared route must be the one this verdict mode really takes` |
| `unassessed` where a stage is structurally absent | `CT01: 'mutex_lowering' is not a stage of route 'task', so it is structurally absent, never unassessed` |
| unknown status word | `CT01: unknown stage status 'believed' for 'proof'` |
| missing stage key | `no entry found for key` (every case states every stage) |
| route vocabulary widened in the data | `manifest route vocabulary drifted from the harness` |
| unmapped case carrying evidence | `CT24: an unmapped case must keep all stages 'unassessed' with no evidence` |

Additionally, `test_ct_stage_summary.py` mutates copies of the manifest in a
scratch tree and asserts the summarizer reports drift for a promoted status, an
inflated count and a mapping change, and exits on an unknown schema version or a
stage with no depth rank.

---

## 9. No mapping change (card item 6)

24/50 mapped, 26 unmapped, unchanged. The pinned `MAPPED_CASES` table is
untouched. Every mapped case translated faithfully into the new schema:

* The 15-stage vocabulary was *already* in use by all three
  `ct_stage_evidence.json` files, so 13 of 24 cases had their schema-2 record
  written for them by their own card. T00's `coverage.json` supplied the other
  11 in the same shape.
* The derived histogram, observation counts and depth profile reproduce the
  hand-kept values in `acceptance.json` **exactly**, which is the check that no
  case was quietly downgraded or promoted in translation.

**No case required an honest downgrade.** The one place schema 1 was strictly
less precise than schema 2 — CT18's `task_ledger_check`, which schema 1 folded
into `type_checker: executed` — became `not_applicable` rather than `executed`,
and that was already how T00's and wave 3's records read it and how the
hand-kept histogram already counted it (`task_ledger_check: executed 15` against
`task_lowering: executed 16`). So this is the schema catching up to the recorded
evidence, not a change of evidence.

### One bookkeeping normalization, recorded

The archived records disagree on *executor names* for a case's source-side
stages: T00 credits them to
`ct_corpus.rs::ct_manifest_and_fixture_set_agree_and_mapped_cases_execute`, wave
3 credits CT03's `task_lowering` to `run_native_heap_scenarios`, and wave 5
credits them to `run_mapped_case`. All three are defensible descriptions of the
same execution. Schema 2 uses `run_mapped_case` uniformly for parse, elaborate,
lowering and the model check, because that is the function that actually calls
`compile_files` on the fixture. `check_provenance` therefore compares statuses
and observation counts, and deliberately **not** executor strings; the reason is
documented in its docstring.

---

## 10. Lean (card item 7)

**Not applicable, by judgment.** Assessed rather than assumed:

* The subject is a JSON schema and a bookkeeping derivation. Its content is
  *which workspace test ran on which file*, which is a fact about this
  repository's test harness, not a mathematical relation.
* The one candidate with a checkable shape is summarizer totality — "every
  mapped case has a deepest executed stage", "the histogram columns sum to the
  mapped count", "no case's depth exceeds a stage it actually executed". Those
  are real properties, and they *are* checked — as executable assertions over
  the real manifest, in `test_totals_reconcile` and
  `test_depth_never_exceeds_executed_evidence`. Restating them in Lean would
  prove them of a Lean-side re-encoding of the manifest, and the encoding step
  is exactly where a bookkeeping error would live. That is a vacuous audit in
  the sense the closing standard warns about, so it was not written.
* Nothing in this card touches a checker rule, a model, or a diagnostic, so no
  existing theorem's statement or premise changes. All 37 Lean audits pass
  unchanged.

`proof` remains `unassessed` for all 24 cases and the harness now pins that
unconditionally; binding a theorem to a fixture is T00.7 and stays open.

---

## 11. Files changed

| File | Change |
|---|---|
| `docs/reevaluation/ct_manifest.json` | schema 1 → 2: per-stage evidence objects, `route`, `harness_verdict_mode`, `stage_vocabulary`; note rewritten |
| `compiler/tests/ct_corpus.rs` | new stage/status/route/runner/depth constants, `StageRecord`/`StageVocabulary`/`StagePlan`, `derive_stage_plan`, full per-stage validation, `ct_stage_summary`, new test `ct_acceptance_summary_is_derived_from_the_manifest` |
| `compiler/tests/fixtures/ct/CT*/expected.json` (24) | `stages_executed` migrated to the new vocabulary; nothing else moved |
| `docs/reviews/r0_3/acceptance.json` | `ct_evidence` gains `generated_fields`, `hand_maintained_fields`, `generator`, `host_scope`, `case_depth`; note rewritten; generated fields regenerated (values unchanged) |
| `scripts/ct_stage_summary.py` | **new** — `summarize`, `merge_evidence`, `check_provenance`, `check_acceptance`, `write_acceptance`, CLI |
| `scripts/tests/test_ct_stage_summary.py` | **new** — 13 tests |
| `docs/reports/remaining-d02/build_matrix.py` | CT section reads the manifest through the summarizer; merges all four archived records; `--out PATH`; emits `ct_rows`; `generator_status` says `regenerating` |

Not touched, on purpose: `docs/reviews/r0_3/acceptance_matrix.json` (integrator's
— see §13), the three `ct_stage_evidence.json` records (archived per-card
evidence), `docs/reports/remaining-t00/coverage.json` (archived audit at
`ea2af0f`), `AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`, and every
evidence-v2 / provider digest file (owned by the parallel REASON-CODES card).

---

## 12. Gate results

Run on x86_64 Linux at this card's head commit.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1625 passed, 0 failed, 1 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **37 audits passed** |
| `python3 -m unittest discover -s scripts/tests` | **108 tests, OK** (95 before, +13) |
| `cd providers && cargo test` | **58 passed, 0 failed** |

`git submodule update --init --recursive` was required once in this worktree
(`third_party/libevent` was absent, and its pin test correctly failed closed
until it was fetched). No load-flaky test (P02 `RLIMIT_NPROC`, P03
confined-write, vp04 process-lifecycle) failed in this run.

---

## 13. For the integrator

1. **`acceptance_matrix.json` needs regenerating, and its `generator_status` must
   change.** The field currently reads that the generator "needs a merge step
   over the two ct_stage_evidence.json files before it can regenerate" and that
   the section is hand-maintained. That merge step now exists. Regenerate with:

   ```sh
   python3 docs/reports/remaining-d02/build_matrix.py . <workspace_test_count>
   ```

   The emitted `ct_conformance.generator_status` already says `regenerating: …`;
   the hand-written text in the committed file will be replaced by it.

2. **Regenerating adds and removes keys.** The regenerated section gains
   `ct_rows` (50 per-case rows — previously built and only counted),
   `host_scope` and `case_depth`, and drops the per-row `manifest_stages` column
   that existed to keep the schema-1 and stage-derived views side by side (they
   are now one object). It will also **drop three keys that were hand-added to
   the committed matrix and are not produced by the generator**:
   `executed_totals_added_by_t03_ct_integration`,
   `executed_totals_added_by_wave5_ct_integration` and
   `newly_decidable_not_mapped_reasons`. Decide deliberately whether to
   re-add them to `ct_section` in `build_matrix.py` or let them go; this card did
   not silently preserve them.

3. **Two pre-existing hand/generated disagreements, not introduced here.**
   `acceptance.json`'s `unmapped_bucket_counts` and `newly_decidable_not_mapped`
   were updated by later waves with bucket names T00's triage does not contain
   (`blocked-on-scenario-mismatch (no local alias form)`, and `CT11` in the
   decidable list), while `acceptance_matrix.json` derives both from T00's
   triage. They already disagreed before this card. Both are explicitly listed
   as hand-maintained in `acceptance.json`'s `hand_maintained_fields`; the
   generator now at least filters T00's rows to the 26 still-unmapped cases and
   asserts all 26 are covered. Reconciling the bucket vocabulary is a separate
   triage task.

4. **Nothing to carry into the status ledgers about coverage.** CT stays 24/50,
   the contract corpus stays 60 modules, no verdict moved, no raw unit was
   promoted, and `qcore-schemas` (5.0.0) and `qcore-compiler` (0.2.0) are
   unchanged. The one increment-record fact worth a line is that the CT manifest
   is now schema 2 and its acceptance numbers are generated.

5. **Named open follow-ons from this card:** T00.1 (CT02 native negative
   control), T00.2 (CT19 control as a first-class record), T00.6
   (`cli_end_to_end` as a stage, deliberately declined — see §1), T00.7 (bind a
   theorem to a fixture), T00.8 (declare transitively included native adapters).
