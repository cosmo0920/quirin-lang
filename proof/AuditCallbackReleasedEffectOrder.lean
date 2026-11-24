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

import QCore.CallbackReleasedEffectOrder

#print axioms QCore.CallbackReleasedEffectOrder.kill_comm
#print axioms QCore.CallbackReleasedEffectOrder.kill_lowers
#print axioms QCore.CallbackReleasedEffectOrder.the_release_writes_no_link
#print axioms QCore.CallbackReleasedEffectOrder.late_admits_only_disjoint
#print axioms QCore.CallbackReleasedEffectOrder.late_is_the_c_order
#print axioms QCore.CallbackReleasedEffectOrder.the_late_order_never_admits_what_the_c_order_refuses
#print axioms QCore.CallbackReleasedEffectOrder.a_link_the_release_releases_refuses
#print axioms QCore.CallbackReleasedEffectOrder.a_kept_path_changes_nothing
#print axioms QCore.CallbackReleasedEffectOrder.the_late_order_is_stricter
#print axioms QCore.CallbackReleasedEffectOrder.order_vectors_hold
