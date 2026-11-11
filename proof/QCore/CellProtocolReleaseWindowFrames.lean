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

import QCore.CellResultSet

/-!
# Callee frames in a record's release window (card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES)

USER DECISION 2026-11-04 (verbatim, on the options (a) callee frame
declarations, (b) a member-actual narrowing, (c) keep the refusal): "(a)
Callee frames". A call in the window between an end's close and the holder's
release admits only where a declared or body-checked clause states that the
callee reads no end member of the record; calls without such a clause keep
refusing.

Gateway rule 38g. `quant cell CALLEE: TAG |-> #untouched;` states that the
callee, with everything it calls, neither reads nor writes a member a
declared protocol of `struct TAG` names, on every object of that type. Abstract
rule theorems over the Rust reading
(`crates/gateway-rules/rule-38-descriptor-cells/src/frame.rs`: `standing`,
`window`, `body_check`, `BODY_VECTORS`, `WINDOW_VECTORS`), not an extraction.

* **The frame's meaning** (`a_framed_call_keeps_every_end`,
  `a_framed_call_reads_no_end`): a callee whose footprint excludes the
  record's protocol members leaves every end of every object as it was, and
  its effect does not depend on any end.
* **The body check** (`an_established_body_names_no_member`,
  `an_established_body_calls_only_established_bodies`,
  `a_missing_body_is_never_established`, `a_recursive_body_is_never_established`):
  a body is established only when it names no protocol member, reads in full
  and calls only bodies that are themselves established.
* **The standing and the reading** (`no_declaration_no_frame`,
  `only_a_checked_or_accepted_frame_passes_unheld`,
  `a_declared_frame_is_held`, `acceptance_never_lifts_a_refutation`,
  `without_a_frame_the_window_refuses_as_before`).
* **Vectors.** `body_vectors_hold`, `window_vectors_hold` (Rust
  `frame::BODY_VECTORS`, `frame::WINDOW_VECTORS`).
-/

namespace QCore.CellProtocolReleaseWindowFrames

open QCore.DescriptorCells
open QCore.CellProtocolPerEnd (Fact empty adopt staleIdentity normalize join tableJoin)
open QCore.CellResultSet (Spec introduced reached holds store releaseHolder refusalOutcome
  fromOne fromZero absZero absMinusOne absBoth minusFive)

/-! ## What a frame means -/

/-- Memory: each object's members (abstract numbers). -/
abbrev Mem := Nat → Nat → Int

/-- A callee with its footprint `fp` (the members of the record's type it
may read or write): its writes stay inside the footprint and its effect on the
footprint reads only the footprint. -/
structure Callee where
  fp : Nat → Bool
  run : Mem → Mem
  writes_in_fp : ∀ m o x, fp x = false → run m o x = m o x
  reads_in_fp : ∀ m m', (∀ o x, fp x = true → m o x = m' o x) →
    ∀ o x, fp x = true → run m o x = run m' o x

/-- **A framed call keeps every end**: where the footprint excludes the
protocol members, every object's ends read after the call as before. -/
theorem a_framed_call_keeps_every_end (c : Callee) (ends : Nat → Bool)
    (h : ∀ x, ends x = true → c.fp x = false) (m : Mem) (o x : Nat) (hx : ends x = true) :
    c.run m o x = m o x :=
  c.writes_in_fp m o x (h x hx)

/-- **A framed call reads no end**: two memories that differ only on ends give
results that differ only on ends. -/
theorem a_framed_call_reads_no_end (c : Callee) (ends : Nat → Bool)
    (h : ∀ x, ends x = true → c.fp x = false) (m m' : Mem)
    (hm : ∀ o x, ends x = false → m o x = m' o x) (o x : Nat) (hx : ends x = false) :
    c.run m o x = c.run m' o x := by
  cases hf : c.fp x
  · rw [c.writes_in_fp m o x hf, c.writes_in_fp m' o x hf]
    exact hm o x hx
  · apply c.reads_in_fp m m' _ o x hf
    intro o' y hy
    cases he : ends y
    · exact hm o' y he
    · exact absurd (h y he) (by simp [hy])

/-! ## The body check -/

/-- What a unit's bodies say of a frame (Rust `BodyCheck`), in join order. -/
inductive BodyCheck where
  | established
  | unestablished
  | refuted
  deriving DecidableEq, Repr

def BodyCheck.rank : BodyCheck → Nat
  | .established => 0
  | .unestablished => 1
  | .refuted => 2

def BodyCheck.max (a b : BodyCheck) : BodyCheck := if a.rank ≤ b.rank then b else a

/-- One body: its member accesses (tag, member), its direct callees and
whether it holds a construct the reader does not read. -/
structure Body where
  accesses : List (Nat × Nat)
  calls : List Nat
  unread : Bool
  deriving DecidableEq, Repr

def lookup (bs : List (Nat × Body)) (n : Nat) : Option Body :=
  (bs.find? (fun p => p.1 == n)).map (·.2)

def own (rec : Nat) (mems : List Nat) (b : Body) : BodyCheck :=
  if b.accesses.any (fun a => a.1 == rec && mems.contains a.2) then .refuted
  else if b.unread then .unestablished else .established

/-- Rust `body_check`: a depth-first walk of the bodies reached from `n`,
bounded by `fuel`, refusing recursion through `stack`. -/
def check (bs : List (Nat × Body)) (rec : Nat) (mems : List Nat) :
    Nat → List Nat → Nat → BodyCheck
  | 0, _, _ => .unestablished
  | fuel + 1, stack, n =>
    if stack.contains n then .unestablished else
    match lookup bs n with
    | none => .unestablished
    | some b => b.calls.foldl (fun acc c => acc.max (check bs rec mems fuel (n :: stack) c)) (own rec mems b)

theorem max_established (a b : BodyCheck) (h : a.max b = .established) :
    a = .established ∧ b = .established := by
  cases a <;> cases b <;> simp_all [BodyCheck.max, BodyCheck.rank]

theorem fold_established (f : Nat → BodyCheck) (cs : List Nat) (acc : BodyCheck)
    (h : cs.foldl (fun a c => a.max (f c)) acc = .established) :
    acc = .established ∧ ∀ c ∈ cs, f c = .established := by
  induction cs generalizing acc with
  | nil => simpa using h
  | cons c rest ih =>
    obtain ⟨h1, h2⟩ := ih _ h
    obtain ⟨ha, hc⟩ := max_established _ _ h1
    refine ⟨ha, ?_⟩
    intro d hd
    rcases List.mem_cons.mp hd with rfl | hd
    · exact hc
    · exact h2 d hd

/-- **An established body names no protocol member** of the record and reads
in full. -/
theorem an_established_body_names_no_member (bs : List (Nat × Body)) (rec : Nat)
    (mems : List Nat) (fuel : Nat) (stack : List Nat) (n : Nat) (b : Body)
    (hb : lookup bs n = some b) (h : check bs rec mems (fuel + 1) stack n = .established) :
    own rec mems b = .established := by
  simp only [check] at h
  split at h
  · cases h
  · rw [hb] at h
    exact (fold_established _ _ _ h).1

/-- **An established body calls only established bodies.** -/
theorem an_established_body_calls_only_established_bodies (bs : List (Nat × Body))
    (rec : Nat) (mems : List Nat) (fuel : Nat) (stack : List Nat) (n : Nat) (b : Body)
    (hb : lookup bs n = some b) (h : check bs rec mems (fuel + 1) stack n = .established) :
    ∀ c ∈ b.calls, check bs rec mems fuel (n :: stack) c = .established := by
  simp only [check] at h
  split at h
  · cases h
  · rw [hb] at h
    exact (fold_established _ _ _ h).2

/-- **A missing body is never established.** -/
theorem a_missing_body_is_never_established (bs : List (Nat × Body)) (rec : Nat)
    (mems : List Nat) (fuel : Nat) (stack : List Nat) (n : Nat) (hb : lookup bs n = none) :
    check bs rec mems fuel stack n ≠ .established := by
  cases fuel with
  | zero => simp [check]
  | succ fuel =>
    simp only [check]
    split
    · simp
    · rw [hb]; simp

/-- **A recursive body is never established through itself.** -/
theorem a_recursive_body_is_never_established (bs : List (Nat × Body)) (rec : Nat)
    (mems : List Nat) (fuel : Nat) (stack : List Nat) (n : Nat) (hs : stack.contains n = true) :
    check bs rec mems fuel stack n ≠ .established := by
  cases fuel with
  | zero => simp [check]
  | succ fuel =>
    have hm : n ∈ stack := by simpa using hs
    simp [check, hm]

/-! ## The standing and the reading -/

/-- A frame's standing at a call (Rust `Standing`). -/
inductive Standing where
  | checked
  | accepted
  | declared
  | absent
  | refuted
  deriving DecidableEq, Repr

/-- Rust `standing`. -/
def standing (declared : Bool) (c : BodyCheck) (accepted : Bool) : Standing :=
  if !declared then .absent else
  match c with
  | .established => .checked
  | .refuted => .refuted
  | .unestablished => if accepted then .accepted else .declared

/-- What a window call makes of a frame (Rust `Reading`). -/
inductive Reading where
  | pass
  | held
  | broken
  | frameRefuted
  deriving DecidableEq, Repr

/-- Rust `window`. -/
def window : Standing → Reading
  | .checked => .pass
  | .accepted => .pass
  | .declared => .held
  | .absent => .broken
  | .refuted => .frameRefuted

theorem no_declaration_no_frame (c : BodyCheck) (a : Bool) : standing false c a = .absent := by
  simp [standing]

/-- **Only a checked or accepted frame passes unheld.** -/
theorem only_a_checked_or_accepted_frame_passes_unheld (d : Bool) (c : BodyCheck) (a : Bool) :
    window (standing d c a) = .pass ↔
      d = true ∧ (c = .established ∨ (c = .unestablished ∧ a = true)) := by
  cases d <;> cases c <;> cases a <;> simp [standing, window]

/-- **A declared frame the unit does not establish is held** (C06.1:
`Pending`, never `Safe`) until the human act accepts it. -/
theorem a_declared_frame_is_held :
    window (standing true .unestablished false) = .held := rfl

/-- **Acceptance never lifts a refutation.** -/
theorem acceptance_never_lifts_a_refutation (a : Bool) :
    window (standing true .refuted a) = .frameRefuted := by
  cases a <;> rfl

/-- **Without a frame the window refuses as before** (rule 38c's
`descriptor-cell-protocol-broken`). -/
theorem without_a_frame_the_window_refuses_as_before (c : BodyCheck) (a : Bool) :
    window (standing false c a) = .broken := by
  simp [standing, window]

/-! ## The window traces -/

inductive WStep where
  | s (step : QCore.CellProtocolPerEnd.Step)
  | f (st : Standing)
  deriving DecidableEq, Repr

inductive Outcome where
  | holds
  | held
  | broken
  | frameRefuted
  | overwritten
  | doubleRelease
  | undecided
  | ownerReleased
  | zeroUndecided
  | edgeUndecided
  deriving DecidableEq, Repr

def lift : QCore.CellResultSet.Outcome → Outcome
  | .holds => .holds
  | .broken => .broken
  | .overwritten => .overwritten
  | .doubleRelease => .doubleRelease
  | .undecided => .undecided
  | .ownerReleased => .ownerReleased
  | .zeroUndecided => .zeroUndecided
  | .edgeUndecided => .edgeUndecided

/-- Rust `run_window_vector` (identity 100); `held` records a held call. -/
def run (s : Spec) : List WStep → Option Fact → Table → Bool → Bool → Outcome
  | [], none, _, _, held => if held then .held else .holds
  | [], some f, t, _, held => if holds s f t then (if held then .held else .holds) else .broken
  | .s .testLive :: rest, _, t, _, held => run s rest (some (introduced s true 100)) t true held
  | .s .testDead :: rest, _, t, _, held => run s rest (some (introduced s false 100)) t false held
  | .s (.close k) :: rest, f, t, live, held =>
    let (f, t) := adopt (f.getD reached) t k
    match t k with
    | none => .undecided
    | some (i, _) =>
      match release t k true with
      | .ok t => run s rest (some (staleIdentity f i)) t live held
      | .error r => lift (refusalOutcome r)
  | .s (.store k v) :: rest, f, t, live, held =>
    match store s (f.getD reached) t k v with
    | .ok f => run s rest (some f) t live held
    | .error true => .overwritten
    | .error false => .broken
  | .s .join :: rest, f, t, live, held =>
    match f with
    | none => if held then .held else .holds
    | some f =>
      let other := introduced s (!live) 100
      let (f, t') := normalize f t empty
      let (o, ot) := normalize other empty t'
      run s rest (some (join f o)) (tableJoin t' ot) live held
  | .s .call :: rest, f, t, live, held =>
    match f with
    | some f => if holds s f t then run s rest none empty live held else .broken
    | none => run s rest none empty live held
  | .f st :: rest, f, t, live, held =>
    match f with
    | some f =>
      if holds s f t then run s rest none empty live held else
      match window st with
      | .pass => run s rest (some f) t live held
      | .held => run s rest (some f) t live true
      | .broken => .broken
      | .frameRefuted => .frameRefuted
    | none => run s rest none empty live held
  | .s .free :: rest, f, t, live, held =>
    match f with
    | some f =>
      match releaseHolder s f t with
      | none => run s rest none t live held
      | some o => lift o
    | none => run s rest none t live held

/-- Rust `WINDOW_VECTORS`, entry for entry (the Rust test spells each trace
in this form). -/
def windowVectors : List (Spec × List WStep × Outcome) := [
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .checked, .s .free], .holds),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .accepted, .s .free], .holds),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .declared, .s .free], .held),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .refuted], .frameRefuted),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .absent], .broken),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .absent, .s .free], .broken),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .f .absent], .holds),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .f .refuted], .holds),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .checked], .broken),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .checked, .s (.close 0)], .doubleRelease),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .checked, .s (.store 0 0)], .holds),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .declared, .f .checked, .s .free], .held),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s (.close 0), .f .declared, .f .absent], .broken),
  (⟨.positive, some fromZero, none⟩, [.s .testLive, .s (.close 0), .s .join, .f .checked, .s .free], .zeroUndecided),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .s .free], .ownerReleased),
  (⟨.positive, some fromOne, some absZero⟩, [.s .testLive, .f .checked, .s .free], .holds)
]

theorem window_vectors_hold :
    windowVectors.all (fun (s, trace, o) => run s trace none empty false false == o) = true := by
  decide

/-- Rust `BODY_VECTORS`, entry for entry: functions `f g h` are `0 1 2`, tags
`rec other mk_list` are `0 1 2`, members `ch event flags prev next` are
`0 1 2 3 4`; the frame is over `rec`, whose protocol members are `ch` and
`event`. The walk's fuel is one more than the number of bodies. -/
def bodyVectors : List (List (Nat × Body) × BodyCheck) := [
  ([(0, ⟨[(2, 3)], [1], false⟩), (1, ⟨[(2, 4)], [], false⟩)], .established),
  ([(0, ⟨[(0, 0)], [], false⟩)], .refuted),
  ([(0, ⟨[(0, 2)], [], false⟩)], .established),
  ([(0, ⟨[(1, 0)], [], false⟩)], .established),
  ([(0, ⟨[], [1], false⟩)], .unestablished),
  ([(0, ⟨[], [0], false⟩)], .unestablished),
  ([(0, ⟨[], [], true⟩)], .unestablished),
  ([(0, ⟨[], [1], true⟩), (1, ⟨[(0, 1)], [], false⟩)], .refuted),
  ([(0, ⟨[], [1, 2], false⟩), (1, ⟨[(0, 2)], [], false⟩), (2, ⟨[], [1], false⟩)], .established),
  ([(0, ⟨[], [1], false⟩), (1, ⟨[], [0], false⟩)], .unestablished),
  ([(0, ⟨[], [1], false⟩), (1, ⟨[(0, 0)], [0], false⟩)], .refuted)
]

theorem body_vectors_hold :
    bodyVectors.all (fun (bs, o) => check bs 0 [0, 1] (bs.length + 1) [] 0 == o) = true := by
  decide

end QCore.CellProtocolReleaseWindowFrames
