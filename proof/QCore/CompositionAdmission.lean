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

end QCore.CompositionAdmission
