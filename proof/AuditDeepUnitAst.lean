import QCore.DeepUnitAst

/-! Audited companion to RDHTTP-HEADER-SUMMARY-GAP: a whole-unit typed AST nesting beyond serde_json's default limit is read within an admitted bound (rule 13o); beyond it the unit keeps no index and its examined bodies stay unvalidated in the fold (rule 13p). -/

#print axioms QCore.DeepUnitAst.an_index_read_before_is_read_after
#print axioms QCore.DeepUnitAst.the_index_is_the_document
#print axioms QCore.DeepUnitAst.beyond_the_bound_there_is_no_index
#print axioms QCore.DeepUnitAst.only_the_deep_band_changes
#print axioms QCore.DeepUnitAst.a_unit_without_an_index_keeps_the_declaration_unvalidated
#print axioms QCore.DeepUnitAst.the_fold_never_skips_a_unit
#print axioms QCore.DeepUnitAst.depth_vectors_hold
#print axioms QCore.DeepUnitAst.band_edges
