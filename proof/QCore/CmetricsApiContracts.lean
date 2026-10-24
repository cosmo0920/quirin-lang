import QCore.TrackedSelfInit

/-! Declared CMetrics API lifecycle models. Source extraction, representation,
SDS allocation/reallocation, synchronization and Rust/model correspondence are
premises. These definitions confer no resource-v1 call permission. -/
namespace QCore.CmetricsApiContracts

structure Opts where
  storageLive : Bool := true
  stored : List Nat := []
  owned : List Nat := []
  deriving DecidableEq, Repr

/-- Failure preserves committed handles; the caller must still clean them up. -/
def partialInit (committed : List Nat) : Opts :=
  { stored := committed, owned := committed }

def canExit (o : Opts) : Bool :=
  o.storageLive && decide o.stored.Nodup &&
    o.stored.all (fun id => o.owned.contains id)

def cleanup (o : Opts) : Option Opts :=
  if canExit o then some { o with owned := [] } else none

theorem failure_keeps_committed_obligations (ids : List Nat) (id : Nat)
    (h : id ∈ ids) : id ∈ (partialInit ids).owned := by
  exact h

theorem cleanup_discharges_contents (o out : Opts) (h : cleanup o = some out) :
    out.owned = [] := by
  unfold cleanup at h
  split at h <;> simp_all

theorem cleanup_preserves_enclosing_storage (o out : Opts)
    (h : cleanup o = some out) : out.storageLive = o.storageLive := by
  unfold cleanup at h
  split at h <;> simp_all

theorem cleanup_does_not_clear_pointer_fields (o out : Opts)
    (h : cleanup o = some out) : out.stored = o.stored := by
  unfold cleanup at h
  split at h <;> simp_all

theorem a_nonempty_cleanup_is_not_generally_idempotent (o : Opts)
    (nonempty : o.stored ≠ []) : (cleanup o).bind cleanup = none := by
  cases fields : o.stored with
  | nil => exact False.elim (nonempty fields)
  | cons id rest =>
    by_cases h : canExit o = true
    · simp [cleanup, h, canExit, fields]
    · simp [cleanup, h]

structure MapState where
  live : Bool := true
  epoch : Nat := 0
  primary : List Nat := []
  secondary : List Nat := []
  cached : Option Nat := none
  deriving DecidableEq, Repr

structure Borrow where
  epoch : Nat
  child : Option Nat
  deriving DecidableEq, Repr

/-- none denotes the embedded record; some denotes a primary-owned child. -/
def borrowValid (m : MapState) (b : Borrow) : Bool :=
  m.live && (b.epoch == m.epoch) &&
    (match b.child with | none => true | some id => m.primary.contains id)

def attach (m : MapState) (id : Nat) : MapState :=
  { m with primary := id :: m.primary }

/-- Storage preparation can fail after a newly created metric was attached. -/
def failedGetAfterAttach (m : MapState) (id : Nat) : MapState × Option Borrow :=
  (attach m id, none)

def destroy (m : MapState) : MapState :=
  { m with live := false, epoch := m.epoch + 1,
           primary := [], secondary := [], cached := none }

/-- Conservative abstract invalidation: retirement ends all older borrow epochs. -/
def expire (m : MapState) (retired : List Nat) : MapState :=
  { m with epoch := m.epoch + 1,
    primary := m.primary.filter (fun id => !retired.contains id),
    secondary := m.secondary.filter (fun id => !retired.contains id),
    cached := m.cached.filter (fun id => !retired.contains id) }

theorem getter_failure_can_keep_new_map_owned_metric (m : MapState) (id : Nat) :
    id ∈ (failedGetAfterAttach m id).1.primary ∧
      (failedGetAfterAttach m id).2 = none := by
  simp [failedGetAfterAttach, attach]

theorem attaching_does_not_retire_the_parent (m : MapState) (id : Nat) :
    (attach m id).live = m.live := by rfl

theorem attaching_preserves_old_primary_members (m : MapState) (id old : Nat)
    (h : old ∈ m.primary) : old ∈ (attach m id).primary := by
  simp [attach, h]

theorem embedded_result_needs_live_map (m : MapState) (e : Nat)
    (h : m.live = false) : borrowValid m ⟨e, none⟩ = false := by
  simp [borrowValid, h]

theorem destroy_invalidates_every_metric_borrow (m : MapState) (b : Borrow) :
    borrowValid (destroy m) b = false := by
  simp [borrowValid, destroy]

theorem destroy_discharges_primary_ownership (m : MapState) :
    (destroy m).primary = [] ∧ (destroy m).secondary = [] ∧
      (destroy m).cached = none := by
  simp [destroy]

theorem expiration_preserves_the_enclosing_map (m : MapState) (ids : List Nat) :
    (expire m ids).live = m.live := by rfl

theorem expiration_invalidates_an_old_borrow_epoch (m : MapState)
    (ids : List Nat) (b : Borrow) (h : b.epoch = m.epoch) :
    borrowValid (expire m ids) b = false := by
  simp [borrowValid, expire, h]

/-- Declared model inspection never supplies an executable contract permission. -/
def modelPermission (_sourceParsed : Bool) : Bool := false

theorem parsed_model_is_not_a_trusted_call (b : Bool) : modelPermission b = false := by rfl

def vectors : List Bool :=
  [canExit (partialInit [1,2]),
   canExit { (partialInit [1,2]) with owned := [] },
   canExit {},
   borrowValid {} ⟨0, none⟩,
   borrowValid { primary := [8] } ⟨0, some 8⟩,
   borrowValid { primary := [8] } ⟨0, some 9⟩,
   borrowValid (destroy { primary := [8] }) ⟨0, some 8⟩,
   borrowValid (expire { primary := [8] } [8]) ⟨0, some 8⟩,
   (failedGetAfterAttach {} 8).1.primary.contains 8,
   modelPermission true,
   canExit (partialInit [1,1])]

theorem shared_vectors : vectors =
    [true,false,true,true,true,false,false,false,true,false,false] := by decide

end QCore.CmetricsApiContracts
