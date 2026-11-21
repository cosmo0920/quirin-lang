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


import QCore.ResultHolderRefusal

/-! Audited companion to CG10-SMALL-BUNDLE (gateway rule 74a). -/

#print axioms QCore.ResultHolderRefusal.the_end_of_record_reading_accepts_the_caller
#print axioms QCore.ResultHolderRefusal.a_member_releasing_destructor_faults
#print axioms QCore.ResultHolderRefusal.the_destructor_read_as_it_is_refuses_the_caller
#print axioms QCore.ResultHolderRefusal.a_destructor_without_member_effect_does_not_fault
#print axioms QCore.ResultHolderRefusal.the_end_of_record_needs_a_destructor_premise
