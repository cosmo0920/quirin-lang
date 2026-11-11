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

import QCore.Ct01OwnedTransfer

/-!
# CT03-PROOF-ALPHABET — the F02 trace alphabet grown by the Buffer write

`QCore.TaskTraceRelation` transcribes the F02 relation over the pre-T05
alphabet: its `Event` deliberately has no Buffer-write constructor, and the
Rust observer refused a reference run carrying one
(`UnsupportedObservedEvent`). Card `CT03-PROOF-ALPHABET` admits the T05 write:
the Rust production alphabet (`qcore_core::task_trace_relation::ObservedEvent`)
gains `Written { task, buffer, value }`, and **this module is the
transcription of the extended production relation**. `QCore.TaskTraceRelation`
is untouched — it remains the transcription of the relation's restriction to
the write-free sublanguage, and the theorems below are what make that sentence
precise rather than assumed:

* `relate_agrees_on_the_embedded_alphabet` / `declare_agrees_on_the_embedded_alphabet`
  — for **arbitrary** traces of the old alphabet, the extended relation and the
  pinned relation return the same class, conjunct for conjunct.
* `write_free_traces_relate_exactly_as_before` — a trace of the *extended*
  alphabet without a `written` event relates exactly as before: it is the
  image of an old-alphabet trace, and the extended relation agrees with the
  pinned relation on it. This is the card's conservativity statement.
* `mixTrace_agrees_on_the_embedded_alphabet` — the digest domain grows without
  moving any existing digest: `encodeEvent` gives `written` the fresh tag `11`
  and every old event its old code, so every recorded trace digest is
  unchanged by construction *and* by theorem.
* `the_pinned_domain_one_verdicts_are_carried` — one pinned F02 verdict digest
  re-derived over the extended relation from the old theorem alone, with no
  new kernel enumeration: the pins are carried, not re-measured.
* `a_stale_read_is_refused` / `a_rewritten_payload_is_admitted` — the one rule
  the write extends (R4's payload agreement) is not vacuous in either
  direction: a read that misses a preceding write's value is refused, and a
  read that reports it is admitted.

The one semantic difference from the pinned relation, stated once: the payload
conjunct (class `8`) is split per generation on whether the generation is ever
written. A never-written generation keeps **exactly** the old rule (reads
report the allocated payload); a written generation's reads must each report
the value established by the nearest preceding allocation or write
(`establishedReads`). On write-free traces the split's second arm is
unreachable, which is precisely what the conservativity theorems check.

The Rust twin of every definition here is
`qcore_core::task_trace_relation`; the Rust harness
`crates/qcore-core/tests/task_trace_relation.rs` re-measures every pinned
domain digest against the extended production relation, and card
CT03's runner (`compiler/tests/ct03_proof_leg.rs`) recomputes this module's
digests from the artifact.

What this module does **not** establish: anything about native executions,
the correctness of the Rust implementation, or programs outside this profile.
-/

namespace QCore.TaskTraceWrites

open TaskTraceRelation (Outcome insertNat sortNats firstFailure mix
  modulus traceSeed traceMul domainSeed domainMul encodeOutcome)

/-- The pre-extension alphabet, unchanged in `QCore.TaskTraceRelation`. -/
abbrev BaseEvent := TaskTraceRelation.Event

/-! ## The extended observable alphabet

Mirrors the extended `ObservedOutcome`/`ObservedEvent`: the ten pre-existing
constructors plus `written`, the T05 Buffer write. -/

inductive Event where
  | allocated (task buffer payload : Nat)
  | allocationFailed (task : Nat)
  | spawned (child worker : Nat)
  | creationFailed (worker : Nat)
  | read (task buffer value : Nat)
  | written (task buffer value : Nat)
  | dropped (task buffer : Nat)
  | loaned (borrower buffer : Nat)
  | returned (borrower buffer : Nat)
  | completed (task : Nat) (outcome : Outcome)
  | joined (child : Nat)
  deriving DecidableEq, BEq, Repr, Inhabited

deriving instance ReflBEq, LawfulBEq for Event

/-- The old alphabet embeds constructor for constructor; `written` is the one
constructor outside the image. -/
def embed : BaseEvent → Event
  | .allocated task buffer payload => .allocated task buffer payload
  | .allocationFailed task => .allocationFailed task
  | .spawned child worker => .spawned child worker
  | .creationFailed worker => .creationFailed worker
  | .read task buffer value => .read task buffer value
  | .dropped task buffer => .dropped task buffer
  | .loaned borrower buffer => .loaned borrower buffer
  | .returned borrower buffer => .returned borrower buffer
  | .completed task outcome => .completed task outcome
  | .joined child => .joined child

/-- P2 (attribution): a write is the writing task's own state change. -/
def actorOf : Event → Option Nat
  | .allocated task _ _ => some task
  | .allocationFailed task => some task
  | .read task _ _ => some task
  | .written task _ _ => some task
  | .dropped task _ => some task
  | .completed task _ => some task
  | .spawned _ _ => some 0
  | .joined _ => some 0
  | .loaned _ _ => some 0
  | .returned _ _ => some 0
  | .creationFailed _ => none

/-- A write mentions the generation it writes. -/
def bufferOf : Event → Option Nat
  | .allocated _ buffer _ => some buffer
  | .read _ buffer _ => some buffer
  | .written _ buffer _ => some buffer
  | .dropped _ buffer => some buffer
  | .loaned _ buffer => some buffer
  | .returned _ buffer => some buffer
  | .completed _ (.buffer generation) => some generation
  | _ => none

def subjectOf : Event → Option Nat
  | .spawned child _ => some child
  | .joined child => some child
  | .loaned borrower _ => some borrower
  | .returned borrower _ => some borrower
  | .completed task _ => if task == 0 then none else some task
  | .allocated task _ _ => if task == 0 then none else some task
  | .allocationFailed task => if task == 0 then none else some task
  | .read task _ _ => if task == 0 then none else some task
  | .written task _ _ => if task == 0 then none else some task
  | .dropped task _ => if task == 0 then none else some task
  | .creationFailed _ => none

/-! ## Positions and predicates -/

def positionsFrom (index : Nat) (p : Event → Bool) : List Event → List Nat
  | [] => []
  | event :: rest =>
      if p event then index :: positionsFrom (index + 1) p rest
      else positionsFrom (index + 1) p rest

def positions (events : List Event) (p : Event → Bool) : List Nat :=
  positionsFrom 0 p events

def isAllocationOf (buffer : Nat) : Event → Bool
  | .allocated _ candidate _ => candidate == buffer
  | _ => false

def isReleaseOf (buffer : Nat) : Event → Bool
  | .dropped _ candidate => candidate == buffer
  | _ => false

def isWriteOf (buffer : Nat) : Event → Bool
  | .written _ candidate _ => candidate == buffer
  | _ => false

def isWrite : Event → Bool
  | .written _ _ _ => true
  | _ => false

def mentions (buffer : Nat) (event : Event) : Bool := bufferOf event == some buffer

def isCreationOf (child : Nat) : Event → Bool
  | .spawned candidate _ => candidate == child
  | _ => false

def isJoinOf (child : Nat) : Event → Bool
  | .joined candidate => candidate == child
  | _ => false

def isCompletionOf (child : Nat) : Event → Bool
  | .completed candidate _ => candidate == child
  | _ => false

def isAbout (child : Nat) (event : Event) : Bool := subjectOf event == some child

def isReturn : Event → Bool
  | .returned _ _ => true
  | _ => false

/-- The loan conjunct's three per-loan position predicates, named so a proof
leg can address the conjunct's own lists without restating them. -/
def isLoanEventOf (borrower buffer : Nat) : Event → Bool :=
  (· == .loaned borrower buffer)

def isReturnEventOf (borrower buffer : Nat) : Event → Bool :=
  (· == .returned borrower buffer)

def isReadBy (borrower buffer : Nat) : Event → Bool
  | .read task candidate _ => task == borrower && candidate == buffer
  | _ => false

def payloadOf (events : List Event) (buffer : Nat) : Option Nat :=
  match (events.filter (isAllocationOf buffer)) with
  | .allocated _ _ payload :: _ => some payload
  | _ => none

/-- The write-aware payload walk: every read of `buffer` must report the value
established by the nearest preceding allocation or write of it. Mirrors the
Rust `established` walk in `check_order`. -/
def establishedReads (buffer : Nat) : Option Nat → List Event → Bool
  | _, [] => true
  | current, event :: rest =>
    match event with
    | .allocated _ candidate payload =>
        if candidate == buffer then establishedReads buffer (some payload) rest
        else establishedReads buffer current rest
    | .written _ candidate value =>
        if candidate == buffer then establishedReads buffer (some value) rest
        else establishedReads buffer current rest
    | .read _ candidate value =>
        if candidate == buffer then
          (current == some value) && establishedReads buffer current rest
        else establishedReads buffer current rest
    | _ => establishedReads buffer current rest

/-! ## R4–R9 as named flags

The pinned relation's `checkOrder` is a `firstFailure` over sixteen indexed
flags; here each flag is a named definition so that a later module can extract
one by name (`loanFlag_false_of_checkOrder_zero` below) without restating the
list. Only the payload flag differs from the pinned relation, and only on a
generation that is written. -/

def duplicateAllocationFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    (positions events (isAllocationOf buffer)).length > 1

def unallocatedUseFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    (positions events (isAllocationOf buffer)).isEmpty

def allocationOrderFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    match (positions events (isAllocationOf buffer)).head? with
    | some allocation =>
        (positions events (mentions buffer)).any (fun index => index < allocation)
    | none => false

/-- The one extended flag. A generation with no write keeps exactly the pinned
rule; a written generation takes the `establishedReads` walk. -/
def payloadFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    if events.any (isWriteOf buffer) then
      !establishedReads buffer none events
    else
      match payloadOf events buffer with
      | some payload => events.any fun event =>
          match event with
          | .read _ candidate value => candidate == buffer && value != payload
          | _ => false
      | none => false

def duplicateReleaseFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    (positions events (isReleaseOf buffer)).length > 1

def releaseOrderFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    match (positions events (isReleaseOf buffer)).head? with
    | some release =>
        (positions events (mentions buffer)).any (fun index => index > release)
    | none => false

def unreleasedBufferFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap bufferOf)).any fun buffer =>
    !(positions events (isAllocationOf buffer)).isEmpty
      && (positions events (isReleaseOf buffer)).isEmpty

def duplicateCreationFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    (positions events (isCreationOf child)).length > 1

def unknownChildFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    (positions events (isCreationOf child)).isEmpty

def creationOrderFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    match (positions events (isCreationOf child)).head? with
    | some creation =>
        (positions events (isAbout child)).any (fun i => i < creation)
    | none => false

def duplicateJoinFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    (positions events (isJoinOf child)).length > 1

def unjoinedChildFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    !(positions events (isCreationOf child)).isEmpty
      && (positions events (isJoinOf child)).isEmpty

def completionOrderFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    match (positions events (isJoinOf child)).head? with
    | some join =>
        (positions events (isCompletionOf child)).length != 1
          || ((positions events (isCompletionOf child)).headD 0) > join
    | none => false

def activityAfterJoinFlag (events : List Event) : Bool :=
  (sortNats (events.filterMap subjectOf)).any fun child =>
    match (positions events (isJoinOf child)).head? with
    | some join =>
        (positions events (fun e => isAbout child e && !isReturn e)).any
          (fun index => index > join)
    | none => false

def loanFlag (events : List Event) : Bool :=
  events.any fun event =>
    match event with
    | .loaned borrower buffer =>
        let opened := (positions events (isLoanEventOf borrower buffer)).headD 0
        let reads := positions events (isReadBy borrower buffer)
        let returns := positions events (isReturnEventOf borrower buffer)
        reads.any (fun index => index < opened)
          || (match (positions events (isJoinOf borrower)).head? with
              | some join => returns.length != 1 || (returns.headD 0) < join
              | none => false)
    | _ => false

def rootResultFlag (events : List Event) : Bool :=
  (positions events (fun event =>
    match event with | .completed task _ => task == 0 | _ => false)).length != 1
    || ((positions events (fun event =>
      match event with | .completed task _ => task == 0 | _ => false)).headD 0) + 1
      != events.length

def orderChecks (events : List Event) : List (Nat × Bool) :=
  [ (5, duplicateAllocationFlag events)
  , (6, unallocatedUseFlag events)
  , (7, allocationOrderFlag events)
  , (8, payloadFlag events)
  , (9, duplicateReleaseFlag events)
  , (10, releaseOrderFlag events)
  , (11, unreleasedBufferFlag events)
  , (12, duplicateCreationFlag events)
  , (13, unknownChildFlag events)
  , (14, creationOrderFlag events)
  , (15, duplicateJoinFlag events)
  , (16, unjoinedChildFlag events)
  , (17, completionOrderFlag events)
  , (18, activityAfterJoinFlag events)
  , (19, loanFlag events)
  , (20, rootResultFlag events)
  ]

def checkOrder (events : List Event) : Nat :=
  firstFailure (orderChecks events)

/-! ## R1–R3 and the relation -/

def hasDuplicate : List Event → Bool
  | [] => false
  | event :: rest => rest.contains event || hasDuplicate rest

def unattributable (events : List Event) : Bool :=
  events.any fun event => (actorOf event).isNone

def projection (events : List Event) (actor : Nat) : List Event :=
  events.filter fun event => actorOf event == some actor

def declare (events : List Event) : Nat :=
  if unattributable events then 1
  else if hasDuplicate events then 2
  else checkOrder events

def relate (reference observed : List Event) : Nat :=
  if unattributable observed then 1
  else if hasDuplicate observed then 2
  else if observed.length != reference.length
      || observed.any (fun event => !reference.contains event) then 3
  else if (sortNats (reference.filterMap actorOf)).any
      (fun actor => projection observed actor != projection reference actor) then 4
  else checkOrder observed

/-! ## Digests

`written` takes the fresh code `11`; every pre-existing constructor keeps its
pinned code, so a write-free trace's digest is unchanged by construction
(`mixTrace_agrees_on_the_embedded_alphabet` states it as a theorem). -/

def encodeEvent : Event → Nat
  | .allocated task buffer payload => 1 + 16 * (task + 8 * (buffer + 64 * payload))
  | .allocationFailed task => 2 + 16 * task
  | .spawned child worker => 3 + 16 * (child + 8 * worker)
  | .creationFailed worker => 4 + 16 * worker
  | .read task buffer value => 5 + 16 * (task + 8 * (buffer + 64 * value))
  | .written task buffer value => 11 + 16 * (task + 8 * (buffer + 64 * value))
  | .dropped task buffer => 6 + 16 * (task + 8 * buffer)
  | .loaned borrower buffer => 7 + 16 * (borrower + 8 * buffer)
  | .returned borrower buffer => 8 + 16 * (borrower + 8 * buffer)
  | .completed task outcome => 9 + 16 * (task + 8 * encodeOutcome outcome)
  | .joined child => 10 + 16 * child

def mixTrace (events : List Event) : Nat :=
  events.foldl (fun accumulator event => mix accumulator (encodeEvent event)) traceSeed

def verdictDigest (reference : List Event) (traces : List (List Event)) : Nat :=
  traces.foldl (fun accumulator trace =>
    (accumulator * domainMul + relate reference trace + 1) % modulus) domainSeed

/-! ## The embedding commutes with every ingredient

Each lemma below moves one primitive across `embed`; the conservativity
theorems at the end are their composition. -/

private theorem actorOf_embed (e : BaseEvent) :
    actorOf (embed e) = TaskTraceRelation.actorOf e := by cases e <;> rfl

private theorem bufferOf_embed (e : BaseEvent) :
    bufferOf (embed e) = TaskTraceRelation.bufferOf e := by
  cases e with
  | completed task outcome => cases outcome <;> rfl
  | _ => rfl

private theorem subjectOf_embed (e : BaseEvent) :
    subjectOf (embed e) = TaskTraceRelation.subjectOf e := by cases e <;> rfl

private theorem embed_beq (a b : BaseEvent) : (embed a == embed b) = (a == b) := by
  cases a <;> cases b <;> rfl

private theorem embed_inj {a b : BaseEvent} (h : embed a = embed b) : a = b := by
  have hb : (embed a == embed b) = true := by rw [h]; simp
  rw [embed_beq] at hb
  exact eq_of_beq hb

private theorem any_congr {α} {p q : α → Bool} :
    ∀ {l : List α}, (∀ x ∈ l, p x = q x) → l.any p = l.any q := by
  intro l
  induction l with
  | nil => intro _; rfl
  | cons a rest ih =>
    intro h
    have ha := h a (List.mem_cons_self ..)
    have hr := ih fun x hx => h x (List.mem_cons_of_mem a hx)
    simp [List.any_cons, ha, hr]

private theorem any_map_embed {p : Event → Bool} {q : BaseEvent → Bool}
    (h : ∀ e, p (embed e) = q e) :
    ∀ t : List BaseEvent, (t.map embed).any p = t.any q := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih => simp [List.any_cons, h e, ih]

private theorem filterMap_map_embed {f : Event → Option Nat} {g : BaseEvent → Option Nat}
    (h : ∀ e, f (embed e) = g e) :
    ∀ t : List BaseEvent, (t.map embed).filterMap f = t.filterMap g := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih => simp [List.filterMap_cons, h e, ih]

private theorem filter_map_embed {p : Event → Bool} {q : BaseEvent → Bool}
    (h : ∀ e, p (embed e) = q e) :
    ∀ t : List BaseEvent, (t.map embed).filter p = (t.filter q).map embed := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih =>
    cases hq : q e with
    | false => simp [h e, hq, ih]
    | true => simp [h e, hq, ih]

private theorem positionsFrom_map_embed {p : Event → Bool} {q : BaseEvent → Bool}
    (h : ∀ e, p (embed e) = q e) :
    ∀ (t : List BaseEvent) (k : Nat),
      positionsFrom k p (t.map embed) = TaskTraceRelation.positionsFrom k q t := by
  intro t
  induction t with
  | nil => intro k; rfl
  | cons e rest ih =>
    intro k
    simp only [List.map_cons, positionsFrom, TaskTraceRelation.positionsFrom, h e, ih]

private theorem positions_map_embed {p : Event → Bool} {q : BaseEvent → Bool}
    (h : ∀ e, p (embed e) = q e) (t : List BaseEvent) :
    positions (t.map embed) p = TaskTraceRelation.positions t q :=
  positionsFrom_map_embed h t 0

private theorem buffers_map_embed (t : List BaseEvent) :
    sortNats ((t.map embed).filterMap bufferOf)
      = sortNats (t.filterMap TaskTraceRelation.bufferOf) := by
  rw [filterMap_map_embed bufferOf_embed]

private theorem children_map_embed (t : List BaseEvent) :
    sortNats ((t.map embed).filterMap subjectOf)
      = sortNats (t.filterMap TaskTraceRelation.subjectOf) := by
  rw [filterMap_map_embed subjectOf_embed]

private theorem isAllocationOf_embed (b : Nat) (e : BaseEvent) :
    isAllocationOf b (embed e) = TaskTraceRelation.isAllocationOf b e := by
  cases e <;> rfl

private theorem isReleaseOf_embed (b : Nat) (e : BaseEvent) :
    isReleaseOf b (embed e) = TaskTraceRelation.isReleaseOf b e := by
  cases e <;> rfl

private theorem isWriteOf_embed (b : Nat) (e : BaseEvent) :
    isWriteOf b (embed e) = false := by cases e <;> rfl

private theorem mentions_embed (b : Nat) (e : BaseEvent) :
    mentions b (embed e) = TaskTraceRelation.mentions b e := by
  unfold mentions TaskTraceRelation.mentions
  rw [bufferOf_embed]

private theorem isCreationOf_embed (c : Nat) (e : BaseEvent) :
    isCreationOf c (embed e) = TaskTraceRelation.isCreationOf c e := by
  cases e <;> rfl

private theorem isJoinOf_embed (c : Nat) (e : BaseEvent) :
    isJoinOf c (embed e) = TaskTraceRelation.isJoinOf c e := by
  cases e <;> rfl

private theorem isCompletionOf_embed (c : Nat) (e : BaseEvent) :
    isCompletionOf c (embed e) = TaskTraceRelation.isCompletionOf c e := by
  cases e <;> rfl

private theorem isAbout_embed (c : Nat) (e : BaseEvent) :
    isAbout c (embed e) = TaskTraceRelation.isAbout c e := by
  unfold isAbout TaskTraceRelation.isAbout
  rw [subjectOf_embed]

private theorem isReturn_embed (e : BaseEvent) :
    isReturn (embed e) = TaskTraceRelation.isReturn e := by
  cases e <;> rfl

private theorem payloadOf_map_embed (t : List BaseEvent) (b : Nat) :
    payloadOf (t.map embed) b = TaskTraceRelation.payloadOf t b := by
  unfold payloadOf TaskTraceRelation.payloadOf
  rw [filter_map_embed (isAllocationOf_embed b)]
  cases t.filter (TaskTraceRelation.isAllocationOf b) with
  | nil => rfl
  | cons e rest => cases e <;> rfl

/-! ### The pinned relation's sixteen flags, named

`base_checkOrder_as_flags` restates `TaskTraceRelation.checkOrder`
definitionally (`rfl` — the kernel's own zeta/beta reduction is the witness
that nothing changed), on the precedent of
`QCore.Ct37CompletionCapsule.checkOrder_as_checks`. -/

private def baseDuplicateAllocationFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    (TaskTraceRelation.positions events (TaskTraceRelation.isAllocationOf buffer)).length > 1

private def baseUnallocatedUseFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    (TaskTraceRelation.positions events (TaskTraceRelation.isAllocationOf buffer)).isEmpty

private def baseAllocationOrderFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    match (TaskTraceRelation.positions events (TaskTraceRelation.isAllocationOf buffer)).head? with
    | some allocation =>
        (TaskTraceRelation.positions events (TaskTraceRelation.mentions buffer)).any
          (fun index => index < allocation)
    | none => false

private def basePayloadFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    match TaskTraceRelation.payloadOf events buffer with
    | some payload => events.any fun event =>
        match event with
        | .read _ candidate value => candidate == buffer && value != payload
        | _ => false
    | none => false

private def baseDuplicateReleaseFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    (TaskTraceRelation.positions events (TaskTraceRelation.isReleaseOf buffer)).length > 1

private def baseReleaseOrderFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    match (TaskTraceRelation.positions events (TaskTraceRelation.isReleaseOf buffer)).head? with
    | some release =>
        (TaskTraceRelation.positions events (TaskTraceRelation.mentions buffer)).any
          (fun index => index > release)
    | none => false

private def baseUnreleasedBufferFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.bufferOf)).any fun buffer =>
    !(TaskTraceRelation.positions events (TaskTraceRelation.isAllocationOf buffer)).isEmpty
      && (TaskTraceRelation.positions events (TaskTraceRelation.isReleaseOf buffer)).isEmpty

private def baseDuplicateCreationFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    (TaskTraceRelation.positions events (TaskTraceRelation.isCreationOf child)).length > 1

private def baseUnknownChildFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    (TaskTraceRelation.positions events (TaskTraceRelation.isCreationOf child)).isEmpty

private def baseCreationOrderFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    match (TaskTraceRelation.positions events (TaskTraceRelation.isCreationOf child)).head? with
    | some creation =>
        (TaskTraceRelation.positions events (TaskTraceRelation.isAbout child)).any
          (fun i => i < creation)
    | none => false

private def baseDuplicateJoinFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    (TaskTraceRelation.positions events (TaskTraceRelation.isJoinOf child)).length > 1

private def baseUnjoinedChildFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    !(TaskTraceRelation.positions events (TaskTraceRelation.isCreationOf child)).isEmpty
      && (TaskTraceRelation.positions events (TaskTraceRelation.isJoinOf child)).isEmpty

private def baseCompletionOrderFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    match (TaskTraceRelation.positions events (TaskTraceRelation.isJoinOf child)).head? with
    | some join =>
        (TaskTraceRelation.positions events (TaskTraceRelation.isCompletionOf child)).length != 1
          || ((TaskTraceRelation.positions events (TaskTraceRelation.isCompletionOf child)).headD 0)
            > join
    | none => false

private def baseActivityAfterJoinFlag (events : List BaseEvent) : Bool :=
  (sortNats (events.filterMap TaskTraceRelation.subjectOf)).any fun child =>
    match (TaskTraceRelation.positions events (TaskTraceRelation.isJoinOf child)).head? with
    | some join =>
        (TaskTraceRelation.positions events
          (fun e => TaskTraceRelation.isAbout child e && !TaskTraceRelation.isReturn e)).any
          (fun index => index > join)
    | none => false

private def baseLoanFlag (events : List BaseEvent) : Bool :=
  events.any fun event =>
    match event with
    | .loaned borrower buffer =>
        let opened := (TaskTraceRelation.positions events
          (fun e => e == .loaned borrower buffer)).headD 0
        let reads := TaskTraceRelation.positions events (fun e =>
          match e with
          | .read task candidate _ => task == borrower && candidate == buffer
          | _ => false)
        let returns := TaskTraceRelation.positions events
          (fun e => e == .returned borrower buffer)
        reads.any (fun index => index < opened)
          || (match (TaskTraceRelation.positions events
                (TaskTraceRelation.isJoinOf borrower)).head? with
              | some join => returns.length != 1 || (returns.headD 0) < join
              | none => false)
    | _ => false

private def baseRootResultFlag (events : List BaseEvent) : Bool :=
  (TaskTraceRelation.positions events (fun event =>
    match event with | .completed task _ => task == 0 | _ => false)).length != 1
    || ((TaskTraceRelation.positions events (fun event =>
      match event with | .completed task _ => task == 0 | _ => false)).headD 0) + 1
      != events.length

private theorem base_checkOrder_as_flags (events : List BaseEvent) :
    TaskTraceRelation.checkOrder events = firstFailure
      [ (5, baseDuplicateAllocationFlag events)
      , (6, baseUnallocatedUseFlag events)
      , (7, baseAllocationOrderFlag events)
      , (8, basePayloadFlag events)
      , (9, baseDuplicateReleaseFlag events)
      , (10, baseReleaseOrderFlag events)
      , (11, baseUnreleasedBufferFlag events)
      , (12, baseDuplicateCreationFlag events)
      , (13, baseUnknownChildFlag events)
      , (14, baseCreationOrderFlag events)
      , (15, baseDuplicateJoinFlag events)
      , (16, baseUnjoinedChildFlag events)
      , (17, baseCompletionOrderFlag events)
      , (18, baseActivityAfterJoinFlag events)
      , (19, baseLoanFlag events)
      , (20, baseRootResultFlag events)
      ] := rfl

/-! ### The sixteen flag transports -/

private theorem duplicateAllocationFlag_embed (t : List BaseEvent) :
    duplicateAllocationFlag (t.map embed) = baseDuplicateAllocationFlag t := by
  unfold duplicateAllocationFlag baseDuplicateAllocationFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  rw [positions_map_embed (isAllocationOf_embed b)]

private theorem unallocatedUseFlag_embed (t : List BaseEvent) :
    unallocatedUseFlag (t.map embed) = baseUnallocatedUseFlag t := by
  unfold unallocatedUseFlag baseUnallocatedUseFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  rw [positions_map_embed (isAllocationOf_embed b)]

private theorem allocationOrderFlag_embed (t : List BaseEvent) :
    allocationOrderFlag (t.map embed) = baseAllocationOrderFlag t := by
  unfold allocationOrderFlag baseAllocationOrderFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  rw [positions_map_embed (isAllocationOf_embed b),
    positions_map_embed (mentions_embed b)]

private theorem payloadFlag_embed (t : List BaseEvent) :
    payloadFlag (t.map embed) = basePayloadFlag t := by
  unfold payloadFlag basePayloadFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  have hwrite : (t.map embed).any (isWriteOf b) = false := by
    rw [any_map_embed (isWriteOf_embed b) t]
    exact List.any_eq_false.mpr fun e _ => by simp
  rw [hwrite, ite_eq_right Bool.false_ne_true, payloadOf_map_embed]
  cases TaskTraceRelation.payloadOf t b with
  | none => rfl
  | some payload => exact any_map_embed (fun e => by cases e <;> rfl) t

private theorem duplicateReleaseFlag_embed (t : List BaseEvent) :
    duplicateReleaseFlag (t.map embed) = baseDuplicateReleaseFlag t := by
  unfold duplicateReleaseFlag baseDuplicateReleaseFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  rw [positions_map_embed (isReleaseOf_embed b)]

private theorem releaseOrderFlag_embed (t : List BaseEvent) :
    releaseOrderFlag (t.map embed) = baseReleaseOrderFlag t := by
  unfold releaseOrderFlag baseReleaseOrderFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  rw [positions_map_embed (isReleaseOf_embed b),
    positions_map_embed (mentions_embed b)]

private theorem unreleasedBufferFlag_embed (t : List BaseEvent) :
    unreleasedBufferFlag (t.map embed) = baseUnreleasedBufferFlag t := by
  unfold unreleasedBufferFlag baseUnreleasedBufferFlag
  rw [buffers_map_embed]
  refine any_congr fun b _ => ?_
  rw [positions_map_embed (isAllocationOf_embed b),
    positions_map_embed (isReleaseOf_embed b)]

private theorem duplicateCreationFlag_embed (t : List BaseEvent) :
    duplicateCreationFlag (t.map embed) = baseDuplicateCreationFlag t := by
  unfold duplicateCreationFlag baseDuplicateCreationFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isCreationOf_embed c)]

private theorem unknownChildFlag_embed (t : List BaseEvent) :
    unknownChildFlag (t.map embed) = baseUnknownChildFlag t := by
  unfold unknownChildFlag baseUnknownChildFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isCreationOf_embed c)]

private theorem creationOrderFlag_embed (t : List BaseEvent) :
    creationOrderFlag (t.map embed) = baseCreationOrderFlag t := by
  unfold creationOrderFlag baseCreationOrderFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isCreationOf_embed c),
    positions_map_embed (isAbout_embed c)]

private theorem duplicateJoinFlag_embed (t : List BaseEvent) :
    duplicateJoinFlag (t.map embed) = baseDuplicateJoinFlag t := by
  unfold duplicateJoinFlag baseDuplicateJoinFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isJoinOf_embed c)]

private theorem unjoinedChildFlag_embed (t : List BaseEvent) :
    unjoinedChildFlag (t.map embed) = baseUnjoinedChildFlag t := by
  unfold unjoinedChildFlag baseUnjoinedChildFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isCreationOf_embed c),
    positions_map_embed (isJoinOf_embed c)]

private theorem completionOrderFlag_embed (t : List BaseEvent) :
    completionOrderFlag (t.map embed) = baseCompletionOrderFlag t := by
  unfold completionOrderFlag baseCompletionOrderFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isJoinOf_embed c),
    positions_map_embed (isCompletionOf_embed c)]

private theorem activityAfterJoinFlag_embed (t : List BaseEvent) :
    activityAfterJoinFlag (t.map embed) = baseActivityAfterJoinFlag t := by
  unfold activityAfterJoinFlag baseActivityAfterJoinFlag
  rw [children_map_embed]
  refine any_congr fun c _ => ?_
  rw [positions_map_embed (isJoinOf_embed c),
    positions_map_embed
      (p := fun e => isAbout c e && !isReturn e)
      (q := fun e => TaskTraceRelation.isAbout c e && !TaskTraceRelation.isReturn e)
      (fun e => by rw [isAbout_embed, isReturn_embed])]

private theorem loanFlag_embed (t : List BaseEvent) :
    loanFlag (t.map embed) = baseLoanFlag t := by
  unfold loanFlag baseLoanFlag
  refine any_map_embed (fun e => ?_) t
  cases e with
  | loaned borrower buffer =>
    dsimp only [embed]
    rw [positions_map_embed
        (p := isLoanEventOf borrower buffer)
        (q := fun e => e == TaskTraceRelation.Event.loaned borrower buffer)
        (fun e => by cases e <;> rfl),
      positions_map_embed
        (p := isReadBy borrower buffer)
        (q := fun e => match e with
          | TaskTraceRelation.Event.read task candidate _ =>
              task == borrower && candidate == buffer
          | _ => false)
        (fun e => by cases e <;> rfl),
      positions_map_embed
        (p := isReturnEventOf borrower buffer)
        (q := fun e => e == TaskTraceRelation.Event.returned borrower buffer)
        (fun e => by cases e <;> rfl),
      positions_map_embed (isJoinOf_embed borrower)]
    rfl
  | allocated task buffer payload => rfl
  | allocationFailed task => rfl
  | spawned child worker => rfl
  | creationFailed worker => rfl
  | read task buffer value => rfl
  | dropped task buffer => rfl
  | returned borrower buffer => rfl
  | completed task outcome => rfl
  | joined child => rfl

private theorem rootResultFlag_embed (t : List BaseEvent) :
    rootResultFlag (t.map embed) = baseRootResultFlag t := by
  unfold rootResultFlag baseRootResultFlag
  rw [positions_map_embed
      (p := fun event => match event with
        | Event.completed task _ => task == 0 | _ => false)
      (q := fun event => match event with
        | TaskTraceRelation.Event.completed task _ => task == 0
        | _ => false)
      (fun e => by cases e <;> rfl),
    List.length_map]
  rfl

private theorem checkOrder_map_embed (t : List BaseEvent) :
    checkOrder (t.map embed) = TaskTraceRelation.checkOrder t := by
  rw [base_checkOrder_as_flags]
  unfold checkOrder orderChecks
  rw [duplicateAllocationFlag_embed, unallocatedUseFlag_embed,
    allocationOrderFlag_embed, payloadFlag_embed, duplicateReleaseFlag_embed,
    releaseOrderFlag_embed, unreleasedBufferFlag_embed,
    duplicateCreationFlag_embed, unknownChildFlag_embed,
    creationOrderFlag_embed, duplicateJoinFlag_embed, unjoinedChildFlag_embed,
    completionOrderFlag_embed, activityAfterJoinFlag_embed, loanFlag_embed,
    rootResultFlag_embed]

private theorem contains_map_embed (r : List BaseEvent) (x : BaseEvent) :
    (r.map embed).contains (embed x) = r.contains x := by
  cases hc : r.contains x with
  | true =>
    have hx : x ∈ r := List.contains_iff_mem.mp hc
    exact List.contains_iff_mem.mpr (List.mem_map.mpr ⟨x, hx, rfl⟩)
  | false =>
    cases hcm : (r.map embed).contains (embed x) with
    | false => rfl
    | true =>
      obtain ⟨a, ha, hae⟩ := List.mem_map.mp (List.contains_iff_mem.mp hcm)
      have hcx : r.contains x = true :=
        List.contains_iff_mem.mpr (embed_inj hae ▸ ha)
      rw [hc] at hcx
      exact absurd hcx (by decide)

private theorem hasDuplicate_map_embed :
    ∀ t : List BaseEvent, hasDuplicate (t.map embed) = TaskTraceRelation.hasDuplicate t := by
  intro t
  induction t with
  | nil => rfl
  | cons e rest ih =>
    simp only [List.map_cons, hasDuplicate, TaskTraceRelation.hasDuplicate,
      contains_map_embed, ih]

private theorem unattributable_map_embed (t : List BaseEvent) :
    unattributable (t.map embed) = TaskTraceRelation.unattributable t := by
  unfold unattributable TaskTraceRelation.unattributable
  exact any_map_embed (fun e => by rw [actorOf_embed]) t

private theorem projection_map_embed (t : List BaseEvent) (a : Nat) :
    projection (t.map embed) a = (TaskTraceRelation.projection t a).map embed := by
  unfold projection TaskTraceRelation.projection
  exact filter_map_embed (fun e => by rw [actorOf_embed]) t

private theorem map_embed_inj :
    ∀ {l1 l2 : List BaseEvent}, l1.map embed = l2.map embed → l1 = l2 := by
  intro l1
  induction l1 with
  | nil => intro l2 h; cases l2 with
    | nil => rfl
    | cons b bs => simp at h
  | cons a rest ih =>
    intro l2 h
    cases l2 with
    | nil => simp at h
    | cons b bs =>
      simp only [List.map_cons, List.cons.injEq] at h
      rw [embed_inj h.1, ih h.2]

private theorem mapEmbed_beq :
    ∀ (l1 l2 : List BaseEvent), (l1.map embed == l2.map embed) = (l1 == l2) := by
  intro l1
  induction l1 with
  | nil => intro l2; cases l2 <;> rfl
  | cons a rest ih =>
    intro l2
    cases l2 with
    | nil => rfl
    | cons b bs =>
      simp only [List.map_cons, List.cons_beq_cons, embed_beq, ih]

private theorem mapEmbed_bne (l1 l2 : List BaseEvent) :
    (l1.map embed != l2.map embed) = (l1 != l2) := by
  simp only [bne, mapEmbed_beq]

/-! ## The conservativity theorems -/

/-- For arbitrary old-alphabet traces, the extended `declare` and the pinned
`declare` return the same class. -/
theorem declare_agrees_on_the_embedded_alphabet (t : List BaseEvent) :
    declare (t.map embed) = TaskTraceRelation.declare t := by
  unfold declare TaskTraceRelation.declare
  rw [unattributable_map_embed, hasDuplicate_map_embed, checkOrder_map_embed]

/-- For arbitrary old-alphabet traces, the extended relation and the pinned
relation return the same class, conjunct for conjunct: not one admitted
rendering is gained or lost, and every refusal keeps its class. -/
theorem relate_agrees_on_the_embedded_alphabet
    (reference observed : List BaseEvent) :
    relate (reference.map embed) (observed.map embed)
      = TaskTraceRelation.relate reference observed := by
  have h3 : ((observed.map embed).length != (reference.map embed).length
      || (observed.map embed).any fun event => !(reference.map embed).contains event)
      = (observed.length != reference.length
      || observed.any fun event => !reference.contains event) := by
    rw [List.length_map, List.length_map,
      any_map_embed (fun e => by rw [contains_map_embed]) observed]
  have h4 : ((sortNats ((reference.map embed).filterMap actorOf)).any
      fun actor => projection (observed.map embed) actor
        != projection (reference.map embed) actor)
      = ((sortNats (reference.filterMap TaskTraceRelation.actorOf)).any
      fun actor => TaskTraceRelation.projection observed actor
        != TaskTraceRelation.projection reference actor) := by
    rw [filterMap_map_embed actorOf_embed]
    refine any_congr fun a _ => ?_
    rw [projection_map_embed observed a, projection_map_embed reference a,
      mapEmbed_bne]
  unfold relate TaskTraceRelation.relate
  rw [unattributable_map_embed, hasDuplicate_map_embed, h3, h4,
    checkOrder_map_embed]

private theorem encodeEvent_embed (e : BaseEvent) :
    encodeEvent (embed e) = TaskTraceRelation.encodeEvent e := by
  cases e <;> rfl

/-- The digest domain grows without moving a single existing digest: on the
embedded alphabet the extended `mixTrace` is the pinned `mixTrace`. -/
theorem mixTrace_agrees_on_the_embedded_alphabet (t : List BaseEvent) :
    mixTrace (t.map embed) = TaskTraceRelation.mixTrace t := by
  unfold mixTrace TaskTraceRelation.mixTrace
  suffices h : ∀ (s : List BaseEvent) (accumulator : Nat),
      (s.map embed).foldl (fun a e => mix a (encodeEvent e)) accumulator
        = s.foldl (fun a e => mix a (TaskTraceRelation.encodeEvent e)) accumulator from
    h t traceSeed
  intro s
  induction s with
  | nil => intro accumulator; rfl
  | cons e rest ih =>
    intro accumulator
    simp only [List.map_cons, List.foldl_cons, encodeEvent_embed, ih]

private theorem verdictDigest_map_embed (reference : List BaseEvent)
    (traces : List (List BaseEvent)) :
    verdictDigest (reference.map embed) (traces.map (·.map embed))
      = TaskTraceRelation.verdictDigest reference traces := by
  unfold verdictDigest TaskTraceRelation.verdictDigest
  suffices h : ∀ (ds : List (List BaseEvent)) (accumulator : Nat),
      (ds.map (·.map embed)).foldl
        (fun a trace => (a * domainMul + relate (reference.map embed) trace + 1) % modulus)
        accumulator
      = ds.foldl
        (fun a trace => (a * domainMul + TaskTraceRelation.relate reference trace + 1) % modulus)
        accumulator from
    h traces domainSeed
  intro ds
  induction ds with
  | nil => intro accumulator; rfl
  | cons d rest ih =>
    intro accumulator
    simp only [List.map_cons, List.foldl_cons, relate_agrees_on_the_embedded_alphabet, ih]

/-- One pinned F02 verdict digest, carried onto the extended relation from the
old theorem alone — no new kernel enumeration. Every pin of
`QCore.TaskTraceRelation` transfers the same way; this instance is recorded so
the transfer is exercised, not merely possible. -/
theorem the_pinned_domain_one_verdicts_are_carried :
    verdictDigest (TaskTraceRelation.roundTrip.map embed)
      (TaskTraceRelation.domainOne.map (·.map embed)) = 6242011289262393457 := by
  rw [verdictDigest_map_embed]
  exact TaskTraceRelation.domain_one_verdict_digest

/-! ## A write-free trace of the extended alphabet relates exactly as before -/

/-- Every write-free trace of the extended alphabet is the image of an
old-alphabet trace. -/
theorem write_free_traces_are_embedded :
    ∀ {t : List Event}, t.all (fun e => !isWrite e) = true →
      ∃ s : List BaseEvent, t = s.map embed := by
  intro t
  induction t with
  | nil => intro _; exact ⟨[], rfl⟩
  | cons e rest ih =>
    intro h
    have hparts : (!isWrite e) = true ∧ rest.all (fun e => !isWrite e) = true := by
      simpa [List.all_cons] using h
    obtain ⟨s, hs⟩ := ih hparts.2
    cases e with
    | written task buffer value => simp [isWrite] at hparts
    | allocated task buffer payload =>
        exact ⟨.allocated task buffer payload :: s, by simp [embed, hs]⟩
    | allocationFailed task => exact ⟨.allocationFailed task :: s, by simp [embed, hs]⟩
    | spawned child worker => exact ⟨.spawned child worker :: s, by simp [embed, hs]⟩
    | creationFailed worker => exact ⟨.creationFailed worker :: s, by simp [embed, hs]⟩
    | read task buffer value => exact ⟨.read task buffer value :: s, by simp [embed, hs]⟩
    | dropped task buffer => exact ⟨.dropped task buffer :: s, by simp [embed, hs]⟩
    | loaned borrower buffer => exact ⟨.loaned borrower buffer :: s, by simp [embed, hs]⟩
    | returned borrower buffer => exact ⟨.returned borrower buffer :: s, by simp [embed, hs]⟩
    | completed task outcome => exact ⟨.completed task outcome :: s, by simp [embed, hs]⟩
    | joined child => exact ⟨.joined child :: s, by simp [embed, hs]⟩

/-- **The card's conservativity statement.** A trace without `written` relates
exactly as before: a write-free reference and a write-free observation are
images of old-alphabet traces, and on them the extended relation is the pinned
relation. -/
theorem write_free_traces_relate_exactly_as_before
    {reference observed : List Event}
    (href : reference.all (fun e => !isWrite e) = true)
    (hobs : observed.all (fun e => !isWrite e) = true) :
    ∃ r o : List BaseEvent, reference = r.map embed ∧ observed = o.map embed ∧
      relate reference observed = TaskTraceRelation.relate r o := by
  obtain ⟨r, hr⟩ := write_free_traces_are_embedded href
  obtain ⟨o, ho⟩ := write_free_traces_are_embedded hobs
  exact ⟨r, o, hr, ho, by rw [hr, ho, relate_agrees_on_the_embedded_alphabet]⟩

/-! ## The extended payload rule is not vacuous

Both directions on the smallest write-carrying run: a read that reports the
value the preceding write established is admitted, and a read that misses it
is refused with the payload class. -/

theorem a_rewritten_payload_is_admitted :
    declare [.allocated 0 1 1, .written 0 1 42, .read 0 1 42, .dropped 0 1,
      .completed 0 (.i32 42)] = 0 := by decide

theorem a_stale_read_is_refused :
    declare [.allocated 0 1 1, .written 0 1 42, .read 0 1 1, .dropped 0 1,
      .completed 0 (.i32 1)] = 8 := by decide

/-! ## The positional toolkit

General facts about `positions`, counted witnesses and `firstFailure`, on
`QCore.Ct37CompletionCapsule`'s pattern but promoted deliberately: the CT03
proof leg (`QCore.Ct03ReadShares`) consumes them, and a copy per module would
have to track the extended alphabet anyway. -/

theorem positionsFrom_length {p : Event → Bool} :
    ∀ (t : List Event) (k : Nat), (positionsFrom k p t).length = t.countP p := by
  intro t
  induction t with
  | nil => intro k; rfl
  | cons a rest ih =>
    intro k
    by_cases ha : p a
    · simp [positionsFrom, ha, ih]
    · simp [positionsFrom, ha, ih]

theorem positions_length {p : Event → Bool} {t : List Event} :
    (positions t p).length = t.countP p :=
  positionsFrom_length t 0

theorem positionsFrom_spec {p : Event → Bool} :
    ∀ {t : List Event} {k i : Nat}, i ∈ positionsFrom k p t →
      k ≤ i ∧ ∃ e, t[i - k]? = some e ∧ p e = true := by
  intro t
  induction t with
  | nil => intro k i h; simp [positionsFrom] at h
  | cons a rest ih =>
    intro k i h
    by_cases ha : p a
    · simp only [positionsFrom, ha, ite_true, List.mem_cons] at h
      rcases h with rfl | h
      · exact ⟨Nat.le_refl i, a, by simp, ha⟩
      · obtain ⟨hk, e, he, hpe⟩ := ih h
        refine ⟨by omega, e, ?_, hpe⟩
        have : i - k = (i - (k + 1)) + 1 := by omega
        rw [this, List.getElem?_cons_succ]
        exact he
    · simp only [positionsFrom, ha] at h
      obtain ⟨hk, e, he, hpe⟩ := ih h
      refine ⟨by omega, e, ?_, hpe⟩
      have : i - k = (i - (k + 1)) + 1 := by omega
      rw [this, List.getElem?_cons_succ]
      exact he

theorem positions_spec {p : Event → Bool} {t : List Event} {i : Nat}
    (h : i ∈ positions t p) : ∃ e, t[i]? = some e ∧ p e = true := by
  have := positionsFrom_spec (p := p) (t := t) (k := 0) h
  simpa using this.2

/-- Completeness, the converse of `positions_spec`: an index holding a
matching event is listed. -/
theorem mem_positions_of_getElem? {p : Event → Bool} :
    ∀ {t : List Event} {i : Nat} {e : Event}, t[i]? = some e → p e = true →
      ∀ k, (k + i) ∈ positionsFrom k p t := by
  intro t
  induction t with
  | nil => intro i e hget _ _; simp at hget
  | cons a rest ih =>
    intro i e hget hp k
    cases i with
    | zero =>
      have : a = e := by simpa using hget
      subst this
      simp [positionsFrom, hp]
    | succ i' =>
      have hget' : rest[i']? = some e := by simpa using hget
      have := ih hget' hp (k + 1)
      have harith : k + (i' + 1) = (k + 1) + i' := by omega
      rw [harith]
      by_cases ha : p a
      · simp only [positionsFrom, ha, ite_true, List.mem_cons]
        exact Or.inr this
      · simpa [positionsFrom, ha] using this

theorem mem_positions {p : Event → Bool} {t : List Event} {i : Nat} {e : Event}
    (hget : t[i]? = some e) (hp : p e = true) : i ∈ positions t p := by
  have := mem_positions_of_getElem? hget hp 0
  simpa [positions] using this

theorem exists_getElem?_of_mem {α} {l : List α} {a : α} (h : a ∈ l) :
    ∃ i : Nat, l[i]? = some a := by
  induction l with
  | nil => simp at h
  | cons x rest ih =>
    rcases List.mem_cons.mp h with rfl | h
    · exact ⟨0, rfl⟩
    · obtain ⟨i, hi⟩ := ih h
      exact ⟨i + 1, by simpa using hi⟩

theorem head?_some_of_mem {α} {l : List α} {a : α} (h : a ∈ l) :
    ∃ b, l.head? = some b := by
  cases l with
  | nil => simp at h
  | cons x rest => exact ⟨x, rfl⟩

theorem mem_of_head? {α} {l : List α} {a : α} (h : l.head? = some a) : a ∈ l := by
  cases l with
  | nil => simp at h
  | cons x rest =>
    have : x = a := by simpa using h
    subst this
    exact List.mem_cons_self ..

theorem headD_eq_of_head? {α} {l : List α} {a : α} {d : α}
    (h : l.head? = some a) : l.headD d = a := by
  cases l with
  | nil => simp at h
  | cons x rest =>
    have : x = a := by simpa using h
    simpa using this

/-- A predicate counted exactly once names one event: any two matching members
are equal. -/
theorem eq_of_countP_eq_one {p : Event → Bool} {t : List Event}
    (hcount : t.countP p = 1) {a b : Event}
    (ha : a ∈ t) (hpa : p a = true) (hb : b ∈ t) (hpb : p b = true) : a = b := by
  have hlen : (t.filter p).length = 1 := by
    rw [← List.countP_eq_length_filter]; exact hcount
  obtain ⟨x, hx⟩ := List.length_eq_one_iff.mp hlen
  have hfa : a ∈ t.filter p := List.mem_filter.mpr ⟨ha, hpa⟩
  have hfb : b ∈ t.filter p := List.mem_filter.mpr ⟨hb, hpb⟩
  rw [hx, List.mem_singleton] at hfa hfb
  rw [hfa, hfb]

theorem sublist_pair_of_getElem? {a b : Event} :
    ∀ {t : List Event} {i j : Nat}, i < j → t[i]? = some a → t[j]? = some b →
      [a, b].Sublist t := by
  intro t
  induction t with
  | nil => intro i j _ ha _; simp at ha
  | cons x rest ih =>
    intro i j hij ha hb
    cases i with
    | zero =>
      have hax : a = x := by simpa using ha.symm
      subst hax
      cases j with
      | zero => omega
      | succ j' =>
        have hbrest : rest[j']? = some b := by simpa using hb
        have hbmem : b ∈ rest := List.mem_of_getElem? hbrest
        exact List.Sublist.cons_cons a (List.singleton_sublist.mpr hbmem)
    | succ i' =>
      cases j with
      | zero => omega
      | succ j' =>
        have ha' : rest[i']? = some a := by simpa using ha
        have hb' : rest[j']? = some b := by simpa using hb
        exact List.Sublist.cons x (ih (by omega) ha' hb')

theorem sublist_triple_of_getElem? {a b c : Event} :
    ∀ {t : List Event} {i j k : Nat}, i < j → j < k →
      t[i]? = some a → t[j]? = some b → t[k]? = some c →
      [a, b, c].Sublist t := by
  intro t
  induction t with
  | nil => intro i j k _ _ ha _ _; simp at ha
  | cons x rest ih =>
    intro i j k hij hjk ha hb hc
    cases i with
    | zero =>
      have hax : a = x := by simpa using ha.symm
      subst hax
      cases j with
      | zero => omega
      | succ j' =>
        cases k with
        | zero => omega
        | succ k' =>
          have hb' : rest[j']? = some b := by simpa using hb
          have hc' : rest[k']? = some c := by simpa using hc
          exact List.Sublist.cons_cons a (sublist_pair_of_getElem? (by omega) hb' hc')
    | succ i' =>
      cases j with
      | zero => omega
      | succ j' =>
        cases k with
        | zero => omega
        | succ k' =>
          have ha' : rest[i']? = some a := by simpa using ha
          have hb' : rest[j']? = some b := by simpa using hb
          have hc' : rest[k']? = some c := by simpa using hc
          exact List.Sublist.cons x (ih (by omega) (by omega) ha' hb' hc')

theorem countP_congr_on {p q : Event → Bool} :
    ∀ {t : List Event}, (∀ e ∈ t, p e = q e) → t.countP p = t.countP q := by
  intro t
  induction t with
  | nil => intro _; rfl
  | cons a rest ih =>
    intro h
    have ha := h a (List.mem_cons_self ..)
    have hr := ih fun e he => h e (List.mem_cons_of_mem a he)
    simp [List.countP_cons, ha, hr]

theorem countP_beq_le_one_of_no_duplicates :
    ∀ {t : List Event}, hasDuplicate t = false →
      ∀ x : Event, t.countP (· == x) ≤ 1 := by
  intro t
  induction t with
  | nil => intro _ x; simp
  | cons a rest ih =>
    intro h x
    have hparts : a ∉ rest ∧ hasDuplicate rest = false := by
      simpa [hasDuplicate, Bool.or_eq_false_iff] using h
    by_cases hax : a = x
    · subst hax
      have hz : rest.countP (· == a) = 0 := by
        rw [List.countP_eq_zero]
        intro e he heq
        exact hparts.1 (eq_of_beq heq ▸ he)
      simp [hz]
    · have hrest := ih hparts.2 x
      have hne : (a == x) = false := by
        simpa using hax
      simp [hne]
      omega

/-- The unique-witness counting lemma, parametric in the reference: a
predicate that holds exactly of one named event on the reference's events
counts to exactly one on any duplicate-free trace drawn from the reference
that contains that event. -/
theorem countP_eq_one_of_unique_witness {reference : List Event}
    {p : Event → Bool} {x : Event} {t : List Event}
    (hax : ∀ e ∈ reference, p e = (e == x))
    (hsub : ∀ e ∈ t, e ∈ reference)
    (hdup : hasDuplicate t = false)
    (hmem : x ∈ t) : t.countP p = 1 := by
  have hagree : t.countP p = t.countP (· == x) :=
    countP_congr_on fun e he => hax e (hsub e he)
  have hle := countP_beq_le_one_of_no_duplicates hdup x
  have hge : t.countP (· == x) ≠ 0 := fun hzero =>
    (List.countP_eq_zero.mp hzero x hmem) (by simp)
  omega

theorem mem_insertNat_of {a b : Nat} :
    ∀ {l : List Nat}, b = a ∨ b ∈ l → b ∈ insertNat a l := by
  intro l
  induction l with
  | nil =>
    intro h
    rcases h with rfl | h
    · simp [TaskTraceRelation.insertNat]
    · simp at h
  | cons x rest ih =>
    intro h
    simp only [TaskTraceRelation.insertNat]
    split
    · next hax =>
      have : a = x := by simpa using hax
      subst this
      rcases h with rfl | h
      · exact List.mem_cons_self ..
      · exact h
    · split
      · next =>
        rcases h with rfl | h
        · exact List.mem_cons_self ..
        · exact List.mem_cons_of_mem a h
      · next =>
        rcases h with rfl | h
        · exact List.mem_cons_of_mem x (ih (Or.inl rfl))
        · rcases List.mem_cons.mp h with rfl | h
          · exact List.mem_cons_self ..
          · exact List.mem_cons_of_mem x (ih (Or.inr h))

theorem mem_foldl_insertNat {a : Nat} :
    ∀ {l : List Nat} {acc : List Nat}, a ∈ acc ∨ a ∈ l →
      a ∈ l.foldl (fun sorted value => insertNat value sorted) acc := by
  intro l
  induction l with
  | nil =>
    intro acc h
    rcases h with h | h
    · exact h
    · simp at h
  | cons v rest ih =>
    intro acc h
    apply ih
    rcases h with h | h
    · exact Or.inl (mem_insertNat_of (Or.inr h))
    · rcases List.mem_cons.mp h with rfl | h
      · exact Or.inl (mem_insertNat_of (Or.inl rfl))
      · exact Or.inr h

theorem mem_sortNats {l : List Nat} {a : Nat} (h : a ∈ l) : a ∈ sortNats l :=
  mem_foldl_insertNat (Or.inr h)

theorem firstFailure_ne_zero {checks : List (Nat × Bool)}
    {c : Nat × Bool} (hmem : c ∈ checks) (hflag : c.2 = true)
    (hnz : ∀ d ∈ checks, d.1 ≠ 0) : firstFailure checks ≠ 0 := by
  intro h0
  unfold TaskTraceRelation.firstFailure at h0
  cases hfind : checks.find? (fun check => check.2) with
  | none =>
    have := List.find?_eq_none.mp hfind c hmem
    simp [hflag] at this
  | some d =>
    rw [hfind] at h0
    exact hnz d (List.mem_of_find?_eq_some hfind) h0

private theorem orderChecks_indices_nonzero (events : List Event) :
    ∀ d ∈ orderChecks events, d.1 ≠ 0 := by
  intro d hd
  simp only [orderChecks, List.mem_cons, List.not_mem_nil, or_false] at hd
  rcases hd with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
    | rfl | rfl | rfl | rfl | rfl | rfl <;> simp

/-- Extraction: an admitted order's loan conjunct holds, by name. -/
theorem loanFlag_false_of_checkOrder_zero {t : List Event}
    (h : checkOrder t = 0) : loanFlag t = false := by
  cases hb : loanFlag t with
  | false => rfl
  | true =>
    refine absurd h ?_
    unfold checkOrder
    exact firstFailure_ne_zero (c := (19, loanFlag t))
      (by simp [orderChecks]) hb (orderChecks_indices_nonzero t)

/-- Extraction: an admitted order's activity-after-join conjunct holds, by
name. -/
theorem activityAfterJoinFlag_false_of_checkOrder_zero {t : List Event}
    (h : checkOrder t = 0) : activityAfterJoinFlag t = false := by
  cases hb : activityAfterJoinFlag t with
  | false => rfl
  | true =>
    refine absurd h ?_
    unfold checkOrder
    exact firstFailure_ne_zero (c := (18, activityAfterJoinFlag t))
      (by simp [orderChecks]) hb (orderChecks_indices_nonzero t)

/-! ## General refusal lemmas for the extended relation

The same statements the pinned module proves for its relation, for arbitrary
extended-alphabet traces. -/

theorem unattributable_is_refused (reference observed : List Event)
    (h : unattributable observed = true) : relate reference observed = 1 := by
  simp [relate, h]

theorem duplicate_occurrences_are_refused (reference observed : List Event)
    (attributable : unattributable observed = false)
    (h : hasDuplicate observed = true) : relate reference observed = 2 := by
  simp [relate, attributable, h]

theorem admitted_traces_are_attributable (reference observed : List Event)
    (h : relate reference observed = 0) : unattributable observed = false := by
  by_cases hu : unattributable observed = true
  · rw [unattributable_is_refused reference observed hu] at h
    exact absurd h (by decide)
  · simpa using hu

theorem admitted_traces_have_distinct_occurrences (reference observed : List Event)
    (h : relate reference observed = 0) : hasDuplicate observed = false := by
  have attributable := admitted_traces_are_attributable reference observed h
  by_cases hd : hasDuplicate observed = true
  · rw [duplicate_occurrences_are_refused reference observed attributable hd] at h
    exact absurd h (by decide)
  · simpa using hd

theorem admitted_traces_satisfy_the_order_constraints (reference observed : List Event)
    (h : relate reference observed = 0) : checkOrder observed = 0 := by
  have attributable := admitted_traces_are_attributable reference observed h
  have distinct := admitted_traces_have_distinct_occurrences reference observed h
  simp only [relate, attributable, distinct, Bool.false_eq_true, ite_false] at h
  split at h
  · exact absurd h (by decide)
  · split at h
    · exact absurd h (by decide)
    · exact h

theorem admitted_traces_report_the_same_facts (reference observed : List Event)
    (h : relate reference observed = 0) :
    observed.length = reference.length := by
  have attributable := admitted_traces_are_attributable reference observed h
  have distinct := admitted_traces_have_distinct_occurrences reference observed h
  refine Decidable.byContradiction fun different => ?_
  simp [relate, attributable, distinct, different] at h

end QCore.TaskTraceWrites
