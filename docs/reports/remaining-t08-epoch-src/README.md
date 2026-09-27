# T08-EPOCH-SRC — a source form for the flat-epoch bound

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Date: 2026-09-09. Baseline: `5589839` (wave-6 consolidation `d91316b` plus two
follow-up commits). Branch: `claude/t08-epoch-src` (isolated worktree).
Workstream W6, lane Source, kind implementation, scope M.

**What landed:** `source.epoch.bound.v1` — a lexical
`thread scope NAME epoch { … }` declaration
(`crates/qcore-frontend/src/source_epoch.rs`) that lowers to a complete
`FlatEpochProgram` and hands it to the **unedited** `FlatEpochBoundV1` checker
(`qcore_core::epoch_accounting::check_flat_epoch`), plus the driver route that
publishes and independently rechecks the resulting certificate. 25 new Rust
tests, 37 audited Lean theorems in a new `QCore.EpochSourceLowering` namespace.
**Nothing is promoted.** No diagnostic code is added, moved or downgraded; no
`qcore-schemas` change; no CT manifest edit; no C emission and no native run.

T08 recorded "no source route" as its first named boundary
(`docs/reports/remaining-t08/README.md` §7) and asked the integrator **not** to
map CT35/CT36/CT38/CT39 until epoch source syntax landed. This card lands that
syntax and supplies the measured stage rows that mapping needs; the manifest
edit itself stays with the CT lane.

## 1. The grammar, and why it is this shape

The four CT scenarios drove the surface, and nothing beyond them plus honest
negative controls was built:

| Case | Scenario | What the source must be able to say |
|---|---|---|
| CT35 | one shared 4096-byte allocation read by two tasks | a declared shared cell, and two workers naming it |
| CT36 | concurrent incremental peaks 3072 and 5120 | a per-worker private peak, and a declared runtime overhead to add to it |
| CT38 | child credits splitting a total to more than the total | a per-worker credit and a whole-epoch allocation, on both axes |
| CT39 | a concurrent work bound read as a wall-clock bound | a per-worker work figure, and a way to *ask* for it as a time |

```ebnf
epoch_module = "import", "std.concurrency", ";", entry ;
entry        = "fn", NAME, "(", ")", "->", "unit", "{", epoch_scope, "}" ;
epoch_scope  = "thread", "scope", NAME, "epoch", "{", { header | worker }, "}" ;

header       = "let", NAME, "=", "share", NAT, ";"                          (* one shared cell *)
             | "bound", NAT, "effort", NAT, ";"                             (* the epoch allocation *)
             | "overhead", "scope", NAT, "thread", NAT, [ "measured", NAT ], ";"
             | "bound", "effort", "as", "latency", ";" ;

worker       = "thread", "scope", NAME, "{", { clause }, "}" ;
clause       = "read", NAME, ";" | "write", NAME, ";"
             | "peak", NAT, ";" | "effort", NAT, ";"
             | "credit", NAT, "effort", NAT, ";" | "retained", NAT, ";"
             | "join", "(", ( "move" | "drop" | "close" ), ")", ";"
             | worker ;
```

Two axes: `memory` carries every byte figure, `effort` carries T08's work
quantity (the certificate's `work_total`). `allocation` and `overhead` appear
exactly once per epoch; `credit` exactly once per worker; `peak`, `effort`,
`retained` and the join clause at most once each. Every `NAT` is read at `u64`;
an out-of-range literal is refused, never wrapped.

Three shape decisions are worth stating because they were forced, not chosen:

* **The join clause is the creator's act, written in the child's block.** A
  worker's `thread scope` name binds only inside its own body (the W6
  frontend-gate rule), so a creator-level statement cannot name a worker at
  all. Putting `join(move|drop|close);` in the child's block is the only
  spelling that keeps the disposal decidable without weakening that gate.
* **The parentheses on `join` are load-bearing.** The scoped-task profile's
  expression scanner claims any `join` **not** followed by `(`, and then
  demands `join move PLACE`, so a bare `join move;` would be a parse error from
  another profile's grammar. `join(...)` is skipped by that scanner by its own
  rule.
* **`effort`, not `work`.** See §5: reserving `work` breaks existing source.

`join(close)` is the representable claim that a retained result stopped being
live at completion with no transfer and no release. It exists so the core can
refuse it (`Q634`) rather than the case being one nobody checks — the same
reason `ResultDisposal::DroppedAtCompletion` exists in the model.

`bound effort as latency;` is a *request*. The lowering records it; the driver
derives and rechecks the bound and only then puts the claim to the profile's
own `epoch_wall_clock_bound`, which always refuses. The epoch may be perfectly
well accounted and the time claim still not follow from it — which is exactly
what `Q639` says.

## 2. Where the work is done, and where it is not

| Component | Responsibility |
|---|---|
| `crates/qcore-frontend/src/source_epoch.rs` (new, 789 lines) | `has_source_epoch` route detection, `lower_source_epoch`, `LoweredEpochModule::locate` |
| `crates/qcore-verification/src/driver.rs` | `compile_epoch_source`: route dispatch, `check_flat_epoch`, `recheck_epoch_bound`, the latency claim, `CheckedSourceModule::CheckedEpochBound`, `SourceExecutionProfile::FlatEpochBoundV1` |
| `crates/qcore-core/src/epoch_accounting.rs` | **untouched** |
| `crates/qcore-core/src/prelude.rs` | additive: `epoch_accounting` and its four entry points join the concurrency prelude |
| `crates/qcore-frontend/src/frontend.rs` | additive: eight reserved words |
| `crates/qcorec/src/main.rs` | one profile label on the existing non-exhaustive match |

Route order: the epoch marker is checked **before** the mutex and task routes,
because an epoch scope is also a `thread scope` and a module declaring a
quantitative epoch must not be handed to a route that models no accounting. A
module without the `epoch` marker keeps its existing route, and a test pins
that.

The driver never publishes a certificate it did not re-derive: every accepted
module goes through `recheck_epoch_bound` before `CompiledModule` is built, and
a recheck failure is `Q600`, not a downgrade.

**This route is check-only.** An admitted epoch is a checked accounting
statement about declared shapes, so `c_source` and `reference_observation` are
`None` by construction and a test asserts both. That matches the model card:
`FlatEpochProgram` declares peaks, results, work, credits and cell footprints,
and nothing here derives any of them from source, from an allocator or from a
run. There is nothing to execute, so no reference execution, no C emission, no
native run and no sanitizer fixture is claimed — the card's allowance for
emission is explicitly declined on those grounds, not skipped.

## 3. What the lowering refuses, and who refuses it

The T03 discipline is kept: shapes the *profile* models as refusals become real
program content, so the **core** issues the refusal from its own band; shapes
the *grammar* cannot represent are refused in the frontend at `Q600`.

| Source | Becomes | Code | Measured cascade |
|---|---|---|---|
| `write page;` | `UnsupportedEpochShape::SharedMutation` | `Q632` | `["Q632"]` |
| nested `thread scope` in a worker | `UnsupportedEpochShape::NestedTask` | `Q632` | `["Q632"]` |
| `read NAME;` where NAME is not a declared cell | a cell identity the storage list does not contain | `Q631` | `["Q631"]` |
| `read page;` twice in one worker | a repeated shared entry, not deduplicated | `Q630` | `["Q630"]` |
| `retained 0;` | an empty retained result | `Q630` | `["Q630"]` |
| an epoch with no workers | `EmptyEpoch` | `Q630` | `["Q630"]` |
| `retained 256;` with `peak 128;` | result beyond the peak it was accounted for | `Q633` | `["Q633"]` |
| `join(close);` with a retained result | `DroppedAtCompletion` | `Q634` | `["Q634"]` |
| a retained result and no join clause | `UntransferredResultAtEpochExit` | `Q635` | `["Q635"]` |
| no result and no join clause | `UnjoinedTask` | `Q635` | `["Q635"]` |
| a demand beyond the worker's own credit (either axis) | `TaskExceedsItsCredit` | `Q636` | `["Q636"]` |
| credits summing beyond the allocation | `CreditsExceedAllocation` | `Q637` | `["Q637"]` |
| an epoch beyond its allocation with every credit fitting | `EpochExceedsAllocation` | `Q638` | `["Q638"]` |
| `bound effort as latency;` | a wall-clock request on a derived certificate | `Q639` | `["Q639"]` |
| `overhead scope 512 thread 128 measured 30;` | `MeasuredNativeRun { runs: 30 }` | `Q640` | `["Q640"]` |
| two `peak u64::MAX` workers | checked overflow, never wrapped | `Q641` | `["Q641"]` |

Refused by the grammar at `Q600`, each with its own message: a clause word this
profile does not have, a worker with no credit clause, two clauses of the same
kind in one worker, two allocations, `join(close)` with nothing retained, an
epoch scope that is not the whole entry body, a second declaration, a
`spawn`/`join` task expression, a missing allocation header and a missing
overhead header. A missing `#import std.concurrency;` is `Q015` from the
existing import gate, before any of this.

Four refusals of the core's band are **unreachable from this source form** and
are recorded rather than claimed: `DuplicateTask` and `DuplicateSharedCell`
(the lowering mints fresh identities), `TaskCountMismatch` (the declared count
is the worker count by construction) and `PhaseVectorMismatch` (no source form
queries a residency configuration).

Diagnostics are attributed: a refusal naming a task is located at that worker's
`thread scope` header, one naming a cell at that cell's declaration or its
`read` statement, the wall-clock refusal at the claim, and anything else at the
epoch scope. A test measures both the worker-located and the scope-located
case.

## 4. Evidence

**Rust — 25 new tests.**

`crates/qcore-frontend/tests/source_epoch.rs` (9), on the lowering itself:

| Test | Covers |
|---|---|
| `every_declared_figure_arrives_in_its_own_slot_unchanged` | full transcription; the axes never mix; the lowered program is the one the core admits |
| `the_join_clause_is_the_disposal_and_its_absence_is_an_unjoined_task` | all three dispositions; the unjoined task and its `Q635` |
| `a_repeated_shared_entry_reaches_the_core_as_a_repeat` | no deduplication; `Q630` |
| `unmodeled_shapes_become_represented_refusals_not_missing_content` | `SharedMutation`, `NestedTask`, the foreign cell identity |
| `the_measured_overhead_suffix_reaches_the_core_as_a_measurement` | `MeasuredNativeRun { runs: 30 }` |
| `a_refusal_is_attributed_to_the_statement_that_caused_it` | worker-located `Q633`; scope-located `Q630` |
| `the_latency_claim_is_recorded_and_never_granted_here` | the claim is carried, and changes no lowered quantity |
| `forms_the_grammar_cannot_represent_are_refused_here` | eight `Q600` shapes |
| `route_detection_is_syntactic_and_does_not_claim_other_modules` | detection selects only epoch modules |

`compiler/tests/epoch_source.rs` (16), end to end through `compile_files`:
the four CT cases with their controls (§6), the whole `Q630`–`Q641` band
reached from source, the `Q600` grammar refusals, the `Q015` import gate and
the route boundary against the mutex profile. Every refusal test asserts the
**exact ordered code vector**, not `is_err()`.

**Lean — 37 audited theorems**, `proof/QCore/EpochSourceLowering.lean`, audited
by `proof/AuditEpochSourceLowering.lean` (39 audits total, up from 38). All
depend only on `propext` and `Quot.sound`; **no new axiom, no `sorry`, no
placeholder.**

The module models the declaration form, the admission predicate and the
lowering, and proves what the lowering is responsible for:

* *Transcription* — the worker count, each peak, effort and credit, the
  declared overhead and its scaling, and that a peak carries no effort and
  effort carries no bytes (7 theorems).
* *The accounting rules survive lowering* — two readers of one declared cell
  lower to one footprint, an unread cell contributes nothing, a second reader
  does not grow the entry allocation, two declared peaks lower to their sum,
  and the componentwise maximum of that same declaration is strictly smaller
  (6 theorems).
* *Admission implies the model bound* — an admitted declaration lowers to a
  **wellformed** epoch, so every configuration of it is bounded by the checked
  bound and the all-running configuration attains it (5 theorems). This is the
  payoff: source-level admission is tied to `EpochAccounting`'s soundness
  claim rather than restated beside it.
* *Refusals* — over-retention, `join(close)`, a missing join, a missing credit,
  a repeated entry, an undeclared entry, a shared write, a nested worker, a
  measured basis and an empty epoch are each **not admitted** (10 theorems, by
  `decide` on concrete declarations).
* *Credits and the latency claim* — the credit total, CT38's refusal through
  `credits_beyond_the_allocation_are_not_covered`, that the latency flag
  changes no lowered quantity, and that no function of the work total is a
  finish time (4 theorems).

Not a vacuous audit: two theorems assert concrete declarations **are** admitted
(`the_two_reader_declaration_is_admitted`,
`the_two_stage_declaration_is_admitted`), so the refusal family is not proved
against a predicate that refuses everything.

The Lean transcription of the Rust lowering is **by inspection**, exactly as in
`EpochAccounting`; there is no digest-checked correspondence of the F01 kind.

## 5. Reserved-word audit

Eight words are reserved: `epoch`, `overhead`, `peak`, `effort`, `credit`,
`retained`, `latency`, `measured`. Ten more clause words are **reused** from
the existing reserved set and add no hazard: `share`, `bound`, `scope`,
`thread`, `read`, `write`, `join`, `move`, `drop`, `close`, `as`, `let`.

Reservation matters because `lexical_references` skips reserved words (so a
clause keyword is not resolved as a name) and `frontend_tasks::identifier`
**refuses** a reserved word as a spawn/join place. The second rule is what
makes a badly chosen keyword break existing source.

*Method.* (a) a scan of every file that can carry source-language text —
`.qcore`/`.qc` fixtures and every `.rs`/`.py`/`.el`/`.md` file holding an
inline source program — for the eight words inside a source-ish line
(267 occurrences, classified below); (b) the whole workspace test suite, which
executes every fixture and inline program in the repository.

*Result — one real collision, found and avoided.* The card originally used
`work` as the effort keyword. `crates/qcore-frontend/tests/unit/frontend.rs`'s
pinned **golden surface** declares `fn work[file_count: Idx<Count<File>>](…)`
and spawns it as `let task = spawn s work(copy shared);`, and reserving `work`
made two golden-surface tests fail with `expected a place or task identifier`.
That is precisely the hazard T03-F recorded. **The keyword was renamed to
`effort` rather than the fixture being edited**: a pinned complete-surface
fixture is intentional, and a keyword that stops existing source parsing is a
compatibility defect, not a fixture problem. `effort` is documented in the
module header as carrying T08's work quantity, and the axis is named `effort`
too so source, diagnostics and certificate share one vocabulary.

*Result — one near miss, recorded.* The same file declares
`extern fn measured() -> quant<…>;`. A *declaration* name is parsed by the
declaration parser, not by statement-level reference resolution, so reserving
`measured` does not affect it and the test passes. A source that **called**
`measured()` would be a different matter; none does.

*Result — the remaining hits are all `credit<…>` in the quant-axis declaration
grammar* (`quant axis memory : credit<Bytes>;`) across `contract/qcore`, the
V04/D01 probe fixtures, the parser/check unit fixtures and the Emacs mode
tests. That grammar is parsed by `quant_index`, which matches the word
positionally and never consults the reserved list, so those are unaffected —
confirmed by the full suite.

*Not updated:* `editors/emacs/qcore-mode.el` does not highlight the eight new
words. Highlighting is lexical editor support and no test cross-checks it
against the reserved list; recorded as an open item, not a silent omission.

## 6. Proposed CT mappings — CT35, CT36, CT38, CT39

**The manifest is not edited by this card.** `docs/reevaluation/ct_manifest.json`
and `docs/reviews/r0_3/acceptance.json` are untouched, CT coverage stays
**24/50**, and the mapping is the integrator's CT lane's to make. The measured
schema-2 rows are in `ct_stage_evidence.json` beside this file, in the shape of
`docs/reports/wave5-ct-integration/ct_stage_evidence.json`.

Summary of what is measured, on the case's own fixture, through `compile_files`:

| Case | Verdict | Measured |
|---|---|---|
| CT35 | accepted | `shared_entry_union.memory = 4096` for two readers of one cell; `epoch_peak.memory = 6912`. Controls: two distinct cells measure 8192; a repeated entry refuses `["Q630"]` |
| CT36 | accepted | `concurrent_private_peak.memory = 8192`, `runtime_overhead.memory = 768`, `epoch_peak.memory = 8960`. Controls straddle the summed figure: 8192 admitted, 8191 refused `["Q637", "Q638"]` |
| CT38 | refused | exact cascade `["Q637"]`; control splitting the same total within it is admitted |
| CT39 | refused | exact cascade `["Q639"]`; control without the claim is admitted with `work_total.effort = 800` and `NoScheduleModel` on the certificate |

Three things the CT lane must do before it can map these, each recorded in the
JSON rather than assumed:

1. **The `epoch` route does not exist in the manifest vocabulary.** It needs
   `epoch_lowering` and `epoch_bound_check` added to `stage_vocabulary.stages`
   (beside the existing per-profile pairs) and a route
   `epoch = [parse, elaborate, epoch_lowering, epoch_bound_check, proof]`, with
   the same names added to `ct_corpus.rs`'s pinned tables — the harness
   cross-checks its tables against the manifest and refuses drift.
2. **CT35 and CT36 are accepts whose evidence is exact certificate figures.**
   `reject` covers CT38/CT39; an `epoch_bound_figures` verdict mode is needed,
   or the mapping claims less than this card measured.
3. **The fixtures are parked outside `tests/fixtures/ct/`.** They live in
   `compiler/tests/fixtures/epoch/CT3x/` because the harness fails closed when
   an unmapped case already owns a fixture directory. Mapping moves them.

`native_sanitized` and `proof` stay `unassessed` for all four, and every stage
of another route is `not_applicable`: this route has no emission and no
execution stage at all. **No native or proof evidence is added by this card** —
sanitized-native would stay 3 of 28 and proof 0 of 28 — so if the lane accepts
these rows, the grown denominator is mapping, not conformance.

## 7. Standing per-change gate

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok |
| `cargo test --workspace` | **1677 passed, 0 failed, 1 ignored** (25 new; the baseline ledger records 1651 at `d91316b`, and this worktree is two commits ahead of it) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0, `failed_expectations: 0` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0, `failed_expectations: 0` |
| `python3 -B scripts/check_lean_audits.py` | **Passed 39 Lean audits** (38 before) |
| `python3 -B -m unittest discover -s scripts/tests` | 108 tests, OK |
| `cargo test` in `providers/` | 58 passed, 0 failed |

One flake observed and re-run in isolation, as the card brief anticipated: the
providers' `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` failed once
under a parallel workspace run and passed on a clean re-run (58/58). Nothing in
this card touches the provider host; the assertion was not weakened.

## 8. What this does not establish

* **No quantity here is imported from anything.** Peaks, results, work,
  credits, cell footprints and overhead are declared figures, and
  `FlatEpochBoundV1`'s four premises (`DeclaredTaskFootprints`,
  `DeclaredRuntimeOverhead`, `DisjointSharedStorageCells`, `NoScheduleModel`)
  are carried unchanged on every certificate this route publishes. A source
  form for declaring them is not a derivation of them.
* **No execution, no emission, no measurement.** The route is check-only by
  construction. A future emission increment would have to say what an epoch
  declaration *runs*, which this profile deliberately does not answer.
* **No schedule, in either direction.** `epoch_wall_clock_bound` still always
  refuses; `bound effort as latency;` only makes the refusal reachable from
  source.
* **The epoch is flat and its workers are declarations, not bodies.** A worker
  block declares quantities; it has no statements, no captures, no owned
  values and no completion capsule. Composition with `ScopedOwnedTasksV1`'s
  real workers, with `ScopedReadSharesV1` shares, with T07's region credit or
  with T09's footprints is untouched and unvalidated.
* **Rust/Lean correspondence is by inspection**, and the Lean admission
  predicate is a transcription of the Rust one, not an extraction of it.
* **Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; the contract corpus stays 60
  modules; CT coverage stays 24/50 mapped; `qcore-schemas` stays 6.0.0. This
  card adds one frontend module, one driver route and eight reserved words, and
  touches no audit path.

## 9. Open, named

| Id | Item |
|---|---|
| `T08-SRC-CT` | The four CT mappings, with the two stage names, the `epoch` route, the `epoch_bound_figures` verdict mode and the fixture move (§6). Integrator's CT lane. |
| `T08-SRC-EMACS` | `editors/emacs/qcore-mode.el` does not highlight the eight new reserved words. |
| `T08-SRC-UNREACHABLE` | `DuplicateTask`, `DuplicateSharedCell`, `TaskCountMismatch` and `PhaseVectorMismatch` are unreachable from this source form; a residency-query source form would be needed for the fourth. |
| `T08-SRC-COMPOSE` | An epoch declaration composed with a real task, share, guard or region-credit module. Every such combination is currently outside every checked profile. |
| `T08-SRC-DERIVE` | Deriving any epoch quantity — a peak, a result, an overhead — from source or from a measured run, instead of declaring it. `Q640` deliberately refuses the measured substitution today. |
