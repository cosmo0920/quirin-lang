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

import QCore.WeakMemory

/-!
# Execution coverage over a bounded fragment (card CG29-EXECUTION-COVERAGE-AND-RECLAMATION)

The Rust side is `crates/qcore-core/src/execution_coverage.rs`
(`check_program_executions`). `QCore.WeakMemory.check` decides **one**
supplied candidate execution; one accepted graph is not race freedom (gap
entry W07, evidence Q34). This module enumerates every candidate execution
of a loop-free fragment and decides race freedom over all of them.

A fragment declares its atomic locations with their initial values, and per
thread a list of guarded instructions: an atomic load, store, exchange or
compare-exchange, a fence, a non-atomic read or write. A guard is a
conjunction of register equalities, evaluated when the instruction is
reached. A load or exchange writes the value it read to a register; a
compare-exchange writes 1 on success and 0 on failure.

A **candidate execution** is the tuple C11 quantifies over:
* a path of each thread, for some value read by each read and some outcome of
  each weak compare-exchange on its expected value (`run`, with any choices);
* a reads-from source for each read;
* a modification order per atomic location, a total order of its writes,
  represented by its enumeration (a permutation) and encoded as successive
  pairs;
* an order S of the seq_cst operations and fences (a permutation);
* synchronises-with, derived from these (`Execution.swEdge`).

It is encoded as `QCore.WeakMemory.Execution` (`encode`), with a thread-0
prelude holding each location's initialisation and one spawn per thread, so
the initialisations happen before every thread.

What is proved:
* **Enumeration completeness** (`encode_mem_candidates`): every candidate
  execution whose read values lie in the value domain is generated.
* **Value-domain soundness** (`read_values_are_literals`): in every candidate
  execution that passes reads-from validation, every read returns a literal
  the fragment writes to that location, or its initial value. The fragment
  has no arithmetic read-modify-write, so this needs no modification-order
  argument, and no out-of-thin-air cycle can escape it: the domain is
  computed without control flow.
* **Verdict soundness** (`verdict_race_free_sound`): `raceFree` means every
  candidate execution, for **any** read values and outcomes, that is
  consistent (every check of `QCore.WeakMemory.check` except the race check
  passes) is race-free. `race` means a consistent candidate races
  (`verdict_race_witnessed`), and every candidate is an execution of the
  fragment (`candidate_is_execution`).
* Loops, a malformed fragment and every exceeded bound give `unknown` with a
  named reason (`loop_is_unknown`, `race_free_needs_bounds`).

The bounds (threads, events, values per location, candidates) are part of the
verdict's evidence identity. Within them the enumeration is exact, not an
under-approximation: the fragment is loop-free, so every path is finite and
generated.
-/

namespace QCore.ExecutionCoverage

open QCore.WeakMemory

/-! ## Generic enumerations -/

/-- `a` inserted at every position of a list. -/
def insertAll (a : α) : List α → List (List α)
  | [] => [[a]]
  | b :: l => (a :: b :: l) :: (insertAll a l).map (b :: ·)

/-- Every permutation of a list. -/
def perms : List α → List (List α)
  | [] => [[]]
  | a :: l => (perms l).flatMap (insertAll a)

theorem mem_insertAll (a : α) : ∀ (s t : List α), s ++ a :: t ∈ insertAll a (s ++ t)
  | [], [] => by simp [insertAll]
  | [], b :: t => by simp [insertAll]
  | b :: s, t => by
    simp only [List.cons_append, insertAll, List.mem_cons, List.mem_map]
    exact .inr ⟨s ++ a :: t, mem_insertAll a s t, rfl⟩

/-- Every permutation is enumerated. -/
theorem perm_mem_perms {l : List α} : ∀ {l' : List α}, l'.Perm l → l' ∈ perms l := by
  induction l with
  | nil =>
    intro l' h
    simp [perms, List.perm_nil.mp h]
  | cons a t ih =>
    intro l' h
    have ha : a ∈ l' := h.symm.mem_iff.mp (List.mem_cons_self ..)
    obtain ⟨s, u, rfl⟩ := List.append_of_mem ha
    have hp : (s ++ u).Perm t :=
      (List.perm_cons a).mp (List.perm_middle.symm.trans h)
    simp only [perms, List.mem_flatMap]
    exact ⟨s ++ u, ih hp, mem_insertAll a s u⟩

/-- Every choice of one element per list. -/
def choices : List (List α) → List (List α)
  | [] => [[]]
  | l :: ls => l.flatMap fun a => (choices ls).map (a :: ·)

/-- `xs` takes its `i`-th element from the `i`-th list. -/
def AllIn : List α → List (List α) → Prop
  | [], [] => True
  | x :: xs, l :: ls => x ∈ l ∧ AllIn xs ls
  | _, _ => False

theorem mem_choices_iff : ∀ {xs : List α} {ls : List (List α)}, xs ∈ choices ls ↔ AllIn xs ls
  | [], [] => by simp [choices, AllIn]
  | x :: xs, [] => by simp [choices, AllIn]
  | [], l :: ls => by simp [choices, AllIn]
  | x :: xs, l :: ls => by
    simp only [choices, List.mem_flatMap, List.mem_map, AllIn]
    constructor
    · rintro ⟨a, ha, ys, hys, heq⟩
      cases heq
      exact ⟨ha, mem_choices_iff.mp hys⟩
    · rintro ⟨hx, hxs⟩
      exact ⟨x, hx, xs, mem_choices_iff.mpr hxs, rfl⟩

/-- Permutations, list by list. -/
def AllPerm : List (List α) → List (List α) → Prop
  | [], [] => True
  | x :: xs, l :: ls => x.Perm l ∧ AllPerm xs ls
  | _, _ => False

theorem allIn_perms : ∀ {xs ls : List (List α)}, AllPerm xs ls → AllIn xs (ls.map perms)
  | [], [], _ => trivial
  | _ :: _, _ :: _, ⟨h, hs⟩ => ⟨perm_mem_perms h, allIn_perms hs⟩
  | [], _ :: _, h => h.elim
  | _ :: _, [], h => h.elim

/-- Duplicates removed (membership is unchanged, `mem_nub`). -/
def nub : List Nat → List Nat
  | [] => []
  | a :: l => if l.contains a then nub l else a :: nub l

theorem mem_nub {a : Nat} : ∀ {l : List Nat}, a ∈ nub l ↔ a ∈ l
  | [] => by simp [nub]
  | b :: l => by
    unfold nub
    by_cases h : l.contains b
    · simp only [h, ↓reduceIte, mem_nub, List.mem_cons]
      constructor
      · exact .inr
      · rintro (rfl | h')
        · simpa using h
        · exact h'
    · have h' : b ∉ l := by simpa using h
      simp [h', mem_nub]

/-! ## Fragments -/

inductive Instr where
  | load (loc : Nat) (o : Order) (reg : Nat)
  | store (loc v : Nat) (o : Order)
  /-- An exchange: writes the literal `v`; its register receives the old value. -/
  | xchg (loc v : Nat) (o : Order) (reg : Nat)
  | cas (loc expected desired : Nat) (success failure : Order) (weak : Bool) (reg : Nat)
  | fence (o : Order) (signal : Bool)
  | naRead (loc : Nat)
  | naWrite (loc : Nat)
  /-- A loop: not enumerated, so a fragment holding one is `unknown`. -/
  | loop
deriving DecidableEq, Repr

structure GInstr where
  guard : List (Nat × Nat)
  instr : Instr
deriving DecidableEq, Repr

/-- `g guard instr`: the instruction, run only when every `(register, value)`
of the guard holds. -/
def g (guard : List (Nat × Nat)) (instr : Instr) : GInstr := ⟨guard, instr⟩

structure Frag where
  init : List (Nat × Nat)
  threads : List (List GInstr)
deriving Repr

def holds (env : List (Nat × Nat)) (guard : List (Nat × Nat)) : Bool :=
  guard.all fun p => env.lookup p.1 == some p.2

/-- The location an instruction reads, when it reads one. -/
def Instr.readLoc? : Instr → Option Nat
  | .load l _ _ | .xchg l _ _ _ | .cas l _ _ _ _ _ _ => some l
  | _ => none

/-- The event an executed instruction produces and its register writes,
given the value `v` it reads and whether a weak compare-exchange on its
expected value fails spuriously. -/
def Instr.step (i : Instr) (v : Nat) (spurious : Bool) : Option (Kind × List (Nat × Nat)) :=
  match i with
  | .load l o r => some (.load l v o, [(r, v)])
  | .store l w o => some (.store l w o, [])
  | .xchg l w o r => some (.rmw l v w o, [(r, v)])
  | .cas l e d s f weak r =>
    some (.cas l e v d s f weak (v == e && !(weak && spurious)),
      [(r, if v == e && !(weak && spurious) then 1 else 0)])
  | .fence o sig => some (.fence o sig, [])
  | .naRead l => some (.naRead l, [])
  | .naWrite l => some (.naWrite l, [])
  | .loop => none

/-- A thread's run for a supply of choices, one `(value, spurious)` per
executed reading instruction. Any choices: this is the declarative side. -/
def run (env : List (Nat × Nat)) : List GInstr → List (Nat × Bool) → Option (List Kind)
  | [], _ => some []
  | gi :: rest, cs =>
    if holds env gi.guard then
      match gi.instr.readLoc?, cs with
      | some _, (v, sp) :: cs' =>
        match gi.instr.step v sp with
        | some (k, upd) => (run (upd ++ env) rest cs').map (k :: ·)
        | none => none
      | some _, [] => none
      | none, cs =>
        match gi.instr.step 0 false with
        | some (k, upd) => (run (upd ++ env) rest cs).map (k :: ·)
        | none => none
    else run env rest cs

/-- The outcomes enumerated for a read of `v`: a weak compare-exchange on its
expected value may also fail spuriously. -/
def Instr.outcomes (i : Instr) (v : Nat) : List Bool :=
  match i with
  | .cas _ e _ _ _ true _ => if v == e then [false, true] else [false]
  | _ => [false]

/-- The generator: every path whose reads take values in `dom`. -/
def paths (dom : Nat → List Nat) (env : List (Nat × Nat)) : List GInstr → List (List Kind)
  | [] => [[]]
  | gi :: rest =>
    if holds env gi.guard then
      match gi.instr.readLoc? with
      | some l =>
        (dom l).flatMap fun v => (gi.instr.outcomes v).flatMap fun sp =>
          match gi.instr.step v sp with
          | some (k, upd) => (paths dom (upd ++ env) rest).map (k :: ·)
          | none => []
      | none =>
        match gi.instr.step 0 false with
        | some (k, upd) => (paths dom (upd ++ env) rest).map (k :: ·)
        | none => []
    else paths dom env rest

/-! ## Paths: completeness and soundness -/

theorem step_reads {i : Instr} {l v : Nat} {sp : Bool} {k : Kind} {upd : List (Nat × Nat)}
    (hl : i.readLoc? = some l) (hs : i.step v sp = some (k, upd)) :
    ∃ o, k.read? = some (l, v, o) := by
  cases i <;> simp_all [Instr.readLoc?, Instr.step, Kind.read?] <;>
    (obtain ⟨rfl, rfl⟩ := hs) <;> simp

theorem step_outcome (i : Instr) (v : Nat) (sp : Bool) :
    ∃ sp' ∈ i.outcomes v, i.step v sp' = i.step v sp := by
  cases i with
  | cas l e d s f weak r =>
    cases weak
    · exact ⟨false, by simp [Instr.outcomes], by simp [Instr.step]⟩
    · by_cases hv : v = e
      · exact ⟨sp, by cases sp <;> simp [Instr.outcomes, hv], rfl⟩
      · exact ⟨false, by simp [Instr.outcomes, hv], by simp [Instr.step, hv]⟩
  | _ => exact ⟨false, by simp [Instr.outcomes], by simp [Instr.step]⟩

/-- The read values of a path lie in `dom`. -/
def InDom (dom : Nat → List Nat) (ks : List Kind) : Prop :=
  ∀ k ∈ ks, ∀ l v o, k.read? = some (l, v, o) → v ∈ dom l

/-- **Path completeness**: every run whose read values lie in `dom` is
generated. -/
theorem paths_complete (dom : Nat → List Nat) :
    ∀ (is : List GInstr) (env : List (Nat × Nat)) (cs : List (Nat × Bool)) (ks : List Kind),
      run env is cs = some ks → InDom dom ks → ks ∈ paths dom env is := by
  intro is
  induction is with
  | nil => intro env cs ks h _; simp [run] at h; subst h; simp [paths]
  | cons gi rest ih =>
    intro env cs ks h hd
    unfold run at h
    unfold paths
    by_cases hg : holds env gi.guard
    · simp only [hg, ↓reduceIte] at h ⊢
      cases hr : gi.instr.readLoc? with
      | some l =>
        rw [hr] at h
        cases cs with
        | nil => simp at h
        | cons c cs' =>
          obtain ⟨v, sp⟩ := c
          simp only at h
          cases hs : gi.instr.step v sp with
          | none => rw [hs] at h; simp at h
          | some ku =>
            obtain ⟨k, upd⟩ := ku
            rw [hs] at h
            simp only [Option.map_eq_some_iff] at h
            obtain ⟨ks', hrun, rfl⟩ := h
            obtain ⟨o, hk⟩ := step_reads hr hs
            have hv : v ∈ dom l := hd k (List.mem_cons_self ..) l v o hk
            obtain ⟨sp', hsp', hsame⟩ := step_outcome gi.instr v sp
            simp only [List.mem_flatMap]
            refine ⟨v, hv, sp', hsp', ?_⟩
            rw [hsame, hs]
            exact List.mem_map.mpr ⟨ks', ih _ _ _ hrun (fun k' hk' => hd k' (List.mem_cons_of_mem _ hk')), rfl⟩
      | none =>
        rw [hr] at h
        simp only at h ⊢
        cases hs : gi.instr.step 0 false with
        | none => rw [hs] at h; simp at h
        | some ku =>
          obtain ⟨k, upd⟩ := ku
          rw [hs] at h
          simp only [Option.map_eq_some_iff] at h
          obtain ⟨ks', hrun, rfl⟩ := h
          exact List.mem_map.mpr ⟨ks', ih _ _ _ hrun (fun k' hk' => hd k' (List.mem_cons_of_mem _ hk')), rfl⟩
    · simp only [hg] at h ⊢
      exact ih env cs ks h hd

/-- **Path soundness**: every generated path is a run, for some choices. -/
theorem paths_sound (dom : Nat → List Nat) :
    ∀ (is : List GInstr) (env : List (Nat × Nat)) (ks : List Kind),
      ks ∈ paths dom env is → ∃ cs, run env is cs = some ks := by
  intro is
  induction is with
  | nil => intro env ks h; simp [paths] at h; exact ⟨[], by simp [run, h]⟩
  | cons gi rest ih =>
    intro env ks h
    unfold paths at h
    unfold run
    by_cases hg : holds env gi.guard
    · simp only [hg, ↓reduceIte] at h ⊢
      cases hr : gi.instr.readLoc? with
      | some l =>
        rw [hr] at h
        simp only [List.mem_flatMap] at h
        obtain ⟨v, _, sp, _, hm⟩ := h
        cases hs : gi.instr.step v sp with
        | none => rw [hs] at hm; simp at hm
        | some ku =>
          obtain ⟨k, upd⟩ := ku
          rw [hs] at hm
          obtain ⟨ks', hks', rfl⟩ := List.mem_map.mp hm
          obtain ⟨cs, hcs⟩ := ih _ _ hks'
          exact ⟨(v, sp) :: cs, by simp [hs, hcs]⟩
      | none =>
        rw [hr] at h
        simp only at h ⊢
        cases hs : gi.instr.step 0 false with
        | none => rw [hs] at h; simp at h
        | some ku =>
          obtain ⟨k, upd⟩ := ku
          rw [hs] at h
          obtain ⟨ks', hks', rfl⟩ := List.mem_map.mp h
          obtain ⟨cs, hcs⟩ := ih _ _ hks'
          exact ⟨cs, by simp [hcs]⟩
    · simp only [hg] at h ⊢
      exact ih env ks h

/-! ## Literals: what a write can write -/

/-- The literal an instruction writes to `l`, if any. -/
def GInstr.literalAt (gi : GInstr) (l : Nat) : Option Nat :=
  match gi.instr with
  | .store l' v _ | .xchg l' v _ _ | .cas l' _ v _ _ _ _ => if l' == l then some v else none
  | _ => none

def literalsOf (is : List GInstr) (l : Nat) : List Nat := is.filterMap (·.literalAt l)

/-- The values location `l` can hold: its initial value and every literal any
instruction writes to it. -/
def Frag.literals (F : Frag) (l : Nat) : List Nat :=
  (F.init.filterMap fun p => if p.1 == l then some p.2 else none) ++
    F.threads.flatMap (literalsOf · l)

/-- The value domain of `l` (`value_domain` in Rust). -/
def Frag.domain (F : Frag) (l : Nat) : List Nat := nub (F.literals l)

theorem step_write_literal {gi : GInstr} {v : Nat} {sp : Bool} {k : Kind} {upd : List (Nat × Nat)}
    (hs : gi.instr.step v sp = some (k, upd)) {l w : Nat} {o : Option Order}
    (hw : k.write? = some (l, w, o)) : gi.literalAt l = some w := by
  unfold GInstr.literalAt
  cases h : gi.instr with
  | load l' o' r =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs; simp [Kind.write?] at hw
  | store l' w' o' =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs
    simp only [Kind.write?, Option.some.injEq, Prod.mk.injEq] at hw
    obtain ⟨rfl, rfl, -⟩ := hw; simp
  | xchg l' w' o' r =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs
    simp only [Kind.write?, Option.some.injEq, Prod.mk.injEq] at hw
    obtain ⟨rfl, rfl, -⟩ := hw; simp
  | cas l' e d s f weak r =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs
    cases hc : (v == e && !(weak && sp))
    · rw [hc] at hw; simp [Kind.write?] at hw
    · rw [hc] at hw
      simp only [Kind.write?, Option.some.injEq, Prod.mk.injEq] at hw
      obtain ⟨rfl, rfl, -⟩ := hw; simp
  | fence o' sig =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs; simp [Kind.write?] at hw
  | naRead l' =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs; simp [Kind.write?] at hw
  | naWrite l' =>
    rw [h] at hs; simp only [Instr.step, Option.some.injEq, Prod.mk.injEq] at hs
    obtain ⟨rfl, -⟩ := hs; simp [Kind.write?] at hw
  | loop => rw [h] at hs; simp [Instr.step] at hs

theorem literalsOf_cons_of_tail {gi : GInstr} {rest : List GInstr} {l w : Nat}
    (h : w ∈ literalsOf rest l) : w ∈ literalsOf (gi :: rest) l := by
  unfold literalsOf at h ⊢
  rw [List.filterMap_cons]
  cases gi.literalAt l <;> simp_all

theorem literalsOf_cons_of_head {gi : GInstr} {rest : List GInstr} {l w : Nat}
    (h : gi.literalAt l = some w) : w ∈ literalsOf (gi :: rest) l := by
  unfold literalsOf
  rw [List.filterMap_cons, h]
  exact List.mem_cons_self ..

/-- Every write a run makes writes a literal of the thread. -/
theorem run_write_literal :
    ∀ (is : List GInstr) (env : List (Nat × Nat)) (cs : List (Nat × Bool)) (ks : List Kind),
      run env is cs = some ks →
        ∀ k ∈ ks, ∀ l w o, k.write? = some (l, w, o) → w ∈ literalsOf is l := by
  intro is
  induction is with
  | nil => intro env cs ks h; simp [run] at h; subst h; simp
  | cons gi rest ih =>
    intro env cs ks h k hk l w o hw
    unfold run at h
    by_cases hg : holds env gi.guard
    · simp only [hg, ↓reduceIte] at h
      cases hr : gi.instr.readLoc? with
      | some l' =>
        rw [hr] at h
        cases cs with
        | nil => simp at h
        | cons c cs' =>
          obtain ⟨v, sp⟩ := c
          simp only at h
          cases hs : gi.instr.step v sp with
          | none => rw [hs] at h; simp at h
          | some ku =>
            obtain ⟨k', upd⟩ := ku
            rw [hs] at h
            simp only [Option.map_eq_some_iff] at h
            obtain ⟨ks', hrun, rfl⟩ := h
            rcases List.mem_cons.mp hk with rfl | hk
            · exact literalsOf_cons_of_head (step_write_literal hs hw)
            · exact literalsOf_cons_of_tail (ih _ _ _ hrun k hk l w o hw)
      | none =>
        rw [hr] at h
        simp only at h
        cases hs : gi.instr.step 0 false with
        | none => rw [hs] at h; simp at h
        | some ku =>
          obtain ⟨k', upd⟩ := ku
          rw [hs] at h
          simp only [Option.map_eq_some_iff] at h
          obtain ⟨ks', hrun, rfl⟩ := h
          rcases List.mem_cons.mp hk with rfl | hk
          · exact literalsOf_cons_of_head (step_write_literal hs hw)
          · exact literalsOf_cons_of_tail (ih _ _ _ hrun k hk l w o hw)
    · simp only [hg] at h
      exact literalsOf_cons_of_tail (ih env cs ks h k hk l w o hw)

/-! ## Assembling the events -/

/-- Each thread's path, tagged with its thread number. -/
def tagThreads : Nat → List (List Kind) → List (Nat × Kind)
  | _, [] => []
  | t, p :: ps => p.map (t, ·) ++ tagThreads (t + 1) ps

/-- Events numbered from `n`: the id of each event is its position. -/
def number : Nat → List (Nat × Kind) → List Event
  | _, [] => []
  | n, (t, k) :: rest => ⟨n, t, k⟩ :: number (n + 1) rest

/-- Thread 0: each location's initialisation, then one spawn per thread. -/
def Frag.prelude (F : Frag) : List (Nat × Kind) :=
  (F.init.map fun p => (0, Kind.init p.1 p.2)) ++
    (List.range F.threads.length).map fun i => (0, Kind.spawn (i + 1))

def assemble (F : Frag) (ps : List (List Kind)) : List Event :=
  number 0 (F.prelude ++ tagThreads 1 ps)

theorem number_id_ge : ∀ (n : Nat) (l : List (Nat × Kind)) (e : Event), e ∈ number n l → n ≤ e.id
  | _, [], _, h => by simp [number] at h
  | n, (t, k) :: rest, e, h => by
    simp only [number, List.mem_cons] at h
    rcases h with rfl | h
    · simp
    · exact Nat.le_of_succ_le (number_id_ge (n + 1) rest e h)

/-- An event is found by its own id. -/
theorem number_find : ∀ (n : Nat) (l : List (Nat × Kind)) (e : Event), e ∈ number n l →
    (number n l).find? (·.id == e.id) = some e
  | _, [], _, h => by simp [number] at h
  | n, (t, k) :: rest, e, h => by
    simp only [number, List.mem_cons] at h ⊢
    rcases h with rfl | h
    · simp
    · have hge := number_id_ge (n + 1) rest e h
      have hne : (n == e.id) = false := by
        simp only [beq_eq_false_iff_ne]; omega
      simp [hne, number_find (n + 1) rest e h]

theorem number_mem : ∀ (n : Nat) (l : List (Nat × Kind)) (e : Event), e ∈ number n l →
    (e.thread, e.kind) ∈ l
  | _, [], _, h => by simp [number] at h
  | n, (t, k) :: rest, e, h => by
    simp only [number, List.mem_cons] at h
    rcases h with rfl | h
    · simp
    · exact List.mem_cons_of_mem _ (number_mem (n + 1) rest e h)

theorem kindOf_event {F : Frag} {ps : List (List Kind)} {x : Execution}
    (hx : x.events = assemble F ps) {e : Event} (he : e ∈ x.events) :
    x.kindOf e.id = some e.kind := by
  simp only [Execution.kindOf, Execution.lookup, hx]
  rw [hx] at he
  unfold assemble at he ⊢
  rw [number_find 0 _ e he]
  rfl

theorem kindOf_mem {x : Execution} {id : Nat} {k : Kind} (h : x.kindOf id = some k) :
    ∃ e ∈ x.events, e.id = id ∧ e.kind = k := by
  simp only [Execution.kindOf, Execution.lookup, Option.map_eq_some_iff] at h
  obtain ⟨e, he, rfl⟩ := h
  exact ⟨e, List.mem_of_find?_eq_some he, by simpa using List.find?_some he, rfl⟩

/-- The paths, each a run of its thread for some choices. -/
def AllRun : List (List GInstr) → List (List Kind) → Prop
  | [], [] => True
  | t :: ts, p :: ps => (∃ cs, run [] t cs = some p) ∧ AllRun ts ps
  | _, _ => False

theorem tag_mem : ∀ (n : Nat) (ps : List (List Kind)) (t : Nat) (k : Kind),
    (t, k) ∈ tagThreads n ps → ∃ p ∈ ps, k ∈ p
  | _, [], _, _, h => by simp [tagThreads] at h
  | n, p :: ps, t, k, h => by
    simp only [tagThreads, List.mem_append, List.mem_map] at h
    rcases h with ⟨k', hk', heq⟩ | h
    · cases heq; exact ⟨p, List.mem_cons_self .., hk'⟩
    · obtain ⟨p', hp', hk⟩ := tag_mem (n + 1) ps t k h
      exact ⟨p', List.mem_cons_of_mem _ hp', hk⟩

theorem allRun_write_literal :
    ∀ (ts : List (List GInstr)) (ps : List (List Kind)), AllRun ts ps →
      ∀ p ∈ ps, ∀ k ∈ p, ∀ l w o, k.write? = some (l, w, o) →
        w ∈ ts.flatMap (literalsOf · l) := by
  intro ts
  induction ts with
  | nil => intro ps h; cases ps <;> simp_all [AllRun]
  | cons t ts ih =>
    intro ps h q hq k hk l w o hw
    cases ps with
    | nil => exact h.elim
    | cons p ps =>
      obtain ⟨⟨cs, hrun⟩, hrest⟩ := h
      simp only [List.flatMap_cons, List.mem_append]
      rcases List.mem_cons.mp hq with rfl | hq
      · exact .inl (run_write_literal t [] cs q hrun k hk l w o hw)
      · exact .inr (ih ps hrest q hq k hk l w o hw)

/-- Every write of an assembled execution writes a literal of its location
(an initial value, or a literal of the fragment). -/
theorem assemble_write_literal {F : Frag} {ps : List (List Kind)} (hruns : AllRun F.threads ps)
    {e : Event} (he : e ∈ assemble F ps) {l w : Nat} {o : Option Order}
    (hw : e.kind.write? = some (l, w, o)) : w ∈ F.literals l := by
  have hm := number_mem 0 _ e he
  simp only [List.mem_append, Frag.prelude, List.mem_map, List.mem_range] at hm
  simp only [Frag.literals, List.mem_append, List.mem_filterMap]
  rcases hm with (⟨p, hp, heq⟩ | ⟨i, _, heq⟩) | htag
  · have hk := (Prod.mk.inj heq).2
    rw [← hk] at hw
    simp only [Kind.write?, Option.some.injEq, Prod.mk.injEq] at hw
    obtain ⟨rfl, rfl, -⟩ := hw
    exact .inl ⟨p, hp, by simp⟩
  · have hk := (Prod.mk.inj heq).2
    rw [← hk] at hw
    simp [Kind.write?] at hw
  · obtain ⟨p, hp, hk⟩ := tag_mem 1 ps _ _ htag
    exact .inr (allRun_write_literal _ _ hruns p hp _ hk l w o hw)

theorem assemble_kind_mem {F : Frag} {ps : List (List Kind)} {p : List Kind} (hp : p ∈ ps)
    {k : Kind} (hk : k ∈ p) : ∃ e ∈ assemble F ps, e.kind = k := by
  have tag : ∀ (n : Nat) (ps : List (List Kind)), p ∈ ps → ∃ t, (t, k) ∈ tagThreads n ps := by
    intro n ps hp
    induction ps generalizing n with
    | nil => simp at hp
    | cons q qs ih =>
      simp only [tagThreads, List.mem_append, List.mem_map]
      rcases List.mem_cons.mp hp with rfl | hp
      · exact ⟨n, .inl ⟨k, hk, rfl⟩⟩
      · obtain ⟨t, ht⟩ := ih (n + 1) hp
        exact ⟨t, .inr ht⟩
  obtain ⟨t, ht⟩ := tag 1 ps hp
  have numbered : ∀ (n : Nat) (l : List (Nat × Kind)), (t, k) ∈ l → ∃ e ∈ number n l, e.kind = k := by
    intro n l hl
    induction l generalizing n with
    | nil => simp at hl
    | cons q qs ih =>
      obtain ⟨t', k'⟩ := q
      simp only [number, List.mem_cons]
      rcases List.mem_cons.mp hl with heq | hl
      · cases heq; exact ⟨_, .inl rfl, rfl⟩
      · obtain ⟨e, he, hek⟩ := ih (n + 1) hl
        exact ⟨e, .inr he, hek⟩
  exact numbered 0 _ (List.mem_append_right _ ht)

/-! ## Candidate executions -/

def readEvents (evs : List Event) : List Event := evs.filter fun e => e.kind.read?.isSome

/-- Whether the write `w` can be read by the read `r`: same location, same
value (`rfValid`'s condition). -/
def readable (w r : Kind) : Bool :=
  match w.write?, r.read? with
  | some (wl, wv, _), some (rl, rv, _) => wl == rl && wv == rv
  | _, _ => false

/-- The reads-from sources of each read. -/
def rfSources (evs : List Event) : List (List Nat) :=
  (readEvents evs).map fun r => (evs.filter fun w => readable w.kind r.kind).map (·.id)

/-- The writes of one location. -/
def writesAt (evs : List Event) (l : Nat) : List Nat :=
  (evs.filter fun e => (e.kind.write?.map (·.1)) == some l).map (·.id)

def moLists (F : Frag) (evs : List Event) : List (List Nat) :=
  F.init.map fun p => writesAt evs p.1

/-- A total order, encoded by its successive pairs. -/
def chain : List Nat → List (Nat × Nat)
  | a :: b :: rest => (a, b) :: chain (b :: rest)
  | _ => []

def scMembers (evs : List Event) : List Nat := (evs.filter Event.isScMember).map (·.id)

/-- The checker's graph of a candidate execution: reads-from pairs each read
with its chosen source, the modification order is the successive pairs of
each location's order, and synchronises-with is derived. -/
def encode (evs : List Event) (srcs : List Nat) (mos : List (List Nat)) (sc : List Nat) :
    Execution :=
  let x : Execution :=
    { events := evs, rf := srcs.zip ((readEvents evs).map (·.id)), mo := mos.flatMap chain,
      sw := [], sc := sc }
  { x with sw := x.pairs.filter fun p => x.swEdge p.1 p.2 }

/-- Every candidate execution with read values in the domain. -/
def candidates (F : Frag) : List Execution :=
  (choices (F.threads.map (paths F.domain []))).flatMap fun ps =>
    (choices (rfSources (assemble F ps))).flatMap fun srcs =>
      (choices ((moLists F (assemble F ps)).map perms)).flatMap fun mos =>
        (perms (scMembers (assemble F ps))).map fun sc => encode (assemble F ps) srcs mos sc

/-- Every check of `QCore.WeakMemory.check` except the race check. -/
def consistent (x : Execution) : Bool :=
  x.ordersValid && x.rfValid && x.moValid && x.casValid && x.atomicityHolds && x.swValid &&
    x.acyclic && x.coherent && x.scValid

theorem check_nil_iff (x : Execution) :
    check x = [] ↔ consistent x = true ∧ x.raceFree = true := by
  rw [check_accepts_iff]
  simp only [consistent, Bool.and_eq_true, and_assoc]

/-! ## The value domain -/

theorem rfValid_pair {x : Execution} (h : x.rfValid = true) {w r : Nat} (hm : (w, r) ∈ x.rf) :
    ∃ kw kr, x.kindOf w = some kw ∧ x.kindOf r = some kr ∧ readable kw kr = true := by
  simp only [Execution.rfValid, Bool.and_eq_true, List.all_eq_true] at h
  have hp := h.1 (w, r) hm
  cases hkw : x.kindOf w with
  | none => simp [hkw] at hp
  | some kw =>
    cases hkr : x.kindOf r with
    | none => simp [hkw, hkr] at hp
    | some kr =>
      refine ⟨kw, kr, rfl, rfl, ?_⟩
      simp only [hkw, hkr] at hp
      unfold readable
      cases hw : kw.write? <;> cases hr : kr.read? <;> simp_all

theorem readable_spec {kw kr : Kind} (h : readable kw kr = true) {l v : Nat} {o : Order}
    (hr : kr.read? = some (l, v, o)) : ∃ o', kw.write? = some (l, v, o') := by
  unfold readable at h
  rw [hr] at h
  cases hw : kw.write? with
  | none => simp [hw] at h
  | some t =>
    obtain ⟨wl, wv, wo⟩ := t
    simp only [hw, Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨wo, rfl⟩

theorem rfValid_source {x : Execution} (h : x.rfValid = true) {e : Event} (he : e ∈ x.events)
    {l v : Nat} {o : Order} (hr : e.kind.read? = some (l, v, o)) : ∃ w, (w, e.id) ∈ x.rf := by
  simp only [Execution.rfValid, Bool.and_eq_true, List.all_eq_true] at h
  have hone := h.2 e he
  simp only [hr, Option.isNone_some, Bool.false_or, beq_iff_eq] at hone
  have hpos : 0 < (x.rf.filter (·.2 == e.id)).length := by omega
  obtain ⟨⟨w, r⟩, hmem⟩ := List.exists_mem_of_length_pos hpos
  obtain ⟨hmem, hrid⟩ := List.mem_filter.mp hmem
  simp only [beq_iff_eq] at hrid
  subst hrid
  exact ⟨w, hmem⟩

/-- **Value-domain soundness.** In a candidate execution whose reads-from
passes validation, every read returns a literal of its location: the
initial value, or a literal some instruction writes there. -/
theorem read_values_are_literals {F : Frag} {ps : List (List Kind)} (hruns : AllRun F.threads ps)
    {srcs : List Nat} {mos : List (List Nat)} {sc : List Nat}
    (hrf : (encode (assemble F ps) srcs mos sc).rfValid = true)
    {e : Event} (he : e ∈ assemble F ps) {l v : Nat} {o : Order}
    (hr : e.kind.read? = some (l, v, o)) : v ∈ F.literals l := by
  obtain ⟨x, hxe⟩ : ∃ x, x = encode (assemble F ps) srcs mos sc := ⟨_, rfl⟩
  rw [← hxe] at hrf
  have hev : x.events = assemble F ps := by rw [hxe]; rfl
  have he' : e ∈ x.events := by rw [hev]; exact he
  obtain ⟨w, hmem⟩ := rfValid_source hrf he' hr
  obtain ⟨kw, kr, hkw, hkr, hread⟩ := rfValid_pair hrf hmem
  rw [kindOf_event hev he'] at hkr
  cases hkr
  obtain ⟨o', hw⟩ := readable_spec hread hr
  obtain ⟨e', he'', -, rfl⟩ := kindOf_mem hkw
  exact assemble_write_literal hruns (by rw [← hev]; exact he'') hw

/-! ## Completeness of the candidates -/

theorem allIn_rfSources {F : Frag} {ps : List (List Kind)} {srcs : List Nat} {mos : List (List Nat)}
    {sc : List Nat} (hlen : srcs.length = (readEvents (assemble F ps)).length)
    (hrf : (encode (assemble F ps) srcs mos sc).rfValid = true) :
    AllIn srcs (rfSources (assemble F ps)) := by
  obtain ⟨x, hxe⟩ : ∃ x, x = encode (assemble F ps) srcs mos sc := ⟨_, rfl⟩
  rw [← hxe] at hrf
  have hev : x.events = assemble F ps := by rw [hxe]; rfl
  have hrfx : x.rf = srcs.zip ((readEvents (assemble F ps)).map (·.id)) := by rw [hxe]; rfl
  have key : ∀ (rs : List Event) (ss : List Nat), ss.length = rs.length →
      (∀ r ∈ rs, r ∈ assemble F ps) →
      (∀ p ∈ ss.zip (rs.map (·.id)), p ∈ x.rf) →
      AllIn ss (rs.map fun r => ((assemble F ps).filter fun w => readable w.kind r.kind).map (·.id)) := by
    intro rs
    induction rs with
    | nil => intro ss hl _ _; cases ss <;> simp_all [AllIn]
    | cons r rs ih =>
      intro ss hl hin hsub
      cases ss with
      | nil => simp at hl
      | cons s ss =>
        simp only [List.map_cons, AllIn]
        refine ⟨?_, ih ss (by simpa using hl) (fun r' h => hin r' (List.mem_cons_of_mem _ h))
          (fun p h => hsub p (by
            rw [List.map_cons, List.zip_cons_cons]; exact List.mem_cons_of_mem _ h))⟩
        obtain ⟨kw, kr, hkw, hkr, hread⟩ :=
          rfValid_pair hrf (hsub (s, r.id) (by
            rw [List.map_cons, List.zip_cons_cons]; exact List.mem_cons_self ..))
        rw [kindOf_event hev (by rw [hev]; exact hin r (List.mem_cons_self ..))] at hkr
        cases hkr
        obtain ⟨e', he', hid, rfl⟩ := kindOf_mem hkw
        simp only [List.mem_map, List.mem_filter]
        exact ⟨e', ⟨by rw [← hev]; exact he', hread⟩, hid⟩
  exact key _ _ hlen (fun r h => (List.mem_filter.mp h).1) (fun p h => by rw [hrfx]; exact h)

theorem allIn_paths {F : Frag} {ps : List (List Kind)} (hruns : AllRun F.threads ps)
    (hdom : ∀ p ∈ ps, InDom F.domain p) : AllIn ps (F.threads.map (paths F.domain [])) := by
  have key : ∀ (ts : List (List GInstr)) (qs : List (List Kind)), AllRun ts qs →
      (∀ p ∈ qs, InDom F.domain p) → AllIn qs (ts.map (paths F.domain [])) := by
    intro ts
    induction ts with
    | nil => intro qs h _; cases qs <;> simp_all [AllRun, AllIn]
    | cons t ts ih =>
      intro qs h hd
      cases qs with
      | nil => exact h.elim
      | cons q qs =>
        obtain ⟨⟨cs, hrun⟩, hrest⟩ := h
        exact ⟨paths_complete F.domain t [] cs q hrun (hd q (List.mem_cons_self ..)),
          ih qs hrest (fun p hp => hd p (List.mem_cons_of_mem _ hp))⟩
  exact key _ _ hruns hdom

/-- **Enumeration completeness.** Every candidate execution of the fragment
(any path choices, reads-from sources, modification orders and S order)
that passes reads-from validation is generated. -/
theorem encode_mem_candidates {F : Frag} {ps : List (List Kind)} (hruns : AllRun F.threads ps)
    {srcs : List Nat} (hlen : srcs.length = (readEvents (assemble F ps)).length)
    {mos : List (List Nat)} (hmos : AllPerm mos (moLists F (assemble F ps)))
    {sc : List Nat} (hsc : sc.Perm (scMembers (assemble F ps)))
    (hrf : (encode (assemble F ps) srcs mos sc).rfValid = true) :
    encode (assemble F ps) srcs mos sc ∈ candidates F := by
  have hdom : ∀ p ∈ ps, InDom F.domain p := by
    intro p hp k hk l v o hr
    obtain ⟨e, he, rfl⟩ := assemble_kind_mem (F := F) hp hk
    exact mem_nub.mpr (read_values_are_literals hruns hrf he hr)
  simp only [candidates, List.mem_flatMap, List.mem_map]
  exact ⟨ps, mem_choices_iff.mpr (allIn_paths hruns hdom),
    srcs, mem_choices_iff.mpr (allIn_rfSources hlen hrf),
    mos, mem_choices_iff.mpr (allIn_perms hmos), sc, perm_mem_perms hsc, rfl⟩

/-- **Generator soundness.** Every candidate is the encoding of a candidate
execution of the fragment: each thread's path is a run of it. -/
theorem candidate_is_execution {F : Frag} {x : Execution} (hx : x ∈ candidates F) :
    ∃ ps srcs mos sc, AllRun F.threads ps ∧ x = encode (assemble F ps) srcs mos sc := by
  simp only [candidates, List.mem_flatMap, List.mem_map] at hx
  obtain ⟨ps, hps, srcs, -, mos, -, sc, -, rfl⟩ := hx
  refine ⟨ps, srcs, mos, sc, ?_, rfl⟩
  have key : ∀ (ts : List (List GInstr)) (qs : List (List Kind)),
      AllIn qs (ts.map (paths F.domain [])) → AllRun ts qs := by
    intro ts
    induction ts with
    | nil => intro qs h; cases qs <;> simp_all [AllRun, AllIn]
    | cons t ts ih =>
      intro qs h
      cases qs with
      | nil => exact h.elim
      | cons q qs =>
        exact ⟨paths_sound F.domain t [] q h.1, ih qs h.2⟩
  exact key _ _ (mem_choices_iff.mp hps)

/-! ## The program-level verdict -/

structure Bounds where
  threads : Nat
  events : Nat
  values : Nat
  candidates : Nat
deriving DecidableEq, Repr

inductive Reason where
  | loop | malformed | threadBound | eventBound | valueBound | candidateBound
  | noConsistentExecution
deriving DecidableEq, Repr

inductive Verdict where
  | raceFree | race | unknown (reason : Reason)
deriving DecidableEq, Repr

def Frag.hasLoop (F : Frag) : Bool := F.threads.any fun t => t.any fun gi => gi.instr == .loop

def Frag.atomicLocs (F : Frag) : List Nat := F.init.map (·.1)

def Instr.atomicLoc? : Instr → Option Nat
  | .load l _ _ | .store l _ _ | .xchg l _ _ _ | .cas l _ _ _ _ _ _ => some l
  | _ => none

def Instr.plainLoc? : Instr → Option Nat
  | .naRead l | .naWrite l => some l
  | _ => none

/-- Each atomic location is declared once; every atomic access is to a
declared location and every non-atomic access to an undeclared one (the
checker races only non-atomic accesses against each other, so a mixed
location is refused rather than half-checked). -/
def Frag.wellFormed (F : Frag) : Bool :=
  (nub F.atomicLocs).length == F.atomicLocs.length &&
    F.threads.all fun t => t.all fun gi =>
      (gi.instr.atomicLoc?.all (F.atomicLocs.contains ·)) &&
        (gi.instr.plainLoc?.all fun l => !F.atomicLocs.contains l)

/-- The static event bound: the prelude and at most one event per
instruction. -/
def Frag.eventBound (F : Frag) : Nat :=
  F.init.length + F.threads.length + (F.threads.map List.length).sum

def verdict (F : Frag) (b : Bounds) : Verdict :=
  if F.hasLoop then .unknown .loop
  else if !F.wellFormed then .unknown .malformed
  else if b.threads < F.threads.length then .unknown .threadBound
  else if b.events < F.eventBound then .unknown .eventBound
  else if F.atomicLocs.any (fun l => b.values < (F.domain l).length) then .unknown .valueBound
  else if b.candidates < (candidates F).length then .unknown .candidateBound
  else if (candidates F).any (fun x => consistent x && !x.raceFree) then .race
  else if (candidates F).any consistent then .raceFree
  else .unknown .noConsistentExecution

/-- Past the refusals, the verdict is decided by the candidates. -/
theorem verdict_decided {F : Frag} {b : Bounds} {v : Verdict} (h : verdict F b = v)
    (hv : v = .raceFree ∨ v = .race) :
    F.hasLoop = false ∧ F.wellFormed = true ∧ F.threads.length ≤ b.threads ∧
      F.eventBound ≤ b.events ∧ (∀ l ∈ F.atomicLocs, (F.domain l).length ≤ b.values) ∧
      (candidates F).length ≤ b.candidates ∧
      v = (if (candidates F).any (fun x => consistent x && !x.raceFree) then .race
        else if (candidates F).any consistent then .raceFree
        else .unknown .noConsistentExecution) := by
  have nu : ∀ r, v ≠ .unknown r := by
    intro r hr; subst hr; rcases hv with hv | hv <;> cases hv
  unfold verdict at h
  have c1 : ¬ (F.hasLoop = true) := fun hc => by simp only [hc, ↓reduceIte] at h; exact nu _ h.symm
  simp only [c1] at h
  have c2 : ¬ ((!F.wellFormed) = true) := fun hc => by simp only [hc, ↓reduceIte] at h; exact nu _ h.symm
  simp only [c2] at h
  have c3 : ¬ (b.threads < F.threads.length) := fun hc => by
    simp only [hc, ↓reduceIte] at h; exact nu _ h.symm
  simp only [c3, ↓reduceIte] at h
  have c4 : ¬ (b.events < F.eventBound) := fun hc => by simp only [hc, ↓reduceIte] at h; exact nu _ h.symm
  simp only [c4, ↓reduceIte] at h
  have c5 : ¬ ((F.atomicLocs.any fun l => b.values < (F.domain l).length) = true) := fun hc => by
    simp only [hc, ↓reduceIte] at h; exact nu _ h.symm
  simp only [c5] at h
  have c6 : ¬ (b.candidates < (candidates F).length) := fun hc => by
    simp only [hc, ↓reduceIte] at h; exact nu _ h.symm
  simp only [c6, ↓reduceIte] at h
  refine ⟨by simpa using c1, by simpa using c2, by omega, by omega, ?_, by omega, h.symm⟩
  intro l hl
  exact (by simpa using c5 : ∀ x ∈ F.atomicLocs, (F.domain x).length ≤ b.values) l hl

theorem verdict_race_free_all {F : Frag} {b : Bounds} (h : verdict F b = .raceFree) :
    ∀ x ∈ candidates F, consistent x = true → x.raceFree = true := by
  intro x hx hc
  obtain ⟨-, -, -, -, -, -, hv⟩ := verdict_decided h (.inl rfl)
  by_cases hany : (candidates F).any (fun x => consistent x && !x.raceFree) = true
  · simp only [hany, ↓reduceIte] at hv; cases hv
  · exact (by simpa using hany :
      ∀ x ∈ candidates F, consistent x = true → x.raceFree = true) x hx hc

/-- **Verdict soundness.** `raceFree` means: every candidate execution of the
fragment, for any read values, outcomes, reads-from sources, modification
orders and S order, that is consistent is race-free. -/
theorem verdict_race_free_sound {F : Frag} {b : Bounds} (h : verdict F b = .raceFree)
    {ps : List (List Kind)} (hruns : AllRun F.threads ps)
    {srcs : List Nat} (hlen : srcs.length = (readEvents (assemble F ps)).length)
    {mos : List (List Nat)} (hmos : AllPerm mos (moLists F (assemble F ps)))
    {sc : List Nat} (hsc : sc.Perm (scMembers (assemble F ps)))
    (hcons : consistent (encode (assemble F ps) srcs mos sc) = true) :
    (encode (assemble F ps) srcs mos sc).raceFree = true := by
  have hrf : (encode (assemble F ps) srcs mos sc).rfValid = true := by
    simp only [consistent, Bool.and_eq_true] at hcons; exact hcons.1.1.1.1.1.1.1.2
  exact verdict_race_free_all h _ (encode_mem_candidates hruns hlen hmos hsc hrf) hcons

/-- The same, as acceptance by the full checker. -/
theorem verdict_race_free_accepts {F : Frag} {b : Bounds} (h : verdict F b = .raceFree)
    {ps : List (List Kind)} (hruns : AllRun F.threads ps)
    {srcs : List Nat} (hlen : srcs.length = (readEvents (assemble F ps)).length)
    {mos : List (List Nat)} (hmos : AllPerm mos (moLists F (assemble F ps)))
    {sc : List Nat} (hsc : sc.Perm (scMembers (assemble F ps)))
    (hcons : consistent (encode (assemble F ps) srcs mos sc) = true) :
    check (encode (assemble F ps) srcs mos sc) = [] :=
  (check_nil_iff _).mpr ⟨hcons, verdict_race_free_sound h hruns hlen hmos hsc hcons⟩

/-- `race` is witnessed by a consistent candidate that races. -/
theorem verdict_race_witnessed {F : Frag} {b : Bounds} (h : verdict F b = .race) :
    ∃ x ∈ candidates F, consistent x = true ∧ x.raceFree = false := by
  obtain ⟨-, -, -, -, -, -, hv⟩ := verdict_decided h (.inr rfl)
  by_cases hany : (candidates F).any (fun x => consistent x && !x.raceFree) = true
  · obtain ⟨x, hx, hp⟩ := List.any_eq_true.mp hany
    simp only [Bool.and_eq_true, Bool.not_eq_true'] at hp
    exact ⟨x, hx, hp.1, hp.2⟩
  · simp only [hany] at hv
    by_cases hc2 : (candidates F).any consistent = true <;> simp [hc2] at hv

theorem loop_is_unknown {F : Frag} (b : Bounds) (h : F.hasLoop = true) :
    verdict F b = .unknown .loop := by
  simp [verdict, h]

/-- A `raceFree` verdict is reached only within every bound, for a
loop-free, well-formed fragment. -/
theorem race_free_needs_bounds {F : Frag} {b : Bounds} (h : verdict F b = .raceFree) :
    F.hasLoop = false ∧ F.wellFormed = true ∧ F.threads.length ≤ b.threads ∧
      F.eventBound ≤ b.events ∧ (∀ l ∈ F.atomicLocs, (F.domain l).length ≤ b.values) ∧
      (candidates F).length ≤ b.candidates := by
  obtain ⟨h1, h2, h3, h4, h5, h6, -⟩ := verdict_decided h (.inl rfl)
  exact ⟨h1, h2, h3, h4, h5, h6⟩

/-! ## Shared fragment vectors

Location 1 and 2 are atomic, 9 is non-atomic data; register 0 holds the
value read. Each `def` below is parsed by the Rust binding test
(`crates/qcore-core/tests/unit/execution_coverage.rs`), checked by
`check_program_executions` under the listed bounds, and its verdict and
candidate count compared with `coverageVectors`. -/

/-- MP: a release store publishes, an acquire load guards the payload read. -/
def mpReleaseAcquire : Frag :=
  { init := [(1, 0)]
    threads := [
      [g [] (.naWrite 9), g [] (.store 1 1 .release)],
      [g [] (.load 1 .acquire 0), g [(0, 1)] (.naRead 9)]] }

/-- MP, relaxed: the guarded read races with the payload write. -/
def mpRelaxed : Frag :=
  { init := [(1, 0)]
    threads := [
      [g [] (.naWrite 9), g [] (.store 1 1 .relaxed)],
      [g [] (.load 1 .relaxed 0), g [(0, 1)] (.naRead 9)]] }

/-- The hazard-pointer handshake (store buffering): the reader publishes its
hazard (location 2) and re-reads the source (location 1, 1 = still linked)
before its access; the reclaimer unlinks and reads the hazard before it
frees. Relaxed: both may miss each other, so the free races with the read. -/
def sbRelaxed : Frag :=
  { init := [(1, 1), (2, 0)]
    threads := [
      [g [] (.store 2 1 .relaxed), g [] (.load 1 .relaxed 0), g [(0, 1)] (.naRead 9)],
      [g [] (.store 1 0 .relaxed), g [] (.load 2 .relaxed 0), g [(0, 0)] (.naWrite 9)]] }

/-- The same with a seq_cst fence between each store and load: race-free. -/
def sbFences : Frag :=
  { init := [(1, 1), (2, 0)]
    threads := [
      [g [] (.store 2 1 .relaxed), g [] (.fence .seqCst false), g [] (.load 1 .relaxed 0),
        g [(0, 1)] (.naRead 9)],
      [g [] (.store 1 0 .relaxed), g [] (.fence .seqCst false), g [] (.load 2 .relaxed 0),
        g [(0, 0)] (.naWrite 9)]] }

/-- Publication through a weak compare-exchange: the payload read is guarded
by the acquire load, the exchange may fail spuriously. -/
def casPublication : Frag :=
  { init := [(1, 0)]
    threads := [
      [g [] (.naWrite 9), g [] (.cas 1 0 1 .release .relaxed true 0)],
      [g [] (.load 1 .acquire 0), g [(0, 1)] (.naRead 9)]] }

/-- An exchange hands the flag over: the consumer's exchange reads 1 only
from the release exchange. -/
def xchgHandoff : Frag :=
  { init := [(1, 0)]
    threads := [
      [g [] (.naWrite 9), g [] (.xchg 1 1 .release 0)],
      [g [] (.xchg 1 2 .acquire 0), g [(0, 1)] (.naRead 9)]] }

/-- A loop is not enumerated. -/
def withLoop : Frag :=
  { init := [(1, 0)]
    threads := [[g [] (.load 1 .acquire 0), g [] .loop]] }

/-- An atomic location also accessed non-atomically. -/
def mixedLocation : Frag :=
  { init := [(1, 0)]
    threads := [[g [] (.store 1 1 .relaxed)], [g [] (.naRead 1)]] }

def wide : Bounds := { threads := 4, events := 16, values := 4, candidates := 64 }

def coverageVectors : List (String × Frag × Bounds × Verdict × Nat) := [
  ("mpReleaseAcquire", mpReleaseAcquire, wide, .raceFree, 4),
  ("mpRelaxed", mpRelaxed, wide, .race, 4),
  ("sbRelaxed", sbRelaxed, wide, .race, 16),
  ("sbFences", sbFences, wide, .raceFree, 32),
  ("casPublication", casPublication, wide, .raceFree, 5),
  ("xchgHandoff", xchgHandoff, wide, .raceFree, 54),
  ("mpReleaseAcquireOneCandidate", mpReleaseAcquire,
    { threads := 4, events := 16, values := 4, candidates := 1 }, .unknown .candidateBound, 4),
  ("mpReleaseAcquireOneThread", mpReleaseAcquire,
    { threads := 1, events := 16, values := 4, candidates := 64 }, .unknown .threadBound, 4),
  ("mpReleaseAcquireFewEvents", mpReleaseAcquire,
    { threads := 4, events := 6, values := 4, candidates := 64 }, .unknown .eventBound, 4),
  ("mpReleaseAcquireOneValue", mpReleaseAcquire,
    { threads := 4, events := 16, values := 1, candidates := 64 }, .unknown .valueBound, 4),
  ("withLoop", withLoop, wide, .unknown .loop, 0),
  ("mixedLocation", mixedLocation, wide, .unknown .malformed, 2)
]


theorem coverage_vectors_hold :
    coverageVectors.all (fun v => verdict v.2.1 v.2.2.1 == v.2.2.2.1 &&
      (v.2.2.2.2 == (candidates v.2.1).length)) = true := by
  decide +kernel

end QCore.ExecutionCoverage
