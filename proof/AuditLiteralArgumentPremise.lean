import QCore.LiteralArgumentPremise

/-! Card HASH-CONTRACTS: declared literal-argument premises, and the hash-table
publication they bound.

Two directions are audited for the mechanism. **Withdrawal**: a poisoned set, an
unmet premise, a mismatched actual and a *missing* actual each withdraw the
declaration, and extra premises can never re-admit what a smaller set withdrew.
**Conservativity**: an applied contract is the declared one unchanged, a call
with no declaration is never applied, and an environment with no premise is the
unrestricted one exactly.

For the declaration itself, the two upstream arms are transcribed and the
sentence the whole card rests on is proved: on no arm does the table free a
pointer the caller handed it. Over the whole v1 parameter vocabulary, `borrowed`
is the *only* mode whose caller-side effect is the measured one, and the `stored`
reading is additionally proved not well formed against the measured signature.
Retention is proved real on the admitted arm and proved not to change the
caller's effect, so neither fact can be read as the other.

Non-vacuity is audited on the mechanism and against the measured fifteen-site
corpus census: five sites satisfy the premise and ten are withdrawn. -/

-- Conservativity: the premise never invents, alters or widens a declaration.
#print axioms QCore.LiteralArgumentPremise.an_applied_contract_is_the_declared_one
#print axioms QCore.LiteralArgumentPremise.an_applied_call_has_a_declaration
#print axioms QCore.LiteralArgumentPremise.an_undeclared_callee_is_never_applied
#print axioms QCore.LiteralArgumentPremise.no_premise_is_transparent

-- Every withdrawal ground, including the absent actual.
#print axioms QCore.LiteralArgumentPremise.a_poisoned_environment_applies_nothing
#print axioms QCore.LiteralArgumentPremise.an_unmet_premise_applies_nothing
#print axioms QCore.LiteralArgumentPremise.a_missing_actual_never_satisfies
#print axioms QCore.LiteralArgumentPremise.only_the_exact_literal_satisfies
#print axioms QCore.LiteralArgumentPremise.a_mismatched_actual_withdraws

-- Monotone: more premises can only refuse more.
#print axioms QCore.LiteralArgumentPremise.extra_premises_preserve_withdrawal
#print axioms QCore.LiteralArgumentPremise.extra_premises_never_admit

-- Non-vacuity of the mechanism, in both directions.
#print axioms QCore.LiteralArgumentPremise.the_literal_call_is_applied
#print axioms QCore.LiteralArgumentPremise.the_positive_call_is_withdrawn
#print axioms QCore.LiteralArgumentPremise.the_short_call_is_withdrawn
#print axioms QCore.LiteralArgumentPremise.poison_withdraws_the_literal_call
#print axioms QCore.LiteralArgumentPremise.without_the_premise_the_positive_call_is_applied

-- The measured corpus census.
#print axioms QCore.LiteralArgumentPremise.the_census_has_fifteen_call_sites
#print axioms QCore.LiteralArgumentPremise.exactly_five_call_sites_satisfy_the_premise
#print axioms QCore.LiteralArgumentPremise.exactly_ten_call_sites_are_withdrawn

-- The two upstream arms, as transcribed.
#print axioms QCore.LiteralArgumentPremise.the_reference_arm_records_minus_one
#print axioms QCore.LiteralArgumentPremise.the_reference_arm_is_never_freed_by_the_table
#print axioms QCore.LiteralArgumentPremise.the_reference_arm_stores_the_callers_pointer
#print axioms QCore.LiteralArgumentPremise.the_copy_arm_stores_the_callees_allocation
#print axioms QCore.LiteralArgumentPremise.the_copy_arm_is_freed_by_the_table
#print axioms QCore.LiteralArgumentPremise.the_callers_pointer_is_never_freed_by_the_table

-- Which mode is faithful, over the whole vocabulary.
#print axioms QCore.LiteralArgumentPremise.borrowed_has_the_measured_effect
#print axioms QCore.LiteralArgumentPremise.borrowed_is_the_only_mode_with_the_measured_effect
#print axioms QCore.LiteralArgumentPremise.the_stored_reading_would_move_the_obligation
#print axioms QCore.LiteralArgumentPremise.the_released_reading_would_discharge_the_obligation
#print axioms QCore.LiteralArgumentPremise.the_consumed_reading_would_transfer_the_obligation
#print axioms QCore.LiteralArgumentPremise.the_owned_reading_would_demand_live_custody

-- The stored reading is not even well formed against the measured signature.
#print axioms QCore.LiteralArgumentPremise.no_stored_reading_of_the_publication_is_well_formed
#print axioms QCore.LiteralArgumentPremise.the_publication_borrows_every_parameter

-- Retention is real, and is not custody.
#print axioms QCore.LiteralArgumentPremise.the_admitted_arm_retains
#print axioms QCore.LiteralArgumentPremise.the_withdrawn_arm_does_not_retain
#print axioms QCore.LiteralArgumentPremise.retention_does_not_change_the_caller_effect

-- The premise admits exactly the arm the declaration was read from.
#print axioms QCore.LiteralArgumentPremise.the_premise_literal_is_the_zero_token
#print axioms QCore.LiteralArgumentPremise.the_admitted_size_takes_the_reference_arm
#print axioms QCore.LiteralArgumentPremise.a_positive_size_takes_the_copy_arm
#print axioms QCore.LiteralArgumentPremise.the_two_arms_are_distinct
