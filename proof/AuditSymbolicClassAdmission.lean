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

import QCore.SymbolicClassAdmission

-- Card `P04-COMPOSE-UNSUPPORTED`: the **symbolic** other-backend
-- `cio_chunk_close` classification through the process boundary. Model of
-- `qcore_verification::unsupported_close_composition`'s symbolic replay domain
-- and of
-- `qcore_verification::composition_source::UnsupportedCloseCompositionSourceTask`.
--
-- Whether either derived text preprocesses as its base unit does, whether any
-- caller can reach the symbolic class at all, the C effective-type rules that
-- make the discriminant an `Int`, and anything about the OS are outside every
-- statement below and stay recorded in
-- `docs/reports/remaining-p04-compose-unsupported/README.md`.

-- **The class is a class, not a sampled value.** It never decides an equality
-- true; it decides `false` for exactly the two excluded constants; it refuses
-- for every integer it contains; a truth test refuses exactly when `0` is
-- inside it, and is decided only when one excluded constant is zero.
#print axioms QCore.SymbolicClassAdmission.the_class_never_decides_an_equality_true
#print axioms QCore.SymbolicClassAdmission.an_excluded_constant_decides_false
#print axioms QCore.SymbolicClassAdmission.an_undecided_equality_refuses
#print axioms QCore.SymbolicClassAdmission.a_truth_test_refuses_when_zero_is_in_the_class
#print axioms QCore.SymbolicClassAdmission.a_truth_test_is_decided_only_when_zero_is_excluded

-- **Both masquerade directions.** A supported discriminant is never symbolic; no
-- symbolic member is ever supported; the symbolic arm is exactly the class;
-- collapsed constants hide the filesystem arm, and distinct constants reach
-- both, so requiring them to differ is not vacuous.
#print axioms QCore.SymbolicClassAdmission.a_supported_kind_is_never_symbolic
#print axioms QCore.SymbolicClassAdmission.a_symbolic_member_is_never_supported
#print axioms QCore.SymbolicClassAdmission.the_symbolic_arm_is_exactly_the_class
#print axioms QCore.SymbolicClassAdmission.collapsed_constants_hide_the_filesystem_arm
#print axioms QCore.SymbolicClassAdmission.distinct_constants_reach_both_arms

-- Admission is decided, and every conjunct the core checks for itself refuses on
-- its own: the core's own reading, the family tag, the claimed identity, the
-- transported nested question, the one composition prerequisite, the unit tie,
-- a collapsed class, a differing proposal, a moved excluded constant and a
-- retargeted path.
#print axioms QCore.SymbolicClassAdmission.admission_is_decided
#print axioms QCore.SymbolicClassAdmission.a_refusing_core_admits_nothing
#print axioms QCore.SymbolicClassAdmission.a_foreign_family_tag_is_refused
#print axioms QCore.SymbolicClassAdmission.the_symbolic_tag_is_no_supported_familys
#print axioms QCore.SymbolicClassAdmission.a_foreign_identity_is_refused
#print axioms QCore.SymbolicClassAdmission.a_foreign_nested_question_is_refused
#print axioms QCore.SymbolicClassAdmission.a_refusing_prerequisite_refuses
#print axioms QCore.SymbolicClassAdmission.a_prerequisite_over_another_unit_is_refused
#print axioms QCore.SymbolicClassAdmission.a_collapsed_class_is_refused
#print axioms QCore.SymbolicClassAdmission.a_differing_proposal_is_refused
#print axioms QCore.SymbolicClassAdmission.a_moved_excluded_constant_is_refused
#print axioms QCore.SymbolicClassAdmission.a_retargeted_path_is_refused

-- What an admission means: it establishes the composition and, through it, three
-- levels below; it names the core's own question and identity; it establishes
-- the unit tie and the symbolic tag; the admitted proposal *is* the core's own
-- reading; that reading names a real class; and two admitted proposals cannot
-- disagree.
#print axioms QCore.SymbolicClassAdmission.an_admission_establishes_the_composition
#print axioms QCore.SymbolicClassAdmission.an_admission_names_the_cores_own_question
#print axioms QCore.SymbolicClassAdmission.an_admission_establishes_the_unit_tie
#print axioms QCore.SymbolicClassAdmission.an_admitted_proposal_is_the_cores_own_reading
#print axioms QCore.SymbolicClassAdmission.an_admitted_reading_names_a_real_class
#print axioms QCore.SymbolicClassAdmission.two_admitted_proposals_agree

-- **And what it deliberately does not mean.** The arm publishes `Unknown` for
-- every admitted proposal, no proposal moves that conclusion, and no admission
-- ever permits the nested discharge. The gathered conjunction carries both
-- halves at once.
#print axioms QCore.SymbolicClassAdmission.an_admission_publishes_unknown
#print axioms QCore.SymbolicClassAdmission.no_proposal_moves_the_conclusion
#print axioms QCore.SymbolicClassAdmission.an_admission_never_permits_the_nested_discharge
#print axioms QCore.SymbolicClassAdmission.admission_implies_every_conjunct

-- Non-vacuity: the rule admits the honest proposal over the honest context, and
-- the core-refusal, family-tag, unit-tie and collapsed-class refusals each
-- really refuse on that same context.
#print axioms QCore.SymbolicClassAdmission.the_honest_proposal_is_admitted
#print axioms QCore.SymbolicClassAdmission.the_refusals_are_not_vacuous
#print axioms QCore.SymbolicClassAdmission.the_family_tag_refusal_is_not_vacuous
#print axioms QCore.SymbolicClassAdmission.the_unit_tie_is_not_vacuous
#print axioms QCore.SymbolicClassAdmission.the_collapsed_class_refusal_is_not_vacuous
