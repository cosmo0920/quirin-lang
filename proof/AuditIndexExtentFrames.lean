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

import QCore.IndexExtentFrames

/-! Audited companion to CG24-ALIAS-EXTENT-FRAME-VOCABULARY (gateway rule 11e). -/

#print axioms QCore.IndexExtentFrames.touch_pointers_sub
#print axioms QCore.IndexExtentFrames.touch_scalars_sub
#print axioms QCore.IndexExtentFrames.clear_touch
#print axioms QCore.IndexExtentFrames.clear_foldl
#print axioms QCore.IndexExtentFrames.clear_write
#print axioms QCore.IndexExtentFrames.clear_moves_or_writes
#print axioms QCore.IndexExtentFrames.clear_hold
#print axioms QCore.IndexExtentFrames.write_frame_withdraws_its_local
#print axioms QCore.IndexExtentFrames.read_frame_preserves
#print axioms QCore.IndexExtentFrames.write_frame_keeps_unrelated_extents
#print axioms QCore.IndexExtentFrames.all_source_touch
#print axioms QCore.IndexExtentFrames.all_source_foldl
#print axioms QCore.IndexExtentFrames.move_frame_withdraws_declared_facts
#print axioms QCore.IndexExtentFrames.source_kept_by_touch
#print axioms QCore.IndexExtentFrames.unstated_is_the_weakest
#print axioms QCore.IndexExtentFrames.held_frame_holds_declared_facts
#print axioms QCore.IndexExtentFrames.derived_view_within_base
#print axioms QCore.IndexExtentFrames.derived_contradiction_sound
#print axioms QCore.IndexExtentFrames.a_contradicted_base_refuses_the_view
#print axioms QCore.IndexExtentFrames.a_refuted_frame_is_unstated
#print axioms QCore.IndexExtentFrames.a_body_within_its_frame_never_refutes
#print axioms QCore.IndexExtentFrames.a_write_through_a_read_frame_refutes
#print axioms QCore.IndexExtentFrames.a_move_through_a_write_frame_refutes
#print axioms QCore.IndexExtentFrames.frame_vectors_hold
#print axioms QCore.IndexExtentFrames.frame_vectors_count
#print axioms QCore.IndexExtentFrames.derived_vectors_hold
#print axioms QCore.IndexExtentFrames.derived_vectors_count
#print axioms QCore.IndexExtentFrames.refute_vectors_hold
#print axioms QCore.IndexExtentFrames.refute_vectors_count
