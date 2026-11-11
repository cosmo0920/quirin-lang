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

import QCore.IndexExtent

/-! Frame effects and derived relations of the bounds property (card
CG24-ALIAS-EXTENT-FRAME-VOCABULARY, contract specification C09.1, gateway rule
11e), extending `QCore.IndexExtent`.

The decisions of `crates/qcore-core/src/index_extent.rs` this card adds,
transcribed:

* a call's **frame** is unstated (no closed frame: every fact on an object the
  source did not establish is withdrawn, CG16's rule), CG16's
  `bounds_preserves_extents()` (everything kept), or **closed**: one mode per
  pointer parameter, `reads`, `writes` or `moves`. A read position keeps every
  fact; a write position withdraws the facts of the local whose address it
  receives (its value facts) and keeps extent facts on objects it only writes;
  a move position (in-place reallocation, the sds case) also withdraws every
  fact on an object the source did not establish. Facts kept across a held
  frame rest on it (`held`);
* a **derived relation** `bounds_return_derived(P, OFF, LEN, UNIT)`: the view
  `[start, start + LEN)` of the base, with `start` the base offset plus `OFF`,
  is checked against the base's extent; a decided containment puts the result
  in the base, a contradiction refuses the view;
* a declaration whose body exceeds its declared frame is **refuted** and its
  callers see an unstated frame.

What this is not: a model of C, of Clang's extraction, of aliasing or of the
external bodies (a closed frame and the absence of retention are the
declaration's `Declared` premise). The Rust correspondence is bound by
`frameVectors`, `derivedVectors` and `refuteVectors`
(`crates/qcore-core/tests/index_extent_frames.rs`). -/
namespace QCore.IndexExtentFrames

open QCore.IndexExtent

inductive Mode where
  | reads
  | writes
  | moves
  deriving DecidableEq, Repr

def Mode.rank : Mode → Nat
  | .reads => 0
  | .writes => 1
  | .moves => 2

/-- One frame position of a call: its declared mode and, when the actual is the
address `&x` of a tracked local, that local. -/
inductive Touch where
  | mk (mode : Mode) (addr : Option Nat)
  deriving DecidableEq, Repr

def Touch.mode : Touch → Mode
  | .mk m _ => m

def Touch.addr : Touch → Option Nat
  | .mk _ a => a

inductive Frame where
  | unstated
  | preserves
  | closed (touches : List Touch) (held : Bool)
  deriving DecidableEq, Repr

/-- A pointer fact: its variable, whether the source established its object,
the stable symbol its count or offset names (if any), and whether it rests on a
held premise. -/
inductive PFact where
  | mk (var : Nat) (source : Bool) (mention : Option Nat) (held : Bool)
  deriving DecidableEq, Repr

def PFact.var : PFact → Nat
  | .mk v _ _ _ => v

def PFact.source : PFact → Bool
  | .mk _ s _ _ => s

def PFact.mentions (x : Nat) : PFact → Bool
  | .mk _ _ m _ => m == some x

def PFact.held : PFact → Bool
  | .mk _ _ _ h => h

def PFact.hold : PFact → PFact
  | .mk v s m _ => .mk v s m true

structure State where
  pointers : List PFact
  scalars : List Fact
  deriving DecidableEq, Repr

/-- `Facts::write` on both parts. -/
def writeLocal (x : Nat) (s : State) : State :=
  { pointers := s.pointers.filter (fun p => p.var != x && !p.mentions x),
    scalars := write x s.scalars }

/-- `Facts::call`: keep only facts on source-established objects. -/
def withdrawNonSource (s : State) : State :=
  { s with pointers := s.pointers.filter (fun p => p.source) }

def touch (s : State) : Touch → State
  | .mk .reads _ => s
  | .mk .writes none => s
  | .mk .writes (some x) => writeLocal x s
  | .mk .moves none => withdrawNonSource s
  | .mk .moves (some x) => withdrawNonSource (writeLocal x s)

def holdNonSource (s : State) : State :=
  { s with pointers := s.pointers.map (fun p => if p.source then p else p.hold) }

/-- `Facts::call_frame` in Rust. -/
def callFrame : Frame → State → State
  | .unstated, s => withdrawNonSource s
  | .preserves, s => s
  | .closed ts held, s =>
      if held then holdNonSource (ts.foldl touch s) else ts.foldl touch s

/-! Every frame only withdraws: a fact after a touch was there before. -/

theorem touch_pointers_sub (s : State) (t : Touch) (p : PFact)
    (h : p ∈ (touch s t).pointers) : p ∈ s.pointers := by
  match t with
  | .mk .reads _ => exact h
  | .mk .writes none => exact h
  | .mk .writes (some x) => exact (List.mem_filter.mp h).1
  | .mk .moves none => exact (List.mem_filter.mp h).1
  | .mk .moves (some x) =>
      exact (List.mem_filter.mp (List.mem_filter.mp h).1).1

theorem touch_scalars_sub (s : State) (t : Touch) (f : Fact)
    (h : f ∈ (touch s t).scalars) : f ∈ s.scalars := by
  match t with
  | .mk .reads _ => exact h
  | .mk .writes none => exact h
  | .mk .writes (some x) => exact (List.mem_filter.mp h).1
  | .mk .moves none => exact h
  | .mk .moves (some x) => exact (List.mem_filter.mp h).1

/-- No fact on, or naming, local `x`. -/
def Clear (x : Nat) (s : State) : Prop :=
  (∀ p ∈ s.pointers, p.var ≠ x ∧ p.mentions x = false) ∧
    (∀ f ∈ s.scalars, f.var ≠ x ∧ f.value.mentions x = false)

theorem clear_touch (x : Nat) (s : State) (t : Touch) (h : Clear x s) : Clear x (touch s t) :=
  ⟨fun p hp => h.1 p (touch_pointers_sub s t p hp),
    fun f hf => h.2 f (touch_scalars_sub s t f hf)⟩

theorem clear_foldl (x : Nat) (ts : List Touch) (s : State) (h : Clear x s) :
    Clear x (ts.foldl touch s) := by
  induction ts generalizing s with
  | nil => exact h
  | cons t rest ih => exact ih (touch s t) (clear_touch x s t h)

theorem clear_write (x : Nat) (s : State) : Clear x (writeLocal x s) := by
  refine ⟨fun p hp => ?_, fun f hf => write_withdraws_every_mention x s.scalars f hf⟩
  have h2 := (List.mem_filter.mp hp).2
  simp at h2
  exact ⟨h2.1, by simpa using h2.2⟩

theorem clear_moves_or_writes (x : Nat) (s : State) (m : Mode) (hm : m ≠ .reads) :
    Clear x (touch s (.mk m (some x))) := by
  cases m with
  | reads => exact absurd rfl hm
  | writes => exact clear_write x s
  | moves =>
      have h := clear_write x s
      exact ⟨fun p hp => h.1 p (List.mem_filter.mp hp).1, h.2⟩

theorem clear_hold (x : Nat) (s : State) (h : Clear x s) : Clear x (holdNonSource s) := by
  refine ⟨fun p hp => ?_, h.2⟩
  obtain ⟨q, hq, he⟩ := List.mem_map.mp hp
  have hc := h.1 q hq
  by_cases hs : q.source = true
  · simp [hs] at he
    subst he
    exact hc
  · simp [hs] at he
    subst he
    cases q with
    | mk v s m hd => exact ⟨hc.1, hc.2⟩

/-- **A write frame invalidates the facts it may touch**: a write or move
position receiving `&x` leaves no fact on `x`, and none naming it. -/
theorem write_frame_withdraws_its_local (ts : List Touch) (held : Bool) (s : State)
    (m : Mode) (x : Nat) (hm : m ≠ .reads) (ht : Touch.mk m (some x) ∈ ts) :
    Clear x (callFrame (.closed ts held) s) := by
  obtain ⟨pre, post, he⟩ := List.append_of_mem ht
  have hmid : Clear x ((pre ++ Touch.mk m (some x) :: post).foldl touch s) := by
    rw [List.foldl_append, List.foldl_cons]
    exact clear_foldl x post _ (clear_moves_or_writes x _ m hm)
  rw [← he] at hmid
  unfold callFrame
  cases held
  · simpa using hmid
  · simpa using clear_hold x _ hmid

/-- **A read frame preserves every fact.** -/
theorem read_frame_preserves (ts : List Touch) (s : State)
    (h : ∀ t ∈ ts, t.mode = .reads) : callFrame (.closed ts false) s = s := by
  unfold callFrame
  simp only [Bool.false_eq_true, ite_false]
  induction ts generalizing s with
  | nil => rfl
  | cons t rest ih =>
      rw [List.foldl_cons]
      have ht := h t (List.mem_cons_self ..)
      have hs : touch s t = s := by
        cases t with
        | mk m a =>
            simp [Touch.mode] at ht
            subst ht
            rfl
      rw [hs]
      exact ih s (fun t' ht' => h t' (List.mem_cons_of_mem _ ht'))

/-- A write position keeps an extent fact on an object it only writes: with
no move position, a pointer fact on no touched local survives. -/
theorem write_frame_keeps_unrelated_extents (ts : List Touch) (s : State) (p : PFact)
    (hnm : ∀ t ∈ ts, t.mode ≠ .moves) (hp : p ∈ s.pointers)
    (hu : ∀ t ∈ ts, ∀ x, t.addr = some x → p.var ≠ x ∧ p.mentions x = false) :
    p ∈ (callFrame (.closed ts false) s).pointers := by
  unfold callFrame
  simp only [Bool.false_eq_true, ite_false]
  induction ts generalizing s with
  | nil => exact hp
  | cons t rest ih =>
      rw [List.foldl_cons]
      apply ih (touch s t)
      · exact fun t' ht' => hnm t' (List.mem_cons_of_mem _ ht')
      · have hm := hnm t (List.mem_cons_self ..)
        have hx := hu t (List.mem_cons_self ..)
        match t, hm, hx with
        | .mk .reads _, _, _ => exact hp
        | .mk .writes none, _, _ => exact hp
        | .mk .writes (some x), _, hx =>
            have := hx x rfl
            exact List.mem_filter.mpr ⟨hp, by simp [this.1, this.2]⟩
        | .mk .moves _, hm, _ => exact absurd rfl hm
      · exact fun t' ht' => hu t' (List.mem_cons_of_mem _ ht')

def AllSource (s : State) : Prop := ∀ p ∈ s.pointers, p.source = true

theorem all_source_touch (s : State) (t : Touch) (h : AllSource s) : AllSource (touch s t) :=
  fun p hp => h p (touch_pointers_sub s t p hp)

theorem all_source_foldl (ts : List Touch) (s : State) (h : AllSource s) :
    AllSource (ts.foldl touch s) := by
  induction ts generalizing s with
  | nil => exact h
  | cons t rest ih => exact ih (touch s t) (all_source_touch s t h)

/-- **A move frame withdraws every fact on an object the source did not
establish** (the base may have moved: the sds in-place reallocation). -/
theorem move_frame_withdraws_declared_facts (ts : List Touch) (held : Bool) (s : State)
    (a : Option Nat) (ht : Touch.mk .moves a ∈ ts) :
    AllSource (callFrame (.closed ts held) s) := by
  obtain ⟨pre, post, he⟩ := List.append_of_mem ht
  have hmid : AllSource ((pre ++ Touch.mk .moves a :: post).foldl touch s) := by
    rw [List.foldl_append, List.foldl_cons]
    apply all_source_foldl
    intro p hp
    cases a with
    | none => exact (List.mem_filter.mp hp).2
    | some x => exact (List.mem_filter.mp hp).2
  rw [← he] at hmid
  unfold callFrame
  cases held
  · simpa using hmid
  · intro p hp
    simp only [ite_true] at hp
    obtain ⟨q, hq, hqe⟩ := List.mem_map.mp hp
    have := hmid q hq
    simp [this] at hqe
    subst hqe
    exact this

theorem source_kept_by_touch (s : State) (t : Touch) (p : PFact) (hp : p ∈ s.pointers)
    (hs : p.source = true) (hu : ∀ x, t.addr = some x → p.var ≠ x ∧ p.mentions x = false) :
    p ∈ (touch s t).pointers := by
  match t, hu with
  | .mk .reads _, _ => exact hp
  | .mk .writes none, _ => exact hp
  | .mk .writes (some x), hu =>
      have := hu x rfl
      exact List.mem_filter.mpr ⟨hp, by simp [this.1, this.2]⟩
  | .mk .moves none, _ => exact List.mem_filter.mpr ⟨hp, hs⟩
  | .mk .moves (some x), hu =>
      have := hu x rfl
      exact List.mem_filter.mpr ⟨List.mem_filter.mpr ⟨hp, by simp [this.1, this.2]⟩, hs⟩

/-- **Frames only add precision**: a fact the unstated rule keeps, on no
touched local, survives every closed frame, held or not. -/
theorem unstated_is_the_weakest (ts : List Touch) (held : Bool) (s : State) (p : PFact)
    (hp : p ∈ (callFrame .unstated s).pointers)
    (hu : ∀ t ∈ ts, ∀ x, t.addr = some x → p.var ≠ x ∧ p.mentions x = false) :
    p ∈ (callFrame (.closed ts held) s).pointers := by
  have hmem := (List.mem_filter.mp hp).1
  have hsrc : p.source = true := (List.mem_filter.mp hp).2
  have hfold : p ∈ (ts.foldl touch s).pointers := by
    induction ts generalizing s with
    | nil => exact hmem
    | cons t rest ih =>
        rw [List.foldl_cons]
        have hk := source_kept_by_touch s t p hmem hsrc (hu t (List.mem_cons_self ..))
        exact ih (touch s t) (List.mem_filter.mpr ⟨hk, hsrc⟩)
          (fun t' ht' => hu t' (List.mem_cons_of_mem _ ht')) hk
  unfold callFrame
  cases held
  · simpa using hfold
  · simp only [ite_true]
    exact List.mem_map.mpr ⟨p, hfold, by simp [hsrc]⟩

/-- A fact kept across a held frame on an object the source did not establish
rests on the held declaration (so it discharges nothing, `held_never_safe`). -/
theorem held_frame_holds_declared_facts (ts : List Touch) (s : State) (p : PFact)
    (hp : p ∈ (callFrame (.closed ts true) s).pointers) (hs : p.source = false) :
    p.held = true := by
  unfold callFrame at hp
  simp only [ite_true] at hp
  obtain ⟨q, _, he⟩ := List.mem_map.mp hp
  by_cases hq : q.source = true
  · simp [hq] at he
    subst he
    simp [hq] at hs
  · simp [hq] at he
    subst he
    cases q with
    | mk v s' m h => rfl

/-! Derived relations. -/

/-- The containment of a derived view `[start, start + len)` in a base of `n`
units (`derived_within` in Rust). -/
def derivedWithin (n : Count) (start : IndexFact) (len : Count) : Decision :=
  match n, start, len with
  | .const n, .const s, .const l =>
      if 0 ≤ s ∧ s + (l : Int) ≤ (n : Int) then .inBounds else .outOfBounds
  | .sym a, .const s, .sym b => if s = 0 ∧ a = b then .inBounds else .undecided .symbolMismatch
  | _, .unknown, _ => .undecided .unsupportedRange
  | _, .range _ _, _ => .undecided .unsupportedRange
  | _, _, _ => .undecided .symbolMismatch

/-- **A derived relation's extent stays within its base's**: when containment
is decided, every element of the view is an in-bounds element of the base. -/
theorem derived_view_within_base (env : Nat → Nat) (n : Count) (start : IndexFact)
    (len : Count) (s i : Int) (h : derivedWithin n start len = .inBounds)
    (hs : Describes env start s) (hi : 0 ≤ i ∧ i < (len.eval env : Int)) :
    0 ≤ s + i ∧ s + i < (n.eval env : Int) := by
  cases n with
  | const nn =>
      cases start with
      | const s0 =>
          cases len with
          | const l =>
              simp only [derivedWithin] at h
              by_cases hc : 0 ≤ s0 ∧ s0 + (l : Int) ≤ (nn : Int)
              · simp [Describes] at hs
                subst hs
                simp [Count.eval] at hi ⊢
                omega
              · simp [hc] at h
          | sym _ => simp [derivedWithin] at h
      | range _ _ => simp [derivedWithin] at h
      | unknown => simp [derivedWithin] at h
  | sym a =>
      cases start with
      | const s0 =>
          cases len with
          | sym b =>
              simp only [derivedWithin] at h
              by_cases hc : s0 = 0 ∧ a = b
              · obtain ⟨h0, hab⟩ := hc
                subst hab
                simp [Describes] at hs
                subst hs
                simp [Count.eval] at hi ⊢
                omega
              · simp [hc] at h
          | const _ => simp [derivedWithin] at h
      | range _ _ => simp [derivedWithin] at h
      | unknown => simp [derivedWithin] at h

/-- A contradiction is called only on exact quantities, and is one: the view
does not fit the base. -/
theorem derived_contradiction_sound (n : Count) (start : IndexFact) (len : Count)
    (h : derivedWithin n start len = .outOfBounds) :
    ∃ nn s l, n = .const nn ∧ start = .const s ∧ len = .const l ∧
      ¬ (0 ≤ s ∧ s + (l : Int) ≤ (nn : Int)) := by
  cases n with
  | const nn =>
      cases start with
      | const s =>
          cases len with
          | const l =>
              refine ⟨nn, s, l, rfl, rfl, rfl, ?_⟩
              intro hc
              simp [derivedWithin, hc] at h
          | sym _ => simp [derivedWithin] at h
      | range _ _ => simp [derivedWithin] at h
      | unknown => simp [derivedWithin] at h
  | sym a =>
      cases start with
      | const s =>
          cases len with
          | sym b =>
              simp only [derivedWithin] at h
              by_cases hc : s = 0 ∧ a = b
              · simp [hc] at h
              · simp [hc] at h
          | const _ => simp [derivedWithin] at h
      | range _ _ => simp [derivedWithin] at h
      | unknown => simp [derivedWithin] at h

/-- Several bases (`multi-base`): the result receives the view only when no
base contradicts it. -/
def multiBaseView (ds : List Decision) : Bool := ds.all (fun d => d != .outOfBounds)

theorem a_contradicted_base_refuses_the_view (ds : List Decision)
    (h : .outOfBounds ∈ ds) : multiBaseView ds = false := by
  unfold multiBaseView
  rw [List.all_eq_false]
  exact ⟨.outOfBounds, h, by decide⟩

/-! Refutation by a body. -/

/-- What a body does through a parameter it declares in a frame. -/
inductive BodyEffect where
  | read
  | written
  | moved
  deriving DecidableEq, Repr

def BodyEffect.rank : BodyEffect → Nat
  | .read => 0
  | .written => 1
  | .moved => 2

/-- A body refutes its declared mode when its effect exceeds it. -/
def refutes (m : Mode) (e : BodyEffect) : Bool := decide (m.rank < e.rank)

/-- The frame a caller applies: a refuted or absent declaration is unstated. -/
def frameAtCall (declared : Option (List Touch)) (refuted held : Bool) : Frame :=
  if refuted then .unstated
  else
    match declared with
    | none => .unstated
    | some ts => .closed ts held

theorem a_refuted_frame_is_unstated (d : Option (List Touch)) (held : Bool) :
    frameAtCall d true held = .unstated := by
  simp [frameAtCall]

theorem a_body_within_its_frame_never_refutes (m : Mode) (e : BodyEffect)
    (h : e.rank ≤ m.rank) : refutes m e = false := by
  unfold refutes
  simp
  omega

theorem a_write_through_a_read_frame_refutes : refutes .reads .written = true := by decide

theorem a_move_through_a_write_frame_refutes : refutes .writes .moved = true := by decide

/-! Shared vectors, decided here and by the Rust module. -/

/-- `(frame, pointer facts, scalar facts, pointer variables kept, scalar
variables kept, pointer variables held after)`. -/
def frameVectors : List (Frame × List PFact × List Fact × List Nat × List Nat × List Nat) := [
  (.unstated, [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)],
    [1], [3], []),
  (.preserves, [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)],
    [1, 2], [3], []),
  (.closed [.mk .reads none, .mk .reads (some 3)] false,
    [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)], [1, 2], [3], []),
  (.closed [.mk .writes (some 3)] false,
    [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)], [1, 2], [], []),
  (.closed [.mk .writes (some 3)] false,
    [.mk 1 true (some 3) false, .mk 2 false none false], [.mk 4 (.range 0 (.sym 3))], [2], [], []),
  (.closed [.mk .writes none] false,
    [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)], [1, 2], [3], []),
  (.closed [.mk .moves none] false,
    [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)], [1], [3], []),
  (.closed [.mk .moves (some 1), .mk .reads none] false,
    [.mk 1 true none false, .mk 2 false none false, .mk 5 true none false],
    [.mk 3 (.const 2)], [5], [3], []),
  (.closed [.mk .reads none] true,
    [.mk 1 true none false, .mk 2 false none false], [.mk 3 (.const 2)], [1, 2], [3], [2]),
  (.closed [.mk .moves none] true,
    [.mk 1 true none false, .mk 2 false none false], [], [1], [], []),
  (.closed [] false, [.mk 2 false none false], [], [2], [], [])
]

def vectorHolds (row : Frame × List PFact × List Fact × List Nat × List Nat × List Nat) : Bool :=
  let (frame, pointers, scalars, kept, keptScalars, heldAfter) := row
  let out := callFrame frame { pointers := pointers, scalars := scalars }
  out.pointers.map PFact.var == kept && out.scalars.map Fact.var == keptScalars &&
    (out.pointers.filter PFact.held).map PFact.var == heldAfter

theorem frame_vectors_hold : frameVectors.all vectorHolds = true := by decide

theorem frame_vectors_count : frameVectors.length = 11 := by decide

/-- `(base count, view start, view length, containment)`. -/
def derivedVectors : List (Count × IndexFact × Count × Decision) := [
  (.const 8, .const 0, .const 8, .inBounds),
  (.const 8, .const 2, .const 4, .inBounds),
  (.const 8, .const 6, .const 4, .outOfBounds),
  (.const 8, .const (-1), .const 2, .outOfBounds),
  (.const 8, .const 8, .const 0, .inBounds),
  (.sym 1, .const 0, .sym 1, .inBounds),
  (.sym 1, .const 2, .sym 1, .undecided .symbolMismatch),
  (.sym 1, .const 0, .sym 2, .undecided .symbolMismatch),
  (.const 8, .unknown, .const 2, .undecided .unsupportedRange),
  (.const 8, .range 0 (.const 2), .const 2, .undecided .unsupportedRange),
  (.const 8, .const 0, .sym 1, .undecided .symbolMismatch)
]

theorem derived_vectors_hold :
    derivedVectors.all (fun (n, s, l, d) => derivedWithin n s l == d) = true := by decide

theorem derived_vectors_count : derivedVectors.length = 11 := by decide

/-- `(declared mode, body effect, refuted)`. -/
def refuteVectors : List (Mode × BodyEffect × Bool) := [
  (.reads, .read, false),
  (.reads, .written, true),
  (.reads, .moved, true),
  (.writes, .read, false),
  (.writes, .written, false),
  (.writes, .moved, true),
  (.moves, .read, false),
  (.moves, .written, false),
  (.moves, .moved, false)
]

theorem refute_vectors_hold :
    refuteVectors.all (fun (m, e, r) => refutes m e == r) = true := by decide

theorem refute_vectors_count : refuteVectors.length = 9 := by decide

end QCore.IndexExtentFrames
