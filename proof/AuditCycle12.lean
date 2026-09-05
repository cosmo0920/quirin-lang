import QCore.Compiler

-- Print the foundational dependencies of every Cycle 12 compiler theorem.
#print axioms QCore.Compiler.compile_preserves_checked_source
#print axioms QCore.Compiler.compile_implies_whole_program_accepts
#print axioms QCore.Compiler.compile_selects_named_entry
#print axioms QCore.Compiler.compile_retains_entry_cfg
#print axioms QCore.Compiler.compile_emits_origin_mapping
#print axioms QCore.Compiler.emitted_c11_matches_reference
