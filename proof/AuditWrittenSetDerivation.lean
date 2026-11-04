import QCore.WrittenSetDerivation

/-! Audited companion to CELL-RESULT-SET-WRITTEN-SET-DERIVATION (gateway rule 38e as amended; user decisions 2026-11-04, "Basically, Q1(a) and Q2(i) ...", "Write [1, MAX]"): a pinned slot written set is derived from the body chain; the paths' slot sets combine soundly, a test's narrowing keeps the values its edge admits and a narrowing without the class constrains nothing, a call's out array holds the callee's written set once its result is one class, an aliasing store joins its value, forwarding keeps the callee's written set; the declared [1, MAX] admits the creation while POSIX's [0, MAX] derives descriptor 0; the Rust vectors hold. -/

#print axioms QCore.WrittenSetDerivation.combine_sound
#print axioms QCore.WrittenSetDerivation.narrow_sound
#print axioms QCore.WrittenSetDerivation.a_narrowing_without_the_class_constrains_nothing
#print axioms QCore.WrittenSetDerivation.resolve_sound
#print axioms QCore.WrittenSetDerivation.an_empty_narrowing_is_infeasible
#print axioms QCore.WrittenSetDerivation.after_store_sound
#print axioms QCore.WrittenSetDerivation.forwarding_sound
#print axioms QCore.WrittenSetDerivation.the_declared_bound_admits_the_creation
#print axioms QCore.WrittenSetDerivation.posix_s_set_derives_descriptor_zero
#print axioms QCore.WrittenSetDerivation.combine_vectors_hold
#print axioms QCore.WrittenSetDerivation.resolve_vectors_hold
