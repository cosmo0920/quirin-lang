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

import QCore.NoBuildScope

#print axioms QCore.NoBuildScope.publish_scope
#print axioms QCore.NoBuildScope.publish_never_configured
#print axioms QCore.NoBuildScope.unprepared_unknown
#print axioms QCore.NoBuildScope.reason_iff_unprepared
#print axioms QCore.NoBuildScope.safe_relative
#print axioms QCore.NoBuildScope.run_length
#print axioms QCore.NoBuildScope.run_reasons
#print axioms QCore.NoBuildScope.noBuild_never_admitted_as_configured
#print axioms QCore.NoBuildScope.noBuild_admitted_only_under_own_assumptions
#print axioms QCore.NoBuildScope.noBuild_not_comparable_with_configured
#print axioms QCore.NoBuildScope.same_text_same_verdict
#print axioms QCore.NoBuildScope.verdict_difference_needs_text_difference
#print axioms QCore.NoBuildScope.favourable_direction_exists
#print axioms QCore.NoBuildScope.no_transfer_between_assumptions
