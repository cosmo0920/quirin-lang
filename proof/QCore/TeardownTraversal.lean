namespace QCore.TeardownTraversal

-- A finite reciprocal ring is represented by the ordered identities between
-- the live sentinel and itself. The child call removes only the current node
-- and preserves the saved successor and the remaining ring (a caller premise).
structure Cursor where
  visited : List Nat
  remaining : List Nat
  deriving DecidableEq

def step (c : Cursor) : Option Cursor :=
  match c.remaining with
  | [] => none
  | child :: savedNext => some ⟨c.visited ++ [child], savedNext⟩

theorem empty_ring_has_no_child_call (visited : List Nat) :
    step ⟨visited, []⟩ = none := rfl

theorem advance_uses_saved_successor (visited : List Nat) (child : Nat) (rest : List Nat) :
    step ⟨visited, child :: rest⟩ = some ⟨visited ++ [child], rest⟩ := rfl

theorem accepted_step_conserves_order {before after : Cursor}
    (h : step before = some after) :
    after.visited ++ after.remaining = before.visited ++ before.remaining := by
  unfold step at h
  split at h
  · contradiction
  · cases Option.some.inj h
    simp_all

theorem accepted_step_decreases_remaining {before after : Cursor}
    (h : step before = some after) :
    after.remaining.length + 1 = before.remaining.length := by
  unfold step at h
  split at h
  · contradiction
  · cases Option.some.inj h
    simp_all

theorem completed_traversal_preserves_exact_identities (initial : List Nat) (c : Cursor)
    (conserved : c.visited ++ c.remaining = initial) (done : c.remaining = []) :
    c.visited = initial := by simpa [done] using conserved

theorem completed_distinct_ring_has_no_duplicate_calls (initial : List Nat) (c : Cursor)
    (unique : initial.Nodup) (conserved : c.visited ++ c.remaining = initial)
    (done : c.remaining = []) : c.visited.Nodup := by
  rw [completed_traversal_preserves_exact_identities initial c conserved done]
  exact unique

end QCore.TeardownTraversal
