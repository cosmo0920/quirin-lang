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

import QCore.CastSemantics

/-!
# The return position's placement of the view removal (card CG10-READER-GAPS, rule 71a)

Rule 12i reads a refused whole-call return with rule W44 but declines a call
whose actuals hide places behind identity-preserving views (rule 15z's
guard). Rule 71a, at rule 12i's placement, removes **every** view the guard
names and reads the uncast call; it reads only when every removed view's
pointee is `void`, a character, a scalar (or a typedef of one) or a tagged
`struct` (`return_cast_view::read_uncast`, bound to `RETURN_VIEW_VECTORS`).

The model proves:

* the decision vectors, and that a view to any other pointee (a union, a
  record typedef: rule 10w's) or a non-whole-call return is never read;
* **nothing is left hidden**: after the removal no actual is a view the guard
  names, so the reason rule 15z declined (a hidden place W44 reads as no
  place) is gone;
* **the callee is handed the same identities**: removing the views keeps
  every actual's identity (`QCore.CastSemantics.view_preserves_identity` for
  each removed view), so the uncast call's reads decide exactly the objects
  the cast call hands over.

## What this is not

A model of the classifier or of W44: that rule 12i and W4 decide the uncast
call at the contract's positions is theirs (`QCore.ReturnCallActuals`), and
the token/AST match is trust boundary 28.
-/

namespace QCore.ReturnCastView

open QCore.CastSemantics

/-- What a removed view points at. -/
inductive ViewPointee where
  | byte
  | scalar
  | taggedStruct
  | other
  deriving DecidableEq, Repr

/-- A view whose pointee is not `other` is removed. -/
def removablePointee (v : ViewPointee) : Bool := v != .other

/-- Rule 71a's decision. -/
def readUncast (wholeCall : Bool) (views : List ViewPointee) : Bool :=
  wholeCall && !views.isEmpty && views.all removablePointee

/-- The Rust's `RETURN_VIEW_VECTORS`. -/
def returnViewVectors : List (Bool × List ViewPointee × Bool) :=
  [ (true, [.byte], true), (true, [.scalar], true), (true, [.taggedStruct], true),
    (true, [.byte, .byte], true), (true, [.other], false), (true, [.byte, .other], false),
    (false, [.byte], false), (true, [], false) ]

theorem return_view_vectors_hold :
    returnViewVectors.all (fun (w, vs, r) => readUncast w vs == r) = true := by
  decide

/-- Only a whole-call return is read. -/
theorem only_a_whole_call_is_read (vs : List ViewPointee) : readUncast false vs = false := by
  simp [readUncast]

/-- A view to a union or a record typedef (`other`) keeps the refusal. -/
theorem a_kept_view_keeps_the_refusal (w : Bool) (vs : List ViewPointee)
    (h : .other ∈ vs) : readUncast w vs = false := by
  cases w with
  | false => rfl
  | true =>
    simp only [readUncast, Bool.true_and, Bool.and_eq_false_iff]
    right
    simp only [List.all_eq_false]
    exact ⟨.other, h, by decide⟩

/-! ## Nothing is left hidden, and the identities are kept -/

/-- One actual of the returned call: a place's value (its identity), a view
of a place (the guard names it), or anything else. -/
inductive Actual where
  | place (identity : Nat)
  | view (identity : Nat)
  | other
  deriving DecidableEq, Repr

/-- The identity an actual hands the callee (`none`: no tracked identity). -/
def handed : Actual → Option Nat
  | .place i => some i
  | .view i => some i
  | .other => none

/-- Whether the guard names this actual. -/
def hides : Actual → Bool
  | .view _ => true
  | _ => false

/-- The removal of one view: its operand's place. -/
def uncast : Actual → Actual
  | .view i => .place i
  | a => a

/-- After the removal no actual hides a place. -/
theorem nothing_is_left_hidden (as : List Actual) : (as.map uncast).any hides = false := by
  induction as with
  | nil => rfl
  | cons a as ih =>
    cases a <;> simp_all [uncast, hides]

/-- The removal hands the callee exactly the identities the cast call does. -/
theorem the_same_identities_are_handed (as : List Actual) :
    (as.map uncast).map handed = as.map handed := by
  induction as with
  | nil => rfl
  | cons a as ih =>
    cases a <;> simp_all [uncast, handed]

/-- Each removed view is an identity-preserving conversion of its operand
(`view_preserves_identity`), which is what `handed (view i) = some i`
abstracts. -/
theorem a_removed_view_is_its_place (transfer : Int → Int) (c : ConvClass) (target : Pointee)
    (v v' : Val) (hc : c = .byteView ∨ c = .typedView ∨ c = .punnedView ∨ c = .qualifier)
    (h : convert transfer c target v = .value v') : identityOf v' = identityOf v :=
  view_preserves_identity transfer c target v v' hc h

end QCore.ReturnCastView
