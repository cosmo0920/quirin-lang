# Cycle 19: Fluent Bit Resource and Event Integration

**Status: In progress — source-bound nested teardown, native QCore replay/identity rules and eight Monkey callback paths checked; native provenance, wider custody/lifecycle and full event composition remain**

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
QCore sources indexed by `contract/corpus.json`, grouped by librdkafka, CFL, ChunkIO, and
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
`providers/qcore-provider-chunkio/contracts/fluent_bit/busy_state.qcore` and
`providers/qcore-provider-chunkio/contracts/chunkio/lock_state.qcore`. `chunk_contract` interprets their
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
against `providers/quirin-provider-fluent-bit/contracts/fluent_bit/memory.qcore` on 145 executions. The
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

## Source/runtime callback binding and Lean verification

The 19D binder now connects all three freshly imported Monkey callback summaries
with actual registration arguments, pre/post memory snapshots, and intercepted
calls from the same generation-checked native observation used by Cycle 18.
The actual pinned run checks one execution of each callback and six native
QCore postconditions. It retains five unobserved source paths as explicit
`Unknown`; observed path indices are execution evidence, not universal path
proofs. Unsupported bindings and contracts remain fail-closed.

`QCore.CallbackBinding` verifies the abstract binding models with 20 audited
theorems covering scope equality, unique path selection, retained obligations,
typed memory/symbol binding, exact-cell writes, release invalidation/frame
properties, duplicate-release rejection, and ordered call consumption. Normal
Lean builds include the module. The dependency audit has no placeholders or
project axioms. C/Clang, the Rust interpreter, supplied native hooks and
snapshots, and allocation provenance remain explicit trusted boundaries.

All 318 Rust tests, formatting, Clippy, Lean build/audit, sanitized native
regression, and the pinned external audit pass. Generic projection and safety
counts remain unchanged. Allocation/registration failure source imports,
complete wrapper/map custody, and the full Cycle 18 event-program composition
remain `Unknown`; no static event or plugin certificate is issued. 19B and 19C
source custody/lifecycle work also remains open. See the [binding report](../reports/cycle19-callback-binding/README.md)
for evidence, reproduction, assumptions, and the exact remaining gates.

## Native replay rules and expanded callback/failure paths

The recent Rust callback-binding and descriptor-observation policies now live
in `monkey/callback_binding.qcore` and `monkey/descriptor_observation.qcore`:
20 executable contracts with 56 postconditions. The checkers supply typed facts
to the native predicate interpreter. Mutation regressions exercise every
extracted contract; false or unsupported predicates fail closed. Existing
callback postconditions remain in `monkey/composition.qcore`.

The expanded native harness exercises all eight callback source paths and
checks 18 callback postconditions on one shared 84-record observation. Seven
executions use native call results; one timer execution uses an explicitly
labeled injected `ECONNABORTED` send failure. Three event-mask cases use
controlled Libevent activation. Reports distinguish native and injected paths;
exercised paths are not universal proofs. Three new Lean theorems retain that
evidence distinction.

The source importer now reports every one of 15 selected Monkey wrappers:
eight produce 37 symbolic paths, and seven remain Unknown with specific
unsupported-source reasons. Loop creation and event update retain allocation
and registration failure exits, opaque calls and exact returns without
inventing cleanup. These source projections are not yet bound to complete
wrapper/map custody or a Cycle 18 event program.

All 323 Rust tests, formatting, Clippy, Lean build/audit, sanitized native run
and pinned external audit pass. Generic projection and Safe/Unsafe/Unknown
counts are unchanged. 19B identity mapping, 19C lifecycle/teardown imports,
unsupported Monkey wrappers, failure-path custody and full event composition
remain open. See [the native-rule report](../reports/cycle19-native-rules/README.md)
for extraction scope, evidence, reproduction and remaining gates.

## Generalization-aligned contract foundation (2026-09-06)

Execution now follows the [generalization plan](../GENERALIZATION_PLAN.md) and
[extended dependency roadmap](../ROADMAP_AFTER_CYCLE_9.md). The original proposal
is preserved unchanged; implementation progress is recorded here and in reports.
The first W1 increment supports the remaining 19B custody work by loading the
19-function resource environment from five native QCore modules selected by
`cycle19-resource-contracts.json`. The audit's Rust production table is
removed after pinned verdict/diagnostic parity; library source/profile routing
remains in place.

The loader maps an explicitly versioned, bounded custody vocabulary to the
existing parameter/result modes. Every parameter and result must be declared;
unknown predicates, richer requirements, error sentinels, conditional effects,
duplicate functions and missing inputs reject the load without a partial
fallback. Detailed observation and lifecycle modules retain their existing
semantic interpreters. The new resource projection modules declare assumptions;
they do not establish that the underlying C satisfies them.

Exact source texts accompany the resource environment in safety certificates.
Independent certificate replay reloads those texts and checks the resulting
contract table; replacing a loaded mode without changing its recorded source
invalidates the certificate. A resource-inventory sidecar records those same
inputs for non-Safe units. See the [loader parity report](../reports/cycle19-contract-loader/README.md)
for validation, signature differences, assumptions and reproduction.

The remaining 19B source identity mapping, 19C lifecycle/teardown imports, 19D
unsupported wrappers/failure custody/full event composition, and 19E complete
formal composition stay open. W1 supplies their contract input boundary; it does
not close them or bypass Cycle 20's semantic-witness firewall.

## Wrapper enum and stack-storage imports (2026-09-06)

The [wrapper storage report](../reports/cycle19-wrapper-storage/README.md)
records 41 symbolic paths from 10/15 wrappers, recovering socketpair forwarding
and timed wait. Explicit Clang enum values and bounded stack arrays/records are
retained with lexical identities; unsupported initialization and aggregate
callback replay remain Unknown. Four Lean storage-identity theorems are audited.
All checks pass and the pinned generic inventory and callback summaries remain
identical. The other five wrappers and full 19B–19E composition remain open.

## Wrapper assertion and terminal-control imports (2026-09-06)

The [wrapper control report](../reports/cycle19-wrapper-control/README.md)
records 81 symbolic paths from 14/15 wrappers. Literal-zero macro bodies,
nonreturning assertion exits, opaque direct calls and source-resolved array
typedefs are retained. Inject's variable-bound loop remains Unknown. Four new
Lean control theorems and 337 workspace tests pass along with formatting,
Clippy, Lean audit and the pinned external audit. Resource inventory and callback
summaries are unchanged. Terminal paths do not claim teardown, and source
projections do not close the remaining custody or composition gates.

## Source-validated ChunkIO helper composition (2026-09-06)

The [chunk helper report](../reports/cycle19-chunk-helper/README.md) records a
checked `cio_chunk_unlock` contract composed with the actual `cio_error_reset`
body. The helper's metadata write and exact source are retained, local helper
shadowing and unsupported effects reject composition, and the contract checks
all four busy/lock pre-states. The new sidecar inventories all 96 selected
functions: three checked field contracts and 93 Unknown. Five new Lean theorems
and all 340 Rust tests pass, along with formatting, Clippy, Lean build/audit and
the pinned audit. Generic resource inventory and Monkey source reports remain
identical. Cross-unit source/build/ABI and caller identity remain explicit
premises; full 19B–19E custody, lifecycle and event composition remain open.

## Structured injection scan import (2026-09-06)

The [scan-loop report](../reports/cycle19-scan-loop/README.md) records source
projections for all 15 selected Monkey wrappers. The previous 81 flat paths
remain; inject contributes two structured branches with explicit per-iteration
reads, induction state and function-level early return. Loop safety, bounds and
termination remain Unknown. Source report schema v2 exposes this distinction.
Four Lean scan-prefix theorems and all 342 Rust tests pass with the required
formatting, Clippy, Lean and pinned external checks. Generic resource counts,
the native chunk sidecar, callbacks and the other 14 wrappers are unchanged.
Full 19B–19E safety/custody composition remains open.

## Native extraction of new source contracts (2026-09-06)

The [contract extraction report](../reports/cycle19-contract-extraction/README.md)
records 26 new declarations across three QCore modules: ten executable source
policies, the executable `cio_error_reset` frame contract, and 15 syntax-preserved
Monkey wrapper behavior specifications. The importers consume the extracted
policies; the pinned audit checks the helper contract and records all exact
inputs. Wrapper behavioral/custody replay remains open. All 345 Rust tests,
formatting, Clippy, Lean build/audit and the pinned audit pass. Resource inventory,
callback/wrapper projections and native chunk-function results are unchanged.

## Source-derived memory backend custody (2026-09-06)

The [memory backend report](../reports/cycle19-memory-backend/README.md) records
an executable `cio_memfs_close` native contract over nine nullable ownership
states. All 13 postconditions check the source-derived child/object release,
order and borrowed-root frame. The backend pointer remains unchanged and may
dangle after close. Provenance, distinct owned fields, exclusive access and libc
binding remain premises. A shared typed C importer now serves Monkey and the
ChunkIO destructor checker with exact pinned projection parity. Seven new Lean
theorems and 348 workspace tests pass with all required checks. Complete chunk
close, list/counter discharge, filesystem backend and higher-level composition
remain open; generic resource verdicts and the 96 scalar results are unchanged.

## Source-derived counter conservation (2026-09-06)

The [counter report](../reports/cycle19-counters/README.md) records four checked
unsigned counter helpers, with native contracts for field selection, update
direction, return identity and nonwrapping entry bounds. Their instantiation
rejects zero decrement and maximum increment. Six new Lean arithmetic and
conservation theorems and 351 workspace tests pass with the required checks.
All prior pinned verdicts, Monkey/scalar results and memory-backend evidence
are identical. The actual close call's context identity and list/teardown
composition still need binding; these helper results do not close Cycle 19.

## Source-derived intrusive-list unlink (2026-09-06)

The [list unlink report](../reports/cycle19-list-unlink/README.md) records native
contracts for `mk_list_del` and its source-validated helper. Both distinct and
aliased neighbor cases check all four writes, reconnection, entry clearing and
preserved object custody. Six new Lean theorems and 354 workspace tests pass
with every required check. Prior pinned verdicts and backend/counter evidence
remain identical. Binding the two embedded chunk links and composing the full
close path with backend and context counter discharge still remain open.

## Source-derived memory chunk close (2026-09-06)

The [memory close report](../reports/cycle19-memory-close/README.md) composes
source-checked reset, backend, both embedded unlinks and the saved-context
counter for `cio_chunk_close`. Null/memory paths 0 and 1 are checked; filesystem
and other-backend paths 2 and 3 remain uncovered. A concrete identity binding
feeds the core lifecycle checker and rejects repeat close and outstanding
wrapper obligations. Lifetime, membership, constructor provenance and ABI
premises remain explicit; backend root-disjointness is now required separately.
Six new Lean theorems, all 357 workspace tests and required checks pass. Generic
verdicts and previous projections remain unchanged. Stream/context iteration,
filesystem error outcomes and full library/event composition still remain open.

## Filesystem cleanup result-use boundary (2026-09-06)

The [filesystem attempts report](../reports/cycle19-filesystem-attempts/README.md)
records 20 complete paths across file close, unmap and their Unix native helpers.
Four native QCore policies check producer-use facts: the outer close ignores
both cleanup results before subsequent free calls; the lower functions guard
their selected cleanup results. OS outcomes and failure custody remain Unknown.
Three new Lean producer-use theorems and 361 workspace tests pass with every
required check. Prior verdicts and certificates are identical. This exposes the
filesystem composition gap explicitly and does not promote it to Safe; broader
source custody, stream/context and Monkey event work remain open.

## Native filesystem outcome branches (2026-09-06)

The [filesystem outcomes report](../reports/cycle19-filesystem-outcomes/README.md)
checks all eight null/absent/failure/success cases across Unix close and unmap.
Typed symbolic replay binds syscall argument identities, error returns, success
resets and failure field preservation under explicit external-call frames. A
failed close leaves OS custody unresolved even though its C fd field remains.
Two native policies, six Lean theorems, 364 workspace tests and required checks
pass with unchanged prior verdicts and certificates. Actual outcome observation,
wrapper failure composition and the wider Cycle 19 lifecycle gates remain open.

## Controlled native cleanup executions (2026-09-06)

The [cleanup observation report](../reports/cycle19-cleanup-observations/README.md)
records 18 sanitized executions of the actual pinned Unix helper bodies with
controlled close/unmap results. Both helpers' null/absent/error/success cases
check returns, call identities, diagnostics and endpoint field preservation or
reset against native QCore. All 365 workspace tests and required checks pass;
prior audit sidecars are byte-identical. These are injected outcomes, not actual
OS resource discharge or a full filesystem custody certificate. Wrapper failure
composition and the remaining lifecycle/event gates are still open.

## Conditional unmap/counter composition (2026-09-06)

The [unmap wrapper report](../reports/cycle19-unmap-wrapper/README.md) composes
all eight `munmap_file` source paths with rechecked native unmap and up-counter
dependencies. Native errors cause no wrapper write/decrement; native success
clears both sizes then decrements exactly once, with counter underflow rejected.
Sync/log framing, mappedness, identity and lifetime assumptions remain explicit.
Five Lean theorems, 368 workspace tests and required checks pass with unchanged
prior audit evidence. Full sync framing and outer close failure custody remain
open alongside the wider lifecycle, provenance and event gates.

## Non-deleting backend close and unresolved custody (2026-09-06)

The [file close report](../reports/cycle19-file-close/README.md) binds the actual
backend/path identities and no-delete cleanup/free sequence to rechecked unmap
and native close dependencies. It retains 43 conditional cases: 17 have no
nested obligation under the premises, while 26 fail the nested-discharge gate
because OS custody remains unresolved despite backend allocation release.
Deletion paths remain uncovered. Eight Lean theorems, 371 workspace tests and
required checks pass with unchanged prior verdicts/certificates. Actual outcome
and caller/provenance binding, full sync/deletion handling, enclosing filesystem
chunk close and the remaining lifecycle/event gates stay open.

## Shared short-circuit condition import (2026-09-06)

The [short-circuit report](../reports/cycle19-short-circuit/README.md) adds bounded
`&&`/`||` condition paths to the shared typed importer, preserving left-to-right
sequencing and skipped effects. The native deletion helper now has four retained
source paths and a guarded-unlink-result policy, while deletion custody remains
Unknown. Eight Lean theorems, 375 workspace tests and required checks pass with
byte-identical prior source/verdict evidence. Unsupported value contexts and
path overflow still reject. Deletion outcomes, full sync/caller/OS binding,
enclosing filesystem close and the wider lifecycle/event gates remain open.

## Explicit filesystem storage frames (2026-09-06)

The [cleanup frame report](../reports/cycle19-cleanup-frames/README.md) strengthens
unmap and non-deleting close with explicit disjoint-storage/root premises and
extends mutation coverage to every native precondition. All 375 workspace tests
and required checks pass. Apart from the exact strengthened contract texts,
all source/case/effect/residual evidence and generic verdicts remain unchanged.
Caller allocation provenance remains an obligation, and Cycle 19's outstanding
deletion, sync, OS binding, traversal and event-composition work stays open.

## Native deletion outcomes (2026-09-06)

The [deletion outcome report](../reports/cycle19-delete-outcomes/README.md) binds
five eligibility/outcome cases to all four native delete source paths. It checks
original path identity, return/diagnostic behavior and unchanged allocations
under explicit path/lifetime/external-call premises. Five Lean theorems and 378
workspace tests pass with every required check and identical prior evidence.
Actual deletion observation and outer deleting-close composition remain open,
along with full sync, OS/provenance binding, traversal and event composition.

## Deleting backend close composition (2026-09-06)

The [deleting close report](../reports/cycle19-deleting-close/README.md) connects
post-cleanup fields to checked native deletion and its outer error diagnostic.
Thirty new deleting cases and 43 unchanged non-deleting cases cover all four
source paths. Only eight deleting cases conditionally establish success; null,
blocked and failed requests remain distinct through backend release. Three new
Lean theorems, 381 workspace tests and all required checks pass with identical
prior evidence. Full sync frames, real OS/caller binding, enclosing filesystem
chunk close, traversal and full event composition still remain open.

## Controlled native deletion observations (2026-09-06)

The [native delete observation report](../reports/cycle19-delete-observations/README.md)
records 16 sanitized executions of the pinned native-delete body: 14 blocked,
one eligible failure and one eligible success. The original 18 native cleanup
executions still pass unchanged. A new 12-clause QCore policy evaluates eligibility,
argument identity, status, diagnostics and preserved record/path endpoints.
All 382 workspace tests and 38 contract modules pass with the required checks.
The complete source evidence and aggregate inventory remain unchanged. Controlled
hooks do not establish actual filesystem deletion or close the remaining sync,
OS/caller provenance, enclosing teardown, traversal and event-composition gates.

## Enclosing filesystem chunk close (2026-09-06)

The [filesystem chunk close report](../reports/cycle19-filesystem-chunk/README.md)
composes the checked file backend with error reset, both embedded list removals,
name/chunk release and saved-context total decrement. Seventy-five conditional
cases retain 39 unresolved OS cleanup outcomes; eight establish conditional
deletion success. Null chunks and null backends remain distinct. The filesystem
scope checks two source paths; together with memory scope, only the unsupported
backend-kind path remains uncovered. A 22-clause QCore policy and six new Lean
theorems preserve the residual outcomes through enclosing allocation release.
All 385 workspace tests and 39 contract modules pass with required checks and
identical prior evidence. This does not establish full sync or caller/OS frames,
constructor provenance, stream/context traversal or wider event composition.

## Source-bound teardown traversal (2026-09-06)

The [traversal report](../reports/cycle19-teardown-traversal/README.md) checks the
complete typed bodies of `cio_chunk_close_stream` and `cio_stream_destroy_all`
against expanded references that input macros cannot rewrite. Saved-next order,
sentinel exclusion, primary-link container recovery and exact child arguments
are retained. Native ring/provenance and child destructor frames remain premises;
source matching discharges no child custody. Six Lean finite-traversal theorems,
388 workspace tests, 40 contract modules and all required checks pass. Prior
source/diagnostic/certificate evidence is unchanged. Enclosing allocation release
and identity/dependency composition remain open with the wider lifecycle/event gates.

## Stream and context parent release (2026-09-06)

The [parent teardown report](../reports/cycle19-parent-teardown/README.md) checks
complete `cio_stream_destroy` and `cio_destroy` bodies, retaining null guards,
child-first traversal, stream unlinking and all optional context-field frees.
Parent release never discharges a child's unresolved obligation. Four Lean
theorems and an 11-clause QCore policy state this conditional boundary. The
shared source matcher now preserves offsetof token boundaries; a regression
rejects colliding struct/typedef spellings. All 392 workspace tests, 41 modules
and required checks pass. Prior evidence is unchanged apart from the two
traversal witnesses' explicit offsetof token representation. Concrete nested
identity/dependency composition and the broader custody/event work remain open.

## Source-bound nested memory teardown (2026-09-06)

The [nested memory report](../reports/cycle19-nested-memory/README.md) composes
all six context/stream/chunk/backend stages with reset, unlink and counter
dependencies. An ordered, complete ring binding instantiates concrete model
identities and checks the resulting lifecycle prefix. Missing/duplicate/foreign
or filesystem children and outstanding wrapper obligations reject; unrelated
contexts remain live. Native allocation/ring provenance, component preconditions
and cross-unit ABI/call bindings remain explicit premises in a 10-clause policy.
All 396 workspace tests, 42 modules and required checks pass with identical prior
audit evidence. Filesystem residual composition and the broader native custody,
sync/OS, constructor and event-composition gates remain open.

## Residual custody through nested parent teardown (2026-09-06)

The [nested cleanup report](../reports/cycle19-nested-cleanup/README.md) composes
the full context/stream source chain with all 43 non-deleting filesystem cases.
Twenty-six retain unresolved OS obligations through parent allocation release;
17 conditionally discharge them. Exact child identities, total-counter equality,
up-counter subtraction and residual up obligations remain explicit. Five Lean
theorems, a 12-clause QCore policy and mutation tests cover residual identity
preservation and discharge rejection. Reference matching now rejects corrupted
record-field types even when both compared bodies share those declarations.
All 400 workspace tests, 43 modules and required checks pass with identical prior
pinned evidence. Native caller/constructor/ring provenance, full sync/OS frames,
unsupported backend classification, general aggregate custody, remaining wrapper
lifecycle functions and full Monkey/Cycle 18 event composition remain open.

## Native constructor and memory teardown observations (2026-09-06)

The [native teardown report](../reports/cycle19-native-teardown/README.md) records
five real pinned memory-constructor/destructor executions. All fourteen selected
ChunkIO/CRC units are rebuilt with ASan/UBSan; leak checking also passes. The
observer records 82 allocation generations, reciprocal embedded-list snapshots,
13 chunk-counter decrements and exact nested release order. An independent
structural adapter rejects altered records and evaluates an 11-clause QCore
runtime policy. Five Lean theorems model the final exact-release-plan gate only.
All 400 workspace tests, 44 modules and required checks pass with unchanged
pinned evidence. Instrumentation and exclusive harness execution remain trusted;
these observations neither prove arbitrary callers nor promote raw units. Native
allocation failures, general caller provenance, full sync/OS guarantees, aggregate
custody, wrapper lifecycle and full Monkey/Cycle 18 composition remain open.

## Native constructor allocation-failure observations (2026-09-06)

The [constructor failure report](../reports/cycle19-constructor-failures/README.md)
injects one ENOMEM at each of nine allocation attempts on the pinned native
context/stream/chunk/metadata memory path, plus a successful control. All 55
acquired generations are accounted for: six rollback releases and 49 releases
through surviving-tree teardown. ASan, UBSan and leak checking pass, as do the
five original positive cases. Context construction can succeed with null chmod;
chunk construction can succeed with a null name; backend allocation failures
return a null chunk while retaining zero error output. These observed outcomes
are preserved rather than treated as universal successful construction. Seven
Python tests include 48 mutation subcases; the new 11-clause QCore runtime policy
and two Lean rollback/teardown theorems pass with 400 workspace tests, 45 modules,
all standing checks and identical pinned audit evidence. General callers, other
failure combinations, full filesystem sync/OS guarantees, unsupported backend
classification, aggregate custody, wrapper lifecycle and full event composition
remain open. No raw unit is promoted.

## Unsupported backend close classification (2026-09-06)

The [unsupported close report](../reports/cycle19-unsupported-close/README.md)
classifies the remaining nonnull `cio_chunk_close` path for the complete integer
domain excluding the source-probed memory and filesystem constants. A symbolic
class avoids representative-value sampling; extra backend comparisons, truth
tests that split the class and changed effects reject. The source skips backend
cleanup, releases both links/name/chunk and decrements only the total counter.
Backend custody and constructor reachability stay unresolved, with explicit
`Unknown` and no nested-discharge permission. Twelve native QCore clauses, four
Rust tests and eight Lean theorems record that boundary. Existing null/memory/file
certificates and aggregate verdicts remain unchanged. Propagating this opaque
obligation through parent teardown, native caller/failure provenance, full
filesystem guarantees, aggregate custody, wrapper lifecycle and full event
composition remain open. No raw unit is promoted.

## Opaque backend custody through parent release (2026-09-06)

The [opaque parent teardown report](../reports/cycle19-nested-unsupported/README.md)
composes the symbolic unsupported close class with the checked context/stream
chain. Exact stream/chunk/backend-kind identities survive both parent releases.
Typed filesystem residuals and opaque obligations are tracked separately; empty
typed OS lists cannot erase unclassified backend custody. Source/dependency
mismatch, known discriminants in the unsupported class, missing evidence,
duplicate identities and counter mismatches reject. The existing 12-clause
policy counts both inventories for its discharge gate. Three Rust tests and
three Lean theorems cover the new composition; previous source cases remain
unchanged. These conditional live-child premises do not establish a native
constructor result. General caller/failure provenance, full filesystem
guarantees, aggregate custody, wrapper lifecycle and event composition remain
open. No raw unit is promoted.

## Native unsupported-type constructor observations (2026-09-06)

The [unsupported constructor report](../reports/cycle19-unsupported-constructors/README.md)
executes four pinned native stream/chunk constructor inputs: `INT_MIN`, `-1`,
`2` and `INT_MAX`. Streams preserve the requested types; chunk open returns
null, leaves the error sentinel unchanged and rolls back its name and object.
All 24 acquired generations are released: eight during constructor rollback and
sixteen during surviving empty-stream/context teardown. The shared tree checker
accepts non-memory streams only when no chunk/backend is interpreted. Ten Python
tests, an 11-clause QCore runtime policy and three Lean recorded-result theorems
cover the observation boundary. All four new native runs and the five positive
plus ten allocation-failure/control regressions pass ASan/UBSan/leak checking.
These four observations do not establish arbitrary constructor reachability,
general caller/failure provenance, full filesystem guarantees, aggregate source
custody, wrapper lifecycle or full event composition. Cycle 19 remains open and
no raw unit is promoted.

## Real Linux cleanup endpoint observations (2026-09-06)

The [OS cleanup report](../reports/cycle19-file-os/README.md) records twelve
sanitized close/unmap/delete helper executions with forwarding syscall wrappers.
Fresh temporary fixtures bind descriptor and mapping endpoints and an inode
hardlink witness; a zero-length unmap failure leaves its real mapping live.
Missing-path deletion and stale-descriptor failure are distinct from successful
cleanup and blocked deletion. A 13-clause runtime policy, independent trace
adapter, 31 trace mutations and four Lean observation-gate theorems check the
bounded evidence. All standing checks and pinned source/verdict parity pass.
Arbitrary OS failures, full sync/durability, enclosing native filesystem teardown,
general caller provenance, aggregate custody, wrapper lifecycle and full event
composition remain open. No raw unit is promoted.

## Native filesystem constructors and residual teardown (2026-09-06)

The [filesystem tree report](../reports/cycle19-file-tree/README.md) binds six
native one-file executions to constructor generations, reciprocal rings, exact
nested release order and original OS identities. All 66 allocation generations
are released. Suppressed unmap and close calls returning EIO retain respectively
a mapping/up-counter and a descriptor after all enclosing allocations are freed.
Suppressed sync still allows cleanup, while the backend's synced field stays
false. The harness records residuals before separately reclaiming its fixtures.
A 16-clause policy, 40 trace mutations and four additional Lean observation-gate
theorems check this boundary. All standing checks, 19 existing sanitized native
regressions and pinned source/verdict parity pass. These fixed one-file Linux
cases do not prove general constructor/caller/failure paths, full sync/durability,
aggregate source custody, remaining wrapper lifecycle or full event composition.
Cycle 19 remains in progress and no raw unit is promoted.

## Native filesystem constructor allocation failures (2026-09-06)

The [filesystem constructor failure report](../reports/cycle19-file-constructor-failures/README.md)
records eleven ENOMEM injections and a control in fresh native processes. Ten
returning cases complete teardown and leak checking. Two UBSan null-argument
terminations after failed root/chunk-name copying retain incomplete prefixes:
no constructor result or completed teardown is invented. The 81 acquired
generations partition into 17 temporary, 10 rollback and 46 teardown releases,
plus eight outstanding prefix generations. A returned stream without a directory
and subsequent ENOENT chunk failure remains distinct from null-stream rollback.
A 16-clause policy, 55 trace/process mutations and four Lean prefix-accounting
theorems preserve these boundaries. General callers/options, multiple failures,
OS failure combinations, full sync/durability, aggregate source custody, remaining
wrapper lifecycle and full event composition remain open. No raw unit is promoted.

## Native constructor OS-failure rollback (2026-09-06)

The [constructor OS-failure report](../reports/cycle19-file-os-constructors/README.md)
records eight fresh native processes with ten selected suppressed OS calls.
All 88 allocation generations are released: 16 temporary, 24 rollback and 48
teardown releases. First/second fstat failures, stat fallback, POSIX resize
return/errno and MAP_FAILED outcomes remain distinct. A suppressed close after
map failure retains the original descriptor/inode after null chunk return and
parent allocation release; harness reclamation is recorded separately. All eight
runs complete ASan/UBSan/leak checking. A 17-clause policy, 60 trace/process
mutations and four Lean rollback-ledger theorems check the bounded observations.
Arbitrary callers/options, ACL failures, existing-file and residency/transaction
paths, other combined OS failures, full sync/durability, aggregate source custody,
remaining wrapper lifecycle and full event composition remain open. No raw unit
is promoted.

## Existing-file profile observations (2026-09-06)

[Seven native existing-file profiles](../reports/cycle19-file-existing/README.md)
release all 70 allocation generations: seven temporary, 20 rollback and 43
parent-teardown releases. Original inode/bytes, read-only mapping permissions,
malformed-magic/short-header/empty-read-only errors and a descriptor residual
after suppressed chmod plus rollback-close failures stay distinct. A 17-clause
policy, strict trace mutations and four Lean theorems check this bounded evidence.
General profiles, residency/transaction paths, caller provenance, full OS and
sync guarantees, aggregate source custody, wrapper lifecycle and full event
composition remain open. No raw unit is promoted.

## Native residency and transaction observations (2026-09-06)

[Five native sequences](../reports/cycle19-file-transitions/README.md) release
all 50 allocation generations and seven descriptor/mapping epochs. The adapter
checks 26 operation results, ring/counter transitions and separate logical/header
lengths. Rollback restores logical length without restoring the mapped header;
down/up reloads appended bytes. Locked begin returns retry while direct write
succeeds. A 16-clause runtime policy and six executable Lean projection theorems
preserve those outcomes. Failed transition syscalls, capacity/metadata/checksum
variants, arbitrary production callers, full sync/durability, aggregate custody,
remaining wrapper lifecycle and full event composition remain open. No raw unit
is promoted.

## Native synchronization failure and retries (2026-09-06)

[Three native one-shot sync-failure sequences](../reports/cycle19-file-sync-failures/README.md) release all 30 allocation
generations and four OS epochs. Failed commit retains the active transaction;
retry clears it. Down still unmaps/closes after failed sync, and commit while
unmapped clears the transaction without syncing. An 18-clause policy, 73 trace/
process mutations and five Lean theorems preserve these bounded observations.
Other transition failures, general callers, full sync/durability, aggregate
custody, wrapper lifecycle and full event composition remain open. No raw unit
is promoted.

## Failed native unmap and down recovery (2026-09-06)

[Four native failed-down sequences](../reports/cycle19-file-unmap-failures/README.md) release all 40 allocation generations and
seven OS epochs. Suppressed unmap retains the original mapping, descriptor,
up ring and counter. A successful preceding sync remains visible after failure;
combined sync/unmap failure remains unsynced. Retry or parent teardown later
releases the original epoch. A 19-clause policy, 75 mutations and five Lean
theorems preserve this boundary. Failed close/reopen, other sync failures,
general callers, full sync/durability, aggregate custody, wrapper lifecycle and
full event composition remain open. No raw unit is promoted.

## Native down/reopen descriptor failures (2026-09-06)

[Four native sequences](../reports/cycle19-file-descriptor-failures/README.md)
release all 40 allocation generations and acquire five descriptor/mapping epochs.
Failed close after unmap moves the chunk into the down ring with a zero up
counter but retains its original descriptor. Down/up retries reject without
releasing it. Parent teardown either releases it or, with a second suppressed
close, leaves an observed descriptor residual after enclosing allocations are
freed. Harness reclamation remains separate. Suppressed reopen creates no new
epoch; successful retry acquires one. The independent adapter, 20-clause runtime
policy and seven Lean theorems preserve this conditional observation boundary.
General reopen size/map failures, production caller provenance, aggregate custody,
remaining wrapper lifecycle and full event composition remain open. No raw unit
is promoted.
