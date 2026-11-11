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

import QCore.NullLookupEvent

/-! Audited companion to NULL-WITNESS-GUARD-CORRELATION (gateway rule 39). -/

#print axioms QCore.NullLookupEvent.a_named_miss_is_decided_or_free
#print axioms QCore.NullLookupEvent.a_tied_lookup_is_named_only_when_decided_null
#print axioms QCore.NullLookupEvent.a_correlated_untied_miss_is_refused
#print axioms QCore.NullLookupEvent.correlation_never_names
#print axioms QCore.NullLookupEvent.touching_never_frees
#print axioms QCore.NullLookupEvent.a_call_before_never_frees_a_global
#print axioms QCore.NullLookupEvent.a_local_or_unrooted_argument_is_correlated
#print axioms QCore.NullLookupEvent.one_correlated_argument_correlates
#print axioms QCore.NullLookupEvent.a_free_lookup_has_only_free_arguments
#print axioms QCore.NullLookupEvent.a_correlated_miss_never_contributes
#print axioms QCore.NullLookupEvent.a_refused_miss_never_concludes_infeasible
#print axioms QCore.NullLookupEvent.a_refused_miss_is_never_a_finding
#print axioms QCore.NullLookupEvent.miss_vectors_hold
#print axioms QCore.NullLookupEvent.state_vectors_hold
