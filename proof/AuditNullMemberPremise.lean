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

import QCore.NullMemberPremise

/-! Audited companion to LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (gateway rules 14x, 14z: premised member facts). -/

#print axioms QCore.NullMemberPremise.mem_premisesOf
#print axioms QCore.NullMemberPremise.a_resting_row_lists_the_premise
#print axioms QCore.NullMemberPremise.a_listed_premise_is_a_dependency
#print axioms QCore.NullMemberPremise.a_listed_premise_comes_from_a_named_fact
#print axioms QCore.NullMemberPremise.a_row_on_no_premised_fact_lists_none
#print axioms QCore.NullMemberPremise.listed_premises_put_every_fact_in_force
#print axioms QCore.NullMemberPremise.listing_changes_no_verdict
#print axioms QCore.NullMemberPremise.more_dependencies_never_drop_a_premise
#print axioms QCore.NullMemberPremise.premise_vectors_hold
