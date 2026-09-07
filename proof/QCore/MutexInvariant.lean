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
  | access (task : TaskId) (footprint : Footprint) (kind : AccessKind)
      (guard : Option GuardId)
  | borrowUnderGuard (task : TaskId) (guard : GuardId) (footprint : Footprint)
      (borrow : BorrowId)
  | endBorrow (task : TaskId) (borrow : BorrowId)
  | assertFact (task : TaskId) (guard : GuardId) (footprint : Footprint) (fact : FactId)
  | useFact (task : TaskId) (fact : FactId)
  | transferCustody (sender receiver : TaskId) (resource : ResourceId) (via : Witness)
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
      step s (.access task f kind (some guard)) = none ∧
      step s (.borrowUnderGuard task guard f borrow) = none ∧
      step s (.assertFact task guard f fact) = none := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;>
    simp [step, canRelease, canOpen, canClose, canAccessGuarded, canBorrow,
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

/-- CT34: the protected shape is fixed at creation. No state enables this; it is
a refusal, not a case the model happens not to cover. -/
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

end QCore.Concurrency.MutexInvariant
