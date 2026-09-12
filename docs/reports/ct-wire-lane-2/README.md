# Wave-28 CT lane — mapping the wave-27 capabilities (CT-WIRE-LANE-2)

Wave 28, card `CT-WIRE-LANE-2`, on the wave-26 `CT-WIRE-LANE` precedent. The
brief named five candidates landed by the wave-27 capability cards; this report
carries a row for each, and all five map.

CT coverage moves **36/50 → 41/50 mapped**, unmapped **14 → 9**.
Harness-gated sanitized-native evidence moves **5 of 36 → 7 of 41**. Proof
stays **0 of 41**.

Base: `main` at `e552e59c` (the wave-27 close). Branch:
`worktree-agent-a311932dc252f9eba` in an isolated worktree.

**Nothing is promoted, and nothing semantic changed.** `git diff` over
`crates/` and `compiler/src/` is empty for this card: the only Rust is harness
and fixture wiring in `compiler/tests/ct_corpus.rs` (two new verdict modes with
their runners and pins), plus the fixture move card `CT23-CREDIT-ERROR-PATH`
prescribed and the two-line path repoint in
`compiler/tests/epoch_credit_error_path.rs` that follows it. No checker rule,
no diagnostic code (the Q-space icebox needed none — this card allocates
zero), no lowering, no model. A grown denominator is mapping, not conformance.

---

## 1. The decisions, in one table

| Case | Expected | Capability card | Decision |
|---|---|---|---|
| **CT07** | *Accept with bounds/disjointness proof* | `CT07-SLICE-PERMISSIONS` | **MAPPED** — with the read-loan boundary stated in the semantic profile (§2.1) |
| **CT08** | *Reject or analyzer potential/unknown* | `CT07-SLICE-PERMISSIONS` | **MAPPED** — on the Reject arm; the `unknown` arm is not claimed (§2.2) |
| **CT09** | *Accept only after layout/access proof* | `AGGREGATES-V1` | **MAPPED** — access proof checked; the layout leg is an executed **observation**, not a proof (§2.3) |
| **CT21** | *Reject* | `QUANT-REGION-SOURCE` | **MAPPED** — the archived "no source path emits `RegionOp::QuantOpen`" reason is retired (§2.4) |
| **CT23** | *Reject* | `CT23-CREDIT-ERROR-PATH` | **MAPPED** — on the new `composed_epoch` route with the proposed `epoch_credit_split` verdict mode (§2.5) |
| **CT22** | *Accept* | `QUANT-REGION-SOURCE` §5 | **NOT MAPPED** — the narrowed reason ("after joins") is published in this lane's own record (§3) |

No row was withdrawn: unlike wave 26's CT24/CT47, none of this lane's labelled
rows touches a derived, test-compared field of the acceptance matrix — the
`newly_decidable_not_mapped_reasons` key set is `["CT24", "CT47"]` and none of
this lane's cases is in it — so nothing had to choose between a label and a
matrix the lane may not regenerate. CT22's row is the lane's one labelled
`unmapped_reasons` entry, and the 193-test Python suite is green with it.

## 2. The five cases mapped

### 2.1 CT07 — "Two disjoint array halves mutated independently"

The wave-26 lane's own measured bucket was
`blocked-on-slice-permission-surface (bounds decided, disjointness not)`. Card
`CT07-SLICE-PERMISSIONS` retired that reason — `borrow_utf8(NAME, LOW, HIGH)`
lends a range, and a write is refused exactly when its byte lies inside a live
loan — and its §8 stated what a lane may claim. This mapping claims exactly
that.

`compiler/tests/fixtures/ct/CT07/case.qcore` is that card's published
disjoint-halves program: two loans of `[0, 2)` and `[4, 6)` of one six-byte
buffer live at once, the owner writing bytes 2 and 3 between them, and a third
view reading the written bytes back so the admission is visible in the
observation (`ab`, `ef`, `XY`) rather than only in the certificate.

**The boundary, stated as narrowly as it is true** (in the manifest's semantic
profile, the fixture's notes and here): one of the two disjoint permissions is
a **read** loan and the other is the owner's **retained write authority**.
That is disjoint permission over two ranges of one array with the disjointness
*decided* (`max low < min high`, the set-theoretic question itself because
every bound is a check-time constant) — it is **not** two parties each holding
write authority over their own range. The write lend is unlanded residual
`SLICE-PERMISSIONS-WRITE-LEND`, and this mapping claims nothing about it.

Evidence: the new `sequential_sanitized` verdict mode (§4). The case's own
emitted C is built once under
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined
-fno-omit-frame-pointer` and run under
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` /
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, both streams compared byte
for byte with the checked observation. ASan is the load-bearing sanitizer: a
sub-extent loan emits a displaced pointer and a length, and an off-by-one in
either is a real out-of-bounds read the emitted `fwrite` performs.

| Control | Role | Measured |
|---|---|---|
| `overlapping.qcore` | disjointness | one byte of the write moved inside `[0, 2)`: **refused**, `["Q078"]`, the message naming the footprint. This program is CT08's own case shape |
| `out_of_extent.qcore` | bounds | one byte of the write moved past the extent: **refused**, `["Q077"]`, *"a refusal and not a run-time trap"* |

The two controls are the two halves of the case's expected text — the bounds
proof and the disjointness proof — each shown to be a gate rather than a
convention.

### 2.2 CT08 — "Overlapping array ranges"

The same rule's other direction, and the case is the program the capability
card published for exactly this purpose (`overlapping_write.qcore`): the write
`store[1] = 88;` enters as the one-byte range `[1, 2)`, `head`'s loan is
`[0, 2)`, the two ranges overlap on exactly one byte, and the program is
refused with the exact cascade `["Q078"]` and a message naming the footprint
that decided it.

The mapping takes the **Reject** arm of *"Reject or analyzer
potential/unknown"*. The other arm is deliberately not claimed: `Unknown` is
the gateway's verdict vocabulary and the source harness's is accept/reject —
the same note the wave-26 lane recorded, kept as a boundary of the mapping
rather than as a blocker.

| Control | Role | Measured |
|---|---|---|
| `disjoint.qcore` | boundary | the byte moved outside both ranges: **admitted** (`StandardStringV1`, emits), the owner's writes read back — so the refusal is keyed to the overlap, not to writing a borrowed buffer. This program is CT07's own case shape |
| `no_loan.qcore` | attribution | the identical write with no loan live: **admitted** — the overlap the case refuses is an overlap with a *live* loan's range |

CT07's control being CT08's case and vice versa is deliberate: the pair
measures one decision from its two sides, exactly as the capability card's own
two fixtures do.

### 2.3 CT09 — "Distinct ordinary struct fields"

Card T00's archived reason — *"no struct/field access in source"* — is retired
by card `AGGREGATES-V1`'s `source.record.v1`. The case is that card's
published three-field program: one value constructed with three different
numbers, all three fields read back and printed, then a second value of the
same type read in a different order (`65`, `66`, `40` on stdout; `[INFO] 9`,
`[INFO] 1` on stderr).

**The honesty boundary is `AGGREGATE-CT09-LAYOUT`, honored verbatim.** The
*access* proof is supplied and checked — construction totality, projection
soundness, distinctness, and the `bad_field.qcore` control refusing a read
outside the declaration with the exact cascade `["Q052", "Q052"]` — but the
*layout* leg is an executed sanitized byte-comparison **observation**: a model
that aliased two fields, or a lowering that gave two fields one C member,
would print one value twice and the per-stream comparison would fail. That a C
compiler's designated initializer and member reads agree with the model is
cross-checked, never proved; the semantic profile says so in as many words,
and the `proof` stage stays `unassessed`. CT09's layout **proof** remains open
(`CT09-LAYOUT-PROOF`).

The sanitized run is a *negative* allocation control (the program allocates
nothing, so LSan reporting a leak would mean the aggregate acquired something
the model says it cannot), where CT07's is a positive one.

### 2.4 CT21 — "Quant opening discards positive access permission"

The archived reason — *"no source path emits `RegionOp::QuantOpen`"*, still
re-verified true by inspection as recently as wave 26 — is retired by card
`QUANT-REGION-SOURCE`, which landed exactly that source path. It was
archive-held (card T00's `blocked-on-W3/W4` bucket), not lane-held, so mapping
the case removes it from the tally with no retirement declaration — the same
mechanics as CT06 in wave 26; the retirement of the *reason* is recorded in
this lane's record prose and here.

`compiler/tests/fixtures/ct/CT21/case.qcore`: the region acquires one resource
and raises, the error edge seals a cleanup pack holding one live duty —
positive access permission — and `quant open R as pack` binds it as a linear
capability the handler then never discharges. Refused by the unedited
`qcore_core::error_region` model, exact cascade `["Q305"]`:
*"cleanup pack is not discharged"*.

| Control | Role | Measured |
|---|---|---|
| `discharged.qcore` | attribution | the same region, the same opening, one `discharge pack;`: **admitted** (`SequentialCleanupV1`, emits) — the refusal is about discarding, not the region or the opening |
| `unopened.qcore` | boundary | the same nonempty pack caught without opening it: **refused**, `["Q306"]` — the neighbouring rule, so discard-after-opening and never-opening are two different refusals and neither way of walking away from live custody is writable |

`Q305` is below the 500 band, so the derivation credits parse and elaborate
and no deeper sequential stage — the standing `CT-SEQ-DEPTH-RULE`
conservatism, inherited unedited.

### 2.5 CT23 — "Parent abandons child-owned credit during its own error"

Followed `CT23-CT-MAPPING` (the capability card's §6) decision for decision:

1. **The route.** A **new** `composed_epoch` route
   (`parse, elaborate, epoch_lowering, task_lowering, task_ledger_check,
   epoch_bound_check, c_emission, native_execution, native_sanitized, proof`)
   rather than growing the `epoch` route. Growing `epoch` would have changed
   the check-only route's structural statements — CT35/CT36/CT38/CT39's
   `native_sanitized: not_applicable` is *"this route emits nothing"*, and a
   route that suddenly could emit would turn those into open items. The
   composed route genuinely emits (the task route's own C, byte-identical
   whatever the error arm says), so its vocabulary carries the emission and
   native stages even though no runner executes them today; it publishes **no
   reference observation** (measured), so it has no reference-execution stage.
2. **The verdict mode.** `epoch_credit_split`, implemented as proposed: the
   case pins the exact cascade **and** every admitting control pins its
   published split, compared wholesale against the driver's re-derived
   figures, because an epoch that merely checks has measured nothing. The
   runner additionally asserts the law on the published object — `restored +
   spent = grant` componentwise and `error_arm_committed = spent` — and the
   fixture-shape gate asserts the declared figures conserve before anything
   compiles.
3. **The fixtures moved** from `compiler/tests/fixtures/epoch_credit/CT23/`
   into `compiler/tests/fixtures/ct/CT23/`, with
   `compiler/tests/epoch_credit_error_path.rs` repointed at the new path — the
   T08-EPOCH-SRC precedent, exactly as the card's own comment prescribed. All
   9 of that card's end-to-end tests stay green on the moved bytes.
4. **`ct_corpus.rs`'s pinned tables grew deliberately** (invariant 5):
   `MAPPED_CASES` 36 → 41 entries, the route table gained `composed_epoch`,
   the runner registry gained the two new runners, and the sanitizer gate grew
   by exactly the two cases §4 justifies. Each is stated in the commit
   message.

The case: the plan's `error(close);` declares that the creator's own error
edge abandons the credit the epoch granted the creation — neither restored nor
spent. Refused, exact cascade `["Q635"]`, message
`CreditAbandonedAtErrorArm { task: 1, axis: "effort", abandoned: 100 }`. The
derived edges the case rests on are real (`{creation_failure: true,
join_failure: true}` — the join edge is the one on which the child existed and
held the grant), asserted by the capability card's own test on the same bytes.

| Control | Role | Measured |
|---|---|---|
| `restored.qcore` | disposition | **admitted**; `restored = {memory 4096, effort 100}`, `spent = 0` |
| `spent.qcore` | disposition | **admitted**; `restored = 0`, `spent = {memory 4096, effort 100}` |
| `no_arm.qcore` | attribution | **admitted**; the identical split `spent.qcore` publishes — removing exactly the clause removes the refusal, and eliding is spelling the conservative reading |
| `partial.qcore` | boundary | **admitted**; `restored = {1024, 25}`, `spent = {3072, 75}` — a real componentwise division |
| `minted.qcore` | the other refusal | **refused**, `["Q636"]` — restoring more than the grant would mint credit into the epoch |

**What the mapping does not claim:** every credit quantity is a **declared**
figure under the four unchanged `FlatEpochBoundV1` premises; the derived half
is the error-edge *shape*, the declared half is the disposition. One
disposition covers both error edges (`CT23-EDGE-SPLIT` open), restored credit
is not re-grantable (`CT23-RESTORE-REUSE` open), and no native or sanitizer
evidence is claimed — the case is a refusal and the epoch half emits nothing.

The stage derivation for this route is worth one sentence: `task_lowering` and
`task_ledger_check` read `executed` on a refused fixture because the
credit-recovery gate derives the creator's error edges from the **checked**
region, so a refusal in the flat epoch's own band (`Q630`–`Q641`) is only
reachable after both halves lowered and checked. `derive_stage_plan` encodes
exactly that rule and refuses drift.

## 3. CT22 — not mapped, with the narrowed reason published

Per the brief and `QUANT-REGION-SOURCE` §5: the **archived** reason (*"as
CT21; also needs credits"*) is retired — the opening has a source path and the
credit exists — and what remains is a narrower gap in the *scenario*, not the
recorded reason. This lane re-measured the credit half on this tree by running
`compiler/tests/quant_region.rs::ct22_a_discharge_abandons_the_regions_unused_credit`:
the flagship's pack carries remainder 3 and the discharge's cleanup trace
contains `AbandonCredit(3)` — discarded explicitly, as an event, which is the
case's property.

The published row (this lane's `ct_stage_evidence.json`, `unmapped_reasons`):

> bucket `blocked-on-joins-composition (credit abandonment measured, joins not
> on this route)` — the `after joins` half is not expressible where the credit
> lives: `source.region.v1` rides the flat sequential route, which has no
> tasks and no join. Follow-on `REGION-CREDIT-WITH-JOINS`.

CT22 was archive-held, so the lane label wins with no supersession. This was
**not** coordinated with the parallel card working the joins composition, per
the brief.

## 4. The two new verdict modes

### 4.1 `sequential_sanitized` (CT07, CT09)

The sequential route's first **accepted** cases needed what no existing mode
supplies: the plain `accept` mode pins a scalar outcome and trace this route
does not publish, and checks no stream and no emission. The new runner
(`run_sequential_sanitized`) executes the whole stack on the case's own
fixture:

* the route's per-stream observation (`out: `/`err: ` lines computed from the
  checked model, never from the emitted C) compared **exactly** against the
  pinned rendering, with a totality check that no observation line escapes the
  two streams;
* the controls, on every host (this route has no platform boundary);
* one binary built from the case's own emitted C with the pinned sanitizer
  flags, run once, both streams compared byte for byte.

The Windows arm asserts the emission exists and returns — **no sanitizer
claim**, exactly as the other sanitized runners. The macOS arm runs
`detect_leaks=0` under the standing platform boundary (Apple's ASan ships no
leak detector), so leak detection stays a Linux observation.

**The sanitizer gate grows 5 → 7 (`SANITIZED_GATED_CASES` += CT07, CT09), and
the growth is real:** both cases' `native_sanitized: executed` entries name
this runner, the case's own artifact, one observation and the
`["linux", "macos"]` host gate, and the harness rederives all of that from its
own dispatch. This is `executed` where true — not a numerator moved by
assertion. Wave 26's `CT-SEQ-SANITIZED` follow-on asked for a sanitized run on
this route's *controls*; this card instead put accepted **cases** on the
route, so the runs are the cases' own artifacts and the follow-on's
control-run idea is superseded for these two (it remains open for CT06/CT11,
whose fixtures are refusals).

### 4.2 `epoch_credit_split` (CT23)

Described in §2.5. Shape rules enforced by the harness: the cascade is pinned;
at least one admitting **and** at least one refusing control; an admitted
control's declared figures must conserve (`restored + spent = granted`,
checked at shape time) and its published figures must equal them (checked at
run time against the driver's re-derived split); a refused control publishes
no split.

## 5. Stage evidence and derived numbers

Both derivations agree, independently (`ct_corpus.rs::
ct_acceptance_summary_is_derived_from_the_manifest` and
`python3 scripts/ct_stage_summary.py . --check`, both green):

```
ct_mapped 41   ct_unmapped 9
depth_profile     elaborate 4, epoch_bound_check 4, legacy_c_twin 3,
                  mutex_invariant_check 8, native_execution 9,
                  native_sanitized 7, task_ledger_check 5, task_lowering 1
native_sanitized  executed 7, recorded_not_gated 5, not_applicable 7, unassessed 22
proof             unassessed 41
unmapped_label_sources   4 of 9 lane-measured (CT22 wave 28, CT41/CT42 wave 17,
                         CT46 wave 13); 5 still t00-archive
```

CT08 and CT21 land at depth `elaborate` under the same `CT-SEQ-DEPTH-RULE`
conservatism CT06 and CT11 recorded; CT23 lands at `task_ledger_check`; CT07
and CT09 at `native_sanitized`.

This lane's `ct_stage_evidence.json` carries: the five cases'
`verified_stages` (mirrored by `scripts/ct_stage_summary.py::check_provenance`
against the manifest), CT22's labelled row, the CT07/CT08 **retirements** of
the wave-26 lane's rows (declared in this record, the earlier record's bytes
untouched — constraint 21 and the wave-17 retirement mechanism), and the
honesty prose for each mapping.

## 6. What this does not establish

* **Mapping is not conformance.** Five more cases execute their own fixtures
  through real machinery. Full language conformance stays `false`.
* **CT07 measures disjoint permission, not two mutating parties.** One read
  loan plus the owner's retained write authority; the write lend is
  `SLICE-PERMISSIONS-WRITE-LEND`, open.
* **CT09's layout leg is observed, not proved.** `AGGREGATE-CT09-LAYOUT` /
  `CT09-LAYOUT-PROOF` stay open; `proof` is 0 of 41.
* **CT23's quantities are declared premises.** Nothing derives a credit from
  an execution, and the four `FlatEpochBoundV1` premises are carried unchanged
  on every certificate measured here.
* **Nothing is promoted.** The Cycle 19 corpus is untouched: raw units stay
  2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only, the leak
  pin stays **266**, the contract corpus stays **73** modules, `qcore-schemas`
  stays 14.0.0. No gateway, contract, audit or provider file is in this
  card's diff.
* **No new diagnostic code and no new reserved word.** The `Q-NAMESPACE-DECISION`
  icebox is untouched; every refusal pinned here lands on a code its owning
  pass already had, and no fixture spells anything the wave-27 cards did not
  already census.

## 7. Gate

Run in this worktree on `main` at `e552e59c` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2`, before the workspace run |
| `cargo test --workspace --no-fail-fast` | **4,376 covered, 8 ignored** across 216 result lines: 4,375 passed in the single run; the 1 that did not is a recorded constraint-16 load row and passes in isolation — §7.1 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 114 Lean audits** (unchanged — this lane adds no Lean), exit 0, read from the text line per the standing environment note |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed |
| `cargo test -p qcore-compiler --test epoch_credit_error_path` | 9 passed on the moved fixtures |
| `compiler/tests/{slice_permissions,quant_region,source_record}.rs` | 25 / 29 / 56 passed, unmoved and unedited |

### 7.1 The workspace row, and the one load row

The wave-27 close measured 4,376 (8 ignored) at its integration tip
(`docs/reports/wave27-close/gate.json`); this card's tree covers the same
**4,376 tests (8 ignored)** across 216 result lines — the count is unchanged
because this lane adds no test *function*: the five new cases run inside
`ct_corpus.rs`'s existing five tests, whose own assertions grew.

**4,375 passed in the single `--no-fail-fast` run and one did not, and both
facts are reported** (the run kept `--no-fail-fast` under `GATE-NO-FAIL-FAST`,
so the failure hid nothing after it):
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` failed its wall-clock
assertion (`elapsed < 500ms`, `cycle20_verifier_plugins.rs:794`) while several
workspace suites ran concurrently on this machine — exactly the shape standing
constraint 16 (`P02-FLAKE`) records for that row, measured failing at load 50
and passing at load 37 on the same binary. Verified in isolation per that
constraint: `cargo test -p qcore-compiler --test cycle20_verifier_plugins
vp04_vp07_vp08_process_lifecycle_and_payload_budgets -- --test-threads=1` →
**1 passed, 0 failed** (1.43s). No assertion was weakened, and nothing in this
card touches the provider host; the other four constraint-16 rows passed on
the first attempt. The `providers/` workspace is untouched by this card and
its 122-test checkpoint is carried, not rerun (constraint 18 owes nothing: no
provider, schema or template changed).

### 7.2 Two Python tests updated, no assertion weakened

Exactly the wave-26 §8.2 shape — tests that encoded tree facts this lane
moved:

* `test_a_measured_label_wins_over_the_archive_and_says_so` — the
  lane-measured label set was pinned as {CT07, CT08 (wave 26), CT41, CT42
  (wave 17), CT46 (wave 13)}; CT07/CT08 leave it (mapped, rows retired) and
  CT22 (wave 28) enters, with its archived `blocked-on-W3/W4` asserted
  retained beside the measured label.
* `test_a_changed_lane_label_moves_the_tally_and_drifts_the_record` — the
  scratch tally sum moves 14 → 9 with `ct_unmapped`.

## 8. Named follow-ons

| Id | What |
|---|---|
| `SLICE-PERMISSIONS-WRITE-LEND` | *(unchanged, the capability card's)* — the half of CT07's scenario this mapping's semantic profile excludes: two parties each holding write authority over their own range. A future card landing it can strengthen CT07's profile; the mapping itself stays honest without it. |
| `CT09-LAYOUT-PROOF` | *(unchanged)* — CT09's layout **proof**. The mapped evidence is the executed observation; a proof would move `proof` off 0. |
| `REGION-CREDIT-WITH-JOINS` | CT22's remaining gap, now the whole of it: the region's credit and the task route's joins in one program. Worked by a parallel card; this lane published the narrowed reason and did not coordinate. |
| `CT23-EDGE-SPLIT`, `CT23-RESTORE-REUSE`, `CT23-T07-SEAM` | *(unchanged, the capability card's)* — carried as boundaries in CT23's semantic profile. |
| `CT-SEQ-SANITIZED` | Narrowed: CT07/CT09 now carry case-artifact sanitized runs, so the control-run idea remains relevant only for the refusal cases CT06/CT11. |
| `CT-SEQ-DEPTH-RULE` | *(unchanged)* — CT08 and CT21 sit at depth `elaborate` under the same sub-500 conservatism as CT06/CT11. |
| `T00.7` | Unchanged and still open: no theorem is bound to any CT fixture, which is why `proof` stays 0 of 41. |

## 9. Integrator carry list

Files this lane owns and changed:

* `docs/reevaluation/ct_manifest.json` — five cases mapped; the
  `composed_epoch` route added to the vocabulary; the note's wave-28 paragraph.
* `compiler/tests/ct_corpus.rs` — two new verdict modes
  (`sequential_sanitized`, `epoch_credit_split`), their runners, the grown
  pinned tables (`MAPPED_CASES` 41, `SEQUENTIAL_REJECT_CASES` + CT08/CT21,
  `SANITIZED_GATED_CASES` + CT07/CT09, `ROUTE_STAGES` + `composed_epoch`,
  `RUNNER_REGISTRY` + `run_sequential_sanitized`, `EVIDENCE_FILES` + this
  record).
* `compiler/tests/fixtures/ct/{CT07,CT08,CT09,CT21}/` — new;
  `compiler/tests/fixtures/ct/CT23/` — moved from
  `compiler/tests/fixtures/epoch_credit/CT23/` plus its new `expected.json`.
* `compiler/tests/epoch_credit_error_path.rs` — fixture path repointed.
* `scripts/ct_stage_summary.py` — this lane's record added to
  `EVIDENCE_FILES`.
* `scripts/tests/test_ct_stage_summary.py` — the two tree-fact pins of §7.2.
* `docs/reviews/r0_3/acceptance.json` — regenerated (`--write`), plus
  `derived_from` grown by this record.
* `docs/reports/ct-wire-lane-2/` — this report and its stage-evidence record.
* `DEVELOPMENT.md` — the wave-28 increment paragraph (per the card brief; the
  wave-26 lane left this to the integrator, this one was asked to write it).

**Not** changed, and left for the integrator: `AGENTS.md`'s headline numbers
(CT row `36/50 → 41/50` mapped, sanitized-native `5 of 36 → 7 of 41`, gate
numbers), `README.md`, and `docs/reviews/r0_3/acceptance_matrix.json`
(regenerate through its generator; none of this lane's cases is a key of the
derived `newly_decidable_not_mapped_reasons`, so that map does not move, and
`scripts/ct_stage_summary.py . --check` is green against the committed
matrix).
