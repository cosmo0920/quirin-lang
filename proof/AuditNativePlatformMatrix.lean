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

import QCore.NativePlatformMatrix

#print axioms QCore.NativePlatformMatrix.required_profiles_are_seven_and_unique
#print axioms QCore.NativePlatformMatrix.valid_matrix_has_exact_profiles
#print axioms QCore.NativePlatformMatrix.selected_matrix_is_valid
#print axioms QCore.NativePlatformMatrix.all_unknown_matrix_is_valid
#print axioms QCore.NativePlatformMatrix.profile_relabel_refuses
#print axioms QCore.NativePlatformMatrix.same_thread_refuses
#print axioms QCore.NativePlatformMatrix.compiler_substitution_refuses
#print axioms QCore.NativePlatformMatrix.artifact_substitution_refuses
#print axioms QCore.NativePlatformMatrix.carried_history_does_not_promote
#print axioms QCore.NativePlatformMatrix.transparency_claim_refuses
