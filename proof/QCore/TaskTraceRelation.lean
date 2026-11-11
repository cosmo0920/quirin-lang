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
# F02 — one source-to-native trace relation

The Lean side of the relation between a reference run of the checked
owned-Buffer worker/capsule/join path and an independently observed rendering
of the same run — for example the stdout of the generated C11 program.

The Rust side is **production code**: `qcore_core::task_trace_relation::relate`
and its helpers. This module is a transcription of it, and the transcription is
not asserted: both sides fold the same digests over the same declared finite
domains, so a transcription that differed anywhere in a domain would be caught
by a digest rather than assumed away. The Rust harness that recomputes every
number below is `crates/qcore-core/tests/task_trace_relation.rs`.

What this file establishes:

* `domain_one_size`, `domain_*_trace_digest` — the enumerated domains are the
  ones the Rust harness enumerates.
* `domain_*_verdict_digest`, `domain_*_census` — the *classification* of every
  trace in each domain, class for class, matches the Rust relation.
* `domain_two_admits_six_schedules`,
  `domain_two_admits_a_schedule_other_than_the_reference` — the relation
  compares by admitted causal constraints and not by identical total schedules.
* `domain_one_admits_only_the_reference_order` — on the plain round trip the
  admitted constraints happen to determine a unique linearization.
* the general refusal lemmas, which hold for arbitrary traces rather than only
  on a domain.

What this file does **not** establish: that a Lean theorem proves the Rust
implementation correct, that a native execution took the order it rendered, or
anything about C programs outside this profile. Native trace replay is not
general C refinement.

Since card `CT03-PROOF-ALPHABET` the production alphabet also carries the T05
Buffer write; `QCore.TaskTraceWrites` transcribes that extended relation and
proves it conservative over this module — this file's alphabet, relation and
pins are unchanged and remain the transcription of the relation's restriction
to the write-free sublanguage.
-/

namespace QCore.TaskTraceRelation

-- D3's generated traces must be expanded once before evaluating its verdicts.
-- Reducing the nested schedules/transpositions afresh at every use makes plain
-- `decide` prohibitively expensive. `cbv` supplies a kernel-checked equality for
-- that expansion; `decide +kernel` then checks the concrete result once.
-- All four D3 measurements share a single expansion (`domain_three_measurements`
-- below) rather than paying for one each; the public statements are unchanged
-- and are projections out of that one conjunction.
-- AuditAll still permits only the standard logical axioms. The domain,
-- relation, theorem statements and pinned results remain unchanged.
-- The bounded computations need more recursion/heartbeats than the defaults.
set_option maxRecDepth 100000
set_option maxHeartbeats 4000000

/-! ## The observable alphabet

Mirrors `ObservedOutcome` and `ObservedEvent`. The declared domains carry no
negative payload and no typed error outcome, so those are represented by `Nat`
and left out of `Outcome`; the Rust digest refuses exactly the same cases. -/

inductive Outcome where
  | unit
  | i32 (value : Nat)
  | buffer (generation : Nat)
  deriving DecidableEq, BEq, Repr, Inhabited

inductive Event where
  | allocated (task buffer payload : Nat)
  | allocationFailed (task : Nat)
  | spawned (child worker : Nat)
  | creationFailed (worker : Nat)
  | read (task buffer value : Nat)
  | dropped (task buffer : Nat)
  | loaned (borrower buffer : Nat)
  | returned (borrower buffer : Nat)
  | completed (task : Nat) (outcome : Outcome)
  | joined (child : Nat)
  deriving DecidableEq, BEq, Repr, Inhabited

/-- P2 (attribution): the task whose state change a line reports, not the
thread that printed it. P3 (flat scopes): the creator, lender and joiner is the
root task. `creationFailed` names no task and is therefore unattributable. -/
def actorOf : Event → Option Nat
  | .allocated task _ _ => some task
  | .allocationFailed task => some task
  | .read task _ _ => some task
  | .dropped task _ => some task
  | .completed task _ => some task
  | .spawned _ _ => some 0
  | .joined _ => some 0
  | .loaned _ _ => some 0
  | .returned _ _ => some 0
  | .creationFailed _ => none

def bufferOf : Event → Option Nat
  | .allocated _ buffer _ => some buffer
  | .read _ buffer _ => some buffer
  | .dropped _ buffer => some buffer
  | .loaned _ buffer => some buffer
  | .returned _ buffer => some buffer
  | .completed _ (.buffer generation) => some generation
  | _ => none

/-- The child a line is *about*, which is not always its actor. -/
def subjectOf : Event → Option Nat
  | .spawned child _ => some child
  | .joined child => some child
  | .loaned borrower _ => some borrower
  | .returned borrower _ => some borrower
  | .completed task _ => if task == 0 then none else some task
  | .allocated task _ _ => if task == 0 then none else some task
  | .allocationFailed task => if task == 0 then none else some task
  | .read task _ _ => if task == 0 then none else some task
  | .dropped task _ => if task == 0 then none else some task
  | .creationFailed _ => none

/-! ## Positions and sorted identifier lists

`positions` mirrors the Rust `single` helper and `sortNats` mirrors iterating a
`BTreeSet`, so both sides visit buffers and children in the same order and
therefore report the same *first* violated conjunct. -/

def positionsFrom (index : Nat) (p : Event → Bool) : List Event → List Nat
  | [] => []
  | event :: rest =>
      if p event then index :: positionsFrom (index + 1) p rest
      else positionsFrom (index + 1) p rest

def positions (events : List Event) (p : Event → Bool) : List Nat :=
  positionsFrom 0 p events

def insertNat (value : Nat) : List Nat → List Nat
  | [] => [value]
  | head :: rest =>
      if value == head then head :: rest
      else if value < head then value :: head :: rest
      else head :: insertNat value rest

def sortNats (values : List Nat) : List Nat :=
  values.foldl (fun sorted value => insertNat value sorted) []

def isAllocationOf (buffer : Nat) : Event → Bool
  | .allocated _ candidate _ => candidate == buffer
  | _ => false

def isReleaseOf (buffer : Nat) : Event → Bool
  | .dropped _ candidate => candidate == buffer
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

def payloadOf (events : List Event) (buffer : Nat) : Option Nat :=
  match (events.filter (isAllocationOf buffer)) with
  | .allocated _ _ payload :: _ => some payload
  | _ => none

def firstFailure (checks : List (Nat × Bool)) : Nat :=
  match checks.find? (fun check => check.2) with
  | some check => check.1
  | none => 0

/-! ## R4–R9: the order constraints one trace must satisfy on its own -/

def checkOrder (events : List Event) : Nat :=
  let buffers := sortNats (events.filterMap bufferOf)
  let children := sortNats (events.filterMap subjectOf)
  let allocations := fun buffer => positions events (isAllocationOf buffer)
  let releases := fun buffer => positions events (isReleaseOf buffer)
  let uses := fun buffer => positions events (mentions buffer)
  let creations := fun child => positions events (isCreationOf child)
  let joins := fun child => positions events (isJoinOf child)
  let completions := fun child => positions events (isCompletionOf child)
  let roots := positions events (fun event =>
    match event with | .completed task _ => task == 0 | _ => false)
  firstFailure
    [ (5, buffers.any fun buffer => (allocations buffer).length > 1)
    , (6, buffers.any fun buffer => (allocations buffer).isEmpty)
    , (7, buffers.any fun buffer =>
            match (allocations buffer).head? with
            | some allocation => (uses buffer).any (fun index => index < allocation)
            | none => false)
    , (8, buffers.any fun buffer =>
            match payloadOf events buffer with
            | some payload => events.any fun event =>
                match event with
                | .read _ candidate value => candidate == buffer && value != payload
                | _ => false
            | none => false)
    , (9, buffers.any fun buffer => (releases buffer).length > 1)
    , (10, buffers.any fun buffer =>
            match (releases buffer).head? with
            | some release => (uses buffer).any (fun index => index > release)
            | none => false)
    , (11, buffers.any fun buffer =>
            !(allocations buffer).isEmpty && (releases buffer).isEmpty)
    , (12, children.any fun child => (creations child).length > 1)
    , (13, children.any fun child => (creations child).isEmpty)
    , (14, children.any fun child =>
            match (creations child).head? with
            | some creation => (positions events (isAbout child)).any (fun i => i < creation)
            | none => false)
    , (15, children.any fun child => (joins child).length > 1)
    , (16, children.any fun child =>
            !(creations child).isEmpty && (joins child).isEmpty)
    , (17, children.any fun child =>
            match (joins child).head? with
            | some join =>
                (completions child).length != 1 || ((completions child).headD 0) > join
            | none => false)
    , (18, children.any fun child =>
            match (joins child).head? with
            | some join =>
                (positions events (fun e => isAbout child e && !isReturn e)).any
                  (fun index => index > join)
            | none => false)
    , (19, events.any fun event =>
            match event with
            | .loaned borrower buffer =>
                let opened := (positions events (fun e => e == .loaned borrower buffer)).headD 0
                let reads := positions events (fun e =>
                  match e with
                  | .read task candidate _ => task == borrower && candidate == buffer
                  | _ => false)
                let returns := positions events (fun e => e == .returned borrower buffer)
                reads.any (fun index => index < opened)
                  || (match (joins borrower).head? with
                      | some join => returns.length != 1 || (returns.headD 0) < join
                      | none => false)
            | _ => false)
    , (20, roots.length != 1 || (roots.headD 0) + 1 != events.length)
    ]

/-! ## R1–R3 and the relation -/

def hasDuplicate : List Event → Bool
  | [] => false
  | event :: rest => rest.contains event || hasDuplicate rest

def unattributable (events : List Event) : Bool :=
  events.any fun event => (actorOf event).isNone

def projection (events : List Event) (actor : Nat) : List Event :=
  events.filter fun event => actorOf event == some actor

/-- Declare a reference linearization: every conjunct the relation later demands
of an observation is first demanded of the reference. `0` means admitted. -/
def declare (events : List Event) : Nat :=
  if unattributable events then 1
  else if hasDuplicate events then 2
  else checkOrder events

/-- Relate an independently observed trace to an admitted causal order. `0`
means admitted; any other value is the first violated conjunct's class. -/
def relate (reference observed : List Event) : Nat :=
  if unattributable observed then 1
  else if hasDuplicate observed then 2
  else if observed.length != reference.length
      || observed.any (fun event => !reference.contains event) then 3
  else if (sortNats (reference.filterMap actorOf)).any
      (fun actor => projection observed actor != projection reference actor) then 4
  else checkOrder observed

/-! ## General refusal lemmas

These hold for arbitrary traces, not only on a declared domain. -/

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
  · rw [unattributable_is_refused reference observed hu] at h; exact absurd h (by decide)
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
  have mismatch : (observed.length != reference.length
      || observed.any fun event => !reference.contains event) = true := by
    simp [different]
  simp [relate, attributable, distinct, mismatch] at h

/-! ## Digests

Plain FNV-style folds over `Nat`, reduced modulo `2 ^ 64` after every step so
that the Rust harness reproduces them exactly with wrapping `u64` arithmetic. A
digest is a comparison device, not a proof device. -/

def modulus : Nat := 18446744073709551616
def traceSeed : Nat := 1469598103934665603
def traceMul : Nat := 1000003
def domainSeed : Nat := 14695981039346656037
def domainMul : Nat := 1099511628211

def mix (accumulator value : Nat) : Nat :=
  (accumulator * traceMul + value + 1) % modulus

def encodeOutcome : Outcome → Nat
  | .unit => 0
  | .i32 value => 1 + 4 * value
  | .buffer generation => 2 + 4 * generation

/-- Injective for the identifiers the declared domains use. -/
def encodeEvent : Event → Nat
  | .allocated task buffer payload => 1 + 16 * (task + 8 * (buffer + 64 * payload))
  | .allocationFailed task => 2 + 16 * task
  | .spawned child worker => 3 + 16 * (child + 8 * worker)
  | .creationFailed worker => 4 + 16 * worker
  | .read task buffer value => 5 + 16 * (task + 8 * (buffer + 64 * value))
  | .dropped task buffer => 6 + 16 * (task + 8 * buffer)
  | .loaned borrower buffer => 7 + 16 * (borrower + 8 * buffer)
  | .returned borrower buffer => 8 + 16 * (borrower + 8 * buffer)
  | .completed task outcome => 9 + 16 * (task + 8 * encodeOutcome outcome)
  | .joined child => 10 + 16 * child

def mixTrace (events : List Event) : Nat :=
  events.foldl (fun accumulator event => mix accumulator (encodeEvent event)) traceSeed

def traceDigest (traces : List (List Event)) : Nat :=
  traces.foldl (fun accumulator trace =>
    (accumulator * domainMul + mixTrace trace + 1) % modulus) domainSeed

def verdictDigest (reference : List Event) (traces : List (List Event)) : Nat :=
  traces.foldl (fun accumulator trace =>
    (accumulator * domainMul + relate reference trace + 1) % modulus) domainSeed

/-- The traces the relation admits. `verdictDigest` already pins the *exact*
class of every trace in domain order — it is a fold of the class codes
themselves — so this count is the readable summary rather than the substance,
and the per-class census is decoded on the Rust side where it costs nothing. -/
def admittedCount (reference : List Event) (traces : List (List Event)) : Nat :=
  traces.countP fun trace => relate reference trace == 0

/-! ## The two selected runs

Both are the real reference runs of admitted `ScopedOwnedTasksV1` source
programs, recorded verbatim in
`crates/qcore-core/tests/fixtures/f02/*.trace`. -/

/-- CT01's owned round trip: allocate, move into the child, the child returns
the same generation, join, read, release. -/
def roundTrip : List Event :=
  [ .allocated 0 1 42
  , .spawned 1 0
  , .completed 1 (.buffer 1)
  , .joined 1
  , .read 0 1 42
  , .dropped 0 1
  , .completed 0 (.i32 42) ]

/-- The same path with parent work that is genuinely concurrent with the
child's: the parent reads and releases a second Buffer between creation and
join, while the child reads and returns the Buffer it was given. -/
def concurrent : List Event :=
  [ .allocated 0 1 42
  , .allocated 0 2 7
  , .spawned 1 0
  , .read 0 2 7
  , .dropped 0 2
  , .read 1 1 42
  , .completed 1 (.buffer 1)
  , .joined 1
  , .read 0 1 42
  , .dropped 0 1
  , .completed 0 (.i32 42) ]

theorem the_round_trip_is_an_admitted_causal_order : declare roundTrip = 0 := by decide

theorem the_concurrent_run_is_an_admitted_causal_order : declare concurrent = 0 := by decide

/-! ## The declared finite domains

Each domain is the set of *schedules* of one run: every ordering that preserves
each actor's own program order, which is what a different observer of the same
execution could legitimately have rendered. Class `4` — an actor's own facts out
of order — is therefore the one class the schedule domains cannot contain, and
D3 supplies it by perturbing admitted schedules.

The Rust harness additionally enumerates all `7! = 5040` orderings of the round
trip, which is outside the Lean-kernel domain for cost. That extension is
recorded as a Rust-only measurement, exactly as it is named in the report. -/

def get (events : List Event) (index : Nat) : Event := events.getD index (.joined 0)

/-- Place the child's facts at `slots` and the parent's facts everywhere else,
each in its own order. Mirrors the Rust `spread` helper. -/
def spread (slots : List Nat) (parents children : List Event) : List Nat → List Event
  | [] => []
  | position :: rest =>
      if slots.contains position then
        children.headD (.joined 0) :: spread slots parents children.tail rest
      else
        parents.headD (.joined 0) :: spread slots parents.tail children rest

def schedules (reference : List Event) (slotSets : List (List Nat)) : List (List Event) :=
  slotSets.map fun slots =>
    spread slots (projection reference 0) (projection reference 1)
      (List.range reference.length)

/-- **D1** — the seven schedules of the round trip: its single child fact at
every position. -/
def domainOne : List (List Event) :=
  schedules roundTrip ((List.range 7).map fun position => [position])

/-- **D2** — the `C(11, 2) = 55` schedules of the concurrent run: its two child
facts at every pair of positions, in order. -/
def domainTwo : List (List Event) :=
  schedules concurrent
    ((List.range 11).flatMap fun first =>
      ((List.range 11).filter fun second => first < second).map fun second => [first, second])

def swapAt (first second : Nat) (events : List Event) : List Event :=
  (List.range events.length).map fun index =>
    if index == first then get events second
    else if index == second then get events first
    else get events index

/-- **D3** — every single transposition of every D2 schedule. -/
def domainThree : List (List Event) :=
  domainTwo.flatMap fun trace =>
    (List.range 11).flatMap fun first =>
      ((List.range 11).filter fun second => first < second).map fun second =>
        swapAt first second trace

/-! ## Domain agreement -/

set_option cbv.maxSteps 4000000

/-- Every D3 measurement, decided against **one** expansion of the domain.

`domainThree` is 3,025 generated traces; expanding it costs the `cbv` step, the
kernel's check of that expansion, and the sharing/compilation of the resulting
literal. Proving the four D3 facts separately paid that cost four times. Here it
is paid once — `pattern (occs := *)` rewrites every occurrence, so the four
decidable propositions below are decided against the same shared literal — and
each public theorem is a projection out of this conjunction. The statements, the
domain, the relation and the pinned numbers are exactly as they were. -/
private theorem domain_three_measurements :
    domainThree.length = 3025
      ∧ traceDigest domainThree = 9380586544780836206
      ∧ verdictDigest concurrent domainThree = 15391469991380593011
      ∧ admittedCount concurrent domainThree = 18 := by
  conv => pattern (occs := *) domainThree; all_goals cbv
  decide +kernel

theorem domain_one_size : domainOne.length = 7 := by decide +kernel
theorem domain_two_size : domainTwo.length = 55 := by decide +kernel
theorem domain_three_size : domainThree.length = 3025 := domain_three_measurements.1

theorem domain_one_trace_digest : traceDigest domainOne = 1827949402977241471 := by decide +kernel
theorem domain_two_trace_digest : traceDigest domainTwo = 7621727749705747350 := by decide +kernel
theorem domain_three_trace_digest : traceDigest domainThree = 9380586544780836206 :=
  domain_three_measurements.2.1

/-! ## Transcription agreement and the measured results -/

theorem domain_one_verdict_digest :
    verdictDigest roundTrip domainOne = 6242011289262393457 := by decide +kernel

theorem domain_two_verdict_digest :
    verdictDigest concurrent domainTwo = 3851450914538809003 := by decide +kernel

theorem domain_three_verdict_digest :
    verdictDigest concurrent domainThree = 15391469991380593011 :=
  domain_three_measurements.2.2.1

/-! ## What the relation admits

The point of the card: the relation compares by admitted causal constraints and
does not demand an identical total schedule. -/

/-- Every causal constraint of the plain round trip happens to determine a
unique schedule. That is a measurement about this program — and the reason
byte-equal stdout was an adequate check for CT01 — not a property of the
relation. -/
theorem domain_one_admits_only_the_reference_order :
    domainOne.filter (fun trace => relate roundTrip trace == 0) = [roundTrip] := by decide +kernel

theorem domain_two_admits_six_schedules : admittedCount concurrent domainTwo = 6 := by decide +kernel

/-- The reference rendering is one of the six. -/
theorem domain_two_admits_the_reference_rendering :
    domainTwo.contains concurrent = true ∧ relate concurrent concurrent = 0 := by decide +kernel

/-- ... and it is not the only one: the relation admits five renderings of this
same run that byte equality against the reference would refuse. -/
theorem domain_two_admits_a_schedule_other_than_the_reference :
    ((domainTwo.filter fun trace => relate concurrent trace == 0).filter
      fun trace => trace != concurrent).length = 5 := by decide +kernel

/-- Eighteen of the 3025 perturbations land on another admitted schedule, so the
admitted set is genuinely a partial order and not the reference total order
under another name. -/
theorem domain_three_admits_eighteen_perturbations :
    admittedCount concurrent domainThree = 18 :=
  domain_three_measurements.2.2.2

/-! ## Non-vacuity

A digest is trivially satisfiable by a relation that admits everything or
refuses everything, so both directions are pinned in every domain. The refusal
side is derived from the two facts already evaluated above rather than costing a
second traversal. -/

theorem domain_one_admits_one_schedule : admittedCount roundTrip domainOne = 1 := by decide +kernel

theorem domain_one_refuses_part_of_its_domain :
    admittedCount roundTrip domainOne < domainOne.length := by
  rw [domain_one_admits_one_schedule, domain_one_size]; decide

theorem domain_two_refuses_part_of_its_domain :
    admittedCount concurrent domainTwo < domainTwo.length := by
  rw [domain_two_admits_six_schedules, domain_two_size]; decide

theorem domain_three_refuses_most_of_its_domain :
    admittedCount concurrent domainThree < domainThree.length := by
  rw [domain_three_admits_eighteen_perturbations, domain_three_size]; decide

end QCore.TaskTraceRelation
