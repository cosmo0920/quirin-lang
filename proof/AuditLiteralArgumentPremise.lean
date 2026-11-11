-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.LiteralArgumentPremise

/-! Cards HASH-CONTRACTS and HASH-CLAUSE: declared literal-argument premises,
the carrier they are written in, and the hash-table publication they bound.

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
corpus census: five sites satisfy the premise and ten are withdrawn.

Card HASH-CLAUSE adds the **carrier**. The refusal set at a call site is proved
to be a function of the premise environment alone, so two carriers stating the
same premises decide every call site identically — that is what makes the Rust
side's measurement of *premise-set* equality sufficient for refusal-set parity.
The carriers are proved able to disagree, so the parity statement is not vacuous.
Retirement is audited in both directions: silently ignoring a leftover comment
directive is proved to re-admit exactly the call site the premise withdrew, and
refusing such a text is proved to withdraw every call site instead. The
fail-open reading — answering an unreadable text with an empty premise set — is
proved to apply the declaration where the poisoned reading refuses, including at
a call that never reaches the declared parameter.

Card HASH-REFINE adds the two boundaries HASH-CLAUSE recorded open. For
**HASH-CLAUSE-LITERAL**, the call-site spelling relation is audited on four
sentences: it is *decided, not evaluated* (the reader selects one integer token
out of a contiguous slice of the actual's own stream and returns its digits and
sign verbatim); it is *exact* (every admitted spelling denotes the declared
constant, and the declared spelling is the relation's own fixed point); it is a
*strict widening* (everything exact token equality admitted is still admitted,
and admission is by identity or by denotation, with no third way in); and it
*never crosses a value* — an admitted spelling of one constant satisfies no
declaration of another, a sign is erased by nothing, and an octal digit run
matches no decimal declaration, which is why a leading zero is refused rather
than stripped. Each refused spelling is audited beside the capability deciding
it would need: arithmetic, a base, reaching into a cast, or symbol resolution.
For **HASH-CLAUSE-TYPE**, the premise is audited on a parameter index the
gateway's fixture gives a pointer type: the null-sentinel call is applied, the
live-pointer call is withdrawn, and a premise no call site can satisfy is proved
to withdraw *every* call — so admitting the clause on any parameter type cannot
make a consumer more permissive. -/

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

-- Card HASH-CLAUSE: the carrier, parity, and the retirement in both directions.
#print axioms QCore.LiteralArgumentPremise.equal_carriers_decide_every_call_site_alike
#print axioms QCore.LiteralArgumentPremise.equal_carriers_withdraw_alike
#print axioms QCore.LiteralArgumentPremise.the_migrated_carriers_state_the_same_premises
#print axioms QCore.LiteralArgumentPremise.the_migration_preserves_every_refusal
#print axioms QCore.LiteralArgumentPremise.the_clause_reading_is_the_declared_environment
#print axioms QCore.LiteralArgumentPremise.the_carriers_can_disagree
#print axioms QCore.LiteralArgumentPremise.an_ignored_directive_readmits_the_withdrawn_call
#print axioms QCore.LiteralArgumentPremise.refusing_the_directive_text_withdraws_every_call
#print axioms QCore.LiteralArgumentPremise.the_fail_open_reading_applies_what_poison_refuses
#print axioms QCore.LiteralArgumentPremise.the_fail_open_reading_even_applies_at_an_unmet_premise

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

-- Card HASH-REFINE, HASH-CLAUSE-LITERAL: which call-site spellings are the
-- declared constant. Decided, not evaluated: the literal reader selects one
-- integer token out of a contiguous slice of the actual's own stream and
-- returns its digits and its sign verbatim.
#print axioms QCore.LiteralArgumentPremise.the_read_literal_is_the_token_itself
#print axioms QCore.LiteralArgumentPremise.an_admitted_spelling_returns_its_own_digits
#print axioms QCore.LiteralArgumentPremise.the_core_is_a_contiguous_slice

-- Exact: every admitted spelling denotes the declared constant, and the
-- declared spelling is a fixed point of the relation.
#print axioms QCore.LiteralArgumentPremise.a_declared_literal_is_its_own_canonical_form
#print axioms QCore.LiteralArgumentPremise.an_admitted_spelling_denotes_the_declared_constant
#print axioms QCore.LiteralArgumentPremise.denote_separates_two_constants

-- A strict widening: everything the exact test admitted is still admitted, and
-- there is no third way in.
#print axioms QCore.LiteralArgumentPremise.exact_token_equality_still_satisfies
#print axioms QCore.LiteralArgumentPremise.the_refinement_only_admits
#print axioms QCore.LiteralArgumentPremise.admission_is_identity_or_denotation

-- The admitted spellings, decided, with the widening's non-vacuity.
#print axioms QCore.LiteralArgumentPremise.the_declared_spelling_is_admitted
#print axioms QCore.LiteralArgumentPremise.a_parenthesised_spelling_is_admitted
#print axioms QCore.LiteralArgumentPremise.a_doubly_parenthesised_spelling_is_admitted
#print axioms QCore.LiteralArgumentPremise.a_suffixed_spelling_is_admitted
#print axioms QCore.LiteralArgumentPremise.a_parenthesised_suffixed_spelling_is_admitted
#print axioms QCore.LiteralArgumentPremise.the_widening_admits_more_than_exact_equality

-- The refused spellings, and the capability each would need.
#print axioms QCore.LiteralArgumentPremise.arithmetic_is_refused
#print axioms QCore.LiteralArgumentPremise.a_negated_zero_is_refused
#print axioms QCore.LiteralArgumentPremise.an_alternate_base_is_refused
#print axioms QCore.LiteralArgumentPremise.an_octal_spelling_is_refused
#print axioms QCore.LiteralArgumentPremise.a_cast_is_refused
#print axioms QCore.LiteralArgumentPremise.an_identifier_is_refused
#print axioms QCore.LiteralArgumentPremise.an_unmatched_parenthesis_pair_is_refused
#print axioms QCore.LiteralArgumentPremise.an_octal_run_matches_no_decimal_declaration

-- The widening never crosses a value, and never erases a sign.
#print axioms QCore.LiteralArgumentPremise.an_admitted_zero_never_satisfies_another_constant
#print axioms QCore.LiteralArgumentPremise.a_sign_is_never_erased

-- Card HASH-REFINE, HASH-CLAUSE-TYPE: the premise on a parameter of any type.
-- The null-sentinel reading decides both ways, and a premise no call site can
-- satisfy withdraws every call rather than admitting one.
#print axioms QCore.LiteralArgumentPremise.the_null_sentinel_call_is_applied
#print axioms QCore.LiteralArgumentPremise.the_live_pointer_call_is_withdrawn
#print axioms QCore.LiteralArgumentPremise.an_unsatisfiable_pointer_premise_withdraws_every_call
