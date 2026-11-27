# Cycle 18 callback task handoff

This card adds an executable callback/task/deferred-work lifetime boundary. It
combines a generic Rust kernel, a keyed Lean trace model, source-derived
operations for a bounded pthread fixture, a fresh native run, and a separate
no-task disposition for the current selected Monkey callbacks. It does not
promote aggregate Cycle 18 acceptance.

## Scope and result

`qcore-core::callback_task_handoff` checks callback entry and return, task
spawn/start/complete, cancellation request/acknowledgement, join, post-return
deferred work, argument retention/drop, and teardown. Callback return may
precede task join and deferred completion. Teardown requires every owned task to
be joined and every owned deferred item to be complete. Owned/shared argument
custody can cross return; frame borrows cannot escape. The certificate reports
any remaining custody instead of implying release.

Dropping a shared argument records release of this callback handoff model's own
shared custody; it does not claim that global shared storage was freed. A frame
borrow expires at callback return and can never be reported as dropped by the
callback owner.

The state machine refuses completion before start, join before completion or
cancel acknowledgement, completion after cancellation request, wrong owners,
unknown or reused identities, duplicate teardown, argument reuse after drop,
an attempted frame-borrow drop, and drop while active or retained work can use
the argument. Deferred work has no cancellation transition in this profile and
must complete.

`compiler/tests/fixtures/cycle18_task_handoff.c` supplies the bounded positive
source instance. Typed source projection derives exact helper calls and integer
identities from `cycle18_taskful_callback` and
`cycle18_taskful_after_return`. The Linux pthread run executes a worker after
callback return, records distinct callback/worker pthread identities, observes
cancel acknowledgement and join, runs deferred work after return, drops the
owned argument, and tears down. Its twelve source-derived operations exactly
match the strict observation bundle and pass the generic kernel.

The native JSON is emitted after the pthread join as `causal_replay`. Its row
order is a checked replay of source operations and explicit synchronization
edges; it is not a claim about unsynchronized physical execution order or race
freedom. The claim is lifetime/custody only and does not include payload
permissions.

The source projection also covers all three selected Monkey callbacks and all
eight imported paths:

| Callback | Imported paths | Task/defer disposition |
| --- | ---: | --- |
| `cb_event` | 4 | no task/defer effect after complete callee resolution |
| `cb_timeout` | 3 | no task/defer effect after complete callee resolution |
| `cb_wait_2_timeout` | 1 | no calls and no task/defer effect |

Every opaque Monkey call must resolve to one unique effect declaration. Missing
or ambiguous callees, unsupported source effects, changed identities, roles,
arguments, calls, or path counts refuse. These declarations are explicit
trusted effect contracts; API spelling alone is not evidence. The executable
policy has 12 declarations and 66 clauses. The taskful declaration owns the
artifact, revision, source set, library, import roles, exact call vectors and
integer identities, and the exact fixture source digest; its primitive
declaration owns every helper spelling.
Rust extracts those facts and applies the generic state machine.

The Monkey observation boundary requires callback-boundary, task-lifecycle, and
deferred-lifecycle completeness. The retained production archives do not
carry those capabilities, so this card does not reinterpret missing actions as
universal absence. The no-task result is source-derived for three callbacks and
eight paths and observation-bounded only when a capable producer supplies the
strict bundle.

`QCore.CallbackTaskHandoff` now includes a keyed whole-trace state machine, not
only pointwise transition predicates. It models callback/task/deferred/argument
maps, owner checks, identity non-reuse, retained argument drop, frame-borrow
drop refusal, canonical certificate ordering, teardown, operation sequencing,
and certificate construction. The audited positive trace
constructs the exact fixture certificate; audited negative traces cover wrong
owners, identity reuse, retention, and premature teardown. Lean/Rust
correspondence remains review-maintained under standing constraint 28.

## Audit route and provenance

`quirin-audit callback-task-handoff-fixture CONFIG.json` reimports both fixture
functions from the pinned source profile, projects their exact operations,
requires an exact observation match, runs the kernel, and publishes an atomic
report. Its strict producer manifest binds the fixture, selected profile,
policy, actual gateway compiler program/full version/target, exact sanitizer
compile argv, binary bytes, exact run argv and exit, stdout/empty-stderr bytes,
the explicit `ASAN_OPTIONS=detect_leaks=0` run environment, observation bytes,
and callback/worker thread identities. Leak detection is disabled because it
cannot run under the supervised host's ptrace boundary; the claim is
AddressSanitizer/UBSan only. The audit protects
and rechecks the manifest, profile file and directory, source, policy,
observation, binary, compiler, and configuration, and refuses report aliases.
The profile pin is `cc3910a46594d839aad6a83bd8fe45fca769e20e`, the first commit
that contains the exact fixture bytes; it is not the older Cycle 18 base.
Producer truth and the declared helper effects remain explicit trust bounds.

`quirin-audit callback-task-handoff CONFIG.json` is the corresponding configured
Monkey route. It loads pinned typed source, verifies the bundled policy,
consumes a strict capability-bearing bundle, rechecks all source inputs, and
publishes the same verdict-contract fields.

## Validation

Final commands, exits, counts, and private log paths are recorded in
`gate.json`. The native producer was compiled with pthreads and
AddressSanitizer/UBSan, exited zero, and produced empty sanitizer stderr.
Focused Rust tests cover positive post-return handoff and negative owner,
identity, transition, capability, source-call and observation substitutions.
The Lean audit contains 26 placeholder-free theorem audits. The integrator owns
the standing full gate.

`run-evidence.sh OUTPUT_DIRECTORY` is the shipped bounded reproduction route.
It builds and runs the native producer, constructs the joined manifest, runs
the configured positive audit, and writes separate logs for five refusing
controls. The observation, helper-body source and profile controls recompute
the affected manifest hashes; the helper-body control keeps the imported call
summaries unchanged and is refused by the Quirin source digest. The stderr
control also recomputes its hash and proves that
nonempty raw stderr refuses; the artifact control exercises Quirin-owned
artifact acceptance. The archived JSON is the fresh positive output of that
route. The private output and per-control logs are named in `gate.json`.

A mistakenly broad early Cargo selector ran host-sensitive `qcore-core` native
rows without their required host setup: 1,000 passed and six failed. That run
is not evidence for this card; all final commands are focused.

## Residual discharge conditions

- **`CYCLE18-TASK-HANDOFF-PRODUCTION-POSITIVE`.** Bind the taskful projection
  and capable native producer to a supported production callback. Monkey is
  task-free under declared effects; the positive taskful instance is the
  checked-in generic fixture.
- **`CYCLE18-TASK-CANCEL-RACE`.** Define and prove arbitration for completion
  racing cancellation. The current profile refuses that transition.
- **`CYCLE18-DEFERRED-CANCELLATION`.** Add a deferred cancellation and
  acknowledgement protocol or retain the run-to-completion restriction.
- **`CYCLE18-TASK-EFFECT-DERIVATION`.** Replace trusted helper/callee effect
  declarations with checked body or provider derivation.
- **`CYCLE18-TASK-EVENT-COMPOSITION`.** Bind the handoff certificate to the
  selected `event_reactor` event generation before aggregate use.
- **`CYCLE18-TASK-NATIVE-GENERALIZATION`.** Replace the bounded fixture producer
  with a reusable instrumentation route while preserving manifest and
  completeness semantics.

## Integrator reconciliation list

1. Update the contract corpus pin by **+1 module / +66 clauses** for
   `contract/qcore/monkey/callback_task_handoff.qcore`; resource selections do
   not change.
2. Register 26 new Lean theorem audits and preserve `proof/QCore.lean`'s import.
3. Carry the six residual names above into the acceptance matrix or record a
   grounded disposition for each.
4. Keep status at no aggregate Cycle 18 promotion. The bounded taskful fixture
   establishes the generic mechanism; it is not production taskful coverage.
5. Reconcile both `qcore-audit` CLI routes with sibling command additions.
