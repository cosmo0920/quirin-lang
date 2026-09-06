namespace QCore.NativeTeardownObservation

-- A trusted native observer and structural adapter derive an ordered release
-- plan from live, distinct constructor generations. This models the final
-- exact-plan gate only; neither C instrumentation nor Python is verified here.
def accepted (plan observed : List Nat) : Prop :=
  plan.Nodup ∧ observed = plan

theorem accepted_preserves_exact_generation_order {plan observed : List Nat}
    (h : accepted plan observed) : observed = plan := h.2

theorem accepted_has_no_duplicate_release {plan observed : List Nat}
    (h : accepted plan observed) : observed.Nodup := h.2 ▸ h.1

theorem accepted_releases_every_planned_generation {plan observed : List Nat}
    (h : accepted plan observed) (id : Nat) : id ∈ observed ↔ id ∈ plan := by
  rw [h.2]

theorem accepted_conserves_release_count {plan observed : List Nat}
    (h : accepted plan observed) : observed.length = plan.length := by rw [h.2]

theorem changed_release_order_rejected {plan observed : List Nat}
    (h : observed ≠ plan) : ¬accepted plan observed := fun a => h a.2

end QCore.NativeTeardownObservation
