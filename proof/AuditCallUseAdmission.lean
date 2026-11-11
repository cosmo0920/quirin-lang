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

import QCore.CallUseAdmission

-- Card `P04-NEXT-AB`: the call-result def/use vocabulary and the admission of a
-- transported proposal. Model of `qcore_c_gateway::call_use_source`'s projector
-- and of `qcore_verification::call_use_source`'s `WitnessChecker::check` body.
-- Successful OS cleanup, descriptor and mapping custody and the
-- `close`/`munmap`/`unlink` ABI bindings are outside every statement below and
-- stay recorded in `docs/reports/remaining-p04-next-ab/README.md`.

-- The vocabulary is closed: scan loops, nonreturning terminations, expression
-- forms outside the closed list and callees outside the closed table all have
-- no projection, and one refused operation refuses the whole path.
#print axioms QCore.CallUseAdmission.a_scan_loop_has_no_projection
#print axioms QCore.CallUseAdmission.a_termination_has_no_projection
#print axioms QCore.CallUseAdmission.an_expression_outside_the_vocabulary_has_no_projection
#print axioms QCore.CallUseAdmission.an_unadmitted_callee_has_no_projection
#print axioms QCore.CallUseAdmission.a_refused_operation_refuses_the_path
#print axioms QCore.CallUseAdmission.a_refused_tail_refuses_the_path
#print axioms QCore.CallUseAdmission.a_path_with_a_scan_loop_has_no_projection

-- The closed callee table is part of the question, and it is load-bearing.
#print axioms QCore.CallUseAdmission.a_narrower_table_never_admits_more
#print axioms QCore.CallUseAdmission.the_table_decides_a_real_body

-- Invariants of the def/use reading, established over every path in the
-- vocabulary rather than over sampled ones.
#print axioms QCore.CallUseAdmission.guard_uses_are_uses
#print axioms QCore.CallUseAdmission.every_reading_has_guard_uses_inside_uses
#print axioms QCore.CallUseAdmission.a_call_free_path_reads_as_no_calls

-- Non-vacuity: the two real body shapes read as the audit measured them, and a
-- dropped guard changes the reading instead of being normalized away.
#print axioms QCore.CallUseAdmission.an_ignored_result_reads_as_ignored
#print axioms QCore.CallUseAdmission.a_stored_and_guarded_result_reads_as_both
#print axioms QCore.CallUseAdmission.dropping_the_guard_changes_the_reading

-- Agreement discharges nothing: the core's own contract evaluation decides,
-- and an admitted proposal is exactly the reading the core computed.
#print axioms QCore.CallUseAdmission.a_refusing_core_admits_nothing
#print axioms QCore.CallUseAdmission.an_admitted_witness_has_a_core_evaluation
#print axioms QCore.CallUseAdmission.only_the_core_reading_is_admitted
#print axioms QCore.CallUseAdmission.admitted_witnesses_never_disagree
#print axioms QCore.CallUseAdmission.a_changed_core_reading_retires_the_witness
#print axioms QCore.CallUseAdmission.a_cross_task_witness_is_refused
#print axioms QCore.CallUseAdmission.a_foreign_task_is_refused
#print axioms QCore.CallUseAdmission.the_core_reading_is_admitted

-- The dependent-capability boundary: a prerequisite is re-established by the
-- core, never accepted from a request. Every field of the identity is
-- load-bearing and a refusing prerequisite can never be established.
#print axioms QCore.CallUseAdmission.a_foreign_prerequisite_is_refused
#print axioms QCore.CallUseAdmission.a_refusing_prerequisite_is_never_established
#print axioms QCore.CallUseAdmission.every_prerequisite_field_is_load_bearing
#print axioms QCore.CallUseAdmission.the_derived_prerequisite_is_established
#print axioms QCore.CallUseAdmission.a_prerequisite_does_not_cross_tasks
