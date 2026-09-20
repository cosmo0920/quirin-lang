# Cycle 18 callback task handoff

This card adds a generic callback/task/deferred-work lifetime kernel and a
source-bound no-task disposition for the current selected Monkey callbacks. It
does not promote aggregate Cycle 18 acceptance.

## Scope and result

`qcore-core::callback_task_handoff` now checks callback entry and return,
task spawn/start/complete, cancellation request/acknowledgement, join,
post-return deferred work, argument retention/drop and teardown. A callback may
return while a task or deferred item remains live. Teardown succeeds only after
every task reaches `Joined` and every deferred item reaches `Completed`.
Owned callback arguments may cross the return boundary and remain explicit in
`remaining_argument_custody`; shared arguments may be shared; a frame borrow
cannot be retained by either kind of work. Identities are single-use within the
checked operation sequence.

The state machine refuses completion before start, join before completion or a
cancellation acknowledgement, completion racing a cancellation request,
unknown/duplicate identities, duplicate teardown, argument reuse after drop,
and argument drop while a callback or retained work can still use it. Deferred
work has no cancellation transition in this profile; it must run and complete.

The source projection covers all three role-selected Monkey callbacks and all
eight imported paths:

| Callback | Imported paths | Task/defer disposition |
| --- | ---: | --- |
| `cb_event` | 4 | no task/defer effect after complete callee resolution |
| `cb_timeout` | 3 | no task/defer effect after complete callee resolution |
| `cb_wait_2_timeout` | 1 | no calls and no task/defer effect |

Every opaque call in those paths must resolve to one unique effect declaration
in `contract/qcore/monkey/callback_task_handoff.qcore`. The executable policy
contains 10 declarations and 38 clauses. Unknown calls, scan-loop effects,
profile/role/source identity mismatch, missing callbacks or changed path counts
refuse. The policy declarations are trusted effect assumptions; spelling a call
does not prove its implementation.

The bounded observation checker requires a strict bundle whose producer names
all three completeness capabilities: callback boundaries, task lifecycle and
deferred lifecycle. It refuses any task/deferred action, incomplete or
duplicated callback windows, out-of-order records, and calls outside that
callback's source-derived callee set. Capability completeness and producer
truth remain explicit premises. The previously retained Cycle 18 native
observer did not declare task/deferred lifecycle completeness, so this card
does not reinterpret its absence of task records as evidence.

`QCore.CallbackTaskHandoff` is matching audited Lean for the transition rules:
frame-borrow escape, argument identity/liveness, cancellation acknowledgement,
start-before-completion, cancellation/completion refusal, joined-task and
completed-deferred teardown, and explicit remaining owned custody. The Lean to
Rust correspondence remains review-maintained under standing constraint 28.

## Audit route

`quirin-audit callback-task-handoff CONFIG.json` loads the pinned, configured
Monkey typed source through the existing source-input boundary, rechecks the
bundled policy bytes, consumes the strict observation bundle, re-runs source
identity before publication, refuses report aliases to inputs and writes an
atomic report carrying artifact, property, semantic profile, scope, bounds,
assumptions and evidence version.

## Validation

The final focused commands, exits and private logs are recorded in
`gate.json`. The focused result is 10 core tests, 7 verification tests, a clean
all-target audit build, formatting, and 16 audited Lean theorems. The worktree's
first mistakenly broad Cargo selector also ran the `qcore-core` library's
host-sensitive native rows: 1,000 passed and six failed because this isolated
run did not satisfy those rows' host setup. That attempt is not used as card
evidence; focused reruns pass, and the integrator owns the standing gate.

## Residual discharge conditions

- **`CYCLE18-TASK-OBSERVER-CAPABILITY`.** Deploy task/deferred lifecycle hooks
  in the native observer, bind their producer/build identity, run all three
  selected callbacks and feed that complete bundle through the configured
  audit. Until then the observation arm is a usable rejecting boundary, not a
  native coverage claim.
- **`CYCLE18-TASK-HANDOFF-SOURCE-POSITIVE`.** Bind a real supported callback
  whose imported source invokes a declared task API, derive its captures and
  argument lifetime, and join that source projection to task/deferred native
  actions. Generic model tests are not application evidence.
- **`CYCLE18-TASK-CANCEL-RACE`.** Define and prove arbitration for completion
  racing a cancellation request, then add source and observation vocabulary.
  The current profile refuses that transition.
- **`CYCLE18-DEFERRED-CANCELLATION`.** Add a deferred-work cancellation and
  acknowledgement protocol or keep requiring run-to-completion. No implicit
  cancellation is inferred.
- **`CYCLE18-TASK-EFFECT-DERIVATION`.** Replace trusted per-callee effect
  declarations with checked body/provider derivation. Unknown callees already
  refuse.
- **`CYCLE18-TASK-EVENT-COMPOSITION`.** Bind the generic callback identity and
  quiescence result to the selected `event_reactor` event generation before an
  aggregate event-program certificate can use it.

## Integrator reconciliation list

1. Update the contract corpus pin by **+1 module / +38 clauses** for
   `contract/qcore/monkey/callback_task_handoff.qcore`; do not alter resource
   selection counts.
2. Register the new Lean audit in merged audit counts and preserve
   `proof/QCore.lean`'s import.
3. Carry the six residual names above into the acceptance matrix or record a
   grounded disposition for each. This card retires no existing matrix entry
   by itself.
4. Keep status at no aggregate Cycle 18 promotion. The source result is three
   callbacks/eight paths with declared no-task effects; it is not positive
   callback/task execution evidence.
5. Reconcile the new `qcore-audit` CLI route with sibling command additions if
   `crates/qcore-audit/src/main.rs` conflicts during integration.
