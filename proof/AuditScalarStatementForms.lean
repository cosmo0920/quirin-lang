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

import QCore.ScalarStatementForms

/-! Card **SCALAR-STATEMENT-FORMS** (round 26 E): Clang-typed scalar
statement forms (gateway rule 41).

Five directions are audited.

**The decision.** The Rust table `VECTORS` is `vectors`
(`decision_vectors_hold`); an admission needs a scalar destination or a
type-only declaration; a pointer destination, a pointer value, a `volatile`
object, an inline record object and a variably modified type or `sizeof`
refuse by name.

**The forms are stores.** A compound assignment is the plain store of
`X op E`, a prefix step is `X += 1`, a postfix step has its side effect.

**The lowering is sound.** A named store's post-state is a run of `forget
X`, a path store's a run of the path write; several declarators are their
stores in order; a type declaration runs nothing.

**Frame preservation.** Every lowering leaves the pointer bindings and the
owed set unchanged; an owed object stays owed; a released base has no write.

**Never `Safe` by declining.**

Clang extraction, node matching and the Rust/model correspondence are trust
boundary 28's, pinned by `crates/qcore-c-gateway/tests/scalar_statement_forms.rs`
and the shared-vector test of `scalar_statement_forms.rs`. -/

-- The decision.
#print axioms QCore.ScalarStatementForms.decision_vectors_hold
#print axioms QCore.ScalarStatementForms.admitted_destination
#print axioms QCore.ScalarStatementForms.pointer_destination_refuses
#print axioms QCore.ScalarStatementForms.volatile_refuses
#print axioms QCore.ScalarStatementForms.inline_record_object_refuses
#print axioms QCore.ScalarStatementForms.variably_modified_type_refuses
#print axioms QCore.ScalarStatementForms.pointer_value_refuses
#print axioms QCore.ScalarStatementForms.evaluated_query_refuses

-- The forms are stores.
#print axioms QCore.ScalarStatementForms.compound_is_assignment
#print axioms QCore.ScalarStatementForms.step_is_compound
#print axioms QCore.ScalarStatementForms.postfix_step_side_effect

-- The lowering is sound.
#print axioms QCore.ScalarStatementForms.named_store_refines_forget
#print axioms QCore.ScalarStatementForms.compound_refines_forget
#print axioms QCore.ScalarStatementForms.step_refines_forget
#print axioms QCore.ScalarStatementForms.path_store_refines_write
#print axioms QCore.ScalarStatementForms.declarators_are_sequential
#print axioms QCore.ScalarStatementForms.type_declaration_is_noop

-- Frame preservation.
#print axioms QCore.ScalarStatementForms.run_preserves_frame
#print axioms QCore.ScalarStatementForms.runs_preserve_frame
#print axioms QCore.ScalarStatementForms.an_owned_object_stays_owed
#print axioms QCore.ScalarStatementForms.a_released_base_has_no_write

-- Never Safe.
#print axioms QCore.ScalarStatementForms.a_declined_statement_is_never_safe
