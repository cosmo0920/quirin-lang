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

import QCore.OwnedFieldCopyReach

-- Card `OWNED-FIELD-ROUTE-REACH` (wave 41): the copy bind's dichotomy — which
-- names the route asks the ownership IR about, and why the ones it does not
-- ask about are safe to copy.

-- The copy rule, totally: every bound place refuses, whatever it is bound to.
-- One arm turned into a success breaks the first of these.
#print axioms QCore.OwnedFieldCopyReach.copy_refuses_every_bound_place
#print axioms QCore.OwnedFieldCopyReach.an_aggregate_binding_is_refused_however_hollow
#print axioms QCore.OwnedFieldCopyReach.an_owner_binding_is_refused_by_the_same_arm

-- The route's decision: it refuses exactly the bound names, it is total, and
-- nothing is both admitted and refused.
#print axioms QCore.OwnedFieldCopyReach.the_route_refuses_exactly_the_bound_names
#print axioms QCore.OwnedFieldCopyReach.the_decision_is_total
#print axioms QCore.OwnedFieldCopyReach.nothing_is_both_admitted_and_refused

-- The admitting side — the card's one new admission, and the invariant-1
-- argument for it: an admitted copy's source owns no resource (under the
-- checker's own ledger-coherence invariant, named as a premise) and the step
-- is the identity on the ledger.
#print axioms QCore.OwnedFieldCopyReach.an_admitted_copy_owns_no_resource
#print axioms QCore.OwnedFieldCopyReach.an_admitted_step_is_the_identity_on_the_ledger

-- Both halves as one statement: a copy either refuses, or moves no duty.
#print axioms QCore.OwnedFieldCopyReach.a_copy_either_refuses_or_moves_no_duty
