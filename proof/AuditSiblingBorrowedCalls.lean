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

import QCore.SiblingBorrowedCalls

/-! Audited companion to CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3 (gateway rule 67). -/

#print axioms QCore.SiblingBorrowedCalls.a_borrowed_sequence_changes_no_custody
#print axioms QCore.SiblingBorrowedCalls.every_order_leaves_the_same_state
#print axioms QCore.SiblingBorrowedCalls.a_read_between_the_calls_is_decided_alike
#print axioms QCore.SiblingBorrowedCalls.a_releasing_sibling_does_not_commute
#print axioms QCore.SiblingBorrowedCalls.a_releasing_sibling_is_never_admitted
#print axioms QCore.SiblingBorrowedCalls.several_calls_are_read
#print axioms QCore.SiblingBorrowedCalls.no_call_is_never_read
