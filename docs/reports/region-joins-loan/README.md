# REGION-JOINS-LOAN — the loan half of CT25

Date: 2026-09-13. Base: `main` at `30aee0b2` (the wave-29 close), in an
isolated worktree on branch `worktree-agent-a4a3a6288b8724c44`. **Wave 30.**

**What landed:** the residual `REGION-JOINS-LOAN` that
`docs/reports/region-joins-duties/README.md` §9 named — *loan expiry now has a
referent on the composed route*, and with it **CT25's whole scenario**
(*"Cleanup destroys owner after joins and loan expiry"*, expected **Accept**)
is expressible and accepted end to end:

```
let owner = qcore_buffer_new(42)?;
thread scope s {
    match spawn s reader(read owner share FULL) {   // the loan
        Result::Ok { value: handle } => {
            let seen = join move handle?;           // …expires here
        }
        Result::Err { error: failure } => { drop owner; return 0; }
    }
    error region R {
        move owner;                                 // the adoption
        credit 5;
        effort 2;
        qcore_check(false, Failure);
    } catch (Failure) quant open R as pack {
        discharge pack;                             // …destroys the owner
    }
    return 1;
}
```

Two admissions carry it, both composing **unedited** checkers:

1. **Read-loan captures** — the task route's own `read PLACE share N` at a
   spawn argument — are admitted on `source.epoch.tasks.v1`, declared in the
   plan as the place-free `read;` clause. This **deliberately discharges
   `T08-COMPOSE-LOAN`** (§2): the refusal's stated reason dissolved rather
   than being overridden.
2. **Adoption** — `move NAME;` in the composed region body — takes a live
   entry owner's cleanup duty into the sealed pack: the same
   `CleanupAction::Destroy` a region acquisition seals, the difference living
   entirely in the erasure (nothing is kept for it, because the owner's
   acquisition already stands in the entry's own text before the region).

"After joins **and loan expiry**" is then a composition of three existing
rules, none edited: the share returns at the join (the task ledger's own
reclamation), every join precedes the region (`Q600`, wave 28's derived span
rule), and the discharge's erased `drop NAME;` is written into the region's
blanks — after the joins, therefore after the expiry. The converse is a
refusal on the same unedited model: a `drop` of the owner while the share is
out refuses at the task ledger's own `LoanOutstanding` (`Q600`), measured
both as a written statement (`live_share_drop.qcore`) and as a region moved
above the join (§6).

**Nothing else moves.** Zero new codes (one `Q302` condition widening and one
deliberate `Q600` condition **narrowing**, both stated in §L21 and §4), zero
reserved words (§5), the composed checkers — the scoped-task route and its
ledger, `check_flat_epoch` + `EpochCreditRecoveryV1`,
`qcore_core::error_region::analyze_error_regions` — all **unedited**, no
`qcore-schemas` change, no CT manifest or acceptance matrix edit, no corpus
artifact touched, and the wave-29 fixture directory
`compiler/tests/fixtures/epoch_credit/CT25/` untouched (its "loan expiry NOT
expressible" comment is superseded by this card; the file itself is a landed
artifact whose byte offsets its own tests pin, so the correction lives here
and in the grammar doc rather than in an edit). Every module admitted before
this card publishes byte-identical certificates, evidence and emitted C.

---

## 1. Why T08-COMPOSE-LOAN could be discharged rather than worked around

The wave-15 refusal (`docs/reports/remaining-t08-compose/README.md` §"open")
reads: *"a loan's residency is shared with the lender, which this flat
accounting does not represent."* That premise is about a residency shared
between two **tasks** of the epoch. On this route it cannot arise:

* A spawned worker is a first-order leaf function (the scoped-task profile's
  own rule), so no worker lends anything. **The lender is always the
  creator.**
* Creator-scope bytes are outside the epoch's allocation — the standing
  wave-28/29 boundary `REGION-JOINS-EPOCH-AXIS`, which this card keeps and
  re-measures (§6: the epoch certificates are byte-identical with and without
  the loan and the adoption).

So a read loan on this route has **no epoch residency to represent**: it
contributes no byte to any task's accounting and no shared-storage cell. What
it does have — share arithmetic, the write exclusion while a share is out,
the refusal of a release of the lent owner, the reclamation at the join — is
exactly the **unedited task ledger's** discipline, which checks the task half
of every composed module before the epoch is even reconciled. The discharge
is therefore a narrowing of a `Q600` refusal condition whose stated premise
did not hold on this route, taken deliberately and stated in §L21 — not a
weakening of any rule that was checking something.

`T08-COMPOSE-SHARED` (shared storage cells, `read NAME;`/`write` clauses) and
`T08-COMPOSE-GUARD` stand untouched.

## 2. Proposed amendment text for standing constraint 15m

For the integrator to apply to `AGENTS.md` (this card does not edit it). Two
sentences of the current text are superseded: the admitted-shape list (which
gains the adoption), and the owner-name sentence ("A region owner's name is
the region's own…", which becomes the acquired arm of a two-arm rule). The
rest is carried verbatim with the loan facts appended:

> 15m. **A composed region on the epoch task route opens its pack after every
> join, and the pack's duties are the parent's own owners.**
> `REGION-CREDIT-JOINS` (wave 28) let a `source.epoch.tasks.v1` entry carry
> one `error region … catch (E) quant open … { discharge …; }` whose
> `AbandonCredit` publishes the parent-local unused credit — CT22's scenario.
> `REGION-JOINS-DUTIES` (wave 29) **deliberately superseded** the duty-free
> premise this constraint recorded: the composed body also admits the task
> route's own acquisition and release — `let NAME = qcore_buffer_new(INT)?;`
> sealing a real `Destroy` duty, `drop NAME;` of a region owner erasing one —
> and the discharge destroys the remaining owners in reverse acquisition
> order, CT25's duty half. `REGION-JOINS-LOAN` (wave 30) **deliberately
> discharged `T08-COMPOSE-LOAN`** and landed the loan half: a worker may
> capture a counted read share of a creator owner (`read PLACE share N` at
> the spawn; the place-free `read;` clause in the plan), admitted because the
> lender on this route is always the creator and creator-scope bytes are
> outside the epoch allocation (`REGION-JOINS-EPOCH-AXIS`), so the loan has
> no epoch residency; and the region body also admits the **adoption**
> `move NAME;`, which takes a live entry owner's cleanup duty into the pack —
> an acquisition from the pack's side, erasing completely, its release
> written into the blanks after every join. "After joins and loan expiry" is
> the composition of unedited rules: the share returns at the join under the
> task ledger, the joins precede the region (`Q600`), and a destruction while
> a share is live refuses as the ledger's own `LoanOutstanding` — do not
> re-implement any of the three. Premises that must survive refactors, each a
> refusal today: every `TrySpawn`/`Join` span of the checked task IR
> **precedes** the region (`Q600` in both directions — children live at the
> opening stays `CheckedRegionTaskCreditV1`'s territory, `REGION-JOINS-T07`);
> the region **erases into its straight-line twin** — a duty-free region
> blanks completely (emitted C, source map and reference observation
> byte-identical with and without it, the wave-28 measurement kept), a
> duty-carrying one keeps its acquisitions and body releases at their own
> byte offsets in the task half with one `drop NAME;` per remaining duty
> written into the blanked space after them, first-fit, newlines never
> crossed, refusing when it does not fit (`REGION-JOINS-ERASURE-FIT`), and an
> adoption keeps nothing (the owner's acquisition already stands in the
> entry's own text) — so the composed checkers (the scoped-task route and its
> ledger, `check_flat_epoch` + `EpochCreditRecoveryV1`,
> `analyze_error_regions`) stay **unedited**, each refusing at its own codes.
> **Do not re-implement a custody rule beside the erasure**: two
> simultaneously live owners refuse at the task route's existing custody
> `Q600` — the second acquisition's `?` failure edge discards the first, the
> erased program's real leak path (`REGION-JOINS-MULTI-OWNER`, reachable
> since wave 30 with an adopted owner beside an acquisition) — and an adopted
> name that is not a live owned Buffer in the erased program refuses at the
> task route's own codes (`Q500` for a non-buffer, `Q600` for custody). Both
> published figures are re-derived before publication — `abandoned_credit`
> against the reader's quantity fold, `released` against its duty fold, over
> the model's own cleanup trace. An **acquired** owner's name is the region's
> own (a collision with an entry binding refuses, since the erasure flattens
> the body's block into the task scope); an **adopted** owner's name is the
> entry's own (a name the entry does not bind refuses); the sequential
> `qcore_acquire` spelling names no task-route resource and stays refused;
> and the declarative epoch route gains no error path (constraint 15j's
> shape). Zero new codes: `Q302` condition widenings and the one stated
> `Q600` narrowing, in §L21. `region-credit-joins`, `region-joins-duties`,
> `region-joins-loan`.

## 3. Where the work is done, and where it is not

| Component | Change |
|---|---|
| `crates/qcore-frontend/src/source_epoch_tasks.rs` | `ComposedCapture` gains `Read`; the ledger derives it from `ScalarOperand::Loan` (the refusal arm removed, reason documented); the plan reader admits exactly `read;` (a `read NAME;` keeps a T08-COMPOSE-SHARED refusal, `write` unchanged); the region reader admits `move NAME;` (owner list gains an adopted/acquired marker; adopted names must be entry bindings, acquired names must not; adoptions keep nothing in the erasure) |
| `crates/qcore-verification/src/driver.rs` | **untouched** — the adopted duty flows through the wave-29 duty fold, trace cross-check and `released` evidence unchanged |
| `compiler/tests/region_joins_loan.rs` | **new**: 11 end-to-end tests, every refusal an exact code vector, including the hand-written-twin byte comparison and the native counting-allocator run |
| `compiler/tests/fixtures/epoch_credit/CT25_LOAN/` | **new**: the case and six controls (§6) — deliberately outside `compiler/tests/fixtures/ct/` and outside the wave-29 `CT25/` directory |
| `crates/qcore-frontend/tests/source_epoch_tasks.rs` | 3 new reader/lowering unit tests (24 total) |
| `proof/QCore/RegionJoinsLoan.lean`, `proof/AuditRegionJoinsLoan.lean`, `proof/QCore.lean` | **new** Lean module, 16 audited theorems (§7); audit count 118 → **119** |
| `docs/QCORE_SPEC_r0_3.md` §L21 | the `Q302` widening and the deliberate `Q600` narrowing stated (§4) |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02f | the `read;` clause and the adoption in the grammar; the refusal list updated (T08-COMPOSE-LOAN marked discharged) |

Not touched, deliberately: `qcore_core::task_execution`,
`qcore_core::scoped_tasks` (the share ledger), `qcore_core::error_region`,
`qcore_core::epoch_accounting`, `qcore_core::epoch_credit_recovery`,
`crates/qcore-verification/src/driver.rs`, the sequential route, `examples/`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance*.json`,
`AGENTS.md`, `compiler/tests/fixtures/ct/`, the wave-29 `CT25/` fixture
directory, the C gateway, and every corpus artifact. Corpus-neutral: no
gateway, contract, audit or provider file is edited, so no arbiter run is
implied.

## 4. Zero new codes, zero new words

Under the `Q-NAMESPACE-DECISION` ice box this card allocates nothing. Stated
in §L21 and here:

* **`Q302`** widens a fourth time on the composed route: the admitted-set
  condition gains the adoption `move NAME;`, and the card's own shape
  refusals land beside the existing ones — an adopted name the entry does not
  bind, and one owner taken twice through either shape.
* **`Q600`** is **narrowed**, deliberately: the blanket read-loan-capture
  refusal (`T08-COMPOSE-LOAN`) is discharged (§1). No `Q600` condition
  widens: the capture reconciliation gains a third clause word inside its
  existing elementwise condition, and every custodial refusal about the loan
  (`LoanOutstanding` on a drop while the share is out) is and was the task
  ledger's own.
* **`Q500`** is reached, not changed: an adopted non-buffer refuses at the
  task route's existing "drop requires an owned buffer" over the erased
  program.

**Constraint 14 census: +0 reserved words, vacuous by construction.** Every
token of the admitted shapes — `read`, `;`, `move`, a name — is already
reserved or already a reference on this route: `read` and `move` are in the
resolver's standing clause-word list ("already reserved and reused",
`frontend.rs`'s T08-EPOCH-SRC block), `read;` was a refused clause shape and
`move NAME;` a refused region-body shape before this card (both measured
baselines, kept as tests). What changed is only which statements two readers
admit.

## 5. Refusals kept, refusals changed, and the two admitted

**Kept exactly** (code and message, verified by the unedited wave-28/29
suites running green — `region_credit_joins.rs` 9, `region_joins_duties.rs`
9, `epoch_tasks_source.rs` 9, `epoch_credit_error_path.rs` 9, frontend
`source_epoch_tasks` 21 of 24):

| Refusal | Code | Status |
|---|---|---|
| shared cells, `read NAME;`, `write` clauses | `Q600` | unchanged, still name `T08-COMPOSE-SHARED`; the `read page;` case still refuses (`Q013` undeclared / `Q600` declared, the wave-15 measurement) |
| a spawn or join after the region, both directions | `Q600` | unchanged; re-verified with an adopted owner (`the_region_cannot_precede_the_loan_expiry`) |
| the mutex composition | `Q600` | `T08-COMPOSE-GUARD` untouched |
| the model's own pack obligations | `Q303`–`Q309`, `Q300` | unchanged codes and wording |
| the wave-29 duty shapes' own refusals | `Q302` | unchanged (outer release, collision, doubled acquisition, erasure fit, `qcore_acquire`) |
| two simultaneously live owners | `Q600` | unchanged (`REGION-JOINS-MULTI-OWNER`), now also reachable with an adopted owner beside an acquisition |
| a drop of a lent owner | `Q600` | the task ledger's own `LoanOutstanding`, untouched — measured as this card's negative control |

**Changed, being the two admitted:** the plan's `read`-clause refusal becomes
the `read;` admission (a `read` clause of any other shape refuses with the
loan clause and the shared-cell rule both named); the region reader's
admitted-set message now lists `move NAME;` and names both cards (the wave-28
probe test passes unedited — its `contains("REGION-JOINS-DUTIES")` assertion
is satisfied by the new message).

## 6. Evidence

### Rust — 14 new tests, all green; the wave-28/29 suites green unedited

`compiler/tests/region_joins_loan.rs` (11), end to end through
`compile_files`:

| Test | Covers |
|---|---|
| `ct25_the_cleanup_destroys_the_lent_owner_after_joins_and_loan_expiry` | **CT25, both halves**: admitted; captures `[Read]` on ledger and row; `released = ["owner"]`, `abandoned_credit = 3`; the adoption handed to the model as the pack acquisition; the creation's span precedes the region; the owner's allocation in the emitted C |
| `the_adopted_region_erases_into_its_straight_line_twin_byte_for_byte` | emitted C, entry, reference observation byte-identical with a **hand-written** twin (the adoption erased, `drop owner;` at the region's first blank run); source-map positions identical row for row, at most the synthesized release's rendering apart |
| `a_live_share_refuses_the_destruction` | `live_share_drop.qcore` refused `["Q600"]`, message the ledger's own `LoanOutstanding` |
| `the_region_cannot_precede_the_loan_expiry` | the region moved above the join refused `["Q600"]` |
| `an_adopted_owner_released_in_the_body_erases_its_duty` | `released = []`, `abandoned_credit = 3` — the frame rule through the adoption |
| `the_adoption_reader_refuses_its_own_shapes` | `["Q302"]` unknown entry binding (named), `["Q302"]` owner taken twice |
| `an_adopted_scalar_is_refused_by_the_unedited_task_route` | `["Q500"]`, "drop requires an owned buffer" — custody not re-implemented |
| `an_acquisition_beside_an_adopted_owner_keeps_the_multi_owner_boundary` | `["Q600"]`, "discards" — `REGION-JOINS-MULTI-OWNER` reached through the adoption |
| `the_capture_reconciliation_reads_loans_in_both_directions` | `copy;` vs. loan and `read;` vs. copy both `Q600`, naming both lists |
| `the_epoch_certificates_are_unmoved_by_a_loan_and_an_adoption` | bound and credit-recovery certificates byte-equal against the duty-free control (`REGION-JOINS-EPOCH-AXIS` re-measured) |
| `the_native_run_frees_the_lent_owner` | Linux/macOS: emitted C under counting allocator hooks; 1 allocation (the lent owner), freed set = allocated set, exit success |

`crates/qcore-frontend/tests/source_epoch_tasks.rs` (3):
`a_read_loan_capture_is_derived_and_declared_as_read` (ledger `[Read]`, row
`[Read]`, elementwise mismatch still refused),
`the_adoption_reader_reads_a_move_as_a_duty` (duties, ops, erasure: nothing
kept, one release at the region's first blank run; the dropped-owner control
keeps one span and inserts none), `the_adoption_reader_refuses_its_own_shapes`
(unknown name, doubled adoption, adopt-then-acquire crossing, bare `move;`).

### Fixtures

`compiler/tests/fixtures/epoch_credit/CT25_LOAN/`, **outside**
`compiler/tests/fixtures/ct/` (the harness fails closed when an unmapped case
owns a directory there; mapping moves the directory — the CT22/CT23
precedent) and outside the wave-29 `CT25/` directory:

| Fixture | Role | Measured |
|---|---|---|
| `case.qcore` | **the case** | admitted; captures `[Read]`, `released = ["owner"]`, abandoned 3; join precedes region; native run frees the owner |
| `live_share_drop.qcore` | destruction while the share is live | refused `["Q600"]`, `LoanOutstanding` — the task ledger's own rule |
| `adopted_then_dropped.qcore` | the frame rule | admitted; `released = []`, abandoned 3 |
| `adopt_unknown.qcore` | a name the entry does not bind (`move reader;`) | refused `["Q302"]`, named |
| `adopt_twice.qcore` | one owner taken twice | refused `["Q302"]` |
| `adopt_scalar.qcore` | an adopted i32 | refused `["Q500"]` by the unedited task route |
| `adopt_and_acquire.qcore` | acquisition beside an adopted owner | refused `["Q600"]`, `REGION-JOINS-MULTI-OWNER` |

### Reference / native byte comparison

The family's standard is kept: the hand-written-twin comparison is
byte-for-byte on emitted C and reference observation, with the loan and its
expiry standing identically in both texts. The native counting-allocator run
(Linux/macOS, `clang -std=c11 -pthread -Wall -Wextra -Werror`,
`QCORE_BUFFER_MALLOC`/`QCORE_BUFFER_FREE` hooks) shows one allocation and one
free of the same address. **No sanitizer claim is made** — the scoped-task
family's native pattern carries none, and none is added here. The loan's own
sanitized native evidence is card T04's (`scoped_read_loans.rs`), unedited
and re-run green.

### Lean — 16 new audited theorems, no new axiom

`proof/QCore/RegionJoinsLoan.lean`, registered in
`proof/AuditRegionJoinsLoan.lean` and imported by `proof/QCore.lean`. Audit
count **118 → 119** (`Passed 119 Lean audits`). Every theorem's axiom surface
is empty, `[propext]` or `[propext, Quot.sound]`; no `sorry`, no placeholder,
no project axiom. `QCore.ReadLoan` (T04), `QCore.EpochSourceLowering`
(wave 15) and `QCore.RegionJoinsDuties` (wave 29) are consumed, not edited.

| Claim | Theorems |
|---|---|
| the capture language: three constructors where wave 15 had two; the inclusion injective; the read capture genuinely new | `admitted_captures_are_exactly_these`, `the_loan_free_capture_language_is_unchanged`, `a_read_capture_is_new` |
| the region body: five shapes where wave 29 had four; the adoption an acquisition of the pack **by definition**; the adoption-free sublanguage unchanged | `admitted_region_shapes_are_exactly_these`, `an_adoption_is_an_acquisition_of_the_pack`, `the_adoption_free_sublanguage_is_unchanged` |
| the pack after joins carries the adopted duties; every duty is the parent's own; an adoption moves no credit | `the_pack_after_joins_carries_the_adopted_duties`, `duties_are_the_parents_own`, `an_adoption_moves_no_credit` |
| **the CT25 property, both halves**: while the share is live the destruction refuses; the join readmits it; the discharge of the adopting pack destroys exactly the owner and abandons exactly the remainder; the whole lending round trip returns the destruction to the owner | `a_live_share_refuses_the_destruction`, `the_loan_expires_at_the_join`, `the_cleanup_destroys_the_lent_owner_after_joins_and_expiry`, `the_round_trip_returns_the_destruction_to_the_owner` |
| the flagship's own figures | `the_flagship_adopts_one_owner_and_abandons_three`, `an_adopted_owner_released_in_the_body_erases_its_duty`, `an_adopted_and_an_acquired_duty_are_destroyed_last_first` |

The Rust/Lean correspondence is **by inspection**, as everywhere in this
family. The after-joins ordering premise is wave 28's span rule (`Q600`),
tested in Rust, and stated as the placement premise of the composed theorem
rather than re-proved.

## 7. CT25 readiness — what a wire lane can and cannot claim

**`docs/reevaluation/ct_manifest.json` and `docs/reviews/r0_3/acceptance.json`
are untouched by this card**; mapping is the CT lane's (constraint 23).

**"Cleanup destroys owner after joins and loan expiry" is now fully
expressible, and accepted end to end**, with every word of the case text
carried by a checked fact:

* *cleanup destroys owner* — the unedited model's `Destroy` trace on the
  discharge of the opened pack, made a real emitted, custody-checked,
  reference-executed and natively-run destruction by the erasure
  (`released = ["owner"]`, cross-checked against the reader's duty fold);
* *after joins* — the derived span rule refused at `Q600` when false, plus
  wave 29's `duties_are_the_bodys_own` (no duty a child held);
* *and loan expiry* — the owner **is lent** (captures `[Read]`, derived from
  the checked IR's `ScalarOperand::Loan`), the loan **expires at the join**
  (the unedited task ledger's reclamation; `the_loan_expires_at_the_join` in
  Lean), and the destruction is **ordered after the expiry** because the
  joins precede the region — while the pre-expiry destruction is a measured
  refusal (`LoanOutstanding`, `a_live_share_refuses_the_destruction` in both
  Rust and Lean).

The proving test is
`region_joins_loan.rs::ct25_the_cleanup_destroys_the_lent_owner_after_joins_and_loan_expiry`
on `epoch_credit/CT25_LOAN/case.qcore`, with
`region_joins_duties.rs::ct25_the_cleanup_destroys_the_owner_after_every_join`
(wave 29) still standing beside it for the duty-only shape. A lane mapping
CT25 should cite the loan case as the scenario's referent — the wave-29
report's narrowed reason *"loan expiry has no referent on the composed route
(`T08-COMPOSE-LOAN`)"* is **retired by this card** — and moves the fixture
directory it maps per the CT22/CT23 precedent. Sanitized-native and proof
stage claims stay governed by constraint 23: this card's native run carries
no sanitizer, so a lane may not read `sanitized-native: executed` from it.

## 8. What this does not establish

* **No write loan and no non-creator lender exist**, and none is claimed: the
  task route executes `read own<Buffer> share FULL` captures only, and its
  own gates keep everything else out; this card admits what that route
  executes and nothing more.
* **The epoch bound is not moved.** The lent owner is a creator-scope buffer
  outside the epoch's allocation (`REGION-JOINS-EPOCH-AXIS` re-measured, §6).
* **No sanitizer claim** for the native run (§6).
* **Rust/Lean correspondence is by inspection.**
* **Nothing is promoted.** No corpus row, CT row, contract count or version
  number moves; the card touches no audit path and no pinned artifact.

## 9. Open, named

| Id | Item |
|---|---|
| `REGION-JOINS-LOAN-ADOPT-ONLY` | An adoption requires the owner to be destroyed by the discharge or released in the region body; a program wanting the owner to *survive* the region simply does not adopt it — no "un-adopt" shape exists, on purpose (the pack is linear). Recorded as a boundary, not a gap. |
| `REGION-JOINS-MULTI-OWNER` | Unchanged from wave 29, now also reachable with an adopted owner beside an acquisition (`adopt_and_acquire.qcore`). |
| `REGION-JOINS-T07`, `REGION-JOINS-ERASURE-FIT`, `REGION-JOINS-EPOCH-AXIS` | Unchanged from waves 28–29. |
| `CT25-CT-MAPPING` | The mapping itself is the CT lane's, on its own stage-evidence record (§7); this card's readiness statement removes the wave-29 blocker. |

Items owed to the integrator rather than edited here: the constraint 15m
amendment (§2), `AGENTS.md`'s headline numbers (Lean audits 118 → 119;
workspace test totals), the `named_followups` reconciliation for
`T08-COMPOSE-LOAN` and `REGION-JOINS-LOAN`, and the acceptance-matrix
regeneration that moves its derived `validation.lean_audits` (the one
disclosed Python failure in §10, the standard audit-adding precedent).

## 10. Standing per-change gate

Run at the card tip with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok |
| `cargo test --workspace --no-fail-fast` | see the final report table (run at the tip) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 119 Lean audits** (118 → 119, `AuditRegionJoinsLoan.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 193 ran, 192 OK, **1 disclosed failure**: the matrix tripwire asserting the committed `validation.lean_audits` (118) equals the tree's recomputed value (119) — exactly this card's new audit file; the reconciliation is the integrator's wave-close matrix pass, the wave-29 precedent |

This card's own rows: `compiler/tests/region_joins_loan.rs` **11 passed**
(native run included), `qcore-frontend`'s `source_epoch_tasks` integration
tests **24 passed** (21 → 24), and the wave-28/29 suites
(`region_credit_joins.rs` 9, `region_joins_duties.rs` 9,
`epoch_tasks_source.rs` 9, `epoch_credit_error_path.rs` 9,
`scoped_read_loans.rs` 4, `scoped_read_shares.rs` 4) **green unedited**.
