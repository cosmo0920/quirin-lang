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

import QCore.NullTriage

/-! Audited companion to NULL-CANDIDATE-TRIAGE (gateway rules 14r-14v). -/

#print axioms QCore.NullTriage.a_correlated_condition_is_never_taken
#print axioms QCore.NullTriage.a_decided_condition_takes_its_arm
#print axioms QCore.NullTriage.free_atom_mem
#print axioms QCore.NullTriage.choose_free
#print axioms QCore.NullTriage.a_validated_path_is_satisfiable
#print axioms QCore.NullTriage.a_validated_finding_faults
#print axioms QCore.NullTriage.a_repeated_free_value_is_not_validated
#print axioms QCore.NullTriage.interval_path_is_satisfiable
#print axioms QCore.NullTriage.triage_never_makes_safe
#print axioms QCore.NullTriage.unsafe_only_for_a_violation_or_a_finding
#print axioms QCore.NullTriage.member_invariant_holds
#print axioms QCore.NullTriage.a_member_declaration_is_never_validated
#print axioms QCore.NullTriage.a_failed_store_withdraws
#print axioms QCore.NullTriage.a_refuting_unit_withdraws
#print axioms QCore.NullTriage.validated_needs_a_validating_body
#print axioms QCore.NullTriage.a_withdrawn_dependency_is_never_safe
#print axioms QCore.NullTriage.travel_never_improves_a_row
#print axioms QCore.NullTriage.admit_vectors_hold
#print axioms QCore.NullTriage.member_vectors_hold
#print axioms QCore.NullTriage.result_vectors_hold
