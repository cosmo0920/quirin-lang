# Wave-29 CT lane — mapping CT22 (CT22-WIRE)

Wave 29, card `CT22-WIRE`, on the `CT-WIRE-LANE-2` mechanics precedent. One
case, one route, one decision: **CT22** — *"Quant opening discards
parent-local unused credit after joins"*, expected **Accept** — is **MAPPED**
on the `composed_epoch` route, converting wave-28 card `REGION-CREDIT-JOINS`'s
capability into a manifest row exactly as that card's §7 prescribed for a lane
taking it.

CT coverage moves **41/50 → 42/50 mapped**, unmapped **9 → 8**.
Harness-gated sanitized-native evidence stays **7, now of 42**. Proof stays
**0 of 42**. A grown denominator is mapping, not conformance (constraint 23).

Base: `main` at `5d8b2ff1` (the wave-28 close). Branch:
`worktree-agent-a27d5f67ea809cc2f` in an isolated worktree.

**Nothing is promoted, and nothing semantic changed.** `git diff` over
`crates/` and `compiler/src/` is empty for this card: the only Rust is harness
and fixture wiring — `compiler/tests/ct_corpus.rs` (the `epoch_credit_split`
verdict mode widened for an accepting case, its pinned tables grown
deliberately) plus the fixture move card `REGION-CREDIT-JOINS` prescribed and
the path repoint in `compiler/tests/region_credit_joins.rs` that follows it.
No checker rule, no diagnostic code (the Q-space icebox needed none — this
card allocates zero and widens zero conditions), no lowering, no model, no
reserved word, no crate-source change.

---

## 1. The decision

| Case | Expected | Capability card | Decision |
|---|---|---|---|
| **CT22** | *Accept* | `REGION-CREDIT-JOINS` (wave 28) | **MAPPED** — the `composed_epoch` route's first accepted case, under the `epoch_credit_split` verdict mode widened for the accepting shape (§3); the wave-28 lane's narrowed reason is retired in this lane's own record (§5) |

The wave-28 lane's own measured reason —
`blocked-on-joins-composition (credit abandonment measured, joins not on this
route)` — is retired by card `REGION-CREDIT-JOINS`: a `source.epoch.tasks.v1`
entry may now write, **after every spawn and join of its task scope**, one
`error region … catch (E) quant open R as pack { discharge pack; }` whose
sealed pack carries no duty and exactly the parent's unused credit, which the
discharge abandons — the kernel's own explicit `AbandonCredit`, decided by the
**unedited** `qcore_core::error_region` model. The credit and the joins are in
one program, so the row's obstacle is gone, and its follow-on
`REGION-CREDIT-WITH-JOINS` is discharged by that card.

**Every word of the scenario is a checked fact**, which is what the mapping
claims and all it claims:

* *quant opening* and *discards* — the model's own `AbandonCredit` on the
  discharge of the opened pack;
* *parent-local* — the duty-free pack (the composed body admits no
  acquisition; `REGION-JOINS-DUTIES` refuses one at `Q302` by name) plus the
  after-joins IR rule (every `TrySpawn`/`Join` span of the checked task IR
  precedes the region statement, so no child exists and no child holds any
  grant at the opening);
* *unused credit* — the published `abandoned_credit` of **3**, which the
  driver re-derives from an independent fold of the written statements before
  publication;
* *after joins* — the `Q600` refusal when false, **in both directions**, each
  a pinned control (§2).

## 2. The case and its controls

`compiler/tests/fixtures/ct/CT22/` — the capability card's own fixtures,
**moved** from `compiler/tests/fixtures/epoch_credit/CT22/` (the parked
location that card chose because the CT harness fails closed when an unmapped
case owns a directory under `fixtures/ct/`; mapping moves the directory — the
CT23/T08-EPOCH-SRC precedent), with
`compiler/tests/region_credit_joins.rs` repointed at the new path and all 9 of
that card's end-to-end tests green on the moved bytes. Two control fixtures
are **added** by this lane, materializing refusals the capability card had
measured only as in-memory variants; both cascades were measured on this tree
before pinning.

The case: the plan grants the worker's credit (`{memory 4096, effort 100}`),
the parent spawns and joins its one child, and only then does the parent's own
region credit 5, spend 2 and take its decided edge; the opening binds the
sealed pack and the discharge abandons exactly the parent-local remainder 3.
**Admitted**, with the published quantities compared wholesale (§3).

| Control | Role | Measured |
|---|---|---|
| `no_region.qcore` | attribution | **admitted**; the identical credit-recovery split with `region: None` — the abandoned figure is the region's own publication, and the region moves neither epoch certificate |
| `spent_out.qcore` | boundary | **admitted**; `abandoned_credit = 0` — the figure is a fold, not a flag |
| `both_models.qcore` | composition | **admitted**; `error(move);` restores the whole grant on the creator's derived error edges (CT23's law) **and** the region abandons 3 — two unedited models, two published quantities, one entry |
| `before_join.qcore` | the scenario's other half, false (join direction) | **refused**, exact cascade `["Q600"]`, the message naming the card rule |
| `spawn_after.qcore` (**new**) | the same rule, spawn direction | **refused**, exact cascade `["Q600"]` |
| `acquiring.qcore` (**new**) | the duty-free-pack premise, false | **refused**, exact cascade `["Q302"]`, the message naming `REGION-JOINS-DUTIES` |
| `no_discharge.qcore` | the model's walk-away refusal | **refused**, exact cascade `["Q305"]` — CT21's shape from this route |
| `no_open.qcore` | the unopened nonempty pack | **refused**, exact cascade `["Q306"]` |
| `wrong_region.qcore` | the capability's identity | **refused**, exact cascade `["Q303"]` |

The refusing controls are what make the acceptance a measurement: `Q305` and
`Q306` together mean neither way of walking away from the remainder is
writable, so the case's explicit `AbandonCredit` is the **only** expressible
way to discard it; the two `Q600` directions and the `Q302` acquisition are
the scenario's premises shown to be gates rather than conventions.

## 3. The verdict mode, widened for an accepting case

`epoch_credit_split` (card `CT-WIRE-LANE-2`) pinned a refusal's exact cascade
plus every admitting control's published credit split, "because an epoch that
merely checks has measured nothing". This card widens the same mode with the
same discipline read from the accepting side — an acceptance with nothing
measured would map nothing:

* the case's cascade is **empty** exactly when the fixture pins its own
  published figures — the credit-recovery split (`restored`/`spent`, which
  must conserve against the grant at shape time, before anything compiles)
  and the region's `abandoned` credit;
* the runner compares all of it **wholesale** against the driver's published
  objects: `granted`/`restored`/`spent` per axis, the conservation law
  `restored + spent = grant` and `error_arm_committed = spent` on the
  published quantities, and `abandoned_credit` against the pinned figure;
* the two premises that make the figure *parent-local* are **re-asserted on
  the published objects** rather than trusted: the one published
  `RegionOp::ErrorRegion`'s body carries no `Acquire`, and every task-ledger
  span ends before the region statement's span begins;
* an accepted **control** that writes a region pins its own abandoned figure
  (`spent_out` 0, `both_models` 3); one that writes none must publish **no**
  region evidence at all (`no_region`), so the case's figure stays
  attributable to the region rather than to the route. A refused control
  still publishes nothing.

CT23's fixture, runner path and manifest row are byte-unchanged; the widening
is additive and the refusing shape's gates are untouched.

### 3.1 The stage derivation, one deliberate change

`derive_stage_plan` gains one rule (stated in the harness beside the code):
an **admitted** `composed_epoch` module's `c_emission` and `native_execution`
read **`unassessed`** rather than `not_applicable` — the module really emits
the task route's own C (the runner asserts `c_source` is present), but no
harness runner executes that emission or runs a binary from it, so the stages
are open items, never claims and never structurally absent. A refused
composed-epoch fixture (CT23) keeps `not_applicable`: its refusal provably
never reaches the backend. `native_sanitized` stays `unassessed` beside them,
and CT22 enters no sanitizer gate.

CT22's executed stages are therefore
`parse, elaborate, epoch_lowering, task_lowering, task_ledger_check,
epoch_bound_check` — the task pair executed on an **accepted** fixture for the
same structural reason CT23's executed on a refused one: the composed route
lowers and checks both halves before anything is published. Depth:
`task_ledger_check`, beside CT23 (the two checks share a rank and the ledger
check names the depth).

## 4. Pinned tables grown deliberately (invariant 5)

Each of these is an intentional expectation, edited with the mapping and
stated in the commit message:

* `MAPPED_CASES` 41 → **42** entries (+ CT22);
* `EPOCH_CREDIT_SPLIT_CASES` = `["CT22", "CT23"]`;
* `EVIDENCE_FILES` 12 → **13** (this lane's record), mirrored in
  `scripts/ct_stage_summary.py::EVIDENCE_FILES` and in `acceptance.json`'s
  `derived_from` — the three-way tie card CT-LABELS built;
* the manifest's CT22 row (route, fixture, semantic profile stating the
  composed premises, stage records, `harness_verdict_mode`) and its note's
  wave-29 paragraph;
* `SANITIZED_GATED_CASES`, `SANITIZER_TRANSCRIPT_CASES`, the route table and
  the runner registry are **unchanged** — this lane adds no runner and no
  sanitizer claim.

## 5. Stage evidence, the retirement, and the derived numbers

Both derivations agree, independently
(`ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` and
`python3 scripts/ct_stage_summary.py . --check`, both green):

```
ct_mapped 42   ct_unmapped 8
depth_profile     elaborate 4, epoch_bound_check 4, legacy_c_twin 3,
                  mutex_invariant_check 8, native_execution 9,
                  native_sanitized 7, task_ledger_check 6, task_lowering 1
native_sanitized  executed 7, recorded_not_gated 5, not_applicable 7, unassessed 23
proof             unassessed 42
unmapped_label_sources   3 of 8 lane-measured (CT41/CT42 wave 17, CT46 wave 13);
                         5 still t00-archive
```

This lane's `ct_stage_evidence.json` carries CT22's `verified_stages`
(mirrored by `scripts/ct_stage_summary.py::check_provenance` against the
manifest), the measured splits and premises, and the **retirement** of the
wave-28 lane's CT22 row — declared in **this** record under
`retired_unmapped_reasons`, the wave-28 record's bytes untouched
(constraint 21; the wave-17 retirement mechanism, exactly as the wave-28 lane
retired CT07/CT08's wave-26 rows). This lane publishes no `unmapped_reasons`
row of its own: the one case it examined, it mapped.

The `newly_decidable_not_mapped_reasons` derived map is **untouched and
verified** (constraint 25): its keys are `["CT24", "CT47"]` before and after,
CT22 is not in card T00's newly-decidable list, and
`check_matrix_ct_reasons` is green against the committed matrix —
`docs/reviews/r0_3/acceptance_matrix.json` is not edited by this lane.
`docs/reviews/r0_3/acceptance.json` is regenerated through the recorded
mechanism (`ct_stage_summary.py --write`), with `derived_from` grown by this
record and the hand-kept `ct_evidence.note`'s stale current-state numbers
updated (7 of 42, 0 of 42, 3 of the 8 measured, the retirement list).

### 5.1 Two Python tests updated, no assertion weakened

Exactly the wave-28 §7.2 shape — tests that encoded tree facts this lane
moved, in `scripts/tests/test_ct_stage_summary.py`:

* `test_a_measured_label_wins_over_the_archive_and_says_so` — the
  lane-measured label set was pinned as {CT22 (wave 28), CT41, CT42 (wave 17),
  CT46 (wave 13)}; CT22 leaves it (mapped, row retired) and is asserted out of
  the label map beside CT07/CT08. The same test's companion,
  `test_a_retired_row_is_admitted_and_publishes_no_label`, gains the CT22
  retirement as a second worked example beside CT40's.
* `test_a_changed_lane_label_moves_the_tally_and_drifts_the_record` — the
  scratch tally sum moves 9 → 8 with `ct_unmapped`.

Suite total unchanged: **193 tests, OK**.

## 6. What this does not establish

* **Mapping is not conformance.** One more case executes its own fixture
  through real machinery. Full language conformance stays `false`.
* **The region's quantities are written statements.** The credit figures are
  folds of the fixture's own `credit`/`effort` statements at check time, the
  epoch's four `FlatEpochBoundV1` premises are carried unchanged, and nothing
  derives a credit from any execution.
* **No native, sanitizer or proof evidence is claimed.** The admitted module
  emits the task route's own C; the region erases from emission (measured
  byte-for-byte by `region_credit_joins.rs`, not re-measured here); no harness
  runner executes the emission, so the emission and native stages read
  `unassessed` — open items, not claims. Sanitized-native stays 7, proof 0.
* **The capability card's boundaries carry through.** `REGION-JOINS-DUTIES`
  (no acquisition in the composed body — also CT25's gap, so a lane should
  not map CT25 on this evidence), `REGION-JOINS-T07` (children live at the
  opening), `REGION-JOINS-EPOCH-AXIS` (the parent-local strict credit is not
  charged against the epoch allocation) — all stated in the semantic profile
  or its controls, none narrowed and none discharged here.
* **Nothing is promoted.** The Cycle 19 corpus is untouched: no gateway,
  contract, audit or provider file is in this card's diff, so the corpus rows
  are carried — raw units 2 Safe / 42 provisional Unsafe / 137 Unknown plus
  four data-only, leak pin **266**, contract corpus **73** modules,
  `qcore-schemas` 14.0.0.

## 7. Gate

Run in this worktree on `main` at `5d8b2ff1` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2`, before the workspace run |
| `cargo test --workspace --no-fail-fast` | **4,392 covered (8 ignored) across 216 result lines: 4,392 passed, 0 failed** in the single run — all five constraint-16 load rows green on the first attempt (§7.1) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 115 Lean audits** (unchanged — this lane adds no Lean), exit 0, read from the text line |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT, `mapped 42 / unmapped 8` |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed |
| `cargo test -p qcore-compiler --test region_credit_joins` | 9 passed on the moved fixtures |

### 7.1 The workspace row

The wave-28 close measured 4,392 (8 ignored) at its integration tip
(`docs/reports/wave28-close/gate.json`); this card's tree covers the same
**4,392 tests (8 ignored)** — the count is unchanged because this lane adds no
test *function*: the new case runs inside `ct_corpus.rs`'s existing five
tests, whose own assertions grew, and `region_credit_joins.rs` keeps its nine
on the moved bytes. All 4,392 passed in the single `--no-fail-fast` run
(`measure/workspace-tests.log`, worktree-private); none of constraint 16's
five load-sensitive rows tripped this time, so no isolation re-run was owed.
The `providers/` workspace is untouched by this card and its 122-test
checkpoint is carried, not rerun (constraint 18 owes nothing: no provider,
schema or template changed).

## 8. Named follow-ons

| Id | What |
|---|---|
| `REGION-JOINS-DUTIES` | *(unchanged, the capability card's)* — acquisitions in the composed region body; also CT25's gap. A lane should not map CT25 on this card's evidence. |
| `REGION-JOINS-T07` | *(unchanged)* — spawning inside a region (children live at the opening) is `CheckedRegionTaskCreditV1`'s discipline and still has no source route. |
| `REGION-JOINS-EPOCH-AXIS` | *(unchanged)* — the parent-local strict credit is not charged against the epoch allocation. |
| `CT-COMPOSED-NATIVE` | New, named by this lane's own `unassessed` rows: an admitted `composed_epoch` module emits the task route's own C that no harness runner executes. A runner that compiles and runs an admitted case's emission (on the `run_sequential_sanitized` shape) would move CT22's emission and native stages off `unassessed` — in the claiming direction only where a real run backs it. |
| `T00.7` | Unchanged and still open: no theorem is bound to any CT fixture, which is why `proof` stays 0 of 42. |

## 9. Integrator carry list

Files this lane owns and changed:

* `docs/reevaluation/ct_manifest.json` — CT22 mapped; the note's wave-29
  paragraph. No route-vocabulary change.
* `compiler/tests/ct_corpus.rs` — the widened `epoch_credit_split` mode (accept
  shape, runner arm, shape gates, the stage-derivation rule of §3.1), the
  grown pinned tables (§4), `EVIDENCE_FILES` 13.
* `compiler/tests/fixtures/ct/CT22/` — moved from
  `compiler/tests/fixtures/epoch_credit/CT22/`, plus its new `expected.json`
  and the two new refusing controls (`spawn_after.qcore`, `acquiring.qcore`).
* `compiler/tests/region_credit_joins.rs` — fixture path repointed.
* `scripts/ct_stage_summary.py` — this lane's record added to
  `EVIDENCE_FILES`.
* `scripts/tests/test_ct_stage_summary.py` — the tree-fact pins of §5.1.
* `docs/reviews/r0_3/acceptance.json` — regenerated (`--write`), plus
  `derived_from` grown by this record and the note's current-state numbers.
* `docs/reports/ct22-wire/` — this report and its stage-evidence record.
* `DEVELOPMENT.md` — the wave-29 increment paragraph.

**Not** changed, and left for the integrator: `AGENTS.md`'s headline numbers
(CT row `41/50 → 42/50` mapped, sanitized-native `7 of 41 → 7 of 42`, proof
`0 of 41 → 0 of 42`, gate numbers), `README.md`, and
`docs/reviews/r0_3/acceptance_matrix.json` (regenerate through its generator;
CT22 is not a key of the derived `newly_decidable_not_mapped_reasons` — its
key set is `["CT24", "CT47"]` before and after, verified — and
`scripts/ct_stage_summary.py . --check` is green against the committed
matrix).
