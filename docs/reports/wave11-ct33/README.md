# Wave-11 CT lane — CT33 mapped from card CT33-RANKS's proposal

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Wave 11. One case moves: **CT33** (lock-order cycle in optional rank mode)
maps on the existing `mutex_guard` route. CT coverage moves **30/50 → 31/50
mapped**, unmapped **20 → 19**.

Base: `main` at `695f0e1`. All work is on this lane's branch.

Nothing is promoted. This is a mapping increment: no checker rule, no
diagnostic code, no lowering and no model changed, and the grown denominator
is mapping rather than conformance. Sanitized-native stays **3 of 31**
(CT03/CT17/CT26) and proof stays **0 of 31**.

`git diff` over `crates/` and `compiler/src/` is empty; the only Rust touched
is the harness's pinned case sets in `compiler/tests/ct_corpus.rs`.

## 1. What was mapped, and from what

Card CT33-RANKS (`docs/reports/remaining-ct33-ranks/README.md`, wave 10) gave
lock ranks a bounded checked existence in `CheckedMutexInvariantV1` and, in the
same change, made a nested `with lock` expressible at all — the two had to move
together, because a lock-order cycle *is* two nestings in opposite directions.
Its §8 proposed CT33's mapping and left it for the CT lane. This lane executed
that proposal: it moved the case into the harness fixture layout, **re-measured
the cascade and both controls on the fixture bytes through the harness before
touching the manifest**, and mapped only the stages that really executed.

## 2. Fixture

`compiler/tests/fixtures/ct/CT33/`

| File | Role |
|---|---|
| `case.qcore` | the scenario: two generations at ranks 1 and 2, nested `b`-in-`a` then `a`-in-`b` |
| `ascending_nesting.qcore` | accepted control — the case with **one statement changed** |
| `unranked.qcore` | refused control — the case's shape with both declarations unranked |
| `expected.json` | verdict mode, exact cascade, controls, stage list, notes |

The case, verbatim:

```qcore
#import std.concurrency;
fn main() -> unit {
    thread scope s {
        let x = protected 4;
        let y = protected 4;
        let a = mutex rank 1 guards x;
        let b = mutex rank 2 guards y;
        with lock a as ga { with lock b as gb { } }
        with lock b as gc { with lock a as gd { } }
    }
}
```

## 3. Measured cascade and controls

All three measured through `compile_files` on the committed fixture bytes, then
re-measured through the harness runner (`run_mapped_case` /
`run_mutex_controls`) after the manifest landed.

| Artifact | Expectation | Measured |
|---|---|---|
| `case.qcore` | reject | `["Q624", "Q612"]` |
| `ascending_nesting.qcore` | accept | checked, and the harness asserts it took the checked mutex-guard route |
| `unranked.qcore` | reject | `["Q622", "Q622"]` |

The case's two diagnostics in full, as the compiler renders them:

```
case.qcore:9:29: Q624: LockRankViolation { event: 7, mutex: 1, rank: 1,
                                           held_guard: 3, held_mutex: 2, held_rank: 2 }
case.qcore:9:49: Q612: DeadGuard { event: 8, guard: 4 }
```

The trailing `Q612` is **pinned, not filtered**, exactly as the card asked. It
is honest fallout: the refused `Lock` mints no guard, so the inner section's own
unlock names a dead one. Pinning it is what shows the refusal neither released
nor minted a guard — a change that started doing either would move the vector.
The first entry names both sides of the comparison that failed (`rank: 1` being
acquired while `held_rank: 2` is held), so a reader sees the decision rather
than being told an order exists.

Two controls rather than one, because they answer two different questions:

* **`ascending_nesting.qcore`** differs from the case by one statement — the
  second section nests `b` inside `a` again instead of the reverse — and
  checks. So `Q624` is attributable to the descending edge, not to nesting, not
  to the two sections existing, and not to the module declaring ranks at all.
  This follows the card's §8 semantic-profile text ("the accepted control
  differs by one statement"). Its §4 table listed a different, single-section
  accepted control carrying an `open`/`write`/`close` region; that form was not
  chosen, because a control that also deletes a whole section attributes the
  refusal less sharply. **Recorded deviation, not an omission.**
* **`unranked.qcore`** keeps the case's shape and unranks both declarations.
  Both nestings refuse at `Q622 UnsupportedSynchronizationConstruct {
  NestedProtectingMutex }` — the represented refusal that stood before card
  CT33-RANKS. That makes "nesting is admitted only where a rank is declared" a
  *measured* statement in the corpus rather than prose in a card, and it pins
  that unranked modules are left exactly where they were.

## 4. Stage rows

Derived by `ct_corpus.rs::derive_stage_plan` from the harness's own dispatch and
the codes the compiler really produced, then compared against the manifest. The
manifest is checked, not self-certifying.

| Stage | Status | Executor | Artifact | Obs |
|---|---|---|---|---|
| `parse` | executed | `ct_corpus.rs::run_mapped_case` | `CT33/case.qcore` | 1 |
| `elaborate` | executed | `ct_corpus.rs::run_mapped_case` | `CT33/case.qcore` | 1 |
| `mutex_lowering` | executed | `ct_corpus.rs::run_mapped_case` | `CT33/case.qcore` | 1 |
| `mutex_invariant_check` | executed | `ct_corpus.rs::run_mapped_case` | `CT33/case.qcore` | 1 |
| `mutex_reference_execution` | not_applicable | — | — | — |
| `c_emission`, `native_execution` | not_applicable | — | — | — |
| `native_sanitized` | **unassessed** | — | — | — |
| `proof` | **unassessed** | — | — | — |
| every task / epoch / sequential / `legacy_c` stage | not_applicable | — | — | — |

Deepest executed stage: `mutex_invariant_check`. `harness_verdict_mode:
reject`; `route: mutex_guard`; `evidence_target: linux`.

`native_sanitized` is **unassessed and was deliberately not upgraded.** Card
CT33-RANKS does carry a sanitized native run of an equivalent ranked nested
trace (`ranked_nested_sections_agree_between_the_reference_and_a_sanitized_native_run`,
`crates/qcore-core/tests/unit/mutex_c_emit.rs`), but that run is at the core
level over a hand-built trace, not through this harness on this case's fixture,
and schema 2's sanitizer gate (`SANITIZED_GATED_CASES`) lists exactly which
cases may read `executed`. Claiming it would be the overclaim the gate exists to
prevent. The card proposed `unassessed` for this reason and this lane agreed;
upgrading it needs a `checked_mutex_sanitized` harness fixture and a deliberate
edit of the gate list.

There is no CFG row to leave open: the `mutex_guard` route emits from a checked
event list and constructs no CFG at all, so schema 2's route vocabulary omits
every CFG-bearing stage. That is the wave-5 rule made structural by
T00-CT-SCHEMA and it applies here unchanged.

`evidence_target: linux` records that an accepted control exists and is
host-gated (the harness asserts the `Q500` refusal on any other target rather
than skipping silently). It records **no** compiled or executed C.

## 5. Mutation checks

Nineteen scratch mutations, each applied to a committed file, run through
`ct_manifest_and_fixture_set_agree_and_mapped_cases_execute`, then reverted.
**All nineteen refuse.** Nothing was committed; `git status` after the runs shows
only this lane's intended files.

| # | Mutation | Result | Refused by |
|---|---|---|---|
| M1 | cascade entry `Q612` dropped from `diagnostic_sequence` | REFUSED | exact diagnostic cascade mismatch |
| M2 | cascade order swapped to `["Q612","Q624"]` | REFUSED | exact diagnostic cascade mismatch |
| M3 | accepted control's expectation flipped to `reject` | REFUSED | target-scoped evidence rule (no accept control left, `evidence_target` must then be null) |
| M4 | unranked control's expectation flipped to `accept` | REFUSED | the control must be refused (`run_mutex_controls`) |
| M5 | unranked control's cascade shortened to one `Q622` | REFUSED | control diagnostic-vector mismatch |
| M6 | all controls removed (refusal made vacuous) | REFUSED | shape rule + target-scoped evidence rule |
| M7 | verdict mode changed to `accept` | REFUSED | declared route must be the one the verdict mode takes |
| M8 | `stages_executed` claims `mutex_reference_execution` | REFUSED | must match the derived plan |
| M9 | manifest claims `native_sanitized: executed` | REFUSED | harness derives `unassessed` |
| M10 | manifest claims `mutex_reference_execution: executed` | REFUSED | harness derives `not_applicable` |
| M11 | manifest claims `route: task` | REFUSED | declared route must be the real one |
| M12 | manifest names a *control* as a stage's artifact | REFUSED | a stage runs on the case's own fixture |
| M13 | manifest inflates an observation count to 2 | REFUSED | count must equal the runner's invocations |
| M14 | manifest declares `harness_verdict_mode: checked_mutex_sanitized` | REFUSED | mode must be the one that produced the evidence |
| M15 | manifest reverts CT33 to `unmapped` with the fixture present | REFUSED | fixture exists while the manifest says unmapped |
| M16 | manifest drops `evidence_target` | REFUSED | target metadata mismatch |
| M17 | case source made ascending (the refusal removed) | REFUSED | expected rejection, checker accepted |
| M18 | case source ranks made equal (`1`/`1`) | REFUSED | exact diagnostic cascade mismatch |
| M19 | case source ranks made partial (one declaration unranked) | REFUSED | exact diagnostic cascade mismatch |

M3 and M6 are recorded honestly: they refuse, but through the target-scoped
evidence rule rather than through the control assertion, because removing the
only accepted control also removes the reason `evidence_target` may be set.
Both are refusals; neither is the refusal one might have predicted.

## 6. Deliberate pin updates

| Pin | Before | After |
|---|---|---|
| `ct_corpus.rs::MAPPED_CASES` | 30 ids | 31 ids (`+CT33`) |
| `ct_corpus.rs::MUTEX_REJECT_CASES` | 7 ids | 8 ids (`+CT33`) |
| `ct_corpus.rs` progress doc comment | 30 of 50 / 20 unmapped / 28 of 30 source-level | 31 of 50 / 19 unmapped / 29 of 31 source-level |
| `acceptance.json` `ct_mapped` / `ct_unmapped` | 30 / 20 | **31 / 19** |
| `acceptance.json` `depth_profile.mutex_invariant_check` | 7 | 8 |
| `acceptance.json` `observation_counts` | parse 30, elaborate 28, mutex_lowering 8, mutex_invariant_check 8 | 31, 29, 9, 9 |
| `acceptance.json` `case_depth` | (no CT33) | `CT33: mutex_invariant_check` |
| `acceptance.json` `ct_evidence.note` | "3 of 30 … 0 of 30" | "3 of 31 … 0 of 31" |
| `acceptance.json` `derived_from`, `cause` | 7 files / 6 reports | + this lane's record |
| `acceptance.json` `unmapped_bucket_counts` `blocked-on-guard-profile-composition-or-unrepresented-form` | 3 | 2 |
| `scripts/ct_stage_summary.py::EVIDENCE_FILES` | 6 files | 7 (`+docs/reports/wave11-ct33/ct_stage_evidence.json`) |
| manifest `note` (mutex_guard route case list) | CT26…CT32/CT48 | + CT33, plus the CT33/CT34/CT46 paragraph |

Every generated field of `acceptance.json` was written by its own generator
(`python3 scripts/ct_stage_summary.py . --write`) and then verified by **both**
derivations, which agree:

* Rust: `ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` — passes.
* Python: `scripts/ct_stage_summary.py . --check` — exit 0, no DRIFT lines.

One cosmetic byte-level change is recorded rather than hidden: the generator
re-serializes `ct_evidence.note` with `ensure_ascii`, so a single literal em
dash in that string became `—`, matching every other em dash already in the
file. The JSON *value* is unchanged apart from the two count edits above
(verified by comparing parsed values against `HEAD`).

`docs/reports/remaining-d02/build_matrix.py`'s CT derivation was verified
against the new manifest with `--out` to a scratch path (never overwriting the
committed matrix): it reads 50 cases, `mapped 31 / unmapped 19`, `parse`
observations 31, and its own independently derived unmapped-bucket counts sum
to 19.

## 7. CT34 and CT46 — reasons re-verified, mapping unchanged

Both stay **unmapped**. Their recorded reasons were checked against what card
CT33-RANKS actually landed, and both hold:

* **CT34** (protected owned graph grows/replaces identities). `CreateMutex`
  still fixes its declared footprints at creation and consumes the creator's
  `FULL_ACCESS` share over each; there is no growth form, and post-creation
  protection growth remains a represented refusal. A rank orders *acquisitions
  of existing generations* and changes nothing about what a mutex protects, so
  CT33-RANKS created no protection-growth form. Reason accurate, unchanged in
  substance.
* **CT46** (non-`Sync` but `Send` payload behind a shared wrapper). Still needs
  `Send`/`Sync` derivation, which is outside L09 and outside this profile
  entirely. Ranks are declared positions in an acquisition order and say nothing
  about which payloads may cross a task boundary. Reason accurate, unchanged in
  substance.

One stale *text* was found and repaired, without changing any mapping. The
acceptance matrix publishes CT34's blocker from T00's archived
`coverage.json`, frozen at revision `ea2af0f`, which reads:

> no source form creates or declares a mutex; the guard-binding blocker batch 2
> named was repaired but is no longer the limit

That first clause has been untrue since card T03 landed `mutex guards`, and is
doubly untrue since CT33-RANKS landed `mutex rank N guards` — while the reason
CT34 is unmapped did not change. The archived file is **not** edited (rewriting
an archived audit would destroy the history it exists to keep). Instead
`build_matrix.py` now publishes the superseding, measured reason beside the
archived one, for the ids a named record actually re-verified:

* `blocker` — unchanged, still T00's archived text.
* `blocker_current` — this lane's measured reason, read from
  `docs/reports/wave11-ct33/ct_stage_evidence.json`.
* `blocker_superseded_by` — the file that supplied it.

The reader is asserted, so this cannot go stale in turn: if the record stops
naming exactly `{CT34, CT46}`, or names a case that has since been mapped, the
generator fails rather than publishing a dangling reference.

## 8. Lean

**Judged not applicable, and recorded** — the same judgment cards
T00-CT-SCHEMA, T08-SRC-CT and CT-CTWIN recorded for their harness increments.
This lane adds no checker rule, no diagnostic code, no lowering and no model; it
adds one fixture, one entry in each pinned case set, and the manifest and
acceptance rows derived from them, over a checker whose own twelve rank theorems
landed with card CT33-RANKS. `proof` stays `unassessed` for CT33; no theorem is
bound to any CT fixture (T00.7 remains open). The Lean gate was run and passes.

## 9. Gate

Run in this worktree at the final commit.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2034 passed, 0 failed, 4 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **51 audits pass** |
| `providers/` `cargo test --no-fail-fast` | **76 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | **125 tests, OK** |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT |
| `build_matrix.py . --out <scratch>` | mapped 31 / unmapped 19 |

The workspace test count is unchanged at 2034: this lane adds no new test
function, it extends two pinned case sets that existing tests already iterate.

Load note: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` failed once on
its recorded load-flaky wall-clock budget (`elapsed < 500ms`) during a
contended run and passed in the clean run above. That is the pre-existing
P02-FLAKE row; no assertion was weakened and this lane does not touch it.

## 10. Integrator carry list

1. **`docs/reviews/r0_3/acceptance_matrix.json` is NOT edited by this lane.**
   Regenerate it whole-file with
   `python3 docs/reports/remaining-d02/build_matrix.py . --gate <gate.json>`
   at the merged numbers. Its CT section will pick up CT33 as mapped, the
   31/19 totals, and CT34/CT46's new `blocker_current` /
   `blocker_superseded_by` fields automatically.
2. **CT coverage line for the wave paragraph:** 30/50 → **31/50 mapped**,
   unmapped 20 → **19**, by executed-stage evidence only. Sanitized-native
   stays 3 of 31 and proof 0 of 31 — the denominator grew by mapping, not by
   conformance.
3. **Nothing is promoted.** No raw Cycle 19 unit moves, the contract corpus
   stays 60 modules, no schema or crate version moves, and the four pinned
   185-unit artifacts are untouched by construction (no gateway, audit,
   contract or schema file is in this lane's diff).
4. **Two follow-ons named, not opened as work here:**
   * `CT33-SANITIZED` — upgrading CT33's `native_sanitized` from `unassessed`
     to `executed` needs a `checked_mutex_sanitized` harness fixture over the
     accepted control plus a deliberate edit of `SANITIZED_GATED_CASES`. The
     ingredient exists at the core level (card CT33-RANKS §9); the harness
     route does not.
   * `D02-BLOCKER-STALE` — T00's archived blocker texts are frozen and others
     besides CT34's may have decayed the same way. This lane re-verified
     **only** CT34 and CT46, the two ids in its brief; the remaining
     seventeen unmapped rows were not audited.
5. **`docs/reports/wave11-ct33/ct_stage_evidence.json` is now load-bearing** in
   two places: `scripts/ct_stage_summary.py::EVIDENCE_FILES` (provenance check —
   a mapped case with no archived record is a DRIFT line) and
   `build_matrix.py`'s superseding-reason reader. Do not move or rename it
   without updating both.
