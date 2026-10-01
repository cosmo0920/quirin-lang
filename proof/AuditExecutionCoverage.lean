import QCore.ExecutionCoverage

/-! Audited companion to CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule
11z): race freedom over every candidate execution of a bounded, loop-free
fragment (gap entry W07). -/
#print axioms QCore.ExecutionCoverage.perm_mem_perms
#print axioms QCore.ExecutionCoverage.mem_choices_iff
#print axioms QCore.ExecutionCoverage.mem_nub
#print axioms QCore.ExecutionCoverage.paths_complete
#print axioms QCore.ExecutionCoverage.paths_sound
#print axioms QCore.ExecutionCoverage.run_write_literal
#print axioms QCore.ExecutionCoverage.assemble_write_literal
#print axioms QCore.ExecutionCoverage.kindOf_event
#print axioms QCore.ExecutionCoverage.check_nil_iff
#print axioms QCore.ExecutionCoverage.read_values_are_literals
#print axioms QCore.ExecutionCoverage.allIn_rfSources
#print axioms QCore.ExecutionCoverage.encode_mem_candidates
#print axioms QCore.ExecutionCoverage.candidate_is_execution
#print axioms QCore.ExecutionCoverage.verdict_decided
#print axioms QCore.ExecutionCoverage.verdict_race_free_all
#print axioms QCore.ExecutionCoverage.verdict_race_free_sound
#print axioms QCore.ExecutionCoverage.verdict_race_free_accepts
#print axioms QCore.ExecutionCoverage.verdict_race_witnessed
#print axioms QCore.ExecutionCoverage.loop_is_unknown
#print axioms QCore.ExecutionCoverage.race_free_needs_bounds
#print axioms QCore.ExecutionCoverage.coverage_vectors_hold
