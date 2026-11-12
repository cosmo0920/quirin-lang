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

import QCore.ElementReleaseSummary

/-! Card **LIST-ENTRY-RELEASE-IDENTITY**: the released list element and its
summary (gateway rule 43).

Four directions are audited.

**The held object at an element destroyer is sound.** Every destroy the rule
admits is defined in every concrete state its abstract state describes and
lands in a released state; an object a path already unlinked is refused, and
that refusal is necessary (its destroy is undefined); rule 43 only replaces a
refusal of rule 34.

**The summary gate is exact.** A body's element custody requires exactly the
formals its containers are rooted at; a static or any other root withholds
the summary; the gate accepts exactly the required clauses.

**The clause is sound at the caller.** Whatever the callee released among
the caller's published objects, none is trusted after the call
(P-LERI-CALLER-PUBLISHED); read as `Borrowed`, one would be.

**The Rust vectors.** `required` agrees with `ELEMENT_RELEASE_VECTORS`.

The premises P-CRR-MEMBER, P-CRR-LINKED and P-LERI-CALLER-PUBLISHED, faithful
lexical reading and the Rust correspondence are trust boundary 28's, pinned
by `crates/qcore-c-gateway/tests/element_release_summary.rs`. -/

-- The held object at an element destroyer.
#print axioms QCore.ElementReleaseSummary.the_destroy_rule_is_sound
#print axioms QCore.ElementReleaseSummary.a_held_object_is_destroyed
#print axioms QCore.ElementReleaseSummary.an_unlinked_object_is_refused
#print axioms QCore.ElementReleaseSummary.an_unlinked_object_has_no_defined_destroy
#print axioms QCore.ElementReleaseSummary.a_destroyed_object_cannot_be_released_again
#print axioms QCore.ElementReleaseSummary.the_held_arm_is_the_only_change
#print axioms QCore.ElementReleaseSummary.the_held_arm_only_replaces_a_refusal

-- The summary gate.
#print axioms QCore.ElementReleaseSummary.a_formal_root_is_required
#print axioms QCore.ElementReleaseSummary.only_formal_roots_are_required
#print axioms QCore.ElementReleaseSummary.a_static_root_withholds_the_summary
#print axioms QCore.ElementReleaseSummary.an_other_root_withholds_the_summary
#print axioms QCore.ElementReleaseSummary.the_gate_is_exact
#print axioms QCore.ElementReleaseSummary.a_body_without_element_custody_states_nothing
#print axioms QCore.ElementReleaseSummary.a_withheld_body_never_passes_the_gate

-- The clause at the caller.
#print axioms QCore.ElementReleaseSummary.forgetting_covers_every_release
#print axioms QCore.ElementReleaseSummary.a_forgotten_object_is_refused
#print axioms QCore.ElementReleaseSummary.a_caller_that_published_nothing_is_unchanged
#print axioms QCore.ElementReleaseSummary.the_borrowed_reading_would_trust_a_released_object
#print axioms QCore.ElementReleaseSummary.the_clause_is_carried_on

-- The Rust vectors.
#print axioms QCore.ElementReleaseSummary.vectors_hold
