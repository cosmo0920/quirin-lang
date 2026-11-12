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

import QCore.StagedCheckpoints

/-! Card **MEASURE-LOOKUP-RECORD-REUSE** — reuse keyed on what a unit read.

A model of the schema-2 pinned group checkpoint
(`crates/qcore-audit/src/pinned_route/checkpoints.rs`) and of the gateway's
lookup record (`crates/qcore-c-gateway/src/lookup_record.rs`). A unit's
analysis is a **consumer** of the stage environment `K → V` (a name to its
entries in every name-keyed table), given as the reads it makes (`Prog`): a
read of one name (`ask`), or a read of the whole environment (`whole`: any
accessor that hands out or iterates a table, serialization, equality).
Running it records the names it asked for (`keys`) and whether it read the
whole environment (`readsWhole`): the lookup record. Relied callback
candidates are names the Rust check requires to have been read by name, so
they add no key.

A group's checkpoint keys its stage environment on the environment
**restricted** to the names its units read (`restrict`, the Rust
`ResourceContracts::restricted_sha256` as the pairs it digests) when no unit
read the whole environment, and on the whole environment's digest otherwise.
The identity recomputed now is computed over the **stored** scope (`current`).

* **One unit.** A consumer that read no whole table, run under another
  environment that agrees on every name it read, makes the same reads and
  returns the same result (`run_eq_of_agree`).
* **A group.** A checkpoint the route sealed, reused only when its run
  identity, its fingerprints and its scope recomputed now are equal, gives
  exactly the rows recomputed under the environment of now
  (`reused_eq_recomputed`), whether its scope is restricted or whole (under
  the premise that equal whole digests are equal environments). A change to
  a name a unit read, or any change for a whole reader, recomputes
  (`changed_key_recomputes`, `whole_reader_recomputes`).
* **The staged run.** With every group of every stage reused or
  recomputed this way, the final rows are the whole run's
  (`resumed_eq_whole`) and so is the publication
  (`resumed_publication_eq_whole`), through `QCore.StagedCheckpoints.whole`
  and `QCore.Checkpoints.compose_eq_whole`.

Deliberately **not** claimed: that the Rust recorder sees every read (that
every accessor of `ResourceContracts` records, that clones share the
recorder, that a read on another thread is recorded: the schema tests and the
real-unit fixture), that SHA-256 is injective (the restricted pairs stand for
their digest, and `wholeSound` is a premise), and that every other input of a
unit is in the run identity or its fingerprint (the schema-1 boundary of
`QCore.StagedCheckpoints`). -/

namespace QCore.LookupRestrictedReuse

/-- A consumer of an environment `K → V` returning `R`, as the reads it
makes. -/
inductive Prog (K V R : Type) where
  | done : R → Prog K V R
  | ask : K → (V → Prog K V R) → Prog K V R
  | whole : ((K → V) → Prog K V R) → Prog K V R

variable {K V R : Type}

/-- Run a consumer under an environment. -/
def run (env : K → V) : Prog K V R → R
  | .done r => r
  | .ask k next => run env (next (env k))
  | .whole next => run env (next env)

/-- The names a run asks for, in order: the lookup record's names. -/
def keys (env : K → V) : Prog K V R → List K
  | .done _ => []
  | .ask k next => k :: keys env (next (env k))
  | .whole next => keys env (next env)

/-- Whether a run reads the whole environment: the record's whole-map flag. -/
def readsWhole (env : K → V) : Prog K V R → Bool
  | .done _ => false
  | .ask k next => readsWhole env (next (env k))
  | .whole _ => true

/-- **One unit: a run that read no whole table is determined by the names it
read.** Under another environment agreeing on them, it reads the same names
and returns the same result. -/
theorem run_eq_of_agree (env env' : K → V) :
    ∀ p : Prog K V R, readsWhole env p = false → (∀ k ∈ keys env p, env k = env' k) →
      run env' p = run env p ∧ keys env' p = keys env p
  | .done _, _, _ => ⟨rfl, rfl⟩
  | .ask k next, whole, agree => by
    have head : env k = env' k := agree k (by simp [keys])
    simp only [readsWhole] at whole
    have tail : ∀ k' ∈ keys env (next (env k)), env k' = env' k' :=
      fun k' h => agree k' (by simp [keys, h])
    obtain ⟨r, ks⟩ := run_eq_of_agree env env' (next (env k)) whole tail
    simp only [run, keys, ← head]
    exact ⟨r, by rw [ks]⟩
  | .whole _, whole, _ => by simp [readsWhole] at whole

/-- The environment restricted to `ks`: each name with its entries. -/
def restrict (env : K → V) (ks : List K) : List (K × V) :=
  ks.map fun k => (k, env k)

theorem restrict_names (env : K → V) (ks : List K) : (restrict env ks).map Prod.fst = ks := by
  induction ks with
  | nil => rfl
  | cons k ks ih => simp only [restrict, List.map_cons] at ih ⊢; rw [ih]

/-- Equal restrictions agree on every restricted name. -/
theorem agree_of_restrict_eq {env env' : K → V} :
    ∀ {ks : List K}, restrict env ks = restrict env' ks → ∀ k ∈ ks, env k = env' k
  | [], _, _, h => by simp at h
  | k :: ks, h, k', mem => by
    simp only [restrict, List.map_cons, List.cons.injEq, Prod.mk.injEq] at h
    rcases List.mem_cons.mp mem with rfl | mem
    · exact h.1.2
    · exact agree_of_restrict_eq (ks := ks) h.2 k' mem

/-- A unit's analysis: its run identity and fingerprint (functions of the
route's inputs), and the consumer its analysis is, given those. -/
structure Consumer (Env Input Fp Row RunId K V : Type) where
  runId : Env → RunId
  fingerprint : Env → Input → Fp
  prog : RunId → Fp → Prog K V Row

variable {Env Input Fp Row RunId W : Type}

/-- The consumer as a staged analysis whose stage facts are the stage
environment. -/
def Consumer.analysis (c : Consumer Env Input Fp Row RunId K V) :
    QCore.StagedCheckpoints.Analysis Env Input Fp Row RunId (K → V) where
  runId := c.runId
  fingerprint := c.fingerprint
  analyze e f i := run f (c.prog (c.runId e) (c.fingerprint e i))

/-- A consumer's analysis meets the staged checkpoints' premise. -/
theorem Consumer.determined (c : Consumer Env Input Fp Row RunId K V) :
    QCore.StagedCheckpoints.Determined c.analysis := by
  intro e e' f i i' same fp
  have same : c.runId e = c.runId e' := same
  have fp : c.fingerprint e i = c.fingerprint e' i' := fp
  show run f (c.prog (c.runId e) (c.fingerprint e i)) =
    run f (c.prog (c.runId e') (c.fingerprint e' i'))
  rw [same, fp]

/-- What a stage identity's environment digest covers. -/
inductive Scope (K V W : Type) where
  | restricted : List (K × V) → Scope K V W
  | whole : W → Scope K V W
  deriving DecidableEq

/-- A sealed group: run identity, fingerprints, scope, rows. -/
structure Checkpoint (Fp Row RunId K V W : Type) where
  run : RunId
  units : List Fp
  scope : Scope K V W
  rows : List Row

/-- The consumers a group's units are, under one run identity. -/
def progs (c : Consumer Env Input Fp Row RunId K V) (e : Env) (g : List Input) :
    List (Prog K V Row) :=
  (g.map (c.fingerprint e)).map (c.prog (c.runId e))

/-- The scope a group's fresh records allow: restricted to every name its
units read when none read the whole environment, otherwise the whole
environment's digest. -/
def sealingScope (digest : (K → V) → W) (f : K → V) (ps : List (Prog K V Row)) :
    Scope K V W :=
  if ps.all (fun p => !readsWhole f p) then .restricted (restrict f (ps.flatMap (keys f)))
  else .whole (digest f)

/-- The checkpoint a group's computation in a stage seals. -/
def sealGroup (c : Consumer Env Input Fp Row RunId K V) (digest : (K → V) → W) (e : Env)
    (f : K → V) (g : List Input) : Checkpoint Fp Row RunId K V W :=
  { run := c.runId e, units := g.map (c.fingerprint e),
    scope := sealingScope digest f (progs c e g), rows := (progs c e g).map (run f) }

/-- The scope recomputed now, over the stored scope's names. -/
def current (digest : (K → V) → W) (f : K → V) : Scope K V W → Scope K V W
  | .restricted ps => .restricted (restrict f (ps.map Prod.fst))
  | .whole _ => .whole (digest f)

/-- Resume a group: reuse a stored checkpoint whose run identity,
fingerprints and scope recomputed now all equal the stored ones. -/
def resume [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K] [DecidableEq V] [DecidableEq W]
    (c : Consumer Env Input Fp Row RunId K V) (digest : (K → V) → W) (e : Env) (f : K → V)
    (g : List Input) : Option (Checkpoint Fp Row RunId K V W) → List Row
  | none => QCore.StagedCheckpoints.recompute c.analysis e f g
  | some cp =>
    if cp.run = c.runId e ∧ cp.units = g.map (c.fingerprint e) ∧ cp.scope = current digest f cp.scope
    then cp.rows else QCore.StagedCheckpoints.recompute c.analysis e f g

/-- A checkpoint the route sealed. -/
def Sealed (c : Consumer Env Input Fp Row RunId K V) (digest : (K → V) → W)
    (cp : Checkpoint Fp Row RunId K V W) : Prop :=
  ∃ e f g, cp = sealGroup c digest e f g

theorem recompute_eq (c : Consumer Env Input Fp Row RunId K V) (e : Env) (f : K → V)
    (g : List Input) :
    QCore.StagedCheckpoints.recompute c.analysis e f g = (progs c e g).map (run f) := by
  simp [QCore.StagedCheckpoints.recompute, progs, Consumer.analysis, List.map_map,
    Function.comp_def]

/-- Equal run identities and fingerprints give equal consumers. -/
theorem progs_eq (c : Consumer Env Input Fp Row RunId K V) {e e' : Env} {g g' : List Input}
    (run : c.runId e = c.runId e') (units : g.map (c.fingerprint e) = g'.map (c.fingerprint e')) :
    progs c e g = progs c e' g' := by
  simp only [progs, run, units]

/-- **A group: a reused checkpoint is the recomputation.** For any
checkpoint the route sealed, in whatever environment, stage and group, given
that equal whole digests are equal environments. -/
theorem reused_eq_recomputed [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K]
    [DecidableEq V] [DecidableEq W] {c : Consumer Env Input Fp Row RunId K V}
    {digest : (K → V) → W} (wholeSound : ∀ f f', digest f = digest f' → f = f')
    {cp : Checkpoint Fp Row RunId K V W} (sealed : Sealed c digest cp) (e : Env) (f : K → V)
    (g : List Input) :
    resume c digest e f g (some cp) = QCore.StagedCheckpoints.recompute c.analysis e f g := by
  obtain ⟨e₀, f₀, g₀, rfl⟩ := sealed
  simp only [resume]
  split
  · rename_i h
    obtain ⟨run, units, scope⟩ := h
    simp only [sealGroup] at run units scope ⊢
    rw [recompute_eq, ← progs_eq c run units]
    generalize progs c e₀ g₀ = ps at scope ⊢
    unfold sealingScope at scope
    split at scope
    · rename_i none
      simp only [current, restrict_names, Scope.restricted.injEq] at scope
      have agree := agree_of_restrict_eq scope
      apply List.map_congr_left
      intro p mem
      have whole : readsWhole f₀ p = false := by
        have := List.all_eq_true.mp none p mem
        simpa using this
      have keys_in : ∀ k ∈ keys f₀ p, f₀ k = f k :=
        fun k h => agree k (List.mem_flatMap.mpr ⟨p, mem, h⟩)
      exact (run_eq_of_agree f₀ f p whole keys_in).1.symm
    · simp only [current, Scope.whole.injEq] at scope
      rw [wholeSound _ _ scope]
  · rfl

/-- A restricted checkpoint is recomputed when a name a unit read changed. -/
theorem changed_key_recomputes [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K]
    [DecidableEq V] [DecidableEq W] (c : Consumer Env Input Fp Row RunId K V)
    (digest : (K → V) → W) (e : Env) (f : K → V) (g : List Input)
    (cp : Checkpoint Fp Row RunId K V W) (ps : List (K × V)) (stored : cp.scope = .restricted ps)
    (k : K) (v : V) (read : (k, v) ∈ ps) (changed : f k ≠ v) :
    resume c digest e f g (some cp) = QCore.StagedCheckpoints.recompute c.analysis e f g := by
  simp only [resume]
  split
  · rename_i h
    obtain ⟨_, _, scope⟩ := h
    rw [stored] at scope
    simp only [current, Scope.restricted.injEq] at scope
    have : (k, v) ∈ restrict f (ps.map Prod.fst) := scope ▸ read
    simp only [restrict, List.mem_map, Prod.mk.injEq] at this
    obtain ⟨_, _, rfl, rfl⟩ := this
    exact absurd rfl changed
  · rfl

/-- A whole-environment checkpoint is recomputed when the environment's
digest changed: a whole reader reruns on any change. -/
theorem whole_reader_recomputes [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K]
    [DecidableEq V] [DecidableEq W] (c : Consumer Env Input Fp Row RunId K V)
    (digest : (K → V) → W) (e : Env) (f : K → V) (g : List Input)
    (cp : Checkpoint Fp Row RunId K V W) (w : W) (stored : cp.scope = .whole w)
    (changed : digest f ≠ w) :
    resume c digest e f g (some cp) = QCore.StagedCheckpoints.recompute c.analysis e f g := by
  simp only [resume]
  split
  · rename_i h
    obtain ⟨_, _, scope⟩ := h
    rw [stored] at scope
    simp only [current, Scope.whole.injEq] at scope
    exact absurd scope.symm changed
  · rfl

/-- Whatever the checkpoint directory holds: per stage environment and
group, a stored checkpoint or none. -/
def Store (Input Fp Row RunId K V W : Type) :=
  (K → V) → List Input → Option (Checkpoint Fp Row RunId K V W)

/-- Every stored checkpoint was sealed by the route. -/
def AllSealed (c : Consumer Env Input Fp Row RunId K V) (digest : (K → V) → W)
    (s : Store Input Fp Row RunId K V W) : Prop :=
  ∀ f g cp, s f g = some cp → Sealed c digest cp

/-- One stage of the lookup-keyed run: every group resumed. -/
def stageResumed [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K] [DecidableEq V]
    [DecidableEq W] (c : Consumer Env Input Fp Row RunId K V) (digest : (K → V) → W) (e : Env)
    (s : Store Input Fp Row RunId K V W) (gs : List (List Input)) (f : K → V) :
    List (List Row) :=
  gs.map fun g => resume c digest e f g (s f g)

/-- **A lookup-keyed stage is the whole run's stage.** -/
theorem stageResumed_eq_stage [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K]
    [DecidableEq V] [DecidableEq W] {c : Consumer Env Input Fp Row RunId K V}
    {digest : (K → V) → W} (wholeSound : ∀ f f', digest f = digest f' → f = f')
    {s : Store Input Fp Row RunId K V W} (sealed : AllSealed c digest s) (e : Env)
    (gs : List (List Input)) (f : K → V) :
    stageResumed c digest e s gs f = QCore.StagedCheckpoints.stage c.analysis e gs f := by
  unfold stageResumed QCore.StagedCheckpoints.stage
  apply List.map_congr_left
  intro g _
  cases h : s f g with
  | none => rfl
  | some cp => exact reused_eq_recomputed wholeSound (sealed f g cp h) e f g

/-- The lookup-keyed run's later stages, from the current rows. -/
def resumed [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K] [DecidableEq V]
    [DecidableEq W] (c : Consumer Env Input Fp Row RunId K V) (digest : (K → V) → W) (e : Env)
    (s : Store Input Fp Row RunId K V W) (gs : List (List Input)) :
    List (List Row) → List (QCore.StagedCheckpoints.Later Row (K → V)) → List (List Row)
  | rows, [] => rows
  | rows, l :: ls =>
    match l.facts rows.flatten with
    | none => resumed c digest e s gs rows ls
    | some f =>
      let next := stageResumed c digest e s gs f
      if l.accept rows.flatten next.flatten then resumed c digest e s gs next ls
      else resumed c digest e s gs rows ls

/-- **The lookup-keyed staged run is the whole run**: from the first stage's
environment through any later stages (the refutation table, the imported
environment), the final rows of every group are the whole run's. -/
theorem resumed_eq_whole [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K]
    [DecidableEq V] [DecidableEq W] {c : Consumer Env Input Fp Row RunId K V}
    {digest : (K → V) → W} (wholeSound : ∀ f f', digest f = digest f' → f = f')
    {s : Store Input Fp Row RunId K V W} (sealed : AllSealed c digest s) (e : Env)
    (gs : List (List Input)) (first : K → V)
    (later : List (QCore.StagedCheckpoints.Later Row (K → V))) :
    resumed c digest e s gs (stageResumed c digest e s gs first) later =
      QCore.StagedCheckpoints.whole c.analysis e gs
        (QCore.StagedCheckpoints.stage c.analysis e gs first) later := by
  rw [stageResumed_eq_stage wholeSound sealed]
  generalize QCore.StagedCheckpoints.stage c.analysis e gs first = rows
  induction later generalizing rows with
  | nil => rfl
  | cons l ls ih =>
    simp only [resumed, QCore.StagedCheckpoints.whole]
    cases l.facts rows.flatten with
    | none => exact ih rows
    | some f =>
      simp only [stageResumed_eq_stage wholeSound sealed]
      split
      · exact ih _
      · exact ih rows

/-- **Its publication is the whole run's.** -/
theorem resumed_publication_eq_whole [DecidableEq Fp] [DecidableEq RunId] [DecidableEq K]
    [DecidableEq V] [DecidableEq W] {Src : Type} [DecidableEq Src]
    {c : Consumer Env Input Fp Row RunId K V} {digest : (K → V) → W}
    (wholeSound : ∀ f f', digest f = digest f' → f = f')
    {s : Store Input Fp Row RunId K V W} (sealed : AllSealed c digest s)
    (tally : Row → Nat) (source : Row → Src) (e : Env) (gs : List (List Input))
    (first : K → V) (later : List (QCore.StagedCheckpoints.Later Row (K → V))) :
    QCore.Checkpoints.composeGroups tally source
        (resumed c digest e s gs (stageResumed c digest e s gs first) later) =
      QCore.Checkpoints.publish tally source
        (QCore.StagedCheckpoints.whole c.analysis e gs
          (QCore.StagedCheckpoints.stage c.analysis e gs first) later).flatten := by
  rw [resumed_eq_whole wholeSound sealed, QCore.Checkpoints.compose_eq_whole]

end QCore.LookupRestrictedReuse
