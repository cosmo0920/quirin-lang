# Cycle 19: Fluent Bit Resource and Event Integration

**Status: In progress — native field contracts and allocator/descriptor observations checked; all three Monkey callback access paths imported; custody/lifecycle and source/runtime composition gates remain**

Cycle 19 turns the pinned Fluent Bit library projection from Cycle 19's initial
measurement into bounded resource and event certificates. It does not claim
that a high parser-projection percentage is memory-safety coverage, and it does
not reinterpret a conservative `Unsafe` result as an upstream defect.

The source and build boundary remains the one recorded in
`CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md`: pinned CMetrics, CTraces,
librdkafka, ChunkIO, Fluent Bit's central input-chunk wrapper, and a separate
Monkey build forced onto its bundled Libevent adapter.

Library-specific semantics continue to stabilize in this milestone. Libevent
and ChunkIO source modules now have in-process plugin boundaries; separately
versioned, process-isolated verifier providers remain Cycle 20 work; Cycle 19 APIs should
therefore expose explicit request, result, profile, and certificate boundaries
without introducing a native Rust plugin ABI.

## Ordered workstreams

### 19A: Contract and control-flow foundation

Replace name-only acquire/release annotations with checked call contracts for
borrowed, owned, consumed, released, and out parameters. Model owned and
nullable-owned results, including `realloc`'s preserve-on-failure and
replace-on-success behavior. Refine nullable custody only along recognized
null-test edges; unresolved arguments, conditions, or aliases remain
`Unknown`.

Import `for`, `switch`, cleanup `goto`, labels, `break`, and `continue` into the
resource CFG. The importer may support bounded structured subprofiles first,
but it must never erase an unsupported jump or infer cleanup from lexical
shape alone.

### 19B: Aggregate and library custody

Add aggregate-field, intrusive-list, typed-queue, refcount, arena, and
destructor summaries. Connect those summaries to function contracts so a
local allocation stored in a container is transferred rather than reported as
a leak, and removal/destruction restores or discharges the same identity.

Thread and callback handoff, atomic publication, and reclamation require
explicit Cycle 17 synchronization evidence. Until then, affected librdkafka
paths remain `Unknown`.

### 19C: Chunk lifecycle

Define an identity-indexed state machine spanning `struct flb_input_chunk` and
its `struct cio_chunk`: memory/file residency, up/down transitions, lock and
busy state, task and route custody, transaction commit/rollback, filesystem
accounting, and close/delete outcomes. Check normal, error, retry, flush, and
teardown paths independently.

The teardown proof must follow the concrete nested chain. `cio_destroy`
destroys every stream; `cio_stream_destroy` closes every linked chunk;
`cio_chunk_close` selects `cio_memfs_close` or `cio_file_close`, unlinks both
list nodes, releases the chunk name and object, and updates the context counter.
Backend teardown must additionally discharge buffers, mappings, file handles,
paths, and deletion state. `cio_stream_delete` and `cio_chunk_delete` must not
be treated as general object destructors: they delete backing content in their
documented contexts but do not by themselves establish the same complete
custody post-state as the destroy/close chain.

### 19D: Monkey and Cycle 18 composition

Add contracts for `evutil_closesocket`, `evutil_socket_geterror`, and
`evutil_socketpair`; import Monkey event/map and descriptor custody; model
nullable event creation and failed registration; and summarize `cb_event`,
`cb_timeout`, and `cb_wait_2_timeout`. Bind the selected runtime backend and a
deployed per-dispatcher observation hook to the Cycle 18 build profile before
issuing an event-model certificate.

This workstream is also the application-specific callback-summary gate for
Cycle 18. The two cycles must share one recorded observation rather than claim
independent evidence for the same adapter execution.

### 19E: Formal composition and audit

Extend the Lean executable account with contract transfer, nullable
acquisition, replacement, container custody, chunk lifecycle, and Monkey event
composition. Prove that accepted transfers conserve identities, failed
replacement preserves old custody, successful replacement invalidates it once,
and accepted chunk/event teardown leaves no live owned obligation.

Rerun the same pinned audit after every foundational workstream. Report source
functions, projected functions, projection failures, and Safe/Unsafe/Unknown
units separately; only fully discharged functions count toward proved
coverage.

## Acceptance gates

Cycle 19 is complete only when:

- the pinned source and compilation-database identity checks still pass;
- every trusted library contract states parameter modes, result mode,
  nullability, failure behavior, and destruction/transfer effects;
- contract mismatch, unsupported control flow, unresolved aliasing, and
  unmodeled concurrency remain fail-closed;
- the central input-chunk wrapper has a checked lifecycle result for every
  selected function, with `Unknown` retained for any unmodeled path;
- a ChunkIO `Safe` result includes the nested `cio_destroy` → stream destroy →
  chunk close → backend close teardown certificate, including list unlinking,
  counter discharge, and error exits; delete-only functions are classified by
  their actual partial post-state rather than assumed to be destructors;
- the forced-Libevent Monkey adapter either produces a Cycle 18-composed event
  certificate or an explicit `Unknown` reason for every uncovered callback;
- no aggregate claim treats provisional `Unsafe` classifications as confirmed
  Fluent Bit defects;
- the Cycle 19 Lean dependency audit has no placeholders or project axioms;
  and
- `cargo test --workspace`, formatting, Clippy with warnings denied,
  `lake build`, and the pinned external audit all pass.

## Implemented 19A foundation

`ResourceFunctionContract` now records parameter and return effects while the
older `ResourceApiKind` interface remains a compatibility layer. The resource
CFG supports edge-sensitive null tests for tracked nullable results. A
reallocation result retains a conditional link to the old allocation: the
null edge deletes only the failed new allocation, while the non-null edge
releases the replaced identity. Direct assignment that loses the old pointer
therefore remains an `Unsafe` leak on the failure path.

The abstract state also has aggregate-custody primitives: a stored child is
discharged or transferred when its owner is discharged or transferred,
audited removal restores direct child custody, and independent or wrong-owner
child removal/destruction is a definite custody conflict. Applying these
primitives to ChunkIO still requires audited constructor and nested teardown
summaries; the current ChunkIO units are not marked Safe by the primitives
alone.

The resource CFG now imports `for`, `do`/`while`, `switch`/`case` fallthrough,
labels, cleanup `goto`, `break`, and `continue` as explicit edges. Empty
`for (;;)` conditions retain only the loop edge, `switch` without `default`
retains an unmatched exit, and unresolved jump targets remain `Unknown`. The
Lean `QCore.ControlFlow` certificate checks that declared control edges are
present, resolved, and attached to retained CFG nodes.

The Cycle 19 audit manifest now marks known allocation results nullable, marks
null-tolerant destructors explicitly, and models `rd_realloc` with the new
replacement contract.

## Implemented 19B custody composition

`external_integration` adds an identity-indexed checker for intrusive-list,
typed-queue, arena, and reference-count custody. Retains are counted even when
one execution context owns multiple references. Store and removal transfer one
reference, arena/container destruction can discharge member references, and
the checker rejects missing containers, invalid holders, underflow-shaped
releases, nonempty non-cascading destruction, overflow, and terminal live
resources or containers.

The imported-library wrapper additionally pins the Fluent Bit revision and
scopes mechanism summaries per bundled library: librdkafka may use its
refcount, list, typed-queue, destructor, and evidence-gated concurrency
summaries; CFL may use list, arena, destructor, publication, and handoff
summaries; and ChunkIO may use its list and destructor summaries. A missing
required summary or a summary outside that library's profile is rejected.

Atomic publication, thread handoff, and reclamation are accepted only with a
matching typed synchronization-evidence identifier. The certificate records
the exact evidence consumed.

The pinned audit now derives those library profiles from eight source
contracts: librdkafka refcount, typed-queue/intrusive-list handoff, and queue
destruction; CFL intrusive-list and arena lifetime; and ChunkIO context,
stream, and chunk teardown. Revision, required files, and semantic fragments
are checked before a profile is emitted. The librdkafka profile intentionally
does not infer ownership publication or reclamation from scalar atomic
primitives. Mapping each accepted source call/path to concrete identity-indexed
custody operations remains pending.

## Implemented 19C bounded lifecycle

`chunk_lifecycle` is an identity-indexed executable checker for the bounded
ChunkIO/Fluent Bit lifecycle profile. It tracks context, stream, and chunk
custody; memory/file backend resources; up/down residency; both intrusive-list
links; the context chunk counter; lock, busy, task, and route obligations; and
transaction commit or rollback. Context destruction follows the audited
context → stream → chunk → backend order and rejects teardown while any wrapper
obligation is outstanding.

Construction failures leave no list or counter residue. File deletion through
`cio_file_close` is recorded only as requested because its `void` API logs but
does not return native deletion failure. Stream deletion therefore retains an
explicit success/failure partial state and is not confused with stream object
destruction.

The corresponding Lean account checks replacement, aggregate custody, unique
identities, backend discharge, list/counter balance, failed-open rollback,
nested teardown order, and delete-only classification. The new import
composition gate requires the pinned Fluent Bit revision, an explicit claim
that the imported CFG is complete, and every exact lifecycle summary used by
the operation trace before invoking `chunk_lifecycle`. Missing source identity,
control-flow coverage, or a used summary fails closed.

The unrestricted-C importer does not yet automatically derive this operation
trace from every raw ChunkIO and wrapper function, so the gate does not promote
any translation unit to `Safe` and the recorded audit counts remain unchanged.

## Implemented 19D Monkey composition

The Cycle 18 API profile now includes the three Libevent descriptor utilities
called by Monkey. `evutil_socketpair` has explicit success/failure out-custody,
`evutil_socket_geterror` is a borrowed query, and `evutil_closesocket` consumes
close custody only when the close succeeds. The event checker rejects duplicate
close, use after close, active-callback close, and terminal live descriptor
obligations. Its pinned source contract includes `event2/util.h` and
`evutil.c`, and the Monkey audit verifies those files are byte-identical to the
Cycle 18 checkout.

The Lean account proves successful pair creation is distinct, pair failure
creates no custody, close failure retains custody, successful close discharges
it once, error query preserves custody, and accepted teardown has no live
owned descriptor.

`check_monkey_composition` now joins the Monkey wrapper obligations to that
event certificate. It requires exact summaries for all three callbacks:
`cb_event` borrows its argument and writes the fired-event array; `cb_timeout`
borrows its argument and may close the descriptor and release the event map;
and `cb_wait_2_timeout` is a borrowed no-effect callback. The gate also requires
the pinned Fluent Bit source revision, event-map custody, nullable base/event
failure paths, failed registration,
an accepted build profile matching the base created by the event program, one
matching dispatcher, a matching observed backend, and an observation trace
refining the checked event trace.

The generic Lean `QCore.ExternalIntegration` account proves the accepted
custody, chunk-import, callback, backend/dispatcher/trace, and teardown
conditions without project axioms. Its module and theorem identifiers are
cycle-neutral. No deployed Monkey observation was produced by this change, so
the current aggregate remains `Unknown` until real hook evidence and a
source-derived wrapper trace are supplied.

## Native QCore contract corpus

The implemented library and composition boundaries are retained as native
QCore sources under `contract/qcore`, grouped by librdkafka, CFL, ChunkIO, and
Monkey. They record resource types, owned and borrowed interfaces, normal and
error postconditions, quantitative custody relations, delete-only partial
states, import completeness, callback summaries, and observation refinement.

The source front end now exposes function contracts as structured clauses with
source spans and preserved predicate text. The regression corpus recursively
parses, round-trips, and compiles every `.qcore` file, pins every
contract-bearing declaration and clause count, and checks the critical
handoff, teardown, import, callback, and observation predicates. This keeps the
contracts directly inspectable and prevents a fixture from disappearing or
silently losing its structural coverage. Arbitrary contract predicates remain
syntax-preserved rather than certificate-producing until their semantic
checker is connected; compiling the corpus does not promote an external unit
to `Safe`.

## First native-contract source binding

`chunk_source` now imports a bounded class of single-chunk field functions from
Clang's typed AST. It checks the actual parameter and field types, rejects
bitfields and volatile/atomic fields, retains ordered assignments and branch
decisions, and evaluates both Boolean entry values. Every statement must be
modeled, including statements on unexecuted paths. Calls, aliases, loops,
unsupported expressions, and missing returns remain `Unknown`.

The contract definitions are native QCore files:
`contract/qcore/fluent_bit/busy_state.qcore` and
`contract/qcore/chunkio/lock_state.qcore`. `chunk_contract` interprets their
bounded predicate vocabulary and checks all postconditions against the C-derived
paths, including the other field's frame condition. Changing a QCore
postcondition changes the semantic verdict. Rust contains the importer and
predicate semantics, not duplicate per-function postcondition tables.

The pinned-source run checks `flb_input_chunk_release_lock` and
`cio_chunk_is_locked` over four Boolean ghost pre-states each. Release-lock's
error path returns -1 without changing busy; its success path clears busy and
returns 0. Neither path unlocks the underlying ChunkIO chunk. Querying the
ChunkIO lock preserves both fields. These claims assume a live borrowed chunk,
exclusive access, Boolean field values, and correct caller identity binding.

The audit now reports every discovered function in the two selected files:
the wrapper has 1 checked native contract and 65 `Unknown` functions;
`cio_chunk.c` has 1 checked native contract and 29 `Unknown` functions. A
function summary can instantiate operations at a concrete live chunk identity
and compose with the existing teardown checker. A checked execution prefix is
kept distinct from a terminal lifecycle certificate. Neither file is promoted
to a whole-unit `Safe` verdict.

`QCore.ChunkSource` adds executable field semantics and proves that an accepted
summary covers every Boolean entry state, accepted predicates hold on those
paths, and busy effects preserve the lock and other identities. Clang and the
AST-to-IR bridge remain explicit trusted boundaries; this is not a proof of
arbitrary C or arbitrary QCore predicates. Full results and the remaining gates
are recorded in `../reports/cycle19-source-contracts/README.md`.


## Native execution evidence and entry-free contract modules

The actual pinned `flb_mem.h` is compiled with its selected Fluent Bit build
profile and instrumented allocator calls. Its five wrappers are evaluated
against `contract/qcore/fluent_bit/memory.qcore` on 145 executions. The
postconditions are defined in QCore; Rust provides a general bounded predicate
interpreter, not a duplicate per-function behavior table. Nonzero-size realloc
failure preservation is explicitly conditional. Zero-size realloc, alternate
allocator profiles, arbitrary caller provenance, and exhaustive C verification
are not claimed by these runtime checks.

The Monkey harness now records all three actual Libevent callbacks on the
observed `epoll` backend, with per-base event identities, a dispatcher/thread
binding, callback entry/exit, registration, frees, descriptor closes, and
teardown. The native run passes address, undefined-behavior, and leak checks
for the instrumented harness. Static dependencies retain their build's
instrumentation level. This supplies real execution evidence for 19D, but does
not yet satisfy the source-derived access/identity trace or failure-path
requirements of `check_monkey_composition`.

The compiler supports `module contract;` and `module library;`, so contract
files no longer need dummy `main` functions. Closed `quant<n where n := 5>`
definitions now elaborate to dimensioned indices, with cycle, freshness,
constraint, and overflow checks. The Lean binding account proves substitution
and scope preservation; it does not prove the Rust graph resolver.

See [the native evaluation and remaining gates](../reports/cycle19-native-contracts/README.md)
and [the module/type syntax](../QCORE_MODULE_MODES.md). General 19B per-call
identity mapping, 19C nested source teardown/failure imports, and the remaining
19D source/observation refinement gates keep Cycle 19 in progress.


## Generation-bound Monkey runtime identities

The next 19D increment records descriptor generations across socketpair
creation, send/receive, event registration and callback entry, successful close,
and descriptor-number reuse. A schema-versioned Rust observation checker in the
Libevent plugin rejects stale identities, mismatched base/dispatcher/thread
bindings, malformed or incomplete traces, and outstanding teardown obligations.
The native runner now invokes this checker instead of relying only on callback
names and sequence continuity.

The actual pinned run contains 48 records, four acquired and closed descriptor
identities, two reused descriptor numbers, all three callbacks, and one injected
socketpair failure through the actual Monkey channel wrapper. The generation
slot model has six new Lean theorems in the Cycle 19 dependency audit. All 145
allocator executions still check; generic source audit verdicts are unchanged.

This closes descriptor binding for the recorded runtime projection only. Source
callback/access refinement and allocation/registration failure imports remain
open, as do the general 19B and 19C source identity/teardown workstreams. No new
whole-source or plugin safety certificate is issued. See the [descriptor
observation report](../reports/cycle19-descriptor-observations/README.md) for
reproduction, evidence, validation, and the precise boundary.


## Source-derived Monkey callback access paths

The next 19D increment imports all three callbacks from the selected, pinned
Libevent adapter through Clang's typed AST. It retains four flag paths for
`cb_event`, three send/error paths for `cb_timeout`, and the single return path
for `cb_wait_2_timeout`. Typed parameter/read/call symbols preserve aliases,
field and fired-slot accesses, branch guards, casts, call arguments, local
lifetimes, and cleanup order. The Linux error-query macro is retained as its
actual errno-location call and read. Calls remain opaque: a close/free call is
not by itself successful custody discharge.

The importer checks every definition node before expansion and rejects
unsupported control flow, qualified or bitfield accesses, unresolved locals,
unsequenced call/read operands, and bounded-profile limit violations. The
external audit additionally byte-checks the textually included adapter and its
core layout/macro headers. Seven new Lean theorems audit typed symbol provenance
and both branch alternatives; Clang/Rust import and full C semantics remain
outside those proofs.

All 300 workspace tests, formatting, Clippy, Lean build/dependency audit, native
regression, and pinned source audit pass. Generic projection and safety counts
are unchanged. Source/runtime payload and map binding, callback-contract
checking under those bindings, and allocation/registration failure imports
remain open, alongside 19B and 19C. See the [callback source report](../reports/cycle19-callback-source/README.md)
for source paths, reproduction, evidence, and the precise remaining gates.

## Resource diagnostic inventory and importer recovery

The next increment inventories per-function diagnostics across all six audited
slices, including unresolved operations inside provisional Unsafe units. The
audit accepts an optional fourth positional argument for the complete JSON
inventory and prints categorized counts with representative preprocessed
locations. Generic Unknown messages distinguish unsupported operations from
unresolved aliases; type operators are no longer called uncontracted functions.

Balanced braced statements now retain the enclosing function and later cleanup,
while aggregate/statement-expression effects remain explicitly Unknown. Null
refinement additionally recognizes parenthesized resource places, negation,
equality/inequality, and preprocessed void-pointer zero. Pointer arithmetic and
nonzero casts are not accepted as null checks, and clearing a pointer does not
discharge its allocation.

The same pinned audit retains 4,688/4,775 functions, up from 4,346. Unit totals
are two Safe, 98 provisional Unsafe, 81 Unknown, four data-only, and zero gateway
failures. This is increased projection visibility, not 342 additional safety
proofs: most newly retained functions remain Unknown, and aggregate Unsafe
verdicts still contain unresolved diagnostics. Monkey and the selected native
chunk/callback contract coverage are unchanged. General 19B identity mapping,
19C lifecycle/teardown imports, and 19D source/runtime/failure composition remain
open. See [the triage report](../reports/cycle19-resource-triage/README.md) for
the full before/after inventory, validation, and next implementation targets.
