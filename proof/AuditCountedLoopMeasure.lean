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

import QCore.CountedLoopMeasure

/-! Audited companion to W45-FB-ENGINE-PROGRESS-MEASURE (a checked measure for a counted loop, gateway rule 44). -/

#print axioms QCore.CountedLoopMeasure.measure_positive_while_running
#print axioms QCore.CountedLoopMeasure.measure_is_the_distance
#print axioms QCore.CountedLoopMeasure.measure_strictly_decreases
#print axioms QCore.CountedLoopMeasure.measure_decreases_by_one
#print axioms QCore.CountedLoopMeasure.step_stays_in_counter_range
#print axioms QCore.CountedLoopMeasure.a_prefix_of_a_run_is_a_run
#print axioms QCore.CountedLoopMeasure.run_length_bounded
#print axioms QCore.CountedLoopMeasure.fixed_bound_runs
#print axioms QCore.CountedLoopMeasure.no_iteration_when_init_reaches_bound
#print axioms QCore.CountedLoopMeasure.measure_along_a_run
#print axioms QCore.CountedLoopMeasure.in_range_conversion_is_identity
#print axioms QCore.CountedLoopMeasure.wrap_step_in_range
#print axioms QCore.CountedLoopMeasure.a_wrapping_counter_never_exits
#print axioms QCore.CountedLoopMeasure.a_written_bound_never_exits
#print axioms QCore.CountedLoopMeasure.an_aliased_counter_reset_never_exits
#print axioms QCore.CountedLoopMeasure.double_step_even
#print axioms QCore.CountedLoopMeasure.a_double_increment_never_exits
#print axioms QCore.CountedLoopMeasure.a_less_or_equal_test_never_exits
#print axioms QCore.CountedLoopMeasure.an_admitted_conjunct_meets_its_premises
#print axioms QCore.CountedLoopMeasure.conjunct_stage_admits
#print axioms QCore.CountedLoopMeasure.an_admitted_shape_meets_the_premises
#print axioms QCore.CountedLoopMeasure.a_while_loop_is_refused
#print axioms QCore.CountedLoopMeasure.a_non_increment_step_is_refused
#print axioms QCore.CountedLoopMeasure.an_address_taken_counter_is_refused
#print axioms QCore.CountedLoopMeasure.a_double_increment_is_refused
#print axioms QCore.CountedLoopMeasure.a_non_less_than_test_is_refused
#print axioms QCore.CountedLoopMeasure.a_wrapping_bound_is_refused
#print axioms QCore.CountedLoopMeasure.a_written_bound_is_refused
#print axioms QCore.CountedLoopMeasure.an_address_taken_wide_bound_is_refused
#print axioms QCore.CountedLoopMeasure.a_converting_comparison_is_refused
#print axioms QCore.CountedLoopMeasure.recording_a_fact_keeps_the_verdict
#print axioms QCore.CountedLoopMeasure.recording_facts_keeps_the_verdict
#print axioms QCore.CountedLoopMeasure.decide_vectors_hold
