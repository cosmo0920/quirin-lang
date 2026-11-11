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

import QCore.DeclaredModeAgreement

/-! Card **LOADER-ABI-MODES**: a declared custody *mode*, compared against the
validated summary derived from the callee's own body in the analysed unit.

Card LOADER-ABI closed the *type* half of this gate and recorded the rest as its
own follow-on — "this card compares TYPES; nothing compares a declared mode
against anything the source says" — so a type-faithful declaration with an
unfaithful mode was exactly as trusted as a faithful one. Three such
declarations were measured buying a **caller** a `Safe` verdict its callee's own
body contradicts.

Six directions are audited.

**Only contradictions refuse.** A name this unit does not define, and a body
that yields no validated summary, are compared against nothing and their
declarations apply exactly as before. That is the half that keeps the
declared-trust architecture standing: requiring validation *before* applying a
contract would withdraw every declaration in a corpus whose bodies live in other
translation units. Absence of evidence is not evidence, and it is proved rather
than asserted.

**A faithful declaration is never withdrawn**, for every contract without
exception, and an agreeing summary leaves the declared contract applied
unchanged.

**Refusal only, never repair.** Whatever is applied came from the *declaration*;
the derived summary is never installed in its place. That is what makes the
implementation's choice — rewrite the call sites, do not swap the contract — a
fact rather than an intention, and it is why a derived summary weaker than the
declaration it convicts (one that dropped a non-null precondition, say) can
never become the interface a caller is checked against.

**The relation is the duty inequality, and nothing else.** Two general theorems
say so at one parameter and at the result, so the tables below are corroboration
rather than the claim. A refinement *inside* one duty — nullability, a
`Stored`/`Removed` owner index, a `Reallocated` replaced parameter — is proved
not to contradict in either direction, which is exactly where this card stops;
the existing `SummaryConflict` remains their only report. A length difference
decides nothing on the parameter axis (that is card LOADER-ABI's gate, already
decided there for a function this unit defines) while the result axis is still
decided, so the rule narrows one axis rather than switching the gate off.

**The fixed point is sound in the direction that matters.** Growing the carried
withdrawn set only ever *removes* applied contracts, and a carried name stays
withdrawn whatever a later round decides — which is the property the
implementation needs, because a later round can acquit a name only by losing the
evidence that convicted it.

**Non-vacuity and closure.** Every refusal class is produced by a decided
witness, including the `Out` and `Removed` duties that no reproduction exercised,
so no branch is unreachable and the two classes are not one another in disguise;
the vocabulary is closed and duplicate-free; and the whole finite duty domain is
decided row by row and reduced to one positional checksum that
`crates/qcore-c-gateway/tests/loader_abi_modes.rs` computes independently from
the production relation over the same domain in the same order.

What this is not: a model of a **decision**. That the derived summary really
describes the body, and that a validated W4 summary is sound under the
environment's other declared premises, are premises supplied from outside — the
same trust boundary every W4 summary already sits behind
(`LOADER-ABI-MODES-RELATIVE`). Nothing here says a withdrawn declaration is
wrong about the C; it says this analysis will not apply it here. -/

-- Only contradictions refuse: the two structurally uncheckable cases.
#print axioms QCore.DeclaredModeAgreement.absence_of_evidence_applies_the_declaration
#print axioms QCore.DeclaredModeAgreement.an_undeclared_callee_is_never_applied

-- A faithful declaration is never withdrawn.
#print axioms QCore.DeclaredModeAgreement.equality_never_contradicts
#print axioms QCore.DeclaredModeAgreement.an_agreeing_summary_applies_the_declaration

-- A contradiction withdraws, through the relation's own result.
#print axioms QCore.DeclaredModeAgreement.a_contradiction_withdraws
#print axioms QCore.DeclaredModeAgreement.a_compared_contradiction_withdraws

-- The three reproduced classes, plus the two duties no reproduction exercised.
#print axioms QCore.DeclaredModeAgreement.a_borrowed_parameter_the_body_releases_contradicts
#print axioms QCore.DeclaredModeAgreement.a_released_parameter_the_body_retains_contradicts
#print axioms QCore.DeclaredModeAgreement.an_absent_result_the_body_produces_contradicts
#print axioms QCore.DeclaredModeAgreement.a_delivered_slot_the_body_never_writes_contradicts
#print axioms QCore.DeclaredModeAgreement.a_restored_child_the_body_never_detaches_contradicts

-- The relation is the duty inequality, stated generally rather than sampled.
#print axioms QCore.DeclaredModeAgreement.a_single_parameter_contradicts_exactly_when_its_duty_differs
#print axioms QCore.DeclaredModeAgreement.a_result_contradicts_exactly_when_its_duty_differs
#print axioms QCore.DeclaredModeAgreement.the_parameter_refusal_takes_precedence

-- Where the card deliberately stops: a refinement inside one duty.
#print axioms QCore.DeclaredModeAgreement.nullability_is_not_a_contradiction
#print axioms QCore.DeclaredModeAgreement.an_owner_index_is_not_a_contradiction
#print axioms QCore.DeclaredModeAgreement.reallocation_and_ownership_do_not_contradict

-- An arity difference narrows one axis rather than switching the gate off.
#print axioms QCore.DeclaredModeAgreement.an_arity_difference_decides_nothing_on_the_parameter_axis
#print axioms QCore.DeclaredModeAgreement.an_arity_difference_still_decides_the_result_axis

-- Refusal only, never repair; and withdrawal only removes.
#print axioms QCore.DeclaredModeAgreement.an_applied_contract_is_the_declared_one
#print axioms QCore.DeclaredModeAgreement.a_derived_summary_is_never_applied
#print axioms QCore.DeclaredModeAgreement.withdrawal_only_removes

-- The fixed point: growing the carried set only removes, and carrying holds.
#print axioms QCore.DeclaredModeAgreement.growing_the_carried_set_only_removes
#print axioms QCore.DeclaredModeAgreement.a_carried_name_stays_withdrawn

-- The relation is about a pair, not about a side.
#print axioms QCore.DeclaredModeAgreement.parametersContradict_symm
#print axioms QCore.DeclaredModeAgreement.parameterAxis_symm
#print axioms QCore.DeclaredModeAgreement.resultAxis_symm
#print axioms QCore.DeclaredModeAgreement.the_relation_is_symmetric

-- Non-vacuity and closure of the vocabulary.
#print axioms QCore.DeclaredModeAgreement.every_refusal_is_reachable
#print axioms QCore.DeclaredModeAgreement.the_refusal_vocabulary_has_two_members
#print axioms QCore.DeclaredModeAgreement.the_refusal_vocabulary_has_no_duplicates
#print axioms QCore.DeclaredModeAgreement.every_refusal_is_in_the_vocabulary

-- The whole finite duty domain, decided and tied to the Rust test by checksum.
#print axioms QCore.DeclaredModeAgreement.the_domain_is_the_size_it_claims
#print axioms QCore.DeclaredModeAgreement.the_parameter_domain_is_decided
#print axioms QCore.DeclaredModeAgreement.the_result_domain_is_decided
#print axioms QCore.DeclaredModeAgreement.the_checksum_is_pinned
