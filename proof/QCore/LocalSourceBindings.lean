import QCore.CjsonRepresentationPremises
import QCore.LibcReadingSiteEvidence

/-! Finite source-local refinement only. Native allocation/execution, Clang
projection and Rust/Lean correspondence are explicit external prerequisites.
No theorem converts a finite scope into an opaque universal graph proposition. -/
namespace QCore.LocalSourceBindings
open CjsonRepresentationPremises

/-- The Rust adapter derives these equations from retained native object fields.
`ownedChild` in this finite view means exactly the selected sequence, not a
claim about arbitrary caller custody. The rank is the sequence index. -/
structure EntryCheck (e : Environment) (rank : Nat → Nat) : Prop where
  parentLive : e.live e.parent
  exactDomain : ∀ x, e.ownedChild x ↔ x ∈ e.children
  childrenLive : ∀ x ∈ e.children, e.live x
  head : e.child = e.children.head?
  adjacent : ∀ before a b suffix,
    e.children = before ++ a :: b :: suffix → e.next a = some b
  nextRank : ∀ a b, e.next a = some b → rank a < rank b
  unique : e.children.Nodup
  emptyHead : e.child = none ↔ e.children = []
  first : firstPrev e
  last : lastNext e
  targets : linkTargets e
  outside : parentOutside e
  previous : predecessor e

theorem next_path_increases_rank (next : Nat → Option Nat) (rank : Nat → Nat)
    (grows : ∀ a b, next a = some b → rank a < rank b)
    {a b : Nat} (path : NextPath next a b) : rank a < rank b := by
  induction path with
  | step edge => exact grows _ _ edge
  | trans _ _ first second => exact Nat.lt_trans first second

/-- A finite checked index ranking supplies the nontrivial acyclicity part of
chain. Eight entry propositions follow; noWriter is deliberately absent. -/
theorem checked_entry_has_eight_finite_equations (e : Environment) (rank : Nat → Nat)
    (checked : EntryCheck e rank) :
    chain e ∧ distinct e ∧ empty e ∧ firstPrev e ∧ lastNext e ∧
      linkTargets e ∧ parentOutside e ∧ predecessor e := by
  refine ⟨?_, checked.unique, checked.emptyHead, checked.first, checked.last,
    checked.targets, checked.outside, checked.previous⟩
  refine ⟨checked.parentLive, checked.exactDomain, checked.childrenLive,
    checked.head, checked.adjacent, ?_⟩
  intro x _ path
  exact (Nat.lt_irrefl (rank x))
    (next_path_increases_rank e.next rank checked.nextRank path)

/-- Native single-thread samples do not establish arbitrary interference. -/
theorem a_foreign_link_write_still_refutes_writer_guard (e : Environment) (s : Step)
    (seen : s ∈ e.trace) (foreign : s.actor ≠ e.callActor)
    (writes : s.writesLinks = true) : ¬ noWriter e := by
  exact a_writing_step_refutes_no_writer e s seen foreign writes

/-- Full finite recorded-domain equality, not selected membership. -/
def domainAccepts (held presented fresh : List S) (local : S → Prop) : Prop :=
  presented = held ∧ fresh = held ∧ ∀ s ∈ fresh, local s

theorem complete_recorded_domain_covers_each_held_site (held presented fresh : List S)
    (local : S → Prop) (accepted : domainAccepts held presented fresh local)
    (s : S) (member : s ∈ held) : local s := by
  apply accepted.2.2 s
  simpa [accepted.2.1] using member

theorem truncated_domain_refused (held presented fresh : List S) (local : S → Prop)
    (s : S) (freshMember : s ∈ fresh) (omitted : s ∉ presented) :
    ¬ domainAccepts held presented fresh local := by
  intro accepted
  apply omitted
  simpa [accepted.1, accepted.2.1] using freshMember

/-- A scalar route supplies the same allowed-route disjunction independently
of nested-route truth; the checker never conjoins both alternatives. -/
theorem scalar_site_preserves_alternative (nested scalar : Bool) (h : scalar = true) :
    nested = true ∨ scalar = true := by
  exact Or.inr h

/-- Exact Context remains a prerequisite even after local source binding.
Dependency records here are supplied by the actual consultation closure. -/
structure Row where
  id : Nat
  context : LibcReadingSiteEvidence.Context
  dependencies : List Nat

def affected (ctx : LibcReadingSiteEvidence.Context) (atom : Nat) (row : Row) : Prop :=
  row.context = ctx ∧ atom ∈ row.dependencies

theorem another_context_is_unaffected (ctx : LibcReadingSiteEvidence.Context)
    (atom : Nat) (row : Row) (foreign : row.context ≠ ctx) : ¬ affected ctx atom row := by
  intro h
  exact foreign h.1

theorem unrelated_dependency_is_unaffected (ctx : LibcReadingSiteEvidence.Context)
    (atom : Nat) (row : Row) (absent : atom ∉ row.dependencies) : ¬ affected ctx atom row := by
  intro h
  exact absent h.2

end QCore.LocalSourceBindings
