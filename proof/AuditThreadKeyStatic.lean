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

/-! Card **CURRENT-CONTEXT-NEW-TRIAGE** (D3, gateway rule 11v (5)): the in-place
`thread_key` kind. A key created into static storage binds no frame obligation
(P-THREAD-KEY-STATIC, stated); a static key's finaliser is refused; an
automatic key's creation is refused, so no key is ever tracked. The
static/automatic split is read off the token stream and Clang (trust boundary
28), pinned by `crates/qcore-c-gateway/tests/thread_key_in_place.rs`. -/

#print axioms QCore.ThreadKeyStatic.in_place_is_cg25
#print axioms QCore.ThreadKeyStatic.a_static_creation_owes_nothing
#print axioms QCore.ThreadKeyStatic.a_static_creation_binds_no_storage
#print axioms QCore.ThreadKeyStatic.a_static_finaliser_is_refused
#print axioms QCore.ThreadKeyStatic.a_static_finaliser_is_never_safe
#print axioms QCore.ThreadKeyStatic.an_automatic_creation_is_refused
#print axioms QCore.ThreadKeyStatic.an_automatic_creation_tracks_nothing
#print axioms QCore.ThreadKeyStatic.the_two_static_creations_owe_nothing
