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

import QCore.ThreadLocalRead

/-! Audited companion to THREAD-LOCAL-CUSTODY-FREE-READ (gateway rule 54). -/

#print axioms QCore.ThreadLocalRead.decision_vectors_hold
#print axioms QCore.ThreadLocalRead.admitted_exactly_on_a_custody_free_use
#print axioms QCore.ThreadLocalRead.a_write_to_the_object_withholds
#print axioms QCore.ThreadLocalRead.a_use_through_it_withholds
#print axioms QCore.ThreadLocalRead.a_non_pointer_object_withholds
#print axioms QCore.ThreadLocalRead.the_copy_is_the_return_none_reading
#print axioms QCore.ThreadLocalRead.the_copy_changes_no_obligation
#print axioms QCore.ThreadLocalRead.the_copy_is_unbound
#print axioms QCore.ThreadLocalRead.no_other_binding_changes
#print axioms QCore.ThreadLocalRead.the_withheld_answer_is_unchanged
#print axioms QCore.ThreadLocalRead.a_bound_source_keeps_the_old_answer
#print axioms QCore.ThreadLocalRead.a_release_of_the_copy_refuses
#print axioms QCore.ThreadLocalRead.returning_the_copy_escapes_nothing
