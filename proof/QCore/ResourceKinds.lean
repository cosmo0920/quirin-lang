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

import QCore.KindedCustody

/-!
# Resource kinds beyond heap, descriptor, stream and directory

Card **CG25-RESOURCE-KINDS** (gateway rule 11v), absorbing
`W44-DLOPEN-DECLARATION-REMEASURE` and `W43-LIBC-IN-PLACE-LIFECYCLE`. Two kinds
join `QCore.KindedCustody.Kind`:

* `dynamicLibrary` (`dlopen`/`dlclose`): carried by a pointer, failure value
  NULL. Its protocol is the pointer-kind protocol `QCore.KindedCustody` already
  proves for every kind; this module states the named instances.
* `regex` (`regcomp`/`regfree`): **in place**. The obligation lives in caller
  storage the initialiser writes, not in a returned pointer, and the failure
  value is a **nonzero status** the initialiser returns. The model below binds
  the obligation to a storage name and the status to a witness name.

## What is proved

* **One spelling and one carrier per kind** (`kindVectors`, the Rust
  `ResourceKind::{spelling, carrier, is_integer, is_in_place}` table, bound by
  `crates/qcore-c-gateway/tests/resource_kinds_lean.rs`), and the declared
  protocol of each new kind (`protocolVectors`, bound to the libc provider
  declarations by the same test).
* **Distinctness**: a release of any other kind discharges nothing and is a
  candidate, for the library handle and for the in-place object alike.
* **The in-place status is the sentinel, soundly**: the zero edge of a test of
  the witness certifies the obligation, the nonzero edge removes it, and a
  written witness decides nothing; a finaliser before the status is decided is
  refused.
* **Lifecycle candidates**: double finalisation, use after finalisation, and a
  leak at the exit; a data route over tracked storage is refused, never
  neutral.
* **No `Safe` from any candidate or refusal**, and witness programs including
  the `rd_regex_match` shape on both of its paths.

## What this is not

A proof about the Rust transfer (trust boundary 28's correspondence premise),
or about libc (`regcomp` returning zero exactly when it initialised the object
is the libc crate's trusted premise). A candidate is never a validated
upstream defect.
-/

namespace QCore.ResourceKinds

open QCore.KindedCustody

/-! ### The kind table -/

/-- What carries a kind's identity. -/
inductive Carrier where
  | pointer
  | integer
  | storage
  deriving DecidableEq, Repr

def _root_.QCore.KindedCustody.Kind.carrier : Kind → Carrier
  | .descriptor => .integer
  | .regex => .storage
  | _ => .pointer

def _root_.QCore.KindedCustody.Kind.inPlace (k : Kind) : Bool := k.carrier = .storage

def _root_.QCore.KindedCustody.Kind.spelling : Kind → String
  | .heap => "heap"
  | .descriptor => "descriptor"
  | .stream => "stream"
  | .directory => "directory"
  | .dynamicLibrary => "dynamic_library"
  | .regex => "regex"
  | .reference => "reference"

def Carrier.word : Carrier → String
  | .pointer => "pointer"
  | .integer => "integer"
  | .storage => "storage"

/-- Every kind, in the Rust `ResourceKind::ALL` order. -/
def allKinds : List Kind :=
  [.heap, .descriptor, .stream, .directory, .dynamicLibrary, .regex, .reference]

/-- `KIND_VECTORS`: spelling, carrier, integer-carried, in place. -/
def kindVectors : List (String × String × Bool × Bool) := [
  ("heap", "pointer", false, false),
  ("descriptor", "integer", true, false),
  ("stream", "pointer", false, false),
  ("directory", "pointer", false, false),
  ("dynamic_library", "pointer", false, false),
  ("regex", "storage", false, true),
  ("reference", "pointer", false, false)
]

theorem kind_vectors_are_the_table :
    kindVectors = allKinds.map (fun k => (k.spelling, k.carrier.word, k.isInteger, k.inPlace)) := by
  decide

theorem every_kind_is_listed (k : Kind) : k ∈ allKinds := by
  cases k <;> decide

/-- One spelling per kind: the spelling decides the kind. -/
theorem spellings_are_distinct (a b : Kind) (h : a.spelling = b.spelling) : a = b := by
  revert h
  cases a <;> cases b <;> decide

/-- `PROTOCOL_VECTORS`: the declared protocol of each new kind — its spelling,
the acquiring call and position, the releasing call and position. -/
def protocolVectors : List (String × String × String × String × String) := [
  ("dynamic_library", "dlopen", "return_owned", "dlclose", "released"),
  ("regex", "regcomp", "out", "regfree", "released")
]

theorem protocol_vectors_count : protocolVectors.length = 2 := by decide

/-! ### The library handle: the pointer-kind protocol, instantiated -/

theorem the_library_handle_is_a_pointer_kind :
    Kind.dynamicLibrary.carrier = .pointer ∧ Kind.dynamicLibrary.isInteger = false := by
  decide

/-- `free(h)` on a `dlopen` handle discharges nothing and is a candidate. -/
theorem freeing_a_library_handle_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .dynamicLibrary) :
    (transfer s (.release x .heap)).status = s.status
      ∧ .kindMismatchedRelease site ∈ (transfer s (.release x .heap)).diagnostics := by
  refine ⟨a_mismatched_release_discharges_nothing s x site .heap hb (by simp [hk]), ?_⟩
  simp [transfer, hb, hk]

/-- `dlclose(p)` on a `malloc` pointer discharges nothing and is a candidate. -/
theorem closing_a_heap_allocation_as_a_library_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .heap) :
    (transfer s (.release x .dynamicLibrary)).status = s.status
      ∧ .kindMismatchedRelease site ∈ (transfer s (.release x .dynamicLibrary)).diagnostics := by
  refine ⟨a_mismatched_release_discharges_nothing s x site .dynamicLibrary hb (by simp [hk]), ?_⟩
  simp [transfer, hb, hk]

/-- `if (!h)`: the null edge removes the handle, as for every pointer kind. -/
theorem a_library_null_edge_removes_the_handle (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .dynamicLibrary) :
    (transfer s (.nullTest x true)).status site = none :=
  a_pointer_null_edge_removes_its_site s x site hb (by simp [hk, Kind.isInteger])

/-- `dlsym(h, …)` after `dlclose(h)`: a use-after-release candidate. -/
theorem a_symbol_lookup_after_close_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hr : s.status site = some .released)
    (hk : s.kind site = .dynamicLibrary) :
    .useAfterClose site ∈ (transfer s (.use x)).diagnostics :=
  a_use_after_close_is_a_candidate s x site hb hr (by simp [hk]) (by simp [hk])

/-! ### The in-place model -/

/-- A state of the in-place model: a status per site, a static kind per site,
the site each **storage** name holds, the site each **status** name
witnesses, whether a site's initialisation may still have failed, and the
diagnostics. -/
structure IState where
  status : Nat → Option Status
  kind : Nat → Kind
  storage : Nat → Option Nat
  witness : Nat → Option Nat
  mayFail : Nat → Bool
  diagnostics : List Diagnostic

inductive IInstr where
  /-- `st = init(&obj, …)`: an in-place initialiser creating `site`, its
  status held by the plain name `st`. -/
  | init (obj st site : Nat)
  /-- An initialiser whose status no plain name holds: refused. -/
  | initUnheld (obj : Nat)
  /-- One edge of a null test of `st`: `zero` is the edge where `st == 0`. -/
  | statusTest (st : Nat) (zero : Bool)
  /-- Any write of the status name. -/
  | writeStatus (st : Nat)
  /-- `fin(&obj)` by a finaliser of kind `k`. -/
  | finalise (obj : Nat) (k : Kind)
  /-- `use(&obj, …)` at a borrowed position of kind `k`. -/
  | use (obj : Nat) (k : Kind)
  /-- A route that reads the storage as data (`other = obj`, `memset(&obj,
  …)`, an unkinded position). -/
  | dataRoute (obj : Nat)
  deriving DecidableEq, Repr

def transferI (s : IState) : IInstr → IState
  | .init obj st site =>
      { s with status := update s.status site (some .live),
               storage := update s.storage obj (some site),
               witness := update s.witness st (some site),
               mayFail := update s.mayFail site true }
  | .initUnheld _ => { s with diagnostics := .unknown :: s.diagnostics }
  | .statusTest st zero =>
      match s.witness st with
      | some site =>
          if zero then { s with mayFail := update s.mayFail site false }
          else { s with status := update s.status site none }
      | none => s
  | .writeStatus st => { s with witness := update s.witness st none }
  | .finalise obj k =>
      match s.storage obj with
      | some site =>
          if s.kind site ≠ k then
            { s with diagnostics := .kindMismatchedRelease site :: s.diagnostics }
          else match s.status site with
            | some .live =>
                if s.mayFail site then { s with diagnostics := .unknown :: s.diagnostics }
                else { s with status := update s.status site (some .released) }
            | some .released => { s with diagnostics := .doubleRelease site :: s.diagnostics }
            | none => { s with diagnostics := .unknown :: s.diagnostics }
      | none => { s with diagnostics := .unknown :: s.diagnostics }
  | .use obj _ =>
      match s.storage obj with
      | some site =>
          if s.status site = some .released then
            { s with diagnostics := .useAfterClose site :: .unknown :: s.diagnostics }
          else s
      | none => s
  | .dataRoute _ => { s with diagnostics := .unknown :: s.diagnostics }

def runI (s : IState) (program : List IInstr) : IState :=
  program.foldl transferI s

def exitI (s : IState) (sites : List Nat) : List Diagnostic :=
  sites.filterMap (fun site => if s.status site = some .live then some (.leak site) else none)
    ++ s.diagnostics

/-! ### The status is the sentinel -/

/-- What the initialiser's status means: zero exactly when the object was
initialised and is owed a finaliser. -/
def Owes (status : Int) : Prop := status = 0

/-- The edge table is sound: on the edge where the status is zero the
obligation is owed, on the other it is not. -/
theorem the_status_table_is_sound (status : Int) :
    (status = 0 → Owes status) ∧ (status ≠ 0 → ¬ Owes status) := by
  simp [Owes]

/-- The zero edge certifies the obligation: still live, no longer failing. -/
theorem the_zero_status_edge_certifies (s : IState) (st site : Nat)
    (hw : s.witness st = some site) :
    (transferI s (.statusTest st true)).status = s.status
      ∧ (transferI s (.statusTest st true)).mayFail site = false := by
  simp [transferI, hw, update]

/-- The nonzero edge owes nothing: the site is gone. -/
theorem the_nonzero_status_edge_owes_nothing (s : IState) (st site : Nat)
    (hw : s.witness st = some site) :
    (transferI s (.statusTest st false)).status site = none := by
  simp [transferI, hw, update]

/-- A written status witnesses nothing: a later test changes nothing. -/
theorem a_written_status_decides_nothing (s : IState) (st : Nat) (zero : Bool) :
    transferI (transferI s (.writeStatus st)) (.statusTest st zero)
      = transferI s (.writeStatus st) := by
  simp [transferI, update]

/-- A finaliser before the status is decided is refused, and finalises
nothing. -/
theorem an_untested_finalise_is_refused (s : IState) (obj site : Nat) (k : Kind)
    (hb : s.storage obj = some site) (hk : s.kind site = k)
    (hl : s.status site = some .live) (hf : s.mayFail site = true) :
    (transferI s (.finalise obj k)).diagnostics = .unknown :: s.diagnostics
      ∧ (transferI s (.finalise obj k)).status = s.status := by
  simp [transferI, hb, hk, hl, hf]

/-- An unheld status is refused. -/
theorem an_unheld_status_is_refused (s : IState) (obj : Nat) :
    .unknown ∈ (transferI s (.initUnheld obj)).diagnostics := by
  simp [transferI]

/-! ### Distinctness and the lifecycle candidates -/

/-- A finaliser of another kind discharges nothing and is a candidate. -/
theorem a_finaliser_of_another_kind_is_a_candidate (s : IState) (obj site : Nat) (k : Kind)
    (hb : s.storage obj = some site) (hk : s.kind site ≠ k) :
    (transferI s (.finalise obj k)).status = s.status
      ∧ (transferI s (.finalise obj k)).diagnostics
          = .kindMismatchedRelease site :: s.diagnostics := by
  simp [transferI, hb, hk]

/-- A certified object is finalised by its own kind. -/
theorem a_certified_object_is_finalised (s : IState) (obj site : Nat)
    (hb : s.storage obj = some site) (hl : s.status site = some .live)
    (hf : s.mayFail site = false) :
    (transferI s (.finalise obj (s.kind site))).status site = some .released := by
  simp [transferI, hb, hl, hf, update]

theorem a_double_finalise_is_a_candidate (s : IState) (obj site : Nat)
    (hb : s.storage obj = some site) (hr : s.status site = some .released) :
    .doubleRelease site ∈ (transferI s (.finalise obj (s.kind site))).diagnostics := by
  simp [transferI, hb, hr]

theorem a_use_after_finalise_is_a_candidate (s : IState) (obj site : Nat) (k : Kind)
    (hb : s.storage obj = some site) (hr : s.status site = some .released) :
    .useAfterClose site ∈ (transferI s (.use obj k)).diagnostics := by
  simp [transferI, hb, hr]

/-- Tracked storage is custody, never data: every data route is refused. -/
theorem a_data_route_over_tracked_storage_is_refused (s : IState) (obj : Nat) :
    (transferI s (.dataRoute obj)).diagnostics = .unknown :: s.diagnostics := by
  simp [transferI]

/-- No `Safe` from any candidate or refusal. -/
theorem no_candidate_is_safe (ds : List Diagnostic) (site : Nat)
    (h : .kindMismatchedRelease site ∈ ds ∨ .useAfterClose site ∈ ds
      ∨ .doubleRelease site ∈ ds ∨ .leak site ∈ ds ∨ .unknown ∈ ds) :
    classify ds ≠ .safe := by
  rcases h with h | h | h | h | h
  · exact a_kind_candidate_is_never_safe ds site (Or.inl h)
  · exact a_kind_candidate_is_never_safe ds site (Or.inr h)
  · unfold classify
    have : ds.any isSafetyIssue = true := List.any_eq_true.mpr ⟨_, h, rfl⟩
    simp [this]
  · unfold classify
    have : ds.any isSafetyIssue = true := List.any_eq_true.mpr ⟨_, h, rfl⟩
    simp [this]
  · exact a_refusal_is_never_safe ds h

/-! ### Witness programs -/

/-- Storage name 0 (`re`), status name 1 (`r`), site 0 of kind `regex`, site 1
of kind `heap`. -/
def startI : IState :=
  { status := fun _ => none,
    kind := fun site => if site = 0 then .regex else .heap,
    storage := fun _ => none,
    witness := fun _ => none,
    mayFail := fun _ => false,
    diagnostics := [] }

/-- `rd_regex_match`, failure path: `r = regcomp(&re, …); if (r) return 0;` —
nothing is owed. -/
theorem the_regex_match_failure_path_owes_nothing :
    exitI (runI startI [.init 0 1 0, .statusTest 1 false]) [0] = [] := by
  decide

/-- `rd_regex_match`, success path: `…; r = regexec(&re, …) != NOMATCH;
regfree(&re); return r;` — discharged, with no diagnostic. -/
theorem the_regex_match_success_path_discharges :
    exitI (runI startI [.init 0 1 0, .statusTest 1 true, .use 0 .regex, .writeStatus 1,
      .finalise 0 .regex]) [0] = [] := by
  decide

/-- Never finalised on the success path: a leak candidate. -/
theorem a_compiled_object_never_freed_leaks :
    exitI (runI startI [.init 0 1 0, .statusTest 1 true, .use 0 .regex]) [0] = [.leak 0] := by
  decide

/-- `regcomp(&re, …); regfree(&re);` with the status discarded: refused. -/
theorem an_unheld_compile_is_never_safe :
    classify (exitI (runI startI [.initUnheld 0]) [0]) ≠ .safe := by
  decide

/-- A status overwritten before its test: the finaliser is refused. -/
theorem an_overwritten_status_refuses_the_finaliser :
    classify (exitI (runI startI [.init 0 1 0, .writeStatus 1, .statusTest 1 true,
      .finalise 0 .regex]) [0]) ≠ .safe := by
  decide

/-- `regfree` twice. -/
theorem a_double_regfree_is_a_candidate :
    (runI startI [.init 0 1 0, .statusTest 1 true, .finalise 0 .regex,
      .finalise 0 .regex]).diagnostics = [.doubleRelease 0] := by
  decide

/-- A library finaliser applied to the compiled object: accused, and the
object still leaks. -/
theorem a_library_release_of_a_regex_leaves_the_obligation :
    exitI (runI startI [.init 0 1 0, .statusTest 1 true, .finalise 0 .dynamicLibrary]) [0]
      = [.leak 0, .kindMismatchedRelease 0] := by
  decide

end QCore.ResourceKinds
