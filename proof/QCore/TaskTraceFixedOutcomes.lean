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

import QCore.TaskTraceWrites
import QCore.ScalarTypes

/-!
# W48-QR-F02-FIXED-OUTCOMES — the F02 alphabet grown by the fixed-width outcome

Card `W48-QR-SCALAR-TYPES-R4` let a task worker return a value of any §G02t
type, and the F02 relation refused a reference run carrying one
(`UnsupportedOutcome`): its observable alphabet had no such outcome. Card
`W48-QR-SCALAR-TYPES-R6` grows the alphabet by CT03-PROOF-ALPHABET's method:
the Rust `ObservedOutcome` gains `Fixed { ty, value }`, and this module is the
statement that the growth is conservative and not vacuous.

**The relation observes an outcome only up to equality and Buffer-ness.** The
production relation (`QCore.TaskTraceWrites.relate`) reads a completion's
outcome in exactly two ways: whether it names a Buffer generation (`bufferOf`)
and whether two events are equal. So renaming outcomes by any function that
is injective and maps each outcome to one naming the same generation, or none
— a *faithful* renaming — changes no verdict
(`relate_is_invariant_under_faithful_renaming`), proved here for arbitrary
traces through every one of the relation's conjuncts.

**The extended relation.** `XOutcome` adds `fixed ty value` to the pinned
`Outcome`. The extended relation `xrelate` is the production relation over
the image of a faithful, injective encoding of the extended alphabet into the
pinned one (`encode_is_injective`, `encode_is_faithful`); by the invariance
theorem its verdicts do not depend on which such encoding is chosen
(`xrelate_does_not_depend_on_the_encoding`), which is what it means for the
Rust relation to treat `Fixed` as one more opaque, Buffer-free outcome.

**Conservativity.** A trace without a fixed outcome relates exactly as before:
`xrelate` over the embedding of pinned-alphabet traces *is* `relate` over
them (`fixed_free_traces_relate_exactly_as_before`), for arbitrary traces. No
pinned domain carries a fixed outcome, so every pinned digest is untouched.

**Non-vacuity, both directions.** A run whose child returns a fixed value is
admitted against itself, and the same run reporting another number, or the
same number at another type, is refused with the fact-mismatch class `3`.

What is not established here: that the Rust implementation is this function
(its twin, as for every F02 statement, is by inspection and the harness), or
anything about native executions, which `compiler/tests/scalar_f02_outcomes.rs`
compares relationally. -/

namespace QCore.TaskTraceFixedOutcomes

open QCore.TaskTraceRelation (Outcome sortNats firstFailure)
open QCore.TaskTraceWrites

/-! ## Faithful renamings of completion outcomes -/

/-- The generation a completion outcome names, if it is a Buffer. -/
def bufferGen : Outcome → Option Nat
  | .buffer generation => some generation
  | _ => none

/-- A renaming keeps every outcome's Buffer-ness and generation. -/
def Faithful (g : Outcome → Outcome) : Prop := ∀ o, bufferGen (g o) = bufferGen o

/-- Rename every completion's outcome by `g`; every other event is kept. -/
def rename (g : Outcome → Outcome) : Event → Event
  | .allocated task buffer payload => .allocated task buffer payload
  | .allocationFailed task => .allocationFailed task
  | .spawned child worker => .spawned child worker
  | .creationFailed worker => .creationFailed worker
  | .read task buffer value => .read task buffer value
  | .written task buffer value => .written task buffer value
  | .dropped task buffer => .dropped task buffer
  | .loaned borrower buffer => .loaned borrower buffer
  | .returned borrower buffer => .returned borrower buffer
  | .completed task outcome => .completed task (g outcome)
  | .joined child => .joined child

theorem rename_injective {g : Outcome → Outcome} (hg : Function.Injective g) :
    Function.Injective (rename g) := by
  intro a b h
  cases a <;> cases b <;> simp only [rename, Event.completed.injEq, Event.allocated.injEq,
    Event.allocationFailed.injEq, Event.spawned.injEq, Event.creationFailed.injEq,
    Event.read.injEq, Event.written.injEq, Event.dropped.injEq, Event.loaned.injEq,
    Event.returned.injEq, Event.joined.injEq, reduceCtorEq] at h ⊢
  all_goals first
    | exact h
    | exact ⟨h.1, hg h.2⟩

section Invariance

variable {g : Outcome → Outcome}

theorem bufferOf_completed (task : Nat) (o : Outcome) :
    bufferOf (.completed task o) = bufferGen o := by
  cases o <;> rfl

theorem actorOf_rename (e : Event) : actorOf (rename g e) = actorOf e := by
  cases e <;> rfl

theorem bufferOf_rename (hg : Faithful g) (e : Event) : bufferOf (rename g e) = bufferOf e := by
  cases e with
  | completed task o => rw [rename, bufferOf_completed, bufferOf_completed, hg]
  | _ => rfl

theorem subjectOf_rename (e : Event) : subjectOf (rename g e) = subjectOf e := by
  cases e <;> rfl

/-! ### The general list lemmas -/

theorem positionsFrom_map {F : Event → Event} {p : Event → Bool} (h : ∀ e, p (F e) = p e) :
    ∀ (t : List Event) (k : Nat), positionsFrom k p (t.map F) = positionsFrom k p t := by
  intro t
  induction t with
  | nil => intro k; rfl
  | cons e rest ih =>
    intro k
    simp only [List.map_cons, positionsFrom, h e, ih]

theorem positions_map {F : Event → Event} {p : Event → Bool} (h : ∀ e, p (F e) = p e)
    (t : List Event) : positions (t.map F) p = positions t p :=
  positionsFrom_map h t 0

theorem any_map_of {F : Event → Event} {p q : Event → Bool} (h : ∀ e, p (F e) = q e) :
    ∀ t : List Event, (t.map F).any p = t.any q := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih => simp [List.any_cons, h e, ih]

theorem any_congr_of {α} {p q : α → Bool} :
    ∀ {l : List α}, (∀ x ∈ l, p x = q x) → l.any p = l.any q := by
  intro l
  induction l with
  | nil => intro _; rfl
  | cons a rest ih =>
    intro h
    have ha := h a (List.mem_cons_self ..)
    have hr := ih fun x hx => h x (List.mem_cons_of_mem a hx)
    simp [List.any_cons, ha, hr]

theorem filterMap_map_of {F : Event → Event} {f : Event → Option Nat}
    (h : ∀ e, f (F e) = f e) : ∀ t : List Event, (t.map F).filterMap f = t.filterMap f := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih => simp [List.filterMap_cons, h e, ih]

theorem filter_map_of {F : Event → Event} {p : Event → Bool} (h : ∀ e, p (F e) = p e) :
    ∀ t : List Event, (t.map F).filter p = (t.filter p).map F := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih =>
    cases hq : p e with
    | false => simp [h e, hq, ih]
    | true => simp [h e, hq, ih]

/-! ### Every predicate of the relation is blind to a faithful renaming -/

theorem isAllocationOf_rename (b : Nat) (e : Event) :
    isAllocationOf b (rename g e) = isAllocationOf b e := by cases e <;> rfl
theorem isReleaseOf_rename (b : Nat) (e : Event) :
    isReleaseOf b (rename g e) = isReleaseOf b e := by cases e <;> rfl
theorem isWriteOf_rename (b : Nat) (e : Event) :
    isWriteOf b (rename g e) = isWriteOf b e := by cases e <;> rfl
theorem isCreationOf_rename (c : Nat) (e : Event) :
    isCreationOf c (rename g e) = isCreationOf c e := by cases e <;> rfl
theorem isJoinOf_rename (c : Nat) (e : Event) :
    isJoinOf c (rename g e) = isJoinOf c e := by cases e <;> rfl
theorem isCompletionOf_rename (c : Nat) (e : Event) :
    isCompletionOf c (rename g e) = isCompletionOf c e := by cases e <;> rfl
theorem isReturn_rename (e : Event) : isReturn (rename g e) = isReturn e := by cases e <;> rfl
theorem isReadBy_rename (br b : Nat) (e : Event) :
    isReadBy br b (rename g e) = isReadBy br b e := by cases e <;> rfl
theorem isLoanEventOf_rename (br b : Nat) (e : Event) :
    isLoanEventOf br b (rename g e) = isLoanEventOf br b e := by cases e <;> rfl
theorem isReturnEventOf_rename (br b : Nat) (e : Event) :
    isReturnEventOf br b (rename g e) = isReturnEventOf br b e := by cases e <;> rfl
theorem mentions_rename (hg : Faithful g) (b : Nat) (e : Event) :
    mentions b (rename g e) = mentions b e := by
  unfold mentions; rw [bufferOf_rename hg]
theorem isAbout_rename (c : Nat) (e : Event) : isAbout c (rename g e) = isAbout c e := by
  unfold isAbout; rw [subjectOf_rename]

/-- The payload rule's read predicate. -/
def staleRead (buffer payload : Nat) : Event → Bool
  | .read _ candidate value => candidate == buffer && value != payload
  | _ => false

theorem staleRead_rename (b p : Nat) (e : Event) :
    staleRead b p (rename g e) = staleRead b p e := by cases e <;> rfl

/-- The root-result rule's predicate. -/
def rootCompletion : Event → Bool
  | .completed task _ => task == 0
  | _ => false

theorem rootCompletion_rename (e : Event) :
    rootCompletion (rename g e) = rootCompletion e := by cases e <;> rfl

theorem payloadOf_rename (t : List Event) (b : Nat) :
    payloadOf (t.map (rename g)) b = payloadOf t b := by
  unfold payloadOf
  rw [filter_map_of (isAllocationOf_rename b)]
  cases t.filter (isAllocationOf b) with
  | nil => rfl
  | cons e rest => cases e <;> rfl

theorem establishedReads_rename (b : Nat) :
    ∀ (t : List Event) (current : Option Nat),
      establishedReads b current (t.map (rename g)) = establishedReads b current t := by
  intro t
  induction t with
  | nil => intro _; rfl
  | cons e rest ih =>
    intro current
    cases e <;> simp only [List.map_cons, rename, establishedReads, ih]

/-! ### The sixteen flags -/

section Flags

variable (hg : Faithful g) (t : List Event)
include hg

theorem buffers_rename :
    sortNats ((t.map (rename g)).filterMap bufferOf) = sortNats (t.filterMap bufferOf) := by
  rw [filterMap_map_of (bufferOf_rename hg)]

omit hg in
theorem children_rename :
    sortNats ((t.map (rename g)).filterMap subjectOf) = sortNats (t.filterMap subjectOf) := by
  rw [filterMap_map_of subjectOf_rename]

theorem duplicateAllocationFlag_rename :
    duplicateAllocationFlag (t.map (rename g)) = duplicateAllocationFlag t := by
  unfold duplicateAllocationFlag
  rw [buffers_rename hg]
  exact any_congr_of fun b _ => by rw [positions_map (isAllocationOf_rename b)]

theorem unallocatedUseFlag_rename :
    unallocatedUseFlag (t.map (rename g)) = unallocatedUseFlag t := by
  unfold unallocatedUseFlag
  rw [buffers_rename hg]
  exact any_congr_of fun b _ => by rw [positions_map (isAllocationOf_rename b)]

theorem allocationOrderFlag_rename :
    allocationOrderFlag (t.map (rename g)) = allocationOrderFlag t := by
  unfold allocationOrderFlag
  rw [buffers_rename hg]
  exact any_congr_of fun b _ => by
    rw [positions_map (isAllocationOf_rename b), positions_map (mentions_rename hg b)]

theorem payloadFlag_rename : payloadFlag (t.map (rename g)) = payloadFlag t := by
  unfold payloadFlag
  rw [buffers_rename hg]
  refine any_congr_of fun b _ => ?_
  rw [any_map_of (isWriteOf_rename b), establishedReads_rename, payloadOf_rename]
  by_cases written : t.any (isWriteOf b) = true
  · rw [ite_eq_left written, ite_eq_left written]
  · rw [ite_eq_right written, ite_eq_right written]
    cases payloadOf t b with
    | none => rfl
    | some p =>
      show (t.map (rename g)).any (staleRead b p) = t.any (staleRead b p)
      exact any_map_of (staleRead_rename b p) t

theorem duplicateReleaseFlag_rename :
    duplicateReleaseFlag (t.map (rename g)) = duplicateReleaseFlag t := by
  unfold duplicateReleaseFlag
  rw [buffers_rename hg]
  exact any_congr_of fun b _ => by rw [positions_map (isReleaseOf_rename b)]

theorem releaseOrderFlag_rename :
    releaseOrderFlag (t.map (rename g)) = releaseOrderFlag t := by
  unfold releaseOrderFlag
  rw [buffers_rename hg]
  exact any_congr_of fun b _ => by
    rw [positions_map (isReleaseOf_rename b), positions_map (mentions_rename hg b)]

theorem unreleasedBufferFlag_rename :
    unreleasedBufferFlag (t.map (rename g)) = unreleasedBufferFlag t := by
  unfold unreleasedBufferFlag
  rw [buffers_rename hg]
  exact any_congr_of fun b _ => by
    rw [positions_map (isAllocationOf_rename b), positions_map (isReleaseOf_rename b)]

omit hg in
theorem duplicateCreationFlag_rename :
    duplicateCreationFlag (t.map (rename g)) = duplicateCreationFlag t := by
  unfold duplicateCreationFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by rw [positions_map (isCreationOf_rename c)]

omit hg in
theorem unknownChildFlag_rename :
    unknownChildFlag (t.map (rename g)) = unknownChildFlag t := by
  unfold unknownChildFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by rw [positions_map (isCreationOf_rename c)]

omit hg in
theorem creationOrderFlag_rename :
    creationOrderFlag (t.map (rename g)) = creationOrderFlag t := by
  unfold creationOrderFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by
    rw [positions_map (isCreationOf_rename c), positions_map (isAbout_rename c)]

omit hg in
theorem duplicateJoinFlag_rename :
    duplicateJoinFlag (t.map (rename g)) = duplicateJoinFlag t := by
  unfold duplicateJoinFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by rw [positions_map (isJoinOf_rename c)]

omit hg in
theorem unjoinedChildFlag_rename :
    unjoinedChildFlag (t.map (rename g)) = unjoinedChildFlag t := by
  unfold unjoinedChildFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by
    rw [positions_map (isCreationOf_rename c), positions_map (isJoinOf_rename c)]

omit hg in
theorem completionOrderFlag_rename :
    completionOrderFlag (t.map (rename g)) = completionOrderFlag t := by
  unfold completionOrderFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by
    rw [positions_map (isJoinOf_rename c), positions_map (isCompletionOf_rename c)]

omit hg in
theorem activityAfterJoinFlag_rename :
    activityAfterJoinFlag (t.map (rename g)) = activityAfterJoinFlag t := by
  unfold activityAfterJoinFlag
  rw [children_rename]
  exact any_congr_of fun c _ => by
    rw [positions_map (isJoinOf_rename c),
      positions_map (p := fun e => isAbout c e && !isReturn e)
        (fun e => by rw [isAbout_rename, isReturn_rename])]

omit hg in
theorem loanFlag_rename : loanFlag (t.map (rename g)) = loanFlag t := by
  unfold loanFlag
  refine any_map_of (fun e => ?_) t
  cases e with
  | loaned borrower buffer =>
    simp only [rename, positions_map (isLoanEventOf_rename borrower buffer),
      positions_map (isReadBy_rename borrower buffer),
      positions_map (isReturnEventOf_rename borrower buffer),
      positions_map (isJoinOf_rename borrower)]
  | _ => rfl

omit hg in
theorem rootResultFlag_rename : rootResultFlag (t.map (rename g)) = rootResultFlag t := by
  unfold rootResultFlag
  show ((positions (t.map (rename g)) rootCompletion).length != 1
      || (positions (t.map (rename g)) rootCompletion).headD 0 + 1 != (t.map (rename g)).length)
    = ((positions t rootCompletion).length != 1
      || (positions t rootCompletion).headD 0 + 1 != t.length)
  rw [positions_map rootCompletion_rename, List.length_map]

theorem checkOrder_rename : checkOrder (t.map (rename g)) = checkOrder t := by
  unfold checkOrder orderChecks
  rw [duplicateAllocationFlag_rename hg, unallocatedUseFlag_rename hg,
    allocationOrderFlag_rename hg, payloadFlag_rename hg, duplicateReleaseFlag_rename hg,
    releaseOrderFlag_rename hg, unreleasedBufferFlag_rename hg, duplicateCreationFlag_rename,
    unknownChildFlag_rename, creationOrderFlag_rename, duplicateJoinFlag_rename,
    unjoinedChildFlag_rename, completionOrderFlag_rename, activityAfterJoinFlag_rename,
    loanFlag_rename, rootResultFlag_rename]

end Flags

/-! ### R1–R3 and the relation -/

theorem contains_map_of_injective {F : Event → Event} (hF : Function.Injective F)
    (l : List Event) (a : Event) : (l.map F).contains (F a) = l.contains a := by
  apply Bool.eq_iff_iff.mpr
  simp only [List.contains_iff_mem, List.mem_map]
  constructor
  · rintro ⟨x, hx, hxa⟩
    exact hF hxa ▸ hx
  · intro ha
    exact ⟨a, ha, rfl⟩

theorem hasDuplicate_map {F : Event → Event} (hF : Function.Injective F) :
    ∀ t : List Event, hasDuplicate (t.map F) = hasDuplicate t := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih =>
    simp only [List.map_cons, hasDuplicate, contains_map_of_injective hF, ih]

theorem map_eq_map_iff {F : Event → Event} (hF : Function.Injective F) :
    ∀ {a b : List Event}, a.map F = b.map F ↔ a = b
  | [], [] => by simp
  | [], _ :: _ => by simp
  | _ :: _, [] => by simp
  | x :: xs, y :: ys => by
    simp only [List.map_cons, List.cons.injEq]
    constructor
    · rintro ⟨hxy, hrest⟩
      exact ⟨hF hxy, (map_eq_map_iff hF).mp hrest⟩
    · rintro ⟨rfl, rfl⟩
      exact ⟨rfl, rfl⟩

theorem bne_map {F : Event → Event} (hF : Function.Injective F) (a b : List Event) :
    (a.map F != b.map F) = (a != b) := by
  have same : (a.map F == b.map F) = (a == b) := by
    apply Bool.eq_iff_iff.mpr
    simp only [beq_iff_eq]
    exact map_eq_map_iff hF
  simp only [bne, same]

theorem projection_rename (t : List Event) (actor : Nat) :
    projection (t.map (rename g)) actor = (projection t actor).map (rename g) := by
  unfold projection
  exact filter_map_of (p := fun event => actorOf event == some actor)
    (fun e => by rw [actorOf_rename]) t

/-- **The relation is invariant under a faithful, injective renaming of
completion outcomes**, for arbitrary traces: it observes an outcome only
through equality and Buffer-ness. -/
theorem relate_is_invariant_under_faithful_renaming (hg : Faithful g)
    (hinj : Function.Injective g) (reference observed : List Event) :
    relate (reference.map (rename g)) (observed.map (rename g)) = relate reference observed := by
  have hF := rename_injective hinj
  have unattr : ∀ t : List Event, unattributable (t.map (rename g)) = unattributable t :=
    fun t => any_map_of (fun e => by rw [actorOf_rename]) t
  have missing : (observed.map (rename g)).any
        (fun event => !(reference.map (rename g)).contains event)
      = observed.any (fun event => !reference.contains event) :=
    any_map_of (fun e => by rw [contains_map_of_injective hF]) observed
  have actors : sortNats ((reference.map (rename g)).filterMap actorOf)
      = sortNats (reference.filterMap actorOf) := by
    rw [filterMap_map_of actorOf_rename]
  have projections :
      (sortNats (reference.filterMap actorOf)).any (fun actor =>
        projection (observed.map (rename g)) actor != projection (reference.map (rename g)) actor)
      = (sortNats (reference.filterMap actorOf)).any (fun actor =>
        projection observed actor != projection reference actor) :=
    any_congr_of fun actor _ => by
      rw [projection_rename, projection_rename, bne_map hF]
  unfold relate
  rw [unattr, hasDuplicate_map hF, List.length_map, List.length_map, missing, actors,
    projections, checkOrder_rename hg]

end Invariance

/-! ## The extended alphabet -/

open QCore.ScalarTypes (Ty)

/-- The pinned outcomes plus the fixed-width worker outcome. -/
inductive XOutcome where
  | base (o : Outcome)
  | fixed (ty : Ty) (value : Int)
  deriving DecidableEq, Repr

/-- The production alphabet with completions carrying an extended outcome. -/
inductive XEvent where
  | allocated (task buffer payload : Nat)
  | allocationFailed (task : Nat)
  | spawned (child worker : Nat)
  | creationFailed (worker : Nat)
  | read (task buffer value : Nat)
  | written (task buffer value : Nat)
  | dropped (task buffer : Nat)
  | loaned (borrower buffer : Nat)
  | returned (borrower buffer : Nat)
  | completed (task : Nat) (outcome : XOutcome)
  | joined (child : Nat)
  deriving DecidableEq, Repr

def tyIndex : Ty → Nat
  | .bool => 0 | .i8 => 1 | .i16 => 2 | .i32 => 3 | .i64 => 4 | .isize => 5
  | .u8 => 6 | .u16 => 7 | .u32 => 8 | .u64 => 9 | .usize => 10

theorem tyIndex_lt (ty : Ty) : tyIndex ty < 11 := by cases ty <;> decide

theorem tyIndex_injective {a b : Ty} (h : tyIndex a = tyIndex b) : a = b := by
  cases a <;> cases b <;> first | rfl | (simp [tyIndex] at h)

/-- The zig-zag code of an integer. -/
def zig : Int → Nat
  | .ofNat n => 2 * n
  | .negSucc n => 2 * n + 1

theorem zig_injective {a b : Int} (h : zig a = zig b) : a = b := by
  cases a <;> cases b <;> simp only [zig] at h <;> first
    | (congr 1; omega)
    | omega

/-- The encoding into the pinned outcomes: an `i32` payload doubled, a fixed
outcome at an odd code, everything else unchanged. -/
def encodeOutcome : XOutcome → Outcome
  | .base .unit => .unit
  | .base (.i32 value) => .i32 (2 * value)
  | .base (.buffer generation) => .buffer generation
  | .fixed ty value => .i32 (2 * (11 * zig value + tyIndex ty) + 1)

theorem encodeOutcome_injective : Function.Injective encodeOutcome := by
  intro a b h
  cases a with
  | base o1 =>
    cases b with
    | base o2 =>
      cases o1 <;> cases o2 <;> simp only [encodeOutcome, reduceCtorEq, Outcome.i32.injEq,
        Outcome.buffer.injEq, XOutcome.base.injEq] at h ⊢
      all_goals omega
    | fixed tb vb =>
      cases o1 <;> simp only [encodeOutcome, reduceCtorEq, Outcome.i32.injEq] at h
      omega
  | fixed ta va =>
    cases b with
    | base o2 =>
      cases o2 <;> simp only [encodeOutcome, reduceCtorEq, Outcome.i32.injEq] at h
      omega
    | fixed tb vb =>
      simp only [encodeOutcome, Outcome.i32.injEq] at h
      have := tyIndex_lt ta
      have := tyIndex_lt tb
      have hv : zig va = zig vb := by omega
      have ht : tyIndex ta = tyIndex tb := by omega
      rw [zig_injective hv, tyIndex_injective ht]

/-- The encoding of an extended event: its completion's outcome encoded. -/
def encode : XEvent → Event
  | .allocated task buffer payload => .allocated task buffer payload
  | .allocationFailed task => .allocationFailed task
  | .spawned child worker => .spawned child worker
  | .creationFailed worker => .creationFailed worker
  | .read task buffer value => .read task buffer value
  | .written task buffer value => .written task buffer value
  | .dropped task buffer => .dropped task buffer
  | .loaned borrower buffer => .loaned borrower buffer
  | .returned borrower buffer => .returned borrower buffer
  | .completed task outcome => .completed task (encodeOutcome outcome)
  | .joined child => .joined child

/-- The extended event's Buffer generation: a fixed outcome names none. -/
def xbufferOf : XEvent → Option Nat
  | .allocated _ buffer _ | .read _ buffer _ | .written _ buffer _ | .dropped _ buffer
  | .loaned _ buffer | .returned _ buffer => some buffer
  | .completed _ (.base (.buffer generation)) => some generation
  | _ => none

/-- **The encoding is faithful**: every extended event names exactly the
Buffer generation its image does — a fixed outcome none. -/
theorem encode_is_faithful (e : XEvent) : bufferOf (encode e) = xbufferOf e := by
  cases e with
  | completed task o => rcases o with (_ | _ | _) | ⟨_, _⟩ <;> rfl
  | _ => rfl

/-- **The encoding is injective**: two distinct extended events never meet. -/
theorem encode_is_injective : Function.Injective encode := by
  intro a b h
  cases a <;> cases b <;> simp only [encode, Event.completed.injEq, Event.allocated.injEq,
    Event.allocationFailed.injEq, Event.spawned.injEq, Event.creationFailed.injEq,
    Event.read.injEq, Event.written.injEq, Event.dropped.injEq, Event.loaned.injEq,
    Event.returned.injEq, Event.joined.injEq, reduceCtorEq, XEvent.completed.injEq,
    XEvent.allocated.injEq, XEvent.allocationFailed.injEq, XEvent.spawned.injEq,
    XEvent.creationFailed.injEq, XEvent.read.injEq, XEvent.written.injEq,
    XEvent.dropped.injEq, XEvent.loaned.injEq, XEvent.returned.injEq,
    XEvent.joined.injEq] at h ⊢
  all_goals first
    | exact h
    | exact ⟨h.1, encodeOutcome_injective h.2⟩

/-- The extended production relation: the pinned relation over the encoding. -/
def xrelate (reference observed : List XEvent) : Nat :=
  relate (reference.map encode) (observed.map encode)

/-- **The choice of encoding does not matter**: post-composing it with any
faithful, injective renaming gives the same verdicts. -/
theorem xrelate_does_not_depend_on_the_encoding {g : Outcome → Outcome} (hg : Faithful g)
    (hinj : Function.Injective g) (reference observed : List XEvent) :
    relate ((reference.map encode).map (rename g)) ((observed.map encode).map (rename g))
      = xrelate reference observed :=
  relate_is_invariant_under_faithful_renaming hg hinj _ _

/-! ## Conservativity -/

/-- The pinned alphabet's embedding. -/
def embed : Event → XEvent
  | .allocated task buffer payload => .allocated task buffer payload
  | .allocationFailed task => .allocationFailed task
  | .spawned child worker => .spawned child worker
  | .creationFailed worker => .creationFailed worker
  | .read task buffer value => .read task buffer value
  | .written task buffer value => .written task buffer value
  | .dropped task buffer => .dropped task buffer
  | .loaned borrower buffer => .loaned borrower buffer
  | .returned borrower buffer => .returned borrower buffer
  | .completed task outcome => .completed task (.base outcome)
  | .joined child => .joined child

/-- The encoding on the embedded alphabet: `i32` payloads doubled. -/
def double : Outcome → Outcome
  | .unit => .unit
  | .i32 value => .i32 (2 * value)
  | .buffer generation => .buffer generation

theorem double_faithful : Faithful double := by
  intro o; cases o <;> rfl

theorem double_injective : Function.Injective double := by
  intro a b h
  cases a <;> cases b <;> simp only [double, reduceCtorEq, Outcome.i32.injEq,
    Outcome.buffer.injEq] at h ⊢
  all_goals first | rfl | omega | exact h

theorem encode_embed : encode ∘ embed = rename double := by
  funext e
  cases e with
  | completed task o => cases o <;> rfl
  | _ => rfl

/-- **A trace without a fixed outcome relates exactly as before**, for
arbitrary traces: the extended relation over the embedding of pinned-alphabet
traces is the pinned relation over them. -/
theorem fixed_free_traces_relate_exactly_as_before (reference observed : List Event) :
    xrelate (reference.map embed) (observed.map embed) = relate reference observed := by
  unfold xrelate
  rw [List.map_map, List.map_map, encode_embed]
  exact relate_is_invariant_under_faithful_renaming double_faithful double_injective _ _

/-! ## Non-vacuity, both directions -/

/-- A child returning `u8` 255 through join, and the root's result. -/
def fixedRun : List XEvent :=
  [.spawned 1 0, .completed 1 (.fixed .u8 255), .joined 1, .completed 0 (.base (.i32 0))]

theorem a_fixed_outcome_run_is_admitted : xrelate fixedRun fixedRun = 0 := by decide

theorem another_number_is_refused :
    xrelate fixedRun
      [.spawned 1 0, .completed 1 (.fixed .u8 254), .joined 1, .completed 0 (.base (.i32 0))]
      = 3 := by decide

theorem the_same_number_at_another_type_is_refused :
    xrelate fixedRun
      [.spawned 1 0, .completed 1 (.fixed .u16 255), .joined 1, .completed 0 (.base (.i32 0))]
      = 3 := by decide

theorem a_fixed_outcome_is_not_an_i32 :
    xrelate fixedRun
      [.spawned 1 0, .completed 1 (.base (.i32 255)), .joined 1, .completed 0 (.base (.i32 0))]
      = 3 := by decide

end QCore.TaskTraceFixedOutcomes
