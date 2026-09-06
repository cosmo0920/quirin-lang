/-!
Summary validation for the unrestricted-C gateway's interprocedural custody
inference (W4 of `docs/GENERALIZATION_PLAN.md`).

`compiler/src/legacy_summary.rs` derives a candidate parameter/return custody
summary for an internal C function and then *validates* it by re-running the
intraprocedural analyzer under the entry state the candidate prescribes. Only a
validated summary is trusted at call sites; anything else leaves the call
uncontracted and the unit `Unknown`.

This module models that validation relation and proves the properties the Rust
implementation relies on. The claims are all about the *checked* relation, not
about a flag: each theorem states what a validated summary necessarily implies
about every reachable exit of the body it summarizes.
-/

namespace QCore.CustodySummary

/-- What became of one caller-owned resource at a particular exit. -/
inductive Disposition where
  /-- Still live: the caller keeps custody. -/
  | live
  /-- Destroyed by the callee. -/
  | released
  /-- Custody moved on through a trusted call. -/
  | escaped
  /-- Attached to an aggregate. -/
  | contained
  /-- Absent from the exit state: the argument was null on this path. -/
  | absent
  /-- Several dispositions joined; no single custody claim describes it. -/
  | mixed
  deriving DecidableEq, Repr

inductive ParameterMode where
  /-- No custody claim at all; nothing is bound at entry. -/
  | borrowed
  /-- The caller must hold a live resource across the call and still holds it. -/
  | owned
  /-- Custody moves to the callee. -/
  | consumed
  /-- The callee destroys the resource. -/
  | released
  deriving DecidableEq, Repr

inductive ResultMode where
  /-- The result carries no custody. -/
  | neutral
  /-- The result is a fresh resource the caller must discharge. -/
  | owned (nullable : Bool)
  deriving DecidableEq, Repr

/-- The value one exit returns, as the exit state describes it. -/
inductive Returned where
  /-- No modeled resource place, including a null constant. -/
  | nothing
  /-- One modeled site. `fromParameter` marks a formal handed straight back. -/
  | site (fromParameter : Bool) (disposition : Disposition) (nullable : Bool)
  /-- More than one site, or an unknown one. -/
  | ambiguous
  deriving DecidableEq, Repr

structure ExitObservation where
  /-- Disposition of each formal; `none` where the contract bound nothing. -/
  parameters : List (Option Disposition)
  returned : Returned
  /-- A fresh resource ended up owned by a caller-supplied aggregate. -/
  attachedToParameter : Bool
  deriving DecidableEq, Repr

structure Summary where
  parameters : List ParameterMode
  result : ResultMode
  deriving DecidableEq, Repr

/-- One formal's declared mode against its observed exit disposition.

    `borrowed` binds nothing, so the observation carries no disposition for it;
    the accompanying requirement that the whole analysis be defect-free is what
    rules out a body that used an unbound formal as a resource. A nullable mode
    is not modeled here: the Rust check additionally accepts `absent` for it,
    which only ever *rejects* fewer bodies than the relation below. -/
def parameterMatches : ParameterMode → Option Disposition → Bool
  | .borrowed, Option.none => true
  | .owned, Option.some .live => true
  | .consumed, Option.some .escaped => true
  | .released, Option.some .released => true
  | _, _ => false

/-- The declared result mode against what one exit returns. -/
def resultMatches : ResultMode → Returned → Bool
  | .neutral, .nothing => true
  | .neutral, _ => false
  | .owned _, .ambiguous => false
  -- Returning nothing is consistent only with a result the caller must
  -- null-check before treating as a resource.
  | .owned nullable, .nothing => nullable
  | .owned nullable, .site fromParameter disposition siteNullable =>
      !fromParameter && disposition == Disposition.escaped && (nullable || !siteNullable)

/-- Every condition one reachable exit has to satisfy. -/
def exitMatches (summary : Summary) (observation : ExitObservation) : Bool :=
  !observation.attachedToParameter
    && observation.parameters.length == summary.parameters.length
    && (summary.parameters.zip observation.parameters).all
        (fun pair => parameterMatches pair.1 pair.2)
    && resultMatches summary.result observation.returned

/-- The validation relation itself: a defect-free analysis, at least one
    reachable exit, and every exit agreeing with the summary. -/
def validate (summary : Summary) (analysisSafe : Bool) (exits : List ExitObservation) : Bool :=
  analysisSafe && !exits.isEmpty && exits.all (exitMatches summary)

/-- A body the analyzer could not clear never yields a summary. This is the
    "validated, not just inferred" discipline: an unsupported construct, an
    unresolved alias, or a leak all reach here as `analysisSafe = false`. -/
theorem unvalidated_analysis_never_validates (summary : Summary)
    (exits : List ExitObservation) : validate summary false exits = false := by
  simp [validate]

/-- A function with no reachable exit describes no call behavior. -/
theorem no_reachable_exit_never_validates (summary : Summary) :
    validate summary true [] = false := by
  simp [validate]


/-- Extraction lemma: a validated summary agrees with *every* reachable exit,
    not with some representative one. -/
theorem validated_summary_matches_every_exit (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true) :
    ∀ observation ∈ exits, exitMatches summary observation = true := by
  intro observation member
  unfold validate at valid
  simp only [Bool.and_eq_true] at valid
  exact List.all_eq_true.mp valid.2 observation member

theorem validated_summary_is_analyzed_safely (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true) :
    safe = true := by
  unfold validate at valid
  simp only [Bool.and_eq_true] at valid
  exact valid.1.1

theorem validated_summary_has_a_reachable_exit (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true) :
    exits.isEmpty = false := by
  unfold validate at valid
  simp only [Bool.and_eq_true] at valid
  have := valid.1.2
  cases isEmpty : exits.isEmpty <;> simp_all

/-- No validated summary hides a resource handed to a caller-owned aggregate.
    Such a transfer is invisible at the call site, so it may never be summarized
    away. -/
theorem validated_summary_attaches_nothing_to_a_formal (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    observation.attachedToParameter = false := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  have attached := matched.1.1.1
  cases flag : observation.attachedToParameter <;> simp_all

/-- Every reachable exit of a validated summary returns what the result mode
    claims. -/
theorem validated_summary_result_matches (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    resultMatches summary.result observation.returned = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  exact matched.2

/-- Every formal's declared mode holds at every reachable exit. -/
theorem validated_summary_parameters_match (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (mode : ParameterMode) (disposition : Option Disposition)
    (paired : (mode, disposition) ∈ summary.parameters.zip observation.parameters) :
    parameterMatches mode disposition = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  exact List.all_eq_true.mp matched.1.2 (mode, disposition) paired

/-- A neutral result claim forces every exit to return no resource. This is the
    exact hole LC01 reproduced: a wrapper that returns a fresh owned resource
    cannot be summarized as carrying no custody. -/
theorem neutral_result_returns_nothing (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (neutral : summary.result = ResultMode.neutral)
    (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    observation.returned = Returned.nothing := by
  have result :=
    validated_summary_result_matches summary safe exits valid observation member
  rw [neutral] at result
  cases returned : observation.returned with
  | nothing => rfl
  | site fromParameter disposition nullable =>
      rw [returned] at result; simp [resultMatches] at result
  | ambiguous => rw [returned] at result; simp [resultMatches] at result

/-- An owned result is never a formal handed straight back: that would mint a
    second custody for one resource. -/
theorem owned_result_never_returns_a_formal (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (nullable : Bool)
    (owned : summary.result = ResultMode.owned nullable)
    (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (disposition : Disposition) (siteNullable : Bool)
    (returned : observation.returned = Returned.site true disposition siteNullable) :
    False := by
  have result :=
    validated_summary_result_matches summary safe exits valid observation member
  rw [owned, returned] at result
  simp [resultMatches] at result

/-- A non-null owned result forbids an exit that returns nothing, so a caller is
    never handed a resource that may not exist without being told. -/
theorem nonnull_owned_result_returns_a_resource (summary : Summary) (safe : Bool)
    (exits : List ExitObservation)
    (owned : summary.result = ResultMode.owned false)
    (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (returned : observation.returned = Returned.nothing) : False := by
  have result :=
    validated_summary_result_matches summary safe exits valid observation member
  rw [owned, returned] at result
  simp [resultMatches] at result

/-- Declaring a formal released commits the callee to releasing it on *every*
    reachable exit; a path that leaves it live refutes the summary. -/
theorem released_formal_is_released_at_every_exit (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (disposition : Option Disposition)
    (paired :
      (ParameterMode.released, disposition) ∈ summary.parameters.zip observation.parameters) :
    disposition = Option.some Disposition.released := by
  have matched :=
    validated_summary_parameters_match summary safe exits valid observation member
      ParameterMode.released disposition paired
  cases disposition with
  | none => simp [parameterMatches] at matched
  | some observed => cases observed <;> simp [parameterMatches] at matched ⊢

/-- A borrowed formal binds nothing: the summary makes no custody claim about
    it, and the exit state carries no disposition for it. -/
theorem borrowed_formal_binds_nothing (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (disposition : Option Disposition)
    (paired :
      (ParameterMode.borrowed, disposition) ∈ summary.parameters.zip observation.parameters) :
    disposition = Option.none := by
  have matched :=
    validated_summary_parameters_match summary safe exits valid observation member
      ParameterMode.borrowed disposition paired
  cases disposition with
  | none => rfl
  | some observed => cases observed <;> simp [parameterMatches] at matched

/-- LC01's counterexample, as observations: a body that returns a fresh owned
    resource cannot validate as a resource-neutral call. -/
theorem lc01_allocation_wrapper_is_not_neutral :
    validate { parameters := [], result := ResultMode.neutral }
      true
      [{ parameters := [],
         returned := Returned.site false Disposition.escaped false,
         attachedToParameter := false }] = false := by
  decide

/-- The same body does validate as an owned-return summary, which is what makes
    the caller's discarded result a reported leak. -/
theorem lc01_allocation_wrapper_is_an_owned_return :
    validate { parameters := [], result := ResultMode.owned false }
      true
      [{ parameters := [],
         returned := Returned.site false Disposition.escaped false,
         attachedToParameter := false }] = true := by
  decide

/-- A formal released on one path and left live on another matches no single
    parameter mode, so the wrapper yields no summary. -/
theorem conditionally_released_formal_validates_under_no_mode (mode : ParameterMode) :
    validate { parameters := [mode], result := ResultMode.neutral }
      true
      [{ parameters := [Option.some Disposition.released],
         returned := Returned.nothing,
         attachedToParameter := false },
       { parameters := [Option.some Disposition.live],
         returned := Returned.nothing,
         attachedToParameter := false }] = false := by
  cases mode <;> decide

/-- A resource left attached to a caller-supplied aggregate refutes every
    summary, whatever the summary claims about the formals. -/
theorem attachment_to_a_formal_validates_under_no_summary (summary : Summary) :
    validate summary true
      [{ parameters := [], returned := Returned.nothing, attachedToParameter := true }]
      = false := by
  simp [validate, exitMatches]

end QCore.CustodySummary
