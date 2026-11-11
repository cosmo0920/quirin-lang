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

import QCore.DeclaredAbiAgreement

/-! Card **LOADER-ABI**: the declared contract's C ABI projections, compared
against the analysed translation unit's own C declaration.

The gap this closes is that the loader checked those spellings against its own
closed table and against nothing else, so an unfaithful spelling passed
silently — and the spellings are load-bearing, because the loader's
admissibility rules are keyed on them.

Four directions are audited.

**Conservativity.** Agreement applies the declared contract *unchanged*: an
applied contract is the declared one, an undeclared callee is never applied, and
withdrawal only ever removes. Reading a *first* declaration for a name is proved
unable to turn an unapplied contract into an applied one.

**Withdrawal.** A disagreement withdraws, and the comparison's own result is
what withdraws it, so every class below reaches the same conclusion through the
same route.

**Fail-closed edges.** Retained texts the loader cannot re-read withdraw *every*
contracted name rather than none of them; an environment with no declared
signature, and a name this unit does not declare, compare nothing and are
recorded rather than implied (`LOADER-ABI-UNDECLARED`).

**Non-vacuity, class by class.** Each refusal is produced by a decided witness —
wrong result type, wrong parameter type, a swapped vector, a wrong arity, a tail
claimed of a non-variadic callee, an unreadable parameter or result, and an
unspecified parameter list — beside the faithful pair that agrees, so no branch
of the comparison is unreachable and none of the refusals is the same class in
disguise.

Two boundaries are audited as the arguments they are rather than as exceptions.
The **LP64 projection** is set-valued: `unsigned long` admits both `u64` and
`usize`, because on the audited targets they are two spellings of one eight-byte
reading; signedness, width and constness are each proved never to cross, and
plain `char` admits both byte spellings because C leaves its signedness
implementation-defined. The **zero-arity admission** — a contract naming no
parameter and claiming no tail survives a C declaration that specifies no
parameters — is audited together with the sentence that makes it safe: under
such a contract the analyzer's existing call-site arity gate admits only the
call that passes nothing, so an unspecified C list has nothing to contradict. A
tail claim is proved *not* to survive there.

What this is not: a model of a decision. That the reader's projection is the
type C really declares, and C's own object model, are premises from outside. -/

-- Conservativity: agreement applies the declared contract, unchanged.
#print axioms QCore.DeclaredAbiAgreement.agreement_applies_the_declared_contract
#print axioms QCore.DeclaredAbiAgreement.an_applied_contract_is_the_declared_one
#print axioms QCore.DeclaredAbiAgreement.an_undeclared_callee_is_never_applied
#print axioms QCore.DeclaredAbiAgreement.withdrawal_only_removes
#print axioms QCore.DeclaredAbiAgreement.reading_a_first_declaration_never_admits

-- A disagreement withdraws, through the comparison's own result.
#print axioms QCore.DeclaredAbiAgreement.a_disagreement_withdraws
#print axioms QCore.DeclaredAbiAgreement.a_compared_disagreement_withdraws

-- The fail-closed edges, and the recorded boundary.
#print axioms QCore.DeclaredAbiAgreement.poison_withdraws_every_contracted_name
#print axioms QCore.DeclaredAbiAgreement.an_environment_without_a_signature_compares_nothing
#print axioms QCore.DeclaredAbiAgreement.an_undeclared_name_is_not_compared

-- The LP64 projection: set-valued on purpose, and crossing nothing.
#print axioms QCore.DeclaredAbiAgreement.an_unsigned_long_admits_both_of_its_spellings
#print axioms QCore.DeclaredAbiAgreement.signedness_never_crosses
#print axioms QCore.DeclaredAbiAgreement.width_never_crosses
#print axioms QCore.DeclaredAbiAgreement.a_plain_char_admits_both_byte_spellings
#print axioms QCore.DeclaredAbiAgreement.constness_never_crosses
#print axioms QCore.DeclaredAbiAgreement.an_unreadable_projection_admits_nothing

-- Every refusal class, reached, beside the faithful pair that agrees.
#print axioms QCore.DeclaredAbiAgreement.the_faithful_pair_agrees
#print axioms QCore.DeclaredAbiAgreement.a_wrong_result_type_is_refused
#print axioms QCore.DeclaredAbiAgreement.a_wrong_parameter_type_is_refused
#print axioms QCore.DeclaredAbiAgreement.a_swapped_parameter_vector_is_refused
#print axioms QCore.DeclaredAbiAgreement.a_wrong_arity_is_refused
#print axioms QCore.DeclaredAbiAgreement.a_tail_claimed_of_a_non_variadic_callee_is_refused
#print axioms QCore.DeclaredAbiAgreement.a_variadic_callee_without_a_declared_tail_agrees
#print axioms QCore.DeclaredAbiAgreement.an_unreadable_parameter_type_is_refused
#print axioms QCore.DeclaredAbiAgreement.an_unreadable_result_type_is_refused
#print axioms QCore.DeclaredAbiAgreement.an_unspecified_list_refuses_a_declared_vector

-- The zero-arity admission, and the gate that makes it safe.
#print axioms QCore.DeclaredAbiAgreement.a_nullary_contract_survives_an_unspecified_list
#print axioms QCore.DeclaredAbiAgreement.a_tail_claim_never_survives_an_unspecified_list
#print axioms QCore.DeclaredAbiAgreement.a_nullary_contract_admits_only_the_empty_call
#print axioms QCore.DeclaredAbiAgreement.a_nullary_contract_refuses_any_actual

-- The refusal vocabulary is closed and duplicate-free.
#print axioms QCore.DeclaredAbiAgreement.the_refusal_vocabulary_has_eight_members
#print axioms QCore.DeclaredAbiAgreement.the_refusal_vocabulary_has_no_duplicates
#print axioms QCore.DeclaredAbiAgreement.every_refusal_is_in_the_vocabulary

-- Composed-tree reconciliation with card NEIGHBOURS-CHAINED-TAIL's clause T5.
-- The implementation withdraws a declaration by rewriting its call sites, not by
-- clearing the tail set rederived from the retained texts, so the sentence that
-- makes the composition safe is stated rather than assumed: no tail T5 may act
-- on survives a withdrawal, and where the comparison agrees T5 decides exactly
-- as it does without this card. The refusal half is discharged by T5's *own*
-- premise, not by a rule invented here.
#print axioms QCore.DeclaredAbiAgreement.a_withdrawn_declaration_supplies_no_tail
#print axioms QCore.DeclaredAbiAgreement.an_agreeing_declaration_supplies_its_own_tail
#print axioms QCore.DeclaredAbiAgreement.a_withdrawn_declaration_admits_no_tail_actual
#print axioms QCore.DeclaredAbiAgreement.an_agreeing_declaration_leaves_t5_alone

-- Composing two declarations of one name: C11 6.2.7p3, and no third reading.
#print axioms QCore.DeclaredAbiAgreement.a_prototype_and_its_definition_compose
#print axioms QCore.DeclaredAbiAgreement.two_agreeing_declarations_are_one
#print axioms QCore.DeclaredAbiAgreement.two_disagreeing_declarations_are_ambiguous
#print axioms QCore.DeclaredAbiAgreement.disagreeing_results_are_ambiguous
#print axioms QCore.DeclaredAbiAgreement.composition_returns_one_of_its_inputs
