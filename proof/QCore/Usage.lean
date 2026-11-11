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

namespace QCore

inductive UsageUpper where
  | finite : Nat -> UsageUpper
  | infinity : UsageUpper
  deriving DecidableEq, Repr

namespace UsageUpper

def covers : UsageUpper -> Nat -> Prop
  | finite upper, lower => lower <= upper
  | infinity, _ => True

def Le : UsageUpper -> UsageUpper -> Prop
  | finite left, finite right => left <= right
  | finite _, infinity => True
  | infinity, finite _ => False
  | infinity, infinity => True

def add : UsageUpper -> UsageUpper -> UsageUpper
  | finite a, finite b => finite (a + b)
  | _, _ => infinity

def mul : UsageUpper -> UsageUpper -> UsageUpper
  | finite a, finite b => finite (a * b)
  | finite 0, infinity => finite 0
  | infinity, finite 0 => finite 0
  | _, _ => infinity

def max : UsageUpper -> UsageUpper -> UsageUpper
  | finite a, finite b => finite (Nat.max a b)
  | _, _ => infinity

theorem add_covers {a b : UsageUpper} {left right : Nat}
    (ha : a.covers left) (hb : b.covers right) :
    (a.add b).covers (left + right) := by
  cases a <;> cases b
  · exact Nat.add_le_add ha hb
  · trivial
  · trivial
  · trivial

theorem mul_covers {a b : UsageUpper} {left right : Nat}
    (ha : a.covers left) (hb : b.covers right) :
    (a.mul b).covers (left * right) := by
  cases a with
  | finite a =>
      cases b with
      | finite b => exact Nat.mul_le_mul ha hb
      | infinity =>
          cases a with
          | zero =>
              have leftZero : left = 0 := Nat.eq_zero_of_le_zero ha
              simp [mul, covers, leftZero]
          | succ a => trivial
  | infinity =>
      cases b with
      | finite b =>
          cases b with
          | zero =>
              have rightZero : right = 0 := Nat.eq_zero_of_le_zero hb
              simp [mul, covers, rightZero]
          | succ b => trivial
      | infinity => trivial

theorem left_le_max (a b : UsageUpper) : Le a (max a b) := by
  cases a <;> cases b
  · exact Nat.le_max_left _ _
  · trivial
  · trivial
  · trivial

theorem right_le_max (a b : UsageUpper) : Le b (max a b) := by
  cases a <;> cases b
  · exact Nat.le_max_right _ _
  · trivial
  · trivial
  · trivial

theorem max_le {a b bound : UsageUpper} (ha : Le a bound) (hb : Le b bound) :
    Le (max a b) bound := by
  cases a <;> cases b <;> cases bound
  · exact Nat.max_le.mpr ⟨ha, hb⟩
  · trivial
  · contradiction
  · trivial
  · contradiction
  · trivial
  · contradiction
  · trivial

end UsageUpper

structure Usage where
  lower : Nat
  upper : UsageUpper
  deriving DecidableEq, Repr

namespace Usage

def Valid (usage : Usage) : Prop := usage.upper.covers usage.lower

/-- `actual.Refines declared` means every actual demand is admitted by the
    declared interval. -/
def Refines (actual declared : Usage) : Prop :=
  declared.lower <= actual.lower ∧ UsageUpper.Le actual.upper declared.upper

/-- The checker accepts only valid inferred intervals that refine the declared
    grade. -/
def Accepts (declared actual : Usage) : Prop := actual.Valid ∧ actual.Refines declared

def add (a b : Usage) : Usage :=
  { lower := a.lower + b.lower, upper := a.upper.add b.upper }

def mul (a b : Usage) : Usage :=
  { lower := a.lower * b.lower, upper := a.upper.mul b.upper }

/-- Least interval enclosing the demands from both branches. -/
def join (a b : Usage) : Usage :=
  { lower := Nat.min a.lower b.lower, upper := a.upper.max b.upper }

def zero : Usage := { lower := 0, upper := .finite 0 }
def one : Usage := { lower := 1, upper := .finite 1 }
def affine : Usage := { lower := 0, upper := .finite 1 }
def relevant : Usage := { lower := 1, upper := .infinity }
def many : Usage := { lower := 0, upper := .infinity }

theorem zero_valid : zero.Valid := by
  exact Nat.le_refl 0

theorem one_valid : one.Valid := by
  exact Nat.le_refl 1

theorem affine_valid : affine.Valid := by
  exact Nat.zero_le 1

theorem relevant_valid : relevant.Valid := by
  trivial

theorem many_valid : many.Valid := by
  trivial

theorem add_valid {a b : Usage} (ha : a.Valid) (hb : b.Valid) : (add a b).Valid := by
  exact UsageUpper.add_covers ha hb

theorem mul_valid {a b : Usage} (ha : a.Valid) (hb : b.Valid) : (mul a b).Valid := by
  exact UsageUpper.mul_covers ha hb

theorem join_valid {a b : Usage} (ha : a.Valid) (hb : b.Valid) : (join a b).Valid := by
  cases a with
  | mk al au =>
      cases b with
      | mk bl bu =>
          cases au <;> cases bu
          · exact Nat.le_trans (Nat.min_le_left al bl)
              (Nat.le_trans ha (Nat.le_max_left _ _))
          · trivial
          · trivial
          · trivial

theorem left_refines_join (a b : Usage) : a.Refines (join a b) := by
  exact And.intro (Nat.min_le_left _ _) (UsageUpper.left_le_max _ _)

theorem right_refines_join (a b : Usage) : b.Refines (join a b) := by
  exact And.intro (Nat.min_le_right _ _) (UsageUpper.right_le_max _ _)

theorem join_refines_of {a b bound : Usage} (ha : a.Refines bound)
    (hb : b.Refines bound) : (join a b).Refines bound := by
  exact And.intro (Nat.le_min.mpr ⟨ha.left, hb.left⟩)
    (UsageUpper.max_le ha.right hb.right)

theorem accepted_by_one_eq {actual : Usage} (accepted : one.Accepts actual) :
    actual = one := by
  rcases actual with ⟨lower, upper⟩
  rcases accepted with ⟨valid, lowerBound, upperBound⟩
  cases upper with
  | finite upper =>
      have upperEq : upper = 1 := Nat.le_antisymm upperBound
        (Nat.le_trans lowerBound valid)
      have lowerEq : lower = 1 := Nat.le_antisymm
        (upperEq ▸ valid) lowerBound
      simp [one, lowerEq, upperEq]
  | infinity => contradiction

theorem accepted_by_zero_eq {actual : Usage} (accepted : zero.Accepts actual) :
    actual = zero := by
  rcases actual with ⟨lower, upper⟩
  rcases accepted with ⟨valid, lowerBound, upperBound⟩
  cases upper with
  | finite upper =>
      have upperEq : upper = 0 := Nat.eq_zero_of_le_zero upperBound
      have lowerEq : lower = 0 := Nat.eq_zero_of_le_zero (upperEq ▸ valid)
      simp [zero, lowerEq, upperEq]
  | infinity => contradiction

end Usage

abbrev UsageVariable := Nat
abbrev UsageContext := UsageVariable -> Usage

namespace UsageContext

def add (left right : UsageContext) : UsageContext :=
  fun name => Usage.add (left name) (right name)

def scale (factor : Usage) (context : UsageContext) : UsageContext :=
  fun name => Usage.mul factor (context name)

def application (function argument : UsageContext) (parameter : Usage) : UsageContext :=
  add function (scale parameter argument)

/-- The cycle-2 usage-substitution law, stated pointwise for a free variable. -/
theorem application_substitution (function argument : UsageContext)
    (parameter : Usage) (name : UsageVariable) :
    application function argument parameter name =
      Usage.add (function name) (Usage.mul parameter (argument name)) := by
  rfl

end UsageContext
end QCore
