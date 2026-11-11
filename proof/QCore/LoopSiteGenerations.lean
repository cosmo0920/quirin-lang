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
# Generation-indexed acquisition sites (card CG04-LOOP-SITE-GENERATIONS)

The abstract rule implemented by `crates/qcore-c-gateway/src/loop_generations.rs`
(`decide`, `previous`, `older`, `base`, `generation`) and its route-G adapter
`legacy_loop_generations.rs` (`shift`, run before a node's transfer).

The custody flow names a resource by the CFG node that acquires it, its *site*.
A node on a CFG cycle runs more than once. The flow used to keep one abstract
identity per site: the re-acquisition wrote it live over a live earlier
generation, and a refuted nullable acquisition on the failure edge then removed
it, earlier generation included. `for (;;) { p = malloc(n); if (p == NULL)
break; use(p); }` was reported `Safe` while every earlier generation leaks
(`the_one_site_flow_hides_the_leak`).

Each site now has three identities:

* **current**, the object the node produced last on the path: one object per
  execution, so precise (strong) updates are sound;
* **previous**, the one produced the time before: also one object per
  execution, also precise;
* **older**, a summary of every generation before that: it may stand for many
  objects, so nothing updates it precisely. It only grows, towards "possibly
  live", never towards "discharged" (`the_summary_never_discharges`).

A re-acquisition (the site is already present) **shifts**: previous is merged
into older, current becomes previous, and the node creates a fresh current.
The failure edge of a nullable acquisition removes only the fresh current
(`the_failure_edge_removes_only_the_new_generation`). The merge is refused
(`Unknown`) when the merged summary may be live and a place other than a
binding (an aggregate edge, a pending reallocation, a publication, an out slot)
holds it, which could update the summary precisely, or when one side may be
live and the other carries an unresolved effect, which would hide it (`decide`,
monotone: `the_decision_is_monotone`).
A binding to the summary is unknown, and every custody effect through an
unknown binding refuses on its own.

## What this is not

A model of one acquisition site's generations and of the decision table. The
flow's other state (bindings, aggregate edges, collections) is the existing
one; that `shift` renames every mention of an identity is a premise of the Rust
code, reviewed, not proved, as is that the flow reports a refusal whenever it
leaves an identity unresolved outside this rule. Faithful extraction of C and
the Rust-Lean correspondence are review premises (trust boundary 28).
`stepVectors`, `identityVectors` and `shapeVectors` bind the Rust tables and
the gateway's verdicts on the pinned shapes mechanically
(`tests/unit/loop_generations.rs`, `tests/loop_site_generations.rs`).
-/

namespace QCore.LoopSiteGenerations

/-! ## Identities -/

/-- `PREVIOUS_TAG`; the older tag is twice it. For a site below the tag, the
Rust bitwise `|` of a tag is this addition. -/
def tag : Nat := 536870912

theorem tag_is_two_to_the_29 : tag = 2 ^ 29 := by decide

inductive Generation where
  | current
  | previous
  | older
  deriving DecidableEq, Repr

def Generation.code : Generation → Nat
  | .current => 0
  | .previous => 1
  | .older => 2

def previous (site : Nat) : Nat := site + tag
def older (site : Nat) : Nat := site + 2 * tag
def base (identity : Nat) : Nat := identity % tag

def generation (identity : Nat) : Generation :=
  if identity < tag then .current else if identity < 2 * tag then .previous else .older

/-- `IDENTITY_VECTORS` of `loop_generations.rs`: `(identity, base, code)`. -/
def identityVectors : List (Nat × Nat × Nat) := [
  (0, 0, 0),
  (7, 7, 0),
  (536870919, 7, 1),
  (1073741831, 7, 2),
  (536870912, 0, 1),
  (1073741824, 0, 2),
  (1073754169, 12345, 2)
]

theorem identity_vectors_hold :
    identityVectors.all (fun (i, b, c) => base i == b && (generation i).code == c) = true := by
  decide

/-- The three identities of an indexable site name it, each its own generation. -/
theorem generations_name_their_site (site : Nat) (h : site < tag) :
    base (previous site) = site ∧ base (older site) = site ∧ base site = site
      ∧ generation site = .current ∧ generation (previous site) = .previous
      ∧ generation (older site) = .older := by
  refine ⟨?_, ?_, Nat.mod_eq_of_lt h, ?_, ?_, ?_⟩
  · simp [base, previous, Nat.mod_eq_of_lt h]
  · simp [base, older, Nat.mod_eq_of_lt h]
  · simp [generation, h]
  · have : ¬ site + tag < tag := by omega
    have : site + tag < 2 * tag := by omega
    simp [generation, previous, *]
  · have : ¬ site + 2 * tag < tag := by omega
    have : ¬ site + 2 * tag < 2 * tag := by omega
    simp [generation, older, *]

/-! ## The decision -/

inductive Step where
  | fresh
  | shift
  | shiftRefused
  deriving DecidableEq, Repr

/-- `decide`: whether the site is present; then, for previous and for older,
whether it may be live, whether a place other than a binding holds it, and
whether it carries an unresolved effect. Every input only grows as the state
grows, and the formula is positive in each, so the decision is monotone. -/
def decide (present pLive pHeld pUnresolved oLive oHeld oUnresolved : Bool) : Step :=
  if !present then .fresh
  else if ((pLive || oLive) && (pHeld || oHeld)) || (pLive && oUnresolved)
      || (oLive && pUnresolved) then .shiftRefused
  else .shift

/-- `STEP_VECTORS` of `loop_generations.rs`, row by row. -/
def stepVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Bool × Step) := [
  (false, false, false, false, false, false, false, .fresh),
  (false, true, true, false, true, true, false, .fresh),
  (true, false, false, false, false, false, false, .shift),
  (true, true, false, false, false, false, false, .shift),
  (true, false, false, false, true, false, false, .shift),
  (true, true, false, false, true, false, false, .shift),
  (true, false, true, false, false, true, false, .shift),
  (true, false, false, true, false, false, true, .shift),
  (true, true, false, true, false, false, false, .shift),
  (true, true, false, true, true, false, true, .shiftRefused),
  (true, true, true, false, false, false, false, .shiftRefused),
  (true, true, false, false, false, true, false, .shiftRefused),
  (true, false, true, false, true, false, false, .shiftRefused),
  (true, false, false, false, true, true, false, .shiftRefused),
  (true, true, false, false, false, false, true, .shiftRefused),
  (true, false, false, true, true, false, false, .shiftRefused),
  (true, true, true, false, true, true, false, .shiftRefused)
]

theorem step_vectors_hold :
    stepVectors.all (fun (p, a, b, c, d, e, f, s) => decide p a b c d e f == s) = true := by
  decide

/-- The merge is refused exactly when a possibly-live summary meets a holder,
or one possibly-live side meets the other's unresolved effect. -/
theorem refused_iff_live_and_exposed (a b c d e f : Bool) :
    decide true a b c d e f = .shiftRefused ↔
      (((a || d) && (b || e)) || (a && f) || (d && c)) = true := by
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases f <;> simp [decide]

/-- Monotone: a refusal is never withdrawn by more liveness, more holders or
more unresolved effects, so the settled state decides as every iterate did. -/
theorem the_decision_is_monotone (a b c d e f a' b' c' d' e' f' : Bool)
    (ha : a = true → a' = true) (hb : b = true → b' = true) (hc : c = true → c' = true)
    (hd : d = true → d' = true) (he : e = true → e' = true) (hf : f = true → f' = true)
    (h : decide true a b c d e f = .shiftRefused) :
    decide true a' b' c' d' e' f' = .shiftRefused := by
  rw [refused_iff_live_and_exposed] at h ⊢
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases f <;>
    simp_all

/-! ## One site's abstract state -/

/-- One identity: whether it may be live (the flow's `LIVE` bit), whether a
place other than a binding holds it, and whether it carries an unresolved
effect (no leak is claimed for it). Its other status bits never make an exit
report, so they are left out. -/
structure Entry where
  live : Bool
  held : Bool
  unresolved : Bool
  deriving DecidableEq, Repr

def Entry.join (a b : Entry) : Entry :=
  ⟨a.live || b.live, a.held || b.held, a.unresolved || b.unresolved⟩

def merge : Option Entry → Option Entry → Option Entry
  | none, b => b
  | a, none => a
  | some a, some b => some (a.join b)

def owes : Option Entry → Bool
  | none => false
  | some e => e.live && !e.unresolved
def heldAt : Option Entry → Bool
  | none => false
  | some e => e.held
def unresolvedAt : Option Entry → Bool
  | none => false
  | some e => e.unresolved
def liveAt : Option Entry → Bool
  | none => false
  | some e => e.live

/-- One acquisition site's three generations, and whether a refusal was
reported on the way. -/
structure Gens where
  current : Option Entry
  previous : Option Entry
  older : Option Entry
  refused : Bool
  deriving DecidableEq, Repr

def Gens.empty : Gens := ⟨none, none, none, false⟩

def Gens.join (a b : Gens) : Gens :=
  ⟨merge a.current b.current, merge a.previous b.previous, merge a.older b.older,
    a.refused || b.refused⟩

def shiftRefused (g : Gens) : Bool :=
  decide true (liveAt g.previous) (heldAt g.previous) (unresolvedAt g.previous)
    (liveAt g.older) (heldAt g.older) (unresolvedAt g.older) == .shiftRefused

/-- `shift`: previous into older, current into previous. A refused merge
leaves the summary unresolved and reports. -/
def shift (g : Gens) : Gens :=
  let merged := merge g.older g.previous
  let older := if shiftRefused g then merged.map (fun e => { e with unresolved := true })
    else merged
  ⟨none, g.current, older, g.refused || shiftRefused g⟩

def fresh : Entry := ⟨true, false, false⟩

/-- The acquisition node: shift when the site is present, then a fresh live
current generation. -/
def acquire (g : Gens) : Gens :=
  let g := if g.current.isSome then shift g else g
  { g with current := some fresh }

/-- A refused acquisition node (`settle_refused`): the call C ran may have
produced a generation the refused transfer did not create, so the current
generation is present, possibly live, with an unresolved effect, and the
refusal is reported. -/
def acquireRefused (g : Gens) : Gens :=
  let g := if g.current.isSome then shift g else g
  { g with current := some ⟨true, false, true⟩, refused := true }

/-- The failure edge of a nullable acquisition: the fresh generation was never
produced. -/
def refuteNull (g : Gens) : Gens := { g with current := none }

/-- A release through the current binding: one object, a strong update. -/
def releaseCurrent (g : Gens) : Gens :=
  { g with current := g.current.map (fun e => { e with live := false }) }

/-- A release through a binding to the previous generation: one object too. -/
def releasePrevious (g : Gens) : Gens :=
  { g with previous := g.previous.map (fun e => { e with live := false }) }

/-- Store the current generation in a place that outlives the binding (a
field link): it stays live, and is held. -/
def linkCurrent (g : Gens) : Gens :=
  { g with current := g.current.map (fun e => { e with held := true }) }

/-- The operations of the flow on one site. None of them updates the summary
precisely: a binding to it is unknown, and an effect through it refuses. -/
inductive Op where
  | acquire
  | acquireRefused
  | refuteNull
  | releaseCurrent
  | releasePrevious
  | linkCurrent
  | refuseThroughSummary
  deriving DecidableEq, Repr

def apply : Op → Gens → Gens
  | .acquire, g => acquire g
  | .acquireRefused, g => acquireRefused g
  | .refuteNull, g => refuteNull g
  | .releaseCurrent, g => releaseCurrent g
  | .releasePrevious, g => releasePrevious g
  | .linkCurrent, g => linkCurrent g
  | .refuseThroughSummary, g =>
      { g with older := g.older.map (fun e => { e with unresolved := true }), refused := true }

def run (ops : List Op) (g : Gens) : Gens := ops.foldl (fun g op => apply op g) g

/-- The exit's report: a leak. -/
def leak (g : Gens) : Bool := owes g.current || owes g.previous || owes g.older

/-- The verdict at an exit: a leak is `Unsafe`, a refusal `Unknown`, else `Safe`. -/
inductive Verdict where
  | safe
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

def verdict (g : Gens) : Verdict :=
  if leak g then .«unsafe» else if g.refused then .unknown else .safe

/-! ## Re-acquisition keeps the earlier obligation -/

theorem merge_live (a b : Option Entry) :
    liveAt (merge a b) = (liveAt a || liveAt b) := by
  cases a <;> cases b <;> simp [merge, liveAt, Entry.join]

theorem shift_live_older (g : Gens) :
    liveAt (shift g).older = (liveAt g.older || liveAt g.previous) := by
  rw [← merge_live]
  simp only [shift]
  split
  · cases merge g.older g.previous <;> simp [liveAt]
  · rfl

/-- **A re-acquisition over a possibly-live generation cannot discharge it**:
the current generation moves to previous unchanged, and previous's live flag
into the summary; no live flag is lost. -/
theorem reacquisition_keeps_the_earlier_obligation (g : Gens) (h : g.current.isSome = true) :
    (acquire g).previous = g.current
      ∧ liveAt (acquire g).older = (liveAt g.older || liveAt g.previous) := by
  refine ⟨?_, ?_⟩
  · simp [acquire, h, shift]
  · simp only [acquire, h, ite_true]
    exact shift_live_older g

/-- A generation owed before the re-acquisition is owed after it, or a refusal
is reported: never silently discharged. -/
theorem an_owed_generation_stays_owed_or_refused (g : Gens) (h : g.current.isSome = true)
    (howes : (owes g.current || owes g.previous || owes g.older) = true) :
    leak (acquire g) = true ∨ (acquire g).refused = true := by
  rcases g with ⟨c, p, o, r⟩
  cases c with
  | none => simp at h
  | some ce =>
    rcases ce with ⟨cl, ch, cu⟩
    cases p with
    | none =>
      cases o with
      | none =>
        cases cl <;> cases cu <;>
          simp_all [acquire, shift, shiftRefused, leak, owes, merge, heldAt, unresolvedAt, liveAt, decide]
      | some oe =>
        rcases oe with ⟨ol, oh, ou⟩
        cases cl <;> cases cu <;> cases ol <;> cases oh <;> cases ou <;>
          simp_all [acquire, shift, shiftRefused, leak, owes, merge, heldAt, unresolvedAt, liveAt, decide]
    | some pe =>
      rcases pe with ⟨pl, ph, pu⟩
      cases o with
      | none =>
        cases cl <;> cases cu <;> cases pl <;> cases ph <;> cases pu <;>
          simp_all [acquire, shift, shiftRefused, leak, owes, merge, heldAt, unresolvedAt, liveAt, decide]
      | some oe =>
        rcases oe with ⟨ol, oh, ou⟩
        cases cl <;> cases cu <;> cases pl <;> cases ph <;> cases pu <;> cases ol <;> cases oh <;>
          cases ou <;>
          simp_all [acquire, shift, shiftRefused, leak, owes, merge, heldAt, unresolvedAt, liveAt, decide,
            Entry.join]

/-! ## The failure edge removes only the new generation -/

theorem the_failure_edge_removes_only_the_new_generation (g : Gens) :
    (refuteNull (acquire g)).previous = (acquire g).previous
      ∧ (refuteNull (acquire g)).older = (acquire g).older
      ∧ (refuteNull (acquire g)).current = none := by
  simp [refuteNull]

/-- In particular a live earlier generation survives a refuted re-acquisition. -/
theorem a_live_earlier_generation_survives_the_failure_edge (g : Gens)
    (h : g.current.isSome = true) (hl : liveAt g.current = true) :
    liveAt (refuteNull (acquire g)).previous = true := by
  have := (reacquisition_keeps_the_earlier_obligation g h).1
  simp [refuteNull, this, hl]

/-! ## The summary never discharges -/

theorem map_unresolved_live (x : Option Entry) :
    liveAt (x.map (fun e => { e with unresolved := true })) = liveAt x := by
  cases x <;> simp [liveAt]

/-- Every operation keeps a possibly-live summary possibly live. -/
theorem the_summary_never_discharges (op : Op) (g : Gens) (h : liveAt g.older = true) :
    liveAt (apply op g).older = true := by
  cases op with
  | acquire =>
    simp only [apply, acquire]
    split
    · simp [shift_live_older, h]
    · simp [h]
  | acquireRefused =>
    simp only [apply, acquireRefused]
    split
    · simp [shift_live_older, h]
    · simp [h]
  | refuteNull => simp [apply, refuteNull, h]
  | releaseCurrent => simp [apply, releaseCurrent, h]
  | releasePrevious => simp [apply, releasePrevious, h]
  | linkCurrent => simp [apply, linkCurrent, h]
  | refuseThroughSummary => simp [apply, map_unresolved_live, h]

theorem the_summary_never_discharges_along_a_run (ops : List Op) (g : Gens)
    (h : liveAt g.older = true) : liveAt (run ops g).older = true := by
  induction ops generalizing g with
  | nil => simpa [run]
  | cons op ops ih =>
    simp only [run, List.foldl_cons]
    exact ih _ (the_summary_never_discharges op g h)

/-- Every unresolved generation was reported: the flow's invariant that an
unresolved effect comes with a refusal, for one site. -/
def reported (g : Gens) : Prop :=
  (unresolvedAt g.current || unresolvedAt g.previous || unresolvedAt g.older) = true →
    g.refused = true

theorem unresolved_merge (a b : Option Entry) :
    unresolvedAt (merge a b) = (unresolvedAt a || unresolvedAt b) := by
  cases a <;> cases b <;> simp [merge, unresolvedAt, Entry.join]

theorem unresolved_marked (x : Option Entry) :
    unresolvedAt (x.map (fun e => { e with unresolved := true })) = x.isSome := by
  cases x <;> simp [unresolvedAt]

theorem unresolved_fresh : unresolvedAt (some fresh) = false := rfl
theorem unresolved_none : unresolvedAt none = false := rfl

theorem unresolved_released (x : Option Entry) :
    unresolvedAt (x.map (fun e => { e with live := false })) = unresolvedAt x := by
  cases x <;> rfl

theorem unresolved_linked (x : Option Entry) :
    unresolvedAt (x.map (fun e => { e with held := true })) = unresolvedAt x := by
  cases x <;> rfl

/-- Every operation keeps every unresolved generation reported. -/
theorem operations_keep_unresolved_reported (op : Op) (g : Gens) (hw : reported g) :
    reported (apply op g) := by
  rcases g with ⟨c, p, o, r⟩
  unfold reported at hw ⊢
  dsimp only at hw ⊢
  cases op with
  | acquire =>
    dsimp only [apply, acquire]
    split
    · dsimp only [shift]
      by_cases hs : shiftRefused ⟨c, p, o, r⟩ = true
      · intro _; simp [hs]
      · have hs' : shiftRefused ⟨c, p, o, r⟩ = false := by simpa using hs
        simp only [hs', Bool.false_eq_true, ite_false, Bool.or_false, unresolved_fresh,
          unresolved_merge, Bool.false_or]
        intro hu
        apply hw
        revert hu
        cases unresolvedAt c <;> cases unresolvedAt p <;> cases unresolvedAt o <;> simp
    · dsimp only
      rw [unresolved_fresh]
      intro hu
      apply hw
      revert hu
      cases unresolvedAt c <;> cases unresolvedAt p <;> cases unresolvedAt o <;> simp
  | acquireRefused => intro _; rfl
  | refuteNull =>
    dsimp only [apply, refuteNull]
    rw [unresolved_none]
    intro hu; apply hw; revert hu
    cases unresolvedAt c <;> cases unresolvedAt p <;> cases unresolvedAt o <;> simp
  | releaseCurrent =>
    dsimp only [apply, releaseCurrent]
    rw [unresolved_released]
    exact hw
  | releasePrevious =>
    dsimp only [apply, releasePrevious]
    rw [unresolved_released]
    exact hw
  | linkCurrent =>
    dsimp only [apply, linkCurrent]
    rw [unresolved_linked]
    exact hw
  | refuseThroughSummary => intro _; rfl

/-- The widened state: once the summary may be live, no run reaches an exit
reported `Safe`; a leak or a refusal is always reported. -/
theorem a_live_summary_is_never_safe (ops : List Op) (g : Gens)
    (h : liveAt g.older = true) (hw : reported g) :
    verdict (run ops g) ≠ .safe := by
  have hl := the_summary_never_discharges_along_a_run ops g h
  have hr : reported (run ops g) := by
    induction ops generalizing g with
    | nil => simpa [run] using hw
    | cons op ops ih =>
      simp only [run, List.foldl_cons]
      exact ih _ (the_summary_never_discharges op g h)
        (operations_keep_unresolved_reported op g hw)
        (the_summary_never_discharges_along_a_run ops _ (the_summary_never_discharges op g h))
  unfold verdict
  cases hk : leak (run ops g)
  · cases ho : (run ops g).older with
    | none => simp [ho, liveAt] at hl
    | some e =>
      rw [ho] at hl
      simp only [liveAt] at hl
      cases hu : e.unresolved
      · simp [leak, owes, ho, hl, hu] at hk
      · have : (run ops g).refused = true := hr (by simp [ho, unresolvedAt, hu])
        simp [this]
  · simp

/-! ## The pinned shapes -/

/-- The old flow: one identity per site, overwritten by the re-acquisition and
removed by the failure edge. -/
def oldAcquire (_ : Option Entry) : Option Entry := some fresh
def oldRefuteNull (_ : Option Entry) : Option Entry := none
def oldLeak : Option Entry → Bool := owes

/-- The concrete path of `for (;;) { p = acquire(); if (p == NULL) break;
use(p); }` (and of `do { p = acquire(); if (p) use(p); } while (p != NULL);`):
`n` successful acquisitions, then a failed one. Concretely, the `n` successful
generations are never released: `n` leaks. -/
def concreteLeaks (n : Nat) : Nat := n

def oldForBreak : Nat → Option Entry
  | 0 => oldRefuteNull (oldAcquire none)
  | n + 1 => oldRefuteNull (oldAcquire (oldForBreak n))

/-- **The defect**: the one-site flow reports no leak on any path. -/
theorem the_one_site_flow_hides_the_leak (n : Nat) :
    oldLeak (oldForBreak (n + 1)) = false ∧ concreteLeaks (n + 1) > 0 := by
  cases n <;> simp [oldForBreak, oldRefuteNull, oldLeak, owes, concreteLeaks]

def acquisitions (n : Nat) : Gens := run (List.replicate n .acquire) Gens.empty

def settled : Gens := ⟨some fresh, some fresh, some fresh, false⟩

theorem acquisitions_succ (n : Nat) : acquisitions (n + 1) = acquire (acquisitions n) := by
  simp [acquisitions, run, List.replicate_succ', List.foldl_append, apply]

/-- From the third acquisition on, the abstract state is the loop's fixed point. -/
theorem acquisitions_settle (n : Nat) : acquisitions (n + 3) = settled := by
  induction n with
  | zero => decide
  | succ n ih =>
    rw [show n + 1 + 3 = (n + 3) + 1 by omega, acquisitions_succ, ih]
    decide

/-- The generation-indexed path state of the first pinned shape. -/
def forBreak (n : Nat) : Gens := refuteNull (acquisitions (n + 1))

/-- **The fixed flow is exact on the pinned path**: it reports a leak exactly
when the concrete path leaks. -/
theorem for_break_reports_exactly_the_concrete_leak (n : Nat) :
    verdict (forBreak n) = (if concreteLeaks n > 0 then .«unsafe» else .safe) := by
  match n with
  | 0 => decide
  | 1 => decide
  | n + 2 =>
    have : forBreak (n + 2) = refuteNull settled := by
      simp only [forBreak]
      rw [show n + 2 + 1 = n + 3 by omega, acquisitions_settle]
    rw [this]
    simp [concreteLeaks]
    decide

/-- The flow joins every path at the loop's exit. Every path state is one of
three, and their join reports the leak. -/
theorem the_joined_exit_reports_the_leak :
    verdict ((forBreak 0).join ((forBreak 1).join (forBreak 2))) = .«unsafe»
      ∧ ∀ n, forBreak n = forBreak 0 ∨ forBreak n = forBreak 1 ∨ forBreak n = forBreak 2 := by
  refine ⟨by decide, ?_⟩
  intro n
  match n with
  | 0 => simp
  | 1 => simp
  | n + 2 =>
    right; right
    simp only [forBreak]
    rw [show n + 2 + 1 = n + 3 by omega, acquisitions_settle,
      show (2 : Nat) + 1 = 0 + 3 by rfl, acquisitions_settle]

/-- `while (flag) { p = acquire(); if (p == NULL) return; use(p); release(p); }`:
the header state after `n` iterations. -/
def releaseEachIteration (n : Nat) : Gens :=
  run (List.flatten (List.replicate n [.acquire, .releaseCurrent])) Gens.empty

def releasedEntry : Entry := ⟨false, false, false⟩

def releaseSettled : Gens := ⟨some releasedEntry, some releasedEntry, some releasedEntry, false⟩

theorem release_each_iteration_succ (n : Nat) :
    releaseEachIteration (n + 1) = releaseCurrent (acquire (releaseEachIteration n)) := by
  simp [releaseEachIteration, run, List.replicate_succ', List.flatten_append,
    List.foldl_append, apply]

theorem release_each_iteration_settles (n : Nat) :
    releaseEachIteration (n + 3) = releaseSettled := by
  induction n with
  | zero => decide
  | succ n ih =>
    rw [show n + 1 + 3 = (n + 3) + 1 by omega, release_each_iteration_succ, ih]
    decide

/-- A release in every iteration stays `Safe` on every path. -/
theorem release_each_iteration_is_safe (n : Nat) :
    verdict (releaseEachIteration n) = .safe := by
  match n with
  | 0 => decide
  | 1 => decide
  | 2 => decide
  | n + 3 => rw [release_each_iteration_settles]; decide

/-- `for (...) { p = acquire(); } release(p);`: only the last generation is
released; every earlier one leaks, and is reported. -/
def releaseAfterTheLoop (n : Nat) : Gens := releaseCurrent (acquisitions (n + 1))

theorem release_after_the_loop_reports_the_earlier_generations (n : Nat) :
    verdict (releaseAfterTheLoop n) = (if concreteLeaks n > 0 then .«unsafe» else .safe) := by
  match n with
  | 0 => decide
  | 1 => decide
  | n + 2 =>
    have : releaseAfterTheLoop (n + 2) = releaseCurrent settled := by
      simp only [releaseAfterTheLoop]
      rw [show n + 2 + 1 = n + 3 by omega, acquisitions_settle]
    rw [this]
    simp [concreteLeaks]
    decide

/-- `for (...) { n = acquire(); link(o, n); }`: a held, live earlier generation
reaching the summary is refused, never merged silently. -/
def linkEachIteration (n : Nat) : Gens :=
  run (List.flatten (List.replicate n [.acquire, .linkCurrent])) Gens.empty

theorem link_each_iteration_is_refused_from_the_third :
    (linkEachIteration 3).refused = true ∧ verdict (linkEachIteration 2) = .«unsafe» := by
  decide

/-- A reported refusal is never withdrawn by a later operation. -/
theorem refusals_persist (op : Op) (g : Gens) (h : g.refused = true) :
    (apply op g).refused = true := by
  cases op with
  | acquire =>
    simp only [apply, acquire]
    split
    · simp [shift, h]
    · simpa using h
  | acquireRefused => rfl
  | refuteNull => simpa [apply, refuteNull] using h
  | releaseCurrent => simpa [apply, releaseCurrent] using h
  | releasePrevious => simpa [apply, releasePrevious] using h
  | linkCurrent => simpa [apply, linkCurrent] using h
  | refuseThroughSummary => rfl

/-- A refused re-acquisition never leaves an exit `Safe`: the generation the
call may have produced is not claimed discharged, and the refusal stays. -/
theorem a_refused_acquisition_is_never_safe (ops : List Op) (g : Gens) :
    verdict (run ops (acquireRefused g)) ≠ .safe := by
  have hr : (run ops (acquireRefused g)).refused = true := by
    have h0 : (acquireRefused g).refused = true := rfl
    generalize acquireRefused g = s at h0 ⊢
    induction ops generalizing s with
    | nil => simpa [run] using h0
    | cons op ops ih =>
      simp only [run, List.foldl_cons]
      exact ih _ (refusals_persist op s h0)
  simp only [verdict, hr, ite_true]
  split <;> simp

/-- `SHAPE_VECTORS` of `tests/loop_site_generations.rs`: the pinned shape and
the verdict the model gives its joined exit (and the gateway its fixture). -/
def shapeVectors : List (String × Verdict) := [
  ("for-break", .«unsafe»),
  ("do-while", .«unsafe»),
  ("release-each-iteration", .safe),
  ("release-after-the-loop", .«unsafe»)
]

/-- The model's verdict of each pinned shape: the join of its path exits. -/
def shapeVerdict : String → Verdict
  | "for-break" => verdict ((forBreak 0).join ((forBreak 1).join (forBreak 2)))
  | "do-while" => verdict ((forBreak 0).join ((forBreak 1).join (forBreak 2)))
  | "release-each-iteration" =>
      verdict ((releaseEachIteration 0).join ((releaseEachIteration 1).join
        ((releaseEachIteration 2).join (releaseEachIteration 3))))
  | "release-after-the-loop" =>
      verdict ((releaseAfterTheLoop 0).join ((releaseAfterTheLoop 1).join (releaseAfterTheLoop 2)))
  | _ => .unknown

theorem shape_vectors_hold :
    shapeVectors.all (fun (s, v) => shapeVerdict s == v) = true := by
  decide

end QCore.LoopSiteGenerations
