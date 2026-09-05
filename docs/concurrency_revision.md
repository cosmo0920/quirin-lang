# QCore-C revision 0.2 — Checked Concurrency Amendment

**Status:** Proposed research specification; not a claim of implementation or proof completion.  
**Base:** Uploaded `qcore_quantitative_types_spec(1).md`, revision 0.1.  
**Audience:** Implementation and verification LLM agents.  
**Date:** 2026-09-05.  
**Conformance profile:** `checked-concurrency-0.2`.  
**Normative vocabulary:** MUST, MUST NOT, SHOULD, MAY retain the meanings in the base specification.

This amendment is incorporated into the consolidated revision 0.2 document. It preserves the base distinctions between usage multiplicity, static indices, credits, live obligations, costs, effects, and exit states. New concurrency rules below take precedence over corresponding sequential-only rules when the concurrency profile is enabled. This is a proposed extension, not content attributed to the uploaded revision.

## 38. Scope, Feasibility, and Guarantee

### 38.1 Design decision

Checked multithreading is feasible as a compositional extension of the quantitative checker **provided access authority is indexed by resource identity and footprint, and all inter-thread transfers have synchronization semantics**.

Usage grades and resource-count vectors alone MUST NOT be used as a race-freedom argument. Two pointers can each be used once while racing on the same allocation. Conversely, multiple reads can be safe without exclusive ownership in every reading thread.

The extension uses bounded, splittable access permissions, ownership/lifetime obligations, synchronization contracts, and existing path-sensitive exits. Fractional permissions and concurrent separation logic are established foundations [C1, C2]. The precise integration with QCore error packs and quantitative flow is a design proposal requiring its own proofs.

### 38.2 Supported revision 0.2 core

The mandatory core supports first-order scoped tasks, move transfer, scoped shared and exclusive borrowing, statically proven disjoint access footprints, and mutex-protected fixed ownership invariants. Every successful task creation has exactly one explicit join on every checked scope-exit path.

The first executable milestone MAY support only whole-object footprints, but it MUST describe that subset as such. Full profile completion requires the disjoint-field/range rules below.

The supported C analyzer subset consists of annotated or recognized equivalents of these operations. Unstructured but provably matched create/join control flow MAY be normalized into the same task graph; unsupported patterns remain unknown.

### 38.3 Separate safety properties

`memory-race-free` means there are no conflicting accesses to a modeled C memory location by different threads without a happens-before ordering, where at least one access is non-atomic. The C11 baseline is WG14 N1570 §5.1.2.4 [C3].

`resource-conflict-free` means tracked resource operations obey their declared access and typestate protocols, including release versus use and competing exclusive transitions. This is a QCore property: it is not identical to the C definition of a data race. A kernel-managed handle may be memory-safe yet violate an application-level operation protocol.

`lifetime-safe` means no checked operation uses a resource after its lifetime ends and no resource is destroyed while a checked borrower can still access it.

The checker MUST report these claims separately from deadlock freedom, termination, fairness, linearizability, transaction atomicity, determinism, and wall-clock bounds. No latter property follows automatically from permission conservation.

### 38.4 Intended theorem

For every execution of a closed, well-typed program in this profile, every modeled non-atomic access and resource operation is authorized; conflicting operations in different threads are ordered by the supported synchronization protocol; and destruction cannot overlap a checked outstanding access. Quantitative live/peak/total bounds hold for the axes and runtime overhead covered by the selected cost model.

The theorem is conditional on valid foreign contracts, faithful primitive/runtime implementations, sound alias abstraction, correct target lowering, and the absence of execution outside the declared model. A report MUST list these assumptions. A linked component with unknown access effects prevents a whole-program guarantee for potentially affected state.

Race freedom is not complete C undefined-behavior freedom. Bounds, alignment, initialization, arithmetic, object lifetime, and other sequential validity obligations remain separate. A whole-program C-safety claim must discharge those obligations too; a race-only report must state its assumptions about them.

Safety is a property of finite execution prefixes. A blocked program may still be safe. No claim of eventual cleanup is made for an execution that never reaches a checked exit. The no-leak statement applies to completed checked normal and recoverable-error exits, not process abort or nontermination.

### 38.5 Deferred features

The safe core excludes detached tasks, asynchronous cancellation, signals as typed errors, `longjmp`, thread-only panic that bypasses cleanup, unmodeled thread-local destructors/callbacks, shared-memory interprocess concurrency, device/DMA mutation, general condition variables, barriers, reentrant mutexes, reader/writer-lock upgrades, atomic-pointer publication, lock-free reclamation, and general lock-free algorithms.

User-visible atomics are not needed for initial conformance. A later atomic profile MUST specify operation ordering and lifetime/publication protocols independently. Merely making a pointer atomic does not grant authority to its pointee. Atomics used internally by a verified synchronization wrapper belong to that wrapper's trusted implementation.

## 39. New Static Domains and Quantitative Access

### 39.1 Kinds

Add compiler-defined kinds:

```text
ObjectId       // fresh abstract allocation/storage identity
Generation     // distinguishes successive lifetimes at reused addresses
Footprint      // a finite region of memory locations or protocol cells
Share          // bounded integer permission units; not a resource dimension
ThreadId
TaskId
MutexId
SyncEventId
AccessQuant
```

These are not ordinary runtime objects and are not user-executable index computations. Resource identity is not a byte count, numeric address, C variable name, or descriptor integer.

### 39.2 Permission units

Each compilation/proof profile fixes a positive integer `FULL`; the initial default is `1024`. All linked checked modules MUST agree on this value, or be explicitly normalized and rechecked.

```text
Share = { k : Nat | 0 <= k <= FULL }
Perm(P,k) = k / FULL access authority at every location in footprint P
```

`FULL` is a scale, not the number of permitted reads. A thread may read repeatedly while retaining a positive share. The finite scale limits the number of independently allocated nonzero read shares, not the number of ordinary local reference aliases.

Shares use exact integer arithmetic, never floating point. Share constraints remain linear integer constraints: `0 < k`, `k1 + k2 == k`, `k <= FULL`. Products/division of symbolic shares are excluded. This preserves the base QF-LIA solver discipline. A rational/share-algebra generalization is optional future work, not a prerequisite.

### 39.3 Surface representation

The optional explicit ghost spelling is:

```text
quant<access, footprint_name, k>
```

This is an identity-indexed access type. It is **not** an ordinary quant vector with a new axis. Existing `quant<credit,...>`, `quant<live,...>`, and `quant<cost,...>` retain their meanings and arities.

An access quant cannot be converted to bytes, spent as allocation credit, scaled as a cost bound, copied, abandoned, or created by a numeric literal. Its only introduction rules are resource initialization and checked authority transfer/splitting. Footprint aliases resolve to compiler-established places and bounds; naming an alias creates no permission.

Ordinary source code SHOULD use existing borrow syntax and explicit task captures. Most ghost tokens remain implicit in the typed CFG.

### 39.4 Two independent structures

```text
Delta : ResourceId -> unique destruction/lifecycle obligation
Pi    : PermissionId -> (footprint, share, custodian, origin)
```

`Delta` answers who is responsible for eventual release or transfer. `Pi` answers who may access which state now. A parent may retain a frozen destruction obligation while a child holds an access share.

An `own<T>` value does not regenerate access authority while its permissions are lent out. The authority associated with ownership must actually be present and usable.

### 39.5 Concurrency type properties

Add compiler-checked properties `Send(T)` and `Sync(T)`:

* `Send(T)` permits transferring ownership and responsibility for `T` to another thread, including its destruction and all reachable state.
* `Sync(T)` permits the supported shared-borrow operations on `T` from multiple threads, subject to the access ledger and wrapper protocol.

These are inspired by, but not claimed identical to, Rust's properties [C4]. `Copy(T)` implies neither property.

Plain immutable scalars and aggregates derive these properties structurally. Owned containers require verified rules for their contents, representation, and destructor. Raw pointers, unknown foreign handles, thread-affine resources, and unsynchronized interior mutation do not derive the properties automatically.

A mutex wrapper can be `Sync` when its protected payload is `Send`, even when that payload is not directly `Sync`. This requires the mutex invariant proof; it is not a rule for ordinary shared references. Guards are thread-affine and not `Send`.

## 40. Footprints, Aliasing, and Global Custody

### 40.1 Footprint representation

A memory footprint is a finite union of typed ranges:

```text
(ObjectId, Generation, memory_location_range)
```

A protocol footprint may instead name an abstract state cell, such as a file's logical offset or a transaction's transition state. A single operation may require several memory and protocol footprints.

Storage addresses alone are insufficient because addresses and handle integers can be reused. Every lifetime introduction gets a fresh abstract generation. Pointer copying preserves provenance and does not introduce a fresh object.

### 40.2 C-compatible granularity

Whole objects are the default conservative unit. A refinement to fields or array ranges is permitted only when layout and access analysis establish distinct C memory locations.

Adjacent nonzero-width bit-fields belonging to one C memory location MUST NOT be treated as independently writable cells. Union overlap, unproven packed layouts, whole-struct copies, byte-wise access to object representations, casts, and library operations touching a wider range require overlapping footprints or an unknown result. Cache-line sharing alone is a performance concern, not a reason to merge otherwise distinct C memory locations.

For ranges in one allocation, prove bounds and disjointness using index constraints. A split at `m` requires `0 <= m <= n` and produces `[0,m)` and `[m,n)`. A half-open empty range has no access authority and must not be used to justify a load/store.

### 40.3 Pointwise conservation

For every live access-controlled location `a`:

```text
sum(k_i for all permission fragments i covering a) == FULL
```

The sum includes fragments held by running tasks, blocked tasks, mutex invariants, pending start records, completed-task capsules, suspended owner/loan frames, and sealed cleanup state. Zero-share fragments are omitted.

The checker may use `<= FULL` as its immediate non-overlap validity test, but a successful checked transition MUST also account for the missing complement in some explicit custodian. No positive share may silently disappear.

A mutex invariant and its guard are alternative custodians, never simultaneous copies. A task handle's promise of future returned authority is not current usable authority and must not be counted twice.

### 40.4 Resource custody

A live resource identity has exactly one destruction custodian, which may be a local owner, an owned aggregate, a checked mutex container, a running child's frame, an error payload, or a completion capsule.

The global live vector is the sum over unique live resource entries. Splitting permissions, creating additional pointer views, or transferring custody does not add another resource footprint.

The task handle tracks the join obligation and names a future post-state. It does not give the parent concurrent ownership of the child's resources. Task runtime storage is a separate identified resource.

### 40.5 Stable facts and interference

SSA indices reflected from copied scalars remain historical facts. A snapshot of mutable shared memory is not a persistent assertion about its current contents.

A function summary MUST identify facts invalidated by exclusive loan transfer, mutex release, and external mutation. Reacquiring a mutex reintroduces only the mutex invariant and fresh checked snapshots, not arbitrary facts from a previous critical section. Unsynchronized repeated reflection of mutable memory is forbidden.

## 41. Permission Algebra and Access Rules

### 41.1 Partial separating composition

Write `Pi1 * Pi2` for composition when fragments refer to compatible provenance and their pointwise sums do not exceed `FULL`. This is a partial commutative composition, not quant-vector `join`.

```text
Perm(P,k1+k2) <-> Perm(P,k1) * Perm(P,k2)
    when 0 < k1, 0 < k2, k1+k2 <= FULL
```

Splitting consumes the original fragment. Merging consumes both input fragments and requires the same footprint, generation, compatible loan origin, and actual custody by the merging context. A promised future fragment cannot be merged.

### 41.2 Spatial splitting

```text
Perm(P union Q,k) <-> Perm(P,k) * Perm(Q,k)
    when P and Q are proved disjoint
```

Both fragments may have `k == FULL`, because they cover different locations. This does not split the allocation's destruction obligation. Freeing the allocation requires reclamation of authority over its entire destruction footprint.

### 41.3 Reads and writes

A non-atomic read requires positive authority at every accessed location and a view whose mutability permits reading. It preserves the authority.

A non-atomic write or read/modify/write operation requires full authority at every affected location and a mutable view. It preserves the authority. Full authority alone does not turn a `const` view into a mutable reference or override the base borrow rules.

In compact form:

```text
Live(P)   ReadableView(t,P)   Pi_t supplies positive share over P
---------------------------------------------------------------- READ
State_t |- read(P) : A       // Pi_t unchanged

Live(P)   MutableView(t,P)   Pi_t supplies FULL over P
---------------------------------------------------------------- WRITE
State_t |- write(P,v) : unit // Pi_t unchanged
```

Ownership-consuming operations additionally require the corresponding `Delta` entry. Their access footprints include any state read/written by the operation and destructor.

### 41.4 Destruction

Release requires all of the following:

```text
unique destruction obligation in current custody
full authority over the destructor's declared access/reclamation footprint
all dependent loans ended
no running/completed-unjoined task retaining such a loan
valid typestate and legal destruction thread
no sealed mutex invariant still owning the resource
```

The operation consumes the appropriate authority and `Delta` entry. A destructor that also touches shared global state needs separately checked authorization for that state.

### 41.5 Local shared-reference copying

The base rule allowing a `Copy` shared borrow remains valid only as copying a runtime view associated with an existing loan group. It does not duplicate a linear permission fragment.

Multiple local views may consult one task-local permission group. Returning that group to its lender requires every associated view and reborrow to have expired. Cross-thread copying is different: task creation MUST partition authority or invoke a verified synchronization-wrapper sharing rule. No `Copy` implementation may bypass that partition.

### 41.6 Branches versus parallel tasks

At mutually exclusive CFG alternatives, retain path alternatives or require compatible exact permission states; do not add their permission amounts. At parallel composition, separate and account for the authority present in both tasks at the same time; do not take their maximum.

A branch merge cannot strengthen access by forgetting a possible outstanding loan. Unknown equality/disjointness is not permission to accept.

## 42. Normative Grammar Additions and Explicit Corrigenda

These are grammar extensions to §5, not pseudocode. Every referenced base nonterminal retains its definition unless explicitly amended. New keywords are contextual. Names for new static kinds are elaboration classes, not runtime values.

### 42.1 New declarations and types

Add `<concurrency-profile-declaration>` to `<declaration>`, and the following productions:

```bnf
<concurrency-profile-declaration> ::= "concurrency" "checked" ";"

<access-quant-type> ::= "quant" "<" "access" "," <identifier> "," <share-expression> ">"

<share-expression> ::= "FULL"
                     | <integer-literal>
                     | <identifier>
                     | "(" <share-expression> ")"
                     | <share-expression> "+" <share-expression>

<task-type> ::= "task" "<" <lifetime> "," <type> "," <error-row> ">"

<mutex-type> ::= "mutex" "<" <type> ">"
```

Add `<access-quant-type>`, `<task-type>`, and `<mutex-type>` to `<type>`. `access` is deliberately not added to the old vector-valued `<quant-mode>` production. The access spelling is a separate type constructor.

A share identifier resolves only to a `Share` binder or compile-time share constant. Add `<identifier> ":" "Share"` to `<type-parameter>` and `<share-expression>` to `<type-argument>`. Expected-kind elaboration distinguishes a share argument from an ordinary index or usage argument. Parameterized share expressions generate bounded linear constraints in the permission solver; they cannot be silently substituted for dimensioned indices. Share preconditions use the separate contract clause below, not an extension of `where` to arbitrary ghost domains.

### 42.2 Statements and footprint aliases

Add these statement alternatives:

```bnf
<thread-scope-statement> ::= "thread" "scope" <identifier> <block>

<footprint-statement> ::= "footprint" <identifier> "=" <place-expression> ";"
                        | "footprint" <identifier> "=" <place-expression>
                          "range" "[" <index-expression> "," <index-expression> ")" ";"

<with-lock-statement> ::= "with" "lock" <place-expression> "as" <identifier> <block>
```

`footprint p = x;` names the currently resolved storage/protocol footprint of `x`. It is ghost-only. The range form is measured in array elements with target-aware lowering to memory locations. A runtime pointer alone cannot establish provenance or a valid footprint alias.

`thread scope s` introduces ghost lifetime `'s`. A guard bound by `with lock` has an inferred generative guard lifetime and can be used through its checked `value` view; users do not fabricate guard types or constructors.

### 42.3 Task expressions and captures

Add these expression alternatives:

```bnf
<spawn-expression> ::= "spawn" <identifier> <identifier>
                       "(" <optional-capture-list> ")"

<optional-capture-list> ::= ε
                         | <capture-list>

<capture-list> ::= <capture>
                 | <capture> "," <capture-list>

<capture> ::= "copy" <place-expression>
            | "move" <place-expression>
            | "read" <place-expression> "share" <share-expression>
            | "write" <place-expression>

<join-expression> ::= "join" "move" <place-expression>
```

The first spawn identifier is a live enclosing scope; the second resolves to a first-order checked function. Captures are evaluated left-to-right as reservation operations. Arbitrary capture expressions and closures are intentionally excluded from this revision.

`copy` requires a copyable, transferable value with no unauthorized shared access hidden inside it. `move` reserves exclusive ownership transfer. `read ... share k` creates or subdivides a shared loan group; `write` transfers an exclusive scoped loan.

The spawn expression returns a discriminated result. Immediate `?` propagation is legal only if its failure edge satisfies all outstanding-task obligations. Deferring discrimination requires a relational exit state; the initial implementation MAY require immediate `?` or `match` and reject deferred discrimination.

### 42.4 Contracts

Add to `<contract-clause>`:

```bnf
<access-contract-clause> ::= "access" <access-action> <place-expression> ";"
                          | "access" <access-action> <place-expression>
                            "range" "[" <index-expression> "," <index-expression> ")" ";"
                          | "thread" <thread-property> <identifier> ";"
                          | "share" "requires" <share-constraint> ";"

<share-constraint> ::= <share-expression> <relation-operator> <share-expression>
                     | "(" <share-constraint> ")"
                     | <share-constraint> "and" <share-constraint>

<access-action> ::= "read"
                  | "write"
                  | "release"
                  | "atomic"

<thread-property> ::= "requires_send"
                    | "requires_sync"
```

Add `<access-contract-clause>` to `<contract-clause>`. Contracts refer to declared parameters/globals with resolved provenance. These clauses summarize accesses; ownership/post-state clauses remain mandatory. `access atomic` is reserved and MUST be rejected outside a separately implemented atomic profile.

`share requires k > 0;` is a static precondition over the separate Share domain. The caller proves it and the callee may assume it; it cannot mint permissions or replace the access ledger. Base `where` restrictions are unchanged. Compiler-inferred access requirements refer to the footprints of parameters and globals, even when their ghost permission parameters are not written in the source.

`share requires k > 0;` is a static precondition over the separate Share domain. The caller proves it and the callee may assume it; it cannot mint permissions or replace the access ledger. Base `where` restrictions are unchanged. Compiler-inferred access requirements refer to the footprints of parameters and globals, even when their ghost permission parameters are not written in the source.

Complex synchronization summaries are initially compiler intrinsics or versioned external contract records (§48), not arbitrary user-written functions in `where`. Do not invent a general assertion language during implementation.

### 42.5 Base syntax corrigenda

The base examples use constructs not fully covered by their original productions. The consolidated revision makes the following corrections explicitly:

1. Function and extern declarations accept `<optional-quant-summary>` after `<optional-effects>` and before `<optional-contract>`.
2. `<pattern>` accepts `<identifier> ":" <type>` for the typed catch binder used by the examples.
3. Unit and tuples of at least two expressions are added to `<primary-expression>`:

```bnf
<unit-expression> ::= "(" ")"
<tuple-expression> ::= "(" <expression> "," <expression> <tuple-expression-tail> ")"
<tuple-expression-tail> ::= ε
                         | "," <expression> <tuple-expression-tail>
```

The consolidated document applies these changes; parser golden tests MUST cover them. Base snippets explicitly called conceptual or schematic remain so. These repairs do not constitute evidence that the full grammar has already been parsed or tested.

## 43. Scoped Tasks, Failure, and Join

### 43.1 Spawn precondition

Before successful spawn, the checker establishes a partition of the parent's owned entries, permission fragments, and strict credits into retained and transferred components. Borrowed resources are not duplicated in `Delta`.

The worker's parameter types, `Send`/`Sync` conditions, effects, entry access requirements, normal/error post-states, and quantitative summary MUST be known. Its global accesses must also be covered. Implicit capture of mutable globals is forbidden.

A child may not receive a guard, a thread-affine destructor obligation, an ancestor's task handle, or a reference whose storage can expire before the child's join.

### 43.2 Two-phase spawn and creation failure

`spawn` is a special resource-aware primitive, not an ordinary function that blindly consumes its argument expressions before failure is known.

**Reserve:** Validate captures, create a private stable start record, reserve ghost transfers, and suspend conflicting parent access. The environment is fully initialized before the runtime can start the child.

**Success:** Commit moves, credit partitions, and loans to the child; produce one `task<'s,T,E>` handle and one corresponding join obligation. The success edge invalidates moved parent places.

**Failure:** No worker execution has occurred. Destroy only private runtime-start storage, cancel the reservations, restore the captured parent places/permissions/credits/cleanup registrations, and return `SpawnError`. Previously running siblings are unaffected.

The runtime adapter MUST satisfy this all-or-nothing contract. If its API can report failure after starting a worker, it is not a valid adapter without an additional recovery protocol.

The special restoration includes usage residuals and SSA ownership versions, not a general rollback of arbitrary expressions. Restricting captures to places makes this tractable. A failure after successful creation is a worker result, never `SpawnError`.

### 43.3 Borrowed parameters

A child can repeatedly use a shared borrow according to its positive share and the referent's shared-access protocol. A child with an exclusive borrow can mutate the permitted footprint, while competing parent/sibling accesses are prohibited.

The owner's lifetime extends through successful join, not merely through the source worker call or an observed completion flag. Borrowed referents and captures must have stable addresses where their C representation requires it.

All borrowed capture authority is returned in the worker's completion post-state on **both** normal and typed-error exits. It cannot escape in a returned view, global, detached callback, or error payload. Owned values may be returned or carried in errors if their types and reachable contents are `Send`.

### 43.4 Task handles

A task handle is a built-in linear synchronization obligation with an explicit `join` eliminator. It is not an ordinary user resource with an automatically runnable total destructor.

It cannot be copied, dropped, detached, moved to a different task, forgotten, or put into a cleanup pack that promises ordinary no-throw discharge. Moving it between local variables in the creating task is allowed. Exactly one creator-context join consumes it.

The worker must join its own descendants before completion. These restrictions make the join graph a structured parent-child graph. They do not, by themselves, prevent deadlocks involving mutexes or external I/O.

### 43.5 Completion capsule

On worker completion, checked worker-local cleanup has run. Its return value or typed error, remaining returned owned values, loan-return evidence, and quantitative post-state are placed in a completion capsule.

The worker no longer uses the capsule. The parent cannot read it or regain its authority before a successful join. Resource identities in a capsule remain globally live, even after worker execution stops.

Thread-local finalizers and foreign callbacks capable of using captured state MUST be completed before the backend declares quiescence, or excluded by contract. A user-level “worker body finished” notification alone is insufficient [C5].

### 43.6 Join

```text
join move h : result<T, E>
```

Join has `may_block` behavior. On successful completion it consumes `h`, establishes the completion-to-parent synchronization edge, transfers the capsule's result/error and owned entries to the parent, and restores the recorded borrowed permissions and post-state facts.

Returning the worker's typed error as a result does not itself raise that error. The caller may propagate it only after required sibling joins and cleanup. This distinction permits deterministic collection of multiple worker outcomes without prematurely abandoning siblings.

A backend join failure does not prove termination or restore permissions. The initial safe wrapper handles an unexpected join failure by process-wide panic/abort without reclaiming possibly accessed storage. A recoverable join-error API that preserves the handle would require a separate retry/recovery protocol and is not part of this core.

No bound on join waiting time is inferred. Join MUST NOT be registered as a base `noexcept` total destructor. `noexcept` and “cannot block forever” are different facts.

### 43.7 Scope exit

Every checked edge leaving a thread scope—fallthrough, return, error, break, or continue—must have no outstanding task handle or child loan. All task results containing owned resources must also be accounted for by ordinary rules.

The checker does not silently detach, cancel, or auto-join. An error may be caught inside the scope so the handler can explicitly join already-created children before propagating. A second spawn failure after a first success therefore needs a path that joins the first task.

A `?` that would bypass a live sibling handle is rejected even if the usual sequential cleanup pack would otherwise be valid.

## 44. Mutexes and Protected Resource Invariants

### 44.1 Initial supported invariant

A built-in `mutex<T>` owns a stable mutex identity and a protected payload with a fixed ownership shape and reclamation footprint. Its invariant records the protected live identities, access permissions, valid typestate, and any declared stable scalar refinements.

The first profile permits mutation of values and protocol state while retaining the protected ownership shape. It does not permit replacing an unbounded owned graph, transferring resource ownership out of the mutex between tasks, or changing its live footprint. Such patterns require the dynamic-invariant extension and different cost accounting.

This subset is useful for counters, fixed buffers, state records, and modeled handle operations. It intentionally does not yet certify every lock-protected growable container.

### 44.2 Closed and open states

```text
closed mutex:
    mutex invariant holds the protected authority and custody
    shared mutex handles do not directly authorize payload access

successful lock:
    invariant authority is transferred to exactly one thread-bound guard
    all other handle holders still lack payload authority

unlock:
    guard's loans end and the invariant is re-established
    authority returns to the closed mutex
```

Lock acquisition is an operation of the synchronization wrapper, not an ordinary unprotected write to its implementation fields. The wrapper's proof/contract covers those internal fields.

### 44.3 Structured locking

```text
with lock m as g {
    // checked access through g.value
}
```

The operation either acquires the lock before entering the body or raises the modeled acquisition failure with no guard. Backend invariant violations may instead use the fixed process-panic policy. The exact adapter policy MUST be recorded.

Every checked exit from the body must return the protected invariant and expire guard-derived views before unlocking. The emitter inserts the unlock action on each such edge. Nested cleanup runs in a dependency-respecting order.

A resource-bearing error may not carry protected ownership or a guard-derived borrow out of the critical section. A copied scalar snapshot or independently owned local resource may escape under ordinary error rules.

### 44.4 Error invariant restoration

The mutex invariant can be temporarily opened inside a critical section, but must hold on all checked unlock edges. A typed error is not a reason to assume it restored itself. Code must roll back, expose a permitted invariant variant in a later profile, or reject the edge.

The initial invariant language is compiler-generated ownership/typestate/footprint structure plus supported index facts, not unrestricted separation-logic formulas. A generic `defer unlock` is insufficient when resources or facts needed by the invariant have been consumed.

### 44.5 Lock lifetime and destruction

Destroying a mutex requires unique ownership of the wrapper, full recovery of all handle-lifetime loans, no guard, no blocked/waiting task with a valid handle, and a closed valid invariant. Joining all scoped handle users is one way to establish this state.

Holding a payload guard does not authorize destruction of the underlying mutex. Guard ownership is thread-affine, and unlock is performed by the legal locking thread.

### 44.6 Blocking and lock order

The initial checker rejects joining a child while holding any tracked mutex guard. This prevents one common cleanup/parent-child deadlock pattern, but is not advertised as complete deadlock freedom.

An optional lock-order mode can require strictly increasing immutable lock ranks on every nested acquisition and propagate rank requirements through calls. Its theorem concerns cycles among the modeled mutex acquisitions only; it does not establish fairness, I/O termination, or absence of unrelated waits. Unknown lock aliases/ranks fail that optional proof rather than being guessed.

## 45. Synchronization and the C Memory Model

### 45.1 Ordering evidence

The concurrent IR records events and ordering relations, including sequenced-before within a task, successful start publication, mutex release/acquire handoff, and successful task termination/join.

For this core:

```text
HB = transitive closure of sequenced-before and supported synchronizes-with edges
```

All cross-thread authority transfers must be backed by the corresponding runtime ordering. Ghost reassignment without a runtime handoff is illegal. Pointer publication is distinct from merely calculating the pointer value.

C11 thread creation/join synchronization is specified in N1570 §7.26.5 [C3]. Mutex semantics must include applicable corrections to that baseline, including the mutex total-order clarification in DR 405 [C6]. A POSIX or Windows backend must separately justify equivalent abstract operations.

### 45.2 Why this excludes modeled data races

Suppose different threads have conflicting non-atomic accesses to an overlapping location. A writing thread must possess full authority. A competing reader needs positive authority, and a competing writer also needs full authority. Those simultaneous authorizations violate pointwise conservation.

If authority instead moved between the two accesses, the only permitted inter-thread transfers establish synchronization. The accesses are therefore ordered by the transfer's happens-before chain. The proof needs both the algebra and the transfer semantics; pointwise arithmetic alone is not the whole C11 argument.

### 45.3 Explicit non-evidence

`volatile`, sleeping, elapsed time, testing a plain “done” flag, a mutex at a different identity, or a successful allocation does not create the required ordering. A raw pointer cast, `const` qualifier, or descriptor-number inequality does not prove disjoint resource state.

A successful join orders the terminated child's effects; it does not retroactively make earlier parent/child conflicting accesses safe.

### 45.4 Atomic boundary

An atomic location must not be accessed non-atomically by bypassing its wrapper. Even a future atomic profile must track initialization, destruction, pointee lifetime, and resource ownership separately.

Release/acquire publication, compare-exchange ownership transfer, reference counting, hazard pointers, epochs, and ABA prevention require explicit protocols. None is certified by a numeric access fraction or an atomic-pointer annotation alone. The initial analyzer reports unsupported such protocols as unknown.

## 46. Recoverable Errors and Checked Quant Opening

### 46.1 Preserve the base credit/live distinction

A matching error discharge may abandon a region-local unused credit remainder. It may not abandon a positive access share, a live resource identity, a task handle, a guard invariant, or a return obligation for a borrowed capture.

A credit transferred to a child is no longer a parent-local remainder. Parent error handling cannot abandon it on the child's behalf.

### 46.2 Extended cleanup precondition

A concurrent cleanup pack records access dependencies in addition to the base action stack:

```text
CleanupPack<r,E,LiveDelta,CreditRemainder,AccessDependencies>
```

Opening for ordinary discharge requires a valid cleanup plan. For each destructor/rollback step, the plan proves its permission, loan, typestate, and thread-affinity preconditions at that point.

```text
NoOutstandingTaskInPack(S)
CleanupPlanAuthorized(Pi,S)
cleanup(S) : (Delta0 + S, Pi) -> (Delta0, Pi0)
EveryExitConsumes(cleanup_capability)
------------------------------------------------------------ QUANT-OPEN-CONCURRENT
matching catch may open and discharge S
```

The post-state need not make all global permissions zero; returned outer loans and mutex invariants remain valid. Only obligations local to the discharged region are eliminated or returned according to their rules.

### 46.3 Quiescence before destruction

For a resource borrowed by a child, joining that child and collecting its loan-return evidence is a prerequisite to destroying the referent. The compiler MUST NOT insert an ordinary destructor before that prerequisite.

The safe pattern is to catch within the still-live thread scope, join outstanding tasks, collect their results, and only then discharge resource cleanup or propagate errors. A region outside a thread scope cannot receive a checked error with unjoined tasks from inside it.

A completed-but-unjoined task does not satisfy the requirement. The parent still lacks the join event and returned authority.

### 46.4 Cleanup dependencies

The base LIFO discipline remains the default order among actions whose dependencies permit it. The checker additionally verifies edges such as:

```text
expire guard-derived views -> restore invariant -> unlock guard
expire local loans -> drop owner
join child (explicit, before ordinary discharge) -> drop borrowed owner
```

The initial implementation rejects a cleanup frame when reverse acquisition order cannot satisfy its dependencies; it need not invent a general cleanup scheduler. A later verified scheduler may use a dependency DAG with stable reverse-acquisition tie-breaking.

No blocking join is smuggled into a `noexcept` cleanup pack. This preserves the base requirement that ordinary cleanup eliminators are total and non-throwing on their accepted states.

### 46.5 Multiple errors

Worker typed errors remain ordinary, possibly linear, result values until collected. The parent can select a primary error by a declared deterministic policy, but must still discharge or transfer resources carried by other errors/results.

Discarding a secondary error is legal only when its payload is discardable. It must not lose owned resources or task obligations. Concurrent errors do not trigger asynchronous stack unwinding in another task.

### 46.6 Panic policy

The initial process-panic policy terminates the whole process and is outside the recoverable cleanup guarantee. It must not merely terminate one worker and leave other tasks running against reclaimed state. No error-opening theorem is claimed for OS termination or unmodeled asynchronous interruption.

## 47. Concurrent Quantitative Flow

### 47.1 Do not reuse conditional maximum

Mutually exclusive branches join costs by maximum. Concurrent computations may overlap, so their independent incremental peaks require addition in the conservative bound.

For an overlap epoch with unchanged shared footprint, no ownership handoff between participants during the epoch, disjoint participant-owned changing resources, and valid per-participant summaries `Fi = (di,pi,ti)`:

```text
epoch.net   = sum(di) + runtime_overhead_net
epoch.peak <= sum(pi) + runtime_overhead_peak
epoch.total <= sum(ti) + runtime_overhead_total
absolute_live_peak <= live_at_epoch_entry + epoch.peak
```

The entry union is counted once, including shared allocations. This is a conservative upper bound, not an equality of observed peaks. Backend overhead terms must be dimensioned and included exactly once. The overhead net is the signed change in separately modeled runtime live storage across the epoch; it may be set to zero only when that equality is proved. Storage already included in a participant summary or the entry baseline must not be charged again.

### 47.2 Scope of the simple formula

The formula applies to the structured, fixed-footprint-invariant core. It is not a universal law for arbitrary concurrently communicating resource flows.

A dynamic shared container, ownership-carrying channel, or transfer between running tasks can change which local baseline an allocation/release belongs to. Such a profile needs explicit global phase accounting or conserved allocation reservations that bound all interleavings. Applying the simple formula without its side conditions is forbidden.

### 47.3 Returned resources and accounting phases

A child returning an owned resource keeps that resource live in its completion capsule. Thread completion does not release it. At join, custody moves to the parent without changing the live vector. Subsequent phases use the resulting global baseline.

Transfer and permission-split events have zero net live footprint unless they also allocate distinct runtime storage. Child start records, result capsules, and thread handles need their own resource entries and release points.

### 47.4 Credits

Before fork, exact credits are partitioned among participants. Two tasks cannot each spend the original full budget. A shared dynamic credit pool requires a verified synchronized protocol and is not implicit in `credit.split`.

Credit and access splitting are different operations: one partitions future resource capacity, the other partitions authority over existing state. Neither implies the other.

### 47.5 Axes and unsupported bounds

Useful additional axes include outstanding task records, configured stack reservation, aggregate work steps, and synchronization operations. OS thread memory or stack usage is claimed only when the backend's model supplies a justified bound. It is not inferred from the presence of `thrd_create`.

Aggregate CPU work may sum across tasks. Wall-clock latency does not automatically become either the sum or maximum of their work bounds. Waiting and scheduling time are unknown without a separate timing model. Race-safety acceptance need not be blocked by an unclaimed timing axis, but a claimed bound must fail if its proof is unknown.

### 47.6 Required numerical checks

Two readers of one 4096-byte allocation still account for 4096 live bytes, not 8192. Two independent computations whose incremental peaks are 3072 and 5120 bytes require up to 8192 additional bytes, not 5120, absent a stronger non-overlap proof. Two tasks cannot each receive 100 units from a single 100-unit credit without an explicit capacity increase.

## 48. Conservative Analysis of Existing Multithreaded C

### 48.1 Extended ingestion pipeline

```text
C AST / CFG / memory SSA
-> provenance and conservative points-to abstraction
-> resource and synchronization API recognition
-> thread entry/callback resolution and may-happen-in-parallel relation
-> access footprints and memory/protocol effects
-> ownership, permission, lock and lifetime obligations
-> path-sensitive normal/error/task-completion summaries
-> modular fixed point and assumption-qualified diagnostics
```

The analyzer MUST resolve the actual selected platform/backend and contracts. Similar function names are not evidence of equivalent synchronization. Contracts may be supplied in versioned sidecar metadata where source annotations are impractical.

### 48.2 Minimum summary record

```text
ConcurrencySummary {
  parameter ownership and aliases;
  normal/error/post-completion ownership states;
  read/write/release/protocol footprints, including globals;
  required and returned permissions;
  acquired/released mutex identities and guard ownership;
  tasks created/joined, with creator and entry function;
  capture lifetime and escape conditions;
  publication and completion synchronization edges;
  callback targets and whether callbacks can run after return;
  thread-affinity / Send / Sync obligations;
  may_block and unsupported-control effects;
  quant summaries plus side conditions;
  trust/proof status, source spans, and contract version;
}
```

Foreign-resource operations are classified by actual protocol effects, not their names or C parameter constness. Reading a handle may mutate its logical state. Multiple descriptors may alias a shared underlying state cell while retaining distinct close obligations; a `dup`-like contract must represent both identities. Unknown sharing is not modeled as independent fresh resources.

### 48.3 Thread creation and joining in C

Recognized C/POSIX/Windows calls need contracts for failure, success, entry arguments, lifetime, and completion. A failed create must not produce a live join obligation. A failed join must not produce quiescence evidence. Unsupported detached or cancelled executions remain unknown.

Arrays/loops of thread handles require a bounded collection model or an invariant preserving the set of outstanding tasks. This is a separate inference milestone, not automatic from scalar handle support.

### 48.4 Alias and parallelism conservatism

Potential aliases are treated as overlapping until disjointness is proved. Holding `m1` while another thread holds `m2` does not protect a shared object merely because the variable names differ. Aliased lock identities must unify.

May-happen-in-parallel analysis must include all reachable thread entry functions and modeled callbacks. An unresolved callback that might access shared state invalidates the corresponding race-freedom claim.

### 48.5 Results and claim boundaries

For each property, return one of:

```text
PROVED_UNDER_ASSUMPTIONS
VIOLATION_WITNESS
POTENTIAL_VIOLATION
UNKNOWN
```

A rejected permission assignment is not automatically proof of an actual C race: it can be a conservative false positive or insufficient annotation. `VIOLATION_WITNESS` needs a validated feasible execution/trace under the modeled semantics. Timeout, unsupported control flow, unresolved aliasing, or a missing foreign summary is `UNKNOWN` or a potential violation, never proof of safety.

For whole-program reporting, every reachable relevant access must be covered. Unknown components are surfaced even when the analyzer proves individual modules. Existing contracts/unsafe blocks are listed as assumptions, not silently treated as verified implementations.

### 48.6 Relation to existing capability analyses

Clang documents compile-time capability checking, shared/exclusive access requirements, and guarded resources [C7]. This is useful evidence that annotation-driven checking is practical. It is not a proof that QCore's stronger ownership, cleanup, permission accounting, or C-ingestion obligations have been solved, and it does not replace QCore's analysis engine.

## 49. Checker Passes and Data Structures

### 49.1 Pass integration

Retain the separate base passes. Extend ordinary elaboration with `Send`/`Sync` and built-in concurrency types. Extend index analysis with a separate bounded-share constraint context. Extend ownership flow with custodians, frozen owners, and child-result states.

Add access-footprint elaboration before checking memory operations. Add task/lock protocol analysis before finalizing exit maps and cleanup legality. Concurrency, ownership, and alias facts may require a controlled fixed point in the C analyzer; they are not assumed available from a single superficial syntax pass.

Quant-flow computes overlap epochs only after the task graph and ownership-transfer side conditions are known. Cleanup lowering runs only after all permission and quiescence requirements are discharged.

### 49.2 Core records

```text
PermissionEntry {
  id, footprint, share, custodian, loan_origin, origin_span
}

Custodian = ThreadFrame(ThreadId)
          | SuspendedLoan(LoanGroupId)
          | MutexInvariant(MutexId)
          | PendingStart(TaskId)
          | CompletionCapsule(TaskId)
          | CleanupFrame(RegionId)

TaskEntry {
  id, creator, scope, worker, state,
  capture_reservations, owned_inputs, loan_inputs, credit_inputs,
  normal_post, error_posts, returned_loan_contract,
  completion_event, runtime_resources
}

TaskState = Reserved | Running | Completed | Joined

MutexEntry {
  id, owner, protected_invariant, state,
  handle_loans, guard_owner, runtime_contract, optional_rank
}

AccessEvent {
  thread, footprint, operation, provenance,
  authorization, synchronization_context, source_span
}
```

Extend `ExitState` with permission state, active/suspended loans, outstanding tasks, held guards, stable shared facts, and synchronization obligations. Extend `CleanupFrame` with an authorization/dependency plan. Preserve the original ownership/live-vector equality over unique resource identities.

`TaskEntry` summaries contain references to future ownership states, not duplicate live `ResourceEntry` objects in the parent's current ledger.

### 49.3 Solver interface

Every access/disjointness/share obligation records assumptions, conclusion, proof result, source spans, and the enabling protocol transition. Preserve proved/disproved/unknown distinctions.

No unsound widening may strengthen permissions. Widening can lose a proof and report unknown. Cached summaries are keyed by relevant type/contract versions, target layout, FULL, effect assumptions, and concurrency profile.

## 50. C Runtime and Lowering Contract

### 50.1 Target architecture

```text
typed concurrent CFG
-> task/lock/cleanup-explicit CFG
-> C AST + small backend synchronization ABI
-> C11 code and selected runtime adapter
```

Proofs, shares, footprint aliases, and ghost custody records erase. Real start records, thread handles, locks, result tags, storage-lifetime flags, and required synchronization do not erase.

The initial portable target may use `<threads.h>` where available. POSIX and native Windows backends are separate adapter implementations with separate conformance tests. The base postponed Windows/MSVC validation remains postponed until actually executed.

### 50.2 Start record

The emitter initializes the entire child environment before calling the backend. After successful creation, fields accessible by the worker are not concurrently modified by the parent. ABI handle output storage and worker-owned input fields must not overlap in an unprotected way.

A stack-allocated environment is allowed only when its stable storage outlives successful join on every checked path. Moving a C struct after giving its address to a child is forbidden unless the address is not observed or the representation is intentionally indirect.

On creation failure, the wrapper performs the declared rollback of reserved captures and internal storage. It must not read a successful child result on that path.

### 50.3 Completion record

The child writes its result/error exactly once after its own required cleanup. The parent reads it only after successful join. The completion record and owned result storage stay live until transferred or destroyed by the parent.

The runtime handles unexpected join failure without treating the result as initialized and without freeing the environment. A process-abort policy is acceptable at this out-of-model boundary; an optimistic “continue cleanup” policy is not.

### 50.4 Locks and cleanup

Lock/unlock calls are explicit IR operations with identity and invariant transfers. The optimizer must not move protected accesses outside their authorized region or erase ordering because the permission token itself is ghost-only.

All generated unlock/error edges are translation-validated. Guard lifetime and same-thread release constraints survive erasure through correct control flow and runtime API selection.

### 50.5 Concurrent translation validation

The sequential corpus still compares exact normalized traces where appropriate. Concurrent executions need not choose the same schedule or produce the same total event ordering.

The concurrent obligation is trace refinement or equality of permitted behaviors for a bounded model: preserve per-thread order, synchronization edges, ownership transfers, values/errors allowed by the source semantics, and quantitative bounds. Compare partial-order event graphs or replay a controlled schedule. Do not fail a correct implementation solely because independent events interleave differently.

A test logger must not accidentally serialize the whole program and hide races. Production-model instrumentation should use per-thread buffers or explicitly account for logging synchronization.

## 51. Formal Obligations

The following are theorem targets, not already-proven theorems. Lean development remains mandatory at every implementation stage, as in the base execution directive. No `sorry`, invented solver result, or testing-only substitution is accepted as proof completion.

### 51.1 Permission algebra

Prove validity of partial composition, pointwise conservation, split/merge inverses on valid fragments, spatial-split preservation, and impossibility of simultaneous full-write plus positive competing access on one location.

### 51.2 Concurrent preservation

A step of any task, lock protocol, start/complete/join transition, or checked cleanup preserves global resource identity uniqueness, permission conservation, valid custody, and compatible local typing.

### 51.3 Authorization and ordering

Every checked access has authority. Every inter-thread transfer that can enable a conflicting access is backed by an HB-producing protocol transition. Derive memory-race freedom for the modeled C-compatible footprint abstraction.

### 51.4 Lifetime and quiescence

Prove that destruction cannot occur with an outstanding dependent loan, and that a returned borrowed authority is usable by the parent only after a successful join. A merely completed task is insufficient.

### 51.5 Error preservation

Worker normal/error completion returns exactly the declared loans and accounts for owned payloads. Parent error propagation cannot bypass outstanding tasks. Error opening discards only eligible local credit and preserves access conservation for surviving state.

### 51.6 Mutex invariant transfer

Prove that the protected authority has exactly one custodian, guard-derived borrows cannot outlive unlock, and every checked unlock restores the invariant. Guard transfer to another task is impossible in the safe core.

### 51.7 Bounds

Prove the overlap-epoch peak bound under its stated side conditions, and prove that shared allocations and returned-result capsules are counted exactly once. Unsupported dynamic transfer patterns cannot use this theorem.

### 51.8 Progress qualification

Replace an unqualified concurrency progress claim with: a well-typed configuration is completed, can take a legal step, has a declared typed exit, has process panic, or is blocked at an explicitly modeled synchronization/I/O operation. It is not stuck due to a missing permission or invalid resource transition.

Deadlock freedom is not silently bundled into this theorem. A lock-rank theorem, when implemented, has its own hypotheses and scope.

### 51.9 Lowering and trust

Prove or translation-validate the reduced concurrent CFG's behavior against the selected C/runtime model. An SC interleaving model alone is not a proof for arbitrary C11 atomics. The initial theorem applies only to the synchronization-disciplined supported subset and its validated adapters.

Track solver soundness, foreign contracts, runtime adapters, C compiler assumptions, and alias abstraction explicitly in the theorem statement/trust manifest.

## 52. Established, Designed, and Unvalidated

| Category | Content | Status in this revision |
|---|---|---|
| Established foundation | Fractional read permissions/full write permission | Prior research [C1] |
| Established foundation | Resource invariants and compositional concurrent reasoning | Prior research [C2] |
| Established interface ideas | Transfer/share type properties and scoped task lifetimes | Primary Rust documentation [C4, C5] |
| Platform foundation | C data-race and synchronization semantics | WG14 baseline and corrections [C3, C6] |
| Practical precedent | Annotation-driven capability checking | Clang documentation [C7] |
| QCore design | Separate `AccessQuant`, bounded share scale, identity/custody ledgers | Proposed here |
| QCore design | Transactional place-only spawn reservations and explicit join | Proposed here |
| QCore design | Quiescence-aware quant opening and no implicit join destructor | Proposed here |
| QCore design | Fixed-footprint mutex invariant profile and overlap epochs | Proposed here |
| Empirical evidence | Supplied small permission model and executed tests | Limited to the report accompanying that model |
| Unvalidated integration | Full source checker, scalable C analysis, C lowering, continuous Lean proofs | Required work; not claimed complete |
| Later research | Dynamic resource invariants, lock-free protocols, scalable inference and richer sharing | Outside this revision's safe core |

The supplied model is deliberately smaller than the normative language. Its tests cannot establish compiler correctness, general race freedom, liveness, or any Lean theorem.

## 53. Implementation and Proof Roadmap

Preserve the base stage dependencies, especially stable ownership/error handling and typed CFG before extending the C analyzer. Concurrency work is not evidence that unfinished sequential stages are complete.

| Stage | Executable deliverable | Lean deliverable and acceptance gate |
|---|---|---|
| C0 | Whole-object permissions, FULL arithmetic, explicit split/merge, authorization checks | Partial-composition and exclusivity lemmas; exhaustive small-share tests |
| C1 | Structured task graph, move/read/write captures, spawn success/failure, explicit join | Reservation conservation, fork/join custody preservation, no early loan recovery |
| C2 | Child typed results, error-side loan returns, scope exits, cleanup dependency checking | Error preservation and quiescent destruction; every failure position tested |
| C3 | Built-in fixed-footprint mutex wrapper and structured guards | Invariant transfer, non-Send guard, unlock-edge correctness |
| C4 | Typed fields/ranges, proven disjointness, access-effect summaries | Spatial splitting and footprint soundness for the accepted layout subset |
| C5 | Concurrent flow epochs and explicit runtime cost axes | Conditional peak theorem, unique-resource accounting, credit partitioning |
| C6 | C11 runtime adapter and concurrent C emitter | Reduced operational simulation/translation-validation evidence; schedule-aware differential tests |
| C7 | Annotated C importer, task/lock summaries, conservative MHP and alias checks | Sound abstraction obligations; seeded positive/negative/unknown fixtures |

Every stage requires implementation tests and a successful `lake build` of its proof increment. Native Windows threading is an additional backend gate only after actual Windows execution; do not mark it complete from Linux tests.

Later extensions have separate gates for dynamic task collections, condition variables, ownership-carrying queues, dynamic mutex ownership invariants, atomics/publication, and reclamation. Do not expand the accepted subset merely because a grammar token exists.

## 54. Stable Diagnostics

Retain base identifiers and add:

```text
Q600  access lacks a positive permission
Q601  mutation requires full permission
Q602  permission over-allocation or unsupported composition
Q603  permission merge has mismatched identity/generation/origin
Q604  overlapping or unproved-disjoint footprints
Q605  destruction while a loan or task access is outstanding
Q606  non-Send value crosses a task boundary
Q607  non-Sync value shared between tasks
Q608  guard or thread-affine obligation crosses threads
Q609  task handle copied, dropped, detached, or escaped
Q610  checked scope exit has an outstanding task
Q611  spawn failure does not restore its reserved state
Q612  worker exit does not return borrowed capture authority
Q613  join fails to establish completion/quiescence
Q614  critical-section exit fails to restore invariant
Q615  guarded access uses the wrong mutex identity
Q616  guarded view or protected ownership escapes unlock
Q617  cleanup lacks access authority or dependency ordering
Q618  error opening attempts to abandon access authority
Q619  join attempted while holding a tracked mutex guard
Q620  unsupported atomic/publication/reclamation protocol
Q621  unknown parallel access, callback, or alias state
Q622  resource-bound composition lacks concurrency side conditions
Q623  shared allocation or task result counted more than once
Q624  stale shared-state refinement reused after interference
Q625  optional lock-order proof failed or is unknown
Q626  backend/profile/permission-scale incompatibility
Q627  permission reconstructed from a pointer, literal, or copied view
Q628  unsupported dynamic protected ownership shape
Q629  post-start task failure misclassified as creation failure
```

Diagnostics SHOULD show both relevant threads/sites, the overlapping footprint, permission amounts, current custody, missing synchronization or loan-return edge, and whether the issue is a proved violation, potential violation, or unknown analysis state.

## 55. Required Conformance Tests

The following is the required future compiler/analyzer corpus. It is **not** a statement that these tests have been implemented by the accompanying algebra model.

| ID | Scenario | Expected result |
|---|---|---|
| CT01 | Move an owned buffer to child; child returns it; parent joins and drops | Accept |
| CT02 | Parent uses buffer after successful move capture | Reject |
| CT03 | Parent and child share reads with complementary positive shares | Accept |
| CT04 | Parent writes while child retains a read share | Reject |
| CT05 | Two children each request FULL for the same memory | Reject |
| CT06 | Same pointer copied into differently named variables | Still same permission identity |
| CT07 | Two disjoint array halves mutated independently | Accept with bounds/disjointness proof |
| CT08 | Overlapping array ranges | Reject or analyzer potential/unknown |
| CT09 | Distinct ordinary struct fields | Accept only after layout/access proof |
| CT10 | Adjacent bit-fields in one memory location | Reject independent concurrent writes |
| CT11 | Full permission reconstructed while local shared aliases survive | Reject |
| CT12 | Create fails before any child starts | Capture state restored; no task obligation |
| CT13 | Second create fails after first succeeds | First task must still be joined |
| CT14 | Child finishes but parent has not joined | No restored parent authority |
| CT15 | Backend join failure | No quiescence, no environment reclamation |
| CT16 | Worker typed error after partial local acquisition | Clean locals, return loans, carry declared payload |
| CT17 | Worker error payload contains owned result | Join transfers it exactly once |
| CT18 | Worker error payload contains scoped capture borrow | Reject |
| CT19 | Parent `?` bypasses a live sibling task | Reject |
| CT20 | In-scope handler joins siblings before rethrow | Accept with all result obligations accounted |
| CT21 | Quant opening discards positive access permission | Reject |
| CT22 | Quant opening discards parent-local unused credit after joins | Accept |
| CT23 | Parent abandons child-owned credit during its own error | Reject |
| CT24 | Cleanup destroys an owner before child join | Reject |
| CT25 | Cleanup destroys owner after joins and loan expiry | Accept |
| CT26 | Two critical sections use the same mutex for the same payload | Accept |
| CT27 | Two different mutex identities allegedly guard the same payload | Reject without a valid composite protocol |
| CT28 | Guard is moved to a child or unlocked by another thread | Reject |
| CT29 | Typed error leaves a temporarily broken mutex invariant | Reject unless restored before unlock |
| CT30 | Guard-derived borrow escapes critical section | Reject |
| CT31 | Mutex destroyed with outstanding handle borrower/waiter | Reject |
| CT32 | Join while a guard is held | Reject in initial profile |
| CT33 | Lock order cycle in optional rank mode | Reject rank proof |
| CT34 | Protected owned graph grows/replaces identities | Unsupported in fixed-shape profile |
| CT35 | One shared 4096-byte allocation read by two tasks | Count 4096 once |
| CT36 | Concurrent incremental peaks 3072 and 5120 bytes | Bound at least 8192 plus modeled overhead |
| CT37 | Child returns allocation and finishes early | Allocation remains live in completion capsule |
| CT38 | Child credit splits total more than original credit | Reject |
| CT39 | Concurrent work bound used as wall-clock bound | Reject unsupported claim |
| CT40 | Atomic pointer points to prematurely freed memory | Not certified; unsupported lifetime/publication protocol |
| CT41 | `volatile` used as a completion flag | No synchronization proof |
| CT42 | Two descriptors share an underlying logical state | Preserve shared protocol-cell identity |
| CT43 | Unresolved callback may run after API return | Unknown; no whole-program proof |
| CT44 | Stack capture expires or environment moves before join | Reject |
| CT45 | Unknown destructor accesses shared global state | Require access contract or report unknown |
| CT46 | Known shared wrapper with non-Sync but Send payload | Accept only through verified mutex protocol |
| CT47 | Raw pointer is Copy but no transfer/share contract exists | Reject automatic cross-thread acceptance |
| CT48 | Stale heap refinement reused after mutex release/reacquire | Invalidate/re-establish, never assume |
| CT49 | Independent event order differs between C and interpreter | Validate partial order, not exact total trace |
| CT50 | New lifetime reuses old numeric address/descriptor | Old capability cannot authorize new identity |

Property tests MUST generate valid and invalid share partitions, alias-equivalent footprints, task success/error paths, and bounded interleavings. Exhaustive exploration is bounded evidence, not an unbounded proof.

C integration tests SHOULD include applicable race and memory sanitizers, but a clean sanitizer run is not proof of race absence. Instrumentation and test scheduler synchronization must be documented to avoid masking the tested behavior.

## 56. Illustrative Source Programs

These are intended surface examples for the grammar extension and predeclared library functions with the stated contracts. They are not claimed to have been compiled. Library-specific resource declarations are omitted, as in the base specification's schematic examples.

### 56.1 Concurrent reads and explicit join

```text
concurrency checked;

fn inspect(b: borrow<'b, const, Buffer<4096>>) -> usize
contract {
    access read b;
}
{
    return checksum(b);
}

fn two_reads(b: borrow<'b, const, Buffer<4096>>) -> bool
// The caller must provide a splittable shared permission group with FULL units.
// inspect and checksum are checked, non-throwing, read-only operations.
effects { throws<SpawnError>, spawn, join, may_block }
{
    thread scope s {
        let h = spawn s inspect(read b share 512)?;
        let local = checksum(b);
        let remote = join move h;
        let other = remote?;
        return local == other;
    }
}
```

The loan group for `b` is divided into 512 units for the child and 512 retained by the parent. Returning from the function returns the caller's borrowed authority; the function has not manufactured FULL merely by receiving a pointer. `remote` has no worker error alternatives in this example, so its `?` is a trivial success elimination; a real implementation may normalize it away.

The final equality compares copied scalar results and introduces no additional shared-memory access. A mutable caller may use the buffer again only after the returned shared loan has ended.

### 56.2 Forbidden write during a child read

```text
thread scope s {
    let h = spawn s inspect(read buffer share 512)?;
    overwrite(&mut buffer);   // Q601/Q105: child still holds a read share
    let result = join move h;
    consume_result(result);
}
```

Joining afterward does not repair the earlier conflicting mutation.

### 56.3 Mutex-protected updates

```text
fn increment(m: borrow<'m, const, mutex<u64>>) -> unit
{
    with lock m as g {
        g.value = checked_increment(g.value);
    }
    return ();
}
```

`checked_increment` must preserve the declared invariant on every normal/error path. Concurrent calls access the payload only through guards, not by treating shared access to the wrapper as writable access to `g.value`.

### 56.4 Cleanup order after an error

```text
// Conceptual CFG; not additional surface syntax.
create child borrowing buffer
parent operation fails
    -> handler INSIDE thread scope
    -> explicitly join child
    -> consume/drop/transfer child result or resource-bearing error
    -> expire returned loan group
    -> leave thread scope
    -> matching outer quant opening
    -> drop buffer and other authorized local resources
    -> abandon only eligible local credit remainder
    -> rethrow or recover
```

A compiler MUST reject the version that jumps directly from the parent failure to `drop buffer`.

## 57. Acceptance Criteria for Revision 0.2

Revision 0.2 is complete only when the sequential baseline gates required by the implemented subset are satisfied and all of the following hold:

1. The grammar additions and explicit corrigenda have parser/round-trip tests.
2. AccessQuant is distinct from vector quantities and from usage grades.
3. Permission and unique-resource ledgers preserve their separate invariants.
4. Provenance-aware whole-object and supported disjoint-footprint accesses are checked.
5. Captures, transactional creation failure, linear task handles, and explicit joins are implemented.
6. Both child normal and typed-error exits return scoped loan authority correctly.
7. No checked scope exit or error opening bypasses an outstanding task/loan.
8. Mutex invariant transfer, guard affinity, and all unlock edges are checked.
9. Runtime handoffs establish the required synchronization, including failed-operation distinctions.
10. Concurrent cost bounds include unique resources, completion capsules, and declared runtime overhead without using unsound max composition.
11. The required positive/negative/unknown corpus and bounded property tests pass.
12. C lowering is validated under schedule-aware comparisons for every claimed backend.
13. The reduced Lean development proves permission preservation, access exclusivity/ordering, quiescent destruction, and concurrent cleanup safety without `sorry`.
14. Analyzer reports distinguish proof, feasible witness, potential violation, and unknown; they expose all trusted assumptions.
15. No report equates this result with general race-condition freedom, deadlock freedom, lock-free correctness, or unrestricted legacy-C verification.

## 58. Added Primary Research and Platform Basis

These references support background mechanisms, not a completed proof of the proposed QCore combination. Existing §36 references are retained as the uploaded baseline bibliography; this revision does not independently revalidate every inherited entry.

**[C1]** John Boyland. *Checking Interference with Fractional Permissions*. SAS 2003, pp. 55–72. DOI: 10.1007/3-540-44898-5_4. The abstract describes partial read authority, full write authority, and structured-parallel semantics.

```text
https://link.springer.com/chapter/10.1007/3-540-44898-5_4
```

**[C2]** Viktor Vafeiadis. *Concurrent Separation Logic and Operational Semantics*. MFPS 2011. In particular, the operational soundness and resource-invariant treatment in §5. This is a foundation for proof organization, not a claim that its rules transfer unchanged to QCore.

```text
https://people.mpi-sws.org/~viktor/papers/mfps2011-cslsound.pdf
```

**[C3]** ISO/IEC JTC1/SC22/WG14. *N1570, Committee Draft, April 12, 2011*. See §5.1.2.4 for multithreaded execution/data races, §3.14 for memory locations, and §7.26 for threads. This is the public C11-era draft, not a claim to be the latest C standard.

```text
https://open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
```

**[C4]** The Rust Project. *The Rustonomicon: Send and Sync*. Transfer/share properties and important non-automatic cases. Consulted 2026-09-05.

```text
https://doc.rust-lang.org/nomicon/send-and-sync.html
```

**[C5]** The Rust Project. *std::thread::scope*. Scoped lifetimes and joining; documentation also distinguishes worker function completion from possible thread-local destructor activity. QCore specifies its own stricter quiescence contract rather than assuming identical semantics. Consulted 2026-09-05.

```text
https://doc.rust-lang.org/std/thread/fn.scope.html
```

**[C6]** WG14. *Clarification Request Summary for C11*, N2244, DR 405. Mutex lock/unlock ordering clarification.

```text
https://www.open-std.org/jtc1/sc22/wg14/www/docs/n2244.htm
```

**[C7]** LLVM Project. *Clang Thread Safety Analysis*. Capability environments, shared/exclusive requirements, and annotation-driven checking. Consulted 2026-09-05.

```text
https://clang.llvm.org/docs/ThreadSafetyAnalysis.html
```

## 59. Migration and Evidence Status

The source attachment remains unchanged. The consolidated revision retains its sequential chapters, applies only the documented banner/cross-reference/grammar changes, and appends this amendment with explicit precedence.

For existing sequential programs, inferred task-local full authority and ordinary borrowing should preserve acceptance except where the explicit grammar repairs clarify previously schematic syntax. Resource declarations used across threads additionally need Send/Sync and access-effect evidence; source-level compatibility is not permission to infer thread safety from old annotations.

The accompanying Python prototype is an executable experiment for a deliberately reduced permission/custody/synchronization model. Its validation report states exactly what ran. It is not the Rust compiler, a C front end, an implementation of this grammar, or a Lean proof. The normative acceptance criteria above remain outstanding until the actual implementation and proof artifacts satisfy them.

**Final decision:** implement checked concurrency through identity-indexed access quants, conserved ownership/custody, synchronization-backed transfers, and quiescence-aware exits. Do not implement it as an extra count in the existing resource vector, and do not let an error region erase a thread's outstanding access rights.
