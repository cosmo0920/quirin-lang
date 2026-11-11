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

import QCore.ModuleImports
import QCore.SourceExtensions

/-! Runtime source-bundle admission. Nodes carry exact retained source text and
file-local standard capabilities. The resolver's completed-node order supplies
a strictly decreasing rank on import edges. This finite admission model does
not establish filesystem identity, parser extraction or Rust correspondence. -/
namespace QCore.RuntimeImports

structure Node where
  source : String
  mode : SourceExtensions.Mode
  explicitMode : Bool
  imports : ModuleImports.Imports
  deriving DecidableEq

structure Edge where
  importer : Nat
  target : Nat
  deriving DecidableEq

structure Bundle where
  nodes : List Node
  roots : List Nat
  edges : List Edge
  /-- Position in dependency-before-importer completion order. -/
  rank : Nat → Nat

def dependencyAllowed (node : Node) : Bool :=
  match node.mode with
  | .contract => false
  | .executable => !node.explicitMode
  | .library => true

def edgeAllowed (bundle : Bundle) (edge : Edge) : Bool :=
  match bundle.nodes[edge.importer]?, bundle.nodes[edge.target]? with
  | some _, some target => dependencyAllowed target && decide (bundle.rank edge.target < bundle.rank edge.importer)
  | _, _ => false

/-- The all-edges check includes every directive, including those in unused
functions' files; no unresolved edge can be omitted from the inventory. The
inventory/extraction correspondence is a stated boundary. -/
def graphAllowed (bundle : Bundle) : Bool :=
  !bundle.roots.isEmpty &&
  bundle.roots.all (fun root => (bundle.nodes[root]?).isSome) &&
  bundle.edges.all (edgeAllowed bundle)

/-- The standalone frontend has no source loader. -/
def standaloneAllowed (runtimeImports : List String) : Bool := runtimeImports.isEmpty

/-- Snapshot lookup wins even when disk disagrees. -/
def selectSnapshot (supplied disk : Option String) : Option String :=
  match supplied with
  | some text => some text
  | none => disk

/-- The checking projection may differ; provenance always keeps the original. -/
def retainedSource (original _staged : String) : String := original

/-- Each file's permission is checked against only that file's imports. -/
def fileAllowsIo (node : Node) (usesIo : Bool) : Bool :=
  !usesIo || node.imports.io

theorem snapshot_overrides_disk (text : String) (disk : Option String) :
    selectSnapshot (some text) disk = some text := by rfl

theorem absent_snapshot_and_disk_refuses : selectSnapshot none none = none := by rfl

theorem staging_preserves_retained_source (original staged : String) :
    retainedSource original staged = original := by rfl

theorem nonempty_runtime_imports_refuse_standalone (path : String) (rest : List String) :
    standaloneAllowed (path :: rest) = false := by rfl

theorem contract_dependency_refuses (source : String) (explicitMode : Bool)
    (imports : ModuleImports.Imports) :
    dependencyAllowed ⟨source, .contract, explicitMode, imports⟩ = false := by rfl

theorem explicit_executable_dependency_refuses (source : String)
    (imports : ModuleImports.Imports) :
    dependencyAllowed ⟨source, .executable, true, imports⟩ = false := by rfl

theorem admitted_edge_has_decreasing_rank (bundle : Bundle) (edge : Edge)
    (h : edgeAllowed bundle edge = true) :
    bundle.rank edge.target < bundle.rank edge.importer := by
  unfold edgeAllowed at h
  split at h <;> simp_all

theorem admitted_graph_checks_every_edge (bundle : Bundle)
    (h : graphAllowed bundle = true) (edge : Edge) (member : edge ∈ bundle.edges) :
    edgeAllowed bundle edge = true := by
  simp only [graphAllowed, Bool.and_eq_true, List.all_eq_true] at h
  exact h.2 edge member

inductive Path (bundle : Bundle) : Nat → Nat → Prop where
  | edge (edge : Edge) (member : edge ∈ bundle.edges) : Path bundle edge.importer edge.target
  | step {first middle last : Nat} : Path bundle first middle → Path bundle middle last → Path bundle first last

theorem admitted_path_decreases_rank (bundle : Bundle) (h : graphAllowed bundle = true)
    {first last : Nat} (path : Path bundle first last) :
    bundle.rank last < bundle.rank first := by
  induction path with
  | edge edge member =>
    exact admitted_edge_has_decreasing_rank bundle edge
      (admitted_graph_checks_every_edge bundle h edge member)
  | step _ _ left right => exact Nat.lt_trans right left

theorem admitted_graph_has_no_cycle (bundle : Bundle) (h : graphAllowed bundle = true)
    (node : Nat) : ¬ Path bundle node node := by
  intro cycle
  exact Nat.lt_irrefl _ (admitted_path_decreases_rank bundle h cycle)

theorem admitted_io_requires_own_import (node : Node)
    (h : fileAllowsIo node true = true) : node.imports.io = true := by
  simpa [fileAllowsIo] using h

/-- Changing a neighbour's imports cannot change the caller's permission. -/
def pairAllowsIo (caller _neighbour : Node) (usesIo : Bool) : Bool := fileAllowsIo caller usesIo

theorem neighbour_cannot_grant_io (caller neighbour : Node)
    (missing : caller.imports.io = false) : pairAllowsIo caller neighbour true = false := by
  simp [pairAllowsIo, fileAllowsIo, missing]
/-- Covers each independently selected bundled capability, not only IO. -/
def fileAllows (permission : ModuleImports.Imports → Bool) (node : Node)
    (used : Bool) : Bool := !used || permission node.imports

theorem admitted_capability_requires_own_import (permission : ModuleImports.Imports → Bool)
    (node : Node) (h : fileAllows permission node true = true) :
    permission node.imports = true := by
  simpa [fileAllows] using h

theorem absent_target_refuses (bundle : Bundle) (edge : Edge)
    (missing : bundle.nodes[edge.target]? = none) : edgeAllowed bundle edge = false := by
  simp [edgeAllowed, missing]

end QCore.RuntimeImports
