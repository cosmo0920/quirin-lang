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

/-!
# A member declared through a pointer typedef (card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3, gateway rule 68)

The abstract rule implemented by `crates/qcore-c-gateway/src/pointer_typedef_member.rs`
and its hooks in `typed_field_access.rs`.

C11 6.7.8p3: a typedef name is a synonym for the type it names. Where every
typedef declaration of `A` in the unit is `typedef SPEC * A;` with one
`SPEC`, a member declared exactly `A NAME;` has type `SPEC *`. The member
classifier reads the declarator's star count over a specifier, so the model
proves that reading `A NAME` through the agreeing alias gives exactly the
class `SPEC * NAME` gets, that a disagreeing or non-pointer alias gives
nothing (the classifier's earlier answer), and that the reading never names a
pointer to a pointer, a `void` pointee or an unresolved pointee a single
pointer.

## What this is not

A model of the lexical reader or of Clang. The Rust decision
(`single_pointer_member`) is bound to `DECISION_VECTORS`; faithful
tokenisation is trust boundary 28.
-/

namespace QCore.PointerTypedefMember

/-- A specifier list's class, as the shared classifier answers it. -/
inductive Spec where
  | scalar
  | struct (tag : Nat)
  | void
  | unresolved
  deriving DecidableEq, Repr

/-- A member declaration: its specifier class and its star count. -/
structure Member where
  spec : Spec
  stars : Nat
  deriving DecidableEq, Repr

/-- The member classes rule 68 touches. -/
inductive MemberClass where
  /-- A single pointer member (exactly one `*` over a resolved pointee). -/
  | singlePointer
  /-- In neither class this rule reads. -/
  | none
  deriving DecidableEq, Repr

/-- The classifier on a written (desugared) declaration. -/
def classify (m : Member) : MemberClass :=
  if m.stars = 1 then
    match m.spec with
    | .scalar | .struct _ => .singlePointer
    | _ => .none
  else .none

/-- What the unit's typedef declarations of one alias say: every one agrees
on `typedef SPEC * A;` (`some SPEC`) or not (`none`). -/
abbrev AliasFact := Option Spec

/-- Rule 68: the class of a member `A NAME` (no star) through the alias. -/
def classifyAlias (fact : AliasFact) (stars : Nat) : MemberClass :=
  match fact with
  | some spec => if stars = 0 then classify ⟨spec, 1⟩ else .none
  | none => .none

/-- The synonym reading: `A NAME` through an agreeing alias is classified as
`SPEC * NAME`. -/
theorem an_agreeing_alias_member_is_its_pointer_spelling (spec : Spec) :
    classifyAlias (some spec) 0 = classify ⟨spec, 1⟩ := by
  simp [classifyAlias]

/-- A disagreeing (or non-pointer) alias reads nothing. -/
theorem a_disagreeing_alias_reads_nothing (stars : Nat) :
    classifyAlias none stars = .none := rfl

/-- `A * NAME` (a pointer to a pointer) is never a single pointer here. -/
theorem a_pointer_to_the_alias_is_never_single (fact : AliasFact) (stars : Nat)
    (h : stars ≠ 0) : classifyAlias fact stars = .none := by
  cases fact <;> simp [classifyAlias, h]

/-- A `void` or unresolved pointee is never a single pointer. -/
theorem an_unresolved_pointee_is_never_single :
    classifyAlias (some .void) 0 = .none ∧ classifyAlias (some .unresolved) 0 = .none := by
  decide

/-- Rule 68's decision on the Rust's terms (`single_pointer_member`). -/
def singlePointerMember (agreeing plainMember resolved : Bool) : Bool :=
  agreeing && plainMember && resolved

/-- The Rust's `DECISION_VECTORS`. -/
def decisionVectors : List (Bool × Bool × Bool × Bool) :=
  [(true, true, true, true), (false, true, true, false), (true, false, true, false),
   (true, true, false, false), (false, false, false, false)]

theorem decision_vectors_hold :
    decisionVectors.all (fun (a, p, r, x) => singlePointerMember a p r == x) = true := by
  decide

/-- The decision is the model's: admitted exactly when the alias agrees, the
member carries no star and the pointee resolves. -/
theorem the_decision_is_the_models (fact : AliasFact) (stars : Nat) :
    (classifyAlias fact stars = .singlePointer) ↔
      ∃ spec, fact = some spec ∧ stars = 0 ∧
        (spec = .scalar ∨ ∃ t, spec = .struct t) := by
  cases fact with
  | none => simp [classifyAlias]
  | some spec =>
      by_cases hs : stars = 0
      · cases spec <;> simp [classifyAlias, classify, hs]
      · simp [classifyAlias, hs]

end QCore.PointerTypedefMember
