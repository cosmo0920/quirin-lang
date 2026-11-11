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

import QCore.AppRkCallbacks

/-! Audited companion to OP-APP-RK-CALLBACKS-VIA-POLL-CB (gateway rule 12n, amended): the application's rk_conf callbacks closed over the run's registrations. A setter's registration closes a member only together with every run store and its candidates are exactly the read and registered ones; a stated function default is a candidate, and `quant live #result: TAG.FIELD |-> f` and its two parts mean the same and are one canonical set; over the run's set an open member never decides, a closed empty set invokes nothing and a closed non-empty one decides as the unit rule, while a unit's own empty set stays refused; skipping a unit that names no token keeps the run set and an unread naming unit keeps it open; a store through an embedded record and one through a pointer to it share the innermost key, and normalising is idempotent. -/

#print axioms QCore.AppRkCallbacks.the_registered_set_is_exactly_the_read_and_registered_sets
#print axioms QCore.AppRkCallbacks.a_registration_closes_the_member_only_with_every_run_store
#print axioms QCore.AppRkCallbacks.an_unread_unit_keeps_the_registered_member_open
#print axioms QCore.AppRkCallbacks.an_open_run_member_never_decides
#print axioms QCore.AppRkCallbacks.a_closed_empty_run_set_invokes_nothing
#print axioms QCore.AppRkCallbacks.a_closed_run_set_decides_as_the_unit_rule
#print axioms QCore.AppRkCallbacks.a_unit_empty_set_stays_refused
#print axioms QCore.AppRkCallbacks.the_fold_keeps_a_run_decision_exactly_when_closed
#print axioms QCore.AppRkCallbacks.skipping_a_silent_unit_keeps_the_run_set
#print axioms QCore.AppRkCallbacks.an_unread_naming_unit_keeps_it_open
#print axioms QCore.AppRkCallbacks.the_function_shorthand_and_its_parts_mean_the_same
#print axioms QCore.AppRkCallbacks.the_function_shorthand_is_live_and_holds_f
#print axioms QCore.AppRkCallbacks.the_absent_value_is_the_earlier_absent_state
#print axioms QCore.AppRkCallbacks.both_function_levels_are_one_canonical_set
#print axioms QCore.AppRkCallbacks.a_stated_default_is_a_candidate
#print axioms QCore.AppRkCallbacks.an_embedded_store_and_a_direct_store_share_the_key
#print axioms QCore.AppRkCallbacks.a_normal_key_restarts_nowhere
#print axioms QCore.AppRkCallbacks.normalise_is_idempotent
#print axioms QCore.AppRkCallbacks.key_vectors_hold
#print axioms QCore.AppRkCallbacks.run_decision_vectors_hold
