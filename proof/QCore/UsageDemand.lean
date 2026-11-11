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

import QCore.Usage

namespace QCore

instance (a b : UsageUpper) : Decidable (UsageUpper.Le a b) := by
  cases a <;> cases b <;> simp only [UsageUpper.Le] <;> infer_instance

instance (u : Usage) : Decidable u.Valid := by
  rcases u with ⟨lower, upper⟩
  cases upper <;> simp only [Usage.Valid, UsageUpper.covers] <;> infer_instance

instance (a b : Usage) : Decidable (a.Refines b) :=
  inferInstanceAs (Decidable (b.lower ≤ a.lower ∧ UsageUpper.Le a.upper b.upper))

instance (a b : Usage) : Decidable (a.Accepts b) :=
  inferInstanceAs (Decidable (b.Valid ∧ b.Refines a))

namespace Usage

/-- Membership of a concrete natural demand in an interval. -/
def Covers (u : Usage) (n : Nat) : Prop := u.lower ≤ n ∧ u.upper.covers n

theorem covers_of_refines {actual declared : Usage} {n : Nat}
    (refines : actual.Refines declared) (covered : actual.Covers n) : declared.Covers n := by
  refine ⟨Nat.le_trans refines.1 covered.1, ?_⟩
  cases ha : actual.upper <;> cases hd : declared.upper
  · exact Nat.le_trans (ha ▸ covered.2) (by simpa [Refines, ha, hd, UsageUpper.Le] using refines.2)
  · trivial
  · have impossible := refines.2
    simp [UsageUpper.Le, ha, hd] at impossible
  · trivial

theorem add_covers {a b : Usage} {m n : Nat}
    (ha : a.Covers m) (hb : b.Covers n) : (a.add b).Covers (m + n) :=
  ⟨Nat.add_le_add ha.1 hb.1, UsageUpper.add_covers ha.2 hb.2⟩

theorem mul_covers {a b : Usage} {m n : Nat}
    (ha : a.Covers m) (hb : b.Covers n) : (a.mul b).Covers (m * n) :=
  ⟨Nat.mul_le_mul ha.1 hb.1, UsageUpper.mul_covers ha.2 hb.2⟩

end Usage

namespace UsageDemand

/-! Executable demand analysis for the reduced Rust UsageExpr core. Sequence
and application vectors are represented by binary composition. Lambda demand
reserves the free-variable resources needed for one invocation, not a runtime
evaluation of the body. Application grades are supplied contracts, as in Rust.
This is not an ordinary/dependent type checker or a beta-preservation proof.
Lean uses unbounded naturals; Rust rejects finite arithmetic overflow. -/

inductive Expr where
  | unit
  | variable (name : Nat)
  | sequence (left right : Expr)
  | branch (condition left right : Expr)
  | apply (parameter : Usage) (function argument : Expr)
  | lambda (name : Nat) (parameter : Usage) (body : Expr)

def erase (name : Nat) (context : UsageContext) : UsageContext :=
  fun x => if x = name then Usage.zero else context x

def synthesize : Expr → Option UsageContext
  | .unit => some (fun _ => Usage.zero)
  | .variable name => some (fun x => if x = name then Usage.one else Usage.zero)
  | .sequence left right => do
      let a ← synthesize left
      let b ← synthesize right
      return UsageContext.add a b
  | .branch condition left right => do
      let c ← synthesize condition
      let a ← synthesize left
      let b ← synthesize right
      return UsageContext.add c (fun x => Usage.join (a x) (b x))
  | .apply parameter function argument => do
      let f ← synthesize function
      let a ← synthesize argument
      return UsageContext.application f a parameter
  | .lambda name parameter body => do
      let b ← synthesize body
      if parameter.Accepts (b name) then some (erase name b) else none

/-- Crucially, scaling occurs after successful binder checking. -/
def synthesizeAt (quantity : Usage) (expression : Expr) : Option UsageContext := do
  let context ← synthesize expression
  return UsageContext.scale quantity context

abbrev Counts := Nat → Nat

/-- Demand realizations: choose one branch and an admitted argument demand.
Lambda counts describe resources reserved for an invocation, with its binder
removed. They are not operational lambda-calculus evaluation traces. -/
inductive Realizes : Expr → Counts → Prop where
  | unit : Realizes .unit (fun _ => 0)
  | variable (name) : Realizes (.variable name) (fun x => if x = name then 1 else 0)
  | sequence {a b ca cb} : Realizes a ca → Realizes b cb →
      Realizes (.sequence a b) (fun x => ca x + cb x)
  | left {c a b cc ca} : Realizes c cc → Realizes a ca →
      Realizes (.branch c a b) (fun x => cc x + ca x)
  | right {c a b cc cb} : Realizes c cc → Realizes b cb →
      Realizes (.branch c a b) (fun x => cc x + cb x)
  | apply {p f a cf ca n} : Realizes f cf → Realizes a ca → p.Covers n →
      Realizes (.apply p f a) (fun x => cf x + n * ca x)
  | lambda {name p body counts} : Realizes body counts →
      Realizes (.lambda name p body) (fun x => if x = name then 0 else counts x)

/-- The executable analysis encloses every admitted demand realization. -/
theorem synthesize_covers {e : Expr} {counts : Counts} (run : Realizes e counts)
    {context : UsageContext} (accepted : synthesize e = some context) :
    ∀ x, (context x).Covers (counts x) := by
  induction run generalizing context with
  | unit =>
      cases accepted
      intro x
      exact ⟨Nat.le_refl 0, Nat.le_refl 0⟩
  | «variable» name =>
      cases accepted
      intro x
      by_cases h : x = name <;> simp [h, Usage.Covers, Usage.one, Usage.zero, UsageUpper.covers]
  | @sequence a b ca cb ra rb iha ihb =>
      simp only [synthesize] at accepted
      cases ha : synthesize a <;> simp [ha] at accepted
      cases hb : synthesize b <;> simp [hb] at accepted
      cases accepted
      exact fun x => Usage.add_covers (iha ha x) (ihb hb x)
  | @left c a b cc ca rc ra ihc iha =>
      simp only [synthesize] at accepted
      cases hc : synthesize c <;> simp [hc] at accepted
      cases ha : synthesize a <;> simp [ha] at accepted
      cases hb : synthesize b <;> simp [hb] at accepted
      cases accepted
      exact fun x => Usage.add_covers (ihc hc x)
        (Usage.covers_of_refines (Usage.left_refines_join _ _) (iha ha x))
  | @right c a b cc cb rc rb ihc ihb =>
      simp only [synthesize] at accepted
      cases hc : synthesize c <;> simp [hc] at accepted
      cases ha : synthesize a <;> simp [ha] at accepted
      cases hb : synthesize b <;> simp [hb] at accepted
      cases accepted
      exact fun x => Usage.add_covers (ihc hc x)
        (Usage.covers_of_refines (Usage.right_refines_join _ _) (ihb hb x))
  | @apply p f a cf ca n rf ra hp ihf iha =>
      simp only [synthesize] at accepted
      cases hf : synthesize f <;> simp [hf] at accepted
      cases ha : synthesize a <;> simp [ha] at accepted
      cases accepted
      exact fun x => Usage.add_covers (ihf hf x) (Usage.mul_covers hp (iha ha x))
  | @lambda name p body counts rb ih =>
      simp only [synthesize] at accepted
      cases hb : synthesize body <;> simp [hb] at accepted
      rcases accepted with ⟨_, rfl⟩
      intro x
      simp only [erase]
      split
      · exact ⟨Nat.le_refl 0, Nat.le_refl 0⟩
      · exact ih hb x

/-- A lambda accepted at any quantity obeys its parameter contract per
invocation, including when the surrounding quantity is zero or unrestricted. -/
theorem lambda_parameter_covers {quantity parameter : Usage} {name : Nat}
    {body : Expr} {context : UsageContext} {counts : Counts}
    (accepted : synthesizeAt quantity (.lambda name parameter body) = some context)
    (run : Realizes body counts) : parameter.Covers (counts name) := by
  simp only [synthesizeAt, synthesize] at accepted
  cases hb : synthesize body <;> simp [hb] at accepted
  split at accepted
  next h => exact Usage.covers_of_refines h.2 (synthesize_covers run hb name)
  next => contradiction

theorem scaled_captures_cover {quantity : Usage} {e : Expr} {context : UsageContext}
    {counts : Counts} {copies : Nat}
    (accepted : synthesizeAt quantity e = some context)
    (run : Realizes e counts) (admitted : quantity.Covers copies) :
    ∀ x, (context x).Covers (copies * counts x) := by
  simp only [synthesizeAt] at accepted
  cases he : synthesize e <;> simp [he] at accepted
  cases accepted
  exact fun x => Usage.mul_covers admitted (synthesize_covers run he x)

/-- Source parameter contracts must be read from the source, not supplied by
an independent certificate. Parsing and Rust/Lean correspondence are outside
this reduced model. -/
def checkParameters (parameters : List (Nat × Usage)) (context : UsageContext) : Bool :=
  parameters.all (fun (name, grade) => decide (grade.Accepts (context name)))

theorem checked_parameters_cover {e : Expr} {context : UsageContext}
    {parameters : List (Nat × Usage)} {name : Nat} {grade : Usage} {counts : Counts}
    (accepted : synthesize e = some context)
    (checked : checkParameters parameters context = true)
    (member : (name, grade) ∈ parameters) (run : Realizes e counts) :
    grade.Covers (counts name) := by
  have all := List.all_eq_true.mp checked (name, grade) member
  have contract : grade.Accepts (context name) := of_decide_eq_true all
  exact Usage.covers_of_refines contract.2 (synthesize_covers run accepted name)

theorem unused_one_parameter_rejected (name : Nat) :
    checkParameters [(name, Usage.one)] (fun _ => Usage.zero) = false := by
  simp [checkParameters, Usage.Accepts, Usage.Refines, Usage.one, Usage.zero]

def twice : Expr := .lambda 0 Usage.affine
  (.lambda 1 Usage.many
    (.apply Usage.many (.variable 0) (.apply Usage.many (.variable 0) (.variable 1))))

theorem duplicated_affine_parameter_rejected (quantity : Usage) :
    synthesizeAt quantity twice = none := by
  simp [synthesizeAt, twice, synthesize, Usage.Accepts, Usage.Refines, Usage.Valid,
    UsageUpper.covers, UsageUpper.Le, erase, UsageContext.application, UsageContext.add,
    UsageContext.scale, Usage.add, Usage.mul, UsageUpper.add, UsageUpper.mul,
    Usage.affine, Usage.one, Usage.zero, Usage.many]

end UsageDemand
end QCore
