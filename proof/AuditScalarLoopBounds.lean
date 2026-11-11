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

import QCore.ScalarLoopBounds

-- Card `W48-QR-SCALAR-TYPES-R5` (`W48-QR-TYPES-LOOP-BOUNDS`): the counted
-- loop's bound and index at every type of the fixed-width table.

-- What an admitted bound is: a positive count inside the ceiling that is a
-- value of the index type, whose range contains every index value.
#print axioms QCore.ScalarLoopBounds.admitted_facts
#print axioms QCore.ScalarLoopBounds.zero_in_range
#print axioms QCore.ScalarLoopBounds.every_index_is_a_value
#print axioms QCore.ScalarLoopBounds.admitted_counts_are_inside_the_ceiling

-- The step `i + 1` is the checked addition and never faults; the loop run with
-- it terminates after exactly `count` iterations, visiting `0 … count-1`.
#print axioms QCore.ScalarLoopBounds.checked_add_of_fits
#print axioms QCore.ScalarLoopBounds.the_step_never_faults
#print axioms QCore.ScalarLoopBounds.run_succ
#print axioms QCore.ScalarLoopBounds.run_from
#print axioms QCore.ScalarLoopBounds.the_loop_runs_exactly_count_times
#print axioms QCore.ScalarLoopBounds.the_count_is_the_iteration_count

-- Refusals, the widths' extremes, and conservativity over `LOOP-V1` on `i32`.
#print axioms QCore.ScalarLoopBounds.a_bool_is_not_a_bound
#print axioms QCore.ScalarLoopBounds.a_target_width_bound_needs_the_target
#print axioms QCore.ScalarLoopBounds.i32_is_loop_v1
#print axioms QCore.ScalarLoopBounds.u64_near_its_maximum_is_refused
#print axioms QCore.ScalarLoopBounds.signed_bounds_near_min_and_max_are_refused
#print axioms QCore.ScalarLoopBounds.u8_reaches_its_maximum_without_fault

-- The vectors replayed through the Rust decision.
#print axioms QCore.ScalarLoopBounds.loopAgreement_holds
