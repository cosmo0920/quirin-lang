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

import QCore.RegistrationById

/-! Audited companion to FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (gateway rule 12n, contract specification C04.19.3; USER DECISION 2026-10-16, "Result + list"): a registration forwarded through an id, validated against the body that creates one record, links it into the list, stores the callback into the member and returns the record's id, contributes exactly the store a direct registration does; an id that resolves to no record, or to two, never decides. -/

#print axioms QCore.RegistrationById.storesOf_of_validates
#print axioms QCore.RegistrationById.a_forwarded_registration_is_the_in_body_store
#print axioms QCore.RegistrationById.a_direct_store_contributes_its_callback
#print axioms QCore.RegistrationById.a_validated_body_returns_only_its_record
#print axioms QCore.RegistrationById.two_records_never_validate
#print axioms QCore.RegistrationById.an_unlinked_record_never_validates
#print axioms QCore.RegistrationById.an_unresolved_or_ambiguous_id_never_decides
#print axioms QCore.RegistrationById.a_unique_id_resolves_its_record
#print axioms QCore.RegistrationById.vectors_hold
