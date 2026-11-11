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

/-! Bounded typed contract facts used by the native source/identity policy
consumer. This models the evaluator's load-bearing decisions: text and integer
facts are disjoint, equality is same-typed, arithmetic and ordering are integer
only, both Boolean operands are evaluated, and duplicate environments refuse.
The Rust parser, source bounds and string decoder remain implementation
boundaries. -/

namespace QCore.TypedContractFacts

inductive Fact where
  | integer (value : Int)
  | text (value : String)
  deriving DecidableEq, Repr

abbrev Facts := List (String × Fact)

inductive Refusal where
  | missing (name : String)
  | duplicate (name : String)
  | typeMismatch
  | overflow
  deriving DecidableEq, Repr

inductive Expr where
  | literal (value : Fact)
  | named (name : String)
  | equal (left right : Expr)
  | notEqual (left right : Expr)
  | less (left right : Expr)
  | add (left right : Expr)
  | and (left right : Expr)
  | or (left right : Expr)
  | not (value : Expr)
  deriving DecidableEq, Repr

def i128Min : Int := -(2 ^ 127)
def i128Max : Int := 2 ^ 127 - 1

def inI128 (value : Int) : Bool := i128Min ≤ value && value ≤ i128Max

def findFact (facts : Facts) (name : String) : Except Refusal Fact :=
  match facts.find? (fun entry => entry.1 == name) with
  | some entry => .ok entry.2
  | none => .error (.missing name)

def validateFacts : Facts → Except Refusal Facts
  | [] => .ok []
  | entry :: rest =>
      if rest.any (fun candidate => candidate.1 == entry.1) then
        .error (.duplicate entry.1)
      else
        match validateFacts rest with
        | .ok checked => .ok (entry :: checked)
        | .error refusal => .error refusal

private def integer (fact : Fact) : Except Refusal Int :=
  match fact with
  | .integer value => .ok value
  | .text _ => .error .typeMismatch

private def truth (fact : Fact) : Except Refusal Bool := do
  return (← integer fact) != 0

private def sameTypedEqual (left right : Fact) : Except Refusal Bool :=
  match left, right with
  | .integer lhs, .integer rhs => .ok (lhs == rhs)
  | .text lhs, .text rhs => .ok (lhs == rhs)
  | _, _ => .error .typeMismatch

def eval (facts : Facts) : Expr → Except Refusal Fact
  | .literal value => .ok value
  | .named name => findFact facts name
  | .equal left right => do
      let lhs ← eval facts left
      let rhs ← eval facts right
      return .integer (if ← sameTypedEqual lhs rhs then 1 else 0)
  | .notEqual left right => do
      let lhs ← eval facts left
      let rhs ← eval facts right
      return .integer (if ← sameTypedEqual lhs rhs then 0 else 1)
  | .less left right => do
      let lhs ← integer (← eval facts left)
      let rhs ← integer (← eval facts right)
      return .integer (if lhs < rhs then 1 else 0)
  | .add left right => do
      let lhs ← integer (← eval facts left)
      let rhs ← integer (← eval facts right)
      let result := lhs + rhs
      if inI128 result then return .integer result else throw .overflow
  | .and left right => do
      -- The two binds are deliberately before the Boolean operation: a false
      -- left operand cannot hide an undefined or ill-typed right operand.
      let lhs ← truth (← eval facts left)
      let rhs ← truth (← eval facts right)
      return .integer (if lhs && rhs then 1 else 0)
  | .or left right => do
      let lhs ← truth (← eval facts left)
      let rhs ← truth (← eval facts right)
      return .integer (if lhs || rhs then 1 else 0)
  | .not value => do
      return .integer (if ← truth (← eval facts value) then 0 else 1)

structure PolicyShape where
  isExtern : Bool
  isSafe : Bool
  parameters : Nat
  unitResult : Bool
  hasEffects : Bool
  normalPostconditions : Nat
  unsupportedClauses : Bool
  deriving DecidableEq, Repr

def strict (shape : PolicyShape) : Bool :=
  shape.isExtern && shape.isSafe && shape.parameters == 0 && shape.unitResult &&
    !shape.hasEffects && shape.normalPostconditions > 0 && !shape.unsupportedClauses

theorem equal_text_is_decided :
    eval [] (.equal (.literal (.text "same")) (.literal (.text "same"))) =
      .ok (.integer 1) := by
  rfl

theorem unequal_text_is_decided :
    eval [] (.notEqual (.literal (.text "left")) (.literal (.text "right"))) =
      .ok (.integer 1) := by
  rfl

theorem mixed_equality_refuses :
    eval [] (.equal (.literal (.text "1")) (.literal (.integer 1))) =
      .error .typeMismatch := by
  rfl

theorem ordered_text_refuses :
    eval [] (.less (.literal (.text "a")) (.literal (.text "z"))) =
      .error .typeMismatch := by
  rfl

theorem false_and_missing_still_refuses :
    eval [] (.and (.literal (.integer 0)) (.named "missing")) =
      .error (.missing "missing") := by
  rfl

theorem true_or_missing_still_refuses :
    eval [] (.or (.literal (.integer 1)) (.named "missing")) =
      .error (.missing "missing") := by
  rfl

theorem i128_addition_overflow_refuses :
    eval [] (.add (.literal (.integer i128Max)) (.literal (.integer 1))) =
      .error .overflow := by
  rfl

theorem duplicate_facts_refuse :
    validateFacts [("path", .text "a"), ("path", .text "b")] =
      .error (.duplicate "path") := by
  rfl

theorem an_unsafe_policy_is_not_strict :
    strict ⟨true, false, 0, true, false, 1, false⟩ = false := by
  decide

theorem an_effectful_policy_is_not_strict :
    strict ⟨true, true, 0, true, true, 1, false⟩ = false := by
  decide

theorem a_postcondition_is_required :
    strict ⟨true, true, 0, true, false, 0, false⟩ = false := by
  decide

end QCore.TypedContractFacts
