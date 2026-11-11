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

import QCore.ReadOnlyActualFrame

#print axioms QCore.ReadOnlyActualFrame.checked_list_count_does_not_bind
#print axioms QCore.ReadOnlyActualFrame.uncertified_list_count_binds
#print axioms QCore.ReadOnlyActualFrame.checked_actual_preserves_decision
#print axioms QCore.ReadOnlyActualFrame.checked_actual_preserves_return
#print axioms QCore.ReadOnlyActualFrame.a_bound_root_is_never_admitted
#print axioms QCore.ReadOnlyActualFrame.other_call_still_refuses
#print axioms QCore.ReadOnlyActualFrame.another_actual_blocks_the_return
#print axioms QCore.ReadOnlyActualFrame.write_still_refuses
