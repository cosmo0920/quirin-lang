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
# Kind-indexed custody and integer-carried identities

Card **W43-DESCRIPTOR-RESOURCE-TRACKING**, discharging
`W43-LIBC-DESCRIPTOR-LIFECYCLE` and `CT-CTWIN-DESCRIPTOR`. Before this card
every tracked resource had one kind, so `free(fopen(…))` and
`fclose(malloc(…))` both discharged, and a file descriptor — an `int` whose
failure value is `-1` — could not be tracked at all. The contract vocabulary now
gives each custody position a **kind** (`resource_kind`, `resource_return_kind`,
contract spec §C04.6), and the C gateway (`crates/qcore-c-gateway/src/
legacy_c.rs`, `legacy_descriptor_custody.rs`) applies the rules this module
models.

## The model

A state has a status per resource site, a **static kind** per site, a binding
per name (the site it holds, if any), whether each site may still be the
failure value, and a diagnostic list. The instructions are the ones the rules
speak about: an acquisition of a kind, a release of a kind, a declared use, a
null test, a validity test, a custody-neutral scalar route reading a name, and
a scalar computation into a name.

## What is proved

* **A release discharges only its own kind.** A release of another kind
  changes no status and is a `kindMismatchedRelease` candidate; a matching one
  releases exactly as before.
* **Validity is the integer counterpart of nullness.** The table of what each
  comparison with `0` or `-1` establishes is **sound** for every value a
  descriptor-returning call can produce (`-1` or a number `≥ 0`), and the
  `fd > 0` false edge is left unrefined because descriptor `0` is valid —
  refining it would be unsound, and a witness says so.
* **A null test of a descriptor refines nothing**, whereas a pointer's null
  edge removes its site.
* **A tracked descriptor on a custody-neutral scalar route is refused**, never
  admitted as data, and the same route over an untracked name stays neutral.
* **Arithmetic creates and aliases no identity.**
* **Use after close** of a non-heap identity is a candidate; the heap keeps its
  pre-existing refusal.
* **No `Safe` from any of it**, and **conservativity**: when every site is heap
  and every release is heap, the transfer is the pre-card one exactly.
* **Witness programs**, including the corpus's `if (fd > 0)` leak shape.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. That the
Rust transfer implements these rules is the Rust–Lean correspondence premise of
trust boundary 28; that `open` returns `-1` or a descriptor and `close` closes
one are the trusted contract premises of the libc crate. Every violation is a
*candidate*, never a validated upstream defect.
-/

namespace QCore.KindedCustody

/-- The resource kinds of contract spec §C04.6. Card CG25-RESOURCE-KINDS adds
the pointer-carried `dynamicLibrary` and the in-place `regex`
(`QCore.ResourceKinds`); every theorem below is stated for every kind. -/
inductive Kind where
  | heap
  | descriptor
  | stream
  | directory
  | dynamicLibrary
  | regex
  /-- Card REFCOUNT-RESOURCE-KIND (§C04.17): one reference a function holds on
  a reference-counted object (`QCore.ReferenceCount`). A use after its last
  drop is refused there, never accused, so this model's use-after-close
  candidate excludes it as it excludes the heap. -/
  | reference
  deriving DecidableEq, Repr

/-- The kinds carried by an integer rather than a pointer. -/
def Kind.isInteger : Kind → Bool
  | .descriptor => true
  | _ => false

inductive Status where
  | live
  | released
  deriving DecidableEq, Repr

inductive Diagnostic where
  | doubleRelease (site : Nat)
  | kindMismatchedRelease (site : Nat)
  | useAfterClose (site : Nat)
  | leak (site : Nat)
  | unknown
  deriving DecidableEq, Repr

/-- What a validity test establishes on one edge. -/
inductive Validity where
  | invalid
  | valid
  | unrefined
  deriving DecidableEq, Repr

structure State where
  status : Nat → Option Status
  /-- The static kind of every site: the kind of the call that creates it. -/
  kind : Nat → Kind
  binding : Nat → Option Nat
  /-- Whether the site may still be the failure value (NULL or `-1`). -/
  mayFail : Nat → Bool
  diagnostics : List Diagnostic

inductive Instruction where
  /-- `x = acquire()` of kind `k` at `site`; `fallible` is `nullable`/`sentinel`. -/
  | acquire (x site : Nat) (fallible : Bool)
  /-- `release(x)` by a release declared of kind `k`. -/
  | release (x : Nat) (k : Kind)
  /-- `x` at a declared `Borrowed` position. -/
  | use (x : Nat)
  /-- The edge of a null test of `x` on which `x` is null (`true`) or not. -/
  | nullTest (x : Nat) (isNull : Bool)
  /-- One edge of a validity test of `x`, with the fact it establishes there. -/
  | validityTest (x : Nat) (fact : Validity)
  /-- A custody-neutral scalar route reading `x` (`c->fd = x`, `*p = x`, …). -/
  | scalarRoute (x : Nat)
  /-- `y = x + 1`: a scalar computation into `y`. -/
  | arith (y x : Nat)
  deriving DecidableEq, Repr

def update {β : Type} (f : Nat → β) (a : Nat) (b : β) : Nat → β :=
  fun x => if x = a then b else f x

/-- The one-kind transfer before this card: kinds are ignored, a null test
removes any site, and a scalar route is always neutral. -/
def transferBase (s : State) : Instruction → State
  | .acquire x site fallible =>
      { s with status := update s.status site (some .live),
               binding := update s.binding x (some site),
               mayFail := update s.mayFail site fallible }
  | .release x _ =>
      match s.binding x with
      | some site =>
          match s.status site with
          | some .live => { s with status := update s.status site (some .released) }
          | some .released => { s with diagnostics := .doubleRelease site :: s.diagnostics }
          | none => { s with diagnostics := .unknown :: s.diagnostics }
      | none => { s with diagnostics := .unknown :: s.diagnostics }
  | .use x =>
      match s.binding x with
      | some site =>
          if s.status site = some .released then
            { s with diagnostics := .unknown :: s.diagnostics }
          else s
      | none => s
  | .nullTest x isNull =>
      match s.binding x with
      | some site =>
          if isNull then
            { s with status := update s.status site none, binding := update s.binding x none }
          else { s with mayFail := update s.mayFail site false }
      | none => s
  | .validityTest _ _ => { s with diagnostics := .unknown :: s.diagnostics }
  | .scalarRoute _ => s
  | .arith y _ => { s with binding := update s.binding y none }

/-- The transfer after this card. -/
def transfer (s : State) : Instruction → State
  | .release x k =>
      match s.binding x with
      | some site =>
          if s.kind site = k then transferBase s (.release x k)
          else { s with diagnostics := .kindMismatchedRelease site :: s.diagnostics }
      | none => transferBase s (.release x k)
  | .use x =>
      match s.binding x with
      | some site =>
          if s.status site = some .released ∧ s.kind site ≠ .heap ∧ s.kind site ≠ .reference then
            { s with diagnostics := .useAfterClose site :: .unknown :: s.diagnostics }
          else transferBase s (.use x)
      | none => s
  | .nullTest x isNull =>
      match s.binding x with
      | some site => if (s.kind site).isInteger then s else transferBase s (.nullTest x isNull)
      | none => s
  | .validityTest x fact =>
      match fact with
      | .unrefined => s
      | .invalid | .valid =>
          match s.binding x with
          | some site =>
              if (s.kind site).isInteger then transferBase s (.nullTest x (fact = .invalid))
              else { s with diagnostics := .unknown :: s.diagnostics }
          | none => s
  | .scalarRoute x =>
      match s.binding x with
      | some site =>
          if (s.kind site).isInteger then { s with diagnostics := .unknown :: s.diagnostics }
          else s
      | none => s
  | i => transferBase s i

def run (s : State) (program : List Instruction) : State :=
  program.foldl transfer s

/-- The exit: every live site is a leak. -/
def exitDiagnostics (s : State) (sites : List Nat) : List Diagnostic :=
  sites.filterMap (fun site => if s.status site = some .live then some (.leak site) else none)
    ++ s.diagnostics

def isSafetyIssue : Diagnostic → Bool
  | .unknown => false
  | _ => true

inductive Verdict where
  | safe
  | defect
  | indeterminate
  deriving DecidableEq, Repr

def classify (diagnostics : List Diagnostic) : Verdict :=
  if diagnostics.any isSafetyIssue then .defect
  else if diagnostics.contains .unknown then .indeterminate
  else .safe

/-! ### A release discharges only its own kind -/

/-- A release of another kind changes no status. -/
theorem a_mismatched_release_discharges_nothing (s : State) (x site : Nat) (k : Kind)
    (hb : s.binding x = some site) (hk : s.kind site ≠ k) :
    (transfer s (.release x k)).status = s.status := by
  simp [transfer, hb, hk]

/-- It is a violation candidate naming the site. -/
theorem a_mismatched_release_is_a_candidate (s : State) (x site : Nat) (k : Kind)
    (hb : s.binding x = some site) (hk : s.kind site ≠ k) :
    (transfer s (.release x k)).diagnostics = .kindMismatchedRelease site :: s.diagnostics := by
  simp [transfer, hb, hk]

/-- A release of the matching kind releases a live site. -/
theorem a_matching_release_releases (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hl : s.status site = some .live) :
    (transfer s (.release x (s.kind site))).status site = some .released := by
  simp [transfer, transferBase, hb, hl, update]

/-- `free(fopen(…))`: a heap release of a stream is accused. -/
theorem freeing_a_stream_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .stream) :
    .kindMismatchedRelease site ∈ (transfer s (.release x .heap)).diagnostics := by
  simp [transfer, hb, hk]

/-- `fclose(malloc(…))`: a stream release of a heap allocation is accused. -/
theorem closing_a_heap_allocation_as_a_stream_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .heap) :
    .kindMismatchedRelease site ∈ (transfer s (.release x .stream)).diagnostics := by
  simp [transfer, hb, hk]

/-- `close(fp)`: a descriptor release of a stream is accused. -/
theorem closing_a_stream_as_a_descriptor_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .stream) :
    .kindMismatchedRelease site ∈ (transfer s (.release x .descriptor)).diagnostics := by
  simp [transfer, hb, hk]

/-! ### Validity: the integer counterpart of nullness -/

/-- The six comparison operators of a validity test. -/
inductive Comparison where
  | less
  | lessEqual
  | greater
  | greaterEqual
  | equal
  | notEqual
  deriving DecidableEq, Repr

/-- The two literals a validity test compares against. -/
inductive Literal where
  | zero
  | minusOne
  deriving DecidableEq, Repr

def Literal.value : Literal → Int
  | .zero => 0
  | .minusOne => -1

/-- Whether `v OP w` holds. -/
def Comparison.Holds (c : Comparison) (v w : Int) : Prop :=
  match c with
  | .less => v < w
  | .lessEqual => v ≤ w
  | .greater => v > w
  | .greaterEqual => v ≥ w
  | .equal => v = w
  | .notEqual => v ≠ w

/-- `descriptor_custody::facts`: what `fd OP L` establishes on its true and
false edges. -/
def facts : Comparison → Literal → Validity × Validity
  | .less, .zero => (.invalid, .valid)
  | .lessEqual, .minusOne => (.invalid, .valid)
  | .equal, .minusOne => (.invalid, .valid)
  | .greaterEqual, .zero => (.valid, .invalid)
  | .greater, .minusOne => (.valid, .invalid)
  | .notEqual, .minusOne => (.valid, .invalid)
  | .greater, .zero => (.valid, .unrefined)
  | .lessEqual, .zero => (.unrefined, .valid)
  -- `fd == 0` / `fd != 0` decide nothing (descriptor `0` is valid), and the
  -- remaining four are unreachable for the declared value set: unrefined.
  | .equal, .zero => (.unrefined, .unrefined)
  | .notEqual, .zero => (.unrefined, .unrefined)
  | .less, .minusOne => (.unrefined, .unrefined)
  | .greaterEqual, .minusOne => (.unrefined, .unrefined)

/-- The values a descriptor-returning call can produce: the sentinel `-1` or a
valid descriptor `≥ 0`. -/
def Possible (v : Int) : Prop := v = -1 ∨ 0 ≤ v

/-- What a fact claims about the value on its edge. -/
def Holds : Validity → Int → Prop
  | .invalid, v => v < 0
  | .valid, v => 0 ≤ v
  | .unrefined, _ => True

/-- **The table is sound**: for every value a descriptor-returning call can
produce, the fact on the edge the comparison takes holds of that value. -/
theorem facts_are_sound (c : Comparison) (l : Literal) (v : Int) (hv : Possible v) :
    (c.Holds v l.value → Holds (facts c l).1 v)
      ∧ (¬ c.Holds v l.value → Holds (facts c l).2 v) := by
  unfold Possible at hv
  cases c <;> cases l <;>
    simp only [facts, Holds, Comparison.Holds, Literal.value] <;>
    constructor <;> intro h <;> first | trivial | omega

/-- **Descriptor `0` is valid**, so the false edge of `fd > 0` cannot be read
as invalid: a possible value takes that edge and is a valid descriptor. This is
why the gateway leaves it unrefined, and why the corpus's `if (fd > 0)` shape
leaks descriptor `0`. -/
theorem the_positive_tests_false_edge_includes_a_valid_descriptor :
    ∃ v, Possible v ∧ ¬ Comparison.greater.Holds v Literal.zero.value ∧ ¬ Holds .invalid v :=
  ⟨0, Or.inr (Int.le_refl 0), by simp [Comparison.Holds, Literal.value], by simp [Holds]⟩

/-- Likewise `fd == 0` says nothing about validity: both outcomes are valid
descriptors or the sentinel. -/
theorem equality_with_zero_decides_no_validity :
    (∃ v, Possible v ∧ Comparison.equal.Holds v Literal.zero.value ∧ ¬ Holds .invalid v)
      ∧ (∃ v, Possible v ∧ ¬ Comparison.equal.Holds v Literal.zero.value
          ∧ ¬ Holds .valid v) :=
  ⟨⟨0, Or.inr (Int.le_refl 0), by simp [Comparison.Holds, Literal.value], by simp [Holds]⟩,
   ⟨-1, Or.inl rfl, by simp [Comparison.Holds, Literal.value], by simp [Holds]⟩⟩

/-- An invalid edge removes the tested descriptor's site, exactly as a null
edge removes a pointer's. -/
theorem an_invalid_edge_removes_the_descriptor (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .descriptor) :
    (transfer s (.validityTest x .invalid)).status site = none := by
  simp [transfer, transferBase, hb, hk, Kind.isInteger, update]

/-- A valid edge keeps it and marks it valid. -/
theorem a_valid_edge_keeps_a_valid_descriptor (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : s.kind site = .descriptor) :
    (transfer s (.validityTest x .valid)).status = s.status
      ∧ (transfer s (.validityTest x .valid)).mayFail site = false := by
  simp [transfer, transferBase, hb, hk, Kind.isInteger, update]

/-- An unrefined edge changes nothing at all. -/
theorem an_unrefined_edge_changes_nothing (s : State) (x : Nat) :
    transfer s (.validityTest x .unrefined) = s := by
  simp [transfer]

/-- A validity test of a name bound to a pointer kind is a refusal, not a
refinement. -/
theorem a_validity_test_of_a_pointer_kind_is_refused (s : State) (x site : Nat)
    (fact : Validity) (hf : fact ≠ .unrefined) (hb : s.binding x = some site)
    (hk : (s.kind site).isInteger = false) :
    (transfer s (.validityTest x fact)).diagnostics = .unknown :: s.diagnostics
      ∧ (transfer s (.validityTest x fact)).status = s.status := by
  cases fact with
  | unrefined => exact absurd rfl hf
  | invalid => simp [transfer, hb, hk]
  | valid => simp [transfer, hb, hk]

/-! ### A null test of a descriptor refines nothing -/

/-- `if (fd)` / `if (!fd)`: neither edge changes the state. -/
theorem a_descriptor_null_test_refines_nothing (s : State) (x site : Nat) (isNull : Bool)
    (hb : s.binding x = some site) (hk : s.kind site = .descriptor) :
    transfer s (.nullTest x isNull) = s := by
  simp [transfer, hb, hk, Kind.isInteger]

/-- Whereas a pointer's null edge removes its site: the rule a descriptor must
not inherit. -/
theorem a_pointer_null_edge_removes_its_site (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : (s.kind site).isInteger = false) :
    (transfer s (.nullTest x true)).status site = none := by
  simp [transfer, transferBase, hb, hk, update]

/-! ### The scalar routes: a tracked descriptor is custody, not data -/

/-- A custody-neutral scalar route reading a tracked descriptor is refused. -/
theorem a_tracked_descriptor_on_a_scalar_route_is_refused (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : (s.kind site).isInteger = true) :
    (transfer s (.scalarRoute x)).diagnostics = .unknown :: s.diagnostics := by
  simp [transfer, hb, hk]

/-- It is never admitted as the neutral no-op the pre-card route was. -/
theorem a_tracked_descriptor_is_never_neutral_data (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hk : (s.kind site).isInteger = true) :
    transfer s (.scalarRoute x) ≠ transferBase s (.scalarRoute x) := by
  intro h
  have := congrArg State.diagnostics h
  simp [transfer, transferBase, hb, hk] at this

/-- The same route over an untracked name, or a pointer-kinded one, stays
neutral: the rule is not vacuous in the admitting direction either. -/
theorem an_untracked_scalar_route_stays_neutral (s : State) (x : Nat)
    (hb : ∀ site, s.binding x = some site → (s.kind site).isInteger = false) :
    transfer s (.scalarRoute x) = s := by
  simp only [transfer]
  split
  · next site h => simp [hb site h]
  · rfl

/-- Arithmetic on a descriptor binds its destination to nothing and changes no
status: it creates and aliases no identity. -/
theorem arithmetic_creates_no_identity (s : State) (y x : Nat) :
    (transfer s (.arith y x)).binding y = none
      ∧ (transfer s (.arith y x)).status = s.status := by
  simp [transfer, transferBase, update]

/-! ### Use after close -/

/-- A use of a released non-heap identity is a candidate (a reference's is
refused, as the heap's is). -/
theorem a_use_after_close_is_a_candidate (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hr : s.status site = some .released)
    (hk : s.kind site ≠ .heap) (hn : s.kind site ≠ .reference) :
    .useAfterClose site ∈ (transfer s (.use x)).diagnostics := by
  simp [transfer, hb, hr, hk, hn]

/-- The heap keeps its pre-existing refusal, unaccused. -/
theorem a_heap_use_after_release_keeps_its_refusal (s : State) (x site : Nat)
    (hb : s.binding x = some site) (hr : s.status site = some .released)
    (hk : s.kind site = .heap) :
    (transfer s (.use x)).diagnostics = .unknown :: s.diagnostics := by
  simp [transfer, transferBase, hb, hr, hk]

/-! ### No `Safe` from any of it -/

theorem the_kind_candidates_are_safety_issues (site : Nat) :
    isSafetyIssue (.kindMismatchedRelease site) = true
      ∧ isSafetyIssue (.useAfterClose site) = true := by
  simp [isSafetyIssue]

/-- A diagnostic list carrying either candidate is never `Safe`. -/
theorem a_kind_candidate_is_never_safe (ds : List Diagnostic) (site : Nat)
    (h : .kindMismatchedRelease site ∈ ds ∨ .useAfterClose site ∈ ds) :
    classify ds ≠ .safe := by
  unfold classify
  have hany : ds.any isSafetyIssue = true := by
    rw [List.any_eq_true]
    rcases h with h | h
    · exact ⟨_, h, rfl⟩
    · exact ⟨_, h, rfl⟩
  simp [hany]

/-- A refusal alone is never `Safe` either. -/
theorem a_refusal_is_never_safe (ds : List Diagnostic) (h : .unknown ∈ ds) :
    classify ds ≠ .safe := by
  unfold classify
  split
  · simp
  · simp [h]

/-! ### Conservativity -/

/-- When every site is heap, every release is a heap release and no validity
test occurs, every instruction behaves exactly as the one-kind transfer did. -/
theorem an_all_heap_program_is_unaffected (s : State) (i : Instruction)
    (hheap : ∀ site, s.kind site = .heap)
    (hrelease : ∀ x k, i = .release x k → k = .heap)
    (hvalid : ∀ x fact, i ≠ .validityTest x fact) :
    transfer s i = transferBase s i := by
  cases i with
  | acquire x site fallible => rfl
  | release x k =>
      have hk := hrelease x k rfl
      subst hk
      cases hb : s.binding x with
      | none => simp [transfer, hb]
      | some site => simp [transfer, hb, hheap site]
  | use x =>
      cases hb : s.binding x with
      | none => simp [transfer, transferBase, hb]
      | some site => simp [transfer, hb, hheap site]
  | nullTest x isNull =>
      cases hb : s.binding x with
      | none => simp [transfer, transferBase, hb]
      | some site => simp [transfer, hb, hheap site, Kind.isInteger]
  | validityTest x fact => exact absurd rfl (hvalid x fact)
  | scalarRoute x =>
      cases hb : s.binding x with
      | none => simp [transfer, transferBase, hb]
      | some site => simp [transfer, transferBase, hb, hheap site, Kind.isInteger]
  | arith y x => rfl

/-! ### Witness programs -/

/-- One descriptor site 0 bound to name 0, and a heap site 1. -/
def start : State :=
  { status := fun _ => none,
    kind := fun site => if site = 0 then .descriptor else .heap,
    binding := fun _ => none,
    mayFail := fun _ => false,
    diagnostics := [] }

/-- `fd = open(); if (fd < 0) return; close(fd);` — the fall-through path is
discharged, with no diagnostic. -/
theorem a_guarded_close_discharges :
    exitDiagnostics
        (run start [.acquire 0 0 true, .validityTest 0 .valid, .release 0 .descriptor]) [0]
      = [] := by
  decide

/-- `fd = open(); if (fd > 0) close(fd);` — on the false edge the descriptor is
unrefined and never closed: a leak candidate, as `ctr_random_get` measures. -/
theorem the_positive_guard_leaks_on_its_false_edge :
    exitDiagnostics (run start [.acquire 0 0 true, .validityTest 0 .unrefined]) [0]
      = [.leak 0] := by
  decide

/-- `fd = open(); if (fd < 0) return; close(fd); close(fd);` — a double close. -/
theorem a_double_close_is_a_candidate :
    (run start [.acquire 0 0 true, .validityTest 0 .valid, .release 0 .descriptor,
      .release 0 .descriptor]).diagnostics = [.doubleRelease 0] := by
  decide

/-- `p = malloc(); …; close(p)`: the descriptor release discharges nothing, so
the heap allocation leaks as well as being accused. -/
theorem a_mismatched_release_leaves_the_obligation :
    exitDiagnostics (run start [.acquire 1 1 false, .release 1 .descriptor]) [1]
      = [.leak 1, .kindMismatchedRelease 1] := by
  decide

/-- `fd = open(); …; conn->fd = fd;` — refused, not a neutral store. -/
theorem storing_a_descriptor_into_a_member_is_refused :
    classify (run start [.acquire 0 0 true, .validityTest 0 .valid, .scalarRoute 0]).diagnostics
      = .indeterminate := by
  decide

end QCore.KindedCustody
