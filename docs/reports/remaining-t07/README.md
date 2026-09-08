# T07 — `CheckedRegionTaskCreditV1`: error-region credit and lifetime obligations

Date: 2026-09-08. Base checkpoint: `c60fd91` (the second delegated-increment
tip). Branch: `worktree-agent-ad95bcee12be337af`.

**Scope.** One flat thread scope in which an error handler joins its children,
lets their read loans expire, collects the credit they were lent, and only then
opens and discharges its own region's cleanup pack. This card is *model level*:
it composes the Cycle 6 error-region credit discipline
(`crates/qcore-core/src/error_region.rs`) with the T04 scoped-task and
read-loan ledger (`crates/qcore-core/src/scoped_tasks.rs`) in one new checked
profile. It adds no source syntax, emits no C, imports no source, promotes no
raw Cycle 19 unit, and maps no CT case — `docs/reevaluation/ct_manifest.json`
is untouched at 15/50 and §9 explains why nothing is proposed for it.

---

## 1. What landed

| Artifact | Change | Role |
|---|---|---|
| `crates/qcore-core/src/region_task_credit.rs` | +1304 (new) | the `CheckedRegionTaskCreditV1` checker, its certificate and its independent recheck |
| `crates/qcore-core/tests/unit/region_task_credit.rs` | +1427 (new) | 27 tests: the deliverable, per-gate negatives, a 13-row mutation matrix, certificate forgery |
| `crates/qcore-core/src/lib.rs` | +2 | the module declaration |
| `proof/QCore/RegionTaskCredit.lean` | +679 (new) | 23 public theorems, 2 private lemmas, 5 of the 23 decidable scenarios |
| `proof/AuditRegionTaskCredit.lean` | +25 (new) | axiom registration for all 23 |
| `proof/QCore.lean` | +1 | the import |

The whole diff against `c60fd91` is **six files, +3438, −0**. Nothing was
edited, deleted or re-pinned. In particular `error_region.rs`, `scoped_tasks.rs`,
`task_execution.rs`, the CT fixtures, `ct_manifest.json`, both `r0_3` acceptance
records, `AGENTS.md` and `CLAUDE.md` are byte-identical to the base.

### 1.1 Why a new module beside two unedited ones

Acceptance 3 requires the existing handler tests to keep passing. The two models
this card composes each carry pinned expectations — the Cycle 6 region analyzer's
`Q302`–`Q309` refusals and their unit tests, and the T04 ledger's nine tests plus
the CT20 source fixture. Editing either to grow a credit/duty integration would
have put those pins at risk for no gain. Instead:

* `ScopedTaskLedger` is used **verbatim as the custody authority**. Every task,
  loan, join, completion and scope rule this profile enforces is the one T04
  already implements; the new module never re-decides one. That is what makes
  this an integration rather than a second, drifting task checker.
* The region machinery is re-implemented in the new module, because
  `CleanupPack`'s fields are private and it has no public constructor. The
  refusals carry over condition for condition, with the same codes (§5).

---

## 2. The admitted profile, exactly

One root task, one thread scope, one straight-line path. Regions may nest; the
outermost is implicit and named by the program, and it is where credit enters
and where the root task's own duties live.

```text
program  = scope, root_region, initial_credit, declared_errors, body
op       = Acquire r | Drop r | MoveToPayload r | DropPayload r
         | Spawn { reservation, child, handle, moves, loans, child_credit, outcome }
         | Complete { child, outcome, resources, unspent_credit }
         | Join { handle } | FailJoin { handle }
         | Spend n | Raise e
         | Region { region, credit, body, catches } | Discharge binding
catch    = error, quant_open?, handler
```

### 2.1 The two things the profile keeps apart

| | Credit | Duty |
|---|---|---|
| what it is | a conserved `u64` | a member of `{Release r, Join h, LoanExpiry r, ChildCredit c}` |
| where it sits | a region frame, a sealed pack, a child, `spent`, `abandoned` | a region frame or a sealed pack |
| how it moves | endowment, spend, lend to a child, return at join, abandonment | opened by an operation, closed only by the ledger |
| can become the other | **never** | **never** |

`Duty::Release` is the ordered destroy list a cleanup runs; the other three are
the region's `obligations`. Whether an obligation is still outstanding is
**derived from the ledger on every query** — a join duty closes only when
`ScopedTaskLedger::join` actually returned a capsule, a loan duty only when the
ledger stops reporting the resource as loaned — never from this module's own
bookkeeping.

### 2.2 The conservation checkpoint

After **every** operation the checker re-derives

```text
Σ frames.allowable + Σ packs.allowable + Σ child_credit + spent + abandoned
```

and refuses any state where it is not the program's `initial_credit` (`Q673`).
The duty sets do not appear in that sum, which is the formal content of
acceptance 1: there is no arithmetic in which a duty could be spent, and no
operation that reads a duty to produce credit.

---

## 3. The deliverable

`crates/qcore-core/tests/unit/region_task_credit.rs::a_handler_joins_its_children_then_discharges_its_own_region`.
Ten allowable credit; a work region endowed with six; a child created with a
read loan on the parent's buffer and an endowment of four; the child completes
having spent one; the region raises; the handler joins, and only then discharges.

Measured certificate, checked and then independently rederived:

| Field | Value |
|---|---|
| `exit` / `scope_exited` | `Normal` / `true` |
| `joined_handles` / `failed_handles` | `{30}` / `{}` |
| `collected` | `[(30, I32(42))]` |
| `credit` | `initial 10, spent 9, abandoned 1, carried 0` |
| `trace` | `Endow{work,6}`, `Lend{1,4}`, `Spend{work,1}`, `ChildSpend{1,1}`, `Return{1,3}`, `Destroy(8)`, `AbandonCredit{work,1}`, `Spend{main,7}` |
| `assumptions` | `DeclaredCreationOutcome`, `DeclaredChildAccounting` |

The order in that trace is the acceptance clause made observable: the child's
credit comes back (`Return`) and the region's own resource is destroyed
(`Destroy`) **before** the remainder is abandoned (`AbandonCredit`).

The same program in the Lean model
(`QCore.RegionTaskCredit.deliverable`) is accepted with **9 spent, 1 abandoned,
10 conserved** — the same three numbers, computed by a separate implementation.

---

## 4. Acceptance

### 4.1 Access permissions and live duties cannot be discarded as unused credit

| Attempt | Measured refusal |
|---|---|
| discharge with the created child unjoined | `Q670` `PendingLifetimeObligation { region: "work", duty: Join(30) }` |
| discharge with the child's read loan outstanding | `Q670` `PendingLifetimeObligation { region: "work", duty: LoanExpiry(7) }` |
| spend the region's whole remainder, then discharge with a live join | `Q670`, unchanged — spending retires nothing |
| exit a region normally with credit left | `Q674` `UnusedCredit { region, remainder }` |
| exit the root region normally with credit left | `Q674` `UnusedCredit { region: "main", remainder: 3 }` |
| release a resource the open pack holds | `Q302` `ResourceHeldByCleanupPack(8)` |
| move a loaned resource into the error payload | `Q600` `LoanOutstanding { resource: 7, borrower: 1 }` (the ledger's own refusal) |

Full message text is pinned, e.g.
`Q670: region \`work\` cannot abandon credit while the join of handle \`30\` is outstanding`.

The last row is where the *access permission* clause is discharged concretely: a
payload move is an exclusive use, so T04's loan rule refuses it without this
profile having to restate the rule.

### 4.2 Parent-local credit is abandoned only after joins and loan expiry; child credit is not the parent's

| Attempt | Measured refusal |
|---|---|
| discharge while the child still holds four of the six endowed | `Q671` `ChildCreditOutstanding { region: "work", child: 1, amount: 3 }` |
| the parent spends what it lent the child | `Q675` `InsufficientCredit { region: "work", requested: 3, available: 2 }` — the frame simply does not have it |
| endow a child beyond the region's credit | `Q675` `InsufficientCredit { requested: 7, available: 6 }` |
| a child returns more than it was lent | `Q675` `InsufficientCredit { requested: 5, available: 4 }` |
| discharge after a **failed** join | `Q670` `Join(30)`, permanently |
| retry a failed join | `Q600` `JoinFailed(30)` (the ledger's own terminal rule) |
| fail a join that already succeeded | `Q600` `HandleConsumed(30)` |

Positive controls: joining first admits the discharge and abandons exactly the
remainder; a **failed creation** endowed nothing, lent nothing and opened no
duty, so its region's whole endowment is abandonable
(`initial 10, spent 4, abandoned 6`) and no `Lend`/`Return` event is emitted at
all.

Reporting precedence is pinned by
`every_duty_family_is_reported_and_none_shadows_another`: child credit, then
the read loan, then the join. Every outstanding duty refuses — only the message
is ordered — and without that order the join duty, which every creation opens,
would shadow the other two out of existence.

**Duty attribution is region-scoped, and that is a decision, not an accident.**
A duty belongs to the region that opened it. An inner region may abandon its own
remainder while an *enclosing* region's loan is live; the enclosing region is
then refused at its own exit, naming its own duty
(`an_enclosing_regions_duty_is_enforced_at_its_own_exit`, `Q670` `LoanExpiry(7)`
at region `main`). The alternative — refusing any abandonment anywhere while any
duty is live anywhere — would be stricter but would stop attributing the refusal
to the region that can act on it.

### 4.3 A handler rethrow carries all remaining payload and cleanup duties

`a_handler_rethrow_carries_its_payload_and_its_remaining_cleanup`: the handler
joins, discharges, moves resource 7 into the error payload and rethrows. The
escaping certificate records

* `exit = Error("Work.Failed")`, `scope_exited = false`;
* `payload = {7}` — carried, not dropped;
* `escaping_pack.releases = [9]` — the root region's remaining destroy duty,
  carried, not run and not forgotten;
* `credit = { initial 6, spent 0, abandoned 2, carried 4 }` — credit that never
  had to be spent is **carried**, never abandoned;
* `collected = [(30, Error("Child.Failed"))]` — the worker's own typed error is
  collected by the join as data, not implicitly propagated.

Refusals around it:

| Attempt | Measured refusal |
|---|---|
| rethrow with the pack still open | `Q307` `RethrowWithUndischargedCleanup` |
| return from the handler without discharging | `Q305` `CleanupPackNotDischarged` |
| let an error carry a join out of the thread scope | `Q672` `ObligationEscapesScope { duty: Join(30) }` |
| drop a payload obligation at a normal exit | `Q677` `PayloadObligationDropped { resource: 7 }` |
| discharge twice | `Q304` `CleanupPackDuplicated("pack")` |
| discharge a binding the handler does not hold | `Q302` `UnknownCleanupCapability("elsewhere")` |
| open the wrong region | `Q303` `WrongRegionOpened { expected: "work", actual: "elsewhere" }` |
| catch a nonempty pack without opening it | `Q306` `CatchWithoutRequiredQuantOpen` |

**Existing handler tests are untouched and pass.** `error_region.rs` and its
`Q302`–`Q309` unit tests, `scoped_tasks.rs` and its nine ledger tests, the CT20
handler-rethrow fixture and the whole `ct_corpus` run are byte-identical inputs
and all pass in the workspace run (§10).

### 4.4 The mutation matrix

`a_mutation_matrix_over_the_deliverable_rejects_every_row` applies **13 single
edits** to the accepted deliverable and pins each one's exact diagnostic: no
join, no discharge, a double discharge, a wrong binding, a rethrow instead of a
discharge, one unit unspent, one unit overspent, a normal exit with duties open,
a child that never completes, a child that returns more than it was lent, an
endowment beyond the region's credit, a release of the lent buffer, and an
undeclared error. Every row is refused; the unmutated program is re-checked in
the same test so the matrix measures the edits and not a broken fixture.

---

## 5. Refusal taxonomy and diagnostic allocation

Three families, kept separate so that "the profile refuses X" is testable:

* **The eight named refusals of this profile** occupy the free `Q670`–`Q677`
  range (`Q600` is the live scoped-task refusal, `Q601`–`Q609` and `Q624`–`Q629`
  appear only as a superseded revision-0.2 design allocation under
  `docs/obsolete/`, and T02 occupies `Q610`–`Q623`):

  | Diagnostic | Code |
  |---|---|
  | `PendingLifetimeObligation` | `Q670` |
  | `ChildCreditOutstanding` | `Q671` |
  | `ObligationEscapesScope` | `Q672` |
  | `CreditNotConserved` | `Q673` |
  | `UnusedCredit` | `Q674` |
  | `InsufficientCredit` | `Q675` |
  | `ObligationOutsideRegion` | `Q676` |
  | `PayloadObligationDropped` | `Q677` |

* **Carried over from the error-region pass**, condition for condition, keeping
  the code that pass already emits: `Q300` an undeclared raised error, `Q302`
  generic error-region structure, `Q303`, `Q304`, `Q305`, `Q306`, `Q307`,
  `Q309`. **No code is repurposed**: each of these names the same condition it
  already names in `error_region.rs`.
* **Carried over from the scoped-task ledger** at `Q600`, with the ledger's own
  `TaskLedgerError` reproduced verbatim and neither restated nor softened.

`the_named_refusals_occupy_the_free_q640_range` pins all three families,
including that every message begins with its own code.

`docs/QCORE_SPEC_r0_3.md` §L21 is **not edited**: its `Q6xx` row already reads
"concurrency and scoped-task checking" as a pass-level allocation, exactly as it
did after T02 took `Q610`–`Q623` without adding a row. Recording the new block in
the per-code table is left to the integrator alongside T02's, so that one edit
covers both (§11).

### 5.1 Three checks are fail-closed backstops, and are recorded as such

Not every check in the module is reachable from an admitted program, and this is
said here rather than left to look load-bearing:

| Check | Why it cannot fire today |
|---|---|
| `Q673` `CreditNotConserved` | it is the conservation checkpoint itself; `step_preserves_accounted` is the proof that it never fires. Its value is that conservation is *checked at runtime after every operation*, not merely argued. |
| `Q676` `ObligationOutsideRegion` | the root region is implicit and pushed before the body, so the frame stack is never empty inside a program. |
| `Q309` `ResourceInPayloadAndCleanupPack` | a payload move removes the resource's release duty, and re-acquiring a payload identity is already `Q302` `DuplicateResourceIdentity`. |

A fourth, the handler-exit `Q307` arm, is a backstop because `Raise` refuses an
open capability at the raise site first. All four are kept deliberately: the
project's fail-closed invariant makes an unreachable guard cheap and a missing
one a defect.

---

## 6. The certificate and its independent recheck

`RegionTaskCertificate` records the profile label, scope, exit tag, whether the
scope closed, the payload, the escaping pack, the credit accounting, the ordered
credit/cleanup trace, the joined and failed handles, the collected completion
outcomes, and the two declared assumptions.

`recheck_region_task_certificate` re-runs the checker from the program alone and
compares twelve fields. `a_forged_certificate_is_refused_field_by_field` pins
four forgeries — a rebalanced accounting, a trace with the abandonment and the
destroy deleted, a swapped exit tag (two mismatches at once), and an emptied
assumption set — plus a certificate presented against a program the checker
refuses, which returns `ProgramRejected` with the refusal rather than a field
diff.

---

## 7. Lean

New `QCore.RegionTaskCredit` namespace: a state of frames, children, an opened
pack and the two sinks, a nine-constructor `Op`, a partial `step` and a `run`.

| Theorem | Obligation |
|---|---|
| `step_preserves_accounted` | credit is conserved by every admitted step |
| `run_preserves_accounted` | and by a whole run, by induction |
| `discharge_only_moves_the_packs_remainder` | abandonment changes the remainder and nothing else — acceptance 1 |
| `spending_retires_no_duty` | spending leaves every duty, loan, join and child credit alone |
| `endowment_moves_credit_and_no_duty` | the same for opening a nested region |
| `the_parent_sinks_cannot_reach_child_credit` | neither sink touches what children hold |
| `discharge_requires_every_join` | acceptance 2, the join clause |
| `discharge_requires_every_loan_to_expire` | acceptance 2, the loan clause |
| `a_parent_cannot_abandon_credit_held_by_a_child` | acceptance 2, the child-credit clause |
| `a_normal_exit_is_gated_like_a_discharge` | there is no exit that discards a live duty |
| `a_normal_exit_refuses_a_remainder` | the error path is the only credit sink |
| `a_failed_join_closes_nothing` | a failed join leaves the state unchanged |
| `a_failed_join_leaves_the_pack_undischargeable` | so the refusal stands forever |
| `a_failed_creation_restores_the_parent_exactly` | a creation that did not happen opened no duty |
| `a_creation_records_every_duty_it_opens` | join, every loan, and the child credit when nonzero |
| `a_join_returns_exactly_the_unspent_child_credit` | no more and no less |
| `sealing_carries_every_duty` | acceptance 3: a rethrow sheds nothing |
| `sealing_requires_no_open_pack` | one pack at a time |
| `the_admitted_run_is_accepted_and_conserves_its_credit` | non-vacuity, and the Rust numbers |
| `discharging_before_the_join_is_refused` | the deliverable's negative control |
| `a_failed_creation_leaves_nothing_to_wait_for` | the failed-creation control |
| `a_normal_exit_with_a_live_join_is_refused` | refused, and accepted with the join |
| `an_outstanding_loan_refuses_the_discharge` | refused, and accepted after the loan closes |

Twenty-three public theorems, two private lemmas (`takeChild_splits`,
`gate_of_member`), and five of the twenty-three decided by `decide +kernel`
against fixtures that mirror the Rust ones. `AuditRegionTaskCredit.lean`
registers all twenty-three; `check_lean_audits.py` passes **24** audits (was 23)
with warnings as errors. Every theorem depends only on `propext` and
`Quot.sound`: **no new axiom, no `sorry`, no placeholder.**

### 7.1 Named Lean boundaries

1. **Rust/Lean correspondence is by construction and by shared fixtures, not
   proved.** The two implementations agree on the deliverable's three numbers
   and on four refusal/acceptance pairs, but no theorem says the Rust checker
   realises this model. F01's representation-relation technique is the shape
   this would take, and it is not done here.
2. **The Lean model has no ledger.** It carries `joined` and `loaned` as plain
   lists rather than embedding `ScopedTaskLedger`, so its notion of "the join
   really happened" is an assumption the Rust side discharges by calling the
   ledger. The theorems are about the credit/duty discipline given those facts.
3. **No threads, no schedule, no memory model, no time.** As in T04's model.

---

## 8. Boundaries, stated as limits

* **Model only.** No source syntax, no frontend lowering, no C11 emission, no
  reference execution, no native run. That is why there is **no ASan/UBSan/LSan
  evidence here**: this profile never reaches a compiler. The source increment
  is the natural follow-on and is the shape T03 took for T02.
* **No branching.** A creation's outcome is fixed by the program, so each
  outcome is a separate program, exactly as the CT native task scenario matrices
  do. There is no `Branch` operation and therefore no join-of-states rule.
* **One thread scope.** Nested regions are admitted; a second thread scope,
  nested workers and child-owned regions are not represented at all.
* **Cancellation and resumable effects are out of scope** by the card, and
  absent from the model rather than approximated.
* **`Complete`'s `unspent_credit` and `Spawn`'s outcome are declared premises**,
  named on every certificate as `DeclaredChildAccounting` and
  `DeclaredCreationOutcome`. This profile has no child program to derive them
  from.
* **The payload is modeled as a custody sink, not a typed value.** A payload
  move releases the resource from the ledger and records its identity; there is
  no owned error payload type, because **T06 is not in this branch** (§11).
  Combining shares (T05) or typed owned payloads (T06) with this profile is
  explicitly open.
* **Credit is `u64` and unitless.** It is a budget, not a quantitative type
  index; no connection to `qcore-core`'s usage grades or interval arithmetic is
  claimed or made.

---

## 9. CT21–CT25 — why nothing is proposed for the manifest

The card names CT21–CT25 as candidates. Three of them are this card's acceptance
clauses word for word:

| Case | Scenario | Model-level evidence here | Mappable? |
|---|---|---|---|
| CT21 | Quant opening discards positive access permission | `an_outstanding_read_loan_blocks_the_abandonment_of_its_regions_credit` — `Q670` `LoanExpiry(7)`, plus the ledger's own `Q600` `LoanOutstanding` on a payload move | **no** |
| CT22 | Quant opening discards parent-local unused credit after joins | the deliverable, plus `an_unjoined_child_blocks_the_abandonment_of_its_regions_credit` | **no** |
| CT23 | Parent abandons child-owned credit during its own error | `a_parent_cannot_abandon_credit_that_is_out_with_a_child` — `Q671` | **no** |
| CT24 | Cleanup destroys an owner before child join | T04 already proposed a source fixture | not this card's |
| CT25 | Cleanup destroys owner after joins and loan expiry | T04 already proposed a source fixture | not this card's |

**None of CT21–CT23 is proposed**, and the reason is structural rather than
procedural: the CT harness runs a `.qcore` **source fixture** through the real
pipeline and fails closed on a fixture whose case still reads `unmapped`. This
profile has no source syntax — there is no `region … catch … quant open` form in
the frontend that lowers onto it — so every CT stage including `parser` and
`type_checker` would have to be claimed `unassessed`, and a mapping whose every
stage is unassessed claims nothing. Recording model evidence in this report and
leaving the manifest at **15/50** is the honest position.

What CT21–CT23 need is one increment: a lexical source form for an error region
with a `quant open` catch, lowered onto this unedited checker, in T03's shape.
That increment can then map all three from one fixture family, with `parser`,
`type_checker` and `reference_execution` executed.

---

## 10. Gate results

Run at this card's tip, on Linux, after
`git submodule update --init --recursive` (libevent at
`d82464a277d0f42703702c4dfd9af6af38595a83`).

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1169 passed, 0 failed** (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **56 passed, 0 failed** |
| `python3 -B -m unittest discover -s scripts/tests` | **43 tests, OK** |
| `cd proof && lake build` | success, 68 jobs, no warnings |
| `python3 -B scripts/check_lean_audits.py` | **Passed 24 Lean audits** (was 23) |

1169 − 1141 = **28 new tests**: the 27 in
`crates/qcore-core/tests/unit/region_task_credit.rs` plus the module's doctest.

### 10.1 A pre-existing flaky test, recorded not weakened

The first baseline run of `cargo test --workspace` at the unmodified base
reported one failure,
`qcore-verifier-host::enforced_process_limits::a_well_behaved_provider_completes_under_the_enforced_profile`,
with `Io("Broken pipe (os error 32)")` at `enforced_process_limits.rs:433`. Run
alone the whole file passes 12/12. It is load-flaky under a fully parallel
workspace run, in the same family as the `RLIMIT_NPROC` flake already recorded
in [P03 §9.1](../remaining-p03/README.md). **No assertion was weakened and no
test was touched.** That failure also aborted the baseline run early, which is
why the truncated baseline read 1110; the complete baseline is 1141, matching the
ledger for this checkpoint, and 1169 − 28 reproduces it exactly.

---

## 11. Integrator decisions needed

1. **T05, T06 and T08 are not in this branch.** The card lists T04, T06 and V05
   as landed dependencies, but `c60fd91` contains only T04 (and V05's evidence);
   `docs/reports/remaining-t05`, `-t06` and `-t08` do not exist here. This card
   was therefore built to be **independent of them**: it uses no share algebra
   and no typed owned error payload, and the payload is a custody sink rather
   than a value. That makes the merge trivial but leaves a real gap — a module
   that uses both counted shares and typed payloads *and* error-region credit is
   unvalidated, and T06's note that the merged driver mislabels a shares+payload
   module as `ScopedReadSharesV1` is untested against this profile. **Decision:
   whether a follow-on card should validate the combined feature set.**
2. **`Q670`–`Q677` are a new block.** `docs/QCORE_SPEC_r0_3.md` §L21 is
   deliberately unedited, matching how T02's `Q610`–`Q623` were left. **Decision:
   whether to record both blocks in the per-code table in one edit.** The codes
   themselves are pinned by test here and by T02's report there.
3. **CT21–CT23 stay unmapped** (§9), and the source-form increment that would
   map them is unclaimed. **Decision: whether to schedule it**, and whether
   T04's proposed CT24/CT25 fixtures should land first since they share the
   handler shape.
4. **Duty attribution is region-scoped** (§4.2). If the integrator prefers the
   stricter global rule, it is a one-line change to `require_discharged` plus one
   test; the current behaviour is deliberate and tested either way.

---

## 12. Pinned artifacts and status language

* **Nothing is promoted.** Raw Cycle 19 units remain **2 Safe / 42 provisional
  Unsafe / 137 Unknown** plus four data-only units. This card touches no
  gateway, contract, profile, schema, policy or provider file, so the four
  pinned 185-unit artifacts and the configured CMetrics audit were **not
  re-measured**; that is by construction — the whole diff is one new
  `qcore-core` module, its tests, and three proof files — rather than by
  measurement. `CORPUS-REPIN` remains open and unchanged.
* The contract corpus stays at **59 modules**, the `third_party/libevent`
  submodule pin is unchanged, and `docs/reevaluation/ct_manifest.json` is
  untouched at **15/50 mapped**.
* `qcore-schemas` stays **5.0.0**; prose revision 0.3, plugin protocol v1,
  evidence v2 and program schema/profile v2 are unchanged. No wire byte moved.
* Full source-language conformance remains **false**. This card checks a model
  and issues certificates within it; it establishes nothing about C source,
  native execution or whole-program safety.
