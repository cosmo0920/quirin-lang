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

import QCore.TeardownAdmission

-- Card `P04-NEXT`: admission of a transported teardown proposal. Model of the
-- two `WitnessChecker::check` bodies in
-- `crates/qcore-verification/src/teardown_source.rs`. The declared premises of
-- the pinned contracts — ring membership, exclusive link access, allocation
-- provenance, field disjointness, the `free`/libc binding — are outside every
-- statement below and stay recorded in
-- `docs/reports/remaining-p04-next/README.md`.

-- Agreement discharges nothing: the core's own contract evaluation decides,
-- and an admitted proposal is exactly the reading the core computed.
#print axioms QCore.TeardownAdmission.a_refusing_core_admits_nothing
#print axioms QCore.TeardownAdmission.an_admitted_witness_has_a_core_evaluation
#print axioms QCore.TeardownAdmission.only_the_core_reading_is_admitted
#print axioms QCore.TeardownAdmission.admitted_witnesses_never_disagree
#print axioms QCore.TeardownAdmission.a_changed_core_reading_retires_the_witness

-- Identity binding in all three directions, plus non-vacuity.
#print axioms QCore.TeardownAdmission.a_cross_task_witness_is_refused
#print axioms QCore.TeardownAdmission.a_foreign_task_is_refused
#print axioms QCore.TeardownAdmission.a_cross_family_proposal_is_refused
#print axioms QCore.TeardownAdmission.an_admitted_witness_is_bound_to_its_own_task
#print axioms QCore.TeardownAdmission.the_core_reading_is_admitted

-- The unlink family transports two bodies, so the recorded identity is a
-- function of both: a helper substitution is a different question, never a
-- reuse of the entry's evidence.
#print axioms QCore.TeardownAdmission.a_changed_helper_changes_the_question
#print axioms QCore.TeardownAdmission.a_changed_entry_changes_the_question
#print axioms QCore.TeardownAdmission.a_changed_contract_changes_the_question
#print axioms QCore.TeardownAdmission.an_entry_witness_does_not_survive_a_helper_substitution

-- The destructor's declared owned-field inventory fixes the enumeration
-- domain, and the core accepts only the pinned contract's own inventory.
#print axioms QCore.TeardownAdmission.the_declared_inventory_has_nine_realizable_states
#print axioms QCore.TeardownAdmission.dropping_a_declared_field_shrinks_the_question
#print axioms QCore.TeardownAdmission.the_empty_state_is_always_realizable
#print axioms QCore.TeardownAdmission.a_state_naming_an_undeclared_field_is_unrealizable
#print axioms QCore.TeardownAdmission.a_child_without_its_parent_is_unrealizable
#print axioms QCore.TeardownAdmission.the_full_inventory_is_realizable
#print axioms QCore.TeardownAdmission.only_the_contracts_own_inventory_is_accepted
#print axioms QCore.TeardownAdmission.the_contracts_own_inventory_is_accepted

-- Both families are inhabited and neither answers for the other.
#print axioms QCore.TeardownAdmission.the_unlink_family_admits_a_reading
#print axioms QCore.TeardownAdmission.the_destructor_family_admits_a_reading
#print axioms QCore.TeardownAdmission.a_destructor_reading_never_satisfies_the_unlink_checker
#print axioms QCore.TeardownAdmission.an_unlink_reading_never_satisfies_the_destructor_checker
