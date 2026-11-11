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

import QCore.LoopOwner

namespace QCore.LoopOwner.CoroPlugin

open QCore.LoopOwner

/-! Card **W47-PREMISES-CORO-PLUGIN** (gateway rule 10p's amendment): three
premises of the loop-owner census derived from the program instead of stated.

**P-CORO-HANDLE.** The census derives a coroutine entry's resumers from its
constructor sites. That is the whole set of first-segment resumers only when
the input coroutine's handle is confined to the coroutine's own functions.
A handle use that leaves the set (a foreign read, a copy passed on, a list
read through) may put a foreign resumer in front of the body; the census then
adds an undetermined resumer, so the body never discharges
(`an_escaping_handle_never_discharges`); a discharge under the census's
resumers is a discharge under every resumer the program may use
(`a_census_discharge_covers_every_possible_resumer`).

**P-PLUGIN-DIR.** A directory's descriptor flags decide a registrant only when
no descriptor's flags are written at run time and the registrant's instance
is derived from that directory's descriptor callbacks through a parameter
chain; anything else keeps the coroutine route
(`a_runtime_flags_write_keeps_the_route`,
`a_registration_on_another_plugins_instance_keeps_the_route`); a parameter
chain keeps the directory it starts from
(`a_parameter_chain_keeps_the_directory`).

**The immutable threaded flag.** A collector is attached to the loop its
instance's flag selects when it is created; with no write after start the
flag read at any later point is the one read at start
(`no_write_after_start_fixes_the_flag`). A run-time write leaves every
mode-fixing edge's callee unresolved (`a_runtime_threaded_write_never_discharges`);
a setter call whose key is another literal cannot reach the threaded arm
(`another_literal_key_cannot_write_the_flag`).

The old test's path stays refused whatever the premises derive
(`the_old_test_path_is_never_discharged_whatever_the_premises`). -/

/-! ## P-CORO-HANDLE -/

/-- One use of an input coroutine's handle (or of the collector it runs): in
a function of the coroutine's own closed set or not, and whether the value
leaves (passed to an unlisted call or stored anywhere but a handle member). -/
structure HandleUse where
  inClosedSet : Bool
  passesOut : Bool
  deriving DecidableEq, Repr

def handleConfined (uses : List HandleUse) : Bool :=
  uses.all fun u => u.inClosedSet && !u.passesOut

/-- The resumers the census uses: the derived ones, and an undetermined one
when the handle is not confined (the entry is left unresolved). -/
def censusResumers (derived : List Context) (uses : List HandleUse) : List Resumer :=
  derived.map .derived ++ (if handleConfined uses then [] else [.undetermined])

/-- The resumers the program may use for the first segment: a foreign
resumer is possible only through a use that leaves the closed set. -/
def possibleResumers (derived foreign : List Context) (uses : List HandleUse) :
    List Resumer :=
  derived.map .derived ++ (if handleConfined uses then [] else foreign.map .derived)

theorem a_confined_handle_has_only_derived_resumers {uses : List HandleUse}
    (derived foreign : List Context) (h : handleConfined uses = true) :
    possibleResumers derived foreign uses = derived.map .derived := by
  simp [possibleResumers, h]

/-- An escaping use leaves the coroutine's operations undischarged. -/
theorem an_escaping_handle_never_discharges {uses : List HandleUse}
    (owners : Owners) (derived : List Context) (h : handleConfined uses = false) :
    coroutineVerdict owners (censusResumers derived uses) ≠ .discharged := by
  apply an_undetermined_resumer_never_discharges
  simp [censusResumers, h]

/-- A use outside the closed set is an escape. -/
theorem a_use_outside_the_set_escapes {uses : List HandleUse} {u : HandleUse}
    (hu : u ∈ uses) (hout : u.inClosedSet = false) : handleConfined uses = false := by
  unfold handleConfined
  rw [Bool.eq_false_iff]
  intro hall
  have := List.all_eq_true.mp hall u hu
  simp [hout] at this

/-- A census discharge is a discharge under every resumer the program may
use: it needs a confined handle, and then both resumer lists are the derived
ones. -/
theorem a_census_discharge_covers_every_possible_resumer {owners : Owners}
    {derived foreign : List Context} {uses : List HandleUse}
    (h : coroutineVerdict owners (censusResumers derived uses) = .discharged) :
    coroutineVerdict owners (possibleResumers derived foreign uses) = .discharged := by
  cases hc : handleConfined uses with
  | false => exact absurd h (an_escaping_handle_never_discharges owners derived hc)
  | true =>
    have hcensus : censusResumers derived uses = derived.map .derived := by
      simp [censusResumers, hc]
    rw [a_confined_handle_has_only_derived_resumers derived foreign hc]
    rw [hcensus] at h
    exact h

/-! ## P-PLUGIN-DIR -/

/-- A write of a descriptor's flags: into a fresh allocation before it is
named anywhere else, or at run time. -/
inductive FlagsWrite where
  | fresh
  | runtime
  deriving DecidableEq, Repr

def flagsFixed (writes : List FlagsWrite) : Bool := writes.all (· == .fresh)

/-- Where a registration's instance comes from: a descriptor callback of a
directory, a parameter passed on by a caller, or anything else. -/
inductive Source (δ : Type) where
  | entry (dir : δ)
  | param (caller : Source δ)
  | other
  deriving Repr

def sourceDir {δ : Type} : Source δ → Option δ
  | .entry d => some d
  | .param s => sourceDir s
  | .other => none

theorem a_parameter_chain_keeps_the_directory {δ : Type} (s : Source δ) :
    sourceDir (.param s) = sourceDir s := rfl

def registrationDerived {δ : Type} [DecidableEq δ] (dir : δ) (sources : List (Source δ)) :
    Bool :=
  sources.all fun s => sourceDir s == some dir

/-- The coroutine flag a directory gives a registrant: its descriptors' flag,
only when no flags are written at run time and every registration's instance
is derived from that directory. -/
def directoryFlag {δ : Type} [DecidableEq δ] (writes : List FlagsWrite) (dir : δ)
    (sources : List (Source δ)) (descriptors : Option Bool) : Option Bool :=
  if flagsFixed writes && registrationDerived dir sources then descriptors else none

theorem a_runtime_flags_write_keeps_the_route {α δ : Type} [DecidableEq δ]
    {writes : List FlagsWrite} (hw : FlagsWrite.runtime ∈ writes)
    {dir : α → δ} {sources : α → List (Source δ)} {descriptors : α → Option Bool}
    {registrants : List α} {r : α} (hr : r ∈ registrants) :
    r ∈ coroutineDispatched
      (fun x => directoryFlag writes (dir x) (sources x) (descriptors x)) registrants := by
  have hf : flagsFixed writes = false := by
    unfold flagsFixed
    rw [Bool.eq_false_iff]
    intro hall
    have := List.all_eq_true.mp hall _ hw
    simp at this
  apply an_underived_flag_keeps_the_hop _ hr
  simp [directoryFlag, hf]

theorem a_registration_on_another_plugins_instance_keeps_the_route {α δ : Type}
    [DecidableEq δ] {writes : List FlagsWrite} {dir : α → δ}
    {sources : α → List (Source δ)} {descriptors : α → Option Bool}
    {registrants : List α} {r : α} {s : Source δ} {other : δ}
    (hs : s ∈ sources r) (ho : sourceDir s = some other) (hne : other ≠ dir r)
    (hr : r ∈ registrants) :
    r ∈ coroutineDispatched
      (fun x => directoryFlag writes (dir x) (sources x) (descriptors x)) registrants := by
  have hd : registrationDerived (dir r) (sources r) = false := by
    unfold registrationDerived
    rw [Bool.eq_false_iff]
    intro hall
    have := List.all_eq_true.mp hall s hs
    simp [ho, hne] at this
  apply an_underived_flag_keeps_the_hop _ hr
  simp [directoryFlag, hd]

/-- Under fixed flags and a derived registration, a directory whose
descriptors derive no coroutine drops the route. -/
theorem a_derived_directory_drops_the_route {α δ : Type} [DecidableEq δ]
    {writes : List FlagsWrite} {dir : α → δ} {sources : α → List (Source δ)}
    {descriptors : α → Option Bool} {registrants : List α} {r : α}
    (hw : flagsFixed writes = true) (hd : registrationDerived (dir r) (sources r) = true)
    (hn : descriptors r = some false) :
    r ∉ coroutineDispatched
      (fun x => directoryFlag writes (dir x) (sources x) (descriptors x)) registrants := by
  apply an_input_without_a_coroutine_is_not_dispatched_by_the_hop
  simp [directoryFlag, hw, hd, hn]

/-! ## The immutable threaded flag -/

/-- The flag's value at time `t`: the last write at or before `t`, else the
initial value. Writes are `(time, value)` in program order. -/
def valueAt (initial : Bool) : List (Nat × Bool) → Nat → Bool
  | [], _ => initial
  | (time, value) :: rest, t =>
    if time ≤ t then valueAt value rest t else valueAt initial rest t

theorem valueAt_before_all {initial : Bool} {writes : List (Nat × Bool)} {t : Nat}
    (h : ∀ w ∈ writes, t < w.1) : valueAt initial writes t = initial := by
  induction writes generalizing initial with
  | nil => rfl
  | cons w rest ih =>
    obtain ⟨time, value⟩ := w
    have hlt : t < time := h (time, value) (List.mem_cons_self ..)
    have hnot : ¬ time ≤ t := Nat.not_le.mpr hlt
    simp only [valueAt, hnot, ite_false]
    exact ih (fun w hw => h w (List.mem_cons_of_mem _ hw))

theorem valueAt_after_all {initial : Bool} {writes : List (Nat × Bool)} {s t : Nat}
    (hall : ∀ w ∈ writes, w.1 < s) (hst : s ≤ t) :
    valueAt initial writes t = valueAt initial writes s := by
  induction writes generalizing initial with
  | nil => rfl
  | cons w rest ih =>
    obtain ⟨time, value⟩ := w
    have hs : time < s := hall (time, value) (List.mem_cons_self ..)
    have hls : time ≤ s := Nat.le_of_lt hs
    have hlt : time ≤ t := Nat.le_trans hls hst
    simp only [valueAt, hls, hlt, ite_true]
    exact ih (fun w hw => hall w (List.mem_cons_of_mem _ hw))

/-- With every write before start, the flag any later operation reads is the
flag read at start, when the instance's collectors were attached to a loop. -/
theorem no_write_after_start_fixes_the_flag {initial : Bool} {writes : List (Nat × Bool)}
    {start t : Nat} (hall : ∀ w ∈ writes, w.1 < start) (ht : start ≤ t) :
    valueAt initial writes t = valueAt initial writes start :=
  valueAt_after_all hall ht

/-- A write of the threaded flag: into an instance its writer created, in
the setter's keyed arm reached with a key (`none`: not one literal), or at
run time. -/
inductive ThreadedWrite where
  | creation
  | setter (key : Option String)
  | runtime
  deriving DecidableEq, Repr

def threadedFixed (writes : List ThreadedWrite) : Bool := writes.all (· != .runtime)

/-- The writes the named residual premise stands for: every creation write
and every setter call whose key may be the threaded key. -/
def residualWrites (writes : List ThreadedWrite) : List ThreadedWrite :=
  writes.filter fun
    | .creation => true
    | .setter none => true
    | .setter (some key) => foldEq key "threaded"
    | .runtime => false

theorem another_literal_key_cannot_write_the_flag {writes : List ThreadedWrite}
    {key : String} (hk : foldEq key "threaded" = false) :
    ThreadedWrite.setter (some key) ∉ residualWrites writes := by
  simp [residualWrites, hk]

/-- An unfixed flag leaves every mode-fixing edge's callee unresolved: an
operation reached through one never discharges. -/
theorem a_runtime_threaded_write_never_discharges {writes : List ThreadedWrite}
    (hw : ThreadedWrite.runtime ∈ writes) (owners : Owners) (contexts : List Context)
    (unresolved : Bool) :
    siteVerdict owners contexts (unresolved || !threadedFixed writes) ≠ .discharged := by
  have hf : threadedFixed writes = false := by
    unfold threadedFixed
    rw [Bool.eq_false_iff]
    intro hall
    have := List.all_eq_true.mp hall _ hw
    simp at this
  rw [hf]
  simpa using unresolved_is_never_discharged owners contexts

/-! ## The old test -/

/-- Whatever the premises derive, they only add unresolved reasons; the old
test's resolved test-thread context still refuses its site. -/
theorem the_old_test_path_is_never_discharged_whatever_the_premises {owners : Owners}
    {contexts : List Context} {unresolved extra : Bool} {c : Context} {list : List Thread}
    (hc : c ∈ contexts) (hl : candidates owners c.mode = some list) (hn : c.thread ∉ list) :
    siteVerdict owners contexts (unresolved || extra) = .notDischarged :=
  a_non_owner_context_refuses hc hl hn

end QCore.LoopOwner.CoroPlugin
