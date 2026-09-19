/-!
# Bounded release/acquire publication

The rule is conditional on one publisher invocation and one successful consumer
observation. It proves ordering, not termination or eventual observation.
Faithful Clang extraction and Lean-to-Rust correspondence remain stated review
premises rather than extraction theorems.
-/

namespace QCore.AtomicPublication

inductive FlagKind where | plain | volatile | atomic
deriving DecidableEq, Repr

inductive StoreOrder where | relaxed | release | seqCst
deriving DecidableEq, Repr

inductive LoadOrder where | relaxed | acquire | seqCst
deriving DecidableEq, Repr

structure Bounds where
  publisherInvocations : Nat
  successfulConsumers : Nat
deriving DecidableEq, Repr

structure Program where
  flagKind : FlagKind
  initial : Int
  published : Int
  observed : Int
  storeOrder : StoreOrder
  loadOrder : LoadOrder
  bounds : Bounds
deriving DecidableEq, Repr

def releases : StoreOrder → Prop
  | .release | .seqCst => True
  | .relaxed => False

def acquires : LoadOrder → Prop
  | .acquire | .seqCst => True
  | .relaxed => False

def identifiesRelease (p : Program) : Prop :=
  p.initial ≠ p.published ∧ p.observed = p.published

def oneShot (p : Program) : Prop :=
  p.bounds.publisherInvocations = 1 ∧ p.bounds.successfulConsumers = 1

def synchronizesWith (p : Program) : Prop :=
  p.flagKind = .atomic ∧ identifiesRelease p ∧
    releases p.storeOrder ∧ acquires p.loadOrder ∧ oneShot p

inductive Event where
  | payloadWrite | releaseStore | acquireLoad | payloadRead
deriving DecidableEq, Repr

inductive PrimitiveEdge (p : Program) : Event → Event → Prop
  | producer : PrimitiveEdge p .payloadWrite .releaseStore
  | synchronization (h : synchronizesWith p) : PrimitiveEdge p .releaseStore .acquireLoad
  | consumer : PrimitiveEdge p .acquireLoad .payloadRead

inductive HappensBefore (p : Program) : Event → Event → Prop
  | step (h : PrimitiveEdge p a b) : HappensBefore p a b
  | trans (left : HappensBefore p a b) (right : HappensBefore p b c) : HappensBefore p a c

theorem synchronized_observation_orders_payload (p : Program)
    (h : synchronizesWith p) : HappensBefore p .payloadWrite .payloadRead := by
  exact .trans (.step .producer) (.trans (.step (.synchronization h)) (.step .consumer))

theorem volatile_has_no_synchronizes_with_edge (p : Program)
    (h : p.flagKind = .volatile) : ¬ synchronizesWith p := by
  intro sync
  exact FlagKind.noConfusion (h.symm.trans sync.1)

theorem plain_has_no_synchronizes_with_edge (p : Program)
    (h : p.flagKind = .plain) : ¬ synchronizesWith p := by
  intro sync
  exact FlagKind.noConfusion (h.symm.trans sync.1)

theorem relaxed_store_has_no_synchronizes_with_edge (p : Program)
    (h : p.storeOrder = .relaxed) : ¬ synchronizesWith p := by
  simp [synchronizesWith, releases, h]

theorem relaxed_load_has_no_synchronizes_with_edge (p : Program)
    (h : p.loadOrder = .relaxed) : ¬ synchronizesWith p := by
  simp [synchronizesWith, acquires, h]

theorem repeated_publisher_has_no_synchronizes_with_edge (p : Program)
    (h : p.bounds.publisherInvocations ≠ 1) : ¬ synchronizesWith p := by
  intro sync
  exact h sync.2.2.2.2.1

def releaseAcquireFixture : Program where
  flagKind := .atomic
  initial := 0
  published := 1
  observed := 1
  storeOrder := .release
  loadOrder := .acquire
  bounds := ⟨1, 1⟩

theorem fixture_has_a_real_three_edge_path :
    synchronizesWith releaseAcquireFixture ∧
      HappensBefore releaseAcquireFixture .payloadWrite .payloadRead := by
  have sync : synchronizesWith releaseAcquireFixture := by
    simp [synchronizesWith, identifiesRelease, releases, acquires, oneShot,
      releaseAcquireFixture]
  constructor
  · exact sync
  · exact synchronized_observation_orders_payload _ sync

theorem equal_initial_and_ready_do_not_identify_the_release :
    ¬ identifiesRelease { releaseAcquireFixture with initial := 1 } := by
  simp [identifiesRelease, releaseAcquireFixture]

end QCore.AtomicPublication
