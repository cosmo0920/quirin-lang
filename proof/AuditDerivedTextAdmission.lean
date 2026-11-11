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

import QCore.DerivedTextAdmission

-- Card `P04-COMPOSE-MEMCLOSE`: importing from **derived** text, and the
-- fail-closed directions of the derived-identity binding. Model of
-- `qcore_c_gateway::legacy_c`'s derivation helpers and
-- `CSourceInput::for_derived_text`, of
-- `qcore_c_gateway::composition_source::import_memory_close_source`, and of
-- `qcore_verification::composition_source::MemoryCloseCompositionSourceTask`.
--
-- Whether the derived text preprocesses as the base unit does, whether the C
-- body Clang parsed is the body that runs, and anything about the OS are
-- outside every statement below and stay recorded in
-- `docs/reports/remaining-memclose/README.md`.

-- **The measured erasure.** `for_derived_text` records neither the spelled
-- input nor the directory, so two base units with equal bytes and different
-- declared identities derive byte-identical derived identities — while their
-- base identities do not agree. That is why the route binds the base import
-- under its own key.
#print axioms QCore.DerivedTextAdmission.two_declared_files_with_equal_bytes_derive_equal_derived_identities
#print axioms QCore.DerivedTextAdmission.a_declared_file_and_an_anonymous_text_derive_one_identity
#print axioms QCore.DerivedTextAdmission.the_base_identities_are_not_equal
#print axioms QCore.DerivedTextAdmission.the_historical_projection_is_not_collapsed
#print axioms QCore.DerivedTextAdmission.the_derived_identity_reads_only_the_text_and_the_recipe

-- A derivation is a change: a nonempty recipe derives new text and an empty one
-- derives none, so a plain import can never be presented as a derived one.
#print axioms QCore.DerivedTextAdmission.a_nonempty_recipe_derives_new_text
#print axioms QCore.DerivedTextAdmission.an_empty_recipe_is_not_a_derivation

-- Admission is decided, and every conjunct the core checks for itself refuses
-- on its own: the core's own reading, the transported derivation, the recipe,
-- the base text, completeness, each of the four prerequisites, the claimed
-- identity set, the base tie and the proposal itself.
#print axioms QCore.DerivedTextAdmission.admission_is_decided
#print axioms QCore.DerivedTextAdmission.a_refusing_core_admits_nothing
#print axioms QCore.DerivedTextAdmission.a_foreign_derivation_is_refused
#print axioms QCore.DerivedTextAdmission.a_substituted_recipe_is_refused
#print axioms QCore.DerivedTextAdmission.substituted_base_text_is_refused
#print axioms QCore.DerivedTextAdmission.a_derivation_that_derives_nothing_is_refused
#print axioms QCore.DerivedTextAdmission.an_incomplete_derivation_is_refused
#print axioms QCore.DerivedTextAdmission.a_refusing_reset_prerequisite_refuses
#print axioms QCore.DerivedTextAdmission.a_refusing_backend_prerequisite_refuses
#print axioms QCore.DerivedTextAdmission.a_refusing_unlink_prerequisite_refuses
#print axioms QCore.DerivedTextAdmission.a_refusing_counter_prerequisite_refuses
#print axioms QCore.DerivedTextAdmission.a_foreign_identity_set_is_refused
#print axioms QCore.DerivedTextAdmission.a_prerequisite_over_another_unit_is_refused
#print axioms QCore.DerivedTextAdmission.a_differing_proposal_is_refused

-- What an admission means: every conjunct holds, the admitted proposal *is* the
-- core's own reading, two admitted proposals cannot disagree, all four
-- prerequisites are established, and the derivation named is the core's own.
#print axioms QCore.DerivedTextAdmission.admission_implies_every_conjunct
#print axioms QCore.DerivedTextAdmission.an_admitted_proposal_is_the_cores_own_reading
#print axioms QCore.DerivedTextAdmission.two_admitted_proposals_agree
#print axioms QCore.DerivedTextAdmission.an_admission_establishes_all_four_prerequisites
#print axioms QCore.DerivedTextAdmission.an_admission_names_the_cores_own_derivation

-- The two substitution directions are separated by measurement.
#print axioms QCore.DerivedTextAdmission.substituted_base_text_moves_the_derived_digest
#print axioms QCore.DerivedTextAdmission.a_substituted_recipe_moves_the_derived_digest
#print axioms QCore.DerivedTextAdmission.the_recipe_field_separates_two_recipes

-- Non-vacuity: the rule admits the core's own reading, the honest derivation
-- records the erasure, and each refusal really refuses on that context.
#print axioms QCore.DerivedTextAdmission.the_honest_proposal_is_admitted
#print axioms QCore.DerivedTextAdmission.the_honest_derivation_records_the_erasure
#print axioms QCore.DerivedTextAdmission.the_refusals_are_not_vacuous
