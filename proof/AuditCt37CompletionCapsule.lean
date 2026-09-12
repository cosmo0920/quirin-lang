import QCore.Ct37CompletionCapsule

-- Card `CT-PROOF-LEG-2`: the completion-capsule discharge, proved on CT37's
-- own trace terms, on the CT01 precedent.
-- `QCore.Ct37CompletionCapsule.referenceRun` transcribes CT37's reference run —
-- the named runner
-- `compiler/tests/ct37_proof_leg.rs::ct37_artifact_is_the_lean_proved_completion_capsule_shape`
-- checks it event for event against the real pipeline's execution of
-- `compiler/tests/fixtures/ct/CT37/case.qcore`, recomputes the digest pin and
-- runs the same total decision procedure on the artifact.

-- The reference run is a declared admitted causal order, its projections, and
-- the transcription pin.
#print axioms QCore.Ct37CompletionCapsule.ct37_reference_run_is_an_admitted_causal_order
#print axioms QCore.Ct37CompletionCapsule.parent_projection_of_the_reference
#print axioms QCore.Ct37CompletionCapsule.child_projection_of_the_reference
#print axioms QCore.Ct37CompletionCapsule.ct37_trace_digest

-- Meaning: what the decision procedure's acceptance has been shown to imply,
-- for arbitrary traces — the parent's and the child's exact program orders,
-- the unique parent-side release after the join, the read-back of the
-- allocation's own payload between join and release, the generation-preserving
-- return, and the cross-actor early finish (the child's completion strictly
-- before the parent's join).
#print axioms QCore.Ct37CompletionCapsule.discharged_parent_facts_are_exactly_the_scenario
#print axioms QCore.Ct37CompletionCapsule.discharged_child_facts_are_exactly_the_scenario
#print axioms QCore.Ct37CompletionCapsule.discharged_release_is_unique
#print axioms QCore.Ct37CompletionCapsule.discharged_join_precedes_the_release
#print axioms QCore.Ct37CompletionCapsule.discharged_read_back_sits_between_join_and_release
#print axioms QCore.Ct37CompletionCapsule.discharged_return_preserves_the_generation
#print axioms QCore.Ct37CompletionCapsule.discharged_return_precedes_the_join

-- CT37's reference run is accepted, and so is every rendering the F02
-- relation admits against it — arbitrary traces, not an enumerated domain,
-- with the cross-actor conjunct extracted from the relation's own order
-- admission.
#print axioms QCore.Ct37CompletionCapsule.ct37_reference_run_discharges_the_completion_capsule
#print axioms QCore.Ct37CompletionCapsule.every_admitted_rendering_discharges_the_completion_capsule

-- Named refusals: the procedure is not vacuous.
#print axioms QCore.Ct37CompletionCapsule.a_join_before_the_return_is_refused
#print axioms QCore.Ct37CompletionCapsule.a_missing_release_is_refused
#print axioms QCore.Ct37CompletionCapsule.a_release_by_the_child_is_refused
#print axioms QCore.Ct37CompletionCapsule.a_double_release_is_refused
#print axioms QCore.Ct37CompletionCapsule.a_generation_changing_return_is_refused
#print axioms QCore.Ct37CompletionCapsule.a_payload_losing_capsule_is_refused
