# Source task syntax and checked custody foundations

Baseline `d938970`, 2026-09-07. W6 source concurrency previously retained
spawn and join only as opaque statement text. The frontend now retains ordered
capture modes, place and share trees, exact spans, propagation syntax and
resolved scope/worker/place identities. Typed statements retain their declaration
binder identity after resolving the initializer, preserving shadowed handles.
Workers must resolve to source functions with matching arity; externs, shadowed
workers, invalid scope roles and unresolved capture dependencies reject.

These are structural records. They do not establish capture types, Send/Sync,
stable addresses, disjoint footprints, share validity or a live task handle.
The actual `compile_files` regression reaches the existing Q500 structured-control
gate after successful task resolution and produces no executable artifact.
The CT manifest remains five mapped cases and 45 unmapped cases.

## Custody relation

`qcore-core::scoped_tasks` adds a private transactional ledger. Ordered unique
owned resources move from the parent to a reservation; rollback restores them,
while successful commit creates a fresh child and a creator-owned join handle.
Completion puts every remaining child resource in a capsule keyed by child
identity. Neither the parent nor the completed child can use those resources.
Only the creator joins, consumes the handle and receives the capsule. A child
error is collected without automatically exiting a parent that has siblings.

Scope exit and child completion reject outstanding reservations or handles.
Completed/joined tasks cannot execute ledger actions. Identity histories prevent
resource, task, scope, reservation and handle reuse. Every rejected operation
leaves the whole ledger unchanged. Seven scenario tests exercise creation
failure/retry, sibling isolation, nested scope discipline, exact completion,
foreign creators, replay and identity reuse; a public doctest demonstrates
completion followed by explicit join.

The ledger currently permits reservation only in the innermost scope, whereas
the source grammar can name a live enclosing scope. Its Unit/I32/Error outcome
is separate from its resource-identity list; this is not typed ownership of a
result payload. Scalar storage, usage/SSA rollback, loans, mutexes, scheduling,
races and runtime creation are outside this model.

`QCore.ScopedTasks` proves ten checked-transfer properties: original custody,
exact selected transfers, preservation of other identities, exact rollback,
reservation/capsule exclusion, completed/joined task exclusion and custody
returned by a checked join. AuditCycle14 includes their axiom surfaces. These
are proofs of the transfer relation, not source-to-runtime refinement or a
proof of operating-system thread creation.

## Remaining source execution

The [next implementation design](NEXT_SOURCE_EXECUTION.md) records the proposed
APIs, source subset, dynamic failure paths and negative gates; it is a plan,
not an implemented capability.

Next, add a typed task/result execution IR with entry and worker bodies, scalar
parameters/results, dynamic creation outcomes, handle identities and explicit
join outcomes. Integrate it with the real driver rather than emitting an empty
sequential CFG. A first executable subset can use i32/unit workers and copy
captures, while retaining all-or-nothing creation and explicit joins.

The C backend then needs stable worker environments and trampolines, native
create/join, failure rollback, completion collection and abort on unexpected
join failure. Task observations need safe per-task storage or synchronization.
Owned moves and loans require additional typed payload/authority composition.
Only actual checked source execution can justify new CT mappings.

## Validation

All 636 workspace tests pass (including the new task scenarios, frontend tests,
public doctest and actual driver rejection), along with nine provider tests,
43 Python tests, formatting, Clippy, both reevaluation probes, Lean build and
19 warning-as-error Lean audits. Logs and `validation.json` are archived here. The companion [schema migration](../schema-1-compatibility/README.md)
repairs the public Rust schema version without changing protocol framing.

The schema-1 provider rebuild preserves all 185 raw unit verdicts: 2 Safe,
47 provisional Unsafe, 132 Unknown and four data-only units. Source, diagnostics
and chunk-contract sidecars are byte-identical to the baseline. Source-effect
evidence changes only three verified package digests; three field contracts
remain checked and 93 functions Unknown. No raw unit is promoted.
