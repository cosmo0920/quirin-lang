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

import QCore.ReturnCastView

/-! Audited companion to CG10-READER-GAPS (gateway rule 71a). -/

#print axioms QCore.ReturnCastView.return_view_vectors_hold
#print axioms QCore.ReturnCastView.only_a_whole_call_is_read
#print axioms QCore.ReturnCastView.a_kept_view_keeps_the_refusal
#print axioms QCore.ReturnCastView.nothing_is_left_hidden
#print axioms QCore.ReturnCastView.the_same_identities_are_handed
#print axioms QCore.ReturnCastView.a_removed_view_is_its_place
