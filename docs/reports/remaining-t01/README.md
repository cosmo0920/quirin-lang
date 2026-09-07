# T01 — Checked fixed-footprint mutex profile: specification and trust-premise ledger

Date: 2026-09-07. Worktree HEAD at authoring: `3f962e1`. Implementation
checkpoint: `e3e7c2c`. Companion schema draft: [`profile.md`](profile.md).

**Scope.** This is a *design* card (`bounded_design`). It writes only under
`docs/reports/remaining-t01/`. No tracked file is modified, no gate is weakened,
no pinned expectation is edited, no CT case is mapped, no verdict is promoted,
and no normative specification text is changed — the `docs/QCORE_SPEC_r0_3.md`
L16 and `docs/QCORE_CONTRACT_SPEC_r0_3.md` edits this design implies are
deferred to T02 implementation under the integrator's single-writer rule.
Everything below is a proposal.

**What this card answers.** R03 (`docs/reviews/r0_3/supplied/REVIEW.md:67-78`) is
classified `unsupported-by-profile` in
[`../remaining-b02/README.md`](../remaining-b02/README.md) because
`crates/qcore-core/src/concurrency.rs:204-207` declares exactly one profile,
`TrustedExplicitTrace`, and carries mutex authority as the named certificate
assumptions `GuardFootprintAuthority` and `SynchronizedCustodyEdges`. T00
(`../remaining-t00/README.md` §5.2) separately established that CT26–CT34, CT46
and CT48 are blocked not on guard-name binding — `w6-frontend-gates` repaired
that — but on the fact that **no source form creates or declares a mutex at
all**. This card specifies the profile that would discharge the first half and
the source surface that would unblock the second, and it states exactly which
premises would *remain* trusted in the older abstract profile after that work.

---

## 0. The defect the profile must repair, stated precisely

`crates/qcore-core/src/concurrency.rs:415-426`:

```rust
let guarded = guarded_by.is_some_and(|mutex| {
    if locks.get(&mutex).map(|owner| owner.0) == Some(task) {
        assumptions.insert(ConcurrencyAssumption::GuardFootprintAuthority);
        true
```

and at `:427`, `if !guarded && !authorized(...)`. The check that runs is *"the
task holds the mutex it named"*. The check that is **named as a premise but
never performed** is *"the accessed footprint falls under that mutex's
invariant"* — `ConcurrentProgram` (`:116-121`) has no mutex→footprint relation
to check against. Holding mutex 1 therefore authorizes writing any footprint of
any live resource, and suppresses `UnauthorizedAccess` while doing so.

`concurrency.rs:478-489` is the second half: `TransferCustody { synchronized:
true }` calls `merge_clock` at `:487`, i.e. a caller-supplied `bool` **injects a
happens-before edge into the race detector's vector clocks**.

Both are deliberate, documented and named on the certificate — the module header
at `concurrency.rs:8-11` says so, and
`certificates_name_trusted_model_authority_without_removing_guard_semantics`
pins that naming. R03's own text permits the bounded model to retain them
("The existing bounded model may retain an explicit trust premise, but its
certificate must not be advertised as having established that premise"). What
R03 additionally requires, and what does not exist, is a *second, stronger*
profile in which those two premises are discharged rather than declared.

The precedent for how to discharge them already exists in this workspace:
`crates/qcore-core/src/weak_memory.rs:175-186` computes
`expected_synchronizes_with` and rejects both directions of disagreement
(`InvalidSynchronizesWith` / `MissingSynchronizesWith`), and
`validate_mutex_histories` (`:404-471`) derives mutex synchronization from
checked lock/unlock alternation rather than accepting it. **The checked mutex
profile should adopt that derive-and-compare discipline verbatim.** That is the
single most important architectural decision in this design.

---

## 1. Profile definition — `CheckedMutexInvariantV1`

### 1.1 Identity

| Object | Identity | Rule |
|---|---|---|
| Task | `TaskId` | as today |
| Protected resource | `SharedResourceId` | as today: one unique lifetime, never a reusable address (`ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds`) |
| **Mutex** | `MutexId` | **new: generation-indexed exactly like a resource.** One `MutexId` denotes one mutex lifetime. A `MutexId` that has been destroyed is dead forever; reusing it is `DeadMutexGeneration`, never a fresh mutex |
| **Guard instance** | `GuardId` | **new:** a fresh, never-reused id minted by each successful `Lock`. Authority is keyed to the *guard instance*, not the mutex id |
| **Protected fact** | `FactId` | **new:** a refinement asserted about protected memory, scoped to one `GuardId` |

Mutex generation liveness is a checked ledger with the same shape and the same
adversarial properties V01 verified for `destroyed: BTreeSet<SharedResourceId>`
(`../remaining-v01/README.md`): a rejected destruction must leave the generation
**live** (so `DeadMutexGeneration` is unreachable through a refused destroy), a
repeat destruction must report `DeadMutexGeneration` and **not** an ownership
complaint, and no post-destruction event of any kind may be admitted. V01's
whole coverage matrix transposes onto `MutexId` and is the acceptance shape for
T02's mutex-liveness tests.

`MutexId` is today a bare `u32` (`concurrency.rs:21`) with no creation event and
no liveness ledger; that is why it needs both.

### 1.2 Fixed footprint, declared at creation

```
CreateMutex { task, mutex, protects: Vec<AccessFootprint>, invariant: InvariantId }
```

Rules:

1. **Fixed shape.** `protects` is recorded at creation and is immutable for the
   mutex's whole lifetime. No later event may add, remove, widen, narrow or
   substitute an interval. Any event proposing to → `DynamicProtectedShape`
   (this is CT34's `Unsupported in fixed-shape profile`).
2. **Well-formedness.** Every interval must satisfy `is_valid()`, every named
   resource must be live at creation, and the intervals must be pairwise
   non-overlapping within the declaration.
3. **Authority is moved, not copied.** The creating task must hold
   `FULL_ACCESS` share over every declared interval. `CreateMutex` **consumes**
   that share and parks it in the mutex. This is the direct implementation of
   `docs/QCORE_SPEC_r0_3.md:341` — "Pointwise shares across all custodians,
   parked mutex invariants, and completed-but-unjoined task capsules MUST be
   conserved." Failure → `MutexAuthorityNotConserved`.
4. **A footprint has at most one protector.** Because creation consumes
   `FULL_ACCESS`, a second `CreateMutex` naming an already-parked interval
   cannot obtain the share → `FootprintAlreadyProtected`. This *is* CT27 ("two
   different mutex identities allegedly guard the same payload → reject without
   a valid composite protocol"), and it falls out of conservation rather than
   needing its own rule.
5. Consequently `permissions` stops being the static `Vec<AccessPermission>`
   table of `concurrency.rs:118` and becomes a **mutable pointwise share
   ledger** whose total over `(task shares) + (parked shares)` is invariant at
   `FULL_ACCESS` per point. That ledger, not a lock name, is the authority.

### 1.3 The state machine

Per mutex generation:

```
                CreateMutex                        DestroyMutex
   (absent) ─────────────────▶ Unlocked ───────────────────────▶ Dead
                                │  ▲
                          Lock  │  │  Unlock  (requires Closed,
                     (mints g)  │  │           no live g-borrow)
                                ▼  │
                          Held(g, task, Closed) ◀──── CloseInvariant(g)
                                │  ▲                   (witness required)
                 OpenInvariant(g)│  │
                                ▼  │
                          Held(g, task, Open)
```

Per guard instance `g` held by `task` over mutex `m`:

| Operation | Precondition | Effect |
|---|---|---|
| `Access{ guarded_by: g, footprint, kind: Read }` | `g` live, `holder(g) == task`, `m.protects` **contains** `footprint` | permitted in `Closed` and `Open` |
| `Access{ guarded_by: g, footprint, kind: Write }` | as above **and** state is `Open` | a write is what breaks the invariant; it must be declared broken first |
| `OpenInvariant{ g }` | `holder(g) == task`, state `Closed` | → `Open` |
| `CloseInvariant{ g, witness }` | `holder(g) == task`, state `Open` | → `Closed`; drops every `FactId` invalidated by the writes performed while `Open` |
| `Unlock{ g }` | `holder(g) == task`, state `Closed`, no live `g`-derived borrow, no live `g`-scoped fact in use | parked share returns to `m`; `g` dies; release clock recorded for the *mutex generation* |
| `ErrorExit{ g, exit }` | state `Closed` | typed-error path unlock; if `Open` → `InvariantOpenAtErrorExit` |

**The invariant holds at unlock and may be broken only between lock and unlock
by the holder** is therefore two rules: `Unlock` requires `Closed`
(`InvariantOpenAtUnlock`), and `OpenInvariant`/writes require guard affinity so
only the holder can break it.

### 1.4 Access authority — the R03 repair

In `CheckedMutexInvariantV1` there is **no fallback**. Exactly one of:

- footprint is unprotected (no mutex parks it) → the pointwise share ledger
  authorizes, exactly as `authorized()` does today;
- footprint is protected by `m` → **only** a live guard `g` on `m`, held by the
  acting task, with `m.protects.contains(footprint)`, authorizes.

Hence:

| Situation | Verdict | Diagnostic |
|---|---|---|
| access under a guard whose mutex does not protect the footprint | **reject** | `UnprotectedFootprint` |
| unguarded access to a protected footprint, even with a stale static permission | **reject** | `UnguardedProtectedAccess` |
| access under a guard held by a different task | **reject** | `WrongGuardAffinity` |
| access under a guard minted by an earlier, already-unlocked critical section | **reject** | `DeadGuard` |

The first row is R03's acceptance sentence "deny unrelated-lock access in a
checked-invariant profile", and it is the one line of behaviour that the current
model provably does not have.

### 1.5 Synchronization witnesses — the second R03 repair

`TransferCustody.synchronized: bool` is **not read** in this profile. Custody
transfer instead carries a witness that the checker verifies against its own
clock history, in the `weak_memory` style:

```
TransferCustody { from, to, resource, via: SyncWitness }

SyncWitness = SpawnEdge { child }            // `from` spawned `to`
            | JoinEdge  { child }            // `to` joined `from`'s descendant
            | MutexHandoff { mutex }         // release(m) by `from` happens-before acquire(m) by `to`
```

`MutexHandoff` additionally requires that `mutex` **protects** `resource` — a
release/acquire pair on an unrelated mutex does not publish an unrelated
resource. A witness whose named edge is absent from the derived happens-before
relation → `ForgedSynchronizationWitness`. This is R03's "reject forged handoff
edges", and it is checkable because the checker already computes the vector
clocks it would need to consult; today it merely *writes* to them on the
strength of the bool.

### 1.6 Guard-derived borrows and stale refinements

- A borrow taken through `g` (`BorrowUnderGuard { g, footprint, borrow }`) is
  keyed to `g`. `Unlock{g}` requires every `g`-derived borrow to be dead →
  `GuardBorrowEscape`. This is CT30.
- A `FactId` asserted about protected memory is scoped to `g` and is **dropped
  at `Unlock`**, unconditionally. Re-acquiring the mutex mints a fresh `g` with
  an empty fact set; the fact must be re-established. Using a fact keyed to a
  dead guard → `StaleProtectedFact`. This is CT48 and it implements
  `docs/QCORE_SPEC_r0_3.md:345` — "Facts about mutable protected memory must be
  invalidated or re-established across release and reacquisition."
- Guards are **not transferable**: any transfer, capture or spawn that would
  carry a `GuardId` across a task boundary → `WrongGuardAffinity`. With
  affinity, this is CT28's "guard is moved to a child or unlocked by another
  thread".

### 1.7 Quiescence for mutex destruction

`DestroyMutex { task, mutex }` requires **all** of:

1. `mutex` live (else `DeadMutexGeneration`, and V01's rule applies: a refused
   destroy leaves it live);
2. state `Unlocked` — no holder;
3. no recorded waiter on `mutex`;
4. no live guard-derived borrow or `g`-scoped fact from **any** generation of
   `mutex`;
5. the parked authority is accounted for: it returns to `task`, who must be
   entitled to receive it, or the protected resources are already destroyed
   under §1.8. Otherwise `MutexAuthorityNotConserved`;
6. every prior access to the protected footprints happens-before this event, by
   the same ordering rule `check_concurrent` already applies to resource
   destruction.

Failure of 2–4 or 6 → `NonquiescentMutexDestruction`. This is CT31.

### 1.8 Interaction with resource destruction (V01)

Destroying a *protected resource* while a live mutex parks authority over it is
`NonquiescentDestruction` — parked authority is outstanding authority. This
strictly strengthens `concurrency.rs:509-518`, whose comment currently says the
quantity it needs is unavailable:

> Fixed permissions belonging to other nonterminal (including future) tasks
> cannot be silently revoked. Guard footprints are trusted rather than
> represented, so conservatively require all other tasks' currently held guards
> to be closed.

With footprints represented, that conservative global sweep ("*all* other
tasks' guards closed") becomes a precise test ("no guard over a mutex protecting
*this* resource is live"), which both admits more correct programs and rejects
the case the sweep misses — a live parked mutex with no guard currently held.

### 1.9 Explicitly rejected dynamic cases

These are refusals, not omissions. Each names its diagnostic so a T02 test can
pin it, and each stays `Unknown`/reject rather than degrading to the older
profile's optimistic behaviour.

| Rejected case | Diagnostic | Note |
|---|---|---|
| Growing / shrinking / replacing the protected graph (**CT34**) | `DynamicProtectedShape` | the whole point of "fixed footprint" |
| Protecting a footprint already parked by another mutex (**CT27**) | `FootprintAlreadyProtected` | no composite protocol |
| Deadlock freedom from lock ranks (**CT33**) | `LockRankViolation` on a cycle; a *passing* rank check certifies nothing | `docs/QCORE_SPEC_r0_3.md:339` — "Lock-order checks do not, by themselves, establish freedom from all deadlocks." Rank mode stays optional and, when used, adds the `LockRankDeclared` assumption |
| Condition variables, wait/notify | `UnsupportedSynchronizationConstruct` | no event constructor; whole program rejects. `PromotedConcurrencyFeature::ConditionVariables` (`native_concurrency.rs:304-311`) already disposes this `Unknown` and must stay that way |
| Wrong-thread unlock (**CT28**) | `WrongGuardAffinity` | |
| Recursive / reentrant acquisition | `DuplicateLockCapability` | matches the existing model; initially-unlocked non-recursive mutexes only |
| `try_lock`, timed lock, `pthread_mutex_trylock` shapes | `UnsupportedSynchronizationConstruct` | a failed acquisition is a control-flow join this profile does not model |
| Partial parking — splitting a share across the mutex boundary | `MutexAuthorityNotConserved` | parking is all-or-nothing `FULL_ACCESS` |
| Nested mutexes where one protects another's footprint | `UnsupportedSynchronizationConstruct` | out of profile; not silently allowed |
| Reader/writer locks, upgradeable guards | `UnsupportedSynchronizationConstruct` | |
| `Send`/`Sync` derivation for a payload type (**CT46**) | out of profile | `docs/QCORE_SPEC_r0_3.md:187` (L09): "Send/Sync derivation do not follow from that API." CT46 stays unmapped after this work |

---

## 2. Trust-premise ledger

Every production site in the workspace where lock-held state, a `synchronized`
flag, or mutex authority is **accepted rather than derived**. Classification is
against `CheckedMutexInvariantV1` as specified above.

- **subsumed** — the checked profile performs the check; the premise disappears
  *from certificates issued in that profile* and remains only in
  `TrustedExplicitTrace`, which is unchanged.
- **retained** — still a declared premise after T02/T03; must stay named on its
  certificate.

| # | Site | Trusted input | Authorizes | Class |
|---|---|---|---|---|
| A1 | `crates/qcore-core/src/concurrency.rs:102` | `Access.guarded_by: Option<MutexId>` | the claim that a mutex invariant covers the access | **subsumed** (§1.4) |
| A2 | `crates/qcore-core/src/concurrency.rs:415-426` | same | bypasses the permission check entirely at `:427`; no footprint↔mutex test exists | **subsumed** (§1.2, §1.4) — the sharpest site, and R03's actual subject |
| A3 | `crates/qcore-core/src/concurrency.rs:108` | `TransferCustody.synchronized: bool` | a caller-asserted happens-before edge | **subsumed** (§1.5) |
| A4 | `crates/qcore-core/src/concurrency.rs:478-489` | same | `merge_clock` at `:487` injects the edge into the race detector | **subsumed** (§1.5) |
| A5 | `crates/qcore-core/src/concurrency.rs:214-217` | `GuardFootprintAuthority`, `SynchronizedCustodyEdges` | records A2/A4 as premises | **subsumed** — a `CheckedMutexInvariantV1` certificate must **never** carry either; pin it (§4.1) |
| A6 | `crates/qcore-core/src/concurrency.rs:509-518` | "guard footprints are trusted rather than represented" | conservative global guard sweep at resource destruction | **subsumed** (§1.8) — becomes precise |
| A7 | `crates/qcore-core/src/concurrency.rs:356-365` (`LockRank`) | declared rank per lock | lock-order claim | **retained**, downgraded to `LockRankDeclared`; never a deadlock proof (§1.9) |
| A8 | `crates/qcore-core/src/event_reactor.rs:275, 1086-1119` | `EventOperation::Access.synchronized: bool` | `!(*synchronized \|\| prior.synchronized)` at `:1102` — either side's bool alone suppresses `L05ConflictingAccess` | **retained** — different model (abstract reactor profile), out of T01 scope, must not be deleted |
| A9 | `crates/qcore-core/src/event_reactor.rs:441` | `RecordedAccess.synchronized` | persists A8 so it excuses *future* conflicting accesses | **retained** with A8 |
| A10 | `crates/qcore-core/src/event_reactor.rs:242-244, 903-907` | `CreateBufferevent { thread_safe, deferred, unlock_callbacks }` | declares the bufferevent carries a lock | **retained** — only shape rules (L10/L04) constrain it |
| A11 | `crates/qcore-core/src/event_reactor.rs:202, 633-644` and `:283, 1132-1143` | `Activate.cross_thread`, `LoopRequest.cross_thread` | stamps `ObservedKind::Synchronization` with no edge derivation | **retained** |
| A12 | `crates/qcore-core/src/event_reactor.rs:146-151, 1125` | `InvalidOrderingEvidence::{Priority, Readiness, Timing, TraceOrder}` | nothing — already surfaced as `L08OrderingAssumption` | **retained, already correct**: this is the pattern the others should follow |
| A13 | `crates/qcore-core/src/library_custody.rs:28-31, 89` | `LibraryCustodyProgram.evidence` map | the evidence table *is* the premise | **retained** — external C libraries, out of scope |
| A14 | `crates/qcore-core/src/library_custody.rs:438-449` | `available.get(&evidence) == Some(&expected)` | `Publish`/`Handoff`/`Reclaim` custody moves | **retained** — a *tag match*, not an ordering check; `checked` is insert-only so one evidence id can authorize unbounded transitions |
| A15 | `compiler/src/verifier_plugins/libevent/event_driven.rs:246, 367-372` | plugin-level `synchronized` | pass-through `From` into A8 | **retained** with A8 |
| A16 | `crates/qcore-core/src/weak_memory.rs:100-106, 175-186, 404-471` | `synchronizes_with`, `mutex_order` | — | **already derived**, both directions checked. Precedent, not a premise. Retained boundary: L17 — the candidate *set* is not complete |
| A17 | `crates/qcore-core/src/native_concurrency.rs:304-311` | `PromotedConcurrencyFeature::{DynamicMutexInvariants, ConditionVariables, …}` | nothing — dispositioned `Unknown` | **retained, fail-closed and correct**; §1.9 keeps it that way |
| A18 | `crates/qcore-schemas/src/obligation.rs:99-101, 173-179` | `ObligationProperty::{MutexGuardReleased, JoinBeforeExit}`, `ObligationSubject::{MutexIdentity, TaskIdentity}` | nothing yet — marked *"reserved: no derivation, no witness kind"* | **subsumed target**: T02/T03 fill these rather than inventing new vocabulary |
| A19 | `crates/qcore-schemas/src/obligation.rs:238, 265`; `plugin.rs:365, 416, 572, 599` | `assumptions: BTreeMap<String, bool>` | any assumption name, as a free-form string | **retained** — no enum constrains an assumption key. A checked-profile certificate must not be forgeable through this map; see §4.1 |
| B1 | `proof/QCore/Concurrency.lean:56, 68-70` | `.transfer … (synchronized : Bool)`, `synchronized = true` | custody transfer legitimacy in the Lean step relation | **subsumed** (needs a new namespace, §4.2) |
| B2 | `proof/QCore/Concurrency.lean:57, 79-80` | `.destroy … (priorAccessesOrdered : Bool)` | destruction ordering | **retained** in `Lifetime`; the checked profile derives it |
| B3 | `proof/QCore/Concurrency.lean:24, 92-94` | `guardHolders : List TaskId`; `.lock task` is unconditional | — | **subsumed and load-bearing**: `abbrev MutexId` exists at `:9` but the `Lifetime` step relation **never uses it**. The Lean model literally cannot express "an unrelated lock", so it cannot state R03's obligation. T02 must add mutex identity to the Lean model |
| B4 | `proof/QCore/Concurrency.lean:387-388, 408-417` | `usedGuardAuthority`, `usedSynchronizedHandoff` bools folded into `assumptions` | the premise list on the Lean certificate | **subsumed** — the checked profile's Lean certificate must not admit them |
| D1 | `contract/qcore/librdkafka/typed_queue.qcore:28, 40` | `requires queue_lock_is_held;` | dequeue preconditions | **retained** — no producer of this fact exists |
| D2 | `contract/qcore/librdkafka/typed_queue.qcore:32, 44` | `ensures … handoff_has_synchronization_evidence;` | handoff postcondition | **retained** |
| D3 | `contract/qcore/library/custody_evidence.qcore:26, 40, 53, 67` | `evidence_kind_is_*`, `every_cross_context_transition_has_typed_evidence` | A13/A14's declared side | **retained**; the file's own header `:4-5` already disclaims it |
| D4 | `contract/qcore/chunkio/lock_state.qcore:14, 30`, `fluent_bit/busy_state.qcore:16-19`, `chunkio/memory_close.qcore:17`, `chunkio/filesystem_chunk_close.qcore:15` | `requires lock_is_boolean;` and the busy/lock frame clauses | Boolean-field routes only | **retained and already correctly scoped** — these files state in-line that the flag is *not* a write-exclusion or synchronization contract. The checked profile must not be read as promoting them |
| D5 | `contract/qcore/monkey/callback_binding.qcore:6-7` | blanket "exclusive access … are trusted inputs" | Monkey adapter | **retained** |

### 2.1 Ledger summary

- **subsumed by the profile: 8 model sites** (A1, A2, A3, A4, A5, A6, A18) plus
  **3 Lean constructs** (B1, B3, B4) — all inside `concurrency.rs`,
  `obligation.rs`'s reserved vocabulary and `Concurrency.lean`. All of them
  remain in force, unchanged, for `TrustedExplicitTrace`.
- **retained after T02/T03: 15 sites** — the whole Libevent/reactor abstract
  profile (A8–A11, A15), library custody evidence (A13, A14), lock ranks (A7),
  the free-form assumption map (A19), the weak-memory candidate-completeness
  boundary (A16), the Lean destruction-ordering bool (B2), and all five contract
  families (D1–D5). Two sites (A12, A17) are already correct fail-closed
  behaviour and are retained as-is.

### 2.2 Two observations recorded, not repaired

Neither is in T01's scope and neither is fixed here (this card writes no tracked
file). Both should be triaged separately.

1. **`crates/qcore-core/src/event_reactor.rs:1022`** binds the computed
   lock-held state to `_`:

   ```rust
   let _lock_held = state.thread_safe && !state.unlock_callbacks;
   ```

   The value is computed and discarded, so the non-deferred bufferevent callback
   path makes no use of lock state. This may be a deliberate reservation, but it
   is not documented as one at that line. It is *not* a Safe-admission bypass on
   its face — the surrounding path is the abstract reactor profile, which
   already declares its access authority trusted (A8) — but it should be either
   used or explicitly commented as reserved.
2. **`crates/qcore-core/src/library_custody.rs:438-449`** records checked
   evidence ids in an insert-only `BTreeSet`, so a single evidence id can
   authorize an unbounded number of custody transitions. Under this design's
   §1.5 discipline that would be a consumable witness. Changing it is a
   behaviour change to a separate model and is out of scope here.

---

## 3. Compatibility

**The design adds a profile. It deletes nothing.**

| Concern | Disposition |
|---|---|
| `check_concurrent` behaviour | **byte-identical.** The recommended T02 cut (§4.1) is a *new sibling module* with its own entry point, so `check_concurrent`'s body is not edited at all |
| The 26 tests in `crates/qcore-core/tests/unit/concurrency.rs` | unchanged and still passing, including `same_mutex_establishes_happens_before`, `different_mutexes_do_not_hide_a_race` and `certificates_name_trusted_model_authority_without_removing_guard_semantics`. R03 explicitly warns: "Do not 'repair' the model by deleting mutex authorization without replacing its intended invariant semantics" |
| `ConcurrencyProfile` enum | a second variant is safe: no exhaustive `match` on it exists outside `concurrency.rs:547` and one test assertion at `tests/unit/concurrency.rs:913`. Adding `#[non_exhaustive]` in the same increment is recommended but optional |
| `ConcurrencyAssumption` | new variants are additive; `GuardFootprintAuthority` and `SynchronizedCustodyEdges` are **kept**, because `TrustedExplicitTrace` still issues them |
| `qcore-schemas` (Rust API 2.0.0) | `MutexGuardReleased`, `JoinBeforeExit`, `MutexIdentity`, `TaskIdentity` already exist and already serialize (`obligation.rs:99-101, 173-179`). Giving them a derivation and a witness kind is additive within 2.x **provided no wire shape changes**. If a new witness variant must be added to a `#[serde(tag = …)]` enum, `docs/reports/schema-1-compatibility/README.md`'s breaking-change policy applies |
| `research_closure.rs:43` (`concurrency: ConcurrencyCertificate`) | the aggregate closure must record *which* profile its embedded certificate came from and must not treat a `TrustedExplicitTrace` certificate as a checked one |
| Cycle 19 artifacts, CT mapping, contract corpus | untouched by construction. Model-only work; 185 units, 11/50 CT and 59 contract modules must all be byte-identical after T02 |
| Removing the abstract profile | **out of scope and forbidden here.** Invariant 3 (parity before deletion) applies: `TrustedExplicitTrace` can only be retired after `CheckedMutexInvariantV1` reproduces its pinned results and the diff is archived. This design does not propose that |

---

## 4. Implementation cut

### 4.1 T02 — model-level checker (`qcore-core`)

**Recommended shape: a new module, not an extension of `check_concurrent`.**

New file `crates/qcore-core/src/mutex_invariant.rs`, entry point
`check_mutex_invariant_program(&MutexInvariantProgram) -> Result<MutexInvariantCertificate, Vec<MutexInvariantDiagnostic>>`,
reusing `TaskId`, `SharedResourceId`, `AccessFootprint`, `AccessKind` and
`FULL_ACCESS` from `concurrency`. Rejected alternative: adding an optional
`mutex_declarations` field to `ConcurrentProgram` and branching inside
`check_concurrent`. That would edit the body of a function with 26 pinned tests
and a Lean counterpart, for no gain — the two profiles do not share a state
representation (static permission table vs. mutable pointwise share ledger).
A later increment may make `check_concurrent` delegate, once parity is archived.

New state: mutex table (`MutexId → {protects, invariant, liveness, lock state}`),
guard table (`GuardId → {mutex, holder, invariant state, borrows, facts}`),
pointwise share ledger, mutex-generation `destroyed` set, mutex release clocks
keyed by generation.

New events: `CreateMutex`, `DestroyMutex`, `Lock` (returns `GuardId`), `Unlock`,
`OpenInvariant`, `CloseInvariant`, `ErrorExit`, `BorrowUnderGuard`,
`EndBorrow`, `AssertFact`, `UseFact`, plus `Access`/`TransferCustody`/`Destroy`
with the §1.4/§1.5 preconditions. Full field-level draft in
[`profile.md`](profile.md).

New diagnostics, and the source-level Q-codes they surface as. `Q600` is the
only Q6xx currently in use (`frontend.rs:2120`, `source_tasks.rs:35`,
`verification/src/driver.rs:1127`), so `Q610`–`Q623` is free:

| Diagnostic | Q-code | Fires when |
|---|---|---|
| `UnprotectedFootprint` | `Q610` | guard's mutex does not protect the accessed footprint (**the R03 repair**) |
| `UnguardedProtectedAccess` | `Q611` | protected footprint touched without a guard |
| `WrongGuardAffinity` | `Q612` | non-holder unlocks, opens, closes, or accesses through a guard; guard crosses a task boundary |
| `InvariantOpenAtUnlock` | `Q613` | `Unlock` while `Open` |
| `InvariantOpenAtErrorExit` | `Q614` | typed-error exit while `Open` |
| `GuardBorrowEscape` | `Q615` | live `g`-derived borrow at `Unlock` |
| `StaleProtectedFact` | `Q616` | fact keyed to a dead guard is used |
| `FootprintAlreadyProtected` | `Q617` | second mutex over a parked footprint |
| `DynamicProtectedShape` | `Q618` | footprint add/replace/widen after creation |
| `NonquiescentMutexDestruction` | `Q619` | holder, waiter, live borrow/fact, or unordered prior access at `DestroyMutex` |
| `ForgedSynchronizationWitness` | `Q620` | named sync edge absent from derived happens-before |
| `MutexAuthorityNotConserved` | `Q621` | pointwise share total ≠ `FULL_ACCESS`; partial parking |
| `UnsupportedSynchronizationConstruct` | `Q622` | condvar, try/timed lock, rwlock, nested-protecting mutex |
| `DeadMutexGeneration` | `Q623` | any event naming a destroyed mutex; repeat `DestroyMutex` |
| `DeadGuard` | `Q612` (affinity family) | guard from an already-unlocked section |

New assumptions on `CheckedMutexInvariantV1` certificates — the honest residue:

- `DeclaredFootprintShape` — the declared intervals correspond to real C layout.
  Retained: `docs/QCORE_SPEC_r0_3.md:187` excludes C layout derivation.
- `InvariantPredicateDeclared` — the `CloseInvariant` witness is a *declared*
  predicate, not a proved one. Retained until source contracts are enforced
  (today `Q021` refuses all contract clauses).
- `LockRankDeclared` — only when rank mode is used.
- `InitialAuthorityAndUniqueLifetimeIds` — carried over.

**Pinned negative test, the discharge claim:** a certificate whose profile is
`CheckedMutexInvariantV1` must **never** contain `GuardFootprintAuthority` or
`SynchronizedCustodyEdges`, and the free-form
`assumptions: BTreeMap<String, bool>` route (A19) must not be able to forge one.
This is the exact counterpart of
`certificates_name_trusted_model_authority_without_removing_guard_semantics`
and is the single test that establishes R03 moved.

Acceptance shape, transposed from V01/V02: a full *event kind × mutex liveness
state* matrix and a full *event kind × guard state* matrix, each asserting the
**exact** diagnostic vector rather than `is_err()`, each with an accepted
positive control, and each asserting the *absence* of the neighbouring
diagnostic (so that a rejection is proved to come from the intended rule).

### 4.2 Lean obligations

The current `Lifetime` model cannot state R03's obligation at all: `guardHolders
: List TaskId` (`Concurrency.lean:24`) has no mutex identity, and `.lock task`
(`:92-94`) is unconditional, so "an unrelated lock" is unrepresentable. T02
therefore needs a **new namespace**, `QCore.Concurrency.MutexInvariant`, not an
edit to `Lifetime` (which stays byte-identical, with all 23 V01/V02 theorems and
its `AuditCycle14.lean` registrations intact).

Required theorems:

| Theorem | Obligation |
|---|---|
| `parked_authority_is_conserved` | pointwise share total is invariant under every step (spec L16:341) |
| `guard_authorizes_only_declared_footprint` | §1.4 row 1 |
| `unrelated_guard_grants_no_authority` | the R03 acceptance sentence |
| `unguarded_access_to_protected_footprint_is_rejected` | §1.4 row 2 |
| `only_holder_can_unlock` / `guard_is_not_transferable` | affinity, CT28 |
| `unlock_requires_closed_invariant` | CT29 (normal exit) |
| `error_exit_requires_closed_invariant` | CT29 (error exit) |
| `guard_borrows_die_at_unlock` | CT30 |
| `facts_do_not_survive_unlock` | CT48 |
| `footprint_has_at_most_one_protector` | CT27 |
| `protected_shape_is_fixed` | CT34 |
| `mutex_destruction_requires_quiescence` | CT31 |
| `destroyed_mutex_generation_is_never_relocked` | V01 transposed |
| `refused_mutex_destruction_leaves_generation_live` | V01's complement rule |
| `synchronization_witness_must_be_derivable` | §1.5, forged handoff |
| `same_mutex_generation_orders_release_acquire` | preserves existing intent |
| `distinct_mutexes_do_not_order` | preserves `different_mutexes_do_not_hide_a_race` |
| `checked_certificate_omits_trusted_guard_assumptions` | the discharge claim, mirroring the Rust pinned test |

Plus one decidable example per CT scenario in §4.3, registered in a new
`proof/AuditCycleN.lean` entry so `scripts/check_lean_audits.py` covers it, with
the axiom closure still limited to `propext`, `Classical.choice`, `Quot.sound`.

Explicit premises that must appear as Lean hypotheses, not be proved: source
extraction, C layout of the declared intervals, and the invariant predicate
itself.

### 4.3 T03 — source surface (`qcore-frontend`)

The surface is a lexical guard over the profile. `with lock <place> as <id>
<block>` **already parses** (`crates/qcore-frontend/src/frontend.rs:1142-1156`),
binds its guard name body-locally (`:1605-1609`, repaired by
`w6-frontend-gates`), routes to `SemanticPass::Concurrency` → `Q600`
(`:2097, 2120`), and `mutex<T>` is already a recognized type category
(`:129, 1019`). What is missing is a **creation form** — T00 §5.2's finding.

Proposed grammar (new `G02c. Guarded fixed-footprint mutex` in
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`, profile id `source.tasks.mutex.v1`):

```
thread scope s {
    mutex m guards buf;                 // creation: footprint FIXED here, syntactically
    with lock m as g {
        open g;                         // invariant may now be broken
        write g.buf = 1;
        close g;                        // invariant re-established
    }                                   // implicit unlock: requires closed, no escapes
}
```

`mutex m guards <place-list>;` makes the fixed footprint syntactically evident
at the declaration, which is what makes `DynamicProtectedShape` a *parse-time*
guarantee rather than a dataflow one. `open`/`close` are statements, not
expressions, so the invariant state is a block-structured property. Scope exit
destroys the mutex and runs §1.7's quiescence check.

CT scenarios this would make **expressible** (expressible ≠ mapped; per T00,
mapping requires executed stage evidence for each stage claimed):

| Case | Scenario | Expected | Mechanism |
|---|---|---|---|
| CT26 | two critical sections, same mutex, same payload | Accept | two guards on one live mutex generation |
| CT27 | two mutexes allegedly guard one payload | Reject | `Q617` at the second `guards` |
| CT28 | guard moved to a child / unlocked by another thread | Reject | `Q612` |
| CT29 | typed error leaves a broken invariant | Reject unless restored | `Q614` |
| CT30 | guard-derived borrow escapes | Reject | `Q615` |
| CT31 | mutex destroyed with outstanding holder/waiter | Reject | `Q619` |
| CT32 | join while a guard is held | Reject | already checkable (`JoinWhileLocked`), now with real guard identity |
| CT33 | lock-order cycle in rank mode | Reject rank proof | `Q618`-adjacent `LockRankViolation`; a *passing* rank still proves nothing |
| CT34 | protected owned graph grows / replaces identities | Unsupported | `Q618` |
| CT48 | stale heap refinement after release/reacquire | Invalidate, never assume | `Q616` |

That is **10 of the 11** cases T00 buckets as "blocked-on-unimplemented-syntax
(no mutex value)". **CT46 stays unmapped**: it needs `Send`/`Sync` derivation for
a payload type, which L09 places outside the API.

Sequencing note: CT29's fixture needs typed-error payload support, which T00
lists as CT17/CT18's separate blocker. If payload-free errors are the only ones
available at T03 time, CT29's fixture must use a payload-free error and say so
in its `semantic_profile`, or wait.

### 4.4 What stays in `TrustedExplicitTrace`, unchanged

- `check_concurrent`'s signature, body and all 26 unit tests, including the
  guard-authority and synchronized-transfer semantics R03 forbids deleting.
- `ConcurrencyAssumption::{GuardFootprintAuthority, SynchronizedCustodyEdges}`
  and the `ConcurrencyProfile::TrustedExplicitTrace` stamp at `:547`.
- The `LockRank` ordering check and the conservative destruction guard sweep at
  `:509-518`.
- `proof/QCore/Concurrency.lean`'s `Lifetime` namespace and every V01/V02
  theorem registered in `AuditCycle14.lean`.
- The whole Libevent / `event_reactor` abstract profile (A8–A11, A15), the
  weak-memory candidate API (A16), library custody evidence (A13, A14), and all
  five contract families (D1–D5).

---

## 5. What this card does not establish

- It is a design. No code exists, no test was written, no theorem was proved,
  and no measurement was taken beyond reading the cited sources.
- Every Q-code, event name, type name and theorem name above is a **proposal**;
  the integrator owns the final vocabulary and the normative-spec edits.
- Even fully implemented, `CheckedMutexInvariantV1` would discharge
  `GuardFootprintAuthority` and `SynchronizedCustodyEdges` **only inside its own
  profile, over supplied traces or the bounded G02c source surface**. It would
  establish no source-to-trace refinement for arbitrary C, no deadlock freedom,
  no completeness of the candidate execution set, and no promotion of any raw
  Cycle 19 unit.
- `DeclaredFootprintShape` and `InvariantPredicateDeclared` are *new* premises
  the profile would introduce. The trust-premise ledger's honest bottom line is
  that this design converts two unconstrained booleans into two narrower,
  explicitly named declarations plus a large body of checked structure — it does
  not reduce the premise count to zero.
