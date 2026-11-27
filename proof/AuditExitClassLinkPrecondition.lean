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

import QCore.ExitClassLinkPrecondition

/-! Card **LERI-HELD-FORMAL-LINK-OBLIGATION**: the link precondition
without the unlink, per exit class (gateway rule 91 (5)).

Four directions are audited.

**The exit states no unlink.** A formal the precondition alone carries is
observed released only when the carried destroyer did not take it; rule
91's formals and uncarried formals read as before; the observation only
refutes.

**One discharge seeds every class.** Classes that state different entry
obligations publish nothing; when they agree, a discharge at the pre-call
state establishes every class's entry seed.

**Every caller discharges or refuses.** A published object, the caller's own
initialisation of the member, or its own formal with an unspent record of
the member establishes the precondition; a foreign element or a spent record
does not; the check spends nothing.

**The Rust vectors.** `observedReleasedFor` and `preDecides` agree with
`PRE_OBSERVE_VECTORS` and `PRE_DISCHARGE_VECTORS`.

P-PUBLISHED-LINKED, the candidates' reading, the kill policy, every
consumer's check or refusal of the clause, route G's per-edge reading and
the Rust correspondence are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs`. -/

-- The exit states no unlink.
#print axioms QCore.ExitClassLinkPrecondition.a_precondition_release_is_not_the_destroyers
#print axioms QCore.ExitClassLinkPrecondition.a_destroyer_clause_reads_as_rule_91
#print axioms QCore.ExitClassLinkPrecondition.an_uncarried_formal_reads_as_before
#print axioms QCore.ExitClassLinkPrecondition.the_observation_only_refutes_for_every_entry

-- One discharge seeds every class.
#print axioms QCore.ExitClassLinkPrecondition.disagreeing_classes_publish_nothing
#print axioms QCore.ExitClassLinkPrecondition.agreeing_classes_share_one_obligation
#print axioms QCore.ExitClassLinkPrecondition.one_discharge_seeds_every_class

-- Every caller discharges or refuses.
#print axioms QCore.ExitClassLinkPrecondition.the_precondition_discharge_establishes_it
#print axioms QCore.ExitClassLinkPrecondition.a_foreign_element_does_not_discharge_a_precondition
#print axioms QCore.ExitClassLinkPrecondition.a_spent_record_does_not_discharge_a_precondition
#print axioms QCore.ExitClassLinkPrecondition.checking_spends_nothing

-- The Rust vectors.
#print axioms QCore.ExitClassLinkPrecondition.pre_decides_is_the_discharge
#print axioms QCore.ExitClassLinkPrecondition.vectors_hold
