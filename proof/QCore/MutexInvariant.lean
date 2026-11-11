-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

namespace QCore.Concurrency.MutexInvariant

/-! Executable model of the `CheckedMutexInvariantV1` profile
(`crates/qcore-core/src/mutex_invariant.rs`).

This is a **new** namespace, not an edit of `QCore.Concurrency.Lifetime`. That
model carries `guardHolders : List TaskId` with no mutex identity and an
unconditional `.lock`, so it cannot even state "an unrelated lock grants no
authority". Everything here is additive: `Lifetime`, its theorems and its
`AuditCycle14` registrations are untouched and still describe the
`TrustedExplicitTrace` profile.

Premises, carried as model structure rather than proved: source-to-trace
refinement, the real C layout of the declared intervals
(`DeclaredFootprintShape`), and the invariant predicate itself
(`InvariantPredicateDeclared`). Share conservation is proved here over the
abstract authority total; the Rust checker additionally verifies it pointwise
per atom, and that refinement is a premise, not a theorem.

Each event is written as an explicit `can…` guard plus a total `…Effect`, so
every refusal below is a statement about the guard and every preservation
result is a statement about the effect. -/

abbrev TaskId := Nat
abbrev MutexId := Nat
abbrev GuardId := Nat
abbrev ResourceId := Nat
abbrev BorrowId := Nat
abbrev FactId := Nat
abbrev InvariantId := Nat

def fullAccess : Nat := 1024

/-- A half-open interval of one resource. -/
structure Footprint where
  resource : ResourceId
  start : Nat
  stop : Nat
  deriving DecidableEq, Repr

def Footprint.valid (f : Footprint) : Bool := f.start < f.stop

def Footprint.overlaps (f g : Footprint) : Bool :=
  f.resource == g.resource && f.start < g.stop && g.start < f.stop

def Footprint.contains (f g : Footprint) : Bool :=
  f.resource == g.resource && f.start ≤ g.start && g.stop ≤ f.stop

inductive AccessKind where
  | read
  | write
  deriving DecidableEq, Repr

inductive Phase where
  | closed
  | opened
  deriving DecidableEq, Repr

/-- Authority is held either by a task or parked in a mutex. Every transition
below moves a holding between custodians; none creates or destroys share. -/
inductive Custodian where
  | task (t : TaskId)
  | mutex (m : MutexId)
  deriving DecidableEq, Repr

structure Holding where
  custodian : Custodian
  footprint : Footprint
  share : Nat
  deriving DecidableEq, Repr

structure MutexRecord where
  id : MutexId
  protects : List Footprint
  invariant : InvariantId
  /-- Every declared invariant this generation has carried, most recent first.
  A protection growth must move the identity to one that is not in here: a
  wider protected region is a different proposition (card `CT34-GROWTH`, §16). -/
  invariants : List InvariantId
  creator : TaskId
  holder : Option GuardId
  waiters : List TaskId
  /-- Tasks that have released this generation. -/
  releasedBy : List TaskId
  /-- `(acquirer, releaser)` pairs: an acquisition that observed a release. -/
  observed : List (TaskId × TaskId)
  deriving DecidableEq, Repr

structure GuardRecord where
  id : GuardId
  mutex : MutexId
  holder : TaskId
  phase : Phase
  borrows : List BorrowId
  facts : List FactId
  wroteWhileOpen : Bool
  deriving DecidableEq, Repr

structure State where
  activeTasks : List TaskId := [0]
  terminalTasks : List TaskId := []
  parents : List (TaskId × TaskId) := [] -- child, parent
  joinedBy : List (TaskId × TaskId) := [] -- child, joiner
  mutexes : List MutexRecord := []
  destroyedMutexes : List MutexId := []
  mintedMutexes : List MutexId := []
  guards : List GuardRecord := []
  mintedGuards : List GuardId := []
  holdings : List Holding := []
  custody : List (ResourceId × TaskId) := []
  destroyedResources : List ResourceId := []
  deriving DecidableEq, Repr

def State.act (s : State) (t : TaskId) : Bool :=
  s.activeTasks.contains t && !s.terminalTasks.contains t

def State.mutexOf (s : State) (m : MutexId) : Option MutexRecord :=
  s.mutexes.find? (fun record => record.id == m)

def State.guardOf (s : State) (g : GuardId) : Option GuardRecord :=
  s.guards.find? (fun record => record.id == g)

def State.mutexSat (s : State) (m : MutexId) (p : MutexRecord → Bool) : Bool :=
  (s.mutexOf m).elim false p

def State.guardSat (s : State) (g : GuardId) (p : GuardRecord → Bool) : Bool :=
  (s.guardOf g).elim false p

/-- The conserved quantity: total authority present, wherever it is held. -/
def State.authorityTotal (s : State) : Nat :=
  (s.holdings.map (fun h => h.share)).sum

/-- The mutex generation, if any, whose declared shape overlaps `f`. -/
def State.protectorOf (s : State) (f : Footprint) : Option MutexId :=
  (s.mutexes.find? (fun record =>
    record.protects.any (fun declared => declared.overlaps f))).map (fun r => r.id)

def State.parkedOver (s : State) (f : Footprint) : Bool :=
  s.holdings.any (fun h =>
    match h.custodian with
    | .task _ => false
    | .mutex _ => h.footprint.overlaps f)

/-- `t` holds exactly `fullAccess` over exactly `f`: parking is all-or-nothing. -/
def State.holdsFull (s : State) (t : TaskId) (f : Footprint) : Bool :=
  s.holdings.any (fun h =>
    h.custodian == .task t && h.footprint == f && h.share == fullAccess)

def State.taskShare (s : State) (t : TaskId) (f : Footprint) : Nat :=
  ((s.holdings.filter (fun h => h.custodian == .task t && h.footprint.contains f)).map
    (fun h => h.share)).sum

/-! ## Synchronization witnesses

A witness names an edge; `derivable` recomputes that edge from the checker's own
history. No caller-supplied flag can create one. -/

inductive Witness where
  | spawnEdge (child : TaskId)
  | joinEdge (child : TaskId)
  | mutexHandoff (mutex : MutexId)
  deriving DecidableEq, Repr

def State.derivable (s : State) (sender receiver : TaskId) (resource : ResourceId) :
    Witness → Bool
  | .spawnEdge child =>
      child == receiver && s.parents.contains (receiver, sender) &&
        !s.terminalTasks.contains receiver
  | .joinEdge child =>
      child == sender && s.joinedBy.contains (sender, receiver)
  | .mutexHandoff m =>
      s.mutexSat m (fun record =>
        -- an unrelated mutex does not publish an unrelated resource
        record.protects.any (fun f => f.resource == resource) &&
          record.observed.contains (receiver, sender))

inductive Event where
  | spawn (parent child : TaskId)
  | join (parent child : TaskId)
  | createMutex (task : TaskId) (mutex : MutexId) (protects : List Footprint)
      (invariant : InvariantId)
  | destroyMutex (task : TaskId) (mutex : MutexId)
  | registerWaiter (task : TaskId) (mutex : MutexId)
  | lock (task : TaskId) (mutex : MutexId) (guard : GuardId)
  | unlock (task : TaskId) (guard : GuardId)
  | errorExit (task : TaskId) (guard : GuardId)
  | openInvariant (task : TaskId) (guard : GuardId)
  | closeInvariant (task : TaskId) (guard : GuardId) (invariant : InvariantId)
  /-- The *checked* invariant witness (`close g unchanged;`): the invariant is
  restored because nothing was written while it was open. It consults no
  declared predicate, so a section closing this way carries no
  `invariantPredicateDeclared` premise. -/
  | closeUnwritten (task : TaskId) (guard : GuardId)
  | access (task : TaskId) (footprint : Footprint) (kind : AccessKind)
      (guard : Option GuardId)
  | borrowUnderGuard (task : TaskId) (guard : GuardId) (footprint : Footprint)
      (borrow : BorrowId)
  | endBorrow (task : TaskId) (borrow : BorrowId)
  | assertFact (task : TaskId) (guard : GuardId) (footprint : Footprint) (fact : FactId)
  | useFact (task : TaskId) (fact : FactId)
  | transferCustody (sender receiver : TaskId) (resource : ResourceId) (via : Witness)
  /-- Resource destruction, the other half of scope exit. Parked authority is
  outstanding authority: a live generation over the resource blocks it. -/
  | destroy (task : TaskId) (resource : ResourceId)
  | extendProtection (task : TaskId) (mutex : MutexId) (footprint : Footprint)
  | unsupportedConstruct (task : TaskId)
  deriving DecidableEq, Repr

/-! ## Guards and effects -/

def canSpawn (s : State) (parent child : TaskId) : Bool :=
  s.act parent && !s.activeTasks.contains child && !s.terminalTasks.contains child

def spawnEffect (s : State) (parent child : TaskId) : State :=
  { s with activeTasks := child :: s.activeTasks, parents := (child, parent) :: s.parents }

def canJoin (s : State) (parent child : TaskId) : Bool :=
  s.act parent && s.act child && s.parents.contains (child, parent) &&
    s.guards.all (fun g => g.holder != parent && g.holder != child)

def joinEffect (s : State) (parent child : TaskId) : State :=
  { s with terminalTasks := child :: s.terminalTasks
           activeTasks := s.activeTasks.erase child
           joinedBy := (child, parent) :: s.joinedBy }

def canCreateMutex (s : State) (task : TaskId) (mutex : MutexId)
    (protects : List Footprint) : Bool :=
  s.act task && !s.mintedMutexes.contains mutex && !protects.isEmpty &&
    protects.all (fun f => f.valid) &&
    -- a footprint has at most one protector
    protects.all (fun f => !s.parkedOver f) &&
    -- authority is moved, not copied
    protects.all (fun f => s.holdsFull task f)

def createMutexEffect (s : State) (task : TaskId) (mutex : MutexId)
    (protects : List Footprint) (invariant : InvariantId) : State :=
  { s with
    mutexes := { id := mutex, protects := protects, invariant := invariant
                 invariants := [invariant]
                 creator := task, holder := none, waiters := []
                 releasedBy := [], observed := [] } :: s.mutexes
    mintedMutexes := mutex :: s.mintedMutexes
    holdings := s.holdings.map (fun h =>
      if h.custodian == .task task && protects.contains h.footprint then
        { h with custodian := .mutex mutex } else h) }

def canDestroyMutex (s : State) (task : TaskId) (mutex : MutexId) : Bool :=
  s.act task && !s.destroyedMutexes.contains mutex &&
    s.mutexSat mutex (fun r =>
      r.holder == none && r.waiters.isEmpty && r.creator == task) &&
    s.guards.all (fun g => g.mutex != mutex)

def destroyMutexEffect (s : State) (task : TaskId) (mutex : MutexId) : State :=
  { s with
    mutexes := s.mutexes.filter (fun r => r.id != mutex)
    destroyedMutexes := mutex :: s.destroyedMutexes
    holdings := s.holdings.map (fun h =>
      if h.custodian == .mutex mutex then { h with custodian := .task task } else h) }

def canRegisterWaiter (s : State) (task : TaskId) (mutex : MutexId) : Bool :=
  s.act task && !s.destroyedMutexes.contains mutex && s.mutexSat mutex (fun _ => true)

def registerWaiterEffect (s : State) (task : TaskId) (mutex : MutexId) : State :=
  { s with mutexes := s.mutexes.map (fun r =>
      if r.id == mutex then { r with waiters := task :: r.waiters } else r) }

def canLock (s : State) (task : TaskId) (mutex : MutexId) (guard : GuardId) : Bool :=
  s.act task && !s.destroyedMutexes.contains mutex &&
    s.mutexSat mutex (fun r => r.holder == none) && !s.mintedGuards.contains guard

def lockEffect (s : State) (task : TaskId) (mutex : MutexId) (guard : GuardId) : State :=
  { s with
    mutexes := s.mutexes.map (fun r =>
      if r.id == mutex then
        { r with holder := some guard
                 waiters := r.waiters.erase task
                 observed := (r.releasedBy.map (fun x => (task, x))) ++ r.observed }
      else r)
    mintedGuards := guard :: s.mintedGuards
    guards := { id := guard, mutex := mutex, holder := task, phase := .closed
                borrows := [], facts := [], wroteWhileOpen := false } :: s.guards }

/-- `Unlock` and `ErrorExit` share their preconditions; only the diagnostic
differs, and diagnostics are not modeled here. -/
def canRelease (s : State) (task : TaskId) (guard : GuardId) : Bool :=
  s.act task &&
    s.guardSat guard (fun g => g.holder == task && g.phase == .closed && g.borrows.isEmpty)

def releaseEffect (s : State) (task : TaskId) (guard : GuardId) : State :=
  { s with
    guards := s.guards.filter (fun r => r.id != guard)
    mutexes := s.mutexes.map (fun r =>
      if r.holder == some guard then
        { r with holder := none, releasedBy := task :: r.releasedBy }
      else r) }

def canOpen (s : State) (task : TaskId) (guard : GuardId) : Bool :=
  s.act task && s.guardSat guard (fun g => g.holder == task && g.phase == .closed)

def openEffect (s : State) (guard : GuardId) : State :=
  { s with guards := s.guards.map (fun g =>
      if g.id == guard then { g with phase := .opened } else g) }

def canClose (s : State) (task : TaskId) (guard : GuardId) (invariant : InvariantId) : Bool :=
  s.act task &&
    s.guardSat guard (fun g =>
      g.holder == task && g.phase == .opened &&
        s.mutexSat g.mutex (fun m => m.invariant == invariant))

/-- The checked witness. Same shape as `canClose` with the declared-predicate
conjunct replaced by a fact the checker owns: no write happened while the
invariant was open. -/
def canCloseUnwritten (s : State) (task : TaskId) (guard : GuardId) : Bool :=
  s.act task &&
    s.guardSat guard (fun g => g.holder == task && g.phase == .opened && !g.wroteWhileOpen)

def closeEffect (s : State) (guard : GuardId) : State :=
  { s with guards := s.guards.map (fun g =>
      if g.id == guard then
        { g with phase := .closed, wroteWhileOpen := false, facts := [] } else g) }

def canAccessUnguarded (s : State) (task : TaskId) (f : Footprint) (kind : AccessKind) : Bool :=
  s.act task && !s.destroyedResources.contains f.resource && f.valid &&
    -- no fallback: a protected footprint is unreachable without a guard
    (s.protectorOf f).isNone &&
    (match kind with
     | .write => s.taskShare task f == fullAccess
     | .read => 0 < s.taskShare task f)

def canAccessGuarded (s : State) (task : TaskId) (f : Footprint) (kind : AccessKind)
    (guard : GuardId) : Bool :=
  s.act task && !s.destroyedResources.contains f.resource && f.valid &&
    s.guardSat guard (fun g =>
      g.holder == task &&
        -- the repair: the guard's own mutex must be this footprint's protector
        decide (s.protectorOf f = some g.mutex) &&
        s.mutexSat g.mutex (fun m => m.protects.any (fun p => p.contains f)) &&
        (kind == .read || g.phase == .opened))

def accessEffect (s : State) (kind : AccessKind) (guard : GuardId) : State :=
  { s with guards := s.guards.map (fun g =>
      if g.id == guard then
        { g with wroteWhileOpen := g.wroteWhileOpen || kind == .write } else g) }

def canBorrow (s : State) (task : TaskId) (guard : GuardId) (f : Footprint) : Bool :=
  s.act task &&
    s.guardSat guard (fun g =>
      g.holder == task && s.mutexSat g.mutex (fun m => m.protects.any (fun p => p.contains f)))

def borrowEffect (s : State) (guard : GuardId) (borrow : BorrowId) : State :=
  { s with guards := s.guards.map (fun g =>
      if g.id == guard then { g with borrows := borrow :: g.borrows } else g) }

def endBorrowEffect (s : State) (borrow : BorrowId) : State :=
  { s with guards := s.guards.map (fun g => { g with borrows := g.borrows.erase borrow }) }

def assertFactEffect (s : State) (guard : GuardId) (fact : FactId) : State :=
  { s with guards := s.guards.map (fun g =>
      if g.id == guard then { g with facts := fact :: g.facts } else g) }

/-- A fact is usable only through a live guard the acting task holds. -/
def canUseFact (s : State) (task : TaskId) (fact : FactId) : Bool :=
  s.act task && s.guards.any (fun g => g.holder == task && g.facts.contains fact)

def canTransfer (s : State) (sender receiver : TaskId) (resource : ResourceId)
    (via : Witness) : Bool :=
  s.act receiver && !s.destroyedResources.contains resource &&
    s.custody.contains (resource, sender) && s.derivable sender receiver resource via

def transferEffect (s : State) (receiver : TaskId) (resource : ResourceId) : State :=
  { s with custody := (resource, receiver) ::
      s.custody.filter (fun entry => entry.1 != resource) }

/-- A holding that keeps `resource` alive against the destroying task: a parked
mutex share, or any live share of another task. -/
def outstandingOver (task : TaskId) (resource : ResourceId) (h : Holding) : Bool :=
  h.footprint.resource == resource &&
    (match h.custodian with
     | .task t => t != task && 0 < h.share
     | .mutex _ => true)

/-- Destruction requires custody, liveness and quiescence. Parked authority is
outstanding authority, so a live generation over any part of the resource
blocks it; so does a share still held by another task. -/
def canDestroy (s : State) (task : TaskId) (resource : ResourceId) : Bool :=
  s.act task && !s.destroyedResources.contains resource &&
    s.custody.contains (resource, task) &&
    !s.mutexes.any (fun m => m.protects.any (fun f => f.resource == resource)) &&
    !s.holdings.any (outstandingOver task resource)

/-- The destroyed resource leaves custody and liveness. Its holding is left in
place on purpose: the conserved total stays exactly invariant, and a dead
resource is unreachable anyway, because every access guard tests
`destroyedResources` first. The Rust ledger instead retires those atoms from
its conservation check; the two agree on what is reachable. -/
def destroyEffect (s : State) (resource : ResourceId) : State :=
  { s with destroyedResources := resource :: s.destroyedResources
           custody := s.custody.filter (fun entry => entry.1 != resource) }

def step (s : State) : Event → Option State
  | .spawn parent child =>
      if canSpawn s parent child then some (spawnEffect s parent child) else none
  | .join parent child =>
      if canJoin s parent child then some (joinEffect s parent child) else none
  | .createMutex task mutex protects invariant =>
      if canCreateMutex s task mutex protects then
        some (createMutexEffect s task mutex protects invariant) else none
  | .destroyMutex task mutex =>
      if canDestroyMutex s task mutex then some (destroyMutexEffect s task mutex) else none
  | .registerWaiter task mutex =>
      if canRegisterWaiter s task mutex then some (registerWaiterEffect s task mutex) else none
  | .lock task mutex guard =>
      if canLock s task mutex guard then some (lockEffect s task mutex guard) else none
  | .unlock task guard =>
      if canRelease s task guard then some (releaseEffect s task guard) else none
  | .errorExit task guard =>
      if canRelease s task guard then some (releaseEffect s task guard) else none
  | .openInvariant task guard =>
      if canOpen s task guard then some (openEffect s guard) else none
  | .closeInvariant task guard invariant =>
      if canClose s task guard invariant then some (closeEffect s guard) else none
  | .closeUnwritten task guard =>
      if canCloseUnwritten s task guard then some (closeEffect s guard) else none
  | .access task footprint kind none =>
      if canAccessUnguarded s task footprint kind then some s else none
  | .access task footprint kind (some guard) =>
      if canAccessGuarded s task footprint kind guard then
        some (accessEffect s kind guard) else none
  | .borrowUnderGuard task guard footprint borrow =>
      if canBorrow s task guard footprint then some (borrowEffect s guard borrow) else none
  | .endBorrow task borrow =>
      if s.act task then some (endBorrowEffect s borrow) else none
  | .assertFact task guard footprint fact =>
      if canBorrow s task guard footprint then some (assertFactEffect s guard fact) else none
  | .useFact task fact =>
      if canUseFact s task fact then some s else none
  | .transferCustody sender receiver resource via =>
      if canTransfer s sender receiver resource via then
        some (transferEffect s receiver resource) else none
  | .destroy task resource =>
      if canDestroy s task resource then some (destroyEffect s resource) else none
  -- The protected shape is fixed at creation: a refusal, not an omission.
  | .extendProtection _ _ _ => none
  -- Condition variables, try/timed locks, rwlocks and nested protecting
  -- mutexes are refused rather than approximated.
  | .unsupportedConstruct _ => none

def run (s : State) : List Event → Option State
  | [] => some s
  | event :: rest => (step s event).bind (fun next => run next rest)

/-! ## 1. Share conservation -/

private theorem map_preserves_share_sum (l : List Holding) (f : Holding → Holding)
    (keeps : ∀ h, (f h).share = h.share) :
    ((l.map f).map (fun h => h.share)).sum = (l.map (fun h => h.share)).sum := by
  induction l with
  | nil => rfl
  | cons h t ih => simp only [List.map_cons, List.sum_cons, keeps h, ih]

theorem createMutexEffect_preserves_authority (s : State) (task : TaskId) (mutex : MutexId)
    (protects : List Footprint) (invariant : InvariantId) :
    (createMutexEffect s task mutex protects invariant).authorityTotal = s.authorityTotal := by
  unfold createMutexEffect State.authorityTotal
  exact map_preserves_share_sum _ _ (by intro h; split <;> rfl)

theorem destroyMutexEffect_preserves_authority (s : State) (task : TaskId) (mutex : MutexId) :
    (destroyMutexEffect s task mutex).authorityTotal = s.authorityTotal := by
  unfold destroyMutexEffect State.authorityTotal
  exact map_preserves_share_sum _ _ (by intro h; split <;> rfl)

/-- Parked authority is conserved: no step creates or destroys share. This is
the model side of `QCORE_SPEC_r0_3.md:341` — pointwise shares across all
custodians and parked mutex invariants must be conserved. -/
theorem parked_authority_is_conserved {s next : State} {event : Event}
    (accepted : step s event = some next) : next.authorityTotal = s.authorityTotal := by
  cases event <;> simp only [step] at accepted
  all_goals repeat' first | contradiction | split at accepted
  all_goals
    (cases Option.some.inj accepted
     first
     | rfl
     | exact createMutexEffect_preserves_authority ..
     | exact destroyMutexEffect_preserves_authority ..)

/-! ## 2. Guard authority is exactly the declared footprint -/

/-- An accepted guarded access is covered by its own mutex's declared shape. -/
theorem guard_authorizes_only_declared_footprint {s next : State} {task : TaskId}
    {f : Footprint} {kind : AccessKind} {guard : GuardId} {record : GuardRecord}
    (held : s.guardOf guard = some record)
    (accepted : step s (.access task f kind (some guard)) = some next) :
    s.protectorOf f = some record.mutex ∧
      s.mutexSat record.mutex (fun m => m.protects.any (fun p => p.contains f)) = true := by
  simp only [step] at accepted
  split at accepted
  · rename_i ok
    simp only [canAccessGuarded, State.guardSat, held, Option.elim, Bool.and_eq_true,
      decide_eq_true_eq] at ok
    exact ⟨ok.2.1.1.2, ok.2.1.2⟩
  · contradiction

/-- The R03 acceptance sentence: a guard whose mutex is not this footprint's
protector grants no authority at all. -/
theorem unrelated_guard_grants_no_authority {s : State} {task : TaskId} {f : Footprint}
    {kind : AccessKind} {guard : GuardId} {record : GuardRecord}
    (held : s.guardOf guard = some record)
    (unrelated : s.protectorOf f ≠ some record.mutex) :
    step s (.access task f kind (some guard)) = none := by
  simp [step, canAccessGuarded, State.guardSat, held, unrelated]

/-- A protected footprint has no unguarded path: the static share ledger is not
consulted, because creation consumed the share. -/
theorem unguarded_access_to_protected_footprint_is_rejected {s : State} {task : TaskId}
    {f : Footprint} {kind : AccessKind} {m : MutexId}
    (protectedBy : s.protectorOf f = some m) :
    step s (.access task f kind none) = none := by
  simp [step, canAccessUnguarded, protectedBy]

/-- A write is what breaks the invariant, so it must be declared broken first. -/
theorem write_requires_open_invariant {s : State} {task : TaskId} {f : Footprint}
    {guard : GuardId} {record : GuardRecord} (held : s.guardOf guard = some record)
    (closed : record.phase = .closed) :
    step s (.access task f .write (some guard)) = none := by
  simp [step, canAccessGuarded, State.guardSat, held, closed]

/-! ## 3. Guard affinity -/

theorem only_holder_can_unlock {s : State} {task : TaskId} {guard : GuardId}
    {record : GuardRecord} (held : s.guardOf guard = some record)
    (foreign : record.holder ≠ task) : step s (.unlock task guard) = none := by
  simp [step, canRelease, State.guardSat, held, foreign]

/-- A guard cannot be used from another task through *any* guard-keyed
operation, so it is not transferable across a task boundary (CT28). -/
theorem guard_is_not_transferable {s : State} {task : TaskId} {guard : GuardId}
    {record : GuardRecord} {f : Footprint} {kind : AccessKind}
    {borrow : BorrowId} {fact : FactId} {invariant : InvariantId}
    (held : s.guardOf guard = some record) (foreign : record.holder ≠ task) :
    step s (.unlock task guard) = none ∧
      step s (.errorExit task guard) = none ∧
      step s (.openInvariant task guard) = none ∧
      step s (.closeInvariant task guard invariant) = none ∧
      step s (.closeUnwritten task guard) = none ∧
      step s (.access task f kind (some guard)) = none ∧
      step s (.borrowUnderGuard task guard f borrow) = none ∧
      step s (.assertFact task guard f fact) = none := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;>
    simp [step, canRelease, canOpen, canClose, canCloseUnwritten, canAccessGuarded, canBorrow,
      State.guardSat, held, foreign]

/-! ## 4. The invariant holds at every exit -/

theorem unlock_requires_closed_invariant {s : State} {task : TaskId} {guard : GuardId}
    {record : GuardRecord} (held : s.guardOf guard = some record)
    (broken : record.phase = .opened) : step s (.unlock task guard) = none := by
  simp [step, canRelease, State.guardSat, held, broken]

theorem error_exit_requires_closed_invariant {s : State} {task : TaskId} {guard : GuardId}
    {record : GuardRecord} (held : s.guardOf guard = some record)
    (broken : record.phase = .opened) : step s (.errorExit task guard) = none := by
  simp [step, canRelease, State.guardSat, held, broken]

/-! ## 5. Guard-derived borrows and guard-scoped facts -/

/-- CT30: a borrow taken through a guard cannot outlive it. -/
theorem guard_borrows_die_at_unlock {s : State} {task : TaskId} {guard : GuardId}
    {record : GuardRecord} {borrow : BorrowId} (held : s.guardOf guard = some record)
    (escaping : borrow ∈ record.borrows) : step s (.unlock task guard) = none := by
  have nonempty : record.borrows.isEmpty = false := by
    cases hb : record.borrows with
    | nil => rw [hb] at escaping; exact absurd escaping List.not_mem_nil
    | cons _ _ => rfl
  simp [step, canRelease, State.guardSat, held, nonempty]

/-- Releasing removes the guard, so nothing keyed to it survives. -/
theorem release_removes_the_guard (s : State) (task : TaskId) (guard : GuardId) :
    (releaseEffect s task guard).guardOf guard = none := by
  unfold releaseEffect State.guardOf
  simp only [List.find?_filter]
  induction s.guards with
  | nil => simp
  | cons g rest ih => by_cases hit : g.id = guard <;> simp_all

/-- A fact is usable only while some live guard of the acting task carries it. -/
theorem use_fact_requires_a_live_guard {s : State} {task : TaskId} {fact : FactId}
    (none_carries : ∀ g ∈ s.guards, fact ∉ g.facts) :
    step s (.useFact task fact) = none := by
  have inner : ¬ ((s.guards.any fun g => g.holder == task && g.facts.contains fact) = true) := by
    rw [List.any_eq_true]
    rintro ⟨g, mem, h⟩
    rw [Bool.and_eq_true] at h
    exact none_carries g mem (by simpa using h.2)
  have refused : canUseFact s task fact = false := by
    unfold canUseFact
    cases hb : (s.guards.any fun g => g.holder == task && g.facts.contains fact)
    · simp
    · exact absurd hb inner
  simp [step, refused]

/-- CT48: a fact established under one guard generation does not survive that
guard's release, so it must be re-established after reacquisition. -/
theorem facts_do_not_survive_unlock {s next : State} {task user : TaskId} {guard : GuardId}
    {fact : FactId}
    (only_this_guard : ∀ g ∈ s.guards, fact ∈ g.facts → g.id = guard)
    (accepted : step s (.unlock task guard) = some next) :
    step next (.useFact user fact) = none := by
  simp only [step] at accepted
  split at accepted
  · cases Option.some.inj accepted
    refine use_fact_requires_a_live_guard ?_
    intro g mem carries
    have parts : g ∈ s.guards ∧ ¬ g.id = guard := by
      simpa [releaseEffect, List.mem_filter] using mem
    exact parts.2 (only_this_guard g parts.1 carries)
  · contradiction

/-! ## 6. Fixed shape and a single protector -/

/-- CT27: creation cannot park a footprint another live mutex already parks, so
two mutex identities never guard one payload. -/
theorem footprint_has_at_most_one_protector {s : State} {task : TaskId} {mutex : MutexId}
    {protects : List Footprint} {invariant : InvariantId} {f : Footprint}
    (declared : f ∈ protects) (taken : s.parkedOver f = true) :
    step s (.createMutex task mutex protects invariant) = none := by
  have blocked : ¬ ((protects.all fun g => !s.parkedOver g) = true) := by
    rw [List.all_eq_true]
    intro every
    have := every f declared
    simp [taken] at this
  simp [step, canCreateMutex, blocked]

/-- A proposal to change the protected shape is refused. No state enables this;
it is a refusal, not a case the model happens not to cover. Card `CT34-GROWTH`
leaves it exactly as it was and adds its admitted growth as a separate layer
(§16), so this remains the unconditional rule for `extendProtection`. -/
theorem protected_shape_is_fixed (s : State) (task : TaskId) (mutex : MutexId)
    (f : Footprint) : step s (.extendProtection task mutex f) = none := rfl

theorem unsupported_synchronization_is_refused (s : State) (task : TaskId) :
    step s (.unsupportedConstruct task) = none := rfl

private theorem map_preserves_shapes (l : List MutexRecord) (f : MutexRecord → MutexRecord)
    (keeps : ∀ r, (f r).id = r.id ∧ (f r).protects = r.protects) :
    (l.map f).map (fun r => (r.id, r.protects)) = l.map (fun r => (r.id, r.protects)) := by
  induction l with
  | nil => simp
  | cons r t ih => simp [(keeps r).1, (keeps r).2, ih]

/-- Locking, releasing and waiting rewrite mutex bookkeeping but never a
declared shape: the `(id, protects)` inventory is preserved exactly. -/
theorem critical_section_effects_preserve_declared_shapes (s : State) (task : TaskId)
    (mutex : MutexId) (guard : GuardId) :
    (lockEffect s task mutex guard).mutexes.map (fun r => (r.id, r.protects)) =
        s.mutexes.map (fun r => (r.id, r.protects)) ∧
      (releaseEffect s task guard).mutexes.map (fun r => (r.id, r.protects)) =
        s.mutexes.map (fun r => (r.id, r.protects)) ∧
      (registerWaiterEffect s task mutex).mutexes.map (fun r => (r.id, r.protects)) =
        s.mutexes.map (fun r => (r.id, r.protects)) := by
  refine ⟨?_, ?_, ?_⟩
  · exact map_preserves_shapes _ _ (by intro r; constructor <;> (split <;> rfl))
  · exact map_preserves_shapes _ _ (by intro r; constructor <;> (split <;> rfl))
  · exact map_preserves_shapes _ _ (by intro r; constructor <;> (split <;> rfl))

/-! ## 7. Mutex generations -/

/-- CT31: destruction demands quiescence — no holder, no waiter, no live guard —
and returns the parked share only to the task that parked it. -/
theorem mutex_destruction_requires_quiescence {s next : State} {task : TaskId}
    {mutex : MutexId} (accepted : step s (.destroyMutex task mutex) = some next) :
    s.mutexSat mutex (fun r => r.holder == none && r.waiters.isEmpty && r.creator == task)
        = true ∧
      s.guards.all (fun g => g.mutex != mutex) = true := by
  simp only [step] at accepted
  split at accepted
  · rename_i ok
    unfold canDestroyMutex at ok
    simp only [Bool.and_eq_true] at ok
    exact ⟨ok.1.2, ok.2⟩
  · contradiction

theorem accepted_destruction_kills_the_generation {s next : State} {task : TaskId}
    {mutex : MutexId} (accepted : step s (.destroyMutex task mutex) = some next) :
    mutex ∈ next.destroyedMutexes ∧ next.mutexOf mutex = none := by
  simp only [step] at accepted
  split at accepted
  · cases Option.some.inj accepted
    refine ⟨by simp [destroyMutexEffect], ?_⟩
    unfold destroyMutexEffect State.mutexOf
    simp only [List.find?_filter]
    induction s.mutexes with
    | nil => simp
    | cons r rest ih => by_cases hit : r.id = mutex <;> simp_all
  · contradiction

/-- V01 transposed onto mutex identity: a destroyed generation is dead forever
and is never relocked, whatever numeric id a later input reuses. -/
theorem destroyed_mutex_generation_is_never_relocked {s next : State} {task locker : TaskId}
    {mutex : MutexId} {guard : GuardId}
    (accepted : step s (.destroyMutex task mutex) = some next) :
    step next (.lock locker mutex guard) = none := by
  have dead := accepted_destruction_kills_the_generation accepted
  simp [step, canLock, State.mutexSat, dead.2]

theorem destroyed_mutex_generation_rejects_repeat_destruction {s next : State}
    {task second : TaskId} {mutex : MutexId}
    (accepted : step s (.destroyMutex task mutex) = some next) :
    step next (.destroyMutex second mutex) = none := by
  have dead := accepted_destruction_kills_the_generation accepted
  simp [step, canDestroyMutex, State.mutexSat, dead.2]

theorem busy_mutex_rejects_destruction {s : State} {task : TaskId} {mutex : MutexId}
    {record : MutexRecord} (present : s.mutexOf mutex = some record)
    (busy : record.holder ≠ none) : step s (.destroyMutex task mutex) = none := by
  simp [step, canDestroyMutex, State.mutexSat, present, busy]

theorem waited_mutex_rejects_destruction {s : State} {task : TaskId} {mutex : MutexId}
    {record : MutexRecord} (present : s.mutexOf mutex = some record)
    (waiting : record.waiters ≠ []) : step s (.destroyMutex task mutex) = none := by
  have nonempty : record.waiters.isEmpty = false := by
    cases hw : record.waiters with
    | nil => exact absurd hw waiting
    | cons _ _ => rfl
  simp [step, canDestroyMutex, State.mutexSat, present, nonempty]

/-- V01's complement rule: a refused destruction leaves the generation exactly
as live as before. `DeadMutexGeneration` is unreachable through a refused
destroy — the same generation is still lockable. -/
theorem refused_mutex_destruction_leaves_generation_live {s : State} {task locker : TaskId}
    {mutex : MutexId} {guard : GuardId} {record : MutexRecord}
    (present : s.mutexOf mutex = some record)
    (waiting : record.waiters ≠ [])
    (lockable : canLock s locker mutex guard = true) :
    step s (.destroyMutex task mutex) = none ∧
      (step s (.lock locker mutex guard)).isSome = true ∧
      s.destroyedMutexes.contains mutex = false := by
  refine ⟨waited_mutex_rejects_destruction present waiting, by simp [step, lockable], ?_⟩
  unfold canLock at lockable
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at lockable
  exact lockable.1.1.2

/-! ## 8. Synchronization witnesses -/

/-- The derive-and-compare discipline: an accepted transfer's named edge was
found in the checker's own history. No caller-supplied flag creates an edge. -/
theorem synchronization_witness_must_be_derivable {s next : State} {sender receiver : TaskId}
    {resource : ResourceId} {via : Witness}
    (accepted : step s (.transferCustody sender receiver resource via) = some next) :
    s.derivable sender receiver resource via = true := by
  simp only [step] at accepted
  split at accepted
  · rename_i ok
    unfold canTransfer at ok
    simp only [Bool.and_eq_true] at ok
    exact ok.2
  · contradiction

theorem forged_witness_is_refused {s : State} {sender receiver : TaskId}
    {resource : ResourceId} {via : Witness}
    (absent : s.derivable sender receiver resource via = false) :
    step s (.transferCustody sender receiver resource via) = none := by
  simp [step, canTransfer, absent]

/-- A release of one generation followed by an acquisition of the *same*
generation orders the pair, and the derived handoff names exactly it. -/
theorem same_mutex_generation_orders_release_acquire {s : State} {releaser acquirer : TaskId}
    {mutex : MutexId} {resource : ResourceId} {record : MutexRecord}
    (present : s.mutexOf mutex = some record)
    (observed : (acquirer, releaser) ∈ record.observed)
    (protects : record.protects.any (fun f => f.resource == resource) = true) :
    s.derivable releaser acquirer resource (.mutexHandoff mutex) = true := by
  simp [State.derivable, State.mutexSat, present, protects, observed]

/-- `different_mutexes_do_not_hide_a_race`, preserved: a handoff over a mutex
that does not protect the resource is not derivable, whatever was released. -/
theorem distinct_mutexes_do_not_order {s : State} {sender receiver : TaskId}
    {resource : ResourceId} {mutex : MutexId} {record : MutexRecord}
    (present : s.mutexOf mutex = some record)
    (unrelated : record.protects.any (fun f => f.resource == resource) = false) :
    s.derivable sender receiver resource (.mutexHandoff mutex) = false := by
  simp [State.derivable, State.mutexSat, present, unrelated]

/-! ## 9. Certificates -/

inductive ModelProfile where
  | trustedExplicitTrace
  | checkedMutexInvariantV1
  deriving DecidableEq, Repr

inductive Assumption where
  | initialAuthorityAndUniqueLifetimeIds
  | guardFootprintAuthority
  | synchronizedCustodyEdges
  | declaredFootprintShape
  | invariantPredicateDeclared
  deriving DecidableEq, Repr

structure Certificate where
  profile : ModelProfile
  premises : List Assumption
  destroyedMutexes : List MutexId
  destroyedResources : List ResourceId
  deriving DecidableEq, Repr

/-- Premises this profile admits. `guardFootprintAuthority` and
`synchronizedCustodyEdges` are structurally unreachable here: the checker
derives both, so neither can be emitted. -/
def premises (declaredShape declaredPredicate : Bool) : List Assumption :=
  [.initialAuthorityAndUniqueLifetimeIds] ++
    (if declaredShape then [.declaredFootprintShape] else []) ++
    (if declaredPredicate then [.invariantPredicateDeclared] else [])

def check (initial : State) (events : List Event) (declaredShape declaredPredicate : Bool) :
    Option Certificate :=
  (run initial events).map (fun final =>
    { profile := .checkedMutexInvariantV1
      premises := premises declaredShape declaredPredicate
      destroyedMutexes := final.destroyedMutexes
      destroyedResources := final.destroyedResources })

/-- The discharge claim, mirroring the pinned Rust test: a checked certificate
names this profile and never the two premises `TrustedExplicitTrace` retains. -/
theorem checked_certificate_omits_trusted_guard_assumptions
    {initial : State} {events : List Event} {declaredShape declaredPredicate : Bool}
    {certificate : Certificate}
    (result : check initial events declaredShape declaredPredicate = some certificate) :
    certificate.profile = .checkedMutexInvariantV1 ∧
      Assumption.guardFootprintAuthority ∉ certificate.premises ∧
      Assumption.synchronizedCustodyEdges ∉ certificate.premises := by
  unfold check at result
  cases hrun : run initial events with
  | none => rw [hrun] at result; simp at result
  | some final =>
      rw [hrun] at result
      simp only [Option.map_some] at result
      cases Option.some.inj result
      refine ⟨rfl, ?_, ?_⟩ <;>
        (cases declaredShape <;> cases declaredPredicate <;> simp [premises])

/-! ## 10. Decidable scenarios

Each mirrors a Rust test in `crates/qcore-core/tests/unit/mutex_invariant.rs`. -/

private def buf : Footprint := { resource := 7, start := 0, stop := 8 }
private def lo : Footprint := { resource := 7, start := 0, stop := 4 }
private def hi : Footprint := { resource := 7, start := 4, stop := 8 }

private def owned : State :=
  { holdings := [{ custodian := .task 0, footprint := buf, share := fullAccess }] }

private def halves : State :=
  { holdings := [{ custodian := .task 0, footprint := lo, share := fullAccess },
                 { custodian := .task 0, footprint := hi, share := fullAccess }] }

-- Accepted: lock, open, write, close, unlock, destroy.
example : (run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .openInvariant 0 10,
     .access 0 lo .write (some 10), .closeInvariant 0 10 5, .unlock 0 10,
     .destroyMutex 0 1]).isSome = true := by decide

-- An unrelated lock authorizes nothing in another mutex's footprint.
example : run halves
    [.createMutex 0 1 [lo] 5, .createMutex 0 2 [hi] 6, .lock 0 2 20,
     .access 0 lo .read (some 20)] = none := by decide

-- The same guard over its own footprint is accepted.
example : (run halves
    [.createMutex 0 1 [lo] 5, .createMutex 0 2 [hi] 6, .lock 0 2 20,
     .access 0 hi .read (some 20)]).isSome = true := by decide

-- No fallback: the protected footprint is unreachable without a guard.
example : run owned
    [.createMutex 0 1 [buf] 5, .access 0 lo .read none] = none := by decide

-- Two mutexes cannot protect one payload; disjoint halves are fine.
example : run owned
    [.createMutex 0 1 [buf] 5, .createMutex 0 2 [lo] 6] = none := by decide
example : (run halves
    [.createMutex 0 1 [lo] 5, .createMutex 0 2 [hi] 6]).isSome = true := by decide

-- Wrong-thread unlock.
example : run owned
    [.createMutex 0 1 [buf] 5, .spawn 0 1, .lock 0 1 10, .unlock 1 10] = none := by decide

-- A broken invariant passes neither the normal nor the error exit.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .openInvariant 0 10, .unlock 0 10] = none := by
  decide
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .openInvariant 0 10, .errorExit 0 10] = none := by
  decide

-- Writing with the invariant closed is refused.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .access 0 lo .write (some 10)] = none := by decide

-- A guard-derived borrow cannot escape unlock; ending it admits the same unlock.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .borrowUnderGuard 0 10 lo 30,
     .unlock 0 10] = none := by decide
example : (run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .borrowUnderGuard 0 10 lo 30,
     .endBorrow 0 30, .unlock 0 10]).isSome = true := by decide

-- A fact from guard generation 10 is unusable under generation 11.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .assertFact 0 10 lo 40, .useFact 0 40,
     .unlock 0 10, .lock 0 1 11, .useFact 0 40] = none := by decide

-- Destruction with a holder, and with a waiter.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .destroyMutex 0 1] = none := by decide
example : run owned
    [.createMutex 0 1 [buf] 5, .spawn 0 1, .registerWaiter 1 1,
     .destroyMutex 0 1] = none := by decide

-- A refused destruction leaves the generation live: `run` rejects the trace
-- containing it, and the very same generation is still lockable and, once
-- quiescent, destroyable.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .destroyMutex 0 1] = none := by decide
example : (run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .unlock 0 10, .lock 0 1 11, .unlock 0 11,
     .destroyMutex 0 1]).isSome = true := by decide

-- A dead generation is never relocked and never re-destroyed.
example : run owned
    [.createMutex 0 1 [buf] 5, .destroyMutex 0 1, .lock 0 1 10] = none := by decide
example : run owned
    [.createMutex 0 1 [buf] 5, .destroyMutex 0 1, .destroyMutex 0 1] = none := by decide

-- Guard identities are never reused.
example : run owned
    [.createMutex 0 1 [buf] 5, .lock 0 1 10, .unlock 0 10, .lock 0 1 10] = none := by decide

-- Only the parking task receives the parked share back.
example : run owned
    [.createMutex 0 1 [buf] 5, .spawn 0 1, .destroyMutex 1 1] = none := by decide

-- Partial parking: half a share cannot park a full invariant.
example : run { holdings := [{ custodian := .task 0, footprint := buf, share := 512 }] }
    [.createMutex 0 1 [buf] 5] = none := by decide

-- The shape is fixed, and unsupported constructs are refused.
example : run owned
    [.createMutex 0 1 [lo] 5, .extendProtection 0 1 hi] = none := by decide
example : run owned [.unsupportedConstruct 0] = none := by decide

-- A forged spawn edge is refused; the derived one is admitted.
example : run { custody := [(9, 1)], activeTasks := [0, 1, 2],
                parents := [(1, 0), (2, 0)] }
    [.transferCustody 1 2 9 (.spawnEdge 2)] = none := by decide
example : (run { custody := [(9, 0)] }
    [.spawn 0 1, .transferCustody 0 1 9 (.spawnEdge 1)]).isSome = true := by decide

-- The join edge publishes from the joined child back to its joiner.
example : (run { custody := [(9, 1)], activeTasks := [0, 1], parents := [(1, 0)] }
    [.join 0 1, .transferCustody 1 0 9 (.joinEdge 1)]).isSome = true := by decide

-- A handoff over a mutex that does not protect the resource is not derivable;
-- over the protecting generation it is.
example : run { custody := [(9, 0)],
                holdings := [{ custodian := .task 0, footprint := buf, share := fullAccess }] }
    [.createMutex 0 1 [buf] 5, .spawn 0 1, .lock 0 1 10, .unlock 0 10, .lock 1 1 11,
     .unlock 1 11, .transferCustody 0 1 9 (.mutexHandoff 1)] = none := by decide
example : (run { custody := [(7, 0)],
                 holdings := [{ custodian := .task 0, footprint := buf, share := fullAccess }] }
    [.createMutex 0 1 [buf] 5, .spawn 0 1, .lock 0 1 10, .unlock 0 10, .lock 1 1 11,
     .unlock 1 11, .transferCustody 0 1 7 (.mutexHandoff 1)]).isSome = true := by decide


/-! ## 11. Source lowering (`source.mutex.guard.v1`, card T03)

The model of `crates/qcore-frontend/src/source_mutex.rs`. A source critical
section is a lexical block; this section says what the lowering of such a block
is allowed to be, and what an accepted lowering forces.

Two boundaries, stated before the definitions. Source extraction — that the Rust
lowering really produces `lowerSection` for the source it reads — is a premise,
as it is for every other source route here. And `Event` above has no
resource-destruction constructor, so `lowerScope` models generation lifetime
only; the Rust lowering additionally emits `Destroy` for each declared resource
and the Rust checker applies its quiescence rule to it. That part of scope exit
is covered by the Rust tests, not by the theorems below. -/

inductive Inner where
  | openInv
  | closeInv
  /-- `close g unchanged;` — the checked witness of card T03-F. -/
  | closeChecked
  | read (f : Footprint)
  | write (f : Footprint)
  | borrow (b : BorrowId) (f : Footprint)
  | release (b : BorrowId)
  deriving DecidableEq, Repr

/-- One `with lock M as G { … }` block. `raises` marks the block whose last
statement is a typed-error exit. -/
structure CriticalSection where
  mutex : MutexId
  guard : GuardId
  invariant : InvariantId
  body : List Inner
  raises : Bool
  deriving DecidableEq, Repr

/-- The block's exit. A typed-error exit and a normal exit differ only in which
event names the path; `typed_error_exit_obeys_the_normal_release_rule` below
shows they are subject to the same rule. -/
def exitEvent (task : TaskId) (s : CriticalSection) : Event :=
  match s.raises with
  | true => .errorExit task s.guard
  | false => .unlock task s.guard

def lowerInner (task : TaskId) (guard : GuardId) (inv : InvariantId) : Inner → Event
  | .openInv => .openInvariant task guard
  | .closeInv => .closeInvariant task guard inv
  | .closeChecked => .closeUnwritten task guard
  | .read f => .access task f .read (some guard)
  | .write f => .access task f .write (some guard)
  | .borrow b f => .borrowUnderGuard task guard f b
  | .release b => .endBorrow task b

def lowerSection (task : TaskId) (s : CriticalSection) : List Event :=
  Event.lock task s.mutex s.guard ::
    (s.body.map (lowerInner task s.guard s.invariant) ++ [exitEvent task s])

/-- One `thread scope` body: the declared generations and the critical sections
over them. Scope exit destroys every generation in reverse declaration order. -/
structure Scope where
  generations : List (MutexId × InvariantId × Footprint)
  sections : List CriticalSection
  /-- The protected resources the scope declared. Scope exit destroys each of
  them after every generation over it, which is the `Destroy` half card T03-F
  adds to this model. Defaulted, so every scope written before it existed
  denotes exactly the trace it denoted then. -/
  resources : List ResourceId := []
  deriving DecidableEq, Repr

/-- Scope exit: every generation in reverse declaration order, then every
resource in reverse declaration order. -/
def scopeExit (task : TaskId) (sc : Scope) : List Event :=
  sc.generations.reverse.map (fun g => Event.destroyMutex task g.1) ++
    sc.resources.reverse.map (Event.destroy task)

def lowerScope (task : TaskId) (sc : Scope) : List Event :=
  sc.generations.map (fun g => Event.createMutex task g.1 [g.2.2] g.2.1) ++
    ((sc.sections.map (lowerSection task)).flatten ++ scopeExit task sc)

/-- Task structure: spawning, joining, and custody transfer between tasks. -/
def isTaskEvent : Event → Bool
  | .spawn _ _ => true
  | .join _ _ => true
  | .transferCustody _ _ _ _ => true
  | _ => false

/-- The two events this profile always refuses. -/
def isRefusedConstruct : Event → Bool
  | .extendProtection _ _ _ => true
  | .unsupportedConstruct _ => true
  | _ => false

def lockedGuard : Event → Option GuardId
  | .lock _ _ g => some g
  | _ => none

/-- Nothing is dropped and nothing is invented: a block of `n` statements lowers
to exactly `n + 2` events, the acquisition and the exit being the two. -/
theorem lowerSection_length (task : TaskId) (s : CriticalSection) :
    (lowerSection task s).length = s.body.length + 2 := by
  simp [lowerSection]

/-- Every lowered block is bracketed: it begins with the acquisition that mints
its guard and ends with that guard's exit. -/
theorem lowerSection_brackets (task : TaskId) (s : CriticalSection) :
    lowerSection task s =
      (Event.lock task s.mutex s.guard :: s.body.map (lowerInner task s.guard s.invariant))
        ++ [exitEvent task s] := by
  simp [lowerSection]

theorem lowerSection_head (task : TaskId) (s : CriticalSection) :
    (lowerSection task s).head? = some (Event.lock task s.mutex s.guard) := by
  simp [lowerSection]

private theorem body_has_no_task_event (task guard : TaskId) (inv : InvariantId)
    (body : List Inner) (e : Event) (mem : e ∈ body.map (lowerInner task guard inv)) :
    isTaskEvent e = false := by
  obtain ⟨i, _, eq⟩ := List.mem_map.mp mem
  subst eq
  cases i <;> rfl

private theorem body_has_no_refused_construct (task guard : TaskId) (inv : InvariantId)
    (body : List Inner) (e : Event) (mem : e ∈ body.map (lowerInner task guard inv)) :
    isRefusedConstruct e = false := by
  obtain ⟨i, _, eq⟩ := List.mem_map.mp mem
  subst eq
  cases i <;> rfl

/-- A guard cannot be moved to a child task by this lowering, because the
lowering emits no task event at all. The source route refuses task creation; the
model records that the refusal leaves nothing behind for a guard to travel on. -/
theorem lowerSection_emits_no_task_event (task : TaskId) (s : CriticalSection) (e : Event)
    (mem : e ∈ lowerSection task s) : isTaskEvent e = false := by
  rw [lowerSection, List.mem_cons, List.mem_append, List.mem_singleton] at mem
  rcases mem with eq | inner | eq
  · subst eq; rfl
  · exact body_has_no_task_event task s.guard s.invariant s.body e inner
  · subst eq; cases hr : s.raises <;> simp [exitEvent, hr, isTaskEvent]

/-- The protected shape is fixed at the declaration, so no lowered statement can
propose to change it, and no lowered statement is an unsupported construct the
checker would have to refuse after the fact. -/
theorem lowerSection_emits_no_refused_construct (task : TaskId) (s : CriticalSection) (e : Event)
    (mem : e ∈ lowerSection task s) : isRefusedConstruct e = false := by
  rw [lowerSection, List.mem_cons, List.mem_append, List.mem_singleton] at mem
  rcases mem with eq | inner | eq
  · subst eq; rfl
  · exact body_has_no_refused_construct task s.guard s.invariant s.body e inner
  · subst eq; cases hr : s.raises <;> simp [exitEvent, hr, isRefusedConstruct]

private theorem sections_have_no_task_event (task : TaskId) (sections : List CriticalSection)
    (e : Event) (mem : e ∈ (sections.map (lowerSection task)).flatten) :
    isTaskEvent e = false := by
  obtain ⟨l, hl, inner⟩ := List.mem_flatten.mp mem
  obtain ⟨s, _, eq⟩ := List.mem_map.mp hl
  subst eq
  exact lowerSection_emits_no_task_event task s e inner

/-- The whole scope emits no task event either: no `spawn`, no `join`, and no
custody transfer to another task appears anywhere in a lowered scope. -/
theorem lowerScope_emits_no_task_event (task : TaskId) (sc : Scope) (e : Event)
    (mem : e ∈ lowerScope task sc) : isTaskEvent e = false := by
  rw [lowerScope, List.mem_append, List.mem_append, scopeExit, List.mem_append] at mem
  rcases mem with create | inner | destroy | retire
  · obtain ⟨g, _, eq⟩ := List.mem_map.mp create
    subst eq
    rfl
  · exact sections_have_no_task_event task sc.sections e inner
  · obtain ⟨g, _, eq⟩ := List.mem_map.mp destroy
    subst eq
    rfl
  · obtain ⟨r, _, eq⟩ := List.mem_map.mp retire
    subst eq
    rfl

private theorem lockedGuard_of_body (task guard : TaskId) (inv : InvariantId)
    (body : List Inner) :
    (body.map (lowerInner task guard inv)).filterMap lockedGuard = [] := by
  induction body with
  | nil => rfl
  | cons i t ih => cases i <;> simp [List.filterMap_cons, lowerInner, lockedGuard, ih]

/-- Authority is keyed to the critical section: one block mints exactly one
guard identity, so two blocks over one generation are two authorities. -/
theorem lowerSection_mints_exactly_its_own_guard (task : TaskId) (s : CriticalSection) :
    (lowerSection task s).filterMap lockedGuard = [s.guard] := by
  have exit : ([exitEvent task s]).filterMap lockedGuard = [] := by
    cases hr : s.raises <;> simp [exitEvent, hr, lockedGuard]
  rw [lowerSection, List.filterMap_cons, List.filterMap_append,
    lockedGuard_of_body task s.guard s.invariant s.body, exit]
  rfl

private theorem lockedGuard_of_sections (task : TaskId) (sections : List CriticalSection) :
    ((sections.map (lowerSection task)).flatten).filterMap lockedGuard
      = sections.map (fun s => s.guard) := by
  induction sections with
  | nil => rfl
  | cons s t ih =>
      rw [List.map_cons, List.flatten_cons, List.filterMap_append,
        lowerSection_mints_exactly_its_own_guard, ih, List.map_cons]
      rfl

private theorem lockedGuard_of_creates (task : TaskId)
    (generations : List (MutexId × InvariantId × Footprint)) :
    (generations.map (fun g => Event.createMutex task g.1 [g.2.2] g.2.1)).filterMap lockedGuard
      = [] := by
  induction generations with
  | nil => rfl
  | cons g t ih => rw [List.map_cons, List.filterMap_cons, ih]; rfl

private theorem lockedGuard_of_destroys (task : TaskId)
    (generations : List (MutexId × InvariantId × Footprint)) :
    (generations.map (fun g => Event.destroyMutex task g.1)).filterMap lockedGuard = [] := by
  induction generations with
  | nil => rfl
  | cons g t ih => rw [List.map_cons, List.filterMap_cons, ih]; rfl

private theorem lockedGuard_of_retires (task : TaskId) (resources : List ResourceId) :
    (resources.map (Event.destroy task)).filterMap lockedGuard = [] := by
  induction resources with
  | nil => rfl
  | cons r t ih => rw [List.map_cons, List.filterMap_cons, ih]; rfl

private theorem lockedGuard_of_scopeExit (task : TaskId) (sc : Scope) :
    (scopeExit task sc).filterMap lockedGuard = [] := by
  rw [scopeExit, List.filterMap_append, lockedGuard_of_destroys task sc.generations.reverse,
    lockedGuard_of_retires task sc.resources.reverse]
  rfl

/-- A scope mints exactly one guard per critical section, in source order: the
lowering neither shares an authority between two blocks nor manufactures one. -/
theorem lowerScope_mints_one_guard_per_section (task : TaskId) (sc : Scope) :
    (lowerScope task sc).filterMap lockedGuard = sc.sections.map (fun s => s.guard) := by
  rw [lowerScope, List.filterMap_append, List.filterMap_append,
    lockedGuard_of_creates, lockedGuard_of_sections, lockedGuard_of_scopeExit]
  simp

/-- Scope exit destroys every generation the scope declared. Whether that
destruction is *admitted* is the checker's quiescence rule; that it is
*attempted* is the lowering's obligation. -/
theorem lowerScope_destroys_every_declared_generation (task : TaskId) (sc : Scope)
    (g : MutexId × InvariantId × Footprint) (mem : g ∈ sc.generations) :
    Event.destroyMutex task g.1 ∈ lowerScope task sc := by
  have base : Event.destroyMutex task g.1 ∈
      sc.generations.reverse.map (fun g => Event.destroyMutex task g.1) :=
    List.mem_map_of_mem (List.mem_reverse.mpr mem)
  rw [lowerScope, List.mem_append, List.mem_append, scopeExit, List.mem_append]
  exact Or.inr (Or.inr (Or.inl base))

/-- The other half of scope exit, and the reason `Event` gained a
resource-destruction constructor in card T03-F: the lowering attempts to
destroy every resource the scope declared, not only every generation. Whether
the attempt is *admitted* is `canDestroy`'s quiescence rule. -/
theorem lowerScope_destroys_every_declared_resource (task : TaskId) (sc : Scope)
    (r : ResourceId) (mem : r ∈ sc.resources) :
    Event.destroy task r ∈ lowerScope task sc := by
  have base : Event.destroy task r ∈ sc.resources.reverse.map (Event.destroy task) :=
    List.mem_map_of_mem (List.mem_reverse.mpr mem)
  rw [lowerScope, List.mem_append, List.mem_append, scopeExit, List.mem_append]
  exact Or.inr (Or.inr (Or.inr base))

/-- Scope exit destroys every generation before it destroys any resource. The
order is not decoration: `canDestroy` refuses while a generation still parks
authority over the resource, so the reverse order could never be admitted. -/
theorem scopeExit_destroys_generations_before_resources (task : TaskId) (sc : Scope) :
    scopeExit task sc =
      sc.generations.reverse.map (fun g => Event.destroyMutex task g.1) ++
        sc.resources.reverse.map (Event.destroy task) := rfl

/-! ### What an accepted lowering forces -/

theorem run_append (s : State) (l r : List Event) :
    run s (l ++ r) = (run s l).bind (fun u => run u r) := by
  induction l generalizing s with
  | nil => simp [run]
  | cons e t ih =>
      cases h : step s e with
      | none => simp [run, h]
      | some next => simp [run, h, ih]

/-- A typed-error exit is not a weaker exit: the two release events have the
same transition, so `close` before `raise` is required exactly as it is before a
normal block end. -/
theorem typed_error_exit_obeys_the_normal_release_rule (s : State) (task : TaskId)
    (guard : GuardId) : step s (.errorExit task guard) = step s (.unlock task guard) := rfl

/-- Neither exit of a lowered block admits a broken invariant. -/
theorem lowered_exit_refuses_an_open_invariant {s : State} {task : TaskId}
    {sec : CriticalSection} {record : GuardRecord}
    (live : s.guardOf sec.guard = some record) (broken : record.phase = .opened) :
    step s (exitEvent task sec) = none := by
  have refused : canRelease s task sec.guard = false := by
    simp [canRelease, State.guardSat, live, broken]
  cases hr : sec.raises <;> simp [exitEvent, hr, step, refused]

/-- Neither exit of a lowered block admits a live guard-derived borrow. -/
theorem lowered_exit_refuses_a_live_borrow {s : State} {task : TaskId}
    {sec : CriticalSection} {record : GuardRecord} {borrow : BorrowId}
    (live : s.guardOf sec.guard = some record) (held : borrow ∈ record.borrows) :
    step s (exitEvent task sec) = none := by
  have nonempty : record.borrows.isEmpty = false := by
    cases hb : record.borrows with
    | nil => rw [hb] at held; simp at held
    | cons _ _ => rfl
  have refused : canRelease s task sec.guard = false := by
    simp [canRelease, State.guardSat, live, nonempty]
  cases hr : sec.raises <;> simp [exitEvent, hr, step, refused]

private theorem find_after_filter_ne (l : List GuardRecord) (g : GuardId) :
    (l.filter (fun r => r.id != g)).find? (fun r => r.id == g) = none := by
  induction l with
  | nil => rfl
  | cons r t ih =>
      by_cases h : r.id = g
      · simp [h, ih]
      · simp [h, ih]

private theorem release_kills_the_guard {u t : State} {task : TaskId} {guard : GuardId}
    (stepped : step u (.unlock task guard) = some t) : t.guardOf guard = none := by
  cases h : canRelease u task guard with
  | false => simp [step, h] at stepped
  | true =>
      simp only [step, h, ite_true, Option.some.injEq] at stepped
      subst stepped
      simp [State.guardOf, releaseEffect, find_after_filter_ne u.guards guard]

/-- A lowered block the checker accepts leaves no live guard behind: the
authority the block minted does not outlive the block. -/
theorem accepted_section_leaves_no_live_guard {s t : State} {task : TaskId}
    {sec : CriticalSection} (accepted : run s (lowerSection task sec) = some t) :
    t.guardOf sec.guard = none := by
  rw [lowerSection_brackets, run_append] at accepted
  cases hu : run s (Event.lock task sec.mutex sec.guard ::
      sec.body.map (lowerInner task sec.guard sec.invariant)) with
  | none => rw [hu] at accepted; simp at accepted
  | some u =>
      rw [hu] at accepted
      simp only [Option.bind_some] at accepted
      have stepped : step u (exitEvent task sec) = some t := by
        simpa [run] using accepted
      cases hr : sec.raises with
      | false =>
          simp only [exitEvent, hr] at stepped
          exact release_kills_the_guard stepped
      | true =>
          simp only [exitEvent, hr, typed_error_exit_obeys_the_normal_release_rule] at stepped
          exact release_kills_the_guard stepped

/-! ### Decidable lowered scenarios

Each mirrors a source fixture in
`crates/qcore-frontend/tests/source_mutex.rs`. -/

private def cell : Footprint := { resource := 1, start := 0, stop := 4 }

private def held : State :=
  { holdings := [{ custodian := .task 0, footprint := cell, share := fullAccess }] }

private def scopeOf (sections : List CriticalSection) : Scope :=
  { generations := [(1, 1, cell)], sections := sections }

-- `with lock m as g { open g; write buf with g; close g; }` checks.
example : (run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell, .closeInv], raises := false }]))).isSome = true := by
  decide

-- The same block without `close` does not: the normal exit refuses it.
example : run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell], raises := false }])) = none := by decide

-- Nor does the typed-error exit of the same unrestored block.
example : run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell], raises := true }])) = none := by decide

-- Restoring it first admits the typed-error exit.
example : (run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell, .closeInv], raises := true }]))).isSome = true := by
  decide

-- A guard-derived borrow must be released inside its own block.
example : run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.borrow 1 cell], raises := false }])) = none := by decide
example : (run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.borrow 1 cell, .read cell, .release 1], raises := false }]))).isSome = true := by
  decide

-- Two blocks over one generation are two authorities, and both check.
example : (run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell, .closeInv], raises := false },
     { mutex := 1, guard := 2, invariant := 1, body := [.read cell], raises := false }]))).isSome
      = true := by decide

-- A write with the invariant closed is refused inside the lowering too.
example : run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.write cell], raises := false }])) = none := by decide

/-! ## 12. Resource destruction (card T03-F)

`Event` gained `destroy` here, so the second half of scope exit is decided by
this model rather than resting on the Rust tests alone. The rules mirror
`qcore_core::mutex_invariant`'s `Destroy` arm: custody, liveness, and
quiescence against parked and foreign authority. Ordering against unsynchronized
accesses is the Rust checker's vector-clock rule and is **not** modeled here;
this model has no clocks, exactly as it has none for `canDestroyMutex`. -/

theorem destroyEffect_preserves_authority (s : State) (resource : ResourceId) :
    (destroyEffect s resource).authorityTotal = s.authorityTotal := rfl

/-- Only the custodian destroys. -/
theorem destroy_requires_custody {s : State} {task : TaskId} {resource : ResourceId}
    (foreign : (resource, task) ∉ s.custody) :
    step s (.destroy task resource) = none := by
  simp [step, canDestroy, foreign]

/-- Parked authority is outstanding authority: a live generation over any part
of the resource blocks its destruction. This is why scope exit destroys every
generation before any resource. -/
theorem protected_resource_rejects_destruction {s : State} {task : TaskId}
    {resource : ResourceId} {record : MutexRecord} (live : record ∈ s.mutexes)
    (protects : record.protects.any (fun f => f.resource == resource) = true) :
    step s (.destroy task resource) = none := by
  have any : s.mutexes.any (fun m => m.protects.any (fun f => f.resource == resource)) = true :=
    List.any_eq_true.mpr ⟨record, live, protects⟩
  simp [step, canDestroy, any]

/-- A share another live task still holds blocks destruction too. -/
theorem foreign_share_rejects_destruction {s : State} {task other : TaskId}
    {resource : ResourceId} {h : Holding} (held : h ∈ s.holdings)
    (over : h.footprint.resource = resource) (custodian : h.custodian = .task other)
    (foreign : other ≠ task) (share : 0 < h.share) :
    step s (.destroy task resource) = none := by
  have any : s.holdings.any (outstandingOver task resource) = true := by
    refine List.any_eq_true.mpr ⟨h, held, ?_⟩
    simp [outstandingOver, over, custodian, foreign, share]
  simp [step, canDestroy, any]

/-- A destroyed resource is dead forever. -/
theorem dead_resource_rejects_repeat_destruction {s : State} {task : TaskId}
    {resource : ResourceId} (dead : resource ∈ s.destroyedResources) :
    step s (.destroy task resource) = none := by
  simp [step, canDestroy, dead]

/-- And it is unreachable: neither an unguarded nor a guarded access to a
destroyed resource is admitted. -/
theorem destroyed_resource_rejects_access {s : State} {task : TaskId} {f : Footprint}
    {kind : AccessKind} {guard : Option GuardId}
    (dead : f.resource ∈ s.destroyedResources) :
    step s (.access task f kind guard) = none := by
  cases guard <;> simp [step, canAccessUnguarded, canAccessGuarded, dead]

/-- An accepted destruction retires the resource from liveness and from
custody. -/
theorem accepted_destruction_kills_the_resource {s next : State} {task : TaskId}
    {resource : ResourceId} (accepted : step s (.destroy task resource) = some next) :
    resource ∈ next.destroyedResources ∧ (resource, task) ∉ next.custody := by
  cases h : canDestroy s task resource with
  | false => simp [step, h] at accepted
  | true =>
      simp only [step, h, ite_true, Option.some.injEq] at accepted
      subst accepted
      refine ⟨List.mem_cons_self .., ?_⟩
      intro mem
      have := (List.mem_filter.mp mem).2
      simp at this

/-- Fail-closed: a refused destruction is not skipped, it stops the trace. -/
theorem refused_destruction_stops_the_trace {s : State} {task : TaskId}
    {resource : ResourceId} {rest : List Event}
    (refused : step s (.destroy task resource) = none) :
    run s (.destroy task resource :: rest) = none := by
  simp [run, refused]

/-! ## 13. The checked invariant witness (card T03-F)

`close g unchanged;` asks the checker to own the fact the declared witness only
asserts. The two witnesses have the same effect and differ only in what
justifies them, so the checked one is not a weaker close. -/

theorem checked_witness_requires_no_write {s : State} {task : TaskId} {guard : GuardId}
    {record : GuardRecord} (held : s.guardOf guard = some record)
    (wrote : record.wroteWhileOpen = true) : step s (.closeUnwritten task guard) = none := by
  simp [step, canCloseUnwritten, State.guardSat, held, wrote]

theorem checked_witness_requires_an_open_invariant {s : State} {task : TaskId}
    {guard : GuardId} {record : GuardRecord} (held : s.guardOf guard = some record)
    (closed : record.phase = .closed) : step s (.closeUnwritten task guard) = none := by
  simp [step, canCloseUnwritten, State.guardSat, held, closed]

/-- The checked witness consults no declared predicate: whatever invariant
identity the generations declare, acceptance is unchanged. That is precisely
why a section closing this way carries no `invariantPredicateDeclared`
premise. -/
theorem checked_witness_ignores_the_declared_predicate (s : State) (task : TaskId)
    (guard : GuardId) (f : MutexRecord → MutexRecord) :
    canCloseUnwritten { s with mutexes := s.mutexes.map f } task guard
      = canCloseUnwritten s task guard := rfl

/-- The two witnesses agree on the state they produce. -/
theorem checked_and_declared_close_agree_on_effect {s t u : State} {task : TaskId}
    {guard : GuardId} {inv : InvariantId}
    (checked : step s (.closeUnwritten task guard) = some t)
    (declared : step s (.closeInvariant task guard inv) = some u) : t = u := by
  cases hc : canCloseUnwritten s task guard with
  | false => simp [step, hc] at checked
  | true =>
      cases hd : canClose s task guard inv with
      | false => simp [step, hd] at declared
      | true =>
          simp only [step, hc, hd, ite_true, Option.some.injEq] at checked declared
          exact checked ▸ declared ▸ rfl

/-- A `check` that declares no predicate names none: the premise is dropped,
not renamed. -/
theorem checked_close_carries_no_declared_predicate_premise {initial : State}
    {events : List Event} {declaredShape : Bool} {certificate : Certificate}
    (result : check initial events declaredShape false = some certificate) :
    Assumption.invariantPredicateDeclared ∉ certificate.premises := by
  unfold check at result
  cases hrun : run initial events with
  | none => rw [hrun] at result; simp at result
  | some final =>
      rw [hrun] at result
      simp only [Option.map_some] at result
      cases Option.some.inj result
      cases declaredShape <;> simp [premises]

/-- Which lowered events would need a declared predicate. -/
def declaresInvariant : Event → Bool
  | .closeInvariant _ _ _ => true
  | _ => false

/-- The source-shape half of the same statement: a section whose body never
uses the declared close lowers to a trace containing no `closeInvariant`, so
nothing in it can demand the declared predicate. -/
theorem a_section_without_a_declared_close_emits_none (task : TaskId) (s : CriticalSection)
    (checked : Inner.closeInv ∉ s.body) (e : Event) (mem : e ∈ lowerSection task s) :
    declaresInvariant e = false := by
  rw [lowerSection, List.mem_cons, List.mem_append, List.mem_singleton] at mem
  rcases mem with eq | inner | eq
  · subst eq; rfl
  · obtain ⟨i, hi, eq⟩ := List.mem_map.mp inner
    subst eq
    cases i with
    | closeInv => exact absurd hi checked
    | _ => rfl
  · subst eq; cases hr : s.raises <;> simp [exitEvent, hr, declaresInvariant]

/-! ## 14. Bounded task composition inside the guard profile (card T03-F)

The source profile's child-task form is a nested `thread scope`, created where
the block opens and joined where it closes. Nothing below widens the ledger:
these are the *existing* rules, stated as the reasons the composition is
bounded where it is. -/

/-- A join is refused while the joining task holds a guard. This is why a child
task written *inside* a critical section is refused: its join lands inside the
guard. -/
theorem join_while_holding_a_guard_is_refused {s : State} {parent child : TaskId}
    {record : GuardRecord} (live : record ∈ s.guards) (holder : record.holder = parent) :
    step s (.join parent child) = none := by
  have quiet : s.guards.all (fun g => g.holder != parent && g.holder != child) = false := by
    refine List.all_eq_false.mpr ⟨record, live, ?_⟩
    simp [holder]
  simp [step, canJoin, quiet]

/-- Nor is a child joinable while it still holds a section of its own: the
child releases before the creator collects it. -/
theorem join_of_a_guard_holding_child_is_refused {s : State} {parent child : TaskId}
    {record : GuardRecord} (live : record ∈ s.guards) (holder : record.holder = child) :
    step s (.join parent child) = none := by
  have quiet : s.guards.all (fun g => g.holder != parent && g.holder != child) = false := by
    refine List.all_eq_false.mpr ⟨record, live, ?_⟩
    simp [holder]
  simp [step, canJoin, quiet]

/-- A child receives no share, so an unprotected footprint is out of reach for
it; `unguarded_access_to_protected_footprint_is_rejected` covers the protected
case. Together: every access a child makes is mutex-mediated or refused. -/
theorem a_shareless_task_cannot_access_unguarded {s : State} {task : TaskId} {f : Footprint}
    {kind : AccessKind} (none_held : s.taskShare task f = 0) :
    step s (.access task f kind none) = none := by
  cases kind <;> simp [step, canAccessUnguarded, none_held, fullAccess]

/-! ## 15. The guard-local join (card T03-GUARD-JOIN)

§14 states why a join inside a critical section is refused: `canJoin` demands
that the joining task hold no guard. That rule is **unedited** — every theorem
above still holds of `step` — and this section adds the one opt-in exception,
as a separate decision procedure over the same `State` plus the auxiliary
ledger the Rust checker maintains beside it.

The hazard the blanket refusal covers is one this model cannot otherwise see: a
join is a *blocking wait*, and nothing here blocks. `canLock` refuses an
acquisition of a held generation outright, and the representable form of "task
`t` is waiting for mutex `m`" is `registerWaiter`. So the admission does not
try to decide when a wait terminates; it removes the possibility of a wait
altogether, by requiring the whole joined subtree to have named no mutex, no
guard and no guard-scoped fact anywhere in the trace.

`GuardLedger` is additive: `State`, `Event`, `step` and `run` are untouched, so
nothing above is restated or weakened. The correspondence between this
procedure and the Rust checker is the standing transcription premise, exactly
as for `step` itself. One deliberate difference is recorded: the Rust checker
marks a task mutex-keyed when the event *occurs*, before its own admission
check, while `GuardLedger.observe` here is threaded alongside accepted steps.
On an accepted trace no event is refused, so the two coincide; on a refused one
the Rust rule is strictly more refusing, never less. -/

/-- The bookkeeping the guard-local rule needs and `State` does not carry:
which sections a task held when each of its children was created, and which
tasks have ever named a mutex, a guard or a guard-scoped fact. -/
structure GuardLedger where
  spawnGuards : List (TaskId × List GuardId) := []
  mutexKeyed : List TaskId := []
  deriving DecidableEq, Repr

def State.guardsHeldBy (s : State) (t : TaskId) : List GuardId :=
  (s.guards.filter (fun g => g.holder == t)).map (fun g => g.id)

/-- The acting task of an event that names a mutex, a guard or a guard-scoped
fact. `none` for the events that name none of the three: task structure,
unguarded access, custody transfer and resource destruction. This is the whole
of G7's vocabulary. -/
def keyingTask : Event → Option TaskId
  | .createMutex t _ _ _ => some t
  | .destroyMutex t _ => some t
  | .registerWaiter t _ => some t
  | .lock t _ _ => some t
  | .unlock t _ => some t
  | .errorExit t _ => some t
  | .openInvariant t _ => some t
  | .closeInvariant t _ _ => some t
  | .closeUnwritten t _ => some t
  | .borrowUnderGuard t _ _ _ => some t
  | .endBorrow t _ => some t
  | .assertFact t _ _ _ => some t
  | .useFact t _ => some t
  | .extendProtection t _ _ => some t
  | .unsupportedConstruct t => some t
  | .access t _ _ (some _) => some t
  | .access _ _ _ none => none
  | .spawn _ _ => none
  | .join _ _ => none
  | .transferCustody _ _ _ _ => none
  | .destroy _ _ => none

def GuardLedger.observe (l : GuardLedger) (s : State) (e : Event) : GuardLedger :=
  let recorded :=
    match e with
    | .spawn parent child =>
        { l with spawnGuards := (child, s.guardsHeldBy parent) :: l.spawnGuards }
    | _ => l
  match keyingTask e with
  | some t => { recorded with mutexKeyed := t :: recorded.mutexKeyed }
  | none => recorded

private def growSubtree (parents : List (TaskId × TaskId)) (acc : List TaskId) :
    List TaskId :=
  acc ++ (parents.filter (fun p => acc.contains p.2 && !acc.contains p.1)).map (fun p => p.1)

private def subtreeAux : Nat → List (TaskId × TaskId) → List TaskId → List TaskId
  | 0, _, acc => acc
  | n + 1, parents, acc =>
      if (growSubtree parents acc).length == acc.length then acc
      else subtreeAux n parents (growSubtree parents acc)

/-- `root` together with every task transitively spawned under it. The parent
relation is finite, so `parents.length + 1` rounds reach the fixed point. -/
def State.subtreeOf (s : State) (root : TaskId) : List TaskId :=
  subtreeAux (s.parents.length + 1) s.parents [root]

/-- G1: the ordinary join's own structure, minus its clause about the *joining*
task's guards — which is exactly the clause this section replaces. The child's
own quiescence is kept verbatim. -/
def glStructure (s : State) (parent child : TaskId) : Bool :=
  s.act parent && s.act child && s.parents.contains (child, parent) &&
    s.guards.all (fun g => g.holder != child)

/-- G2 and G3: `guard` is live, held by `parent`, and the only section it
holds. Two held sections would need a wait-for argument across both. -/
def glSection (s : State) (parent : TaskId) (guard : GuardId) : Bool :=
  s.guardsHeldBy parent == [guard]

/-- G4: the child was created while `parent` held exactly this same section.
Guard identities are never reused, so this is "strictly inside *this* critical
section" rather than "inside some section over the same generation". -/
def glCreatedInSection (l : GuardLedger) (child : TaskId) (guard : GuardId) : Bool :=
  l.spawnGuards.contains (child, [guard])

/-- G5 and G6: the invariant is closed and no guard-derived borrow is live. -/
def glSectionQuiet (s : State) (guard : GuardId) : Bool :=
  s.guardSat guard (fun g => g.phase == Phase.closed && g.borrows.isEmpty)

/-- G7: nobody in the joined subtree ever named a mutex, a guard or a
guard-scoped fact. This is what makes the wait impossible rather than merely
unlikely. -/
def glQuietSubtree (s : State) (l : GuardLedger) (child : TaskId) : Bool :=
  (s.subtreeOf child).all (fun t => !l.mutexKeyed.contains t)

/-- G8: no live generation records anyone in the subtree as a waiter. -/
def glNoWaiter (s : State) (child : TaskId) : Bool :=
  s.mutexes.all (fun m => m.waiters.all (fun w => !(s.subtreeOf child).contains w))

/-- G9: the subtree carries no share, so its completion hands the creator no
pointwise authority. On a protected atom this holds anyway — creation parked the
whole share — so the check bites on unprotected footprints. -/
def glNoShare (s : State) (child : TaskId) : Bool :=
  s.holdings.all (fun h =>
    match h.custodian with
    | .task t => !(s.subtreeOf child).contains t || h.share == 0
    | .mutex _ => true)

/-- G10: the subtree is custodian of no live resource. -/
def glNoCustody (s : State) (child : TaskId) : Bool :=
  s.custody.all (fun c => !(s.subtreeOf child).contains c.2)

def canGuardLocalJoin (s : State) (l : GuardLedger) (parent child : TaskId)
    (guard : GuardId) : Bool :=
  glStructure s parent child && glSection s parent guard &&
    glCreatedInSection l child guard && glSectionQuiet s guard &&
    glQuietSubtree s l child && glNoWaiter s child && glNoShare s child &&
    glNoCustody s child

/-- The admitted effect is the ordinary join's effect and nothing more: the
guard-local rule changes *whether* a join is admitted, never what it does. -/
def stepGuardLocalJoin (s : State) (l : GuardLedger) (parent child : TaskId)
    (guard : GuardId) : Option State :=
  if canGuardLocalJoin s l parent child guard then some (joinEffect s parent child) else none

/-! ### What an accepted guard-local join does, and does not, disturb -/

theorem guard_local_join_effect_is_the_ordinary_join {s next : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId}
    (accepted : stepGuardLocalJoin s l parent child guard = some next) :
    next = joinEffect s parent child := by
  simp only [stepGuardLocalJoin] at accepted
  split at accepted
  · exact (Option.some.inj accepted).symm
  · contradiction

/-- Conservation: the join moves no share, so the pointwise ledger the profile
verifies before and after every event is untouched. -/
theorem guard_local_join_preserves_authority {s next : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId}
    (accepted : stepGuardLocalJoin s l parent child guard = some next) :
    next.authorityTotal = s.authorityTotal ∧ next.holdings = s.holdings := by
  rw [guard_local_join_effect_is_the_ordinary_join accepted]
  exact ⟨rfl, rfl⟩

/-- The declared shapes and the custody map survive unchanged. -/
theorem guard_local_join_preserves_declared_shapes {s next : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId}
    (accepted : stepGuardLocalJoin s l parent child guard = some next) :
    next.mutexes = s.mutexes ∧ next.custody = s.custody := by
  rw [guard_local_join_effect_is_the_ordinary_join accepted]
  exact ⟨rfl, rfl⟩

/-- Nothing the invariant needs is dropped: every guard record — its phase, its
borrows and its guard-scoped facts — is exactly what it was. This is the other
half of the soundness argument, beside conservation. -/
theorem guard_local_join_drops_nothing_the_invariant_needs {s next : State}
    {l : GuardLedger} {parent child : TaskId} {guard : GuardId}
    (accepted : stepGuardLocalJoin s l parent child guard = some next) :
    next.guards = s.guards ∧ next.guardOf guard = s.guardOf guard ∧
      next.guardsHeldBy parent = s.guardsHeldBy parent := by
  rw [guard_local_join_effect_is_the_ordinary_join accepted]
  exact ⟨rfl, rfl, rfl⟩

/-- Exactly the joined child becomes terminal. -/
theorem guard_local_join_completes_exactly_the_child {s next : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId}
    (accepted : stepGuardLocalJoin s l parent child guard = some next) :
    next.terminalTasks = child :: s.terminalTasks ∧
      next.joinedBy = (child, parent) :: s.joinedBy := by
  rw [guard_local_join_effect_is_the_ordinary_join accepted]
  exact ⟨rfl, rfl⟩

/-- Every one of the seven obligations really was decided; none is a comment. -/
theorem accepted_guard_local_join_discharges_its_obligations {s : State}
    {l : GuardLedger} {parent child : TaskId} {guard : GuardId}
    (accepted : (stepGuardLocalJoin s l parent child guard).isSome = true) :
    glStructure s parent child = true ∧ glSection s parent guard = true ∧
      glCreatedInSection l child guard = true ∧ glSectionQuiet s guard = true ∧
      glQuietSubtree s l child = true ∧ glNoWaiter s child = true ∧
      glNoShare s child = true ∧ glNoCustody s child = true := by
  simp only [stepGuardLocalJoin] at accepted
  split at accepted
  · rename_i ok
    simp only [canGuardLocalJoin, Bool.and_eq_true] at ok
    exact ⟨ok.1.1.1.1.1.1.1, ok.1.1.1.1.1.1.2, ok.1.1.1.1.1.2, ok.1.1.1.1.2, ok.1.1.1.2,
      ok.1.1.2, ok.1.2, ok.2⟩
  · simp at accepted

/-- The creator still holds its one section, closed, on the far side of the
join: the admission collects a child, it does not leave the critical section. -/
theorem guard_local_join_keeps_the_creator_in_its_only_section {s next : State}
    {l : GuardLedger} {parent child : TaskId} {guard : GuardId}
    (accepted : stepGuardLocalJoin s l parent child guard = some next) :
    next.guardsHeldBy parent = [guard] ∧
      next.guardSat guard (fun g => g.phase == Phase.closed && g.borrows.isEmpty) = true := by
  have obligations :=
    accepted_guard_local_join_discharges_its_obligations (s := s) (l := l)
      (parent := parent) (child := child) (guard := guard) (by rw [accepted]; rfl)
  have kept := guard_local_join_drops_nothing_the_invariant_needs accepted
  refine ⟨?_, ?_⟩
  · rw [kept.2.2]
    exact of_decide_eq_true (by simpa [glSection] using obligations.2.1)
  · rw [State.guardSat, kept.2.1]
    exact obligations.2.2.2.1

/-! ### The refusals: everything outside the rule -/

private theorem subtreeAux_keeps (n : Nat) (parents : List (TaskId × TaskId))
    (acc : List TaskId) (t : TaskId) (mem : t ∈ acc) : t ∈ subtreeAux n parents acc := by
  induction n generalizing acc with
  | zero => simpa [subtreeAux] using mem
  | succ n ih =>
      simp only [subtreeAux]
      split
      · exact mem
      · exact ih _ (by unfold growSubtree; exact List.mem_append_left _ mem)

/-- G7 is non-vacuous: the joined child is always in its own subtree. -/
theorem the_quiet_subtree_contains_the_child (s : State) (child : TaskId) :
    child ∈ s.subtreeOf child :=
  subtreeAux_keeps _ _ _ _ (by simp)

/-- G7: a child that ever named a mutex, a guard or a guard-scoped fact is not
joinable inside a section. This is the rule that makes the deadlock hazard
structurally impossible rather than merely improbable. -/
theorem guard_local_join_requires_a_quiet_subtree {s : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId} (keyed : child ∈ l.mutexKeyed) :
    stepGuardLocalJoin s l parent child guard = none := by
  have noisy : glQuietSubtree s l child = false :=
    List.all_eq_false.mpr
      ⟨child, the_quiet_subtree_contains_the_child s child, by simpa using keyed⟩
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- G7 is a subtree property, not a child property: a *grandchild* that named a
mutex refuses the join just as the child would. -/
theorem guard_local_join_refuses_a_keyed_descendant {s : State} {l : GuardLedger}
    {parent child descendant : TaskId} {guard : GuardId}
    (inSubtree : descendant ∈ s.subtreeOf child) (keyed : descendant ∈ l.mutexKeyed) :
    stepGuardLocalJoin s l parent child guard = none := by
  have noisy : glQuietSubtree s l child = false :=
    List.all_eq_false.mpr ⟨descendant, inSubtree, by simpa using keyed⟩
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- G4: a child created anywhere but strictly inside this same section — before
the lock, or in an earlier section over the same generation — is refused. -/
theorem guard_local_join_requires_creation_in_the_same_section {s : State}
    {l : GuardLedger} {parent child : TaskId} {guard : GuardId}
    (elsewhere : (child, [guard]) ∉ l.spawnGuards) :
    stepGuardLocalJoin s l parent child guard = none := by
  have wrong : glCreatedInSection l child guard = false := by
    simpa [glCreatedInSection] using elsewhere
  simp [stepGuardLocalJoin, canGuardLocalJoin, wrong]

private theorem guardsHeldBy_mem {s : State} {t : TaskId} {r : GuardRecord}
    (mem : r ∈ s.guards) (holder : r.holder = t) : r.id ∈ s.guardsHeldBy t := by
  unfold State.guardsHeldBy
  exact List.mem_map_of_mem (List.mem_filter.mpr ⟨mem, by simp [holder]⟩)

/-- G3: two live sections held by the joining task refuse the admission. -/
theorem guard_local_join_requires_exactly_one_held_section {s : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId} {other : GuardRecord}
    (live : other ∈ s.guards) (holder : other.holder = parent) (distinct : other.id ≠ guard) :
    stepGuardLocalJoin s l parent child guard = none := by
  have wrong : glSection s parent guard = false := by
    unfold glSection
    refine beq_eq_false_iff_ne.mpr (fun equal => distinct ?_)
    have := guardsHeldBy_mem live holder
    rw [equal] at this
    simpa using this
  simp [stepGuardLocalJoin, canGuardLocalJoin, wrong]

/-- G2: the named section must be the joining task's own. A section held by
another task is not a section this join is local to. -/
theorem guard_local_join_requires_the_joining_task_to_hold_the_section {s : State}
    {l : GuardLedger} {parent child : TaskId} {guard : GuardId}
    (foreign : ∀ r ∈ s.guards, r.id = guard → r.holder ≠ parent) :
    stepGuardLocalJoin s l parent child guard = none := by
  have wrong : glSection s parent guard = false := by
    unfold glSection
    refine beq_eq_false_iff_ne.mpr (fun equal => ?_)
    have mem : guard ∈ s.guardsHeldBy parent := by rw [equal]; simp
    unfold State.guardsHeldBy at mem
    simp only [List.mem_map, List.mem_filter] at mem
    obtain ⟨r, ⟨rmem, rholder⟩, rid⟩ := mem
    exact foreign r rmem rid (by simpa using rholder)
  simp [stepGuardLocalJoin, canGuardLocalJoin, wrong]

/-- G5: a join while the invariant is open is refused. Protected memory is in a
declared-broken state there, and `DeclaredFootprintShape` is a premise, so the
model cannot rule out collateral the declaration does not mention. -/
theorem guard_local_join_requires_a_closed_invariant {s : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId} {record : GuardRecord}
    (present : s.guardOf guard = some record) (broken : record.phase = Phase.opened) :
    stepGuardLocalJoin s l parent child guard = none := by
  have noisy : glSectionQuiet s guard = false := by
    simp [glSectionQuiet, State.guardSat, present, broken]
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- G6: a live guard-derived borrow refuses the join. -/
theorem guard_local_join_requires_no_live_borrow {s : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId} {record : GuardRecord}
    (present : s.guardOf guard = some record) (borrowed : record.borrows ≠ []) :
    stepGuardLocalJoin s l parent child guard = none := by
  have nonempty : record.borrows.isEmpty = false := by
    cases hb : record.borrows with
    | nil => exact absurd hb borrowed
    | cons _ _ => rfl
  have noisy : glSectionQuiet s guard = false := by
    simp [glSectionQuiet, State.guardSat, present, nonempty]
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- G8: the deadlock shape itself. A subtree that has announced an acquisition
would never return from the join, so the join is refused. -/
theorem guard_local_join_refuses_a_waiting_subtree {s : State} {l : GuardLedger}
    {parent child waiter : TaskId} {guard : GuardId} {record : MutexRecord}
    (live : record ∈ s.mutexes) (waiting : waiter ∈ record.waiters)
    (inSubtree : waiter ∈ s.subtreeOf child) :
    stepGuardLocalJoin s l parent child guard = none := by
  have inner : record.waiters.all (fun w => !(s.subtreeOf child).contains w) = false :=
    List.all_eq_false.mpr ⟨waiter, waiting, by simpa using inSubtree⟩
  have noisy : glNoWaiter s child = false :=
    List.all_eq_false.mpr ⟨record, live, by simpa using inner⟩
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- G9: a child holding share of anything is refused. -/
theorem guard_local_join_refuses_a_sharing_child {s : State} {l : GuardLedger}
    {parent child holder : TaskId} {guard : GuardId} {h : Holding}
    (live : h ∈ s.holdings) (owned : h.custodian = .task holder)
    (inSubtree : holder ∈ s.subtreeOf child) (nonzero : h.share ≠ 0) :
    stepGuardLocalJoin s l parent child guard = none := by
  have noisy : glNoShare s child = false :=
    List.all_eq_false.mpr ⟨h, live, by simp [owned, nonzero]; simpa using inSubtree⟩
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- G10: a child holding custody of a live resource is refused. -/
theorem guard_local_join_refuses_a_custodian_child {s : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId} {entry : ResourceId × TaskId}
    (live : entry ∈ s.custody) (inSubtree : entry.2 ∈ s.subtreeOf child) :
    stepGuardLocalJoin s l parent child guard = none := by
  have noisy : glNoCustody s child = false :=
    List.all_eq_false.mpr ⟨entry, live, by simpa using inSubtree⟩
  simp [stepGuardLocalJoin, canGuardLocalJoin, noisy]

/-- The child's own quiescence clause is carried over verbatim from `canJoin`:
a child still holding a section of its own is refused here too. -/
theorem guard_local_join_refuses_a_guard_holding_child {s : State} {l : GuardLedger}
    {parent child : TaskId} {guard : GuardId} {record : GuardRecord}
    (live : record ∈ s.guards) (holder : record.holder = child) :
    stepGuardLocalJoin s l parent child guard = none := by
  have busy : glStructure s parent child = false := by
    unfold glStructure
    have : s.guards.all (fun g => g.holder != child) = false :=
      List.all_eq_false.mpr ⟨record, live, by simp [holder]⟩
    simp [this]
  simp [stepGuardLocalJoin, canGuardLocalJoin, busy]

/-! ### The ledger's own vocabulary -/

/-- A spawn records exactly the sections its creator held, which is what G4
later compares against. -/
theorem spawn_records_the_creators_open_sections (l : GuardLedger) (s : State)
    (parent child : TaskId) :
    (l.observe s (.spawn parent child)).spawnGuards =
      (child, s.guardsHeldBy parent) :: l.spawnGuards := rfl

/-- An acquisition marks its task, so G7 sees it. -/
theorem a_lock_is_recorded_as_mutex_keyed (l : GuardLedger) (s : State)
    (task : TaskId) (mutex : MutexId) (guard : GuardId) :
    (l.observe s (.lock task mutex guard)).mutexKeyed = task :: l.mutexKeyed := rfl

/-- An announced acquisition marks its task: the deadlock shape cannot slip past
G7 by never actually locking. -/
theorem a_registered_waiter_is_recorded_as_mutex_keyed (l : GuardLedger) (s : State)
    (task : TaskId) (mutex : MutexId) :
    (l.observe s (.registerWaiter task mutex)).mutexKeyed = task :: l.mutexKeyed := rfl

/-- A guarded access marks its task; an unguarded one does not. The boundary is
exact: an unguarded access names no mutex, and a child holding no share cannot
make one anyway (`a_shareless_task_cannot_access_unguarded`). -/
theorem guarded_and_unguarded_access_are_recorded_differently (l : GuardLedger)
    (s : State) (task : TaskId) (f : Footprint) (kind : AccessKind) (guard : GuardId) :
    (l.observe s (.access task f kind (some guard))).mutexKeyed = task :: l.mutexKeyed ∧
      (l.observe s (.access task f kind none)).mutexKeyed = l.mutexKeyed :=
  ⟨rfl, rfl⟩

/-- Releasing a borrow marks its task even though `endBorrow` itself checks only
liveness. This is why a child's release of its creator's borrow — accepted by
the model on its own terms — still refuses the guard-local join. -/
theorem an_end_borrow_is_recorded_as_mutex_keyed (l : GuardLedger) (s : State)
    (task : TaskId) (borrow : BorrowId) :
    (l.observe s (.endBorrow task borrow)).mutexKeyed = task :: l.mutexKeyed := rfl

/-! ### Decidable guard-local scenarios -/

private def sectionHeld : State :=
  { activeTasks := [0, 1]
    parents := [(1, 0)]
    guards := [{ id := 1, mutex := 1, holder := 0, phase := Phase.closed,
                 borrows := [], facts := [], wroteWhileOpen := false }]
    mutexes := [{ id := 1, protects := [cell], invariant := 1, invariants := [1],
                  creator := 0,
                  holder := some 1, waiters := [], releasedBy := [], observed := [] }]
    holdings := [{ custodian := .mutex 1, footprint := cell, share := fullAccess }] }

private def quietLedger : GuardLedger := { spawnGuards := [(1, [1])], mutexKeyed := [] }

-- The accepted control: a mutex-silent child created inside this section.
example : (stepGuardLocalJoin sectionHeld quietLedger 0 1 1).isSome = true := by decide

-- The child named a mutex somewhere: refused.
example : stepGuardLocalJoin sectionHeld { quietLedger with mutexKeyed := [1] } 0 1 1
    = none := by decide

-- The child was created outside this section: refused.
example : stepGuardLocalJoin sectionHeld { quietLedger with spawnGuards := [(1, [])] } 0 1 1
    = none := by decide

-- The child was created in a different section over the same generation:
-- refused, because guard identities are never reused.
example : stepGuardLocalJoin sectionHeld { quietLedger with spawnGuards := [(1, [2])] } 0 1 1
    = none := by decide

-- The invariant is open where the join lands: refused.
example : stepGuardLocalJoin
    { sectionHeld with guards := [{ id := 1, mutex := 1, holder := 0, phase := Phase.opened,
                                    borrows := [], facts := [], wroteWhileOpen := false }] }
    quietLedger 0 1 1 = none := by decide

-- A guard-derived borrow is still live: refused.
example : stepGuardLocalJoin
    { sectionHeld with guards := [{ id := 1, mutex := 1, holder := 0, phase := Phase.closed,
                                    borrows := [3], facts := [], wroteWhileOpen := false }] }
    quietLedger 0 1 1 = none := by decide

-- The section belongs to another task: refused.
example : stepGuardLocalJoin
    { sectionHeld with guards := [{ id := 1, mutex := 1, holder := 2, phase := Phase.closed,
                                    borrows := [], facts := [], wroteWhileOpen := false }] }
    quietLedger 0 1 1 = none := by decide

-- The child is a recorded waiter on the held generation: the deadlock shape.
example : stepGuardLocalJoin
    { sectionHeld with mutexes := [{ id := 1, protects := [cell], invariant := 1,
                                     invariants := [1],
                                     creator := 0, holder := some 1, waiters := [1],
                                     releasedBy := [], observed := [] }] }
    quietLedger 0 1 1 = none := by decide

-- The child holds share of something: refused.
example : stepGuardLocalJoin
    { sectionHeld with holdings :=
        [{ custodian := .mutex 1, footprint := cell, share := fullAccess },
         { custodian := .task 1, footprint := cell, share := 1 }] }
    quietLedger 0 1 1 = none := by decide

-- The child is custodian of a live resource: refused.
example : stepGuardLocalJoin { sectionHeld with custody := [(7, 1)] } quietLedger 0 1 1
    = none := by decide

-- A quiet grandchild is admitted, and is genuinely inside the subtree.
private def withGrandchild : State :=
  { sectionHeld with parents := [(1, 0), (2, 1)], terminalTasks := [2] }

example : (2 : TaskId) ∈ withGrandchild.subtreeOf 1 := by decide

example : (stepGuardLocalJoin withGrandchild quietLedger 0 1 1).isSome = true := by decide

-- but a grandchild that named a mutex refuses the whole admission.
example : stepGuardLocalJoin withGrandchild { quietLedger with mutexKeyed := [2] } 0 1 1
    = none := by decide

-- The ordinary join over the very same state is still refused: the admission is
-- opt-in, and `step` is unedited.
example : step sectionHeld (.join 0 1) = none := by decide

/-! ### Decidable scenarios for §12-§14 -/

private def custodied : State :=
  { holdings := [{ custodian := .task 0, footprint := cell, share := fullAccess }]
    custody := [(1, 0)] }

private def scopeOver (sections : List CriticalSection) : Scope :=
  { generations := [(1, 1, cell)], sections := sections, resources := [1] }

-- Whole scope exit: one generation, one section, then both destructions.
example : (run custodied (lowerScope 0 (scopeOver
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell, .closeInv], raises := false }]))).isSome = true := by
  decide

-- The resource may not be destroyed while its generation is still live.
example : run custodied [.createMutex 0 1 [cell] 1, .destroy 0 1] = none := by decide

-- Destroying the generation first admits it.
example : (run custodied
    [.createMutex 0 1 [cell] 1, .destroyMutex 0 1, .destroy 0 1]).isSome = true := by decide

-- A resource is destroyed once.
example : run custodied [.destroy 0 1, .destroy 0 1] = none := by decide

-- A destroyed resource is unreachable.
example : run custodied [.destroy 0 1, .access 0 cell .read none] = none := by decide

-- `open g; read buf with g; close g unchanged;` checks, and needs no declared
-- predicate.
example : (run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .read cell, .closeChecked], raises := false }]))).isSome = true := by
  decide

-- The same section with a write cannot use the checked witness.
example : run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell, .closeChecked], raises := false }])) = none := by decide

-- but it closes by declaration.
example : (run held (lowerScope 0 (scopeOf
    [{ mutex := 1, guard := 1, invariant := 1,
       body := [.openInv, .write cell, .closeInv], raises := false }]))).isSome = true := by
  decide

-- One child task: it takes the generation itself, writes, releases, and is
-- joined; the creator then reads under its own section.
example : (run custodied
    [.createMutex 0 1 [cell] 1, .spawn 0 1,
     .lock 1 1 1, .openInvariant 1 1, .access 1 cell .write (some 1),
     .closeInvariant 1 1 1, .unlock 1 1,
     .join 0 1,
     .lock 0 1 2, .access 0 cell .read (some 2), .unlock 0 2,
     .destroyMutex 0 1, .destroy 0 1]).isSome = true := by decide

-- A child written inside a critical section is refused: its join lands while
-- the creator still holds the guard.
example : run held
    [.createMutex 0 1 [cell] 1, .lock 0 1 1, .spawn 0 1, .join 0 1] = none := by decide

-- and a child that names the creator's guard is refused before that.
example : run held
    [.createMutex 0 1 [cell] 1, .lock 0 1 1, .spawn 0 1,
     .access 1 cell .read (some 1)] = none := by decide

-- An announced acquisition outlives the announcing task's join and blocks the
-- generation's destruction.
example : run custodied
    [.createMutex 0 1 [cell] 1, .spawn 0 1, .registerWaiter 1 1, .join 0 1,
     .destroyMutex 0 1] = none := by decide


/-! ## 12. The opt-in lock-rank discipline (card `CT33-RANKS`)

Ranks are declared, not derived: a `RankMap` says where each generation sits in
an order the *program* chose. Nothing here claims a rank is correct, and a
passing check is not a deadlock-freedom proof — this model has no blocking at
all (`canLock` refuses a held generation rather than parking the acquirer), so
the wait-for graph a real deadlock argument needs is not represented.

What *is* proved is exactly the discipline's own claim. Every acquisition made
while holding another generation contributes an edge `(held, acquired)` to the
relation this model can express, `rankAdmits` refuses every edge that does not
strictly increase in rank, and a relation whose every edge strictly increases a
`Nat` has no cycle — no self-loop, and no path that returns to where it began.

`step`, `run` and every definition above are untouched: the discipline is a
layer over them, matching the Rust checker, where the rank test is the last
precondition of `Lock` and an empty map skips it entirely. -/

/-- A declared rank assignment: generation to position. -/
abbrev RankMap := List (MutexId × Nat)

def rankOf (rs : RankMap) (m : MutexId) : Option Nat :=
  (rs.find? (fun p => p.1 == m)).map Prod.snd

/-- The generations `task` currently holds a live guard over. -/
def heldMutexes (s : State) (task : TaskId) : List MutexId :=
  (s.guards.filter (fun g => g.holder == task)).map GuardRecord.mutex

/-- R1's well-formedness half: every live generation has a declared position.
A module that ranks only part of itself has no order, so the Rust checker
refuses it outright rather than checking the ranked part. -/
def ranksTotal (rs : RankMap) (s : State) : Bool :=
  s.mutexes.all (fun r => (rankOf rs r.id).isSome)

/-- R2. `task` may acquire `mutex` only if every generation it already holds
sits strictly below `mutex`. Both `none` arms refuse: an unranked generation on
either side of the comparison has no position, and an acquisition with no
position is not admitted on the strength of the other side having one. -/
def rankAdmits (rs : RankMap) (s : State) (task : TaskId) (mutex : MutexId) : Bool :=
  match rankOf rs mutex with
  | none => false
  | some r =>
      (heldMutexes s task).all (fun h =>
        match rankOf rs h with
        | none => false
        | some hr => decide (hr < r))

/-- The `(held, acquired)` edges one admitted acquisition contributes. This is
the whole of the would-block relation this model can express. -/
def lockEdges (s : State) (task : TaskId) (mutex : MutexId) : List (MutexId × MutexId) :=
  (heldMutexes s task).map (fun h => (h, mutex))

/-- One step under the discipline, collecting the edges it creates. Only `.lock`
differs from `step`; every other event is `step` with no edge. -/
def stepRanked (rs : RankMap) (s : State) : Event → Option (State × List (MutexId × MutexId))
  | .lock task mutex guard =>
      if canLock s task mutex guard && rankAdmits rs s task mutex then
        some (lockEffect s task mutex guard, lockEdges s task mutex)
      else none
  | e => (step s e).map (fun next => (next, []))

def runRanked (rs : RankMap) (s : State) :
    List Event → Option (State × List (MutexId × MutexId))
  | [] => some (s, [])
  | e :: rest =>
      match stepRanked rs s e with
      | none => none
      | some (next, here) => (runRanked rs next rest).map (fun p => (p.1, here ++ p.2))

/-- An edge strictly increases rank. -/
def rankIncreases (rs : RankMap) (e : MutexId × MutexId) : Bool :=
  match rankOf rs e.1, rankOf rs e.2 with
  | some a, some b => decide (a < b)
  | _, _ => false

/-! ### 12.1 Every admitted acquisition contributes only increasing edges -/

theorem admitted_edges_increase_rank (rs : RankMap) (s : State) (task : TaskId)
    (mutex : MutexId) (admits : rankAdmits rs s task mutex = true) :
    (lockEdges s task mutex).all (rankIncreases rs) = true := by
  simp only [lockEdges, List.all_map]
  simp only [rankAdmits] at admits
  cases hm : rankOf rs mutex with
  | none => rw [hm] at admits; exact absurd admits (by simp)
  | some r =>
      rw [hm] at admits
      rw [List.all_eq_true] at admits ⊢
      intro h hmem
      have := admits h hmem
      simp only [Function.comp_apply, rankIncreases, hm]
      cases hh : rankOf rs h with
      | none => rw [hh] at this; exact absurd this (by simp)
      | some hr => rw [hh] at this; simpa using this

/-- Contrapositive: an acquisition whose edge would not increase rank is
refused. Equal ranks are the case that matters — the order is strict. -/
theorem a_non_increasing_acquisition_is_refused (rs : RankMap) (s : State) (task : TaskId)
    (mutex held : MutexId) (holds : held ∈ heldMutexes s task)
    (flat : rankIncreases rs (held, mutex) = false) :
    rankAdmits rs s task mutex = false := by
  cases hb : rankAdmits rs s task mutex with
  | false => rfl
  | true =>
      have all := admitted_edges_increase_rank rs s task mutex hb
      rw [lockEdges, List.all_map, List.all_eq_true] at all
      have := all held holds
      simp only [Function.comp_apply] at this
      rw [flat] at this
      exact absurd this (by simp)

/-- Equal ranks are never admitted, stated on its own so the strictness of the
order is not left to be read out of `<`. -/
theorem equal_ranks_are_never_nested (rs : RankMap) (s : State) (task : TaskId)
    (mutex held : MutexId) (r : Nat) (holds : held ∈ heldMutexes s task)
    (hheld : rankOf rs held = some r) (hmutex : rankOf rs mutex = some r) :
    rankAdmits rs s task mutex = false := by
  refine a_non_increasing_acquisition_is_refused rs s task mutex held holds ?_
  simp [rankIncreases, hheld, hmutex]

/-- An unranked generation is refused on either side of the comparison: the
discipline fails closed rather than admitting what it cannot place. -/
theorem an_unranked_acquisition_is_refused (rs : RankMap) (s : State) (task : TaskId)
    (mutex : MutexId) (unranked : rankOf rs mutex = none) :
    rankAdmits rs s task mutex = false := by
  simp [rankAdmits, unranked]

theorem an_unranked_held_generation_refuses_every_acquisition (rs : RankMap) (s : State)
    (task : TaskId) (mutex held : MutexId) (holds : held ∈ heldMutexes s task)
    (unranked : rankOf rs held = none) :
    rankAdmits rs s task mutex = false := by
  refine a_non_increasing_acquisition_is_refused rs s task mutex held holds ?_
  simp [rankIncreases, unranked]

/-! ### 12.2 A whole run collects only increasing edges -/

theorem runRanked_edges_increase_rank (rs : RankMap) :
    ∀ (events : List Event) (s : State) (t : State) (edges : List (MutexId × MutexId)),
      runRanked rs s events = some (t, edges) → edges.all (rankIncreases rs) = true := by
  intro events
  induction events with
  | nil =>
      intro s t edges h
      simp only [runRanked, Option.some.injEq, Prod.mk.injEq] at h
      rw [← h.2]
      rfl
  | cons e rest ih =>
      intro s t edges h
      simp only [runRanked] at h
      cases hstep : stepRanked rs s e with
      | none => rw [hstep] at h; exact absurd h (by simp)
      | some p =>
          rw [hstep] at h
          simp only [Option.map_eq_some_iff] at h
          obtain ⟨q, hq, hedges⟩ := h
          have tail := ih p.1 q.1 q.2 hq
          have head : p.2.all (rankIncreases rs) = true := by
            cases e with
            | lock task mutex guard =>
                simp only [stepRanked] at hstep
                by_cases hc : canLock s task mutex guard && rankAdmits rs s task mutex
                · rw [ite_eq_left hc] at hstep
                  have admits : rankAdmits rs s task mutex = true :=
                    (Bool.and_eq_true _ _ |>.mp hc).2
                  have : p.2 = lockEdges s task mutex := by
                    simp only [Option.some.injEq] at hstep; rw [← hstep]
                  rw [this]
                  exact admitted_edges_increase_rank rs s task mutex admits
                · rw [ite_eq_right hc] at hstep; exact absurd hstep (by simp)
            | _ =>
                -- Every other event is `step` unchanged and contributes no
                -- edge, so there is nothing to check.
                simp only [stepRanked, Option.map_eq_some_iff] at hstep
                obtain ⟨_, _, hp⟩ := hstep
                have empty : p.2 = [] := by rw [← hp]
                rw [empty]
                rfl
          have : edges = p.2 ++ q.2 := by
            have := congrArg Prod.snd hedges
            simpa using this.symm
          rw [this, List.all_append, head, tail]
          rfl

/-! ### 12.3 Increasing edges admit no cycle

A path is a list of generations whose consecutive pairs are edges. If every
edge strictly increases rank then every path's rank strictly increases, so a
path of length two or more can never return to where it began. That is exactly
"no cycle in the would-block relation the model can express". -/

/-- `path` walks `edges`: every consecutive pair is one of them. -/
def isPath (edges : List (MutexId × MutexId)) : List MutexId → Bool
  | [] => true
  | [_] => true
  | a :: b :: rest => edges.contains (a, b) && isPath edges (b :: rest)

/-- Rank, defaulted so the statement is total; every generation on an admitted
path is ranked, by `admitted_edges_increase_rank`. -/
def rankAt (rs : RankMap) (m : MutexId) : Nat := (rankOf rs m).getD 0

private theorem increasing_edge_ranks (rs : RankMap) (edges : List (MutexId × MutexId))
    (inc : edges.all (rankIncreases rs) = true) (a b : MutexId)
    (mem : edges.contains (a, b) = true) : rankAt rs a < rankAt rs b := by
  rw [List.all_eq_true] at inc
  have : rankIncreases rs (a, b) = true := inc (a, b) (by simpa using mem)
  simp only [rankIncreases] at this
  cases ha : rankOf rs a with
  | none => rw [ha] at this; exact absurd this (by simp)
  | some x =>
      cases hb : rankOf rs b with
      | none => rw [ha, hb] at this; exact absurd this (by simp)
      | some y =>
          rw [ha, hb] at this
          simp only [rankAt, ha, hb, Option.getD_some]
          simpa using this

/-- Along any path through increasing edges, rank strictly increases from the
head to every later element. -/
theorem path_rank_strictly_increases (rs : RankMap) (edges : List (MutexId × MutexId))
    (inc : edges.all (rankIncreases rs) = true) :
    ∀ (a : MutexId) (rest : List MutexId), isPath edges (a :: rest) = true →
      ∀ b ∈ rest, rankAt rs a < rankAt rs b := by
  intro a rest
  induction rest generalizing a with
  | nil => intro _ b hb; exact absurd hb (by simp)
  | cons c tail ih =>
      intro hpath b hb
      simp only [isPath, Bool.and_eq_true] at hpath
      have edge : rankAt rs a < rankAt rs c :=
        increasing_edge_ranks rs edges inc a c hpath.1
      rcases List.mem_cons.mp hb with rfl | hlater
      · exact edge
      · exact Nat.lt_trans edge (ih c hpath.2 b hlater)

/-- **The discipline's soundness claim.** A rank-respecting run admits no cycle
in the would-block relation it can express: no path through the edges it
collected returns to where it began. -/
theorem a_rank_respecting_run_admits_no_cycle (rs : RankMap) (events : List Event)
    (s t : State) (edges : List (MutexId × MutexId))
    (ran : runRanked rs s events = some (t, edges)) (a : MutexId) (rest : List MutexId)
    (nonempty : rest ≠ []) (walks : isPath edges (a :: rest) = true) :
    rest.getLast nonempty ≠ a := by
  have inc := runRanked_edges_increase_rank rs events s t edges ran
  have := path_rank_strictly_increases rs edges inc a rest walks
    (rest.getLast nonempty) (List.getLast_mem nonempty)
  intro hcycle
  rw [hcycle] at this
  exact absurd this (Nat.lt_irrefl _)

/-- The self-loop case, stated separately because it is the one a reader checks
first: a generation is never acquired while it is itself held. -/
theorem no_generation_is_acquired_while_held (rs : RankMap) (s : State) (task : TaskId)
    (mutex : MutexId) (holds : mutex ∈ heldMutexes s task) :
    rankAdmits rs s task mutex = false := by
  refine a_non_increasing_acquisition_is_refused rs s task mutex mutex holds ?_
  simp only [rankIncreases]
  cases h : rankOf rs mutex with
  | none => simp
  | some r => simp

/-! ### 12.4 The discipline only ever refuses

Everything the ranked layer admits, the unranked model admits too, and every
non-`lock` event is `step` unchanged. So turning ranks on cannot make a refused
trace check, and cannot change what a checking trace does. -/

theorem stepRanked_agrees_with_step_off_lock (rs : RankMap) (s : State) (e : Event)
    (notLock : ∀ task mutex guard, e ≠ .lock task mutex guard) :
    stepRanked rs s e = (step s e).map (fun next => (next, [])) := by
  cases e with
  | lock task mutex guard => exact absurd rfl (notLock task mutex guard)
  | _ => rfl

theorem stepRanked_refines_step (rs : RankMap) (s : State) (e : Event) (next : State)
    (edges : List (MutexId × MutexId)) (ranked : stepRanked rs s e = some (next, edges)) :
    step s e = some next := by
  cases e with
  | lock task mutex guard =>
      simp only [stepRanked] at ranked
      by_cases hc : canLock s task mutex guard && rankAdmits rs s task mutex
      · rw [ite_eq_left hc] at ranked
        simp only [Option.some.injEq, Prod.mk.injEq] at ranked
        simp only [step, ite_eq_left (Bool.and_eq_true _ _ |>.mp hc).1, Option.some.injEq]
        exact ranked.1
      · rw [ite_eq_right hc] at ranked; exact absurd ranked (by simp)
  | _ =>
      simp only [stepRanked, Option.map_eq_some_iff] at ranked
      obtain ⟨w, hw, hp⟩ := ranked
      rw [hw]
      simp only [Option.some.injEq]
      exact (Prod.mk.injEq .. ▸ hp).1.symm ▸ rfl

theorem runRanked_refines_run (rs : RankMap) :
    ∀ (events : List Event) (s t : State) (edges : List (MutexId × MutexId)),
      runRanked rs s events = some (t, edges) → run s events = some t := by
  intro events
  induction events with
  | nil =>
      intro s t edges h
      simp only [runRanked, Option.some.injEq, Prod.mk.injEq] at h
      simp [run, h.1]
  | cons e rest ih =>
      intro s t edges h
      simp only [runRanked] at h
      cases hstep : stepRanked rs s e with
      | none => rw [hstep] at h; exact absurd h (by simp)
      | some p =>
          rw [hstep] at h
          simp only [Option.map_eq_some_iff] at h
          obtain ⟨q, hq, hedges⟩ := h
          have head := stepRanked_refines_step rs s e p.1 p.2 (by rw [hstep])
          have tail := ih p.1 q.1 q.2 hq
          have same : t = q.1 := by
            have := congrArg Prod.fst hedges
            simpa using this.symm
          rw [same]
          simp only [run, head, Option.bind_some, tail]

/-! ### 12.5 Decidable scenarios

The shapes the Rust tests drive, decided here over the same rules. -/

private def twoCells : State :=
  { holdings := [{ custodian := .task 0, footprint := ⟨1, 0, 8⟩, share := fullAccess },
                 { custodian := .task 0, footprint := ⟨2, 0, 8⟩, share := fullAccess }] }

private def ranks : RankMap := [(1, 1), (2, 2)]

private def flatRanks : RankMap := [(1, 4), (2, 4)]

-- Nesting the higher rank inside the lower one is admitted, and its one edge
-- increases.
example : (runRanked ranks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 1 10, .lock 0 2 11, .unlock 0 11, .unlock 0 10]).isSome = true := by decide

example : (runRanked ranks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 1 10, .lock 0 2 11, .unlock 0 11, .unlock 0 10]).map Prod.snd
    = some [(1, 2)] := by decide

-- The reverse nesting is the CT33 shape, and is refused.
example : runRanked ranks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 2 10, .lock 0 1 11] = none := by decide

-- Equal ranks are refused in both directions.
example : runRanked flatRanks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 1 10, .lock 0 2 11] = none := by decide

example : runRanked flatRanks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 2 10, .lock 0 1 11] = none := by decide

-- A released guard imposes no order on the next acquisition.
example : (runRanked ranks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 2 10, .unlock 0 10, .lock 0 1 11, .unlock 0 11]).isSome = true := by decide

-- R1: a module that ranks only part of itself places nothing, so every
-- acquisition of the unplaced generation is refused.
example : ranksTotal [(1, 1)] { twoCells with
    mutexes := [{ id := 1, protects := [⟨1, 0, 8⟩], invariant := 1, invariants := [1],
                  creator := 0,
                  holder := none, waiters := [], releasedBy := [], observed := [] },
                { id := 2, protects := [⟨2, 0, 8⟩], invariant := 2, invariants := [2],
                  creator := 0,
                  holder := none, waiters := [], releasedBy := [], observed := [] }] }
    = false := by decide

example : runRanked [(1, 1)] twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .lock 0 2 10] = none := by decide

-- A three-generation chain checks, and the closing edge is refused.
private def threeRanks : RankMap := [(1, 1), (2, 2), (3, 3)]

private def threeCells : State :=
  { holdings := [{ custodian := .task 0, footprint := ⟨1, 0, 8⟩, share := fullAccess },
                 { custodian := .task 0, footprint := ⟨2, 0, 8⟩, share := fullAccess },
                 { custodian := .task 0, footprint := ⟨3, 0, 8⟩, share := fullAccess }] }

example : (runRanked threeRanks threeCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .createMutex 0 3 [⟨3, 0, 8⟩] 3,
     .lock 0 1 10, .lock 0 2 11, .lock 0 3 12,
     .unlock 0 12, .unlock 0 11, .unlock 0 10]).isSome = true := by decide

example : runRanked threeRanks threeCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .createMutex 0 2 [⟨2, 0, 8⟩] 2,
     .createMutex 0 3 [⟨3, 0, 8⟩] 3,
     .lock 0 3 10, .lock 0 1 11] = none := by decide

-- The unsupported rank construct still refuses under the discipline.
example : runRanked ranks twoCells
    [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .unsupportedConstruct 0] = none := by decide

/-! ## 16. The opt-in protection growth (card `CT34-GROWTH`)

`extendProtection` is, and stays, the unconditional refusal proved above
(`protected_shape_is_fixed`). Beside it this section models one *admitted*
growth as a separate layer over an unedited `step`, exactly as §15's guard-local
join and §12's rank discipline are layered: a trace that never grows is
untouched, and nothing here relaxes an existing rule.

The soundness argument is the ledger's own. Growth performs `createMutexEffect`'s
move on a further interval — the grower's `fullAccess` holding is re-custodied to
the mutex — so `authorityTotal` is preserved for exactly the same reason. The
other conditions keep the invariants a *mid-trace* shape change could disturb:
the grower holds the generation's own live section with its invariant open, the
interval has no protector, the grower is the creator holding full authority over
it, and the declared predicate identity moves to one this generation has never
carried. -/

/-- The growth event, as its own guarded transition. The generation is read off
`guard`, never named separately, so a growth can only reach a mutex whose live
section the acting task holds (W1). -/
def canGrowProtection (s : State) (task : TaskId) (guard : GuardId) (f : Footprint)
    (invariant : InvariantId) : Bool :=
  s.act task && f.valid && !s.destroyedResources.contains f.resource &&
    s.guardSat guard (fun g =>
      -- W1: the acting task's own live section, and W2: with the invariant open
      g.holder == task && g.phase == .opened &&
        s.mutexSat g.mutex (fun m =>
          -- W5: destruction returns parked share to the creator
          m.creator == task &&
            -- W3: a wider region is a different proposition
            !m.invariants.contains invariant)) &&
    -- W4: over memory no live generation protects, from both sides
    (s.protectorOf f).isNone && !s.parkedOver f &&
    -- W5: parking is all-or-nothing, exactly as at creation
    s.holdsFull task f

/-- The effect: the interval joins the generation's declared shape, the
generation's declared predicate identity moves, the grower's holding is
re-custodied to the mutex, and the section is marked as having changed the
protected region — so the *checked* `NoWriteOccurred` witness, which asserts the
invariant is unchanged, becomes unavailable to it. -/
def growProtectionEffect (s : State) (task : TaskId) (guard : GuardId) (f : Footprint)
    (invariant : InvariantId) (mutex : MutexId) : State :=
  { s with
    mutexes := s.mutexes.map (fun r =>
      if r.id == mutex then
        { r with protects := r.protects ++ [f], invariant := invariant
                 invariants := invariant :: r.invariants }
      else r)
    guards := s.guards.map (fun r =>
      if r.id == guard then { r with wroteWhileOpen := true } else r)
    holdings := s.holdings.map (fun h =>
      if h.custodian == .task task && h.footprint == f then
        { h with custodian := .mutex mutex } else h) }

/-- One growth step. `none` is a refusal, and a refusal produces no state at
all, so nothing about the generation can move on a refused growth. -/
def stepGrowProtection (s : State) (task : TaskId) (guard : GuardId) (f : Footprint)
    (invariant : InvariantId) : Option State :=
  if canGrowProtection s task guard f invariant then
    match s.guardOf guard with
    | none => none
    | some g => some (growProtectionEffect s task guard f invariant g.mutex)
  else none

/-! ### 16.1 Conservation and the single protector -/

/-- The conserved quantity is untouched: growth re-custodies a holding, it does
not create or destroy share. Same proof as creation's, for the same reason. -/
theorem growProtectionEffect_preserves_authority (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId) (mutex : MutexId) :
    (growProtectionEffect s task guard f invariant mutex).authorityTotal = s.authorityTotal := by
  unfold growProtectionEffect State.authorityTotal
  exact map_preserves_share_sum _ _ (by intro h; split <;> rfl)

/-- The same statement over the whole guarded step, so an admitted growth
anywhere in a trace conserves authority. -/
theorem grow_step_preserves_authority {s next : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId}
    (accepted : stepGrowProtection s task guard f invariant = some next) :
    next.authorityTotal = s.authorityTotal := by
  unfold stepGrowProtection at accepted
  split at accepted
  · split at accepted
    · contradiction
    · cases Option.some.inj accepted
      exact growProtectionEffect_preserves_authority ..
  · contradiction

/-- W4, the single-protector half: an admitted growth's interval had no
protector at all, so the growth cannot give a footprint a second one. -/
theorem admitted_growth_had_no_protector {s : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId}
    (admitted : canGrowProtection s task guard f invariant = true) :
    (s.protectorOf f).isNone = true ∧ s.parkedOver f = false := by
  unfold canGrowProtection at admitted
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at admitted
  exact ⟨admitted.1.1.2, admitted.1.2⟩

/-- The record a growth installs in place of `r`. -/
private def grown (f : Footprint) (invariant : InvariantId) (r : MutexRecord) : MutexRecord :=
  { r with protects := r.protects ++ [f], invariant := invariant
           invariants := invariant :: r.invariants }

/-- Growth touches exactly one generation's declared shape: every other record
survives it verbatim. -/
theorem grow_preserves_every_other_declared_shape (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId) (mutex : MutexId) (r : MutexRecord)
    (other : r ∈ s.mutexes) (distinct : ¬ r.id = mutex) :
    r ∈ (growProtectionEffect s task guard f invariant mutex).mutexes := by
  have rewritten : (if r.id == mutex then grown f invariant r else r) = r := by
    simp [distinct]
  have := List.mem_map_of_mem (f := fun x : MutexRecord =>
    if x.id == mutex then grown f invariant x else x) other
  rw [rewritten] at this
  simpa [growProtectionEffect, grown] using this

/-- Growth is permanent and additive: the declared intervals survive it with the
new one appended, and the declared predicate identity moves with the shape,
recorded in the generation's history. There is no shrink, no replacement and no
reordering. -/
theorem grow_appends_the_interval_and_moves_the_predicate (s : State) (task : TaskId)
    (guard : GuardId) (f : Footprint) (invariant : InvariantId) (r : MutexRecord)
    (present : r ∈ s.mutexes) :
    grown f invariant r ∈ (growProtectionEffect s task guard f invariant r.id).mutexes ∧
      (grown f invariant r).protects = r.protects ++ [f] ∧
      (grown f invariant r).invariant = invariant ∧
      (grown f invariant r).invariants = invariant :: r.invariants := by
  refine ⟨?_, rfl, rfl, rfl⟩
  have := List.mem_map_of_mem (f := fun x : MutexRecord =>
    if x.id == r.id then grown f invariant x else x) present
  simpa [growProtectionEffect, grown] using this

/-! ### 16.2 The refusals: everything outside the rule -/

theorem grow_refuses_a_dead_guard (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId) (dead : s.guardOf guard = none) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, State.guardSat, dead]

theorem grow_requires_the_holder {s : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId} {g : GuardRecord}
    (live : s.guardOf guard = some g) (foreign : g.holder ≠ task) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, State.guardSat, live, foreign]

/-- W2. A closed invariant asserts the declared predicate of the protected
region, so widening the region under one would assert it of memory nothing
established it for. -/
theorem grow_requires_an_open_invariant {s : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId} {g : GuardRecord}
    (live : s.guardOf guard = some g) (shut : g.phase = .closed) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, State.guardSat, live, shut]

/-- W5. Destruction returns every parked share to the creator, so a grower who
is not the creator would park authority that arrives at a task which never gave
it up. -/
theorem grow_requires_the_creator {s : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId} {g : GuardRecord} {m : MutexRecord}
    (live : s.guardOf guard = some g) (generation : s.mutexOf g.mutex = some m)
    (foreign : m.creator ≠ task) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, State.guardSat, State.mutexSat, live, generation, foreign]

/-- W3. An identity this generation already carried is refused, so a growth can
never re-use a declaration made for a narrower region. -/
theorem grow_requires_an_unused_predicate_identity {s : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId} {g : GuardRecord} {m : MutexRecord}
    (live : s.guardOf guard = some g) (generation : s.mutexOf g.mutex = some m)
    (carried : invariant ∈ m.invariants) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, State.guardSat, State.mutexSat, live, generation, carried]

/-- W4. An interval a live generation already protects is refused — this
generation's own declared or already grown intervals included. -/
theorem grow_refuses_an_already_protected_interval (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId) (taken : (s.protectorOf f).isNone = false) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, taken]

theorem grow_refuses_a_parked_interval (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId) (parked : s.parkedOver f = true) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, parked]

/-- W5. Parking is all-or-nothing: a partial share over the new interval is
refused, so no growth can park authority the grower did not wholly hold. -/
theorem grow_requires_full_authority (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId) (partial_share : s.holdsFull task f = false) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, partial_share]

theorem grow_refuses_a_destroyed_resource (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId)
    (dead : f.resource ∈ s.destroyedResources) :
    canGrowProtection s task guard f invariant = false := by
  simp [canGrowProtection, dead]

/-- A refused growth produces no state, so nothing about the generation, the
section or the ledger can move on one. -/
theorem refused_growth_produces_no_state (s : State) (task : TaskId) (guard : GuardId)
    (f : Footprint) (invariant : InvariantId)
    (refused : canGrowProtection s task guard f invariant = false) :
    stepGrowProtection s task guard f invariant = none := by
  simp [stepGrowProtection, refused]

/-! ### 16.3 What an admitted growth forces afterwards -/

/-- Looking a guard up after a map that preserves identities finds the mapped
record. The growth only ever rewrites the section it grew under, so this is what
lets its post-state be read off the pre-state's guard. -/
private theorem guardOf_after_id_preserving_map (l : List GuardRecord)
    (m : GuardRecord → GuardRecord) (keeps : ∀ r, (m r).id = r.id) (guard : GuardId)
    (r : GuardRecord) (found : l.find? (fun x => x.id == guard) = some r) :
    (l.map m).find? (fun x => x.id == guard) = some (m r) := by
  induction l with
  | nil => simp at found
  | cons h t ih =>
    have head : ((m h).id == guard) = (h.id == guard) := by rw [keeps h]
    cases hb : h.id == guard with
    | true =>
      simp only [List.find?_cons, hb] at found
      have same : h = r := Option.some.inj found
      subst same
      simp only [List.map_cons, List.find?_cons, head, hb]
    | false =>
      simp only [List.find?_cons, hb] at found
      simp only [List.map_cons, List.find?_cons, head, hb]
      exact ih found

/-- A guard the lookup found does carry the identity it was looked up by. -/
private theorem found_guard_matches (l : List GuardRecord) (guard : GuardId) (r : GuardRecord)
    (found : l.find? (fun x => x.id == guard) = some r) : (r.id == guard) = true := by
  induction l with
  | nil => simp at found
  | cons h t ih =>
    cases hb : h.id == guard with
    | true =>
      simp only [List.find?_cons, hb] at found
      have same : h = r := Option.some.inj found
      exact same ▸ hb
    | false =>
      simp only [List.find?_cons, hb] at found
      exact ih found

/-- The section a growth ran under keeps its identity, its holder and its phase,
and is marked as having changed the protected region. -/
theorem grow_marks_the_section_it_ran_under {s : State} {task : TaskId} {guard : GuardId}
    {f : Footprint} {invariant : InvariantId} {mutex : MutexId} {g : GuardRecord}
    (live : s.guardOf guard = some g) :
    (growProtectionEffect s task guard f invariant mutex).guardOf guard =
      some { g with wroteWhileOpen := true } := by
  have matched : (g.id == guard) = true := found_guard_matches s.guards guard g live
  have mapped := guardOf_after_id_preserving_map s.guards
    (fun r => if r.id == guard then { r with wroteWhileOpen := true } else r)
    (by intro r; split <;> rfl) guard g live
  have unfolded : (growProtectionEffect s task guard f invariant mutex).guardOf guard =
      (s.guards.map fun r => if r.id == guard then { r with wroteWhileOpen := true } else r).find?
        (fun x => x.id == guard) := rfl
  rw [unfolded, mapped]
  simp [matched]

/-- W3, from the witness side. Growth changes what the protected region *is*, so
the checked `NoWriteOccurred` witness — "the invariant is unchanged" — is
unavailable to the section that grew. It must close by declaration. -/
theorem a_grown_section_cannot_close_with_the_checked_witness (s : State) (task : TaskId)
    (guard : GuardId) (f : Footprint) (invariant : InvariantId) (mutex : MutexId)
    (g : GuardRecord) (live : s.guardOf guard = some g) :
    canCloseUnwritten (growProtectionEffect s task guard f invariant mutex) task guard = false := by
  simp [canCloseUnwritten, State.guardSat, grow_marks_the_section_it_ran_under live]

/-- The section stays open after a growth, so it cannot be released until an
explicit close has restored the new declared invariant. Growth cannot escape a
critical section without one. -/
theorem a_grown_section_cannot_be_released_before_closing (s : State) (task : TaskId)
    (guard : GuardId) (f : Footprint) (invariant : InvariantId) (mutex : MutexId)
    (g : GuardRecord) (live : s.guardOf guard = some g) (open_now : g.phase = .opened) :
    canRelease (growProtectionEffect s task guard f invariant mutex) task guard = false := by
  simp [canRelease, State.guardSat, grow_marks_the_section_it_ran_under live, open_now]

/-! ### 16.4 Decidable scenarios

One accepted control and its refusals, each a single mutation. `growable` holds
both cells outright and protects only the first; `grown` is the state after
locking and opening the first generation's section. -/

private def growable : State :=
  { holdings := [{ custodian := .task 0, footprint := ⟨1, 0, 8⟩, share := fullAccess },
                 { custodian := .task 0, footprint := ⟨2, 0, 8⟩, share := fullAccess }] }

private def afterOpen : State :=
  match run growable [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .lock 0 1 10, .openInvariant 0 10] with
  | some s => s
  | none => growable

private def afterLock : State :=
  match run growable [.createMutex 0 1 [⟨1, 0, 8⟩] 1, .lock 0 1 10] with
  | some s => s
  | none => growable

-- The control: the creator grows its own open section over the unprotected
-- cell, and it is admitted.
example : (stepGrowProtection afterOpen 0 10 ⟨2, 0, 8⟩ 2).isSome = true := by decide

-- W2: the same growth with the invariant closed is refused.
example : stepGrowProtection afterLock 0 10 ⟨2, 0, 8⟩ 2 = none := by decide

-- W4: the generation's own declared interval is already protected.
example : stepGrowProtection afterOpen 0 10 ⟨1, 0, 8⟩ 2 = none := by decide

-- W3: the identity this generation was created with is refused.
example : stepGrowProtection afterOpen 0 10 ⟨2, 0, 8⟩ 1 = none := by decide

-- W5: a task that is not the creator, and does not hold the section, is
-- refused; so is a growth over an interval the grower does not wholly hold.
example : stepGrowProtection afterOpen 1 10 ⟨2, 0, 8⟩ 2 = none := by decide
example : stepGrowProtection afterOpen 0 10 ⟨3, 0, 8⟩ 2 = none := by decide

-- W1: a guard that was never minted is refused.
example : stepGrowProtection afterOpen 0 11 ⟨2, 0, 8⟩ 2 = none := by decide

-- An admitted growth conserves the total authority it moved.
example : (stepGrowProtection afterOpen 0 10 ⟨2, 0, 8⟩ 2).map State.authorityTotal
    = some afterOpen.authorityTotal := by decide

-- And it leaves the grown interval reachable only through the generation: the
-- unguarded write that was admitted before the growth is refused after it.
example : (run afterOpen [.access 0 ⟨2, 0, 8⟩ .write none]).isSome = true := by decide
example : (stepGrowProtection afterOpen 0 10 ⟨2, 0, 8⟩ 2).bind
    (fun s => run s [.access 0 ⟨2, 0, 8⟩ .write none]) = none := by decide

-- The unconditional fixed-shape proposal still refuses in the same state.
example : step afterOpen (.extendProtection 0 1 ⟨2, 0, 8⟩) = none := by decide

end QCore.Concurrency.MutexInvariant
