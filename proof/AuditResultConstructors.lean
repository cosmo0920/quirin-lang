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

import QCore.ResultConstructors

/-! Rule 37: every public companion theorem is audited. -/

#print axioms QCore.ResultConstructors.an_evaluated_dereference_still_refuses
#print axioms QCore.ResultConstructors.another_escape_still_refuses
#print axioms QCore.ResultConstructors.only_a_checked_operand_is_newly_exempt
#print axioms QCore.ResultConstructors.no_checked_operand_keeps_the_old_scan
#print axioms QCore.ResultConstructors.a_nullable_constructor_owner_never_links
#print axioms QCore.ResultConstructors.a_checked_sizeof_constructor_can_link
#print axioms QCore.ResultConstructors.an_evaluated_dereference_refuses_the_same_constructor
#print axioms QCore.ResultConstructors.a_checked_sizeof_does_not_excuse_an_alias
#print axioms QCore.ResultConstructors.an_asserting_wrapper_never_returns_null
#print axioms QCore.ResultConstructors.an_assert_disabled_wrapper_can_return_null
#print axioms QCore.ResultConstructors.an_asserting_allocation_preserves_its_identity
#print axioms QCore.ResultConstructors.a_constructor_return_carries_the_linked_child
#print axioms QCore.ResultConstructors.a_shallow_release_does_not_discharge_the_child
#print axioms QCore.ResultConstructors.a_checked_result_requires_the_independent_source_proof
#print axioms QCore.ResultConstructors.an_ordinary_declaration_keeps_its_result_fact
#print axioms QCore.ResultConstructors.an_independent_source_proof_is_still_usable
#print axioms QCore.ResultConstructors.the_checked_operand_contributes_no_escape_event
