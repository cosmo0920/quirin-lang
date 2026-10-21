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
  apply changed_flow_entry_refuses
  rw [absent, present]
  exact Option.noConfusion

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

end QCore.SymbolicOverlapAdmission
