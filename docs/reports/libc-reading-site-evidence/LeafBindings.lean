import QCore.LibcReadingPremises

/-! Named leaves of the already audited guarded decomposition. These are Prop
inputs in an independently held environment, not extracted C facts or a typed
registry. Their domain is all relevant guarded sites, not successful captures. -/
namespace QCore.LibcReadingLeafBindings
open QCore.LibcReadingPremises

def accessorRoute (e : ReadingEnvironment S) : Prop :=
  ∀ s, e.observed s → e.nestedRoute s ∨ e.scalarRoute s

def accessorDeclaration (e : ReadingEnvironment S) : Prop :=
  ∀ s, e.observed s → e.scalarDeclaration s

def accessorEffects (e : ReadingEnvironment S) : Prop :=
  ∀ s, e.observed s → e.untrackedEffects s

def nestedSite (e : NestedEnvironment S) : Prop :=
  ∀ s, e.observed s → e.checkedSite s

def contractForce (e : NestedEnvironment S) : Prop := e.contractInForce

def resultNone (e : NestedEnvironment S) : Prop := e.noCustodyResult

def parametersBorrowed (e : NestedEnvironment S) : Prop := e.parametersBorrowed

def nestedRest (e : NestedEnvironment S) : Prop :=
  ∀ s, e.observed s → e.remainingChecks s

theorem accessor_leaf_definitions_preserve_sentence (e : ReadingEnvironment S) :
    accessorSentence e ↔ accessorRoute e ∧ accessorDeclaration e ∧ accessorEffects e := by
  exact accessor_decomposition_preserves_guarded_alternative e

theorem nested_leaf_definitions_preserve_sentence (e : NestedEnvironment S) :
    nestedSentence e ↔ nestedSite e ∧ contractForce e ∧ resultNone e ∧
      parametersBorrowed e ∧ nestedRest e := by
  exact nested_decomposition_preserves_declaration_and_site_obligations e

theorem a_guarded_declaration_counterexample_refutes_its_leaf
    (e : ReadingEnvironment S) (s : S) (seen : e.observed s)
    (counterexample : ¬ e.scalarDeclaration s) : ¬ accessorDeclaration e := by
  intro universal
  exact counterexample (universal s seen)

end QCore.LibcReadingLeafBindings

#print axioms QCore.LibcReadingLeafBindings.accessor_leaf_definitions_preserve_sentence
#print axioms QCore.LibcReadingLeafBindings.nested_leaf_definitions_preserve_sentence
#print axioms QCore.LibcReadingLeafBindings.a_guarded_declaration_counterexample_refutes_its_leaf
