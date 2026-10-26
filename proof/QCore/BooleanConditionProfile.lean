import QCore.ChunkSource

/-! Exact mathematical caller domains for the bounded Boolean condition reader.
Model/source correspondence, mapped-object inventory and caller conditions are
inputs. These proofs establish neither C liveness nor synchronization. -/
namespace QCore.BooleanConditionProfile

def Domain (value : Int) : Prop := value = 0 ∨ value = 1

def Live (modeled : Object → Prop) (live : Object → Prop) : Prop :=
  ∀ object, modeled object → live object

def Exclusive (modeled : Object → Prop) (during : Event → Prop)
    (access : Event → Object → Prop) (actor : Event → Actor) (caller : Actor) : Prop :=
  ∀ event object, modeled object ∧ during event ∧ access event object → actor event = caller

theorem domain_iff_boolean_encoding (value : Int) :
    Domain value ↔ ∃ boolean : Bool, value = ChunkSource.boolInt boolean := by
  constructor
  · intro valid
    rcases valid with zero | one
    · exact ⟨false, by simpa [ChunkSource.boolInt] using zero⟩
    · exact ⟨true, by simpa [ChunkSource.boolInt] using one⟩
  · rintro ⟨boolean, rfl⟩
    cases boolean <;> simp [Domain, ChunkSource.boolInt]

theorem one_domain_exhaustive (predicate : Int → Prop) :
    (∀ value, Domain value → predicate value) ↔ (predicate 0 ∧ predicate 1) := by
  constructor
  · intro allValues
    exact ⟨allValues 0 (Or.inl rfl), allValues 1 (Or.inr rfl)⟩
  · rintro ⟨zero, one⟩ value (valid | valid) <;> simpa [valid]

theorem two_domains_exhaustive (predicate : Int → Int → Prop) :
    (∀ first second, Domain first → Domain second → predicate first second) ↔
    (predicate 0 0 ∧ predicate 0 1 ∧ predicate 1 0 ∧ predicate 1 1) := by
  constructor
  · intro allValues
    exact ⟨allValues 0 0 (Or.inl rfl) (Or.inl rfl),
      allValues 0 1 (Or.inl rfl) (Or.inr rfl),
      allValues 1 0 (Or.inr rfl) (Or.inl rfl),
      allValues 1 1 (Or.inr rfl) (Or.inr rfl)⟩
  · rintro ⟨zz, zo, oz, oo⟩ first second (hf | hf) (hs | hs) <;> simp_all

theorem live_applies_only_under_modeled_membership (modeled live : Object → Prop)
    (callerLive : Live modeled live) (object : Object) :
    modeled object → live object := by
  intro member
  exact callerLive object member

theorem exclusive_selects_modeled_access (modeled : Object → Prop)
    (during : Event → Prop) (access : Event → Object → Prop)
    (actor : Event → Actor) (caller : Actor)
    (callerExclusive : Exclusive modeled during access actor caller)
    (event : Event) (object : Object) (member : modeled object)
    (active : during event) (touches : access event object) : actor event = caller :=
  callerExclusive event object ⟨member, active, touches⟩

end QCore.BooleanConditionProfile
