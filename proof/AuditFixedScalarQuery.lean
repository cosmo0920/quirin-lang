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

import QCore.FixedScalarQuery
#print axioms QCore.FixedScalarQuery.erasure_preserves_effects
#print axioms QCore.FixedScalarQuery.variable_array_refuses
#print axioms QCore.FixedScalarQuery.opaque_operand_refuses
#print axioms QCore.FixedScalarQuery.alignof_local_refuses
#print axioms QCore.FixedScalarQuery.scalar_queries_preserve_a_sibling_call
#print axioms QCore.FixedScalarQuery.query_assignment_preserves_custody
#print axioms QCore.FixedScalarQuery.query_assignment_forgets_old_value
#print axioms QCore.FixedScalarQuery.other_scalar_facts_survive
#print axioms QCore.FixedScalarQuery.query_assignment_keeps_both_branches
