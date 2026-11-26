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

import QCore.SettledLeakJoined

/-! Card **SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN**: a leak rule 74c (A)
withdraws is published when a path avoiding every mark of its identity
carries it (gateway rule 84).

Four directions are audited.

**The cut join is the avoiding join.** The replay that drops every marked
edge joins exactly the paths no mark lies on.

**A witness avoids every refused release.** Every refused release marks its
identity and every refusal blocks the replay, so a witnessed path passes no
refused release and no refusal; a leak whose every live path
passes one stays withdrawn.

**The decision.** `publishVectors` (`publish_vectors_hold`); an unwithdrawn
report is untouched.

**Only `Unknown -> Unsafe`.** A re-published report folds its function to
provisional `Unsafe`; the pinned instance publishes, the both-arms control
does not.

Faithful replay and path search in the Rust, and the correspondence with it,
are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/legacy_joined_leak_witness.rs` and
`crates/qcore-c-gateway/tests/settled_leak_joined.rs`. -/

-- The cut join is the avoiding join.
#print axioms QCore.SettledLeakJoined.the_cut_join_is_the_avoiding_join

-- A witness avoids every refused release.
#print axioms QCore.SettledLeakJoined.a_witness_avoids_every_refused_release
#print axioms QCore.SettledLeakJoined.a_witness_passes_no_refusal
#print axioms QCore.SettledLeakJoined.no_witness_through_a_refused_release
#print axioms QCore.SettledLeakJoined.a_refused_only_leak_stays_withdrawn

-- The decision.
#print axioms QCore.SettledLeakJoined.publish_vectors_hold
#print axioms QCore.SettledLeakJoined.an_unwithdrawn_report_is_untouched

-- Only Unknown -> Unsafe.
#print axioms QCore.SettledLeakJoined.a_publication_is_never_safe
#print axioms QCore.SettledLeakJoined.the_pinned_instance
#print axioms QCore.SettledLeakJoined.the_both_arms_control

-- Card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE: every refusal kind.
-- A witness avoids every refusal mark of every kind.
#print axioms QCore.SettledLeakJoined.a_witness_avoids_every_refusal_kind
-- Publication is order-independent.
#print axioms QCore.SettledLeakJoined.any_perm
#print axioms QCore.SettledLeakJoined.held_at_an_unmarked_exit
#print axioms QCore.SettledLeakJoined.publication_is_order_independent
#print axioms QCore.SettledLeakJoined.the_old_decision_read_the_schedule
#print axioms QCore.SettledLeakJoined.both_orders_publish_the_avoiding_arm
-- A refused-only leak stays withdrawn.
#print axioms QCore.SettledLeakJoined.a_refused_only_leak_stays_withdrawn_any_kind
#print axioms QCore.SettledLeakJoined.every_kind_on_both_arms_is_withdrawn
-- A publication is never Safe.
#print axioms QCore.SettledLeakJoined.a_decided_report_is_never_safe

-- Card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF: a mark beside a Pending hold.
-- Order independence, the Pending case included.
#print axioms QCore.SettledLeakJoined.publication_is_order_independent_beside_a_hold
#print axioms QCore.SettledLeakJoined.the_pending_case_read_the_schedule
#print axioms QCore.SettledLeakJoined.both_orders_publish_beside_a_hold
#print axioms QCore.SettledLeakJoined.a_hold_on_both_arms_is_withdrawn
#print axioms QCore.SettledLeakJoined.a_refused_only_leak_stays_withdrawn_beside_a_hold
#print axioms QCore.SettledLeakJoined.a_mark_beside_no_blocking_leaf_is_left_to_rule_74c
-- Every Pending reason is preserved; never Safe.
#print axioms QCore.SettledLeakJoined.a_pending_reason_is_preserved
#print axioms QCore.SettledLeakJoined.a_decision_beside_a_blocking_leaf_is_never_safe
#print axioms QCore.SettledLeakJoined.fold_of_pendings
#print axioms QCore.SettledLeakJoined.a_withdrawal_beside_holds_only_is_pending
