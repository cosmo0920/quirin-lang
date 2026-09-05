# Cycle 19: Fluent Bit Resource and Event Integration

**Status: In progress — contract foundation and bounded ChunkIO lifecycle implemented**

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

The abstract state also has the first aggregate-custody primitive: a stored
child is discharged or transferred when its owner is discharged or
transferred, while independent child destruction is a definite custody
conflict. Applying this primitive to ChunkIO still requires audited constructor,
list-removal, and nested teardown summaries; the current ChunkIO units are not
marked Safe by the primitive alone.

The Cycle 19 audit manifest now marks known allocation results nullable, marks
null-tolerant destructors explicitly, and models `rd_realloc` with the new
replacement contract. Aggregate/container transfer and general control-flow
work remain pending.

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
nested teardown order, and delete-only classification. This bounded checker is
not yet composed with the unrestricted-C importer, so it does not promote any
raw ChunkIO or Fluent Bit translation unit to `Safe`. Importer composition,
remaining control flow, and Monkey event composition remain Cycle 19 work.
