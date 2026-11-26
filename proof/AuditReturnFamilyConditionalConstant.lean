import QCore.ReturnFamilyConditionalConstant

/-! Card **RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING**: family S
(`legacy_return_family`) decides the condition of an accessed conditional
only when it is an integer constant expression of literals and the integer
operators that the reader evaluates exactly under the LP64 premise (gateway
rule 74z). It then keeps only the roots of the arm C evaluates (C11
6.5.15p4). Both arms are still classified, so no refusal is added or lost.
An undecided condition keeps rule 74x's reading.

The evaluator leaves every undefined or implementation-defined case
undecided. The pruned roots cover every root of every execution. A root
rule 74x reads but pruning drops is in no execution. If the tip admits, rule
74x admits once the dropped bases discharge. The condition's expression and
each group's arms are trust boundary 28's, bound to `constVectors` and
`pruneVectors`. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_conditional_constant.rs`
and by `crates/qcore-c-gateway/tests/return_family_untracked_conditional_access/constant_pruning.rs`. -/

-- The evaluator's exactness.
#print axioms QCore.ReturnFamilyConditionalConstant.div_by_zero_undecided
#print axioms QCore.ReturnFamilyConditionalConstant.rem_by_zero_undecided
#print axioms QCore.ReturnFamilyConditionalConstant.signed_overflow_undecided
#print axioms QCore.ReturnFamilyConditionalConstant.signed_arith_exact
#print axioms QCore.ReturnFamilyConditionalConstant.unsigned_arith_modular
#print axioms QCore.ReturnFamilyConditionalConstant.shift_out_of_range_undecided
#print axioms QCore.ReturnFamilyConditionalConstant.negative_signed_shift_undecided
#print axioms QCore.ReturnFamilyConditionalConstant.undecided_propagates
#print axioms QCore.ReturnFamilyConditionalConstant.decided_only_when_evaluated
#print axioms QCore.ReturnFamilyConditionalConstant.undecided_condition

-- C11 6.5.15p4 and pruning.
#print axioms QCore.ReturnFamilyConditionalConstant.only_one_arm_is_evaluated
#print axioms QCore.ReturnFamilyConditionalConstant.undecided_keeps_74x_reading
#print axioms QCore.ReturnFamilyConditionalConstant.pruned_refuses_iff
#print axioms QCore.ReturnFamilyConditionalConstant.pruned_subset
#print axioms QCore.ReturnFamilyConditionalConstant.pruned_covers_every_run
#print axioms QCore.ReturnFamilyConditionalConstant.pruning_removes_only_unevaluated_bases

-- The group and the read.
#print axioms QCore.ReturnFamilyConditionalConstant.pruned_group_refuses_iff
#print axioms QCore.ReturnFamilyConditionalConstant.pruning_keeps_every_refusal
#print axioms QCore.ReturnFamilyConditionalConstant.old_base_kept
#print axioms QCore.ReturnFamilyConditionalConstant.only_dropped_bases_are_discharged

-- The readings and the token reader's vectors.
#print axioms QCore.ReturnFamilyConditionalConstant.constant_false_prunes_the_middle
#print axioms QCore.ReturnFamilyConditionalConstant.const_vectors_hold
#print axioms QCore.ReturnFamilyConditionalConstant.prune_vectors_hold
