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

import QCore.InteriorBorrow

/-! Audited companion to SLOT-MOVE-AND-INTERIOR-BORROW: class 8 interior borrows (gateway rule 12y). -/

#print axioms QCore.InteriorBorrow.run_append
#print axioms QCore.InteriorBorrow.endUnlessFamily_refused
#print axioms QCore.InteriorBorrow.step_keeps_refused
#print axioms QCore.InteriorBorrow.run_keeps_refused
#print axioms QCore.InteriorBorrow.a_borrow_release_is_refused
#print axioms QCore.InteriorBorrow.no_borrow_no_family
#print axioms QCore.InteriorBorrow.ended_after_end
#print axioms QCore.InteriorBorrow.a_parent_release_ends_its_borrows
#print axioms QCore.InteriorBorrow.an_invalidating_mutation_ends_a_borrow
#print axioms QCore.InteriorBorrow.a_use_after_the_end_is_refused
#print axioms QCore.InteriorBorrow.in_own_family
#print axioms QCore.InteriorBorrow.a_write_through_a_borrow_keeps_it
#print axioms QCore.InteriorBorrow.a_write_through_a_borrow_does_not_end_it
#print axioms QCore.InteriorBorrow.a_safe_path_never_uses_an_ended_borrow
#print axioms QCore.InteriorBorrow.borrow_trace_vectors_hold
#print axioms QCore.InteriorBorrow.a_lazily_created_child_is_owned_by_the_parent
#print axioms QCore.InteriorBorrow.a_write_through_a_borrow_ends_its_sibling
