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

import QCore.CarriedLinkObligation

/-! Card **FB-RETRY-OR6-ELEMENT-IDENTITY**: a formal's link obligation,
carried to the caller (gateway rule 91).

Four directions are audited.

**The body's admission rests on the clause.** The entry's `ready` records
hold when every caller discharges the clause; they survive exactly the
statements that cannot write the object, and the join; the held arm admits
only on a `ready` record of the declared member, never on a spent one or for
a member-less destroyer.

**The summary is exact.** A carried formal is observed released only when
its record is spent (the carried destroyer took it); an uncarried formal is
observed as before; the observation only refutes.

**Every caller discharges or refuses.** Each way a caller may pass the
actual (its own initialisation, an element its list links through the
member, its own formal carried again) establishes the precondition; any
other actual is refused.

**The Rust vectors.** `admits` and `observedReleased` agree with
`ADMIT_VECTORS` and `OBSERVE_VECTORS`.

The candidates' reading off the destroyer calls, the kill policy's
over-approximation of a statement's writes, every consumer's application or
refusal of a container effect, P-CRR-LINKED and the Rust correspondence are
trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/link_member_premises.rs (module `carried_link_obligation`)`. -/

-- The body's admission.
#print axioms QCore.CarriedLinkObligation.the_seeded_record_holds
#print axioms QCore.CarriedLinkObligation.retaining_keeps_every_ready_record
#print axioms QCore.CarriedLinkObligation.the_join_keeps_every_ready_record
#print axioms QCore.CarriedLinkObligation.a_record_on_one_side_only_is_dropped
#print axioms QCore.CarriedLinkObligation.the_carried_admission_is_sound
#print axioms QCore.CarriedLinkObligation.a_spent_record_admits_nothing
#print axioms QCore.CarriedLinkObligation.a_memberless_destroyer_takes_no_carried_object
#print axioms QCore.CarriedLinkObligation.no_record_no_admission
#print axioms QCore.CarriedLinkObligation.spending_admits_nothing_more

-- The summary is exact.
#print axioms QCore.CarriedLinkObligation.a_carried_release_is_the_destroyers
#print axioms QCore.CarriedLinkObligation.an_uncarried_disposition_is_unchanged
#print axioms QCore.CarriedLinkObligation.the_observation_only_refutes

-- Every caller discharges or refuses.
#print axioms QCore.CarriedLinkObligation.the_discharge_establishes_the_precondition
#print axioms QCore.CarriedLinkObligation.no_discharge_no_admission

-- The Rust vectors.
#print axioms QCore.CarriedLinkObligation.vectors_hold
