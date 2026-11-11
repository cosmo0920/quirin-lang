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
# Body evidence for bounds declarations (card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS, rules 14f–14g)

The abstract rules implemented by `crates/qcore-core/src/bounds_body_standing.rs`
(coverage, a unit's outcome, the run's standing and the Safe roll-up) and
used by `crates/qcore-c-gateway/src/index_extent/body_evidence.rs` (the
phase-one check of every declared body a unit defines, header definitions
included) and `crates/qcore-audit/src/bounds_evidence.rs` (the fold over the
run's units), contract specification C09.1 in C06.5's shape.

* **Coverage.** A body whose own row is `Safe` under its in-force declaration
  and does not refute it covers its quant checks and preconditions; a frame
  clause is covered only by the positive frame check of its parameter; a
  derived relation, a return extent, `bounds_preserves_extents()` and a global
  extent are covered by nothing. A refuting body covers nothing.
* **One unit.** Held: no evidence. Refuted: a failure. Some clause covered:
  validated with the covered clauses. Otherwise unvalidated.
* **The run.** `held` passes through; `validated` only when some unit examined
  a body, every examining unit validated it and every clause is covered in
  every one; `partlyValidated` when every unit validated it and some, not all,
  clauses are covered everywhere; otherwise a `declaredAssumption`.
* **Roll-up.** A Safe row rests only on validated declarations exactly when
  each declaration it rests on is validated; a declaration with no standing is
  an assumption.

## What this is not

A model of the C bodies, of the bounds consumer's row, or of the frame walk:
that a `Safe` row means every access through a quant-checked formal was decided
inside (rule 12l(2)), and that the frame walk's syntactic conditions imply the
closed frame, are review premises (trust boundary 28), pinned by the Rust tests
`crates/qcore-c-gateway/tests/bounds_body_evidence.rs`. Clause labels are
natural numbers, as in the Rust they are the clause texts of one declaration.
-/

namespace QCore.BoundsBodyStanding

/-- What can cover a clause. -/
inductive ClauseKind where
  | quantCheck | precondition | frame | relation | returnExtent | preserves | globalExtent
  deriving DecidableEq, Repr

/-- The phase-one check of one body under its declaration. -/
structure BodyCheck where
  safe : Bool
  refuted : Bool
  deriving DecidableEq, Repr

inductive Evidence where
  | body | frame
  deriving DecidableEq, Repr

/-- `bounds_body_standing::covers`. -/
def covers (k : ClauseKind) (b : BodyCheck) (frameOk : Bool) : Option Evidence :=
  if b.refuted then none
  else match k with
    | .quantCheck => if b.safe then some .body else none
    | .precondition => if b.safe then some .body else none
    | .frame => if frameOk then some .frame else none
    | _ => none

theorem a_refuting_body_covers_nothing (k : ClauseKind) (b : BodyCheck) (f : Bool)
    (h : b.refuted = true) : covers k b f = none := by
  simp [covers, h]

theorem nothing_covers_these (b : BodyCheck) (f : Bool) :
    covers .relation b f = none ∧ covers .returnExtent b f = none ∧
      covers .preserves b f = none ∧ covers .globalExtent b f = none := by
  cases hr : b.refuted <;> simp [covers, hr]

/-- Body evidence needs a `Safe`, unrefuted body. -/
theorem body_evidence_needs_a_safe_body (k : ClauseKind) (b : BodyCheck) (f : Bool)
    (h : covers k b f = some .body) : b.safe = true ∧ b.refuted = false := by
  cases hr : b.refuted <;> cases hs : b.safe <;> cases k <;> cases f <;>
    simp_all [covers]

/-- Frame evidence covers only a frame clause, and only with the frame check. -/
theorem frame_evidence_needs_the_frame_check (k : ClauseKind) (b : BodyCheck) (f : Bool)
    (h : covers k b f = some .frame) : k = .frame ∧ f = true := by
  cases hr : b.refuted <;> cases hs : b.safe <;> cases k <;> cases f <;>
    simp_all [covers]

/-- A `Safe` body alone never covers a frame clause. -/
theorem a_safe_body_alone_never_covers_a_frame (b : BodyCheck) :
    covers .frame b false = none := by
  cases hr : b.refuted <;> simp [covers, hr]

/-! ## One unit -/

inductive UnitOutcome where
  | held
  | failed
  | validated (covered : List Nat)
  | unvalidated
  deriving DecidableEq, Repr

/-- `bounds_body_standing::unit_outcome`. -/
def unitOutcome (held refuted : Bool) (covered : List Nat) : UnitOutcome :=
  if held then .held
  else if refuted then .failed
  else if covered = [] then .unvalidated
  else .validated covered

theorem a_held_unit_keeps_no_evidence (r : Bool) (cs : List Nat) :
    unitOutcome true r cs = .held := by
  simp [unitOutcome]

theorem a_refuting_unit_fails (cs : List Nat) : unitOutcome false true cs = .failed := by
  simp [unitOutcome]

/-! ## The run -/

inductive Standing where
  | held
  | validated
  | partlyValidated (uncovered : List Nat)
  | declaredAssumption
  deriving DecidableEq, Repr

def UnitOutcome.isValidated : UnitOutcome → Bool
  | .validated _ => true
  | _ => false

def UnitOutcome.covers (c : Nat) : UnitOutcome → Bool
  | .validated cs => cs.contains c
  | _ => false

/-- A clause covered in every unit. -/
def everywhere (us : List UnitOutcome) (c : Nat) : Bool := us.all (UnitOutcome.covers c)

/-- `bounds_body_standing::standing`. -/
def standing (held : Bool) (cs : List Nat) (us : List UnitOutcome) : Standing :=
  if held || us.contains .held then .held
  else if us.isEmpty || cs.isEmpty then .declaredAssumption
  else if !us.all UnitOutcome.isValidated then .declaredAssumption
  else
    let uncovered := cs.filter (fun c => !everywhere us c)
    if uncovered.isEmpty then .validated
    else if uncovered.length = cs.length then .declaredAssumption
    else .partlyValidated uncovered

theorem a_held_declaration_is_held (cs : List Nat) (us : List UnitOutcome) :
    standing true cs us = .held := by
  simp [standing]

theorem no_examined_body_is_a_declared_assumption (cs : List Nat) :
    standing false cs [] = .declaredAssumption := by
  simp [standing]

/-- `validated` needs an examined body, every examining unit validated, and
every clause covered in every one. -/
theorem validated_needs_every_unit_and_clause {h : Bool} {cs : List Nat}
    {us : List UnitOutcome} (v : standing h cs us = .validated) :
    us ≠ [] ∧ (∀ u ∈ us, u.isValidated = true) ∧ ∀ c ∈ cs, ∀ u ∈ us, u.covers c = true := by
  unfold standing at v
  cases hh : (h || us.contains .held) <;> simp only [hh, Bool.false_eq_true, ↓reduceIte] at v
  · cases he : (us.isEmpty || cs.isEmpty) <;> simp only [he, Bool.false_eq_true, ↓reduceIte] at v
    · cases ha : (!us.all UnitOutcome.isValidated) <;>
        simp only [ha, Bool.false_eq_true, ↓reduceIte] at v
      · cases hu : (cs.filter (fun c => !everywhere us c)).isEmpty <;>
          simp only [hu, Bool.false_eq_true, ↓reduceIte] at v
        · split at v <;> cases v
        · refine ⟨?_, ?_, ?_⟩
          · intro hn
            subst hn
            simp at he
          · intro u hu'
            have hall : us.all UnitOutcome.isValidated = true := by
              cases hx : us.all UnitOutcome.isValidated
              · rw [hx] at ha; cases ha
              · rfl
            exact List.all_eq_true.mp hall u hu'
          · intro c hc u hu'
            have hf := List.isEmpty_iff.mp hu
            have hev : everywhere us c = true := by
              cases hx : everywhere us c
              · have hin : c ∈ cs.filter (fun c => !everywhere us c) :=
                  List.mem_filter.mpr ⟨hc, by simp [hx]⟩
                rw [hf] at hin
                cases hin
              · rfl
            exact List.all_eq_true.mp hev u hu'
      · cases v
    · cases v
  · cases v

/-- A failure in any examining unit is never validated nor partly validated. -/
theorem a_failure_wins (h : Bool) (cs : List Nat) (us : List UnitOutcome)
    (f : UnitOutcome.failed ∈ us) :
    standing h cs us = .held ∨ standing h cs us = .declaredAssumption := by
  have hv : us.all UnitOutcome.isValidated = false := by
    rw [List.all_eq_false]
    exact ⟨_, f, by simp [UnitOutcome.isValidated]⟩
  unfold standing
  cases hh : (h || us.contains .held)
  · right
    cases he : (us.isEmpty || cs.isEmpty)
    · simp only [Bool.false_eq_true, hv, Bool.not_false, ↓reduceIte]
    · simp only [Bool.false_eq_true, ↓reduceIte]
  · left
    simp only [↓reduceIte]

/-- A partly-validated declaration names its uncovered clauses, each a clause
of the declaration that some unit left uncovered. -/
theorem partly_names_its_uncovered_clauses {h : Bool} {cs : List Nat} {us : List UnitOutcome}
    {un : List Nat} (v : standing h cs us = .partlyValidated un) :
    un ≠ [] ∧ ∀ c ∈ un, c ∈ cs ∧ everywhere us c = false := by
  unfold standing at v
  cases hh : (h || us.contains .held) <;> simp only [hh, Bool.false_eq_true, ↓reduceIte] at v
  · cases he : (us.isEmpty || cs.isEmpty) <;> simp only [he, Bool.false_eq_true, ↓reduceIte] at v
    · cases ha : (!us.all UnitOutcome.isValidated) <;>
        simp only [ha, Bool.false_eq_true, ↓reduceIte] at v
      · cases hu : (cs.filter (fun c => !everywhere us c)).isEmpty <;>
          simp only [hu, Bool.false_eq_true, ↓reduceIte] at v
        · split at v
          · cases v
          · cases v
            refine ⟨(fun hn => by simp [hn] at hu), ?_⟩
            intro c hc
            have := List.mem_filter.mp hc
            refine ⟨this.1, ?_⟩
            cases hx : everywhere us c
            · rfl
            · have := this.2
              simp [hx] at this
        · cases v
      · cases v
    · cases v
  · cases v

/-! ## The Safe roll-up -/

inductive Rollup where
  | noDeclaration | onlyValidated | partlyValidated | declaredAssumption
  deriving DecidableEq, Repr

/-- `bounds_body_standing::rollup_class` (a name with no standing is an
assumption). -/
def rollup (deps : List Nat) (st : Nat → Option Standing) : Rollup :=
  if deps.isEmpty then .noDeclaration
  else if deps.all (fun d => st d == some .validated) then .onlyValidated
  else if deps.all (fun d => match st d with
      | some .validated => true
      | some (.partlyValidated _) => true
      | _ => false) then .partlyValidated
  else .declaredAssumption

theorem only_validated_needs_every_dependency_validated (deps : List Nat)
    (st : Nat → Option Standing) (h : rollup deps st = .onlyValidated) :
    ∀ d ∈ deps, st d = some .validated := by
  unfold rollup at h
  cases he : deps.isEmpty <;> simp only [he, Bool.false_eq_true, ↓reduceIte] at h
  · cases ha : deps.all (fun d => st d == some .validated) <;>
      simp only [ha, Bool.false_eq_true, ↓reduceIte] at h
    · split at h <;> cases h
    · intro d hd
      have := List.all_eq_true.mp ha d hd
      simpa using this
  · cases h

theorem an_unknown_dependency_is_an_assumption (deps : List Nat) (st : Nat → Option Standing)
    (d : Nat) (hd : d ∈ deps) (hn : st d = none) : rollup deps st = .declaredAssumption := by
  unfold rollup
  have he : deps.isEmpty = false := by
    cases deps with
    | nil => simp at hd
    | cons _ _ => rfl
  have ha : deps.all (fun d => st d == some .validated) = false := by
    rw [List.all_eq_false]
    exact ⟨d, hd, by simp [hn]⟩
  have hb : deps.all (fun d => match st d with
      | some .validated => true
      | some (.partlyValidated _) => true
      | _ => false) = false := by
    rw [List.all_eq_false]
    exact ⟨d, hd, by simp [hn]⟩
  simp [he, ha, hb]

/-! ## Vectors shared with Rust (`crates/qcore-core/tests/bounds_body_standing.rs`) -/

/-- (kind, safe, refuted, frame check, covered by). -/
def coverageVectors : List (ClauseKind × Bool × Bool × Bool × Option Evidence) := [
  (.quantCheck, true, false, false, some .body),
  (.quantCheck, false, false, true, none),
  (.quantCheck, true, true, true, none),
  (.precondition, true, false, false, some .body),
  (.precondition, false, false, false, none),
  (.frame, true, false, false, none),
  (.frame, false, false, true, some .frame),
  (.frame, true, true, true, none),
  (.relation, true, false, true, none),
  (.returnExtent, true, false, true, none),
  (.preserves, true, false, true, none),
  (.globalExtent, true, false, true, none)
]

theorem coverage_vectors_hold :
    coverageVectors.all (fun (k, s, r, f, e) => covers k ⟨s, r⟩ f == e) = true := by decide

theorem coverage_vectors_count : coverageVectors.length = 12 := by decide

/-- (held, clauses, units, standing). -/
def standingVectors : List (Bool × List Nat × List UnitOutcome × Standing) := [
  (true, [1], [.validated [1]], .held),
  (false, [1], [.held], .held),
  (false, [1], [], .declaredAssumption),
  (false, [], [.unvalidated], .declaredAssumption),
  (false, [1, 2], [.validated [1, 2]], .validated),
  (false, [1, 2], [.validated [1, 2], .validated [1, 2]], .validated),
  (false, [1, 2], [.validated [1, 2], .validated [1]], .partlyValidated [2]),
  (false, [1, 2], [.validated [1], .validated [2]], .declaredAssumption),
  (false, [1, 2], [.validated [1, 2], .failed], .declaredAssumption),
  (false, [1, 2], [.validated [1, 2], .unvalidated], .declaredAssumption),
  (false, [1, 2], [.validated [2]], .partlyValidated [1])
]

theorem standing_vectors_hold :
    standingVectors.all (fun (h, cs, us, s) => standing h cs us == s) = true := by decide

theorem standing_vectors_count : standingVectors.length = 11 := by decide

end QCore.BoundsBodyStanding
