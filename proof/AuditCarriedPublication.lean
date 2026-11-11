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

import QCore.CarriedPublication

/-! Card **MONKEY-VHOST-PROPERTY-CUSTODY**: a publication carries its owner's
field-linked children, and an unlink hands them back (gateway rule 12d).

Four directions are audited.

**The decision tables.** A child is carried exactly when it is exactly live,
linked on every path, with no unresolved effect, not itself published and
owning nothing; an untracked child and each encumbrance alone keep rule 10d's
withdrawal. An unlink hands back exactly a must-linked, exactly escaped child;
one carried on some paths only refuses. The Rust tables are these
(`publish_vectors_hold`, `unlink_vectors_hold`).

**The `Name` branch.** Monkey's `mk_vhost_set_property` shape is `Safe` under
the carry and was `Unknown` under the withdrawal (non-vacuity).

**Custody comes back.** An unlink returns the owner and the carried child;
releasing the owner alone then leaks the child (P-B08-SHALLOW); releasing
both is `Safe`; releasing a carried child or the published owner is a
violation candidate.

**Refusal is preserved.** An encumbered child, another container, an unlink
without a publication and a second publication refuse; nothing certifies after
a refusal or a violation; the carry only replaces a refusal.

Faithful reading of the flow's status bits, links and sets, and the Rust
correspondence, are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/carried_publication.rs` and
`crates/qcore-c-gateway/tests/monkey_vhost_property_custody.rs`. -/

-- The decision tables.
#print axioms QCore.CarriedPublication.carry_iff
#print axioms QCore.CarriedPublication.an_untracked_child_refuses
#print axioms QCore.CarriedPublication.a_partial_link_refuses
#print axioms QCore.CarriedPublication.an_unresolved_child_refuses
#print axioms QCore.CarriedPublication.a_published_child_refuses
#print axioms QCore.CarriedPublication.a_child_that_owns_refuses
#print axioms QCore.CarriedPublication.restore_iff
#print axioms QCore.CarriedPublication.a_partially_carried_child_refuses
#print axioms QCore.CarriedPublication.a_never_escaped_child_is_kept
#print axioms QCore.CarriedPublication.publish_vectors_hold
#print axioms QCore.CarriedPublication.unlink_vectors_hold

-- The `Name` branch.
#print axioms QCore.CarriedPublication.the_name_branch_is_safe
#print axioms QCore.CarriedPublication.the_withdrawal_refused_the_name_branch

-- Custody comes back.
#print axioms QCore.CarriedPublication.an_unlink_hands_back_both
#print axioms QCore.CarriedPublication.releasing_the_owner_after_the_unlink_leaks_the_child
#print axioms QCore.CarriedPublication.a_full_teardown_after_the_unlink_is_safe
#print axioms QCore.CarriedPublication.releasing_a_carried_child_is_a_violation
#print axioms QCore.CarriedPublication.releasing_the_published_owner_is_a_violation

-- Refusal is preserved.
#print axioms QCore.CarriedPublication.an_encumbered_child_keeps_the_refusal
#print axioms QCore.CarriedPublication.another_container_refuses
#print axioms QCore.CarriedPublication.an_unlink_without_a_publication_refuses
#print axioms QCore.CarriedPublication.a_second_publication_refuses
#print axioms QCore.CarriedPublication.no_step_certifies_after_a_refusal
#print axioms QCore.CarriedPublication.no_step_certifies_after_a_violation
#print axioms QCore.CarriedPublication.the_carry_only_replaces_a_refusal
#print axioms QCore.CarriedPublication.run_refused
#print axioms QCore.CarriedPublication.run_violation
