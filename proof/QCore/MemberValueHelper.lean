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
# A stated member's value handed to a declared unit helper (gateway rule 50)

Card SCHEDULER-DESTROY-MEMBER-STATES (contract spec C04.19.6 amended; USER
DECISION 2026-11-14, "Go with 1, 2A and typed refinements for 3"). Rule 30's
body reading of a declaration with a path `M` stated `#stale` composes a
direct call, in the declaration's own frame, of a `static` function the unit
defines that has a helper declaration in force and receives `M`'s value
(directly, or through the one local whose single write copies the untouched
member): the object `M` held is taken there. The take is **established**
only where every path to every exit took it exactly once that way and no
other call, store or return carried the value; otherwise it stays a declared
assumption.

The model is one path of the declaration's own frame, read as a list of
steps, and the reading's bits (`UNTAKEN`, `TAKEN` and the copy's `UNSET` of
`member_path_reading.rs`), joined by union over paths. `St` is the path's
exact count; `abs` is the slot the Rust reading keeps; `rstep` is the Rust
transfer. The theorems say the slot is an exact abstraction of the count,
that an established exit took the object exactly once on every path, and
that every way the brief keeps refusing (an uncomposed receipt, a second
take, a copy not yet written, a helper reached in a deeper frame) never
establishes.

The last part is the custody check of decision 1: at a direct call of the
destroyer, the stale path takes the object the member names once (rule 30's
`taken`) and the release ends the binder once; when the two are one identity
the second end refuses, so the pair never yields a double release a `Safe`
could rest on.

## What this is not

A proof about Monkey's C bodies, the Clang AST reading, or the helper's own
declaration (its states stay what its own rule 30 check makes them). The
Rust correspondence is reviewed (trust boundary 28) and pinned by
`member_path_check`'s unit tests and `tests/member_value_helper.rs`.
-/

namespace QCore.MemberValueHelper

/-- One step of the declaration's own frame, as the reading sees the stated
`#stale` path's value. -/
inductive Step where
  /-- The qualified local's one write, from the untouched member. -/
  | copy
  /-- A call handing the member's value to a callee: `composed` when the
  callee is a `static` unit function with a helper declaration in force that
  releases that formal (closed empty invocations, its record, its position)
  and the call is in the declaration's own frame; `viaCopy` when the value
  is handed through the local. -/
  | hand (composed : Bool) (viaCopy : Bool)
  /-- Any other call, store or return that may carry the value. -/
  | escape
  /-- A step that carries nothing. -/
  | other
  deriving DecidableEq, Repr

/-- The exact state of one path: how many times the object was taken by a
composed helper, whether a receipt the reading does not compose spoilt it,
and whether the copy is written. -/
structure St where
  count : Nat
  spoiled : Bool
  copySet : Bool
  deriving DecidableEq, Repr

def St.start : St := ⟨0, false, false⟩

/-- Whether a hand-off is a composed take: a composed callee, and the copy
written when the value comes through it. -/
def takes (s : St) (composed viaCopy : Bool) : Bool :=
  composed && (!viaCopy || s.copySet)

def step (s : St) : Step → St
  | .copy => { s with copySet := true }
  | .hand composed viaCopy =>
      if takes s composed viaCopy then { s with count := s.count + 1 }
      else { s with spoiled := true }
  | .escape => { s with spoiled := true }
  | .other => s

def run (p : List Step) : St := p.foldl step St.start

/-- The Rust slot: (`UNTAKEN`, `TAKEN`, copy written). -/
structure Slot where
  untaken : Bool
  taken : Bool
  copySet : Bool
  deriving DecidableEq, Repr

def abs (s : St) : Slot :=
  ⟨s.count != 1 || s.spoiled, decide (1 ≤ s.count) || s.spoiled, s.copySet⟩

/-- The Rust transfer on the slot (`compose`, `escape`, `write_copy_or_escape`). -/
def rstep (s : Slot) : Step → Slot
  | .copy => { s with copySet := true }
  | .hand composed viaCopy =>
      if composed && (!viaCopy || s.copySet) then
        if s.taken then { s with untaken := true, taken := true }
        else { s with untaken := false, taken := true }
      else { s with untaken := true, taken := true }
  | .escape => { s with untaken := true, taken := true }
  | .other => s

/-- An exit establishes the take: `TAKEN` exactly. -/
def established (s : Slot) : Bool := !s.untaken && s.taken

/-- The slot abstracts the exact path state step by step. -/
theorem the_slot_abstracts_the_path (s : St) (e : Step) :
    abs (step s e) = rstep (abs s) e := by
  cases e with
  | copy => rfl
  | escape => simp [abs, step, rstep]
  | other => rfl
  | hand composed viaCopy =>
    cases composed <;> cases viaCopy <;> cases hs : s.spoiled <;> cases hc : s.copySet <;>
      rcases s with ⟨n, sp, cs⟩ <;> simp_all [abs, step, rstep, takes] <;>
      rcases n with _ | _ | n <;> simp_all

/-- An exit's slot is established exactly when the path took the object once
and nothing spoilt it. -/
theorem established_iff_taken_once (s : St) :
    established (abs s) = true ↔ s.count = 1 ∧ s.spoiled = false := by
  rcases s with ⟨n, sp, cs⟩
  cases sp <;> simp [established, abs] <;> omega

theorem run_abs (p : List Step) (s : St) :
    abs (p.foldl step s) = p.foldl rstep (abs s) := by
  induction p generalizing s with
  | nil => rfl
  | cons e p ih => simp [List.foldl, ih, the_slot_abstracts_the_path]

/-- The reading's slot over a whole path is the abstraction of its exact run. -/
theorem the_reading_is_the_abstract_run (p : List Step) :
    p.foldl rstep (abs St.start) = abs (run p) := by
  simp [run, run_abs]

theorem spoiled_stays (p : List Step) (s : St) (h : s.spoiled = true) :
    (p.foldl step s).spoiled = true := by
  induction p generalizing s with
  | nil => simpa using h
  | cons e p ih =>
    apply ih
    cases e with
    | copy => simpa [step] using h
    | escape => simp [step]
    | other => simpa [step] using h
    | hand composed viaCopy =>
      simp only [step]; split <;> simp_all

theorem count_grows (p : List Step) (s : St) : s.count ≤ (p.foldl step s).count := by
  induction p generalizing s with
  | nil => simp
  | cons e p ih =>
    refine Nat.le_trans ?_ (ih (step s e))
    cases e with
    | copy => simp [step]
    | escape => simp [step]
    | other => simp [step]
    | hand composed viaCopy => simp only [step]; split <;> simp

/-- A receipt the reading does not compose (an escape anywhere on the path)
never establishes the take. -/
theorem an_escape_never_establishes (pre post : List Step) :
    established (abs (run (pre ++ Step.escape :: post))) = false := by
  have h : (run (pre ++ Step.escape :: post)).spoiled = true := by
    simp only [run, List.foldl_append, List.foldl_cons]
    exact spoiled_stays post _ (by simp [step])
  cases hb : established (abs (run (pre ++ Step.escape :: post)))
  · rfl
  · have := (established_iff_taken_once _).mp hb
    simp_all

/-- A callee that is not composed (not `static`, not in force, not
releasing, an open or non-empty invocation set, another record or position,
or reached in a deeper frame) never establishes the take. -/
theorem an_uncomposed_callee_never_establishes (pre post : List Step) (viaCopy : Bool) :
    established (abs (run (pre ++ Step.hand false viaCopy :: post))) = false := by
  have h : (run (pre ++ Step.hand false viaCopy :: post)).spoiled = true := by
    simp only [run, List.foldl_append, List.foldl_cons]
    exact spoiled_stays post _ (by simp [step, takes])
  cases hb : established (abs (run (pre ++ Step.hand false viaCopy :: post)))
  · rfl
  · have := (established_iff_taken_once _).mp hb
    simp_all

/-- A path with no hand-off to a composed helper never establishes it. -/
theorem no_take_never_establishes (p : List Step)
    (h : ∀ e ∈ p, e = .copy ∨ e = .other) :
    established (abs (run p)) = false := by
  have hc : (run p).count = 0 := by
    suffices ∀ s : St, s.count = 0 → (p.foldl step s).count = 0 from this _ rfl
    induction p with
    | nil => intro s hs; simpa using hs
    | cons e p ih =>
      intro s hs
      have he := h e (by simp)
      have hp : ∀ e ∈ p, e = .copy ∨ e = .other := fun e' m => h e' (by simp [m])
      rcases he with rfl | rfl <;> simpa [step] using ih hp _ (by simpa [step] using hs)
  cases hb : established (abs (run p))
  · rfl
  · have := (established_iff_taken_once _).mp hb
    omega

/-- Two composed takes on one path never establish it: the object would be
released twice. -/
theorem a_second_take_never_establishes (s : St) (h : 1 ≤ s.count) (viaCopy : Bool)
    (post : List Step) :
    established (abs (post.foldl step (step s (.hand true viaCopy)))) = false := by
  cases hb : established (abs (post.foldl step (step s (.hand true viaCopy))))
  · rfl
  · have ⟨hone, hsp⟩ := (established_iff_taken_once _).mp hb
    by_cases ht : takes s true viaCopy = true
    · have grow := count_grows post (step s (.hand true viaCopy))
      have hstep : (step s (.hand true viaCopy)).count = s.count + 1 := by
        simp [step, ht]
      omega
    · have hs : (step s (.hand true viaCopy)).spoiled = true := by
        simp [step, ht]
      have := spoiled_stays post _ hs
      simp_all

/-- A hand-off through the local before its one write is no take. -/
theorem a_copy_not_yet_written_is_no_take (s : St) (h : s.copySet = false) :
    (step s (.hand true true)).spoiled = true := by
  simp [step, takes, h]

/-- The positive control: the copy, then one composed take through it. -/
theorem the_destroyers_path_establishes :
    established (abs (run [.copy, .other, .hand true true])) = true := by
  decide

/-- Paths are joined by union of the slot's bits. -/
def join (a b : Slot) : Slot :=
  ⟨a.untaken || b.untaken, a.taken || b.taken, a.copySet && b.copySet⟩

/-- A joined exit is established exactly when both paths to it are. -/
theorem a_join_is_established_iff_both_are (a b : St) :
    established (join (abs a) (abs b)) = true ↔
      established (abs a) = true ∧ established (abs b) = true := by
  rw [established_iff_taken_once, established_iff_taken_once]
  rcases a with ⟨n, sp, cs⟩
  rcases b with ⟨n', sp', cs'⟩
  cases sp <;> cases sp' <;> simp [established, join, abs] <;> omega

/-! ## The stale path beside the release (decision 1) -/

/-- The caller's custody: the identities it holds. -/
abbrev Custody := List Nat

/-- End one held identity; `none` refuses (a double release). -/
def finish (c : Custody) (x : Nat) : Option Custody :=
  if x ∈ c then some (c.erase x) else none

/-- A direct call of the destroyer: the stale path takes the object linked at
the member (`linked`), then the release ends the binder (`binder`). -/
def destroy (c : Custody) (linked binder : Nat) : Option Custody :=
  (finish c linked).bind (fun c => finish c binder)

/-- Two held identities: each ends once, and neither is held after. -/
theorem distinct_identities_each_end_once (c : Custody) (linked binder : Nat)
    (hl : linked ∈ c) (hb : binder ∈ c) (hne : linked ≠ binder) (hnd : c.Nodup) :
    ∃ c', destroy c linked binder = some c' ∧ linked ∉ c' ∧ binder ∉ c' := by
  have hb' : binder ∈ c.erase linked := (List.mem_erase_of_ne (Ne.symm hne)).mpr hb
  refine ⟨(c.erase linked).erase binder, ?_, ?_, ?_⟩
  · simp [destroy, finish, hl, hb']
  · intro hm
    have := (List.mem_erase_of_ne hne).mp hm
    exact (List.Nodup.not_mem_erase hnd) this
  · exact List.Nodup.not_mem_erase (List.Nodup.erase _ hnd)

/-- One identity read at both (the member names the binder's own object, and
the caller tracked them as one): the second end refuses, never a double
release the row could rest on. -/
theorem one_identity_refuses_the_second_end (c : Custody) (x : Nat) (hnd : c.Nodup) :
    destroy c x x = none := by
  unfold destroy finish
  by_cases h : x ∈ c
  · simp [h, List.Nodup.not_mem_erase hnd]
  · simp [h]

end QCore.MemberValueHelper
