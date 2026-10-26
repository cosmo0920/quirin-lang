/-! A theorem application retains every prerequisite. The model implication is
an independent input; syntax, registration and artifact hashes do not supply it.
No theorem here asserts plugin source correspondence or caller-held evidence. -/
namespace QCore.QuantTheorem

def Prerequisites (premises : List Prop) : Prop := ∀ p ∈ premises, p

structure Obligations (premises : List Prop) (conclusion : Prop) : Prop where
  modelProof : Prerequisites premises → conclusion
  sourcePrerequisites : Prerequisites premises

theorem instantiate {premises : List Prop} {conclusion : Prop}
    (obligations : Obligations premises conclusion) : conclusion :=
  obligations.modelProof obligations.sourcePrerequisites

theorem compose {premises : List Prop} {intermediate conclusion : Prop}
    (first : Prerequisites premises → intermediate)
    (second : intermediate → conclusion) : Prerequisites premises → conclusion :=
  fun h => second (first h)

theorem every_prerequisite_remains {premises : List Prop} {conclusion p : Prop}
    (obligations : Obligations premises conclusion) (member : p ∈ premises) : p :=
  obligations.sourcePrerequisites p member

theorem reference_does_not_supply_prerequisite : ¬ (∀ p : Prop, (p → p) → p) := by
  intro h
  exact h False (fun p => p)

inductive PluginRole where
  | input | output | filter | processor | custom
  deriving DecidableEq, Repr

/-- Role-specific correspondence is supplied independently for every role. -/
theorem role_instantiation (premise : PluginRole → Prop) (conclusion : PluginRole → Prop)
    (model : ∀ role, premise role → conclusion role) (role : PluginRole)
    (sourceBinding : premise role) : conclusion role :=
  model role sourceBinding

end QCore.QuantTheorem
