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

import QCore.HypothesisGraph

/-! Conditional proposed logic, not source truth or C program termination.
A resolved finite expression is evaluated only under independently checked or
explicitly assumed assignments. Source-reference resolution is fuel-bounded and
rejects active references and nested terminals before evaluation. -/
namespace QCore.PremiseFlow
open HypothesisGraph

def proposition (valuation : Nat → Prop) (atom : Nat) : Prop := valuation atom

def agreement (status : Disposition) (p : Prop) : Prop :=
  match status with
  | .open => True
  | .supported => p
  | .refuted => ¬ p

def all (a b : Disposition) : Disposition :=
  match a, b with
  | .refuted, _ | _, .refuted => .refuted
  | .supported, .supported => .supported
  | _, _ => .open

def any (a b : Disposition) : Disposition :=
  match a, b with
  | .supported, _ | _, .supported => .supported
  | .refuted, .refuted => .refuted
  | _, _ => .open

def without (a b : Disposition) : Disposition :=
  match a, b with
  | .refuted, _ | _, .supported => .refuted
  | .supported, .refuted => .supported
  | _, _ => .open

theorem conjunction_sound (a b : Disposition) (p q : Prop)
    (ha : agreement a p) (hb : agreement b q) : agreement (all a b) (p ∧ q) := by
  cases a <;> cases b <;> simp_all [agreement, all]

theorem independent_alternatives_sound (a b : Disposition) (p q : Prop)
    (ha : agreement a p) (hb : agreement b q) : agreement (any a b) (p ∨ q) := by
  cases a <;> cases b <;> simp_all [agreement, any]

theorem positive_refutation_sound (a b : Disposition) (p q : Prop)
    (ha : agreement a p) (hb : agreement b q) : agreement (without a b) (p ∧ ¬ q) := by
  cases a <;> cases b <;> simp_all [agreement, without]

theorem open_is_not_negation_evidence : without .supported .open = .open := rfl
theorem supported_forbidden_refutes : without .supported .supported = .refuted := rfl
theorem positive_refutation_permits : without .supported .refuted = .supported := rfl

inductive Expr where
  | atom : Nat → Expr
  | conj : Expr → Expr → Expr
  | alternative : Expr → Expr → Expr
  | except : Expr → Expr → Expr
  | terminal : Expr → Expr → Expr
  deriving DecidableEq

def evaluate (assignments : Nat → Disposition) : Expr → Disposition
  | .atom n => assignments n
  | .conj a b => all (evaluate assignments a) (evaluate assignments b)
  | .alternative a b => any (evaluate assignments a) (evaluate assignments b)
  | .except a b => without (evaluate assignments a) (evaluate assignments b)
  | .terminal a b => all (evaluate assignments a) (evaluate assignments b)

def holds (valuation : Nat → Prop) : Expr → Prop
  | .atom n => valuation n
  | .conj a b => holds valuation a ∧ holds valuation b
  | .alternative a b => holds valuation a ∨ holds valuation b
  | .except a b => holds valuation a ∧ ¬ holds valuation b
  | .terminal a b => holds valuation a ∧ holds valuation b

theorem expression_sound (assignments : Nat → Disposition) (valuation : Nat → Prop)
    (ha : ∀ n, agreement (assignments n) (valuation n)) (e : Expr) :
    agreement (evaluate assignments e) (holds valuation e) := by
  induction e with
  | atom n => exact ha n
  | conj a b ihA ihB => exact conjunction_sound _ _ _ _ ihA ihB
  | alternative a b ihA ihB => exact independent_alternatives_sound _ _ _ _ ihA ihB
  | except a b ihA ihB => exact positive_refutation_sound _ _ _ _ ihA ihB
  | terminal a b ihA ihB => exact conjunction_sound _ _ _ _ ihA ihB

def terminated (assignments : Nat → Disposition) : Expr → Bool
  | .terminal a b => decide (evaluate assignments a = .supported ∧
      evaluate assignments b = .supported)
  | _ => false

theorem terminal_requires_both_conditions (assignments : Nat → Disposition) (a b : Expr) :
    terminated assignments (.terminal a b) = true ↔
      evaluate assignments a = .supported ∧ evaluate assignments b = .supported := by
  simp [terminated]

theorem terminal_success_implies_condition (assignments : Nat → Disposition)
    (valuation : Nat → Prop) (ha : ∀ n, agreement (assignments n) (valuation n))
    (a b : Expr) (ht : terminated assignments (.terminal a b) = true) :
    holds valuation a ∧ holds valuation b := by
  have h := (terminal_requires_both_conditions assignments a b).mp ht
  have hA := expression_sound assignments valuation ha a
  have hB := expression_sound assignments valuation ha b
  simpa [agreement, h.1, h.2] using And.intro hA hB

def admissible (root : Bool) : Expr → Bool
  | .atom _ => true
  | .conj a b | .alternative a b | .except a b => admissible false a && admissible false b
  | .terminal a b => root && admissible false a && admissible false b

theorem terminal_cannot_continue (a b c : Expr) :
    admissible true (.conj (.terminal a b) c) = false := by simp [admissible]

def atoms : Expr → List Nat
  | .atom n => [n]
  | .conj a b | .alternative a b | .except a b | .terminal a b => atoms a ++ atoms b

theorem alternatives_retain_both_dependencies (a b : Expr) (n : Nat) :
    n ∈ atoms (.alternative a b) ↔ n ∈ atoms a ∨ n ∈ atoms b := by simp [atoms]

inductive Op where | conj | alternative | except | terminal deriving DecidableEq
inductive Source where
  | atom : Nat → Source
  | reference : Nat → Source
  | binary : Op → Source → Source → Source
  deriving DecidableEq

def node (op : Op) (a b : Expr) : Expr :=
  match op with
  | .conj => .conj a b
  | .alternative => .alternative a b
  | .except => .except a b
  | .terminal => .terminal a b

def resolve : Nat → (Nat → Option Source) → List Nat → Bool → Source → Option Expr
  | 0, _, _, _, _ => none
  | fuel + 1, lookup, active, root, source =>
    match source with
    | .atom n => some (.atom n)
    | .reference n => if n ∈ active then none else
        (lookup n).bind (resolve fuel lookup (n :: active) root)
    | .binary op a b => if op = .terminal ∧ root = false then none else do
        let left ← resolve fuel lookup active false a
        let right ← resolve fuel lookup active false b
        pure (node op left right)

theorem exhausted_resolution_refuses (lookup : Nat → Option Source) (active : List Nat)
    (root : Bool) (source : Source) : resolve 0 lookup active root source = none := rfl

theorem active_reference_refuses (fuel : Nat) (lookup : Nat → Option Source)
    (active : List Nat) (root : Bool) (name : Nat) (h : name ∈ active) :
    resolve (fuel + 1) lookup active root (.reference name) = none := by simp [resolve, h]

theorem missing_reference_refuses (fuel : Nat) (lookup : Nat → Option Source)
    (active : List Nat) (root : Bool) (name : Nat) (h : lookup name = none) :
    resolve (fuel + 1) lookup active root (.reference name) = none := by simp [resolve, h]

theorem nested_terminal_refuses (fuel : Nat) (lookup : Nat → Option Source)
    (active : List Nat) (a b : Source) :
    resolve fuel lookup active false (.binary .terminal a b) = none := by
  cases fuel <;> simp [resolve]

theorem terminal_reference_in_continuation_refuses (fuel : Nat)
    (lookup : Nat → Option Source) (active : List Nat) (name : Nat) (a b : Source)
    (h : lookup name = some (.binary .terminal a b)) :
    resolve (fuel + 1) lookup active false (.reference name) = none := by
  simp [resolve, h, nested_terminal_refuses]

structure Identity where
  source : Nat
  graph : Nat
  model : Nat
  deriving DecidableEq

def checked (expected actual : Context) (expectedId actualId : Identity)
    (assignments : Nat → Disposition) (e : Expr) : Option Disposition :=
  if expected = actual ∧ expectedId = actualId then some (evaluate assignments e) else none

theorem foreign_context_refuses (expected actual : Context) (expectedId actualId : Identity)
    (assignments : Nat → Disposition) (e : Expr) (h : expected ≠ actual) :
    checked expected actual expectedId actualId assignments e = none := by simp [checked, h]

theorem wrong_identity_refuses (expected actual : Context) (expectedId actualId : Identity)
    (assignments : Nat → Disposition) (e : Expr) (h : expectedId ≠ actualId) :
    checked expected actual expectedId actualId assignments e = none := by simp [checked, h]

/-- A solution of a new proposed expression entails no unrelated legacy truth. -/
theorem proposed_solution_does_not_establish_legacy :
    ∃ valuation : Nat → Prop,
      holds valuation (.conj (.atom 0) (.atom 1)) ∧ ¬ valuation 2 := by
  refine ⟨fun n => n < 2, ?_⟩
  simp [holds]
end QCore.PremiseFlow
