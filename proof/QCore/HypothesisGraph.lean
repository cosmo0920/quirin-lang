/-! Evidence-only conjunction graphs. Context equality and disposition are distinct
from warrant; no theorem claims C extraction, provider authority or semantic discharge. -/
namespace QCore.HypothesisGraph

structure Context where
  artifact : String
  property : String
  profile : String
  scope : String
  bounds : String
  assumptions : String
  source : String
  build : String
  version : String
  deriving DecidableEq, BEq

inductive Disposition where
  | open | supported | refuted
  deriving DecidableEq, BEq
inductive Warrant where
  | derived | observed | declared | humanReviewed | providerProposed
  deriving DecidableEq, BEq
structure Atom where
  id : Nat
  context : Context
  disposition : Disposition
  warrant : Warrant
  deriving DecidableEq, BEq

structure Premise where
  name : String
  statement : String
  origin : String
  deriving DecidableEq, BEq
structure Conjunction where
  erasure : Premise
  children : List Nat

inductive Node where
  | premise (id : Nat)
  | atom (id : Nat)
  deriving DecidableEq

def edge (graph : List (Nat × Conjunction)) (a b : Node) : Prop :=
  match a, b with
  | .premise p, .atom h => ∃ c, (p, c) ∈ graph ∧ h ∈ c.children
  | _, _ => False

def rank : Node → Nat
  | .premise _ => 1
  | .atom _ => 0

theorem edges_decrease (graph : List (Nat × Conjunction)) (a b : Node)
    (h : edge graph a b) : rank b < rank a := by
  cases a <;> cases b <;> simp_all [edge, rank]

inductive Reaches (graph : List (Nat × Conjunction)) : Node → Node → Prop
  | step {a b} : edge graph a b → Reaches graph a b
  | trans {a b c} : Reaches graph a b → Reaches graph b c → Reaches graph a c

theorem paths_decrease {graph : List (Nat × Conjunction)} {a b : Node}
    (h : Reaches graph a b) : rank b < rank a := by
  induction h with
  | step e => exact edges_decrease _ _ _ e
  | trans _ _ hab hbc => exact Nat.lt_trans hbc hab

theorem graph_acyclic (graph : List (Nat × Conjunction)) (a : Node) :
    ¬ Reaches graph a a := by
  intro h
  exact Nat.lt_irrefl _ (paths_decrease h)

def supported (context : Context) (atoms : List Atom) : Bool :=
  !atoms.isEmpty && atoms.all (fun a => decide (a.context = context ∧ a.disposition = .supported))

def compose (ctx : Context) (atoms : List Atom) : Disposition :=
  if supported ctx atoms then .supported
  else if atoms.all (fun a => decide (a.context = ctx)) &&
      atoms.any (fun a => a.disposition == .refuted) then .refuted
  else .open

theorem composition_support_needs_every_atom (ctx : Context) (atoms : List Atom) :
    compose ctx atoms = .supported ↔ supported ctx atoms = true := by
  simp only [compose]
  split <;> simp_all
  split <;> simp_all

/-- Warrant is absent from the reporting rule's premises. -/
theorem warrant_changes_no_support (ctx : Context) (a : Atom) (w : Warrant) :
    supported ctx [{ a with warrant := w }] = supported ctx [a] := by
  simp [supported]

theorem incompatible_context_refuses (ctx : Context) (a : Atom) (rest : List Atom)
    (h : a.context ≠ ctx) : supported ctx (a :: rest) = false := by
  simp [supported, h]

theorem open_atom_never_promotes (ctx : Context) (a : Atom) (rest : List Atom)
    (h : a.disposition = .open) : supported ctx (a :: rest) = false := by
  simp [supported, h]

theorem refuted_atom_never_promotes (ctx : Context) (a : Atom) (rest : List Atom)
    (h : a.disposition = .refuted) : supported ctx (a :: rest) = false := by
  simp [supported, h]

theorem inserting_open_atom_never_promotes (ctx : Context) (a : Atom)
    (left right : List Atom) (h : a.disposition = .open) :
    supported ctx (left ++ a :: right) = false := by
  simp [supported, List.all_append, h]

theorem inserting_refuted_atom_never_promotes (ctx : Context) (a : Atom)
    (left right : List Atom) (h : a.disposition = .refuted) :
    supported ctx (left ++ a :: right) = false := by
  simp [supported, List.all_append, h]

def legacy (p : Premise) (id : Nat) : Conjunction := ⟨p, [id]⟩
def erase (nodes : List Conjunction) : List Premise := nodes.map (·.erasure)

theorem legacy_erasure (premises : List Premise) (id : Nat) :
    erase (premises.map (fun p => legacy p id)) = premises := by
  induction premises with
  | nil => rfl
  | cons p ps ih => simp_all [erase, legacy]

/-- Publishing metadata is independent of an existing static verdict. -/
def publish (verdict : α) (graph : List Conjunction) := (verdict, graph)
theorem erasing_publication_preserves_verdict (v : α) (g : List Conjunction) :
    (publish v g).1 = v := rfl

end QCore.HypothesisGraph
