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

import QCore.VerdictSemigroup

/-!
# Connecting measurements and contracts (card QUIRIN-AUDIT-CONNECT)

The model of `quirin-audit measure chain` (Part A), `measure compose` (Part B)
and the declared cross-crate contract dependencies (Part C), with one
definition per term of the CX block of the C04.18.9 typed rendering
(`cxTerms`) and one theorem per formula of `qcore_schemas::connect::FORMULAS`
(`formula_<id>`; the formulas are stated as written in `formulaTable`, bound
by `crates/qcore-frontend/tests/connect_formulas.rs`).

* **Part A.** A chain is a list of steps; step `i` compares point `i` (its
  before side) with point `i + 1` (its after side). A link is joined exactly
  when every identity field agrees and the required ones are recorded, and
  the steps share a route whose inputs agree (`joined`). A row's verdict at a
  point is the after verdict of the nearest listing on its left, else the
  before verdict of the nearest listing on its right (`verdict`, the Rust
  `chain::reconcile`). `recon_sound`: when every step's record is faithful
  to its two sides' reports (a listed row states both, an unlisted row did not
  change) the reconstruction is the reports' verdict at every point, so
  `chain_net_is_last`: the net verdict of a joined chain is its last step's
  after verdict. `link_refused_of_differing_identity`: a link with a
  differing identity is refused. `a_cancelled_move_is_reported`.
* **Part B.** A function's composed verdict is the four-value fold
  (`VerdictSemigroup.fold`) of the host's verdict and every declaring
  bundle's contribution, and its reasons the union of theirs
  (`composition_is_the_join`, `composition_keeps_every_reason`,
  `composition_is_order_independent`).
* **Part C.** `Held` is the least set closed under: a hold of the
  environment, a dependency that fails a local check, a cycle, and a held
  target (`held_dependency_holds_dependents`). It reads the dependencies
  only through membership (`held_is_order_independent`), and a declaration
  that is not held reaches no held declaration and no cycle
  (`resolution_is_acyclic`).

Premises stated, not proved: a side's report is a function of its identity
(the measurement's determinism, checked by byte comparison elsewhere); the
Rust resolver's iteration reaches this least fixed point; the records the
tools read are the ones the trusted producers wrote.
-/

namespace QCore.MeasureConnect

open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-! ## Part A: chained measurements -/

/-- An identity field a link compares. -/
inductive Field where
  | tool
  | revision
  | analyzer
  | bundles
  | buildCommit
  | linkResolution
  | rulesOff
  | dependencyGraph
  deriving DecidableEq, Repr

/-- The fields a link needs recorded on both sides. -/
def Field.required : Field → Prop
  | .tool | .revision | .analyzer | .bundles => True
  | _ => False

/-- An identity value; `none` is a field a record does not carry. -/
abbrev Ident := Option Nat

/-- The unrecorded marker. -/
def unrecorded : Ident := none

structure Side where
  id : Field → Ident
  inputs : Nat → Ident

structure Step where
  tool : Ident
  before : Side
  after : Side
  compares : Nat → Prop
  statusComplete : Nat → Prop
  unmatched : Nat → Prop
  unmatchedUnits : Nat → Prop
  conflicted : Nat → Prop
  /-- What the step lists of a row: both verdicts, or nothing. -/
  recorded : Nat → Option (V × V)
  evidenceChanged : Nat → Prop
  declaredMove : Nat → Prop
  dependencyChanged : Nat → Prop

instance : Inhabited Step :=
  ⟨{ tool := none, before := ⟨fun _ => none, fun _ => none⟩,
     after := ⟨fun _ => none, fun _ => none⟩, compares := fun _ => False,
     statusComplete := fun _ => False, unmatched := fun _ => False,
     unmatchedUnits := fun _ => False, conflicted := fun _ => False,
     recorded := fun _ => none, evidenceChanged := fun _ => False,
     declaredMove := fun _ => False, dependencyChanged := fun _ => False }⟩

/-- A chain: its steps, each row's route, and the reports: a side's verdict
of a row is a function of the side's identity (tool excluded) and the inputs
of the row's route (the determinism premise). -/
structure Chain where
  steps : List Step
  rowRoute : Nat → Nat
  report : (Field → Ident) → Ident → Nat → V

namespace Chain

def step (ch : Chain) (i : Nat) : Step := ch.steps.getD i default

/-- The nearest listing of row `w` on the left of point `p`. -/
def lastBefore (ch : Chain) : Nat → Nat → Option (V × V)
  | 0, _ => none
  | p + 1, w =>
    match (ch.step p).recorded w with
    | some x => some x
    | none => lastBefore ch p w

/-- The nearest listing at or on the right of point `p`, within `k` steps. -/
def firstFromAux (ch : Chain) : Nat → Nat → Nat → Option (V × V)
  | 0, _, _ => none
  | k + 1, p, w =>
    match (ch.step p).recorded w with
    | some x => some x
    | none => firstFromAux ch k (p + 1) w

variable (c : Chain)

def n : Nat := c.steps.length

/-- `#cx_next`. -/
def next (i j : Nat) : Prop := j = i + 1 ∧ j < c.n

/-- `#cx_after_id`. -/
def afterId (i : Nat) (f : Field) : Ident :=
  if f = .tool then (c.step i).tool else (c.step i).after.id f

/-- `#cx_before_id`. -/
def beforeId (i : Nat) (f : Field) : Ident :=
  if f = .tool then (c.step i).tool else (c.step i).before.id f

/-- `#cx_required`. -/
def required (f : Field) : Prop := f.required

/-- `#cx_compares`. -/
def compares (i r : Nat) : Prop := (c.step i).compares r

/-- `#cx_after_inputs`. -/
def afterInputs (i r : Nat) : Ident := (c.step i).after.inputs r

/-- `#cx_before_inputs`. -/
def beforeInputs (i r : Nat) : Ident := (c.step i).before.inputs r

/-- `#cx_joined`: the link check of `chain::link`. -/
def joined (i j : Nat) : Prop :=
  ((c.next i j ∧ ∀ f, c.afterId i f = c.beforeId j f ∧ (f.required → c.afterId i f ≠ unrecorded))
    ∧ ∃ r, c.compares i r ∧ c.compares j r)
  ∧ ∀ r, (c.compares i r ∧ c.compares j r) →
      (c.afterInputs i r = c.beforeInputs j r ∧ c.afterInputs i r ≠ unrecorded)

/-- `#cx_before` and `#cx_after`: step `i` compares point `i` with `i + 1`. -/
def before (i : Nat) : Nat := i
def after (i : Nat) : Nat := i + 1

/-- `#cx_member`. -/
def member (i : Nat) : Prop := i < c.n

def statusComplete (i r : Nat) : Prop := (c.step i).statusComplete r
def unmatched (i r : Nat) : Prop := (c.step i).unmatched r
def unmatchedUnits (i r : Nat) : Prop := (c.step i).unmatchedUnits r
def conflicted (i r : Nat) : Prop := (c.step i).conflicted r

def firstFrom (p w : Nat) : Option (V × V) := c.firstFromAux (c.n - p) p w

/-- `#cx_verdict`: the reconstruction of `chain::reconcile`. -/
def verdict (p w : Nat) : V :=
  match c.lastBefore p w with
  | some (_, a) => a
  | none =>
    match c.firstFrom p w with
    | some (b, _) => b
    | none => .unknown

/-- `#cx_row_route`. -/
def rowRoute' (w : Nat) : Nat := c.rowRoute w

/-- `#cx_listed`. -/
def listed (i w : Nat) : Prop := (c.step i).recorded w ≠ none

/-- `#cx_disagrees`: a row of `r` the step lists whose before verdict is not
the after verdict of its nearest earlier listing. -/
def disagrees (i r : Nat) : Prop :=
  ∃ w, c.rowRoute w = r ∧ ∃ b a b' a', (c.step i).recorded w = some (b, a) ∧
    c.lastBefore i w = some (b', a') ∧ a' ≠ b

/-- `#cx_chain_complete`. -/
def chainComplete (r : Nat) : Prop :=
  ∀ i, c.member i → ((((c.compares i r ∧ c.statusComplete i r) ∧ ¬ c.unmatched i r) ∧
    (¬ c.unmatchedUnits i r ∧ ¬ c.conflicted i r)) ∧ ¬ c.disagrees i r)

/-- `#cx_first` and `#cx_last`. -/
def first : Nat := 0
def last : Nat := c.n

/-- `#cx_net_before` and `#cx_net_after`. -/
def netBefore (w : Nat) : V := c.verdict Chain.first w
def netAfter (w : Nat) : V := c.verdict c.last w

/-- Whether step `i` moves row `w`, as a decision. -/
def movedB (i w : Nat) : Bool :=
  match (c.step i).recorded w with
  | some (b, a) => decide (b ≠ a)
  | none => false

/-- `#cx_moved`. -/
def moved (i w : Nat) : Prop := c.member i ∧ c.movedB i w = true

/-- `#cx_moves`. -/
def moves (w : Nat) : Nat := ((List.range c.n).filter (fun i => c.movedB i w)).length

/-- `#cx_evidence_changed`. -/
def evidenceChanged (i w : Nat) : Prop := c.member i ∧ (c.step i).evidenceChanged w

/-- `#cx_reported`. -/
def reported (w : Nat) : Prop :=
  ∃ i, c.member i ∧ (c.movedB i w = true ∨ c.listed i w ∨ (c.step i).evidenceChanged w)

/-- `#cx_cancelled` and `#cx_cascade`: the record's patterns. -/
def cancelled (w : Nat) : Prop := c.reported w ∧ c.moves w ≥ 2 ∧ c.netBefore w = c.netAfter w
def cascade (w : Nat) : Prop := c.reported w ∧ c.moves w ≥ 2 ∧ c.netBefore w ≠ c.netAfter w

/-- `#cx_published`: the chain states the reconstruction, nothing else. -/
def published (p w : Nat) : V := c.verdict p w

/-- `#cx_new_safe`. -/
def newSafe (i w : Nat) : Prop :=
  c.moved i w ∧ c.verdict (i + 1) w = .safe ∧ c.verdict i w ≠ .safe

def declaredMove (i w : Nat) : Prop := (c.step i).declaredMove w
def unattributed (i w : Nat) : Prop := c.moved i w ∧ ¬ c.declaredMove i w
def review (i w : Nat) : Prop := c.newSafe i w ∧ ¬ c.declaredMove i w
def accepted (i w : Nat) : Prop := c.newSafe i w ∧ c.declaredMove i w
def dependencyChanged (i w : Nat) : Prop := (c.step i).dependencyChanged w
def dependencyListed (i w : Nat) : Prop := c.moved i w ∧ c.dependencyChanged i w

/-! ### Reconstruction lemmas -/

theorem lastBefore_succ_none {p w : Nat} (h : (c.step p).recorded w = none) :
    c.lastBefore (p + 1) w = c.lastBefore p w := by
  simp [lastBefore, h]

theorem firstFrom_succ_none {p w : Nat} (h : (c.step p).recorded w = none) :
    c.firstFrom p w = c.firstFrom (p + 1) w := by
  unfold firstFrom
  cases hk : c.n - p with
  | zero =>
    have : c.n - (p + 1) = 0 := by omega
    rw [this]; simp [firstFromAux]
  | succ k =>
    have : c.n - (p + 1) = k := by omega
    rw [this]; simp [firstFromAux, h]

/-- An unlisted step keeps every verdict across it. -/
theorem verdict_unlisted {p w : Nat} (h : (c.step p).recorded w = none) :
    c.verdict p w = c.verdict (p + 1) w := by
  unfold verdict
  rw [c.lastBefore_succ_none h, c.firstFrom_succ_none h]

/-- The point verdicts a faithful record states: a listed row states both
sides, an unlisted one did not change. -/
def Faithful (truth : Nat → V) (w : Nat) : Prop :=
  ∀ i, i < c.n →
    (∀ b a, (c.step i).recorded w = some (b, a) → b = truth i ∧ a = truth (i + 1)) ∧
    ((c.step i).recorded w = none → truth i = truth (i + 1))

theorem lastBefore_sound {truth : Nat → V} {w : Nat} (hf : c.Faithful truth w) :
    ∀ p, p ≤ c.n → ∀ b a, c.lastBefore p w = some (b, a) → a = truth p := by
  intro p
  induction p with
  | zero => intro _ b a h; simp [lastBefore] at h
  | succ p ih =>
    intro hp b a h
    have hlt : p < c.n := by omega
    cases hr : (c.step p).recorded w with
    | some x =>
      simp [lastBefore, hr] at h
      subst h
      exact ((hf p hlt).1 b a hr).2
    | none =>
      simp [lastBefore, hr] at h
      rw [ih (by omega) b a h]
      exact (hf p hlt).2 hr

theorem firstFromAux_sound {truth : Nat → V} {w : Nat} (hf : c.Faithful truth w) :
    ∀ k p, p + k ≤ c.n → ∀ b a, c.firstFromAux k p w = some (b, a) → b = truth p := by
  intro k
  induction k with
  | zero => intro p _ b a h; simp [firstFromAux] at h
  | succ k ih =>
    intro p hp b a h
    have hlt : p < c.n := by omega
    cases hr : (c.step p).recorded w with
    | some x =>
      simp [firstFromAux, hr] at h
      subst h
      exact ((hf p hlt).1 b a hr).1
    | none =>
      simp [firstFromAux, hr] at h
      rw [(hf p hlt).2 hr]
      exact ih (p + 1) (by omega) b a h

theorem lastBefore_none {w : Nat} :
    ∀ p, c.lastBefore p w = none → ∀ i, i < p → (c.step i).recorded w = none := by
  intro p
  induction p with
  | zero => intro _ i hi; omega
  | succ p ih =>
    intro h i hi
    cases hr : (c.step p).recorded w with
    | some x => simp [lastBefore, hr] at h
    | none =>
      simp [lastBefore, hr] at h
      by_cases hip : i = p
      · subst hip; exact hr
      · exact ih h i (by omega)

theorem firstFromAux_none {w : Nat} :
    ∀ k p, c.firstFromAux k p w = none → ∀ i, p ≤ i → i < p + k →
      (c.step i).recorded w = none := by
  intro k
  induction k with
  | zero => intro p _ i h1 h2; omega
  | succ k ih =>
    intro p h i h1 h2
    cases hr : (c.step p).recorded w with
    | some x => simp [firstFromAux, hr] at h
    | none =>
      simp [firstFromAux, hr] at h
      by_cases hip : i = p
      · subst hip; exact hr
      · exact ih (p + 1) h i (by omega) (by omega)

/-- **Soundness of the reconstruction**: when every step's record is
faithful, the chained record states the reports' verdict at every point of a
row some step lists. -/
theorem recon_sound {truth : Nat → V} {w : Nat} (hf : c.Faithful truth w)
    (listedSomewhere : ∃ i, i < c.n ∧ (c.step i).recorded w ≠ none) :
    ∀ p, p ≤ c.n → c.verdict p w = truth p := by
  intro p hp
  unfold verdict
  cases hl : c.lastBefore p w with
  | some x =>
    obtain ⟨b, a⟩ := x
    exact c.lastBefore_sound hf p hp b a hl
  | none =>
    cases hfst : c.firstFrom p w with
    | some y =>
      obtain ⟨b, a⟩ := y
      exact c.firstFromAux_sound hf (c.n - p) p (by omega) b a hfst
    | none =>
      exfalso
      obtain ⟨i, hi, hne⟩ := listedSomewhere
      by_cases hip : i < p
      · exact hne (c.lastBefore_none p hl i hip)
      · exact hne (c.firstFromAux_none (c.n - p) p hfst i (by omega) (by omega))

/-- A side's report of a row: its identity without the tool, and the inputs
of the row's route. -/
def rep (s : Side) (w : Nat) : V :=
  c.report (fun f => if f = .tool then none else s.id f) (s.inputs (c.rowRoute w)) w

/-- Every step's record is faithful to its own two sides' reports. -/
def StepsFaithful (w : Nat) : Prop :=
  ∀ i, i < c.n →
    (∀ b a, (c.step i).recorded w = some (b, a) →
      b = c.rep (c.step i).before w ∧ a = c.rep (c.step i).after w) ∧
    ((c.step i).recorded w = none → c.rep (c.step i).before w = c.rep (c.step i).after w)

/-- A joined link shares its side: the two records read one report. -/
theorem joined_same_report {i w : Nat} (h : c.joined i (i + 1))
    (left : c.compares i (c.rowRoute w)) (right : c.compares (i + 1) (c.rowRoute w)) :
    c.rep (c.step i).after w = c.rep (c.step (i + 1)).before w := by
  obtain ⟨⟨⟨_, ids⟩, _⟩, inputs⟩ := h
  unfold rep
  have hid : (fun f => if f = Field.tool then none else (c.step i).after.id f) =
      (fun f => if f = Field.tool then none else (c.step (i + 1)).before.id f) := by
    funext f
    by_cases hf : f = Field.tool
    · simp [hf]
    · have := (ids f).1
      simp [afterId, beforeId, hf] at this
      simp [hf, this]
  have hin := (inputs (c.rowRoute w) ⟨left, right⟩).1
  simp only [afterInputs, beforeInputs] at hin
  rw [hid, hin]

/-- The point truth of a chain: each point's report. -/
def truth (w : Nat) (p : Nat) : V :=
  if p < c.n then c.rep (c.step p).before w else c.rep (c.step (c.n - 1)).after w

/-- **The net verdict of a joined chain is its last step's verdict**: every
link joined, every step faithful and comparing the row's route, and the row
listed somewhere. -/
theorem chain_net_is_last {w : Nat} (nonempty : 0 < c.n) (hf : c.StepsFaithful w)
    (links : ∀ i, i + 1 < c.n → c.joined i (i + 1))
    (comparing : ∀ i, i < c.n → c.compares i (c.rowRoute w))
    (listedSomewhere : ∃ i, i < c.n ∧ (c.step i).recorded w ≠ none) :
    c.netAfter w = c.rep (c.step (c.n - 1)).after w := by
  have shared : ∀ i, i < c.n → c.rep (c.step i).after w = c.truth w (i + 1) := by
    intro i hi
    unfold truth
    by_cases hnext : i + 1 < c.n
    · simp only [hnext, ↓reduceIte]
      exact c.joined_same_report (links i hnext) (comparing i hi) (comparing (i + 1) hnext)
    · have : i = c.n - 1 := by omega
      simp only [hnext, ↓reduceIte]; rw [this]
  have faithful : c.Faithful (c.truth w) w := by
    intro i hi
    have hb : c.truth w i = c.rep (c.step i).before w := by unfold truth; simp only [hi, ↓reduceIte]
    refine ⟨fun b a hr => ?_, fun hr => ?_⟩
    · obtain ⟨h1, h2⟩ := (hf i hi).1 b a hr
      exact ⟨h1.trans hb.symm, h2.trans (shared i hi)⟩
    · rw [hb, (hf i hi).2 hr]; exact shared i hi
  have := c.recon_sound faithful listedSomewhere c.n (Nat.le_refl _)
  unfold netAfter last
  rw [this]
  unfold truth
  simp only [Nat.lt_irrefl, ↓reduceIte]

/-- **A link with a differing identity is refused.** -/
theorem link_refused_of_differing_identity {i j : Nat} (f : Field)
    (h : c.afterId i f ≠ c.beforeId j f) : ¬ c.joined i j := by
  intro hj
  exact h (hj.1.1.2 f).1

theorem moved_listed {i w : Nat} (h : c.movedB i w = true) : c.listed i w := by
  unfold movedB at h
  unfold listed
  cases hr : (c.step i).recorded w with
  | none => simp [hr] at h
  | some x => simp

/-- **A cancelled move is still reported**: a row any step moves is listed
with its verdict at every side, whatever its net move. -/
theorem a_cancelled_move_is_reported {w : Nat} (h : c.moves w ≥ 1) : c.reported w := by
  unfold moves at h
  have : ∃ i, i ∈ (List.range c.n).filter (fun i => c.movedB i w) := by
    cases hl : (List.range c.n).filter (fun i => c.movedB i w) with
    | nil => rw [hl] at h; simp at h
    | cons x xs => exact ⟨x, by simp⟩
  obtain ⟨i, hi⟩ := this
  simp [List.mem_filter, List.mem_range] at hi
  exact ⟨i, hi.1, Or.inl hi.2⟩

end Chain

/-! ## Parts B and C: composed bundles and declared dependencies -/

/-- A checker plugin's answer, as the firewall classified it (rule 10h). -/
inductive Claim where
  | none
  | certifiedSafe
  | validatedUnsafe
  | uncertifiedSafe
  | unvalidatedUnsafe
  | undecided
  | failure
  deriving DecidableEq, Repr

/-- The verdict a class stands for: an uncertified `Safe` or unvalidated
`Unsafe` is `Pending`. -/
def Claim.verdict : Claim → V
  | .none | .certifiedSafe => .safe
  | .validatedUnsafe => .provisionalUnsafe
  | .uncertifiedSafe | .unvalidatedUnsafe => .pending
  | .undecided | .failure => .unknown

/-- A declared dependency: who states it, what it names, and its local
checks as the resolver computes them from the environment. -/
structure Dep where
  dependentBundle : Nat
  dependent : Nat
  targetBundle : Nat
  target : Nat
  loaded : Prop
  inRange : Prop
  digestMatches : Prop
  declaredCrate : Prop

/-- A declaration: its bundle and function. -/
abbrev Node := Nat × Nat

def Dep.source (d : Dep) : Node := (d.dependentBundle, d.dependent)
def Dep.goal (d : Dep) : Node := (d.targetBundle, d.target)

/-- The environment's own hold class of a declaration (a pin, a review, a
library version, a supersession conflict). -/
inductive HoldClass where
  | pending
  | unknown
  deriving DecidableEq, Repr

structure Model where
  bundles : List Nat
  /-- The digest of a bundle's reviewed declaration of a function. -/
  text : Nat → Nat → Ident
  libcName : Nat → Prop
  libcOwner : Nat → Prop
  stampsDecide : Nat → Prop
  host : Nat → V
  claim : Nat → Nat → Claim
  recorded : Nat → Nat → V
  plugin : Nat → Prop
  deps : List Dep
  env : Node → Option HoldClass

namespace Model

variable (m : Model)

/-- `#cx_declares`. -/
def declares (b g : Nat) : Prop := b ∈ m.bundles ∧ m.text b g ≠ none

/-- `#cx_refused`: a libc name outside the libc crate, or an undecided
disagreement. -/
def refused (b g : Nat) : Prop :=
  m.declares b g ∧ ((m.libcName g ∧ ¬ m.libcOwner b) ∨
    (¬ m.stampsDecide g ∧ ∃ c, m.declares c g ∧ m.text c g ≠ m.text b g))

/-- `#cx_owner_named`. -/
def ownerNamed (d : Dep) : Prop := ¬ (m.libcName d.target ∧ ¬ m.libcOwner d.targetBundle)

def localOk (d : Dep) : Prop :=
  d.loaded ∧ d.inRange ∧ d.digestMatches ∧ d.declaredCrate ∧ m.ownerNamed d

/-- Reachability through declared dependencies, over a list of them. -/
inductive Reach (ds : List Dep) : Node → Node → Prop where
  | edge {d : Dep} : d ∈ ds → Reach ds d.source d.goal
  | trans {x y z : Node} : Reach ds x y → Reach ds y z → Reach ds x z

/-- `#cx_on_cycle`. -/
def OnCycle (ds : List Dep) (x : Node) : Prop := Reach ds x x

/-- `#cx_held`: the least set closed under the four causes. -/
inductive Held (ds : List Dep) : Node → Prop where
  | env {x : Node} : m.env x ≠ none → Held ds x
  | fails {d : Dep} : d ∈ ds → ¬ m.localOk d → Held ds d.source
  | cycle {x : Node} : OnCycle ds x → Held ds x
  | through {d : Dep} : d ∈ ds → Held ds d.goal → Held ds d.source

/-- A hold with a cause no review resolves. -/
inductive HeldUnknown (ds : List Dep) : Node → Prop where
  | env {x : Node} : m.env x = some .unknown → HeldUnknown ds x
  | fails {d : Dep} : d ∈ ds →
      (¬ d.loaded ∨ ¬ d.inRange ∨ ¬ d.declaredCrate ∨ ¬ m.ownerNamed d) → HeldUnknown ds d.source
  | cycle {x : Node} : OnCycle ds x → HeldUnknown ds x
  | through {d : Dep} : d ∈ ds → HeldUnknown ds d.goal → HeldUnknown ds d.source

theorem heldUnknown_held {ds : List Dep} {x : Node} (h : m.HeldUnknown ds x) : m.Held ds x := by
  induction h with
  | env h => exact .env (by rw [h]; simp)
  | fails hd hfail =>
    refine .fails hd ?_
    intro hok
    obtain ⟨l, r, _, c, o⟩ := hok
    rcases hfail with h | h | h | h
    · exact h l
    · exact h r
    · exact h c
    · exact h o
  | cycle h => exact .cycle h
  | through hd _ ih => exact .through hd ih

open Classical in
/-- `#cx_hold_verdict`. -/
noncomputable def holdVerdict (b g : Nat) : V :=
  if m.HeldUnknown m.deps (b, g) then .unknown else if m.Held m.deps (b, g) then .pending else .safe

/-- `#cx_human_resolvable`. -/
def humanResolvable (b g : Nat) : Prop := m.Held m.deps (b, g) ∧ ¬ m.HeldUnknown m.deps (b, g)

/-- `#cx_checker_verdict`. -/
def checkerVerdict (b g : Nat) : V := (m.claim b g).verdict

/-- `#cx_proposes_decision` and `#cx_checked`. -/
def proposesDecision (b g : Nat) : Prop :=
  m.claim b g = .certifiedSafe ∨ m.claim b g = .validatedUnsafe ∨
  m.claim b g = .uncertifiedSafe ∨ m.claim b g = .unvalidatedUnsafe
def checked (b g : Nat) : Prop := m.claim b g = .certifiedSafe ∨ m.claim b g = .validatedUnsafe

open Classical in
noncomputable def standing (b g : Nat) : V := if m.refused b g then .unknown else .safe

open Classical in
noncomputable def checkerContribution (b g : Nat) : V :=
  if m.recorded b g = m.checkerVerdict b g then m.checkerVerdict b g
  else join (m.checkerVerdict b g) .unknown

/-- `#cx_contribution`. -/
noncomputable def contribution (b g : Nat) : V :=
  join (m.standing b g) (join (m.checkerContribution b g) (m.holdVerdict b g))

open Classical in
noncomputable def memberValue (g b : Nat) : V := if m.declares b g then m.contribution b g else .safe

/-- `#cx_composed`. -/
noncomputable def composed (g : Nat) : V := fold (m.host g :: m.bundles.map (m.memberValue g))

/-- `#cx_satisfied`. -/
def satisfied (d : Dep) : Prop := m.localOk d ∧ ¬ m.Held m.deps d.goal

/-- `#cx_rank`. -/
def rank (v : V) : Nat := v.rank

/-! ### Part B theorems -/

theorem fold_mem_or_safe : ∀ xs : List V, fold xs = .safe ∨ fold xs ∈ xs
  | [] => Or.inl rfl
  | x :: xs => by
    rcases fold_mem_or_safe xs with h | h
    · simp [fold, h, join_safe]
    · unfold fold
      by_cases hle : x.rank ≤ (fold xs).rank
      · right; simp [join, hle, h]
      · right; simp [join, hle]

theorem member_le_composed {b g : Nat} (h : m.declares b g) :
    (m.contribution b g).rank ≤ (m.composed g).rank := by
  have hm : m.memberValue g b = m.contribution b g := by simp [memberValue, h]
  apply every_link_is_below_the_fold
  simp only [List.mem_cons, List.mem_map]
  exact Or.inr ⟨b, h.1, hm⟩

/-- **Composition is the semigroup join** of the host's verdict and every
declaring bundle's contribution: an upper bound of each, attained by one. -/
theorem composition_is_the_join (g : Nat) :
    (m.host g).rank ≤ (m.composed g).rank ∧
    (∀ b, m.declares b g → (m.contribution b g).rank ≤ (m.composed g).rank) ∧
    (m.composed g = m.host g ∨ ∃ b, m.declares b g ∧ m.composed g = m.contribution b g) := by
  refine ⟨every_link_is_below_the_fold (List.mem_cons_self ..), fun b h => m.member_le_composed h,
    ?_⟩
  have hostSafe : m.composed g = .safe → m.host g = .safe := fun h =>
    (fold_eq_safe_iff _).mp h _ (List.mem_cons_self ..)
  rcases fold_mem_or_safe (m.host g :: m.bundles.map (m.memberValue g)) with h | h
  · left; rw [show m.composed g = .safe from h, hostSafe h]
  · simp only [List.mem_cons, List.mem_map] at h
    rcases h with h | ⟨b, hb, hv⟩
    · left; exact h
    · by_cases hd : m.declares b g
      · right; refine ⟨b, hd, ?_⟩
        simp [memberValue, hd] at hv
        exact hv.symm
      · simp [memberValue, hd] at hv
        left
        have : m.composed g = .safe := hv.symm
        rw [this, hostSafe this]

/-- Composition reads the bundles only as a set: reordering them keeps the
composed verdict. -/
theorem composition_is_order_independent (m' : Model) (g : Nat)
    (same : m'.bundles.Perm m.bundles) (hosts : m'.host g = m.host g)
    (members : ∀ b, m'.memberValue g b = m.memberValue g b) :
    m'.composed g = m.composed g := by
  unfold composed
  have hl : m'.bundles.map (m'.memberValue g) = m'.bundles.map (m.memberValue g) := by
    apply List.map_congr_left; intro b _; exact members b
  rw [hosts, hl]
  have perm : (m'.bundles.map (m.memberValue g)).Perm (m.bundles.map (m.memberValue g)) :=
    same.map _
  have key : ∀ (xs ys : List V), xs.Perm ys → fold xs = fold ys := by
    intro xs ys p
    induction p with
    | nil => rfl
    | cons x _ ih => simp [fold, ih]
    | swap x y l => simp [fold, ← join_assoc, join_comm x y]
    | trans _ _ ih1 ih2 => exact ih1.trans ih2
  exact key _ _ (List.Perm.cons _ perm)

/-- The reasoned composition: every contributor's reasons, folded with the
semigroup's join. -/
def composeReasoned {R : Type} : List (Reasoned R) → Reasoned R
  | [] => Reasoned.identity
  | x :: xs => Reasoned.join x (composeReasoned xs)

theorem composeReasoned_value {R : Type} :
    ∀ xs : List (Reasoned R), (composeReasoned xs).value = fold (xs.map Reasoned.value)
  | [] => rfl
  | x :: xs => by simp [composeReasoned, Reasoned.join, fold, composeReasoned_value xs]

/-- **Composition keeps every member's reason.** -/
theorem composition_keeps_every_reason {R : Type} :
    ∀ (xs : List (Reasoned R)) (x : Reasoned R) (r : R), x ∈ xs → r ∈ x.reasons →
      r ∈ (composeReasoned xs).reasons
  | [], _, _, h, _ => by simp at h
  | y :: ys, x, r, h, hr => by
    simp only [List.mem_cons] at h
    rcases h with rfl | h
    · exact Reasoned.join_keeps_reasons _ _ r (Or.inl hr)
    · exact Reasoned.join_keeps_reasons _ _ r (Or.inr (composition_keeps_every_reason ys x r h hr))

/-! ### Part C theorems -/

/-- **A held dependency holds its dependents.** -/
theorem held_dependency_holds_dependents {d : Dep} (hd : d ∈ m.deps)
    (h : m.Held m.deps d.goal) : m.Held m.deps d.source :=
  .through hd h

theorem reach_mono {ds ds' : List Dep} (sub : ∀ d, d ∈ ds → d ∈ ds') {x y : Node}
    (h : Reach ds x y) : Reach ds' x y := by
  induction h with
  | edge hd => exact .edge (sub _ hd)
  | trans _ _ ih1 ih2 => exact .trans ih1 ih2

theorem held_mono {ds ds' : List Dep} (sub : ∀ d, d ∈ ds → d ∈ ds') {x : Node}
    (h : m.Held ds x) : m.Held ds' x := by
  induction h with
  | env h => exact .env h
  | fails hd hn => exact .fails (sub _ hd) hn
  | cycle h => exact .cycle (reach_mono sub h)
  | through hd _ ih => exact .through (sub _ hd) ih

/-- **Resolution is order-independent**: it reads the dependencies only
through membership, so any two lists of the same dependencies hold the same
declarations. -/
theorem held_is_order_independent {ds ds' : List Dep} (same : ∀ d, d ∈ ds ↔ d ∈ ds')
    (x : Node) : m.Held ds x ↔ m.Held ds' x :=
  ⟨m.held_mono (fun d h => (same d).mp h), m.held_mono (fun d h => (same d).mpr h)⟩

/-- **Resolution is acyclic**: a declaration that is not held reaches, through
its declared dependencies, no held declaration and no cycle. -/
theorem resolution_is_acyclic {x y : Node} (free : ¬ m.Held m.deps x)
    (h : Reach m.deps x y) : ¬ m.Held m.deps y ∧ ¬ OnCycle m.deps y := by
  have notHeld : ∀ {x y : Node}, Reach m.deps x y → ¬ m.Held m.deps x → ¬ m.Held m.deps y := by
    intro x y r
    induction r with
    | edge hd => intro hx hy; exact hx (.through hd hy)
    | trans _ _ ih1 ih2 => intro hx; exact ih2 (ih1 hx)
  have hy := notHeld h free
  exact ⟨hy, fun hc => hy (.cycle hc)⟩

theorem holdVerdict_of_unknown {x : Node} (h : m.HeldUnknown m.deps x) :
    m.holdVerdict x.1 x.2 = .unknown := by
  simp [holdVerdict, h]

theorem holdVerdict_of_held {x : Node} (h : m.Held m.deps x) :
    m.holdVerdict x.1 x.2 = .unknown ∨ m.holdVerdict x.1 x.2 = .pending := by
  unfold holdVerdict
  by_cases hu : m.HeldUnknown m.deps (x.1, x.2)
  · left; simp [hu]
  · right; simp [hu, h]

theorem holdVerdict_le_contribution (b g : Nat) :
    (m.holdVerdict b g).rank ≤ (m.contribution b g).rank := by
  unfold contribution
  exact Nat.le_trans (le_join_right _ _) (le_join_right _ _)

end Model

/-! ## The formulas, one theorem each -/

section Formulas

variable (c : Chain) (m : Model)

open Chain Model

theorem formula_cx_joined_is_every_identity : ∀ s t, c.joined s t →
    ((c.next s t ∧ ∀ f, c.afterId s f = c.beforeId t f) ∧
      ∀ r, (c.compares s r ∧ c.compares t r) → c.afterInputs s r = c.beforeInputs t r) := by
  intro s t h
  exact ⟨⟨h.1.1.1, fun f => (h.1.1.2 f).1⟩, fun r hr => (h.2 r hr).1⟩

theorem formula_cx_every_identity_is_joined : ∀ s t,
    (((c.next s t ∧ ∀ f, (c.afterId s f = c.beforeId t f ∧
        (Chain.required f → ¬ c.afterId s f = unrecorded))) ∧
      (∃ r, c.compares s r ∧ c.compares t r)) ∧
      (∀ r, (c.compares s r ∧ c.compares t r) →
        (c.afterInputs s r = c.beforeInputs t r ∧ ¬ c.afterInputs s r = unrecorded))) →
    c.joined s t := by
  intro s t h
  exact h

theorem formula_cx_refuse_unrecorded : ∀ s t f,
    (Chain.required f ∧ (c.afterId s f = unrecorded ∨ c.beforeId t f = unrecorded)) →
    ¬ c.joined s t := by
  intro s t f ⟨hreq, hu⟩ hj
  have ⟨heq, hrec⟩ := hj.1.1.2 f
  rcases hu with hu | hu
  · exact hrec hreq hu
  · exact hrec hreq (heq.trans hu)

theorem formula_cx_refuse_tool : ∀ s t,
    ¬ c.afterId s .tool = c.beforeId t .tool → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .tool h

theorem formula_cx_refuse_revision : ∀ s t,
    ¬ c.afterId s .revision = c.beforeId t .revision → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .revision h

theorem formula_cx_refuse_analyzer : ∀ s t,
    ¬ c.afterId s .analyzer = c.beforeId t .analyzer → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .analyzer h

theorem formula_cx_refuse_bundles : ∀ s t,
    ¬ c.afterId s .bundles = c.beforeId t .bundles → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .bundles h

theorem formula_cx_refuse_build_commit : ∀ s t,
    ¬ c.afterId s .buildCommit = c.beforeId t .buildCommit → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .buildCommit h

theorem formula_cx_refuse_link_resolution : ∀ s t,
    ¬ c.afterId s .linkResolution = c.beforeId t .linkResolution → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .linkResolution h

theorem formula_cx_refuse_rules_off : ∀ s t,
    ¬ c.afterId s .rulesOff = c.beforeId t .rulesOff → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .rulesOff h

theorem formula_cx_refuse_dependency_graph : ∀ s t,
    ¬ c.afterId s .dependencyGraph = c.beforeId t .dependencyGraph → ¬ c.joined s t :=
  fun _ _ h => c.link_refused_of_differing_identity .dependencyGraph h

theorem formula_cx_refuse_inputs : ∀ s t r,
    ((c.compares s r ∧ c.compares t r) ∧ ¬ c.afterInputs s r = c.beforeInputs t r) →
    ¬ c.joined s t := by
  intro s t r ⟨hc, hne⟩ hj
  exact hne (hj.2 r hc).1

theorem formula_cx_refuse_no_shared_route : ∀ s t,
    ¬ (∃ r, (c.compares s r ∧ c.compares t r)) → ¬ c.joined s t := by
  intro s t h hj
  exact h hj.1.2

theorem formula_cx_joined_shares_the_side : ∀ s t, c.joined s t → Chain.after s = Chain.before t := by
  intro s t hj
  simp [Chain.after, Chain.before, hj.1.1.1.1]

theorem formula_cx_complete_is_every_step : ∀ r, c.chainComplete r → ∀ s, (c.member s →
    ((((c.compares s r ∧ c.statusComplete s r) ∧ ¬ c.unmatched s r) ∧
      (¬ c.unmatchedUnits s r ∧ ¬ c.conflicted s r)) ∧ ¬ c.disagrees s r)) :=
  fun _ h => h

theorem formula_cx_incomplete_not_measured : ∀ r s,
    (c.member s ∧ ¬ c.compares s r) → ¬ c.chainComplete r :=
  fun _ s ⟨hm, hn⟩ h => hn (h s hm).1.1.1.1

theorem formula_cx_incomplete_status : ∀ r s,
    (c.member s ∧ ¬ c.statusComplete s r) → ¬ c.chainComplete r :=
  fun _ s ⟨hm, hn⟩ h => hn (h s hm).1.1.1.2

theorem formula_cx_incomplete_unmatched : ∀ r s,
    (c.member s ∧ c.unmatched s r) → ¬ c.chainComplete r :=
  fun _ s ⟨hm, hu⟩ h => (h s hm).1.1.2 hu

theorem formula_cx_incomplete_unmatched_units : ∀ r s,
    (c.member s ∧ c.unmatchedUnits s r) → ¬ c.chainComplete r :=
  fun _ s ⟨hm, hu⟩ h => (h s hm).1.2.1 hu

theorem formula_cx_incomplete_conflicts : ∀ r s,
    (c.member s ∧ c.conflicted s r) → ¬ c.chainComplete r :=
  fun _ s ⟨hm, hu⟩ h => (h s hm).1.2.2 hu

theorem formula_cx_incomplete_row_disagrees : ∀ r s,
    (c.member s ∧ c.disagrees s r) → ¬ c.chainComplete r :=
  fun _ s ⟨hm, hd⟩ h => (h s hm).2 hd

theorem formula_cx_unlisted_is_unchanged : ∀ s w,
    (c.compares s (c.rowRoute' w) ∧ ¬ c.listed s w) →
    c.verdict (Chain.before s) w = c.verdict (Chain.after s) w := by
  intro s w ⟨_, hn⟩
  have : (c.step s).recorded w = none := by
    unfold Chain.listed at hn; exact Classical.byContradiction hn
  exact c.verdict_unlisted this

theorem formula_cx_net_is_first : ∀ w, c.netBefore w = c.verdict Chain.first w := fun _ => rfl

theorem formula_cx_net_is_last : ∀ w, c.netAfter w = c.verdict c.last w := fun _ => rfl

theorem formula_cx_moved_is_a_change : ∀ s w,
    (c.moved s w ∧ ¬ c.disagrees s (c.rowRoute' w)) →
    ¬ c.verdict (Chain.before s) w = c.verdict (Chain.after s) w := by
  intro s w ⟨⟨hs, hb⟩, hnd⟩
  unfold Chain.movedB at hb
  cases hr : (c.step s).recorded w with
  | none => simp [hr] at hb
  | some x =>
    obtain ⟨b, a⟩ := x
    simp [hr] at hb
    have hafter : c.verdict (s + 1) w = a := by
      simp [Chain.verdict, Chain.lastBefore, hr]
    simp only [Chain.before, Chain.after]
    rw [hafter]
    unfold Chain.verdict
    cases hl : c.lastBefore s w with
    | some y =>
      obtain ⟨b', a'⟩ := y
      have : a' = b := by
        apply Classical.byContradiction
        intro hne
        exact hnd ⟨w, rfl, b, a, b', a', hr, hl, hne⟩
      simp only
      rw [this]; exact hb
    | none =>
      have hfirst : c.firstFrom s w = some (b, a) := by
        unfold Chain.firstFrom
        have hm : s < c.n := hs
        cases hk : c.n - s with
        | zero => omega
        | succ k => simp [Chain.firstFromAux, hr]
      simp only [hfirst]
      exact hb

theorem formula_cx_moved_is_reported : ∀ s w, c.moved s w → c.reported w :=
  fun s _ ⟨hm, hb⟩ => ⟨s, hm, Or.inl hb⟩

theorem formula_cx_cancelled_is_reported : ∀ w, c.cancelled w →
    ((c.reported w ∧ c.moves w ≥ 2) ∧ c.netBefore w = c.netAfter w) :=
  fun _ ⟨h1, h2, h3⟩ => ⟨⟨h1, h2⟩, h3⟩

theorem formula_cx_cascade_is_reported : ∀ w, c.cascade w →
    ((c.reported w ∧ c.moves w ≥ 2) ∧ ¬ c.netBefore w = c.netAfter w) :=
  fun _ ⟨h1, h2, h3⟩ => ⟨⟨h1, h2⟩, h3⟩

theorem formula_cx_evidence_change_is_reported : ∀ s w, c.evidenceChanged s w → c.reported w :=
  fun s _ ⟨hm, he⟩ => ⟨s, hm, Or.inr (Or.inr he)⟩

theorem formula_cx_reports_only : ∀ p w, c.published p w = c.verdict p w := fun _ _ => rfl

theorem formula_cx_new_safe_is_a_move_to_safe : ∀ s w, c.newSafe s w →
    (c.moved s w ∧ c.verdict (Chain.after s) w = .safe) :=
  fun _ _ ⟨h1, h2, _⟩ => ⟨h1, h2⟩

theorem formula_cx_unattributed_move_is_listed : ∀ s w,
    (c.moved s w ∧ ¬ c.declaredMove s w) → c.unattributed s w := fun _ _ h => h

theorem formula_cx_unattributed_new_safe_is_reviewed : ∀ s w,
    (c.newSafe s w ∧ ¬ c.declaredMove s w) → c.review s w := fun _ _ h => h

theorem formula_cx_review_is_never_accepted : ∀ s w, c.review s w → ¬ c.accepted s w :=
  fun _ _ ⟨_, hn⟩ ⟨_, hd⟩ => hn hd

theorem formula_cx_dependency_change_listed : ∀ s w,
    (c.moved s w ∧ c.dependencyChanged s w) → c.dependencyListed s w := fun _ _ h => h

theorem formula_cx_refuse_libc_redeclared : ∀ b g,
    ((m.declares b g ∧ m.libcName g) ∧ ¬ m.libcOwner b) → m.refused b g :=
  fun _ _ ⟨⟨hd, hl⟩, ho⟩ => ⟨hd, Or.inl ⟨hl, ho⟩⟩

theorem formula_cx_refuse_conflicting_declarations : ∀ b c g,
    (((m.declares b g ∧ m.declares c g) ∧ ¬ m.text b g = m.text c g) ∧ ¬ m.stampsDecide g) →
    (m.refused b g ∧ m.refused c g) := by
  intro b c g ⟨⟨⟨hb, hc⟩, hne⟩, hs⟩
  exact ⟨⟨hb, Or.inr ⟨hs, c, hc, fun h => hne h.symm⟩⟩, ⟨hc, Or.inr ⟨hs, b, hb, hne⟩⟩⟩

theorem formula_cx_refused_is_unknown : ∀ b g,
    m.refused b g → Model.rank .unknown ≤ Model.rank (m.contribution b g) := by
  intro b g h
  have : m.standing b g = .unknown := by simp [Model.standing, h]
  unfold Model.rank Model.contribution
  rw [this]; exact le_join_left _ _

theorem formula_cx_composed_bounds_every_member : ∀ b g,
    m.declares b g → Model.rank (m.contribution b g) ≤ Model.rank (m.composed g) :=
  fun _ _ h => m.member_le_composed h

theorem formula_cx_composed_bounds_the_host : ∀ g, Model.rank (m.host g) ≤ Model.rank (m.composed g) :=
  fun g => (m.composition_is_the_join g).1

theorem formula_cx_composed_is_a_member : ∀ g,
    (m.composed g = m.host g ∨ ∃ b, (m.declares b g ∧ m.composed g = m.contribution b g)) :=
  fun g => (m.composition_is_the_join g).2.2

/-- `#cx_reasons_kept`: in the reasoned composition of a function's
contributors, every reason of the bundle's contributor is a reason of the
composed verdict (`composition_keeps_every_reason`). -/
def reasonsKept {R : Type} (contributors : Nat → Nat → Reasoned R)
    (members : Nat → List Nat) (b g : Nat) : Prop :=
  ∀ r, r ∈ (contributors b g).reasons →
    r ∈ (composeReasoned ((members g).map (fun b' => contributors b' g))).reasons

theorem formula_cx_every_reason_is_kept {R : Type} (contributors : Nat → Nat → Reasoned R)
    (members : Nat → List Nat) (declares : ∀ b g, m.declares b g → b ∈ members g) :
    ∀ b g, m.declares b g → reasonsKept contributors members b g := by
  intro b g hd r hr
  exact composition_keeps_every_reason _ (contributors b g) r
    (List.mem_map.mpr ⟨b, declares b g hd, rfl⟩) hr

theorem formula_cx_uncertified_plugin_is_pending : ∀ b g,
    ((m.plugin b ∧ m.proposesDecision b g) ∧ ¬ m.checked b g) → m.checkerVerdict b g = .pending := by
  intro b g ⟨⟨_, hp⟩, hc⟩
  unfold Model.checkerVerdict
  unfold Model.checked at hc
  rcases hp with h | h | h | h
  · exact absurd (Or.inl h) hc
  · exact absurd (Or.inr h) hc
  · rw [h]; rfl
  · rw [h]; rfl

theorem formula_cx_checker_bounds_contribution : ∀ b g,
    Model.rank (m.checkerVerdict b g) ≤ Model.rank (m.contribution b g) := by
  intro b g
  unfold Model.rank Model.contribution
  refine Nat.le_trans ?_ (Nat.le_trans (le_join_left _ _) (le_join_right _ _))
  unfold Model.checkerContribution
  by_cases h : m.recorded b g = m.checkerVerdict b g
  · simp [h]
  · simp [h]; exact le_join_left _ _

theorem formula_cx_refuse_misclassified_record : ∀ b g,
    ¬ m.recorded b g = m.checkerVerdict b g → Model.rank .unknown ≤ Model.rank (m.contribution b g) := by
  intro b g h
  unfold Model.rank Model.contribution
  refine Nat.le_trans ?_ (Nat.le_trans (le_join_left _ _) (le_join_right _ _))
  unfold Model.checkerContribution
  simp [h]; exact le_join_right _ _

/-- The dependencies as a sort: the declared ones. -/
abbrev D := {d : Dep // d ∈ m.deps}

theorem formula_cx_satisfied_is_every_check : ∀ d : D m, m.satisfied d.1 →
    ((((d.1.loaded ∧ d.1.inRange) ∧ d.1.digestMatches) ∧ (d.1.declaredCrate ∧ m.ownerNamed d.1)) ∧
      ¬ m.Held m.deps (d.1.targetBundle, d.1.target)) := by
  intro d ⟨⟨l, r, g, c', o⟩, h⟩
  exact ⟨⟨⟨⟨l, r⟩, g⟩, ⟨c', o⟩⟩, h⟩

theorem formula_cx_every_check_is_satisfied : ∀ d : D m,
    ((((d.1.loaded ∧ d.1.inRange) ∧ d.1.digestMatches) ∧ (d.1.declaredCrate ∧ m.ownerNamed d.1)) ∧
      ¬ m.Held m.deps (d.1.targetBundle, d.1.target)) → m.satisfied d.1 := by
  intro d ⟨⟨⟨⟨l, r⟩, g⟩, ⟨c', o⟩⟩, h⟩
  exact ⟨⟨l, r, g, c', o⟩, h⟩

theorem formula_cx_unsatisfied_holds_the_dependent : ∀ d : D m,
    ¬ m.satisfied d.1 → m.Held m.deps (d.1.dependentBundle, d.1.dependent) := by
  intro d h
  by_cases hok : m.localOk d.1
  · have : m.Held m.deps d.1.goal :=
      Classical.byContradiction (fun hn => h ⟨hok, hn⟩)
    exact .through d.2 this
  · exact .fails d.2 hok

theorem formula_cx_held_dependency_holds_dependents : ∀ d : D m,
    m.Held m.deps (d.1.targetBundle, d.1.target) →
    m.Held m.deps (d.1.dependentBundle, d.1.dependent) :=
  fun d h => m.held_dependency_holds_dependents d.2 h

theorem formula_cx_cycle_holds : ∀ b g, OnCycle m.deps (b, g) → m.Held m.deps (b, g) :=
  fun _ _ h => .cycle h

theorem formula_cx_refuse_libc_target_elsewhere : ∀ d : D m,
    (m.libcName d.1.target ∧ ¬ m.libcOwner d.1.targetBundle) → ¬ m.ownerNamed d.1 :=
  fun _ h hn => hn h

theorem formula_cx_missing_is_unknown : ∀ d : D m, ¬ d.1.loaded →
    Model.rank .unknown ≤ Model.rank (m.holdVerdict d.1.dependentBundle d.1.dependent) := by
  intro d h
  have := m.holdVerdict_of_unknown (HeldUnknown.fails (m := m) d.2 (Or.inl h))
  simp only [Dep.source] at this
  unfold Model.rank; rw [this]; exact Nat.le_refl _

theorem formula_cx_out_of_range_is_unknown : ∀ d : D m, ¬ d.1.inRange →
    Model.rank .unknown ≤ Model.rank (m.holdVerdict d.1.dependentBundle d.1.dependent) := by
  intro d h
  have := m.holdVerdict_of_unknown (HeldUnknown.fails (m := m) d.2 (Or.inr (Or.inl h)))
  simp only [Dep.source] at this
  unfold Model.rank; rw [this]; exact Nat.le_refl _

theorem formula_cx_undeclared_crate_is_unknown : ∀ d : D m,
    (¬ d.1.declaredCrate ∨ ¬ m.ownerNamed d.1) →
    Model.rank .unknown ≤ Model.rank (m.holdVerdict d.1.dependentBundle d.1.dependent) := by
  intro d h
  have : m.HeldUnknown m.deps d.1.source := by
    rcases h with h | h
    · exact HeldUnknown.fails d.2 (Or.inr (Or.inr (Or.inl h)))
    · exact HeldUnknown.fails d.2 (Or.inr (Or.inr (Or.inr h)))
  have := m.holdVerdict_of_unknown this
  simp only [Dep.source] at this
  unfold Model.rank; rw [this]; exact Nat.le_refl _

theorem formula_cx_digest_change_is_at_least_pending : ∀ d : D m, ¬ d.1.digestMatches →
    Model.rank .pending ≤ Model.rank (m.holdVerdict d.1.dependentBundle d.1.dependent) := by
  intro d h
  have held : m.Held m.deps d.1.source := .fails d.2 (fun ⟨_, _, g, _⟩ => h g)
  rcases m.holdVerdict_of_held held with e | e <;>
    · simp only [Dep.source] at e; unfold Model.rank; rw [e]; decide

theorem formula_cx_cycle_is_unknown : ∀ b g, OnCycle m.deps (b, g) →
    Model.rank .unknown ≤ Model.rank (m.holdVerdict b g) := by
  intro b g h
  have := m.holdVerdict_of_unknown (x := (b, g)) (.cycle h)
  unfold Model.rank; rw [this]; exact Nat.le_refl _

theorem formula_cx_hold_is_pending_or_unknown : ∀ b g, m.Held m.deps (b, g) →
    (m.holdVerdict b g = .pending ∨ m.holdVerdict b g = .unknown) := by
  intro b g h
  rcases m.holdVerdict_of_held (x := (b, g)) h with e | e
  · exact Or.inr e
  · exact Or.inl e

theorem formula_cx_pending_hold_is_human_resolvable : ∀ b g,
    (m.Held m.deps (b, g) ∧ m.holdVerdict b g = .pending) → m.humanResolvable b g := by
  intro b g ⟨h, hp⟩
  refine ⟨h, fun hu => ?_⟩
  have := m.holdVerdict_of_unknown (x := (b, g)) hu
  simp only at this
  rw [this] at hp; cases hp

theorem formula_cx_hold_bounds_contribution : ∀ b g, m.Held m.deps (b, g) →
    Model.rank (m.holdVerdict b g) ≤ Model.rank (m.contribution b g) :=
  fun b g _ => m.holdVerdict_le_contribution b g

theorem formula_cx_held_is_never_safe : ∀ b g, m.Held m.deps (b, g) →
    ¬ m.contribution b g = .safe := by
  intro b g h hs
  have hle := m.holdVerdict_le_contribution b g
  rw [hs] at hle
  rcases m.holdVerdict_of_held (x := (b, g)) h with e | e <;>
    · simp only at e; rw [e] at hle; simp [V.rank] at hle

end Formulas

/-! ## The CX terms and the formula table -/

/-- One definition per CX term: the term and the Lean definition it denotes. -/
def cxTerms : List (String × String) := [
  ("cx_next", "Chain.next"), ("cx_joined", "Chain.joined"),
  ("cx_after_id", "Chain.afterId"), ("cx_before_id", "Chain.beforeId"),
  ("cx_required", "Chain.required"), ("cx_unrecorded", "unrecorded"),
  ("cx_compares", "Chain.compares"), ("cx_after_inputs", "Chain.afterInputs"),
  ("cx_before_inputs", "Chain.beforeInputs"), ("cx_f_tool", "Field.tool"),
  ("cx_f_revision", "Field.revision"), ("cx_f_analyzer", "Field.analyzer"),
  ("cx_f_bundles", "Field.bundles"), ("cx_f_build_commit", "Field.buildCommit"),
  ("cx_f_link_resolution", "Field.linkResolution"), ("cx_f_rules_off", "Field.rulesOff"),
  ("cx_f_dependency_graph", "Field.dependencyGraph"), ("cx_before", "Chain.before"),
  ("cx_after", "Chain.after"), ("cx_member", "Chain.member"),
  ("cx_chain_complete", "Chain.chainComplete"), ("cx_status_complete", "Chain.statusComplete"),
  ("cx_unmatched", "Chain.unmatched"), ("cx_unmatched_units", "Chain.unmatchedUnits"),
  ("cx_conflicted", "Chain.conflicted"), ("cx_disagrees", "Chain.disagrees"),
  ("cx_row_route", "Chain.rowRoute'"), ("cx_listed", "Chain.listed"),
  ("cx_verdict", "Chain.verdict"), ("cx_first", "Chain.first"), ("cx_last", "Chain.last"),
  ("cx_net_before", "Chain.netBefore"), ("cx_net_after", "Chain.netAfter"),
  ("cx_moved", "Chain.moved"), ("cx_reported", "Chain.reported"), ("cx_moves", "Chain.moves"),
  ("cx_cancelled", "Chain.cancelled"), ("cx_cascade", "Chain.cascade"),
  ("cx_evidence_changed", "Chain.evidenceChanged"), ("cx_published", "Chain.published"),
  ("cx_safe", "V.safe"), ("cx_pending", "V.pending"), ("cx_unknown", "V.unknown"),
  ("cx_new_safe", "Chain.newSafe"), ("cx_declared_move", "Chain.declaredMove"),
  ("cx_unattributed", "Chain.unattributed"), ("cx_review", "Chain.review"),
  ("cx_accepted", "Chain.accepted"), ("cx_dependency_changed", "Chain.dependencyChanged"),
  ("cx_dependency_listed", "Chain.dependencyListed"), ("cx_declares", "Model.declares"),
  ("cx_text", "Model.text"), ("cx_libc_name", "Model.libcName"),
  ("cx_libc_owner", "Model.libcOwner"), ("cx_refused", "Model.refused"),
  ("cx_stamps_decide", "Model.stampsDecide"), ("cx_rank", "Model.rank"),
  ("cx_contribution", "Model.contribution"), ("cx_composed", "Model.composed"),
  ("cx_host", "Model.host"), ("cx_reasons_kept", "reasonsKept"), ("cx_plugin", "Model.plugin"),
  ("cx_proposes_decision", "Model.proposesDecision"), ("cx_checked", "Model.checked"),
  ("cx_checker_verdict", "Model.checkerVerdict"), ("cx_recorded_verdict", "Model.recorded"),
  ("cx_satisfied", "Model.satisfied"), ("cx_loaded", "Dep.loaded"),
  ("cx_in_range", "Dep.inRange"), ("cx_digest_matches", "Dep.digestMatches"),
  ("cx_declared_crate", "Dep.declaredCrate"), ("cx_owner_named", "Model.ownerNamed"),
  ("cx_held", "Model.Held"), ("cx_target_bundle", "Dep.targetBundle"),
  ("cx_target", "Dep.target"), ("cx_dependent_bundle", "Dep.dependentBundle"),
  ("cx_dependent", "Dep.dependent"), ("cx_on_cycle", "Model.OnCycle"),
  ("cx_hold_verdict", "Model.holdVerdict"), ("cx_human_resolvable", "Model.humanResolvable")
]

theorem cxTerms_count : cxTerms.length = 80 := by decide

/-- The formulas of `qcore_schemas::connect::FORMULAS`, as written. -/
def formulaTable : List (String × String) := [
  ("cx_joined_is_every_identity",
   "forall (#s: cx_step) { forall (#t: cx_step) { (#cx_joined(#s,#t) => ((#cx_next(#s,#t) && forall (#f: cx_field) { #cx_after_id(#s,#f) == #cx_before_id(#t,#f) }) && forall (#r: cx_route) { ((#cx_compares(#s,#r) && #cx_compares(#t,#r)) => #cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r)) })) } }"),
  ("cx_every_identity_is_joined",
   "forall (#s: cx_step) { forall (#t: cx_step) { ((((#cx_next(#s,#t) && forall (#f: cx_field) { (#cx_after_id(#s,#f) == #cx_before_id(#t,#f) && (#cx_required(#f) => !#cx_after_id(#s,#f) == #cx_unrecorded)) }) && exists (#r: cx_route) { (#cx_compares(#s,#r) && #cx_compares(#t,#r)) }) && forall (#r: cx_route) { ((#cx_compares(#s,#r) && #cx_compares(#t,#r)) => (#cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r) && !#cx_after_inputs(#s,#r) == #cx_unrecorded)) }) => #cx_joined(#s,#t)) } }"),
  ("cx_refuse_unrecorded",
   "forall (#s: cx_step) { forall (#t: cx_step) { forall (#f: cx_field) { ((#cx_required(#f) && (#cx_after_id(#s,#f) == #cx_unrecorded || #cx_before_id(#t,#f) == #cx_unrecorded)) => !#cx_joined(#s,#t)) } } }"),
  ("cx_refuse_tool",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_tool) == #cx_before_id(#t,#cx_f_tool) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_revision",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_revision) == #cx_before_id(#t,#cx_f_revision) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_analyzer",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_analyzer) == #cx_before_id(#t,#cx_f_analyzer) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_bundles",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_bundles) == #cx_before_id(#t,#cx_f_bundles) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_build_commit",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_build_commit) == #cx_before_id(#t,#cx_f_build_commit) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_link_resolution",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_link_resolution) == #cx_before_id(#t,#cx_f_link_resolution) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_rules_off",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_rules_off) == #cx_before_id(#t,#cx_f_rules_off) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_dependency_graph",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_dependency_graph) == #cx_before_id(#t,#cx_f_dependency_graph) => !#cx_joined(#s,#t)) } }"),
  ("cx_refuse_inputs",
   "forall (#s: cx_step) { forall (#t: cx_step) { forall (#r: cx_route) { (((#cx_compares(#s,#r) && #cx_compares(#t,#r)) && !#cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r)) => !#cx_joined(#s,#t)) } } }"),
  ("cx_refuse_no_shared_route",
   "forall (#s: cx_step) { forall (#t: cx_step) { (!exists (#r: cx_route) { (#cx_compares(#s,#r) && #cx_compares(#t,#r)) } => !#cx_joined(#s,#t)) } }"),
  ("cx_joined_shares_the_side",
   "forall (#s: cx_step) { forall (#t: cx_step) { (#cx_joined(#s,#t) => #cx_after(#s) == #cx_before(#t)) } }"),
  ("cx_complete_is_every_step",
   "forall (#r: cx_route) { (#cx_chain_complete(#r) => forall (#s: cx_step) { (#cx_member(#s) => ((((#cx_compares(#s,#r) && #cx_status_complete(#s,#r)) && !#cx_unmatched(#s,#r)) && (!#cx_unmatched_units(#s,#r) && !#cx_conflicted(#s,#r))) && !#cx_disagrees(#s,#r))) }) }"),
  ("cx_incomplete_not_measured",
   "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && !#cx_compares(#s,#r)) => !#cx_chain_complete(#r)) } }"),
  ("cx_incomplete_status",
   "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && !#cx_status_complete(#s,#r)) => !#cx_chain_complete(#r)) } }"),
  ("cx_incomplete_unmatched",
   "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_unmatched(#s,#r)) => !#cx_chain_complete(#r)) } }"),
  ("cx_incomplete_unmatched_units",
   "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_unmatched_units(#s,#r)) => !#cx_chain_complete(#r)) } }"),
  ("cx_incomplete_conflicts",
   "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_conflicted(#s,#r)) => !#cx_chain_complete(#r)) } }"),
  ("cx_incomplete_row_disagrees",
   "forall (#r: cx_route) { forall (#s: cx_step) { ((#cx_member(#s) && #cx_disagrees(#s,#r)) => !#cx_chain_complete(#r)) } }"),
  ("cx_unlisted_is_unchanged",
   "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_compares(#s,#cx_row_route(#w)) && !#cx_listed(#s,#w)) => #cx_verdict(#cx_before(#s),#w) == #cx_verdict(#cx_after(#s),#w)) } }"),
  ("cx_net_is_first",
   "forall (#w: cx_row) { #cx_net_before(#w) == #cx_verdict(#cx_first,#w) }"),
  ("cx_net_is_last",
   "forall (#w: cx_row) { #cx_net_after(#w) == #cx_verdict(#cx_last,#w) }"),
  ("cx_moved_is_a_change",
   "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_moved(#s,#w) && !#cx_disagrees(#s,#cx_row_route(#w))) => !#cx_verdict(#cx_before(#s),#w) == #cx_verdict(#cx_after(#s),#w)) } }"),
  ("cx_moved_is_reported",
   "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_moved(#s,#w) => #cx_reported(#w)) } }"),
  ("cx_cancelled_is_reported",
   "forall (#w: cx_row) { (#cx_cancelled(#w) => ((#cx_reported(#w) && #cx_moves(#w) >= 2) && #cx_net_before(#w) == #cx_net_after(#w))) }"),
  ("cx_cascade_is_reported",
   "forall (#w: cx_row) { (#cx_cascade(#w) => ((#cx_reported(#w) && #cx_moves(#w) >= 2) && !#cx_net_before(#w) == #cx_net_after(#w))) }"),
  ("cx_evidence_change_is_reported",
   "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_evidence_changed(#s,#w) => #cx_reported(#w)) } }"),
  ("cx_reports_only",
   "forall (#p: cx_point) { forall (#w: cx_row) { #cx_published(#p,#w) == #cx_verdict(#p,#w) } }"),
  ("cx_new_safe_is_a_move_to_safe",
   "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_new_safe(#s,#w) => (#cx_moved(#s,#w) && #cx_verdict(#cx_after(#s),#w) == #cx_safe)) } }"),
  ("cx_unattributed_move_is_listed",
   "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_moved(#s,#w) && !#cx_declared_move(#s,#w)) => #cx_unattributed(#s,#w)) } }"),
  ("cx_unattributed_new_safe_is_reviewed",
   "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_new_safe(#s,#w) && !#cx_declared_move(#s,#w)) => #cx_review(#s,#w)) } }"),
  ("cx_review_is_never_accepted",
   "forall (#s: cx_step) { forall (#w: cx_row) { (#cx_review(#s,#w) => !#cx_accepted(#s,#w)) } }"),
  ("cx_dependency_change_listed",
   "forall (#s: cx_step) { forall (#w: cx_row) { ((#cx_moved(#s,#w) && #cx_dependency_changed(#s,#w)) => #cx_dependency_listed(#s,#w)) } }"),
  ("cx_refuse_libc_redeclared",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (((#cx_declares(#b,#g) && #cx_libc_name(#g)) && !#cx_libc_owner(#b)) => #cx_refused(#b,#g)) } }"),
  ("cx_refuse_conflicting_declarations",
   "forall (#b: cx_bundle) { forall (#c: cx_bundle) { forall (#g: cx_fn) { ((((#cx_declares(#b,#g) && #cx_declares(#c,#g)) && !#cx_text(#b,#g) == #cx_text(#c,#g)) && !#cx_stamps_decide(#g)) => (#cx_refused(#b,#g) && #cx_refused(#c,#g))) } } }"),
  ("cx_refused_is_unknown",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_refused(#b,#g) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_contribution(#b,#g))) } }"),
  ("cx_composed_bounds_every_member",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_declares(#b,#g) => #cx_rank(#cx_contribution(#b,#g)) <= #cx_rank(#cx_composed(#g))) } }"),
  ("cx_composed_bounds_the_host",
   "forall (#g: cx_fn) { #cx_rank(#cx_host(#g)) <= #cx_rank(#cx_composed(#g)) }"),
  ("cx_composed_is_a_member",
   "forall (#g: cx_fn) { (#cx_composed(#g) == #cx_host(#g) || exists (#b: cx_bundle) { (#cx_declares(#b,#g) && #cx_composed(#g) == #cx_contribution(#b,#g)) }) }"),
  ("cx_every_reason_is_kept",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_declares(#b,#g) => #cx_reasons_kept(#b,#g)) } }"),
  ("cx_uncertified_plugin_is_pending",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (((#cx_plugin(#b) && #cx_proposes_decision(#b,#g)) && !#cx_checked(#b,#g)) => #cx_checker_verdict(#b,#g) == #cx_pending) } }"),
  ("cx_checker_bounds_contribution",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { #cx_rank(#cx_checker_verdict(#b,#g)) <= #cx_rank(#cx_contribution(#b,#g)) } }"),
  ("cx_refuse_misclassified_record",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (!#cx_recorded_verdict(#b,#g) == #cx_checker_verdict(#b,#g) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_contribution(#b,#g))) } }"),
  ("cx_satisfied_is_every_check",
   "forall (#d: cx_dep) { (#cx_satisfied(#d) => ((((#cx_loaded(#d) && #cx_in_range(#d)) && #cx_digest_matches(#d)) && (#cx_declared_crate(#d) && #cx_owner_named(#d))) && !#cx_held(#cx_target_bundle(#d),#cx_target(#d)))) }"),
  ("cx_every_check_is_satisfied",
   "forall (#d: cx_dep) { (((((#cx_loaded(#d) && #cx_in_range(#d)) && #cx_digest_matches(#d)) && (#cx_declared_crate(#d) && #cx_owner_named(#d))) && !#cx_held(#cx_target_bundle(#d),#cx_target(#d))) => #cx_satisfied(#d)) }"),
  ("cx_unsatisfied_holds_the_dependent",
   "forall (#d: cx_dep) { (!#cx_satisfied(#d) => #cx_held(#cx_dependent_bundle(#d),#cx_dependent(#d))) }"),
  ("cx_held_dependency_holds_dependents",
   "forall (#d: cx_dep) { (#cx_held(#cx_target_bundle(#d),#cx_target(#d)) => #cx_held(#cx_dependent_bundle(#d),#cx_dependent(#d))) }"),
  ("cx_cycle_holds",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_on_cycle(#b,#g) => #cx_held(#b,#g)) } }"),
  ("cx_refuse_libc_target_elsewhere",
   "forall (#d: cx_dep) { ((#cx_libc_name(#cx_target(#d)) && !#cx_libc_owner(#cx_target_bundle(#d))) => !#cx_owner_named(#d)) }"),
  ("cx_missing_is_unknown",
   "forall (#d: cx_dep) { (!#cx_loaded(#d) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }"),
  ("cx_out_of_range_is_unknown",
   "forall (#d: cx_dep) { (!#cx_in_range(#d) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }"),
  ("cx_undeclared_crate_is_unknown",
   "forall (#d: cx_dep) { ((!#cx_declared_crate(#d) || !#cx_owner_named(#d)) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }"),
  ("cx_digest_change_is_at_least_pending",
   "forall (#d: cx_dep) { (!#cx_digest_matches(#d) => #cx_rank(#cx_pending) <= #cx_rank(#cx_hold_verdict(#cx_dependent_bundle(#d),#cx_dependent(#d)))) }"),
  ("cx_cycle_is_unknown",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_on_cycle(#b,#g) => #cx_rank(#cx_unknown) <= #cx_rank(#cx_hold_verdict(#b,#g))) } }"),
  ("cx_hold_is_pending_or_unknown",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => (#cx_hold_verdict(#b,#g) == #cx_pending || #cx_hold_verdict(#b,#g) == #cx_unknown)) } }"),
  ("cx_pending_hold_is_human_resolvable",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { ((#cx_held(#b,#g) && #cx_hold_verdict(#b,#g) == #cx_pending) => #cx_human_resolvable(#b,#g)) } }"),
  ("cx_hold_bounds_contribution",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => #cx_rank(#cx_hold_verdict(#b,#g)) <= #cx_rank(#cx_contribution(#b,#g))) } }"),
  ("cx_held_is_never_safe",
   "forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => !#cx_contribution(#b,#g) == #cx_safe) } }")
]

theorem formulaTable_count : formulaTable.length = 60 := by decide

end QCore.MeasureConnect
