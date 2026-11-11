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

/-! Call exits and boundaries (card CG21-CALL-EXITS-AND-BOUNDARIES, gateway
rule 11o): a model of `crates/qcore-c-gateway/src/call_exits.rs` and of the
route-G lowering in `legacy_call_exits.rs` (a `Statement::PathEnd` node has no
successor and is not an exit).

Bound mechanically: `call_exits::tests::cg21_lean_vectors_match` reads
`decisionVectors` (= `DECISION_VECTORS`) and `boundaryTable`
(= `BOUNDARIES`); `tests/cg21_call_exits.rs` runs the route-G controls. The
Clang-JSON reading of `noreturn` stays a review premise (trust boundary 28),
and so does each `resource_no_return()` declaration (a trusted assumption of
the libc provider, constraint 28).

Stated here, over straight paths of custody steps and calls:
* **Path end.** A call lowered as a path end contributes no exit state and
  nothing after it is checked (`path_end_no_exit`, `path_end_rest_unchecked`),
  but its own precondition still is (`path_end_checks_pre`).
* **Only when established.** The lowering is a path end exactly when the
  callee is contracted, declares `resource_no_return()` and its Clang
  declaration resolves to `noreturn` (`path_end_iff`); the attribute alone, or
  the declaration alone, is not enough (`attribute_alone_not_path_end`,
  `declaration_alone_not_path_end`).
* **Soundness.** Under the two premises the loader and the provider state
  (a no-return declaration is a terminating exit; a non-local exit is never
  contracted), a leak-free model outcome is a leak-free run
  (`model_sound`).
* **Why the exit kind matters.** A non-local exit taken as a path end would
  hide an abandoned resource (`nonlocal_path_end_hides_leak`).
* **Boundaries.** Every boundary is refused (`boundaries_refused`), and a
  refused step makes no leak-free claim (`refused_no_claim`). -/
namespace QCore.CallExits

/-- How a call leaves its frame in C. -/
inductive ExitKind where
  /-- It returns normally. -/
  | normal
  /-- The program terminates (or the behaviour is undefined): no frame of the
  calling thread resumes. -/
  | terminates
  /-- Control resumes in another frame (`longjmp`) or the thread ends
  (`pthread_exit`) while the program goes on. -/
  | nonlocal
  deriving DecidableEq, Repr

/-- What the body's Clang AST says of the callee (Rust `Resolution`). -/
inductive Resolution where
  | noReturn
  | returns
  | unresolved
  deriving DecidableEq, Repr

/-- The lowering of one call (Rust `Decision`). -/
inductive Decision where
  | «continue»
  | pathEnd
  | refused
  deriving DecidableEq, Repr

/-- Rust `call_exits::decide`. -/
def decide (contracted declared : Bool) (r : Resolution) : Decision :=
  if !contracted then .refused
  else if !declared then .continue
  else if r = .noReturn then .pathEnd
  else .refused

/-- `decisionVectors` = `DECISION_VECTORS`: `(contracted, declared,
resolution, decision)`. -/
def decisionVectors : List (Bool × Bool × Resolution × Decision) := [
  (false, false, .noReturn, .refused),
  (false, false, .returns, .refused),
  (false, false, .unresolved, .refused),
  (false, true, .noReturn, .refused),
  (false, true, .returns, .refused),
  (false, true, .unresolved, .refused),
  (true, false, .noReturn, .continue),
  (true, false, .returns, .continue),
  (true, false, .unresolved, .continue),
  (true, true, .noReturn, .pathEnd),
  (true, true, .returns, .refused),
  (true, true, .unresolved, .refused)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun v => decide v.1 v.2.1 v.2.2.1 == v.2.2.2) = true := by
  decide

theorem path_end_iff (contracted declared : Bool) (r : Resolution) :
    decide contracted declared r = .pathEnd ↔
      contracted = true ∧ declared = true ∧ r = .noReturn := by
  cases contracted <;> cases declared <;> cases r <;> simp [decide]

/-- The `noreturn` attribute without a no-return declaration is no path end. -/
theorem attribute_alone_not_path_end (contracted : Bool) :
    decide contracted false .noReturn ≠ .pathEnd := by
  cases contracted <;> simp [decide]

/-- A no-return declaration the Clang declaration does not establish is
refused, never a path end. -/
theorem declaration_alone_not_path_end (r : Resolution) (h : r ≠ .noReturn) :
    decide true true r = .refused := by
  cases r <;> simp_all [decide]

/-! ## Paths -/

/-- A call: its C exit kind, whether it is contracted, whether its contract
declares `resource_no_return()`, the AST's resolution, and its precondition
over the live resources. -/
structure Call where
  kind : ExitKind
  contracted : Bool
  declared : Bool
  resolution : Resolution
  pre : List Nat → Bool

/-- One custody step of a straight path. -/
inductive Step where
  | acquire (n : Nat)
  | release (n : Nat)
  | call (c : Call)

/-- How a path ends. -/
inductive Outcome where
  /-- The function returned with these resources live. -/
  | exit (live : List Nat)
  /-- The program terminated. -/
  | halt
  /-- Control left the frame non-locally, abandoning these resources. -/
  | abandon (live : List Nat)
  /-- A precondition failed. -/
  | fault
  /-- The route refused (no claim). -/
  | unknown

/-- C's behaviour. -/
def run : List Nat → List Step → Outcome
  | s, [] => .exit s
  | s, .acquire n :: rest => run (n :: s) rest
  | s, .release n :: rest => run (s.erase n) rest
  | s, .call c :: rest =>
    if c.pre s then
      match c.kind with
      | .normal => run s rest
      | .terminates => .halt
      | .nonlocal => .abandon s
    else .fault

/-- Route G's behaviour: the call's lowering decides. -/
def model : List Nat → List Step → Outcome
  | s, [] => .exit s
  | s, .acquire n :: rest => model (n :: s) rest
  | s, .release n :: rest => model (s.erase n) rest
  | s, .call c :: rest =>
    match decide c.contracted c.declared c.resolution with
    | .refused => .unknown
    | .pathEnd => if c.pre s then .halt else .fault
    | .continue => if c.pre s then model s rest else .fault

/-- The custody claim: no resource is left live at an exit or abandoned by a
non-local exit, and no precondition fails. A refusal claims nothing. -/
def leakFree : Outcome → Bool
  | .exit s => s.isEmpty
  | .halt => true
  | .abandon s => s.isEmpty
  | .fault => false
  | .unknown => false

theorem path_end_no_exit (s : List Nat) (c : Call) (rest : List Step)
    (h : decide c.contracted c.declared c.resolution = .pathEnd) (live : List Nat) :
    model s (.call c :: rest) ≠ .exit live := by
  simp only [model, h]
  split <;> simp

theorem path_end_rest_unchecked (s : List Nat) (c : Call) (rest rest' : List Step)
    (h : decide c.contracted c.declared c.resolution = .pathEnd) :
    model s (.call c :: rest) = model s (.call c :: rest') := by
  simp [model, h]

theorem path_end_checks_pre (s : List Nat) (c : Call) (rest : List Step)
    (h : decide c.contracted c.declared c.resolution = .pathEnd) (hp : c.pre s = false) :
    model s (.call c :: rest) = .fault := by
  simp [model, h, hp]

theorem refused_no_claim (s : List Nat) (c : Call) (rest : List Step)
    (h : decide c.contracted c.declared c.resolution = .refused) :
    leakFree (model s (.call c :: rest)) = false := by
  simp [model, h, leakFree]

/-- The premises a call satisfies: a `resource_no_return()` declaration states
a terminating exit (the provider's trusted assumption), and a non-local exit
has no contract (the loader refuses one). -/
def Call.WellFormed (c : Call) : Prop :=
  (c.declared = true → c.kind = .terminates) ∧ (c.kind = .nonlocal → c.contracted = false)

/-- A leak-free model outcome is a leak-free run. -/
theorem model_sound (path : List Step) (s : List Nat)
    (hw : ∀ c, Step.call c ∈ path → c.WellFormed)
    (h : leakFree (model s path) = true) : leakFree (run s path) = true := by
  induction path generalizing s with
  | nil => simpa [model, run] using h
  | cons step rest ih =>
    have hrest : ∀ c, Step.call c ∈ rest → c.WellFormed :=
      fun c hc => hw c (List.mem_cons_of_mem _ hc)
    cases step with
    | acquire n => simpa [model, run] using ih (n :: s) hrest (by simpa [model] using h)
    | release n => simpa [model, run] using ih (s.erase n) hrest (by simpa [model] using h)
    | call c =>
      have hc := hw c (List.mem_cons_self ..)
      obtain ⟨hdecl, hnonlocal⟩ := hc
      cases hd : decide c.contracted c.declared c.resolution with
      | refused => simp [model, hd, leakFree] at h
      | pathEnd =>
        have ⟨_, hdeclared, _⟩ := (path_end_iff _ _ _).mp hd
        have hk := hdecl hdeclared
        cases hp : c.pre s <;> simp_all [model, run, leakFree]
      | «continue» =>
        cases hp : c.pre s
        · simp [model, hd, hp, leakFree] at h
        · have hm : leakFree (model s rest) = true := by simpa [model, hd, hp] using h
          have hcontracted : c.contracted = true := by
            cases hcc : c.contracted
            · simp [decide, hcc] at hd
            · rfl
          cases hk : c.kind with
          | normal => simpa [run, hp, hk] using ih s hrest hm
          | terminates => simp [run, hp, hk, leakFree]
          | nonlocal => simp_all

/-- Taking a non-local exit as a path end would hide the resource it
abandons: `p = acquire(); longjmp(…);` leaks, but a path end claims
nothing is left. -/
theorem nonlocal_path_end_hides_leak :
    ∃ s, run [] [.acquire 1, .call ⟨.nonlocal, true, true, .noReturn, fun _ => true⟩]
        = .abandon s ∧ leakFree (.abandon s) = false ∧ leakFree .halt = true := by
  exact ⟨[1], rfl, rfl, rfl⟩

/-! ## Boundaries -/

/-- A statement form route G bounds explicitly (Rust `Boundary`). -/
inductive Boundary where
  | inlineAssembly
  | assemblyGoto
  | structuredExceptions
  | cleanupAttribute
  | computedGoto
  | labelAddress
  deriving DecidableEq, Repr

/-- `boundaryTable` = `BOUNDARIES`: each boundary and whether route G admits
it. -/
def boundaryTable : List (Boundary × Bool) := [
  (.inlineAssembly, false),
  (.assemblyGoto, false),
  (.structuredExceptions, false),
  (.cleanupAttribute, false),
  (.computedGoto, false),
  (.labelAddress, false)
]

/-- The lowering of a boundary statement: always the refusal. -/
def boundaryDecision (_b : Boundary) : Decision := .refused

theorem boundaries_refused (b : Boundary) : boundaryDecision b = .refused := rfl

theorem boundary_table_refused :
    boundaryTable.all (fun v => (boundaryDecision v.1 == .refused) && !v.2) = true := by
  decide

end QCore.CallExits
