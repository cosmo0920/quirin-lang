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

/-! Typed lifecycle metadata only. Neither constructor executes a resource
transition nor changes a proposition's evidence disposition. Source parsing and
Rust/model correspondence remain separate obligations. -/
namespace QCore.ContractLifecycle
open HypothesisGraph

inductive Operation where
  | create
  | retire
  deriving DecidableEq

inductive Target where
  | resource : Nat → Target
  | hypothesis : Nat → Target
  deriving DecidableEq

structure Entry where
  id : Nat
  operation : Operation
  target : Target
  deriving DecidableEq

def targetDeclared (atoms : List Nat) (target : Target) : Bool :=
  match target with
  | .resource _ => true
  | .hypothesis h => decide (h ∈ atoms)

def admissible (atoms : List Nat) (entries : List Entry) : Bool :=
  decide (entries.length ≤ 256 ∧ (entries.map Entry.id).Nodup) &&
    entries.all (fun entry => targetDeclared atoms entry.target)

/-- Erasure projects metadata away; it never performs either named action. -/
def erase (semantic : α) (_entries : List Entry) : α := semantic

/-- No resource or hypothesis fact is derived from either symbolic action. -/
def facts (_entry : Entry) : List Nat := []

def dispositionAfter (_entry : Entry) (before : Disposition) : Disposition := before

theorem erasure_preserves_semantics (semantic : α) (entries : List Entry) :
    erase semantic entries = semantic := rfl

theorem creation_derives_no_fact (id : Nat) (target : Target) :
    facts ⟨id, .create, target⟩ = [] := rfl

theorem retirement_derives_no_fact (id : Nat) (target : Target) :
    facts ⟨id, .retire, target⟩ = [] := rfl

theorem creation_does_not_support_open (id : Nat) (target : Target) :
    dispositionAfter ⟨id, .create, target⟩ .open = .open := rfl

theorem retirement_is_not_refutation (id : Nat) (target : Target) (before : Disposition) :
    dispositionAfter ⟨id, .retire, target⟩ before = before := rfl

theorem missing_hypothesis_refuses (atoms : List Nat) (h : Nat) (missing : h ∉ atoms) :
    targetDeclared atoms (.hypothesis h) = false := by simp [targetDeclared, missing]

theorem resource_target_is_symbolic (atoms : List Nat) (r : Nat) :
    targetDeclared atoms (.resource r) = true := rfl

theorem duplicate_entry_refuses (atoms : List Nat) (a b : Entry) (same : a.id = b.id) :
    admissible atoms [a, b] = false := by simp [admissible, same]

theorem oversized_declaration_refuses (atoms : List Nat) (entries : List Entry)
    (large : 256 < entries.length) : admissible atoms entries = false := by
  have h : ¬ entries.length ≤ 256 := Nat.not_le_of_lt large
  simp [admissible, h]

end QCore.ContractLifecycle
