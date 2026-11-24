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

import QCore.PointerOffsetPlace
#print axioms QCore.PointerOffsetPlace.deref_offset_is_subscript
#print axioms QCore.PointerOffsetPlace.deref_difference_is_subscript
#print axioms QCore.PointerOffsetPlace.subscript_of_offset
#print axioms QCore.PointerOffsetPlace.offset_vectors_hold
#print axioms QCore.PointerOffsetPlace.a_pointer_difference_is_refused
#print axioms QCore.PointerOffsetPlace.an_integer_minus_a_pointer_is_refused
#print axioms QCore.PointerOffsetPlace.a_non_pointer_result_is_refused
#print axioms QCore.PointerOffsetPlace.an_other_operand_is_refused
#print axioms QCore.PointerOffsetPlace.a_deref_offset_reads_as_the_subscript
#print axioms QCore.PointerOffsetPlace.an_offset_subscript_reads_as_the_subscript
#print axioms QCore.PointerOffsetPlace.the_decision_is_the_subscripts
#print axioms QCore.PointerOffsetPlace.a_nested_offset_is_refused
#print axioms QCore.PointerOffsetPlace.a_refused_offset_keeps_the_refusal
#print axioms QCore.PointerOffsetPlace.a_bare_offset_is_not_a_place
#print axioms QCore.PointerOffsetPlace.every_older_reading_stands
#print axioms QCore.PointerOffsetPlace.the_census_shapes_read
