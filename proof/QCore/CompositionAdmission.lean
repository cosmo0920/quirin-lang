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

namespace QCore.CompositionAdmission

/-! Admission of a transported **composition** proposal (card `P04-COMPOSE`).

Model of the fifth transported source vocabulary and of
`qcore_verification::composition_source`'s `WitnessChecker::check` body, whose
distinguishing feature is that its reading is a function of *other
capabilities'* certificates.

Three things are modelled, and nothing else:

1. **Prerequisite revalidation is total and fail-closed.** For every claimed
   identity and every prerequisite context, revalidation is a decided function
   with no partial case: it answers `some` exactly when the claim is the
   identity the core derives *and* that prerequisite's own evaluation succeeds.
   Each of the three fields of the identity is load-bearing, an incomplete
   triple names nothing, and a repeated triple names a prerequisite it has not
   established.
2. **Admission is a conjunction the core decides for itself.** Every one of the
   seven conjuncts — the task binding, the completeness and distinctness of the
   triple, the profile being the contract's own declaration, the two bodies
   being the ones the prerequisites checked, all three prerequisites
   re-established, and the core's own composition succeeding — refuses on its
   own, before anything a producer sent is compared.
3. **No composed result travels.** The transported projection determines no
   reading at all: two cores that publish different readings can present the
   very same projection, so a producer's proposal can never be evidence for
   itself.

Everything the core cannot decide — whether a mapping was torn down, and the
`munmap`/sync/logging ABI bindings — stays a declared premise of the pinned
contracts and is outside every statement here. A recorded call is a recorded
call. -/

/-! ### The prerequisite identity triple -/

/-- A prerequisite identity: the semantic profile, the prerequisite's own task
digest and its single obligation identifier. `0` stands for an empty field. -/
structure Identity where
  profile : Nat
  taskDigest : Nat
  obligation : Nat
  deriving DecidableEq, Repr

/-- An identity names something only when every field is present. -/
def complete (i : Identity) : Bool :=
  i.profile ≠ 0 && i.taskDigest ≠ 0 && i.obligation ≠ 0

/-- The three prerequisites of the moved composition, named separately because
each is re-established independently. -/
structure Triple where
  callUse : Identity
  cleanup : Identity
  counter : Identity
  deriving DecidableEq, Repr

def tripleComplete (t : Triple) : Bool :=
  complete t.callUse && complete t.cleanup && complete t.counter

/-- One prerequisite cannot stand in for another. -/
def distinct (t : Triple) : Bool :=
  t.callUse ≠ t.cleanup && t.callUse ≠ t.counter && t.cleanup ≠ t.counter

/-! ### Prerequisite contexts and revalidation -/

/-- The core's **own** context for one prerequisite: the identity it derives,
whether that prerequisite's own contract evaluation succeeds, and the body it
checked. Nothing here comes from a request. -/
structure Prereq where
  identity : Identity
  evaluates : Bool
  body : Nat
  deriving DecidableEq, Repr

/-- Re-establish a prerequisite from a claimed identity. This is a **total**
function: every (context, claim) pair is decided, and the claim is only ever
used as a lookup key — the answer is the core's own evaluation. -/
def revalidate (p : Prereq) (claimed : Identity) : Option Unit :=
  if complete claimed && claimed == p.identity && p.evaluates then some () else none

/-- Totality: revalidation answers on every input, and its answer is exactly
the decided conjunction. There is no third outcome and no partial case. -/
theorem revalidation_is_total (p : Prereq) (claimed : Identity) :
    (revalidate p claimed).isSome =
      (complete claimed && claimed == p.identity && p.evaluates) := by
  unfold revalidate
  by_cases h : (complete claimed && claimed == p.identity && p.evaluates) = true <;>
    simp [h]

/-- A claim that is not the identity the core derives is refused. -/
theorem a_foreign_claim_is_refused (p : Prereq) (claimed : Identity)
    (h : claimed ≠ p.identity) : revalidate p claimed = none := by
  unfold revalidate
  simp [beq_iff_eq, h]

/-- A prerequisite whose own contract evaluation refuses can never be
established, however honest the claim. -/
theorem a_refusing_prerequisite_is_never_established (p : Prereq) (claimed : Identity)
    (h : p.evaluates = false) : revalidate p claimed = none := by
  unfold revalidate
  simp [h]

/-- An incomplete claim names nothing. -/
theorem an_incomplete_claim_is_refused (p : Prereq) (claimed : Identity)
    (h : complete claimed = false) : revalidate p claimed = none := by
  unfold revalidate
  simp [h]

/-- Non-vacuity: the core's own identity over an evaluating prerequisite is
established. -/
theorem the_cores_own_identity_is_established (p : Prereq)
    (hc : complete p.identity = true) (he : p.evaluates = true) :
    revalidate p p.identity = some () := by
  unfold revalidate
  simp [hc, he]

/-- Each of the three fields of the identity is load-bearing: moving any one of
them alone is already a foreign claim. -/
theorem the_profile_field_is_load_bearing (p : Prereq) (n : Nat)
    (h : n ≠ p.identity.profile) :
    revalidate p { p.identity with profile := n } = none := by
  apply a_foreign_claim_is_refused
  intro heq
  exact h (congrArg Identity.profile heq)

theorem the_task_digest_field_is_load_bearing (p : Prereq) (n : Nat)
    (h : n ≠ p.identity.taskDigest) :
    revalidate p { p.identity with taskDigest := n } = none := by
  apply a_foreign_claim_is_refused
  intro heq
  exact h (congrArg Identity.taskDigest heq)

theorem the_obligation_field_is_load_bearing (p : Prereq) (n : Nat)
    (h : n ≠ p.identity.obligation) :
    revalidate p { p.identity with obligation := n } = none := by
  apply a_foreign_claim_is_refused
  intro heq
  exact h (congrArg Identity.obligation heq)

/-! ### The transported projection and the core -/

/-- What crosses the boundary: three identity triples (lookup keys), the two
bodies' paths and the prerequisite record profile. There is no field in which a
reading, a certificate or a verdict could be carried. -/
structure Projection where
  claimed : Triple
  wrapper : Nat
  native : Nat
  profile : Nat
  deriving DecidableEq, Repr

/-- A composed reading. Its content is irrelevant here; what matters is that it
is decided by the core and compared, never transported. -/
abbrev Reading := Nat

/-- The trusted core's own context: its three prerequisite contexts, the bodies
it imported, the profile it derived from its own pinned contract, and its own
composition, which may refuse. -/
structure Core where
  callUse : Prereq
  cleanup : Prereq
  counter : Prereq
  wrapper : Nat
  native : Nat
  declared : Nat
  composes : Option Reading
  deriving DecidableEq, Repr

def identities (c : Core) : Triple :=
  { callUse := c.callUse.identity, cleanup := c.cleanup.identity, counter := c.counter.identity }

/-- Every prerequisite re-established, in the order the implementation does it.
-/
def established (c : Core) (claimed : Triple) : Bool :=
  (revalidate c.callUse claimed.callUse).isSome &&
    (revalidate c.cleanup claimed.cleanup).isSome &&
    (revalidate c.counter claimed.counter).isSome

/-- The admission rule: seven conjuncts, every one of them work the core does,
and only then equality with the proposal. `bound` is the outer task-digest
binding the host checks first. -/
def admit (c : Core) (p : Projection) (bound : Bool) (proposed : Reading) : Bool :=
  bound && tripleComplete p.claimed && distinct p.claimed &&
    p.profile == c.declared && p.wrapper == c.callUse.body && p.native == c.cleanup.body &&
    p.wrapper == c.wrapper && p.native == c.native &&
    established c p.claimed &&
    (match c.composes with
      | none => false
      | some reading => proposed == reading)

/-! ### Fail-closed directions -/

/-- A core whose own composition refuses admits nothing, whatever was proposed.
-/
theorem a_refusing_core_admits_nothing (c : Core) (p : Projection) (b : Bool) (r : Reading)
    (h : c.composes = none) : admit c p b r = false := by
  unfold admit
  simp [h]

/-- An unbound witness is refused before any of the core's own work matters. -/
theorem an_unbound_witness_is_refused (c : Core) (p : Projection) (r : Reading) :
    admit c p false r = false := by
  unfold admit
  simp

/-- Each of the three prerequisites refuses on its own. -/
theorem a_refusing_call_use_prerequisite_refuses (c : Core) (p : Projection) (b : Bool)
    (r : Reading) (h : c.callUse.evaluates = false) : admit c p b r = false := by
  unfold admit established
  simp [a_refusing_prerequisite_is_never_established _ _ h]

theorem a_refusing_cleanup_prerequisite_refuses (c : Core) (p : Projection) (b : Bool)
    (r : Reading) (h : c.cleanup.evaluates = false) : admit c p b r = false := by
  unfold admit established
  simp [a_refusing_prerequisite_is_never_established _ _ h]

theorem a_refusing_counter_prerequisite_refuses (c : Core) (p : Projection) (b : Bool)
    (r : Reading) (h : c.counter.evaluates = false) : admit c p b r = false := by
  unfold admit established
  simp [a_refusing_prerequisite_is_never_established _ _ h]

/-- A transported profile that is not the contract's own declaration is
refused, so a producer cannot choose the record shape it answers over. -/
theorem a_profile_that_is_not_the_declaration_is_refused (c : Core) (p : Projection) (b : Bool)
    (r : Reading) (h : p.profile ≠ c.declared) : admit c p b r = false := by
  unfold admit
  simp [beq_iff_eq, h]

/-- A wrapper body the def/use prerequisite never checked is refused, so a
projection cannot compose one body's paths with another body's re-established
certificate. -/
theorem a_wrapper_body_the_prerequisite_did_not_check_is_refused (c : Core) (p : Projection)
    (b : Bool) (r : Reading) (h : p.wrapper ≠ c.callUse.body) : admit c p b r = false := by
  unfold admit
  simp [beq_iff_eq, h]

/-- The same for the prerequisite body. -/
theorem a_prerequisite_body_the_prerequisite_did_not_check_is_refused (c : Core) (p : Projection)
    (b : Bool) (r : Reading) (h : p.native ≠ c.cleanup.body) : admit c p b r = false := by
  unfold admit
  simp [beq_iff_eq, h]

/-- An incomplete triple is refused. -/
theorem an_incomplete_triple_is_refused (c : Core) (p : Projection) (b : Bool) (r : Reading)
    (h : tripleComplete p.claimed = false) : admit c p b r = false := by
  unfold admit
  simp [h]

/-- A repeated triple is refused: one prerequisite cannot stand in for another.
-/
theorem a_repeated_triple_is_refused (c : Core) (p : Projection) (b : Bool) (r : Reading)
    (h : distinct p.claimed = false) : admit c p b r = false := by
  unfold admit
  simp [h]

/-- A foreign triple is refused even when everything else agrees. -/
theorem a_foreign_triple_is_refused (c : Core) (p : Projection) (b : Bool) (r : Reading)
    (h : p.claimed.counter ≠ c.counter.identity) : admit c p b r = false := by
  unfold admit established
  simp [a_foreign_claim_is_refused _ _ h]

/-- A proposal that is not the reading the core computed is refused. -/
theorem a_proposal_that_differs_is_refused (c : Core) (p : Projection) (b : Bool)
    (r reading : Reading) (hc : c.composes = some reading) (h : r ≠ reading) :
    admit c p b r = false := by
  unfold admit
  simp [hc, beq_iff_eq, h]

/-! ### What an admission means -/

/-- Admission implies every conjunct: the binding, the triple's completeness and
distinctness, the declared profile, both body ties, all three prerequisites
re-established, and a reading the core itself computed. -/
theorem admission_implies_every_conjunct (c : Core) (p : Projection) (b : Bool) (r : Reading)
    (h : admit c p b r = true) :
    b = true ∧ tripleComplete p.claimed = true ∧ distinct p.claimed = true ∧
      p.profile = c.declared ∧ p.wrapper = c.callUse.body ∧ p.native = c.cleanup.body ∧
      established c p.claimed = true ∧ c.composes = some r := by
  unfold admit at h
  cases hc : c.composes with
  | none => rw [hc] at h; simp at h
  | some reading =>
      rw [hc] at h
      simp only [Bool.and_eq_true, beq_iff_eq] at h
      obtain ⟨⟨⟨⟨⟨⟨⟨⟨hb, htc⟩, hd⟩, hp⟩, hw⟩, hn⟩, _hw2⟩, _hn2⟩, he⟩ := h.1
      exact ⟨hb, htc, hd, hp, hw, hn, he, congrArg some h.2.symm⟩

/-- An admitted proposal **is** the reading the core computed itself: agreement
is a consequence of the core's own work, never a substitute for it. -/
theorem an_admitted_proposal_is_the_cores_own_reading (c : Core) (p : Projection) (b : Bool)
    (r : Reading) (h : admit c p b r = true) : c.composes = some r :=
  (admission_implies_every_conjunct c p b r h).2.2.2.2.2.2.2

/-- Two admitted proposals over one core cannot disagree. -/
theorem two_admitted_proposals_cannot_disagree (c : Core) (p q : Projection) (b b' : Bool)
    (r r' : Reading) (h : admit c p b r = true) (h' : admit c q b' r' = true) : r = r' := by
  have hr := an_admitted_proposal_is_the_cores_own_reading c p b r h
  have hr' := an_admitted_proposal_is_the_cores_own_reading c q b' r' h'
  rw [hr] at hr'
  exact Option.some.inj hr'

/-- Changing what the core computes retires the witness. -/
theorem a_changed_core_reading_retires_the_witness (c c' : Core) (p : Projection) (b : Bool)
    (r : Reading) (h : admit c p b r = true) (hne : c'.composes ≠ c.composes) :
    admit c' p b r = false := by
  have hr := an_admitted_proposal_is_the_cores_own_reading c p b r h
  cases hb : admit c' p b r with
  | false => rfl
  | true =>
      have hr' := an_admitted_proposal_is_the_cores_own_reading c' p b r hb
      exact absurd (by rw [hr', hr]) hne

/-- Admission implies every prerequisite was established, so a composition over
an unestablished prerequisite is impossible by construction. -/
theorem an_admission_establishes_all_three (c : Core) (p : Projection) (b : Bool) (r : Reading)
    (h : admit c p b r = true) :
    (revalidate c.callUse p.claimed.callUse).isSome = true ∧
      (revalidate c.cleanup p.claimed.cleanup).isSome = true ∧
      (revalidate c.counter p.claimed.counter).isSome = true := by
  have he := (admission_implies_every_conjunct c p b r h).2.2.2.2.2.2.1
  unfold established at he
  simp only [Bool.and_eq_true] at he
  exact ⟨he.1.1, he.1.2, he.2⟩

/-! ### Non-vacuity -/

/-- The composition the core computed over its own prerequisites, its own
bodies and its own declared profile is admitted. Without this every theorem
above would hold of a rule that admits nothing. -/
theorem the_cores_own_composition_is_admitted (c : Core) (reading : Reading)
    (hc : c.composes = some reading)
    (h1 : complete c.callUse.identity = true)
    (h2 : complete c.cleanup.identity = true)
    (h3 : complete c.counter.identity = true)
    (hdistinct : distinct (identities c) = true)
    (hcu : c.callUse.evaluates = true) (hcl : c.cleanup.evaluates = true)
    (hco : c.counter.evaluates = true)
    (hw : c.wrapper = c.callUse.body) (hn : c.native = c.cleanup.body) :
    admit c { claimed := identities c, wrapper := c.wrapper, native := c.native,
              profile := c.declared } true reading = true := by
  unfold identities at hdistinct
  simp [admit, established, revalidate, tripleComplete, identities, hc, hcu, hcl, hco, hw, hn,
    hdistinct, h1, h2, h3]

/-- The distinctness rule is not vacuous: a repeated triple really is rejected
and a genuinely distinct one really is accepted. -/
theorem the_distinctness_rule_is_decidable :
    distinct { callUse := ⟨1, 1, 1⟩, cleanup := ⟨2, 2, 2⟩, counter := ⟨3, 3, 3⟩ } = true ∧
      distinct { callUse := ⟨1, 1, 1⟩, cleanup := ⟨2, 2, 2⟩, counter := ⟨1, 1, 1⟩ } = false := by
  refine ⟨by decide, by decide⟩

/-- The completeness rule is not vacuous either. -/
theorem the_completeness_rule_is_decidable :
    tripleComplete { callUse := ⟨1, 1, 1⟩, cleanup := ⟨2, 2, 2⟩, counter := ⟨3, 3, 3⟩ } = true ∧
      tripleComplete { callUse := ⟨1, 1, 1⟩, cleanup := ⟨0, 2, 2⟩, counter := ⟨3, 3, 3⟩ }
        = false := by
  refine ⟨by decide, by decide⟩

/-! ### No composed result travels -/

/-- The decisive structural statement: the transported projection determines no
reading. Two cores that publish **different** readings present the very same
projection, so nothing a producer sends can be evidence for the composition —
the reading has to come from the core in both cases. -/
theorem the_projection_determines_no_reading :
    ∃ (c c' : Core) (p : Projection) (r r' : Reading),
      r ≠ r' ∧ admit c p true r = true ∧ admit c' p true r' = true := by
  let prereq (n : Nat) : Prereq := { identity := ⟨n, n, n⟩, evaluates := true, body := n }
  let base (reading : Reading) : Core :=
    { callUse := prereq 1, cleanup := prereq 2, counter := prereq 3,
      wrapper := 1, native := 2, declared := 7, composes := some reading }
  refine ⟨base 0, base 1,
    { claimed := { callUse := ⟨1, 1, 1⟩, cleanup := ⟨2, 2, 2⟩, counter := ⟨3, 3, 3⟩ },
      wrapper := 1, native := 2, profile := 7 }, 0, 1, by decide, by decide, by decide⟩

/-- And a projection alone never admits anything: without a core that composed,
the same projection is refused. -/
theorem the_same_projection_is_refused_by_a_refusing_core :
    admit { callUse := { identity := ⟨1, 1, 1⟩, evaluates := true, body := 1 },
            cleanup := { identity := ⟨2, 2, 2⟩, evaluates := true, body := 2 },
            counter := { identity := ⟨3, 3, 3⟩, evaluates := true, body := 3 },
            wrapper := 1, native := 2, declared := 7, composes := none }
      { claimed := { callUse := ⟨1, 1, 1⟩, cleanup := ⟨2, 2, 2⟩, counter := ⟨3, 3, 3⟩ },
        wrapper := 1, native := 2, profile := 7 } true 0 = false := by decide

/-! ### The chain

A dependent's prerequisite may itself be a dependent. The cleanup prerequisite
re-establishes *its* prerequisite before it evaluates, so an unestablished link
anywhere in the chain refuses the whole chain rather than being absorbed. -/

/-- A chained prerequisite: its own evaluation succeeds only when its inner
prerequisite is established too. -/
def chained (inner : Prereq) (innerClaim : Identity) (outer : Prereq) : Prereq :=
  { outer with evaluates := outer.evaluates && (revalidate inner innerClaim).isSome }

/-- An unestablished inner prerequisite refuses the outer one, so a chain is
never admitted by its weakest link. -/
theorem an_unestablished_inner_prerequisite_refuses_the_chain (inner outer : Prereq)
    (innerClaim claimed : Identity) (h : revalidate inner innerClaim = none) :
    revalidate (chained inner innerClaim outer) claimed = none := by
  have hfalse : (chained inner innerClaim outer).evaluates = false := by
    simp [chained, h]
  exact a_refusing_prerequisite_is_never_established _ _ hfalse

/-- And a chain whose links all hold is established, so the rule is not
vacuous. -/
theorem an_established_chain_is_established (inner outer : Prereq)
    (hic : complete inner.identity = true) (hie : inner.evaluates = true)
    (hoc : complete outer.identity = true) (hoe : outer.evaluates = true) :
    revalidate (chained inner inner.identity outer) outer.identity = some () := by
  simp [chained, revalidate, hic, hie, hoc, hoe]

/-! ## The nested composition (card `P04-COMPOSE-A`)

The `cio_file_close` compositions are the first route whose own prerequisite is
**another composition**. Two things are modelled here and nothing else:

1. **The two-level re-establishment is total.** Re-establishing a nested
   composition is re-running the whole leaf admission work — its identity, its
   own three prerequisites and its own composition — and the result is a decided
   function of every input, with no partial case. A forged *leaf* prerequisite
   under an honest *middle* identity refuses, which is the decisive property:
   every level is the core's own work, and no level can be skipped by naming the
   level above it honestly.
2. **It is order-independent.** The outer route discharges three or four
   obligations; the value it computes does not depend on the order in which they
   are discharged, so the implementation's fixed order is a choice and not a
   load-bearing assumption.

The deleting arm's fourth obligation is present exactly when the question has a
deleting arm, in both directions. Whether a file was unlinked, a descriptor
closed or a mapping torn down stays outside every statement here. -/

/-- A nested composition context: the identity a dependent names it by, and the
whole leaf `Core` it is. -/
structure Nested where
  identity : Identity
  core : Core
  deriving DecidableEq, Repr

/-- A nested composition seen as a prerequisite of its dependent. Its "own
evaluation succeeds" is exactly the leaf work: all three of *its* prerequisites
re-established under the identities the core derives, **and** its own
composition succeeding. Nothing a producer sends enters this. -/
def asPrereq (n : Nested) : Prereq :=
  { identity := n.identity,
    evaluates := established n.core (identities n.core) && n.core.composes.isSome,
    body := n.core.wrapper }

/-- The two-level re-establishment is **total**: it answers on every (nested
context, claim) pair, and its answer is exactly the decided conjunction of the
outer identity check and the whole inner level. There is no partial case. -/
theorem two_level_revalidation_is_total (n : Nested) (claimed : Identity) :
    (revalidate (asPrereq n) claimed).isSome =
      (complete claimed && claimed == n.identity &&
        (established n.core (identities n.core) && n.core.composes.isSome)) := by
  simpa [asPrereq] using revalidation_is_total (asPrereq n) claimed

/-- **The decisive statement of this card.** A forged leaf prerequisite under an
honest middle certificate refuses. The claim naming the nested composition is
the identity the core derives, the nested composition's own composition
succeeds, and it still refuses — because the core re-establishes the nested
composition's *own* prerequisites itself. -/
theorem a_forged_leaf_prerequisite_refuses_under_an_honest_middle_identity
    (n : Nested) (claimed : Identity)
    (hhonest : claimed = n.identity)
    (hcomposes : n.core.composes.isSome = true)
    (hleaf : n.core.counter.evaluates = false) :
    revalidate (asPrereq n) claimed = none := by
  have _honest := hhonest
  have _composes := hcomposes
  have hfalse : (asPrereq n).evaluates = false := by
    simp [asPrereq, established,
      a_refusing_prerequisite_is_never_established n.core.counter (identities n.core).counter hleaf]
  exact a_refusing_prerequisite_is_never_established _ _ hfalse

/-- The same for each of the other two leaf prerequisites, so no single leaf can
be skipped. -/
theorem a_refusing_leaf_call_use_refuses_the_nested_composition (n : Nested) (claimed : Identity)
    (h : n.core.callUse.evaluates = false) : revalidate (asPrereq n) claimed = none := by
  have hfalse : (asPrereq n).evaluates = false := by
    simp [asPrereq, established,
      a_refusing_prerequisite_is_never_established n.core.callUse (identities n.core).callUse h]
  exact a_refusing_prerequisite_is_never_established _ _ hfalse

theorem a_refusing_leaf_cleanup_refuses_the_nested_composition (n : Nested) (claimed : Identity)
    (h : n.core.cleanup.evaluates = false) : revalidate (asPrereq n) claimed = none := by
  have hfalse : (asPrereq n).evaluates = false := by
    simp [asPrereq, established,
      a_refusing_prerequisite_is_never_established n.core.cleanup (identities n.core).cleanup h]
  exact a_refusing_prerequisite_is_never_established _ _ hfalse

/-- A nested composition whose own composition refuses is never established,
however honest every identity is. -/
theorem a_refusing_nested_composition_is_never_established (n : Nested) (claimed : Identity)
    (h : n.core.composes = none) : revalidate (asPrereq n) claimed = none := by
  have hfalse : (asPrereq n).evaluates = false := by simp [asPrereq, h]
  exact a_refusing_prerequisite_is_never_established _ _ hfalse

/-- A foreign identity for the nested composition is refused even when the whole
inner level holds: the middle level's identity is load-bearing on its own. -/
theorem a_foreign_nested_identity_is_refused (n : Nested) (claimed : Identity)
    (h : claimed ≠ n.identity) : revalidate (asPrereq n) claimed = none :=
  a_foreign_claim_is_refused (asPrereq n) claimed (by simpa [asPrereq] using h)

/-- Non-vacuity of the two-level rule: a whole honest chain is established. -/
theorem an_honest_two_level_chain_is_established (n : Nested) (reading : Reading)
    (hid : complete n.identity = true)
    (h1 : complete n.core.callUse.identity = true)
    (h2 : complete n.core.cleanup.identity = true)
    (h3 : complete n.core.counter.identity = true)
    (hcu : n.core.callUse.evaluates = true) (hcl : n.core.cleanup.evaluates = true)
    (hco : n.core.counter.evaluates = true)
    (hc : n.core.composes = some reading) :
    revalidate (asPrereq n) n.identity = some () := by
  simp [asPrereq, revalidate, established, identities, hid, h1, h2, h3, hcu, hcl, hco, hc]

/-! ### The outer `file_close` obligations -/

/-- The outer route's prerequisite set: three named prerequisites plus an
optional fourth, present exactly on the deleting arm. -/
structure CloseSet where
  callUse : Identity
  nested : Identity
  cleanup : Identity
  delete : Option Identity
  deriving DecidableEq, Repr

/-- The outer core: its own three contexts, its nested composition context, and
the optional guarded-delete context. -/
structure CloseCore where
  callUse : Prereq
  nested : Nested
  cleanup : Prereq
  delete : Option Prereq
  deriving DecidableEq, Repr

/-- The obligations the outer route discharges, as a list, so the order in which
they are discharged can be quantified over. The optional fourth contributes an
obligation exactly when the question has a deleting arm; a set and a core that
disagree about its presence contribute `false`, which is the fail-closed answer.
-/
def closeObligations (c : CloseCore) (s : CloseSet) : List Bool :=
  [ (revalidate c.callUse s.callUse).isSome,
    (revalidate (asPrereq c.nested) s.nested).isSome,
    (revalidate c.cleanup s.cleanup).isSome,
    (match c.delete, s.delete with
     | none, none => true
     | some p, some i => (revalidate p i).isSome
     | _, _ => false) ]

/-- Every obligation discharged. -/
def establishClose (c : CloseCore) (s : CloseSet) : Bool :=
  (closeObligations c s).all id

/-- A conjunction over a list is invariant under permutation. -/
theorem all_id_is_permutation_invariant {l l' : List Bool} (h : l.Perm l') :
    l.all id = l'.all id := by
  have forward : ∀ {a b : List Bool}, a.Perm b → a.all id = true → b.all id = true := by
    intro a b hp ha
    exact List.all_eq_true.mpr fun x hx => List.all_eq_true.mp ha x (hp.mem_iff.mpr hx)
  cases hl : l.all id with
  | true => exact (forward h hl).symm
  | false =>
      cases hl' : l'.all id with
      | true =>
          have hback := forward h.symm hl'
          rw [hl] at hback
          exact absurd hback (by simp)
      | false => rfl

/-- **The two-level re-establishment is order-independent.** The outer route's
answer is the same however its obligations are ordered, so the implementation's
fixed order — its own def/use, then the nested composition, then the close
replay, then the guarded delete — is a choice and not a load-bearing
assumption. -/
theorem establishing_the_obligations_is_order_independent
    (c : CloseCore) (s : CloseSet) (order : List Bool)
    (h : (closeObligations c s).Perm order) :
    establishClose c s = order.all id :=
  all_id_is_permutation_invariant h

/-- Totality of the outer rule: it is a decided function of every input, and its
answer is exactly the conjunction of its four obligations. There is no partial
case and no obligation is skipped. -/
theorem establishing_the_obligations_is_total (c : CloseCore) (s : CloseSet) :
    establishClose c s =
      ((revalidate c.callUse s.callUse).isSome &&
        (revalidate (asPrereq c.nested) s.nested).isSome &&
        (revalidate c.cleanup s.cleanup).isSome &&
        (match c.delete, s.delete with
         | none, none => true
         | some p, some i => (revalidate p i).isSome
         | _, _ => false)) := by
  cases h1 : (revalidate c.callUse s.callUse).isSome <;>
    cases h2 : (revalidate (asPrereq c.nested) s.nested).isSome <;>
      cases h3 : (revalidate c.cleanup s.cleanup).isSome <;>
        simp [establishClose, closeObligations, h1, h2, h3]

/-- The decisive row, at the outer level: a forged leaf prerequisite refuses the
whole outer route, with every outer identity honest and the outer route's own
contracts untouched. -/
theorem a_forged_leaf_refuses_the_whole_outer_route (c : CloseCore) (s : CloseSet)
    (h : c.nested.core.counter.evaluates = false) : establishClose c s = false := by
  have hnested : revalidate (asPrereq c.nested) s.nested = none := by
    have hfalse : (asPrereq c.nested).evaluates = false := by
      simp [asPrereq, established,
        a_refusing_prerequisite_is_never_established c.nested.core.counter
          (identities c.nested.core).counter h]
    exact a_refusing_prerequisite_is_never_established _ _ hfalse
  rw [establishing_the_obligations_is_total, hnested]
  simp

/-- The deleting arm's obligation is present exactly when the question has one,
in both directions: a set that names a delete prerequisite the core does not
have, and a core that has one the set does not name, both refuse. -/
theorem the_deleting_arm_must_agree_in_both_directions (c : CloseCore) (s : CloseSet)
    (h : c.delete.isSome ≠ s.delete.isSome) : establishClose c s = false := by
  rw [establishing_the_obligations_is_total]
  cases hc : c.delete <;> cases hs : s.delete <;>
    simp_all

/-- Non-vacuity of the outer rule: a whole honest two-level route, with or
without a deleting arm, is established. -/
theorem an_honest_outer_route_is_established (c : CloseCore) (reading : Reading)
    (hcu : complete c.callUse.identity = true) (hcue : c.callUse.evaluates = true)
    (hcl : complete c.cleanup.identity = true) (hcle : c.cleanup.evaluates = true)
    (hnid : complete c.nested.identity = true)
    (h1 : complete c.nested.core.callUse.identity = true)
    (h2 : complete c.nested.core.cleanup.identity = true)
    (h3 : complete c.nested.core.counter.identity = true)
    (he1 : c.nested.core.callUse.evaluates = true)
    (he2 : c.nested.core.cleanup.evaluates = true)
    (he3 : c.nested.core.counter.evaluates = true)
    (hnc : c.nested.core.composes = some reading)
    (hd : c.delete = none) :
    establishClose c
      { callUse := c.callUse.identity, nested := c.nested.identity,
        cleanup := c.cleanup.identity, delete := none } = true := by
  rw [establishing_the_obligations_is_total]
  simp [revalidate, asPrereq, established, identities, hcu, hcue, hcl, hcle, hnid,
    h1, h2, h3, he1, he2, he3, hnc, hd]

/-- The named members of the outer set are pairwise distinct, and the optional
fourth is subject to the rule rather than exempt from it. -/
def closeDistinct (s : CloseSet) : Bool :=
  s.callUse ≠ s.nested && s.callUse ≠ s.cleanup && s.nested ≠ s.cleanup &&
    (match s.delete with
     | none => true
     | some d => d ≠ s.callUse && d ≠ s.nested && d ≠ s.cleanup)

/-- The distinctness rule really discriminates, including on the optional
fourth member. -/
theorem the_outer_distinctness_rule_is_decidable :
    closeDistinct { callUse := ⟨1, 1, 1⟩, nested := ⟨2, 2, 2⟩, cleanup := ⟨3, 3, 3⟩,
                    delete := some ⟨4, 4, 4⟩ } = true ∧
      closeDistinct { callUse := ⟨1, 1, 1⟩, nested := ⟨2, 2, 2⟩, cleanup := ⟨3, 3, 3⟩,
                      delete := some ⟨3, 3, 3⟩ } = false ∧
      closeDistinct { callUse := ⟨1, 1, 1⟩, nested := ⟨1, 1, 1⟩, cleanup := ⟨3, 3, 3⟩,
                      delete := none } = false ∧
      closeDistinct { callUse := ⟨1, 1, 1⟩, nested := ⟨2, 2, 2⟩, cleanup := ⟨3, 3, 3⟩,
                      delete := none } = true := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-- The nesting depth is a measured property of the question, not an assumption:
this vocabulary nests exactly one composition inside another, and a question
that nested a `file_close` question inside a `file_close` question would be a
third level the outer rule has no prerequisite for. -/
def nestingDepth : Nat → Nat
  | 0 => 1
  | n + 1 => 1 + nestingDepth n

theorem the_file_close_question_nests_exactly_twice : nestingDepth 1 = 2 := by decide

end QCore.CompositionAdmission
