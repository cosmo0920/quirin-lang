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

import Std

namespace QCore.PremiseMetadataBridge

/-- The source-byte/AST correspondence and actual loader are external checked
inputs. This model erases only tagged, already validated metadata pieces. -/
inductive Metadata where
  | introduce (entry atom : String)
  | conjoin (entry premise : String) (children : List String)
  deriving DecidableEq
inductive Piece where
  | original (bytes : List Nat)
  | metadata (meaning : Metadata) (bytes : List Nat)
  deriving DecidableEq

def erase : List Piece → List Nat
  | [] => []
  | .original b :: rest => b ++ erase rest
  | .metadata _ _ :: rest => erase rest

def emit : List Piece → List Nat
  | [] => []
  | .original b :: rest => b ++ emit rest
  | .metadata _ b :: rest => b ++ emit rest

def decorate : List (List Nat × Option (Metadata × List Nat)) → List Piece
  | [] => []
  | (b, none) :: rest => .original b :: decorate rest
  | (b, some (m, added)) :: rest => .original b :: .metadata m added :: decorate rest

def originals (xs : List (List Nat × Option (Metadata × List Nat))) : List Nat :=
  (xs.map Prod.fst).flatten

theorem exact_original_bytes (xs : List (List Nat × Option (Metadata × List Nat))) :
    erase (decorate xs) = originals xs := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    rcases x with ⟨b, choice⟩
    cases choice with
    | none => simpa [decorate, erase, originals] using congrArg (b ++ ·) ih
    | some added => rcases added with ⟨m, bytes⟩; simpa [decorate, erase, originals] using congrArg (b ++ ·) ih

def valid (atoms : List String) (groups : List (String × List String)) : Metadata → Prop
  | .introduce entry atom => entry = "introduce_" ++ atom ∧ atom ∈ atoms
  | .conjoin entry name children => entry = "decompose_" ++ name ∧ (name, children) ∈ groups

theorem wrong_introduction_entry (atoms : List String) (groups : List (String × List String))
    (entry atom : String) (h : entry ≠ "introduce_" ++ atom) :
    ¬ valid atoms groups (.introduce entry atom) := by
  intro yes; exact h yes.1

theorem missing_atom_refuses (atoms : List String) (groups : List (String × List String))
    (entry atom : String) (h : atom ∉ atoms) :
    ¬ valid atoms groups (.introduce entry atom) := by
  intro yes; exact h yes.2

theorem changed_child_order_refuses (atoms : List String) (groups : List (String × List String))
    (entry name : String) (children : List String) (h : (name, children) ∉ groups) :
    ¬ valid atoms groups (.conjoin entry name children) := by
  intro yes; exact h yes.2

structure Checked (old new : List Nat) where
  pieces : List Piece
  emitted : emit pieces = new
  erased : erase pieces = old

theorem checked_roundtrip (old new : List Nat) (c : Checked old new) : erase c.pieces = old := c.erased

theorem stale_source_refuses (old new claimed : List Nat) (c : Checked old new)
    (wrong : claimed ≠ old) : erase c.pieces ≠ claimed := by
  rw [c.erased]; exact Ne.symm wrong

/-- Semantic projections retain the original graph, interpreted clauses and
binding text. The new syntax has a separate slot and supplies no fact. -/
structure Declaration where
  graph : List String
  interpreted : List String
  bindings : List String
  proposed : List Metadata

def semantics (d : Declaration) := (d.graph, d.interpreted, d.bindings)
def appendMetadata (d : Declaration) (m : List Metadata) : Declaration :=
  { d with proposed := d.proposed ++ m }

theorem graph_unchanged (d : Declaration) (m : List Metadata) :
    (appendMetadata d m).graph = d.graph := rfl

theorem interpreted_bindings_unchanged (d : Declaration) (m : List Metadata) :
    semantics (appendMetadata d m) = semantics d := rfl

theorem no_disposition_transport (d : Declaration) (m : List Metadata)
    (disposition : List String → Nat) :
    disposition (appendMetadata d m).graph = disposition d.graph := rfl

end QCore.PremiseMetadataBridge
