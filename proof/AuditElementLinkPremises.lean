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

import QCore.ElementLinkPremises

/-! Card **LERI-HELD-AND-LINK-MEMBER-PREMISES**: an element destroyer's link
premises, checked (gateway rule 46).

Four directions are audited.

**P-LERI-HELD-LINKS is decided.** A fresh or zeroed link record has no
defined unlink, so rule 43's unconditional held arm admitted an undefined
one; a self-initialisation covering both link fields of one member makes it
unlinkable and half of one does not; the recorded facts survive exactly the
statements that cannot write the object, and the join; the held arm admits
only a recorded member and only replaces admissions with refusals.

**P-LERI-LINK-MEMBER is decided.** An unlink of the member every
`container_of` binding names removes the element from the iterated list; an
unlink of another member leaves it linked while it is freed, and the foreign
arm refuses it.

**A destroyer that names no member** takes neither a held object nor an
element.

**The Rust vectors.** `foreignAdmits`, `initialisedMember` and
`heldDecides` agree with `FOREIGN_VECTORS`, `INIT_VECTORS` and
`HELD_VECTORS`.

Faithful lexical reading of the `container_of` member and of the rule 36b
write set, the kill policy's over-approximation of a statement's writes,
P-CRR-MEMBER and the Rust correspondence are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs`. -/

-- P-LERI-HELD-LINKS.
#print axioms QCore.ElementLinkPremises.a_fresh_object_has_no_defined_unlink
#print axioms QCore.ElementLinkPremises.a_zeroed_object_has_no_defined_unlink
#print axioms QCore.ElementLinkPremises.a_half_initialisation_is_not_enough
#print axioms QCore.ElementLinkPremises.an_initialised_member_is_unlinkable
#print axioms QCore.ElementLinkPremises.an_initialised_member_is_the_path
#print axioms QCore.ElementLinkPremises.retaining_preserves_every_fact
#print axioms QCore.ElementLinkPremises.the_join_preserves_every_fact
#print axioms QCore.ElementLinkPremises.an_initialisation_establishes_its_fact
#print axioms QCore.ElementLinkPremises.the_held_arm_is_sound
#print axioms QCore.ElementLinkPremises.the_unchecked_held_arm_admits_an_undefined_unlink
#print axioms QCore.ElementLinkPremises.the_check_only_refuses

-- P-LERI-LINK-MEMBER.
#print axioms QCore.ElementLinkPremises.an_admitted_unlink_removes_the_element
#print axioms QCore.ElementLinkPremises.a_mismatched_member_leaves_the_element_linked

-- A destroyer that names no member.
#print axioms QCore.ElementLinkPremises.a_memberless_destroyer_refuses
#print axioms QCore.ElementLinkPremises.a_memberless_destroyer_takes_no_element

-- The Rust vectors.
#print axioms QCore.ElementLinkPremises.vectors_hold
