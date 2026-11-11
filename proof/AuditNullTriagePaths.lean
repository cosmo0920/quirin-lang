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

import QCore.NullTriagePaths

/-! Audited companion to NULL-TRIAGE-UNRESOLVED-PATHS (gateway rules 15q-15u). -/

#print axioms QCore.NullTriagePaths.an_inexact_arm_is_never_exact
#print axioms QCore.NullTriagePaths.a_call_result_never_contributes
#print axioms QCore.NullTriagePaths.a_witness_holds_whatever_the_callees_return
#print axioms QCore.NullTriagePaths.a_bound_excludes_its_outside_arm
#print axioms QCore.NullTriagePaths.exhaustive_search_is_sound
#print axioms QCore.NullTriagePaths.difference_attains
#print axioms QCore.NullTriagePaths.sum_attains
#print axioms QCore.NullTriagePaths.lt_arm_is_attained
#print axioms QCore.NullTriagePaths.an_unmet_arm_has_no_value
#print axioms QCore.NullTriagePaths.counter_stays_above
#print axioms QCore.NullTriagePaths.explore_vectors_hold
#print axioms QCore.NullTriagePaths.a_finding_needs_an_exact_path
#print axioms QCore.NullTriagePaths.infeasible_needs_a_complete_empty_search
#print axioms QCore.NullTriagePaths.conclude_vectors_hold
#print axioms QCore.NullTriagePaths.arms_vectors_hold
