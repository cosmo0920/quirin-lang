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

import QCore.ContainerRootReach

/-! Card **W45-CONTAINER-ROOT-REACH**: static containers and the foreign
unlink (gateway constraint 10d, second paragraph).

Four directions are audited.

**Each admitted root outlives the function, under its premise.** An
unrebound formal reaches the caller's memory; a static object lives for the
program (P-CRR-STATIC).

**Refusal is preserved for every unlisted shape.** Exactly the two roots are
admitted; a local container does not outlive the call; a root with no premise
is refused.

**A foreign unlink transfers custody exactly once.** The element moves from
its container to the body once; a second unlink refuses; the body must then
release or re-publish it: a leak is a leak and a double release is a
violation.

**The summary gate.** A `Borrowed` summary for a draining body would hide a
caller's double release, so such a body is withheld.

The premises P-CRR-MEMBER and P-CRR-LINKED, faithful lexical reading and the
Rust correspondence are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/container_root_reach.rs`. -/

-- Each admitted root outlives the function.
#print axioms QCore.ContainerRootReach.an_admitted_root_outlives_the_call
#print axioms QCore.ContainerRootReach.a_static_root_lives_for_the_program

-- Refusal is preserved for every unlisted shape.
#print axioms QCore.ContainerRootReach.only_the_listed_roots_are_admitted
#print axioms QCore.ContainerRootReach.a_local_container_does_not_outlive_the_call
#print axioms QCore.ContainerRootReach.a_root_without_a_premise_is_refused
#print axioms QCore.ContainerRootReach.an_unlink_from_an_unadmitted_root_is_refused
#print axioms QCore.ContainerRootReach.a_republication_into_a_local_container_is_refused

-- A foreign unlink transfers custody exactly once.
#print axioms QCore.ContainerRootReach.a_foreign_unlink_hands_the_element_to_the_body
#print axioms QCore.ContainerRootReach.a_second_unlink_is_refused
#print axioms QCore.ContainerRootReach.custody_moves_once
#print axioms QCore.ContainerRootReach.a_foreign_unlink_then_release_is_safe
#print axioms QCore.ContainerRootReach.a_foreign_unlink_then_republication_is_safe
#print axioms QCore.ContainerRootReach.a_foreign_unlink_then_leak_is_a_leak
#print axioms QCore.ContainerRootReach.a_foreign_unlink_then_double_release_is_a_violation
#print axioms QCore.ContainerRootReach.a_linked_element_owes_nothing
#print axioms QCore.ContainerRootReach.run_refused
#print axioms QCore.ContainerRootReach.run_violation

-- The summary gate.
#print axioms QCore.ContainerRootReach.the_borrowed_summary_would_hide_a_double_release
#print axioms QCore.ContainerRootReach.the_draining_body_is_withheld
