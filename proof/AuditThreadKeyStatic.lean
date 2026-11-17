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

import QCore.ThreadKeyStatic

/-! Cards **CURRENT-CONTEXT-NEW-TRIAGE** (D3, gateway rule 11v (5)) and
**CG25-THREAD-KEY-AUTOMATIC-REVISIT** (rule 11v (7)): the in-place
`thread_key` kind. A key created into static storage binds no frame
obligation (P-THREAD-KEY-STATIC, stated); a static key's finaliser is
refused. A key created into the one automatic key object of the function is
CG25's initialiser; a creation into any other storage is refused. A by-value
use is admitted only where the key is certified (live, its status test
decided success); a use after the delete is a candidate, every other
uncertified use is refused, so a `Safe` verdict used only certified keys.
The static/automatic split is read off the token stream and Clang (trust
boundary 28), pinned by `crates/qcore-c-gateway/tests/thread_key_in_place.rs`. -/

#print axioms QCore.ThreadKeyStatic.in_place_is_cg25
-- Static keys (rule 11v (5)).
#print axioms QCore.ThreadKeyStatic.a_static_creation_owes_nothing
#print axioms QCore.ThreadKeyStatic.a_static_creation_binds_no_storage
#print axioms QCore.ThreadKeyStatic.a_static_finaliser_is_refused
#print axioms QCore.ThreadKeyStatic.a_static_finaliser_is_never_safe
-- Automatic keys (rule 11v (7)).
#print axioms QCore.ThreadKeyStatic.an_automatic_creation_is_cg25
#print axioms QCore.ThreadKeyStatic.another_creation_is_refused
#print axioms QCore.ThreadKeyStatic.another_creation_tracks_nothing
#print axioms QCore.ThreadKeyStatic.a_certified_use_changes_nothing
#print axioms QCore.ThreadKeyStatic.a_use_after_delete_is_a_candidate
#print axioms QCore.ThreadKeyStatic.an_uncertified_use_is_refused
#print axioms QCore.ThreadKeyStatic.an_uncertified_use_is_never_safe
#print axioms QCore.ThreadKeyStatic.a_use_changes_no_custody
#print axioms QCore.ThreadKeyStatic.transferI_diagnostics_grow
#print axioms QCore.ThreadKeyStatic.diagnostics_only_grow
#print axioms QCore.ThreadKeyStatic.run_keeps_diagnostics
#print axioms QCore.ThreadKeyStatic.a_run_without_refusal_used_only_certified_keys
#print axioms QCore.ThreadKeyStatic.a_safe_verdict_used_only_certified_keys
-- Witness programs.
#print axioms QCore.ThreadKeyStatic.the_create_use_delete_program_is_safe
#print axioms QCore.ThreadKeyStatic.the_failure_edge_owes_nothing
#print axioms QCore.ThreadKeyStatic.a_use_after_the_delete_is_a_candidate
#print axioms QCore.ThreadKeyStatic.a_missing_delete_leaks
#print axioms QCore.ThreadKeyStatic.a_double_delete_is_a_candidate
#print axioms QCore.ThreadKeyStatic.a_use_before_the_test_is_refused
#print axioms QCore.ThreadKeyStatic.a_use_on_the_failure_edge_is_refused
#print axioms QCore.ThreadKeyStatic.a_use_before_the_creation_is_refused
#print axioms QCore.ThreadKeyStatic.the_two_static_creations_owe_nothing
