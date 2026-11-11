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

import QCore.CallbackContracts
import QCore.DerefDecomposition

/-!
# Op callbacks and served queues (card CG22-OP-CALLBACKS-AND-SERVED-QUEUES)

The model of contract spec C04.19.1–C04.19.3 and gateway rules 12m–12o, on the
user's decision of 2026-10-14 (clause 1's word `supplied_owned`, clause 2's
per-arm `when` / `otherwise` form, clause 3's dotted `TAG.FIELD` with the
run-wide closure). Abstract rule theorems, not extraction of the Rust
analyzer (`crates/qcore-c-gateway/src/callback_contracts.rs`,
`callback_member_closure.rs`, `arm_release_check.rs`,
`crates/qcore-schemas/src/arm_release.rs`,
`crates/qcore-audit/src/callback_member_fold.rs`).

* **A supplied object handed to the callback (rule 12m).** At a
  `supplied_owned` position the callee hands each invocation an object it
  owns. A candidate is admitted only when it takes the object (consumed or
  released) unconditionally; one that borrows, requires or takes it on some
  return codes only is refused. Hence an admitted call never leaves the
  callee holding the object (`an_admitted_hand_off_never_leaves_the_callee_holding`).
* **A type-switched release (rule 12o).** Each arm is a release clause guarded
  by the discriminant; an undecided discriminant reads the cautious join over
  every arm (`the_cautious_join_is_sound`), and a value no arm and no
  `otherwise` declares has no arm, never a neutral one
  (`an_undeclared_arm_is_unknown`, `a_call_without_otherwise_is_refused`).
* **A registration carried by the object (rule 12n).** The candidates are
  the closed set of the run's stores into the member; an open set never
  decides (`an_open_member_never_decides`), and the run's fold keeps a unit's
  decision only when the run's set is contained in the set the unit decided
  (`a_kept_decision_covers_every_run_candidate`).

The vector lists are `SUPPLIED_CLASSIFY_VECTORS`, `SUPPLIED_DECISION_VECTORS`
(`callback_contracts.rs`) and `ARM_JOIN_VECTORS` (`arm_release.rs`), bound by
their unit tests.
-/

namespace QCore.OpCallbacks

open QCore.CallbackContracts (Decision Mode)

/-! ## The supplied object (rule 12m) -/

/-- What a candidate receives at one position. -/
inductive SActual where
  | parameter (i : Nat)
  | supplied
  | suppliedOwned
  deriving DecidableEq, Repr

/-- A candidate's class at an invocation with a `supplied_owned` position. -/
inductive SuppliedClass where
  | missing
  | identity
  | takesSupplied
  | keepsSupplied
  | effectful
  deriving DecidableEq, Repr

/-- A position that leaves the supplied object with the callee. -/
def keepsAt : Mode × SActual → Bool
  | (.borrowed, .suppliedOwned) => true
  | (.owned, .suppliedOwned) => true
  | _ => false

/-- A position that takes the supplied object (`released`, or `other` for
`resource_consumed`). -/
def takesAt : Mode × SActual → Bool
  | (.released, .suppliedOwned) => true
  | (.other, .suppliedOwned) => true
  | _ => false

/-- A position the identity admits elsewhere. -/
def plainAt : Mode × SActual → Bool
  | (.borrowed, .parameter _) => true
  | (.borrowed, .supplied) => true
  | (.owned, .parameter _) => true
  | (.owned, .supplied) => true
  | _ => false

/-- A position some class admits. -/
def admittedS (p : Mode × SActual) : Bool := plainAt p || keepsAt p || takesAt p

/-- `classify` of `callback_contracts.rs` over positions with a supplied
object. -/
def classifySupplied (ms : List (Mode × SActual)) : SuppliedClass :=
  if ms.all admittedS then
    if ms.any keepsAt then .keepsSupplied
    else if ms.any takesAt then .takesSupplied
    else .identity
  else .effectful

/-- `decide` of `callback_contracts.rs` outside a registry invocation. -/
def decideSupplied (isOpen : Bool) (cs : List SuppliedClass) : Decision :=
  if isOpen || cs.isEmpty then .refused
  else if cs.any (· == .missing) then .refused
  else if cs.any (· == .effectful) then .refused
  else if cs.any (· == .keepsSupplied) then .refused
  else .identity

/-- A candidate that keeps the supplied object is refused. -/
theorem a_kept_supplied_object_is_refused (o : Bool) (cs : List SuppliedClass)
    (m : SuppliedClass.keepsSupplied ∈ cs) : decideSupplied o cs = .refused := by
  unfold decideSupplied
  have h : cs.any (· == .keepsSupplied) = true := List.any_eq_true.mpr ⟨_, m, by decide⟩
  split
  · rfl
  · split
    · rfl
    · split
      · rfl
      · simp_all

/-- An admitted decision has only identity and taking candidates. -/
theorem an_admitted_decision_takes_or_is_identity {o : Bool} {cs : List SuppliedClass}
    (h : decideSupplied o cs = .identity) :
    ∀ c ∈ cs, c = .identity ∨ c = .takesSupplied := by
  unfold decideSupplied at h
  split at h
  · cases h
  · split at h
    · cases h
    · rename_i hm
      split at h
      · cases h
      · rename_i he
        split at h
        · cases h
        · rename_i hk
          intro c mc
          cases c with
          | identity => exact Or.inl rfl
          | takesSupplied => exact Or.inr rfl
          | missing => exact absurd (List.any_eq_true.mpr ⟨_, mc, by decide⟩) hm
          | effectful => exact absurd (List.any_eq_true.mpr ⟨_, mc, by decide⟩) he
          | keepsSupplied => exact absurd (List.any_eq_true.mpr ⟨_, mc, by decide⟩) hk

/-- **A taking candidate takes the object at every supplied position.** -/
theorem a_taking_candidate_takes_the_object (ms : List (Mode × SActual))
    (h : classifySupplied ms = .takesSupplied) :
    ∀ p ∈ ms, p.2 = .suppliedOwned → takesAt p = true := by
  unfold classifySupplied at h
  split at h
  · rename_i hall
    split at h
    · cases h
    · rename_i hk
      intro p mp hs
      have ha : admittedS p = true := List.all_eq_true.mp hall p mp
      have hnk : keepsAt p = false := by
        cases hp : keepsAt p
        · rfl
        · exact absurd (List.any_eq_true.mpr ⟨p, mp, hp⟩) hk
      obtain ⟨mode, actual⟩ := p
      simp only at hs
      subst hs
      cases mode <;> simp_all [admittedS, plainAt, keepsAt, takesAt]
  · cases h

/-- An identity candidate receives no supplied object. -/
theorem an_identity_candidate_receives_no_supplied_object (ms : List (Mode × SActual))
    (h : classifySupplied ms = .identity) : ∀ p ∈ ms, p.2 ≠ .suppliedOwned := by
  unfold classifySupplied at h
  split at h
  · rename_i hall
    split at h
    · cases h
    · rename_i hk
      split at h
      · cases h
      · rename_i ht
        intro p mp hs
        have ha : admittedS p = true := List.all_eq_true.mp hall p mp
        obtain ⟨mode, actual⟩ := p
        simp only at hs
        subst hs
        have nk : keepsAt (mode, .suppliedOwned) = false := by
          cases hp : keepsAt (mode, .suppliedOwned)
          · rfl
          · exact absurd (List.any_eq_true.mpr ⟨_, mp, hp⟩) hk
        have nt : takesAt (mode, .suppliedOwned) = false := by
          cases hp : takesAt (mode, .suppliedOwned)
          · rfl
          · exact absurd (List.any_eq_true.mpr ⟨_, mp, hp⟩) ht
        cases mode <;> simp_all [admittedS, plainAt, keepsAt, takesAt]
  · cases h

/-- **A callback hand-off never leaves the callee holding custody.** When the
decision over the candidates' positions is admitted, every candidate takes
the supplied object at every position it receives it. -/
theorem an_admitted_hand_off_never_leaves_the_callee_holding {o : Bool}
    {candidates : List (List (Mode × SActual))}
    (h : decideSupplied o (candidates.map classifySupplied) = .identity) :
    ∀ ms ∈ candidates, ∀ p ∈ ms, p.2 = .suppliedOwned → takesAt p = true := by
  intro ms mms p mp hs
  have := an_admitted_decision_takes_or_is_identity h (classifySupplied ms)
    (List.mem_map.mpr ⟨ms, mms, rfl⟩)
  rcases this with hi | ht
  · exact absurd hs (an_identity_candidate_receives_no_supplied_object ms hi p mp)
  · exact a_taking_candidate_takes_the_object ms ht p mp hs

/-- `SUPPLIED_CLASSIFY_VECTORS`: positions and the class. -/
def suppliedClassifyVectors : List (List (Mode × SActual) × SuppliedClass) := [
  ([(.borrowed, .supplied), (.released, .suppliedOwned), (.borrowed, .parameter 5)], .takesSupplied),
  ([(.other, .suppliedOwned)], .takesSupplied),
  ([(.borrowed, .suppliedOwned)], .keepsSupplied),
  ([(.owned, .suppliedOwned)], .keepsSupplied),
  ([(.released, .suppliedOwned), (.released, .supplied)], .effectful),
  ([(.borrowed, .parameter 0), (.owned, .supplied)], .identity)
]

theorem supplied_classify_vectors_hold :
    suppliedClassifyVectors.all (fun (ms, c) => classifySupplied ms == c) = true := by
  decide

/-- `SUPPLIED_DECISION_VECTORS`: (open?, classes, decision: 0 identity, 2 refused). -/
def suppliedDecisionVectors : List (Bool × List SuppliedClass × Nat) := [
  (false, [.takesSupplied], 0),
  (false, [.takesSupplied, .takesSupplied], 0),
  (false, [.takesSupplied, .keepsSupplied], 2),
  (false, [.takesSupplied, .missing], 2),
  (false, [.effectful], 2),
  (true, [.takesSupplied], 2),
  (false, [], 2)
]

theorem supplied_decision_vectors_hold :
    suppliedDecisionVectors.all (fun (o, cs, k) => (decideSupplied o cs).kind == k) = true := by
  decide

/-! ## The type-switched release (rule 12o) -/

/-- One arm clause: its guard (`none` for `otherwise`) and the member path it
releases (`none` for `resource_arm_empty`). -/
abbrev ArmClause := Option Int × Option String

/-- The paths the arm of value `v` releases: the clauses guarded by `v`, else
the `otherwise` clauses; `none` when neither exists (an undeclared arm). -/
def armOf (arms : List ArmClause) (v : Int) : Option (List String) :=
  if arms.any (fun c => c.1 == some v) then
    some ((arms.filter (fun c => c.1 == some v)).filterMap (·.2))
  else if arms.any (fun c => c.1 == Option.none) then
    some ((arms.filter (fun c => c.1 == Option.none)).filterMap (·.2))
  else Option.none

/-- The join of the arms at one path. -/
inductive Join where
  | every
  | some
  | none
  deriving DecidableEq, Repr

/-- Whether the clauses of guard `g` release `p`. -/
def releases (arms : List ArmClause) (g : Option Int) (p : String) : Bool :=
  arms.any (fun c => c.1 == g && c.2 == some p)

/-- `join_at` of `arm_release.rs`: every guard releases `p` and `otherwise`
is declared; none does; or some do. -/
def joinAt (arms : List ArmClause) (p : String) : Join :=
  if arms.all (fun c => releases arms c.1 p) && arms.any (fun c => c.1 == Option.none) then
    .every
  else if arms.any (fun c => releases arms c.1 p) then .some
  else .none

/-- **A value no arm and no `otherwise` declares has no arm**, never a
neutral one. -/
theorem an_undeclared_arm_is_unknown (arms : List ArmClause) (v : Int)
    (hv : ∀ c ∈ arms, c.1 ≠ some v) (ho : ∀ c ∈ arms, c.1 ≠ Option.none) :
    armOf arms v = Option.none := by
  have h1 : arms.any (fun c => c.1 == some v) = false :=
    List.any_eq_false.mpr (fun c mc hc => hv c mc (beq_iff_eq.mp hc))
  have h2 : arms.any (fun c => c.1 == Option.none) = false :=
    List.any_eq_false.mpr (fun c mc hc => ho c mc (beq_iff_eq.mp hc))
  simp only [armOf, h1, h2, Bool.false_eq_true, ↓reduceIte]

/-- **The cautious join is sound.** When every arm releases `p` and
`otherwise` is declared, the arm of every value releases `p`. -/
theorem the_cautious_join_is_sound (arms : List ArmClause) (p : String)
    (h : joinAt arms p = .every) (v : Int) :
    ∃ ps, armOf arms v = some ps ∧ p ∈ ps := by
  unfold joinAt at h
  split at h
  · rename_i hj
    simp only [Bool.and_eq_true] at hj
    obtain ⟨hall, hoth⟩ := hj
    have hall' := List.all_eq_true.mp hall
    -- The arm of a guard every clause's guard releases contains `p`.
    have arm : ∀ g, arms.any (fun c => c.1 == g) = true →
        p ∈ (arms.filter (fun c => c.1 == g)).filterMap (·.2) := by
      intro g hg
      obtain ⟨c, mc, hc⟩ := List.any_eq_true.mp hg
      have hr := hall' c mc
      unfold releases at hr
      obtain ⟨d, md, hd⟩ := List.any_eq_true.mp hr
      simp only [Bool.and_eq_true, beq_iff_eq] at hd hc
      apply List.mem_filterMap.mpr
      exact ⟨d, List.mem_filter.mpr ⟨md, by simp [hd.1, hc]⟩, hd.2⟩
    unfold armOf
    by_cases hv : arms.any (fun c => c.1 == some v) = true
    · simp only [hv, ↓reduceIte]
      exact ⟨_, rfl, arm (some v) hv⟩
    · simp only [hv, hoth, ↓reduceIte, Bool.false_eq_true]
      exact ⟨_, rfl, arm Option.none hoth⟩
  · split at h <;> cases h

/-- A call's decision under an undecided discriminant: admitted only with an
`otherwise` arm and no linked path some arms release and others do not. -/
def armCall (arms : List ArmClause) (links : List String) : Bool :=
  arms.any (fun c => c.1 == Option.none) && links.all (fun l => joinAt arms l != .some)

/-- **Without `otherwise`, a call under an undecided discriminant is refused**
(some value has no declared arm). -/
theorem a_call_without_otherwise_is_refused (arms : List ArmClause) (links : List String)
    (h : ∀ c ∈ arms, c.1 ≠ Option.none) : armCall arms links = false := by
  have h0 : arms.any (fun c => c.1 == Option.none) = false :=
    List.any_eq_false.mpr (fun c mc hc => h c mc (beq_iff_eq.mp hc))
  simp only [armCall, h0, Bool.false_and]

/-- A linked path some arms release and others do not refuses the call. -/
theorem an_undecided_link_is_refused (arms : List ArmClause) (links : List String)
    (l : String) (ml : l ∈ links) (h : joinAt arms l = .some) : armCall arms links = false := by
  unfold armCall
  have : links.all (fun l => joinAt arms l != .some) = false := by
    apply List.all_eq_false.mpr
    exact ⟨l, ml, by simp [h]⟩
  simp [this]

/-- `ARM_JOIN_VECTORS`: arms, a path and the join. -/
def armJoinVectors : List (List ArmClause × String × Join) := [
  ([(some 6, some "u.c.p"), (some 6, some "u.c.r"), (Option.none, Option.none)], "u.c.p", .some),
  ([(some 1, some "u.a"), (some 2, some "u.a"), (Option.none, some "u.a")], "u.a", .every),
  ([(some 1, some "u.a"), (some 2, some "u.a")], "u.a", .some),
  ([(some 1, some "u.a"), (Option.none, Option.none)], "u.b", .none)
]

theorem arm_join_vectors_hold :
    armJoinVectors.all (fun (arms, p, j) => joinAt arms p == j) = true := by
  decide

/-! ## The member closure (rule 12n) -/

/-- A member's candidate set over some units: closed with its designators, or
open. -/
inductive Closure where
  | closed (names : List String)
  | opened
  deriving DecidableEq, Repr

/-- A member invocation's decision: an open or empty set is refused, a
closed one is decided as rule 13u decides it. -/
def memberDecision (c : Closure) (classOf : String → SuppliedClass) : Decision :=
  match c with
  | .opened => .refused
  | .closed names => decideSupplied false (names.map classOf)

/-- **An open member set never decides.** -/
theorem an_open_member_never_decides (classOf : String → SuppliedClass) :
    memberDecision .opened classOf = .refused := rfl

/-- The run's fold keeps a unit's decision only when the run's set is closed
and contained in the unit's. -/
def foldKeeps (unit run : Closure) : Bool :=
  match unit, run with
  | .closed us, .closed rs => rs.all (fun x => us.contains x)
  | _, _ => false

/-- An open run demotes every decision that rests on the member. -/
theorem an_open_run_demotes (unit : Closure) : foldKeeps unit .opened = false := by
  cases unit <;> rfl

/-- **A kept decision covers every candidate the run can store.** When the
fold keeps a unit's admitted decision, every run candidate was one the unit
decided, so it takes or is the identity. -/
theorem a_kept_decision_covers_every_run_candidate {us rs : List String}
    {classOf : String → SuppliedClass}
    (keep : foldKeeps (.closed us) (.closed rs) = true)
    (h : memberDecision (.closed us) classOf = .identity) :
    ∀ x ∈ rs, classOf x = .identity ∨ classOf x = .takesSupplied := by
  intro x mx
  have hx : us.contains x = true := List.all_eq_true.mp keep x mx
  have mu : x ∈ us := by simpa using hx
  exact an_admitted_decision_takes_or_is_identity h (classOf x) (List.mem_map.mpr ⟨x, mu, rfl⟩)

/-! ## Every unit read (card CALLBACK-MEMBER-DATA-ONLY-UNITS, rule 12n)

Each unit of the run, analysed or data-only, contributes its reading of the
member: `none` when its Clang AST was not read (its preprocessing, Clang run or
JSON failed), otherwise the closure its own facts give. A unit read with no
store into the member contributes `.closed []`, read from its AST, not
assumed. -/

/-- One unit's reading of a member. -/
abbrev UnitReading := Option Closure

/-- The run's closure: open when any unit is unread or open, otherwise every
read unit's designators. -/
def runClosure : List UnitReading → Closure
  | [] => .closed []
  | none :: _ => .opened
  | some .opened :: _ => .opened
  | some (.closed ns) :: rest =>
      match runClosure rest with
      | .opened => .opened
      | .closed rs => .closed (ns ++ rs)

/-- **An unread unit keeps the member open.** -/
theorem an_unread_unit_keeps_the_member_open {us : List UnitReading} (h : none ∈ us) :
    runClosure us = .opened := by
  induction us with
  | nil => cases h
  | cons u rest ih =>
    rcases List.mem_cons.mp h with he | hr
    · subst he; rfl
    · rcases u with _ | (ns | _)
      · rfl
      · simp [runClosure, ih hr]
      · rfl

/-- Hence the fold demotes every decision resting on a member while any unit
of the run is unread. -/
theorem an_unread_unit_demotes (unit : Closure) {us : List UnitReading} (h : none ∈ us) :
    foldKeeps unit (runClosure us) = false := by
  rw [an_unread_unit_keeps_the_member_open h]
  exact an_open_run_demotes unit

/-- A unit read with no store into the member changes nothing. -/
theorem a_read_unit_without_a_store_adds_nothing (us : List UnitReading) :
    runClosure (some (.closed []) :: us) = runClosure us := by
  cases hr : runClosure us <;> simp [runClosure, hr]

/-- **A unit's set read from its AST is exact.** When the run is closed, its
set is exactly the designators the units read, so a candidate a data-only
unit's initializer stores is in it, and nothing else is. -/
theorem the_run_set_is_exactly_the_read_sets {us : List UnitReading} {rs : List String}
    (h : runClosure us = .closed rs) (x : String) :
    x ∈ rs ↔ ∃ ns, some (Closure.closed ns) ∈ us ∧ x ∈ ns := by
  induction us generalizing rs with
  | nil =>
    simp only [runClosure, Closure.closed.injEq] at h
    subst h
    simp
  | cons u rest ih =>
    rcases u with _ | (ns | _)
    · simp [runClosure] at h
    · cases hr : runClosure rest with
      | opened => simp [runClosure, hr] at h
      | closed rs' =>
        simp only [runClosure, hr, Closure.closed.injEq] at h
        subst h
        rw [List.mem_append, ih hr]
        constructor
        · rintro (hx | ⟨ms, hm, hx⟩)
          · exact ⟨ns, by simp, hx⟩
          · exact ⟨ms, List.mem_cons_of_mem _ hm, hx⟩
        · rintro ⟨ms, hm, hx⟩
          rcases List.mem_cons.mp hm with he | hm
          · cases he
            exact Or.inl hx
          · exact Or.inr ⟨ms, hm, hx⟩
    · simp [runClosure] at h

/-- `RUN_VECTORS` (`callback_member_fold.rs`): each unit's reading of `T.f`
(`none` unread, `some none` open, `some (some names)` closed) and the run's
closure (`none` open). -/
def runVectors : List (List (Option (Option (List String))) × Option (List String)) := [
  ([], some []),
  ([some (some ["a"]), some (some [])], some ["a"]),
  ([some (some ["a"]), some (some ["b"])], some ["a", "b"]),
  ([some (some ["a"]), none], none),
  ([some (some []), none, some (some ["b"])], none),
  ([some (some ["a"]), some none], none)
]

/-- The vector spelling of a reading. -/
def readingOf : Option (Option (List String)) → UnitReading
  | none => none
  | some none => some .opened
  | some (some names) => some (.closed names)

/-- The vector spelling of a closure. -/
def closureOf : Closure → Option (List String)
  | .opened => none
  | .closed names => some names

theorem run_vectors_hold :
    runVectors.all (fun (us, c) => closureOf (runClosure (us.map readingOf)) == c) = true := by
  decide

/-! ## Constructor result states (card OP-CONSTRUCTOR-DECLARATIONS, rules 12o, 13w, 14e)

USER DECISIONS 2026-10-14: the four-operand `resource_discriminant_set`, the
member registration `resource_callback_member_registered`, and the `quant`
state clause `quant live #result: TAG.FIELD |-> #absent;` with its two levels.
A constructor states its result's discriminant and members; the calling
body keeps the state per path; a later call reads it only while every store
the run makes into the member is classified (the stability check). Values
here are the discriminant already read under the release's mask (the mask is
the schema's, bound by `crates/qcore-schemas/src/constructed_state.rs`). -/

/-- The decided join at `p` over the values `vs` a decided discriminant may
reach: `none` when some value has no arm; otherwise every, none or some of
their arms release `p` (`decided_join` of `constructed_state.rs`). -/
def decidedJoin (arms : List ArmClause) (vs : List Int) (p : String) : Option Join :=
  (vs.mapM (armOf arms)).map fun sets =>
    if !(sets.any (fun s => s.contains p)) then .none
    else if sets.all (fun s => s.contains p) then .every
    else .some

/-- **A decided discriminant selects exactly its arm.** -/
theorem a_decided_discriminant_selects_exactly_its_arm (arms : List ArmClause) (v : Int)
    (p : String) (ps : List String) (h : armOf arms v = some ps) :
    decidedJoin arms [v] p = some (if ps.contains p then .every else .none) := by
  by_cases hp : p ∈ ps <;> simp [decidedJoin, List.mapM, List.mapM.loop, h, hp]

/-- A value no arm and no `otherwise` declares leaves the decided reading
undecided (`none`), never neutral. -/
theorem a_decided_value_with_no_arm_is_unknown (arms : List ArmClause) (v : Int) (p : String)
    (hv : ∀ c ∈ arms, c.1 ≠ some v) (ho : ∀ c ∈ arms, c.1 ≠ Option.none) :
    decidedJoin arms [v] p = Option.none := by
  simp [decidedJoin, List.mapM, List.mapM.loop, an_undeclared_arm_is_unknown arms v hv ho]

/-- A rewrite inside one arm keeps it: two values with the same arm read as
one. -/
theorem a_rewrite_within_one_arm_keeps_it (arms : List ArmClause) (v w : Int) (p : String)
    (same : armOf arms w = armOf arms v) :
    decidedJoin arms [v, w] p = decidedJoin arms [v] p := by
  cases hv : armOf arms v with
  | none => simp [decidedJoin, List.mapM, List.mapM.loop, hv]
  | some ps =>
      rw [hv] at same
      by_cases hp : p ∈ ps <;> simp [decidedJoin, List.mapM, List.mapM.loop, hv, same, hp]

/-- **The decided join refines the cautious one**: where every arm releases
`p` (`otherwise` declared), every decided value's arm releases it too. -/
theorem the_decided_join_refines_the_cautious_join (arms : List ArmClause) (p : String)
    (h : joinAt arms p = .every) (v : Int) :
    decidedJoin arms [v] p = some .every := by
  obtain ⟨ps, hps, mp⟩ := the_cautious_join_is_sound arms p h v
  rw [a_decided_discriminant_selects_exactly_its_arm arms v p ps hps]
  simp [mp]

/-- What a destroy reads of the discriminant: the decided values when the
path state is decided, otherwise the cautious join. -/
def armReading (arms : List ArmClause) (state : Option (List Int)) (p : String) : Option Join :=
  match state with
  | some vs => decidedJoin arms vs p
  | Option.none => some (joinAt arms p)

/-- **An undecided discriminant gives the join.** -/
theorem an_undecided_discriminant_gives_the_join (arms : List ArmClause) (p : String) :
    armReading arms Option.none p = some (joinAt arms p) := rfl

/-! ### The stability of a stated member -/

/-- One store into a stated member, as the run's stores are classified. -/
inductive Store where
  /-- Into an object its function obtained from calls only, each handing an
  owned result (`owned`) or not. -/
  | fresh (owned : Bool)
  | automatic
  | null
  | flagOr
  | rewrite
  | other
  deriving DecidableEq, Repr

/-- Whether a store keeps a stated state (a discriminant's, or a
function-pointer member's). -/
def Store.keeps (discriminant : Bool) : Store → Bool
  | .fresh owned => owned
  | .automatic => true
  | .null => !discriminant
  | .flagOr => discriminant
  | .rewrite => discriminant
  | .other => false

/-- The run's stores keep a stated state when every one does. -/
def stable (discriminant : Bool) (stores : List Store) : Bool :=
  stores.all (Store.keeps discriminant)

/-- The path state a later call reads: the stated one only when the stores
keep it. -/
def pathState {α : Type} (keeps : Bool) (state : Option α) : Option α :=
  if keeps then state else Option.none

/-- **An unstable discriminant gives the join.** -/
theorem an_unstable_discriminant_gives_the_join (arms : List ArmClause) (p : String)
    (stores : List Store) (h : stable true stores = false) (state : Option (List Int)) :
    armReading arms (pathState (stable true stores) state) p = some (joinAt arms p) := by
  simp [pathState, h, armReading]

/-- A member invocation over a path state: the registered candidates (`[]`
is `absent`: no candidate, nothing invoked), else the run closure (rule
12n). -/
def stateDecision (candidates : List String) (classOf : String → SuppliedClass) : Decision :=
  if candidates.isEmpty then .identity else decideSupplied false (candidates.map classOf)

def invokedDecision (state : Option (List String)) (closure : Closure)
    (classOf : String → SuppliedClass) : Decision :=
  match state with
  | some candidates => stateDecision candidates classOf
  | Option.none => memberDecision closure classOf

/-- **A constructor registration followed by destroy invokes only the
registered candidate**: the decision reads that candidate's class alone,
whatever the run closure holds. -/
theorem a_registration_then_destroy_invokes_only_the_registered_candidate (c : String)
    (closure : Closure) (classOf : String → SuppliedClass) :
    invokedDecision (some [c]) closure classOf = decideSupplied false [classOf c] := rfl

/-- **An op that maps to `absent` invokes no candidate at its destroy.** -/
theorem an_absent_member_invokes_no_candidate (closure : Closure)
    (classOf : String → SuppliedClass) :
    invokedDecision (some []) closure classOf = .identity := rfl

/-- **An unclassified store voids the `absent` state**: the destroy reads the
run closure again (an open one never decides). -/
theorem an_unclassified_store_voids_the_absent_state (stores : List Store)
    (h : Store.other ∈ stores) (closure : Closure) (classOf : String → SuppliedClass) :
    invokedDecision (pathState (stable false stores) (some [])) closure classOf =
      memberDecision closure classOf := by
  have hs : stable false stores = false :=
    List.all_eq_false.mpr ⟨.other, h, by simp [Store.keeps]⟩
  simp [pathState, hs, invokedDecision]

/-! ### The two levels of `quant live #result: TAG.FIELD |-> #absent` (rule 14e)

The shorthand is the liveness part of `QCore.DerefDecomposition` (`quant live
#result`: the identity has not been released) together with the member state
`absent`. Its meaning is derived from the parts' meanings, not asserted. -/

/-- A returned object: the pointer value of `QCore.DerefDecomposition` and
what each function-pointer member holds (`none`: `absent`). -/
structure Result where
  ptr : QCore.DerefDecomposition.Ptr
  member : String → Option String

/-- A canonical part. -/
inductive Part where
  | live
  | absent (m : String)
  deriving DecidableEq, Repr

/-- What a part says of the returned object. -/
def Part.holds : Part → Result → Prop
  | .live, r => QCore.DerefDecomposition.liveness r.ptr
  | .absent m, r => r.member m = Option.none

/-- A written form: the shorthand, or one of its two parts. -/
inductive Form where
  | shorthand (m : String)
  | live
  | memberAbsent (m : String)
  deriving DecidableEq, Repr

/-- The loader's elaboration into canonical parts. -/
def elaborate : Form → List Part
  | .shorthand m => [.live, .absent m]
  | .live => [.live]
  | .memberAbsent m => [.absent m]

/-- What a list of written forms says: every part it elaborates to holds. -/
def meaning (fs : List Form) (r : Result) : Prop :=
  ∀ f ∈ fs, ∀ p ∈ elaborate f, p.holds r

/-- **The shorthand entails its decomposition, and back.** -/
theorem the_shorthand_and_its_parts_mean_the_same (m : String) (r : Result) :
    meaning [.shorthand m] r ↔ meaning [.live, .memberAbsent m] r := by
  simp [meaning, elaborate]

/-- **The shorthand means the result is live and its member maps to
`absent`.** -/
theorem the_shorthand_is_live_and_absent (m : String) (r : Result) :
    meaning [.shorthand m] r ↔
      QCore.DerefDecomposition.liveness r.ptr ∧ r.member m = Option.none := by
  simp [meaning, elaborate, Part.holds]

/-- The shorthand entails `quant live #result` alone. -/
theorem the_shorthand_entails_quant_live (m : String) (r : Result)
    (h : meaning [.shorthand m] r) : meaning [.live] r := by
  intro f hf q hq
  simp only [List.mem_singleton] at hf
  subst hf
  simp only [elaborate, List.mem_singleton] at hq
  subst hq
  exact h (.shorthand m) (by simp) .live (by simp [elaborate])

/-- The canonical set of a list of forms: whether liveness is stated and the
members stated `absent`. -/
def canonical (fs : List Form) : Bool × (String → Bool) :=
  (fs.any (fun f => (elaborate f).contains .live),
   fun x => fs.any (fun f => (elaborate f).contains (.absent x)))

/-- **Both levels, and both together, are one canonical set**: a shorthand and
its decomposition read identically, and an agreeing overlap merges. -/
theorem both_levels_are_one_canonical_set (m : String) :
    canonical [.shorthand m] = canonical [.live, .memberAbsent m] ∧
      canonical [.shorthand m, .live, .memberAbsent m] = canonical [.shorthand m] := by
  constructor <;>
    · refine Prod.ext ?_ (funext fun x => ?_)
      · simp [canonical, elaborate]
      · by_cases hx : x = m
        · subst hx
          simp [canonical, elaborate]
        · simp [canonical, elaborate, hx]

/-- `DECIDED_JOIN_VECTORS`: arms, the reachable values, a path and the
decided join. -/
def decidedJoinVectors : List (List ArmClause × List Int × String × Option Join) := [
  ([(some 6, some "u.c.p"), (some 9, some "u.x"), (Option.none, Option.none)], [6], "u.c.p", some .every),
  ([(some 6, some "u.c.p"), (some 9, some "u.x"), (Option.none, Option.none)], [9], "u.c.p", some .none),
  ([(some 6, some "u.c.p"), (Option.none, Option.none)], [17], "u.c.p", some .none),
  ([(some 6, some "u.c.p"), (Option.none, Option.none)], [16, 17], "u.c.p", some .none),
  ([(some 6, some "u.c.p"), (Option.none, Option.none)], [6, 17], "u.c.p", some .some),
  ([(some 6, some "u.c.p")], [7], "u.c.p", Option.none)
]

theorem decided_join_vectors_hold :
    decidedJoinVectors.all (fun (arms, vs, p, j) => decidedJoin arms vs p == j) = true := by
  decide

end QCore.OpCallbacks
