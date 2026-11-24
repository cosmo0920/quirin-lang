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

import QCore.BorrowedPathLocal

/-! Audited companion to K5-LOCAL-ORIGIN-ALIASES (gateway rule 77). -/

#print axioms QCore.BorrowedPathLocal.decision_vectors_hold
#print axioms QCore.BorrowedPathLocal.a_root_is_admitted_exactly_on_a_read
#print axioms QCore.BorrowedPathLocal.a_step_or_address_withholds_the_local
#print axioms QCore.BorrowedPathLocal.value_vectors_hold
#print axioms QCore.BorrowedPathLocal.binding_vectors_hold
#print axioms QCore.BorrowedPathLocal.a_widened_state_is_never_untracked
#print axioms QCore.BorrowedPathLocal.an_other_value_withholds
#print axioms QCore.BorrowedPathLocal.eval_untracked
#print axioms QCore.BorrowedPathLocal.an_admitted_write_keeps_every_local_untracked
#print axioms QCore.BorrowedPathLocal.every_admitted_local_holds_an_untracked_value
#print axioms QCore.BorrowedPathLocal.the_admitted_guard_leaves_the_old_state
#print axioms QCore.BorrowedPathLocal.the_guard_touches_no_obligation
#print axioms QCore.BorrowedPathLocal.the_guard_narrows_only_its_binding
#print axioms QCore.BorrowedPathLocal.the_admitted_guard_reports_nothing
#print axioms QCore.BorrowedPathLocal.a_withheld_guard_is_the_old_guard
#print axioms QCore.BorrowedPathLocal.a_tracked_binding_keeps_the_old_guard
#print axioms QCore.BorrowedPathLocal.returning_an_untracked_local_escapes_nothing
#print axioms QCore.BorrowedPathLocal.the_old_return_escaped_nothing_either
#print axioms QCore.BorrowedPathLocal.a_release_of_an_untracked_local_still_refuses
