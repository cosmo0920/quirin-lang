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

import QCore.RefusedStoreHandOn

/-! Card **CURRENT-CONTEXT-NEW-TRIAGE** (rule 5l (6)): a B08 field store the
gateway refuses on its owner hands on its value, as W44-LEAK-PIN-AUDIT's
refused hand-on rule does for a refused statement and rule 5l (5) for a call
refused before its effects (`legacy_transfer::refused_store_hand_ons`).

Audited: it is the W44 rule at a wider refusal signal and extends the call
rule; it only withdraws and changes no status; an admitted store is
untouched; a site the store does not name keeps its leak; the stored value is
withdrawn; and the node's refusal keeps the function out of `Safe`. The
extraction of the stored name is trust boundary 28, pinned by
`crates/qcore-c-gateway/tests/refused_store_hand_on.rs`. -/

#print axioms QCore.RefusedStoreHandOn.is_the_hand_on_rule
#print axioms QCore.RefusedStoreHandOn.extends_the_call_rule
#print axioms QCore.RefusedStoreHandOn.it_only_withdraws
#print axioms QCore.RefusedStoreHandOn.it_changes_no_status
#print axioms QCore.RefusedStoreHandOn.an_admitted_store_is_untouched
#print axioms QCore.RefusedStoreHandOn.an_unnamed_site_keeps_its_leak
#print axioms QCore.RefusedStoreHandOn.the_stored_value_is_withdrawn
#print axioms QCore.RefusedStoreHandOn.a_refused_store_is_never_safe
