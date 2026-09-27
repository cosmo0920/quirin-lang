# Native scoped-copy source tasks

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Baseline `9262b29`, 2026-09-07. `compile_files` and the ordinary `qcorec check`
and `emit` commands now admit a named scoped-copy task profile on Linux C11
threads. Source programs carry `concurrency checked;`, first-order leaf workers,
immutable i32 copies, i32/unit results, explicit scopes and creator joins.
The generated module contains every checked worker and its native trampoline.
This is executable source support, not admission of the general concurrency
language or of resource/loan captures.

## Source, evidence and runtime

The lowerer independently reparses and elaborates the source before using
symbol metadata. It consumes complete declarations, headers, statements,
expressions and scope bodies. It preserves shadowed binder identities and
checks scalar types, worker arity and required effect/error permissions.
Declared effect rows are permissions, so a row may contain unused permissions
or an error family whose variants are not all raised. Unsupported contracts,
resources, globals, externs, non-copy/compound captures, loops and general
branches reject in every definition, including uncalled workers. A regression
covers an extra structural block that previously could hide a scope body.

Core checking produces an opaque `CheckedTaskModule` after validating all
functions, a finite acyclic CFG, slot definitions/initialization, worker effects,
all dynamic creation alternatives and declared worker outcome alternatives.
The exploration limit rejects exhausted checks. Creation has distinct success
and failure edges; no source Boolean selects success. Handles, collected
Results, scalar values and error tags occupy distinct checked slot classes.
Join collects a result; `?` separately propagates it. Scope exit, return and
error propagation cannot discard a live handle or an unconsumed Result.

`CompiledModule.checked` is now an explicit `CheckedSourceModule` enum. Task
modules carry their real task evidence and no invented sequential CFG or fixed
reference observation. Existing sequential consumers use `as_sequential()`;
`as_scoped_tasks()` selects task evidence. Reference execution takes an explicit
creation-outcome script and is separate from the all-path checker.

The C runtime fully initializes private start records and copies scalar
arguments before `thrd_create`. Failed allocation creates no record; failed
creation frees only the unstarted record. Successful creation retains the
record until `thrd_join` proves quiescence. The trampoline writes only its own
completion. The creator then collects the exact result and frees the record.
Unexpected native join/status failure aborts before reclamation or completion
publication. Runtime fault wrappers are test instrumentation, not source flags.

Only the parent prints task observations, after successful creation and join.
Workers do not share a mutable global trace buffer. The reference executor
selects an eager leaf schedule; transcript comparison checks these parent-ordered
observations and exact outcomes, not all possible total worker schedules.
Native execution is currently validated on Linux with C11 threads; the source
route rejects other host targets explicitly. Portable core/frontend tests do
not imply native support on those targets.

## Verification

Native source tests compile emitted C with warnings as errors and exercise real
workers, signed i32 extrema, payload-free worker errors, two sequential tasks,
second-create failure after joining the first, allocation/create failure, and
join-failure abort before reclamation. CLI tests run check, emit, C compilation
and execution, and verify that rejected source cannot overwrite an artifact.

`QCore.TaskExecution` adds 27 audited theorems for typed ordered copies, bounded
i32 representation, failed creation, fresh success, completion retention,
creator-only result collection, exact error tags and quiescent propagation.
Source import, inventory completeness, Rust/Lean correspondence and native C
refinement remain explicit premises or open correspondence work. These proofs
must not be presented as a theorem about all operating-system executions.

The combined gate passes 663 workspace tests, nine excluded provider tests,
43 Python tests, formatting, Clippy, both reevaluation probes, Lean build and
20 warning-as-error Lean audits. Logs and validation.json are archived here.
All four reports from the full 185-unit audit are byte-identical to the schema-1
baseline: 2 Safe /47 provisional Unsafe /132 Unknown plus four data-only units.
Three field contracts remain checked and 93 functions Unknown. No raw unit
is promoted. See audit-parity.json and the original compressed outputs.

## Remaining language work

Move/read/write captures, typed owned result payloads, loan/permission return,
usage/SSA rollback, nested workers, mutexes, shared memory, general control flow,
sibling recovery handlers and other native platforms remain open. Immediate
`spawn ...?` cannot leave a preceding sibling unjoined on creation failure;
that program rejects until a source handler can explicitly recover and join it.
Entry-free task modules are also still rejected. The new source path does not
complete W6, the whole v0.3 language or Cycle19's aggregate evidence gates.


## CT scope and sanitizer record

CT15 now runs the actual source, checked task CFG, successful reference/native
control and failed-native-join process. The success control proves native join
then environment free; the failure control observes abort with no free or
completion publication. Its manifest explicitly limits native evidence to
Linux. Other hosts check the unsupported-target rejection and report native
stages unexecuted there. CT19 now rejects a real second `spawn ...?` with an
outstanding sibling, and checks a separate joined-sibling positive source.
Six cases are mapped and 44 remain unmapped; CT01/CT12 authority restoration,
CT13 sibling recovery and other broader requirements are not credited to copies.

The archived `native-copy.qcore` also executes under ASan, UBSan and LeakSanitizer.
LeakSanitizer cannot run inside the tracing sandbox; its initial environmental
failure is retained in `sanitizers-traced.log`. Running that same generated
binary outside the tracer passes all three checks (`sanitizers-untraced.log`).
This is one real copied-scalar create/join observation, not a sanitizer proof
of arbitrary thread runtimes or failed-join reclamation.

```sh
cargo run --quiet -p qcorec -- emit \
  docs/reports/w6-native-source-tasks/native-copy.qcore -o /tmp/qcore-task-copy.c
clang -std=c11 -pthread -Wall -Wextra -Werror \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  /tmp/qcore-task-copy.c -o /tmp/qcore-task-copy
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 UBSAN_OPTIONS=halt_on_error=1 \
  /tmp/qcore-task-copy
```
