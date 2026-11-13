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
# A list-length bound under a declared list-finiteness premise
(card W45-FB-LIST-FINITENESS-PREMISE)

The abstract rule implemented by `crates/qcore-c-gateway/src/list_length_bound.rs`,
gateway rule 48. Beside the resource verdict, the rule records, for an
intrusive-list traversal `for (C = H->next; C != H; C = C->next)` (or its
`_safe` form) over a head a list-finiteness premise declares, the bound "at
most `|H|` iterations", where the loop's frame (its body and every function it
can call) is read in the unit and writes no list link. It moves no verdict and
states no progress property.

## The model

The links are a successor function `next : α → α`; `links j` are the links
the `j`-th test sees (`links 0` at loop entry). The cursor starts at
`links 0 h` and, after the `j`-th body, steps to `links (j+1)` of itself; the
safe form saves the successor before the body and steps to it. A run of `m`
iterations is one whose first `m` tests hold (`cursor ≠ h`).

**The declared premise** is `WellFormed (links 0) h n`: following the links
from the head meets it again first after `n + 1` steps — a list of `n` nodes,
finite and well formed. **The frame check** establishes that no body writes a
link: `links (j+1) = links j` (stated, not proved, from the syntactic check
and the premises rule 48 names).

## What is proved

* **A frame that writes no link fixes the links** (`a_link_preserving_frame_fixes_the_links`),
  and the cursor is then the `j+1`-th step of the walk from the head
  (`cursor_is_the_walk`); the safe form's cursor is the plain one
  (`safe_cursor_is_the_plain_cursor`).
* **The bound.** Every run has at most `n` iterations (`run_length_bounded`,
  for the safe form too: `safe_run_length_bounded`); the loop meets the head
  after exactly `n` iterations (`the_loop_exits_after_the_length`,
  `the_full_run_has_the_length`); a prefix of a run is a run
  (`a_prefix_of_a_run_is_a_run`), so an early exit only shortens it.
* **The measure** `n - j` is positive while the test holds
  (`measure_positive_while_running`) and decreases by one per iteration
  (`measure_decreases_by_one`).
* **Each premise is necessary.** A body that appends a node per iteration keeps
  every links state finite and well formed and still never meets the head
  (`an_inserting_body_never_exits`, `every_inserted_state_is_well_formed`); a
  list that is not well formed, a cycle that misses the head, never exits
  (`an_ill_formed_list_never_exits`).
* **The decision** mirrors the Rust rule's precedence; an admitted shape meets
  every gate (`an_admitted_shape_meets_the_premises`), each refusal shape is
  refused, and `decideVectors` equals the Rust `LIST_DECIDE_VECTORS`
  (`decide_vectors_hold`).
* **No verdict moves** (`recording_a_fact_keeps_the_verdict`,
  `recording_facts_keeps_the_verdict`).
-/

namespace QCore.ListLengthBound

/-! ## Walks and well-formed lists -/

/-- Following `next` `k` times from `x`. -/
def walk {α : Type} (next : α → α) (x : α) : Nat → α
  | 0 => x
  | k + 1 => next (walk next x k)

/-- The declared premise: following the links from the head `h` meets it
again first after `n + 1` steps, so the list holds `n` nodes. -/
def WellFormed {α : Type} (next : α → α) (h : α) (n : Nat) : Prop :=
  walk next h (n + 1) = h ∧ ∀ k, 1 ≤ k → k ≤ n → walk next h k ≠ h

/-! ## The loop -/

/-- The plain traversal's cursor at the `j`-th test. -/
def cursor {α : Type} (links : Nat → α → α) (h : α) : Nat → α
  | 0 => links 0 h
  | j + 1 => links (j + 1) (cursor links h j)

/-- The safe traversal's cursor and saved successor at the `j`-th test: the
successor is read before the body, and the cursor steps to it. -/
def safePair {α : Type} (links : Nat → α → α) (h : α) : Nat → α × α
  | 0 => (links 0 h, links 0 (links 0 h))
  | j + 1 =>
    let saved := (safePair links h j).2
    (saved, links (j + 1) saved)

/-- `m` iterations ran: the first `m` tests held. -/
def Run {α : Type} (c : Nat → α) (h : α) (m : Nat) : Prop :=
  ∀ j, j < m → c j ≠ h

/-- The frame writes no link: every body leaves the links as it found them. -/
def FrameKeepsLinks {α : Type} (links : Nat → α → α) : Prop :=
  ∀ j, links (j + 1) = links j

theorem a_link_preserving_frame_fixes_the_links {α : Type} (links : Nat → α → α)
    (hf : FrameKeepsLinks links) : ∀ j, links j = links 0 := by
  intro j
  induction j with
  | zero => rfl
  | succ k ih => rw [hf k, ih]

theorem cursor_is_the_walk {α : Type} (links : Nat → α → α) (h : α)
    (hf : FrameKeepsLinks links) : ∀ j, cursor links h j = walk (links 0) h (j + 1) := by
  have fixed := a_link_preserving_frame_fixes_the_links links hf
  intro j
  induction j with
  | zero => simp [cursor, walk]
  | succ k ih =>
    simp only [cursor]
    rw [ih, fixed (k + 1)]
    rfl

theorem safe_cursor_is_the_plain_cursor {α : Type} (links : Nat → α → α) (h : α)
    (hf : FrameKeepsLinks links) :
    ∀ j, (safePair links h j).1 = cursor links h j ∧
      (safePair links h j).2 = links 0 (cursor links h j) := by
  have fixed := a_link_preserving_frame_fixes_the_links links hf
  intro j
  induction j with
  | zero => simp [safePair, cursor]
  | succ k ih =>
    obtain ⟨_, h2⟩ := ih
    simp only [safePair, cursor]
    rw [h2, fixed (k + 1)]
    exact ⟨rfl, rfl⟩

theorem a_prefix_of_a_run_is_a_run {α : Type} (c : Nat → α) (h : α) (m k : Nat)
    (hr : Run c h m) (hk : k ≤ m) : Run c h k :=
  fun j hj => hr j (Nat.lt_of_lt_of_le hj hk)

theorem the_loop_exits_after_the_length {α : Type} (links : Nat → α → α) (h : α) (n : Nat)
    (hw : WellFormed (links 0) h n) (hf : FrameKeepsLinks links) :
    cursor links h n = h := by
  rw [cursor_is_the_walk links h hf n]
  exact hw.1

theorem the_full_run_has_the_length {α : Type} (links : Nat → α → α) (h : α) (n : Nat)
    (hw : WellFormed (links 0) h n) (hf : FrameKeepsLinks links) :
    Run (cursor links h) h n := by
  intro j hj
  rw [cursor_is_the_walk links h hf j]
  exact hw.2 (j + 1) (Nat.succ_le_succ (Nat.zero_le j)) hj

theorem run_length_bounded {α : Type} (links : Nat → α → α) (h : α) (n m : Nat)
    (hw : WellFormed (links 0) h n) (hf : FrameKeepsLinks links)
    (hr : Run (cursor links h) h m) : m ≤ n := by
  apply Nat.le_of_not_lt
  intro hlt
  exact hr n hlt (the_loop_exits_after_the_length links h n hw hf)

theorem safe_run_length_bounded {α : Type} (links : Nat → α → α) (h : α) (n m : Nat)
    (hw : WellFormed (links 0) h n) (hf : FrameKeepsLinks links)
    (hr : Run (fun j => (safePair links h j).1) h m) : m ≤ n := by
  apply run_length_bounded links h n m hw hf
  intro j hj
  have : (safePair links h j).1 ≠ h := hr j hj
  rw [(safe_cursor_is_the_plain_cursor links h hf j).1] at this
  exact this

/-! ## The measure -/

/-- The measure after `j` iterations. -/
def measure (n j : Nat) : Nat := n - j

theorem measure_positive_while_running {α : Type} (links : Nat → α → α) (h : α) (n j : Nat)
    (hw : WellFormed (links 0) h n) (hf : FrameKeepsLinks links)
    (hr : Run (cursor links h) h (j + 1)) : 0 < measure n j := by
  have := run_length_bounded links h n (j + 1) hw hf hr
  simp [measure]
  omega

theorem measure_decreases_by_one {α : Type} (links : Nat → α → α) (h : α) (n j : Nat)
    (hw : WellFormed (links 0) h n) (hf : FrameKeepsLinks links)
    (hr : Run (cursor links h) h (j + 1)) : measure n (j + 1) + 1 = measure n j := by
  have := run_length_bounded links h n (j + 1) hw hf hr
  simp [measure]
  omega

/-! ## Each premise is necessary -/

/-- A body that appends one node per iteration: before the `j`-th test the
list is `0 → 1 → … → j+1 → 0`, head `0`. -/
def appending (j : Nat) (k : Nat) : Nat := if k < j + 1 then k + 1 else 0

theorem appending_cursor : ∀ j, cursor appending 0 j = j + 1 := by
  intro j
  induction j with
  | zero => simp [cursor, appending]
  | succ k ih =>
    simp only [cursor]
    rw [ih]
    simp [appending]

theorem walk_appending (j : Nat) : ∀ k, k ≤ j + 1 → walk (appending j) 0 k = k := by
  intro k
  induction k with
  | zero => intro _; rfl
  | succ i ih =>
    intro hk
    simp only [walk]
    rw [ih (by omega)]
    simp [appending]
    omega

theorem every_inserted_state_is_well_formed (j : Nat) :
    WellFormed (appending j) 0 (j + 1) := by
  constructor
  · show appending j (walk (appending j) 0 (j + 1)) = 0
    rw [walk_appending j (j + 1) (Nat.le_refl _)]
    simp [appending]
  · intro k hk hkn
    rw [walk_appending j k hkn]
    omega

theorem an_inserting_body_never_exits : ∀ m, Run (cursor appending 0) 0 m := by
  intro m j _
  rw [appending_cursor j]
  omega

/-- A cycle that misses the head: `0 → 1 → 1 → …`. -/
def missesTheHead (_ _ : Nat) : Nat := 1

theorem an_ill_formed_list_never_exits : ∀ m, Run (cursor missesTheHead 0) 0 m := by
  intro m j _
  have : ∀ i, cursor missesTheHead 0 i = 1 := by
    intro i
    induction i with
    | zero => simp [cursor, missesTheHead]
    | succ k ih => simp only [cursor]; rw [ih]; simp [missesTheHead]
  rw [this j]
  omega

theorem the_ill_formed_list_has_no_length (n : Nat) : ¬ WellFormed (missesTheHead 0) 0 n := by
  intro hw
  have : ∀ i, 1 ≤ i → walk (missesTheHead 0) 0 i = 1 := by
    intro i hi
    induction i with
    | zero => omega
    | succ k ih =>
      simp only [walk]
      simp [missesTheHead]
  have := hw.1
  rw [‹∀ i, 1 ≤ i → walk (missesTheHead 0) 0 i = 1› (n + 1) (by omega)] at this
  omega

/-! ## The decision -/

inductive ListRefusal where
  | notAListTraversal
  | cursorWritten
  | headNotMember
  | headRootNotStable
  | premiseNotDeclared
  | opaqueControl
  | labelInLoop
  | frameWritesLinks
  | frameIndirectCall
  | frameCalleeUndefined
  deriving DecidableEq, Repr

def ListRefusal.code : ListRefusal → String
  | .notAListTraversal => "not-a-list-traversal"
  | .cursorWritten => "cursor-written"
  | .headNotMember => "head-not-member"
  | .headRootNotStable => "head-root-not-stable"
  | .premiseNotDeclared => "premise-not-declared"
  | .opaqueControl => "opaque-control"
  | .labelInLoop => "label-in-loop"
  | .frameWritesLinks => "frame-writes-links"
  | .frameIndirectCall => "frame-indirect-call"
  | .frameCalleeUndefined => "frame-callee-undefined"

/-- The loop, abstracted to what the decision reads; the Rust `ListShape`,
field for field. -/
structure ListShape where
  listTraversal : Bool
  cursorStable : Bool
  headMember : Bool
  headRootStable : Bool
  premiseDeclared : Bool
  opaqueControl : Bool
  labelInLoop : Bool
  frameWritesLinks : Bool
  frameIndirectCall : Bool
  frameCalleeUndefined : Bool
  deriving DecidableEq, Repr

/-- The first failing gate, in the Rust precedence. -/
def decide (s : ListShape) : Option ListRefusal :=
  if !s.listTraversal then some .notAListTraversal
  else if !s.cursorStable then some .cursorWritten
  else if !s.headMember then some .headNotMember
  else if !s.headRootStable then some .headRootNotStable
  else if !s.premiseDeclared then some .premiseNotDeclared
  else if s.opaqueControl then some .opaqueControl
  else if s.labelInLoop then some .labelInLoop
  else if s.frameWritesLinks then some .frameWritesLinks
  else if s.frameIndirectCall then some .frameIndirectCall
  else if s.frameCalleeUndefined then some .frameCalleeUndefined
  else none

theorem an_admitted_shape_meets_the_premises (s : ListShape) (h : decide s = none) :
    s.listTraversal = true ∧ s.cursorStable = true ∧ s.headMember = true ∧
      s.headRootStable = true ∧ s.premiseDeclared = true ∧ s.opaqueControl = false ∧
      s.labelInLoop = false ∧ s.frameWritesLinks = false ∧ s.frameIndirectCall = false ∧
      s.frameCalleeUndefined = false := by
  cases s with
  | mk a b c d e f g i j k =>
    cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> cases f <;> cases g <;>
      cases i <;> cases j <;> cases k <;> simp [decide] at h ⊢

theorem an_undeclared_head_is_refused (s : ListShape) (ht : s.listTraversal = true)
    (hc : s.cursorStable = true) (hm : s.headMember = true) (hr : s.headRootStable = true)
    (hp : s.premiseDeclared = false) : decide s = some .premiseNotDeclared := by
  simp [decide, ht, hc, hm, hr, hp]

theorem a_frame_writing_links_is_refused (s : ListShape) (h : s.frameWritesLinks = true) :
    decide s ≠ none := by
  intro hn
  have := (an_admitted_shape_meets_the_premises s hn).2.2.2.2.2.2.2.1
  rw [h] at this
  contradiction

theorem an_undefined_callee_is_refused (s : ListShape) (h : s.frameCalleeUndefined = true) :
    decide s ≠ none := by
  intro hn
  have := (an_admitted_shape_meets_the_premises s hn).2.2.2.2.2.2.2.2.2
  rw [h] at this
  contradiction

theorem a_non_traversal_is_refused (s : ListShape) (h : s.listTraversal = false) :
    decide s = some .notAListTraversal := by
  simp [decide, h]

/-! ## No verdict moves -/

/-- The four published verdicts; `accused` is `Unsafe` (a Lean keyword). -/
inductive Verdict where
  | safe
  | pending
  | unknown
  | accused
  deriving DecidableEq, Repr

/-- A function's published row: its custody verdict and the list facts
recorded beside it. -/
structure Row where
  verdict : Verdict
  facts : List (Option ListRefusal)

def record (r : Row) (fact : Option ListRefusal) : Row :=
  { r with facts := r.facts ++ [fact] }

theorem recording_a_fact_keeps_the_verdict (r : Row) (fact : Option ListRefusal) :
    (record r fact).verdict = r.verdict := by
  simp [record]

theorem recording_facts_keeps_the_verdict (r : Row) (facts : List (Option ListRefusal)) :
    (facts.foldl record r).verdict = r.verdict := by
  induction facts generalizing r with
  | nil => simp
  | cons f rest ih =>
    simp only [List.foldl]
    rw [ih]
    exact recording_a_fact_keeps_the_verdict r f

/-! ## Shared vectors -/

def shapeOf : List Bool → ListShape
  | [a, b, c, d, e, f, g, i, j, k] => ⟨a, b, c, d, e, f, g, i, j, k⟩
  | _ => ⟨false, false, false, false, false, false, false, false, false, false⟩

def outcomeCode : Option ListRefusal → String
  | none => "admitted"
  | some r => r.code

/-- Identical to the Rust `LIST_DECIDE_VECTORS`. -/
def decideVectors : List (List Bool × String) := [
  ([true, true, true, true, true, false, false, false, false, false], "admitted"),
  ([false, false, false, false, false, false, false, false, false, false], "not-a-list-traversal"),
  ([true, false, true, true, true, false, false, false, false, false], "cursor-written"),
  ([true, true, false, false, false, false, false, false, false, false], "head-not-member"),
  ([true, true, true, false, true, false, false, false, false, false], "head-root-not-stable"),
  ([true, true, true, true, false, false, false, false, false, false], "premise-not-declared"),
  ([true, true, true, true, true, true, false, false, false, false], "opaque-control"),
  ([true, true, true, true, true, false, true, false, false, false], "label-in-loop"),
  ([true, true, true, true, true, false, false, true, false, true], "frame-writes-links"),
  ([true, true, true, true, true, false, false, false, true, true], "frame-indirect-call"),
  ([true, true, true, true, true, false, false, false, false, true], "frame-callee-undefined")
]

theorem decide_vectors_hold :
    decideVectors.all (fun (g, code) => outcomeCode (decide (shapeOf g)) == code) = true := by
  decide

end QCore.ListLengthBound
