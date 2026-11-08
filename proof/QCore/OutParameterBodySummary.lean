/-!
# Out-parameter summaries read off the body (card OUT-PARAMETER-BODY-SUMMARY)

The abstract decisions implemented by
`crates/qcore-c-gateway/src/legacy_unwritten_class.rs` (rule 11r(6)), the
generalised W4 gate `observations_match_class` of `legacy_summary.rs`, the
per-class row of `legacy_relational_forwarding.rs` (rule 12s(2)), and the
scalar out-slot store of `legacy_classify.rs` (rule 10k(5)).

## The unwritten class (rule 11r(6))

A body's exits are modelled by the value each returns and what the one out
slot holds there: unwritten, the null constant, a fresh live site, or anything
else (a released, escaped, possibly-null, shared or ambiguous value). A class
is the set of exits returning one value. Its mode is read off its exits
(`classMode`): **unwritten** only when every exit leaves the slot unwritten;
**out** only when every exit stores null or a fresh site and at least one a
fresh site, nullable exactly when one stores null; nothing otherwise.

* The mode's effect on the caller's slot equals the body's on every exit of
  the class (`class_mode_sound`); an unwritten class keeps the caller's value
  on every exit (`unwritten_class_keeps`).
* An exit that writes the slot, or one holding any other value, refuses the
  unwritten reading (`written_exit_refuses_unwritten`,
  `other_exit_refuses`): a mixed class, a store after release and a shared
  site keep the refusal.
* Over a whole body, every exit's effect is the one the class its value
  selects admits (`summary_effect_equals_body_on_every_path`), and the
  relation keeps an unwritten class only where another class writes the slot
  (`relation_requires_a_writer`).

## The scalar store (rule 10k(5))

A scalar store through a pointer writes one scalar cell: no binding of a
pointer name changes and no site's disposition changes
(`scalar_store_preserves_custody`), and it is admitted only through a base the
path has not released (`released_base_refuses`).

## What this is not

A model of the decisions, not of C. That the analyzer's exit observations are
what the body does on each path, that `for (;;);` removes exactly the paths of
the other classes, and faithful Clang extraction of the scalar destination's
type are review premises (trust boundary 28).
-/

namespace QCore.OutParameterBodySummary

/-! ## What one out slot holds at one exit -/

inductive SlotAt where
  | unwritten
  | null
  | fresh (site : Nat)
  | other
  deriving DecidableEq, Repr

/-- The caller's slot after a call that took one exit. -/
inductive Effect where
  | keeps
  | null
  | delivers (site : Nat)
  | unknown
  deriving DecidableEq, Repr

def bodyEffect : SlotAt → Effect
  | .unwritten => .keeps
  | .null => .null
  | .fresh site => .delivers site
  | .other => .unknown

/-- The mode a class publishes for the slot. -/
inductive ClassMode where
  | out (nullable : Bool)
  | unwritten
  deriving DecidableEq, Repr

def isUnwritten : SlotAt → Bool
  | .unwritten => true
  | _ => false

def isStored : SlotAt → Bool
  | .null => true
  | .fresh _ => true
  | _ => false

def isFresh : SlotAt → Bool
  | .fresh _ => true
  | _ => false

def isNull : SlotAt → Bool
  | .null => true
  | _ => false

/-- The class reading (`out_mode` and the unwritten reading). -/
def classMode (exits : List SlotAt) : Option ClassMode :=
  if exits.isEmpty then none
  else if exits.all isUnwritten then some .unwritten
  else if exits.all isStored && exits.any isFresh then some (.out (exits.any isNull))
  else none

/-- Whether a published mode admits one exit's effect on the caller's slot. -/
def admits : ClassMode → Effect → Bool
  | .unwritten, .keeps => true
  | .out _, .delivers _ => true
  | .out nullable, .null => nullable
  | _, _ => false

theorem all_mem {p : α → Bool} {xs : List α} (h : xs.all p = true) {x : α}
    (hx : x ∈ xs) : p x = true := by
  rw [List.all_eq_true] at h
  exact h x hx

theorem any_mem {p : α → Bool} {xs : List α} {x : α} (hx : x ∈ xs) (h : p x = true) :
    xs.any p = true := by
  rw [List.any_eq_true]
  exact ⟨x, hx, h⟩

theorem class_mode_sound (exits : List SlotAt) (m : ClassMode)
    (h : classMode exits = some m) : ∀ s ∈ exits, admits m (bodyEffect s) = true := by
  intro s hs
  unfold classMode at h
  by_cases he : exits.isEmpty = true
  · simp [he] at h
  · simp only [he] at h
    by_cases hu : exits.all isUnwritten = true
    · simp only [hu, ite_true] at h
      cases h
      have := all_mem hu hs
      cases s <;> simp_all [isUnwritten, bodyEffect, admits]
    · simp only [hu] at h
      by_cases hst : (exits.all isStored && exits.any isFresh) = true
      · simp only [hst, ite_true] at h
        cases h
        have hstored : exits.all isStored = true := by
          simp only [Bool.and_eq_true] at hst; exact hst.1
        have := all_mem hstored hs
        cases s with
        | unwritten => simp [isStored] at this
        | null => simp [bodyEffect, admits, any_mem hs (by rfl : isNull SlotAt.null = true)]
        | fresh site => simp [bodyEffect, admits]
        | other => simp [isStored] at this
      · simp [hst] at h

theorem unwritten_class_keeps (exits : List SlotAt)
    (h : classMode exits = some .unwritten) : ∀ s ∈ exits, bodyEffect s = .keeps := by
  intro s hs
  have := class_mode_sound exits .unwritten h s hs
  cases s <;> simp_all [bodyEffect, admits]

theorem written_exit_refuses_unwritten (exits : List SlotAt) (s : SlotAt) (hs : s ∈ exits)
    (hw : isUnwritten s = false) : classMode exits ≠ some .unwritten := by
  intro h
  have := unwritten_class_keeps exits h s hs
  cases s <;> simp_all [isUnwritten, bodyEffect]

theorem other_exit_refuses (exits : List SlotAt) (hs : SlotAt.other ∈ exits) :
    classMode exits = none := by
  cases hm : classMode exits with
  | none => rfl
  | some m =>
    have := class_mode_sound exits m hm .other hs
    cases m <;> simp [bodyEffect, admits] at this

/-! ## Over a whole body -/

structure Exit where
  value : Int
  slot : SlotAt
  deriving DecidableEq, Repr

/-- The exits of one class: the per-value body of rule 11r(4). -/
def classExits (body : List Exit) (v : Int) : List SlotAt :=
  (body.filter (fun e => decide (e.value = v))).map Exit.slot

theorem exit_in_its_class (body : List Exit) (e : Exit) (he : e ∈ body) :
    e.slot ∈ classExits body e.value := by
  unfold classExits
  exact List.mem_map.mpr ⟨e, List.mem_filter.mpr ⟨he, by simp⟩, rfl⟩

/-- The relation published: every returned value's class decided. -/
def decided (body : List Exit) (modes : Int → Option ClassMode) : Prop :=
  ∀ e ∈ body, modes e.value = classMode (classExits body e.value) ∧
    (modes e.value).isSome

theorem summary_effect_equals_body_on_every_path (body : List Exit)
    (modes : Int → Option ClassMode) (h : decided body modes) :
    ∀ e ∈ body, ∃ m, modes e.value = some m ∧ admits m (bodyEffect e.slot) = true := by
  intro e he
  obtain ⟨heq, hsome⟩ := h e he
  cases hm : modes e.value with
  | none => simp [hm] at hsome
  | some m =>
    refine ⟨m, rfl, ?_⟩
    rw [hm] at heq
    exact class_mode_sound _ m heq.symm e.slot (exit_in_its_class body e he)

/-- Rule 11r(6)'s relation check: an unwritten class stands only beside a
class that writes the slot. -/
def keepsRelation (values : List Int) (modes : Int → Option ClassMode) : Bool :=
  values.all fun v =>
    modes v != some .unwritten ||
      values.any fun w => match modes w with
        | some (.out _) => true
        | _ => false

theorem relation_requires_a_writer (values : List Int) (modes : Int → Option ClassMode)
    (h : keepsRelation values modes = true) (v : Int) (hv : v ∈ values)
    (hu : modes v = some .unwritten) :
    ∃ w ∈ values, ∃ n, modes w = some (.out n) := by
  have := all_mem h hv
  simp only [hu, bne_self_eq_false, Bool.false_or] at this
  rw [List.any_eq_true] at this
  obtain ⟨w, hw, hout⟩ := this
  cases hmw : modes w with
  | none => simp [hmw] at hout
  | some m =>
    cases m with
    | out n => exact ⟨w, hw, n, hmw⟩
    | unwritten => simp [hmw] at hout

/-! ## The scalar store -/

structure State where
  /-- The site a pointer name is bound to. -/
  binding : String → Option Nat
  /-- Whether a site was released on this path. -/
  released : Nat → Bool
  /-- Scalar memory cells. -/
  cells : Nat → Int

def scalarStore (st : State) (cell : Nat) (v : Int) : State :=
  { st with cells := fun c => if c = cell then v else st.cells c }

theorem scalar_store_preserves_custody (st : State) (cell : Nat) (v : Int) :
    (scalarStore st cell v).binding = st.binding ∧
      (scalarStore st cell v).released = st.released := ⟨rfl, rfl⟩

/-- The dereference obligation of the store's base: unbound, or bound to a
site this path has not released. -/
def baseAdmitted (st : State) (base : String) : Bool :=
  match st.binding base with
  | none => true
  | some site => !st.released site

theorem released_base_refuses (st : State) (base : String) (site : Nat)
    (hb : st.binding base = some site) (hr : st.released site = true) :
    baseAdmitted st base = false := by
  simp [baseAdmitted, hb, hr]

end QCore.OutParameterBodySummary
