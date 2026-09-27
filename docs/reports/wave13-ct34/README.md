# Wave-13 CT lane — CT34 mapped from card CT34-GROWTH's proposal

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Wave 13. One case moves: **CT34** (protected owned graph grows/replaces
identities) maps on the existing `mutex_guard` route. CT coverage moves
**31/50 → 32/50 mapped**, unmapped **19 → 18**.

Base: `main` at `bf210eb`. All work is on this lane's branch.

Nothing is promoted. This is a mapping increment: no checker rule, no
diagnostic code, no lowering and no model changed, and the grown denominator is
mapping rather than conformance. Sanitized-native stays **3 of 32**
(CT03/CT17/CT26) and proof stays **0 of 32**.

`git diff` over `crates/` and `compiler/src/` is empty; the only Rust touched is
the harness (`compiler/tests/ct_corpus.rs`) — its pinned case sets plus one new
*control expectation*, which only ever adds assertions (§4).

## 1. What was mapped, and from what

Card CT34-GROWTH (`docs/reports/remaining-ct34-growth/README.md`, wave 12) gave
protection growth a checked existence in `CheckedMutexInvariantV1` under five
conditions, a source form (`grow g guards y;`), and — in the same change — a
recorded decision that the C backend refuses to render it. Its §7 proposed
CT34's mapping and left it for the CT lane.

This lane executed that proposal: it moved the case into the harness fixture
layout, **re-measured the cascade and the one-token control on the fixture bytes
before touching the manifest**, and mapped only the stages that really executed.

The proposal survived measurement in substance and needed one structural change,
found by measurement and recorded in §4: the one-token control cannot be an
`accept` control, because the route that admits the growth is check-only.

## 2. Fixture

`compiler/tests/fixtures/ct/CT34/`

| File | Role |
|---|---|
| `case.qcore` | the scenario: three resources, two generations, `m` grown onto memory `n` already protects |
| `growth_onto_unprotected.qcore` | the admitted control — the case with **one token changed** |
| `growth_over_own_footprint.qcore` | refused control — the growth aimed at the generation's own declared interval |
| `expected.json` | verdict mode, exact cascade, controls, stage list, notes |

The case, verbatim:

```qcore
import std.concurrency;
fn main() -> unit {
    thread scope s {
        let x = protected 4;
        let y = protected 4;
        let z = protected 4;
        let m = mutex guards x;
        let n = mutex guards y;
        with lock m as g { open g; grow g guards y; close g; }
    }
}
```

`growth_onto_unprotected.qcore` is that file with `y` replaced by `z` on the
growth line, and nothing else. `growth_over_own_footprint.qcore` is that file
with `y` replaced by `x` on the growth line, and nothing else.

## 3. Measured cascade and controls

Measured first through `compile_files` on the committed fixture bytes and,
independently, through the card's own route (parse → elaborate → lower →
`check_mutex_invariant_program`), then re-measured through the harness runners
(`run_mapped_case` / `run_mutex_controls`) after the manifest landed. The two
routes agree on the case and on the refused control.

| Artifact | Expectation | Measured |
|---|---|---|
| `case.qcore` | reject | `["Q613","Q617","Q619","Q613","Q600","Q600","Q600"]` |
| `growth_onto_unprotected.qcore` | accept_check_only | core **accepts**, 1 admitted growth; `compile_files` refuses `["Q600"]` |
| `growth_over_own_footprint.qcore` | reject | `["Q613","Q617","Q619","Q613","Q600","Q600","Q600"]` |

The case's seven diagnostics in full, as the compiler renders them:

```
CT34/case.qcore:9:62: Q613: InvariantOpenAtUnlock { event: 6, guard: 1 }
CT34/case.qcore:9:36: Q617: FootprintAlreadyProtected { event: 4, mutex: 1 }
CT34/case.qcore:10:5: Q619: NonquiescentMutexDestruction { event: 8, mutex: 1 }
CT34/case.qcore:9:53: Q613: InvalidInvariantWitness { event: 5, guard: 1 }
CT34/case.qcore:10:5: Q600: NonquiescentDestruction { event: 11, resource: 1 }
CT34/case.qcore:3:5:  Q600: OutstandingGuard { guard: 1 }
CT34/case.qcore:3:5:  Q600: OutstandingMutex { mutex: 1 }
```

This reproduces card CT34-GROWTH §4.1's pinned vector exactly, on this lane's
own fixture bytes.

### 3.1 Every entry, and why the growth's own diagnostic sits second

| # | Entry | What it says |
|---|---|---|
| 1 | `Q613 InvariantOpenAtUnlock { event: 6, guard: 1 }` | the section unlocks with its invariant still open, because its `close` was refused |
| 2 | `Q617 FootprintAlreadyProtected { event: 4, mutex: 1 }` | **the growth's own refusal**: W4, the new interval already has a protector |
| 3 | `Q619 NonquiescentMutexDestruction { event: 8, mutex: 1 }` | the generation is destroyed with its guard still live |
| 4 | `Q613 InvalidInvariantWitness { event: 5, guard: 1 }` | the refused growth installed nothing, so `close` names a declared identity the generation never took |
| 5 | `Q600 NonquiescentDestruction { event: 11, resource: 1 }` | scope exit finds the protected resource still held |
| 6 | `Q600 OutstandingGuard { guard: 1 }` | scope exit finds the guard outstanding |
| 7 | `Q600 OutstandingMutex { mutex: 1 }` | scope exit finds the generation outstanding |

The leading diagnostic is **not** the growth's own. That is card boundary
**CT34-GROWTH-CASCADE**, and it is recorded rather than concealed: the lowering
mints the new predicate identity before the core decides, so when the core
refuses the growth the generation keeps its old identity and the section's
`close` names one it never took; the core then reports in its own enum order,
which puts the unlock complaint first. Filtering the vector to its "interesting"
entry would hide exactly the evidence entries 3–7 carry — that the refusal left
the generation's shape, its declared identity and the share ledger exactly as
they were. A change that started mutating a generation on a refused growth would
move this vector.

### 3.2 The one-token control

`growth_onto_unprotected.qcore` differs from the case by a single token (`z` for
`y` on the growth line). The core admits it and publishes a certificate
recording **one** admitted protection growth, with

```
protected_shape = { 1: [x[0,4), z[0,4)],  2: [y[0,4)] }
```

so generation 1 really widened and generation 2 is untouched. That makes `Q617`
attributable to *the interval already having a protector* — not to growth being
written down at all, not to the third resource existing, and not to the second
generation being declared.

The harness asserts the growth was recorded, not merely that the module checked:
a control that parsed, elaborated, lowered and checked while performing no
growth would attribute nothing, and mutation M25 (§5) shows that assertion
firing.

### 3.3 The second control

`growth_over_own_footprint.qcore` aims the growth at `x`, generation `m`'s **own**
declared interval, and refuses with the identical seven-entry cascade. W4's rule
is "the new interval must overlap no live generation's declared shape, **this
generation's included**"; this control makes that half a measured statement in
the corpus rather than prose in a card, and shows the rule is about protection
rather than about which generation holds it.

## 4. One structural deviation from the proposal, found by measurement

Card CT34-GROWTH §7 proposed the one-token control as an ordinary accepted
control. Measured on the fixture bytes **before the manifest was touched**, that
is not available:

```
$ qcorec check growth_onto_unprotected.qr
Q600: the checked mutex backend refused this trace: UnsupportedEvent { event: 4 }
```

The card's own §4 matrix measured its rows through parse → elaborate → lower →
core check. The CT harness's `accept` expectation goes further: it requires
`compile_files` to return a compiled module. `compile_files`'s mutex route runs
the C backend, and card CT34-GROWTH boundary 3 (**CT34-GROWTH-NATIVE**) is that
both `emit_mutex_c11` and `interpret_mutex_program` deliberately refuse a
`GrowProtection` event, because nothing in that backend reads a generation's
declared shape and an emitted `mtx_t` would protect less than the model says.
So the growth route is **checked but not compiled**, and no `accept` control on
it can exist while that decision stands.

Rather than weaken the `accept` expectation, drop the one-token control, or
quietly demote it to a `reject` control (which would say nothing about whether
the growth was admitted), this lane added a third control expectation:

**`accept_check_only`** asserts *both* halves and neither alone:

1. the control's own bytes parse, elaborate, lower and **check**, and the
   published certificate records at least one admitted protection growth; and
2. `compile_files` refuses with the **exact ordered vector** the fixture pins
   (`["Q600"]`), so the check-only admission is never reported as a full
   acceptance and the backend's refusal stays a recorded decision.

Direction of the change is **strictly more assertion**:

| Rule | Before | After |
|---|---|---|
| `accept` control | `diagnostic_sequence` empty | empty **and** `backend_refusal` empty |
| `reject` control | `diagnostic_sequence` non-empty | non-empty **and** `backend_refusal` empty |
| `accept_check_only` | — | `diagnostic_sequence` empty, `backend_refusal` non-empty, growth recorded, exact backend vector |
| a mutex rejection's controls | at least one control | at least one control **and** at least one that is not a `reject` |

The last row is new and applies corpus-wide. Every pre-existing mutex rejection
case was measured to satisfy it before the assertion was added
(CT27/CT29/CT31/CT32 carry one `accept`; CT28/CT30/CT33/CT48 carry `accept` plus
`reject`), so it tightened the rule without moving any case.

`evidence_target` is unaffected in meaning and stays `linux`: the existing
target-scoped-evidence rule is an exact iff over control kinds that reach
`compile_files`, and both admitted kinds do — the mutex route refuses a host
outside {linux, macOS, Windows} at `Q500` *before* the backend is reached, so an
`accept_check_only` control is host-gated in precisely the way an `accept`
control is. It records the `Q500` gate and **no** compiled or executed C. The
predicate was extended to name the new kind rather than left to fall through to
`None`, and mutation M20 (§5) shows dropping `evidence_target` refusing.

## 5. Stage rows

Derived by `ct_corpus.rs::derive_stage_plan` from the harness's own dispatch and
the codes the compiler really produced, then compared against the manifest. The
manifest is checked, not self-certifying. No change to `derive_stage_plan` was
needed: the case's codes are all in the ≥600 band, so the derived plan is exactly
the proposal's.

| Stage | Status | Executor | Artifact | Obs |
|---|---|---|---|---|
| `parse` | executed | `ct_corpus.rs::run_mapped_case` | `CT34/case.qcore` | 1 |
| `elaborate` | executed | `ct_corpus.rs::run_mapped_case` | `CT34/case.qcore` | 1 |
| `mutex_lowering` | executed | `ct_corpus.rs::run_mapped_case` | `CT34/case.qcore` | 1 |
| `mutex_invariant_check` | executed | `ct_corpus.rs::run_mapped_case` | `CT34/case.qcore` | 1 |
| `mutex_reference_execution` | not_applicable | — | — | — |
| `c_emission`, `native_execution` | not_applicable | — | — | — |
| `native_sanitized` | **unassessed** | — | — | — |
| `proof` | **unassessed** | — | — | — |
| every task / epoch / sequential / `legacy_c` stage | not_applicable | — | — | — |

Deepest executed stage: `mutex_invariant_check`. `harness_verdict_mode: reject`;
`route: mutex_guard`; `evidence_target: linux`.

`native_sanitized` is **unassessed and was deliberately not upgraded**, per the
proposal: this card carries no sanitized native run, and the schema-2 gate
(`SANITIZED_GATED_CASES`) lists exactly which cases may read `executed`. Note
that `not_applicable` would be equally wrong and is refused by the harness (M14):
the `mutex_guard` route *does* have a `c_emission` stage — CT26 compiles and runs
under ASan/UBSan/LSan — so the route rule leaves this case at `unassessed` rather
than declaring the stage structurally absent. That this particular case emits no
C is a fact about the case, and schema 2 decides `native_sanitized` by route.

There is no CFG row to leave open: the `mutex_guard` route emits from a checked
event list and constructs no CFG at all, so schema 2's route vocabulary omits
every CFG-bearing stage. Unchanged from wave 5, made structural by
T00-CT-SCHEMA.

## 6. Mutation checks

Twenty-seven scratch mutations, each applied to a committed file, run through the
CT harness **and** `scripts/ct_stage_summary.py --check`, then reverted.
**All twenty-seven refuse.** Nothing was committed; `git status` after the runs
is clean, and the baseline passes again afterwards.

| # | Mutation | Result | Refused by |
|---|---|---|---|
| M1 | cascade entry `Q617` dropped from `diagnostic_sequence` | REFUSED | exact diagnostic cascade mismatch |
| M2 | cascade order swapped to `["Q617","Q613",…]` | REFUSED | exact diagnostic cascade mismatch |
| M3 | admitted control's expectation flipped to `reject` | REFUSED | target-scoped evidence rule (no admitted control left, so `evidence_target` must be null) |
| M4 | admitted control's expectation flipped to `accept` | REFUSED | an accepted control carries no `backend_refusal` |
| M5 | admitted control's `backend_refusal` emptied | REFUSED | a check-only control must state the refusal that makes it check-only |
| M6 | admitted control's `backend_refusal` changed to `["Q613"]` | REFUSED | exact backend refusal vector mismatch |
| M7 | own-footprint control's expectation flipped to `accept` | REFUSED | the control must check (`run_mutex_controls`) |
| M8 | own-footprint control's cascade shortened to `["Q617"]` | REFUSED | control diagnostic-vector mismatch |
| M9 | all controls removed (refusal made vacuous) | REFUSED | target-scoped evidence rule + shape rule |
| M10 | only the refusing control kept | REFUSED | **new**: a mutex rejection needs an admitted control |
| M11 | fixture verdict mode changed to `accept` | REFUSED | the declared route must be the one that verdict mode takes |
| M12 | manifest declares `harness_verdict_mode: checked_mutex_sanitized` | REFUSED | the mode must be the one that produced the evidence |
| M13 | manifest claims `native_sanitized: executed` | REFUSED | harness derives `unassessed` (sanitizer gate); acceptance derivation also refuses |
| M14 | manifest claims `native_sanitized: not_applicable` | REFUSED | harness derives `unassessed` (route rule); acceptance derivation also refuses |
| M15 | manifest claims `mutex_reference_execution: executed` | REFUSED | harness derives `not_applicable`; acceptance derivation also refuses |
| M16 | manifest claims `route: task` | REFUSED | declared route must be the real one |
| M17 | manifest names a *control* as a stage's artifact | REFUSED | a stage runs on the case's own fixture |
| M18 | manifest inflates an observation count to 2 | REFUSED | count must equal the runner's invocations; acceptance derivation also refuses |
| M19 | manifest reverts CT34 to `unmapped` with the fixture present | REFUSED | pinned mapped-case count, then the fixture-vs-status rule |
| M20 | manifest drops `evidence_target` | REFUSED | target metadata mismatch against `expected.json` |
| M21 | `stages_executed` claims `mutex_reference_execution` | REFUSED | must match the derived plan |
| M22 | case source grows onto the unprotected `z` (refusal removed) | REFUSED | exact diagnostic cascade mismatch |
| M23 | case source drops the second generation (nothing protects `y`) | REFUSED | exact diagnostic cascade mismatch |
| M24 | control source grows onto the protected `y` | REFUSED | the control must check, so the case's refusal is not vacuous |
| M25 | control source drops the growth entirely | REFUSED | **new**: the control must actually record the growth it performs |
| M26 | `acceptance.json` left at 31/19 | REFUSED | both derivations (`ct_mapped` not what the manifest derives) |
| M27 | `acceptance.json` `case_depth` drops CT34 | REFUSED | both derivations (`case_depth` not what the manifest derives) |

Four rows are recorded honestly as refusing somewhere other than one might
predict:

* **M3 and M9** refuse through the target-scoped evidence rule rather than
  through a control assertion, because removing the only admitted control also
  removes the reason `evidence_target` may be set. Same shape as the wave-11
  lane's M3/M6.
* **M22 and M23** remove the case's *refusal*, but do not make it accept: the
  case then contains an admissible growth, so `compile_files` refuses at the
  backend with `["Q600"]` instead. They refuse on the exact-cascade assertion
  rather than on "expected rejection but the checker accepted". That is the
  check-only route showing through, and it is the honest reading.
* **M13/M14/M15/M18** were additionally re-run against the harness test alone, to
  confirm they are refused by the per-case derived plan and not only by the
  acceptance derivation that happened to report first. All four are:
  `CT34: stage 'native_sanitized' claims 'executed' but this harness derives
  'unassessed'`, and the analogous messages for the other three.

## 7. Deliberate pin updates

| Pin | Before | After |
|---|---|---|
| `ct_corpus.rs::MAPPED_CASES` | 31 ids | 32 ids (`+CT34`) |
| `ct_corpus.rs::MUTEX_REJECT_CASES` | 8 ids | 9 ids (`+CT34`) |
| `ct_corpus.rs` progress doc comment | 31 of 50 / 19 unmapped / 29 of 31 source-level | 32 of 50 / 18 unmapped / 30 of 32 source-level |
| `acceptance.json` `ct_mapped` / `ct_unmapped` | 31 / 19 | **32 / 18** |
| `acceptance.json` `depth_profile.mutex_invariant_check` | 8 | 9 |
| `acceptance.json` `observation_counts` | parse 31, elaborate 29, mutex_lowering 9, mutex_invariant_check 9 | 32, 30, 10, 10 |
| `acceptance.json` `case_depth` | (no CT34) | `CT34: mutex_invariant_check` |
| `acceptance.json` `stage_histogram` | 17 stages at 31 cases | 17 stages at 32 cases |
| `acceptance.json` `ct_evidence.note` | "3 of 31 … 0 of 31" | "3 of 32 … 0 of 32" |
| `acceptance.json` `derived_from` / `cause` | 8 entries / 7 reports | 9 / 8 (+ this lane's record) |
| `acceptance.json` `unmapped_bucket_counts` `blocked-on-guard-profile-composition-or-unrepresented-form` | 2 | 1 |
| `scripts/ct_stage_summary.py::EVIDENCE_FILES` | 7 files | 8 (`+docs/reports/wave13-ct34/ct_stage_evidence.json`) |
| `build_matrix.py::SUPERSEDING_REASONS` | `wave11-ct33/ct_stage_evidence.json`, asserted `{CT34, CT46}` | `wave13-ct34/ct_stage_evidence.json`, asserted `{CT46}` plus a new assertion that CT34 is mapped |
| manifest `note`, closing CT34/CT46 paragraph | both unmapped, CT34 on "no growth form" and CT46 on "needs Send/Sync derivation" | CT34 mapped with its `accept_check_only` control recorded; CT46 unmapped on its new measured reason |

Every generated field of `acceptance.json` was written by its own generator
(`python3 scripts/ct_stage_summary.py . --write`) and then verified by **both**
derivations, which agree:

* Rust: `ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` — passes.
* Python: `scripts/ct_stage_summary.py . --check` — exit 0, no DRIFT lines.

`docs/reports/remaining-d02/build_matrix.py`'s CT derivation was verified against
the new manifest with `--out` to a scratch path (never overwriting the committed
matrix): 50 cases, `mapped 32 / unmapped 18`, `parse` observations 32, and its own
independently derived unmapped-bucket counts sum to 18. Its CT34 row is a mapped
row carrying both auxiliary control records; its CT46 row carries the new
`blocker_current` / `blocker_superseded_by` pair.

Both JSON files were round-trip-checked (`json.loads` → `json.dumps(indent=2,
ensure_ascii=True)`) against `HEAD` before any edit, so the diffs contain only
the intended value changes and no reformatting.

## 8. Reason-text verification (task item 5)

### CT33 — mapped; text unchanged and re-verified

Card CT34-GROWTH measured the rank/growth interaction directly (its §4: a ranked
module with a growth and *ascending* nesting is accepted; the same with
*descending* nesting refuses at `["Q624","Q612"]`). Growth orders nothing, so
CT33's pinned `[Q624, Q612]` cascade, its one-statement ascending control and its
unranked `[Q622, Q622]` control are unaffected. Re-measured on this lane's tree
through the harness: CT33's case and both controls pass unchanged. **No edit.**

### CT34 — its stale blocker text leaves publication by the case being mapped

The acceptance matrix published CT34's blocker from T00's archived
`coverage.json`, frozen at revision `ea2af0f`:

> no source form creates or declares a mutex; the guard-binding blocker batch 2
> named was repaired but is no longer the limit

That first clause has been untrue since card T03, and the wave-11 lane published
a measured `blocker_current` beside it. Card CT34-GROWTH then supplied the growth
form, so CT34 is now **mapped** and simply leaves the unmapped triage rows: no
stale blocker text is published for it any more. The archived wave-11 file is
**not** edited — rewriting an archived record would destroy the history it exists
to keep — and `build_matrix.py` gains an explicit assertion that CT34 is mapped,
so "CT34 no longer appears in this table" is a checked transition rather than an
accident.

### CT46 — stale, and updated

The manifest's `note` still carried the wave-11 reason ("CT46 needs Send/Sync
derivation"). That is now **false**: card CT46-SENDSYNC (wave 12) landed a closed
`transferable(T)` judgment that decides both matrices whole, retiring exactly the
derivation the old text said was missing. The measured remaining obstacle is
different and is named **CT46-MUTEX-CAPTURE**: the mutex ledger and the task
ledger compose today by mutual exclusion — the mutex route admits no task capture
of a guarded payload and the task route refuses mutexes outright — so no source
text can ask for mutex-mediated sharing across a task boundary, and CT46's
*accept* half is unwritable. Mapping only its refusing half would answer a
different question.

Updated in two places, both checked:

* the manifest `note`'s closing paragraph, rewritten to state the retirement and
  the new obstacle;
* `build_matrix.py`'s `blocker_current` for CT46, now read from this lane's
  `ct_stage_evidence.json`, with `blocker_superseded_by` pointing at it. The
  reader is asserted (`set(_superseded) == {"CT46"}` and
  `set(_superseded) <= set(unmapped_ids)`), so it cannot go stale in turn.

CT46 stays **unmapped**. Retiring a wrong reason is not progress on the case.

## 9. Lean

**Judged not applicable, and recorded** — the same judgment cards
T00-CT-SCHEMA, T08-SRC-CT, CT-CTWIN and the wave-11 CT lane recorded for their
harness increments. This lane adds no checker rule, no diagnostic code, no
lowering and no model. It adds one fixture, one harness control expectation that
only ever adds assertions, one entry in each pinned case set, and the manifest and
acceptance rows derived from them, over an unedited checker whose own 18 growth
theorems landed with card CT34-GROWTH. `proof` stays `unassessed` for CT34; no
theorem is bound to any CT fixture (T00.7 remains open). The Lean gate was run
and passes at **57 audits**.

## 10. Gate

Run in this worktree at the final commit.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2318 passed, 0 failed, 5 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **57 audits pass** |
| `providers/` `cargo test --no-fail-fast` | **87 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | **125 tests, OK** |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT |
| `build_matrix.py . --out <scratch>` | mapped 32 / unmapped 18 |

The workspace test count is unchanged in *functions*: this lane adds no new test
function, it extends two pinned case sets that existing tests already iterate.

Load note: one earlier full-workspace run failed in
`qcore-verifier-host --test enforced_process_limits` — the recorded load-flaky
P02 row — during a contended run. It passed in isolation (12/12) and in the
clean full run reported above; the number above is from the run where it passed.
No assertion was weakened and this lane does not touch it.

## 11. Recorded, not repaired

**`D02-BUCKETS-STALE`.** `acceptance.json`'s hand-maintained
`unmapped_bucket_counts` no longer sums to `ct_unmapped`: after this lane's edit
it sums to **22** against `ct_unmapped: 18`. The gap is exactly its
`blocked-on-model-only-epoch-accounting (no source syntax): 4` entry, whose four
cases (CT35/CT36/CT38/CT39) were mapped by wave 8 without that hand-kept row
being decremented. This lane edited only the entry its own case left
(`blocked-on-guard-profile-composition-or-unrepresented-form`, 2 → 1) and did not
silently rewrite four rows belonging to another lane's record. The **derived**
bucket counts in `acceptance_matrix.json` are correct and unaffected —
`build_matrix.py` recomputes them from T00's triage over the currently-unmapped
ids and they sum to 18 — so the authoritative numbers are right and only the
hand-kept mirror in `acceptance.json` is behind.

## 12. Integrator carry list

1. **`docs/reviews/r0_3/acceptance_matrix.json` is NOT edited by this lane.**
   Regenerate it whole-file with
   `python3 docs/reports/remaining-d02/build_matrix.py . --gate <gate.json>`
   at the merged numbers. Its CT section will pick up CT34 as mapped, the 32/18
   totals, CT34's two auxiliary control records, and CT46's new
   `blocker_current` / `blocker_superseded_by` automatically.
2. **CT coverage line for the wave paragraph:** 31/50 → **32/50 mapped**,
   unmapped 19 → **18**, by executed-stage evidence only. Sanitized-native stays
   3 of 32 and proof 0 of 32 — the denominator grew by mapping, not by
   conformance.
3. **Nothing is promoted.** No raw Cycle 19 unit moves, the contract corpus stays
   60 modules, no schema or crate version moves, and the pinned 185-unit
   artifacts are untouched by construction (no gateway, audit, contract or schema
   file is in this lane's diff).
4. **One harness capability is new and worth naming in the wave paragraph:** the
   `accept_check_only` control expectation, which lets a control on a
   *check-only* feature attribute a refusal without claiming an emission that
   the backend deliberately refuses. It is strictly more assertion, and it
   tightened the corpus-wide rule that a mutex rejection must carry an admitted
   control.
5. **Three follow-ons named, not opened as work here:**
   * `CT34-SANITIZED` — CT34's `native_sanitized` cannot be upgraded while
     CT34-GROWTH-NATIVE stands: the growth route emits no C by pinned decision.
     Upgrading it needs the backend to render a grown generation first, which is
     a checker/backend card, not a CT lane.
   * `CT46-MUTEX-CAPTURE` — carried forward from card CT46-SENDSYNC and now the
     published current blocker for CT46.
   * `D02-BUCKETS-STALE` — §11.
6. **`docs/reports/wave13-ct34/ct_stage_evidence.json` is now load-bearing** in
   two places: `scripts/ct_stage_summary.py::EVIDENCE_FILES` (provenance check —
   a mapped case with no archived record is a DRIFT line) and
   `build_matrix.py`'s superseding-reason reader. Do not move or rename it
   without updating both. The wave-11 record stays load-bearing for CT33 in the
   first of those two.
