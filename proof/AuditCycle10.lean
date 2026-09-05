import QCore.Frontend

-- Print the foundational dependencies of every Cycle 10 front-end theorem.
#print axioms QCore.Frontend.elaborate_implies_wellFormed
#print axioms QCore.Frontend.elaborate_preserves_source
#print axioms QCore.Frontend.elaborate_preserves_declaration_count
#print axioms QCore.Frontend.elaborate_preserves_spans
#print axioms QCore.Frontend.elaborate_has_unique_declarations
#print axioms QCore.Frontend.elaborate_resolves_references
#print axioms QCore.Frontend.elaborate_produces_valid_spans
#print axioms QCore.Frontend.elaborate_symbol_count
#print axioms QCore.Frontend.elaborate_symbols_unique
#print axioms QCore.Frontend.invalid_program_is_rejected
