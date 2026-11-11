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

import QCore.NullMemberScope

/-! Audited companion to NULL-MEMBER-INVARIANT-SCOPE (gateway rules 14x-14z). -/

#print axioms QCore.NullMemberScope.step_keeps
#print axioms QCore.NullMemberScope.stable_fact_persists
#print axioms QCore.NullMemberScope.a_fresh_store_does_not_reach
#print axioms QCore.NullMemberScope.a_scoped_use_sees_non_null
#print axioms QCore.NullMemberScope.an_exit_store_breaks_persistence
#print axioms QCore.NullMemberScope.an_unproved_store_breaks_persistence
#print axioms QCore.NullMemberScope.a_kept_fact_is_non_null
#print axioms QCore.NullMemberScope.nothing_is_kept_without_stability
#print axioms QCore.NullMemberScope.a_stable_declaration_is_never_validated
#print axioms QCore.NullMemberScope.a_failed_store_withdraws_it
#print axioms QCore.NullMemberScope.failure_is_monotone
#print axioms QCore.NullMemberScope.stable_vectors_hold
#print axioms QCore.NullMemberScope.persist_vectors_hold
