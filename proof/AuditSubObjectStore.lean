import QCore.SubObjectStore

/-! Audited Lean companion to card W45-SUB-OBJECT-STORES.

A store into a sub-object (`R->F`, `R[I].F`, `(R)->F`, `S.F`, `(&S)->F`,
`A[I].F`) that the lexical gate refused is lowered, through a resolved typed
base, to a store whose custody effect is decided on the state at the node.

* **Shape of the decision** — neutral, refused, or the escape of the one known
  target whose premises hold.
* **Scalar and untracked values** — custody-neutral; the member's old fact is
  forgotten; no other name is rebound (constraint 5i).
* **Base premises** — an unresolved base, or a tracked base under a pointer
  member, refuses.
* **Owned values** — a status changes only by an escape into the caller's
  record; a stored identity is never dropped unseen; an invisible record, a
  caller's own site, a descriptor, a derived or an unknown pointer refuses.
* **The fallback hands on** — it changes no status and marks the stored
  identity unresolved.
* **Non-vacuity** — each decision on a concrete state. -/

-- Shape of the decision.
#print axioms QCore.SubObjectStore.decide_cases
#print axioms QCore.SubObjectStore.an_escape_needs_a_caller_reachable_base
#print axioms QCore.SubObjectStore.store_admitted

-- Scalar and untracked values.
#print axioms QCore.SubObjectStore.a_neutral_store_changes_no_status
#print axioms QCore.SubObjectStore.an_admitted_store_forgets_the_member
#print axioms QCore.SubObjectStore.poisonAliases_targets
#print axioms QCore.SubObjectStore.an_admitted_store_rebinds_no_other_name
#print axioms QCore.SubObjectStore.a_neutral_store_rebinds_nothing_else

-- Base premises.
#print axioms QCore.SubObjectStore.an_unresolved_base_refuses
#print axioms QCore.SubObjectStore.a_pointer_store_through_a_tracked_base_refuses

-- Owned values.
#print axioms QCore.SubObjectStore.a_status_changes_only_by_an_escape_into_the_callers_record
#print axioms QCore.SubObjectStore.a_stored_identity_is_never_dropped_unseen
#print axioms QCore.SubObjectStore.an_owned_identity_into_an_invisible_record_refuses
#print axioms QCore.SubObjectStore.a_callers_site_is_never_escaped
#print axioms QCore.SubObjectStore.a_descriptor_is_never_stored
#print axioms QCore.SubObjectStore.a_derived_pointer_is_never_escaped
#print axioms QCore.SubObjectStore.an_unknown_value_refuses
#print axioms QCore.SubObjectStore.an_escaped_identity_is_named_only_unknown
#print axioms QCore.SubObjectStore.a_formal_value_keeps_the_refusal

-- The fallback hands on.
#print axioms QCore.SubObjectStore.the_fallback_changes_no_status
#print axioms QCore.SubObjectStore.the_fallback_marks_the_stored_identity

-- Non-vacuity.
#print axioms QCore.SubObjectStore.the_owned_pointer_escapes_into_the_callers_record
#print axioms QCore.SubObjectStore.the_owned_pointer_into_a_local_record_refuses
#print axioms QCore.SubObjectStore.the_descriptor_refuses
#print axioms QCore.SubObjectStore.the_callers_own_pointer_refuses
#print axioms QCore.SubObjectStore.an_unbound_pointer_is_neutral
#print axioms QCore.SubObjectStore.the_formal_base_resolves
#print axioms QCore.SubObjectStore.a_scalar_store_is_admitted
