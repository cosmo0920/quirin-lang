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

namespace QCore.ControlFlow

/-! Formal boundary for the resource importer's structured and named
control-flow projection. The Rust importer constructs the concrete CFG; this
executable certificate prevents a successful import from omitting any declared
`for`, `switch`, `goto`, `break`, or `continue` edge. -/

abbrev NodeId := Nat

structure Edge where
  source : NodeId
  target : NodeId
  deriving DecidableEq, Repr

structure Import where
  nodes : List NodeId
  cfgEdges : List Edge
  forEdges : List Edge
  switchEdges : List Edge
  gotoEdges : List Edge
  breakEdges : List Edge
  continueEdges : List Edge
  unresolvedJump : Bool
  deriving DecidableEq, Repr

def Import.declaredEdges (input : Import) : List Edge :=
  input.forEdges ++ input.switchEdges ++ input.gotoEdges ++
    input.breakEdges ++ input.continueEdges

def Import.edgesReferenceKnownNodes (input : Import) : Bool :=
  input.cfgEdges.all fun edge =>
    input.nodes.contains edge.source && input.nodes.contains edge.target

def Import.declaredEdgesPresent (input : Import) : Bool :=
  input.declaredEdges.all fun edge => input.cfgEdges.contains edge

def Import.Valid (input : Import) : Prop :=
  input.unresolvedJump = false ∧
  input.edgesReferenceKnownNodes = true ∧
  input.declaredEdgesPresent = true

instance (input : Import) : Decidable input.Valid := by
  unfold Import.Valid
  infer_instance

structure Certificate where
  checked : Import
  deriving DecidableEq, Repr

def check (input : Import) : Option Certificate :=
  if input.Valid then some { checked := input } else none

private theorem check_valid
    {input : Import} {certificate : Certificate}
    (result : check input = some certificate) : input.Valid := by
  unfold check at result
  split at result
  · assumption
  · contradiction

/-- Every edge declared by a recognized control construct occurs in the
resulting CFG; recognized jumps therefore cannot be erased lexically. -/
theorem accepted_import_preserves_declared_control_edges
    {input : Import} {certificate : Certificate}
    (result : check input = some certificate) :
    input.declaredEdgesPresent = true :=
  (check_valid result).2.2

/-- An accepted import contains no unresolved label, break, or continue
target. The Rust side turns any such projection gap into `Unknown`. -/
theorem accepted_import_has_no_unresolved_jump
    {input : Import} {certificate : Certificate}
    (result : check input = some certificate) :
    input.unresolvedJump = false :=
  (check_valid result).1

/-- All emitted control-flow edges reference nodes retained in the CFG. -/
theorem accepted_import_edges_reference_known_nodes
    {input : Import} {certificate : Certificate}
    (result : check input = some certificate) :
    input.edgesReferenceKnownNodes = true :=
  (check_valid result).2.1

end QCore.ControlFlow
