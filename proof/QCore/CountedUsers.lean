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
# Counted users (card W45-FB-RETRY-LIFETIME)

The abstract rule implemented by `legacy_c::counted_users`
(`crates/qcore-c-gateway/src/legacy_counted_users.rs`), gateway constraint
10e, contract specification §C04.7.

An object counts its users in a member. An addition is an obligation, a drop
discharges it, and a drop without the release check leaves the object possibly
unreferenced until someone runs the check. The pass enumerates a body's paths
and reads, per path, the operations on one tracked formal:

* `UOp` is one operation: an addition, a drop (with or without the release
  check), the check alone, a call that changes no user (`neutral`), and an
  `opaque` call, whose effect is not known.
* `delta` is a path's net count: the sum of the operations' steps, or `none`
  once an opaque call occurs.
* `pending` is whether a drop below the entry count is still waiting for a
  release check at the end of the path.
* A clause `(ret, d)` states that every path returning `ret` has count `d`.
  `clauseVerdict` decides it over the paths: refuted when a path that returns
  `ret` has a known count other than `d`, undecided when such a path's count is
  not known, held otherwise.

## What this is not

A model of the decision. Path enumeration, the pruning of decided branches,
attribution of an operation to an unrebound formal and the correspondence with
the compiled Rust are review premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/retry_lifetime.rs`. The frame premise UC-FRAME —
that a declaration with no counted-user clause changes no count — is stated,
not proved. A refutation is a provisional `Unsafe` candidate: branch
feasibility beyond the decided conditions is not proved, and nothing here is a
statement about Fluent Bit's behaviour.
-/

namespace QCore.CountedUsers

/-- One operation on the tracked formal's count. -/
inductive UOp where
  | add
  | drop
  | dropChecked
  | check
  | neutral
  | opaque
  deriving DecidableEq, Repr

/-- The step one operation contributes, or `none` when it is not known. -/
def stepOf : UOp → Option Int
  | .add => some 1
  | .drop => some (-1)
  | .dropChecked => some (-1)
  | .check => some 0
  | .neutral => some 0
  | .opaque => none

/-- A path's net count: the sum of its steps, or `none` once one is unknown. -/
def delta : List UOp → Option Int
  | [] => some 0
  | op :: ops =>
    match stepOf op, delta ops with
    | some step, some rest => some (step + rest)
    | _, _ => none

/-- Whether a drop below the entry count still waits for a release check at the
end of the path. `count` is the count so far; a drop that only rolls back an
addition of this path owes no check of its own. -/
def pendingFrom : Int → Bool → List UOp → Bool
  | _, owed, [] => owed
  | count, _, .add :: ops => pendingFrom (count + 1) false ops
  | count, owed, .drop :: ops => pendingFrom (count - 1) (owed || count - 1 < 0) ops
  | count, _, .dropChecked :: ops => pendingFrom (count - 1) false ops
  | count, _, .check :: ops => pendingFrom count false ops
  | count, owed, .neutral :: ops => pendingFrom count owed ops
  | count, _, .opaque :: ops => pendingFrom count false ops

def pending (ops : List UOp) : Bool :=
  pendingFrom 0 false ops

/-- One path to an exit: its operations and the value it returns. -/
structure Path where
  ops : List UOp
  returned : Int
  deriving DecidableEq, Repr

/-- A clause's verdict. -/
inductive ClauseVerdict where
  | held
  | refuted
  | undecided
  deriving DecidableEq, Repr

/-- What one path says about the clause `(ret, d)`. -/
def pathVerdict (ret d : Int) (path : Path) : ClauseVerdict :=
  if path.returned = ret then
    match delta path.ops with
    | some found => if found = d then .held else .refuted
    | none => .undecided
  else .held

/-- Refuted beats undecided beats held. -/
def combine : ClauseVerdict → ClauseVerdict → ClauseVerdict
  | .refuted, _ => .refuted
  | _, .refuted => .refuted
  | .undecided, _ => .undecided
  | _, .undecided => .undecided
  | .held, .held => .held

def clauseVerdict (ret d : Int) : List Path → ClauseVerdict
  | [] => .held
  | path :: paths => combine (pathVerdict ret d path) (clauseVerdict ret d paths)

-- Counted-reference preservation: a path's net count is the sum of its
-- operations' summaries, over any split of the path.

theorem delta_append (first second : List UOp) (a b : Int)
    (h₁ : delta first = some a) (h₂ : delta second = some b) :
    delta (first ++ second) = some (a + b) := by
  induction first generalizing a with
  | nil =>
    simp [delta] at h₁
    subst h₁
    simp [h₂]
  | cons op ops ih =>
    simp only [delta] at h₁
    cases hs : stepOf op with
    | none => simp [hs] at h₁
    | some step =>
      cases hr : delta ops with
      | none => simp [hs, hr] at h₁
      | some rest =>
        simp [hs, hr] at h₁
        subst h₁
        simp [delta, hs, ih rest hr, Int.add_assoc]

/-- An opaque call anywhere leaves the count unknown. -/
theorem an_opaque_call_leaves_the_count_unknown (first second : List UOp) :
    delta (first ++ .opaque :: second) = none := by
  induction first with
  | nil => simp [delta, stepOf]
  | cons op ops ih =>
    simp only [List.cons_append, delta]
    rw [ih]
    cases stepOf op <;> rfl

/-- Without opaque calls the count is additions minus drops. -/
def adds : List UOp → Int
  | [] => 0
  | .add :: ops => adds ops + 1
  | _ :: ops => adds ops

def drops : List UOp → Int
  | [] => 0
  | .drop :: ops => drops ops + 1
  | .dropChecked :: ops => drops ops + 1
  | _ :: ops => drops ops

theorem delta_counts (ops : List UOp) (h : UOp.opaque ∉ ops) :
    delta ops = some (adds ops - drops ops) := by
  induction ops with
  | nil => rfl
  | cons op ops ih =>
    have hop : op ≠ .opaque := fun e => h (e ▸ List.mem_cons_self)
    have hrest : UOp.opaque ∉ ops := fun m => h (List.mem_cons_of_mem _ m)
    rw [delta, ih hrest]
    cases op <;> simp_all [stepOf, adds, drops] <;> omega

-- A failure exit with a positive residual is never `held`.

theorem a_positive_residual_refutes_a_restored_clause (ret : Int) (path : Path)
    (paths : List Path) (found : Int) (hret : path.returned = ret)
    (hfound : delta path.ops = some found) (hpos : 0 < found) :
    clauseVerdict ret 0 (path :: paths) = .refuted := by
  have hne : found ≠ 0 := by omega
  simp [clauseVerdict, pathVerdict, hret, hfound, hne, combine]

theorem a_positive_residual_is_not_held (ret : Int) (path : Path) (paths : List Path)
    (found : Int) (hret : path.returned = ret) (hfound : delta path.ops = some found)
    (hpos : 0 < found) : clauseVerdict ret 0 (path :: paths) ≠ .held := by
  rw [a_positive_residual_refutes_a_restored_clause ret path paths found hret hfound hpos]
  decide

-- `held` requires every path that returns the clause's value to have a known
-- count equal to the clause's.

theorem combine_held (one other : ClauseVerdict) (h : combine one other = .held) :
    one = .held ∧ other = .held := by
  cases one <;> cases other <;> simp_all [combine]

theorem held_means_every_returning_path_agrees (ret d : Int) (paths : List Path)
    (h : clauseVerdict ret d paths = .held) :
    ∀ path ∈ paths, path.returned = ret → delta path.ops = some d := by
  induction paths with
  | nil => intro path hmem; cases hmem
  | cons first rest ih =>
    intro path hmem hret
    have ⟨hfirst, hrest⟩ := combine_held _ _ h
    rcases List.mem_cons.mp hmem with heq | hmem'
    · rw [heq] at hret ⊢
      unfold pathVerdict at hfirst
      simp only [hret, ↓reduceIte] at hfirst
      cases hd : delta first.ops with
      | none => rw [hd] at hfirst; cases hfirst
      | some found =>
        by_cases hfd : found = d
        · rw [hfd]
        · simp [hd, hfd] at hfirst
    · exact ih hrest path hmem' hret

/-- An opaque call on a path that returns the clause's value keeps it from
being held. -/
theorem an_opaque_call_is_never_held (ret d : Int) (first second : List UOp)
    (paths : List Path) :
    clauseVerdict ret d (⟨first ++ .opaque :: second, ret⟩ :: paths) ≠ .held := by
  intro h
  have := held_means_every_returning_path_agrees ret d _ h ⟨first ++ .opaque :: second, ret⟩
    List.mem_cons_self rfl
  rw [an_opaque_call_leaves_the_count_unknown] at this
  cases this

-- The release check.

/-- The C3 shape: a drop below the entry count with no re-addition and no
check is still pending at the exit. -/
theorem an_unchecked_drop_below_entry_is_pending :
    pending [.drop, .neutral] = true := by
  decide

/-- A rollback of this path's own addition owes no check of its own. -/
theorem a_rollback_owes_no_check : pending [.add, .drop] = false := by
  decide

/-- A re-addition or a check discharges the pending check. -/
theorem a_readdition_discharges_the_check : pending [.drop, .add] = false := by
  decide

theorem a_check_discharges_the_check : pending [.drop, .check] = false := by
  decide

-- The card's three shapes, as the pass reads them.

/-- C2: add, a neutral enqueue that fails, return -1. -/
def c2Failure : Path := ⟨[.add, .neutral], -1⟩
def c2Success : Path := ⟨[.add, .neutral], 0⟩

theorem the_c2_shape_is_refuted :
    clauseVerdict (-1) 0 [c2Failure, c2Success] = .refuted := by
  decide

theorem the_c2_success_arm_holds :
    clauseVerdict 0 1 [c2Failure, c2Success] = .held := by
  decide

/-- N2: add, a dispatch that fails, a rollback, and a return of 0. -/
def n2Failure : Path := ⟨[.add, .neutral, .drop], 0⟩
def n2Success : Path := ⟨[.add, .neutral], 0⟩

theorem the_n2_shape_is_refuted :
    clauseVerdict 0 1 [n2Failure, n2Success] = .refuted := by
  decide

theorem the_n2_rollback_owes_no_check : pending n2Failure.ops = false := by
  decide

/-- A balanced body holds both clauses. -/
def balancedFailure : Path := ⟨[.add, .neutral, .drop, .check], -1⟩
def balancedSuccess : Path := ⟨[.add, .neutral], 0⟩

theorem a_balanced_body_holds :
    clauseVerdict (-1) 0 [balancedFailure, balancedSuccess] = .held ∧
      clauseVerdict 0 1 [balancedFailure, balancedSuccess] = .held := by
  decide

/-! W46-FLUENT-EVENT-CONTRACT-GAPS: a body-grounded scalar or status helper is
classified as `neutral` only after reviewing its complete callees. The model
does not prove that classification for Fluent Bit C; it proves the effect of a
faithful classification and keeps an opaque lifecycle call undecided. -/

/-- Calling a neutral helper before a path cannot alter its known net count. -/
theorem a_neutral_helper_preserves_delta (ops : List UOp) :
    delta (.neutral :: ops) = delta ops := by
  cases h : delta ops with
  | none => simp [delta, stepOf, h]
  | some n => simp [delta, stepOf, h]

/-- A neutral helper cannot discharge a release check already owed. -/
theorem a_neutral_helper_preserves_pending (count : Int) (owed : Bool)
    (ops : List UOp) :
    pendingFrom count owed (.neutral :: ops) = pendingFrom count owed ops := by
  rfl

/-- An unmodeled flush lifecycle call after neutral helpers remains unknown. -/
theorem an_opaque_flush_after_neutral_is_unknown :
    delta [.neutral, .neutral, .opaque] = none := by
  decide

/-! W46-FLUENT-ROUTING-QUERY-CONTRACTS: the five reviewed query bodies are
    represented by neutral operations. This is a model of the declared premise,
    not a proof that arbitrary Fluent Bit C or callbacks have that effect. -/

/-- Five classified queries preserve the caller's known counted-user delta. -/
theorem five_routing_queries_preserve_delta (ops : List UOp) :
    delta ([.neutral, .neutral, .neutral, .neutral, .neutral] ++ ops) = delta ops := by
  simp only [List.cons_append, List.nil_append]
  rw [a_neutral_helper_preserves_delta, a_neutral_helper_preserves_delta,
    a_neutral_helper_preserves_delta, a_neutral_helper_preserves_delta,
    a_neutral_helper_preserves_delta]

/-- Classifying queries does not decide a later opaque routing lifecycle call. -/
theorem opaque_routing_after_queries_remains_unknown :
    delta [.neutral, .neutral, .neutral, .neutral, .neutral, .opaque] = none := by
  decide

end QCore.CountedUsers
