# Unit 3: executable scoped tasks through compile_files

Assessment of the shared tree on 2026-09-07. Proposal only. Units 1/2 provide
structured/resolved task syntax and a private-state custody ledger; neither
admits source concurrency. No CT mapping follows from this plan.

## First executable boundary

Use the existing language syntax, including `concurrency checked;`,
`thread scope s`, `spawn s worker(copy x)?`, and `join move h`. Do not introduce
an event-list source language, a spawn-success literal, or a separate CLI.

The first module subset has an ordinary parameterless entry, first-order
nonrecursive leaf workers, i32/unit results, immutable i32 local/parameter
slots, copy captures of direct local/parameter names, and explicit lexical
thread scopes. Workers may return an i32 literal/parameter/local or bare unit;
allow checked scalar failure only when its typed error is retained. No global
reads, external calls, pointers, resources, task captures, loans, mutexes,
worker spawns, loops, general branching, arbitrary capture expressions or
user source contracts enter this first execution profile. Reject those forms
in every definition, including uncalled definitions. This is a subset of the
existing grammar, not a replacement specification.

Initially require immediate `?` on spawn; the failure edge must be valid with
no outstanding handles in scopes it leaves. Consequently a second `spawn ...?`
while a first handle is outstanding rejects until source match/catch lowering
can explicitly join the sibling on the failure arm. Sequential spawn/join
pairs are supported; do not silently serialize two requested spawns to make
their failure obligations disappear.

For join support `let result = join move h;` as a typed Result slot, and
`let value = join move h?;` as join followed by explicit propagation. Also
support `let value = result?;` so collecting and propagating stay distinct.
A bare `join move h;` can discard only a statically infallible unit result;
otherwise reject discarded results until explicit result elimination exists.
No implicit propagation at Join. A worker's error is different from SpawnError.

## Concrete core representation

Add `qcore-core::task_execution` and `task_c_emit`. Keep old sequential
TypedCfg/emit_c11 behavior unchanged. Core types contain IDs and byte offsets,
not frontend SymbolId types or compiler dependencies. Suggested public input:

```text
TaskModule { entry: FunctionId, functions: BTreeMap<FunctionId, TaskFunction> }
TaskFunction { parameters: Vec<(SlotId, ScalarType)>, result: ScalarType,
               errors: Set<ErrorId>, declared_effects: TaskEffects,
               entry: BlockId, blocks: BTreeMap<BlockId, TaskBlock> }
ScalarType = Unit | I32
SlotType = Scalar(ScalarType) | Result(ScalarType, Set<ErrorId>)
           | Handle { scope: ScopeId, worker: FunctionId }
ScalarOperand = I32(i32) | Unit | Slot(SlotId)
TaskInstruction = BindScalar {slot, value}
                | EnterScope(ScopeId) | ExitScope(ScopeId)
                | Join {handle, outcome_slot}
TaskTerminator = Goto(BlockId)
               | TrySpawn {scope, worker, ordered_copy_arguments,
                           handle_slot, success, spawn_error}
               | TryOutcome {result_slot, value_slot, success, error}
               | Return(ScalarOperand) | Raise(ErrorId)
```

`TrySpawn` is dynamic, never `succeeds: bool`. Failure identifies the built-in
SpawnError outcome and creates no handle. `TryOutcome` preserves the exact
runtime worker error tag; use an error-value slot or typed error-edge mapping,
not one guessed static ErrorId when the worker has multiple errors. Add that
slot explicitly during implementation. Local bindings are immutable in this
subset, so initializer identities are not reconstructed from mutable names.
Handle and scalar/result slots remain disjoint, even if a source type alias
or spelling attempts to confuse them. Each instruction/terminator retains a
source span through a source-map table keyed by function/block/instruction.

API: `check_task_module(&TaskModule) -> Result<CheckedTaskModule, Vec<...>>`.
CheckedTaskModule has private fields and is the input to
`emit_task_c11(&CheckedTaskModule)` and the reference executor. A caller-built
TaskModule alone is not execution evidence. Check target existence, arity,
exact scalar/result types, slot dominance/initialization, no duplicate slot
writes, worker-effect closure, source binding identity, finite CFG/call graph,
all exits and scope stack agreement. Keep i32 inputs symbolic (literal or slot
identity); do not enumerate integer values. Validate all dynamic creation and
worker outcome alternatives. Any exploration bound exhaustion rejects. Preserve distinct states when handles differ; never
merge a possibly absent handle into a definitely live one.

Reuse ScopedTaskLedger transitions: reserve(copy arguments => empty owned
list), rollback, commit, complete, creator join. Each modeled child execution
gets fresh task/reservation/handle IDs; source slot IDs are not runtime task
IDs. A task holds its result until join; its completed state is terminal.
Reject scope exit with handles/reservations on *any* path. Return/propagation
first checks every scope being left, then emits lexical scope exits; do not
insert joins or classify blocking join as ordinary cleanup. With this first
copy-only subset there are no source resources to clean up. Resource-bearing
source and move captures remain rejected until cleanup and outcome typing are
connected; an empty-resource model must not erase a real source allocation.

## Frontend and compile_files integration

Add a frontend `source_tasks` lowerer consuming SourceProgram plus
TypedSourceProgram. Use TypedStatement.binding, resolved scope/worker/root IDs,
parameter declarations and structured captures. Verify metadata belongs to
this source; lower whole expressions, not just the task subexpression (so an
outer arithmetic expression or assignment cannot be silently discarded).

Retain/resolve scalar initializer and return operands needed by the subset,
and Result propagation. Retain scope binder IDs on the typed ThreadScope
statement as well: unit1 currently exposes those IDs only through spawn
records, which is insufficient for an empty scope or direct scope-exit map.
Retain exact supported effect rows. Existing core Effect has no Spawn, Join or
MayBlock variants and the driver hardcodes Alloc; either extend Effect with
those explicit variants or give this profile a checked TaskEffects record.
Use the existing spelling `effects { throws<SpawnError>, spawn, join, may_block }`.
Reject unsupported effects/clauses; never ignore an annotation. Worker error
sets and `?` propagation must agree with the caller's declared error effects.

compile_files selects this route whenever any real task expression/thread
scope is present, after the existing parsing, name, module and verifier gates.
All definitions must be checked by this route or an explicit safe sequential
adapter. Entry selection stays unchanged. Existing sequential modules take
the existing route, preserving their results and emitted C.

Do not fabricate CheckedProgram/TypedCfg values with task operations erased.
The clean integration is a checked-artifact sum:
`CheckedSourceModule = Sequential(CheckedProgram) | ScopedTasks(CheckedTaskModule)`.
Change CompiledModule.checked to this sum with function/declaration-count and
sequential-accessor methods; migrate its handful of CLI/test field consumers.
A differently named additive artifact field is acceptable only if old checked
fields become explicitly optional; do not populate them with dummy evidence.
Preserve c_source, entry, mode and source_map. Add an explicit execution profile
and task-result profile; source.exec12's separate identity-ledger wording must
not imply typed task payloads. Task-free library/contract behavior stays intact;
non-executable task modules can be checked without emitting a process entry.

## Reference execution and emitted C

Reference checking explores every dynamic creation outcome within the finite
module; evaluation for a selected outcome script is separate test execution,
not the proof of all branches. Add TaskExecutionObservation containing root
outcome and per-task lifecycle/result observations. The existing one fixed
ExecutionObservation is insufficient as the sole evidence for dynamic spawn;
retain it for sequential modules and expose task validation/observations via
an explicit enum or additional task field.

C emission includes entry AND every admitted worker definition, copied i32
parameter storage, stable start/completion records, and a platform runtime
adapter. Initialize the complete start record before thrd_create (or an
explicitly selected pthread equivalent); start-record allocation failure and
create failure take the same checked SpawnError branch. On create failure,
free only private runtime storage. On success retain storage until successful
join; copy the completion result into parent Result storage before reclaiming
it. The worker trampoline publishes its tagged outcome and returns; actual
native join, not a completion flag, supplies quiescence. Unexpected join
failure aborts without freeing possibly accessed records. Do not reuse the
std::thread schedule executor as the emitted language runtime.

Use per-task trace storage, collected at join, or synchronized instrumentation;
old c_emit's unsynchronized global trace buffer cannot be shared by workers.
Compare per-task order and spawn/join edges, not one scheduler-chosen total
trace as if it were the only legal execution. This pure-copy subset has no
shared source memory, so schedule independence can be stated/proved for its
observables without claiming C11 weak-memory or general race freedom.

Backend fault injection belongs in test/runtime wrappers, never source flags.
Exercise real native workers, forced create/allocation failure, and join-failure
abort in isolated subprocesses. Document platform support and reject emit on
unsupported targets rather than falling back to sequential execution.

## Implementation split and gates

1. Frontend/source lowerer: exact binding/type/effect/whole-expression checks;
   scope binder identity; source span mapping. Own frontend files/tests.
2. Core execution IR/checker/reference semantics: finite branch/outcome checking,
   ledger adapter, error/value slots, completed results and well-formedness.
3. C module/runtime emitter + compiler driver/CompiledModule/CLI integration.
   Coordinator owns public artifact changes and merges units 1/2 before admission.
4. Independent source/CLI/native tests plus Lean: creation failure creates no
   handle, committed captures copied exactly, completion preserves result until
   creator join, join consumes once, every admitted scope exit is task-free,
   explicit propagation cannot strand siblings, typed result slot agreement.
   Parser/Clang/native C refinement remains an explicit correspondence boundary.

Negative gates: missing marker; wrong scope/worker/arity/types; globals/externs;
non-copy or compound captures; duplicate/uninitialized/wrong-type slots;
undiscriminated spawn result used as handle; dropped/copied/returned handle;
wrong creator/double join; use after scope exit; return/error with child alive
or completed-unjoined; second failing create with a sibling; worker error
misclassified as SpawnError; ignored failing join result; unsupported branches,
loops/resources/clauses; worker instruction omitted from emission; same-name
shadowed binder confusion; copied scalar treated as parent-shared storage.

Acceptance requires normal qcorec check+emit source tests, generated C on the
supported platform with fault injection, reference/outcome correspondence,
all standing Rust/Python/Lean gates, unchanged legacy sequential tests, and
exact archived pinned185 parity. Update CT rows only for cases whose actual
specified source/runtime path has executed; do not credit move/loan cases to
copy-only results. Full move/read/write capture, sibling failure handlers,
nested workers, typed owned/error payloads, quantitative task storage and
C11/platform refinements remain the active subsequent objective.
