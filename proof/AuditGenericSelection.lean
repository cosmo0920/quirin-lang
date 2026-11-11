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

import QCore.GenericSelection

/-! Audited Lean companion to card W44-C11-GENERIC-SELECTION.

A C11 generic selection evaluates exactly its selected association and never
its controlling expression. glibc 2.43's C23 const-preserving string macros
spell `strchr (S, C)` as a selection whose two associations are the call and
the call cast to `const char *`. The card's reader lowers a selection to its
common expression when every association is that expression up to enclosing
parentheses and pointer casts, a removed cast requiring a declared
object-pointer result, and leaves every other selection as it was.

* **C11 §6.5.1.1** — only the selected association is evaluated; the
  controlling expression is not.
* **Selection invariance** — an admitted selection's associations all evaluate
  as the core, so the lowered effect does not depend on the selection; the
  whole rewrite preserves every defined outcome, whatever the compiler selects.
* **What refuses** — differing associations, a removed cast without a pointer
  core, a non-pointer cast, two `default` associations, no association.
* **Non-vacuity** — the glibc shape in both selections, a nested selection, a
  side-effecting controlling expression, and the refused non-invariant and
  integer-cast selections, each of whose two selections differ. -/

-- C11 §6.5.1.1.
#print axioms QCore.GenericSelection.exactly_the_selected_association_is_evaluated
#print axioms QCore.GenericSelection.the_control_is_unevaluated

-- Selection invariance and the lowering.
#print axioms QCore.GenericSelection.strip_preserves
#print axioms QCore.GenericSelection.strip_mem_cores
#print axioms QCore.GenericSelection.reader_admits
#print axioms QCore.GenericSelection.an_admitted_association_evaluates_as_the_core
#print axioms QCore.GenericSelection.the_lowered_effect_is_independent_of_the_selection
#print axioms QCore.GenericSelection.resolve_preserves_outcomes
#print axioms QCore.GenericSelection.resolve_assocs_preserves_outcomes
#print axioms QCore.GenericSelection.a_refused_selection_is_unchanged
#print axioms QCore.GenericSelection.the_reader_ignores_the_control

-- What refuses.
#print axioms QCore.GenericSelection.differing_associations_refuse
#print axioms QCore.GenericSelection.a_cast_without_a_pointer_core_refuses
#print axioms QCore.GenericSelection.a_non_pointer_cast_is_not_removed
#print axioms QCore.GenericSelection.two_defaults_refuse
#print axioms QCore.GenericSelection.no_association_refuses

-- Non-vacuity.
#print axioms QCore.GenericSelection.semEx_value_preserving
#print axioms QCore.GenericSelection.semEx_pointer_results
#print axioms QCore.GenericSelection.the_example_satisfies_the_premises
#print axioms QCore.GenericSelection.the_glibc_shape_resolves_to_the_call
#print axioms QCore.GenericSelection.both_glibc_selections_are_the_call
#print axioms QCore.GenericSelection.a_controls_effect_never_appears
#print axioms QCore.GenericSelection.a_nested_selection_resolves_to_the_call
#print axioms QCore.GenericSelection.a_non_invariant_selection_is_refused_and_must_be
#print axioms QCore.GenericSelection.an_integer_cast_is_refused_and_must_be
