import QCore.ChainedAssignment

/-! Audited Lean companion to card CHAINED-ASSIGNMENT-INNER-DESTINATION
(gateway rule 5i, amended).

The statement classifier read `d = safe = rd_malloc(n)` as `d = rd_malloc(n)`
and dropped the write of `safe`. The guard replaces such an admission by a
named refusal; rule 10s's elaborator lowers the chain as its sequential
assignments.

* **C11 6.5.16p3** — a chain equals its two (and its n) sequential
  assignments, and both names hold the same identity.
* **The defect** — the old reading differs from C at the inner name.
* **The guard** — never admits, keeps refusals, refuses every admitted inner
  write, is the identity elsewhere, never promotes.
* **Non-vacuity** — the scram shape and a scalar chain. -/

-- C11 6.5.16p3.
#print axioms QCore.ChainedAssignment.chain_eq_sequential
#print axioms QCore.ChainedAssignment.chainN_eq_sequential
#print axioms QCore.ChainedAssignment.both_names_hold_the_identity

-- The defect.
#print axioms QCore.ChainedAssignment.the_old_reading_drops_the_inner_write

-- The guard.
#print axioms QCore.ChainedAssignment.the_guard_never_admits
#print axioms QCore.ChainedAssignment.the_guard_keeps_refusals
#print axioms QCore.ChainedAssignment.the_guard_refuses_every_inner_write
#print axioms QCore.ChainedAssignment.the_guard_is_identity_elsewhere
#print axioms QCore.ChainedAssignment.the_guard_never_promotes

-- Non-vacuity.
#print axioms QCore.ChainedAssignment.the_scram_shape_is_dropped_by_the_old_reading
#print axioms QCore.ChainedAssignment.the_scram_shape_is_its_sequential_assignments
#print axioms QCore.ChainedAssignment.a_scalar_chain_is_sequential
