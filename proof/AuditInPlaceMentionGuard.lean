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

import QCore.InPlaceMentionGuard

/-! Card **CG25-VOID-CAST-GUARD-ESCAPE** (gateway rule 11v (6)): every mention
of a tracked in-place object is admitted only as its declarator, a `sizeof`
operand or a custody actual at a position of its kind that some call node
carries, whatever route lowered it. Otherwise the function is refused on every
path. Every admitted lowering passed the guard; a stray mention and an
uncarried custody actual are refused on every route; a passing body is CG25's
reading unchanged. The census shapes are bound to the Rust fixtures by
`crates/qcore-c-gateway/tests/in_place_mention_guard.rs`. The lexical
classification of a token as a mention is trust boundary 28's. -/

-- The guard over every route.
#print axioms QCore.InPlaceMentionGuard.a_failing_body_is_refused
#print axioms QCore.InPlaceMentionGuard.every_admitted_lowering_passed_the_guard
#print axioms QCore.InPlaceMentionGuard.a_safe_body_mentions_only_admitted_forms
#print axioms QCore.InPlaceMentionGuard.a_safe_body_carries_every_custody_actual
#print axioms QCore.InPlaceMentionGuard.a_stray_mention_is_refused_on_every_route
#print axioms QCore.InPlaceMentionGuard.an_uncarried_custody_actual_is_refused
#print axioms QCore.InPlaceMentionGuard.a_passing_body_is_cg25
#print axioms QCore.InPlaceMentionGuard.a_body_with_no_mention_passes

-- The census shapes and the witnesses.
#print axioms QCore.InPlaceMentionGuard.shape_admissions_are_the_model
#print axioms QCore.InPlaceMentionGuard.the_void_cast_getter_is_refused
#print axioms QCore.InPlaceMentionGuard.the_regex_match_mentions_pass
#print axioms QCore.InPlaceMentionGuard.the_regex_match_mutant_is_refused
