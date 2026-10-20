/-! Cycle1 mathematical decomposition of the cJSON representation declaration.
These propositions describe an assumed heap and an interference trace. No theorem
establishes that imported C, a caller heap or a provider claim satisfies them. -/
namespace QCore.CjsonRepresentationPremises

structure Step where
  actor : Nat
  writesLinks : Bool

structure Environment where
  callActor : Nat
  parent : Nat
  children : List Nat
  ownedChild : Nat → Prop
  live : Nat → Prop
  child : Option Nat
  next : Nat → Option Nat
  prev : Nat → Option Nat
  trace : List Step

inductive NextPath (next : Nat → Option Nat) : Nat → Nat → Prop
  | step {a b} : next a = some b → NextPath next a b
  | trans {a b c} : NextPath next a b → NextPath next b c → NextPath next a c

/-- The finite sequence is the exact owned-child domain; adjacent nodes have
    the next equation. Finiteness is supplied by List, not inferred from C. -/
def chain (e : Environment) : Prop :=
  e.live e.parent ∧
  (∀ x, e.ownedChild x ↔ x ∈ e.children) ∧
  (∀ x ∈ e.children, e.live x) ∧
  e.child = e.children.head? ∧
  (∀ before a b suffix, e.children = before ++ a :: b :: suffix → e.next a = some b) ∧
  (∀ x ∈ e.children, ¬ NextPath e.next x x)

def distinct (e : Environment) : Prop := e.children.Nodup

def empty (e : Environment) : Prop :=
  (e.child = none ↔ e.children = [])

def firstPrev (e : Environment) : Prop :=
  ∀ first last, e.children.head? = some first →
    e.children.getLast? = some last → e.prev first = some last

def lastNext (e : Environment) : Prop :=
  ∀ last, e.children.getLast? = some last → e.next last = none

def linkTargets (e : Environment) : Prop :=
  ∀ x ∈ e.children, ∀ y,
    e.next x = some y ∨ e.prev x = some y → y ∈ e.children ∧ e.live y

def noWriter (e : Environment) : Prop :=
  ∀ step ∈ e.trace, step.actor ≠ e.callActor → step.writesLinks = false

def parentOutside (e : Environment) : Prop := e.parent ∉ e.children

def predecessor (e : Environment) : Prop :=
  ∀ before a b suffix, e.children = before ++ a :: b :: suffix → e.prev b = some a

/-- In the entry heap, a link target is in the parent's exact custody domain
    and cannot denote the parent. No trace-preservation theorem is claimed.
    The link-target and exclusion assumptions are essential. -/
theorem links_stay_live_owned_and_exclude_parent (e : Environment)
    (hc : chain e) (ht : linkTargets e) (hp : parentOutside e)
    (x y : Nat) (hx : x ∈ e.children)
    (hl : e.next x = some y ∨ e.prev x = some y) :
    e.live y ∧ e.ownedChild y ∧ y ≠ e.parent := by
  obtain ⟨hy, live⟩ := ht x hx y hl
  refine ⟨live, (hc.2.1 y).mpr hy, ?_⟩
  intro heq
  exact hp (heq ▸ hy)

/-- The exact finite representation precludes two different list occurrences
    naming one child, expressed as the injectivity of member indices. -/
theorem distinct_indices_identify_distinct_children (e : Environment)
    (hd : distinct e) (i j : Fin e.children.length)
    (same : e.children[i] = e.children[j]) : i = j := by
  apply Fin.ext
  exact hd.eq_of_getElem_eq i.isLt j.isLt same

/-- Writer exclusion is an explicit temporal antecedent, independent from the
    heap's link equations; any listed other-actor writing step refutes that antecedent. -/
theorem a_writing_step_refutes_no_writer (e : Environment) (step : Step)
    (member : step ∈ e.trace) (other : step.actor ≠ e.callActor)
    (writes : step.writesLinks = true) : ¬ noWriter e := by
  intro h
  have no := h step member other
  rw [writes] at no
  contradiction

end QCore.CjsonRepresentationPremises
