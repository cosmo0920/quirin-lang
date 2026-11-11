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

/-! `qcore.obs-reordering.v1` — comparing an observation stream up to a
**declared** reordering (card `OBS-REORDERING`).

This is a model of `crates/qcore-core/src/obs_reordering.rs`, and of the
soundness statement in `docs/design/OBS_REORDERING_V1.md` §3.1.

A stream here is a **list of windows**, each a list of records, and a record is
the emitter that wrote it paired with the bytes it wrote. That presentation is
the Rust module's own window decomposition, one step further along: the Rust side
carries a per-record window index and refuses a declaration whose indices go
backwards (`WindowsNotMonotone`), which is exactly the statement that a stream
*is* a list of windows. Nothing else about the Rust side is assumed.

What is proved:

* **`≡` is an equivalence relation** — reflexive, symmetric, transitive — so
  "these two runs agree" is a property of the pair rather than of which one was
  called the reference.
* **Per-emitter order is preserved.** Two equivalent streams carry, for every
  emitter, the same whole-stream projection. This is what an accepted native run
  has actually been shown to satisfy about each thread of control.
* **Barrier structure is preserved.** A window holding one record — the shape a
  quiescent point produces, where the parent is the only thread of control — is
  carried across unchanged, record for record and position for position.
* **The strict comparison is the empty-relation case.** A stream whose windows
  each have at most one emitter is equivalent to *itself alone*. Every route in
  this repository before this card declares such a relation, so this card
  replaces no existing comparison: it generalises it, and the old one is the
  special case.
* **The commuting rule generates exactly this equivalence.** Transposing two
  adjacent records of one window with **distinct** emitters yields an equivalent
  window; transposing two adjacent records of the **same** emitter with different
  bytes does not. §2.1 of the design record and §3's closed form therefore agree
  on the generators.
* **The decision procedure is exact.** `decideEquiv` — the transcription of the
  Rust comparator's per-window, per-emitter loop — returns `true` exactly on
  equivalent streams.

What is **not** claimed, and each of these is a boundary rather than a gap:

* Nothing about memory, visibility, atomicity or data races. A record's bytes are
  an opaque `Text` and the only things modelled are which emitter wrote it and in
  which window. There is no TSan-shaped statement anywhere here.
* Nothing about **which** admitted order a run takes, or that any admitted order
  is reachable. Inside a window this relation asserts the *absence* of order
  between distinct emitters; it asserts no presence.
* Nothing about the two streams' relative order. `stdout` and `stderr` are two
  separate applications of everything below.
* That the Rust derivation reads the window structure faithfully off a run's
  spawn/collect events, or that the emitted C performs the writes at all. Both
  are tested Rust boundaries, on the `ConcurrencyPrelude` precedent. -/

namespace QCore.ObsReordering

/-- The thread of control that wrote a record's bytes — **not** the task whose
state it reports. `QCore.TaskTraceRelation` attributes an observed line the other
way, because it relates causal order and this relates byte order. -/
inductive Emitter where
  | parent
  | worker (k : Nat)
  deriving DecidableEq, Repr

/-- The bytes of one line. Opaque on purpose: nothing below reads them, so no
theorem here can depend on a rendering. -/
abbrev Text := Nat

/-- One record: who wrote it, and what it said. -/
abbrev Record := Emitter × Text

/-- One concurrency window. Records of **distinct** emitters inside a window
commute; records of one emitter never do. -/
abbrev Window := List Record

/-- An observation stream, decomposed into its windows in order. -/
abbrev Stream := List Window

/-- One emitter's own lines in one window, in that emitter's own order. -/
def proj (a : Window) (e : Emitter) : List Text :=
  (a.filter (fun r => r.1 == e)).map Prod.snd

/-- One emitter's own lines across the whole stream. -/
def streamProj (s : Stream) (e : Emitter) : List Text :=
  (s.map (fun a => proj a e)).flatten

/-- **Equivalence of one window.** The two conjuncts are the Rust comparator's
own two checks, in its own order: the window carries as many records
(`LengthMismatch`), and every emitter's lines are that emitter's declared lines
in that emitter's declared order (`EmitterOrder`). A record that no emitter of
the window could have written is `Residue` on the Rust side, and appears here as
a projection that does not match. -/
def windowEquiv (a b : Window) : Prop :=
  a.length = b.length ∧ ∀ e, proj a e = proj b e

/-- **`R`-equivalence of two streams.** Window for window; windows never merge
and never split, because a window boundary is a point at which one thread of
control is the only writer. -/
inductive Equiv : Stream → Stream → Prop where
  | nil : Equiv [] []
  | cons {a b : Window} {s t : Stream} : windowEquiv a b → Equiv s t → Equiv (a :: s) (b :: t)

/-! ## Basic shape -/

theorem proj_nil (e : Emitter) : proj [] e = [] := rfl

theorem proj_cons_same {p : Record} {t : Window} {e : Emitter} (h : p.1 = e) :
    proj (p :: t) e = p.2 :: proj t e := by
  simp [proj, List.filter, h]

theorem proj_cons_other {p : Record} {t : Window} {e : Emitter} (h : p.1 ≠ e) :
    proj (p :: t) e = proj t e := by
  have hb : (p.1 == e) = false := by simp [h]
  simp [proj, List.filter, hb]

theorem proj_length_le (a : Window) (e : Emitter) : (proj a e).length ≤ a.length := by
  induction a with
  | nil => simp [proj]
  | cons p t ih =>
      by_cases h : p.1 = e
      · rw [proj_cons_same h]; simpa using ih
      · rw [proj_cons_other h]; exact Nat.le_succ_of_le ih

/-! ## `≡` is an equivalence relation -/

theorem windowEquiv_refl (a : Window) : windowEquiv a a :=
  ⟨rfl, fun _ => rfl⟩

theorem windowEquiv_symm {a b : Window} (h : windowEquiv a b) : windowEquiv b a :=
  ⟨h.1.symm, fun e => (h.2 e).symm⟩

theorem windowEquiv_trans {a b c : Window}
    (h₁ : windowEquiv a b) (h₂ : windowEquiv b c) : windowEquiv a c :=
  ⟨h₁.1.trans h₂.1, fun e => (h₁.2 e).trans (h₂.2 e)⟩

theorem equiv_refl (s : Stream) : Equiv s s := by
  induction s with
  | nil => exact Equiv.nil
  | cons a t ih => exact Equiv.cons (windowEquiv_refl a) ih

theorem equiv_symm {s t : Stream} (h : Equiv s t) : Equiv t s := by
  induction h with
  | nil => exact Equiv.nil
  | cons hab _ ih => exact Equiv.cons (windowEquiv_symm hab) ih

theorem equiv_trans {s t u : Stream} (h₁ : Equiv s t) (h₂ : Equiv t u) : Equiv s u := by
  induction h₁ generalizing u with
  | nil => cases h₂; exact Equiv.nil
  | cons hab _ ih =>
      cases h₂ with
      | cons hbc hrest => exact Equiv.cons (windowEquiv_trans hab hbc) (ih hrest)

/-- The window decomposition itself is preserved: the same number of windows. -/
theorem equiv_length {s t : Stream} (h : Equiv s t) : s.length = t.length := by
  induction h with
  | nil => rfl
  | cons _ _ ih => simp [ih]

/-- And window for window, the same number of records. -/
theorem equiv_window_sizes {s t : Stream} (h : Equiv s t) :
    s.map List.length = t.map List.length := by
  induction h with
  | nil => rfl
  | cons hab _ ih => simp [hab.1, ih]

/-! ## Soundness 1 — per-emitter order is preserved -/

/-- **An accepted run performed each thread's writes in that thread's program
order.** The strict statement about a single-threaded emitter survives the
widening intact; this is the theorem that says so. -/
theorem equiv_preserves_emitter_projection {s t : Stream} (h : Equiv s t) (e : Emitter) :
    streamProj s e = streamProj t e := by
  unfold streamProj
  induction h with
  | nil => rfl
  | cons hab _ ih => simp only [List.map_cons, List.flatten_cons, hab.2 e]; rw [ih]

/-- **Per-window content is preserved**: nothing is dropped, duplicated or
invented inside a window, for any emitter. -/
theorem equiv_preserves_window_content {s t : Stream} (h : Equiv s t) :
    ∀ i (hs : i < s.length) (ht : i < t.length) (e : Emitter), proj s[i] e = proj t[i] e := by
  induction h with
  | nil => intro i hs; simp at hs
  | cons hab _ ih =>
      intro i hs ht e
      cases i with
      | zero => exact hab.2 e
      | succ n => exact ih n (Nat.lt_of_succ_lt_succ hs) (Nat.lt_of_succ_lt_succ ht) e

/-! ## Soundness 2 — barrier structure is preserved

A **barrier** is a window holding exactly one record: the shape a quiescent point
produces, where no worker is live and the parent is the only thread of control.
Such a record commutes with nothing. -/

def IsBarrier (a : Window) : Prop := a.length = 1

theorem barrier_is_carried_across {a b : Window} (hb : IsBarrier a) (h : windowEquiv a b) :
    a = b := by
  match a, hb with
  | [(e, x)], _ =>
      match b, h.1 with
      | [(f, y)], _ =>
          have hp := h.2 e
          by_cases hfe : f = e
          · subst hfe
            rw [proj_cons_same (p := (f, x)) rfl, proj_cons_same (p := (f, y)) rfl] at hp
            simp at hp
            simp [hp]
          · exfalso
            rw [proj_cons_same (p := (e, x)) rfl,
              proj_cons_other (p := (f, y)) (by simpa using hfe)] at hp
            simp [proj] at hp

theorem equiv_preserves_barriers {s t : Stream} (h : Equiv s t) :
    ∀ i (hs : i < s.length) (ht : i < t.length), IsBarrier s[i] → s[i] = t[i] := by
  induction h with
  | nil => intro i hs; simp at hs
  | cons hab _ ih =>
      intro i hs ht hbar
      cases i with
      | zero => exact barrier_is_carried_across hbar hab
      | succ n => exact ih n (Nat.lt_of_succ_lt_succ hs) (Nat.lt_of_succ_lt_succ ht) hbar

/-! ## Soundness 3 — the strict comparison is the empty-relation case -/

/-- A window admits no reordering when every record in it has one emitter. -/
def StrictWindow (a : Window) : Prop := ∀ p ∈ a, ∀ q ∈ a, p.1 = q.1

/-- A stream admits no reordering when every window does. Every route in this
repository before card `OBS-REORDERING` declares such a stream. -/
def Strict (s : Stream) : Prop := ∀ a ∈ s, StrictWindow a

theorem length_proj_of_single_emitter {a : Window} {e : Emitter}
    (h : ∀ q ∈ a, q.1 = e) : (proj a e).length = a.length := by
  induction a with
  | nil => rfl
  | cons p t ih =>
      rw [proj_cons_same (h p (List.mem_cons_self ..))]
      simp [ih fun q hq => h q (List.mem_cons_of_mem _ hq)]

theorem single_emitter_of_length_proj {a : Window} {e : Emitter}
    (h : (proj a e).length = a.length) : ∀ q ∈ a, q.1 = e := by
  induction a with
  | nil => intro q hq; simp at hq
  | cons p t ih =>
      by_cases hp : p.1 = e
      · rw [proj_cons_same hp] at h
        simp only [List.length_cons] at h
        have ht := ih (Nat.succ.inj h)
        intro q hq
        cases List.mem_cons.1 hq with
        | inl he => exact he ▸ hp
        | inr hm => exact ht q hm
      · exfalso
        rw [proj_cons_other hp] at h
        simp only [List.length_cons] at h
        have hle := proj_length_le t e
        rw [h] at hle
        exact Nat.not_succ_le_self t.length hle

theorem window_of_single_emitter {a : Window} {e : Emitter}
    (h : ∀ p ∈ a, p.1 = e) : a = (proj a e).map (fun x => (e, x)) := by
  induction a with
  | nil => rfl
  | cons p t ih =>
      have hp : p.1 = e := h p (List.mem_cons_self ..)
      have hrec := ih fun q hq => h q (List.mem_cons_of_mem _ hq)
      have hpair : p = (e, p.2) := by
        cases p
        cases hp
        rfl
      rw [proj_cons_same hp, List.map_cons, ← hrec, ← hpair]

/-- **On a strict relation, `R`-equivalence is byte equality.** The existing
per-stream byte comparison is not replaced by this card; it *is* this relation,
at the declaration that admits nothing. -/
theorem strict_window_admits_only_itself {a b : Window}
    (hs : StrictWindow a) (h : windowEquiv a b) : a = b := by
  match a with
  | [] =>
      have : b.length = 0 := h.1 ▸ rfl
      exact (List.eq_nil_of_length_eq_zero this).symm
  | p :: t =>
      have he : ∀ q ∈ p :: t, q.1 = p.1 := fun q hq => hs q hq p (List.mem_cons_self ..)
      have hproj : proj (p :: t) p.1 = proj b p.1 := h.2 p.1
      have hlen : (proj b p.1).length = b.length :=
        (hproj ▸ length_proj_of_single_emitter he).trans h.1
      have hb := single_emitter_of_length_proj hlen
      rw [window_of_single_emitter he, window_of_single_emitter hb, hproj]

theorem strict_admits_only_itself {s t : Stream} (hs : Strict s) (h : Equiv s t) : s = t := by
  induction h with
  | nil => rfl
  | cons hab _ ih =>
      rename_i a _ s' _ _
      have hstrict : StrictWindow a := hs a (List.mem_cons_self ..)
      have hrest : Strict s' := fun c hc => hs c (List.mem_cons_of_mem _ hc)
      rw [strict_window_admits_only_itself hstrict hab, ih hrest]

/-- The same fact as an iff, so "strict is byte equality" is stated in both
directions rather than in one. -/
theorem strict_iff_byte_equality {s t : Stream} (hs : Strict s) : Equiv s t ↔ s = t :=
  ⟨strict_admits_only_itself hs, fun h => h ▸ equiv_refl s⟩

/-- A program that spawns nothing writes only barriers, and a stream of barriers
is strict. This is the shape of every route before card `OBS-REORDERING`. -/
theorem barriers_are_strict {s : Stream} (h : ∀ a ∈ s, IsBarrier a) : Strict s := by
  intro a ha p hp q hq
  match a, h a ha with
  | [r], _ =>
      have hp' : p = r := by simpa using hp
      have hq' : q = r := by simpa using hq
      rw [hp', hq']

/-! ## The commuting rule generates exactly this equivalence -/

theorem proj_append (a b : Window) (e : Emitter) :
    proj (a ++ b) e = proj a e ++ proj b e := by
  simp [proj, List.filter_append]

/-- **Two records of distinct emitters commute.** This is the generator §2.1 of
the design record names, and it lands inside §3's closed form. -/
theorem distinct_emitters_commute (pre post : Window) {e f : Emitter} (x y : Text)
    (hne : e ≠ f) :
    windowEquiv (pre ++ (e, x) :: (f, y) :: post) (pre ++ (f, y) :: (e, x) :: post) := by
  refine ⟨by simp, fun g => ?_⟩
  rw [proj_append, proj_append]
  refine congrArg (proj pre g ++ ·) ?_
  by_cases hg : e = g
  · subst hg
    rw [proj_cons_same (p := (e, x)) rfl, proj_cons_other (p := (f, y)) (Ne.symm hne),
      proj_cons_other (p := (f, y)) (Ne.symm hne), proj_cons_same (p := (e, x)) rfl]
  · by_cases hg' : f = g
    · subst hg'
      rw [proj_cons_other (p := (e, x)) hg, proj_cons_same (p := (f, y)) rfl,
        proj_cons_same (p := (f, y)) rfl, proj_cons_other (p := (e, x)) hg]
    · rw [proj_cons_other (p := (e, x)) hg, proj_cons_other (p := (f, y)) hg',
        proj_cons_other (p := (f, y)) hg', proj_cons_other (p := (e, x)) hg]

/-- **Two records of one emitter never commute.** Program order within a thread
of control is preserved, which is the other half of what makes the relation a
statement about the program rather than about the schedule. -/
theorem one_emitter_never_commutes (pre post : Window) (e : Emitter) {x y : Text}
    (hne : x ≠ y) :
    ¬ windowEquiv (pre ++ (e, x) :: (e, y) :: post) (pre ++ (e, y) :: (e, x) :: post) := by
  intro h
  have hp := h.2 e
  rw [proj_append, proj_append] at hp
  have h' := List.append_cancel_left hp
  rw [proj_cons_same (p := (e, x)) rfl, proj_cons_same (p := (e, y)) rfl,
    proj_cons_same (p := (e, y)) rfl, proj_cons_same (p := (e, x)) rfl] at h'
  exact hne (List.cons.inj h').1

/-! ## The decision procedure, and its exactness

`decideEquiv` is the transcription of the Rust comparator: per window, the length
check and then a per-emitter comparison over the emitters that actually occur. It
is exact — no stream is accepted that the definition above rejects, and none
rejected that it accepts. -/

/-- The emitters that occur in a window. An emitter outside this list writes
nothing there, which is the lemma the exactness proof turns on. -/
def emitters (a : Window) : List Emitter := a.map Prod.fst

theorem proj_eq_nil_of_not_mem {a : Window} {e : Emitter} (h : e ∉ emitters a) :
    proj a e = [] := by
  induction a with
  | nil => rfl
  | cons p t ih =>
      simp only [emitters, List.map_cons, List.mem_cons, not_or] at h
      rw [proj_cons_other (Ne.symm h.1)]
      exact ih (by simpa [emitters] using h.2)

def decideWindow (a b : Window) : Bool :=
  a.length == b.length && (emitters a ++ emitters b).all (fun e => proj a e == proj b e)

def decideEquiv (s t : Stream) : Bool :=
  s.length == t.length && (s.zip t).all (fun p => decideWindow p.1 p.2)

theorem decideWindow_correct (a b : Window) : decideWindow a b = true ↔ windowEquiv a b := by
  constructor
  · intro h
    simp only [decideWindow, Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at h
    refine ⟨h.1, fun e => ?_⟩
    by_cases hm : e ∈ emitters a ++ emitters b
    · simpa using h.2 e hm
    · simp only [List.mem_append, not_or] at hm
      rw [proj_eq_nil_of_not_mem hm.1, proj_eq_nil_of_not_mem hm.2]
  · intro h
    simp only [decideWindow, Bool.and_eq_true, beq_iff_eq, List.all_eq_true]
    exact ⟨h.1, fun e _ => by simp [h.2 e]⟩

private theorem zip_forward : ∀ (s t : Stream), s.length = t.length →
    (∀ p ∈ s.zip t, decideWindow p.1 p.2 = true) → Equiv s t
  | [], [], _, _ => Equiv.nil
  | [], _ :: _, hlen, _ => by simp at hlen
  | _ :: _, [], hlen, _ => by simp at hlen
  | a :: s', b :: t', hlen, hall =>
      Equiv.cons
        ((decideWindow_correct a b).1
          (hall (a, b) (by rw [List.zip_cons_cons]; exact List.mem_cons_self ..)))
        (zip_forward s' t' (by simpa using hlen) (fun p hp =>
          hall p (by rw [List.zip_cons_cons]; exact List.mem_cons_of_mem _ hp)))

private theorem zip_backward {s t : Stream} (h : Equiv s t) :
    ∀ p ∈ s.zip t, decideWindow p.1 p.2 = true := by
  induction h with
  | nil => intro p hp; simp at hp
  | cons hab _ ih =>
      intro p hp
      rw [List.zip_cons_cons, List.mem_cons] at hp
      cases hp with
      | inl he => exact he ▸ (decideWindow_correct _ _).2 hab
      | inr hm => exact ih p hm

theorem decideEquiv_correct (s t : Stream) : decideEquiv s t = true ↔ Equiv s t := by
  constructor
  · intro h
    simp only [decideEquiv, Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at h
    exact zip_forward s t h.1 h.2
  · intro h
    simp only [decideEquiv, Bool.and_eq_true, beq_iff_eq, List.all_eq_true]
    exact ⟨equiv_length h, zip_backward h⟩

end QCore.ObsReordering
