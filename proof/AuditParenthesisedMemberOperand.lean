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

import QCore.ParenthesisedMemberOperand

/-! Audited companion to K1-CALL-ACTUAL-ACCESS (gateway rule 75). -/

#print axioms QCore.ParenthesisedMemberOperand.decision_vectors_hold
#print axioms QCore.ParenthesisedMemberOperand.a_whole_address_is_seeded
#print axioms QCore.ParenthesisedMemberOperand.only_a_member_operand_is_dropped
#print axioms QCore.ParenthesisedMemberOperand.at_least_the_plain_reader
#print axioms QCore.ParenthesisedMemberOperand.agrees_with_the_plain_reader_on_the_arrow
#print axioms QCore.ParenthesisedMemberOperand.the_old_reading_is_kept_without_the_arm
#print axioms QCore.ParenthesisedMemberOperand.the_closure_reads_with_the_arm
#print axioms QCore.ParenthesisedMemberOperand.every_whole_address_is_seeded
#print axioms QCore.ParenthesisedMemberOperand.the_closure_only_drops_seeds
