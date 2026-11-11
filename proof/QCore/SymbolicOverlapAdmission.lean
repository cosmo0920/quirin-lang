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

import QCore.PluginNeutralOverlap
import QCore.HypothesisGraph

/-! Conditional model of the source-only cross-view guard. Both property
loaders and original proposition/erasure matching are independent prerequisites.
Maps retain every typed entry; no Context or evidence disposition is transported.
This is not a Rust extraction theorem or an executable lifecycle judgment. -/
namespace QCore.SymbolicOverlapAdmission

def admitted (original boundsLoaded resourceLoaded graphMatched erasureNeutral : Prop)
    (nullFlows resourceFlows : Nat → Option Flow)
    (nullLifecycle resourceLifecycle : Nat → Option Lifecycle) : Prop :=
  original ∧ boundsLoaded ∧ resourceLoaded ∧ graphMatched ∧ erasureNeutral ∧
    nullFlows = resourceFlows ∧ nullLifecycle = resourceLifecycle

theorem valid_matching_metadata_preserves_original_guard
    (original boundsLoaded resourceLoaded graphMatched erasureNeutral : Prop)
    (flow : Nat → Option Flow) (lifecycle : Nat → Option Lifecycle)
    (boundsValid : boundsLoaded) (resourceValid : resourceLoaded)
    (graphValid : graphMatched) (erasureValid : erasureNeutral) :
    admitted original boundsLoaded resourceLoaded graphMatched erasureNeutral
      flow flow lifecycle lifecycle ↔ original := by
  simp [admitted, boundsValid, resourceValid, graphValid, erasureValid]

theorem failed_bounds_loader_refuses (original resourceLoaded graphMatched erasureNeutral : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle) :
    ¬ admitted original False resourceLoaded graphMatched erasureNeutral nf rf nl rl := by
  simp [admitted]

theorem failed_resource_loader_refuses (original boundsLoaded graphMatched erasureNeutral : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle) :
    ¬ admitted original boundsLoaded False graphMatched erasureNeutral nf rf nl rl := by
  simp [admitted]

theorem failed_original_graph_match_refuses (original boundsLoaded resourceLoaded erasureNeutral : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle) :
    ¬ admitted original boundsLoaded resourceLoaded False erasureNeutral nf rf nl rl := by
  simp [admitted]

theorem failed_erasure_refuses (original boundsLoaded resourceLoaded graphMatched : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle) :
    ¬ admitted original boundsLoaded resourceLoaded graphMatched False nf rf nl rl := by
  simp [admitted]

theorem changed_flow_entry_refuses
    (original boundsLoaded resourceLoaded graphMatched erasureNeutral : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle)
    (key : Nat) (different : nf key ≠ rf key) :
    ¬ admitted original boundsLoaded resourceLoaded graphMatched erasureNeutral nf rf nl rl := by
  intro accepted
  exact different (congrFun accepted.2.2.2.2.2.1 key)

theorem changed_lifecycle_entry_refuses
    (original boundsLoaded resourceLoaded graphMatched erasureNeutral : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle)
    (key : Nat) (different : nl key ≠ rl key) :
    ¬ admitted original boundsLoaded resourceLoaded graphMatched erasureNeutral nf rf nl rl := by
  intro accepted
  exact different (congrFun accepted.2.2.2.2.2.2 key)

theorem one_sided_flow_refuses
    (original boundsLoaded resourceLoaded graphMatched erasureNeutral : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle)
    (key : Nat) (entry : Flow) (absent : nf key = none) (present : rf key = some entry) :
    ¬ admitted original boundsLoaded resourceLoaded graphMatched erasureNeutral nf rf nl rl := by
  apply changed_flow_entry_refuses (key := key)
  simp [absent, present]

/-- A nonempty, exactly matched metadata map with every validation assumption
holds alongside an Open old proposition. Metadata cannot entail support. -/
theorem metadata_does_not_prove_legacy_support :
    ¬ (∀ (legacy : HypothesisGraph.Disposition),
      admitted True True True True True
        (fun _ => some (0 : Nat)) (fun _ => some (0 : Nat))
        (fun _ => some (1 : Nat)) (fun _ => some (1 : Nat)) →
      legacy = .supported) := by
  intro claimed
  have valid : admitted True True True True True
      (fun _ => some (0 : Nat)) (fun _ => some (0 : Nat))
      (fun _ => some (1 : Nat)) (fun _ => some (1 : Nat)) := by
    simp [admitted]
  have wrong := claimed .open valid
  cases wrong

/-- Independent pure-null admission validates both property views while requiring
an empty null graph and empty null metadata. Resource payloads are not copied. -/
def independentPureNull (original boundsLoaded resourceLoaded graphEmpty metadataEmpty nullOnly : Prop) : Prop :=
  original ∧ boundsLoaded ∧ resourceLoaded ∧ graphEmpty ∧ metadataEmpty ∧ nullOnly

theorem pure_null_preserves_original_guard
    (original boundsLoaded resourceLoaded graphEmpty metadataEmpty nullOnly : Prop)
    (boundsValid : boundsLoaded) (resourceValid : resourceLoaded)
    (emptyGraph : graphEmpty) (emptyMetadata : metadataEmpty) (onlyNull : nullOnly) :
    independentPureNull original boundsLoaded resourceLoaded graphEmpty metadataEmpty nullOnly ↔ original := by
  simp [independentPureNull, boundsValid, resourceValid, emptyGraph, emptyMetadata, onlyNull]

theorem pure_null_graph_introduction_refuses
    (original boundsLoaded resourceLoaded metadataEmpty nullOnly : Prop) :
    ¬ independentPureNull original boundsLoaded resourceLoaded False metadataEmpty nullOnly := by
  simp [independentPureNull]

theorem pure_null_metadata_introduction_refuses
    (original boundsLoaded resourceLoaded graphEmpty nullOnly : Prop) :
    ¬ independentPureNull original boundsLoaded resourceLoaded graphEmpty False nullOnly := by
  simp [independentPureNull]

theorem both_cases_require_resource_validation
    (original boundsLoaded resourceLoaded graphMatched erasureNeutral graphEmpty metadataEmpty nullOnly : Prop)
    (nf rf : Nat → Option Flow) (nl rl : Nat → Option Lifecycle)
    (accepted : admitted original boundsLoaded resourceLoaded graphMatched erasureNeutral nf rf nl rl ∨
      independentPureNull original boundsLoaded resourceLoaded graphEmpty metadataEmpty nullOnly) :
    resourceLoaded := by
  cases accepted with
  | inl shared => exact shared.2.2.1
  | inr independent => exact independent.2.2.1

end QCore.SymbolicOverlapAdmission
