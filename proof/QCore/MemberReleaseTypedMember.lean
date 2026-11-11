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

import QCore.MemberEffectSummary

/-!
# A member release whose member is spelled through a pointer typedef (card MEMBER-EFFECT-COMPONENT-SCOPE, gateway rule 38j amended)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_call_argument_access.rs`
(`candidate_arguments`, the `Refusal::UntypedMember` arm) and
`crates/qcore-c-gateway/src/legacy_member_effect.rs`
(`Parser::typed_member_release`).

Rule 1 of card W45-CALLEE-SUMMARY-CASCADE admits an actual `B->F` handed to a
`Released { nullable: true }` position only for a place `(B, F)` whose body
check holds (rule 5k's P1-P4 or rule 38j's P1-P3, `placeOk` of
`QCore.MemberEffectSummary`) and only when the actual reads one pointer value
from a member of `*B`. Before this card that last fact came only from the
lexical declaration reader, which leaves a pointer typedef (`typedef char
*cfl_sds_t;`) unresolved and so refuses `opts->ns` as an *untyped member*.
The amendment lets Clang's reading of the actual establish the same fact,
only where the lexical reader refused it as an untyped member: the actual is
the value (`LValueToRValue`) of an arrow member of the parameter `B`, its
type an unqualified object pointer, its base no `union`.

Proved here:

* **Nothing lexical is relaxed** (`an_old_admission_is_kept`,
  `a_new_admission_is_an_untyped_member`,
  `an_admission_needs_the_place_and_a_nullable_release`,
  `an_untyped_base_is_never_admitted`): the only new admissions are actuals
  the lexical reader refused as an untyped member, and every admission still
  needs the place's body check and a nullable release.
* **An admitted actual is a pointer member's value**
  (`an_admitted_actual_is_an_object_pointer`), given the lexical reader's own
  premise for a `*`-spelled member; a decayed array, a function pointer, a
  qualified pointer and a `union` base are refused
  (`a_decayed_array_is_refused` and three more).
* **The body stays sound** (`the_body_stays_sound`): the admitted actual is
  the release event of a place whose `placeOk` holds, so
  `QCore.MemberEffectSummary.body_sound` applies to every path unchanged.
* **The row's scope plays no part** (`the_unit_plays_no_part`): admission is
  a function of the actual and its own body's place check, so the other
  functions of the unit (the `hypothesis_context.scope` list) cannot change
  it; the registered diagnosis (a multi-function component) was not the
  premise.

## What this is not

A model of Clang or of the lexical reader. That Clang's `type` of a
`MemberExpr` is the member's declared type, and that a `*`-spelled member the
lexical reader classes is an object pointer, are trust boundary 28's; they
enter as the `Actual.type` field and the `LexicalSound` hypothesis.
-/

namespace QCore.MemberReleaseTypedMember

open QCore.MemberEffectSummary

/-- The member's C type, as Clang records it. -/
inductive MemberType where
  | objectPointer
  | qualifiedPointer
  | functionPointer
  | array
  | scalar
  deriving DecidableEq, Repr

/-- How the source spells the member's declaration. -/
inductive Spelling where
  /-- `T *F;` with a specifier list the lexical reader classes. -/
  | star
  /-- `P F;` with `typedef T *P;`, which the reader leaves unresolved. -/
  | pointerTypedef
  /-- Any other spelling (an array, a function pointer, a scalar). -/
  | other
  deriving DecidableEq, Repr

/-- The lexical recogniser's answer for `B->F` (`pointer_read`). -/
inductive Lexical where
  | pointerRead
  | untypedMember
  | untypedBase
  deriving DecidableEq, Repr

/-- Clang's node for the actual, through parentheses. -/
inductive Node where
  /-- An `LValueToRValue` read of a member of that type. -/
  | value (t : MemberType)
  /-- An `ArrayToPointerDecay`: an address, no read. -/
  | decay
  deriving DecidableEq, Repr

/-- One actual `B->F` at a call. -/
structure Actual where
  place : Place
  /-- The lexical reader typed `B` as a pointer to a record. -/
  baseTyped : Bool
  /-- `B` points to a `union`. -/
  unionBase : Bool
  /-- Clang's arrow `MemberExpr` is this actual's, named `F`, over the
  parameter `B`. -/
  nodeIsActual : Bool
  spelling : Spelling
  type : MemberType
  deriving DecidableEq, Repr

def lexical (a : Actual) : Lexical :=
  if a.baseTyped then
    match a.spelling with
    | .star => .pointerRead
    | _ => .untypedMember
  else .untypedBase

/-- Clang reads an array member as its decay, every other member as a value. -/
def node (a : Actual) : Node :=
  match a.type with
  | .array => .decay
  | t => .value t

/-- `Parser::typed_member_release`. -/
def typedPointer (a : Actual) : Bool :=
  a.nodeIsActual && !a.unionBase && node a == .value .objectPointer

/-- Rule 1 before this card. -/
def oldAdmits (nullable placeChecked : Bool) (a : Actual) : Bool :=
  nullable && placeChecked && lexical a == .pointerRead

/-- Rule 1 with the amendment. -/
def admits (nullable placeChecked : Bool) (a : Actual) : Bool :=
  nullable && placeChecked &&
    (lexical a == .pointerRead || (lexical a == .untypedMember && typedPointer a))

/-- The lexical reader's own premise: a `*`-spelled member it classes over a
record pointer it typed is an object pointer, read from no `union`. -/
def LexicalSound (a : Actual) : Prop :=
  a.baseTyped = true → a.spelling = .star → a.type = .objectPointer ∧ a.unionBase = false

theorem an_old_admission_is_kept (n c : Bool) (a : Actual) (h : oldAdmits n c a = true) :
    admits n c a = true := by
  simp_all [oldAdmits, admits]

theorem a_new_admission_is_an_untyped_member (n c : Bool) (a : Actual)
    (h : admits n c a = true) (hold : oldAdmits n c a = false) :
    lexical a = .untypedMember ∧ typedPointer a = true := by
  simp only [admits, oldAdmits, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at h hold
  rcases h with ⟨⟨hn, hc⟩, hl | ⟨hu, ht⟩⟩
  · simp_all
  · exact ⟨hu, ht⟩

theorem an_admission_needs_the_place_and_a_nullable_release (n c : Bool) (a : Actual)
    (h : admits n c a = true) : n = true ∧ c = true := by
  simp only [admits, Bool.and_eq_true] at h
  exact h.1

theorem an_untyped_base_is_never_admitted (n c : Bool) (a : Actual)
    (h : a.baseTyped = false) : admits n c a = false := by
  simp [admits, lexical, h]

theorem a_decayed_array_is_refused (a : Actual) (h : a.type = .array) :
    typedPointer a = false := by
  simp [typedPointer, node, h]

theorem a_function_pointer_is_refused (a : Actual) (h : a.type = .functionPointer) :
    typedPointer a = false := by
  simp [typedPointer, node, h]

theorem a_qualified_pointer_is_refused (a : Actual) (h : a.type = .qualifiedPointer) :
    typedPointer a = false := by
  simp [typedPointer, node, h]

theorem a_union_base_is_refused (a : Actual) (h : a.unionBase = true) :
    typedPointer a = false := by
  simp [typedPointer, h]

theorem typedPointer_type (a : Actual) (h : typedPointer a = true) :
    a.type = .objectPointer ∧ a.unionBase = false := by
  unfold typedPointer node at h
  cases ht : a.type <;> simp_all

theorem an_admitted_actual_is_an_object_pointer (n c : Bool) (a : Actual)
    (hs : LexicalSound a) (h : admits n c a = true) :
    a.type = .objectPointer ∧ a.unionBase = false := by
  simp only [admits, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at h
  rcases h with ⟨_, hl | ⟨_, ht⟩⟩
  · unfold lexical at hl
    cases hb : a.baseTyped
    · simp [hb] at hl
    · cases hsp : a.spelling
      · exact hs hb hsp
      all_goals simp_all
  · exact typedPointer_type a ht

/-- The admitted actual is the release event of its place; with the place's
body check holding, every path of the body is free of use after release and
double release (`body_sound`, unchanged). -/
theorem the_body_stays_sound (p : Place) (b s : List Ev) (a : Actual)
    (h : admits true (placeOk p b) a = true) (hs : s.Sublist b) :
    safe p s = true := by
  have := (an_admission_needs_the_place_and_a_nullable_release _ _ a h).2
  exact body_sound p b s this hs

/-- Admission read in a unit whose other functions are `unit`: the same for
every unit. -/
def admitsIn (_unit : List String) (n c : Bool) (a : Actual) : Bool := admits n c a

theorem the_unit_plays_no_part (u v : List String) (n c : Bool) (a : Actual) :
    admitsIn u n c a = admitsIn v n c a := rfl

end QCore.MemberReleaseTypedMember
