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

import QCore.MemberReleaseTypedMember

/-!
# A member release whose base is spelled through a typedef (card MEMBER-RELEASE-UNTYPED-BASE, gateway rule 38j amended again)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_call_argument_access.rs`
(`candidate_arguments`, the `Refusal::UntypedMember | Refusal::UntypedBase`
arm) and `crates/qcore-c-gateway/src/legacy_member_effect.rs`
(`Parser::typed_member_release` with `untyped_base`).

`QCore.MemberReleaseTypedMember` admitted, at a `Released { nullable: true }`
position of a place whose body check holds, an actual `B->F` the lexical
reader refused as an *untyped member*, when Clang reads it as the value of an
object-pointer member of the parameter `B`; it proved that an *untyped base*
is never admitted (`an_untyped_base_is_never_admitted`). This card extends
that rule, never that theorem's subject: `admits` there is the rule before
this card, `admitsBase` here the rule after it. An actual whose base the
lexical reader refused (a formal spelled `T *B` with `typedef struct TAG T;`,
or `TP B` with `typedef struct TAG *TP;`) is admitted when Clang's reading of
the member holds (`typedPointer`) **and** Clang reads the base as a pointer to
a complete `struct` holding `F` (`BaseType.completeStruct`): the member Clang
resolved is a member of a complete `struct` the unit defines, and the base's
pointer type carries no `volatile`/`_Atomic` and no function type.

Proved here:

* **Nothing earlier is relaxed** (`a_previous_admission_is_kept`,
  `a_new_admission_is_an_untyped_base`,
  `an_admission_still_needs_the_place_and_a_nullable_release`,
  `a_failed_place_check_is_refused`): the only new admissions are actuals the
  lexical reader refused as an untyped base, and every admission still needs
  the place's body check (so a released member used again, a second release,
  a loop, a local base all stay refused, as before) and a nullable release.
* **The base must be a complete struct** (`a_union_base_is_refused`,
  `an_incomplete_record_is_refused`, `a_qualified_base_is_refused`,
  `a_local_base_is_refused`), and the member must still be an object
  pointer's value (`a_decayed_array_is_refused`,
  `a_function_pointer_is_refused`).
* **An admitted actual is a struct's object-pointer member**
  (`an_admitted_actual_is_a_struct_pointer_member`), given the lexical
  reader's premise for a `*`-spelled member (`LexicalSound`).
* **The body stays sound** (`the_body_stays_sound`): the admitted actual is
  the release event of a place whose `placeOk` holds, so
  `QCore.MemberEffectSummary.body_sound` applies unchanged.
* **The unit plays no part** (`the_unit_plays_no_part`).

## What this is not

A model of Clang. That a `MemberExpr`'s `referencedMemberDecl` names a member
of the record its base points to, and that its `type` is the member's
declared type, are trust boundary 28's; they enter as the `BaseRead.base`
and `Actual.type` fields.
-/

namespace QCore.MemberReleaseUntypedBase

open QCore.MemberEffectSummary
open QCore.MemberReleaseTypedMember

/-- Clang's reading of the base `B` of an arrow `MemberExpr` naming `F`. -/
inductive BaseType where
  /-- A pointer, no `volatile`/`_Atomic`, whose resolved member is a member
  named `F` of a complete `struct` the unit defines. -/
  | completeStruct
  /-- The resolved member is a `union`'s (directly, or an anonymous union's
  inside the struct, whose base is then no parameter). -/
  | unionRecord
  /-- No complete definition: Clang reads no member at all. -/
  | incomplete
  /-- The base's pointer type carries `volatile` or `_Atomic`. -/
  | qualified
  deriving DecidableEq, Repr

/-- One actual `B->F` at a call, with Clang's reading of its base. -/
structure BaseRead where
  actual : Actual
  base : BaseType
  deriving DecidableEq, Repr

/-- `Parser::typed_member_release` with `untyped_base`. -/
def typedBase (x : BaseRead) : Bool :=
  typedPointer x.actual && x.base == .completeStruct

/-- Rule 1 after this card. -/
def admitsBase (nullable placeChecked : Bool) (x : BaseRead) : Bool :=
  nullable && placeChecked &&
    (lexical x.actual == .pointerRead ||
      (lexical x.actual == .untypedMember && typedPointer x.actual) ||
      (lexical x.actual == .untypedBase && typedBase x))

theorem a_previous_admission_is_kept (n c : Bool) (x : BaseRead)
    (h : admits n c x.actual = true) : admitsBase n c x = true := by
  simp only [admits, admitsBase, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at h ⊢
  rcases h with ⟨hnc, hl | hm⟩
  · exact ⟨hnc, Or.inl (Or.inl hl)⟩
  · exact ⟨hnc, Or.inl (Or.inr hm)⟩

theorem a_new_admission_is_an_untyped_base (n c : Bool) (x : BaseRead)
    (h : admitsBase n c x = true) (hold : admits n c x.actual = false) :
    lexical x.actual = .untypedBase ∧ typedBase x = true := by
  simp only [admitsBase, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at h
  rcases h with ⟨⟨hn, hc⟩, (hl | ⟨hu, ht⟩) | hb⟩
  · simp [admits, hn, hc, hl] at hold
  · simp [admits, hn, hc, hu, ht] at hold
  · exact hb

theorem an_admission_still_needs_the_place_and_a_nullable_release (n c : Bool)
    (x : BaseRead) (h : admitsBase n c x = true) : n = true ∧ c = true := by
  simp only [admitsBase, Bool.and_eq_true] at h
  exact h.1

theorem a_failed_place_check_is_refused (n : Bool) (x : BaseRead) :
    admitsBase n false x = false := by
  simp [admitsBase]

theorem a_union_base_is_refused (x : BaseRead) (h : x.base = .unionRecord) :
    typedBase x = false := by
  simp [typedBase, h]

theorem an_incomplete_record_is_refused (x : BaseRead) (h : x.base = .incomplete) :
    typedBase x = false := by
  simp [typedBase, h]

theorem a_qualified_base_is_refused (x : BaseRead) (h : x.base = .qualified) :
    typedBase x = false := by
  simp [typedBase, h]

/-- A base that is a local, not the parameter: Clang's node is not the
parameter's (`nodeIsActual` false), and rule 5k's and rule 38j's P1 refuse
the place besides. -/
theorem a_local_base_is_refused (x : BaseRead) (h : x.actual.nodeIsActual = false) :
    typedBase x = false := by
  simp [typedBase, typedPointer, h]

theorem a_decayed_array_is_refused (x : BaseRead) (h : x.actual.type = .array) :
    typedBase x = false := by
  simp [typedBase, MemberReleaseTypedMember.a_decayed_array_is_refused x.actual h]

theorem a_function_pointer_is_refused (x : BaseRead)
    (h : x.actual.type = .functionPointer) : typedBase x = false := by
  simp [typedBase, MemberReleaseTypedMember.a_function_pointer_is_refused x.actual h]

theorem an_admitted_actual_is_a_struct_pointer_member (n c : Bool) (x : BaseRead)
    (hs : LexicalSound x.actual) (h : admitsBase n c x = true) :
    x.actual.type = .objectPointer ∧ x.actual.unionBase = false ∧
      (lexical x.actual = .untypedBase → x.base = .completeStruct) := by
  simp only [admitsBase, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at h
  rcases h with ⟨hnc, (hl | hm) | ⟨hu, hb⟩⟩
  · have hold : admits n c x.actual = true := by
      simp only [admits, Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq]
      exact ⟨hnc, Or.inl hl⟩
    have ⟨ht, hun⟩ := an_admitted_actual_is_an_object_pointer n c x.actual hs hold
    exact ⟨ht, hun, fun hu => by rw [hl] at hu; cases hu⟩
  · have ⟨ht, hun⟩ := typedPointer_type x.actual hm.2
    exact ⟨ht, hun, fun hu => by rw [hm.1] at hu; cases hu⟩
  · simp only [typedBase, Bool.and_eq_true, beq_iff_eq] at hb
    have ⟨ht, hun⟩ := typedPointer_type x.actual hb.1
    exact ⟨ht, hun, fun _ => hb.2⟩

/-- The admitted actual is the release event of its place; with the place's
body check holding, every path of the body is free of use after release and
double release (`body_sound`, unchanged). -/
theorem the_body_stays_sound (p : Place) (b s : List Ev) (x : BaseRead)
    (h : admitsBase true (placeOk p b) x = true) (hs : s.Sublist b) :
    safe p s = true := by
  have := (an_admission_still_needs_the_place_and_a_nullable_release _ _ x h).2
  exact body_sound p b s this hs

/-- Admission read in a unit whose other functions are `unit`: the same for
every unit. -/
def admitsBaseIn (_unit : List String) (n c : Bool) (x : BaseRead) : Bool :=
  admitsBase n c x

theorem the_unit_plays_no_part (u v : List String) (n c : Bool) (x : BaseRead) :
    admitsBaseIn u n c x = admitsBaseIn v n c x := rfl

end QCore.MemberReleaseUntypedBase
