# W6 CT corpus, batch 2: full 47-case triage and two new mappings (2026-09-06)

Continues [`docs/reports/w6-ct-harness/README.md`](../w6-ct-harness/README.md)
(`docs/GENERALIZATION_PLAN.md` §W6 task 2). The harness itself is unchanged in
design; this record adds the **complete triage of all 47 previously unmapped
spec §55 cases** so future batches are mechanical, and moves the two cases that
already-complete functionality genuinely decides at source level.

**Result: 5 of 50 mapped, 45 unmapped** (was 3/47). No checker, loader or
provider code was modified.

## What the source pipeline can actually decide today

The harness runs each fixture through `compile_files` (reevaluation §5.1: no
hand-constructed Rust events). That pins the mappable surface to the **Cycle 12
executable subset** in `compiler/src/driver.rs::synthesize_function`, whose
entire statement vocabulary is:

| Form | Meaning |
| --- | --- |
| `let x = qcore_acquire(<id>, true\|false, ErrorType);` | owned acquisition that succeeds or fails |
| `qcore_check(true\|false, ErrorType);` | fallible operation leaving the ledger unchanged |
| `drop x;` | release |
| `move x;` | transfer ownership out of the frame (to the result) |
| `f();` | zero-argument call of an observationally pure source function |
| `return <i32 literal>;` | normal exit |

Every other statement — and **any nested block at all** — is refused with
`Q500 "structured control flow is not in the Cycle 12 executable subset"`.
Function parameters, usage grades, `own`/`borrow` modes and library/contract
module modes are checked, but no source form reaches shares, footprints,
credits, quant opening, error regions, mutexes, thread scopes, tasks or the C
gateway.

From that vocabulary the pipeline decides exactly seven distinct outcomes:

1. `Q202 ResourceNotLive(n)` — use after drop/transfer. *(CT02, mapped)*
2. Failed acquisition ⇒ declared error with an **empty** trace. *(CT12, mapped)*
3. Failing check after acquisition ⇒ cleanup + declared error. *(CT16, mapped)*
4. `Q202 FallibleOperationAfterTransfer` — an error edge while an obligation
   that left the frame is still outstanding. *(CT19, mapped here)*
5. Transfer + normal exit ⇒ identity carried in the completion record payload
   rather than released. *(CT37, mapped here)*
6. `Q202 LiveResourcesAtNormalReturn([…])` — normal exit with an undischarged
   local. *(no §55 row is decided by this alone; see CT13/CT44 below)*
7. `Q202 DuplicateResource(n)` — any reuse of a numeric identity, even after
   release. *(the CT50 lesson; not a case decision)*

Items 1–5 are now all consumed. **The subset has no further undecided §55
content**; the remaining 45 rows need work recorded in the triage table.

## Cases mapped in this batch (2)

| Case | Spec expectation | Fixture behavior | Result |
| --- | --- | --- | --- |
| CT19 | Parent `?` bypasses a live sibling task → Reject | The sibling obligation is `move`d out of the parent frame — outside the cleanup pack, still undischarged — and the parent's error exit is a failing `qcore_check`. The pack itself is empty and would be valid. | Rejected: `Q202 FallibleOperationAfterTransfer` |
| CT37 | Child returns allocation and finishes early → Allocation remains live in completion capsule | One frame-local scratch identity plus one allocation handed to the result, then an early `return 5;`. | Accepted; reference execution: `Ok value=5`, completion payload `[6]`, trace `acquire:5, acquire:6, transfer:6, release:5` |

CT19 matches §43.7 exactly — *"A `?` that would bypass a live sibling handle is
rejected even if the usual sequential cleanup pack would otherwise be valid."*
The fixture reproduces that shape: an obligation outside the pack, a valid
(empty) pack, and a refused error edge.

CT37 matches §43.5 — *"Resource identities in a capsule remain globally live,
even after worker execution stops."* The reference execution shows identity 6
leaving as a **transfer** with no matching release and still present in the
completion record, while the frame's own identity 5 is reclaimed. The contrast
between the two identities in one observation is what makes this a decision
rather than a restatement of CT02.

Per-stage manifest statuses: CT19 marks `parser`/`type_checker` executed (it is
a static rejection, like CT02); CT37 additionally marks
`reference_execution`/`cfg`. `native_backend` and `proof` stay `unassessed` for
both — neither is exercised.

### One harness change

`ExpectedOutcome` in `compiler/tests/ct_corpus.rs` gains an optional `payload`
field, and a normal-exit (`ok_value`) case must now pin the completion-record
identities; an error-exit case must not carry one. This is a fail-closed
addition: without it CT37 could claim a normal exit without stating what the
capsule still holds, which is the whole property under test. Existing fixtures
(CT12/CT16 both exit with errors) are unaffected.

## Full triage of the 47 previously unmapped cases

Buckets are the four named in the W6 task: **decidable-now**,
**blocked-on-W3/W4** (the case needs the obligation/verdict-composition or
resource-accounting vocabulary those workstreams land),
**blocked-on-unimplemented-syntax** (no source or harness form reaches a
checker at all), and **ambiguous** (the case's distinguishing content is
absent, or its expected result is not an accept/reject verdict).

Counts: **decidable-now 2** (both mapped), **blocked-on-W3/W4 8**,
**blocked-on-unimplemented-syntax 33**, **ambiguous 4** — 47 total.

| Case | Bucket | Reason |
| --- | --- | --- |
| CT01 | unimplemented-syntax | Needs ownership to *return* to the caller. `ParameterOwnership::Returned` exists in `whole_program.rs` but the driver only synthesizes `Consumed`/`Borrowed`, and executable calls must take zero arguments. No source form round-trips an identity. |
| CT03 | unimplemented-syntax | Fractional/positive shares (`quant<access, …>`) have no executable source form; the subset's only footprint is a fixed `("resources", 1)`. |
| CT04 | unimplemented-syntax | Same as CT03; also needs a parent/child interference model. |
| CT05 | unimplemented-syntax | Same as CT03. A `move`-twice fixture would reject with `ResourceNotLive`, i.e. CT02's mechanism, not permission exclusivity. |
| CT06 | ambiguous | Expected result ("Still same permission identity") is not an accept/reject verdict, and the subset has no aliasing or copy binding: every `let` must be a fresh `qcore_acquire`, so two names for one identity are unwritable. |
| CT07 | unimplemented-syntax | No array types, index/bounds reasoning or disjointness proof reaches the executable subset. |
| CT08 | unimplemented-syntax | Same as CT07; also wants an `Unknown` verdict the harness does not have. |
| CT09 | unimplemented-syntax | No struct/field layout access in the subset. |
| CT10 | unimplemented-syntax | No bit-fields; the C-path importer rejects bit-field access outright, and there is no C twin stage. |
| CT11 | unimplemented-syntax | Needs share reconstruction with surviving local aliases; no share or alias form exists. |
| CT13 | unimplemented-syntax | Reaffirmed from batch 1 with a sharper reason: the natural fixture (successful acquire, then a *failing* acquire) is **Accepted**, because the executable subset's implicit cleanup pack silently releases the first obligation. That is precisely the auto-discharge §43.4/§43.7 forbid for task handles ("The checker does not silently detach, cancel, or auto-join"). Mapping it would record the wrong answer. |
| CT14 | ambiguous | "No restored parent authority" has no fixture distinct from CT02 (post-relinquish use → reject) or CT37 (identity stays outside the frame). "Child finishes" is not representable, so the distinguishing content is absent. |
| CT15 | unimplemented-syntax | Backend join failure needs the native concurrency backend; nothing in the source pipeline models a join, let alone a failing one. |
| CT17 | blocked-on-W3/W4 | `error_region.rs` already models `RegionOp::MoveToErrorPayload`, but `driver.rs` always passes `region_operations: Vec::new()`, and `ExecutionOutcome::Error` carries no payload. Requires driver work (a checker change) — out of scope for this batch. |
| CT18 | blocked-on-W3/W4 | Same as CT17, plus a borrow form. |
| CT19 | **decidable-now** | Mapped. See above. |
| CT20 | blocked-on-W3/W4 | `error region … catch` parses, but any nested block is refused by the executable subset, so `analyze_error_regions` is never fed from source. |
| CT21 | blocked-on-W3/W4 | `RegionOp::QuantOpen` exists; no source path emits region operations. |
| CT22 | blocked-on-W3/W4 | Same as CT21; also needs credits. |
| CT23 | unimplemented-syntax | Quantitative credits are not in the executable subset. The resource-shaped analogue collapses onto CT19's fixture and mechanism. |
| CT24 | unimplemented-syntax | Cleanup order is reverse-acquisition **by construction** in `lower_cleanup_cfg`, so a dependency-order violation cannot be written; and there is no borrow/join to be outstanding. |
| CT25 | ambiguous | Its distinguishing content versus CT16 is exactly "after joins and loan expiry", which the subset lacks. A fixture would restate CT16's mechanism with an extra nested acquisition. |
| CT26–CT34 | unimplemented-syntax | All mutex rows. `with lock m as g { … }` cannot reach a checker from source at all: `statement_metadata` records no binding for the guard, so `g` always fails name resolution (`Q013`), and when the names do resolve the statement is refused as structured control flow (`Q500`). `concurrency.rs` is unreachable from `compile_files`. Its documented bound (initially-unlocked non-recursive mutexes) is therefore not the limiting factor — source elaboration is. |
| CT35 | blocked-on-W3/W4 | Needs multiple consumers of one identity (no `read`/share form) and byte-valued quantities; the subset counts unit resources. |
| CT36 | blocked-on-W3/W4 | The peak arithmetic is real (`Flow::then` gives peak 2 for two simultaneously-live acquisitions, 1 if released between), but the subset has no conditional, so sum-vs-max is not contrasted, there are no byte quantities, and `expected.json` cannot pin a bound. Needs the reevaluation §5.2 resource-accounting oracle. |
| CT37 | **decidable-now** | Mapped. See above. |
| CT38 | unimplemented-syntax | Credit partitions are not in the subset; an over-release fixture reduces to CT02's `ResourceNotLive`. |
| CT39 | blocked-on-W3/W4 | `contract { quant normal …; }` clauses **parse and are then silently ignored** — `check_whole_program` never reads `function_contract`. Rejecting an unsupported claim needs the W3 obligation vocabulary. (Also noted below as an incidental finding.) |
| CT40 | unimplemented-syntax | C-path row (`Unknown` expected). The harness has no C-twin convention: it needs a legacy-C stage in the manifest schema (which all 50 rows share), an `unknown` verdict vocabulary, and Clang gating. |
| CT41 | unimplemented-syntax | Same as CT40. The gateway plausibly already decides it — `typed_source.rs::c_type` refuses `volatile`/`_Atomic`/`restrict` with "qualified access requires additional synchronization/alias evidence" — but no harness path can run it. |
| CT42 | unimplemented-syntax | Same as CT40. |
| CT43 | unimplemented-syntax | Same as CT40. |
| CT44 | ambiguous | The available rejection (`LiveResourcesAtNormalReturn`) fires because the frame would *leak* its own resource, not because an external dependent still needs it. Different reason from the case's. |
| CT45 | unimplemented-syntax | Same as CT40. |
| CT46 | unimplemented-syntax | Needs `Send`/`Sync` conditions and a verified mutex protocol; neither reaches source. |
| CT47 | unimplemented-syntax | A `ptr<T>` parameter yields no ownership mode at all (`filter_map` keeps only `own`/`borrow`), and there is no cross-thread boundary to reject. |
| CT48 | unimplemented-syntax | Needs mutex release/reacquire and heap refinements. |
| CT49 | unimplemented-syntax | The driver validates an **exact** total observation (`interpret_source` vs `interpret_cfg`), which is sound only because the sequential subset's order is total. Independent-event partial order is not modeled. |
| CT50 | unimplemented-syntax | Reaffirmed from batch 1, now demonstrated: reusing identity 1 after `drop` yields `Q202 DuplicateResource(1)`. The ledger forbids reuse outright, which is fail-closed but decides nothing about whether an old capability can authorize a new identity. |

### What unblocks the largest groups

- **A source-level driver for structured statements** (nested blocks, `error
  region`/`catch`, `with lock`, `thread scope`) unblocks CT20–CT22 and CT26–CT34
  — 12 rows — because `error_region.rs` and `concurrency.rs` already exist and
  are simply unreachable from `compile_files`. The guard-binding gap in
  `statement_metadata` (a `with lock … as g` guard is never bound, so `g` cannot
  resolve) must be fixed first.
- **A C-twin stage in the harness** (new manifest stage name, `unknown` verdict,
  Clang gating) unblocks CT40–CT43 and CT45 — 5 rows — against the existing
  fail-closed gateway. This changes the manifest stage schema for all 50 cases,
  so it wants its own batch.
- **W3 obligations** unblock CT39 and give CT17/CT18/CT21/CT22 their vocabulary.

## Incidental findings (not repaired here)

1. Function `contract { … }` clauses parse into `SourceFunctionContract` and are
   never consumed by `check_whole_program`. A `quant normal wall_clock <= 5;`
   clause compiles clean. This is a silent-acceptance path, not a fail-closed
   one, and is the direct blocker for CT39.
2. `with lock <m> as <g> { … }` and `thread scope <s> { … }` can never pass name
   resolution: `frontend.rs::statement_metadata` returns no binding for the
   guard/scope name, so the name is reported `Q013 unknown name`. Both forms are
   therefore dead at the source level regardless of the executable subset.

Neither is weakened or worked around by this batch; both are recorded so a
future workstream can decide them deliberately.

## What this batch does not establish

- The two new fixtures exercise the **sequential** ownership/completion-record
  core only. Tasks, joins, captures, shares, credits, mutexes, error payloads
  and the concurrency backend remain unexercised; each `semantic_profile`
  scopes this out explicitly.
- The triage above states what *this* pipeline decides today. It is not a claim
  that the remaining 45 cases are undecidable, only that deciding them requires
  the changes named per row.
- 45 cases remain `unmapped`, counted by `ct_mapping_progress_is_pinned_and_counted`.

## Standing gate (all run at this record's commit)

| Command | Exit |
| --- | --- |
| `cargo test --workspace` | 0 |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 |
| `cd proof && lake build` | 0 |
