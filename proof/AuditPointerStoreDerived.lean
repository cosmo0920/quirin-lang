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

import QCore.PointerStoreDerived

/-! Audited companion to K3-POINTER-VALUED-STORES (gateway rule 10v amended). -/

#print axioms QCore.PointerStoreDerived.a_non_integer_offset_refuses
#print axioms QCore.PointerStoreDerived.an_integer_cast_refuses
#print axioms QCore.PointerStoreDerived.another_root_refuses
#print axioms QCore.PointerStoreDerived.a_place_past_the_root_refuses
#print axioms QCore.PointerStoreDerived.admitted_only_from_the_own_object
#print axioms QCore.PointerStoreDerived.a_refused_reading_keeps_the_refusal
#print axioms QCore.PointerStoreDerived.an_admitted_reading_is_decided_by_the_owner
#print axioms QCore.PointerStoreDerived.reading_vectors_hold
#print axioms QCore.PointerStoreDerived.offset_keeps_provenance
#print axioms QCore.PointerStoreDerived.readMember_mem
#print axioms QCore.PointerStoreDerived.a_derived_store_adds_no_reference
#print axioms QCore.PointerStoreDerived.a_foreign_value_may_add_a_reference
#print axioms QCore.PointerStore.decision_vectors_hold
#print axioms QCore.PointerStore.admitted_exactly_when
