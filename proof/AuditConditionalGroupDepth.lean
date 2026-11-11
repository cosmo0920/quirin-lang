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

import QCore.ConditionalGroupDepth

-- Unit `E01-RAW-COND`: conditional-group brace balance in the raw token scan.
-- Model of `crates/qcore-c-gateway/src/legacy_c.rs`'s `apply_conditional` and
-- the group-aware `function_names`. The Rust byte lexer's recognition of the
-- directive spellings, and `function_names`' reading of a depth-zero name as a
-- definition, are Rust boundaries recorded in
-- `docs/reports/remaining-e01-raw-cond/README.md`.

-- The per-group rule: the greatest arm depth dominates every arm, is exactly
-- the common depth when the arms agree, and is always a depth some arm reaches.
#print axioms QCore.ConditionalGroupDepth.groupDepth_dominates
#print axioms QCore.ConditionalGroupDepth.agreeing_arms_are_exact
#print axioms QCore.ConditionalGroupDepth.groupDepth_is_reached

-- Disagreement is refused and agreement is not, so the refusal flag is exactly
-- the undecided case.
#print axioms QCore.ConditionalGroupDepth.disagreeing_arms_refuse
#print axioms QCore.ConditionalGroupDepth.agreeing_arms_do_not_refuse

-- The arms measured: the one that ends at `#endif`, and the implicit empty arm
-- of a group with no `#else`.
#print axioms QCore.ConditionalGroupDepth.the_last_arm_is_measured
#print axioms QCore.ConditionalGroupDepth.a_group_without_else_carries_the_entry_depth

-- Monotonicity (E01-F1 §1.4): every arm is scanned from the depth the group
-- began at, so a definition written in any arm keeps its row.
#print axioms QCore.ConditionalGroupDepth.a_separator_restores_the_entry_depth
#print axioms QCore.ConditionalGroupDepth.a_group_opens_at_the_entry_depth

-- An ordinary token is the identity, which is why the exhaustive domain omits
-- it; and a recorded refusal is never cleared.
#print axioms QCore.ConditionalGroupDepth.other_is_the_identity
#print axioms QCore.ConditionalGroupDepth.refusal_is_monotone

-- Non-vacuity: the reduced `rd_kafka_dir_is_empty` shape, decided under the
-- rule and under the summing rule this unit replaces.
#print axioms QCore.ConditionalGroupDepth.the_rule_balances_the_measured_shape
#print axioms QCore.ConditionalGroupDepth.the_measured_shape_is_not_refused
#print axioms QCore.ConditionalGroupDepth.the_summing_rule_desynchronises_the_measured_shape
#print axioms QCore.ConditionalGroupDepth.the_unit_changes_the_measured_shape
#print axioms QCore.ConditionalGroupDepth.every_selection_of_the_measured_shape_is_matched

-- The dangerous direction: the summing rule reached file scope inside a body
-- and produced a phantom definition; the rule dominates instead.
#print axioms QCore.ConditionalGroupDepth.the_summing_rule_reaches_file_scope_inside_a_body
#print axioms QCore.ConditionalGroupDepth.the_rule_dominates_the_phantom_shape
#print axioms QCore.ConditionalGroupDepth.every_selection_of_the_phantom_shape_is_dominated

-- `#elif` chains and nested groups.
#print axioms QCore.ConditionalGroupDepth.an_elif_chain_is_decided_exactly
#print axioms QCore.ConditionalGroupDepth.nested_groups_compose

-- A stream no selection semantics can read is refused.
#print axioms QCore.ConditionalGroupDepth.unstructured_streams_are_refused

-- The exhaustive length-4 domain: dominated, exact when unrefused, refused when
-- unreadable — and tied to the Rust implementation by a checksum both sides
-- compute independently.
#print axioms QCore.ConditionalGroupDepth.the_domain_is_exhaustive
#print axioms QCore.ConditionalGroupDepth.the_domain_is_decided
#print axioms QCore.ConditionalGroupDepth.the_domain_checksum_is_pinned
