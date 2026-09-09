import QCore.BorrowLiveness

/-! Card ALIAS-LOCALS-BORROW-LIVENESS: the `Borrowed` liveness precondition.

Card ALIAS-LOCALS recorded, as its first open item, that a `Borrowed` position
has **no implemented precondition at all**, so a body may hand a callee an
identity the projection itself records `RELEASED` and still be certified `Safe`.
That card reached the hazard only along its *may*-alias relation, between two
distinct names. This audit covers the general rule, over one name's own binding,
in both directions.

Card HYGIENE-14 (`BORROW-LIVENESS-MIXED`) then derived the exact status-set
condition from the join semantics and widened the rule by the one union it
covers: a release on one branch joined with a **consumption** on the other. Two
theorems of the first card were retired by name because that made them false —
`a_release_joined_with_an_escape_is_never_refused` and
`a_refusal_implies_every_incoming_path_released` — as was
`a_refusal_names_a_settled_released_identity`, whose final conjunct named the
old condition. Their replacements are audited below.

* **The decision, positively** — a settled binding over an identity released on
  every reaching path is refused, so the refusals below are not vacuous. This is
  the shape the may-alias relation cannot reach: one name, no peer anywhere. The
  mixed join is refused too, and the whole sixteen-status domain is decided by
  bitmap code against the Rust module's own enumeration of `0..=15`.
* **Every way it fails closed** — a live, escaped, contained or absent identity;
  a release joined with a live or a contained path; an unbound name (**every
  scalar actual**, §C04.2); an unknown, possibly-null, ambiguous or empty
  binding; and any site already carrying an unresolved effect.
* **The definite/may separation** — a name the projection will not settle, which
  is exactly the shape an uncontracted formal parameter has, is never refused
  here whatever any other name did, and the decision is a function of its own
  actual's binding and status alone. The two relations do not blur.
* **The converse** — every refusal is backed by a settled binding, a clean
  unresolved flag and a status whose custody definitely ended. The rule invents
  nothing.
* **The join, and why the fact is a must fact** — a live *or contained* incoming
  path prevents the refusal, and a refusal at a merge implies no incoming path
  left the identity live or contained and that some path released it.
* **Against the existing `Unsafe` bar** — every state this rule refuses also
  satisfies the state test `DoubleRelease` fires on, and the converse fails, so
  the rule requires strictly more of the projection than the kinds published as
  provisional `Unsafe` do. The weaker published class is not a weaker
  measurement.
* **The published class** — a refusal withdraws a certificate, can never certify,
  never manufactures a `violation`, and never softens an existing one.
* **Monotonicity and non-vacuity** — recording an unresolved effect only drops
  refusals, and every guard, including both conjuncts of the widened status
  condition, is shown load-bearing by a fixture that moves without it. -/

-- The decision, positively, and what a refusal publishes.
#print axioms QCore.BorrowLiveness.a_settled_released_identity_is_refused
#print axioms QCore.BorrowLiveness.a_refusal_withdraws_a_certificate
#print axioms QCore.BorrowLiveness.a_release_joined_with_an_escape_is_refused
#print axioms
  QCore.BorrowLiveness.the_refusable_statuses_are_exactly_released_and_released_with_escaped
#print axioms QCore.BorrowLiveness.the_enumerated_domain_is_every_bitmap

-- Every way the decision fails closed.
#print axioms QCore.BorrowLiveness.a_live_identity_is_never_refused
#print axioms QCore.BorrowLiveness.an_escaped_identity_is_never_refused
#print axioms QCore.BorrowLiveness.a_contained_identity_is_never_refused
#print axioms QCore.BorrowLiveness.an_identity_absent_from_the_state_is_never_refused
#print axioms QCore.BorrowLiveness.a_release_on_one_branch_only_is_never_refused
#print axioms QCore.BorrowLiveness.a_release_joined_with_a_containment_is_never_refused
#print axioms QCore.BorrowLiveness.an_unbound_name_is_never_refused
#print axioms QCore.BorrowLiveness.an_unknown_binding_is_never_refused
#print axioms QCore.BorrowLiveness.a_possibly_null_binding_is_never_refused
#print axioms QCore.BorrowLiveness.an_ambiguous_binding_is_never_refused
#print axioms QCore.BorrowLiveness.an_empty_binding_is_never_refused
#print axioms QCore.BorrowLiveness.an_unresolved_effect_site_is_never_refused

-- The definite/may separation.
#print axioms QCore.BorrowLiveness.a_name_the_projection_will_not_settle_is_never_refused
#print axioms QCore.BorrowLiveness.the_decision_reads_only_its_own_actual

-- The converse: a refusal names the facts it claims.
#print axioms QCore.BorrowLiveness.a_refusal_names_a_settled_identity_whose_custody_ended

-- The join, and the must half.
#print axioms QCore.BorrowLiveness.the_join_is_commutative
#print axioms QCore.BorrowLiveness.the_join_is_idempotent
#print axioms QCore.BorrowLiveness.a_live_incoming_path_prevents_the_refusal
#print axioms QCore.BorrowLiveness.an_entitling_incoming_path_prevents_the_refusal
#print axioms QCore.BorrowLiveness.a_refusal_implies_no_incoming_path_entitles_a_read

-- Against the existing provisional-`Unsafe` bar.
#print axioms QCore.BorrowLiveness.a_refusal_satisfies_the_existing_bar
#print axioms QCore.BorrowLiveness.the_existing_violation_bar_is_strictly_weaker

-- The published verdict class.
#print axioms QCore.BorrowLiveness.the_rule_never_certifies
#print axioms QCore.BorrowLiveness.a_refusal_is_never_the_violation_class
#print axioms QCore.BorrowLiveness.an_existing_violation_survives
#print axioms QCore.BorrowLiveness.not_firing_changes_nothing

-- Monotonicity and non-vacuity.
#print axioms QCore.BorrowLiveness.recording_an_unresolved_effect_only_drops_refusals
#print axioms QCore.BorrowLiveness.the_unresolved_guard_is_load_bearing
#print axioms QCore.BorrowLiveness.the_settled_guard_is_load_bearing
#print axioms QCore.BorrowLiveness.the_released_conjunct_is_load_bearing
#print axioms QCore.BorrowLiveness.the_entitling_conjunct_is_load_bearing
#print axioms QCore.BorrowLiveness.a_reacquired_identity_is_admitted_again
