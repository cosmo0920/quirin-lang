# `CheckedMutexInvariantV1` — schema draft

Companion to [`README.md`](README.md). **Draft. Proposal only** — no such type
exists in the workspace. Field and variant names are suggestions for the T02
integrator; the semantics are what matter. Rust-shaped for concreteness because
the target is `crates/qcore-core/src/mutex_invariant.rs`, a new sibling of
`concurrency.rs` (see README §4.1 for why it is a new module and not an
extension of `check_concurrent`).

Everything here is *additive*. Nothing in `concurrency.rs`,
`event_reactor.rs`, `library_custody.rs`, `weak_memory.rs` or
`proof/QCore/Concurrency.lean`'s `Lifetime` namespace is edited.

---

## 1. Identities

```rust
// Reused unchanged from `crate::concurrency`.
pub use crate::concurrency::{AccessFootprint, AccessKind, FULL_ACCESS, SharedResourceId, TaskId};

/// One mutex lifetime. Generation-indexed exactly like `SharedResourceId`:
/// never a reusable address, handle or source variable. A destroyed id is
/// dead forever.
pub type MutexId = u32;

/// One critical section. Minted fresh by each successful `Lock`; never reused,
/// including across two locks of the same mutex generation.
pub type GuardId = u32;

/// A declared invariant predicate. The predicate itself is a premise
/// (`InvariantPredicateDeclared`), not a proved proposition.
pub type InvariantId = u32;

/// A borrow taken through a guard. Dies at that guard's unlock.
pub type BorrowId = u32;

/// A refinement fact asserted about protected memory. Scoped to one `GuardId`.
pub type FactId = u32;

pub type LockRank = u32;
```

Rationale for `GuardId` being distinct from `MutexId`: authority must be keyed
to the *critical section*, not the mutex, so that CT48 (stale refinement across
release/reacquire) and CT30 (guard-derived borrow escape) are expressible at
all. Two locks of one mutex generation must be two distinct authorities.

---

## 2. Program

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MutexInvariantProgram {
    /// Pointwise share authority at trace entry, per task. Unlike
    /// `ConcurrentProgram::permissions` this is the *initial* state of a
    /// mutable ledger, not a static table consulted at every access.
    pub initial_shares: Vec<ShareGrant>,
    pub initial_custody: BTreeMap<SharedResourceId, TaskId>,
    /// Optional. When present, adds `LockRankDeclared` to the certificate and
    /// enables cycle rejection. A *passing* rank check certifies nothing about
    /// deadlock freedom (spec L16, `QCORE_SPEC_r0_3.md:339`).
    pub lock_ranks: Option<BTreeMap<MutexId, LockRank>>,
    pub events: Vec<MutexInvariantEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShareGrant {
    pub task: TaskId,
    pub footprint: AccessFootprint,
    pub share: u16,
}
```

**Ledger invariant, checked at every step.** For every point `p` of every
resource:

```
  sum over tasks of share(task, p)  +  parked(p)  ==  FULL_ACCESS
```

where `parked(p)` is the share held by whichever live mutex protects `p`, or 0
if none does. Violation → `MutexAuthorityNotConserved`. This is the direct
implementation of `QCORE_SPEC_r0_3.md:341` ("Pointwise shares across all
custodians, parked mutex invariants, and completed-but-unjoined task capsules
MUST be conserved"). Note the third conjunct: composing with the scoped-task
ledger's completion capsules is **out of scope for v1** and any program mixing
them rejects.

---

## 3. Events

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutexInvariantEvent {
    // ---- task structure (semantics unchanged from `concurrency.rs`) ----
    Spawn { parent: TaskId, child: TaskId },
    Join  { parent: TaskId, child: TaskId },

    // ---- mutex lifecycle ----
    /// Fixes the protected shape. `protects` is immutable for this mutex
    /// generation's whole lifetime.
    CreateMutex {
        task: TaskId,
        mutex: MutexId,
        protects: Vec<AccessFootprint>,
        invariant: InvariantId,
    },
    DestroyMutex { task: TaskId, mutex: MutexId },

    // ---- critical sections ----
    /// Mints `guard`. The trace supplies the id; the checker requires it to be
    /// globally fresh (never previously minted), which is the `GuardId`
    /// analogue of the unique-lifetime-id rule.
    Lock   { task: TaskId, mutex: MutexId, guard: GuardId },
    Unlock { task: TaskId, guard: GuardId },
    /// Typed-error exit from a critical section. Distinguished from `Unlock`
    /// only so the diagnostic can name the error path (CT29).
    ErrorExit { task: TaskId, guard: GuardId, exit: u32 },

    // ---- invariant state ----
    OpenInvariant  { task: TaskId, guard: GuardId },
    CloseInvariant { task: TaskId, guard: GuardId, witness: InvariantWitness },

    // ---- access ----
    Access {
        task: TaskId,
        footprint: AccessFootprint,
        kind: AccessKind,
        /// `None` is legal only for a footprint no live mutex protects.
        /// There is no fallback: a protected footprint with `None` rejects.
        guard: Option<GuardId>,
    },

    // ---- guard-derived borrows (CT30) ----
    BorrowUnderGuard { task: TaskId, guard: GuardId, footprint: AccessFootprint, borrow: BorrowId },
    EndBorrow        { task: TaskId, borrow: BorrowId },

    // ---- guard-scoped refinement facts (CT48) ----
    AssertFact { task: TaskId, guard: GuardId, footprint: AccessFootprint, fact: FactId },
    UseFact    { task: TaskId, fact: FactId },

    // ---- custody ----
    /// No `synchronized: bool`. The witness is verified against the checker's
    /// own derived happens-before relation.
    TransferCustody {
        from: TaskId,
        to: TaskId,
        resource: SharedResourceId,
        via: SyncWitness,
    },
    Destroy { task: TaskId, resource: SharedResourceId },
}

/// The premise the checker cannot discharge: that the declared predicate
/// actually holds of the bytes. Recorded, named, and never inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvariantWitness {
    /// Restored by a declared contract clause. Adds
    /// `InvariantPredicateDeclared` to the certificate.
    Declared { invariant: InvariantId },
    /// No write occurred while open, so the invariant is unchanged. This one
    /// is *checked*, not declared, and adds no assumption.
    NoWriteOccurred,
}

/// Every variant names an edge the checker must find in its own clock history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncWitness {
    /// `from` spawned `to`.
    SpawnEdge { child: TaskId },
    /// `to` joined a descendant of `from`.
    JoinEdge { child: TaskId },
    /// A release of `mutex` by `from` happens-before an acquire by `to`, AND
    /// `mutex` protects `resource`. A release/acquire pair on an unrelated
    /// mutex does not publish an unrelated resource.
    MutexHandoff { mutex: MutexId },
}
```

**No `bool` appears in any authority position.** That is the structural
difference from `ConcurrentEvent` (`concurrency.rs:79-114`), where
`Access.guarded_by` asserts coverage that is never checked (`:415-426`) and
`TransferCustody.synchronized` injects a clock edge (`:478-489`).

---

## 4. Checker state

```rust
struct MutexState {
    protects: Vec<AccessFootprint>,   // fixed at CreateMutex, never mutated
    invariant: InvariantId,
    lock: LockState,
    waiters: BTreeSet<TaskId>,
    release_clock: Option<VectorClock>,   // keyed to this generation
}

enum LockState {
    Unlocked,
    Held { guard: GuardId, holder: TaskId, invariant: InvariantState },
}

enum InvariantState { Closed, Open }

struct GuardState {
    mutex: MutexId,
    holder: TaskId,
    borrows: BTreeSet<BorrowId>,
    facts: BTreeSet<FactId>,
    wrote_while_open: bool,   // gates `InvariantWitness::NoWriteOccurred`
}
```

Plus: `shares: pointwise ledger`, `custody: BTreeMap<SharedResourceId, TaskId>`,
`destroyed_resources: BTreeSet<SharedResourceId>`,
`destroyed_mutexes: BTreeSet<MutexId>`, `minted_guards: BTreeSet<GuardId>`
(freshness), `clocks: BTreeMap<TaskId, VectorClock>`, and the recorded-access
list for race detection — all as in `concurrency.rs`, plus the two liveness
ledgers.

---

## 5. Transition rules

Ordering matters: liveness is tested **before** authority, so a
use-after-destruction is never reported as an ownership complaint. This is the
V01 rule (`../remaining-v01/README.md`, repeat-destruction row) transposed onto
mutexes.

| Event | Preconditions, in evaluation order | Diagnostic on failure |
|---|---|---|
| `CreateMutex` | task active · mutex id never used · every interval valid, live, pairwise disjoint · no interval already parked · task holds `FULL_ACCESS` on every interval | `DeadMutexGeneration` · `InvalidFootprint` · `FootprintAlreadyProtected` · `MutexAuthorityNotConserved` |
| `Lock` | task active · mutex live · `Unlocked` · guard id fresh · rank order (if rank mode) | `DeadMutexGeneration` · `DuplicateLockCapability` · `LockRankViolation` |
| `Unlock` / `ErrorExit` | task active · guard live · `holder == task` · state `Closed` · `borrows` empty · no live fact in use | `DeadGuard` · `WrongGuardAffinity` · `InvariantOpenAtUnlock` / `InvariantOpenAtErrorExit` · `GuardBorrowEscape` |
| `OpenInvariant` | guard live · `holder == task` · `Closed` | `WrongGuardAffinity` · `DoubleOpenInvariant` |
| `CloseInvariant` | guard live · `holder == task` · `Open` · witness consistent with `wrote_while_open` | `WrongGuardAffinity` · `InvalidInvariantWitness` |
| `Access` (unprotected footprint) | resource live · share ledger authorizes (`FULL_ACCESS` for `Write`, `> 0` otherwise) | `DeadResource` · `UnauthorizedAccess` |
| `Access` (protected footprint) | resource live · `guard == Some(g)` · `g` live · `holder(g) == task` · `mutex(g).protects` **contains** `footprint` · for `Write`, state is `Open` | `DeadResource` · `UnguardedProtectedAccess` · `DeadGuard` · `WrongGuardAffinity` · **`UnprotectedFootprint`** · `WriteWithClosedInvariant` |
| `BorrowUnderGuard` | guard live · affinity · footprint within `mutex(g).protects` · borrow id fresh | `UnprotectedFootprint` · `WrongGuardAffinity` |
| `AssertFact` / `UseFact` | fact's guard still live | `StaleProtectedFact` |
| `TransferCustody` | both tasks active · resource live · `custody[resource] == from` · **the edge named by `via` exists in the derived happens-before relation** · for `MutexHandoff`, that mutex protects `resource` | `DeadResource` · `WrongCustodian` · **`ForgedSynchronizationWitness`** |
| `Destroy` (resource) | task active · resource live · `custody == task` · **no live mutex parks any of its footprint** · other holders terminal · prior accesses ordered | `DeadResource` · `WrongCustodian` · `NonquiescentDestruction` |
| `DestroyMutex` | task active · mutex live · `Unlocked` · no waiters · no live borrow/fact from any guard of this mutex · parked share accounted for · prior accesses to protected footprints ordered | `DeadMutexGeneration` · `NonquiescentMutexDestruction` · `MutexAuthorityNotConserved` |
| any event naming a destroyed mutex | — | `DeadMutexGeneration` |
| condvar / try-lock / timed lock / rwlock / nested-protecting mutex / footprint mutation | — | `UnsupportedSynchronizationConstruct` · `DynamicProtectedShape` |

End-of-trace sweep: outstanding tasks, outstanding guards, **outstanding live
mutexes**, and a final ledger-conservation check.

The **row that carries R03** is `Access` (protected footprint), fifth
precondition. `concurrency.rs` has no equivalent: it checks only
`locks.get(&mutex).map(|owner| owner.0) == Some(task)`.

---

## 6. Certificate

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexInvariantCertificate {
    pub profile: ConcurrencyProfile,              // == CheckedMutexInvariantV1
    pub assumptions: BTreeSet<ConcurrencyAssumption>,
    pub checked_events: usize,
    pub completed_tasks: Vec<TaskId>,
    pub destroyed_resources: Vec<SharedResourceId>,
    pub destroyed_mutexes: Vec<MutexId>,
    /// Which mutex protected which footprint. Makes the discharged relation
    /// auditable rather than implicit.
    pub protected_shape: BTreeMap<MutexId, Vec<AccessFootprint>>,
    /// Every synchronization edge the checker *derived*, so a reader can see
    /// that none was supplied.
    pub derived_sync_edges: Vec<(TaskId, TaskId, SyncWitness)>,
}
```

Admissible assumptions in this profile:

| Assumption | Status | Why it remains |
|---|---|---|
| `InitialAuthorityAndUniqueLifetimeIds` | carried over | ids denote lifetimes; unmodeled authority is out of scope |
| `DeclaredFootprintShape` | **new** | declared intervals correspond to real C layout; `QCORE_SPEC_r0_3.md:187` excludes C layout derivation |
| `InvariantPredicateDeclared` | **new**, only when some `CloseInvariant` used `Declared` | the predicate is not proved; source contract clauses are still centrally refused at `Q021` |
| `LockRankDeclared` | **new**, only in rank mode | and never a deadlock proof |

**Forbidden in this profile, and pinned by a negative test:**
`GuardFootprintAuthority` and `SynchronizedCustodyEdges`. Their absence *is* the
R03 discharge claim. Both remain in force, unchanged, for
`TrustedExplicitTrace`.

The free-form `assumptions: BTreeMap<String, bool>` route in
`qcore-schemas` (`obligation.rs:238, 265`; `plugin.rs:365, 416, 572, 599`) is
unconstrained by any enum, so the pinned test must check the *typed*
`BTreeSet<ConcurrencyAssumption>` and the serialized map independently.

---

## 7. Schema-surface impact

| Item | Change | Compatibility |
|---|---|---|
| `ConcurrencyProfile` | `+ CheckedMutexInvariantV1` | safe: no exhaustive match outside `concurrency.rs:547` and `tests/unit/concurrency.rs:913`. `#[non_exhaustive]` recommended |
| `ConcurrencyAssumption` | `+ DeclaredFootprintShape`, `+ InvariantPredicateDeclared`, `+ LockRankDeclared` | additive; existing variants kept |
| `ObligationProperty::MutexGuardReleased` | gains a derivation and a witness kind | currently *"reserved: no derivation, no witness kind"* (`obligation.rs:99-101`); already serializes as `"mutex_guard_released"`, so filling it changes no wire shape |
| `ObligationSubject::MutexIdentity { name }` | becomes the subject of derived obligations | already reserved and serializing (`obligation.rs:173-179`, canonical `"mutex\|{name}"`) |
| `qcore-schemas` version | no bump if the above hold | if a new `#[serde(tag = …)]` variant is required, `docs/reports/schema-1-compatibility/README.md`'s breaking-change policy applies |
| `research_closure.rs:43` | must record which profile its embedded certificate came from | a `TrustedExplicitTrace` certificate must not be consumed as a checked one |
| `crates/qcore-core/src/lib.rs:72-74` | new re-exports | additive |
| Cycle 19 artifacts, CT manifest, contract corpus | none | model-only; byte parity required after T02 |
