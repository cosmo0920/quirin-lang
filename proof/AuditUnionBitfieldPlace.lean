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

import QCore.UnionBitfieldPlace

/-! Audited companion to CG13-UNION-AND-BITFIELD-PLACES (gateway rule 10w). -/

#print axioms QCore.UnionBitfieldPlace.decision_vectors_hold
#print axioms QCore.UnionBitfieldPlace.unlisted_forms_are_refused
#print axioms QCore.UnionBitfieldPlace.a_bitfield_pointer_access_is_refused
#print axioms QCore.UnionBitfieldPlace.an_admitted_step_is_listed
#print axioms QCore.UnionBitfieldPlace.a_union_pointer_store_is_refused
#print axioms QCore.UnionBitfieldPlace.a_punned_pointer_read_is_refused
#print axioms QCore.UnionBitfieldPlace.a_union_scalar_read_is_neutral
#print axioms QCore.UnionBitfieldPlace.bitfield_store_is_unit_rmw
#print axioms QCore.UnionBitfieldPlace.a_bitfield_footprint_is_its_unit
#print axioms QCore.UnionBitfieldPlace.disjoint_bits_do_not_make_disjoint_footprints
#print axioms QCore.UnionBitfieldPlace.one_unit_is_never_disjoint
#print axioms QCore.UnionBitfieldPlace.a_union_store_invalidates_sibling_identities
#print axioms QCore.UnionBitfieldPlace.a_custody_free_union_store_is_frame_neutral
#print axioms QCore.UnionBitfieldPlace.a_spared_local_keeps_its_binding
