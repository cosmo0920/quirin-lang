-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

/-!
# Indexed resource collections (card W43-INDEXED-RESOURCE-COLLECTIONS)

The abstract rule implemented by `crates/qcore-c-gateway/src/indexed_collection.rs`
and the element operations of `legacy_c.rs` (`ElementStore`, `ElementRelease`,
`BranchOnElementNull`, `CountedLoopHeader`, `CountedLoopStep`).

A heap array this body acquired, whose elements hold owned resources, is a
*collection*. Its elements are described by segments `[lo, hi)` over symbolic
bounds, each in one state: unfilled, null, live (owed) or released. A bound is a
literal or `max(0, x + k)` for the current value of a scalar `x`; indices are
clamped at zero, the indices an allocator's pointer can be subscripted with.

This module models:

* the **meaning** of a segment list under a valuation of the scalars
  (`cell`), with the first containing segment deciding an index;
* the **step** `x++` (renaming `x + k` to `x + (k - 1)`) and the counted loop's
  **exit substitution** `x := bound`, both meaning-preserving;
* the three **loop invariants** — fill (`[0, i)` live), release sweep
  (`[0, j)` released, `[j, n)` live) and partial cleanup (the filled prefix
  released, the failed element null) — each preserved by one iteration, and
  their exits;
* the **bound facts** (`index < bound` on a counted loop's body edge, a known
  zero, a clamped zero) and the **vacuous** body edge;
* the **per-element decisions** (release, null test, store) and the
  **violation rules** (`double_release`, `use_after_release`, element leak);
* the **join rule**, the **fallback** of an unresolved base, and the verdicts.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. Faithful
C token/type extraction, the correspondence with the compiled Rust, and the C
facts the counted loop's recognition rests on — the index starts at zero, is
written only by its step, does not wrap before the bound (a signed index's
overflow is undefined; an unsigned one is at least as wide as an unsigned
bound), the bound is not written in the body, no `break`/`goto`/label lets
control leave or enter other than through the header, and every name read by a
bound is a slot no other lvalue designates — are review premises (trust
boundary 28), stated here as the exit equation, not proved. No bounds claim is
made: an index being inside the allocation is outside the property.
-/

namespace QCore.IndexedCollection

/-! ## Terms and their meaning -/

/-- A symbolic bound: a literal, or `max(0, x + k)` for scalar `x`. -/
inductive Term where
  | const (c : Nat)
  | var (x : Nat) (k : Int)
  deriving DecidableEq, Repr

/-- A valuation of the scalars. -/
abbrev Valuation := Nat → Int

/-- The clamped value of a bound. -/
def Term.eval (ρ : Valuation) : Term → Nat
  | .const c => c
  | .var x k => (ρ x + k).toNat

@[simp] theorem eval_const (ρ : Valuation) (c : Nat) : (Term.const c).eval ρ = c := rfl

@[simp] theorem eval_var (ρ : Valuation) (x : Nat) (k : Int) :
    (Term.var x k).eval ρ = (ρ x + k).toNat := rfl

inductive Slot where
  | unfilled
  | null
  | live
  | released
  deriving DecidableEq, Repr

structure Segment where
  lo : Term
  hi : Term
  slot : Slot
  deriving DecidableEq, Repr

/-- The state of element `n`: the first segment containing it decides, and
every index no segment contains is unfilled. -/
def cell (ρ : Valuation) : List Segment → Nat → Slot
  | [], _ => .unfilled
  | s :: rest, n => if s.lo.eval ρ ≤ n ∧ n < s.hi.eval ρ then s.slot else cell ρ rest n

/-- `ρ` with scalar `x` set to `v`. -/
def update (ρ : Valuation) (x : Nat) (v : Int) : Valuation :=
  fun y => if y = x then v else ρ y

/-! ## The step `x++` -/

def Term.step (x : Nat) : Term → Term
  | .const c => .const c
  | .var y k => if y = x then .var y (k - 1) else .var y k

def Segment.step (x : Nat) (s : Segment) : Segment :=
  { lo := s.lo.step x, hi := s.hi.step x, slot := s.slot }

theorem step_term_preserves_value (ρ : Valuation) (x : Nat) (t : Term) :
    (t.step x).eval (update ρ x (ρ x + 1)) = t.eval ρ := by
  cases t with
  | const c => rfl
  | var y k =>
    by_cases h : y = x
    · subst h
      simp [Term.step, Term.eval, update]
      congr 1
      omega
    · simp [Term.step, Term.eval, update, h]

/-- The step renames every bound so that each element keeps its state after
`x` grows by one: the ranges mean the same elements before and after. -/
theorem the_step_preserves_every_element (ρ : Valuation) (x : Nat) (R : List Segment) (n : Nat) :
    cell (update ρ x (ρ x + 1)) (R.map (Segment.step x)) n = cell ρ R n := by
  induction R with
  | nil => rfl
  | cons s rest ih =>
    simp only [List.map, cell, Segment.step, step_term_preserves_value, ih]

/-! ## The counted loop's exit substitution -/

def Term.subst (t : Term) (x : Nat) (b : Term) : Term :=
  match t with
  | .const c => .const c
  | .var y k => if y = x ∧ k = 0 then b else .var y k

def Segment.subst (x : Nat) (b : Term) (s : Segment) : Segment :=
  { lo := s.lo.subst x b, hi := s.hi.subst x b, slot := s.slot }

/-- A bound mentions `x` only as `x + 0`; the Rust refuses any other offset. -/
def Term.onlyPlain (x : Nat) : Term → Prop
  | .const _ => True
  | .var y k => y = x → k = 0

def Segment.onlyPlain (x : Nat) (s : Segment) : Prop :=
  s.lo.onlyPlain x ∧ s.hi.onlyPlain x

theorem subst_term_preserves_value (ρ : Valuation) (x : Nat) (b t : Term)
    (exit : (ρ x).toNat = b.eval ρ) (plain : t.onlyPlain x) :
    (t.subst x b).eval ρ = t.eval ρ := by
  cases t with
  | const c => rfl
  | var y k =>
    by_cases h : y = x
    · have hk : k = 0 := plain h
      subst h; subst hk
      simp only [Term.subst, and_self, ite_true]
      rw [← exit]
      simp
    · simp only [Term.subst, h, false_and, ite_false]

/-- The exit equation `x = max(0, bound)` makes the substitution exact. -/
theorem the_exit_substitution_preserves_every_element (ρ : Valuation) (x : Nat) (b : Term)
    (exit : (ρ x).toNat = b.eval ρ) (R : List Segment)
    (plain : ∀ s ∈ R, s.onlyPlain x) (n : Nat) :
    cell ρ (R.map (Segment.subst x b)) n = cell ρ R n := by
  induction R with
  | nil => rfl
  | cons s rest ih =>
    have hs := plain s (List.mem_cons_self ..)
    have hrest : ∀ t ∈ rest, t.onlyPlain x := fun t ht => plain t (List.mem_cons_of_mem _ ht)
    simp only [List.map, cell, Segment.subst,
      subst_term_preserves_value ρ x b _ exit hs.1,
      subst_term_preserves_value ρ x b _ exit hs.2, ih hrest]

/-! ## The fill loop: `[0, i)` live -/

/-- The fill invariant's form. -/
def fillForm (i : Nat) : List Segment := [⟨.const 0, .var i 0, .live⟩]

/-- After storing an acquired element at `i`: `[0, i + 1)` live. -/
def filledOne (i : Nat) : List Segment := [⟨.const 0, .var i 1, .live⟩]

theorem the_fill_form_is_the_filled_prefix (ρ : Valuation) (i n : Nat) :
    cell ρ (fillForm i) n = if n < (ρ i).toNat then .live else .unfilled := by
  simp only [fillForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var] at *)
  all_goals first | rfl | omega

/-- One fill iteration: the element at `i` becomes live, then `i` grows. The
store appends `[i, i + 1)`, which merges into `[0, i + 1)`, and the step
renames it back to the invariant's form. -/
theorem one_fill_iteration_keeps_the_invariant (ρ : Valuation) (i : Nat) (hi : 0 ≤ ρ i) (n : Nat) :
    cell (update ρ i (ρ i + 1)) ((filledOne i).map (Segment.step i)) n = cell ρ (filledOne i) n
    ∧ cell ρ (filledOne i) n
      = (if n = (ρ i).toNat then .live else cell ρ (fillForm i) n) := by
  refine ⟨the_step_preserves_every_element ρ i (filledOne i) n, ?_⟩
  simp only [filledOne, fillForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var] at *)
  all_goals first | rfl | omega

/-- The stepped form of one filled element is the invariant's form again. -/
theorem the_stepped_fill_is_the_fill_form (i : Nat) :
    (filledOne i).map (Segment.step i) = fillForm i := by
  simp [filledOne, fillForm, Segment.step, Term.step]

/-- The fill loop's exit: with `i = max(0, n)`, the filled prefix is `[0, n)`. -/
theorem the_fill_exit_is_the_whole_range (ρ : Valuation) (i nb : Nat)
    (exit : (ρ i).toNat = (Term.var nb 0).eval ρ) (n : Nat) :
    cell ρ ((fillForm i).map (Segment.subst i (.var nb 0))) n = cell ρ (fillForm i) n := by
  apply the_exit_substitution_preserves_every_element ρ i _ exit
  intro s hs
  simp [fillForm] at hs
  subst hs
  simp [Segment.onlyPlain, Term.onlyPlain]

/-! ## The release sweep: `[0, j)` released, `[j, n)` live -/

def sweepForm (j nb : Nat) : List Segment :=
  [⟨.const 0, .var j 0, .released⟩, ⟨.var j 0, .var nb 0, .live⟩]

def sweptOne (j nb : Nat) : List Segment :=
  [⟨.const 0, .var j 1, .released⟩, ⟨.var j 1, .var nb 0, .live⟩]

/-- On the body edge (`0 <= j < n`), releasing element `j` of the sweep form
gives `[0, j + 1)` released and `[j + 1, n)` live. -/
theorem one_release_iteration_releases_exactly_element_j (ρ : Valuation) (j nb : Nat)
    (hj : 0 ≤ ρ j) (body : ρ j < ρ nb) (n : Nat) :
    cell ρ (sweptOne j nb) n
      = (if n = (ρ j).toNat then .released else cell ρ (sweepForm j nb) n) := by
  simp only [sweptOne, sweepForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var] at *)
  all_goals first | rfl | omega

theorem the_stepped_sweep_is_the_sweep_form (j nb : Nat) (h : nb ≠ j) :
    (sweptOne j nb).map (Segment.step j) = sweepForm j nb := by
  simp [sweptOne, sweepForm, Segment.step, Term.step, h]

/-- The sweep's exit: with `j = max(0, n)`, nothing is live. -/
theorem the_sweep_exit_owes_nothing (ρ : Valuation) (j nb : Nat)
    (exit : (ρ j).toNat = (ρ nb).toNat) (n : Nat) :
    cell ρ (sweepForm j nb) n ≠ .live := by
  simp only [sweepForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var] at *)
  all_goals first | omega | (intro h; cases h)

/-! ## The partial cleanup: exactly the filled prefix -/

/-- The fill loop's failure edge: `[0, i)` live and element `i` null. -/
def failedFill (i : Nat) : List Segment :=
  [⟨.const 0, .var i 0, .live⟩, ⟨.var i 0, .var i 1, .null⟩]

/-- The cleanup `for (j = 0; j < i; j++) release(v[j])` in its sweep form. -/
def cleanupForm (j i : Nat) : List Segment :=
  [⟨.const 0, .var j 0, .released⟩, ⟨.var j 0, .var i 0, .live⟩, ⟨.var i 0, .var i 1, .null⟩]

/-- At the cleanup's entry (`j = 0`) its form means exactly the failed fill. -/
theorem the_cleanup_starts_from_the_failed_fill (ρ : Valuation) (j i : Nat) (hj : ρ j = 0)
    (n : Nat) : cell ρ (cleanupForm j i) n = cell ρ (failedFill i) n := by
  simp only [cleanupForm, failedFill, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var, hj] at *)
  all_goals first | rfl | omega

/-- At the cleanup's exit (`j = max(0, i)`) nothing is live: the partial
cleanup discharges exactly the filled prefix, and the failed element owes
nothing because it is null. -/
theorem the_partial_cleanup_discharges_exactly_the_prefix (ρ : Valuation) (j i : Nat)
    (exit : (ρ j).toNat = (ρ i).toNat) (n : Nat) :
    cell ρ (cleanupForm j i) n ≠ .live := by
  simp only [cleanupForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var] at *)
  all_goals first | omega | (intro h; cases h)

/-! ## Bound facts -/

/-- A counted loop's body-edge fact `x < t` (raw values, `x` never negative)
proves `x + ka <= t + kb` whenever `kt + ka - 1 <= kb` — the Rust `le_fact`. -/
theorem a_body_fact_orders_the_bounds (ρ : Valuation) (x y : Nat) (ka kt kb : Int)
    (fact : ρ x < ρ y + kt) (offsets : kt + ka - 1 ≤ kb) :
    (Term.var x ka).eval ρ ≤ (Term.var y kb).eval ρ := by
  simp only [Term.eval]
  omega

/-- A known zero evaluates a bound to its clamped offset. -/
theorem a_known_zero_evaluates_its_bounds (ρ : Valuation) (x : Nat) (k : Int) (h : ρ x = 0) :
    (Term.var x k).eval ρ = k.toNat := by
  simp [Term.eval, h]

/-- A clamped zero (`max(0, n) = 0`, `n` possibly negative) evaluates every
bound `n + k` with `k <= 0` to zero. -/
theorem a_clamped_zero_bounds_nonpositive_offsets (ρ : Valuation) (x : Nat) (k : Int)
    (h : (ρ x).toNat = 0) (hk : k ≤ 0) : (Term.var x k).eval ρ = 0 := by
  simp only [Term.eval]
  omega

/-- A counted loop that left with its index still zero ran no iteration, so
its bound is zero when clamped — the fact the Rust records on that exit. -/
theorem an_exit_at_zero_clamps_the_bound (ρ : Valuation) (i nb : Nat)
    (exit : (ρ i).toNat = (ρ nb).toNat) (h : ρ i = 0) : (ρ nb).toNat = 0 := by
  omega

/-- The vacuous body edge: an index never negative cannot be below a bound that
is zero when clamped, so no execution takes that edge. -/
theorem a_zero_bound_admits_no_iteration (ρ : Valuation) (i nb : Nat)
    (hi : 0 ≤ ρ i) (body : ρ i < ρ nb) (zero : (ρ nb).toNat = 0) : False := by
  omega

/-! ## Per-element decisions and violations -/

inductive ReleaseOutcome where
  | released
  | null
  | again
  | refused
  deriving DecidableEq, Repr

/-- The release of one element whose index provably falls in a segment. -/
def releaseOutcome : Slot → ReleaseOutcome
  | .live => .released
  | .null => .null
  | .released => .again
  | .unfilled => .refused

inductive TestOutcome where
  | refined
  | releasedRead
  | refused
  deriving DecidableEq, Repr

def testOutcome : Slot → TestOutcome
  | .live => .refined
  | .null => .refined
  | .released => .releasedRead
  | .unfilled => .refused

inductive StoreOutcome where
  | stored
  | overwrote
  | refused
  deriving DecidableEq, Repr

/-- Storing over one element. `nonnull` says whether a live element provably
holds a non-null resource. -/
def storeOutcome (nonnull : Bool) : Slot → StoreOutcome
  | .live => if nonnull then .overwrote else .refused
  | _ => .stored

theorem a_second_release_is_a_double_release (s : Slot) :
    releaseOutcome s = .again ↔ s = .released := by
  cases s <;> simp [releaseOutcome]

theorem only_a_live_element_is_released (s : Slot) :
    releaseOutcome s = .released ↔ s = .live := by
  cases s <;> simp [releaseOutcome]

theorem an_unfilled_element_is_never_released (s : Slot) :
    s = .unfilled → releaseOutcome s = .refused := by
  intro h; subst h; rfl

theorem reading_a_released_element_is_a_use_after_release (s : Slot) :
    testOutcome s = .releasedRead ↔ s = .released := by
  cases s <;> simp [testOutcome]

theorem an_unfilled_element_is_never_read (s : Slot) :
    s = .unfilled → testOutcome s = .refused := by
  intro h; subst h; rfl

theorem overwriting_a_live_element_is_a_leak (nonnull : Bool) (s : Slot) :
    storeOutcome nonnull s = .overwrote ↔ (s = .live ∧ nonnull = true) := by
  cases s <;> cases nonnull <;> simp [storeOutcome]

theorem a_possibly_null_live_element_refuses_a_store (s : Slot) :
    s = .live → storeOutcome false s = .refused := by
  intro h; subst h; rfl

/-! ## Exits -/

inductive ExitClaim where
  | discharged
  | elementLeak
  | undecided
  deriving DecidableEq, Repr

/-- What one exit claims for one collection: its array escaped exactly (the
elements go with it), or else any live element leaks; a collection whose
ranges were not established claims nothing. -/
def exitClaim (known : Bool) (escapedExactly : Bool) (someLive : Bool) : ExitClaim :=
  if !known then .undecided
  else if !someLive then .discharged
  else if escapedExactly then .discharged
  else .elementLeak

theorem freeing_the_array_with_live_elements_leaks_them :
    exitClaim true false true = .elementLeak := rfl

theorem an_escaped_array_carries_its_elements :
    exitClaim true true true = .discharged := rfl

theorem an_array_owing_nothing_is_discharged (escaped : Bool) :
    exitClaim true escaped false = .discharged := by
  cases escaped <;> rfl

theorem unknown_ranges_claim_nothing (escaped live : Bool) :
    exitClaim false escaped live = .undecided := rfl

/-- A never-filled array owes nothing. -/
theorem a_never_filled_array_owes_nothing (ρ : Valuation) (n : Nat) :
    cell ρ [] n ≠ .live := by
  simp [cell]

/-! ## The join rule -/

/-- The join keeps the more symbolic form `R` when the other side's form `L`
means the same elements under the other side's facts. `R` then describes both
sides: the left by the hypothesis, the right by itself. -/
theorem the_joined_form_describes_both_sides (ρL ρR : Valuation) (L R : List Segment)
    (collapse : ∀ n, cell ρL R n = cell ρL L n) :
    (∀ n, cell ρL R n = cell ρL L n) ∧ (∀ n, cell ρR R n = cell ρR R n) :=
  ⟨collapse, fun _ => rfl⟩

/-- The fill loop's entry (nothing stored, `i = 0`) collapses to the fill form,
which is what lets the header keep the symbolic invariant. -/
theorem the_fill_entry_collapses_to_the_fill_form (ρ : Valuation) (i : Nat) (h : ρ i = 0)
    (n : Nat) : cell ρ (fillForm i) n = cell ρ [] n := by
  simp only [fillForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var, h] at *)
  all_goals first | rfl | omega

/-- The sweep's entry (`j = 0`, everything live) collapses to the sweep form. -/
theorem the_sweep_entry_collapses_to_the_sweep_form (ρ : Valuation) (j nb : Nat) (h : ρ j = 0)
    (n : Nat) : cell ρ (sweepForm j nb) n = cell ρ [⟨.const 0, .var nb 0, .live⟩] n := by
  simp only [sweepForm, cell]
  repeat' split
  all_goals (try simp only [eval_const, eval_var, h] at *)
  all_goals first | rfl | omega

/-! ## Classification, fallback and verdicts -/

inductive Lowering where
  | admitted (instruction : Nat)
  | refused (description : Nat)
  deriving DecidableEq, Repr

/-- The element route is consulted only after the existing classifier refused a
statement; an admitted statement keeps its lowering. -/
def classify (base : Lowering) (element : Option Nat) : Lowering :=
  match base, element with
  | .refused _, some instruction => .admitted instruction
  | other, _ => other

theorem an_admitted_statement_keeps_its_lowering (instruction : Nat) (element : Option Nat) :
    classify (.admitted instruction) element = .admitted instruction := by
  cases element <;> rfl

theorem a_refusal_without_an_element_shape_is_kept (description : Nat) :
    classify (.refused description) none = .refused description := rfl

/-- An element operation whose array does not resolve to exactly one live,
non-null identity this body acquired is exactly its fallback refusal. -/
def elementNode (resolves : Bool) (fallback modeled : Nat) : Nat :=
  if resolves then modeled else fallback

theorem an_unresolved_array_is_exactly_the_fallback (fallback modeled : Nat) :
    elementNode false fallback modeled = fallback := rfl

inductive Diagnostic where
  | unknownAlias
  | unsupported
  | leak
  | doubleRelease
  | useAfterRelease
  deriving DecidableEq, Repr

inductive Status where
  | certified
  | accused
  | undecided
  deriving DecidableEq, Repr

def Diagnostic.accuses : Diagnostic → Bool
  | .leak | .doubleRelease | .useAfterRelease => true
  | _ => false

/-- `certified` is `Safe`, `accused` a provisional `Unsafe`, `undecided`
`Unknown`. -/
def status (ds : List Diagnostic) : Status :=
  if ds.any Diagnostic.accuses then .accused
  else if ds.isEmpty then .certified
  else .undecided

theorem a_refusal_is_never_safe (ds : List Diagnostic) :
    Diagnostic.unknownAlias ∈ ds → status ds ≠ .certified := by
  intro h
  have hne : ds.isEmpty = false := by
    cases ds with
    | nil => cases h
    | cons _ _ => rfl
  unfold status
  split
  · intro e; cases e
  · simp [hne]

theorem a_candidate_is_unsafe (ds : List Diagnostic) (d : Diagnostic)
    (h : d ∈ ds) (acc : d.accuses = true) : status ds = .accused := by
  unfold status
  have : ds.any Diagnostic.accuses = true := List.any_eq_true.mpr ⟨d, h, acc⟩
  simp [this]

/-! ## Owner uses -/

inductive Position where
  | borrowed
  | released
  | consumed
  | stored
  | owned
  | reallocated
  deriving DecidableEq, Repr

/-- A collection that may still owe an element is handed to a call only to be
released (element obligations then checked at the exits) or consumed (they
leave with it); every other position refuses. -/
def ownerUseAdmitted : Position → Bool
  | .released | .consumed => true
  | _ => false

theorem a_borrow_of_an_owing_collection_refuses : ownerUseAdmitted .borrowed = false := rfl
theorem a_reallocation_of_an_owing_collection_refuses :
    ownerUseAdmitted .reallocated = false := rfl
theorem releasing_or_consuming_the_array_is_admitted :
    ownerUseAdmitted .released = true ∧ ownerUseAdmitted .consumed = true := ⟨rfl, rfl⟩

/-! ## Non-vacuity -/

/-- A concrete fill of three elements, then its sweep, owes nothing. -/
example : ∀ n, cell (fun _ => 3) (sweepForm 0 1) n ≠ .live := by
  intro n
  exact the_sweep_exit_owes_nothing (fun _ => 3) 0 1 rfl n

/-- The same three elements with no sweep are all live. -/
theorem three_filled_elements_are_live :
    cell (fun _ => 3) (fillForm 0) 0 = .live ∧ cell (fun _ => 3) (fillForm 0) 2 = .live
    ∧ cell (fun _ => 3) (fillForm 0) 3 = .unfilled := by
  simp [fillForm, cell, Term.eval]

/-! ## Kinds (integration of W43-DESCRIPTOR-RESOURCE-TRACKING)

A collection's owner, its elements' acquisitions and the releases that
discharge them are all of heap kind; a stream, directory or descriptor keeps
the classifier's refusal, and so does any statement that reads a tracked
descriptor. -/

inductive Kind where
  | heap
  | descriptor
  | stream
  | directory
  deriving DecidableEq, Repr

def Kind.isHeap : Kind → Bool
  | .heap => true
  | _ => false

/-- Whether one element operation is lowered: the owner, the acquisition (or
release) and the statement's descriptor reads all qualify. -/
def elementLowered (owner operation : Kind) (readsDescriptor : Bool) : Bool :=
  owner.isHeap && operation.isHeap && !readsDescriptor

theorem only_heap_elements_are_collected (owner operation : Kind) (d : Bool) :
    elementLowered owner operation d = true ↔
      (owner = .heap ∧ operation = .heap ∧ d = false) := by
  cases owner <;> cases operation <;> cases d <;> simp [elementLowered, Kind.isHeap]

theorem a_descriptor_read_keeps_the_fallback (owner operation : Kind) (fallback modeled : Nat) :
    elementNode (elementLowered owner operation true) fallback modeled = fallback := by
  simp [elementLowered, elementNode]

theorem a_non_heap_element_keeps_the_fallback (owner : Kind) (operation : Kind)
    (h : operation ≠ .heap) (d : Bool) (fallback modeled : Nat) :
    elementNode (elementLowered owner operation d) fallback modeled = fallback := by
  cases operation <;> simp_all [elementLowered, elementNode, Kind.isHeap]

end QCore.IndexedCollection
