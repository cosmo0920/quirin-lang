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

/-! Error-region credit composed with scoped-task lifetime obligations (card T07).

The model has two kinds of thing and keeps them apart on purpose:

* **credit**, a conserved natural number that sits in a region frame, in a
  sealed cleanup pack, with a child, or in one of the two sinks `spent` and
  `abandoned`; and
* **duties**, a collection of obligations — a release, a join, a read loan's
  expiry, a child's endowment — that no operation ever converts into credit and
  no operation ever creates out of credit.

The theorems below say three things. Credit is conserved by every admitted
step, so nothing can be manufactured or destroyed outside the two sinks. The
only sink an error path has, abandoning a discharged pack's remainder, is
refused while any join, loan or child endowment the region opened is still
outstanding. And sealing a frame on an error edge carries its duties forward
verbatim, so a rethrow sheds nothing.

These proofs concern that discipline. They are not about source lowering,
native execution, scheduling, the C memory model, cancellation or resumable
effects, none of which this model has. -/
namespace QCore.RegionTaskCredit

/-- A live obligation. Deliberately not a quantity. -/
inductive Duty where
  | release (resource : Nat)
  | join (handle : Nat)
  | loanExpiry (resource : Nat)
  | childCredit (child : Nat)
  deriving DecidableEq, Repr

/-- One region frame: its own allowable credit, its ordered destroy duties and
its other obligations. -/
structure Frame where
  allowable : Nat
  releases : List Nat
  obligations : List Duty
  deriving DecidableEq, Repr

/-- A frame sealed by an error edge. The same shape: sealing loses nothing. -/
structure Pack where
  allowable : Nat
  releases : List Nat
  obligations : List Duty
  deriving DecidableEq, Repr

/-- Credit currently out with children, as an association list. -/
abbrev Children := List (Nat × Nat)

structure St where
  frames : List Frame
  children : Children
  opened : Option Pack
  spent : Nat
  abandoned : Nat
  /-- Handles whose join actually returned a completion capsule. -/
  joined : List Nat
  /-- Resources whose read loan is still outstanding. -/
  loaned : List Nat
  deriving DecidableEq, Repr

def frameTotal : List Frame → Nat
  | [] => 0
  | frame :: rest => frame.allowable + frameTotal rest

def childTotal : Children → Nat
  | [] => 0
  | (_, credit) :: rest => credit + childTotal rest

def packTotal : Option Pack → Nat
  | none => 0
  | some pack => pack.allowable

/-- Every unit of credit the state can account for, wherever it sits. The duty
collections are deliberately absent from this sum. -/
def accounted (state : St) : Nat :=
  frameTotal state.frames + childTotal state.children + packTotal state.opened +
    state.spent + state.abandoned

/-- Find one child's credit and remove it, preserving the rest in order. -/
def takeChild : Children → Nat → Option (Nat × Children)
  | [], _ => none
  | (name, credit) :: rest, child =>
      if name = child then some (credit, rest)
      else (takeChild rest child).map fun found =>
        (found.1, (name, credit) :: found.2)

/-- Is this duty still live? Read off the ledger facts, never from bookkeeping. -/
def outstanding (state : St) : Duty → Bool
  | .release _ => true
  | .join handle => !state.joined.contains handle
  | .loanExpiry resource => state.loaned.contains resource
  | .childCredit child => (takeChild state.children child).isSome

/-- The abandonment gate: no join, loan or child endowment the pack opened may
still be live. Release duties are *run*, not abandoned, so they do not gate. -/
def dischargeable (state : St) (pack : Pack) : Bool :=
  pack.obligations.all fun duty =>
    match duty with
    | .release _ => true
    | other => !outstanding state other

inductive Op where
  /-- Open a nested region, drawing its endowment from the enclosing frame. -/
  | endow (credit : Nat)
  /-- A normal region exit: refused unless the frame is credit-free and has no
  live obligation. -/
  | closeRegion
  | spend (amount : Nat)
  /-- One creation. `created = false` is a creation that did not happen. -/
  | create (child handle amount : Nat) (loans : List Nat) (created : Bool)
  /-- A child's declared accounting at completion. -/
  | complete (child unspent : Nat)
  | joinChild (child handle : Nat) (loans : List Nat)
  /-- A join that observed nothing. Terminal for the handle. -/
  | failJoin (handle : Nat)
  /-- An error edge: seal the innermost frame as the opened cleanup pack. -/
  | sealFrame
  | discharge
  deriving Repr

def dropAll (xs ys : List Nat) : List Nat :=
  xs.filter fun x => !ys.contains x

def step (state : St) : Op → Option St
  | .endow credit =>
      match state.frames with
      | [] => none
      | frame :: rest =>
          if credit ≤ frame.allowable then
            some { state with
              frames :=
                { allowable := credit, releases := [], obligations := [] } ::
                  { frame with allowable := frame.allowable - credit } :: rest }
          else none
  | .closeRegion =>
      match state.frames with
      | [] => none
      | frame :: rest =>
          if frame.allowable = 0 ∧ dischargeable state
              { allowable := frame.allowable, releases := frame.releases,
                obligations := frame.obligations } then
            some { state with frames := rest }
          else none
  | .spend amount =>
      match state.frames with
      | [] => none
      | frame :: rest =>
          if amount ≤ frame.allowable then
            some { state with
              frames := { frame with allowable := frame.allowable - amount } :: rest
              spent := state.spent + amount }
          else none
  | .create child handle amount loans created =>
      match created with
      | false => some state
      | true =>
        match state.frames with
        | [] => none
        | frame :: rest =>
            if amount ≤ frame.allowable then
              some { state with
                frames :=
                  { frame with
                    allowable := frame.allowable - amount
                    obligations :=
                      .join handle :: (loans.map Duty.loanExpiry ++
                        (if amount = 0 then [] else [.childCredit child]) ++
                        frame.obligations) } :: rest
                children := if amount = 0 then state.children
                  else (child, amount) :: state.children
                loaned := loans ++ state.loaned }
            else none
  | .complete child unspent =>
      match takeChild state.children child with
      | none => none
      | some (held, remainder) =>
          if unspent ≤ held then
            some { state with
              children := (child, unspent) :: remainder
              spent := state.spent + (held - unspent) }
          else none
  | .joinChild child handle loans =>
      if state.joined.contains handle then none
      else
        match state.frames, takeChild state.children child with
        | [], _ => none
        | _ :: _, none =>
            some { state with
              joined := handle :: state.joined
              loaned := dropAll state.loaned loans }
        | frame :: rest, some (returned, remainder) =>
            some { state with
              frames := { frame with allowable := frame.allowable + returned } :: rest
              children := remainder
              joined := handle :: state.joined
              loaned := dropAll state.loaned loans }
  | .failJoin _ => some state
  | .sealFrame =>
      match state.opened, state.frames with
      | none, frame :: rest =>
          some { state with
            frames := rest
            opened := some
              { allowable := frame.allowable, releases := frame.releases,
                obligations := frame.obligations } }
      | _, _ => none
  | .discharge =>
      match state.opened with
      | none => none
      | some pack =>
          if dischargeable state pack then
            some { state with
              opened := none
              abandoned := state.abandoned + pack.allowable }
          else none

def run (state : St) : List Op → Option St
  | [] => some state
  | op :: rest => (step state op).bind fun next => run next rest

-- ---------------------------------------------------------------------------
-- Credit conservation
-- ---------------------------------------------------------------------------

private theorem takeChild_splits :
    ∀ {children : Children} {child found : Nat} {remainder : Children},
      takeChild children child = some (found, remainder) →
        childTotal children = found + childTotal remainder := by
  intro children
  induction children with
  | nil => intro child found remainder h; simp [takeChild] at h
  | cons head tail ih =>
    intro child found remainder h
    obtain ⟨name, credit⟩ := head
    simp only [takeChild] at h
    split at h
    · cases h
      simp [childTotal]
    · simp only [Option.map_eq_some_iff] at h
      obtain ⟨⟨inner, innerRest⟩, taken, built⟩ := h
      cases built
      have carried := ih taken
      simp only [childTotal, carried]
      omega

/-- Every admitted step preserves the accounted total: credit is neither
manufactured nor destroyed, only moved between frames, packs, children and the
two sinks. -/
theorem step_preserves_accounted {state next : St} {op : Op}
    (checked : step state op = some next) : accounted next = accounted state := by
  obtain ⟨frames, children, opened, spent, abandoned, joined, loaned⟩ := state
  cases op with
  | endow credit =>
    cases frames with
    | nil => simp [step] at checked
    | cons frame rest =>
      simp only [step] at checked
      split at checked
      · cases checked
        simp only [accounted, frameTotal]
        omega
      · simp at checked
  | closeRegion =>
    cases frames with
    | nil => simp [step] at checked
    | cons frame rest =>
      simp only [step] at checked
      split at checked
      · rename_i closable
        cases checked
        simp only [accounted, frameTotal, closable.1]
        omega
      · simp at checked
  | spend amount =>
    cases frames with
    | nil => simp [step] at checked
    | cons frame rest =>
      simp only [step] at checked
      split at checked
      · cases checked
        simp only [accounted, frameTotal]
        omega
      · simp at checked
  | create child handle amount loans created =>
    cases created with
    | false =>
      simp only [step] at checked
      cases checked
      rfl
    | true =>
      cases frames with
      | nil => simp [step] at checked
      | cons frame rest =>
        simp only [step] at checked
        split at checked
        · cases checked
          by_cases zero : amount = 0
          · simp only [accounted, frameTotal, zero, ite_eq_left]
            omega
          · simp only [accounted, frameTotal, childTotal, ite_eq_right zero]
            omega
        · simp at checked
  | complete child unspent =>
    cases taken : takeChild children child with
    | none => simp [step, taken] at checked
    | some pair =>
      obtain ⟨held, remainder⟩ := pair
      simp only [step, taken] at checked
      split at checked
      · cases checked
        have carried := takeChild_splits taken
        simp only [accounted, childTotal, carried]
        omega
      · simp at checked
  | joinChild child handle loans =>
    simp only [step] at checked
    split at checked
    · simp at checked
    · cases frames with
      | nil => simp at checked
      | cons frame rest =>
        cases taken : takeChild children child with
        | none =>
          simp only [taken] at checked
          cases checked
          simp only [accounted, frameTotal]
        | some pair =>
          obtain ⟨returned, remainder⟩ := pair
          simp only [taken] at checked
          cases checked
          have carried := takeChild_splits taken
          simp only [accounted, frameTotal, carried]
          omega
  | failJoin handle =>
    simp only [step] at checked
    cases checked
    rfl
  | sealFrame =>
    cases opened with
    | some pack => simp [step] at checked
    | none =>
      cases frames with
      | nil => simp [step] at checked
      | cons frame rest =>
        simp only [step] at checked
        cases checked
        simp only [accounted, frameTotal, packTotal]
        omega
  | discharge =>
    cases opened with
    | none => simp [step] at checked
    | some pack =>
      simp only [step] at checked
      split at checked
      · cases checked
        simp only [accounted, packTotal]
        omega
      · simp at checked

/-- The headline conservation statement: an entire admitted run leaves the
accounted total exactly where it started. -/
theorem run_preserves_accounted :
    ∀ {ops : List Op} {state next : St},
      run state ops = some next → accounted next = accounted state := by
  intro ops
  induction ops with
  | nil =>
    intro state next h
    simp only [run, Option.some.injEq] at h
    rw [h]
  | cons op rest ih =>
    intro state next h
    simp only [run, Option.bind_eq_some_iff] at h
    obtain ⟨middle, stepped, tail⟩ := h
    rw [ih tail, step_preserves_accounted stepped]

-- ---------------------------------------------------------------------------
-- A duty is not credit
-- ---------------------------------------------------------------------------

/-- Acceptance 1, stated on the operation that could discard a duty: a
discharge abandons exactly the pack's remainder and changes **nothing else** —
not a frame, not a duty, not a child's credit, not a loan, not a join. -/
theorem discharge_only_moves_the_packs_remainder {state next : St}
    (checked : step state .discharge = some next) :
    ∃ pack, state.opened = some pack ∧
      next = { state with
        opened := none
        abandoned := state.abandoned + pack.allowable } := by
  cases opened : state.opened with
  | none => simp [step, opened] at checked
  | some pack =>
    refine ⟨pack, rfl, ?_⟩
    simp only [step, opened] at checked
    split at checked
    · cases checked
      rfl
    · simp at checked

/-- Spending credit cannot retire an obligation: the frames' duty lists, the
loans, the joins and the children's credit are all untouched by it. -/
theorem spending_retires_no_duty {state next : St} {amount : Nat}
    (checked : step state (.spend amount) = some next) :
    next.frames.map Frame.obligations = state.frames.map Frame.obligations ∧
      next.loaned = state.loaned ∧ next.joined = state.joined ∧
      next.children = state.children := by
  cases frames : state.frames with
  | nil => simp [step, frames] at checked
  | cons frame rest =>
    simp only [step, frames] at checked
    split at checked
    · cases checked
      simp
    · simp at checked

/-- Nor can endowing a nested region: the enclosing frame keeps every duty it
had, and the new frame starts with none of its own. -/
theorem endowment_moves_credit_and_no_duty {state next : St} {credit : Nat}
    {frame : Frame} {rest : List Frame} (frames : state.frames = frame :: rest)
    (checked : step state (.endow credit) = some next) :
    next.frames = { allowable := credit, releases := [], obligations := [] } ::
        { frame with allowable := frame.allowable - credit } :: rest := by
  simp only [step, frames] at checked
  split at checked
  · cases checked
    rfl
  · simp at checked

/-- The parent's own sinks never reach a child's credit: neither spending nor
abandoning changes what children hold. -/
theorem the_parent_sinks_cannot_reach_child_credit {state spentState abandonedState : St}
    {amount : Nat}
    (spending : step state (.spend amount) = some spentState)
    (abandoning : step state .discharge = some abandonedState) :
    childTotal spentState.children = childTotal state.children ∧
      childTotal abandonedState.children = childTotal state.children := by
  refine ⟨?_, ?_⟩
  · rw [(spending_retires_no_duty spending).2.2.2]
  · obtain ⟨_, _, rebuilt⟩ := discharge_only_moves_the_packs_remainder abandoning
    rw [rebuilt]

-- ---------------------------------------------------------------------------
-- The abandonment gate
-- ---------------------------------------------------------------------------

private theorem gate_of_member {state : St} {pack : Pack} {duty : Duty}
    (present : duty ∈ pack.obligations) (live : outstanding state duty = true)
    (concurrency : ∀ resource, duty ≠ .release resource) :
    dischargeable state pack = false := by
  cases positive : dischargeable state pack with
  | false => rfl
  | true =>
    simp only [dischargeable, List.all_eq_true] at positive
    have entry := positive duty present
    cases duty with
    | release resource => exact (concurrency resource rfl).elim
    | join handle => simp [live] at entry
    | loanExpiry resource => simp [live] at entry
    | childCredit child => simp [live] at entry

/-- Acceptance 2, first clause: a pack whose creation is still unjoined cannot
be discharged, so its remainder cannot be abandoned. -/
theorem discharge_requires_every_join {state : St} {pack : Pack} {handle : Nat}
    (opened : state.opened = some pack) (present : Duty.join handle ∈ pack.obligations)
    (unjoined : state.joined.contains handle = false) :
    step state .discharge = none := by
  have live : outstanding state (.join handle) = true := by
    simp only [outstanding, unjoined, Bool.not_false]
  have gate := gate_of_member present live (by intro resource h; cases h)
  simp only [step, opened, gate, Bool.false_eq_true, ite_false]

/-- The same for a read loan the region opened: read access held by somebody
else is a duty, not spare credit. -/
theorem discharge_requires_every_loan_to_expire {state : St} {pack : Pack} {resource : Nat}
    (opened : state.opened = some pack)
    (present : Duty.loanExpiry resource ∈ pack.obligations)
    (lent : state.loaned.contains resource = true) :
    step state .discharge = none := by
  have live : outstanding state (.loanExpiry resource) = true := by
    simp only [outstanding, lent]
  have gate := gate_of_member present live (by intro other h; cases h)
  simp only [step, opened, gate, Bool.false_eq_true, ite_false]

/-- Acceptance 2, second clause: credit that is out with a child is not the
parent's to abandon. -/
theorem a_parent_cannot_abandon_credit_held_by_a_child {state : St} {pack : Pack}
    {child : Nat}
    (opened : state.opened = some pack)
    (present : Duty.childCredit child ∈ pack.obligations)
    (held : (takeChild state.children child).isSome = true) :
    step state .discharge = none := by
  have live : outstanding state (.childCredit child) = true := by
    simp only [outstanding, held]
  have gate := gate_of_member present live (by intro resource h; cases h)
  simp only [step, opened, gate, Bool.false_eq_true, ite_false]

/-- A normal region exit is gated identically, so there is no exit at all that
discards a live obligation. -/
theorem a_normal_exit_is_gated_like_a_discharge {state : St} {frame : Frame}
    {rest : List Frame} {handle : Nat}
    (frames : state.frames = frame :: rest)
    (present : Duty.join handle ∈ frame.obligations)
    (unjoined : state.joined.contains handle = false) :
    step state .closeRegion = none := by
  have live : outstanding state (.join handle) = true := by
    simp only [outstanding, unjoined, Bool.not_false]
  have gate : dischargeable state
      { allowable := frame.allowable
        releases := frame.releases
        obligations := frame.obligations } = false :=
    gate_of_member present live (by intro resource h; cases h)
  simp [step, frames, gate]

/-- A normal region exit also refuses to leave credit behind: only the error
path has a sink, and only through a gated discharge. -/
theorem a_normal_exit_refuses_a_remainder {state next : St} {frame : Frame}
    {rest : List Frame}
    (frames : state.frames = frame :: rest)
    (checked : step state .closeRegion = some next) : frame.allowable = 0 := by
  simp only [step, frames] at checked
  split at checked
  · rename_i closable
    exact closable.1
  · simp at checked

/-- A failed join observes nothing and therefore closes nothing: the state is
unchanged, so every refusal above still stands afterwards, forever. -/
theorem a_failed_join_closes_nothing {state next : St} {handle : Nat}
    (checked : step state (.failJoin handle) = some next) : next = state := by
  simp only [step, Option.some.injEq] at checked
  rw [checked]

/-- Consequently a pack whose child was never observed to finish can never be
discharged, no matter how many failed joins are attempted. -/
theorem a_failed_join_leaves_the_pack_undischargeable {state next : St} {pack : Pack}
    {handle : Nat}
    (opened : state.opened = some pack) (present : Duty.join handle ∈ pack.obligations)
    (unjoined : state.joined.contains handle = false)
    (attempted : step state (.failJoin handle) = some next) :
    step next .discharge = none := by
  rw [a_failed_join_closes_nothing attempted]
  exact discharge_requires_every_join opened present unjoined

-- ---------------------------------------------------------------------------
-- Creation, join and sealing
-- ---------------------------------------------------------------------------

/-- A creation that did not happen endowed nothing, lent nothing and opened no
duty: the state is restored exactly. -/
theorem a_failed_creation_restores_the_parent_exactly {state next : St}
    {child handle amount : Nat} {loans : List Nat}
    (checked : step state (.create child handle amount loans false) = some next) :
    next = state := by
  simp only [step, Option.some.injEq] at checked
  rw [checked]

/-- A successful creation records the join duty, every loan it opened and, when
it endowed any, the child's credit duty. -/
theorem a_creation_records_every_duty_it_opens {state next : St}
    {child handle amount : Nat} {loans : List Nat} {frame : Frame} {rest : List Frame}
    (frames : state.frames = frame :: rest)
    (checked : step state (.create child handle amount loans true) = some next) :
    ∃ head tail, next.frames = head :: tail ∧
      Duty.join handle ∈ head.obligations ∧
      (∀ resource ∈ loans, Duty.loanExpiry resource ∈ head.obligations) ∧
      (amount ≠ 0 → Duty.childCredit child ∈ head.obligations) := by
  simp only [step, frames] at checked
  split at checked
  · cases checked
    refine ⟨_, rest, rfl, ?_, ?_, ?_⟩
    · simp
    · intro resource member
      simp only [List.mem_cons, List.mem_append, List.mem_map]
      exact Or.inr (Or.inl (Or.inl ⟨resource, member, rfl⟩))
    · intro nonzero
      simp only [List.mem_cons, List.mem_append, ite_eq_right nonzero]
      exact Or.inr (Or.inl (Or.inr (by simp)))
  · simp at checked

/-- A successful join returns exactly what the child still held, and nothing
else moves into the frame. -/
theorem a_join_returns_exactly_the_unspent_child_credit {state next : St}
    {child handle found : Nat} {remainder : Children} {loans : List Nat}
    {frame : Frame} {rest : List Frame}
    (frames : state.frames = frame :: rest)
    (fresh : state.joined.contains handle = false)
    (held : takeChild state.children child = some (found, remainder))
    (checked : step state (.joinChild child handle loans) = some next) :
    next.frames = { frame with allowable := frame.allowable + found } :: rest ∧
      next.children = remainder ∧ next.joined = handle :: state.joined := by
  simp only [step, frames, fresh, held, Bool.false_eq_true, ite_false] at checked
  cases checked
  exact ⟨rfl, rfl, rfl⟩

/-- Sealing a frame on an error edge carries every duty and every destroy
obligation forward verbatim: a rethrow sheds nothing. Acceptance 3. -/
theorem sealing_carries_every_duty {state next : St} {frame : Frame} {rest : List Frame}
    (closed : state.opened = none) (frames : state.frames = frame :: rest)
    (checked : step state .sealFrame = some next) :
    next.opened = some
        { allowable := frame.allowable
          releases := frame.releases
          obligations := frame.obligations } ∧ next.frames = rest := by
  simp only [step, closed, frames] at checked
  cases checked
  exact ⟨rfl, rfl⟩

/-- One pack at a time: an error edge inside a handler that still holds a pack
is refused, so two packs can never be merged or one silently dropped. -/
theorem sealing_requires_no_open_pack {state : St} {pack : Pack}
    (opened : state.opened = some pack) : step state .sealFrame = none := by
  simp [step, opened]

-- ---------------------------------------------------------------------------
-- Non-vacuity: the admitted shape really is admitted
-- ---------------------------------------------------------------------------

/-- The card's deliverable, in the model: a region endowed with credit creates a
child with a loan and an endowment, the child completes having spent some, the
region raises, and the handler joins the child before discharging the pack. -/
def deliverable : List Op :=
  [ .endow 6,
    .create 1 30 4 [7] true,
    .spend 1,
    .complete 1 3,
    .sealFrame,
    .joinChild 1 30 [7],
    .discharge,
    .spend 7,
    .closeRegion ]

def start : St :=
  { frames := [{ allowable := 10, releases := [], obligations := [] }]
    children := [], opened := none, spent := 0, abandoned := 0
    joined := [], loaned := [] }

/-- The whole admitted run is accepted, and its accounting matches the Rust
certificate: nine spent, one abandoned, nothing left over. -/
theorem the_admitted_run_is_accepted_and_conserves_its_credit :
    (run start deliverable).map (fun s => (s.spent, s.abandoned, accounted s)) =
      some (9, 1, 10) := by
  decide +kernel

/-- Discharging before the join is refused, and the refusal is not an artefact
of the fixture: the deliverable with the join first is accepted above. -/
theorem discharging_before_the_join_is_refused :
    run start
      [ .endow 6, .create 1 30 4 [7] true, .spend 1, .complete 1 3, .sealFrame,
        .discharge ] = none := by
  decide +kernel

/-- A creation that failed leaves the run able to abandon its whole endowment,
because it opened no duty at all. -/
theorem a_failed_creation_leaves_nothing_to_wait_for :
    (run start
      [ .endow 6, .create 1 30 4 [7] false, .sealFrame, .discharge, .spend 4,
        .closeRegion ]).map (fun s => (s.spent, s.abandoned)) = some (4, 6) := by
  decide +kernel

/-- A normal region exit with a live join is refused, and with the join
performed it is accepted. -/
theorem a_normal_exit_with_a_live_join_is_refused :
    run start [ .endow 0, .create 1 30 0 [] true, .closeRegion ] = none ∧
      (run start
        [ .endow 0, .create 1 30 0 [] true, .joinChild 1 30 [], .closeRegion ]).isSome
        = true := by
  decide +kernel

/-- An outstanding read loan refuses the discharge, and the join that closes the
loan admits it. -/
theorem an_outstanding_loan_refuses_the_discharge :
    run start [ .endow 4, .create 1 30 0 [7] true, .sealFrame, .discharge ] = none ∧
      (run start
        [ .endow 4, .create 1 30 0 [7] true, .sealFrame, .joinChild 1 30 [7],
          .discharge ]).map St.abandoned = some 4 := by
  decide +kernel

end QCore.RegionTaskCredit
