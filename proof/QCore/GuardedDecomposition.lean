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

import QCore.PremiseFlow

/-! Explicit guarded obligations, separate from propositional flow-v1. All
agreement assumptions are scoped model inputs; no C/source truth is imported. -/
namespace QCore.GuardedDecomposition
open HypothesisGraph

inductive Status where
  | open | inactive | satisfied | refuted
  deriving DecidableEq

def implication (guard conclusion : Disposition) : Status :=
  match guard, conclusion with
  | .refuted, _ => .inactive
  | .open, _ => .open
  | .supported, .open => .open
  | .supported, .supported => .satisfied
  | .supported, .refuted => .refuted

def negate : Disposition → Disposition
  | .open => .open
  | .supported => .refuted
  | .refuted => .supported

def conjunction : List Disposition → Disposition
  | [] => .supported
  | d :: ds => PremiseFlow.all d (conjunction ds)

def coverage : List Disposition → Disposition
  | [] => .refuted
  | d :: ds => PremiseFlow.any d (coverage ds)

structure Guard where
  antecedents : List Nat
  operation : Nat
  forbidden : Bool
  deriving DecidableEq
structure Alternatives where
  operation : Nat
  branches : List Nat
  deriving DecidableEq
structure Terminal where
  branch : Nat
  postcondition : Nat
  deriving DecidableEq
structure Lowered where
  guards : List Guard
  alternatives : List Alternatives
  terminals : List Terminal

inductive Role where
  | environment | prerequisite | operation | condition | branch | postcondition
  deriving DecidableEq

def guardRoles (roles : Nat → Option Role) (g : Guard) : Bool :=
  decide (roles g.operation = some .operation) &&
  if g.forbidden then
    match g.antecedents with
    | [condition] => decide (roles condition = some .condition)
    | _ => false
  else
    match g.antecedents with
    | environment :: prereqs =>
      decide (roles environment = some .environment ∧
        1 ≤ prereqs.length ∧ prereqs.length ≤ 32 ∧ prereqs.Nodup) &&
        prereqs.all (fun p => decide (roles p = some .prerequisite))
    | [] => false

def enabledOperations (d : Lowered) : List Nat :=
  (d.guards.filter (fun g => !g.forbidden)).map Guard.operation

def branchDomain (d : Lowered) : List Nat :=
  d.alternatives.flatMap Alternatives.branches

def completeCases (d : Lowered) : Bool :=
  decide ((branchDomain d).Nodup ∧ (d.terminals.map Terminal.branch).Nodup) &&
  (branchDomain d).all (fun b => decide (b ∈ d.terminals.map Terminal.branch)) &&
  d.terminals.all (fun t => decide (t.branch ∈ branchDomain d))

/-- Source lexing/reference replay is separate. This is the finite typed domain
check corresponding to the lowering's guard/operation/case/terminal invariant. -/
def admissible (roles : Nat → Option Role) (d : Lowered) : Bool :=
  decide (d.guards.length + d.alternatives.length + d.terminals.length ≤ 256 ∧
    d.guards.Nodup ∧ (enabledOperations d).Nodup ∧ (d.alternatives.map Alternatives.operation).Nodup ∧
    enabledOperations d ≠ []) &&
  d.guards.all (fun g => guardRoles roles g &&
    decide (g.operation ∈ enabledOperations d)) &&
  (enabledOperations d).all (fun op => decide (op ∈ d.alternatives.map Alternatives.operation)) &&
  d.alternatives.all (fun a =>
    decide (a.operation ∈ enabledOperations d ∧ 2 ≤ a.branches.length ∧ a.branches.length ≤ 32) &&
    a.branches.all (fun b => decide (roles b = some .branch))) &&
  d.terminals.all (fun t => decide (roles t.branch = some .branch ∧
    roles t.postcondition = some .postcondition)) && completeCases d

theorem missing_terminal_refuses (d : Lowered) (b : Nat)
    (branch : b ∈ branchDomain d) (missing : b ∉ d.terminals.map Terminal.branch) :
    completeCases d = false := by
  have notall : ¬ (branchDomain d).all
      (fun c => decide (c ∈ d.terminals.map Terminal.branch)) = true := by
    intro all
    have member := List.all_eq_true.mp all b branch
    simp [missing] at member
  have hfalse : (branchDomain d).all
      (fun c => decide (c ∈ d.terminals.map Terminal.branch)) = false := by
    cases h : (branchDomain d).all
        (fun c => decide (c ∈ d.terminals.map Terminal.branch)) with
    | false => rfl
    | true => exact False.elim (notall h)
  unfold completeCases
  rw [hfalse]
  simp

def evalGuard (v : Nat → Disposition) (g : Guard) : Status :=
  if g.forbidden then
    implication (conjunction (g.antecedents.map v)) (negate (v g.operation))
  else implication (v g.operation) (conjunction (g.antecedents.map v))
def evalAlternative (v : Nat → Disposition) (a : Alternatives) : Status :=
  implication (v a.operation) (coverage (a.branches.map v))
def evalTerminal (v : Nat → Disposition) (t : Terminal) : Status :=
  implication (v t.branch) (v t.postcondition)

theorem satisfied_requires_positive_guard_and_conclusion (g c : Disposition) :
    implication g c = .satisfied ↔ g = .supported ∧ c = .supported := by
  cases g <;> cases c <;> simp [implication]


/-- Guard-left syntax lowers to necessary conditions for proposed operation
admission. It never infers execution from a true antecedent. -/
theorem enablement_requires_all_prerequisites (v : Nat → Disposition) (g : Guard)
    (ordinary : g.forbidden = false) :
    evalGuard v g = .satisfied ↔ v g.operation = .supported ∧
      conjunction (g.antecedents.map v) = .supported := by
  simp [evalGuard, ordinary, satisfied_requires_positive_guard_and_conclusion]

theorem guarded_satisfaction_sound (g c : Disposition) (p q : Prop)
    (hg : PremiseFlow.agreement g p) (hc : PremiseFlow.agreement c q)
    (hs : implication g c = .satisfied) : p ∧ q := by
  obtain ⟨rfl, rfl⟩ := (satisfied_requires_positive_guard_and_conclusion g c).mp hs
  exact ⟨hg, hc⟩

theorem forbidden_requires_positive_refutation (g op : Disposition) :
    implication g (negate op) = .satisfied ↔ g = .supported ∧ op = .refuted := by
  cases g <;> cases op <;> simp [implication, negate]

theorem absent_operation_does_not_discharge_forbidden (g : Disposition) :
    implication g (negate .open) ≠ .satisfied := by
  cases g <;> simp [implication, negate]

theorem open_guard_does_not_discharge (c : Disposition) :
    implication .open c = .open := rfl

theorem terminal_requires_separate_postcondition (v : Nat → Disposition) (t : Terminal) :
    evalTerminal v t = .satisfied ↔
      v t.branch = .supported ∧ v t.postcondition = .supported :=
  satisfied_requires_positive_guard_and_conclusion _ _

/-- Lowering retains all branch postconditions; selected-case coverage does not
remove the other terminal records or turn their missing assignments into truth. -/
def terminalChecks (d : Lowered) (v : Nat → Disposition) : List Status :=
  d.terminals.map (evalTerminal v)

theorem every_terminal_is_checked (d : Lowered) (v : Nat → Disposition) :
    (terminalChecks d v).length = d.terminals.length := by
  simp [terminalChecks]

theorem open_terminal_remains_open (v : Nat → Disposition) (t : Terminal)
    (hb : v t.branch = .supported) (hp : v t.postcondition = .open) :
    evalTerminal v t = .open := by simp [evalTerminal, hb, hp, implication]

/-- A recheck set grows along dependency edges, without assigning dispositions. -/
def propagate (edges : Nat → List Nat) (dirty : List Nat) : List Nat :=
  dirty ++ dirty.flatMap edges

theorem changed_symbol_is_retained (edges : Nat → List Nat) (dirty : List Nat)
    (s : Nat) (h : s ∈ dirty) : s ∈ propagate edges dirty := by
  simp only [propagate, List.mem_append]
  exact Or.inl h

theorem dependency_is_marked_for_recheck (edges : Nat → List Nat) (dirty : List Nat)
    (s target : Nat) (h : s ∈ dirty) (edge : target ∈ edges s) :
    target ∈ propagate edges dirty := by
  simp only [propagate, List.mem_append]
  exact Or.inr (List.mem_flatMap.mpr ⟨s, h, edge⟩)

/-- Original semantic objects are untouched by proposed obligation publication. -/
def erase (original : α) (_d : Lowered) : α := original

theorem proposed_lowering_preserves_original (original : α) (d : Lowered) :
    erase original d = original := rfl
end QCore.GuardedDecomposition
