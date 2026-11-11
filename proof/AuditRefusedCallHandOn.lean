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

import QCore.RefusedCallHandOn

/-! Card **W45-FB-BACKLOG-CUSTODY** (integration review): a contracted call the
gateway refuses before its effects hands on its actuals, as W44-LEAK-PIN-AUDIT's
refused hand-on rule does for a refused statement
(`legacy_refused_operands::withdraw_refused_call`).

Audited: it is the W44 rule at a wider refusal signal; it only withdraws; a
site the call does not name keeps its leak; the named object is withdrawn;
and the node's refusal keeps the function out of `Safe`. The extraction of the
call's actuals is trust boundary 28, pinned by
`crates/qcore-c-gateway/tests/refused_call_hand_on.rs`. -/

#print axioms QCore.RefusedCallHandOn.is_the_hand_on_rule
#print axioms QCore.RefusedCallHandOn.it_only_withdraws
#print axioms QCore.RefusedCallHandOn.an_unnamed_site_keeps_its_leak
#print axioms QCore.RefusedCallHandOn.the_named_object_is_withdrawn
#print axioms QCore.RefusedCallHandOn.a_refused_call_is_never_safe
