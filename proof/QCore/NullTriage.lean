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

import QCore.NullDeref

/-!
# The null-candidate triage (card NULL-CANDIDATE-TRIAGE, gateway rules 14r–14v)

The abstract rules of `crates/qcore-core/src/null_triage.rs`, of the gateway's
path search `crates/qcore-c-gateway/src/null_deref/witness.rs` and of the
audit's run fold `crates/qcore-audit/src/null_fold.rs`, on top of
`QCore.NullDeref`.

* **A validated path is a real execution** (`a_validated_path_is_satisfiable`).
  A witness path is a list of branch decisions over values. A decision is
  taken only when the consumer decides its condition and the arm agrees, or
  when the condition reads one *fresh* value no earlier decision constrained
  (`admit`). Every such path has a concrete valuation that takes every one of
  its arms, so the NULL dereference at its end is reached
  (`a_validated_finding_faults`).
* **Interval decisions.** A comparison of one value with constants narrows
  its interval; an arm is taken only when the narrowed interval is inhabited,
  and every value of the final interval takes every recorded arm
  (`interval_path_is_satisfiable`).
* **Findings never make `Safe`** (`triage_never_makes_safe`); a row is
  `Unsafe` only for a reachable violation or a validated finding
  (`unsafe_only_for_a_violation_or_a_finding`).
* **Declared members** (`member_invariant_holds`): when the construction
  premise holds and every store keeps the member non-null, every read along
  any trace of stores and reads sees a non-null member. A declaration with
  a premise is never `validated` (`a_member_declaration_is_never_validated`);
  a failed store withdraws it (`a_failed_store_withdraws`).
* **Declared results** follow rule 13g's fold (`result_vectors_hold`).
* **Withdrawal and travel never improve a row**
  (`a_withdrawn_dependency_is_never_safe`, `travel_never_improves_a_row`).

## What this is not

A model of C or of the gateway's reading of Clang's AST: that the search reads
each branch's condition and the values it names as the Rust does is a review
premise (trust boundary 28), pinned by the controls in
`crates/qcore-c-gateway/tests/null_candidate_triage.rs`. A finding is
provisional: the function's own entry is assumed reachable with any
arguments its preconditions admit, and it is never an upstream defect.
-/

namespace QCore.NullTriage

open QCore.NullDeref (Verdict Outcome verdict rank compose)

/-! ## Path decisions -/

/-- What the consumer says of one condition on a path (`null_triage::Condition`). -/
inductive Condition where
  | decided (value : Bool)
  | free
  | correlated
  deriving DecidableEq, Repr

inductive Admission where
  | taken | infeasible | unresolved
  deriving DecidableEq, Repr

/-- `null_triage::admit`. -/
def admit : Condition → Bool → Admission
  | .decided v, arm => if v = arm then .taken else .infeasible
  | .free, _ => .taken
  | .correlated, _ => .unresolved

/-- A correlated condition is never taken. -/
theorem a_correlated_condition_is_never_taken (arm : Bool) :
    admit .correlated arm ≠ .taken := by
  simp [admit]

/-- A decided condition is taken only on its own arm. -/
theorem a_decided_condition_takes_its_arm (v arm : Bool) (h : admit (.decided v) arm = .taken) :
    v = arm := by
  cases v <;> cases arm <;> simp_all [admit]

/-- One step of a witness path: a branch over one value (`atom`), decided
(`some value`) or free, and the arm the path takes. -/
structure Step where
  atom : Nat
  decided : Option Bool
  arm : Bool
  deriving DecidableEq, Repr

/-- The atoms the path's free steps read, in order. -/
def freeAtoms : List Step → List Nat
  | [] => []
  | s :: rest =>
      match s.decided with
      | none => s.atom :: freeAtoms rest
      | some _ => freeAtoms rest

/-- Whether a decided step agrees with its arm (`admit` takes it). -/
def agrees (s : Step) : Bool :=
  match s.decided with
  | some v => v == s.arm
  | none => true

/-- A path the search validates: every decided step is taken on its own arm,
and a free step reads a value no other free step reads (the search ends a
path, unresolved, at a condition over a value an earlier decision
constrained: `admit .correlated`). -/
def validated (path : List Step) : Prop :=
  (∀ s ∈ path, agrees s = true) ∧ (freeAtoms path).Nodup

/-- The concrete semantics: a valuation of the atoms gives each free
condition its value; a decided step is the consumer's own decision. -/
def takes (σ : Nat → Bool) (s : Step) : Prop :=
  match s.decided with
  | some v => v = s.arm
  | none => σ s.atom = s.arm

/-- The valuation a validated path chooses: each free step's atom gets that
step's arm. -/
def choose : List Step → Nat → Bool
  | [], _ => false
  | s :: rest, a => if s.decided = none ∧ s.atom = a then s.arm else choose rest a

theorem free_atom_mem : ∀ (path : List Step) (s : Step), s ∈ path → s.decided = none →
    s.atom ∈ freeAtoms path := by
  intro path
  induction path with
  | nil => intro s hs; cases hs
  | cons t rest ih =>
      intro s hs hd
      rcases List.mem_cons.mp hs with rfl | hs
      · simp [freeAtoms, hd]
      · have := ih s hs hd
        cases ht : t.decided <;> simp [freeAtoms, ht, this]

theorem choose_free : ∀ (path : List Step), (freeAtoms path).Nodup →
    ∀ s ∈ path, s.decided = none → choose path s.atom = s.arm := by
  intro path
  induction path with
  | nil => intro _ s hs; cases hs
  | cons t rest ih =>
      intro hnd s hs hd
      rcases List.mem_cons.mp hs with rfl | hs
      · simp [choose, hd]
      · by_cases hc : t.decided = none ∧ t.atom = s.atom
        · exfalso
          have hmem := free_atom_mem rest s hs hd
          have : (t.atom :: freeAtoms rest).Nodup := by
            simpa [freeAtoms, hc.1] using hnd
          exact (List.nodup_cons.mp this).1 (hc.2 ▸ hmem)
        · have hnd' : (freeAtoms rest).Nodup := by
            cases ht : t.decided with
            | none =>
                have : (t.atom :: freeAtoms rest).Nodup := by simpa [freeAtoms, ht] using hnd
                exact (List.nodup_cons.mp this).2
            | some _ => simpa [freeAtoms, ht] using hnd
          have hstep : choose (t :: rest) s.atom = choose rest s.atom := by
            simp only [choose]
            split
            · rename_i hyes; exact absurd hyes hc
            · rfl
          rw [hstep]
          exact ih hnd' s hs hd

/-- **A validated path is satisfiable**: the valuation it chooses takes every
one of its arms. -/
theorem a_validated_path_is_satisfiable (path : List Step) (h : validated path) :
    ∀ s ∈ path, takes (choose path) s := by
  intro s hs
  unfold takes
  cases hd : s.decided with
  | some v =>
      have := h.1 s hs
      simp [agrees, hd] at this
      exact this
  | none => exact choose_free path h.2 s hs hd

/-- **A validated finding is reached.** A validated path ends at the
dereference, where the pointer is NULL under its named condition (an
allocation that fails, a lookup that misses, a null constant, a test's null
arm); some valuation of the fresh values takes every arm, so that
dereference is executed (where `QCore.NullDeref.run` faults). -/
theorem a_validated_finding_faults (path : List Step) (h : validated path) :
    ∃ σ : Nat → Bool, ∀ s ∈ path, takes σ s :=
  ⟨choose path, a_validated_path_is_satisfiable path h⟩

/-- **A correlated step is never validated as free**: two free steps over
one value are not a validated path. -/
theorem a_repeated_free_value_is_not_validated (a : Nat) (x y : Bool) :
    ¬ validated [⟨a, none, x⟩, ⟨a, none, y⟩] := by
  intro h
  have := h.2
  simp [freeAtoms] at this

/-! ## Interval decisions over one value -/

/-- One comparison of a value with a constant, and the arm taken. -/
structure Cmp where
  lt : Bool  -- `x < k` when true, `x == k` when false
  k : Int
  arm : Bool
  deriving DecidableEq, Repr

def Cmp.holds (c : Cmp) (x : Int) : Bool :=
  if c.lt then decide (x < c.k) == c.arm else decide (x = c.k) == c.arm

/-- The path's interval reasoning keeps every value that took every arm so
far, and takes an arm only while some value remains. -/
def survivors (vals : List Int) (cs : List Cmp) : List Int :=
  vals.filter (fun x => cs.all (fun c => c.holds x))

/-- **Interval decisions are satisfiable**: a value surviving every decision
takes every arm. -/
theorem interval_path_is_satisfiable (vals : List Int) (cs : List Cmp) (x : Int)
    (hx : x ∈ survivors vals cs) : ∀ c ∈ cs, c.holds x = true := by
  simp only [survivors, List.mem_filter, List.all_eq_true] at hx
  exact hx.2

/-! ## Findings and the verdict -/

/-- `null_triage::triaged_verdict`. -/
def triaged (base : Verdict) (findings : Nat) : Verdict :=
  if findings > 0 then compose base .«unsafe» else base

/-- **Triage never makes a row `Safe`.** -/
theorem triage_never_makes_safe (base : Verdict) (n : Nat) (h : triaged base n = .safe) :
    base = .safe ∧ n = 0 := by
  unfold triaged at h
  split at h
  · cases base <;> simp [compose, rank] at h
  · exact ⟨h, by omega⟩

/-- **`Unsafe` only for a reachable violation or a validated finding.** -/
theorem unsafe_only_for_a_violation_or_a_finding (os : List Outcome) (r : Bool) (n : Nat)
    (h : triaged (verdict os r) n = .«unsafe») :
    os.any (· == .violation true) = true ∨ n > 0 := by
  unfold triaged at h
  split at h
  · right; assumption
  · left
    have := QCore.NullDeref.unsafe_only_for_a_reachable_violation os r h
    simpa using this

/-! ## Declared members -/

/-- A trace event on one declared member: a store of a value proved non-null
or not, or a read. -/
inductive Ev where
  | store (proved : Bool)
  | read
  deriving DecidableEq, Repr

/-- The member's concrete null-ness after a trace (`true`: null), from its
state at construction; an unproved store may store null. -/
def after (start : Bool) (oracle : Nat → Bool) : List Ev → Bool
  | [] => start
  | .store true :: rest => after false oracle rest
  | .store false :: rest => after (oracle rest.length) oracle rest
  | .read :: rest => after start oracle rest

/-- Every read of the trace, with the state it sees. -/
def readsSeeNonNull (start : Bool) (oracle : Nat → Bool) : List Ev → Prop
  | [] => True
  | .read :: rest => start = false ∧ readsSeeNonNull start oracle rest
  | .store true :: rest => readsSeeNonNull false oracle rest
  | .store false :: rest => readsSeeNonNull (oracle rest.length) oracle rest

def everyStoreProved : List Ev → Bool
  | [] => true
  | .store p :: rest => p && everyStoreProved rest
  | .read :: rest => everyStoreProved rest

/-- **The member invariant.** When the construction premise holds (the
member is non-null before the first read) and every store is proved
non-null, every read sees it non-null. -/
theorem member_invariant_holds : ∀ (trace : List Ev) (oracle : Nat → Bool),
    everyStoreProved trace = true → readsSeeNonNull false oracle trace := by
  intro trace oracle
  induction trace with
  | nil => intro _; trivial
  | cons e rest ih =>
      intro h
      cases e with
      | read => exact ⟨rfl, ih h⟩
      | store p =>
          cases p with
          | false => simp [everyStoreProved] at h
          | true => exact ih (by simpa [everyStoreProved] using h)

/-- `null_triage::member_standing`. -/
def memberStanding (examined failed : Nat) : String :=
  if failed > 0 then "withdrawn"
  else if examined = 0 then "declared-assumption"
  else "partly-validated"

/-- **A member declaration is never `validated`**: its construction clause
is a premise no store covers. -/
theorem a_member_declaration_is_never_validated (e f : Nat) :
    memberStanding e f ≠ "validated" := by
  unfold memberStanding
  split
  · decide
  · split <;> decide

/-- **A failed store withdraws it.** -/
theorem a_failed_store_withdraws (e f : Nat) (h : f > 0) : memberStanding e f = "withdrawn" := by
  simp [memberStanding, h]

/-! ## Declared results: rule 13g's fold -/

inductive Evidence where
  | validated | unvalidated | refuted
  deriving DecidableEq, Repr

/-- `null_triage::result_standing`. -/
def resultStanding (units : List Evidence) : String :=
  if units.contains .refuted then "withdrawn"
  else if !units.isEmpty && units.all (· == .validated) then "validated"
  else "declared-assumption"

theorem a_refuting_unit_withdraws (units : List Evidence) (h : Evidence.refuted ∈ units) :
    resultStanding units = "withdrawn" := by
  simp [resultStanding, h]

theorem validated_needs_a_validating_body (units : List Evidence)
    (h : resultStanding units = "validated") : units ≠ [] ∧ ∀ u ∈ units, u = .validated := by
  unfold resultStanding at h
  split at h
  · exact absurd h (by decide)
  · split at h
    · rename_i h2
      simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false_iff,
        List.all_eq_true, beq_iff_eq] at h2
      exact ⟨h2.1, h2.2⟩
    · exact absurd h (by decide)

/-! ## Withdrawal and travel -/

/-- `null_triage::demote`. -/
def demote (v : Verdict) (relied : Bool) : Verdict :=
  if relied then compose v .unknown else v

theorem a_withdrawn_dependency_is_never_safe (v : Verdict) (h : demote v true = .safe ∨ demote v true = .pending) :
    False := by
  cases v <;> simp [demote, compose, rank] at h

/-- **Travel never improves a row**: the verdict after added caller checks is
the join with theirs, never below the row's. -/
theorem travel_never_improves_a_row (base added : Verdict) :
    rank base ≤ rank (compose base added) := by
  cases base <;> cases added <;> simp [compose, rank]

/-! ## Vectors bound to the Rust -/

def admissionName : Admission → String
  | .taken => "taken" | .infeasible => "infeasible" | .unresolved => "unresolved"

/-- Condition (`d0`/`d1` decided, `free`, `corr`), arm, admission: `ADMIT_VECTORS`. -/
def admitVectors : List (String × Bool × String) := [
  ("d1", true, "taken"),
  ("d1", false, "infeasible"),
  ("d0", false, "taken"),
  ("d0", true, "infeasible"),
  ("free", true, "taken"),
  ("free", false, "taken"),
  ("corr", true, "unresolved"),
  ("corr", false, "unresolved")
]

def conditionOf : String → Condition
  | "d1" => .decided true
  | "d0" => .decided false
  | "free" => .free
  | _ => .correlated

theorem admit_vectors_hold :
    admitVectors.all (fun (c, a, o) => admissionName (admit (conditionOf c) a) == o) = true := by
  decide

/-- Examined stores, failed stores, standing: `MEMBER_VECTORS`. -/
def memberVectors : List (Nat × Nat × String) := [
  (0, 0, "declared-assumption"),
  (3, 0, "partly-validated"),
  (3, 1, "withdrawn"),
  (0, 1, "withdrawn")
]

theorem member_vectors_hold :
    memberVectors.all (fun (e, f, s) => memberStanding e f == s) = true := by
  decide

def evidenceOf : String → Evidence
  | "v" => .validated
  | "u" => .unvalidated
  | _ => .refuted

/-- Unit evidence, standing: `RESULT_VECTORS`. -/
def resultVectors : List (List String × String) := [
  ([], "declared-assumption"),
  (["v"], "validated"),
  (["v", "v"], "validated"),
  (["v", "u"], "declared-assumption"),
  (["v", "r"], "withdrawn"),
  (["u"], "declared-assumption")
]

theorem result_vectors_hold :
    resultVectors.all (fun (us, s) => resultStanding (us.map evidenceOf) == s) = true := by
  decide

end QCore.NullTriage
