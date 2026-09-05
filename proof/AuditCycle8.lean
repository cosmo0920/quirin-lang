import QCore.ExecutableModel

-- Print the foundational dependencies of every Cycle 8 safety theorem.
-- Run with: lake env lean AuditCycle8.lean
#print axioms QCore.ExecutableModel.progress_up_to_typed_error
#print axioms QCore.ExecutableModel.preservation
#print axioms QCore.ExecutableModel.steps_preservation
#print axioms QCore.ExecutableModel.no_duplication_of_resource_identities
#print axioms QCore.ExecutableModel.no_double_release
#print axioms QCore.ExecutableModel.region_cleanup_theorem
