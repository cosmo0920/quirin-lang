import QCore.ReturnFamilyUntrackedReversedSubscript

/-! Card **RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT**: C11 6.5.2.1p2 makes
`E1[E2]` `(*((E1)+(E2)))`, so the pointer a subscript dereferences may stand
inside its brackets. An untracked, non-scalar root of family S
(`legacy_return_family`) filling a subscript's brackets, through cast groups,
parentheses and rule 74w's sums, is a base of the read node unless the
subscripted operand is a declared pointer or array (gateway rule 74y). A
crossed unclassified sum refuses, and a may-bind root inside brackets whose
subscripted operand is not a declared pointer refuses.

The subscript model grounds the reading: the object read is the pointer
operand's on either side, so "index, no base" only under a pointer outside
never misses an inside pointer. Every old base is kept and the new refusals
are the expression's, so nothing admits more than at the base.

The layers, the outside class and the may-bind position (the Rust reader's
output, bound to `subscriptVectors` and `mayBindVectors`) are trust boundary
28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_reversed_subscript.rs`
and by `crates/qcore-c-gateway/tests/return_family_untracked_reversed_subscript.rs`. -/

-- C11 6.5.2.1p2.
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.subscript_comm
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.reversed_reads_the_inside_pointer
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.a_pointer_outside_reads_its_own_object
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.an_integer_outside_puts_the_pointer_inside

-- The reader.
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.no_bracket_keeps_the_reading
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.index_only_under_a_pointer_outside
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.an_inside_pointer_is_never_missed

-- The read: old readings kept, nothing admits more.
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.old_base_kept
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.admitted_was_admitted
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.a_may_bind_root_inside_undecided_brackets_refuses
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.names_kept
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.the_fix_only_refuses_more

-- The readings.
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.old_misses_the_base
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.a_declared_pointer_keeps_the_index
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.an_undecided_outside_is_a_base
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.an_unclassified_sum_refuses

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.subscript_vectors_hold
#print axioms QCore.ReturnFamilyUntrackedReversedSubscript.may_bind_vectors_hold
