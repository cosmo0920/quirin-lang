# QUANT-REGION-SOURCE — the first source path to `RegionOp::QuantOpen`

Wave 27, card `QUANT-REGION-SOURCE`. Base: `main` at `f122cc03` (the wave-26
close), in an isolated worktree on branch
`worktree-agent-a8e2448925b04e8c4`.

The card closes CT21/CT22's recorded gap, stated by
`docs/reports/ct-wire-lane/README.md` §3.3 as *"no source path emits
`RegionOp::QuantOpen`; the only mention outside `qcore-core` is a no-op
traversal arm in `crates/qcore-frontend/src/whole_program.rs`."*

**Headline.** `source.region.v1` lands on the flat sequential route:
`error region NAME { … } catch (ERROR) quant open NAME as BINDING { … }`, with
`discharge BINDING;`. It reserves **no word**, allocates **no code**, adds **no
`SourceOp`**, **no CFG terminator** and **no C backend rule**. Every *semantic*
refusal is the unedited `qcore_core::error_region` model's own, reported under
the `Q302`–`Q309` codes that pass has carried since Cycle 6 (plus `Q300` for an
undeclared error type); the reader's own **shape** refusals land on `Q302`, the
pass's generic presentation code, and are each named in §4. An admitted region **erases from emission**: it compiles to
byte-identical C with the hand-written straight-line program it *is* — a region
buys checking, not code.

---

## 1. What a quantitative region is, from the model

This section is read out of `crates/qcore-core/src/error_region.rs` and
`docs/QCORE_SPEC_r0_3.md` §55 before any surface was designed, because the
construct had to be a faithful spelling of an existing operation rather than a
new semantics.

**An error region is a lexical frame.** `RegionOp::ErrorRegion { region, body,
catches }` pushes a `RegionFrame` that records three things: the ledger the
region was entered with (its **baseline**), an ordered list of **cleanup
actions** — one `Destroy` per resource acquired inside the region, plus a
`Rollback` per `Defer` — and a **strict credit remainder**, a `u64` moved by
`AddStrictCredit` and `SpendStrictCredit`. The remainder is the region's
*quantity*.

**Its close is lexical, and the two closes differ.**

| Edge | What the frame becomes |
|---|---|
| normal | the remainder must be **zero** (`UnusedStrictCredit`), and `run_actions` runs the remaining actions in **reverse acquisition order**, releasing each resource and appending `CleanupEvent::Destroy` to the trace |
| error | the frame **seals** into a `CleanupPack { region, error, baseline, actions, credit_remainder }` — a path-specific snapshot, not a merge |

**`quant open` does not discard the pack.** `docs/QCORE_SPEC_r0_3.md` §55:
*"Opening a pack creates a linear cleanup capability; it does not discard the
pack's contents."* `RegionCatchClause::quant_open: Option<QuantOpen>` binds the
sealed pack as an `OpenCapability` under a name, and that capability is
**linear**: `discharge` consumes it, `consumed_bindings` refuses a second
consumption, and a handler that leaves by *any* edge with it still open is
refused. That linearity is the whole content of the word *quant* here — the
capability is a quantity of exactly one.

**The obligations, exhaustively, with the codes they already own:**

| Obligation | Refusal | Code |
|---|---|---|
| an opening is on a catch clause | `QuantOpenOutsideCatch` | `Q302` |
| it opens *this* region's pack | `WrongRegionOpened` | `Q303` |
| one pack is consumed at most once | `CleanupPackDuplicated` | `Q304` |
| a handler does not return with a live pack | `CleanupPackNotDischarged` | `Q305` |
| a **nonempty** pack is not caught without opening it | `CatchWithoutRequiredQuantOpen` | `Q306` |
| an error does not leave a handler with a live pack | `RethrowWithUndischargedCleanup` | `Q307` |
| every cleanup action is total and noexcept | `FallibleCleanup` | `Q308` |
| no resource is in both the payload and a pack | `ResourceInPayloadAndCleanupPack` | `Q309` |
| a sealed resource is not separately dropped | `ResourceHeldByCleanupPack` | `Q302` |
| the quantity does not go negative | `InsufficientCredit` | `Q302` |

**The close condition, in one sentence.** A region closes lexically; on the
error edge the pack it sealed must be *consumed exactly once* by a `discharge`
of the capability a matching `quant open` bound, and that discharge runs the
duties in reverse acquisition order, restores the baseline and **abandons**
exactly the region-local credit remainder.

### 1.1 A correction the model forces on the card's own framing

`RegionOp::QuantOpen` — the **operation variant** — is **unreachable from a
valid program, by design**. Its own doc comment says so: *"Surface `quant open` belongs on
a catch clause. This IR operation is retained so an invalid opening outside a
catch has an explicit `Q302`."* The admitting constructor is the
`error_region::QuantOpen` **struct** in `RegionCatchClause::quant_open`; the
`RegionOp` variant is the refusal.

So "a source path to `RegionOp::QuantOpen`" resolves into two statements, and
this card lands the one that matters: a source path now constructs
`error_region::QuantOpen` and hands it to the model on a catch clause, where
the model *uses* it. The operation variant stays what it was built to be — a
refusal — and the source route reaches its condition through
`WrongRegionOpened` and `CatchWithoutRequiredQuantOpen` rather than by writing
an opening into a body position, which the grammar does not admit. This is
recorded as `REGION-QUANTOPEN-OP-VARIANT`: nothing in source can place the
operation variant in an operation list, and that is the model's design rather
than a gap.

Separately, `qcore_core::region_task_credit` carries its **own** `QuantOpen`
for the `CheckedRegionTaskCreditV1` profile (`Q670`–`Q677`). That one is not
touched here; the two structures are distinct types and this card spells the
`error_region` one.

---

## 2. The construct

```
error region R {
    let outer = qcore_acquire(1, true, Failure);
    let inner = qcore_acquire(2, true, Failure);
    credit 3;
    qcore_check(false, Failure);
} catch (Failure) quant open R as pack {
    discharge pack;
}
```

`compiler/tests/fixtures/region/discharged_pack.qcore`. The body admits five
statements and nothing else:

| Statement | Kernel operation |
|---|---|
| `let NAME = qcore_acquire(ID, true, ERROR);` | `RegionOp::Acquire { resource, destructor_noexcept: true }` |
| `drop NAME;` | `RegionOp::Drop(resource)` |
| `credit N;` | `RegionOp::AddStrictCredit(N)` |
| `effort N;` | `RegionOp::SpendStrictCredit(N)` |
| `qcore_check(false, ERROR);` | `RegionOp::Raise(ERROR)` — the decided error edge |

and the handler admits `discharge BINDING;` → `RegionOp::Discharge(binding)`.

### 2.1 Route

**The sequential route**, as the card recommends — `source.region.v1` is not a
route of its own and selects nothing. A region-carrying module stays
`SequentialCleanupV1`, which is what gives it the existing certificate,
reference execution, C emission and native comparison for free. Every other
route keeps the refusal it already had, measured rather than assumed: a module
that spawns *and* writes a region reaches the task route and keeps its `Q500`
(`the_task_route_keeps_its_profile_refusal`).

### 2.2 Constraint-14 census: zero reserved words, and an exact census

Every word of the form is **already** in
`qcore_frontend::frontend::is_reserved_source_word`: `error`, `region`,
`catch`, `quant`, `open`, `as`, `discharge` (all Cycle 6), and `credit` and
`effort` (card `T08-EPOCH-SRC`). The shape already parsed —
`frontend::error_region_statement` has read `catch (…) quant open … as …`
since Cycle 6.

The census is therefore **exact rather than statistical**. Measured on this
tree: **435** `.qcore`/`.qc` files before this card (437 with its two fixtures,
444 counting the worktree-private probes under gitignored `measure/`) and
**47,358** Rust string literals. Before this
card, `error region R { … }` collected `R` as a referenced name and refused at
`Q013` — measured directly:

```
probe_region.qcore:4:5: Q013: unknown name `R`
probe_region.qcore:9:9: Q013: unknown name `pack`
```

So **no program in this repository could contain the statement at all**, and
the reservation question does not arise. Two name-resolution facts were changed
to make it writable, both narrow:

1. `frontend::lexical_references` returns the empty set for an `ErrorRegion`
   statement. The statement's window is truncated at the body's first `{`
   before this card, so all it ever contributed was the region **label** — a
   name that denotes no binding. This is `LoopAttribute`'s existing rule reached
   by a second statement.
2. `StatementKind::ErrorRegion` gains `catch_bindings: Vec<Option<String>>`,
   parallel to `blocks[1..]`, and `elaborate_block` scopes each clause's
   `quant open … as B` binding to **that clause's block**. The parser records
   only the name the clause *binds*, which is `STD-EVENT`'s and `LOOP-V1`'s
   stated rule — everything else about the clause is read out of the token
   window by `crates/qcore-frontend/src/source_region.rs`, as constraint 15b
   requires.

**The `credit`/`effort` seam with the epoch route is one-way and measured.**
The census re-found `credit N effort M;` inside `epoch { … }` blocks
(`compiler/tests/fixtures/ct/CT35`, `CT36`) and `credit` as a contract
parameter name and reference in
`contract/qcore/language/quantified_indices.qcore`. Neither moves: this card
adds no reservation, and the two directions were probed.

* An epoch module is selected by `qcore_frontend::source_epoch::has_source_epoch`,
  which matches the token pair `epoch` `{` — a spelling no region body admits —
  so a region that credits never reaches the epoch route.
* The epoch spelling written inside a region body is **refused**, not silently
  read as a credit: `parse_region_quantity` reads exactly `WORD INTEGER ;`, so
  `credit 4096 effort 100;` falls through to the body's shape refusal
  (measured, `measure/r_epoch_spelling.qcore`).

The card touches no file of the epoch route.

### 2.3 Zero new codes

Under the standing `Q-NAMESPACE-DECISION` ice box, this card allocates nothing
and extends existing conditions, argued in `docs/QCORE_SPEC_r0_3.md` §L21's
`Q3xx` row and here.

* Every **shape** refusal of the reader is a condition of `Q302`, the
  `ErrorRegion` pass's own generic presentation code.
* Every **model** refusal keeps the exact code that pass already allocates for
  that condition — the table in §1, which is the same allocation
  `qcore_core::region_task_credit::RegionTaskDiagnostic::code` reads back for
  the refusals it carries over from this pass. `Q303`–`Q309` have been written
  into `error_region.rs`'s own `Display` since Cycle 6 and were given a
  machine-readable mapping by card T07's `RegionTaskDiagnostic::code`; this card
  reaches them from a second route, which is §L21's rule that a code is
  allocated to a **condition**. Before this card,
  `region_operations` was empty on every path that reaches
  `whole_program::check_local`, so **no existing diagnostic moves**.
* An undeclared caught error type is `Q300`, the typed-error band's own.

One presentation detail: `ErrorRegionError`'s `Display` writes its own code into
its message text. `source_region::error_region_message` strips that prefix
exactly when the presentation carries the same code, so the model's wording is
carried verbatim and the code is written once.

---

## 3. The emission, and why it is not an approximation

Every outcome on this route is a check-time literal, so an admitted region has
**one** path. The region therefore **erases** into the straight-line program
that path runs, in the existing flat `SourceOp` vocabulary:

| Region | Emitted |
|---|---|
| body acquisitions before the edge | `SourceOp::Acquire { succeeds: true, … }` each |
| body `drop` | `SourceOp::Drop` |
| `credit` / `effort` | **nothing** — an accounting quantity has no runtime object |
| the decided error edge | **nothing** — the edge is *recovered*, and its runtime effect is the handler's |
| `discharge pack;` | `SourceOp::Drop` for each pack duty, in **reverse** order |

This is card `LOOP-UNROLL-BUDGET`'s precedent — its licence erases from
emission too — and it is what buys the whole existing pipeline without a new
`SourceOp`, a new `CfgTerminator`, or a line of backend work.

**The erasure is measured, not asserted**, in three independent ways.

1. `an_admitted_region_emits_what_its_hand_written_twin_emits`: the flagship
   and `hand_written_twin.qcore` produce **byte-identical** C once the `#line`
   directives (which name two different files) are removed, and identical
   reference observations. Published beside this report:
   `artifacts/discharged_pack.c` is the region's whole emitted program, and
   `artifacts/region_vs_twin.diff` is the entire difference from the twin's —
   **one line**, the `#line` directive naming the other file.
2. `the_emitted_releases_are_the_packs_duties_in_reverse`: the model's own
   `RegionExitState::cleanup_trace()` is
   `[Destroy(2), Destroy(1), AbandonCredit(3)]` and the emitted program's trace
   is `[Acquire(1), Acquire(2), Release(2), Release(1)]`. The releases *are*
   the pack's duties in reverse, in order. The credit has no trace event, which
   is precisely why abandoning it is admissible and losing a resource is not.
3. `the_native_run_of_a_discharged_region_matches_its_reference`: the emitted C
   compiled with `-std=c11 -pedantic -Wall -Wextra -Werror -g -O0
   -fsanitize=address,undefined -fno-omit-frame-pointer` and run under
   `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1`, its own instrumentation
   output byte-compared with the reference observation. The leak check is a
   **positive control** for this card: a discharge that lost a duty surfaces as
   an LSan leak, not only as a disagreement.

### 3.1 One refactor, and its control

`FunctionLowering::record_acquisition` and `record_release` were factored out of
`lower_sequence`'s statement match so a region body's acquisition is *the same*
acquisition, in all five checked models (lowering, ownership, typed errors,
resource flow, usage), as one written outside a region. Two copies could drift;
one cannot. `a_program_without_a_region_is_untouched` is the direct control, and
the whole-workspace suite — `branch_statement.rs`, `branch_arms.rs`,
`examples_suite.rs` and every other sequential-route test — is the real one.

---

## 4. The interior-operations boundary decision

The card asked for this decision explicitly. **The interior is not empty**: the
model's quantitative region has admissible interior operations reachable from
source, and all five of them are landed — `Acquire`, `Drop`, `AddStrictCredit`,
`SpendStrictCredit` and `Raise`, which is every `RegionOp` constructor except
`Defer`, `MoveToErrorPayload`, `DropPayload`, `Branch`, the nested
`ErrorRegion` and the `QuantOpen` variant that is a refusal by construction.
So this is **not** the "open+close with an empty interior" fallback the card
allowed for.

What *is* bounded is the set of **paths**, and it is bounded deliberately and
refused by name rather than left silent. The v1 region must end in its decided
error edge and carry exactly one catch clause naming that error. The reason is
the model's path-sensitivity: `analyze_error_regions` calls `handle_catch` only
for an error exit that reaches a matching clause, so a clause on a path the
region does not take is a clause **nothing decides**. Admitting one would be
silent rather than fail-closed (invariant 1). Every boundary is a named
follow-on:

| Refused shape | Follow-on | What it needs |
|---|---|---|
| a region with no error edge | `REGION-NORMAL-EXIT` | deciding an unreached handler — either a multi-path region analysis or a reader rule that stands in for one |
| a second catch clause | `REGION-MULTIPLE-CATCH` | the same |
| a second region in one entry | `REGION-MULTIPLE` | per-region diagnostic attribution; the model reports no position of its own |
| an uncaught raise | `REGION-PROPAGATE` | a second emission shape — a sealed pack leaving the entry |
| a failing acquisition as the edge | `REGION-FAILING-ACQUIRE` | the frame and the emitted slot disagree about which identity exists |
| a `qcore_check(true, …)` inside a region | — | its cleanup edge would leave the region rather than reach its handler; refused rather than emitted as wrong dead code |
| a handler statement other than a discharge | `REGION-HANDLER-BODY` | the handler's own statement vocabulary |
| a region inside a branch arm | `REGION-IN-ARM` | a pack sealed on one path only, and four models that join a branch |
| a catch pattern with a payload binder | `REGION-CATCH-PAYLOAD-BINDING` | the payload vocabulary (`Q300`'s band) |
| a nested region | — | falls out of the body's admitted set; `REGION-NESTED` |

Two further boundaries are recorded rather than refused:

* **`REGION-EFFECT-ROW`.** The sequential route has no `effects { … }` syntax
  and declares `[Alloc]`. A region-carrying entry's `Throws` row is therefore
  **derived** from the module's declared error set rather than written. That is
  a boundary, not a claim; an entry that writes no region carries exactly the
  row it always carried, which is what keeps existing programs byte-identical.
* **`REGION-OUTER-LIVE-RESOURCES`.** A resource acquired *outside* the region
  contributes no `RegionOp`, so the region model is not told about it. That is
  correct — the pack holds only region-acquired duties, and the emitted
  flattening leaves the outer resource alone — but it means the model's ledger
  is a region-local view rather than the entry's.

---

## 5. CT21 and CT22 readiness

The CT manifest is **not** edited by this card. What a mapping lane can now
claim is recorded here and measured by two named workspace tests in
`compiler/tests/quant_region.rs`.

### CT21 — *"Quant opening discards positive access permission"*, expected **Reject**

**Writable, and it refuses.** `ct21_opening_a_pack_and_discarding_it_refuses`:
a region acquires one resource and raises; the clause opens the pack and the
handler walks away.

```
Q305: cleanup pack is not discharged
```

The pack holds one live resource — positive access permission — and the
opening did not discard it, exactly as the spec says. The refusal is
attributable to the construct the case names, with two controls in the test:
remove *only* the opening and the refusal becomes `Q306` (a different rule
about the same pack); add the one `discharge` and the module is admitted and
emits. A lane mapping CT21 would map it on the `sequential` route at the
`sequential_whole_program` stage, with `c_emission` and `native_sanitized`
available on the accepting control.

### CT22 — *"Quant opening discards parent-local unused credit after joins"*, expected **Accept**

**The credit half is writable and accepts;** the *"after joins"* half is not.
`ct22_a_discharge_abandons_the_regions_unused_credit`: the flagship's pack
carries remainder 3, and the discharge's cleanup trace contains
`AbandonCredit(3)` — the credit is discarded **explicitly**, as an event, not by
omission. That is the case's property.

What this route does not have is a join: the sequential route has no tasks and
no `join`, so "parent-local" and "after joins" are not expressible here. That
is the same missing composition `docs/reports/ct-wire-lane/README.md` §3.4
records for CT23 — credits on one route, error paths and joins on another —
and it is named here as **`REGION-CREDIT-WITH-JOINS`**.

So the **archived reasons** are retired for both cases — CT21's *"no source path
emits `RegionOp::QuantOpen`"* and CT22's *"as CT21; also needs credits"*, since
the opening and the credit both exist now — and what remains for CT22 is a
narrower gap in its **scenario** rather than in its recorded reason: *"after
joins"*. A lane taking CT22 should publish that narrower reason in its own
stage-evidence record rather than carrying the archived one (constraint 23).

A lane taking these rows must re-measure them in its own record; this card
publishes the capability and the measurements, not the mapping. `CT21` and
`CT22`'s rows in `docs/reevaluation/ct_manifest.json` are untouched.

---

## 6. Lean

`proof/QCore/QuantRegionSource.lean`, registered in
`proof/AuditQuantRegionSource.lean` and imported by `proof/QCore.lean`.
`QCore.ErrorRegion` already models what the operation *is* and nothing in it
moves; this module proves the three things the **source lowering** rests on.

| Claim | Theorems |
|---|---|
| the pack a source region seals is the frame its body built | `sealed_pack_carries_the_bodys_duties`, `sealed_pack_carries_the_regions_quantity` |
| **open/close pairing soundness** — a handler is admitted exactly when it consumes the capability once | `handler_admits_iff_one_discharge`, `a_handler_that_never_discharges_is_refused` (`Q305`), `a_handler_that_discharges_twice_is_refused` (`Q304`), `one_discharge_is_admitted` |
| **quantity totality** — the remainder is a total function and an overspend is an explicit refusal, never a wrap | `remainder_is_total_on_admitted_bodies`, `overspending_refuses`, `spending_the_whole_credit_leaves_nothing` |
| **faithfulness to the kernel op** — the emitted releases *are* the kernel's own `discharge` cleanup trace | `emit_releases_are_the_packs_duties`, `an_admitted_region_emits_its_flat_twin`, `two_duties_are_released_last_first`, `an_explicit_release_removes_its_duty`, `a_release_erases_an_earlier_duty` |
| a credit is not a resource, which is why one may be abandoned and the other may not | `credit_contributes_no_duty`, `effort_contributes_no_duty` |
| the admitted shapes as one list each | `admitted_body_shapes_are_exactly_these`, `admitted_handler_shapes_are_exactly_these` |

`emit_releases_are_the_packs_duties` is the load-bearing one: it produces the
kernel's own `discharge` result for the pack the region sealed and proves the
emitted release sequence is that result's `cleanupTrace`. The Rust test
`the_emitted_releases_are_the_packs_duties_in_reverse` is the same statement
measured on real emitted C.

Not proved here, and stated: that the Rust reader extracts these statements
faithfully from source tokens (a tested Rust boundary, on the
`ConcurrencyPrelude` / `PipeDesugaring` / `BranchJoin` precedent), and that a C
compiler frees what the emitted program tells it to — which is what the
sanitized native byte comparison cross-checks.

---

## 7. What changed

| File | Change |
|---|---|
| `crates/qcore-frontend/src/source_region.rs` | **new** — the catch-header reader, the `ErrorRegionError` → code map and the message de-prefixer |
| `crates/qcore-frontend/tests/unit/source_region.rs` | **new** — 8 unit tests |
| `crates/qcore-frontend/src/frontend.rs` | `StatementKind::ErrorRegion` gains `catch_bindings`; the parser records them; `lexical_references` returns empty for the statement; `elaborate_block` scopes each clause's binding to its own block |
| `crates/qcore-frontend/src/lib.rs` | registers `source_region` |
| `crates/qcore-frontend/src/whole_program.rs` | `CheckedFunction` gains `region_operations`; the region model's refusal carries its own code |
| `crates/qcore-verification/src/driver.rs` | `record_acquisition`/`record_release` extracted; `lower_error_region`; `parse_region_quantity`; `region_refusal`; the derived `Throws` row; the early model run |
| `compiler/tests/quant_region.rs` | **new** — 29 tests |
| `compiler/tests/fixtures/region/*.qcore` | **new** — the flagship and its hand-written twin |
| `proof/QCore/QuantRegionSource.lean`, `proof/AuditQuantRegionSource.lean`, `proof/QCore.lean` | **new** Lean module and audit |
| `docs/QCORE_SPEC_r0_3.md` | §L21's `Q3xx` row: the finer `Q303`–`Q309` conditions written out, and the two widenings stated |

Not touched: `AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reviews/r0_3/acceptance_matrix.json`, `docs/reevaluation/ct_manifest.json`,
`examples/`, the C gateway, the epoch route, and the buffer/string/aggregate
territories. Corpus-neutral: no gateway, contract, audit or provider file is
edited, so no arbiter run is implied and the corpus rows are unmoved.

---

## 8. Gate

Run at the card tip with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,243 passed**, 2 failed, 8 ignored, over 199 test binaries. Both failures are constraint-16 rows and are re-verified below |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **111 Lean audits** pass (110 → 111; this card's `AuditQuantRegionSource.lean`) |

This card's own rows: `compiler/tests/quant_region.rs` **29 passed** (including
the sanitized native run) and `qcore-frontend`'s `source_region` unit module
**8 passed**.

**Constraint-16 rows, re-verified in isolation.** The two failures are
`qcore-verifier-host`'s `confined_process_execution` binary —
`a_confined_fixture_cannot_write_outside_the_admitted_paths` (constraint 16's
third row, by name) and its sibling
`the_evidence_records_the_achieved_confinement_and_its_boundary`, both failing
with **`Io("Broken pipe (os error 32)")`**, which is exactly the shape that row
records. The machine was carrying **load 88–111 on 16 cores** — three sibling
wave-27 worktrees running their own full suites concurrently. Re-run alone at
load ~19:

```
cargo test -p qcore-verifier-host --test confined_process_execution -- --test-threads=1
test result: ok. 12 passed; 0 failed; 0 ignored
```

Nothing is weakened, and nothing about this card touches the provider host.
(An earlier run of the same tree, before `git submodule update --init`, also
tripped constraint 16's **first** row,
`enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`,
with the same `Io(Broken pipe)` shape; that binary passes 12/12 in the run
recorded above.)

---

## 9. Follow-ons

Named above and repeated here for a lane to pick up:
`REGION-NORMAL-EXIT`, `REGION-MULTIPLE-CATCH`, `REGION-MULTIPLE`,
`REGION-PROPAGATE`, `REGION-FAILING-ACQUIRE`, `REGION-IN-ARM`,
`REGION-HANDLER-BODY`, `REGION-CATCH-PAYLOAD-BINDING`, `REGION-NESTED`,
`REGION-EFFECT-ROW`, `REGION-OUTER-LIVE-RESOURCES`,
`REGION-QUANTOPEN-OP-VARIANT`, `REGION-CREDIT-WITH-JOINS`.
