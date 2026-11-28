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

import QCore.SettledLeakLeafless

/-! Card **SETTLED-LEAK-MARK-LEAFLESS**: a live identity a settled exit marks
beside no non-`Safe` leaf is decided from the settled states; unwitnessed, the
function records the `Unknown` reason "mark without refusal" (gateway rule 84,
amended).

Five directions are audited.

**The reason is `Unknown`.** Never `Safe`, never `Unsafe`.

**Never `Safe`.** A decided candidate leaves a leak or the reason; a
withdrawal in a leafless row is exactly `Unknown`.

**Schedule independence.** Two schedules of the same runs decide the same
beside a blocking leaf, for a claimable identity, or at an unmarked exit; the
base decision read the schedule.

**Agreement.** Beside a blocking leaf the decision is rule 84's and the
reason never fires; a witnessed mark is published.

**The fixtures and the table.** Both orders publish the open arm; both
orders record the reason for the arm through a retained exit;
`outcomeVectors`.

The correspondence with the Rust is trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/legacy_mark_leafless.rs` and
`crates/qcore-c-gateway/tests/cross_unit_elements_released/mark_leafless.rs`. -/

-- The reason is Unknown.
#print axioms QCore.SettledLeakLeafless.the_reason_is_unknown
#print axioms QCore.SettledLeakLeafless.the_reason_is_never_safe_or_unsafe

-- Never Safe.
#print axioms QCore.SettledLeakLeafless.a_leafless_decision_is_never_safe
#print axioms QCore.SettledLeakLeafless.a_withdrawal_in_a_leafless_row_is_unknown

-- Schedule independence.
#print axioms QCore.SettledLeakLeafless.outcome_is_order_independent
#print axioms QCore.SettledLeakLeafless.the_leafless_case_read_the_schedule

-- Agreement.
#print axioms QCore.SettledLeakLeafless.agrees_with_rule_84_beside_a_blocking_leaf
#print axioms QCore.SettledLeakLeafless.a_witnessed_mark_is_published
#print axioms QCore.SettledLeakLeafless.an_unclaimable_mark_is_left_to_rule_74c

-- The fixtures and the table.
#print axioms QCore.SettledLeakLeafless.both_orders_publish_the_open_arm
#print axioms QCore.SettledLeakLeafless.both_orders_record_the_reason
#print axioms QCore.SettledLeakLeafless.outcome_vectors_hold
