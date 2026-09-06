namespace QCore.NestedCleanup

-- Post-child accounting: allocations can be released while exact per-child
-- residual identities or an up-counter obligation remain undischargeable.
structure State where
  parentsLive : Bool
  up : Nat
  residuals : List Nat
  deriving DecidableEq

def finish (entry consumed : Nat) (residuals : List Nat) : Option State :=
  if consumed ≤ entry then some ⟨false, entry - consumed, residuals⟩ else none

def discharged (s : State) : Bool := s.residuals.isEmpty && decide (s.up = 0)

theorem accepted_finish_preserves_residual_identities {entry consumed : Nat}
    {residuals : List Nat} {after : State} (h : finish entry consumed residuals = some after) :
    after.residuals = residuals := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem accepted_finish_balances_up_counter {entry consumed : Nat}
    {residuals : List Nat} {after : State} (h : finish entry consumed residuals = some after) :
    after.up + consumed = entry := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    simp_all
  · contradiction

theorem up_counter_underflow_rejected (entry consumed : Nat) (residuals : List Nat)
    (h : entry < consumed) : finish entry consumed residuals = none := by
  simp [finish, Nat.not_le.mpr h]

theorem accepted_discharge_has_no_child_or_counter_obligation (s : State)
    (h : discharged s = true) : s.residuals = [] ∧ s.up = 0 := by
  simpa [discharged] using h

theorem unresolved_child_blocks_discharge {entry consumed : Nat}
    {residuals : List Nat} {after : State} (h : finish entry consumed residuals = some after)
    (unresolved : residuals ≠ []) : discharged after = false := by
  have same := accepted_finish_preserves_residual_identities h
  simp [discharged, same, unresolved]

end QCore.NestedCleanup
