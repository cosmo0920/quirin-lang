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

import QCore.OwnedOutMemberSlot

/-! Audited companion to CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT (gateway rule 72). -/

#print axioms QCore.OwnedOutMemberSlot.decision_vectors_hold
#print axioms QCore.OwnedOutMemberSlot.admitted_iff_untracked
#print axioms QCore.OwnedOutMemberSlot.slot_vectors_hold
#print axioms QCore.OwnedOutMemberSlot.a_dot_first_is_never_a_slot
#print axioms QCore.OwnedOutMemberSlot.an_unaddressed_actual_is_never_a_slot
#print axioms QCore.OwnedOutMemberSlot.another_write_is_never_a_root
#print axioms QCore.OwnedOutMemberSlot.a_written_formal_is_only_a_derived_root
#print axioms QCore.OwnedOutMemberSlot.the_cmetrics_chain_is_rooted
#print axioms QCore.OwnedOutMemberSlot.an_admitted_delivery_keeps_the_frames_obligations
#print axioms QCore.OwnedOutMemberSlot.the_delivered_identity_is_held
#print axioms QCore.OwnedOutMemberSlot.a_tracked_owner_is_refused
#print axioms QCore.OwnedOutMemberSlot.a_frame_obligation_survives
#print axioms QCore.OwnedOutMemberSlot.a_delivery_withholds_the_summary
