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

import QCore.IndirectChainAccess

/-!
Clang-typed statement paths (W46-ACCESS-PATH-STATEMENTS). The AST reader is a
trusted extraction premise; this module models its fail-closed shape decision
and the unchanged W43/W44 flow transfer. A path obligation names each pointer
read, including the root even for a single selection. Untracked objects are
outside the resource property, as in the prior chain model.
-/

namespace QCore.TypedAccessPathStatements

open QCore.IndirectScalarAccess

structure Shape where
  exactType : Bool
  knownFieldOwner : Bool
  -- Normalized extraction fact: a proven builtin scalar for scalar/no-access
  -- mode, or a positively typed pointer in whole borrowed-read mode. A
  -- typedef still naming an anonymous aggregate proves neither alternative.
  knownValueType : Bool
  -- True only for a whole pointer-valued read with a proven pointer type.
  borrowedPointerValue : Bool
  unionField : Bool
  bitfield : Bool
  incompleteRecord : Bool
  castInPath : Bool
  pointerArithmetic : Bool
  volatileAtomic : Bool
  pointerStore : Bool
  call : Bool
  otherForm : Bool
  mixedPath : Bool
  untrackedRoot : Bool

/-- Each admitted fact is positive, and each unlisted form is a refusal. -/
def admits (p : Shape) : Bool :=
  p.exactType && p.knownFieldOwner && p.knownValueType && !p.unionField && !p.bitfield &&
  !p.incompleteRecord && !p.castInPath && !p.pointerArithmetic &&
  !p.volatileAtomic && !p.pointerStore && !p.call && !p.otherForm &&
  (!p.mixedPath || p.untrackedRoot)

structure Statement where
  shape : Shape
  scalar : Access
  hops : List IndirectChainAccess.Obligation

/-- `none` is a retained Unknown, never an optimistic fallback. -/
def lower (s : IndirectChainAccess.State) (p : Statement) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  if admits p.shape then some (IndirectChainAccess.transfer s p.scalar p.hops)
  else none

theorem missing_type_refuses (p : Shape) (h : p.exactType = false) : admits p = false := by
  simp [admits, h]

theorem missing_field_owner_refuses (p : Shape) (h : p.knownFieldOwner = false) :
    admits p = false := by
  simp [admits, h]

theorem unproven_value_type_refuses (p : Shape) (h : p.knownValueType = false) :
    admits p = false := by
  simp [admits, h]

theorem pointer_store_refuses (p : Shape) (h : p.pointerStore = true) : admits p = false := by
  simp [admits, h]

theorem union_refuses (p : Shape) (h : p.unionField = true) : admits p = false := by
  simp [admits, h]

theorem cast_refuses (p : Shape) (h : p.castInPath = true) : admits p = false := by
  simp [admits, h]

theorem call_refuses (p : Shape) (h : p.call = true) : admits p = false := by
  simp [admits, h]

theorem any_unlisted_form_refuses (p : Shape) (h : p.otherForm = true) : admits p = false := by
  simp [admits, h]

theorem mixed_path_with_tracked_root_refuses (p : Shape)
    (hm : p.mixedPath = true) (hr : p.untrackedRoot = false) : admits p = false := by
  simp [admits, hm, hr]

theorem refused_shape_has_no_lowering (s : IndirectChainAccess.State) (p : Statement)
    (h : admits p.shape = false) : lower s p = none := by
  simp [lower, h]

/-- All base and loaded-pointer obligations have been checked at this state. -/
def everyHop (s : IndirectChainAccess.State) (p : Statement) : Prop :=
  (∀ base ∈ p.scalar.bases, IndirectScalarAccess.resolves s.base base = true) ∧
  (∀ hop ∈ p.hops, IndirectChainAccess.resolves s hop = true)

/-- Per-hop discharge suffices for this abstract access to add no diagnostic. -/
theorem every_hop_discharge_adds_no_diagnostic (s : IndirectChainAccess.State)
    (p : Statement) (ha : admits p.shape = true) (hh : everyHop s p) :
    ∃ after, lower s p = some (after, []) := by
  rcases hh with ⟨hb, hc⟩
  have hbase := IndirectScalarAccess.an_admitted_statement_adds_no_diagnostic s.base p.scalar hb
  have hchain := IndirectChainAccess.admitted_chains_add_no_diagnostic s p.scalar p.hops hc
  exact ⟨(IndirectChainAccess.transfer s p.scalar p.hops).1,
    by
      have hempty : (IndirectChainAccess.transfer s p.scalar p.hops).2 = [] := by
        rw [hchain, hbase]
      cases htrans : IndirectChainAccess.transfer s p.scalar p.hops with
      | mk after diagnostics =>
        simp only [htrans] at hempty
        subst diagnostics
        simp [lower, ha, htrans]⟩

/-- A non-pointer store has no frame; all custody and field-link state stays. -/
theorem nonpointer_store_preserves_custody (s : IndirectChainAccess.State)
    (p : Statement) (ha : admits p.shape = true) (hf : p.scalar.frame = none) :
    ∃ after diagnostics, lower s p = some (after, diagnostics) ∧
      after.base.rest = s.base.rest ∧ after.fieldLink = s.fieldLink ∧
      (∀ name, p.scalar.destination ≠ some name → after.base.binding name = s.base.binding name) := by
  refine ⟨(IndirectChainAccess.transfer s p.scalar p.hops).1,
    (IndirectChainAccess.transfer s p.scalar p.hops).2, ?_, ?_, ?_, ?_⟩
  · simp [lower, ha]
  · rfl
  · exact IndirectChainAccess.field_links_are_unchanged s p.scalar p.hops
  · intro name hd
    rw [IndirectChainAccess.a_chain_changes_no_binding]
    exact IndirectScalarAccess.without_a_frame_only_the_destination_changes s.base p.scalar name hf hd

/-- The pointer-valued statement read is a borrowed alias. It carries no
tracked target, so a later custody use must prove its own obligation. -/
def borrowedAlias : Binding :=
  { targets := [], unknown := true, mayBeNull := true, derivation := .identity }

def bindBorrowed (s : IndirectChainAccess.State) (destination : Nat) :
    IndirectChainAccess.State :=
  { s with base := { s.base with binding := fun name =>
      if name = destination then some borrowedAlias else s.base.binding name } }

def lowerBorrowed (s : IndirectChainAccess.State) (p : Statement) (destination : Nat) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  if p.shape.borrowedPointerValue && p.scalar.frame.isNone then
    (lower s p).map fun (after, diagnostics) => (bindBorrowed after destination, diagnostics)
  else none

theorem pointer_read_binds_no_custody (s : IndirectChainAccess.State)
    (p : Statement) (destination : Nat) (ha : admits p.shape = true)
    (hp : p.shape.borrowedPointerValue = true)
    (hf : p.scalar.frame = none) :
    ∃ after diagnostics, lowerBorrowed s p destination = some (after, diagnostics) ∧
      (after.base.binding destination).map Binding.targets = some [] ∧
      after.base.rest = s.base.rest ∧ after.fieldLink = s.fieldLink := by
  refine ⟨bindBorrowed (IndirectChainAccess.transfer s p.scalar p.hops).1 destination,
    (IndirectChainAccess.transfer s p.scalar p.hops).2, ?_, ?_, ?_, ?_⟩
  · simp [lowerBorrowed, lower, ha, hp, hf]
  · simp [bindBorrowed, borrowedAlias]
  · rfl
  · rfl

theorem refused_pointer_read_stays_unknown (s : IndirectChainAccess.State)
    (p : Statement) (destination : Nat) (h : admits p.shape = false) :
    lowerBorrowed s p destination = none := by
  simp [lowerBorrowed, lower, h]

/-- The Rust reader sets `borrowed_destination` only for a whole pointer
read. A pointer store is refused, so no reachable borrowed lowering carries
a write frame. Refuse this impossible combination in the model too. -/
theorem borrowed_read_with_frame_refuses (s : IndirectChainAccess.State)
    (p : Statement) (destination : Nat) (frame : Nat → Bool)
    (hf : p.scalar.frame = some frame) :
    lowerBorrowed s p destination = none := by
  simp [lowerBorrowed, hf]

/-- A declaration of several aliases is admitted as a no-access statement
only when the gateway's static custody closure proves every source and
destination untracked. It neither reads memory nor transfers custody. -/
def lowerUntrackedAliases (s : IndirectChainAccess.State)
    (everySourceUntracked everyDestinationUntracked : Bool) :
    Option (IndirectChainAccess.State × List Diagnostic) :=
  if everySourceUntracked && everyDestinationUntracked then some (s, []) else none

theorem untracked_alias_declarations_preserve_custody (s : IndirectChainAccess.State) :
    lowerUntrackedAliases s true true = some (s, []) := by
  rfl

theorem tracked_alias_declaration_refuses (s : IndirectChainAccess.State)
    (sourceUntracked destinationUntracked : Bool)
    (h : sourceUntracked = false ∨ destinationUntracked = false) :
    lowerUntrackedAliases s sourceUntracked destinationUntracked = none := by
  rcases h with hs | hd
  · simp [lowerUntrackedAliases, hs]
  · simp [lowerUntrackedAliases, hd]

end QCore.TypedAccessPathStatements
