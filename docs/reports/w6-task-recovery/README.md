# Explicit scoped-task Result recovery

Baseline `0d344b4`, 2026-09-07. The existing `source.tasks.copy.v1` profile now
executes explicit two-arm Result handlers through `compile_files`, `qcorec` and
Linux C11 threads. Failed second creation preserves the first sibling until
its handler explicitly joins it. Join handlers retain exact worker error tags.
This is bounded copied-scalar recovery; general concurrency remains open.

## Source and checked behavior

Canonical patterns are `Result::Ok { value: name }` and
`Result::Err { error: name }`, exactly once each in either order. A scrutinee
is a whole unpropagated spawn, join, or existing Result binding. Arm names are
local; bodies may exit or continue with the original outer identities.
`raise name;` rethrows the exact bound creation or worker error. Handling a
creation error does not require permission to propagate SpawnError, but the
spawn operation still requires its effect. No arm silently joins a sibling.

Structural parsing retains complete patterns, scrutinees and body ranges;
unsupported patterns remain distinguishable from canonical patterns. Typed
children have one owner and contiguous arm ranges, avoiding recursive subtree
duplication. Independent reparsing/elaboration and exact token consumption
bind executable lowering to the supplied source. The core checks all reachable
branch states, handle/Result consumption and every declared worker outcome.

Recovery exposed an over-rejection in infallible `?`: its impossible error edge
could leave another live sibling stranded. `UnwrapInfallible` now consumes a
Result only after the core establishes its empty declared error set. It binds
the original scalar value without an error branch. A nonempty permitted error
row cannot use this operation. Explicit matching of an infallible Result still
conservatively checks both arms; general dead-branch elimination is not claimed.

## CT13 and native observations

`compiler/tests/fixtures/ct/CT13` contains actual source for a first child
returning 42 and a second returning 7. The success control joins both and returns 7.
First-create failure creates no child and propagates SpawnError. Second-create
failure frees only its unstarted record, then joins the original first thread
and returns 42. The native adapter records the exact successful thread and start
record; freeing a still-unjoined record is an immediate test failure. A matching
source arm that omits the first join rejects with Q600 ScopeBusy.

The manifest records parser, type-checker, task-CFG, reference and Linux native
stages separately. Other hosts verify the explicit unsupported-target rejection
and do not claim native execution. CT13 adds one mapped case: 7 mapped, 43
unmapped. CT15 and CT19 retain their earlier join-failure/rejection evidence.
This does not establish move/loan restoration, usage/SSA rollback or CT14.

Native integration tests additionally cover second start-allocation failure,
exact worker-error rethrow after joining a sibling, direct join matching and
matching an existing collected Result. A CLI process test checks, emits,
compiles and executes CT13's second-create failure with exact observations.
The fault wrappers affect the runtime test environment, not source semantics.

## Proof and validation boundaries

`QCore.TaskExecution` now has 49 audited theorems, 22 added here. They cover
retained siblings, explicit join before normal/error handler exit, exact worker
error collection and empty-error Result extraction. These are model theorems;
source import completeness and Rust/Lean/native-C correspondence remain open.
The 20 audit modules contain no project axioms or placeholders.

The combined standing gate passes 686 workspace tests, nine provider tests,
43 Python tests, formatting, Clippy, both probes, Lean build and all 20 audits.
Normal and second-create failure controls pass ASan/UBSan/LeakSanitizer outside
the tracing sandbox. Initial runs fail because LeakSanitizer cannot run under
ptrace; both environmental logs are retained. The same hash-pinned binaries
then pass outside tracing. Input, generated-C and binary hashes, commands and
observations are recorded in `sanitizer-results.json` and companion files.
These fixed runs do not establish arbitrary failure behavior or other targets.
See `validation.json` and archived logs. All four full 185 audit outputs are byte-identical to the
baseline: 2 Safe, 47 provisional Unsafe, 132 Unknown and 4 data-only units.
Source-effect analysis retains 3 checked function contracts and 93 Unknown
functions. Persistent Monkey build inputs and the existing schema 1 ChunkIO
package were reused. No raw unit was promoted.

Moves, loans, owned results, shared memory, mutexes, nested workers, general
source control flow, entry-free task modules and other native targets remain
open. This increment does not complete W6, v0.3 conformance or Cycle19.
