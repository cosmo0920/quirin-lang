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
W48-BOTTOM-UP-BATCH3: observing a scalar from an external zero-argument
accessor, after applying its ordinary contract. Source/type extraction,
contract truth and Rust correspondence remain premises, not proved here.
The gate admits no caller safety conclusion. The scalar value is never used
to remove a successor. The call's errors and complete output state survive.
-/
namespace QCore.ScalarAccessorCondition

def eligible (external scalar empty none identity : Bool) : Bool :=
  external && scalar && empty && none && identity

def vectors : List Bool := [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true]

def evaluatedVectors : List Bool :=
  [false, true].flatMap fun a => [false, true].flatMap fun b =>
  [false, true].flatMap fun c => [false, true].flatMap fun d =>
  [false, true].map fun e => eligible a b c d e

theorem vectors_match : evaluatedVectors = vectors := by decide

theorem admission_needs_every_fact (a b c d e : Bool)
    (h : eligible a b c d e = true) :
    a = true ∧ b = true ∧ c = true ∧ d = true ∧ e = true := by
  simpa [eligible, Bool.and_eq_true, and_assoc] using h

theorem an_unread_pointee_refuses (a c d e : Bool) :
    eligible a false c d e = false := by simp [eligible]

theorem a_custody_result_refuses (a b c e : Bool) :
    eligible a b c false e = false := by simp [eligible]

/-- Full post-call state and diagnostics. The scalar observation cannot
    manufacture success from a refused ordinary call. -/
inductive Outcome (σ ε : Type) where
  | refused (state : σ) (errors : List ε)
  | checked (state : σ)

def observe {σ ε : Type} : Outcome σ ε → Except (σ × List ε) (List (Bool × σ))
  | .refused state errors => .error (state, errors)
  | .checked state => .ok [(true, state), (false, state)]

theorem call_errors_and_state_survive {σ ε : Type} (state : σ) (errors : List ε) :
    observe (Outcome.refused state errors) = .error (state, errors) := by rfl

/-- Every possible concrete scalar comparison outcome is represented. No
    relation between accessor value, literal and custody is assumed. -/
theorem every_concrete_comparison_is_covered {σ ε : Type}
    (state : σ) (comparison : Bool) :
    ∃ edges, observe (Outcome.checked state : Outcome σ ε) = .ok edges ∧
      (comparison, state) ∈ edges := by
  cases comparison <;> simp [observe]

theorem every_successor_keeps_the_complete_post_call_state {σ ε : Type}
    (state next : σ) (edge : Bool) (edges : List (Bool × σ))
    (h : observe (Outcome.checked state : Outcome σ ε) = .ok edges)
    (member : (edge, next) ∈ edges) : next = state := by
  simp [observe] at h
  subst edges
  simp only [List.mem_cons, List.not_mem_nil, or_false, Prod.mk.injEq] at member
  rcases member with h | h <;> exact h.2

end QCore.ScalarAccessorCondition
