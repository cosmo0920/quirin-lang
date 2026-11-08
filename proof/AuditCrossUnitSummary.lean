import QCore.CrossUnitSummary

/-! Card **W44-CALLEE-CROSS-UNIT-SUMMARY**: conditional transport and the
missing, ambiguous, source-changed, and environment-changed refusals. -/

#print axioms QCore.CrossUnitSummary.declaration_wins
#print axioms QCore.CrossUnitSummary.admitted_import_is_valid
#print axioms QCore.CrossUnitSummary.missing_resolution_refuses
#print axioms QCore.CrossUnitSummary.ambiguous_home_refuses
#print axioms QCore.CrossUnitSummary.changed_source_refuses
#print axioms QCore.CrossUnitSummary.changed_environment_refuses

/-! Card **LINK-STANDING** (rule 40): a derived resolution, read from the
linked executable's link-resolution artifact, discharges the same admission
obligation as a stated map; every failed condition refuses, and a refused
home never supplies a summary. -/

#print axioms QCore.CrossUnitSummary.standing_none_iff
#print axioms QCore.CrossUnitSummary.failed_condition_refuses
#print axioms QCore.CrossUnitSummary.database_mismatch_refuses
#print axioms QCore.CrossUnitSummary.unverified_digest_refuses
#print axioms QCore.CrossUnitSummary.unlinked_caller_refuses
#print axioms QCore.CrossUnitSummary.shared_object_refuses
#print axioms QCore.CrossUnitSummary.weak_definition_refuses
#print axioms QCore.CrossUnitSummary.duplicate_definition_refuses
#print axioms QCore.CrossUnitSummary.interposable_refuses
#print axioms QCore.CrossUnitSummary.unlinked_home_configuration_refuses
#print axioms QCore.CrossUnitSummary.unanalysed_linked_configuration_refuses
#print axioms QCore.CrossUnitSummary.ambiguous_key_refuses
#print axioms QCore.CrossUnitSummary.unavailable_type_refuses
#print axioms QCore.CrossUnitSummary.unequal_type_refuses
#print axioms QCore.CrossUnitSummary.typeAgrees_iff
#print axioms QCore.CrossUnitSummary.missing_caller_type_disagrees
#print axioms QCore.CrossUnitSummary.derived_resolution_discharges
#print axioms QCore.CrossUnitSummary.derived_admission_is_valid
#print axioms QCore.CrossUnitSummary.derived_refusal_is_not_admitted
#print axioms QCore.CrossUnitSummary.refused_home_supplies_nothing
#print axioms QCore.CrossUnitSummary.standing_vectors_hold
