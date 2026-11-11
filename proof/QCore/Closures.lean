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

import QCore.Transferability

/-!
# Closures: an explicit capture list lifted into a hidden named worker

Card **W48-QR-CLOSURES** (card 4 of the `.qr` language track; implemented
grammars §G02a and §G02v, language specification §L16 and §L21). A closure

```text
spawn SCOPE |CAPTURES| -> TYPE [effects { … }] { BODY }
```

is lifted by the parser into a hidden worker declared before the function that
spawns it: one parameter per capture, the written result and effect row, the
closure's body. The spawn names that worker with the written captures. A
capture list spells modes, not types, so the task route decides each
parameter's type at the spawn site from the captured place
(`crates/qcore-frontend/src/source_tasks.rs`, `Builder::closure_parameter`),
and then lowers every function exactly as it lowers a hand-written twin. The
models below are transcriptions of that Rust, bound to it by the test rows of
`compiler/tests/closures.rs`, not by extraction.

## What is proved

* **Capture admission equals the transfer judgment.** The parameter a closure's
  capture binds (`closureKind`, `closureParameter`) is the only parameter the
  judgment could admit for that place and mode
  (`theClosureParameterIsTheOnlyAdmissibleOne`,
  `aRecordCaptureAdmitsOnlyItsOwnRecordType`); so a closure capture is admitted,
  with a transfer `t`, exactly when some hand-written parameter is
  (`aClosureCaptureIsAdmittedExactlyWhenSomeTwinIs`), its admission is the
  kind-level judgment itself (`aClosureCaptureIsDecidedByTheKindJudgment`), and
  its refusal is the judgment's own reason for the kind and mode — never a
  mismatched parameter (`aClosureRefusalIsTheJudgmentsOwnReason`).
* **No implicit capture.** The hidden worker's body is resolved in a scope made
  of the capture binders and the module's globals; the enclosing scope does
  not enter it (`theBodyScopeIgnoresTheEnclosingScope`). A body that resolves
  uses no outer name but a capture binder
  (`theBodysFreeOuterNamesAreExactlyTheCaptureBinders`), and a body using an
  outer name the list does not name is refused, naming an unresolved name
  (`anUncapturedOuterNameIsRefused`, `aRefusalNamesAnUnresolvedName`).
* **The desugaring preserves the task IR.** Lowering a spawn against the
  parameters its spawn site decides gives the same transfers and the same
  worker IR as lowering it against any hand-written twin's parameters that
  check (`desugaringPreservesTheTaskIR`), and a closure that is refused is
  refused for every twin (`aRefusedClosureHasNoCheckingTwin`).
* **The hidden name cannot collide.** It contains `#`, which no source
  identifier does (`aHiddenNameIsNoSourceIdentifier`).

## What is not

Nothing here is a data-race proof, a native-refinement claim or a statement
about the emitted C; the twin identity of the emitted C and certificates is
the byte comparison in `compiler/tests/closures.rs`, separately. The mutex
route's `share … under M` is a scope clause, not a capture, and closures are
not lowered on that route (`W48-QR-CLOSURE-MUTEX`), so the sharing mode is
outside the closure modes below.
-/

namespace QCore.Closures

open QCore.Transferability

/-! ## The parameter a capture binds -/

/-- The capture modes a closure's list spells: `copy`, `move`, `read … share`
(`lend` here), `lend … LOW..HIGH` (`lendWrite`) and the refused `write`. -/
def closureModes : List Mode := [.copy, .move, .lend, .lendWrite, .write]

/-- `Builder::closure_parameter` over the flat kinds: the captured place's own
kind, except that a read share and a range lease of an owned Buffer deliver its
loan and its lease. -/
def closureKind (place : Kind) : Mode → Kind
  | .lend => if place = .ownedBuffer then .readLoan else place
  | .lendWrite => if place = .ownedBuffer then .writeLease else place
  | _ => place

/-- Over the union of the flat and aggregate vocabularies: a record binds a
parameter of its own record type. -/
def closureParameter : Crossing → Mode → Crossing
  | .kind k, m => .kind (closureKind k m)
  | .record id leaves, _ => .record id leaves

set_option maxRecDepth 16000 in
/-- The flat cells, decided: a parameter the judgment admits is the closure's. -/
theorem flatAdmissionNamesTheClosureKind :
    ∀ k ∈ allKinds, ∀ m ∈ closureModes, ∀ q ∈ allKinds,
      (admitsCapture k m q).isSome → q = closureKind k m := by decide

/-- The flat cells, decided: the closure's parameter meets the kind judgment
itself. -/
theorem flatClosureAdmissionIsTheKindJudgment :
    ∀ k ∈ allKinds, ∀ m ∈ closureModes, admitsCapture k m (closureKind k m) = admits k m := by
  decide

/-- The flat cells, decided: the closure's refusal is the kind and mode's own. -/
theorem flatClosureRefusalIsTheKindsOwn :
    ∀ k ∈ allKinds, ∀ m ∈ closureModes, refuseCapture k m (closureKind k m) = refuse k m := by
  decide

/-- **Uniqueness.** For a flat kind in a closure mode, the parameter a closure
binds is the only parameter the judgment admits — a hand-written twin that
checks declares exactly it. -/
theorem theClosureParameterIsTheOnlyAdmissibleOne (k : Kind) (m : Mode) (q : Kind) (t : Transfer)
    (hm : m ∈ closureModes) (admitted : admitsCapture k m q = some t) : q = closureKind k m :=
  flatAdmissionNamesTheClosureKind k (kindInventoryIsComplete k) m hm q
    (kindInventoryIsComplete q) (by simp [admitted])

/-- **Uniqueness, records.** A record capture is admitted only into its own
record type — the closure's parameter. -/
theorem aRecordCaptureAdmitsOnlyItsOwnRecordType (p : Nat) (leaves : List Kind) (m : Mode)
    (q : Nat) (t : Transfer) (admitted : admitsAggregateCapture p leaves m q = some t) : q = p := by
  unfold admitsAggregateCapture at admitted
  cases h : admitsAggregate leaves m with
  | none => simp [h] at admitted
  | some _ =>
    by_cases same : p = q
    · exact same.symm
    · simp [h, same] at admitted

/-- **Capture admission equals the transfer judgment**: over both
vocabularies, the closure's parameter admits a capture with transfer `t`
exactly when some hand-written parameter does. -/
theorem aClosureCaptureIsAdmittedExactlyWhenSomeTwinIs (place : Crossing) (m : Mode)
    (t : Transfer) (hm : m ∈ closureModes) :
    admitsCrossing place m (closureParameter place m) = some t ↔
      ∃ parameter, admitsCrossing place m parameter = some t := by
  constructor
  · intro admitted
    exact ⟨_, admitted⟩
  · rintro ⟨parameter, admitted⟩
    cases place with
    | kind k =>
      cases parameter with
      | kind q =>
        have same := theClosureParameterIsTheOnlyAdmissibleOne k m q t hm admitted
        subst same
        exact admitted
      | record _ _ => simp [admitsCrossing] at admitted
    | record p leaves =>
      cases parameter with
      | kind _ => simp [admitsCrossing] at admitted
      | record q other =>
        have same := aRecordCaptureAdmitsOnlyItsOwnRecordType p leaves m q t admitted
        subst same
        simpa [admitsCrossing, closureParameter] using admitted

/-- A flat closure capture is decided by the kind-level judgment itself: what
it admits, it admits with the kind judgment's transfer. -/
theorem aClosureCaptureIsDecidedByTheKindJudgment (k : Kind) (m : Mode)
    (hm : m ∈ closureModes) :
    admitsCrossing (.kind k) m (closureParameter (.kind k) m) = admits k m := by
  simpa [admitsCrossing, closureParameter] using
    flatClosureAdmissionIsTheKindJudgment k (kindInventoryIsComplete k) m hm

/-- A closure's flat refusal is the judgment's own reason for the kind and the
mode — a closure is never refused for a mismatched parameter, since it never
declares one. -/
theorem aClosureRefusalIsTheJudgmentsOwnReason (k : Kind) (m : Mode) (hm : m ∈ closureModes) :
    refuseCrossing (.kind k) m (closureParameter (.kind k) m) = (refuse k m).map .flat := by
  simp [refuseCrossing, closureParameter,
    flatClosureRefusalIsTheKindsOwn k (kindInventoryIsComplete k) m hm]

/-- A write capture in a closure is refused by name, as in any capture list. -/
theorem aClosureWriteCaptureIsRefusedByName (k : Kind) :
    refuseCrossing (.kind k) .write (closureParameter (.kind k) .write) =
      some (.flat .writeCaptureUnimplemented) := by
  simp [refuseCrossing, closureParameter, closureKind, refuseCapture, admits, refuse]

/-! ## No implicit capture -/

/-- The scope the hidden worker's body is resolved in: the capture binders and
the module's globals. The enclosing scope is an argument only so that its
absence can be stated. -/
def bodyScope (binders globals : List String) (_enclosing : List String) : List String :=
  binders ++ globals

/-- Name resolution over a statement's used names, in order: the first name the
scope does not hold is the refusal. -/
def resolve (scope : List String) : List String → Option String
  | [] => none
  | name :: rest => if name ∈ scope then resolve scope rest else some name

theorem resolve_none_iff (scope used : List String) :
    resolve scope used = none ↔ ∀ name ∈ used, name ∈ scope := by
  induction used with
  | nil => simp [resolve]
  | cons name rest ih =>
    by_cases held : name ∈ scope
    · simp [resolve, held, ih]
    · simp [resolve, held]

/-- A refusal names a used name that the scope does not hold. -/
theorem aRefusalNamesAnUnresolvedName (scope used : List String) (name : String)
    (refused : resolve scope used = some name) : name ∈ used ∧ name ∉ scope := by
  induction used with
  | nil => simp [resolve] at refused
  | cons first rest ih =>
    by_cases held : first ∈ scope
    · simp only [resolve, held, ite_true] at refused
      exact ⟨List.mem_cons_of_mem _ (ih refused).1, (ih refused).2⟩
    · simp only [resolve, held, ite_false, Option.some.injEq] at refused
      subst refused
      exact ⟨List.mem_cons_self .., held⟩

/-- **The enclosing scope does not enter the body**: resolution is the same
whatever the enclosing function holds at the spawn site. -/
theorem theBodyScopeIgnoresTheEnclosingScope (binders globals outer other used : List String) :
    resolve (bodyScope binders globals outer) used =
      resolve (bodyScope binders globals other) used := by
  simp [bodyScope]

/-- **No implicit capture**: when a closure's body resolves, every name it uses
that is not a module global is a capture binder — the body's free outer names
are exactly among the capture list's. -/
theorem theBodysFreeOuterNamesAreExactlyTheCaptureBinders (binders globals outer used : List String)
    (resolved : resolve (bodyScope binders globals outer) used = none) :
    ∀ name ∈ used, name ∉ globals → name ∈ binders := by
  intro name used_name not_global
  have held := (resolve_none_iff _ used).1 resolved name used_name
  simp [bodyScope] at held
  rcases held with binder | global
  · exact binder
  · exact absurd global not_global

/-- An outer name the capture list does not name is refused, even though the
enclosing scope holds it. -/
theorem anUncapturedOuterNameIsRefused (binders globals outer used : List String) (name : String)
    (_visible : name ∈ outer) (used_name : name ∈ used) (not_binder : name ∉ binders)
    (not_global : name ∉ globals) :
    resolve (bodyScope binders globals outer) used ≠ none := by
  intro resolved
  exact not_binder
    (theBodysFreeOuterNamesAreExactlyTheCaptureBinders binders globals outer used resolved
      name used_name not_global)

/-! ## The desugaring preserves the task IR -/

/-- One capture at its spawn site: the mode written and the captured place's
kind there. -/
structure Capture where
  mode : Mode
  place : Crossing

/-- Lowering a spawn against a worker's parameter list: each capture decided by
the judgment against its parameter, in order, with exact arity. The spawn's IR
is the list of transfers. -/
def lowerSpawn : List Capture → List Crossing → Option (List Transfer)
  | [], [] => some []
  | capture :: captures, parameter :: parameters =>
    match admitsCrossing capture.place capture.mode parameter, lowerSpawn captures parameters with
    | some t, some ts => some (t :: ts)
    | _, _ => none
  | _, _ => none

/-- The closure's parameters, decided at its spawn site. -/
def discover (captures : List Capture) : List Crossing :=
  captures.map (fun capture => closureParameter capture.place capture.mode)

/-- A module of one spawn and its worker: the spawn's transfers, and the worker
lowered from its parameters and its body (by any body lowering). -/
def lowerModule {Body IR : Type} (lowerBody : List Crossing → Body → IR)
    (captures : List Capture) (parameters : List Crossing) (body : Body) :
    Option (List Transfer × IR) :=
  (lowerSpawn captures parameters).map (fun ts => (ts, lowerBody parameters body))

/-- Record types are nominal: one record identity names one leaf list. The
premise the twin's declared record parameters meet, because a record type's
leaves are its declaration's. -/
def Nominal (captures : List Capture) (parameters : List Crossing) : Prop :=
  ∀ capture ∈ captures, ∀ parameter ∈ parameters,
    ∀ p leaves q other, capture.place = .record p leaves → parameter = .record q other →
      p = q → leaves = other

/-- Every capture is written in a closure mode. -/
def ClosureModes (captures : List Capture) : Prop :=
  ∀ capture ∈ captures, capture.mode ∈ closureModes

/-- A parameter list that checks against the captures is the discovered one. -/
theorem aCheckingParameterListIsTheDiscoveredOne (captures : List Capture)
    (parameters : List Crossing) (ts : List Transfer) (modes : ClosureModes captures)
    (nominal : Nominal captures parameters)
    (checked : lowerSpawn captures parameters = some ts) : parameters = discover captures := by
  induction captures generalizing parameters ts with
  | nil =>
    cases parameters with
    | nil => rfl
    | cons _ _ => simp [lowerSpawn] at checked
  | cons capture rest ih =>
    cases parameters with
    | nil => simp [lowerSpawn] at checked
    | cons parameter parameters =>
      simp only [lowerSpawn] at checked
      cases admitted : admitsCrossing capture.place capture.mode parameter with
      | none => simp [admitted] at checked
      | some t =>
        cases tail : lowerSpawn rest parameters with
        | none => simp [admitted, tail] at checked
        | some ts' =>
          have head : parameter = closureParameter capture.place capture.mode := by
            have mode := modes capture (List.mem_cons_self ..)
            cases hp : capture.place with
            | kind k =>
              rw [hp] at admitted
              cases parameter with
              | kind q =>
                simp [closureParameter,
                  theClosureParameterIsTheOnlyAdmissibleOne k capture.mode q t mode
                    (by simpa [admitsCrossing] using admitted)]
              | record _ _ => simp [admitsCrossing] at admitted
            | record p leaves =>
              rw [hp] at admitted
              cases parameter with
              | kind _ => simp [admitsCrossing] at admitted
              | record q other =>
                have same := aRecordCaptureAdmitsOnlyItsOwnRecordType p leaves capture.mode q t
                  (by simpa [admitsCrossing] using admitted)
                have leaves_same := nominal capture (List.mem_cons_self ..) (.record q other)
                  (List.mem_cons_self ..) p leaves q other hp rfl same.symm
                subst same
                subst leaves_same
                rfl
          have rest_same := ih parameters ts'
            (fun c hc => modes c (List.mem_cons_of_mem _ hc))
            (fun c hc q hq => nominal c (List.mem_cons_of_mem _ hc) q (List.mem_cons_of_mem _ hq))
            tail
          simp [discover, head, rest_same]

/-- **The desugaring preserves the task IR**: whenever a hand-written twin's
parameters check, lowering the closure — its parameters decided at the spawn
site — gives the same transfers and the same worker IR. -/
theorem desugaringPreservesTheTaskIR {Body IR : Type} (lowerBody : List Crossing → Body → IR)
    (captures : List Capture) (parameters : List Crossing) (body : Body)
    (r : List Transfer × IR) (modes : ClosureModes captures)
    (nominal : Nominal captures parameters)
    (twin : lowerModule lowerBody captures parameters body = some r) :
    lowerModule lowerBody captures (discover captures) body = some r := by
  unfold lowerModule at twin
  cases checked : lowerSpawn captures parameters with
  | none => simp [checked] at twin
  | some ts =>
    have same := aCheckingParameterListIsTheDiscoveredOne captures parameters ts modes nominal
      checked
    subst same
    simpa [lowerModule, checked] using twin

/-- A closure that is refused has no hand-written twin that checks. -/
theorem aRefusedClosureHasNoCheckingTwin {Body IR : Type} (lowerBody : List Crossing → Body → IR)
    (captures : List Capture) (body : Body) (modes : ClosureModes captures)
    (refused : lowerModule lowerBody captures (discover captures) body = none)
    (parameters : List Crossing) (nominal : Nominal captures parameters) :
    lowerModule lowerBody captures parameters body = none := by
  cases twin : lowerModule lowerBody captures parameters body with
  | none => rfl
  | some r =>
    have lifted := desugaringPreservesTheTaskIR lowerBody captures parameters body r modes
      nominal twin
    rw [refused] at lifted
    exact absurd lifted (by simp)

/-! ## The hidden name -/

/-- A source identifier's characters: letters, digits and `_`. -/
def IsIdentifier (name : List Char) : Prop :=
  ∀ c ∈ name, c.isAlphanum = true ∨ c = '_'

/-- The hidden worker's name: `ENCLOSING#closureN`. -/
def hiddenName (enclosing : List Char) (ordinal : Nat) : List Char :=
  enclosing ++ '#' :: ("closure".toList ++ (toString ordinal).toList)

/-- **No collision**: the hidden name contains `#`, so it is no source
identifier and no declaration a reader writes can take it. -/
theorem aHiddenNameIsNoSourceIdentifier (enclosing : List Char) (ordinal : Nat) :
    ¬ IsIdentifier (hiddenName enclosing ordinal) := by
  intro identifier
  have hash := identifier '#' (by simp [hiddenName])
  simp at hash

/-- Non-vacuity, decided: the end-to-end example's three captures — a scalar
copied, a record that owns moved, an owned Buffer leased — bind an `i32`, the
record and a lease, and each is admitted with the transfer a twin gets. -/
theorem theExampleCapturesBindWhatTheTwinDeclares :
    discover [⟨.copy, .kind .i32⟩, ⟨.move, .record 0 [.i32, .ownedBuffer]⟩,
        ⟨.lendWrite, .kind .ownedBuffer⟩] =
      [.kind .i32, .record 0 [.i32, .ownedBuffer], .kind .writeLease] ∧
    lowerSpawn [⟨.copy, .kind .i32⟩, ⟨.move, .record 0 [.i32, .ownedBuffer]⟩,
        ⟨.lendWrite, .kind .ownedBuffer⟩]
        [.kind .i32, .record 0 [.i32, .ownedBuffer], .kind .writeLease] =
      some [.duplicate, .moveCustody, .lendWriteRange .disjointWriteLeases] := by
  decide

end QCore.Closures
