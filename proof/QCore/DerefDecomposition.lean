import QCore.NullDerefGrade
import QCore.IndexQuant
import QCore.BorrowLiveness

/-!
# A dereference read through the `quant` family: existence, extent, liveness (card NULL-DEREF-QTT-GROUNDING)

The user's direction (2026-10-11): `quant null-deref` is read through the
original `quant` family, and the relations between the parts are to be derived
from the proven models, verbatim: "This can be derived from the proven lean.
And it can be natural." Normative text: contract specification C09.3
("Decomposition"), language specification §L07.

A dereference `*B` or `B->f` is safe only when three parts hold. Each part is
a projection of the same dereference use, owned by an existing system:

* **(a) existence**, `quant null-deref #B`: `B` names an identity, so it is
  not NULL (§L07 grade, §L09 identity; `QCore.NullDeref`,
  `QCore.NullDerefGrade`);
* **(b) extent**, `quant checks B, Idx #i: #i == 0` in element units of `*B`:
  `B`'s extent covers the dereferenced object (§L06, C09.1–C09.2;
  `QCore.IndexExtent`, `QCore.IndexQuant`);
* **(c) liveness**, custody's `live` obligation: the identity has not been
  released (§L08 and the custody property; `QCore.BorrowLiveness`).

Nothing below is a new rule or axiom. The one modelling choice is the concrete
pointer value `Ptr`: NULL, or a position in an identity's object with the
number of elements that remain from it and that identity's custody status.
"NULL has no extent" and "NULL names no identity" are what `Ptr.null` *is*,
not premises. A bounds fact's count is *sound* for a pointer when it does not
overstate the elements that remain (`ExtentSound`), which is what a C09.1
extent claims.

The entailments, each a theorem:

1. `checks_zero_entails_null_deref`: a caller's discharged `quant checks B,
   Idx #i: #i == 0` (the existing call-side decision, `QCore.IndexQuant`, on
   the existing element access, `QCore.IndexExtent`) puts a non-NULL value at
   `B`, which `relevant` admits and `QCore.NullDeref` may read non-null.
2. `null_deref_does_not_entail_checks_zero`, `an_unknown_extent_is_never_in_bounds`:
   the converse fails for a one-past-the-end pointer and for a non-NULL pointer
   with no known extent, so `null-deref` is a separate, weaker projection and is
   not defined as `checks #i == 0`.
3. `relevant_obligates_existence`, `relevant_obligates_a_stated_part`,
   `an_unstated_part_is_never_obligated`, `zero_lower_obligates_no_part`: a
   part's caller demand is §L07 demand substitution, the grade times the part's
   per-use demand (`one` when stated, `zero` otherwise), so a lower bound of at
   least one obligates (a) and each part the callee's contract states, and a
   lower bound of 0 obligates none.
4. `existence_and_extent_do_not_discharge_liveness`,
   `a_released_identity_is_refused_although_it_exists_and_has_extent`: a
   released identity has (a) and (b) and fails (c), so no theorem can let (a)
   or (b) discharge liveness.

What this is not: a change to any analysis. No consumer uses entailment 1 to
discharge a NULL obligation (a later behaviour card,
`NULL-DEREF-FROM-INDEX-ZERO-BOUNDS`), and the composite `quant deref #B`
shorthand is not introduced (the user has not decided on it).
-/

namespace QCore.DerefDecomposition

open QCore.IndexExtent (Count IndexFact Decision elementAccess element_access_sound Describes)
open QCore.IndexQuant (Pred Rel Term Admitted admitted reading requirement bothInBounds
  both_in_bounds_sound)
open QCore.NullDerefGrade (requiresIdentity admits relevant_admits_only_an_existing_identity)

/-! ## The concrete pointer value at a dereference -/

/-- NULL, or a position in an identity's object: the elements that remain from
it (`0` at one past the end) and that identity's custody status. -/
inductive Ptr where
  | null
  | into (remaining : Nat) (status : QCore.BorrowLiveness.Status)

def Ptr.isNull : Ptr → Bool
  | .null => true
  | .into _ _ => false

/-- The elements a dereference through the pointer may reach: none through
NULL. -/
def Ptr.extent : Ptr → Nat
  | .null => 0
  | .into n _ => n

/-- Custody entitles a read of the identity the pointer names (`live` or
`contained`, `QCore.BorrowLiveness.Status.entitlesARead`); NULL names none. -/
def Ptr.live : Ptr → Bool
  | .null => false
  | .into _ st => st.entitlesARead

/-- (a) existence: the pointer names an identity. -/
abbrev existence (p : Ptr) : Prop := p.isNull = false

/-- (b) extent: the dereferenced object (element 0) is within the extent. -/
abbrev extentCovers (p : Ptr) : Prop := 1 ≤ p.extent

/-- (c) liveness: the identity has not been released. -/
abbrev liveness (p : Ptr) : Prop := p.live = true

/-- A safe dereference needs all three parts. -/
abbrev safeDeref (p : Ptr) : Prop := existence p ∧ extentCovers p ∧ liveness p

/-- A bounds fact's count is sound for the pointer when it does not overstate
the elements that remain from it (what a C09.1 extent claims). -/
abbrev ExtentSound (env : Nat → Nat) (n : Count) (p : Ptr) : Prop := n.eval env ≤ p.extent

/-- NULL has no extent: every count sound for it is zero. -/
theorem null_has_no_extent (env : Nat → Nat) (n : Count) (h : ExtentSound env n .null) :
    n.eval env = 0 := by
  simpa [ExtentSound, Ptr.extent] using h

/-! ## The clause `quant checks B, Idx #i: #i == 0` and its caller decision -/

/-- `#i == 0`. -/
def zeroCheck : Pred := .cmp .eq (.mk 0 none)

/-- Its admitted set is the bounded `[0, 0]`: the caller's obligation is the
element at index 0. -/
theorem zeroCheck_admits_index_zero (ρ : Nat → Int) :
    admitted (reading ρ zeroCheck) = .bounded 0 0 := rfl

/-- The call-side decision of `QCore.IndexQuant` for an actual at offset `o`
with extent `n`: the element accesses at the hull's two ends, joined; `none`
when the set gives no obligation. -/
def callerDecision (n : Option Count) (o : Int) (a : Admitted) : Option Decision :=
  match requirement a with
  | none => none
  | some (lo, hi) =>
      some (bothInBounds (elementAccess n (.const (o + lo))) (elementAccess n (.const (o + hi))))

/-! ## Entailment 1: `checks(B, #i == 0) ⊢ null-deref(B)` -/

/-- A discharged index-0 check puts element 0 within the count. -/
theorem zero_check_covers_element_zero (env : Nat → Nat) (ρ : Nat → Int) (n : Count)
    (h : callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) = some .inBounds) :
    0 < (n.eval env : Int) := by
  rw [zeroCheck_admits_index_zero] at h
  simp only [callerDecision, requirement, Option.some.injEq] at h
  have hb := (both_in_bounds_sound _ _ h).1
  have := element_access_sound env n _ 0 hb (by simp [Describes])
  exact this.2

/-- **Entailment 1.** A caller whose `quant checks B, Idx #i: #i == 0` is
discharged against a count sound for its actual passes a non-NULL value: one
that `relevant` admits (`QCore.NullDerefGrade`) and that `QCore.NullDeref` may
soundly read non-null. NULL has no extent, so a bounds proof of index 0
discharges the NULL obligation. -/
theorem checks_zero_entails_null_deref (env : Nat → Nat) (ρ : Nat → Int) (n : Count) (p : Ptr)
    (hs : ExtentSound env n p)
    (h : callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) = some .inBounds) :
    existence p ∧ admits Usage.relevant (!p.isNull) = true ∧
      ∀ ps, QCore.NullDeref.sound (.nonNull ps) p.isNull := by
  have hpos := zero_check_covers_element_zero env ρ n h
  have hne : p.isNull = false := by
    cases p with
    | null =>
        have := null_has_no_extent env n hs
        omega
    | into _ _ => rfl
  refine ⟨hne, ?_, fun _ => hne⟩
  rw [relevant_admits_only_an_existing_identity, hne]
  rfl

/-- Entailment 1 also gives (b): the extent covers the dereferenced object. -/
theorem checks_zero_entails_extent (env : Nat → Nat) (ρ : Nat → Int) (n : Count) (p : Ptr)
    (hs : ExtentSound env n p)
    (h : callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) = some .inBounds) :
    extentCovers p := by
  have hpos := zero_check_covers_element_zero env ρ n h
  simp only [ExtentSound] at hs
  simp only [extentCovers]
  omega

/-! ## Entailment 2: the converse fails -/

/-- One past the end of a live object. -/
def onePast : Ptr := .into 0 QCore.BorrowLiveness.Status.liveOnly

/-- **The converse fails.** A one-past-the-end pointer exists (it is not NULL,
so `null-deref` holds of it), yet no count sound for it discharges the index-0
check: null-deref is the weaker projection, and must not be defined as
`checks #i == 0`. -/
theorem null_deref_does_not_entail_checks_zero :
    existence onePast ∧ ¬ extentCovers onePast ∧
      ∀ (env : Nat → Nat) (ρ : Nat → Int) (n : Count), ExtentSound env n onePast →
        callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) ≠ some .inBounds := by
  refine ⟨rfl, by simp [extentCovers, onePast, Ptr.extent], ?_⟩
  intro env ρ n hs h
  have hpos := zero_check_covers_element_zero env ρ n h
  simp only [ExtentSound, onePast, Ptr.extent] at hs
  omega

/-- **The converse fails for an unknown extent too.** A non-NULL pointer with
no known extent never discharges the index-0 check (`missingExtent`), whatever
it points to. -/
theorem an_unknown_extent_is_never_in_bounds (ρ : Nat → Int) :
    existence (.into 1 QCore.BorrowLiveness.Status.liveOnly) ∧
      callerDecision none 0 (admitted (reading ρ zeroCheck)) =
        some (.undecided .missingExtent) := by
  refine ⟨rfl, ?_⟩
  rw [zeroCheck_admits_index_zero]
  rfl

/-! ## Entailment 3: the grade decides which parts are caller obligations -/

inductive Part where
  | existence | extent | liveness
  deriving DecidableEq, Repr

/-- A part's demand per use of `B`: `one` when the callee's contract states it
(the clause `quant null-deref #B` states existence; `quant checks B, Idx #i:
#i == 0` states extent; a `Borrowed` custody position states liveness),
`zero` otherwise. -/
def partDemand (stated : Part → Bool) (part : Part) : Usage :=
  if stated part then Usage.one else Usage.zero

/-- §L07 demand substitution: a part's caller demand is the use's grade times
the part's per-use demand (`QCore.UsageContext.application`'s scaling). -/
def callerDemand (g : Usage) (stated : Part → Bool) (part : Part) : Usage :=
  Usage.mul g (partDemand stated part)

/-- The caller is obligated to a part when its caller demand requires the
identity. -/
def obligated (g : Usage) (stated : Part → Bool) (part : Part) : Bool :=
  requiresIdentity (callerDemand g stated part)

/-- The scaling is the substitution law of `QCore.UsageContext`. -/
theorem callerDemand_is_substitution (g : Usage) (stated : Part → Bool) (part : Part)
    (x : Nat) :
    QCore.UsageContext.application (fun _ => Usage.zero) (fun _ => partDemand stated part) g x =
      Usage.add Usage.zero (callerDemand g stated part) :=
  QCore.UsageContext.application_substitution _ _ g x

private theorem obligated_iff (g : Usage) (stated : Part → Bool) (part : Part) :
    obligated g stated part = (requiresIdentity g && stated part) := by
  unfold obligated callerDemand partDemand requiresIdentity
  cases stated part
  · simp [Usage.mul, Usage.zero]
  · simp [Usage.mul, Usage.one]

/-- **(a) is a caller obligation under `relevant`**: the null-deref clause
states existence, and a grade with lower bound at least one obligates it. -/
theorem relevant_obligates_existence (g : Usage) (stated : Part → Bool)
    (hg : requiresIdentity g = true) (hc : stated .existence = true) :
    obligated g stated .existence = true := by
  rw [obligated_iff, hg, hc]; rfl

/-- **(b) and (c) are caller obligations wherever the contract states them.** -/
theorem relevant_obligates_a_stated_part (g : Usage) (stated : Part → Bool) (part : Part)
    (hg : requiresIdentity g = true) (hc : stated part = true) :
    obligated g stated part = true := by
  rw [obligated_iff, hg, hc]; rfl

/-- A part the contract does not state is never obligated, whatever the grade. -/
theorem an_unstated_part_is_never_obligated (g : Usage) (stated : Part → Bool) (part : Part)
    (hc : stated part = false) : obligated g stated part = false := by
  rw [obligated_iff, hc]; simp

/-- **With lower bound 0 (`many`) no part is a caller obligation.** -/
theorem zero_lower_obligates_no_part (g : Usage) (stated : Part → Bool) (hg : g.lower = 0) :
    ∀ part, obligated g stated part = false := by
  intro part
  rw [obligated_iff]
  simp [requiresIdentity, hg]

/-- The body's grade decides it: a body that dereferences `B` first on every
terminating path (the walk's inference) obligates every stated part; a body
with one path that does not obligates none. -/
theorem body_grade_decides_the_obligations (paths : List (List QCore.NullDeref.Event))
    (g : Usage) (hg : QCore.NullDerefGrade.bodyGrade paths = some g) (stated : Part → Bool) :
    (QCore.NullDeref.inferred (paths.map QCore.NullDerefGrade.firstOf) = true →
        ∀ part, obligated g stated part = stated part) ∧
      (QCore.NullDeref.inferred (paths.map QCore.NullDerefGrade.firstOf) = false →
        ∀ part, obligated g stated part = false) := by
  have hv := QCore.NullDerefGrade.bodyGrade_valid paths g hg
  constructor
  · intro hi part
    obtain ⟨g', hg', ha⟩ := (QCore.NullDerefGrade.relevant_accepts_body_iff_inferred paths).mpr hi
    rw [hg] at hg'; cases hg'
    have hl := (QCore.NullDerefGrade.relevant_accepts_iff g hv).mp ha
    rw [obligated_iff, (QCore.NullDerefGrade.requiresIdentity_iff g).mpr hl]
    simp
  · intro hn part
    have hz : g.lower = 0 := by
      have hna : ¬ (1 ≤ g.lower) := by
        intro hl
        have := (QCore.NullDerefGrade.relevant_accepts_body_iff_inferred paths).mp
          ⟨g, hg, (QCore.NullDerefGrade.relevant_accepts_iff g hv).mpr hl⟩
        rw [hn] at this; cases this
      omega
    exact zero_lower_obligates_no_part g stated hz part

/-! ## Entailment 4: (a) and (b) never discharge (c) -/

/-- A position at the start of a one-element object whose identity was
released on every reaching path (a dangling pointer). -/
def dangling : Ptr := .into 1 QCore.BorrowLiveness.Status.releasedOnly

/-- **Independence of liveness.** The dangling pointer has (a) and (b), and a
sound count discharges its index-0 check, yet it fails (c): no theorem can let
existence or extent discharge liveness. -/
theorem existence_and_extent_do_not_discharge_liveness :
    existence dangling ∧ extentCovers dangling ∧ ¬ liveness dangling ∧
      ExtentSound (fun _ => 0) (.const 1) dangling ∧
      callerDecision (some (.const 1)) 0 (admitted (reading (fun _ => 0) zeroCheck)) =
        some .inBounds := by
  refine ⟨rfl, by decide, by decide, by decide, ?_⟩
  rw [zeroCheck_admits_index_zero]
  rfl

/-- The custody rule refuses that identity (`QCore.BorrowLiveness`): (c) is
decided by custody alone, although (a) and (b) hold. -/
theorem a_released_identity_is_refused_although_it_exists_and_has_extent :
    existence dangling ∧ extentCovers dangling ∧
      QCore.BorrowLiveness.liveness
        (QCore.BorrowLiveness.settledAt QCore.BorrowLiveness.Status.releasedOnly) =
        .definitelyNotLive :=
  ⟨rfl, by decide, QCore.BorrowLiveness.a_settled_released_identity_is_refused⟩

/-- The three parts are independent of each other's absence: each fails alone. -/
theorem each_part_fails_alone :
    (¬ existence .null) ∧
      (existence onePast ∧ liveness onePast ∧ ¬ extentCovers onePast) ∧
      (existence dangling ∧ extentCovers dangling ∧ ¬ liveness dangling) := by
  refine ⟨by decide, ⟨rfl, by decide, by decide⟩, ⟨rfl, by decide, by decide⟩⟩

end QCore.DerefDecomposition
