import QCore.PublicationCustody

/-! Card CTWIN-PUBLISH: publication into an object that outlives the call.

Card CT-CTWIN recorded, as the open finding `CT-CTWIN-PUBLISH`, that a store of
an owned pointer into a file-scope object is invisible to the unrestricted-C
resource projection: publish-then-release was **`Safe` with no diagnostic**, and
the dangling published reference had no representation at all. This audit covers
the decision that closes it, in both directions, and the two-direction argument
for refusing the store rather than modelling it as an escape.

* **The decision, positively** — an alias, an acquisition and a custody-yielding
  call bound to a file-scope object are each refused, as is a store into a
  body's own `static` local; and the refusal withdraws a certificate.
* **Why refusal, not an escape** — under the escape model the
  published-and-not-released body certifies `safe`, which the implemented model
  refuses. The promotion is exhibited concretely and then stated as an
  existential, and the implemented model is shown to certify nothing and to
  manufacture no violation on either compared input.
* **Every way it fails closed** — a null store, an opaque value, a call with no
  custody result, an unbound source, a caller-held (borrowed) identity, a
  formal, a shadowing local declaration, and a name no reader recognised. The
  last is the recorded residual `CTWIN-PUBLISH-SCAN`, stated rather than hidden.
* **What the decision reads** — the destination half is a function of the four
  name sets alone, so no declared qualifier can move it (the model half of the
  `CT-CTWIN-SYNC` decision); the value half reads no per-site status, so a
  dangling identity is refused exactly as a live one is.
* **Monotonicity and the load-bearing conjuncts** — recognising more
  static-storage names only adds refusals, and each of the three conjuncts is
  shown moving a fixture.
* **The published verdict class** — a refusal never certifies, never
  manufactures a `violation`, never softens an existing one, and not firing
  changes nothing.
* **Non-vacuity** — the rule both refuses and admits.

The readers' faithfulness — that the file-scope declarators, the body's own
declarations, the formal names and the flow-sensitive bindings are extracted
correctly from C source — is a premise here, exactly as in
`QCore.BorrowedActual` and `QCore.BorrowLiveness`. -/

-- The decision, positively.
#print axioms
  QCore.PublicationCustody.a_store_of_a_body_acquired_identity_into_a_file_scope_object_is_refused
#print axioms QCore.PublicationCustody.an_acquisition_bound_to_such_an_object_is_refused
#print axioms QCore.PublicationCustody.a_custody_yielding_call_bound_to_such_an_object_is_refused
#print axioms QCore.PublicationCustody.a_static_local_is_a_destination
#print axioms QCore.PublicationCustody.a_refusal_withdraws_a_certificate
#print axioms QCore.PublicationCustody.a_publication_followed_by_a_release_never_certifies

-- Why refusal, and not an escape.
#print axioms QCore.PublicationCustody.the_escape_model_certifies_a_body_the_refusal_withdraws
#print axioms QCore.PublicationCustody.the_escape_model_promotes_on_some_input
#print axioms QCore.PublicationCustody.the_refusal_model_certifies_nothing
#print axioms QCore.PublicationCustody.a_publication_is_never_a_validated_violation

-- Every way the decision fails closed.
#print axioms QCore.PublicationCustody.a_null_store_is_never_refused
#print axioms QCore.PublicationCustody.an_untracked_value_is_never_refused
#print axioms QCore.PublicationCustody.a_call_with_no_custody_result_is_never_refused
#print axioms QCore.PublicationCustody.an_unbound_source_is_never_refused
#print axioms QCore.PublicationCustody.a_caller_held_identity_is_never_refused
#print axioms QCore.PublicationCustody.a_formal_is_not_a_destination
#print axioms QCore.PublicationCustody.a_shadowed_name_is_not_a_destination
#print axioms QCore.PublicationCustody.an_unrecognised_name_is_not_a_destination

-- What the decision reads, and what it does not.
#print axioms QCore.PublicationCustody.the_destination_decision_reads_only_the_four_name_sets
#print axioms QCore.PublicationCustody.the_rule_reads_no_status

-- Monotonicity and the load-bearing conjuncts.
#print axioms QCore.PublicationCustody.a_wider_destination_set_only_adds_refusals
#print axioms QCore.PublicationCustody.a_wider_scan_only_adds_destinations
#print axioms QCore.PublicationCustody.the_static_duration_conjunct_is_load_bearing
#print axioms QCore.PublicationCustody.the_tracked_value_conjunct_is_load_bearing
#print axioms QCore.PublicationCustody.the_caller_site_exclusion_is_load_bearing

-- The published verdict class.
#print axioms QCore.PublicationCustody.the_rule_never_certifies
#print axioms QCore.PublicationCustody.a_refusal_is_never_the_violation_class
#print axioms QCore.PublicationCustody.an_existing_violation_survives
#print axioms QCore.PublicationCustody.not_firing_changes_nothing

-- Non-vacuity.
#print axioms QCore.PublicationCustody.the_rule_both_refuses_and_admits
