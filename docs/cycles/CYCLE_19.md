# Cycle 19: Fluent Bit Resource and Event Integration

**Status: In progress — first typed-source field contracts checked from native QCore; general custody/lifecycle imports and deployed Monkey observation pending**

Cycle 19 turns the pinned Fluent Bit library projection from Cycle 19's initial
measurement into bounded resource and event certificates. It does not claim
that a high parser-projection percentage is memory-safety coverage, and it does
not reinterpret a conservative `Unsafe` result as an upstream defect.

The source and build boundary remains the one recorded in
`CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md`: pinned CMetrics, CTraces,
librdkafka, ChunkIO, Fluent Bit's central input-chunk wrapper, and a separate
Monkey build forced onto its bundled Libevent adapter.

Library-specific mechanisms remain in core modules for this milestone so their
semantics and evidence can stabilize. Their extraction into separately
versioned, process-isolated verifier providers is Cycle 20; Cycle 19 APIs should
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
