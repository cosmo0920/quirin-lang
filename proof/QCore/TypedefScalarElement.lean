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
# A decayed array member whose element is a scalar typedef (card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4, gateway rule 66)

The abstract rule implemented by `crates/qcore-c-gateway/src/typedef_scalar_element.rs`
and its hook in `rule_hosts.rs::Reader::decayed_address`.

Rule 35's second arm reads a decayed one-dimensional array member at an
admissible actual as the address of its initial element when the element is
a builtin scalar, and refuses every other element class. C11 6.7.8p3: a
typedef name is a synonym for the type it names. Where every typedef
declaration of `T` in the unit resolves to one builtin scalar, an array
`T[N]` is that scalar's array, so the model proves that rule 66's reading of
the element through the agreeing alias gives exactly rule 35's answer on the
scalar spelling, and that a disagreeing alias, a non-scalar resolution or a
non-member operand gives nothing (the earlier refusal).

## What this is not

A model of Clang or of the typed reader. The Rust decision (`decide`) is
bound to `DECISION_VECTORS`; faithful extraction of the written array type
and of each `TypedefDecl`'s desugared type is trust boundary 28.
-/

namespace QCore.TypedefScalarElement

/-- An element class, as rule 35's `element_of` answers a spelling. -/
inductive Element where
  | scalar
  | pointer
  | record
  | array
  | other
  deriving DecidableEq, Repr

/-- Rule 35's arm on a decayed operand: (member access, element class,
qualified) -> admitted. -/
def rule35 (member : Bool) (e : Element) (qualified : Bool) : Bool :=
  member && !qualified && e == .scalar

/-- What the unit's typedef declarations of one name say: every one resolves
to the same element class (`some e`), or they disagree (`none`). -/
abbrev AliasFact := Option Element

/-- Rule 66 on a member whose written element is the typedef name: read the
element through the agreeing alias; only a scalar resolution is admitted. -/
def rule66 (member : Bool) (fact : AliasFact) (qualified : Bool) : Bool :=
  match fact with
  | some e => rule35 member e qualified
  | none => false

/-- The synonym reading: an agreeing alias decides as its resolution's
spelling does under rule 35. -/
theorem an_agreeing_scalar_typedef_is_its_spelling (member qualified : Bool) (e : Element) :
    rule66 member (some e) qualified = rule35 member e qualified := rfl

/-- Two disagreeing declarations read nothing. -/
theorem a_disagreeing_typedef_reads_nothing (member qualified : Bool) :
    rule66 member none qualified = false := rfl

/-- A pointer, record, array or unknown resolution reads nothing. -/
theorem a_non_scalar_resolution_reads_nothing (member qualified : Bool) (e : Element)
    (h : e ≠ .scalar) : rule66 member (some e) qualified = false := by
  cases e <;> simp_all [rule66, rule35]

/-- A non-member operand is never read. -/
theorem a_non_member_is_never_read (fact : AliasFact) (qualified : Bool) :
    rule66 false fact qualified = false := by
  cases fact <;> simp [rule66, rule35]

/-- Rule 66's decision on the Rust's terms (`decide`): (member, the element is
one typedef name, every declaration agrees on a builtin scalar) -> admitted. -/
def decide (member typedefElement agreeingScalar : Bool) : Bool :=
  member && typedefElement && agreeingScalar

/-- The Rust's `DECISION_VECTORS`. -/
def decisionVectors : List (Bool × Bool × Bool × Bool) :=
  [(true, true, true, true), (true, true, false, false), (true, false, true, false),
   (false, true, true, false), (false, false, false, false)]

theorem decision_vectors_hold :
    decisionVectors.all (fun (m, t, a, x) => decide m t a == x) = true := by
  decide

/-- The decision is the model's: for a member whose written element is one
unqualified-but-`const` typedef name, admitted exactly when the alias agrees
on a scalar. -/
theorem the_decision_is_the_models (member : Bool) (fact : AliasFact) :
    rule66 member fact false = decide member true (fact == some .scalar) := by
  cases member <;> cases fact with
  | none => rfl
  | some e => cases e <;> rfl

end QCore.TypedefScalarElement
