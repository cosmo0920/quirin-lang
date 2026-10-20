/-! Conditional logical decomposition of the opaque libc reading premises.
The subject is one reading site in an independently held audited unit/context.
Predicates below are inputs, not established C facts or graph-discharge rules. -/
namespace QCore.LibcReadingPremises

structure ReadingEnvironment (Site : Type) where
  observed : Site → Prop
  nestedRoute : Site → Prop
  scalarRoute : Site → Prop
  scalarDeclaration : Site → Prop
  untrackedEffects : Site → Prop

/-- The pre-migration accessor sentence, with its disjunction intact. -/
def accessorSentence (e : ReadingEnvironment S) : Prop :=
  ∀ s, e.observed s →
    (e.nestedRoute s ∨ e.scalarRoute s) ∧ e.scalarDeclaration s ∧ e.untrackedEffects s

/-- The three opaque graph atoms, each quantified over the same reading sites. -/
def accessorAtoms (e : ReadingEnvironment S) : Prop :=
  (∀ s, e.observed s → e.nestedRoute s ∨ e.scalarRoute s) ∧
  (∀ s, e.observed s → e.scalarDeclaration s) ∧
  (∀ s, e.observed s → e.untrackedEffects s)

theorem accessor_decomposition_preserves_guarded_alternative (e : ReadingEnvironment S) :
    accessorSentence e ↔ accessorAtoms e := by
  constructor
  · intro h
    exact ⟨fun s hs => (h s hs).1,
      fun s hs => (h s hs).2.1, fun s hs => (h s hs).2.2⟩
  · rintro ⟨route, declaration, effects⟩ s hs
    exact ⟨route s hs, declaration s hs, effects s hs⟩

theorem scalar_route_does_not_require_nested_route (e : ReadingEnvironment S) (s : S)
    (scalar : e.scalarRoute s) (decl : e.scalarDeclaration s)
    (effects : e.untrackedEffects s) :
    (e.nestedRoute s ∨ e.scalarRoute s) ∧ e.scalarDeclaration s ∧ e.untrackedEffects s := by
  exact ⟨Or.inr scalar, decl, effects⟩

/-- A concrete observed-site counterexample refutes the quantified assumption,
not a C program or its custody verdict. -/
theorem observed_declaration_counterexample_refutes_accessor (e : ReadingEnvironment S)
    (counterexample : ∃ s, e.observed s ∧ ¬ e.scalarDeclaration s) :
    ¬ accessorSentence e := by
  rintro assumption
  obtain ⟨s, seen, incompatible⟩ := counterexample
  exact incompatible (assumption s seen).2.1

structure NestedEnvironment (Site : Type) where
  observed : Site → Prop
  checkedSite : Site → Prop
  contractInForce : Prop
  noCustodyResult : Prop
  parametersBorrowed : Prop
  remainingChecks : Site → Prop

def nestedSentence (e : NestedEnvironment S) : Prop :=
  (∀ s, e.observed s → e.checkedSite s) ∧
  (e.contractInForce ∧ e.noCustodyResult ∧ e.parametersBorrowed) ∧
  (∀ s, e.observed s → e.remainingChecks s)

def nestedAtoms (e : NestedEnvironment S) : Prop :=
  (∀ s, e.observed s → e.checkedSite s) ∧ e.contractInForce ∧
  e.noCustodyResult ∧ e.parametersBorrowed ∧
  (∀ s, e.observed s → e.remainingChecks s)

theorem nested_decomposition_preserves_declaration_and_site_obligations
    (e : NestedEnvironment S) : nestedSentence e ↔ nestedAtoms e := by
  constructor
  · rintro ⟨site, ⟨force, result, parameters⟩, rest⟩
    exact ⟨site, force, result, parameters, rest⟩
  · rintro ⟨site, force, result, parameters, rest⟩
    exact ⟨site, ⟨force, result, parameters⟩, rest⟩

end QCore.LibcReadingPremises
