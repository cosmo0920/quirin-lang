# CT23-CREDIT-ERROR-PATH — an error path over the flat epoch's credits

Date: 2026-09-12. Baseline: `f122cc03` (wave-26 close). Branch:
`worktree-agent-a50095313616f5e4a` (isolated worktree). Wave 27.

**What landed:** `EpochCreditRecoveryV1` — a new core sibling module
(`crates/qcore-core/src/epoch_credit_recovery.rs`) that checks what the
creator's **own error edges** do with the credit a flat epoch granted each
creation, plus the `error(move|drop|close);` row clause that spells it on the
epoch routes and the driver wiring that publishes and independently re-derives
the resulting split. 22 new Rust tests, 17 new audited Lean theorems in the
existing `QCore.EpochSourceLowering` namespace.

**Nothing is promoted, and nothing is relaxed.** No diagnostic code is added,
moved or downgraded; no reserved word is added; `qcore_core::epoch_accounting`
is **untouched**; no `qcore-schemas` change; no CT manifest edit; no corpus
artifact touched. Every epoch program admitted before this card is admitted
after it, with a byte-identical `EpochBoundCertificate` and byte-identical
emitted C. The card's whole effect is **two new refusals** and one published
quantity.

---

## 1. The measured choice: (a), on the composed route

The card offered two candidates in soundness order:

> **(a)** a recoverable failure arm on an epoch-route operation whose credit
> accounting is checked through the failure — the rollback discipline the task
> route already has for failed creation; **(b)** composing
> `source.epoch.bound.v1` with the existing typed-error vocabulary so an epoch
> module can spell an error region.

**(b) was measured first and found to be already done, in the wrong half.** The
composed route `source.epoch.tasks.v1` (card `T08-COMPOSE`) puts a flat-epoch
plan in front of a real checked scoped-task region, and that region **already
carries the whole typed-error vocabulary**: `?`, `Result::Ok`/`Result::Err`,
`throws<AllocError>`, `throws<SpawnError>`, `raise`. A composed module can
already spell the parent's error. What it could not do is make the *epoch half*
see those edges: `check_flat_epoch` models exactly one way out of an epoch —
every task created, every task joined, the exit reached — so an error in the
parent moved no credit, which is precisely what
`docs/reports/ct-wire-lane/README.md` §3.4 recorded. Building a *second* error
vocabulary for the epoch would have duplicated one the composition already has,
and — the operational reason — a parallel card owns `quant open` source
statements, so region grammar is not this card's to touch.

**`CheckedRegionTaskCreditV1` (T07) was measured next, because the card brief
asked whether it already serves CT23.** It very nearly does: its `Duty::ChildCredit`
is credit endowed to a child and not yet returned, its `Q670`/`Q671` gate
abandonment on the error path, its `Q672` refuses an obligation escaping the
thread scope on an error edge, and its `Q673` checkpoint re-derives
`initial = Σ frames + Σ child-held + spent + abandoned`. But `region_task_credit`
is a **model with no source route at all** — its own header says it "checks a
model; it emits no C and imports no source" — and a CT mapping needs a fixture
compiled through `compile_files`. Adopting T07 would therefore have meant
landing a region-credit *source route*, which is the parallel card's territory.
The seam is flagged in §8.

So: **(a)**, and the "epoch-route operation whose failure is recoverable" is the
one the checked task IR already has —
`TaskTerminatorKind::TrySpawn { success, spawn_error }`, the creation, together
with `TaskTerminatorKind::TryOutcome { success, error }`, the join of a child
that raised. Both are the *creator's own* error edges, both are derived from the
checked module, and neither needed a line of new region grammar.

### Why a spelled disposition, and not a derived one

The credit figures of this profile are declared premises
(`EpochAssumption::DeclaredTaskFootprints`), and the checked task IR contains no
credits at all — so what happens to a grant on an error edge is not derivable
from anything. What *is* derivable is the **shape**: whether the creator has an
error edge over a creation. The card splits them exactly there, on the
`T08-COMPOSE` precedent for the disposal clause:

| Fact | Source |
|---|---|
| the creator's error edges over a creation | **derived** from `TrySpawn.spawn_error` and from whether a `TryOutcome` eliminates that creation's join outcome |
| what the error edge does with the grant | **declared**, and then checked against the grant the epoch actually makes |

A plan therefore cannot claim an error path the program does not have, and
cannot restore credit the epoch never granted.

---

## 2. The accounting law — the card's deliverable

> **Credit conservation across the creator's error edges.** For every task `t`
> of an admitted plan,
>
> ```
> restored(t) + spent(t) = credit(t)      and     restored(t) ≤ credit(t)
> ```
>
> componentwise on both axes; and over the whole epoch
>
> ```
> restored_credits + spent_credits = granted_credits ≤ allocation.
> ```

Both directions are **refusals**, not conventions:

* **No leak.** `ErrorArmCredit::AbandonedAtErrorArm` is the representable claim
  that a grant is neither restored nor spent — the creator walked its error edge
  and the credit simply stopped being accounted for. Always refused,
  `Q635`. **This is CT23.** It exists to be spellable for the same reason
  `ResultDisposal::DroppedAtCompletion` does: the case must be sayable so that
  the *core* is the thing that refuses it, rather than it being a case nobody
  checks (the T03 discipline).
* **No mint.** A restoration larger than its grant would put credit into the
  epoch that the epoch never granted, and the allocation bound would stop
  bounding anything. Refused per task and per axis, `Q636`.

**The corollary is what makes the error path safe without any second bound.**
Because `spent_credits ≤ granted_credits`, the credit committed on **every**
error edge is bounded by the credit committed on the success exit, which
`check_flat_epoch` already bounds by the allocation at `Q637`. So `Q637` is
deliberately **not** extended: a second allocation check there would be an
unreachable condition rather than a gate. The corollary is proved in Lean
(`the_error_arm_commits_no_more_than_the_granted_total`) and asserted on every
admitted certificate in Rust, rather than being asserted in prose.

### Q630–Q641 conditions extended, and the argument for each

Zero new codes. Four existing conditions of the flat epoch's own band gain a
sibling, and each choice is argued from what that code already means:

| New condition | Code | Why that band |
|---|---|---|
| `ArmCountMismatch`, `UnknownTask`, `DuplicateArm`, `ErrorArmWithoutErrorEdge` | `Q630` | `Q630` is the flat epoch's **structural** band (`EmptyEpoch`, `DuplicateTask`, `TaskCountMismatch`, `DuplicateSharedCell`). All four are the same kind of statement: the plan disagrees with the epoch it is a plan for |
| `CreditAbandonedAtErrorArm` | `Q635` | `Q635` is the **outstanding-at-the-epoch-exit** band, and already carries *two* conditions for that reason (`UntransferredResultAtEpochExit`, `UnjoinedTask`). Abandoned credit is the third: a quantity the epoch granted that no exit accounts for |
| `RestoredCreditExceedsGrant` | `Q636` | `Q636` is the **per-task credit** band (`TaskExceedsItsCredit`: a demand above the grant). A restoration above the grant is the same per-task relation read in the other direction |
| `Arithmetic` | `Q641` | the arithmetic band. The error path overflows and underflows into it **exactly as the success path does**, and a test pins both halves of that sentence on the same overflowing epoch |

No code is reused by a second *pass*: every one of these is issued by the
`Concurrency` pass, which owns `Q630`–`Q641` under §L21, from a module that is a
sibling of the one already in that band.

---

## 3. The surface — zero new reserved words

```ebnf
row_clause = … | "error", "(", ( "move" | "drop" | "close" ), ")", ";"
                | "error", "(", "move", NAT, "effort", NAT, ")", ";" ;
```

| Clause | Meaning |
|---|---|
| `error(move);` | the **whole** grant returns to the epoch |
| `error(move BYTES effort UNITS);` | that much returns; the remainder is spent by the creator's failure handling |
| `error(drop);` | nothing returns: the failure handling spends the whole grant |
| `error(close);` | neither restored nor spent — **abandoned**. Always refused (`Q635`) |
| *elided* | `error(drop);` — exactly the accounting that existed before this card |

**The constraint-14 census is vacuous, by construction.** `error`, `move`,
`drop`, `close` and `effort` are *all already reserved* — the first four by the
language (`is_reserved_source_word`), the fifth by `T08-EPOCH-SRC`. The clause
reserves **zero** new words, so no existing source can stop parsing because of
it, and the whole workspace suite (which executes every fixture and inline
program in the repository) is the confirmation.

The shape is the deliberate mirror of the existing `join(move|drop|close);`
clause, and `close` carries the same meaning it already has there: *stopped
being live with neither a transfer nor a release*. `error(move 0 effort 0);` and
`error(drop);` are one claim, and a test and a Lean theorem both pin that.

`error(move);` with no figures is resolved against the row's own `credit`
clause, in either written order — the clause does not repeat a figure the row
already wrote.

---

## 4. Where the work is done, and where it is not

| Component | Change |
|---|---|
| `crates/qcore-core/src/epoch_credit_recovery.rs` | **new** (623 lines): the model, `check_epoch_credit_recovery`, `recheck_epoch_credit_recovery`, `EpochCreditDiagnostic` with its `Q630`–`Q641` `code()` |
| `crates/qcore-core/tests/unit/epoch_credit_recovery.rs` | **new**: 9 unit tests |
| `crates/qcore-core/src/epoch_accounting.rs` | **untouched** |
| `crates/qcore-core/src/region_task_credit.rs` | **untouched** |
| `crates/qcore-core/src/task_execution.rs` | **untouched** |
| `crates/qcore-core/src/lib.rs`, `prelude.rs` | one `pub mod`; two additive prelude re-exports |
| `crates/qcore-frontend/src/source_epoch.rs` | the shared clause parser (`parse_error_arm`, `resolve_error_arm`), two new `LoweredEpochModule` fields, `locate_credit`, the pure route's arm assembly |
| `crates/qcore-frontend/src/source_epoch_tasks.rs` | `joins_can_fail` (the derived join edge), `LedgerSpawn::error_edges`, the row clause, the composed arm assembly |
| `crates/qcore-verification/src/driver.rs` | both epoch routes call `check_epoch_credit_recovery` + `recheck_epoch_credit_recovery`; one new field on each of `CheckedEpochModule` and `ComposedEpochModule` |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02e, §G02f | the clause added to both consumer grammars, with its meaning on the composed route and its refusal on the declarative one |
| `docs/QCORE_SPEC_r0_3.md` §L21 | the `Q630`–`Q641` row now names the sibling profile that extends four of its conditions. **No code is added, moved or reallocated**, and the band stays with the `Concurrency` pass, so this records the band's contents rather than going through the §L21 new-code process |

Not touched, deliberately: `AGENTS.md`, `CLAUDE.md`, `README.md`,
`DEVELOPMENT.md`, `docs/reviews/r0_3/acceptance*.json`,
`docs/reevaluation/ct_manifest.json`, `examples/`, the C gateway, the buffer,
string and aggregate territories, and every region statement grammar.

The driver never publishes a split it did not re-derive: `recheck_epoch_credit_recovery`
runs on every accepted module before `CompiledModule` is built, and a recheck
failure is `Q600`, exactly as the bound's own recheck failure is.

### The declarative route's boundary, stated from the core's band

`source.epoch.bound.v1`'s workers are *declarations*, not creations, so the
creator has no error edge over them at all. The grammar admits the clause there
and the **core** refuses it (`ErrorArmWithoutErrorEdge`, `Q630`), attributed to
the `error(…)` clause itself. That is the T03 discipline again: the boundary is
a checked refusal from the owning band, not a grammar that pretends the word
does not exist. An *elided* arm on that route is admitted and publishes an
all-spent split — which is why every pre-existing declarative epoch is unchanged.

---

## 5. Evidence

### Rust — 22 new tests, all green

`crates/qcore-core/tests/unit/epoch_credit_recovery.rs` (9), on the model:

| Test | Covers |
|---|---|
| `every_admitted_arm_conserves_its_grant` | the law in the admitting direction, per row and over the epoch; the corollary against the allocation; the recheck |
| `an_elided_arm_spends_the_whole_grant_and_changes_no_figure` | elided ≡ `SpentByCreator` ≡ `RestoredToEpoch { 0 }`; the bound's `granted_credits` unmoved |
| `an_abandoned_grant_is_refused_and_names_the_task_and_the_axis` | the leak refusal, `["Q635"]`, including a grant of nothing |
| `restoring_more_than_the_grant_is_refused_on_each_axis` | the mint refusal, `["Q636"]`, on memory, on effort, and on an axis the grant does not carry |
| `the_error_path_overflows_and_underflows_into_the_arithmetic_band` | `["Q641"]`, **beside** the success path refusing the same overflow in the same band |
| `a_plan_that_does_not_describe_this_epoch_is_refused_structurally` | all four `Q630` conditions, and the elided/declared asymmetry |
| `either_error_edge_alone_admits_a_declared_arm` | the creation edge alone and the join edge alone; `CreationErrorEdges::default().any() = false` |
| `a_forged_split_is_named_by_the_recheck` | `CreditLedgerUnbalanced`, `RestoredCreditMinted`, `ArmUnbalanced`, `PlanRejected`, a foreign profile label |
| `concrete_plans_are_admitted_and_the_display_carries_the_code` | non-vacuity: three admitted plans; `Display` prefixes `Q635` |

`crates/qcore-frontend/tests/source_epoch_tasks.rs` (3), on the derivation:
`the_creators_error_edges_are_derived_from_the_checked_region` (an infallible
join contributes **no** edge; a raising worker contributes one),
`every_row_carries_one_arm_with_its_declared_disposition` (all four spellings,
and `error(move);` resolving to the row's own grant),
`the_error_clause_refuses_the_forms_it_cannot_represent` (five `Q600` shapes).

`crates/qcore-frontend/tests/source_epoch.rs` (1):
`the_declarative_route_lowers_an_arm_per_worker_and_gives_it_no_error_edge` —
one arm per declared worker with no edges, the elided plan admitted with an
all-spent split, a spelled clause refused by the core at `Q630` **and attributed
to the clause's own span**.

`compiler/tests/epoch_credit_error_path.rs` (9), end to end through
`compile_files`, every refusal asserting the **exact ordered code vector**:

| Test | Covers |
|---|---|
| `abandoning_a_childs_credit_on_the_creators_error_edge_is_refused` | **the CT23 case**: `["Q635"]`, naming task 1 and the abandoned figure; and the derived edges the case rests on |
| `every_admitted_arm_splits_its_grant_and_the_split_adds_back` | four fixtures; every published figure; conservation; the corollary; the recheck |
| `restoring_more_than_the_grant_is_refused_end_to_end` | `["Q636"]` with its figures |
| `the_bound_certificate_is_unchanged_by_the_error_path` | the `EpochBoundCertificate` is equal with and without the clause, and with restoration |
| `the_emitted_c_is_unchanged_by_the_error_arm_byte_for_byte` | `c_source`, `source_map`, `entry` and `reference_observation` equal across four clause spellings of one module |
| `the_error_clause_grammar_refuses_its_own_malformed_forms` | four `["Q600"]` shapes with their messages |
| `the_declarative_route_has_no_error_edge_and_says_so_from_the_cores_band` | the pure route admitted elided, refused `["Q630"]` for all three spellings |
| `the_derived_edges_follow_the_checked_region_and_nothing_else` | a worker that cannot raise gives `join_failure: false` |
| `a_forged_plan_against_a_published_program_is_refused` | the driver's recheck condition, exercised directly |

### Reference / native byte comparison — the route's own standard

`source.epoch.tasks.v1`'s standard is emission parity against the task route's
own output for the blanked text (`T08-COMPOSE` §5), and this card keeps it:
`the_emitted_c_is_unchanged_by_the_error_arm_byte_for_byte` compiles one module
under four different `error(…)` clauses — one path, one line count, so
`source_map` and `reference_observation` are compared too rather than being
excused by a fixture's filename — and asserts all four outputs equal. **The
epoch half emits nothing and the error path emits nothing**, so no new native
fixture or sanitizer run is claimed; the emitted program is the scoped-task
route's, and that route's native evidence is unchanged and not re-attributed
here.

### Lean — 17 new audited theorems, no new axiom

`proof/QCore/EpochSourceLowering.lean` gains an error-path section; the audit
list in `proof/AuditEpochSourceLowering.lean` grows from 67 names to 84. **The
audit *file* count is unchanged at 110**, so the acceptance-matrix derived-facts
test is not disturbed. Every new theorem depends only on `propext` and
`Quot.sound`; no `sorry`, no placeholder, no project axiom.

The law, **both directions**, per arm and over the epoch:

* `an_admitted_arm_restores_no_more_than_its_grant` — **no mint**, per axis.
* `an_admitted_arm_conserves_its_grant` — **no leak**: `restored + spent = credit`.
* `an_admitted_plan_conserves_the_granted_total` — the same identity over an
  arbitrary list of arms, by induction (not by `decide` on an example).
* `an_admitted_plan_restores_no_more_than_it_granted` — the mint direction over
  the epoch.
* `the_error_arm_commits_no_more_than_the_granted_total` — **the corollary**:
  the credit committed on the all-error path never exceeds the granted total,
  which is why no second allocation check exists.

The refusals: `an_abandoned_grant_is_not_admitted`,
`a_plan_with_an_abandoned_arm_is_not_admitted` (universally quantified over the
list and the position), `a_declared_arm_without_an_error_edge_is_not_admitted`,
and by `decide` on concrete arms `the_abandoning_plan_is_refused`,
`an_over_restoring_plan_is_refused` (both axes), `a_declarative_arm_is_refused`.

The invariance: `an_elided_arm_is_admitted_and_spends_its_whole_grant` and
`restoring_nothing_is_spending_everything` are the two theorems that say this
card changed no existing accounting.

**Not a vacuous audit:** `the_worked_plans_are_admitted` asserts four concrete
plans **are** admitted and `the_worked_plans_publish_their_split` pins their
exact published figures (1024 / 3072 / 25 / 75 / 4096 — the same numbers
`partial.qcore` measures in Rust), so the refusal family is not proved against a
predicate that refuses everything.

The Lean transcription of the Rust checker is **by inspection**, as everywhere
else in this namespace; there is no digest-checked correspondence of the F01
kind.

---

## 6. CT23 readiness — what a lane can now claim

**`docs/reevaluation/ct_manifest.json` and `docs/reviews/r0_3/acceptance.json`
are untouched**, and CT coverage stays **33/50 mapped**. The measured rows are
in `ct_stage_evidence.json` beside this file, as a *proposal*.

The fixtures are parked at `compiler/tests/fixtures/epoch_credit/CT23/` and
**not** under `compiler/tests/fixtures/ct/`: `ct_corpus.rs` fails closed when an
unmapped case already owns a fixture directory there. Mapping moves the
directory — the `T08-EPOCH-SRC` precedent.

| Fixture | Role | Measured |
|---|---|---|
| `case.qcore` | **the case** | refused, exact cascade `["Q635"]`, `CreditAbandonedAtErrorArm { task: 1, axis: "effort", abandoned: 100 }` |
| `restored.qcore` | control (the other disposition) | admitted; `restored = {memory 4096, effort 100}`, `spent = 0` |
| `spent.qcore` | control (the conservative disposition) | admitted; `restored = 0`, `spent = {4096, 100}` |
| `no_arm.qcore` | control (attribution) | admitted; the same certificate `spent.qcore` publishes — removing **exactly** the clause removes the refusal |
| `partial.qcore` | boundary | admitted; `restored = {1024, 25}`, `spent = {3072, 75}` — the split is a real division, not a flag |
| `minted.qcore` | boundary (the other direction) | refused, `["Q636"]` — the same clause over-restoring |

What CT23 asks for is *"Parent abandons child-owned credit during its own
error"*, expected **Reject**. The fixture's worker declares
`throws<AllocError>` and the creator joins it with `?`, so the derived edges are
`{creation_failure: true, join_failure: true}` — the join edge is the one on
which **the child existed and held the grant**, and the creator is on its own
error path. A test asserts those derived edges on the case's own shape, so the
mapping does not rest on a reading of the fixture.

Three things the CT lane must decide before it can map this, recorded rather
than assumed:

1. **The route.** The case is on `source.epoch.tasks.v1`, which the manifest's
   `stage_vocabulary.routes` does not have. It needs either a new
   `composed_epoch` route (parse, elaborate, `epoch_lowering`,
   `epoch_bound_check`, `task_lowering`, `task_ledger_check`,
   `task_reference_execution`, `c_emission`, …, proof) or the existing `epoch`
   route with the task stages added; `ct_corpus.rs`'s pinned tables must grow
   with it, since the harness cross-checks them and refuses drift.
2. **The verdict mode.** `reject` covers the case, and the four admitting
   controls carry exact certificate figures — an `epoch_credit_split` mode
   (beside `epoch_bound_figures`) would let a mapping claim what this card
   actually measured rather than less.
3. **`native_sanitized` and `proof` stay `unassessed`.** This card runs no
   native fixture and adds no proof stage evidence, so sanitized-native stays
   **4 of 33** and proof **0 of 33**. If the lane accepts the row, the grown
   denominator is **mapping, not conformance** (constraint 23).

---

## 7. Standing per-change gate

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok |
| `cargo test --workspace --no-fail-fast` | **4,232 passed, 0 failed, 8 ignored** (22 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0, `failed_expectations: 0` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0, `failed_expectations: 0` |
| `python3 -B scripts/check_lean_audits.py` | **Passed 110 Lean audits** (unchanged; the 17 new theorems extend `AuditEpochSourceLowering.lean`, 67 names → 84) |
| `python3 -B -m unittest discover -s scripts/tests` | 193 tests, OK |
| `cargo test` in `providers/` | 41 passed, 0 failed |

**No flake was observed.** None of the five constraint-16 rows failed at any
point in this card's runs, so none needed an isolated re-run; nothing in this
card touches the provider host or the verifier host. Every new theorem's axiom
surface was read directly and is `[propext]`, `[propext, Quot.sound]` or empty
— no project axiom, no `sorry`.

The workspace total is the whole suite at this baseline, not a delta the card
computed: 4,232 covered with 22 of them new. `crates/qcore-core/src/epoch_accounting.rs`
has an empty `git diff` for this card, which is the mechanical form of "the
unedited `FlatEpochBoundV1`".

---

## 8. What this does not establish

* **No quantity is imported from anything.** `restored` joins `credit`, `peak`,
  `retained`, the allocation and the overhead as a **declared** figure, and all
  four `FlatEpochBoundV1` premises are carried unchanged on every certificate
  either route publishes. A source form for declaring a rollback is not a
  derivation of one.
* **Restoration relaxes no bound.** `granted_credits` is still the plain sum of
  the grants and `Q637` still refuses it above the allocation; restored credit
  is **not** re-granted to a replacement creation. An epoch that wants a retry
  to reuse a failed creation's credit still needs twice the allocation, and
  that is deliberate: making the committed total path-sensitive would *admit*
  programs `check_flat_epoch` currently refuses, which is the forbidden
  direction. Named `CT23-RESTORE-REUSE`.
* **One disposition covers both edges.** A row declares one disposition for the
  creation-failure edge and the join-failure edge together. A plan that wants to
  say different things about them is `CT23-EDGE-SPLIT`.
* **The declarative route has no error path**, and says so as a refusal rather
  than acquiring one. Giving `source.epoch.bound.v1` a declared creation outcome
  would be declaring a second premise, not checking anything.
* **This is not `CheckedRegionTaskCreditV1`.** T07's `Duty::ChildCredit`,
  `Q670`–`Q677` and its `initial = Σ frames + Σ child-held + spent + abandoned`
  checkpoint remain a model with no source route, untouched here. The two
  disciplines agree in shape — abandonment is the error path's gated credit sink
  in both — and are not connected. Named `CT23-T07-SEAM`.
* **Rust/Lean correspondence is by inspection.**
* **Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; leak diagnostics stay **272**;
  the contract corpus stays 72 modules; CT coverage stays 33/50 mapped;
  `qcore-schemas` stays 14.0.0. This card touches no audit path and no pinned
  artifact.

---

## 9. Open, named

| Id | Item |
|---|---|
| `CT23-CT-MAPPING` | The manifest route, the verdict mode and the fixture move of §6. Integrator's CT lane. |
| `CT23-RESTORE-REUSE` | Restored credit is not re-grantable: a creation reachable only through another's failure arm still pays for its own credit out of the same allocation. Making the committed total path-sensitive would admit programs the current `Q637` refuses, so it needs its own card and its own parity argument. |
| `CT23-EDGE-SPLIT` | One disposition per row covers both of the creator's error edges. Distinguishing "the creation failed" from "the child raised" needs two clauses and two published splits. |
| `CT23-T07-SEAM` | `CheckedRegionTaskCreditV1` (T07) models the same discipline over error *regions* and has no source route. Connecting the two — an epoch plan over a region that spends, lends and abandons credit — waits on the parallel card's `quant open` source statements. |
| `CT23-ARM-UNREACHABLE` | `ArmCountMismatch`, `UnknownTask` and `DuplicateArm` are unreachable from either source form (both lowerings emit exactly one arm per task, in task order) and are tested at the core level only. `ErrorArmWithoutErrorEdge` is reachable, from the declarative route. |
| `CT23-EMACS` | `editors/emacs/qcore-mode.el` is unchanged; it already highlights every word the clause uses, since the clause reserves none. Recorded for completeness, not as an omission. |
