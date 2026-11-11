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
W48-BOTTOM-UP-BATCH5. A complete typed scalar read retains its checked read
obligations and forgets the destination, instead of inventing opaque pointer
identity. C extraction and Rust/model correspondence remain premises 28.
-/
namespace QCore.ScalarReadIdentity

def eligible (same scalar noFrame noReadback : Bool) : Bool :=
  same && scalar && noFrame && noReadback

def vectors : List Bool := [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true]
def evaluatedVectors : List Bool :=
  [false, true].flatMap fun a => [false, true].flatMap fun b =>
  [false, true].flatMap fun c => [false, true].map fun d => eligible a b c d

theorem vectors_match : evaluatedVectors = vectors := by decide

theorem admission_needs_every_fact (a b c d : Bool)
    (h : eligible a b c d = true) :
    a = true ∧ b = true ∧ c = true ∧ d = true := by
  simpa [eligible, Bool.and_eq_true, and_assoc] using h

theorem pointer_destination_refuses (a c d : Bool) :
    eligible a false c d = false := by simp [eligible]

/-- Preserve the older lexical alias obligation in addition to every typed
    read obligation. Repeated consultation does not duplicate that obligation. -/
def retainAlias {α : Type} [DecidableEq α] (prior : Option α) (reads : List α) : List α :=
  match prior with
  | none => reads
  | some alias => if alias ∈ reads then reads else reads ++ [alias]

theorem original_alias_is_retained {α : Type} [DecidableEq α]
    (alias : α) (reads : List α) : alias ∈ retainAlias (some alias) reads := by
  simp only [retainAlias]
  split <;> simp_all

theorem every_typed_alias_is_retained {α : Type} [DecidableEq α]
    (prior : Option α) (reads : List α) (alias : α) (member : alias ∈ reads) :
    alias ∈ retainAlias prior reads := by
  cases prior with
  | none => exact member
  | some old =>
    simp only [retainAlias]
    split <;> simp_all

structure State (ι ρ ε : Type) where
  bindings : String → Option ι
  remainder : ρ
  diagnostics : List ε

def forget {ι ρ ε : Type} (destination : String) (s : State ι ρ ε) : State ι ρ ε :=
  { s with bindings := fun name => if name = destination then none else s.bindings name }

theorem scalar_result_has_no_resource_identity {ι ρ ε : Type}
    (destination : String) (s : State ι ρ ε) :
    (forget destination s).bindings destination = none := by simp [forget]

theorem every_other_binding_survives {ι ρ ε : Type}
    (destination name : String) (s : State ι ρ ε) (different : name ≠ destination) :
    (forget destination s).bindings name = s.bindings name := by simp [forget, different]

theorem checked_read_errors_survive {ι ρ ε : Type}
    (destination : String) (s : State ι ρ ε) :
    (forget destination s).diagnostics = s.diagnostics := by rfl

theorem all_other_state_survives {ι ρ ε : Type}
    (destination : String) (s : State ι ρ ε) :
    (forget destination s).remainder = s.remainder := by rfl

/-- No scalar value is synthesized. Each concrete comparison outcome keeps
    the complete checked-read state after scalar forgetting. -/
def successors {ι ρ ε : Type} (destination : String) (s : State ι ρ ε) :=
  [(true, forget destination s), (false, forget destination s)]

theorem neither_scalar_comparison_outcome_is_removed {ι ρ ε : Type}
    (destination : String) (s : State ι ρ ε) (comparison : Bool) :
    (comparison, forget destination s) ∈ successors destination s := by
  cases comparison <;> simp [successors]

end QCore.ScalarReadIdentity
