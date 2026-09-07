# V02 — R02 terminal task state: adversarial verification

Baseline `6c2782f`, 2026-09-07. Work card V02 (`verify_or_repair`) re-examined
R02 ("a joined child can still emit events",
`docs/reviews/r0_3/supplied/REVIEW.md`) beyond B02's smoke classification.

**Verdict: verified fixed. No soundness failure found; no production code
changed.** The gap was in *evidence*, not behavior: the existing Rust matrix
test asserted only `is_err()` (an incidental downstream rejection would have
satisfied it), the refused-join direction had no dedicated regression, and the
Lean lifetime model named terminal rejection for only two of its eight event
constructors. Four adversarial Rust tests and nineteen Lean theorems close
that evidence gap. Every new test and theorem passed on its first run.

## Model boundary

Two independent state machines carry the R02 obligation; both are inputs to a
trusted explicit trace or a checked task IR, not reconstructed C source.

| Model | File | States |
|---|---|---|
| Restricted concurrent event checker | `crates/qcore-core/src/concurrency.rs` | *unknown* (absent from `clocks`), *active* (`active` set), *terminal* (in `clocks`, not in `active`) |
| Scoped task/custody ledger | `crates/qcore-core/src/scoped_tasks.rs` | `TaskState::{Active, Completed, Joined}` plus *unknown* |
| Native C11 backend | `crates/qcore-core/src/task_c_emit.rs` | a failed `thrd_join` calls `abort()` before reading completion data or reclaiming the environment; there is no *join-failed* state to authorize anything |

The concurrency checker deliberately keeps the joined child's vector clock in
`clocks` (join merges it into the parent, and it must stay to order later
parent events). The separation that discharges R02 is that `active_task` tests
membership of `active`, while `clocks` membership only chooses between
`TerminalTask` and `UnknownTask`. Retained history is therefore ordering
evidence, never execution authority.

## Coverage matrix — concurrency event checker

State × event → expected verdict → covering test. Every terminal-state row is
probed at trace index 2 after `Spawn(0,1); Join(0,1)`, with a static
`FULL_ACCESS` permission and `initial_custody = {7: 1}` for child 1 — authority
that *would* admit the event if the terminal check were skipped.

| Event kind | Active child | Terminal (joined) child | Covering test |
|---|---|---|---|
| `Access` read | accepted | `TerminalTask{2,1}` | `joined_child_event_kinds_each_reject_with_their_own_terminal_diagnostic` (new), `active_child_event_kinds_and_postjoin_parent_events_are_preserved` (new) |
| `Access` write | accepted | `TerminalTask{2,1}` | same |
| `Access` atomic write | accepted | `TerminalTask{2,1}` | same |
| `Access` guarded | accepted under a held guard | `TerminalTask{2,1}`, and **no** `WrongMutexInvariant` | same |
| `Lock` | accepted | `TerminalTask{2,1}` | same |
| `Unlock` | accepted | `TerminalTask{2,1}` | same, plus `terminal_tasks_cannot_receive_custody_or_unlock` |
| `TransferCustody` (child as sender) | accepted | `TerminalTask{2,1}` | same |
| `TransferCustody` (child as recipient) | accepted | `TerminalTask{2,1}` | same, plus `terminal_tasks_cannot_receive_custody_or_unlock` |
| `Destroy` | accepted with custody + quiescence | `TerminalTask{2,1}` | same |
| `Spawn` (child as parent) | accepted | `TerminalTask{2,1}` | same, plus `nested_join_requires_child_quiescence_and_consumes_handle_once` |
| `Join` (child as parent) | accepted | `TerminalTask{2,1}` | same |
| `Join` (repeated on the same child) | n/a | `UnknownJoin{2,1}` | same, plus `nested_join_requires_child_quiescence_and_consumes_handle_once` |
| `Spawn` reusing the terminal identity | n/a | `DuplicateTask{2,1}` | same |

The new matrix additionally asserts that **no** `WrongMutexInvariant`,
`UnauthorizedAccess` or `WrongCustodian` diagnostic appears in any terminal
row. That is the load-bearing part: it proves the terminal decision precedes
every authority check rather than coinciding with one, which the previous
`is_err()` assertion could not distinguish.

### Refused join (acceptance 2)

A refused join must not imply quiescence. In `check_concurrent` a rejected
`Join` takes the `continue` path *before* `joined.insert`/`active.remove`, so
the child stays authorized and stays counted by the final `OutstandingTask`
sweep. `refused_join_preserves_child_authority_and_outstanding_duties` (new)
asserts the exact diagnostic vector, not just membership:

| Refusal | Diagnostic vector | What the exact vector proves |
|---|---|---|
| Unjoined descendant | `[NonquiescentJoin{2,1}]` | the child's later `Access`, its own `Join` of its descendant, and the parent's retried `Join` were all accepted — no `TerminalTask`, no `OutstandingTask` |
| Child holds a guard | `[NonquiescentJoin{2,1}]` | the child could still `Unlock` and be joined afterwards; its duty survived |
| Unknown parent | `[UnknownTask{1,9}, OutstandingTask{1}]` | a join that never happened leaves the child outstanding, and its later `Access` still runs |

This matches the native direction: CT15 ("Backend join failure" → "No
quiescence, no environment reclamation",
`docs/reports/w6-task-recovery/README.md`) and the `qcore_task_collect`
`abort()` before any reclamation in `task_c_emit.rs`.

### Positive control (acceptance 2)

`active_child_event_kinds_and_postjoin_parent_events_are_preserved` (new) runs
one 12-event trace in which the child receives custody, locks, writes under the
guard, unlocks, reads under its permission, spawns and joins a grandchild, then
destroys its resource; the parent then joins it and performs its own write and
destruction on a distinct generation. Certificate: `completed_tasks == [1, 2]`,
`destroyed_resources == [7, 8]`, `checked_events == 12`. The pre-existing
`join_orders_prejoin_access_and_allows_destruction_then_distinct_generation`
covers the ordering half separately.

## Coverage matrix — scoped task ledger (acceptance 3)

Completion is not custody return. `complete` moves each remaining resource to
`Custodian::Capsule(child)`, which is nobody's `Custodian::Task`, so
`use_resource` fails for the parent *and* the child until the creator's `join`.

`completed_and_joined_children_reject_every_ledger_operation` (new) pins a
fixture with child 1 `Completed` (holding capsule resource 7) and child 2
`Joined`, then probes all ten public operations for each phase:

| Operation | `Completed` child | `Joined` child |
|---|---|---|
| `acquire` | `InactiveTask(c)`, ledger unchanged | same |
| `use_resource` | `InactiveTask(c)` | same |
| `drop_resource` | `InactiveTask(c)`, ledger unchanged | same |
| `enter_scope` | `InactiveTask(c)`, ledger unchanged | same |
| `exit_scope` | `InactiveTask(c)`, ledger unchanged | same |
| `reserve` | `InactiveTask(c)`, ledger unchanged | same |
| `rollback` (on a live pending reservation) | `InactiveTask(c)`, ledger unchanged | same |
| `commit` (on a live pending reservation) | `InactiveTask(c)`, ledger unchanged | same |
| `complete` | `InactiveTask(c)`, ledger unchanged | same |
| `join` (on a live handle) | `InactiveTask(c)`, ledger unchanged | same |

Every one funnels through the single `ScopedTaskLedger::active` gate before any
custody, scope, reservation, handle or creator check, so no phase can be
bypassed by supplying a valid identity for a later check. The same test pins
the capsule facts directly: `resource_state(7) == Live(Capsule(1))`,
`drop_resource(0, 7)` and `use_resource(0, 7)` give
`NotCustodian{task: 0, resource: 7}`, `exit_scope(0, 10)` gives
`ScopeBusy(10)`, and only the creator's `join` returns resource 7 to task 0 —
after which a repeated `join` is refused as `HandleConsumed(30)`.

This is the ledger counterpart of CT14 ("Child finishes but parent has not
joined" → "No restored parent authority") and CT37 ("Child returns allocation
and finishes early" → "Allocation remains live in completion capsule"),
recorded in `docs/reports/w6-completed-child-custody/README.md`.

## Lean parity

`proof/QCore/Concurrency.lean` `Lifetime.step` already rejected terminal tasks
for every constructor, but only `.access` and `.taskAction` had named theorems.
Nineteen theorems were added so the Lean matrix matches the Rust matrix
constructor for constructor:

- Per-kind terminal rejection: `terminal_task_rejects_lock`,
  `terminal_task_rejects_unlock`, `terminal_sender_rejects_transfer`,
  `terminal_recipient_rejects_transfer`, `terminal_task_rejects_destroy`,
  `terminal_parent_rejects_spawn`, `terminal_parent_rejects_join`,
  `terminal_child_rejects_join`.
- Post-join corollaries composed with `join_terminates_child`:
  `join_prevents_child_lock`, `join_prevents_child_unlock`,
  `join_prevents_child_transfer`, `join_prevents_child_receiving_custody`,
  `join_prevents_child_destruction`, `join_prevents_child_spawn`,
  `join_cannot_be_repeated`.
- Refused join: `nonquiescent_child_rejects_join`, `locked_parent_rejects_join`,
  and `refused_join_preserves_child_authority` (a refused join leaves the child
  non-terminal with its own actions still enabled).

They join the existing `terminal_task_rejects_access`,
`terminal_task_rejects_action`, `join_terminates_child`,
`join_prevents_child_action`, `step_preserves_terminal` and
`run_preserves_terminal`. All nineteen are registered in
`proof/AuditCycle14.lean`. Acceptance 3's Lean side was already complete and is
cited unchanged: `QCore.ScopedTasks.unjoined_capsule_is_unavailable_to_every_task`,
`completed_task_cannot_execute_resource_use`, `joined_task_cannot_execute_resource_use`,
`checked_join_returns_capsule_resources_to_creator`, and
`QCore.OwnedTaskExecution.completed_owned_payload_is_inaccessible_to_every_task`,
`running_owned_task_cannot_be_collected`, `only_creator_can_collect_owned_payload`,
`owned_join_cannot_be_repeated`.

No axiom, `sorry`, placeholder or model definition changed. `AuditAll` still
admits only `propext`, `Classical.choice` and `Quot.sound`.

## Changes

| File | Change |
|---|---|
| `crates/qcore-core/tests/unit/concurrency.rs` | +3 tests (terminal matrix, active/post-join positive control, refused join) |
| `crates/qcore-core/src/scoped_tasks.rs` | +1 test and a `rejected` helper asserting exact error and unchanged ledger |
| `proof/QCore/Concurrency.lean` | +19 theorems in `Lifetime` |
| `proof/AuditCycle14.lean` | +19 `#print axioms` entries |

No production behavior changed, so no verdict, coverage, diagnostic or
artifact could move: CT coverage remains 11/50 mapped, the contract corpus
remains 59 modules, and the four full 185-unit Cycle19 artifacts are untouched
by this increment.

## Gate results

| Gate | Result |
|---|---|
| `cargo test --workspace` | 823 passed, 0 failed (819 at baseline; +4 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `lake build` (proof) | 62 jobs, success |
| `scripts/check_lean_audits.py` | 20 audits passed, warnings-as-errors |
| `cargo test --manifest-path providers/Cargo.toml` | 45 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 43 passed |

## Boundaries this does not establish

These are model-level regressions over trusted explicit traces and a checked
finite task IR. They do not establish source-to-trace refinement, that arbitrary
C source lowers into these traces, checked mutex invariants, loans, fractional
shares, general owned graphs, nested workers, arbitrary native schedules, or
Rust/Lean/native-C correspondence. `ConcurrencyAssumption::GuardFootprintAuthority`
and `SynchronizedCustodyEdges` remain declared trace assertions (R03), and no
raw Cycle19 unit is promoted.
