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
# The null-dereference property (card NULL-SAFETY-PROPERTY, gateway rules 14k–14p)

The abstract rules of `crates/qcore-core/src/null_deref.rs` (the property
`c.null.deref.v1`), of the gateway consumer `crates/qcore-c-gateway/src/null_deref`
and of custody's premise reading `crates/qcore-c-gateway/src/legacy_null_premise.rs`
(USER DECISION 2026-10-10).

* **Nullness.** A pointer's abstract value is non-null (with its proofs), null,
  or possibly null (with its sources). The join of two paths is non-null only
  when both are, null only when both are.
* **The check.** A dereference of a non-null value is discharged, of a null
  value a violation, of a possibly-null value a candidate.
* **Dominating dereference (decision 1).** A dereference that does not fault
  leaves the pointer non-null with its question *moved* to that dereference;
  until the pointer is assigned again its null-ness is exactly the one the
  earlier dereference checked (`moved_question_is_the_origins`). An offset keeps
  null-ness (C11 6.5.6p8).
* **Guards.** The arm of a NULL test on which the pointer is not null proves it
  there (`guard_sound`).
* **The verdict.** `Safe` only when every check is discharged; a candidate is
  never `Safe` (`no_safe_on_an_undecided_pointer`); `Unsafe` only for a
  reachable violation.
* **The clause `quant null-deref #B;`.** The callee's body reads `B` non-null at
  entry exactly when every caller's obligation (the actual proved non-null)
  holds (`caller_obligation_establishes_the_entry`); the inference is sound:
  when every terminating path dereferences `B` before any test or assignment,
  no terminating execution starts with `B` null, so the inferred clause is
  never stronger than the body (`an_inferred_clause_is_necessary`).
* **Custody composition.** Custody reads a dereference through a premised
  binding non-null only with a listed premise
  (`a_relied_on_reading_is_listed`); a tracked, widened, mixed or non-unknown
  binding is never premised; the combined verdict is `Safe` only when the null
  row is (`combined_safe_needs_null_safe`).

## What this is not

A model of C, of Clang or of the custody transfer: that the gateway walk
records a check at every dereference, refines exactly at the tests it reads and
forgets memory facts at every call and pointer store, is a review premise
(trust boundary 28), pinned by the Rust controls
`crates/qcore-c-gateway/tests/null_safety_property.rs` and
`crates/qcore-c-gateway/tests/null_premise_custody.rs`. Proofs and sources are
reduced to what the theorems read; the vectors below bind the Rust decisions
(`crates/qcore-core/tests/null_deref.rs`, `crates/qcore-c-gateway/src/null_premise.rs`).
-/

namespace QCore.NullDeref

/-! ## Nullness, its join and the check -/

/-- A proof of non-null-ness; `moved s` is a dominating dereference at check `s`. -/
inductive Proof where
  | tested | asserted | precondition | nonNullResult | address
  | moved (site : Nat)
  /-- Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15t): every path explored, none
  reaches the dereference NULL (`QCore.NullTriagePaths.exhaustive_search_is_sound`). -/
  | infeasible
  deriving DecidableEq, Repr

/-- Where a possibly-null value may come from. -/
inductive Source where
  | allocation | nullableResult | lookup | uncontracted | formal
  | memoryField | global | nullLiteral | uninitialized | other
  deriving DecidableEq, Repr

inductive Nullness where
  | nonNull (proofs : List Proof)
  | null
  | maybe (sources : List Source)
  deriving DecidableEq, Repr

/-- `Nullness::join`. -/
def join : Nullness → Nullness → Nullness
  | .nonNull a, .nonNull b => .nonNull (a ++ b)
  | .null, .null => .null
  | .maybe a, .maybe b => .maybe (a ++ b)
  | .maybe a, .nonNull _ => .maybe a
  | .nonNull _, .maybe b => .maybe b
  | .maybe a, .null => .maybe (a ++ [.nullLiteral])
  | .null, .maybe b => .maybe (b ++ [.nullLiteral])
  | .nonNull _, .null => .maybe [.nullLiteral]
  | .null, .nonNull _ => .maybe [.nullLiteral]

/-- A concrete pointer is null (`true`) or not; an abstract value is sound for
it when it does not claim what is false. -/
def sound : Nullness → Bool → Prop
  | .nonNull _, isNull => isNull = false
  | .null, isNull => isNull = true
  | .maybe _, _ => True

/-- **The join is sound for a value either path produced**: a value sound
for the first path's reading stays sound for the joined one. -/
theorem join_sound_left (a b : Nullness) (v : Bool) (h : sound a v) : sound (join a b) v := by
  cases a <;> cases b <;> cases v <;> simp_all [join, sound]

theorem join_sound_right (a b : Nullness) (v : Bool) (h : sound b v) : sound (join a b) v := by
  cases a <;> cases b <;> cases v <;> simp_all [join, sound]

/-- The outcome class of one check. -/
inductive Outcome where
  | proved | moved | held | candidate | violation (reachable : Bool) | excluded
  deriving DecidableEq, Repr

def hasMoved : List Proof → Bool
  | [] => false
  | .moved _ :: _ => true
  | _ :: rest => hasMoved rest

/-- `null_deref::check`. -/
def check (n : Nullness) (reachable : Bool) : Outcome :=
  match n with
  | .nonNull ps => if hasMoved ps then .moved else .proved
  | .null => .violation reachable
  | .maybe _ => .candidate

def discharged : Outcome → Bool
  | .proved | .moved | .excluded => true
  | _ => false

theorem a_discharged_check_is_of_a_non_null_value (n : Nullness) (r : Bool)
    (h : discharged (check n r) = true) : ∃ ps, n = .nonNull ps := by
  cases n with
  | nonNull ps => exact ⟨ps, rfl⟩
  | null => simp [check, discharged] at h
  | maybe s => simp [check, discharged] at h

/-- A discharged check is sound: the pointer it reads is not null. -/
theorem check_sound (n : Nullness) (r : Bool) (v : Bool) (hs : sound n v)
    (h : discharged (check n r) = true) : v = false := by
  obtain ⟨ps, rfl⟩ := a_discharged_check_is_of_a_non_null_value n r h
  exact hs

/-! ## Dominating dereferences, offsets and guards on one path -/

/-- What one path does with one pointer variable. -/
inductive Event where
  /-- An assignment of a value the walk reads as `a`. -/
  | assign (a : Nullness)
  /-- A real read or write through the pointer: check `site`. -/
  | deref (site : Nat)
  /-- The non-null arm of a NULL test. -/
  | nonNullArm
  /-- The null arm of a NULL test. -/
  | nullArm
  /-- `B = B ± n`. -/
  | offset
  deriving DecidableEq, Repr

/-- `null_deref::after_dereference`. -/
def afterDereference (n : Nullness) (site : Nat) : Nullness :=
  match n with
  | .nonNull ps => .nonNull ps
  | _ => .nonNull [.moved site]

/-- `null_deref::offset`. -/
def offset : Nullness → Nullness
  | .null => .maybe [.nullLiteral]
  | n => n

/-- `null_deref::refine`: `none` when the arm is infeasible. -/
def refine (n : Nullness) (nonNullArm : Bool) : Option Nullness :=
  match n, nonNullArm with
  | .nonNull ps, true => some (.nonNull ps)
  | .nonNull _, false => none
  | .null, true => none
  | .null, false => some .null
  | .maybe _, true => some (.nonNull [.tested])
  | .maybe _, false => some .null

/-- The walk over one path (`none`: unreachable). -/
def step : Option Nullness → Event → Option Nullness
  | none, _ => none
  | some _, .assign a => some a
  | some n, .deref s => some (afterDereference n s)
  | some n, .nonNullArm => refine n true
  | some n, .nullArm => refine n false
  | some n, .offset => some (offset n)

def walk (start : Option Nullness) (evs : List Event) : Option Nullness :=
  evs.foldl step start

/-- The concrete path: a dereference of a null pointer faults (the execution
stops); a test arm the value contradicts is not taken; an offset keeps
null-ness. `run v evs = some v'` is an execution that reaches the end. -/
def run : Bool → List Event → (Nat → Bool) → Option Bool
  | v, [], _ => some v
  | _, .assign _ :: rest, oracle => run (oracle rest.length) rest oracle
  | v, .deref _ :: rest, oracle => if v then none else run v rest oracle
  | v, .nonNullArm :: rest, oracle => if v then none else run v rest oracle
  | v, .nullArm :: rest, oracle => if v then run v rest oracle else none
  | v, .offset :: rest, oracle => run v rest oracle

/-- Every assignment's concrete value agrees with what the walk reads. -/
def oracleSound : List Event → (Nat → Bool) → Prop
  | [], _ => True
  | .assign a :: rest, oracle => sound a (oracle rest.length) ∧ oracleSound rest oracle
  | _ :: rest, oracle => oracleSound rest oracle

theorem afterDereference_sound (n : Nullness) (s : Nat) : sound (afterDereference n s) false := by
  cases n <;> rfl

theorem offset_sound (n : Nullness) (v : Bool) (h : sound n v) : sound (offset n) v := by
  cases n <;> simp_all [offset, sound]

/-- **Transfer soundness.** On every execution that reaches the end of the
path, the walk's value is sound (the walk never says non-null of a null
pointer); an execution the walk says is unreachable never reaches the end. -/
theorem walk_sound : ∀ (evs : List Event) (n : Nullness) (v : Bool) (oracle : Nat → Bool),
    sound n v → oracleSound evs oracle → ∀ v', run v evs oracle = some v' →
    ∃ n', walk (some n) evs = some n' ∧ sound n' v'
  | [], n, v, _, hs, _, v', hr => by
      simp [run] at hr; subst hr; exact ⟨n, rfl, hs⟩
  | .assign a :: rest, _, _, oracle, _, ho, v', hr => by
      simp only [oracleSound] at ho
      simp only [run] at hr
      exact walk_sound rest a (oracle rest.length) oracle ho.1 ho.2 v' hr
  | .deref s :: rest, n, v, oracle, _, ho, v', hr => by
      simp only [oracleSound] at ho
      simp only [run] at hr
      cases v with
      | true => simp at hr
      | false =>
          simp at hr
          exact walk_sound rest (afterDereference n s) false oracle
            (afterDereference_sound n s) ho v' hr
  | .nonNullArm :: rest, n, v, oracle, hs, ho, v', hr => by
      simp only [oracleSound] at ho
      simp only [run] at hr
      cases v with
      | true => simp at hr
      | false =>
          simp at hr
          cases n with
          | nonNull ps => exact walk_sound rest (.nonNull ps) false oracle rfl ho v' hr
          | null => simp [sound] at hs
          | maybe s => exact walk_sound rest (.nonNull [.tested]) false oracle rfl ho v' hr
  | .nullArm :: rest, n, v, oracle, hs, ho, v', hr => by
      simp only [oracleSound] at ho
      simp only [run] at hr
      cases v with
      | false => simp at hr
      | true =>
          simp at hr
          cases n with
          | nonNull ps => simp [sound] at hs
          | null => exact walk_sound rest .null true oracle rfl ho v' hr
          | maybe s => exact walk_sound rest .null true oracle rfl ho v' hr
  | .offset :: rest, n, v, oracle, hs, ho, v', hr => by
      simp only [oracleSound] at ho
      simp only [run] at hr
      exact walk_sound rest (offset n) v oracle (offset_sound n v hs) ho v' hr

/-- **Guard soundness.** The non-null arm of a NULL test proves the pointer
there: an execution taking it holds a non-null pointer, and the walk says so. -/
theorem guard_sound (n : Nullness) (v : Bool) (hs : sound n v) (v' : Bool) (oracle : Nat → Bool)
    (h : run v [.nonNullArm] oracle = some v') :
    v' = false ∧ ∃ ps, walk (some n) [.nonNullArm] = some (.nonNull ps) := by
  cases v with
  | true => simp [run] at h
  | false =>
      simp [run] at h
      refine ⟨h, ?_⟩
      cases n with
      | nonNull ps => exact ⟨ps, rfl⟩
      | null => simp [sound] at hs
      | maybe s => exact ⟨[.tested], rfl⟩

/-- No assignment on a path. -/
def noAssign : List Event → Bool
  | [] => true
  | .assign _ :: _ => false
  | _ :: rest => noAssign rest

/-- Without an assignment the null-ness the path ends with is the one it
started with. -/
theorem nullness_kept_without_assignment : ∀ (evs : List Event) (v : Bool) (oracle : Nat → Bool),
    noAssign evs = true → ∀ v', run v evs oracle = some v' → v' = v
  | [], _, _, _, v', h => by simp [run] at h; exact h.symm
  | .assign _ :: _, _, _, hna, _, _ => by simp [noAssign] at hna
  | .deref _ :: rest, v, oracle, hna, v', h => by
      simp only [run] at h
      cases v <;> simp at h
      exact nullness_kept_without_assignment rest false oracle hna v' h
  | .nonNullArm :: rest, v, oracle, hna, v', h => by
      simp only [run] at h
      cases v <;> simp at h
      exact nullness_kept_without_assignment rest false oracle hna v' h
  | .nullArm :: rest, v, oracle, hna, v', h => by
      simp only [run] at h
      cases v <;> simp at h
      exact nullness_kept_without_assignment rest true oracle hna v' h
  | .offset :: rest, v, oracle, hna, v', h => by
      simp only [run] at h
      exact nullness_kept_without_assignment rest v oracle hna v' h

/-- **Decision 1: the moved question is exactly the earlier dereference's
obligation.** After a dereference at check `s` and no assignment since, the
pointer's null-ness at a later dereference is the one check `s` judged: the
later check is discharged whenever check `s` was (the dereference at `s` did not
fault), and it says nothing the earlier one did not. -/
theorem moved_question_is_the_origins (s : Nat) (rest : List Event) (v : Bool)
    (oracle : Nat → Bool) (hna : noAssign rest = true) (v' : Bool)
    (h : run v (.deref s :: rest) oracle = some v') : v' = v ∧ v = false := by
  simp only [run] at h
  cases v with
  | true => simp at h
  | false =>
      simp at h
      exact ⟨nullness_kept_without_assignment rest false oracle hna v' h, rfl⟩

/-! ## The verdict -/

inductive Verdict where
  | safe | pending | unknown | «unsafe»
  deriving DecidableEq, Repr

/-- `null_deref::null_verdict` on the outcome classes and whether a
function-level reason remains. -/
def verdict (outcomes : List Outcome) (reasons : Bool) : Verdict :=
  if outcomes.any (· == .violation true) then .«unsafe»
  else if reasons || outcomes.any (fun o => o == .candidate || o == .violation false) then .unknown
  else if outcomes.any (· == .held) then .pending
  else .safe

theorem a_safe_function_discharged_every_check (os : List Outcome) (r : Bool)
    (h : verdict os r = .safe) : r = false ∧ ∀ o ∈ os, discharged o = true := by
  unfold verdict at h
  split at h
  · cases h
  · split at h
    · cases h
    · split at h
      · cases h
      · rename_i h1 h2 h3
        simp only [Bool.or_eq_true, not_or, List.any_eq_true, not_exists, not_and] at h1 h2 h3
        refine ⟨by simpa using h2.1, fun o ho => ?_⟩
        cases o with
        | proved | moved | excluded => rfl
        | held => exact absurd (by rfl) (h3 _ ho)
        | candidate => exact absurd (by decide) (h2.2 _ ho).1
        | violation r =>
            cases r
            · exact absurd (by decide) (h2.2 _ ho).2
            · exact absurd (by rfl) (h1 _ ho)

/-- **No `Safe` on an undecided pointer.** -/
theorem no_safe_on_an_undecided_pointer (os : List Outcome) (r : Bool) (n : Nullness) (reach : Bool)
    (hn : ∃ s, n = .maybe s) (hin : check n reach ∈ os) : verdict os r ≠ .safe := by
  intro hs
  obtain ⟨s, rfl⟩ := hn
  have := (a_safe_function_discharged_every_check os r hs).2 _ hin
  simp [check, discharged] at this

/-- `Unsafe` only for a reachable violation. -/
theorem unsafe_only_for_a_reachable_violation (os : List Outcome) (r : Bool)
    (h : verdict os r = .«unsafe») : .violation true ∈ os := by
  unfold verdict at h
  split at h
  · rename_i hv
    simp only [List.any_eq_true] at hv
    obtain ⟨o, ho, he⟩ := hv
    simp at he
    exact he ▸ ho
  · split at h <;> (try split at h) <;> cases h

/-! ## The clause `quant null-deref #B;` -/

/-- The callee's entry value of `B` under its clause. -/
def entry : Nullness := .nonNull [.precondition]

/-- **Caller and callee obligations.** A caller's check of the actual at the
clause's position is discharged exactly when the actual is non-null on every
path; then the callee's entry reading of `B` is sound for every value the
caller can pass. -/
theorem caller_obligation_establishes_the_entry (actual : Nullness) (r : Bool) (v : Bool)
    (hs : sound actual v) (h : discharged (check actual r) = true) : sound entry v :=
  check_sound actual r v hs h

/-- Whether the formal's first event on a path is a dereference (offsets
before it do not count: an offset keeps null-ness). -/
def firstIsDeref : List Event → Bool
  | [] => false
  | .deref _ :: _ => true
  | .offset :: rest => firstIsDeref rest
  | _ => false

/-- **Inference soundness.** When a path dereferences the formal before any test
or assignment of it, no execution of that path that starts with the formal null
reaches its end: the inferred clause asks callers for nothing the body did not
already need. -/
theorem an_inferred_clause_is_necessary : ∀ (evs : List Event) (oracle : Nat → Bool),
    firstIsDeref evs = true → run true evs oracle = none
  | [], _, h => by simp [firstIsDeref] at h
  | .deref _ :: _, _, _ => by simp [run]
  | .offset :: rest, oracle, h => by
      simp only [firstIsDeref] at h
      simp only [run]
      exact an_inferred_clause_is_necessary rest oracle h
  | .assign _ :: _, _, h => by simp [firstIsDeref] at h
  | .nonNullArm :: _, _, h => by simp [firstIsDeref] at h
  | .nullArm :: _, _, h => by simp [firstIsDeref] at h

/-- What a path did first with the formal (`null_deref::First`). -/
inductive First where
  | pending | dereferenced | other
  deriving DecidableEq, Repr

def First.deref : First → First
  | .pending => .dereferenced
  | f => f

def First.test : First → First
  | .pending => .other
  | f => f

/-- `null_deref::inferred`: at least one terminating path and every one
dereferenced first. -/
def inferred (exits : List First) : Bool :=
  !exits.isEmpty && exits.all (· == .dereferenced)

/-- The Rust's `first_after`, on the event names `deref` and `test`. -/
def firstAfter (events : List String) : First :=
  events.foldl (fun f e => if e == "deref" then f.deref else f.test) .pending

/-! ## Custody composition (rule 14n) -/

/-- A custody binding, as the dereference obligation reads it. -/
structure Binding where
  targetsEmpty : Bool
  unknown : Bool
  mayBeNull : Bool
  identity : Bool
  widened : Bool
  deriving DecidableEq, Repr

/-- `null_premise::premised_shape`: the target-free, possibly-null borrowed
alias decision 1 reads non-null at a dereference. -/
def premised (b : Binding) : Bool :=
  b.targetsEmpty && b.unknown && b.mayBeNull && b.identity && !b.widened

/-- The obligation with decision 1's reading: `some premises` when it is met,
with the premises it relied on; `none` when it refuses. `resolves` is the
obligation without the reading. -/
def admit (resolves : Bool) (b : Binding) : Option (List Unit) :=
  if resolves then some [] else if premised b then some [()] else none

/-- **A relied-on non-null reading is always a listed premise.** -/
theorem a_relied_on_reading_is_listed (resolves : Bool) (b : Binding) (ps : List Unit)
    (h : admit resolves b = some ps) (hr : resolves = false) : ps ≠ [] := by
  simp [admit, hr] at h
  obtain ⟨_, rfl⟩ := h
  simp

theorem a_tracked_binding_is_never_premised (b : Binding) (h : b.targetsEmpty = false) :
    premised b = false := by
  simp [premised, h]

theorem a_known_or_widened_binding_is_never_premised (b : Binding)
    (h : b.unknown = false ∨ b.widened = true ∨ b.identity = false) : premised b = false := by
  rcases h with h | h | h <;> simp [premised, h]

/-- The combined verdict of a request naming custody and null: the join. -/
def rank : Verdict → Nat
  | .safe => 0 | .pending => 1 | .unknown => 2 | .«unsafe» => 3

def compose (custody null : Verdict) : Verdict :=
  if rank custody ≥ rank null then custody else null

/-- **The combined verdict is `Safe` only when the null row is.** A custody
row resting on a null premise is combined-`Safe` only once the null property
proves its checks. -/
theorem combined_safe_needs_null_safe (c n : Verdict) (h : compose c n = .safe) :
    n = .safe ∧ c = .safe := by
  cases c <;> cases n <;> simp_all [compose, rank]

/-! ## Vectors bound to the Rust -/

/-- Nullness by class (`0` non-null, `1` null, `2` possibly null), the joined
class: the Rust's `JOIN_VECTORS`. -/
def classOf : Nullness → Nat
  | .nonNull _ => 0
  | .null => 1
  | .maybe _ => 2

def ofClass : Nat → Nullness
  | 0 => .nonNull [.tested]
  | 1 => .null
  | _ => .maybe [.formal]

def joinVectors : List (Nat × Nat × Nat) := [
  (0, 0, 0),
  (0, 1, 2),
  (0, 2, 2),
  (1, 0, 2),
  (1, 1, 1),
  (1, 2, 2),
  (2, 0, 2),
  (2, 1, 2),
  (2, 2, 2)
]

theorem join_vectors_hold :
    joinVectors.all (fun (a, b, j) => classOf (join (ofClass a) (ofClass b)) == j) = true := by
  decide

/-- The check by class and reachability: `CHECK_VECTORS`. -/
def checkVectors : List (Nat × Bool × String) := [
  (0, true, "proved"),
  (0, false, "proved"),
  (1, true, "violation"),
  (1, false, "violation-reachability-unresolved"),
  (2, true, "candidate"),
  (2, false, "candidate")
]

def outcomeName : Outcome → String
  | .proved => "proved"
  | .moved => "moved"
  | .held => "held"
  | .candidate => "candidate"
  | .violation true => "violation"
  | .violation false => "violation-reachability-unresolved"
  | .excluded => "excluded"

theorem check_vectors_hold :
    checkVectors.all (fun (c, r, o) => outcomeName (check (ofClass c) r) == o) = true := by
  decide

def outcomeOf : String → Outcome
  | "proved" => .proved
  | "moved" => .moved
  | "held" => .held
  | "candidate" => .candidate
  | "violation" => .violation true
  | "violation-reachability-unresolved" => .violation false
  | _ => .excluded

def verdictName : Verdict → String
  | .safe => "Safe" | .pending => "Pending" | .unknown => "Unknown" | .«unsafe» => "Unsafe"

/-- Outcome lists, a function-level reason, the verdict: `VERDICT_VECTORS`. -/
def verdictVectors : List (List String × Bool × String) := [
  ([], false, "Safe"),
  (["proved", "moved", "excluded"], false, "Safe"),
  (["proved"], true, "Unknown"),
  (["proved", "candidate"], false, "Unknown"),
  (["held"], false, "Pending"),
  (["held", "candidate"], false, "Unknown"),
  (["violation-reachability-unresolved"], false, "Unknown"),
  (["candidate", "violation"], false, "Unsafe")
]

theorem verdict_vectors_hold :
    verdictVectors.all (fun (os, r, v) => verdictName (verdict (os.map outcomeOf) r) == v) = true := by
  decide

/-- Event names, the first event: `FIRST_VECTORS`. -/
def firstVectors : List (List String × First) := [
  ([], .pending),
  (["deref"], .dereferenced),
  (["deref", "test"], .dereferenced),
  (["test", "deref"], .other),
  (["test"], .other)
]

theorem first_vectors_hold :
    firstVectors.all (fun (es, f) => firstAfter es == f) = true := by
  decide

/-- Exit sets, inferred: `INFERRED_VECTORS`. -/
def inferredVectors : List (List First × Bool) := [
  ([], false),
  ([.dereferenced], true),
  ([.dereferenced, .other], false),
  ([.pending], false)
]

theorem inferred_vectors_hold :
    inferredVectors.all (fun (es, i) => inferred es == i) = true := by
  decide

/-- Binding shapes, premised: `PREMISED_VECTORS` (targets empty, unknown,
possibly null, identity, widened). -/
def premisedVectors : List (Bool × Bool × Bool × Bool × Bool × Bool) := [
  (true, true, true, true, false, true),
  (false, true, true, true, false, false),
  (true, false, true, true, false, false),
  (true, true, false, true, false, false),
  (true, true, true, false, false, false),
  (true, true, true, true, true, false)
]

theorem premised_vectors_hold :
    premisedVectors.all (fun (t, u, m, i, w, p) => premised ⟨t, u, m, i, w⟩ == p) = true := by
  decide

end QCore.NullDeref
