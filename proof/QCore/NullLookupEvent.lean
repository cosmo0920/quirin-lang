import QCore.NullTriagePaths

/-!
# A lookup's miss as a NULL event (card NULL-WITNESS-GUARD-CORRELATION, gateway rule 39)

The abstract rule of `crates/qcore-core/src/null_lookup.rs` and of the path
search `crates/qcore-c-gateway/src/null_deref/witness.rs`, on top of
`QCore.NullTriagePaths`.

A *lookup* is a call whose declared result is borrowed from an argument and
nullable. Its miss is a function of the state its arguments reach, so the
path search (rule 14r) may name it as the NULL event of a witness only when
the miss is feasible on the path by facts the search reads:

* **A tie decides or refuses** (`a_tied_lookup_is_named_only_when_decided_null`).
  When the unit holds the lookup's body and it is one read of a place over
  its parameters, the result is that place: the miss is named only when the
  path's own test fixed the place NULL; any other reading refuses it.
* **Otherwise only free state is named** (`a_named_miss_is_decided_or_free`).
  An argument is free when it is a constant, or rooted at a formal no earlier
  decision, store, assignment or call read or wrote through, or at a global
  with, in addition, no call before; a touched root, a local and anything
  unrooted are correlated (`touching_never_frees`,
  `a_local_or_unrooted_argument_is_correlated`), and one correlated argument
  correlates the lookup (`one_correlated_argument_correlates`).
* **A correlated miss is a value the path chose** (`missOrigin`,
  `a_correlated_miss_never_contributes`): read through
  `QCore.NullTriagePaths`, a free miss is an entry value and a correlated one
  is like a call's result, which never contributes to an exact witness.
* **A refusal never proves anything** (`a_refused_miss_never_concludes_infeasible`,
  `a_refused_miss_is_never_a_finding`): the refused path ends unresolved, so
  the search is incomplete: never `infeasible` (no candidate is removed, no
  row becomes `Safe`), and with no exact reach never a `finding`.

## What this is not

A model of C or of Clang's AST: that the search roots an argument, recognises
a place-read body and sees the decisions, stores and calls that touch a root
as the Rust does is a review premise (trust boundary 28), pinned by the
controls in `crates/qcore-c-gateway/tests/null_witness_guard_correlation.rs`.
The tie is read only to name or refuse a miss, never to prove a result
non-null.
-/

namespace QCore.NullLookupEvent

/-- What the lookup's body ties its result to (`null_lookup::Tie`). -/
inductive Tie where
  | none
  | decidedNull
  | other
  deriving DecidableEq, Repr

/-- Where an argument is rooted (`null_lookup::Root`). -/
inductive Root where
  | constant | formal | global | local | unrooted
  deriving DecidableEq, Repr

/-- The state a lookup reads (`null_lookup::State`). -/
inductive State where
  | free | correlated
  deriving DecidableEq, Repr

/-- Whether the search may name the miss (`null_lookup::MissEvent`). -/
inductive MissEvent where
  | named | refused
  deriving DecidableEq, Repr

/-- `null_lookup::argument_state`. -/
def argumentState : Root → Bool → Bool → State
  | .constant, _, _ => .free
  | .formal, false, _ => .free
  | .global, false, false => .free
  | _, _, _ => .correlated

/-- `null_lookup::lookup_state`. -/
def lookupState (args : List State) : State :=
  if args.contains .correlated then .correlated else .free

/-- `null_lookup::miss_event`. -/
def missEvent : Tie → State → MissEvent
  | .decidedNull, _ => .named
  | .none, .free => .named
  | _, _ => .refused

/-! ## The miss event -/

/-- **A named miss is the path's own NULL test or a miss over free state.** -/
theorem a_named_miss_is_decided_or_free (t : Tie) (s : State)
    (h : missEvent t s = .named) : t = .decidedNull ∨ (t = .none ∧ s = .free) := by
  cases t <;> cases s <;> simp_all [missEvent]

/-- **A tied lookup is named only when the path fixed its place NULL.** -/
theorem a_tied_lookup_is_named_only_when_decided_null (t : Tie) (s : State)
    (ht : t ≠ .none) : missEvent t s = .named ↔ t = .decidedNull := by
  cases t <;> cases s <;> simp_all [missEvent]

/-- **Correlation never names an untied miss.** -/
theorem a_correlated_untied_miss_is_refused : missEvent .none .correlated = .refused := rfl

/-- **Correlation never turns a refusal into a name.** -/
theorem correlation_never_names (t : Tie)
    (h : missEvent t .free = .refused) : missEvent t .correlated = .refused := by
  cases t <;> simp_all [missEvent]

/-! ## The state a lookup reads -/

/-- **A touched root is never free** (only a constant is). -/
theorem touching_never_frees (r : Root) (c : Bool)
    (h : argumentState r true c = .free) : r = .constant := by
  cases r <;> cases c <;> simp_all [argumentState]

/-- **A call before the lookup correlates a global.** -/
theorem a_call_before_never_frees_a_global (t : Bool) :
    argumentState .global t true = .correlated := by
  cases t <;> rfl

/-- **A local's or an unrooted argument's state is correlated.** -/
theorem a_local_or_unrooted_argument_is_correlated (r : Root) (t c : Bool)
    (h : r = .local ∨ r = .unrooted) : argumentState r t c = .correlated := by
  rcases h with rfl | rfl <;> cases t <;> cases c <;> rfl

/-- **One correlated argument correlates the lookup.** -/
theorem one_correlated_argument_correlates (args : List State)
    (h : State.correlated ∈ args) : lookupState args = .correlated := by
  simp [lookupState, h]

/-- **A free lookup has only free arguments.** -/
theorem a_free_lookup_has_only_free_arguments (args : List State)
    (h : lookupState args = .free) : ∀ s ∈ args, s = .free := by
  intro s hs
  cases s with
  | free => rfl
  | correlated =>
      have := one_correlated_argument_correlates args hs
      rw [this] at h
      cases h

/-! ## Bridge to the path search -/

open QCore.NullTriagePaths (Origin exactWitness contributes conclude)

/-- A miss read as a value: free state is an entry value, correlated state a
value the path's own decisions, stores or calls chose (like a call result). -/
def missOrigin : State → Origin
  | .free => .entry
  | .correlated => .callResult

/-- **A correlated miss never contributes to an exact witness.** -/
theorem a_correlated_miss_never_contributes (origin : Nat → Origin)
    (path : List QCore.NullTriage.Step) (h : exactWitness origin path) (a : Nat)
    (ha : origin a = missOrigin .correlated) : ¬ contributes path a :=
  QCore.NullTriagePaths.a_call_result_never_contributes origin path h a ha

/-- **A refused miss never proves infeasibility**: its path ends unresolved,
so the search is incomplete. -/
theorem a_refused_miss_never_concludes_infeasible (e i : Bool) :
    conclude e i false ≠ "infeasible" := by
  cases e <;> cases i <;> decide

/-- **A refused miss is never a finding**: it adds no exact reach. -/
theorem a_refused_miss_is_never_a_finding (i c : Bool) :
    conclude false i c ≠ "finding" :=
  QCore.NullTriagePaths.a_finding_needs_an_exact_path i c

/-! ## Vectors bound to the Rust -/

def tieOf : String → Tie
  | "none" => .none
  | "decided-null" => .decidedNull
  | _ => .other

def stateOf : String → State
  | "free" => .free
  | _ => .correlated

def stateName : State → String
  | .free => "free"
  | .correlated => "correlated"

def eventName : MissEvent → String
  | .named => "named"
  | .refused => "refused"

def rootOf : String → Root
  | "constant" => .constant
  | "formal" => .formal
  | "global" => .global
  | "local" => .local
  | _ => .unrooted

/-- Tie, state, event: `MISS_VECTORS`. -/
def missVectors : List (String × String × String) := [
  ("none", "free", "named"),
  ("none", "correlated", "refused"),
  ("decided-null", "free", "named"),
  ("decided-null", "correlated", "named"),
  ("other", "free", "refused"),
  ("other", "correlated", "refused")
]

theorem miss_vectors_hold :
    missVectors.all (fun (t, s, o) => eventName (missEvent (tieOf t) (stateOf s)) == o) = true := by
  decide

/-- Root, touched, call before, state: `STATE_VECTORS`. -/
def stateVectors : List (String × Bool × Bool × String) := [
  ("constant", true, true, "free"),
  ("formal", false, false, "free"),
  ("formal", false, true, "free"),
  ("formal", true, false, "correlated"),
  ("global", false, false, "free"),
  ("global", false, true, "correlated"),
  ("global", true, false, "correlated"),
  ("local", false, false, "correlated"),
  ("unrooted", false, false, "correlated")
]

theorem state_vectors_hold :
    stateVectors.all (fun (r, t, c, o) => stateName (argumentState (rootOf r) t c) == o) = true := by
  decide

end QCore.NullLookupEvent
