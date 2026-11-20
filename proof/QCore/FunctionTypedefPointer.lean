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

import QCore.MemberStoreRetention

/-!
# `T *` for a function typedef `T` is a function pointer (card CG10-READER-GAPS, rule 71b)

Clang desugars only a type's top level, so a formal `T *f` with
`typedef R (T)(A);` is spelled `T *`, which rule 10v read as an object
pointer and rule 70 retained. Rule 71b reads it as a function-pointer name
exactly when the spelling is `NAME *` and every declaration of `NAME` is
directly a function type (`function_typedef_pointer`, bound to
`DECISION_VECTORS`).

The model proves:

* the decision vectors; a typedef's answer is the conjunction over all its
  declarations, so one non-function declaration keeps the object reading;
* only a direct function type (through parentheses and elaboration) is a
  function typedef: a pointer, an array of function pointers or another
  typedef is not (`a_pointer_array_typedef_is_an_object`);
* **a function pointer adds no obligation**: rule 70 records nothing for it
  (`QCore.MemberStoreRetention.a_function_pointer_adds_no_obligation`), and a
  pointer to a function type designates no object (C11 6.2.5p20, 6.3.2.3p8),
  so the reading never drops an obligation an object pointer would carry.
-/

namespace QCore.FunctionTypedefPointer

/-- The type node of a `TypedefDecl`, as far as the reading looks. -/
inductive TyNode where
  | paren (t : TyNode)
  | elaborated (t : TyNode)
  | functionProto
  | functionNoProto
  | pointer (t : TyNode)
  | array (t : TyNode)
  | typedefRef
  | record
  deriving Repr

/-- Directly a function type, through parentheses and elaboration only. -/
def declaresFunctionType : TyNode → Bool
  | .paren t => declaresFunctionType t
  | .elaborated t => declaresFunctionType t
  | .functionProto => true
  | .functionNoProto => true
  | _ => false

/-- A typedef name's answer over all its declarations. -/
def functionTypedef (decls : List TyNode) : Bool := decls.all declaresFunctionType

/-- Rule 71b on a type spelling's tokens. -/
def namesFunctionPointer (toks : List String) (fnTypedef : String → Bool) : Bool :=
  match toks with
  | [name, "*"] => fnTypedef name
  | _ => false

/-- The Rust's `DECISION_VECTORS` (`cb_t` a function typedef or not). -/
def decisionVectors : List (List String × Bool × Bool) :=
  [ (["cb_t", "*"], true, true), (["cb_t", "*"], false, false),
    (["cb_t", "*", "*"], true, false), (["const", "cb_t", "*"], true, false),
    (["cb_t"], true, false), (["struct", "s", "*"], true, false) ]

theorem decision_vectors_hold :
    decisionVectors.all
      (fun (toks, td, r) => namesFunctionPointer toks (fun n => td && n == "cb_t") == r) = true := by
  decide

/-- One declaration that is not a function type keeps the object reading. -/
theorem a_disagreeing_declaration_keeps_the_object_reading (decls : List TyNode) (d : TyNode)
    (hd : d ∈ decls) (hn : declaresFunctionType d = false) : functionTypedef decls = false := by
  simp only [functionTypedef, List.all_eq_false]
  exact ⟨d, hd, by simp [hn]⟩

/-- `typedef int (*cb[3])(void)` is an array of function pointers: an object. -/
theorem a_pointer_array_typedef_is_an_object (t : TyNode) :
    declaresFunctionType (.array (.pointer t)) = false := rfl

/-- `typedef int (*cb)(void)` is a function pointer type, not a function type:
`cb *` points at an object (the pointer), so it stays retained. -/
theorem a_pointer_typedef_is_not_a_function_typedef (t : TyNode) :
    declaresFunctionType (.pointer t) = false := rfl

/-- A typedef of a typedef is not followed (fail-closed). -/
theorem an_alias_is_not_followed : declaresFunctionType .typedefRef = false := rfl

/-- Only the spelling `NAME *` is read, and only for a function typedef. -/
theorem read_only_for_a_function_typedef (toks : List String) (f : String → Bool)
    (h : namesFunctionPointer toks f = true) : ∃ n, toks = [n, "*"] ∧ f n = true := by
  unfold namesFunctionPointer at h
  split at h
  · exact ⟨_, rfl, h⟩
  · exact absurd h (by simp)

/-- **A function pointer adds no obligation**: rule 70's store of a value
with no object (a function pointer) records nothing, whatever the holder. -/
theorem a_function_pointer_adds_no_obligation (h : QCore.RetentionPath.Holder)
    (rs : QCore.RetentionPath.Records) :
    QCore.RetentionPath.step rs (.retain (QCore.RetentionPath.decide .static h)) = some rs :=
  QCore.MemberStoreRetention.a_function_pointer_adds_no_obligation h rs

end QCore.FunctionTypedefPointer
