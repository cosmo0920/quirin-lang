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

namespace QCore.SymbolicClassAdmission

/-! Card `P04-COMPOSE-UNSUPPORTED`: the **symbolic** other-backend
`cio_chunk_close` classification through the process boundary.

Model of `qcore_verification::unsupported_close_composition`'s symbolic replay
domain and of
`qcore_verification::composition_source::UnsupportedCloseCompositionSourceTask`'s
`derive` and `check` bodies.

Three things are modelled here, and nothing else:

1. **The symbolic class is a class, not a sampled value.** The backend
   discriminant on this arm is every `Int` other than the two the two probes
   return. A comparison the class does not decide for the *whole* class is a
   refusal on both sides — never an answer read off one representative.
2. **Neither masquerade direction is possible.** A supported discriminant
   cannot be classified onto the symbolic arm, and no member of the symbolic
   class can be classified onto a supported arm; and a proposal tagged for
   another family is refused before any figure is read.
3. **The conclusion is `Unknown`, and admission cannot move it.** An admitted
   proposal establishes the wrapper's own releases and *withholds* the backend's
   custody and the constructor's reachability. Nothing a producer sends changes
   that, because the proposal carries no conclusion at all.

Deliberately outside every statement here: whether either derived text
preprocesses as its base unit does, whether any caller can reach the symbolic
class at all, the C effective-type rules that make the discriminant an `Int`,
and anything about the OS. Those stay recorded in
`docs/reports/remaining-p04-compose-unsupported/README.md`. -/

/-! ### The symbolic class -/

/-- The two constants the two probes return. The class this arm reads is every
`Int` other than these two, and the classification is total only when they
differ. -/
structure Excluded where
  memory : Int
  filesystem : Int
  deriving DecidableEq, Repr

/-- Membership in the symbolic class: everything except the two constants. -/
def inClass (e : Excluded) (v : Int) : Bool :=
  (v ≠ e.memory) && (v ≠ e.filesystem)

/-- An equality test against one integer, decided **for the whole class**.

`some false` when the integer is one of the two excluded constants — no member
of the class equals it. `none` otherwise: that integer is inside the class, so
the class contains a member that equals it and members that do not, and no
single answer covers the class. This is the Rust `Value::OtherBackend` equality
arm, which returns `Err` in exactly the `none` case. -/
def decideEquality (e : Excluded) (v : Int) : Option Bool :=
  if v = e.memory ∨ v = e.filesystem then some false else none

/-- A truth test on the class.

`none` when both constants are nonzero — `0` is then inside the class, so both
arms of the branch are reachable. `some true` otherwise: one of the excluded
constants *is* zero, so the class is entirely nonzero. This is the Rust
`Value::OtherBackend` arm of `truth`. -/
def decideTruth (e : Excluded) : Option Bool :=
  if e.memory ≠ 0 ∧ e.filesystem ≠ 0 then none else some true

/-- **The class never decides an equality true.** A producer cannot make the
symbolic arm take a branch guarded by `type == k` for any `k`. -/
theorem the_class_never_decides_an_equality_true (e : Excluded) (v : Int) :
    decideEquality e v ≠ some true := by
  unfold decideEquality
  split <;> simp

/-- An equality against either excluded constant is decided, and decided
`false`: the two supported arms are exactly the ones the class excludes. -/
theorem an_excluded_constant_decides_false (e : Excluded) (v : Int)
    (h : v = e.memory ∨ v = e.filesystem) : decideEquality e v = some false := by
  simp [decideEquality, h]

/-- **The decisive refusal.** An equality against an integer the class contains
is *not* decided: the replay declines instead of sampling a representative. -/
theorem an_undecided_equality_refuses (e : Excluded) (v : Int)
    (h : inClass e v = true) : decideEquality e v = none := by
  simp [inClass, Bool.and_eq_true] at h
  simp [decideEquality, h.1, h.2]

/-- A truth test refuses exactly when `0` is inside the class. -/
theorem a_truth_test_refuses_when_zero_is_in_the_class (e : Excluded)
    (h : inClass e 0 = true) : decideTruth e = none := by
  simp [inClass, Bool.and_eq_true] at h
  simp [decideTruth, Ne.symm h.1, Ne.symm h.2]

/-- And is decided only when one excluded constant is zero, in which case the
whole class is nonzero and the test is `true`. -/
theorem a_truth_test_is_decided_only_when_zero_is_excluded (e : Excluded) (b : Bool)
    (h : decideTruth e = some b) : b = true ∧ inClass e 0 = false := by
  unfold decideTruth at h
  by_cases hz : e.memory ≠ 0 ∧ e.filesystem ≠ 0
  · rw [ite_eq_left hz] at h; cases h
  · rw [ite_eq_right hz] at h
    refine ⟨by cases h; rfl, ?_⟩
    by_cases hm : e.memory = 0
    · simp [inClass, hm]
    · by_cases hf : e.filesystem = 0
      · simp [inClass, hf]
      · exact (hz ⟨hm, hf⟩).elim

/-! ### Both masquerade directions -/

/-- Which arm of the wrapper a concrete discriminant belongs to. -/
inductive Arm where
  | memory
  | filesystem
  | symbolic
  deriving DecidableEq, Repr

/-- The classification the wrapper's own guards perform. It is total, and — when
the two constants differ — the three arms are pairwise disjoint. -/
def classify (e : Excluded) (v : Int) : Arm :=
  if v = e.memory then Arm.memory
  else if v = e.filesystem then Arm.filesystem
  else Arm.symbolic

/-- **Masquerade direction one.** A supported discriminant is never classified
onto the symbolic arm. -/
theorem a_supported_kind_is_never_symbolic (e : Excluded) (v : Int)
    (h : v = e.memory ∨ v = e.filesystem) : classify e v ≠ Arm.symbolic := by
  unfold classify
  by_cases hm : v = e.memory
  · rw [ite_eq_left hm]; simp
  · have hf : v = e.filesystem := h.resolve_left hm
    rw [ite_eq_right hm, ite_eq_left hf]; simp

/-- **Masquerade direction two.** No member of the symbolic class is ever
classified onto a supported arm. -/
theorem a_symbolic_member_is_never_supported (e : Excluded) (v : Int)
    (h : inClass e v = true) :
    classify e v ≠ Arm.memory ∧ classify e v ≠ Arm.filesystem := by
  simp [inClass, Bool.and_eq_true] at h
  unfold classify
  rw [ite_eq_right h.1, ite_eq_right h.2]
  exact ⟨by simp, by simp⟩

/-- The symbolic arm is exactly the class, in both directions. -/
theorem the_symbolic_arm_is_exactly_the_class (e : Excluded) (v : Int) :
    classify e v = Arm.symbolic ↔ inClass e v = true := by
  unfold classify inClass
  constructor
  · intro h
    by_cases hm : v = e.memory
    · rw [ite_eq_left hm] at h; cases h
    · by_cases hf : v = e.filesystem
      · rw [ite_eq_right hm, ite_eq_left hf] at h; cases h
      · simp [hm, hf]
  · intro h
    simp [Bool.and_eq_true] at h
    rw [ite_eq_right h.1, ite_eq_right h.2]

/-- **Collapsed constants make the classification non-total**: with one constant
the filesystem arm is unreachable, so the two probes are required to differ.
This is the `kinds[0] == kinds[1]` refusal in the Rust body. -/
theorem collapsed_constants_hide_the_filesystem_arm (e : Excluded)
    (h : e.memory = e.filesystem) (v : Int) : classify e v ≠ Arm.filesystem := by
  unfold classify
  by_cases hm : v = e.memory
  · rw [ite_eq_left hm]; simp
  · have hf : ¬ v = e.filesystem := by rw [← h]; exact hm
    rw [ite_eq_right hm, ite_eq_right hf]; simp

/-- And with distinct constants both supported arms really are reachable, so the
requirement is not a vacuous one. -/
theorem distinct_constants_reach_both_arms (e : Excluded) (h : e.memory ≠ e.filesystem) :
    classify e e.memory = Arm.memory ∧ classify e e.filesystem = Arm.filesystem := by
  unfold classify
  refine ⟨by rw [ite_eq_left rfl], ?_⟩
  rw [ite_eq_right (Ne.symm h), ite_eq_left rfl]

/-! ### The one composition prerequisite -/

/-- The one prerequisite this family names: the filesystem composition, itself a
three-level composed reading. What the dependent's core sees of it is the
identity it derives, whether its own whole re-establishment succeeds, the unit
it was checked over, and the question it imported. -/
structure Prerequisite where
  identity : String
  evaluates : Bool
  unitDigest : String
  question : String
  deriving DecidableEq, Repr

/-- The core's own pinned selection. None of it travels. -/
structure Contracts where
  composition : String
  probeFunction : String
  revision : String
  deriving DecidableEq, Repr

/-- The reading the core computes: the two excluded constants and the single
wrapper path the class selects. Two figures, because the certificate decides
two — the six release facts are constants of the admission, not decided
figures, so they are deliberately not here. -/
structure Reading where
  excluded : Excluded
  sourcePath : Nat
  deriving DecidableEq, Repr

/-- The three family tags a proposal on this wire can carry. Only one of them is
this route's. -/
inductive FamilyTag where
  | memoryClose
  | filesystemChunkClose
  | unsupportedClose
  deriving DecidableEq, Repr

/-- What travels: the family tag, the claimed prerequisite identity, the
transported nested question and the proposed reading. **No conclusion**, at any
level, is in this record — which is why no theorem below can promote one. -/
structure Proposal where
  tag : FamilyTag
  identity : String
  question : String
  reading : Reading
  deriving DecidableEq, Repr

/-- The whole context one admission decides in. -/
structure Context where
  base : String
  contracts : Contracts
  prerequisite : Prerequisite
  /-- The reading the core's own classification computes, when it computes one.
  `none` is every refusal inside the relocated decision procedure: a refusing
  prerequisite recheck, a probe identity mismatch, colliding constants, a class
  the body's guards do not decide, a release chain that is not the exact one, a
  reading that depends on the delete flag, or a falsified contract. -/
  coreReading : Option Reading
  deriving DecidableEq, Repr

/-- The single-unit tie. Unlike the filesystem family's, this route's own
prerequisite *is* over the wrapper's own text, so the tie is an equality — and
it is checked in both directions by the two theorems below. -/
def unitTie (c : Context) : Bool :=
  c.prerequisite.unitDigest = c.base

/-- The whole admission rule, as a decided conjunction. Every conjunct is work
the core does for itself; the proposal's reading is compared last. -/
def admits (c : Context) (p : Proposal) : Bool :=
  (p.tag = FamilyTag.unsupportedClose)
    && (p.identity = c.prerequisite.identity)
    && (p.question = c.prerequisite.question)
    && c.prerequisite.evaluates
    && unitTie c
    && (match c.coreReading with
        | none => false
        | some reading =>
            (reading = p.reading) && (reading.excluded.memory ≠ reading.excluded.filesystem))

/-! ### Fail-closed directions -/

theorem admission_is_decided (c : Context) (p : Proposal) :
    admits c p = true ∨ admits c p = false := by
  cases admits c p <;> simp

/-- A core that computes no reading admits nothing, whatever was sent. -/
theorem a_refusing_core_admits_nothing (c : Context) (p : Proposal)
    (h : c.coreReading = none) : admits c p = false := by
  simp [admits, h]

/-- **The family-tag masquerade, refused.** A proposal tagged for a supported
family is refused before any figure is read. -/
theorem a_foreign_family_tag_is_refused (c : Context) (p : Proposal)
    (h : p.tag ≠ FamilyTag.unsupportedClose) : admits c p = false := by
  simp [admits, h]

/-- And the reverse direction is a statement about the tag alone: the symbolic
tag is not any supported family's, so no supported family's checker can read
this proposal either. -/
theorem the_symbolic_tag_is_no_supported_familys :
    FamilyTag.unsupportedClose ≠ FamilyTag.memoryClose
      ∧ FamilyTag.unsupportedClose ≠ FamilyTag.filesystemChunkClose := by
  exact ⟨by simp, by simp⟩

/-- A claimed identity that is not the one the core's own context derives is
refused. -/
theorem a_foreign_identity_is_refused (c : Context) (p : Proposal)
    (h : p.identity ≠ c.prerequisite.identity) : admits c p = false := by
  simp [admits, h]

/-- A transported nested question that is not the one the core's own filesystem
context imported is refused. -/
theorem a_foreign_nested_question_is_refused (c : Context) (p : Proposal)
    (h : p.question ≠ c.prerequisite.question) : admits c p = false := by
  simp [admits, h]

/-- The one composition prerequisite refuses on its own, and it re-establishes
three levels when it does not. -/
theorem a_refusing_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisite.evaluates = false) : admits c p = false := by
  simp [admits, h]

/-- The unit tie is load-bearing: a prerequisite checked over another unit is
refused. -/
theorem a_prerequisite_over_another_unit_is_refused (c : Context) (p : Proposal)
    (h : c.prerequisite.unitDigest ≠ c.base) : admits c p = false := by
  simp [admits, unitTie, h]

/-- **Collapsed constants are refused at admission too**, not only inside the
replay: a reading whose two excluded constants are equal names no symbolic
class at all. -/
theorem a_collapsed_class_is_refused (c : Context) (p : Proposal) (reading : Reading)
    (hc : c.coreReading = some reading)
    (h : reading.excluded.memory = reading.excluded.filesystem) :
    admits c p = false := by
  simp [admits, hc, h]

/-- A proposal that differs from the core's own reading is refused. -/
theorem a_differing_proposal_is_refused (c : Context) (p : Proposal) (reading : Reading)
    (hc : c.coreReading = some reading) (h : reading ≠ p.reading) :
    admits c p = false := by
  simp [admits, hc, h]

/-- A proposal that moves either excluded constant is refused, which is the
mutation matrix's class-widening group in one statement. -/
theorem a_moved_excluded_constant_is_refused (c : Context) (p : Proposal) (reading : Reading)
    (hc : c.coreReading = some reading) (h : reading.excluded ≠ p.reading.excluded) :
    admits c p = false := by
  refine a_differing_proposal_is_refused c p reading hc ?_
  intro contra
  exact h (by rw [contra])

/-- A proposal that retargets the selected path is refused. -/
theorem a_retargeted_path_is_refused (c : Context) (p : Proposal) (reading : Reading)
    (hc : c.coreReading = some reading) (h : reading.sourcePath ≠ p.reading.sourcePath) :
    admits c p = false := by
  refine a_differing_proposal_is_refused c p reading hc ?_
  intro contra
  exact h (by rw [contra])

/-! ### What an admission means — and what it deliberately does not -/

/-- An admission establishes the one composition, which established three levels
of its own in turn. -/
theorem an_admission_establishes_the_composition (c : Context) (p : Proposal)
    (h : admits c p = true) : c.prerequisite.evaluates = true := by
  cases he : c.prerequisite.evaluates with
  | false => rw [a_refusing_prerequisite_refuses c p he] at h; cases h
  | true => rfl

/-- An admission names the core's own identity and its own question. -/
theorem an_admission_names_the_cores_own_question (c : Context) (p : Proposal)
    (h : admits c p = true) :
    p.identity = c.prerequisite.identity ∧ p.question = c.prerequisite.question := by
  constructor
  · by_cases hi : p.identity = c.prerequisite.identity
    · exact hi
    · rw [a_foreign_identity_is_refused c p hi] at h; cases h
  · by_cases hq : p.question = c.prerequisite.question
    · exact hq
    · rw [a_foreign_nested_question_is_refused c p hq] at h; cases h

/-- An admission establishes the unit tie and the symbolic tag. -/
theorem an_admission_establishes_the_unit_tie (c : Context) (p : Proposal)
    (h : admits c p = true) :
    c.prerequisite.unitDigest = c.base ∧ p.tag = FamilyTag.unsupportedClose := by
  constructor
  · by_cases hu : c.prerequisite.unitDigest = c.base
    · exact hu
    · rw [a_prerequisite_over_another_unit_is_refused c p hu] at h; cases h
  · by_cases ht : p.tag = FamilyTag.unsupportedClose
    · exact ht
    · rw [a_foreign_family_tag_is_refused c p ht] at h; cases h

/-- An admitted proposal **is** the core's own reading. -/
theorem an_admitted_proposal_is_the_cores_own_reading (c : Context) (p : Proposal)
    (h : admits c p = true) : c.coreReading = some p.reading := by
  cases hc : c.coreReading with
  | none => rw [a_refusing_core_admits_nothing c p hc] at h; cases h
  | some reading =>
      by_cases hr : reading = p.reading
      · exact congrArg some hr
      · rw [a_differing_proposal_is_refused c p reading hc hr] at h; cases h

/-- An admitted reading names a real symbolic class: its two excluded constants
differ, so all three arms are reachable and the class is exactly their
complement. -/
theorem an_admitted_reading_names_a_real_class (c : Context) (p : Proposal)
    (h : admits c p = true) :
    p.reading.excluded.memory ≠ p.reading.excluded.filesystem := by
  have hc := an_admitted_proposal_is_the_cores_own_reading c p h
  by_cases hd : p.reading.excluded.memory = p.reading.excluded.filesystem
  · rw [a_collapsed_class_is_refused c p p.reading hc hd] at h; cases h
  · exact hd

/-- Two admitted proposals cannot disagree. -/
theorem two_admitted_proposals_agree (c : Context) (p q : Proposal)
    (hp : admits c p = true) (hq : admits c q = true) : p.reading = q.reading := by
  have hp' := an_admitted_proposal_is_the_cores_own_reading c p hp
  have hq' := an_admitted_proposal_is_the_cores_own_reading c q hq
  rw [hp'] at hq'
  exact Option.some.inj hq'

/-! ### The withheld conclusion -/

/-- The published verdict vocabulary of this arm. It has one inhabitant, by
construction: `Unknown`. -/
inductive Conclusion where
  | unknown
  deriving DecidableEq, Repr

/-- What the certificate publishes, for any reading whatsoever. -/
def conclusionOf (_ : Reading) : Conclusion := Conclusion.unknown

/-- Whether the nested backend custody may be discharged. It may not, ever. -/
def nestedDischargePermitted (_ : Reading) : Bool := false

/-- **The arm's conclusion is `Unknown` for every admitted proposal.** -/
theorem an_admission_publishes_unknown (c : Context) (p : Proposal)
    (_ : admits c p = true) : conclusionOf p.reading = Conclusion.unknown := rfl

/-- **And no proposal can change it.** The conclusion is a function of nothing a
producer sends, so two different proposals — admitted or not — publish the same
conclusion. -/
theorem no_proposal_moves_the_conclusion (p q : Proposal) :
    conclusionOf p.reading = conclusionOf q.reading := rfl

/-- An admission never permits the nested discharge. This is the modelled half
of `UnsupportedCloseEvidence::require_nested_discharge`, which returns an
`Unknown:` refusal unconditionally. -/
theorem an_admission_never_permits_the_nested_discharge (c : Context) (p : Proposal)
    (_ : admits c p = true) : nestedDischargePermitted p.reading = false := rfl

/-- Every conjunct of the rule, gathered, so the whole meaning of an admission —
including what it withholds — is one statement. -/
theorem admission_implies_every_conjunct (c : Context) (p : Proposal)
    (h : admits c p = true) :
    c.prerequisite.evaluates = true
      ∧ (p.identity = c.prerequisite.identity ∧ p.question = c.prerequisite.question)
      ∧ (c.prerequisite.unitDigest = c.base ∧ p.tag = FamilyTag.unsupportedClose)
      ∧ c.coreReading = some p.reading
      ∧ p.reading.excluded.memory ≠ p.reading.excluded.filesystem
      ∧ conclusionOf p.reading = Conclusion.unknown
      ∧ nestedDischargePermitted p.reading = false := by
  exact ⟨an_admission_establishes_the_composition c p h,
    an_admission_names_the_cores_own_question c p h,
    an_admission_establishes_the_unit_tie c p h,
    an_admitted_proposal_is_the_cores_own_reading c p h,
    an_admitted_reading_names_a_real_class c p h,
    an_admission_publishes_unknown c p h,
    an_admission_never_permits_the_nested_discharge c p h⟩

/-! ### Non-vacuity -/

def honestContext (base identity question composition probeFunction revision : String)
    (reading : Reading) : Context :=
  { base := base,
    contracts :=
      { composition := composition, probeFunction := probeFunction, revision := revision },
    prerequisite :=
      { identity := identity, evaluates := true, unitDigest := base, question := question },
    coreReading := some reading }

def honestProposal (identity question : String) (reading : Reading) : Proposal :=
  { tag := FamilyTag.unsupportedClose,
    identity := identity,
    question := question,
    reading := reading }

/-- **The rule is not vacuous**: the honest proposal over the honest context is
admitted, whenever the reading names a real class. -/
theorem the_honest_proposal_is_admitted
    (base identity question composition probeFunction revision : String) (reading : Reading)
    (h : reading.excluded.memory ≠ reading.excluded.filesystem) :
    admits (honestContext base identity question composition probeFunction revision reading)
      (honestProposal identity question reading) = true := by
  simp [admits, honestContext, honestProposal, unitTie, h]

/-- And each refusal really refuses on that same honest context, so the
fail-closed theorems are not about unreachable states. -/
theorem the_refusals_are_not_vacuous
    (base identity question composition probeFunction revision : String) (reading : Reading) :
    admits
      { honestContext base identity question composition probeFunction revision reading with
        coreReading := none }
      (honestProposal identity question reading) = false := by
  apply a_refusing_core_admits_nothing
  rfl

/-- The family-tag refusal is not vacuous either: the same honest context refuses
a proposal that differs only in its tag, in both supported directions. -/
theorem the_family_tag_refusal_is_not_vacuous
    (base identity question composition probeFunction revision : String) (reading : Reading) :
    admits (honestContext base identity question composition probeFunction revision reading)
        { honestProposal identity question reading with tag := FamilyTag.memoryClose } = false
      ∧ admits
        (honestContext base identity question composition probeFunction revision reading)
        { honestProposal identity question reading with
          tag := FamilyTag.filesystemChunkClose } = false := by
  constructor
  · apply a_foreign_family_tag_is_refused; simp
  · apply a_foreign_family_tag_is_refused; simp

/-- The unit tie is not vacuous: a prerequisite moved onto another unit is
refused on the same honest context. -/
theorem the_unit_tie_is_not_vacuous
    (base other identity question composition probeFunction revision : String)
    (reading : Reading) (h : other ≠ base) :
    admits
      { honestContext base identity question composition probeFunction revision reading with
        prerequisite :=
          { identity := identity, evaluates := true, unitDigest := other,
            question := question } }
      (honestProposal identity question reading) = false := by
  apply a_prerequisite_over_another_unit_is_refused
  exact h

/-- And the collapsed-class refusal is not vacuous: the same honest context
refuses a reading whose two constants are equal. -/
theorem the_collapsed_class_refusal_is_not_vacuous
    (base identity question composition probeFunction revision : String) (k : Int)
    (path : Nat) :
    admits
      (honestContext base identity question composition probeFunction revision
        { excluded := { memory := k, filesystem := k }, sourcePath := path })
      (honestProposal identity question
        { excluded := { memory := k, filesystem := k }, sourcePath := path }) = false := by
  apply a_collapsed_class_is_refused (reading :=
    { excluded := { memory := k, filesystem := k }, sourcePath := path })
  · rfl
  · rfl

end QCore.SymbolicClassAdmission
