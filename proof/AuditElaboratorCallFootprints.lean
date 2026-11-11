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

import QCore.ElaboratorCallFootprints

/-! Audited companion to CG05-ELABORATOR-CALL-FOOTPRINTS (gateway rule 11n). -/

#print axioms QCore.ElaboratorCallFootprints.a_footprintless_call_is_refused
#print axioms QCore.ElaboratorCallFootprints.empty_map_is_check_with
#print axioms QCore.ElaboratorCallFootprints.callPair_symm
#print axioms QCore.ElaboratorCallFootprints.a_scalar_destination_read_commutes_with_a_call
#print axioms QCore.ElaboratorCallFootprints.a_disjoint_read_observes_alike
#print axioms QCore.ElaboratorCallFootprints.the_compound_assignment_runs_alike
#print axioms QCore.ElaboratorCallFootprints.a_read_through_the_written_identity_is_refused
#print axioms QCore.ElaboratorCallFootprints.an_address_taken_destination_is_refused
#print axioms QCore.ElaboratorCallFootprints.two_identity_writes_are_refused
#print axioms QCore.ElaboratorCallFootprints.aliased_den_sound
#print axioms QCore.ElaboratorCallFootprints.aliased_den_meets_the_premises
#print axioms QCore.ElaboratorCallFootprints.a_local_write_respects_its_footprint
#print axioms QCore.ElaboratorCallFootprints.the_address_boundary_is_necessary
#print axioms QCore.ElaboratorCallFootprints.footprint_vectors_hold
#print axioms QCore.ElaboratorCallFootprints.footprint_vectors_count
