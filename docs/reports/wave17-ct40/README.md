# Wave-17 CT lane — CT40 mapped on the `legacy_c` route

Wave 17, card `CT40-LANE`. One case moves: **CT40** ("atomic pointer points to
prematurely freed memory") maps as a C twin on the existing `legacy_c` route.
CT coverage moves **32/50 → 33/50 mapped**, unmapped **18 → 17**.

Base: `main` at `8ec8402` (wave-16 consolidation). Branch:
`worktree-agent-a6aebbf98373cb48b` (isolated worktree).

Nothing is promoted. This is a mapping increment: no checker rule, no
diagnostic code, no lowering and no model changed — `git diff` over `crates/`
and `compiler/src/` is **empty** — and the grown denominator is mapping, not
conformance. Harness-gated sanitized-native evidence stays at **4** (CT03,
CT17, CT26, CT34) and proof stays at **0**, so the honest reading is 4 of 33
and 0 of 33: the case added is a C twin on a route that emits nothing and
therefore *structurally cannot* contribute to either.

---

## 1. What the parked candidate rows said, and what re-measurement changed

Card `CTWIN-PUBLISH` (wave 16) closed the gateway hazard that had kept CT40
unmapped and recorded candidate schema-2 rows at
[`docs/reports/remaining-ctwin-publish/artifacts/ct40-candidate-rows.json`](../remaining-ctwin-publish/artifacts/ct40-candidate-rows.json),
explicitly for the next CT lane. Following the wave-8 precedent — parked rows
are re-measured, never trusted — every value was measured again on **this
tree's own fixture bytes** before the manifest was touched.

The candidate rows survive in substance. Two corrections, one of them
load-bearing:

| # | Parked candidate | Re-measured | Consequence |
|---|---|---|---|
| C1 | diagnostic rendered `subject:4:unknown_alias:p` | the harness renders `subject:4:unknown-alias:p` — a **hyphen** | Adopting the parked string verbatim would have failed the pin on its first run. The candidate was measured through the gateway directly; the CT vocabulary is `ct_corpus.rs::legacy_diagnostic_label`'s, deliberately an explicit match rather than `Debug`, so it cannot drift with the gateway (card CT-CTWIN §2.1). |
| C2 | one control (`no_publication.c`, safe) | three controls | The parked row satisfies the attribution rule with one control, but publishes the `_Atomic` invariance only as prose in a report. This lane makes it a **fixture control**, and adds a third for the other half of the scenario title. §3. |

Everything else reproduced exactly: the unit verdict, the verbatim withdrawal
reason, the per-function status map, the diagnostic *content* and order, and
the parked stage plan.

## 2. The pinned verdict

`compiler/tests/fixtures/ct/CT40/case.c`, whose whole body is:

```c
void *q_alloc(void);
void q_free(void *p);

void *_Atomic g_published;

void subject(void) { void *p = q_alloc(); g_published = p; q_free(p); }
```

analyzed through the real gateway (`validate_unrestricted_c11` then
`analyze_unrestricted_c11`) with `subject` as the declared entry, against the
same synthetic `q_*` contract surface CT43 and CT45 use:

| Row | Measured |
|---|---|
| `route` | `legacy_c` |
| `harness_verdict_mode` | `legacy_c_twin` |
| `semantic_profile` | `UnrestrictedC11ResourceProjection` |
| unit verdict | `unknown` |
| reason (verbatim) | `resource projection unresolved: 1 unsupported operation(s), 1 unresolved alias(es), 0 custody summary conflict(s)` |
| `function_status` (complete) | `{ "subject": "unknown" }` |
| diagnostics (complete, **ordered**) | 1. `subject:4:unknown-alias:p`<br>2. ``subject:4:unsupported:publication of a tracked resource into the static-storage object `g_published` is outside the resource projection`` |

Line 4 is the position in the **preprocessed** text (`-E -P`), which is what
the gateway reports. The whole verdict is rebuilt from the analysis and
compared wholesale under the CT-CTWIN convention: unit result, verbatim reason,
the **complete** per-function status map, and the **complete ordered**
diagnostic inventory (mutations M01, M02, M09).

### Stage records

| Stage | Status | Evidence |
|---|---|---|
| `parse` | executed | `ct_corpus.rs::run_legacy_c_twin`, 1 observation, `tool_gate: ["clang"]`, artifact `compiler/tests/fixtures/ct/CT40/case.c` |
| `legacy_c_twin` | executed | same runner, 1 observation, same tool gate and artifact |
| `proof` | unassessed | no theorem is bound to any CT fixture (T00.7 remains open) |
| every other stage | `not_applicable` | structurally absent on this route |

**`native_sanitized` is `not_applicable`, and that is the structural value the
schema prescribes**, not an unassessed shrug and not an overclaim: the route
has no `c_emission` stage because the gateway *analyzes* C rather than
producing any, so there is nothing to sanitize. That is the rule card
T08-SRC-CT narrowed for the check-only epoch route and card CT-CTWIN applied to
this one; the route vocabulary, not the case, decides it. Claiming `executed`
there is refused three ways at once (mutation M12).

The three controls execute on their **own** artifacts, so they are not counted
as this case's observations: `legacy_c_twin` reads 1, not 4. A
contract-varying control would have counted (the one deliberate exception,
CT45); CT40 carries none of that kind.

## 3. Three controls — one that moves the verdict, two pinned for not moving it

| Control | Role | Measured | What it establishes |
|---|---|---|---|
| `no_publication.c` | attribution | `safe`, `{subject: safe}`, no diagnostic | Removing **exactly** `g_published = p;` moves the verdict. Clause (b) of the wave-9 attribution rule, for the publication. |
| `plain_pointer.c` | invariance | byte-identical to the case in verdict, reason, statuses and ordered diagnostics | `_Atomic` is **not load-bearing**. |
| `no_release.c` | boundary | `unknown`, reason `… 1 unsupported operation(s), 0 unresolved alias(es) …`, **one** diagnostic (the publication) | The refusal is the publication, not the premature free. |

### 3.1 The lane decision the parked rows deliberately left open

`CTWIN-PUBLISH` measured that `_Atomic` is non-load-bearing and wrote: *"This
card does not decide whether that satisfies the lane's rule."* This lane
decides it, and states the reasoning rather than the conclusion alone.

The wave-9 rule requires the verdict to be **(a)** the class the scenario
demands and **(b)** attributable to *the construct the scenario names*. CT40's
`expected` field — the normative half of a CT row, and the half the harness
measures against — reads **"Not certified; unsupported lifetime/publication
protocol"**. The gateway answers `unknown` (= not certified), names the
publication protocol verbatim in its own diagnostic, and the control that
removes exactly the publication statement moves the verdict. Both clauses hold
on that text.

What does **not** hold is stated in the fixture rather than buried:

* the projection derives no atomicity or synchronization fact, so no qualifier
  on the destination can move one of its verdicts — the same argument
  `CTWIN-PUBLISH` §5 makes for `volatile` and CT41;
* this projection does not decide "prematurely freed" either: publishing with
  no release at all is refused just the same.

So the mapping records that the gateway **declines to certify a publication**,
which is what CT40's expectation asks for in words, and it records nothing
about a dangling atomic pointer. Pinning both invariances as fixture controls
is what makes that bound structural: if a future change ever made `_Atomic`
load-bearing here, `plain_pointer.c` moves and the mapping's stated scope has
to be re-argued rather than quietly widening. Mutation M06 — flipping that
control to `safe`, i.e. claiming atomicity *is* what buys the refusal — is
refused by the harness.

## 4. CT41 and CT42 — blockers re-verified on this tree, both still unmapped

Neither is mapped. Both recorded blockers were re-measured rather than
inherited.

### CT41 — accurate, and now a decision rather than a gap

Three shapes measured over the same `q_*` surface: a `volatile int` completion
flag beside an acquisition and release, the same body with a plain `int`, and
the body with no flag at all.

| Shape | Measured |
|---|---|
| `volatile int done; … done = 1; …` | `unknown`, `[subject:4:unknown-alias:p, subject:4:unsupported:unsupported assignment expression]` |
| `int done;` — identical otherwise | **byte-identical** |
| no flag at all | `safe`, no diagnostic |

The qualifier moves nothing; the store does. The blocker is accurate, and
`CTWIN-PUBLISH` §5 has since converted it from an open question into an
asserted decision, cited verbatim in the lane's evidence record and quoted
here:

> *"the resource projection has no synchronization model; the `volatile`
> invariance is asserted as correct by `CTWIN-PUBLISH` §5 rather than open"*

— because C11 `volatile` is an access-ordering qualifier for the abstract
machine, not a synchronization primitive (§5.1.2.4 gives happens-before to
atomics, mutexes and thread creation/join only), so a projection deriving no
synchronization fact **must** answer identically with and without it.
Answering differently would be the defect. A mapping would be vacuous under
clause (b).

The published `blocker_current` is updated through the dual-publication
mechanism: the wave-17 record carries CT41's row, **the bucket label is
unchanged**, and no supersession is declared — re-verification reaching the
same label is the record saying the label still holds. Inventing a new bucket
for a decision that changes no classification would be label churn and would
move the tally shape for a case nothing about which moved.

### CT42 — unchanged, re-verified

| Probe | Measured on this tree |
|---|---|
| `extern fn q_open_fd() -> i32` with an owned result | contract loader refuses, verbatim: `contracts.qcore: q_open_fd at byte 18: owned result requires a pointer return type` |
| descriptor twin with no such contract | `unknown` on three uncontracted calls (`q_close_fd`, `q_dup_fd`, `q_open_fd`) |
| pointer-alias analogue (`void *b = a;` + two releases) | provisional `unsafe`, one `double-release` — a *syntactic* alias surviving, which is a different question |

An int-typed resource identity is still not expressible at all, and no verdict
measured here establishes a *preserved identity*, which is the positive
modelling property the case asks for. `CT-CTWIN-DESCRIPTOR` stays open;
CT42's row is re-published unchanged.

## 5. The one mechanism change: retiring a row instead of editing an archive

Mapping CT40 hit a real hole in the label mechanism. `docs/reports/wave9-ctwin/ct_stage_evidence.json`
holds CT40's `unmapped_reasons` row, and **both** derivations refuse a lane row
naming a case the manifest maps. That left two bad options — refuse the
mapping, or edit an archived record — and the second is exactly the practice
card `CT-LABELS` built the dual-publication mechanism to replace and card
`HYGIENE-16` reverted (`CT-LABELS-T00-REVERT`).

The third option is the one `supersedes` already uses for labels: the mapping
lane declares the retirement **in its own record**, under
`retired_unmapped_reasons` as `{CASE: {retires, mapped_by}}`, and the earlier
record stays **byte-identical**. `docs/reports/wave9-ctwin/ct_stage_evidence.json`
is untouched by this lane; so is `docs/reports/remaining-t00/coverage.json`.

Total-or-reject in both derivations, mirrored line for line
(`ct_corpus.rs::lane_row_retirements`, `ct_stage_summary.py::_lane_row_retirements`):

| Refusal | Why it is a real way to go wrong |
|---|---|
| a retirement naming a case the manifest still leaves **unmapped** | it would silently drop a live label |
| `retires` not a stage-evidence record | citing evidence that is not there |
| `retires` naming a record that carries **no row** for that case | vacuous retirement |
| `retires` not the **newest** record carrying one | a retirement going stale past a supersession |
| `retires` not strictly **earlier** than the retiring record | a lane retiring its own row, or the future |
| two records retiring one case | a second, silent claim |
| a row for a mapped case with **no** retirement | unchanged: silence is not a retirement |

Seven new Python tests drive each direction plus the admitted one; four
scratch mutations (M20–M23) reach the same refusals through the Rust
derivation.

## 6. Fail-closed verification — 24 scratch mutations, 24 refused

Each was applied to a clean tree, run against the named gate, and restored from
bytes held in memory (two of the files are new and untracked, so `git checkout`
could not restore them). **None was committed**; the tree was verified clean
afterwards, and the driver's own output is archived at
[`artifacts/mutations.json`](artifacts/mutations.json).

| # | Mutation | Refused by |
|---|---|---|
| M01 | diagnostic order swapped | `CT40 (case): the whole ordered diagnostic inventory` |
| M02 | pin a convenient subset of the diagnostics | same |
| M03 | claim the twin is provisionally unsafe | `CT40 (case): gateway verdict` |
| M04 | inflate the withdrawal reason's alias count | `the withdrawal reason is part of the measurement` |
| M05 | drop the accepted control, keep only the refusals | `a C twin needs a control the gateway checks` |
| M06 | **claim `_Atomic` is load-bearing** (invariance control flipped to `safe`) | `CT40 (plain_pointer.c): gateway verdict` |
| M07 | claim the pre-`CTWIN-PUBLISH` leak for the no-release control | `CT40 (no_release.c): gateway verdict` |
| M08 | `case.c` loses the publication (the case becomes its own control) | `CT40 (case): gateway verdict` |
| M09 | drop the per-function status map's only entry | shape rule: the map is the measurement, not a sample |
| M10 | `legacy_c_twin` observations 1 → 2 | derived plan mismatch + acceptance drift |
| M11 | drop the `parse` stage's `tool_gate` | ``must record the exact external tools its runner needs`` |
| M12 | promote `native_sanitized` to executed | route rule + sanitizer gate + derived-summary drift |
| M13 | declare the `task` route instead | `the declared route must be the one this verdict mode really takes` |
| M14 | credit the stage to CT43's artifact | ``must be executed on the case's own fixture`` |
| M15 | record the `reject` verdict mode | `the manifest must record the harness mode that produced its stage evidence` |
| M16 | back to `unmapped` while the fixture exists | mapped-count pin (32 ≠ 33) + fixture-exists gate |
| M17 | `acceptance.json` left at `ct_mapped: 32` | Rust `ct_mapped is generated from the manifest` |
| M18 | `unmapped_bucket_counts` re-inflated by hand | `DRIFT: … is not what the manifest derives` |
| M19 | the wave-17 record's own stage count inflated | `DRIFT: CT40/legacy_c_twin: manifest counts 1 runs, … counts 2` |
| M20 | the retirement is dropped | `records an unmapped-case bucket, but the manifest maps this case` |
| M21 | the retirement names the wrong record | `retires the bucket in …, but the record holding it is …` |
| M22 | a lane retires its **own** row | `retires …, which is not an earlier record` |
| M23 | a row claims CT43, a case the manifest maps | `records an unmapped-case bucket, but the manifest maps this case` |
| M24 | the wave-17 record removed from the Python file set | `derived_from` pin + every-mapped-case-has-a-record test |

## 7. Derived numbers

Both derivations agree, independently:

```
ct_mapped 33   ct_unmapped 17
depth_profile     legacy_c_twin 3, epoch_bound_check 4, task_lowering 1,
                  task_ledger_check 4, mutex_invariant_check 8,
                  native_execution 9, native_sanitized 4
observations      parse 32 -> 33, legacy_c_twin 3 -> 4
tool_scope        legacy_c_twin ["clang"], parse ["clang","none"], all others ["none"]
native_sanitized  executed 4, recorded_not_gated 5, not_applicable 7, unassessed 17
proof             unassessed 33
unmapped_label_sources   4 of 17 lane-measured (CT11 wave 3, CT41/CT42 wave 17,
                         CT46 wave 13); 13 still t00-archive
```

* Rust: `ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` — passes.
* Python: `scripts/ct_stage_summary.py . --check` — exit 0, no DRIFT lines.
* `docs/reports/remaining-d02/build_matrix.py . 0 --out <scratch>` — 50 cases,
  `mapped 33 / unmapped 17`, bucket counts summing to 17, CT40 a mapped row
  carrying `legacy_c_controls_executed: 3` and naming this lane's evidence
  record, CT41/CT42 carrying `blocker_current` from it. Written to a scratch
  path only; the committed matrix is **not** edited by this lane (§13).

Both JSON files were round-trip-checked (`json.loads` → `json.dumps(indent=2,
ensure_ascii=True)`) against `HEAD` before any edit, so their diffs carry only
the intended value changes and no reformatting.

## 8. Deliberate pin updates

| Pin | Before | After |
|---|---|---|
| `ct_corpus.rs::MAPPED_CASES` | 32 ids | 33 (`+CT40`) |
| `ct_corpus.rs::LEGACY_C_TWIN_CASES` | `CT43, CT45` | `CT40, CT43, CT45` |
| `ct_corpus.rs::EVIDENCE_FILES` | 9 files | 10 (`+docs/reports/wave17-ct40/ct_stage_evidence.json`) |
| `ct_corpus.rs` progress doc comment | 32 of 50 / 18 unmapped / 30 of 32 source-level / 4 of 32 sanitized / 0 of 32 proof | 33 / 17 / 30 of 33 / 4 of 33 / 0 of 33 |
| `scripts/ct_stage_summary.py::EVIDENCE_FILES` | 9 files | 10 |
| `acceptance.json` `ct_mapped` / `ct_unmapped` | 32 / 18 | **33 / 17** |
| `acceptance.json` `depth_profile.legacy_c_twin` | 2 | 3 |
| `acceptance.json` `observation_counts` | parse 32, legacy_c_twin 3 | 33, 4 |
| `acceptance.json` `case_depth` | (no CT40) | `CT40: legacy_c_twin` |
| `acceptance.json` `stage_histogram` | 17 stages at 32 cases | 17 stages at 33 cases |
| `acceptance.json` `unmapped_bucket_counts` `blocked-on-gateway-expressiveness (C-twin measured)` | 3 | 2 |
| `acceptance.json` `unmapped_label_sources` | CT40/CT41/CT42 → wave-9 record | CT40 gone; CT41/CT42 → this lane's record |
| `acceptance.json` `derived_from` / `cause` | 9 records / 9 reports | 10 / 10 |
| `acceptance.json` `ct_evidence.note` | "4 of 32 … 0 of 32", "5 of the 18 measured" | "4 of 33 … 0 of 33", "4 of the 17 measured", plus the retirement mechanism |
| manifest `note` | legacy_c route is CT43/CT45 | CT40 joins, with its two invariance controls and what the mapping does not claim |
| `test_ct_stage_summary.py` label pins | CT40/CT41/CT42 lane-measured by wave 9 | CT41/CT42 by wave 17; `CT40 not in labels` asserted in both places |
| `test_the_historical_stale_bucket_block_is_refused` | sum pinned at literal 22 | `ct_unmapped + 4` — the four undecremented rows are what is pinned, not the total |
| `build_matrix.py` CT notes | "Twenty-eight … CT43 and CT45"; "Only CT03, CT17 and CT26 carry the harness-gated sanitized run" | thirty / three C twins; **CT34 added** — a pre-existing staleness since card CT34-SANITIZED, repaired in passing because it is in a note this lane had to touch |

## 9. Lean

**Judged not applicable, and recorded** — the same judgment cards
T00-CT-SCHEMA, T08-SRC-CT, CT-CTWIN and the wave-11/13 CT lanes recorded for
their harness increments. This lane adds no checker rule, no diagnostic code,
no lowering and no model. It adds one fixture, pins, and one derived-mechanism
extension over an unedited gateway, whose 30 publication theorems
(`QCore.PublicationCustody`) landed with card `CTWIN-PUBLISH`. `proof` stays
`unassessed` for CT40; no theorem is bound to any CT fixture (T00.7 remains
open). The Lean gate was run and passes at **72 audits**.

## 10. Gate

Run in this worktree at the final commit.

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok (`third_party/libevent` at `d82464a2`) — see §10.1 |
| `cargo test --workspace --no-fail-fast` | exit 0: **2551 passed, 0 failed, 8 ignored** over 142 test targets |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean — see §10.1 item 3 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **72** Lean audits, all pass |
| `python3 -m unittest discover -s scripts/tests` | **168** tests, OK (161 before; +7 for the retirement mechanism) |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed |

### 10.1 An environment note, recorded rather than hidden

Three environment failures were hit and are recorded rather than absorbed
silently, because each looks like a test failure in a log.

1. **Disk.** The first full-workspace run failed to **link**, not to check: the
   host filesystem reached 100% (`No space left on device`, then `ld terminated
   with signal 7`) while several agents' worktrees shared one disk. `cargo
   clean` plus `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0` completed it.
2. **Submodule.** `libevent_submodule` and two `profile_loader` tests failed on
   an uninitialised `third_party/libevent`; `git submodule update --init
   --recursive` (the standing gate's own once-per-checkout step) fixed it, and
   both targets then pass — 1 and 18 tests. This worktree is a fresh checkout,
   so it needed the step; nothing about it is specific to this change.

3. **Clippy's ordering.** The clean `cargo clippy --all-targets -- -D warnings`
   recorded above ran **after every Rust edit and after `cargo fmt`**, which is
   the state committed: no `.rs` file changed afterwards (the later edits are
   JSON, Python and Markdown, and the mutation driver restored every file it
   touched). A confirming re-run at the very end could not start — `couldn't
   create a temp dir: No space left on device`, the same shared-disk exhaustion
   as item 1, with 273 MB free at that moment. The recorded result stands on the
   identical Rust tree; that it could not be repeated on demand is stated rather
   than glossed.

All three are environment failures, not source defects — precisely the
distinction the `environment-unavailable` reason category exists to keep
separate — and no assertion was weakened to get past any of them.

## 11. What this does not establish

* **Mapping is not conformance.** One more case executes its own fixture
  through real machinery. Sanitized-native evidence is unchanged at 4 and proof
  at 0; this route contributes neither and, having no emission, structurally
  cannot.
* **A different artifact and a different property.** Thirty of the 33 mapped
  cases are QCore source through `compile_files`. CT40, CT43 and CT45 are C
  twins through the unrestricted-C gateway, whose property is a resource
  projection over *declared* contracts. The `q_*` contracts in this fixture are
  trusted synthetic test assumptions, not facts about any library.
* **The twin is synthetic.** It is shaped after the scenario, not extracted
  from any upstream project, and no verdict here says anything about Fluent
  Bit, ChunkIO or any real code.
* **Publication is refused, not understood.** Nothing here decides what reads
  `g_published`, whether the pointer dangles, or what an atomic access would
  order. `Unknown` is a decline, not a proof of absence.
* **Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; the contract corpus stays 67
  modules; `qcore-schemas` stays 12.0.0; `qcore-compiler` stays 0.4.0; full
  language conformance stays false.

## 12. Named follow-ons

| Id | What |
|---|---|
| `CT-CTWIN-DESCRIPTOR` | unchanged and re-verified: an int-typed resource identity is not expressible, so CT42's *preserved identity* has nothing to be established by. |
| `CT-CTWIN-SYNC` | **decided, not open** (card CTWIN-PUBLISH §5), re-measured here over a third shape. CT41 stays unmapped because a vacuous mapping is worse than none. |
| `CT-CTWIN-ENTRY` | unchanged: the declared entry selects the summary's label, not what a C twin measures. CT40 inherits that boundary — its verdict is entry-independent for the same reason CT43's is. |
| `CT40-CANDIDATE` | **closed** by this lane. |
| `CT-LANE-RETIREMENT` | new, and satisfied for the one case it was built for. The mechanism is general; no other lane row currently needs retiring. |
| `D02-GEN-REST` | untouched: the acceptance matrix's non-CT sections stay the integrator's. |

## 13. Integrator carry list

Files this lane owns and changed: `docs/reevaluation/ct_manifest.json`,
`compiler/tests/ct_corpus.rs`, the new
`compiler/tests/fixtures/ct/CT40/` directory, `scripts/ct_stage_summary.py`,
`scripts/tests/test_ct_stage_summary.py`, `docs/reviews/r0_3/acceptance.json`,
`docs/reports/remaining-d02/build_matrix.py` (CT notes only), and this report
plus its `ct_stage_evidence.json` and `artifacts/`.

**Not** touched, deliberately: `docs/reports/wave9-ctwin/ct_stage_evidence.json`
and `docs/reports/remaining-t00/coverage.json` (both frozen; CT40's row is
retired by declaration, §5), `docs/reviews/r0_3/acceptance_matrix.json`,
`AGENTS.md`, `CLAUDE.md`, `DEVELOPMENT.md`, and every non-CT matrix section.

The integrator must:

1. **Regenerate `docs/reviews/r0_3/acceptance_matrix.json`'s `ct_conformance`
   section** from `docs/reports/remaining-d02/build_matrix.py`, as waves 9, 11
   and 13 did. The regenerated section reads mapped **33** / unmapped **17**,
   `depth_profile.legacy_c_twin = 3`, a mapped CT40 row on the `legacy_c`
   route, and CT41/CT42 rows whose `blocker_current` comes from this lane.
2. **Update `named_followups`**: `CT40-CANDIDATE` is **closed**;
   `CT-CTWIN-SYNC` stays *decided*; `CT-CTWIN-DESCRIPTOR` and
   `CT-CTWIN-ENTRY` are unchanged and re-verified.
3. **Carry the status paragraphs**: CT coverage 32/50 → **33/50 mapped**,
   unmapped 18 → **17**, sanitized-native **4 of 33**, proof **0 of 33**,
   nothing promoted. Thirty of the 33 are QCore source and three are C twins on
   the gateway.
4. **Note the additive record extension**: a CT stage-evidence record may now
   carry `retired_unmapped_reasons`, checked total-or-reject by both
   derivations. `schema_version` stays 2 in the manifest and 1 in the evidence
   records; no existing row changed shape.

---

## Increment paragraph (draft, for `DEVELOPMENT.md`)

> Wave-17 CT lane (CT40-LANE): **CT40 maps on the `legacy_c` route, moving CT
> coverage 32/50 → 33/50**, after card CTWIN-PUBLISH closed the gateway hazard
> that had left it unmapped. Every parked candidate row was re-measured on the
> current tree's fixture bytes and two corrections found: the parked diagnostic
> string used `unknown_alias` where the harness renders `unknown-alias`, so
> adopting it verbatim would have failed on the first run, and one control was
> not enough. The mapping rests on the construct CT40's `expected` text names —
> the publication protocol, which the gateway names verbatim in its own
> diagnostic — and explicitly not on the two its title names: `_Atomic` is
> pinned **non-load-bearing** by a control that is byte-identical to the case,
> and "prematurely freed" is not decided either, pinned by a control that
> publishes with no release and is refused just the same. `native_sanitized` is
> the structural `not_applicable` its route prescribes; sanitized-native stays 4
> and proof 0, so the grown denominator is mapping, not conformance. CT41 and
> CT42 stay unmapped with their blockers re-measured rather than inherited —
> CT41's is now an asserted decision (C11 `volatile` is not a synchronization
> primitive, so a projection with no synchronization model must answer
> identically), CT42's an int-typed resource identity the contract loader still
> refuses. One mechanism change: mapping a case whose bucket an earlier lane
> recorded no longer requires editing that lane's archived record — the mapping
> lane declares a `retired_unmapped_reasons` row in its own, checked
> total-or-reject in both derivations (seven new Python tests), so the wave-9
> record stays byte-identical. 24 scratch mutations, all refused. Lean judged
> not applicable with reasoning recorded.
