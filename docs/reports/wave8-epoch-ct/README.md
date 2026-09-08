# T08-SRC-CT — mapping the four epoch CT cases on a new stage-derived route

Date: 2026-09-09. Baseline: `8d0790a` (main at `60939ff` plus two follow-up
commits). Branch: `worktree-agent-abc955df6be3a7168` (isolated worktree).
Wave 8, lane CT integration, kind harness/manifest, scope M.

**What landed:** the schema-2 CT manifest, harness and summarizer gain the
`epoch` route and the `epoch_bound_figures` verdict mode, and CT35, CT36, CT38
and CT39 are mapped on it from card T08-EPOCH-SRC's parked fixtures. **CT
coverage moves 24/50 → 28/50 mapped (unmapped 26 → 22), by executed-stage
evidence only.** No production checker, frontend, backend or proof file changed;
`git diff --stat` over `crates/` and `compiler/src/` is empty. Sanitized-native
stays 3 of 28 and proof 0 of 28, so **the grown denominator is mapping, not
conformance**, and nothing is promoted.

## 1. What the card had to supply, and what it found

T08-EPOCH-SRC landed `source.epoch.bound.v1`, measured the four cases and parked
their rows with three named harness prerequisites (its §6). All three are done:

| Prerequisite | Disposition |
|---|---|
| an `epoch` route with `epoch_lowering`/`epoch_bound_check` stage names | done; the route vocabulary is `[parse, elaborate, epoch_lowering, epoch_bound_check, proof]`, and it is **narrower than proposed** — see §3 |
| an `epoch_bound_figures` verdict mode for the two ACCEPT cases | done, and made exhaustive rather than selective — see §2 |
| move the fixtures into the harness's mapped layout | done; `compiler/tests/fixtures/epoch/CT3x/` → `compiler/tests/fixtures/ct/CT3x/`, and `compiler/tests/epoch_source.rs` was repointed at the new path and kept |

Every cascade and figure in the parked rows was **re-measured on the fixture's
own bytes through the harness before the manifest was edited**, not copied. Two
corrections came out of that re-measurement (§4).

## 2. `epoch_bound_figures`: why a verdict is not the evidence here

`reject` already covers CT38 and CT39. CT35 and CT36 are accepts, and an
accepted epoch is a weak observation on its own: `FlatEpochBoundV1` refuses
nothing about a program that declares comfortable figures, so "the module
checked" would map both cases without answering what either one asks. CT35 asks
whether one shared allocation read by two tasks is counted once; CT36 asks
whether two concurrent peaks are summed rather than maximized. Both questions are
answered by numbers on the published certificate, not by the verdict.

So the mode asserts the certificate. Specifically, `run_epoch_bound_figures`:

* asserts the module's `SourceExecutionProfile`, so a case that silently changed
  route would not still pass;
* asserts `c_source.is_none() && reference_observation.is_none()` — the
  check-only shape is measured, not assumed from the route table;
* asserts the exact premise set, in `epoch_certificate_of`, for **every**
  accepted module including controls: a certificate that had quietly shed
  `NoScheduleModel` would be a stronger claim than anything measured here;
* rebuilds the whole `{field}.{axis}` figure map **from the certificate** and
  compares it wholesale against the fixture's map. A fixture cannot pin the
  three figures that suit it: any nonzero axis the fixture omits is a mismatch,
  which is what mutation M02 measures.

Accepted controls are held to the same standard — task count and whole
certificate — so "the control also checks" is never the whole control.

## 3. The one deviation from the parked rows, and why

T08-EPOCH-SRC's rows proposed `native_sanitized: unassessed` for all four cases,
noting the manifest's rule that "`native_sanitized` and `proof` are never
`not_applicable`". The mapping records **`not_applicable`** instead.

The epoch route is check-only by construction: `compile_epoch_source` publishes
with `c_source: None`, so the route has no `c_emission` stage, and there is
nothing to sanitize where nothing is emitted. Schema 2's route rule then decides
it structurally — the harness refuses any status but `not_applicable` for a
stage outside a case's route vocabulary — so the choice is between the two
statements *this route can never reach a sanitized run as it stands* and *this
case has an open sanitized run somebody could do today*. The first is true and
the second is not.

The manifest note's universal sentence was therefore narrowed **deliberately**,
in the same change, to: `proof` is never `not_applicable`; `native_sanitized` is
`not_applicable` on a route with no `c_emission` stage and `unassessed`
everywhere else. No gate is weakened by this: no case gains an executed claim,
`native_sanitized: executed` remains pinned to CT03/CT17/CT26, and
`recorded_not_gated` remains pinned to the five transcript cases. What changes is
that four cases move from an indefinite status to a structural one, which is the
same repair schema 2 made for the mutex route's absent CFG stage.

If a future increment gives the epoch route an emission, the route vocabulary
gains `c_emission`/`native_execution`/`native_sanitized` and these rows become
`unassessed` again — by editing the route table, deliberately.

## 4. Per-case rows, re-measured

All four run through `compile_files` in `ct_corpus.rs::run_mapped_case`. Stages
below are the executed set; every other stage is `not_applicable` except `proof`,
which is `unassessed` for all four. `evidence_target` is absent for all four: the
route has no `target_os` gate anywhere, so every stage is host-unconditional and
the manifest records no host gate.

| Case | Verdict mode | Executed stages | Measured |
|---|---|---|---|
| CT35 | `epoch_bound_figures` | parse, elaborate, epoch_lowering, epoch_bound_check | `shared_entry_union.memory = 4096` for two readers of one cell; `epoch_peak.memory = 6912 = 4096 + (1024+1024) + (512 + 2*128)`; whole certificate pinned (11 axes + task count 2) |
| CT36 | `epoch_bound_figures` | same four | `concurrent_private_peak.memory = 8192` (summed, not maximized); `runtime_overhead.memory = 768`; `epoch_peak.memory = 8960`; whole certificate pinned (10 axes + task count 2) |
| CT38 | `reject` | same four | exact ordered cascade `["Q637"]`, `CreditsExceedAllocation { axis: "memory", granted: 12288, allocation: 8192 }` |
| CT39 | `reject` | same four | exact ordered cascade `["Q639"]`, `WallClockUnavailable`, measured after the bound was derived and independently rechecked |

Controls, each executed by `run_epoch_controls` on its own fixture:

| Case | Control | Outcome | What it isolates |
|---|---|---|---|
| CT35 | `two_cells.qcore` | accept, `shared_entry_union.memory = 8192`, `epoch_peak.memory = 11008` | the union counts cell identities, not readers: exactly four figures move |
| CT35 | `repeated_entry.qcore` | reject `["Q630"]` | counting-once is a checked rule, not a lowering that collapses repeats |
| CT36 | `summed_allocation.qcore` | accept, `epoch_peak.memory = allocation.memory = 8192` | the upper half of the straddle; overhead is a declared addend, not folded into the peak |
| CT36 | `maximized_allocation.qcore` | reject `["Q637", "Q638"]` | one byte below the summed peak; a maximized reading would have admitted both, so this is what makes the summing falsifiable |
| CT38 | `split_within_total.qcore` | accept, `granted_credits.memory = allocation.memory = 8192` | the refusal is keyed to the granted sum, not to the split |
| CT39 | `work_bound_only.qcore` | accept, `work_total.effort = 800`, `NoScheduleModel` present | the epoch is fine; only the time claim does not follow |

**One correction to the parked rows, found by re-measurement.**
T08-EPOCH-SRC's `accepted_certificate` blocks named five figures for CT35 and
four for CT36 — the ones each case's prose argues about. The published
certificates carry **eleven** and **ten** nonzero axes respectively, including
`completed_residency` and `exit_residency` (CT35: 4864 and 4608; CT36: 768 and
512), `work_total.effort` (CT35: 20; CT36: 30), and both axes of
`granted_credits` and `allocation`. Pinning the whole map (§2) is what surfaced
them, and it matters: a residency or credit figure moving while the four
argued-about numbers held still would otherwise pass unnoticed. Every parked
value was correct as far as it went, and every parked cascade — `["Q630"]`,
`["Q637","Q638"]`, `["Q637"]`, `["Q639"]` — reproduced exactly on the fixture
bytes.

## 5. Derived numbers

`scripts/ct_stage_summary.py` and `compiler/tests/ct_corpus.rs::ct_stage_summary`
derive these independently and `ct_acceptance_summary_is_derived_from_the_manifest`
compares both against the committed `docs/reviews/r0_3/acceptance.json`:

```
ct_mapped 28   ct_unmapped 22
depth_profile   epoch_bound_check 4, task_lowering 1, task_ledger_check 4,
                mutex_invariant_check 7, native_execution 9, native_sanitized 3
native_sanitized  executed 3, recorded_not_gated 5, not_applicable 4, unassessed 16
proof             unassessed 28
new observation counts  epoch_lowering 4, epoch_bound_check 4
```

`docs/reports/remaining-d02/build_matrix.py` regenerates its CT section from the
same manifest through the same summarizer; the existing
`test_build_matrix_regenerates_the_ct_section_from_the_manifest` verifies it row
for row and passes. Its one CT-derivation edit is an `epoch_controls_executed`
column beside the existing `source_reject_controls_executed`, because both epoch
verdict modes carry controls and the older name would be wrong for CT35/CT36.

## 6. Fail-closed verification — 18 scratch mutations, 18 refused

Each mutation was applied to a clean tree, checked against
`cargo test -p qcore-compiler --test ct_corpus` and the Python summarizer suite,
then reverted with `git checkout -- .`. **None was committed**; the tree was
verified clean afterwards.

| # | Mutation | Refused by |
|---|---|---|
| M01 | CT35 figure: `shared_entry_union.memory` 4096 → 8192 | `CT35: published certificate figures` |
| M02 | CT36 figures: drop `work_total.effort` (pin a convenient subset) | `CT36: published certificate figures` |
| M03 | CT35 control figure: `two_cells` union 8192 → 4096 | `CT35 control two_cells.qcore: published certificate figures` |
| M04 | CT35 premise set: drop `NoScheduleModel` | `the certificate's premise set is part of the measurement` |
| M05 | CT38 cascade: `["Q637"]` → `["Q637","Q638"]` | `CT38: exact diagnostic cascade mismatch` |
| M06 | CT36 control cascade: drop `Q638` from the straddle refusal | `CT36 control maximized_allocation.qcore: exact diagnostic vector mismatch` |
| M07 | CT35: the refused `repeated_entry` control claimed as accepted | `CT35 control repeated_entry.qcore must check` |
| M08 | CT39 stage claim: `native_sanitized` promoted to `executed` | route rule + sanitizer gate + derived-summary drift |
| M09 | CT35 stage claim: `native_sanitized` weakened to `unassessed` | ``CT35: `native_sanitized` is not a stage of route `epoch`, so it is structurally absent, never unassessed`` |
| M10 | CT38 stage claim: `epoch_bound_check` demoted to `not_applicable` | derived plan mismatch + derived-summary drift |
| M11 | CT36 stage claim: `epoch_lowering` credited to CT35's artifact | ``CT36: `epoch_lowering` must be executed on the case's own fixture`` |
| M12 | CT35 verdict mode → `accept` in the manifest | `the manifest must record the harness mode that produced its stage evidence` |
| M13 | CT36 verdict mode → `accept` in the fixture | mode metadata / case-set pin |
| M14 | CT39 route: `epoch` → `task` | `the declared route must be the one this verdict mode really takes` |
| M15 | CT35 `mapping_status` → `unmapped` while its fixture exists | fixture-exists gate, `MAPPED_CASES` pin, count pin, summary drift |
| M16 | `acceptance.json`: `ct_mapped` left at 24 | `ct_mapped is generated from the manifest` |
| M17 | archived evidence: CT35 `epoch_bound_check` observations 1 → 2 | `test_archived_stage_evidence_agrees_with_the_manifest` |
| M18 | archived evidence: CT38's record removed | `CT38: mapped with no archived stage-evidence record` |

M08/M09 together are the point of §3: the route rule is what makes the
sanitizer status structural in *both* directions — it cannot be promoted and it
cannot be softened back to an open item without editing the route table.

## 7. Lean

**Not applicable, by the closing standard's own terms.** This card adds no
semantics: no checker rule, no diagnostic code, no lowering and no model. It
moves manifest rows and harness assertions over an unchanged
`source.epoch.bound.v1` route, whose 37 audited theorems
(`proof/QCore/EpochSourceLowering.lean`) landed with T08-EPOCH-SRC and are
untouched here. Those theorems model the declaration form, not these fixture
bytes, and no digest ties the Lean transcription to a fixture — which is exactly
why `proof` stays `unassessed` for all four cases rather than being credited
from the adjacent Lean module. Card T00-CT-SCHEMA recorded the same judgment for
the same reason; T00.7 (a theorem bound to a CT fixture) remains open.

The Lean gate was run and passes: 42 audits.

## 8. Standing per-change gate

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok (`third_party/libevent` at `d82464a2`) |
| `cargo test --workspace` | **1752 passed, 0 failed, 3 ignored** |
| `cargo fmt --all -- --check` | clean (both workspaces) |
| `cargo clippy --all-targets -- -D warnings` | clean (both workspaces) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | Passed **42** Lean audits |
| `python3 -B -m unittest discover -s scripts/tests` | 108 tests, OK |
| `cargo test` in `providers/` | 58 passed, 0 failed |

No flake was observed on this run; the known load-flaky P02/P03/vp04 assertions
all passed first time and were not touched.

## 9. What this does not establish

* **Mapping is not conformance.** Four more cases now execute their own fixture
  through the real source pipeline. Sanitized-native evidence is unchanged at 3
  of 28 and proof at 0 of 28; the epoch route contributes neither and, as it
  stands, structurally cannot.
* **Every figure on these certificates is declared.** Peaks, results, work,
  credits, cell footprints and overhead are declared shapes;
  `DeclaredTaskFootprints`, `DeclaredRuntimeOverhead`,
  `DisjointSharedStorageCells` and `NoScheduleModel` are carried unchanged on
  every certificate the mapping measures. Pinning them exactly is a check that
  the premises did not quietly shrink — it is not a discharge of any of them.
* **No schedule, in either direction.** CT39 maps the refusal of a wall-clock
  reading, not a latency result.
* **No composition.** An epoch declaration composed with a real task, share,
  guard or region-credit module is outside every checked profile
  (`T08-SRC-COMPOSE`), and no CT case exercises it.
* **Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; the contract corpus stays 60
  modules; `qcore-schemas` stays 6.0.0; full language conformance stays false.

## 10. Integrator carry list

Files this card owns and changed: `docs/reevaluation/ct_manifest.json`,
`compiler/tests/ct_corpus.rs`, `compiler/tests/epoch_source.rs` (fixture path and
header comment only), the four `compiler/tests/fixtures/ct/CT3x/` directories
(moved, plus a new `expected.json` each), `scripts/ct_stage_summary.py`,
`docs/reviews/r0_3/acceptance.json`, `docs/reports/remaining-d02/build_matrix.py`
(one CT column) and this report.

The integrator must:

1. **Regenerate `docs/reviews/r0_3/acceptance_matrix.json`'s `ct_conformance`
   section** from `docs/reports/remaining-d02/build_matrix.py`, adopting it
   surgically as wave 6 did (the generator's non-CT sections remain the stale
   hand-coded snapshot recorded open as `D02-GEN-REST`; its `GATE` block still
   hardcodes 47 provider and 43 Python tests). The regenerated CT section reads
   mapped 28 / unmapped 22, `depth_profile.epoch_bound_check = 4`, and adds an
   `epoch_controls_executed` column.
2. **Update `named_followups`** in the same file: `T08-SRC-CT` is closed;
   `T08-SRC-EMACS`, `T08-SRC-UNREACHABLE`, `T08-SRC-COMPOSE` and
   `T08-SRC-DERIVE` are unchanged and still open.
3. **Carry the status paragraphs** in `AGENTS.md`, `CLAUDE.md` and
   `DEVELOPMENT.md`: CT coverage 24/50 → **28/50 mapped**, unmapped 26 → **22**,
   sanitized-native **3 of 28**, proof **0 of 28**, nothing promoted. The gate
   line at this checkpoint is 1752 workspace tests (3 ignored), 58 provider
   tests, 108 Python tests, 42 Lean audits, both probes exit 0, fmt and clippy
   clean.
4. **Note the narrowed manifest rule** (§3) when reconciling documentation: the
   manifest note no longer says `native_sanitized` is never `not_applicable`.
