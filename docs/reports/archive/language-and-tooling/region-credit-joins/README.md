# REGION-CREDIT-JOINS — the composed region: parent-local credit after the joins

Date: 2026-09-12. Base: `main` at `e552e59c` (the wave-27 close plus the
uninitialised-submodule precondition fix), in an isolated worktree on branch
`claude/vibrant-curie-9f9380`. **Wave 28.**

**What landed:** the composition gap two wave-27 reports named from opposite
sides — `CT23-T07-SEAM` (`docs/reports/ct23-credit-error-path/README.md` §9:
credits and the creator's error edges live on the epoch routes) and
`REGION-CREDIT-WITH-JOINS` (`docs/reports/quant-region-source/README.md` §5:
`quant open` lives on the flat sequential route, which has no join) — closed by
the minimal honest composition. A `source.epoch.tasks.v1` entry may now write,
**after every spawn and join of its task scope**, one

```
error region R {
    credit 5;
    effort 2;
    qcore_check(false, Failure);
} catch (Failure) quant open R as pack {
    discharge pack;
}
```

whose sealed pack carries **no duty and exactly the parent's unused credit**,
which the opening's discharge abandons — the kernel's own explicit
`AbandonCredit`, decided by the **unedited**
`qcore_core::error_region::analyze_error_regions`. That is CT22's scenario —
*"Quant opening discards parent-local unused credit after joins"*, expected
**Accept** — expressible end to end for the first time, and measured by
`compiler/tests/region_credit_joins.rs::ct22_the_opening_discards_the_parents_unused_credit_after_all_joins`
on `compiler/tests/fixtures/epoch_credit/CT22/case.qcore`.

**Nothing is promoted and nothing is relaxed.** No diagnostic code is added
(`Q-NAMESPACE-DECISION` ice box: two condition widenings, stated in §L21 and
§3 below), no reserved word is added (constraint 14: every word of the form was
already reserved and the statement already parsed), `qcore_core::error_region`,
`qcore_core::epoch_accounting`, `qcore_core::epoch_credit_recovery` and
`qcore_core::region_task_credit` are all **untouched**, no `qcore-schemas`
change, no CT manifest edit, no corpus artifact touched. Every composed epoch
module admitted before this card is admitted after it with byte-identical
certificates and byte-identical emitted C (`region: None` on its evidence), and
the sequential route's `source.region.v1` is not edited at all.

---

## 1. The composition, and what each unedited model decides

The card brief named two models to hand facts to, and both are composed here
without an edit:

| Model | What it is handed | What it decides |
|---|---|---|
| `qcore_core::error_region` (`analyze_error_regions`) | the region statement's own operations — `AddStrictCredit`/`SpendStrictCredit`, the decided `Raise`, one catch clause with its `QuantOpen` struct and its `Discharge` handler | every pack obligation: the linear capability, the exactly-once discharge, the abandoned remainder, at its own `Q303`–`Q309`/`Q300` codes (constraint 15l) |
| `EpochCreditRecoveryV1` (`check_epoch_credit_recovery`) | unchanged — the plan's `error(…)` arms over edges derived from the checked task IR | unchanged — the credit-conservation law of constraint 15j |

plus the two the route already ran unedited: the scoped-task route on the
blanked task half, and `check_flat_epoch`/`recheck_epoch_bound` on the
reconciled plan.

The composition's own work is exactly three facts, each checked rather than
assumed:

1. **The region is read, then blanked.**
   `qcore_frontend::source_epoch_tasks::read_composed_region` reads the
   statement from the parsed program's own tokens (constraint 15b), and
   `split_composed_epoch` blanks its byte range exactly as it blanks the plan's
   — offsets preserved, so every span still points at the original file. The
   task half the scoped-task route compiles is therefore byte-for-byte the
   program the entry runs: the region **erases from emission** (constraint 15l
   reached from a second route), measured by
   `the_emitted_c_is_unchanged_by_the_region_byte_for_byte` over one in-memory
   module — same path, same line count — whose `c_source`, `source_map`,
   `entry` and `reference_observation` are all equal with the region written
   and with its lines blank.
2. **"After joins" is derived from the checked task IR, never declared.** The
   driver (`check_composed_region`) reads every `TrySpawn` terminator span and
   every `Join` instruction span out of the checked creator and refuses at
   `Q600` when any follows the region statement's start
   (`a_spawn_or_join_after_the_region_is_refused`, both directions). The
   entry's statements run in written order on this route, and the scoped-task
   ledger already requires every creation joined before its scope exits — so at
   the region's entry **no child exists and no child holds any grant**, which
   is what makes the sealed remainder *parent-local*.
3. **The published quantity is re-derived.** The abandoned credit is read out
   of the model's own cleanup trace **and** cross-checked against the reader's
   independent fold of the written `credit`/`effort` statements; a disagreement
   is `Q600`, exactly as the bound's and the split's recheck failures are. The
   soundness of that cross-check — the two computations are one function on
   every body — is a Lean theorem
   (`the_readers_fold_is_the_models_remainder`), not a convention.

### Why the region body admits no acquisition here

The composed body is narrowed to `credit N;`, `effort N;` and the decided edge
— a `let … = qcore_acquire(…)` is refused by name (`REGION-JOINS-DUTIES`, §5).
Two reasons, both structural. The scoped-task route's resources are buffers
with their own custody model, and the sequential route's
`qcore_acquire`/`SourceOp` vocabulary does not exist in the task program — so a
sealed duty would need an emission for the discharge's releases that this
composition does not have, and admitting one would assert a custody composition
neither checker was given (the `T08-COMPOSE-GUARD` discipline). And the
narrowing is exactly what makes the CT22 claim honest: a pack with **no
actions** has precisely one thing an opening can discard, the parent-local
remainder — proved as `a_parent_local_body_builds_no_duty` and stated as a
shape by the model (`ParentStmt` has two constructors).

## 2. Where the work is done, and where it is not

| Component | Change |
|---|---|
| `crates/qcore-frontend/src/source_epoch_tasks.rs` | `ComposedRegion`, `read_composed_region` (the reader: discovery, placement, body/handler shapes, the independent remainder fold), `split_composed_epoch` gains the `region_span` parameter and blanks it |
| `crates/qcore-verification/src/driver.rs` | `check_composed_region` (the after-joins IR check, the unedited model run, the exit/trace read-back, the cross-check), `ComposedRegionEvidence`, one new field `ComposedEpochModule::region` |
| `compiler/src/lib.rs` | re-exports `ComposedRegionEvidence` |
| `compiler/tests/region_credit_joins.rs` | **new**: 9 end-to-end tests, every refusal an exact ordered code vector |
| `compiler/tests/fixtures/epoch_credit/CT22/` | **new**: the case and six controls (§4) |
| `crates/qcore-frontend/tests/source_epoch_tasks.rs` | 4 new reader/split unit tests (18 total in the file) |
| `proof/QCore/RegionCreditJoins.lean`, `proof/AuditRegionCreditJoins.lean`, `proof/QCore.lean` | **new** Lean module, 14 audited theorems (§6); audit count 114 → **115** |
| `docs/QCORE_SPEC_r0_3.md` §L21 | the two condition widenings stated (§3) |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02f | the `region_epilogue` grammar and its boundary paragraph |

Not touched, deliberately: `qcore_core::error_region`,
`qcore_core::epoch_accounting`, `qcore_core::epoch_credit_recovery`,
`qcore_core::region_task_credit`, `qcore_core::task_execution`, the sequential
route's `source_region.rs` reader and driver lowering, `examples/`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance*.json`, the
C gateway, and every corpus artifact. Corpus-neutral: no gateway, contract,
audit or provider file is edited, so no arbiter run is implied.

## 3. Zero new codes, zero new words

Under the `Q-NAMESPACE-DECISION` ice box this card allocates nothing and widens
two conditions, stated in §L21 and here:

* **`Q302`** widens a second time (first: `QUANT-REGION-SOURCE`), on the
  `SOCKET-EVENT-COMPOSE` precedent — the same shape conditions reached from a
  second route. The composed reader's own refusals all land here: a region in a
  worker, outside the creator's task scope, a second region, a second catch
  clause, a body statement outside the admitted set (including the named
  acquisition refusal), a missing or passing edge, an uncaught raise, a handler
  statement other than the discharge. `Q300` keeps its undeclared-error
  condition from this reader, and `Q303`–`Q309` are reached through the
  unedited model exactly as from the sequential route — constraint 15l's rule,
  verified by `the_models_own_refusals_arrive_at_their_own_codes`
  (`Q305`, `Q306`, `Q303`, `Q304`, and the model's own `InsufficientCredit`
  wording at `Q302`).
* **`Q600`** widens its composed-route reconciliation condition: a spawn or
  join of the checked task region whose span follows the region statement. The
  same kind of statement every other `Q600` of this route makes — the program
  disagrees with the composition's premise — and derived from the checked IR.

**Constraint 14 census: +0 reserved words, vacuous by construction.** Every
word of the form — `error`, `region`, `catch`, `quant`, `open`, `as`,
`discharge`, `credit`, `effort`, and the body's `qcore_check` call shape — is
already reserved or already an ordinary call, and the whole statement already
parsed (`frontend::error_region_statement`, Cycle 6). What was newly *reachable*
is only the composed route's reading of it: before this card a region anywhere
in a composed module refused with the task profile's own "statement is outside
the copy-scalar task execution profile" at `Q600` — a measured baseline, now a
deliberate set of named refusals instead.

## 4. Evidence

### Rust — 13 new tests, all green

`compiler/tests/region_credit_joins.rs` (9), end to end through
`compile_files`:

| Test | Covers |
|---|---|
| `ct22_the_opening_discards_the_parents_unused_credit_after_all_joins` | **the CT22 case**: admitted; `abandoned_credit = 3`; the published operations are exactly the written statements; the pack holds no duty; the ledger's join precedes the region in the checked IR |
| `the_boundary_spends_everything_and_abandons_nothing` | `spent_out.qcore`: `abandoned_credit = 0` — the figure is a fold, not a flag |
| `the_epoch_certificates_are_unmoved_by_the_region` | bound and credit-recovery certificates equal with and without the region; a region-free module publishes `region: None` |
| `the_emitted_c_is_unchanged_by_the_region_byte_for_byte` | one in-memory module, same path and line count: `c_source`, `source_map`, `entry`, `reference_observation` all equal across the two variants |
| `a_spawn_or_join_after_the_region_is_refused` | `["Q600"]` in both directions (a join after the region; a spawn after the region), message naming the card rule |
| `the_models_own_refusals_arrive_at_their_own_codes` | `["Q305"]` walk-away, `["Q306"]` unopened nonempty pack, `["Q303"]` wrong region, `Q304` doubled discharge, `Q302` + "insufficient strict credit" overspend |
| `the_composed_reader_refuses_its_own_shapes` | `Q302` acquisition (`REGION-JOINS-DUTIES` named), `Q302` passing check, `Q300` non-error name, `Q302` region in a worker, `Q302` second region (`REGION-MULTIPLE` named) |
| `both_models_fire_in_one_module` | `both_models.qcore`: `error(move);` restores the epoch grant on the derived edges **and** the region abandons 3 — two unedited models, two published quantities, one entry |
| `the_pure_epoch_route_keeps_its_refusal` | a declarative-route module with a region keeps `["Q600"]` — the declarative route gains no error path (constraint 15j's shape) |

`crates/qcore-frontend/tests/source_epoch_tasks.rs` (4):
`the_composed_region_reader_reads_the_parents_epilogue` (the ops, the fold, the
declared error set, and `Ok(None)` for a region-free module),
`the_region_reader_refuses_placement_and_shape` (seven shapes with their named
follow-ons), `the_split_blanks_the_region_and_preserves_every_byte_offset`.

### Reference / native byte comparison — the route's own standard

`source.epoch.tasks.v1`'s standard is emission parity against the task route's
own output for the blanked text (`T08-COMPOSE` §5, kept by
`CT23-CREDIT-ERROR-PATH`), and this card keeps it:
`the_emitted_c_is_unchanged_by_the_region_byte_for_byte` compares one module
under the region and under blank lines — one path, one line count, so
`source_map` and `reference_observation` are compared too. **The epoch half
emits nothing and the region emits nothing**, so no new native fixture or
sanitizer run is claimed; the emitted program is the scoped-task route's, and
that route's native evidence is unchanged and not re-attributed here.

### Lean — 14 new audited theorems, no new axiom

`proof/QCore/RegionCreditJoins.lean`, registered in
`proof/AuditRegionCreditJoins.lean` and imported by `proof/QCore.lean`. Audit
count **114 → 115**. Every theorem's axiom surface is empty, `[propext]` or
`[propext, Quot.sound]`; no `sorry`, no placeholder, no project axiom.
`QCore.ErrorRegion` and `QCore.QuantRegionSource` are consumed, not edited.

| Claim | Theorems |
|---|---|
| the pack is parent-local: a body of quantities builds no duty, so the sealed pack holds only the remainder | `a_parent_local_body_builds_no_duty`, `the_pack_after_joins_holds_only_the_quantity` |
| **the CT22 property**: the discharge restores the baseline, runs no action, abandons exactly the remainder, consumes the capability; the empty pack still consumes | `the_opening_abandons_exactly_the_parents_remainder`, `discharging_an_empty_pack_still_consumes` |
| the erasure: no acquisition, no release reaches the task program | `a_parent_local_body_emits_no_acquisition`, `a_parent_local_body_emits_no_release`, `the_region_erases_from_the_task_program` |
| the driver's cross-check is sound: the reader's fold and the model's remainder are one function; the overspend still refuses; the flagship's own figures | `the_readers_fold_is_the_models_remainder`, `overspending_refuses_here`, `the_flagship_abandons_three`, `the_boundary_abandons_nothing` |
| after the joins, no child holds anything | `a_join_returns_its_spawn`, `the_sequential_spawn_join_discipline_leaves_no_child` |
| the admitted shapes as one list | `admitted_parent_shapes_are_exactly_these` |

The Rust/Lean correspondence is **by inspection**, as everywhere in this
family. The `live` lemmas consume the scoped-task ledger's "every creation
joined before its scope exits" as a hypothesis shape rather than re-proving it;
that fact is the task model's own.

### Fixtures

`compiler/tests/fixtures/epoch_credit/CT22/`, **outside**
`compiler/tests/fixtures/ct/` (the harness fails closed when an unmapped case
owns a directory there; mapping moves the directory — the CT23 precedent):

| Fixture | Role | Measured |
|---|---|---|
| `case.qcore` | **the case** | admitted; `abandoned_credit = 3`; join span precedes region span |
| `no_region.qcore` | attribution control | admitted; `region: None`; certificates equal to the case's |
| `spent_out.qcore` | boundary | admitted; `abandoned_credit = 0` |
| `before_join.qcore` | the scenario's other half, false | refused, exact cascade `["Q600"]` |
| `no_discharge.qcore` | CT21's shape from this route | refused, `["Q305"]` |
| `no_open.qcore` | the unopened nonempty pack | refused, `["Q306"]` |
| `wrong_region.qcore` | the wrong pack | refused, `["Q303"]` |
| `both_models.qcore` | both compositions in one entry | admitted; restored `{4096, 100}` **and** abandoned 3 |

## 5. What is refused, and why — the v1 boundary, named

Every boundary is a refusal with a name, never a silent gap:

| Refused shape | Name | Discharge condition |
|---|---|---|
| a region acquisition (`let … = qcore_acquire(…)` in the composed body) | `REGION-JOINS-DUTIES` | a task-route duty vocabulary: buffers acquired inside the region sealed as pack duties, with an emission for the discharge's releases in the task program and an ownership story against the task custody model. This is also **CT25's gap** (§7) |
| a spawn or join after the region | (the card's own rule, `Q600`) | a region the parent enters while children are live — which is `CheckedRegionTaskCreditV1`'s territory (`Duty::ChildCredit`), see `REGION-JOINS-T07` below |
| a region in a worker | (reader, `Q302`) | workers are first-order leaf functions; nothing to discharge |
| a region outside the creator's task scope, or a second region | `REGION-MULTIPLE` precedent (reader, `Q302`) | as on the sequential route |
| a second catch clause, a missing/passing edge, an uncaught raise, a handler statement other than discharge | `REGION-MULTIPLE-CATCH`, `REGION-NORMAL-EXIT`, `REGION-PROPAGATE`, `REGION-HANDLER-BODY` (reader, `Q302`) | the sequential route's own v1 path bound (constraint 15l), reached from this route with the same names |
| the declarative epoch route with a region | measured `["Q600"]`, unchanged | the declarative route has no error path (constraint 15j) and gains none here |

Two boundaries are recorded rather than refused:

* **`REGION-JOINS-EPOCH-AXIS`.** The parent's strict credit is the region
  model's one-axis `u64` quantity; the epoch's credits are two-axis
  memory/effort quantities bounded by `Q637`. The parent-local credit is
  **not** charged against the epoch's `bound` — deliberately, since charging it
  would need a decision about which axis a strict credit is, and inventing one
  silently would be a second premise. Discharge condition: an integrator
  decision on the axis mapping, then a deliberate extension of the plan's
  allocation check — in the refusing direction only.
* **`REGION-JOINS-EFFECT-ROW`.** The region's caught error is not required in
  the entry's written `effects { … }` row: a caught error never leaves the
  entry, so the model's declaration/effect rows are derived from the module's
  declared error types, exactly as the sequential route derives them
  (`REGION-EFFECT-ROW` reached from a second route).

And the seam this card narrows but does not close:

* **`REGION-JOINS-T07`** (the remainder of `CT23-T07-SEAM`).
  `CheckedRegionTaskCreditV1` (`Q670`–`Q677`) models spawning **inside** an
  error region — credit lent to a child across a region edge, `Duty::ChildCredit`,
  the `Q673` checkpoint — and still has no source route. This card deliberately
  put the region **after** the joins, so no child is live at the opening and
  the region model alone decides the pack; a region entered with live children
  is exactly T07's territory and stays refused (the after-joins `Q600`).
  Discharge condition: a source route handing spawn/join/credit facts to the
  unedited `region_task_credit` checker, with the emission story
  `REGION-JOINS-DUTIES` names.

## 6. Standing per-change gate

Run at the card tip with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok |
| `cargo test --workspace --no-fail-fast` | see the final gate table in the integrator report; 13 new tests, 0 failed among them |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 115 Lean audits** (114 → 115, `AuditRegionCreditJoins.lean`) |

## 7. CT22 and CT25 readiness — what a wire lane can now claim

**`docs/reevaluation/ct_manifest.json` and `docs/reviews/r0_3/acceptance.json`
are untouched by this card**; mapping is the CT lane's (constraint 23), and a
parallel `CT-WIRE-LANE-2` agent was directed not to map CT22 in its own pass.

**CT22** — *"Quant opening discards parent-local unused credit after joins"*,
expected **Accept** — is now **expressible and accepted end to end**, with
every word of the scenario carried by a checked fact: *quant opening* and
*discards* by the unedited region model's `AbandonCredit` on the discharge of
the opened pack; *parent-local* by the duty-free pack (no acquisition shape)
plus the after-joins IR check (no child holds anything at the opening);
*unused credit* by the published, re-derived `abandoned_credit`; *after joins*
by the derived span rule refused at `Q600` when false. The proving test is
`region_credit_joins.rs::ct22_the_opening_discards_the_parents_unused_credit_after_all_joins`
on `epoch_credit/CT22/case.qcore`. What `QUANT-REGION-SOURCE` §5 recorded as
the narrowed reason — *"after joins is not expressible"* — is retired; a lane
taking the row should publish its own measurements in its own stage-evidence
record. The route decisions CT23's §6 recorded (a `composed_epoch` route or the
`epoch` route with task stages; `ct_corpus.rs`'s pinned tables) apply to CT22
identically, since the case sits on the same route.

**CT25** — *"Cleanup destroys owner after joins and loan expiry"*, expected
**Accept** — is **not** made expressible by this card, and the gap is now
exactly one named residual rather than two: the composed region body admits no
acquisition (`REGION-JOINS-DUTIES`), so no owner can be sealed as a pack duty
for the cleanup to destroy, and read-loan captures remain refused on this route
(`T08-COMPOSE-LOAN`), so *loan expiry* has no referent. A lane should not map
CT25 on this card's evidence.

## 8. What this does not establish

* **No quantity is imported from anything.** The parent's `credit`/`effort`
  figures are written statements, exactly as the sequential route's are; the
  epoch's premises (`DeclaredTaskFootprints` and the rest) are carried
  unchanged on every certificate.
* **The epoch bound is not moved.** The parent-local credit is outside the
  epoch's allocation (`REGION-JOINS-EPOCH-AXIS`), `granted_credits` and `Q637`
  are untouched, and constraint 15j's three do-not-move items are all
  unmoved — `qcore_core::epoch_accounting` and `epoch_credit_recovery` have
  empty diffs for this card.
* **Constraint 15l's two do-not-repair rules are honored.** The
  `RegionOp::QuantOpen` operation variant is still unreachable from a valid
  program (this route also constructs the `QuantOpen` **struct** on the catch
  clause), and the v1 path bound is refused by name from this route too.
* **Rust/Lean correspondence is by inspection.**
* **Nothing is promoted.** No corpus row, CT row, contract count or version
  number moves; the card touches no audit path and no pinned artifact.

## 9. Open, named

| Id | Item |
|---|---|
| `REGION-JOINS-DUTIES` | Acquisitions in the composed region body — a task-route duty vocabulary with a real emission for the discharge; also CT25's gap. |
| `REGION-JOINS-T07` | The narrowed remainder of `CT23-T07-SEAM`: spawning inside a region (children live at the opening) is `CheckedRegionTaskCreditV1`'s discipline and still has no source route. |
| `REGION-JOINS-EPOCH-AXIS` | The parent-local strict credit is not charged against the epoch allocation; charging it needs an axis decision and a refusing-direction extension. |
| `REGION-JOINS-EFFECT-ROW` | The region's error set is derived from declarations, not from the entry's written effects row (`REGION-EFFECT-ROW` reached from a second route). |
| `CT22-CT-MAPPING` | The manifest route, verdict mode and fixture move of §7 — integrator's CT lane, on CT23's §6 terms. |

Items owed to the integrator rather than edited here: `AGENTS.md`'s headline
numbers (Lean audits 114 → 115; workspace test total; the constraints section
if the integrator wants the composed-region rule recorded as a standing
constraint beside 15l), and `acceptance_matrix.json` follow-on reconciliation.
