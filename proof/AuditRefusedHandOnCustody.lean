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

import QCore.RefusedHandOnCustody

/-! Card **W44-LEAK-PIN-AUDIT**: the refused hand-on rule. A statement the
projection refuses that names a live identity — stores or returns it, passes it
or its interior address to a call, or uses it as an operand — leaves that
identity with an unresolved effect, the projection's existing reading of "an
unmodelled action may have discharged it".

Five directions are audited. **Only unresolved effects grow**: no status and no
exemption changes, and an admitted node is untouched. **Only withdrawal**: a
leak reported after the rule was reported before it; a site the statement did
not hand on keeps its report exactly; a handed-on live site is withdrawn.
**Composition**: the rule distributes over the union join. **No `Safe`**: the
rule fires only where the node reported `Unsupported`, so a fully withdrawn
function is `Unknown`. **Stated collateral**: the union join withdraws a clean
path joined with a handed-on one, which the card measures leaf by leaf.

What this is not: a proof about the Rust (trust boundary 28) or about the
reader that extracts hand-ons from tokens (an extraction premise). A remaining
leak is a provisional candidate, never an upstream defect. -/

#print axioms QCore.RefusedHandOnCustody.the_rule_only_grows_unresolved
#print axioms QCore.RefusedHandOnCustody.the_rule_changes_no_status
#print axioms QCore.RefusedHandOnCustody.an_admitted_node_is_untouched
#print axioms QCore.RefusedHandOnCustody.the_rule_only_withdraws
#print axioms QCore.RefusedHandOnCustody.an_unhanded_site_keeps_its_report
#print axioms QCore.RefusedHandOnCustody.a_refused_hand_on_is_withdrawn
#print axioms QCore.RefusedHandOnCustody.the_rule_distributes_over_the_join
#print axioms QCore.RefusedHandOnCustody.a_withdrawal_by_the_rule_is_never_safe
#print axioms QCore.RefusedHandOnCustody.a_fully_withdrawn_function_is_unknown
#print axioms QCore.RefusedHandOnCustody.the_union_join_withdraws_a_clean_path
