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

import QCore.TaskTraceFixedOutcomes

-- Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-F02-FIXED-OUTCOMES`): the F02 observable
-- alphabet grown by the fixed-width worker outcome.

-- The production relation observes a completion's outcome only up to equality
-- and Buffer-ness: a faithful, injective renaming changes no verdict.
#print axioms QCore.TaskTraceFixedOutcomes.rename_injective
#print axioms QCore.TaskTraceFixedOutcomes.checkOrder_rename
#print axioms QCore.TaskTraceFixedOutcomes.relate_is_invariant_under_faithful_renaming

-- The extended relation over a faithful, injective encoding, independent of the
-- encoding chosen.
#print axioms QCore.TaskTraceFixedOutcomes.encodeOutcome_injective
#print axioms QCore.TaskTraceFixedOutcomes.encode_is_injective
#print axioms QCore.TaskTraceFixedOutcomes.encode_is_faithful
#print axioms QCore.TaskTraceFixedOutcomes.xrelate_does_not_depend_on_the_encoding

-- Conservativity: a trace without a fixed outcome relates exactly as before.
#print axioms QCore.TaskTraceFixedOutcomes.fixed_free_traces_relate_exactly_as_before

-- Non-vacuity in both directions.
#print axioms QCore.TaskTraceFixedOutcomes.a_fixed_outcome_run_is_admitted
#print axioms QCore.TaskTraceFixedOutcomes.another_number_is_refused
#print axioms QCore.TaskTraceFixedOutcomes.the_same_number_at_another_type_is_refused
#print axioms QCore.TaskTraceFixedOutcomes.a_fixed_outcome_is_not_an_i32
