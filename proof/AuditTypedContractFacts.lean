import QCore.TypedContractFacts

-- Typed equality is exact and never coerces text into integers.
#print axioms QCore.TypedContractFacts.equal_text_is_decided
#print axioms QCore.TypedContractFacts.unequal_text_is_decided
#print axioms QCore.TypedContractFacts.mixed_equality_refuses
#print axioms QCore.TypedContractFacts.ordered_text_refuses

-- Boolean evaluation is eager, matching the Rust fail-closed interpreter.
#print axioms QCore.TypedContractFacts.false_and_missing_still_refuses
#print axioms QCore.TypedContractFacts.true_or_missing_still_refuses

-- The integer domain and fact environment are bounded/refusing.
#print axioms QCore.TypedContractFacts.i128_addition_overflow_refuses
#print axioms QCore.TypedContractFacts.duplicate_facts_refuse

-- Only the strict native policy declaration shape is eligible.
#print axioms QCore.TypedContractFacts.an_unsafe_policy_is_not_strict
#print axioms QCore.TypedContractFacts.an_effectful_policy_is_not_strict
#print axioms QCore.TypedContractFacts.a_postcondition_is_required
