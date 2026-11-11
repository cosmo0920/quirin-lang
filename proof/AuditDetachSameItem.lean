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

import QCore.DetachSameItem

#print axioms QCore.DetachSameItem.decision_vectors_hold
#print axioms QCore.DetachSameItem.restoration_needs_checked_membership
#print axioms QCore.DetachSameItem.the_result_is_the_same_item
#print axioms QCore.DetachSameItem.restoration_does_not_allocate_or_release
#print axioms QCore.DetachSameItem.the_parent_identity_and_aliases_are_preserved
#print axioms QCore.DetachSameItem.the_item_leaves_parent_custody
#print axioms QCore.DetachSameItem.every_other_child_is_preserved
#print axioms QCore.DetachSameItem.the_same_item_has_direct_custody
#print axioms QCore.DetachSameItem.null_transfers_nothing
#print axioms QCore.DetachSameItem.direct_custody_stays_unique
#print axioms QCore.DetachSameItem.custody_identity_inventory_is_preserved
#print axioms QCore.DetachSameItem.the_distinct_parent_is_not_the_result
#print axioms QCore.DetachSameItem.the_item_links_are_cleared
#print axioms QCore.DetachSameItem.the_head_changes_only_when_detaching_the_first
#print axioms QCore.DetachSameItem.the_successor_links_to_the_old_predecessor_or_tail
#print axioms QCore.DetachSameItem.detaching_the_tail_updates_the_head_tail_link
#print axioms QCore.DetachSameItem.every_object_outside_the_write_frame_is_preserved
