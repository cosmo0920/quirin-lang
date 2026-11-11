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

import QCore.LegacyCAnalyzer

-- Print the foundational dependencies of every Cycle 9 safety theorem.
#print axioms QCore.LegacyCAnalyzer.safe_implies_no_exit_leak
#print axioms QCore.LegacyCAnalyzer.safe_implies_no_safety_diagnostic
#print axioms QCore.LegacyCAnalyzer.safe_implies_precise
#print axioms QCore.LegacyCAnalyzer.safe_report_is_sound
#print axioms QCore.LegacyCAnalyzer.safe_implies_no_double_release_diagnostic
#print axioms QCore.LegacyCAnalyzer.safe_implies_no_release_after_escape
#print axioms QCore.LegacyCAnalyzer.imprecise_is_never_safe
#print axioms QCore.LegacyCAnalyzer.unsupported_becomes_unknown
#print axioms QCore.LegacyCAnalyzer.unsupported_is_never_safe
#print axioms QCore.LegacyCAnalyzer.seeded_normal_leak_is_unsafe
#print axioms QCore.LegacyCAnalyzer.seeded_double_release_is_unsafe
#print axioms QCore.LegacyCAnalyzer.acquired_then_released_is_safe
#print axioms QCore.LegacyCAnalyzer.join_preserves_left_live
#print axioms QCore.LegacyCAnalyzer.join_preserves_right_live
#print axioms QCore.LegacyCAnalyzer.alias_join_preserves_unknown_left
#print axioms QCore.LegacyCAnalyzer.alias_join_preserves_unknown_right

#print axioms QCore.LegacyCAnalyzer.released_then_unmodeled_access_is_indeterminate
