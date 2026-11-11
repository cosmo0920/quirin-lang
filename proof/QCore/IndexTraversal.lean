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

import QCore.IndirectScalarAccess

/-!
# Index and pointer traversal (card W43-INDEX-TRAVERSAL)

The abstract rules implemented by `LegacyInstruction::PointerStep`,
`Derivation::step`, `Derivation::join`, `derived_release_is_accusable`, the
scalar-fact arms of `AbstractState::join`, the unbound-copy arm of `Alias`,
`Parser::condition_access` and `index_traversal::stepped_access` in
`crates/qcore-c-gateway`.

* **Interior derivation by arithmetic.** `P ± E` over a binding keeps its
  targets and is *derived* with an interior extent. A forward step by a
  positive literal is `forward`; any other step, and a join with the base
  identity, is `any` (it may sit at the object's start).
* **The derivation lattice.** The join is commutative, associative and
  idempotent over every derivation, including the pre-card ones, which keep
  their answers: an identity joined with an unstepped derived result is still
  `mixed`.
* **Soundness of the loop fixed point for these facts.** Against a concrete
  offset semantics under the no-undefined-behaviour reading (a pointer stays
  inside its object, so an offset is a natural number), the step and the join
  over-approximate every concrete move, so a post-fixed point of a loop's
  header over-approximates every concrete iteration count
  (`a_post_fixed_point_covers_every_iteration`). The lattice is finite, so the
  existing work list reaches one.
* **Custody.** A step changes no status. A stepped pointer is never an
  identity, so no release through it is a release of the base; releasing a
  forward-stepped pointer is accusable and never at the start, releasing one
  that may be at the start is not accusable. Stepping in place the only
  identity of a live object refuses.
* **Index facts.** A declared scalar's fact joins with "no fact" to "no fact",
  which admits every concrete value.
* **Conditions and stepped accesses.** A typed condition lowers to the typed
  read of `QCore.IndirectScalarAccess`, which changes no binding; a stepped
  access checks its obligations over the old values.

## What this is not

A model of the decision, as the gateway's other Lean modules are. Faithful C
token/type extraction and the Rust–Lean correspondence are review premises
(trust boundary 28). Nothing here decides bounds: an offset is only ever
compared with zero, and the premise that a pointer stays inside its object is
the no-undefined-behaviour reading constraint 10a already rests on.
-/

namespace QCore.IndexTraversal

/-! ## The derivation lattice -/

inductive Extent where
  | exact
  | interior
  deriving DecidableEq, Repr

/-- What pointer arithmetic did to a derived binding (`Stepped` in Rust). -/
inductive Stepped where
  | no
  | forward
  | any
  deriving DecidableEq, Repr

def Stepped.max : Stepped → Stepped → Stepped
  | .any, _ => .any
  | _, .any => .any
  | .forward, _ => .forward
  | _, .forward => .forward
  | .no, .no => .no

inductive Derivation where
  | identity
  | derived (extent : Extent) (constDropped : Bool) (stepped : Stepped)
  | mixed
  deriving DecidableEq, Repr

def isStepped : Stepped → Bool
  | .no => false
  | _ => true

/-- `Derivation::step`: the derivation of `P ± E`. -/
def step (d : Derivation) (forward : Bool) : Derivation :=
  let movement := if forward then Stepped.forward else Stepped.any
  match d with
  | .identity => .derived .interior false movement
  | .derived _ c s => .derived .interior c (s.max movement)
  | .mixed => .mixed

/-- `Derivation::join_derivations`. -/
def join : Derivation → Derivation → Derivation
  | .identity, .identity => .identity
  | .identity, .derived .interior c s =>
      if isStepped s then .derived .interior c .any else .mixed
  | .derived .interior c s, .identity =>
      if isStepped s then .derived .interior c .any else .mixed
  | .derived e c s, .derived e' c' s' =>
      if e = e' ∧ isStepped s = isStepped s' then .derived e (c || c') (s.max s')
      else .mixed
  | _, _ => .mixed

/-- Every derivation, for the finite checks below. -/
def all : List Derivation :=
  [.identity, .mixed] ++
    ([Extent.exact, Extent.interior].flatMap fun e =>
      [false, true].flatMap fun c =>
        [Stepped.no, Stepped.forward, Stepped.any].map fun s => .derived e c s)

theorem mem_all (d : Derivation) : d ∈ all := by
  cases d with
  | identity => decide
  | mixed => decide
  | derived e c s => cases e <;> cases c <;> cases s <;> decide

theorem join_comm_all : ∀ a ∈ all, ∀ b ∈ all, join a b = join b a := by decide

theorem join_idem_all : ∀ a ∈ all, join a a = a := by decide

theorem join_assoc_all :
    ∀ a ∈ all, ∀ b ∈ all, ∀ c ∈ all, join (join a b) c = join a (join b c) := by decide

theorem join_comm (a b : Derivation) : join a b = join b a :=
  join_comm_all a (mem_all a) b (mem_all b)

theorem join_idem (a : Derivation) : join a a = a :=
  join_idem_all a (mem_all a)

theorem join_assoc (a b c : Derivation) : join (join a b) c = join a (join b c) :=
  join_assoc_all a (mem_all a) b (mem_all b) c (mem_all c)

/-- `mixed` absorbs, so it is the lattice's top. -/
theorem mixed_absorbs (a : Derivation) : join a .mixed = .mixed := by
  have key : ∀ x ∈ all, join x .mixed = .mixed := by decide
  exact key a (mem_all a)

/-- The pre-card answers are kept: an identity and an unstepped derived result
join to `mixed`, whatever their extent and constness. -/
theorem an_unstepped_derived_result_still_joins_the_identity_to_mixed (e : Extent) (c : Bool) :
    join .identity (.derived e c .no) = .mixed := by
  cases e <;> cases c <;> rfl

/-- The loop header of `for (q = p; *q; q++)`: the base identity joined with a
stepped pointer is a stepped pointer that may be anywhere, the start included. -/
theorem the_header_joins_the_identity_and_its_step_to_any (c : Bool) :
    join .identity (step .identity true) = .derived .interior false .any ∧
      join .identity (.derived .interior c .forward) = .derived .interior c .any := by
  cases c <;> exact ⟨rfl, rfl⟩

theorem stepped_and_unstepped_families_stay_apart (c c' : Bool) (s : Stepped) (h : s ≠ .no) :
    join (.derived .interior c .no) (.derived .interior c' s) = .mixed := by
  cases s <;> cases c <;> cases c' <;> simp_all [join, isStepped]

/-! ## Arithmetic never yields an identity -/

theorem a_step_is_never_the_identity (d : Derivation) (f : Bool) : step d f ≠ .identity := by
  cases d <;> cases f <;> simp [step]

theorem a_step_of_a_non_mixed_binding_is_interior (d : Derivation) (f : Bool)
    (h : d ≠ .mixed) : ∃ c s, step d f = .derived .interior c s := by
  cases d with
  | identity => exact ⟨false, _, rfl⟩
  | derived e c s => exact ⟨c, _, rfl⟩
  | mixed => exact absurd rfl h

theorem a_backward_step_may_be_at_the_start (d : Derivation) (h : d ≠ .mixed) :
    ∃ c, step d false = .derived .interior c .any := by
  cases d with
  | identity => exact ⟨false, rfl⟩
  | derived e c s => cases s <;> exact ⟨c, rfl⟩
  | mixed => exact absurd rfl h

theorem const_dropped_survives_a_step (e : Extent) (s : Stepped) (f : Bool) :
    ∃ s', step (.derived e true s) f = .derived .interior true s' := ⟨_, rfl⟩

/-! ## Concrete offsets and soundness -/

/-- What a derivation commits to about its pointer's offset in its object,
measured from the object's start. Under the no-undefined-behaviour reading
every offset is a natural number (the pointer stays inside its object). An
identity is the start; a forward-stepped pointer is strictly after it; nothing
else is committed to. -/
inductive Kind where
  | start
  | afterStart
  | anywhere
  deriving DecidableEq, Repr

def kind : Derivation → Kind
  | .identity => .start
  | .derived _ _ .forward => .afterStart
  | _ => .anywhere

def admits (d : Derivation) (o : Nat) : Prop :=
  match kind d with
  | .start => o = 0
  | .afterStart => 0 < o
  | .anywhere => True

/-- `k` admits every offset `k'` admits. -/
def covers (k k' : Kind) : Bool := k == .anywhere || k == k'

theorem admits_of_covers (a b : Derivation) (h : covers (kind b) (kind a) = true) (o : Nat)
    (ha : admits a o) : admits b o := by
  unfold admits at *
  revert h ha
  cases kind a <;> cases kind b <;> simp [covers]

/-- The order the join induces. -/
def le (a b : Derivation) : Prop := join a b = b

theorem join_covers_all : ∀ a ∈ all, ∀ b ∈ all, covers (kind (join a b)) (kind a) = true := by
  decide

theorem join_admits_left (a b : Derivation) (o : Nat) (h : admits a o) :
    admits (join a b) o :=
  admits_of_covers a (join a b) (join_covers_all a (mem_all a) b (mem_all b)) o h

theorem join_admits_right (a b : Derivation) (o : Nat) (h : admits b o) :
    admits (join a b) o := by
  rw [join_comm]
  exact join_admits_left b a o h

theorem le_admits (a b : Derivation) (hle : le a b) (o : Nat) (h : admits a o) : admits b o := by
  unfold le at hle
  rw [← hle]
  exact join_admits_left a b o h

/-- A forward step by a positive amount stays admitted. -/
theorem a_forward_step_is_sound (d : Derivation) (o k : Nat) (hk : 0 < k)
    (h : admits d o) : admits (step d true) (o + k) := by
  cases d with
  | identity => simp [step, admits, kind]; omega
  | mixed => simp [step, admits, kind]
  | derived e c s => cases s <;> simp [step, Stepped.max, admits, kind] <;> omega

/-- Any other step lands anywhere in the object, and is admitted there. -/
theorem an_unknown_step_is_sound (d : Derivation) (o' : Nat) : admits (step d false) o' := by
  cases d with
  | identity => simp [step, admits, kind]
  | mixed => simp [step, admits, kind]
  | derived e c s => cases s <;> simp [step, Stepped.max, admits, kind]

/-- One concrete move of a loop body over the stepped pointer: a forward step
by a positive amount, an arbitrary step, or no move. -/
inductive Move where
  | forward (k : Nat) (hk : 0 < k)
  | anywhere (target : Nat)
  | stay

def Move.apply : Move → Nat → Nat
  | .forward k _, o => o + k
  | .anywhere target, _ => target
  | .stay, o => o

/-- The abstract body transfer for one move. -/
def Move.abstract : Move → Derivation → Derivation
  | .forward _ _, d => step d true
  | .anywhere _, d => step d false
  | .stay, d => d

theorem a_move_is_sound (m : Move) (d : Derivation) (o : Nat) (h : admits d o) :
    admits (m.abstract d) (m.apply o) := by
  cases m with
  | forward k hk => exact a_forward_step_is_sound d o k hk h
  | anywhere target => exact an_unknown_step_is_sound d target
  | stay => exact h

/-- From the header's own state: if every body move maps the header below
itself, every offset reached from an admitted header offset stays admitted. -/
theorem every_iteration_from_the_header_is_covered (h : Derivation)
    (hbody : ∀ m : Move, le (m.abstract h) h) (moves : List Move) (o : Nat)
    (ho : admits h o) : admits h (moves.foldl (fun offset m => m.apply offset) o) := by
  induction moves generalizing o with
  | nil => exact ho
  | cons m rest ih =>
      simp only [List.foldl]
      exact ih _ (le_admits _ h (hbody m) _ (a_move_is_sound m h o ho))

/-- **The loop fixed point is sound for the stepped facts.** If the header's
derivation `h` is a post-fixed point (the entry is below it and every body
move maps it below itself), then every concrete offset reached after any
number of iterations, from an admitted entry offset, is admitted by `h`. The
work list computes such an `h` by joining, and the lattice is finite. -/
theorem a_post_fixed_point_covers_every_iteration (entry h : Derivation)
    (hentry : le entry h) (hbody : ∀ m : Move, le (m.abstract h) h)
    (moves : List Move) (o : Nat) (ho : admits entry o) :
    admits h (moves.foldl (fun offset m => m.apply offset) o) :=
  every_iteration_from_the_header_is_covered h hbody moves o (le_admits entry h hentry o ho)

/-- The header the work list reaches for `for (q = p; *q; q++)` (and for any
body mixing forward and arbitrary steps) is a post-fixed point. -/
theorem the_walk_header_is_a_post_fixed_point :
    le .identity (.derived .interior false .any) ∧
      ∀ m : Move, le (m.abstract (.derived .interior false .any)) (.derived .interior false .any) := by
  refine ⟨rfl, ?_⟩
  intro m
  cases m <;> rfl

/-! ## Release: accused, refused, never credited -/

/-- `derived_release_is_accusable` on the derivation alone. -/
def accusable : Derivation → Bool
  | .derived .interior _ .no => true
  | .derived .interior _ .forward => true
  | _ => false

/-- A release credits the base only through an identity. -/
def credits : Derivation → Bool
  | .identity => true
  | _ => false

theorem releasing_a_stepped_pointer_never_releases_its_base (d : Derivation) (f : Bool) :
    credits (step d f) = false := by
  cases d <;> cases f <;> rfl

theorem a_forward_stepped_release_is_accusable_and_never_at_the_start (c : Bool) (o : Nat)
    (h : admits (.derived .interior c .forward) o) :
    accusable (.derived .interior c .forward) = true ∧ o ≠ 0 := by
  refine ⟨rfl, ?_⟩
  simp [admits, kind] at h
  omega

theorem a_pointer_that_may_be_at_the_start_is_never_accused (e : Extent) (c : Bool) :
    accusable (.derived e c .any) = false := by
  cases e <;> rfl

theorem the_walk_header_is_never_accused :
    accusable (join .identity (step .identity true)) = false := rfl

theorem a_mixed_binding_is_never_accused_or_credited :
    accusable .mixed = false ∧ credits .mixed = false := ⟨rfl, rfl⟩

/-! ## Stepping in place: statuses unchanged, the base never lost -/

structure Binding where
  derivation : Derivation
  targets : List Nat
  deriving DecidableEq, Repr

structure State where
  binding : Nat → Option Binding
  live : Nat → Bool
  /-- Whether a field link or a containment edge still holds the site. -/
  held : Nat → Bool

inductive Diagnostic where
  | unknownAlias (name : Nat)
  deriving DecidableEq, Repr

/-- Sites of `x`'s identity binding that no other identity binding, field link
or containment still names, and that are live. -/
def lost (s : State) (names : List Nat) (x : Nat) : List Nat :=
  match s.binding x with
  | some ⟨.identity, targets⟩ =>
      targets.filter fun site =>
        s.live site && !s.held site &&
          !names.any fun other =>
            other != x &&
              match s.binding other with
              | some ⟨.identity, others⟩ => others.contains site
              | _ => false
  | _ => []

/-- `PointerStep { destination = x, source = x }`: the new binding, and the
refusal when the step would lose a live object's only name. `names` lists the
bound names. -/
def stepInPlace (s : State) (names : List Nat) (x : Nat) (f : Bool) :
    State × List Diagnostic :=
  let binding := fun n =>
    if n = x then (s.binding x).map fun b => { b with derivation := step b.derivation f }
    else s.binding n
  ({ s with binding := binding },
    if (lost s names x).isEmpty then [] else [.unknownAlias x])

theorem a_step_changes_no_status (s : State) (names : List Nat) (x : Nat) (f : Bool) :
    (stepInPlace s names x f).1.live = s.live ∧ (stepInPlace s names x f).1.held = s.held :=
  ⟨rfl, rfl⟩

theorem a_step_keeps_its_targets (s : State) (names : List Nat) (x : Nat) (f : Bool)
    (b : Binding) (hb : s.binding x = some b) :
    (stepInPlace s names x f).1.binding x = some ⟨step b.derivation f, b.targets⟩ := by
  simp [stepInPlace, hb]

theorem an_unbound_pointer_stays_unbound (s : State) (names : List Nat) (x : Nat) (f : Bool)
    (hb : s.binding x = none) : (stepInPlace s names x f).1.binding x = none := by
  simp [stepInPlace, hb]

theorem losing_the_only_name_of_a_live_object_refuses (s : State) (names : List Nat) (x : Nat)
    (f : Bool) (h : lost s names x ≠ []) :
    (stepInPlace s names x f).2 = [.unknownAlias x] := by
  simp [stepInPlace]
  intro hnil
  exact absurd hnil h

theorem a_step_that_loses_nothing_adds_no_diagnostic (s : State) (names : List Nat) (x : Nat)
    (f : Bool) (h : lost s names x = []) : (stepInPlace s names x f).2 = [] := by
  simp [stepInPlace, h]

/-- The witness: `p = malloc(n); p++;` with nothing else naming the object. -/
def onlyName : State :=
  { binding := fun n => if n = 0 then some ⟨.identity, [7]⟩ else none
    live := fun site => site = 7
    held := fun _ => false }

/-- The same object also named by `q` (`q = p; p++;` or `q = p; q++;`). -/
def twoNames : State :=
  { onlyName with binding := fun n => if n = 0 ∨ n = 1 then some ⟨.identity, [7]⟩ else none }

theorem stepping_the_only_name_refuses :
    (stepInPlace onlyName [0] 0 true).2 = [.unknownAlias 0] := by decide

theorem stepping_a_copy_is_admitted :
    (stepInPlace twoNames [0, 1] 1 true).2 = [] := by decide

/-! ## Index facts -/

/-- A declared scalar's binding carries at most the null constant's "zero". -/
inductive ScalarFact where
  | zero
  deriving DecidableEq, Repr

def scalarAdmits : Option ScalarFact → Int → Prop
  | some .zero, v => v = 0
  | none, _ => True

/-- The scalar arm of `AbstractState::join`: one path without a fact makes the
join factless. -/
def scalarJoin : Option ScalarFact → Option ScalarFact → Option ScalarFact
  | some a, some b => if a = b then some a else none
  | _, _ => none

theorem a_one_sided_scalar_fact_joins_to_no_fact (f : ScalarFact) :
    scalarJoin (some f) none = none ∧ scalarJoin none (some f) = none := ⟨rfl, rfl⟩

theorem the_scalar_join_is_sound (a b : Option ScalarFact) (v : Int)
    (h : scalarAdmits a v ∨ scalarAdmits b v) : scalarAdmits (scalarJoin a b) v := by
  cases a with
  | none => trivial
  | some x =>
      cases b with
      | none => trivial
      | some y => cases x; cases y; simpa [scalarJoin, scalarAdmits] using h

/-- No fact admits the value a loop counter reaches, whatever it is. -/
theorem no_fact_admits_every_count (v : Int) : scalarAdmits none v := trivial

/-! ## Copies of unbound names -/

/-- The `Alias` arm for two declared scalars or two declared pointer objects:
an unbound source gives an unbound destination and no diagnostic; a bound
source is copied. `refusal` is the pre-card answer for an unbound source. -/
def copy (typed : Bool) (source : Option Binding) : Option Binding × Bool :=
  match source with
  | some b => (some b, false)
  | none => if typed then (none, false) else (none, true)

theorem an_unbound_typed_copy_is_unbound_and_admitted :
    copy true none = (none, false) := rfl

theorem an_untyped_unbound_copy_keeps_its_refusal : (copy false none).2 = true := rfl

theorem a_bound_copy_is_unchanged (typed : Bool) (b : Binding) :
    copy typed (some b) = (some b, false) := rfl

/-! ## Conditions and stepped accesses -/

open QCore.IndirectScalarAccess in
/-- A typed condition is the typed read: no destination and no frame, so every
binding is kept. -/
theorem a_typed_condition_changes_no_binding (s : QCore.IndirectScalarAccess.State)
    (bases : List Nat) :
    (QCore.IndirectScalarAccess.transfer s ⟨bases, none, none⟩).1.binding = s.binding := by
  funext name
  exact without_a_frame_only_the_destination_changes s ⟨bases, none, none⟩ name rfl (by simp)

open QCore.IndirectScalarAccess in
/-- Its obligations are the read's: an admitted condition adds nothing, and a
refused base leaves a reason. -/
theorem a_typed_condition_keeps_its_obligations (s : QCore.IndirectScalarAccess.State)
    (bases : List Nat) (base : Nat) (hmem : base ∈ bases)
    (hr : resolves s base = false) :
    safe (QCore.IndirectScalarAccess.transfer s ⟨bases, none, none⟩).2 = false :=
  a_refusal_is_never_safe s ⟨bases, none, none⟩ base hmem hr

/-- The stepped-access recogniser's syntactic premises, as booleans. -/
structure SteppedCandidate where
  typedAccess : Bool
  steppedNameElsewhere : Bool
  steppedTwice : Bool
  indexWritesOtherwise : Bool
  hasStep : Bool
  deriving DecidableEq, Repr

def admitsStepped (c : SteppedCandidate) : Bool :=
  c.typedAccess && !c.steppedNameElsewhere && !c.steppedTwice && !c.indexWritesOtherwise &&
    c.hasStep

theorem an_unsequenced_stepped_name_refuses (c : SteppedCandidate)
    (h : c.steppedNameElsewhere = true) : admitsStepped c = false := by
  simp [admitsStepped, h]

theorem a_double_step_refuses (c : SteppedCandidate) (h : c.steppedTwice = true) :
    admitsStepped c = false := by
  simp [admitsStepped, h]

/-- The repaired hole: an index that writes in any other shape refuses, so
`out[m += 1] = c` never keeps `m`'s old fact. -/
theorem an_index_write_outside_the_shape_refuses (c : SteppedCandidate)
    (h : c.indexWritesOtherwise = true) : admitsStepped c = false := by
  simp [admitsStepped, h]

/-- The lowering order: the access over the old values, then each step. The
access's obligation is therefore checked on the pre-step binding. -/
def lower (access : α) (steps : List α) : List α := access :: steps

theorem the_access_precedes_its_steps (access : α) (steps : List α) :
    (lower access steps).head? = some access := rfl

theorem a_full_stepped_candidate_is_admitted :
    admitsStepped ⟨true, false, false, false, true⟩ = true := by decide

end QCore.IndexTraversal

namespace QCore.IndexTraversal

/-! W48 bottom-up batch 2. The formal slot is a by-value C parameter.
The caller separately holds its own slot; the exact AST use classifier
certifies only scalar dereference reads and self-steps whose pointer result
is discarded. This extraction
predicate and Rust correspondence remain premises, as in the base module. -/

def formalStepEligible (scalarPointer fixed readOnly : Bool) : Bool :=
  scalarPointer && !fixed && readOnly

structure FormalState where
  localState : State
  callerSlots : Nat → Option Binding

def checkedFormalStep (scalarPointer fixed readOnly : Bool) (s : FormalState)
    (names : List Nat) (x : Nat) (forward : Bool) : Option (FormalState × List Diagnostic) :=
  if formalStepEligible scalarPointer fixed readOnly then
    let result := stepInPlace s.localState names x forward
    some ({ s with localState := result.1 }, result.2)
  else none

theorem a_fixed_formal_keeps_the_refusal (typed reads : Bool) (s : FormalState)
    (names : List Nat) (x : Nat) (f : Bool) :
    checkedFormalStep typed true reads s names x f = none := by
  simp [checkedFormalStep, formalStepEligible]

theorem an_exposed_formal_keeps_the_refusal (typed fixed : Bool) (s : FormalState)
    (names : List Nat) (x : Nat) (f : Bool) :
    checkedFormalStep typed fixed false s names x f = none := by
  simp [checkedFormalStep, formalStepEligible]

theorem a_checked_formal_step_preserves_custody_and_caller_slots
    (typed fixed reads : Bool) (s : FormalState) (names : List Nat)
    (x : Nat) (f : Bool) (result : FormalState × List Diagnostic)
    (h : checkedFormalStep typed fixed reads s names x f = some result) :
    result.1.localState.live = s.localState.live ∧
    result.1.localState.held = s.localState.held ∧ result.1.callerSlots = s.callerSlots := by
  unfold checkedFormalStep at h
  split at h
  · cases Option.some.inj h
    exact ⟨rfl, rfl, rfl⟩
  · contradiction

theorem a_read_only_unbound_formal_step_is_admitted (s : FormalState)
    (names : List Nat) (x : Nat) (f : Bool) (h : s.localState.binding x = none) :
    ∃ result, checkedFormalStep true false true s names x f = some (result, []) ∧
      result.localState.binding x = none := by
  refine ⟨{ s with localState := (stepInPlace s.localState names x f).1 }, ?_, ?_⟩
  · simp [checkedFormalStep, formalStepEligible, stepInPlace, lost, h]
  · exact an_unbound_pointer_stays_unbound s.localState names x f h

def formalStepVectors : List ((Bool × Bool × Bool) × Bool) :=
  [((false, false, false), false), ((false, false, true), false),
   ((false, true, false), false), ((false, true, true), false),
   ((true, false, false), false), ((true, false, true), true),
   ((true, true, false), false), ((true, true, true), false)]

theorem formal_step_vectors_hold : ∀ row ∈ formalStepVectors,
    formalStepEligible row.1.1 row.1.2.1 row.1.2.2 = row.2 := by decide

end QCore.IndexTraversal
