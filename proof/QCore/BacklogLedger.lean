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
FB-BACKLOG-ACCOUNTING (gateway rule 92). Fluent Bit keeps per-output byte
ledgers (`fs_backlog_chunks_size`, `fs_chunks_size`) that it updates with a
guarded subtraction and a guarded addition, clamping on the other arm. This
module states:

* the arithmetic: the guarded forms never wrap in `size_t` (modulo `M`)
  arithmetic, and the bare forms do;
* the decision table of `ledger_transition::decide` (`decide_vectors`);
* the ledger invariant `ledger = total of the linked charges` and its
  preservation across charge-on-link, discharge-on-unlink, repeated removal,
  residency up/down, partial fan-out over shared chunks and rollback, with
  the clamp arms unreachable under it (the overflow arm under the declared
  physical bound);
* the mutations rule 92's pairing refuses (missing and doubled discharge)
  break the invariant;
* the admission inequality of `flb_input_chunk_output_would_exceed_limit`
  (`src/flb_input_chunk.c:1176-1199`), the batch overshoot bound of
  `cb_queue_chunks` (`sb.c:602-720`) and the signed-delta magnitude of
  `flb_input_chunk_update_output_instances` (`src/flb_input_chunk.c:3987-3992`).

The Rust/model correspondence, exclusive access to the ledger and callees not
writing it are premises (P-LEDGER-EXCLUSIVE, P-LEDGER-CALLEES).
-/
namespace QCore.BacklogLedger

/-! ## Saturating arithmetic -/

/-- `if (d > L) L = 0; else L -= d;` -/
def satSub (l d : Nat) : Nat := if d > l then 0 else l - d

/-- `if (d > MAX - L) L = MAX; else L += d;` -/
def satAdd (max l d : Nat) : Nat := if d > max - l then max else l + d

/-- The unsigned C subtraction of two values below `M` (wraps modulo `M`). -/
def cSub (m l d : Nat) : Nat := (l + m - d) % m

/-- The unsigned C addition (wraps modulo `M`). -/
def cAdd (m l d : Nat) : Nat := (l + d) % m

theorem guarded_sub_does_not_wrap (m l d : Nat) (hl : l < m) (h : ¬ d > l) :
    cSub m l d = l - d := by
  unfold cSub
  have : l + m - d = (l - d) + m := by omega
  rw [this, Nat.add_mod_right, Nat.mod_eq_of_lt (by omega)]

theorem guarded_add_does_not_wrap (m l d : Nat) (hl : l < m) (h : ¬ d > (m - 1) - l) :
    cAdd m l d = l + d := by
  unfold cAdd
  exact Nat.mod_eq_of_lt (by omega)

/-- The bare decrement wraps: `0 - 1` is `M - 1`, not the clamped `0`. -/
theorem bare_sub_wraps (m : Nat) (hm : 2 ≤ m) : cSub m 0 1 ≠ satSub 0 1 := by
  unfold cSub satSub
  have : (0 + m - 1) % m = m - 1 := by
    rw [Nat.mod_eq_of_lt (by omega : 0 + m - 1 < m)]; omega
  rw [this]; simp; omega

/-- The bare increment wraps: `(M - 1) + 1` is `0`, not the clamped maximum. -/
theorem bare_add_wraps (m : Nat) (hm : 2 ≤ m) : cAdd m (m - 1) 1 ≠ satAdd (m - 1) (m - 1) 1 := by
  unfold cAdd satAdd
  have h1 : m - 1 + 1 = m := by omega
  rw [h1, Nat.mod_self]
  simp; omega

theorem satSub_exact (l d : Nat) (h : d ≤ l) : satSub l d + d = l := by
  unfold satSub; simp [show ¬ d > l by omega]; omega

theorem satSub_le (l d : Nat) : satSub l d ≤ l := by
  unfold satSub; split <;> omega

theorem satAdd_le_max (max l d : Nat) (hl : l ≤ max) : satAdd max l d ≤ max := by
  unfold satAdd; split <;> omega

theorem satAdd_exact (max l d : Nat) (h : l + d ≤ max) : satAdd max l d = l + d := by
  unfold satAdd; simp [show ¬ d > max - l by omega]

/-! ## The decision table (`ledger_transition::decide`) -/

inductive Form where
  | subtract | add | zero | maximum | other
  deriving DecidableEq, Repr

inductive Transition where
  | discharge | underflowClamp | charge | overflowClamp
  deriving DecidableEq, Repr

/-- `guard = (isOverflowShape, taken)`. -/
def decide' : Form → Option (Bool × Bool) → Bool → Option Transition
  | .subtract, some (false, false), true => some .discharge
  | .zero, some (false, true), _ => some .underflowClamp
  | .add, some (true, false), true => some .charge
  | .maximum, some (true, true), _ => some .overflowClamp
  | _, _, _ => none

def forms : List Form := [.subtract, .add, .zero, .maximum, .other]
def guards : List (Option (Bool × Bool)) :=
  [none, some (false, false), some (false, true), some (true, false), some (true, true)]
def bools : List Bool := [false, true]

def admitted : List (Form × Option (Bool × Bool) × Bool × Transition) :=
  forms.flatMap fun f => guards.flatMap fun g => bools.filterMap fun c =>
    (decide' f g c).map fun t => (f, g, c, t)

/-- The same six admitted rows as the Rust `decision_vectors` test. -/
theorem decide_vectors : admitted =
    [(.subtract, some (false, false), true, .discharge),
     (.add, some (true, false), true, .charge),
     (.zero, some (false, true), false, .underflowClamp),
     (.zero, some (false, true), true, .underflowClamp),
     (.maximum, some (true, true), false, .overflowClamp),
     (.maximum, some (true, true), true, .overflowClamp)] := by decide

/-- No transition without a guard on the ledger. -/
theorem unguarded_refuses (f : Form) (c : Bool) : decide' f none c = none := by
  cases f <;> cases c <;> rfl

/-- An update with a dirty window refuses. -/
theorem dirty_update_refuses (g : Option (Bool × Bool)) :
    decide' .subtract g false = none ∧ decide' .add g false = none := by
  cases g with
  | none => exact ⟨rfl, rfl⟩
  | some p => obtain ⟨a, b⟩ := p; cases a <;> cases b <;> exact ⟨rfl, rfl⟩

/-- An admitted discharge is under the untaken underflow guard, so in
    `size_t` arithmetic it computes exactly `satSub`. -/
theorem discharge_is_exact (g : Option (Bool × Bool)) (c : Bool)
    (h : decide' .subtract g c = some .discharge)
    (m l d : Nat) (hl : l < m) (guard : ¬ d > l) : cSub m l d = satSub l d := by
  have _ := h
  rw [guarded_sub_does_not_wrap m l d hl guard]; unfold satSub; simp [guard]

/-! ## The ledger invariant -/

structure Entry where
  id : Nat
  size : Nat
  resident : Bool
  deriving DecidableEq, Repr

def total : List Entry → Nat
  | [] => 0
  | e :: q => e.size + total q

def residentTotal : List Entry → Nat
  | [] => 0
  | e :: q => (if e.resident then e.size else 0) + residentTotal q

structure State where
  queue : List Entry
  ledger : Nat
  deriving DecidableEq, Repr

/-- The conservation invariant OB-8 names. -/
def Inv (s : State) : Prop := s.ledger = total s.queue

/-- `sb_append_chunk_to_segregated_backlog`: link at the head, then charge. -/
def append (max : Nat) (s : State) (e : Entry) : State :=
  { queue := e :: s.queue, ledger := satAdd max s.ledger e.size }

/-- P-LEDGER-PHYSICAL: the charged bytes fit the ledger type. -/
def Bounded (max : Nat) (s : State) (e : Entry) : Prop := total s.queue + e.size ≤ max

theorem append_preserves (max : Nat) (s : State) (e : Entry)
    (hinv : Inv s) (hb : Bounded max s e) : Inv (append max s e) := by
  unfold Inv at *; unfold append Bounded at *; simp only [total]
  rw [satAdd_exact _ _ _ (by omega)]; omega

theorem append_overflow_clamp_unreachable (max : Nat) (s : State) (e : Entry)
    (hinv : Inv s) (hb : Bounded max s e) : ¬ e.size > max - s.ledger := by
  unfold Inv Bounded at *; omega

/-- The first wrapper of the target (`chunk->chunk == target_chunk`, then
    `break`), unlinked. -/
def removeFirst (t : Nat) : List Entry → Option (Entry × List Entry)
  | [] => none
  | e :: q =>
    if e.id = t then some (e, q)
    else (removeFirst t q).map fun (r, q') => (r, e :: q')

/-- `sb_remove_chunk_from_segregated_backlog`: unlink, then discharge the
    stored size; no wrapper, no change. -/
def remove (s : State) (t : Nat) : State :=
  match removeFirst t s.queue with
  | none => s
  | some (e, q) => { queue := q, ledger := satSub s.ledger e.size }

theorem removeFirst_total (t : Nat) (q : List Entry) (e : Entry) (q' : List Entry)
    (h : removeFirst t q = some (e, q')) : total q = e.size + total q' := by
  induction q generalizing q' with
  | nil => simp [removeFirst] at h
  | cons x xs ih =>
    unfold removeFirst at h
    split at h
    · simp at h; obtain ⟨rfl, rfl⟩ := h; rfl
    · cases hr : removeFirst t xs with
      | none => simp [hr] at h
      | some p =>
        obtain ⟨r, rest⟩ := p
        simp [hr] at h
        obtain ⟨rfl, rfl⟩ := h
        have := ih rest hr
        simp only [total]; omega

theorem remove_underflow_clamp_unreachable (s : State) (t : Nat) (e : Entry) (q : List Entry)
    (hinv : Inv s) (h : removeFirst t s.queue = some (e, q)) : ¬ e.size > s.ledger := by
  have := removeFirst_total t s.queue e q h
  unfold Inv at hinv; omega

theorem remove_preserves (s : State) (t : Nat) (hinv : Inv s) : Inv (remove s t) := by
  unfold remove
  cases h : removeFirst t s.queue with
  | none => exact hinv
  | some p =>
    obtain ⟨e, q⟩ := p
    have ht := removeFirst_total t s.queue e q h
    unfold Inv at *; simp only
    have := satSub_exact s.ledger e.size (by omega)
    omega

theorem removeFirst_none_of_absent (t : Nat) (q : List Entry)
    (h : t ∉ q.map Entry.id) : removeFirst t q = none := by
  induction q with
  | nil => rfl
  | cons x xs ih =>
    simp only [List.map, List.mem_cons, not_or] at h
    unfold removeFirst
    simp [show ¬ x.id = t from fun e => h.1 e.symm, ih h.2]

theorem removeFirst_removes (t : Nat) (q : List Entry) (e : Entry) (q' : List Entry)
    (hnd : (q.map Entry.id).Nodup) (h : removeFirst t q = some (e, q')) :
    t ∉ q'.map Entry.id := by
  induction q generalizing q' with
  | nil => simp [removeFirst] at h
  | cons x xs ih =>
    simp only [List.map, List.nodup_cons] at hnd
    unfold removeFirst at h
    split at h
    · simp at h; obtain ⟨rfl, rfl⟩ := h
      rename_i hx; rw [← hx]; exact hnd.1
    · cases hr : removeFirst t xs with
      | none => simp [hr] at h
      | some p =>
        obtain ⟨r, rest⟩ := p
        simp [hr] at h
        obtain ⟨rfl, rfl⟩ := h
        rename_i hx
        have := ih rest hnd.2 hr
        simp only [List.map, List.mem_cons, not_or]
        exact ⟨fun e => hx e.symm, this⟩

/-- Repeated removal: with one wrapper per chunk per queue (OB-3, a premise
    here), the second removal of the same chunk finds nothing and changes
    nothing, so it cannot discharge twice. -/
theorem repeated_removal_is_a_no_op (s : State) (t : Nat)
    (hnd : (s.queue.map Entry.id).Nodup) : remove (remove s t) t = remove s t := by
  unfold remove
  cases h : removeFirst t s.queue with
  | none => simp [h]
  | some p =>
    obtain ⟨e, q⟩ := p
    simp only
    rw [removeFirst_none_of_absent t q (removeFirst_removes t s.queue e q hnd h)]

/-- Residency up/down changes no charge (`cio_chunk_up_force` /
    `cio_chunk_down` touch the physical state only). -/
def setResidency (b : Bool) (t : Nat) (s : State) : State :=
  { s with queue := s.queue.map fun e => if e.id = t then { e with resident := b } else e }

theorem total_map_residency (b : Bool) (t : Nat) (q : List Entry) :
    total (q.map fun e => if e.id = t then { e with resident := b } else e) = total q := by
  induction q with
  | nil => rfl
  | cons x xs ih => simp only [List.map, total]; split <;> simp [ih]

theorem residency_preserves (b : Bool) (t : Nat) (s : State) (hinv : Inv s) :
    Inv (setResidency b t s) := by
  unfold Inv setResidency at *; simp only; rw [total_map_residency]; exact hinv

theorem resident_le_total (q : List Entry) : residentTotal q ≤ total q := by
  induction q with
  | nil => exact Nat.le_refl 0
  | cons x xs ih => simp only [residentTotal, total]; split <;> omega

/-- The rollback that unlinks and discharges restores the state exactly. -/
theorem rollback_restores (max : Nat) (s : State) (e : Entry)
    (hinv : Inv s) (hb : Bounded max s e) : remove (append max s e) e.id = s := by
  have hx : satAdd max s.ledger e.size = s.ledger + e.size :=
    satAdd_exact _ _ _ (by unfold Inv Bounded at *; omega)
  have h1 : removeFirst e.id (e :: s.queue) = some (e, s.queue) := by simp [removeFirst]
  unfold remove append
  rw [h1]; simp only
  rw [hx, satSub, ite_eq_right_iff.mpr (by omega)]
  cases s; simp

/-! ### Mutations rule 92's pairing refuses -/

/-- The removal that discharges twice. -/
def removeTwice (s : State) (t : Nat) : State :=
  match removeFirst t s.queue with
  | none => s
  | some (e, q) => { queue := q, ledger := satSub (satSub s.ledger e.size) e.size }

/-- The removal (or rollback) that unlinks without discharging. -/
def removeWithout (s : State) (t : Nat) : State :=
  match removeFirst t s.queue with
  | none => s
  | some (_, q) => { queue := q, ledger := s.ledger }

/-- A doubled discharge either breaks the invariant or is hidden by the
    underflow clamp of its second update (the queue's remaining charges are
    smaller than the size): the arithmetic alone cannot see it, which is why
    rule 92 refuses it statically by pairing. -/
theorem doubled_discharge_breaks_or_clamps (s : State) (t : Nat) (e : Entry) (q : List Entry)
    (hinv : Inv s) (h : removeFirst t s.queue = some (e, q)) (hpos : 0 < e.size) :
    ¬ Inv (removeTwice s t) ∨ e.size > s.ledger - e.size := by
  have ht := removeFirst_total t s.queue e q h
  unfold Inv at hinv
  have h1 : satSub s.ledger e.size = s.ledger - e.size := by
    unfold satSub; simp [show ¬ e.size > s.ledger by omega]
  by_cases h2 : e.size > s.ledger - e.size
  · exact Or.inr h2
  · left
    unfold removeTwice Inv; rw [h]; simp only
    rw [h1]; unfold satSub; simp only [h2, ite_false]; omega

/-- With at least the size still charged to the rest of the queue, the
    doubled discharge breaks the invariant outright. -/
theorem doubled_discharge_breaks (s : State) (t : Nat) (e : Entry) (q : List Entry)
    (hinv : Inv s) (h : removeFirst t s.queue = some (e, q)) (hpos : 0 < e.size)
    (hrest : e.size ≤ total q) : ¬ Inv (removeTwice s t) := by
  have ht := removeFirst_total t s.queue e q h
  unfold Inv at hinv
  rcases doubled_discharge_breaks_or_clamps s t e q hinv h hpos with h3 | h3
  · exact h3
  · omega

theorem missing_discharge_breaks (s : State) (t : Nat) (e : Entry) (q : List Entry)
    (hinv : Inv s) (h : removeFirst t s.queue = some (e, q)) (hpos : 0 < e.size) :
    ¬ Inv (removeWithout s t) := by
  have ht := removeFirst_total t s.queue e q h
  unfold removeWithout Inv at *; rw [h]; simp only; omega

/-! ### Shared chunks and partial fan-out -/

/-- Fan-out charges one per-output ledger per routed queue; a failure after
    `k` queues (`return -3`, `sb.c:349`) leaves exactly the first `k`
    charged. -/
def fanoutPrefix (max : Nat) (k : Nat) (ss : List State) (e : Entry) : List State :=
  (ss.take k).map (fun s => append max s e) ++ ss.drop k

theorem partial_fanout_preserves (max k : Nat) (ss : List State) (e : Entry)
    (h : ∀ s ∈ ss, Inv s ∧ Bounded max s e) : ∀ s ∈ fanoutPrefix max k ss e, Inv s := by
  intro s hs
  unfold fanoutPrefix at hs
  rcases List.mem_append.mp hs with hs | hs
  · obtain ⟨s0, h0, rfl⟩ := List.mem_map.mp hs
    have := h s0 (List.mem_of_mem_take h0)
    exact append_preserves max s0 e this.1 this.2
  · exact (h s (List.mem_of_mem_drop hs)).1

/-! ## Admission, batch overshoot, signed magnitude -/

/-- `flb_input_chunk_output_would_exceed_limit` as written. -/
def wouldExceed (limit fs bl c : Nat) : Bool :=
  if fs ≤ limit then
    if bl ≤ limit - fs then decide (c > limit - fs - bl) else true
  else true

/-- The exact admission inequality: admit iff `fs + bl + c ≤ T`. -/
theorem wouldExceed_iff (limit fs bl c : Nat) :
    wouldExceed limit fs bl c = true ↔ fs + bl + c > limit := by
  unfold wouldExceed
  split
  · split
    · simp; omega
    · simp; omega
  · simp; omega

def sumList : List Nat → Nat
  | [] => 0
  | x :: xs => x + sumList xs

/-- `cb_queue_chunks`: tested once before a round, one chunk per queue per
    round. The round ends below `limit + (chunks) * (largest chunk)`. -/
theorem batch_overshoot_bound (start limit m : Nat) (xs : List Nat)
    (h : start < limit) (hs : ∀ x ∈ xs, x ≤ m) :
    start + sumList xs < limit + xs.length * m := by
  induction xs with
  | nil => simp [sumList]; omega
  | cons x xs ih =>
    have hx := hs x (List.mem_cons_self ..)
    have := ih (fun y hy => hs y (List.mem_cons_of_mem _ hy))
    simp only [sumList, List.length_cons, Nat.succ_mul]; omega

/-- The bound is attained up to one byte: the test does not prevent the
    overshoot. -/
theorem batch_overshoot_reachable (limit m : Nat) (hm : 0 < m) (hl : 0 < limit) :
    (limit - 1) + sumList [m] ≥ limit := by
  simp [sumList]; omega

/-- `bytes = (size_t) (-(chunk_size + 1)) + 1` for `chunk_size < 0`. -/
theorem negative_magnitude (x : Int) (h : x < 0) : (-(x + 1)).toNat + 1 = x.natAbs := by
  omega

/-- The intermediate `-(chunk_size + 1)` is representable in `ssize_t`. -/
theorem negative_magnitude_in_range (x : Int) (lo : -(2 ^ 63) ≤ x) (h : x < 0) :
    0 ≤ -(x + 1) ∧ -(x + 1) ≤ 2 ^ 63 - 1 := by
  omega

end QCore.BacklogLedger
