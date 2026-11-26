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

import QCore.NameKeyedHolderShadowing

/-! Card **NAME-KEYED-HOLDER-SHADOWING** (gateway rule 86): rule 12t's
formal-keyed readings (a caller's object, an interior of one, a path holder
and a removal keyed on them) need the formal to be one declaration, the
`ParmVarDecl` every reference of the name designates. Rule 86's reading is a
formal only where rule 12t's was, so a `Safe` replay under it was `Safe`
before, and a shadowed name is refused. Which declaration a reference
designates is read off Clang (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/name_keyed_holder_shadowing.rs`. -/

-- One declaration.
#print axioms QCore.NameKeyedHolderShadowing.a_formal_was_one_before
#print axioms QCore.NameKeyedHolderShadowing.a_shadowed_name_is_not_a_formal
#print axioms QCore.NameKeyedHolderShadowing.a_foreign_reference_is_not_a_formal
#print axioms QCore.NameKeyedHolderShadowing.the_readings_agree_on_one_declaration
-- Rule 86's readings are rule 12t's or undecided.
#print axioms QCore.NameKeyedHolderShadowing.retained_weakens
#print axioms QCore.NameKeyedHolderShadowing.holder_weakens
#print axioms QCore.NameKeyedHolderShadowing.decision_weakens
#print axioms QCore.NameKeyedHolderShadowing.key_weakens
-- Only refusals are added.
#print axioms QCore.NameKeyedHolderShadowing.remove_covers
#print axioms QCore.NameKeyedHolderShadowing.step_covers
#print axioms QCore.NameKeyedHolderShadowing.lower_covers
#print axioms QCore.NameKeyedHolderShadowing.run_lowerAll_covers
#print axioms QCore.NameKeyedHolderShadowing.no_verdict_becomes_safe
-- Witness programs.
#print axioms QCore.NameKeyedHolderShadowing.probe_j2_was_safe_and_is_refused
#print axioms QCore.NameKeyedHolderShadowing.probe_j6_was_safe_and_is_refused
#print axioms QCore.NameKeyedHolderShadowing.a_shadowed_removal_drops_nothing
#print axioms QCore.NameKeyedHolderShadowing.the_control_is_safe_under_both
