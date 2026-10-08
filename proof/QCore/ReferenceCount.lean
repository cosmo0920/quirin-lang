/-!
# Reference counts (card REFCOUNT-RESOURCE-KIND, rules 13b and 13c)

The abstract rule implemented by `crates/qcore-c-gateway/src/reference_count.rs`
(`decide_keep`, `decide_drop`, `owed`, `decide_exit`, `ref_step`,
`ref_replay`) and its route-G adapter `legacy_reference_count.rs`, contract
specification §C04.17.

USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 3): taking a
reference ("keep") is +1 owned reference; dropping one ("destroy", "drop") is
−1; each function must balance the references it takes, or hand them off
(return, store into a tracked owner, a declared transfer), exactly as with
other owned resources; a plain counter that is not a reference needs its own
declaration and never inherits the reference reading. **Rejected:** treating
keep and drop as custody-neutral (a missing drop would read `Safe`).

## The model

* A **family** is the references one function holds on one object: a count
  and whether one of them was handed off. `decideKeep` and `decideDrop` are
  the call-site tables over what the actual denotes (`Binding`).
* A path is a list of `Event`s over names. A keep of an unbound name mints a
  family held once; a keep of a live family adds one; a drop subtracts one,
  and the drop that takes the count to zero is possibly final: the family is
  discharged and every name bound to it becomes invalid. A read of an invalid
  name, a drop the function does not hold, an undeclared primitive (a plain
  counter), another refused statement and a withdrawn declaration refuse, and
  a refusal is sticky.
* `outcome`: refused if the path refused; a leak if a family still owes a
  reference (its count minus its hand-off); otherwise safe.
* `readCall` is how a call becomes an event: a keep or a drop only through a
  declared reference clause, a use through any other declaration, and the
  undeclared primitive is a refused call.

## The theorems

* `a_safe_path_is_balanced` (balance soundness): on a safe path every family
  owes nothing.
* `a_missing_drop_is_never_safe`: a family that still owes a reference makes
  the path not safe; `a_kept_reference_never_dropped_leaks`.
* `a_statistics_counter_is_never_read_as_a_reference`: a keep or a drop is
  read only through a declared reference clause, and the undeclared primitive
  changes no family (`a_counter_changes_no_family`).
* `the_neutral_reading_is_not_admitted`: a declaration with a reference
  clause is withheld from the function map; the neutral reading (keep and
  drop as uses) would certify a missing drop the route reports as a leak;
  a withdrawn declaration refuses.
* `an_alias_use_after_a_possibly_final_drop_is_refused`.
* `reference_vectors_hold`, `reference_trace_vectors_hold`: the tables shared
  with Rust (`crates/qcore-c-gateway/tests/refcount_resource_kind.rs` checks
  that the Rust constants are these rows).

## What this is not

A model of the decision over abstract names. That the gateway reads a keep's
actual as the classes say, that its family bindings are the names whose
bindings target the family's site, that a refused statement on a path makes
the exit a refusal, and the correspondence with the compiled Rust are review
premises (trust boundary 28), pinned by the shared vectors and the route-G
controls. That a declared wrapper's body performs the count arithmetic it
declares is the declared assumption rule 13c checks only structurally.
-/

namespace QCore.ReferenceCount

inductive Held where
  | known (n : Nat)
  | undecided
  deriving DecidableEq, Repr

/-- Two paths' counts (`Held::join`). -/
def Held.join : Held → Held → Held
  | .known a, .known b => if a = b then .known a else .undecided
  | _, _ => .undecided

theorem disagreeing_counts_are_undecided (a b : Nat) (h : a ≠ b) :
    Held.join (.known a) (.known b) = .undecided := by
  simp [Held.join, h]

inductive Binding where
  | unbound
  | family (held : Held) (handed : Bool)
  | invalid
  | other
  deriving DecidableEq, Repr

inductive Reason where
  | dropped
  | unheld
  | undecided
  deriving DecidableEq, Repr

inductive KeepDecision where
  | mint
  | add
  | refuse (reason : Reason)
  deriving DecidableEq, Repr

inductive DropDecision where
  | decrement
  | final
  | refuse (reason : Reason)
  deriving DecidableEq, Repr

/-- Rule 13b's keep table (`reference_count::decide_keep`). -/
def decideKeep : Binding → KeepDecision
  | .unbound => .mint
  | .family (.known _) false => .add
  | .invalid => .refuse .dropped
  | .family _ _ => .refuse .undecided
  | .other => .refuse .undecided

/-- Rule 13b's drop table (`reference_count::decide_drop`). -/
def decideDrop : Binding → DropDecision
  | .family (.known 0) false => .refuse .unheld
  | .family (.known 1) false => .final
  | .family (.known _) false => .decrement
  | .family .undecided _ => .refuse .undecided
  | .family (.known _) true => .refuse .unheld
  | .invalid => .refuse .dropped
  | .unbound => .refuse .unheld
  | .other => .refuse .unheld

/-- Only a name bound to nothing tracked mints a family. -/
theorem a_mint_needs_an_unbound_name (b : Binding) (h : decideKeep b = .mint) :
    b = .unbound := by
  cases b with
  | unbound => rfl
  | family held handed =>
      cases held <;> cases handed <;> simp [decideKeep] at h
  | invalid => simp [decideKeep] at h
  | other => simp [decideKeep] at h

/-- Only the last reference a function holds is a possibly-final drop. -/
theorem a_final_drop_is_the_last_reference (b : Binding) (h : decideDrop b = .final) :
    b = .family (.known 1) false := by
  cases b with
  | unbound => simp [decideDrop] at h
  | family held handed =>
      cases held with
      | known n =>
          cases handed with
          | true => simp [decideDrop] at h
          | false =>
              match n, h with
              | 1, _ => rfl
              | 0, h => simp [decideDrop] at h
              | n + 2, h => simp [decideDrop] at h
      | undecided => cases handed <;> simp [decideDrop] at h
  | invalid => simp [decideDrop] at h
  | other => simp [decideDrop] at h

/-- A drop of a reference the function does not hold is refused. -/
theorem an_unheld_drop_is_refused :
    decideDrop .unbound = .refuse .unheld ∧ decideDrop .other = .refuse .unheld ∧
      decideDrop .invalid = .refuse .dropped := ⟨rfl, rfl, rfl⟩

/-- What a family still owes at an exit (`reference_count::owed`). -/
def owed : Held → Bool → Option Nat
  | .known n, true => if n = 0 then none else some (n - 1)
  | .known n, false => some n
  | .undecided, _ => none

inductive ExitOutcome where
  | balanced
  | leak
  | refused
  deriving DecidableEq, Repr

/-- Rule 13b's exit table (`reference_count::decide_exit`). -/
def decideExit : Option Nat → Bool → ExitOutcome
  | some 0, _ => .balanced
  | some _, true => .leak
  | _, _ => .refused

/-- An exit on a path that refused something never accuses. -/
theorem an_unmodeled_exit_never_leaks (o : Option Nat) : decideExit o false ≠ .leak := by
  match o with
  | some 0 => simp [decideExit]
  | some (n + 1) => simp [decideExit]
  | none => simp [decideExit]

/-- An exit is balanced only when nothing is owed. -/
theorem a_balanced_exit_owes_nothing (o : Option Nat) (m : Bool)
    (h : decideExit o m = .balanced) : o = some 0 := by
  match o, m with
  | some 0, _ => rfl
  | some (n + 1), true => simp [decideExit] at h
  | some (n + 1), false => simp [decideExit] at h
  | none, true => simp [decideExit] at h
  | none, false => simp [decideExit] at h

/-! ## The replay -/

inductive Name where
  | bound (family : Nat)
  | invalid
  deriving DecidableEq, Repr

structure State where
  names : List (Nat × Name)
  /-- `(family, held, handed)`. -/
  families : List (Nat × Nat × Bool)
  next : Nat
  refused : Bool
  deriving DecidableEq, Repr

def State.empty : State := ⟨[], [], 0, false⟩

def nameOf (st : State) (n : Nat) : Option Name :=
  (st.names.find? fun e => e.1 == n).map (·.2)

def setName (st : State) (n : Nat) : Option Name → State
  | some b => { st with names := (n, b) :: st.names.filter (fun e => e.1 != n) }
  | none => { st with names := st.names.filter (fun e => e.1 != n) }

def familyOf (st : State) (f : Nat) : Option (Nat × Bool) :=
  (st.families.find? fun e => e.1 == f).map fun e => (e.2.1, e.2.2)

def bindingOf (st : State) (n : Nat) : Binding :=
  match nameOf st n with
  | none => .unbound
  | some .invalid => .invalid
  | some (.bound f) =>
      match familyOf st f with
      | some (h, handed) => .family (.known h) handed
      | none => .other

def update (st : State) (f h : Nat) (handed : Bool) : State :=
  { st with families := st.families.map fun e => if e.1 == f then (f, h, handed) else e }

def refuse (st : State) : State := { st with refused := true }

def keep (st : State) (n : Nat) : State :=
  match decideKeep (bindingOf st n), nameOf st n with
  | .mint, _ =>
      { setName st n (some (.bound st.next)) with
          families := st.families ++ [(st.next, 1, false)], next := st.next + 1 }
  | .add, some (.bound f) =>
      match familyOf st f with
      | some (h, handed) => update st f (h + 1) handed
      | none => st
  | _, _ => refuse st

inductive Event where
  | keep (n : Nat)
  | keepTo (d n : Nat)
  | alias (d n : Nat)
  | drop (n : Nat)
  | use (n : Nat)
  | handoff (n : Nat)
  | counter (n : Nat)
  | refuse
  | withdrawn (n : Nat)
  deriving DecidableEq, Repr

/-- `reference_count::ref_step`. -/
def step (st : State) : Event → State
  | .keep n => keep st n
  | .keepTo d n =>
      let s := keep st n
      if s.refused then s else setName s d (nameOf s n)
  | .alias d n =>
      match nameOf st n with
      | some .invalid => refuse st
      | b => setName st d b
  | .drop n =>
      match decideDrop (bindingOf st n), nameOf st n with
      | .decrement, some (.bound f) =>
          match familyOf st f with
          | some (h, handed) => update st f (h - 1) handed
          | none => st
      | .final, some (.bound f) =>
          { st with families := st.families.filter (fun e => e.1 != f),
                    names := st.names.map fun e => if e.2 == .bound f then (e.1, .invalid) else e }
      | _, _ => refuse st
  | .use n => if nameOf st n == some .invalid then refuse st else st
  | .handoff n =>
      match nameOf st n with
      | some .invalid => refuse st
      | some (.bound f) =>
          match familyOf st f with
          | some (h, false) => update st f h true
          | _ => refuse st
      | none => st
  | .counter _ => refuse st
  | .refuse => refuse st
  | .withdrawn _ => refuse st

def run (es : List Event) : State := es.foldl step State.empty

/-- The families that still owe a reference. -/
def owing (st : State) : List (Nat × Nat × Bool) :=
  st.families.filter fun e => owed (.known e.2.1) e.2.2 != some 0

inductive Outcome where
  | safe
  | leak
  | refused
  deriving DecidableEq, Repr

def outcome (st : State) : Outcome :=
  if st.refused then .refused else if (owing st).isEmpty then .safe else .leak

/-! ## Balance -/

/-- **Balance soundness**: on a safe path every family owes nothing. -/
theorem a_safe_path_is_balanced (st : State) (h : outcome st = .safe) :
    ∀ e ∈ st.families, owed (.known e.2.1) e.2.2 = some 0 := by
  intro e he
  unfold outcome at h
  split at h
  · cases h
  · split at h
    · rename_i hempty
      have hnil : owing st = [] := List.isEmpty_iff.mp hempty
      unfold owing at hnil
      have := List.filter_eq_nil_iff.mp hnil e he
      simpa using this
    · cases h

/-- **A missing drop is never `Safe`**: a family that still owes a reference
makes the path not safe. -/
theorem a_missing_drop_is_never_safe (st : State) (e : Nat × Nat × Bool)
    (he : e ∈ st.families) (howes : owed (.known e.2.1) e.2.2 ≠ some 0) :
    outcome st ≠ .safe := by
  intro h
  exact howes (a_safe_path_is_balanced st h e he)

theorem a_kept_reference_never_dropped_leaks : outcome (run [.keep 0]) = .leak := by
  decide

theorem setName_refused (st : State) (n : Nat) (b : Option Name) :
    (setName st n b).refused = st.refused := by
  cases b <;> rfl

/-- A refusal is sticky. -/
theorem step_keeps_refused (st : State) (e : Event) (h : st.refused = true) :
    (step st e).refused = true := by
  have hk : ∀ n, (keep st n).refused = true := by
    intro n
    unfold keep
    split
    · simpa [setName] using (by cases (some (Name.bound st.next) : Option Name) <;> simpa [setName] using h)
    · split
      · simpa [update] using h
      · exact h
    · rfl
  cases e with
  | keep n => exact hk n
  | keepTo d n =>
      simp only [step]
      split
      · assumption
      · rename_i hs
        simp [hk n] at hs
  | alias d n =>
      simp only [step]
      split
      · rfl
      · rw [setName_refused]; exact h
  | drop n =>
      simp only [step]
      split
      · split
        · simpa [update] using h
        · exact h
      · simpa using h
      · rfl
  | use n => simp only [step]; split <;> simp_all [refuse]
  | handoff n =>
      simp only [step]
      split
      · rfl
      · split
        · simpa [update] using h
        · rfl
      · exact h
  | counter n => rfl
  | refuse => rfl
  | withdrawn n => rfl

theorem foldl_keeps_refused (st : State) (es : List Event) (h : st.refused = true) :
    (es.foldl step st).refused = true := by
  induction es generalizing st with
  | nil => simpa using h
  | cons e es ih => exact ih _ (step_keeps_refused st e h)

/-- A path with a refusal anywhere is never safe. -/
theorem a_refused_path_is_never_safe (pre post : List Event) :
    outcome ((pre ++ .refuse :: post).foldl step State.empty) = .refused := by
  have hr : (step (pre.foldl step State.empty) .refuse).refused = true := rfl
  have := foldl_keeps_refused _ post hr
  rw [List.foldl_append, List.foldl_cons]
  simp [outcome, this]

/-! ## The primitive and the neutral reading -/

inductive Clause where
  | kept
  | dropped
  deriving DecidableEq, Repr

/-- A declaration as the route reads it: its reference clause, if any. -/
structure Decl where
  reference : Option Clause
  deriving DecidableEq, Repr

/-- How a call to `callee` on name `n` becomes an event: a keep or a drop only
through a declared reference clause; a use through any other declaration; an
undeclared callee (the atomic primitive) is a refused call. -/
def readCall (decls : String → Option Decl) (callee : String) (n : Nat) : Event :=
  match decls callee with
  | some ⟨some .kept⟩ => .keep n
  | some ⟨some .dropped⟩ => .drop n
  | some ⟨none⟩ => .use n
  | none => .counter n

/-- **A statistics counter is never read as a reference**: a keep or a drop
is read only through a declared reference clause, never through the name of
the primitive it may expand to. -/
theorem a_statistics_counter_is_never_read_as_a_reference
    (decls : String → Option Decl) (callee : String) (n m : Nat) :
    (readCall decls callee n = .keep m → decls callee = some ⟨some .kept⟩) ∧
      (readCall decls callee n = .drop m → decls callee = some ⟨some .dropped⟩) := by
  unfold readCall
  constructor <;> intro h <;> split at h <;> simp_all

/-- The undeclared primitive refuses and changes no family. -/
theorem a_counter_changes_no_family (st : State) (n : Nat) :
    (step st (.counter n)).families = st.families ∧ (step st (.counter n)).refused = true :=
  ⟨rfl, rfl⟩

/-- The custody-contract table a consumer that knows nothing of references
reads: a declaration with a reference clause is withheld. -/
def functionMapEntry (d : Decl) : Option Decl :=
  if d.reference.isSome then none else some d

/-- The neutral reading: keep and drop as plain uses. -/
def neutral : Event → Event
  | .keep n => .use n
  | .keepTo _ n => .use n
  | .drop n => .use n
  | e => e

/-- **The neutral reading is not admitted**: a reference declaration is
withheld from the function map; the neutral reading would certify a missing
drop that the route reports as a leak; and a declaration rule 13c withdraws
keeps refusing instead of falling back to it. -/
theorem the_neutral_reading_is_not_admitted (d : Decl) (c : Clause) (st : State) (n : Nat)
    (hd : d.reference = some c) :
    functionMapEntry d = none ∧
      outcome (run ([Event.keep 0].map neutral)) = .safe ∧
      outcome (run [Event.keep 0]) = .leak ∧
      (step st (.withdrawn n)).refused = true := by
  refine ⟨by simp [functionMapEntry, hd], by decide, by decide, rfl⟩

/-! ## The last drop -/

theorem nameOf_map_invalidate (st : State) (f b : Nat)
    (hb : nameOf st b = some (.bound f)) :
    nameOf { st with names := st.names.map fun e => if e.2 == .bound f then (e.1, .invalid) else e,
                     families := st.families.filter (fun e => e.1 != f) } b = some .invalid := by
  unfold nameOf at hb ⊢
  simp only [List.find?_map]
  have hkey : ((fun e : Nat × Name => e.1 == b) ∘
      fun e : Nat × Name => if e.2 == .bound f then (e.1, Name.invalid) else e) =
      fun e : Nat × Name => e.1 == b := by
    funext e
    simp only [Function.comp]
    split <;> rfl
  rw [hkey]
  cases hfind : st.names.find? (fun e => e.1 == b) with
  | none => simp [hfind] at hb
  | some e =>
      simp [hfind] at hb
      simp [hb]

/-- **An alias used after a possibly-final drop is refused**: two names bound
to a family the function holds once; a drop through one invalidates the
other, and its use refuses. -/
theorem an_alias_use_after_a_possibly_final_drop_is_refused (st : State) (f a b : Nat)
    (ha : nameOf st a = some (.bound f)) (hb : nameOf st b = some (.bound f))
    (hf : familyOf st f = some (1, false)) :
    (step (step st (.drop a)) (.use b)).refused = true := by
  have hbind : bindingOf st a = .family (.known 1) false := by
    simp [bindingOf, ha, hf]
  have hdrop : step st (.drop a) =
      { st with families := st.families.filter (fun e => e.1 != f),
                names := st.names.map fun e => if e.2 == .bound f then (e.1, .invalid) else e } := by
    simp [step, hbind, decideDrop, ha]
  rw [hdrop]
  have hinv := nameOf_map_invalidate st f b hb
  simp only [step]
  rw [hinv]
  simp [refuse]

/-! ## Shared vectors -/

/-- `REFERENCE_VECTORS` of `reference_count.rs`, row by row. -/
def referenceVectors : List (Binding × KeepDecision × DropDecision) := [
  (.unbound, .mint, (.refuse .unheld)),
  ((.family (.known 1) false), .add, .final),
  ((.family (.known 2) false), .add, .decrement),
  ((.family (.known 1) true), (.refuse .undecided), (.refuse .unheld)),
  ((.family .undecided false), (.refuse .undecided), (.refuse .undecided)),
  ((.family (.known 0) false), .add, (.refuse .unheld)),
  (.invalid, (.refuse .dropped), (.refuse .dropped)),
  (.other, (.refuse .undecided), (.refuse .unheld))
]

theorem reference_vectors_hold :
    referenceVectors.all (fun (b, k, d) => decideKeep b == k && decideDrop b == d) = true := by
  decide

/-- `REFERENCE_TRACE_VECTORS`: each path and its outcome. -/
def referenceTraceVectors : List (List Event × Outcome) := [
  ([.keep 0, .drop 0], .safe),
  ([.keep 0], .leak),
  ([.keep 0, .refuse], .refused),
  ([.keep 0, .drop 0, .drop 0], .refused),
  ([.keepTo 1 0, .handoff 1], .safe),
  ([.keepTo 1 0, .alias 2 1, .handoff 2], .safe),
  ([.keep 0, .keep 0, .drop 0], .leak),
  ([.keep 0, .keep 0, .drop 0, .use 0, .drop 0], .safe),
  ([.keep 0, .alias 1 0, .drop 0, .use 1], .refused),
  ([.keep 0, .drop 0, .use 0], .refused),
  ([.drop 0], .refused),
  ([.use 0], .safe),
  ([.keep 0, .keep 0, .handoff 0], .leak),
  ([.keep 0, .handoff 0, .drop 0], .refused),
  ([.keep 0, .counter 0, .drop 0], .refused),
  ([.withdrawn 0], .refused)
]

theorem reference_trace_vectors_hold :
    referenceTraceVectors.all (fun (es, o) => outcome (run es) == o) = true := by
  decide

end QCore.ReferenceCount
