/-! Evidence-only finite observations. The source reader, extraction fidelity,
loaded C contract truth, and Rust correspondence remain explicit assumptions.
No definition below returns a custody verdict or universal hypothesis evidence. -/
namespace QCore.LibcReadingSiteEvidence

structure Context where
  artifact : Nat
  property : Nat
  profile : Nat
  scope : Nat
  bounds : Nat
  assumptions : Nat
  source : Nat
  build : Nat
  version : Nat
  deriving DecidableEq

structure Observation where
  owner : Nat
  callStart : Nat
  callEnd : Nat
  siblings : List (Nat × Nat)
  predicates : List Bool
  deriving DecidableEq

structure Receipt where
  context : Context
  graph : Nat
  atoms : List Nat
  observation : Observation

/-- Independent replay supplies the actual reader observations; this predicate
cannot establish that any recorded declaration describes real C execution. -/
def accepts (held : Context) (graph : Nat) (atoms : List Nat)
    (fresh : List Observation) (r : Receipt) : Prop :=
  r.context = held ∧ r.graph = graph ∧ r.atoms = atoms ∧
  r.observation ∈ fresh ∧ r.observation.callStart < r.observation.callEnd ∧
  r.observation.predicates.all id = true

theorem stale_context_rejected (held : Context) (g : Nat) (a : List Nat)
    (fresh : List Observation) (r : Receipt) (stale : r.context ≠ held) :
    ¬ accepts held g a fresh r := by
  intro accepted
  exact stale accepted.1

theorem self_consistent_but_unreproduced_rejected (held : Context) (g : Nat)
    (a : List Nat) (fresh : List Observation) (r : Receipt)
    (absent : r.observation ∉ fresh) : ¬ accepts held g a fresh r := by
  intro accepted
  exact absent accepted.2.2.2.1

/-- Replay checks each recorded local predicate; this is a finite domain only. -/
theorem accepted_local_predicate_is_true (held : Context) (g : Nat) (a : List Nat)
    (fresh : List Observation) (r : Receipt) (p : Bool)
    (accepted : accepts held g a fresh r) (member : p ∈ r.observation.predicates) :
    p = true := by
  have every := List.all_eq_true.mp accepted.2.2.2.2.2
  exact every p member

/-- One actual finite observation cannot establish the unobserved universe. -/
theorem selected_success_does_not_imply_universal :
    (∀ n ∈ ([0] : List Nat), n = 0) ∧ ¬ (∀ n : Nat, n = 0) := by
  constructor
  · intro n member
    simpa using member
  · intro universal
    have impossible := universal 1
    contradiction

/-- A declined probe restores the exact retained observations, even if a local
checker had temporarily appended successful observations before declining. -/
def finishProbe (before during : List Observation) (keep : Bool) : List Observation :=
  if keep then during else before

theorem declined_probe_does_not_publish_new_site (before during : List Observation)
    (o : Observation) (absent : o ∉ before) :
    o ∉ finishProbe before during false := by
  simpa [finishProbe] using absent

end QCore.LibcReadingSiteEvidence
