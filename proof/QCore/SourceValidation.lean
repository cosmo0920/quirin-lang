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

namespace QCore.SourceValidation

/-! Reduced r0.3 model of the executable result gate, signed literal boundary,
and unconditional axis declaration validation. This models the Rust boundary
rules; it is not a parser refinement or a theorem about arbitrary result ABIs. -/

inductive ResultType where
  | i32 | unit | unsupported
  deriving DecidableEq, Repr

inductive ReturnValue where
  | literal (value : Int)
  | bare
  | unsupported
  deriving DecidableEq, Repr

def InSignedRange (limit : Nat) (value : Int) : Prop :=
  -(Int.ofNat limit) ≤ value ∧ value < Int.ofNat limit

instance (limit : Nat) (value : Int) : Decidable (InSignedRange limit value) := by
  unfold InSignedRange
  infer_instance

def admitReturn : ResultType → ReturnValue → Option Int
  | .i32, .literal value => if InSignedRange 2147483648 value then some value else none
  | .unit, .bare => some 0
  | _, _ => none

theorem admitted_return_has_declared_shape {result : ResultType} {input : ReturnValue}
    {value : Int} (accepted : admitReturn result input = some value) :
    (result = .i32 ∧ input = .literal value ∧ InSignedRange 2147483648 value) ∨
    (result = .unit ∧ input = .bare ∧ value = 0) := by
  cases result <;> cases input <;> simp [admitReturn] at accepted ⊢
  · exact ⟨accepted.2, accepted.2 ▸ accepted.1⟩
  · exact accepted.symm

theorem unsupported_result_rejected (input : ReturnValue) :
    admitReturn .unsupported input = none := by cases input <;> rfl

theorem integer_cannot_satisfy_unit (value : Int) :
    admitReturn .unit (.literal value) = none := rfl

theorem bare_cannot_satisfy_integer : admitReturn .i32 .bare = none := rfl

/-- Custody is a separate identity-ledger input; the scalar gate never
claims to establish its owned-result type or lifetime invariant. -/
def recordReturn (result : ResultType) (input : ReturnValue) (custody : List Nat) :
    Option (Int × List Nat) :=
  (admitReturn result input).map (fun value => (value, custody))

theorem scalar_check_preserves_separate_custody {result : ResultType} {input : ReturnValue}
    {custody : List Nat} {output : Int × List Nat}
    (accepted : recordReturn result input custody = some output) : output.2 = custody := by
  unfold recordReturn at accepted
  cases h : admitReturn result input <;> simp [h] at accepted
  cases accepted
  rfl

def signedLiteral (limit : Nat) (negative : Bool) (magnitude : Nat) : Option Int :=
  let value := if negative then -(Int.ofNat magnitude) else Int.ofNat magnitude
  if InSignedRange limit value then some value else none

theorem signed_literal_is_in_range {limit magnitude : Nat} {negative : Bool}
    {value : Int} (accepted : signedLiteral limit negative magnitude = some value) :
    InSignedRange limit value := by
  cases negative <;> simp [signedLiteral] at accepted
  · exact accepted.2 ▸ accepted.1
  · exact accepted.2 ▸ accepted.1

example : admitReturn .i32 (.literal (-2147483648)) = some (-2147483648) := by decide
example : signedLiteral 2147483648 true 2147483648 = some (-2147483648) := by decide
example : signedLiteral 2147483648 false 2147483648 = none := by decide
example : signedLiteral 2147483648 true 2147483649 = none := by decide
example : signedLiteral 170141183460469231731687303715884105728 true
    170141183460469231731687303715884105728 =
    some (-170141183460469231731687303715884105728) := by decide

inductive Dimension where
  | named (name : String)
  | count (nominal : String)
  deriving DecidableEq, Repr

structure Axis where
  name : String
  dimension : Dimension
  deriving DecidableEq, Repr

def dimensionValid (dimensions : List String) : Dimension → Bool
  | .named name => dimensions.contains name
  | .count _ => true

def declarationsValid (dimensions : List String) (axes : List Axis) : Prop :=
  dimensions.Nodup ∧ (axes.map Axis.name).Nodup ∧
  axes.all (fun axis => dimensionValid dimensions axis.dimension) = true

instance (dimensions : List String) (axes : List Axis) :
    Decidable (declarationsValid dimensions axes) := by
  unfold declarationsValid
  infer_instance

def validateDeclarations (dimensions : List String) (axes : List Axis)
    (_quantitativeTypeUses : List String) : Bool := decide (declarationsValid dimensions axes)

theorem declaration_validation_independent_of_uses
    (dimensions : List String) (axes : List Axis) (first second : List String) :
    validateDeclarations dimensions axes first = validateDeclarations dimensions axes second := rfl

theorem accepted_declarations_are_valid {dimensions : List String} {axes : List Axis}
    {uses : List String} (accepted : validateDeclarations dimensions axes uses = true) :
    declarationsValid dimensions axes := of_decide_eq_true accepted

example : validateDeclarations [] [⟨"bad", .named "Missing"⟩] [] = false := by decide
example : validateDeclarations [] [⟨"objects", .count "Nominal"⟩] [] = true := by decide

end QCore.SourceValidation
