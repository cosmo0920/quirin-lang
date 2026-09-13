# Wave-31 CT lane — CT25 mapped, CT24 withdrawn with the measurement (CT-WIRE-LANE-3)

Wave 31, card `CT-WIRE-LANE-3`, on the `CT22-WIRE`/`CT07-WRITE-REMAP`
mechanics precedents and the `CT-WIRE-LANE` withdrawal-with-measurement
precedent. Two decisions, the two the waves 29–30 capability cards made
ready: **CT25** — *"Cleanup destroys owner after joins and loan expiry"*,
expected **Accept** — is **MAPPED** on the `composed_epoch` route, exactly as
`docs/reports/region-joins-loan/README.md` §7 prescribed for a lane taking
the row; **CT24** — *"Cleanup destroys an owner before child join"*, expected
**Reject** — was examined on both routes it could name and is **WITHDRAWN
(left unmapped)**, with the measurement published in this lane's own record
(§4), the wave-26 lane's own mechanism for exactly this case.

CT coverage moves **42/50 → 43/50 mapped**, unmapped **8 → 7**. Harness-gated
sanitized-native evidence stays **7, now of 43**. Proof stays **2 of 43**
(CT01, CT37). A grown denominator is mapping, not conformance (constraint 23).

Base: `main` at `469f6509` (the wave-30 close plus the census-hygiene and
`check_lean_audits` hardening commits). Branch:
`worktree-agent-aa7f0452464002289` in an isolated worktree.

**Nothing is promoted, and nothing semantic changed.** `git diff` over
`crates/` and `compiler/src/` is empty for this card: the only Rust is harness
and fixture wiring — `compiler/tests/ct_corpus.rs` (the `epoch_credit_split`
verdict mode widened for the **adopting** accepted shape, its pinned tables
grown deliberately) plus the fixture move card `REGION-JOINS-LOAN` prescribed
and the path repoint in `compiler/tests/region_joins_loan.rs` that follows
it. No checker rule, no diagnostic code (the Q-space icebox needed none —
this card allocates zero and widens zero conditions), no lowering, no model,
no reserved word, no crate-source change.

---

## 1. The decisions

| Case | Expected | Capability cards | Decision |
|---|---|---|---|
| **CT25** | *Accept* | `REGION-JOINS-DUTIES` (wave 29), `REGION-JOINS-LOAN` (wave 30) | **MAPPED** — the `composed_epoch` route's second accepted case and its first adopting one, under the `epoch_credit_split` verdict mode widened for the adopting shape (§3). No retirement is declared, because none is possible: no lane record ever held a CT25 row (§5) |
| **CT24** | *Reject* | `CT24-CLEANUP-PREEMPTED` (wave 30) posed the question | **WITHDRAWN** — examined on both routes, left unmapped with card T00's `DECIDABLE-NOW` bucket unchanged; the measurement, including the measured constraint-25 consequence of a future mapping, is published in this lane's record (§4) |

### 1.1 Why CT25 maps now — the honesty question, answered in the affirmative

The readiness statement (region-joins-loan §7) poses the question
deliberately: does the composed program give **every word** of the scenario a
checked referent, on the CT22 standard? Decided yes, word by word, each word
carried by a **named rule of an unedited model** and re-asserted by the
harness on the published objects rather than trusted:

* *cleanup destroys owner* — the model's own `Destroy` on the discharge of
  the opened pack, made a real emitted, custody-checked and
  reference-executed destruction by the erasure (the discharge's
  `drop owner;` is written into the region's blanks after every join, where
  the unedited task route checks it). Published as `released = ["owner"]`,
  read out of the model's own cleanup trace and cross-checked by the driver
  against the reader's independent duty fold before publication; the runner
  compares it wholesale and re-asserts the adopting-pack premise on the
  published body (exactly one `Acquire` per released owner — the adoption).
  This is an **executed destroy**, not a refusal about one: the capability
  card's native counting-allocator run frees the lent owner.
* *after joins* — wave 28's derived span rule, `Q600` in both directions when
  false, re-asserted on the published ledger (every task-ledger span ends
  before the region statement begins).
* *and loan expiry* — three facts of unedited models: the owner **is lent**
  (the derived capture list is `[["read"]]`, from the checked IR's
  `ScalarOperand::Loan`, reconciled against the declared `read;` clause and
  pinned against both the ledger and the declared rows); the share **returns
  at the join** (the task ledger's own reclamation); and the destruction is
  **ordered after the expiry** because the joins precede the region. Both
  false directions are pinned refusing controls (§2).

The wave-29/30 narrowed reasons — *"loan expiry has no referent on the
composed route (`T08-COMPOSE-LOAN`)"* and its duty-side sibling — were
retired by the capability cards themselves (`REGION-JOINS-LOAN` §7 says so of
the wave-29 text), and this lane's mapping is their conversion into a
manifest row.

## 2. The case and its controls

`compiler/tests/fixtures/ct/CT25/` — card `REGION-JOINS-LOAN`'s own fixtures,
**moved** from `compiler/tests/fixtures/epoch_credit/CT25_LOAN/` (the parked
location that card chose because the CT harness fails closed when an unmapped
case owns a directory under `fixtures/ct/`; mapping moves the directory — the
CT22/CT23 precedent), with `compiler/tests/region_joins_loan.rs` repointed at
the new path and all 11 of that card's end-to-end tests green on the moved
bytes (native counting-allocator run included). One control fixture is
**added** by this lane, materializing a refusal the capability card had
measured only as an in-memory mutation (the CT22-WIRE precedent); its cascade
was measured on this tree before pinning. The wave-29 duty-half directory
`compiler/tests/fixtures/epoch_credit/CT25/` stays where it is: its tests pin
its own byte offsets, and the mapped referent is the loan case, per the
readiness statement.

The case: the parent owns `owner`, lends the worker a counted read share of
it at the spawn (`read owner share FULL`, declared in the plan as the
place-free `read;` clause), joins — the loan expires there — and only then
does its error region adopt the owner (`move owner;`), credit 5, spend 2 and
take its decided edge; the discharge destroys the owner and abandons the
parent-local remainder 3. **Admitted**, with the published quantities, duty
list and capture lists compared wholesale (§3).

| Control | Role | Measured |
|---|---|---|
| `adopted_then_dropped.qcore` | attribution (the frame rule) | **admitted**; the adopted owner is released in the region body, so `released = []` and `abandoned = 3` — the case's destroyed owner is the adoption's own publication, not the route's |
| `live_share_drop.qcore` | the loan-expiry premise, false (written direction) | **refused**, exact cascade `["Q600"]` — the unedited task ledger's own `LoanOutstanding` on a drop while the share is out |
| `before_join.qcore` (**new**) | the ordering premise, false (region direction) | **refused**, exact cascade `["Q600"]` — the whole adopting region above the spawn: its erased `drop owner;` destroys the owner before the loan is even made, and the lend finds no owned buffer to lend |
| `adopt_unknown.qcore` | the adoption reader's shape gate | **refused**, exact cascade `["Q302"]`, the message naming the unbound name |
| `adopt_twice.qcore` | the pack's linearity | **refused**, exact cascade `["Q302"]` |
| `adopt_scalar.qcore` | custody not re-implemented (constraint 15m) | **refused**, exact cascade `["Q500"]` — the unedited task route's own executable-subset gate over the erased drop |
| `adopt_and_acquire.qcore` | the multi-owner boundary | **refused**, exact cascade `["Q600"]` — `REGION-JOINS-MULTI-OWNER`, unchanged, reached through the adoption |

## 3. The verdict mode, widened for the adopting shape

Card `CT22-WIRE` widened `epoch_credit_split` for an accepting case whose
pack is **duty-free**; CT25's pack is the opposite — the adoption is the
scenario. The same discipline, read from the adopting side (an acceptance
with nothing measured would map nothing):

* the accepted case additionally pins the region evidence's **released duty
  list** (`["owner"]`), compared wholesale against the driver's re-derived
  publication, and the **derived capture list** (`[["read"]]`), compared
  against both the checked ledger and the declared rows;
* the duty premise **splits in two**: a case pinning no released owner keeps
  CT22's duty-free assertion (no `Acquire` in the published body,
  byte-for-byte the old premise); a case pinning released owners must publish
  **exactly one body `Acquire` per released owner** — the adoptions, handed
  to the unedited model as the pack acquisitions they are;
* an accepted **control** that publishes a region pins its released list too
  (`adopted_then_dropped` pins `[]`), so the case's destroyed owner stays
  attributable to the adoption; a refused control still publishes nothing
  (the shape gates refuse a `released` or `captures` pin on any refusal);
* everything CT22 pinned is still pinned: the credit split wholesale, the
  conservation law and `error_arm_committed = spent` on the published
  quantities, the abandoned figure, the ledger-span-before-region premise,
  and the region-evidence attribution rule for controls.

CT22's and CT23's fixtures, pins and manifest rows are byte-unchanged; the
widening is additive, and both old shapes' gates are untouched (verified by
the unchanged `ct_corpus` suite passing over all three cases).

### 3.1 Stage derivation

Unchanged — `CT22-WIRE`'s rule applies as written: an admitted
`composed_epoch` module's `c_emission` and `native_execution` read
**`unassessed`** (the module really emits the task route's own C, the runner
asserts it, and no harness runner executes that emission), `native_sanitized`
beside them. CT25's executed stages are `parse, elaborate, epoch_lowering,
task_lowering, task_ledger_check, epoch_bound_check`; depth
`task_ledger_check`, beside CT22 and CT23. The follow-on those `unassessed`
rows name is `CT-COMPOSED-NATIVE`, already open since wave 29 — this lane
adds no runner and no sanitizer claim, and the capability card's native
counting-allocator run carries no sanitizer, so none is readable from it
(constraint 23, per the readiness statement's own warning).

## 4. CT24 — withdrawn, with the measurement

The wave-30 card `CT24-CLEANUP-PREEMPTED` §6 left one question to this lane:
does a refusal at the exit that *would* run cleanup satisfy *"cleanup
destroys an owner before child join"*, given that the checked task route
executes no destroy at scope exit? Measured on this tree (probe transcripts
in `measure/`, worktree-private), on **both** routes the scenario could name:

| Shape | Measured |
|---|---|
| task route: `?` inside `thread scope`, child unjoined holding `read buffer share 512` | `Q600`: `ScopeBusy(5): cleanup preempted — this exit would run the frame's cleanup over resource 1 while task(s) [2] still hold a read share of it, destroying the owner before the child's join; the scope refuses the exit with that loan outstanding` |
| composed route: adopting region **between** spawn and join | `Q600`: `LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2] }` at the region statement — the erased cleanup release while the share is out |
| composed route: adopting region **before** the spawn | `Q600`: `slot 5 is not an owned buffer this task may lend` — the erased drop leaves nothing to lend |
| composed route: the same placements with the adoption removed | still refused — `Q600` at the wave-28 span rule or as a discard, depending on where the owner's release then falls |

The decision, and its two grounds:

1. **Attribution.** On the composed route — the only route with an *executed*
   cleanup-destroy — a region before the join is refused for its
   **placement** whatever its duties: removing exactly the construct CT24
   names (the destroy) does not move the verdict, so no control can isolate
   it, and the wave-9 attribution rule is not met. On the task route the
   refined message *names* the construct, but the route's checked semantics
   never executes a destroy at scope exit — the wave-30 card's own published
   boundary — so the "cleanup" in that refusal is a counterfactual of the
   message, not a checked fact of the tree.
2. **Coherence with CT25.** CT25's cell reads "cleanup destroys owner" off
   the composed region discharge — an executed, natively-run destruction.
   Mapping CT24 on the task-route exit shape would give the same word a
   different referent in the adjacent cell of the same scenario pair, which
   is a mapping claim more than the tree shows.

Both readiness statements said this outcome is valid; taking it keeps the
classification unchanged — CT24 keeps card T00's
`DECIDABLE-NOW (with a distinguishing fixture)` bucket, and re-verification
reaching the same bucket is the record saying the label holds. The residual
is the wave-30 card's own: a cleanup/task composition in which an executed
scope-exit destroy meets a live loan (`CT24-CLEANUP-TASK-COMPOSITION`), or a
deliberate decision that the preemption is the answer — not taken here.

### 4.1 The constraint-25 trap, measured rather than assumed

`measure/ct24_constraint25_probe.py` staged `mapping_status: "mapped"` for
CT24 on a scratch copy and ran the standing checks: the derived
`newly_decidable_not_mapped_reasons` key set moves `["CT24", "CT47"]` →
`["CT47"]`, `check_matrix_ct_reasons` goes red against the committed matrix
(*"explains why CT24 is not mapped, but the manifest maps it"*), and both
hand-kept `acceptance.json` halves flag. So a future lane that maps CT24 must
regenerate `acceptance_matrix.json` through the standing generator **and**
edit both hand-kept halves in the same change — published here and in the
record so that lane inherits a measurement, not a surprise. This lane's
withdrawal leaves the matrix untouched and every check green; no labelled
`unmapped_reasons` row is published for CT24 (that text would move the
derived map — the exact wave-26 trap), the measurement living in this
record's `cases_examined_and_left_unmapped` and in `acceptance.json`'s
hand-kept CT24 reason instead.

## 5. Pinned tables, stage evidence and the derived numbers

Grown deliberately (invariant 5), each stated in the commit message:

* `MAPPED_CASES` 42 → **43** entries (+ CT25);
* `EPOCH_CREDIT_SPLIT_CASES` = `["CT22", "CT23", "CT25"]`;
* `EVIDENCE_FILES` 16 → **17** (this lane's record), mirrored in
  `scripts/ct_stage_summary.py::EVIDENCE_FILES` and in `acceptance.json`'s
  `derived_from` — the three-way tie card CT-LABELS built;
* the manifest's CT25 row (route, fixture, the semantic profile stating which
  rule carries each word and the boundaries verbatim, stage records,
  `harness_verdict_mode`) and the note's wave-31 paragraph;
* `SANITIZED_GATED_CASES`, `PROOF_GATED_CASES`, `SANITIZER_TRANSCRIPT_CASES`,
  the route table and the runner registry are **unchanged** — this lane adds
  no runner, no sanitizer claim and no proof claim.

**No retirement is declared, and that is the mechanism working, not an
omission:** CT25's label was archive-held (`t00-archive`,
*"joins now exist; loan expiry does not"*) — no lane record ever published an
`unmapped_reasons` row for it, the wave-29/30 **narrowed** reasons living in
those capability reports' prose rather than in any stage-evidence record — so
there is no row for `retired_unmapped_reasons` to name, and both derivations
refuse a retirement that names a record holding no row (stale) or the archive
(vacuous). Mapping removes the case from the unmapped set and the archive
label stops resolving, exactly as CT21's and CT06's archive-held rows left in
earlier waves. The two Python pins that encoded the moved tree facts are
updated on the `CT22-WIRE` §5.1 precedent, no assertion weakened: the scratch
tally sum 8 → 7, and the label-resolution test's prose plus an explicit
`assertNotIn("CT25", labels)` beside CT21's.

Both derivations agree, independently
(`ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` and
`python3 scripts/ct_stage_summary.py . --check`, both green):

```
ct_mapped 43   ct_unmapped 7
depth_profile     elaborate 4, epoch_bound_check 4, legacy_c_twin 3,
                  mutex_invariant_check 8, native_execution 7,
                  native_sanitized 7, proof 2, task_ledger_check 7,
                  task_lowering 1
native_sanitized  executed 7 · proof executed 2 (CT01, CT37)
unmapped_label_sources   3 of 7 lane-measured (CT41/CT42 wave 17, CT46 wave 13);
                         4 still t00-archive (CT10, CT24, CT44, CT47)
```

`acceptance.json` is regenerated through the recorded mechanism
(`ct_stage_summary.py --write`), with `derived_from` grown by this record and
the hand-kept `ct_evidence.note`'s current-state numbers refreshed — 7 of 43
gated, proof **2 of 43** (that sentence still read "0 of 42" from before the
proof-leg cards, a staleness refreshed here and said so in the note itself),
3 of the 7 measured — plus the wave-31 denominator paragraph and the CT24
reason row's re-measurement. `docs/reviews/r0_3/acceptance_matrix.json` is
**not edited**: the derived reasons map's key set is `["CT24", "CT47"]`
before and after (CT25 is not in card T00's newly-decidable list, CT24 stays
unmapped), verified by `check_matrix_ct_reasons` green against the committed
matrix.

## 6. What this does not establish

* **Mapping is not conformance.** One more case executes its own fixture
  through real machinery. Full language conformance stays `false`.
* **The region's quantities are written statements.** The credit figures are
  folds of the fixture's own statements at check time; nothing derives a
  credit from any execution.
* **No native, sanitizer or proof evidence is claimed for CT25.** The
  admitted module emits the task route's own C; no harness runner executes
  it; the emission and native stages read `unassessed` — open items, not
  claims. Sanitized-native stays 7 of 43, proof 2 of 43.
* **The capability cards' boundaries carry through.** No write loan and no
  non-creator lender exist; the epoch bound is unmoved by the loan and the
  adoption (`REGION-JOINS-EPOCH-AXIS`); a region entered with live children
  stays refused (`REGION-JOINS-T07`); the erasure-fit and multi-owner
  boundaries stand (`REGION-JOINS-ERASURE-FIT`, `REGION-JOINS-MULTI-OWNER`).
  None narrowed, none discharged here.
* **CT24's classification did not move.** Withdrawal is not a new reason —
  the bucket holds, the measurement is published, and the derived matrix map
  is untouched.
* **Nothing is promoted.** The Cycle 19 corpus is untouched: no gateway,
  contract, audit or provider file is in this card's diff, so the corpus rows
  are carried — raw units 2 Safe / 42 provisional Unsafe / 137 Unknown plus
  four data-only, leak pin **266**, contract corpus **73** modules,
  `qcore-schemas` 14.0.0.

## 7. Gate

Run in this worktree on `main` at `469f6509` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2`, before the workspace run |
| `cargo test --workspace --no-fail-fast` | **4,466 passed / 0 failed / 8 ignored** across 223 result lines, exit 0, in the single run (§7.1) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | `Passed 121 Lean audits` (unchanged — this lane adds no Lean), exit 0, read from the text line |
| `python3 -m unittest discover -s scripts/tests` | **200 tests, OK** — 193 at the wave-30 close plus the 7 the post-close `check_lean_audits` hardening commit (`46f06722`) added to `test_check_lean_audits.py`; this lane adds none |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT, `mapped 43 / unmapped 7` |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed |
| `cargo test -p qcore-compiler --test region_joins_loan` | 11 passed on the moved fixtures, native run included |

### 7.1 The workspace row

The wave-30 close measured 4,466 (8 ignored) at its integration tip
(`docs/reports/wave30-close/gate.json`); this card's tree covers the same
**4,466 tests (8 ignored)** — the count is unchanged because this lane adds no
test *function*: the new case runs inside `ct_corpus.rs`'s existing five
tests, whose own assertions grew, and `region_joins_loan.rs` keeps its eleven
on the moved bytes. All 4,466 passed in the single `--no-fail-fast` run
(`measure/workspace-tests.log`, worktree-private); none of constraint 16's
five load-sensitive rows tripped this time, so no isolation re-run was owed.
The `providers/` workspace is untouched by this card and its 122-test
checkpoint is carried, not rerun (constraint 18 owes nothing: no provider,
schema or template changed).

## 8. Named follow-ons

| Id | What |
|---|---|
| `CT-COMPOSED-NATIVE` | Unchanged, now covering CT22 **and** CT25: an admitted `composed_epoch` module emits the task route's own C that no harness runner executes. A runner that compiles and runs an admitted case's emission would move both cases' emission and native stages off `unassessed` — in the claiming direction only where a real run backs it. |
| `CT24-CLEANUP-TASK-COMPOSITION` | The wave-30 card's naming, now this lane's measured residual for CT24: an executed scope-exit destroy meeting a live loan needs a cleanup/task composition that does not exist, and mapping without it fails attribution on both routes (§4). A lane taking it inherits §4.1's measured constraint-25 consequence. |
| `REGION-JOINS-T07`, `REGION-JOINS-EPOCH-AXIS`, `REGION-JOINS-ERASURE-FIT`, `REGION-JOINS-MULTI-OWNER`, `REGION-JOINS-LOAN-ADOPT-ONLY` | Unchanged, the capability cards'. |
| `T00.7` | Unchanged and still open for CT25: no theorem is bound to this CT fixture. The capability card's `QCore.RegionJoinsLoan` proves the model's round trip, not this fixture — proof stays 2 of 43. |

## 9. Integrator carry list

Files this lane owns and changed:

* `docs/reevaluation/ct_manifest.json` — CT25 mapped; the note's wave-31
  paragraph. No route-vocabulary change.
* `compiler/tests/ct_corpus.rs` — the widened `epoch_credit_split` mode
  (released duty list, capture list, the two-arm duty premise, shape gates),
  the grown pinned tables (§5), `EVIDENCE_FILES` 17, and the
  `ct_mapping_progress` doc comment's stale "proof 0 of 42" refreshed (the
  staleness `CT07-WRITE-REMAP` §6 observed in passing).
* `compiler/tests/fixtures/ct/CT25/` — moved from
  `compiler/tests/fixtures/epoch_credit/CT25_LOAN/`, plus its new
  `expected.json` and the new refusing control (`before_join.qcore`); the
  case's comment header notes the move.
* `compiler/tests/region_joins_loan.rs` — fixture path repointed.
* `scripts/ct_stage_summary.py` — this lane's record added to
  `EVIDENCE_FILES`.
* `scripts/tests/test_ct_stage_summary.py` — the tree-fact pins of §5.
* `docs/reviews/r0_3/acceptance.json` — regenerated (`--write`), plus
  `derived_from` grown, the note's current-state numbers and wave-31
  paragraph, and the hand-kept CT24 reason's re-measurement.
* `docs/reports/ct-wire-lane-3/` — this report and its stage-evidence record.
* `DEVELOPMENT.md` — the wave-31 increment paragraph.

**Not** changed, and left for the integrator: `AGENTS.md`'s headline numbers
(CT row `42/50 → 43/50` mapped, sanitized-native `7 of 42 → 7 of 43`, proof
`2 of 42 → 2 of 43`, gate numbers), `README.md`, and
`docs/reviews/r0_3/acceptance_matrix.json` (regenerate through its generator;
the derived `newly_decidable_not_mapped_reasons` key set is unchanged at
`["CT24", "CT47"]`, verified, and `scripts/ct_stage_summary.py . --check` is
green against the committed matrix).
