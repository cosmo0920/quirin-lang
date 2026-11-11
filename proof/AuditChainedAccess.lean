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

import QCore.TypedFieldAccess
import QCore.ExitResiduals
import QCore.BorrowedActual
import QCore.ChainedAccess

/-! Card O04-ARMS: the bounded access chain that replaces C02-P's blanket
`chained` refusal. Both directions are audited — every link the reader did not
fully type refuses the whole chain, and the rule is a strict extension of the
depth-one admission rather than a replacement. -/

-- Structural refusals of the whole chain.
#print axioms QCore.ChainedAccess.an_empty_chain_never_admits
#print axioms QCore.ChainedAccess.unread_declarations_never_admit_a_chain
#print axioms QCore.ChainedAccess.a_mismatched_base_never_admits_a_chain
#print axioms QCore.ChainedAccess.a_bound_root_never_admits_a_chain
#print axioms QCore.ChainedAccess.a_continued_access_never_admits
#print axioms QCore.ChainedAccess.a_chain_beyond_the_bound_never_admits

-- One untyped link refuses, wherever in the chain it sits.
#print axioms QCore.ChainedAccess.linksOk_false_of_bad_link
#print axioms QCore.ChainedAccess.a_bad_link_never_admits
#print axioms QCore.ChainedAccess.a_union_link_never_admits
#print axioms QCore.ChainedAccess.an_incomplete_link_never_admits
#print axioms QCore.ChainedAccess.an_ambiguous_link_never_admits
#print axioms QCore.ChainedAccess.a_qualified_link_never_admits
#print axioms QCore.ChainedAccess.an_aggregate_link_never_admits

-- The clause the chain adds: only a fully typed `struct POINTEE *` continues.
#print axioms QCore.ChainedAccess.linksOk_false_of_unContinuable
#print axioms QCore.ChainedAccess.a_non_continuable_intermediate_never_admits
#print axioms QCore.ChainedAccess.a_scalar_intermediate_never_admits

-- Extension, not replacement; and non-vacuity in both directions.
#print axioms QCore.ChainedAccess.a_depth_one_chain_is_the_direct_admission
#print axioms QCore.ChainedAccess.the_chain_rule_is_not_vacuous
#print axioms QCore.ChainedAccess.one_union_link_flips_the_admitted_chain
#print axioms QCore.ChainedAccess.the_bound_is_exactly_four

-- Fail-closed bridging into the statement classification.
#print axioms QCore.ChainedAccess.a_refused_chain_refuses_the_statement
#print axioms QCore.ChainedAccess.a_refused_chain_is_never_safe
#print axioms QCore.ChainedAccess.an_admitted_chain_does_not_rescue_a_statement
