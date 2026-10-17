/-!
# Conditional transport of a W4 summary across translation units

Card **W44-CALLEE-CROSS-UNIT-SUMMARY**. A source corpus alone does not
establish a linked program. The call-to-home relation below is an explicit
premise, and the source and environment identities are equalities checked by
both replays. The caller receives a summary only if the home derivation is
valid, the selected home is unique, and no declaration takes precedence.
-/

namespace QCore.CrossUnitSummary

abbrev Name := String
abbrev Unit := String

structure Home (σ : Type) where
  function : Name
  unit : Unit
  sourceDigest : String
  environmentDigest : String
  summary : σ

/-- A digest names the caller's explicit linked-program resolution premise. -/
structure LinkPremise where
  digest : String
  resolves : Name → Unit → Prop

def uniqueHome (definitions : List (Unit × Name)) (h : Home σ) : Prop :=
  definitions.filter (fun row => row.2 == h.function) = [(h.unit, h.function)]

def admitted (definitions : List (Unit × Name)) (link : LinkPremise)
    (declared : Name → Option σ) (home : Home σ)
    (replayedSource replayedEnvironment : String) (valid : Home σ → Prop) : Prop :=
  uniqueHome definitions home ∧
  link.resolves home.function home.unit ∧
  declared home.function = none ∧
  home.sourceDigest = replayedSource ∧
  home.environmentDigest = replayedEnvironment ∧
  valid home

/-- Imported summaries occupy a separate channel; declarations win. -/
def lookup (declared : Name → Option σ) (imports : List (Home σ))
    (name : Name) : Option σ :=
  match declared name with
  | some contract => some contract
  | none => (imports.find? (fun home => home.function == name)).map (·.summary)

theorem declaration_wins {σ : Type} (declared : Name → Option σ)
    (imports : List (Home σ)) (name : Name) (contract : σ)
    (h : declared name = some contract) :
    lookup declared imports name = some contract := by
  simp [lookup, h]

theorem admitted_import_is_valid {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (admission : admitted definitions link declared home source environment valid) :
    valid home ∧ link.resolves home.function home.unit ∧
      uniqueHome definitions home := by
  exact ⟨admission.2.2.2.2.2, admission.2.1, admission.1⟩

theorem missing_resolution_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (missing : ¬ link.resolves home.function home.unit) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact missing admission.2.1

theorem ambiguous_home_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (ambiguous : ¬ uniqueHome definitions home) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact ambiguous admission.1

theorem changed_source_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (changed : home.sourceDigest ≠ source) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact changed admission.2.2.2.1

theorem changed_environment_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (changed : home.environmentDigest ≠ environment) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact changed admission.2.2.2.2.1

end QCore.CrossUnitSummary
