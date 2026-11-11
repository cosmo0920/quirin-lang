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

import QCore.ElaborationCore

/-!
# Nested and sibling call effects (card CG05-NESTED-CALL-EFFECTS, rule 10y)

The abstract rule implemented by `crates/qcore-c-gateway/src/nested_call_effects.rs`
(the call tree and its footprints), `elaboration.rs` (`Cell`, `cells_overlap`,
`footprints_commute`, `EvalGraph::check_footprints`) and the route-G adapter
`legacy_nested_call.rs`.

Stated here:

* **Cells and overlap.** A footprint cell is a named object's storage, the
  identity a pointer-valued name denotes, or the fresh identity of an owned
  result. `overlap` is the Rust
  `cells_overlap`, bound by `overlapVectors`. It is *sound* for any denotation
  meeting the stated premises (`overlap_sound`): distinct declarations are
  distinct objects, a fresh identity is no other cell's, and an automatic
  object whose address is never taken is reached by no identity. What a call
  does beyond its contract is not modelled: its effect is its contract's
  (constraint 4's trust), as for a call standing alone.
* **The footprint commutation theorem** (`footprint_commute_sound`): two
  effects that respect their footprints and whose footprints commute (no cell
  one writes overlaps a cell the other reads or writes) commute on every
  state.
* **Every interleaving** (`interleaving_runs_as_append`): when every effect of
  one operand commutes with every effect of the other, every interleaving C
  permits runs as the two in sequence.
* **Inside-out, at any depth** (`every_permitted_order_runs_alike`, by
  induction over the call tree): a call's actuals, nested calls included,
  precede its entry (`inner_effects_precede_the_outer_entry`); the checked
  tree reaches one state in every permitted order, and that state is the
  lowering order's (`post`, `post_is_permitted`).
* **Refusals.** A non-commuting pair refuses the tree
  (`a_non_commuting_pair_is_refused`), the refusal is necessary
  (`overlapping_writes_do_not_commute`), the card's two refusal controls are
  refused (`a_release_and_a_borrow_of_one_identity_are_refused`,
  `a_read_unordered_with_its_release_is_refused`), and an unlisted form is
  refused (`only_listed_forms_are_admitted`).

What this is not: a model of Clang, of the contract clauses (the flow's
`apply_contract_call` applies them, unchanged), or a proof that two names are
distinct identities: two identity cells always overlap. The Rust-Lean
correspondence is bound by `overlapVectors` and `orderVectors`
(`tests/unit/nested_call_effects.rs`); faithful Clang extraction is a review
premise (trust boundary 28).
-/

namespace QCore.NestedCallEffects

open QCore.ElaborationCore (Interleave)

/-! ## Cells and overlap -/

inductive Cell where
  | object (d : Nat)
  | identity (n : Nat)
  | fresh (k : Nat)
  deriving DecidableEq, Repr

/-- `cells_overlap`, with `u d` saying declaration `d` is an automatic object
whose address is never taken. -/
def overlap (u : Nat → Bool) : Cell → Cell → Bool
  | .object a, .object b => a == b
  | .fresh a, .fresh b => a == b
  | .fresh _, _ => false
  | _, .fresh _ => false
  | .identity _, .identity _ => true
  | .object d, .identity _ => !u d
  | .identity _, .object d => !u d

theorem overlap_symm (u : Nat → Bool) (a b : Cell) : overlap u a b = overlap u b a := by
  cases a <;> cases b <;> simp only [overlap] <;>
    first | rfl | (apply Bool.eq_iff_iff.mpr; simp only [beq_iff_eq]; exact ⟨Eq.symm, Eq.symm⟩)

/-- Declaration `0` is unaliased, declaration `1` has its address taken. -/
def u0 (d : Nat) : Bool := d == 0

/-- `OVERLAP_VECTORS` of `nested_call_effects.rs`, row by row. -/
def overlapVectors : List (Cell × Cell × Bool) := [
  (.object 0, .object 0, true),
  (.object 0, .object 1, false),
  (.identity 0, .identity 1, true),
  (.identity 0, .identity 0, true),
  (.fresh 0, .fresh 0, true),
  (.fresh 0, .fresh 1, false),
  (.fresh 0, .identity 0, false),
  (.fresh 0, .object 1, false),
  (.object 0, .identity 0, false),
  (.object 1, .identity 0, true)
]

theorem overlap_vectors_hold :
    overlapVectors.all (fun (a, b, r) => overlap u0 a b == r && overlap u0 b a == r) = true := by
  decide

/-- A denotation: the abstract locations each cell may name. -/
abbrev Den (L : Type) := Cell → L → Prop

/-- `overlap` is sound for a denotation when two cells naming one location
overlap. -/
def Sound {L : Type} (u : Nat → Bool) (ρ : Den L) : Prop :=
  ∀ a b l, ρ a l → ρ b l → overlap u a b = true

/-- The premises the Rust reading rests on (stated, not proved): distinct
declarations are distinct objects; a fresh identity is no other cell's
location (it is a site no earlier state holds); an automatic object whose
address is never taken is reached by no identity. -/
structure Premises {L : Type} (u : Nat → Bool) (ρ : Den L) : Prop where
  objects : ∀ a b l, ρ (.object a) l → ρ (.object b) l → a = b
  fresh : ∀ k c l, ρ (.fresh k) l → ρ c l → c = .fresh k
  unaliasedIdentity : ∀ d n l, u d = true → ρ (.object d) l → ¬ ρ (.identity n) l

theorem overlap_sound {L : Type} (u : Nat → Bool) (ρ : Den L) (P : Premises u ρ) :
    Sound u ρ := by
  intro a b l ha hb
  cases a with
  | object x =>
    cases b with
    | object y => simp [overlap, P.objects x y l ha hb]
    | identity n =>
      cases hu : u x
      · simp [overlap, hu]
      · exact absurd hb (P.unaliasedIdentity x n l hu ha)
    | fresh k => exact absurd (P.fresh k _ l hb ha) (by simp)
  | identity n =>
    cases b with
    | object y =>
      cases hu : u y
      · simp [overlap, hu]
      · exact absurd ha (P.unaliasedIdentity y n l hu hb)
    | identity m => simp [overlap]
    | fresh k => exact absurd (P.fresh k _ l hb ha) (by simp)
  | fresh k =>
    have := P.fresh k b l ha hb
    subst this
    simp [overlap]

/-! ## Footprints and the commutation theorem -/

structure Footprint where
  reads : List Cell
  writes : List Cell

def touches (f : Footprint) : List Cell := f.reads ++ f.writes

/-- One footprint writes a cell the other touches. -/
def clash (u : Nat → Bool) (f g : Footprint) : Bool :=
  f.writes.any fun w => (touches g).any (overlap u w)

/-- `footprints_commute`. -/
def fpCommute (u : Nat → Bool) (f g : Footprint) : Bool := !clash u f g && !clash u g f

def inCells {L : Type} (ρ : Den L) (cs : List Cell) (l : L) : Prop := ∃ c ∈ cs, ρ c l

/-- An effect respects its footprint: it changes only what its writes name,
and what it writes there depends only on what its reads and writes name. -/
structure Respects {L V : Type} (ρ : Den L) (fp : Footprint) (f : (L → V) → (L → V)) :
    Prop where
  frame : ∀ s l, ¬ inCells ρ fp.writes l → f s l = s l
  local_ : ∀ s t, (∀ l, inCells ρ (touches fp) l → s l = t l) →
    ∀ l, inCells ρ fp.writes l → f s l = f t l

theorem no_clash {u : Nat → Bool} {f g : Footprint} (h : clash u f g = false) :
    ∀ w ∈ f.writes, ∀ c ∈ touches g, overlap u w c = false := by
  intro w hw c hc
  unfold clash at h
  rw [List.any_eq_false] at h
  have hw' := h w hw
  rw [List.any_eq_true] at hw'
  cases hov : overlap u w c
  · rfl
  · exact absurd ⟨c, hc, hov⟩ hw'

theorem disjoint_of_no_clash {L : Type} {u : Nat → Bool} {ρ : Den L} (hρ : Sound u ρ)
    {f g : Footprint} (h : clash u f g = false) :
    ∀ l, inCells ρ f.writes l → ¬ inCells ρ (touches g) l := by
  rintro l ⟨w, hw, hwl⟩ ⟨c, hc, hcl⟩
  have hov := hρ w c l hwl hcl
  rw [no_clash h w hw c hc] at hov
  cases hov

theorem writes_touched {L : Type} {ρ : Den L} {f : Footprint} {l : L}
    (h : inCells ρ f.writes l) : inCells ρ (touches f) l := by
  obtain ⟨c, hc, hl⟩ := h
  exact ⟨c, List.mem_append_right _ hc, hl⟩

/-- **The footprint commutation theorem.** Two effects whose footprints
commute, under a sound overlap, commute on every state. -/
theorem footprint_commute_sound {L V : Type} (u : Nat → Bool) (ρ : Den L) (hρ : Sound u ρ)
    (F G : Footprint) (f g : (L → V) → (L → V)) (hf : Respects ρ F f) (hg : Respects ρ G g)
    (h : fpCommute u F G = true) (s : L → V) : f (g s) = g (f s) := by
  have hFG : clash u F G = false := by
    unfold fpCommute at h
    cases hc : clash u F G <;> simp_all
  have hGF : clash u G F = false := by
    unfold fpCommute at h
    cases hc : clash u G F <;> simp_all
  have dFG := disjoint_of_no_clash hρ hFG
  have dGF := disjoint_of_no_clash hρ hGF
  -- `g` leaves every location `f` touches alone, and conversely.
  have gKeeps : ∀ l, inCells ρ (touches F) l → g s l = s l := fun l hl =>
    hg.frame s l (fun hw => dGF l hw hl)
  have fKeeps : ∀ l, inCells ρ (touches G) l → f s l = s l := fun l hl =>
    hf.frame s l (fun hw => dFG l hw hl)
  funext l
  by_cases h₁ : inCells ρ F.writes l
  · have hnG : ¬ inCells ρ G.writes l := fun hw => dFG l h₁ (writes_touched hw)
    rw [hg.frame (f s) l hnG]
    exact hf.local_ (g s) s gKeeps l h₁
  · by_cases h₂ : inCells ρ G.writes l
    · rw [hf.frame (g s) l h₁]
      exact (hg.local_ (f s) s fKeeps l h₂).symm
    · rw [hf.frame (g s) l h₁, hg.frame s l h₂, hg.frame (f s) l h₂, hf.frame s l h₁]

/-- A one-location denotation and a footprint writing its one identity cell. -/
def oneCell : Den Unit := fun c _ => c = .identity 0

def writeOne : Footprint := ⟨[], [.identity 0]⟩

/-- The effect storing `v` in the one location. -/
def store (v : Int) : (Unit → Int) → (Unit → Int) := fun _ _ => v

theorem store_respects (v : Int) : Respects oneCell writeOne (store v) :=
  { frame := by
      intro s l h
      exact absurd ⟨.identity 0, by simp [writeOne], rfl⟩ h
    local_ := by
      intro s t _ l _
      rfl }

/-- The refusal is necessary: two effects that respect one footprint writing
one identity cell, storing different values, do not commute. -/
theorem overlapping_writes_do_not_commute (v w : Int) (hv : v ≠ w) :
    store v (store w (fun _ => 0)) ≠ store w (store v (fun _ => 0)) := by
  intro h
  exact hv (congrFun h ())

/-- Example B's refusal controls. `f(free_and_status(p), use(p))`: a release
and a borrow of one name are refused. -/
theorem a_release_and_a_borrow_of_one_identity_are_refused (u : Nat → Bool) (p : Nat) :
    fpCommute u ⟨[], [.identity p]⟩ ⟨[.identity p], []⟩ = false := by
  simp [fpCommute, clash, touches, overlap]

/-- `h(p, g(p))` with `g` releasing `p`: the read of `p` (its storage and its
identity) is unordered with the release, and refused. -/
theorem a_read_unordered_with_its_release_is_refused (u : Nat → Bool) (d p : Nat) :
    fpCommute u ⟨[.object d, .identity p], []⟩ ⟨[], [.identity p]⟩ = false := by
  simp [fpCommute, clash, touches, overlap]

/-- Two releases of names that may alias are refused. -/
theorem two_releases_are_refused (u : Nat → Bool) (p q : Nat) :
    fpCommute u ⟨[], [.identity p]⟩ ⟨[], [.identity q]⟩ = false := by
  simp [fpCommute, clash, touches, overlap]

/-- Two calls producing fresh identities from borrowed reads commute. -/
theorem fresh_results_from_borrowed_reads_commute (u : Nat → Bool) (p : Nat) (j k : Nat)
    (hjk : j ≠ k) :
    fpCommute u ⟨[.identity p], [.fresh j]⟩ ⟨[.identity p], [.fresh k]⟩ =
      true := by
  simp [fpCommute, clash, touches, overlap, hjk, Ne.symm hjk]

/-! ## Every interleaving -/

def run {σ : Type} : List (σ → σ) → σ → σ
  | [], s => s
  | f :: fs, s => run fs (f s)

theorem run_append {σ : Type} (a b : List (σ → σ)) (s : σ) : run (a ++ b) s = run b (run a s) := by
  induction a generalizing s with
  | nil => rfl
  | cons x a ih => exact ih (x s)

theorem step_through {σ : Type} (y : σ → σ) (a : List (σ → σ))
    (h : ∀ x ∈ a, ∀ s, x (y s) = y (x s)) (s : σ) : run a (y s) = y (run a s) := by
  induction a generalizing s with
  | nil => rfl
  | cons x a ih =>
    show run a (x (y s)) = y (run a (x s))
    rw [h x (List.mem_cons_self ..) s]
    exact ih (fun x' hx => h x' (List.mem_cons_of_mem _ hx)) (x s)

/-- Every interleaving C permits of two operands whose effects pairwise commute
runs as the two in sequence. -/
theorem interleaving_runs_as_append {σ : Type} (a b m : List (σ → σ))
    (hc : ∀ x ∈ a, ∀ y ∈ b, ∀ s, x (y s) = y (x s)) (hi : Interleave a b m) (s : σ) :
    run m s = run (a ++ b) s := by
  induction hi generalizing s with
  | nil => rfl
  | left x _ ih =>
    exact ih (fun x' hx y hy => hc x' (List.mem_cons_of_mem _ hx) y hy) (x s)
  | @right a b m y _ ih =>
    show run m (y s) = run (a ++ y :: b) s
    rw [ih (fun x hx y' hy => hc x hx y' (List.mem_cons_of_mem _ hy)) (y s),
      run_append, run_append,
      step_through y a (fun x hx => hc x hx y (List.mem_cons_self ..)) s]
    rfl

theorem interleave_mem {α : Type} {a b m : List α} (hi : Interleave a b m) :
    ∀ x ∈ m, x ∈ a ∨ x ∈ b := by
  induction hi with
  | nil => intro x hx; cases hx
  | left y _ ih =>
    intro x hx
    rcases List.mem_cons.mp hx with rfl | hx
    · exact Or.inl (List.mem_cons_self ..)
    · rcases ih x hx with h | h
      · exact Or.inl (List.mem_cons_of_mem _ h)
      · exact Or.inr h
  | right y _ ih =>
    intro x hx
    rcases List.mem_cons.mp hx with rfl | hx
    · exact Or.inr (List.mem_cons_self ..)
    · rcases ih x hx with h | h
      · exact Or.inl h
      · exact Or.inr (List.mem_cons_of_mem _ h)

theorem interleave_append {α : Type} (a b : List α) : Interleave a b (a ++ b) := by
  induction a with
  | nil =>
    induction b with
    | nil => exact .nil
    | cons y b ih => exact .right y ih
  | cons x a ih => exact .left x ih

/-! ## Call trees: inside-out at any depth -/

/-- A nested-call term over events `E`: an operand with no call (its events),
a call (its actuals, then its own event), or two operands C leaves
unsequenced (a call's actual list is a right-nested `pair`). -/
inductive Tm (E : Type) where
  | leaf (es : List E)
  | call (e : E) (args : Tm E)
  | pair (a b : Tm E)

/-- The lowering order: inside-out, siblings in sequence. -/
def post {E : Type} : Tm E → List E
  | .leaf es => es
  | .call e args => post args ++ [e]
  | .pair a b => post a ++ post b

/-- The orders C permits: a call's actuals, nested calls included, complete
before its entry; two unsequenced operands interleave. -/
inductive Permitted {E : Type} : Tm E → List E → Prop where
  | leaf (es : List E) : Permitted (.leaf es) es
  | call (e : E) (args : Tm E) (o : List E) : Permitted args o → Permitted (.call e args) (o ++ [e])
  | pair (a b : Tm E) (oa ob m : List E) :
      Permitted a oa → Permitted b ob → Interleave oa ob m → Permitted (.pair a b) m

/-- Inside-out: in every permitted order of a call, the last event is the
call's own entry, and everything before it is a permitted order of its
actuals (the inner calls' effects precede the outer entry). -/
theorem inner_effects_precede_the_outer_entry {E : Type} (e : E) (args : Tm E) (o : List E)
    (h : Permitted (.call e args) o) : ∃ o', o = o' ++ [e] ∧ Permitted args o' := by
  cases h with
  | call _ _ o' h' => exact ⟨o', rfl, h'⟩

theorem post_is_permitted {E : Type} (t : Tm E) : Permitted t (post t) := by
  induction t with
  | leaf es => exact .leaf es
  | call e args ih => exact .call e args _ ih
  | pair a b iha ihb => exact .pair a b _ _ _ iha ihb (interleave_append _ _)

theorem permitted_mem {E : Type} {t : Tm E} {o : List E} (h : Permitted t o) :
    ∀ x ∈ o, x ∈ post t := by
  induction h with
  | leaf es => intro x hx; exact hx
  | call e args o _ ih =>
    intro x hx
    rcases List.mem_append.mp hx with hx | hx
    · exact List.mem_append_left _ (ih x hx)
    · exact List.mem_append_right _ hx
  | pair a b oa ob m _ _ hi iha ihb =>
    intro x hx
    rcases interleave_mem hi x hx with hx | hx
    · exact List.mem_append_left _ (iha x hx)
    · exact List.mem_append_right _ (ihb x hx)

/-- The order check over a tree: every event of one operand of a pair commutes
with every event of the other, at every depth. -/
def Admitted {σ : Type} : Tm (σ → σ) → Prop
  | .leaf _ => True
  | .call _ args => Admitted args
  | .pair a b => Admitted a ∧ Admitted b ∧ ∀ x ∈ post a, ∀ y ∈ post b, ∀ s, x (y s) = y (x s)

/-- **Depth-n composition** (by induction over the tree): an admitted tree
reaches the lowering order's state in every order C permits. -/
theorem permitted_runs_as_post {σ : Type} (t : Tm (σ → σ)) (o : List (σ → σ))
    (h : Permitted t o) (ha : Admitted t) (s : σ) : run o s = run (post t) s := by
  induction h generalizing s with
  | leaf es => rfl
  | call e args o _ ih =>
    simp only [post, run_append]
    rw [ih ha s]
  | pair a b oa ob m hpa hpb hi iha ihb =>
    obtain ⟨haa, hab, hc⟩ := ha
    rw [interleaving_runs_as_append oa ob m
      (fun x hx y hy => hc x (permitted_mem hpa x hx) y (permitted_mem hpb y hy)) hi s,
      run_append, iha haa s, ihb hab]
    simp only [post, run_append]

/-- Effects carrying the footprints the Rust check reads. -/
structure Ev (L V : Type) where
  fp : Footprint
  eff : (L → V) → (L → V)

def Tm.map {E F : Type} (g : E → F) : Tm E → Tm F
  | .leaf es => .leaf (es.map g)
  | .call e args => .call (g e) (args.map g)
  | .pair a b => .pair (a.map g) (b.map g)

theorem post_map {E F : Type} (g : E → F) (t : Tm E) : post (t.map g) = (post t).map g := by
  induction t with
  | leaf es => rfl
  | call e args ih => simp [Tm.map, post, ih]
  | pair a b iha ihb => simp [Tm.map, post, iha, ihb]

/-- `EvalGraph::check_footprints` over a tree: every pair of events of two
unsequenced operands commutes by its footprints. -/
def check {L V : Type} (u : Nat → Bool) : Tm (Ev L V) → Bool
  | .leaf _ => true
  | .call _ args => check u args
  | .pair a b =>
    check u a && check u b && (post a).all fun x => (post b).all fun y => fpCommute u x.fp y.fp

theorem check_admits {L V : Type} (u : Nat → Bool) (ρ : Den L) (hρ : Sound u ρ)
    (t : Tm (Ev L V)) (hr : ∀ x ∈ post t, Respects ρ x.fp x.eff) (hc : check u t = true) :
    Admitted (t.map Ev.eff) := by
  induction t with
  | leaf es => trivial
  | call e args ih =>
    exact ih (fun x hx => hr x (List.mem_append_left _ hx)) hc
  | pair a b iha ihb =>
    simp only [check, Bool.and_eq_true, List.all_eq_true] at hc
    obtain ⟨⟨hca, hcb⟩, hab⟩ := hc
    refine ⟨iha (fun x hx => hr x (List.mem_append_left _ hx)) hca,
      ihb (fun x hx => hr x (List.mem_append_right _ hx)) hcb, ?_⟩
    intro x hx y hy s
    rw [post_map] at hx hy
    obtain ⟨x', hx', rfl⟩ := List.mem_map.mp hx
    obtain ⟨y', hy', rfl⟩ := List.mem_map.mp hy
    exact footprint_commute_sound u ρ hρ x'.fp y'.fp x'.eff y'.eff
      (hr x' (List.mem_append_left _ hx')) (hr y' (List.mem_append_right _ hy'))
      (hab x' hx' y' hy') s

/-- **Every interleaving runs alike.** A call tree the footprint check admits,
whose effects respect their footprints under a sound overlap, reaches one
state in every order C permits: the state of the inside-out lowering. -/
theorem every_interleaving_runs_alike {L V : Type} (u : Nat → Bool) (ρ : Den L)
    (hρ : Sound u ρ) (t : Tm (Ev L V)) (hr : ∀ x ∈ post t, Respects ρ x.fp x.eff)
    (hc : check u t = true) (o : List ((L → V) → (L → V))) (ho : Permitted (t.map Ev.eff) o)
    (s : L → V) : run o s = run ((post t).map Ev.eff) s := by
  rw [← post_map]
  exact permitted_runs_as_post _ o ho (check_admits u ρ hρ t hr hc) s

/-- A non-commuting pair of unsequenced events refuses the whole tree. -/
theorem a_non_commuting_pair_is_refused {L V : Type} (u : Nat → Bool) (a b : Tm (Ev L V))
    (x y : Ev L V) (hx : x ∈ post a) (hy : y ∈ post b) (h : fpCommute u x.fp y.fp = false) :
    check u (.pair a b) = false := by
  simp only [check, Bool.and_eq_false_iff]
  right
  rw [List.all_eq_false]
  refine ⟨x, hx, ?_⟩
  rw [List.all_eq_true]
  intro hall
  rw [hall y hy] at h
  cases h

/-! ## Forms -/

/-- The forms the reading names. -/
inductive Form where
  | plainActual
  | boundCall
  | dataWithCalls
  | indirectCall
  | uncontractedCallee
  | containerEffect
  | reallocatedResult
  | ownedScalarResult
  | dataAtCustody
  | refusedData
  | arityMismatch
  | nonCommutingPair
  deriving DecidableEq, Repr

def admits : Form → Bool
  | .plainActual => true
  | .boundCall => true
  | .dataWithCalls => true
  | _ => false

/-- Only the three listed forms are read; every other form refuses. -/
theorem only_listed_forms_are_admitted (f : Form) (h : admits f = true) :
    f = .plainActual ∨ f = .boundCall ∨ f = .dataWithCalls := by
  cases f <;> simp_all [admits]

/-- `ORDER_VECTORS` of `nested_call_effects.rs`: a C body whose last statement
the Rust reader reads over the fixture contracts, and its admission. -/
def orderVectors : List (String × Bool) := [
  ("void *p = q_alloc(); cg_sink(r_release_code(p));", true),
  ("void *p = q_alloc(); q_free(r_wrap(p));", true),
  ("void *p = q_alloc(); cg_sink(r_borrow_code(r_wrap(p)));", true),
  ("void *p = q_alloc(); cg_sink(r_borrow_code(p) + 1);", true),
  ("void *p = q_alloc(); cg_sink2(r_borrow_code(r_wrap(p)), r_borrow_code(r_wrap(p)));", true),
  ("void *p = q_alloc(); cg_sink2(r_release_code(p), r_borrow_code(p));", false),
  ("void *p = q_alloc(); void *q = q_alloc(); cg_sink2(r_release_code(p), r_release_code(q));", false),
  ("void *p = q_alloc(); cg_pair(p, r_release_code(p));", false)
]

theorem order_vectors_count : orderVectors.length = 8 := by decide

end QCore.NestedCallEffects
