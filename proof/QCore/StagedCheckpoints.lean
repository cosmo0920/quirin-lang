import QCore.Checkpoints

/-! Card **CORPUS-GROUP-CHECKPOINTS** — per-group checkpoints of a staged run.

This is a model of `crates/qcore-audit/src/pinned_route/checkpoints.rs`, the
checkpointed pinned corpus route. Unlike the configured route modelled in
`QCore.Checkpoints`, the pinned route runs in **stages**: a first stage under
the loaded contracts, then, when the first stage's rows refute a declaration,
a stage under the refutation table, then, when a link map selects cross-unit
homes, a stage under the imported environment, accepted only when its own
rows leave the refutation table unchanged. Each later stage's whole-run facts
(the table, the homes) are computed from the previous stage's rows of every
group, and every unit of the stage reads them.

A group's checkpoint therefore records, besides the run identity and its
units' fingerprints, the **facts** of the stage it was sealed in, and is
reused only when all three equal the ones recomputed now. Three facts are
proved.

* **A stage's reuse is recomputation.** Under the premise that a unit's row
  is determined by the run identity, the stage's facts and its own
  fingerprint (`Determined`), a reused group's rows are exactly the
  recomputed ones, for any checkpoint the route sealed in any stage
  (`reused_eq_recomputed`); a checkpoint sealed under other facts is never
  reused (`other_facts_recompute`).
* **The staged run is the whole run.** Whatever the stored files, every
  stage's whole-run facts, every acceptance decision and the final rows of
  the checkpointed run equal the whole run's (`resumed_eq_whole`), however
  many stages it has.
* **Its publication is the whole run's.** Composed group by group, the final
  rows publish exactly the whole run's publication (`resumed_publication_eq_whole`,
  through `QCore.Checkpoints.compose_eq_whole`).

Deliberately **not** claimed: that the Rust identity captures every input of
a unit's analysis (the `Determined` premise; the report's identity table and
the fixture `a_changed_input_of_each_kind_invalidates_exactly_the_groups_that_read_it`
argue it input by input, and the corpus runs test it), that SHA-256 detects
tampering (a refused file is modelled as no file), and that the Rust fold
prints the rows byte for byte (the corpus comparison does). -/

namespace QCore.StagedCheckpoints

/-- A run's identity and a unit's fingerprint (functions of the
environment), and a unit's analysis, which also reads its stage's whole-run
facts. -/
structure Analysis (Env Input Fp Row RunId Facts : Type) where
  runId : Env → RunId
  fingerprint : Env → Input → Fp
  analyze : Env → Facts → Input → Row

variable {Env Input Fp Row RunId Facts : Type}

/-- The premise every reuse rests on: a unit's row is determined by the run
identity, the stage's facts and the unit's own fingerprint. -/
def Determined (a : Analysis Env Input Fp Row RunId Facts) : Prop :=
  ∀ e e' f i i', a.runId e = a.runId e' → a.fingerprint e i = a.fingerprint e' i' →
    a.analyze e f i = a.analyze e' f i'

/-- A sealed group: the run, the stage's facts, its units' fingerprints and
its rows. -/
structure Checkpoint (Fp Row RunId Facts : Type) where
  run : RunId
  facts : Facts
  units : List Fp
  rows : List Row

/-- The checkpoint a group's computation in a stage seals. -/
def sealGroup (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (f : Facts)
    (g : List Input) : Checkpoint Fp Row RunId Facts :=
  { run := a.runId e, facts := f, units := g.map (a.fingerprint e),
    rows := g.map (a.analyze e f) }

/-- A group's rows in a stage, computed now. -/
def recompute (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (f : Facts)
    (g : List Input) : List Row :=
  g.map (a.analyze e f)

/-- A checkpoint the route sealed, in some environment, stage and group. -/
def Sealed (a : Analysis Env Input Fp Row RunId Facts) (c : Checkpoint Fp Row RunId Facts) :
    Prop :=
  ∃ e f g, c = sealGroup a e f g

/-- Resume a group in a stage: reuse a stored checkpoint whose run, facts and
fingerprints all equal the ones recomputed now; otherwise recompute. -/
def resume [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (f : Facts) (g : List Input) :
    Option (Checkpoint Fp Row RunId Facts) → List Row
  | none => recompute a e f g
  | some c =>
    if c.run = a.runId e ∧ c.facts = f ∧ c.units = g.map (a.fingerprint e) then c.rows
    else recompute a e f g

/-- Equal fingerprints unit by unit, under one run identity and one stage's
facts, give equal rows unit by unit. -/
theorem rows_eq_of_fingerprints_eq {a : Analysis Env Input Fp Row RunId Facts}
    (determined : Determined a) {e e' : Env} (f : Facts) (same : a.runId e = a.runId e') :
    ∀ (g g' : List Input), g.map (a.fingerprint e) = g'.map (a.fingerprint e') →
      g.map (a.analyze e f) = g'.map (a.analyze e' f)
  | [], [], _ => rfl
  | [], _ :: _, h => by simp at h
  | _ :: _, [], h => by simp at h
  | i :: g, i' :: g', h => by
    simp only [List.map_cons, List.cons.injEq] at h ⊢
    exact ⟨determined e e' f i i' same h.1, rows_eq_of_fingerprints_eq determined f same g g' h.2⟩

/-- **A stage's reused group equals the recomputed one**, for any checkpoint
the route sealed, in whatever environment, stage and group. -/
theorem reused_eq_recomputed [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    {a : Analysis Env Input Fp Row RunId Facts} (determined : Determined a)
    {c : Checkpoint Fp Row RunId Facts} (sealed : Sealed a c) (e : Env) (f : Facts)
    (g : List Input) :
    resume a e f g (some c) = recompute a e f g := by
  obtain ⟨e₀, f₀, g₀, rfl⟩ := sealed
  simp only [resume]
  by_cases h : (sealGroup a e₀ f₀ g₀).run = a.runId e ∧ (sealGroup a e₀ f₀ g₀).facts = f ∧
      (sealGroup a e₀ f₀ g₀).units = g.map (a.fingerprint e)
  · rw [ite_eq_left h]
    obtain ⟨run, facts, units⟩ := h
    simp only [sealGroup] at run facts units ⊢
    subst facts
    exact rows_eq_of_fingerprints_eq determined f₀ run g₀ g units
  · rw [ite_eq_right h]

/-- A checkpoint sealed under other whole-run facts is never reused: the
group is recomputed under the facts of now. -/
theorem other_facts_recompute [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (f : Facts) (g : List Input)
    (c : Checkpoint Fp Row RunId Facts) (other : c.facts ≠ f) :
    resume a e f g (some c) = recompute a e f g := by
  simp only [resume]
  rw [ite_eq_right (fun h => other h.2.1)]

/-- Whatever files the checkpoint directory holds: per stage facts and group,
a stored checkpoint or none. -/
def Store (Input Fp Row RunId Facts : Type) :=
  Facts → List Input → Option (Checkpoint Fp Row RunId Facts)

/-- Every stored checkpoint was sealed by the route. -/
def AllSealed (a : Analysis Env Input Fp Row RunId Facts) (s : Store Input Fp Row RunId Facts) :
    Prop :=
  ∀ f g c, s f g = some c → Sealed a c

/-- One stage of the whole run: every group, analysed under the stage's facts. -/
def stage (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (gs : List (List Input))
    (f : Facts) : List (List Row) :=
  gs.map (recompute a e f)

/-- One stage of the checkpointed run: every group resumed. -/
def stageResumed [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (s : Store Input Fp Row RunId Facts)
    (gs : List (List Input)) (f : Facts) : List (List Row) :=
  gs.map fun g => resume a e f g (s f g)

/-- **A checkpointed stage is the whole run's stage.** -/
theorem stageResumed_eq_stage [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    {a : Analysis Env Input Fp Row RunId Facts} (determined : Determined a)
    {s : Store Input Fp Row RunId Facts} (sealed : AllSealed a s) (e : Env)
    (gs : List (List Input)) (f : Facts) :
    stageResumed a e s gs f = stage a e gs f := by
  unfold stageResumed stage
  apply List.map_congr_left
  intro g _
  cases h : s f g with
  | none => rfl
  | some c => exact reused_eq_recomputed determined (sealed f g c h) e f g

/-- A later stage: from the previous stage's rows of every group, the facts
it runs under (or none: the stage does not run), and whether its own rows are
accepted over the previous ones (the import stage is refused when its rows
change the refutation table). -/
structure Later (Row Facts : Type) where
  facts : List Row → Option Facts
  accept : List Row → List Row → Bool

/-- The whole run's later stages, from the current rows. -/
def whole (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (gs : List (List Input)) :
    List (List Row) → List (Later Row Facts) → List (List Row)
  | rows, [] => rows
  | rows, l :: ls =>
    match l.facts rows.flatten with
    | none => whole a e gs rows ls
    | some f =>
      let next := stage a e gs f
      if l.accept rows.flatten next.flatten then whole a e gs next ls else whole a e gs rows ls

/-- The checkpointed run's later stages, from the current rows. -/
def resumed [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    (a : Analysis Env Input Fp Row RunId Facts) (e : Env) (s : Store Input Fp Row RunId Facts)
    (gs : List (List Input)) : List (List Row) → List (Later Row Facts) → List (List Row)
  | rows, [] => rows
  | rows, l :: ls =>
    match l.facts rows.flatten with
    | none => resumed a e s gs rows ls
    | some f =>
      let next := stageResumed a e s gs f
      if l.accept rows.flatten next.flatten then resumed a e s gs next ls
      else resumed a e s gs rows ls

/-- **The staged checkpointed run is the whole run**: from the first stage's
facts, through any later stages, the final rows of every group are the whole
run's, whatever the stored checkpoints, as long as the route sealed them. -/
theorem resumed_eq_whole [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    {a : Analysis Env Input Fp Row RunId Facts} (determined : Determined a)
    {s : Store Input Fp Row RunId Facts} (sealed : AllSealed a s) (e : Env)
    (gs : List (List Input)) (first : Facts) (later : List (Later Row Facts)) :
    resumed a e s gs (stageResumed a e s gs first) later =
      whole a e gs (stage a e gs first) later := by
  rw [stageResumed_eq_stage determined sealed]
  generalize stage a e gs first = rows
  induction later generalizing rows with
  | nil => rfl
  | cons l ls ih =>
    simp only [resumed, whole]
    cases l.facts rows.flatten with
    | none => exact ih rows
    | some f =>
      simp only [stageResumed_eq_stage determined sealed]
      split
      · exact ih _
      · exact ih rows

/-- **Its publication is the whole run's.** The final groups, published group
by group and folded in plan order (`QCore.Checkpoints.composeGroups`), give
the whole run's publication over its final rows. -/
theorem resumed_publication_eq_whole [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Facts]
    {Src : Type} [DecidableEq Src] {a : Analysis Env Input Fp Row RunId Facts}
    (determined : Determined a) {s : Store Input Fp Row RunId Facts} (sealed : AllSealed a s)
    (tally : Row → Nat) (source : Row → Src) (e : Env) (gs : List (List Input))
    (first : Facts) (later : List (Later Row Facts)) :
    QCore.Checkpoints.composeGroups tally source
        (resumed a e s gs (stageResumed a e s gs first) later) =
      QCore.Checkpoints.publish tally source
        (whole a e gs (stage a e gs first) later).flatten := by
  rw [resumed_eq_whole determined sealed, QCore.Checkpoints.compose_eq_whole]

end QCore.StagedCheckpoints
