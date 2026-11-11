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

/-! Proposed logger-effect custody and recursive obligation model. These are
conditional finite-model theorems, not C extraction or callback truth proofs. -/
namespace QCore.LoggerEffects
inductive Custody where
  | absent | owned | queued | dead
  deriving DecidableEq
abbrev State := Custody × Custody
inductive Event where
  | createOp | createText | forward | reply | transfer | take | callback
  | destroyText | destroyOp
  deriving DecidableEq
open Custody Event

def step : State → Event → Option State
  | (absent, absent), createOp => some (owned, absent)
  | (owned, absent), createText => some (owned, owned)
  | (owned, owned), forward => some (owned, owned)
  | (owned, owned), reply => some (owned, owned)
  | (owned, owned), transfer => some (queued, queued)
  | (queued, queued), take => some (owned, owned)
  | (absent, absent), callback => some (absent, absent)
  | (owned, owned), callback => some (owned, owned)
  | (owned, owned), destroyText => some (owned, dead)
  | (owned, dead), destroyOp => some (dead, dead)
  | _, _ => none

def run : State → List Event → Option State
  | s, [] => some s
  | s, e :: es => (step s e).bind (fun t => run t es)

def invariant (s : State) : Prop :=
  s = (absent, absent) ∨ s = (owned, absent) ∨ s = (owned, owned) ∨
  s = (queued, queued) ∨ s = (owned, dead) ∨ s = (dead, dead)

theorem step_preserves_paired_custody (s t : State) (e : Event)
    (h : step s e = some t) : invariant t := by
  rcases s with ⟨a,b⟩
  cases a <;> cases b <;> cases e <;> simp [step] at h <;>
    subst t <;> simp [invariant]

theorem run_preserves_paired_custody (s t : State) (es : List Event)
    (start : invariant s) (h : run s es = some t) : invariant t := by
  induction es generalizing s with
  | nil =>
    simp only [run, Option.some.injEq] at h
    exact h ▸ start
  | cons e es ih =>
    simp only [run] at h
    cases eq : step s e with
    | none => simp [eq] at h
    | some next =>
      simp only [eq, Option.bind_some] at h
      exact ih next (step_preserves_paired_custody s next e eq) h

theorem annihilation_requires_text_cleanup (s t : State)
    (h : step s destroyOp = some t) : s = (owned, dead) := by
  rcases s with ⟨a,b⟩
  cases a <;> cases b <;> simp [step] at h ⊢

theorem creation_requires_fresh_resources (s t : State)
    (h : step s createOp = some t) : s = (absent, absent) := by
  rcases s with ⟨a,b⟩
  cases a <;> cases b <;> simp [step] at h ⊢

theorem transfer_preserves_both_resources (s t : State)
    (h : step s transfer = some t) : t = (queued, queued) := by
  rcases s with ⟨a,b⟩
  cases a <;> cases b <;> simp [step] at h <;> exact h.symm

theorem queued_resource_cannot_be_freed_locally :
    step (queued, queued) destroyText = none ∧
    step (queued, queued) destroyOp = none := by decide

theorem dead_operation_cannot_be_recreated :
    step (dead, dead) createOp = none := by decide

theorem immediate_callback_has_no_owned_allocation :
    run (absent, absent) [callback] = some (absent, absent) := by decide

theorem forwarding_then_disabled_can_destroy :
    run (absent, absent) [createOp, createText, forward, destroyText, destroyOp] =
      some (dead, dead) := by decide

/-- The C forward branch ignores its recursive return value and returns one.
This observation is modeled separately from custody; a return bit is no proof. -/
def enqueueReturn (forwarded : Bool) (inner : Bool) : Bool :=
  if forwarded then true else inner

theorem forwarded_failure_still_returns_one :
    enqueueReturn true false = true := by decide

/-- Finite reachability permits cycles. `closed` is precisely the postcondition
checked by the Rust worklist's complete node-domain validation and traversal. -/
inductive Reachable (calls : Nat → Nat → Prop) (root : Nat) : Nat → Prop
  | root : Reachable calls root root
  | edge {a b} : Reachable calls root a → calls a b → Reachable calls root b

theorem cyclic_closure_retains_all_reachable_effects
    (calls : Nat → Nat → Prop) (localEffects : Nat → Nat → Prop)
    (root : Nat) (visited effects : Nat → Prop)
    (starts : visited root)
    (closed : ∀ a b, visited a → calls a b → visited b)
    (collected : ∀ a leaf, visited a → localEffects a leaf → effects leaf)
    (node leaf : Nat) (reachable : Reachable calls root node)
    (localEffect : localEffects node leaf) : effects leaf := by
  have member : visited node := by
    clear localEffect
    induction reachable with
    | root => exact starts
    | edge reach edge ih => exact closed _ _ ih edge
  exact collected node leaf member localEffect

/-- No decreasing rank or acyclicity is assumed by the reachability theorem. -/
theorem mutually_recursive_calls_reach_both (calls : Nat → Nat → Prop) (a b : Nat)
    (ab : calls a b) (ba : calls b a) :
    Reachable calls a b ∧ Reachable calls b a := by
  exact ⟨Reachable.edge Reachable.root ab, Reachable.edge Reachable.root ba⟩
end QCore.LoggerEffects
