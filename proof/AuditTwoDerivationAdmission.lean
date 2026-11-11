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

import QCore.TwoDerivationAdmission

-- Card `P04-COMPOSE-FS`: **two** derived units in one question, and two
-- composition prerequisites checked over two different translation units. Model
-- of `qcore_c_gateway::composition_source::import_filesystem_chunk_close_source`
-- and of
-- `qcore_verification::composition_source::FilesystemChunkCloseCompositionSourceTask`.
--
-- The single-derivation half is `QCore.DerivedTextAdmission` and is reused
-- rather than restated. Whether either derived text preprocesses as its base
-- unit does, whether the linker resolves the backend prerequisite's wrapper to
-- the body checked in the other unit, and anything about the OS are outside
-- every statement below and stay recorded in
-- `docs/reports/remaining-p04-compose-fs/README.md`.

-- **The two derivations are two.** Different recipes over one base derive
-- different texts; one recipe used twice is not two derivations; two base texts
-- are not one question; and two recipes deriving one text are refused.
#print axioms QCore.TwoDerivationAdmission.different_recipes_derive_different_texts
#print axioms QCore.TwoDerivationAdmission.one_recipe_twice_is_not_two_derivations
#print axioms QCore.TwoDerivationAdmission.two_base_texts_are_not_one_question
#print axioms QCore.TwoDerivationAdmission.two_recipes_deriving_one_text_are_refused
#print axioms QCore.TwoDerivationAdmission.a_nonempty_recipe_changes_the_text

-- Admission is decided, and every conjunct the core checks for itself refuses
-- on its own: the core's own reading, this family's own derivation, its recipe,
-- its base text, completeness, the nested derivation, the two-derivation
-- separation, each of the two composition prerequisites, the deleting backend
-- arm, the identity set, each transported nested question, and both directions
-- of the cross-unit tie.
#print axioms QCore.TwoDerivationAdmission.admission_is_decided
#print axioms QCore.TwoDerivationAdmission.a_refusing_core_admits_nothing
#print axioms QCore.TwoDerivationAdmission.a_foreign_own_derivation_is_refused
#print axioms QCore.TwoDerivationAdmission.a_substituted_recipe_is_refused
#print axioms QCore.TwoDerivationAdmission.substituted_base_text_is_refused
#print axioms QCore.TwoDerivationAdmission.a_derivation_that_derives_nothing_is_refused
#print axioms QCore.TwoDerivationAdmission.an_incomplete_derivation_is_refused
#print axioms QCore.TwoDerivationAdmission.a_foreign_nested_derivation_is_refused
#print axioms QCore.TwoDerivationAdmission.unseparated_derivations_are_refused
#print axioms QCore.TwoDerivationAdmission.a_refusing_memory_prerequisite_refuses
#print axioms QCore.TwoDerivationAdmission.a_refusing_backend_prerequisite_refuses
#print axioms QCore.TwoDerivationAdmission.a_non_deleting_backend_prerequisite_is_refused
#print axioms QCore.TwoDerivationAdmission.a_foreign_identity_set_is_refused
#print axioms QCore.TwoDerivationAdmission.a_foreign_nested_question_is_refused
#print axioms QCore.TwoDerivationAdmission.a_memory_prerequisite_over_another_unit_is_refused
#print axioms QCore.TwoDerivationAdmission.a_backend_prerequisite_over_this_unit_is_refused
#print axioms QCore.TwoDerivationAdmission.a_backend_prerequisite_at_another_revision_is_refused
#print axioms QCore.TwoDerivationAdmission.a_backend_prerequisite_at_another_prototype_is_refused
#print axioms QCore.TwoDerivationAdmission.a_differing_proposal_is_refused

-- What an admission means: it names both of the core's own derivations, it
-- carries the separation, it establishes both compositions and the whole
-- cross-unit tie, it names the core's own questions and identities, the
-- admitted proposal *is* the core's own reading, two admitted proposals cannot
-- disagree, and every conjunct holds together.
#print axioms QCore.TwoDerivationAdmission.an_admission_names_both_of_the_cores_own_derivations
#print axioms QCore.TwoDerivationAdmission.an_admission_separates_the_two_derivations
#print axioms QCore.TwoDerivationAdmission.an_admission_establishes_both_compositions
#print axioms QCore.TwoDerivationAdmission.an_admission_establishes_the_cross_unit_tie
#print axioms QCore.TwoDerivationAdmission.an_admission_names_the_cores_own_questions
#print axioms QCore.TwoDerivationAdmission.an_admitted_proposal_is_the_cores_own_reading
#print axioms QCore.TwoDerivationAdmission.two_admitted_proposals_agree
#print axioms QCore.TwoDerivationAdmission.admission_implies_every_conjunct

-- Non-vacuity: the rule admits the honest proposal over the honest context,
-- each refusal really refuses on that same context, and the cross-unit
-- expectation is two-sided rather than a one-way convenience.
#print axioms QCore.TwoDerivationAdmission.the_honest_proposal_is_admitted
#print axioms QCore.TwoDerivationAdmission.the_refusals_are_not_vacuous
#print axioms QCore.TwoDerivationAdmission.the_cross_unit_expectation_is_two_sided
