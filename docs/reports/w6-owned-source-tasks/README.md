# Owned Buffer source tasks

Baseline `fa3e947`, 2026-09-07. `compile_files` and normal `qcorec check`/`emit`
now support `ScopedOwnedTasksV1`: root allocation of an immutable i32 Buffer,
affine move into a leaf child, typed original-input return, creator join and
parent read/drop. Linux native execution uses actual malloc-backed pointers.
Failed allocation produces AllocError; failed task creation restores the
original parent bindings without creating a child or join obligation.

## Checked ownership and execution

The frontend consumes exact supported forms, independently validates supplied
structural/resolved records, and lowers all definitions to checked task IR.
`own<Buffer> use one` parameters require move captures; i32 parameters require
copies. Duplicate captures, implicit owned copies, unsupported annotations,
missing allocation permissions and unhandled allocation failures reject.
The [implemented grammar](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02b-owned-buffer-task-execution)
records the complete bounded input surface.

Core states retain distinct scalar, owned-value, handle, Result and consumed
slots, backed by the scoped-task custody ledger. A successful move permanently
consumes the old parent slot. Joining a returned Buffer restores custody under
a fresh Result/value slot, never under the original moved binding. Failed
creation rolls back the reserved identities and retains the original bindings
and siblings. Every return/error exit accounts for all owned slots and resource
generations; a forgotten Buffer, Result or handle rejects.

Each leaf worker is checked independently before the entry. Its exit relation
identifies the original parameter returned and every other parameter dropped.
The caller applies that checked relation to its exact captures; a declared
owned result cannot manufacture a new resource identity. Scalar/unit/error
workers must drop every owned input. Scalar result alternatives may conservatively
use the declared error row only after complete input cleanup is checked.
Owned alternatives use actual checked exits. Typed completion rejects a resource
payload that omits, duplicates or substitutes the owned result identity.

The reference executor chooses explicit allocation/creation outcomes and tracks
buffer payloads, fresh generations and custody through the same checked module.
Unexpected unused decisions or residual buffers reject. Native start records
hold typed argument/result values. Creation failure frees only the unstarted
environment; successful creator join copies the typed result before freeing
that environment. Failed native join or unexpected worker status aborts before
completion inspection or environment/payload reclamation.

Worker buffer events are private until join, then printed before the collected
result. This normalized transcript is not a wall-clock schedule. The native
runtime has no shared worker trace counter. Multiple owned inputs can drop one
in the child and return another; second-allocation failure can explicitly drop
the first input and recover.

## Conformance and regressions

CT01 now executes actual move/return/join/drop, including exact allocation/free
pointer identity and join-before-environment-free controls. CT02 replaces the
old sequential surrogate with an actual forbidden parent read after successful
move capture. CT12 replaces an empty failed acquisition with real failed native
creation, restored Buffer read/drop and no child; it retains a successful-create
control. Coverage is **8 mapped /42 unmapped**. Native evidence is Linux-scoped;
other hosts check explicit target rejection and report native stages unexecuted.
Proof stages remain unassessed by the CT source harness.

Eight portable frontend tests exercise reference outcomes and ownership rejection
boundaries, including multiple inputs, allocation recovery and adversarial raw
IR mutations. A ledger regression checks transactional rejection of forged typed
completion payloads. Seven native tests cover pointer-preserving return, actual
child read/drop, exact child error, allocation/environment/create failure, multiple
owned captures and abort without reclamation after join failure. A CLI process
test checks profile attribution and compiles/runs emitted Buffer transfer. Existing
copy-task tests remain passing.

## Proof and validation boundaries

`QCore.OwnedTaskExecution` adds 29 audited theorems for affine slots, exact owned
Result extraction, reservation/rollback/commit, generation history, checked
worker exit relations, typed completion and creator-only consuming join.
The model represents dropped custody as absent while retaining generation
history. Complete inventory and Rust/model correspondence are explicit premises;
source lowering, heap pointer identity and C11 publication/join refinement are
not proved. No project axioms or placeholders occur in the final audit surface.

The standing gate passes **754 workspace tests**, 19 excluded-provider tests,
43 Python tests, formatting, Clippy, both reevaluation probes, Lean build and
all 20 Lean audit modules. Six fixed Linux ASan/UBSan/LeakSanitizer runs cover
round trip, allocation/environment/create failure, child read/drop and child
error. Initial traced executions fail because LeakSanitizer cannot run under
ptrace; the same hash-pinned binaries then pass outside tracing. Both sets of
logs and exact build/input identities are retained. No abort-path leak-clean
claim is made: unexpected join failure intentionally terminates without reclaiming
storage whose quiescence was not established.

All four full 185-unit Cycle19 audit artifacts are byte-identical to
`w5-configured-frame-helpers`: **2 Safe /42 provisional Unsafe /137 Unknown**,
plus four data-only units. Three function contracts remain checked and 93 Unknown;
raw units remain unpromoted. Persistent Monkey build inputs and the existing
schema-1 ChunkIO package were reused without rebuilding production providers.
See `validation.json`, `audit-parity.json` and `sanitizer-results.json`.

This profile excludes loans, arbitrary owned graphs, owned error payloads,
worker allocation, shared memory, mutexes, nested workers, local move aliases,
general branches/loops and other native platforms. All task CFG blocks must be
reachable under checked alternatives; impossible owned Result arms can therefore
reject. Infallible owned `?` is supported. General error-row/path reasoning and
full Rust/Lean/native refinement remain open. This increment does not complete
W6, revision-0.3 conformance, Cycle19 or the broader generalization goal.
