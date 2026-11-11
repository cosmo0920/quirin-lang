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

/-! # Fixed scalar C queries (wave 23)

The gateway erases a query ONLY for its custody predicate. C11 6.5.3.4 and
faithful extraction of a scalar type from the unit are premises. This model
proves that erasing admitted query operands preserves surrounding effects,
including effects beside the query; VLA and opaque operands cannot be erased.
No size, alignment, target ABI or concrete C evaluation is computed here.
-/
namespace QCore.FixedScalarQuery

inductive Operator | sizeOf | alignOf deriving DecidableEq, Repr
inductive Operand
  | scalarType | scalarLocal | variableArray | pointer | compound | opaque
  deriving DecidableEq, Repr

def admitted (op : Operator) (operand : Operand) : Bool :=
  match op, operand with
  | _, .scalarType => true
  | .sizeOf, .scalarLocal => true
  | _, _ => false

/-- An effect outside a query must survive. The list in `query` is the
potential evaluation effect of an unadmitted operand, e.g. a VLA bound. -/
inductive Piece
  | effect (id : Nat)
  | query (op : Operator) (operand : Operand) (potential : List Nat)
  deriving DecidableEq, Repr

def effects : List Piece → List Nat
  | [] => []
  | .effect id :: rest => id :: effects rest
  | .query op operand potential :: rest =>
    (if admitted op operand then [] else potential) ++ effects rest

def erase : List Piece → Option (List Nat)
  | [] => some []
  | .effect id :: rest => (erase rest).map (id :: ·)
  | .query op operand _ :: rest =>
    if admitted op operand then erase rest else none

/-- The whole surrounding effect sequence is preserved, not just its length. -/
theorem erasure_preserves_effects (xs : List Piece) (ys : List Nat)
    (h : erase xs = some ys) : effects xs = ys := by
  induction xs generalizing ys with
  | nil => simpa [erase, effects] using h
  | cons head tail ih =>
    cases head with
    | effect id =>
      cases ht : erase tail with
      | none => simp [erase, ht] at h
      | some zs =>
        have eq : id :: zs = ys := by simpa [erase, ht] using h
        simpa [effects, ih zs ht] using eq
    | query op operand potential =>
      cases ha : admitted op operand <;> simp [erase, ha] at h
      simpa [effects, ha] using ih ys h

theorem variable_array_refuses (op : Operator) (potential : List Nat)
    (tail : List Piece) : erase (.query op .variableArray potential :: tail) = none := by
  cases op <;> rfl

theorem opaque_operand_refuses (op : Operator) (potential : List Nat)
    (tail : List Piece) : erase (.query op .opaque potential :: tail) = none := by
  cases op <;> rfl

theorem alignof_local_refuses : admitted .alignOf .scalarLocal = false := rfl

theorem scalar_queries_preserve_a_sibling_call :
    erase [.query .sizeOf .scalarType [], .effect 7,
      .query .alignOf .scalarType []] = some [7] := rfl

/-- Scalar facts and custody are separate. A query assignment forgets its
scalar destination's previous value, since this projection computes no size. -/
structure State where
  scalar : Nat → Option Nat
  custody : List Nat

def assignQuery (s : State) (destination : Nat) : State :=
  { scalar := fun name => if name = destination then none else s.scalar name
    custody := s.custody }

theorem query_assignment_preserves_custody (s : State) (destination : Nat) :
    (assignQuery s destination).custody = s.custody := rfl

theorem query_assignment_forgets_old_value (s : State) (destination : Nat) :
    (assignQuery s destination).scalar destination = none := by simp [assignQuery]

theorem other_scalar_facts_survive (s : State) (destination name : Nat)
    (h : name ≠ destination) :
    (assignQuery s destination).scalar name = s.scalar name := by simp [assignQuery, h]

/-- Unknown scalar data permits either branch. No predicate marker becomes a
concrete result or a nullness refinement. -/
def permits (fact : Option Nat) (branch : Bool) : Bool :=
  match fact with
  | none => true
  | some n => (n != 0) == branch

theorem query_assignment_keeps_both_branches (s : State) (destination : Nat)
    (branch : Bool) : permits ((assignQuery s destination).scalar destination) branch = true := by
  simp [query_assignment_forgets_old_value, permits]

end QCore.FixedScalarQuery
