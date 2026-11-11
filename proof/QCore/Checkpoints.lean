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

/-! Card **W45-AUDIT-CHECKPOINTS** — per-configuration audit checkpoints: identity-guarded
reuse and composition.

This is a model of `crates/qcore-audit/src/checkpoint_run.rs` (resume) and
`crates/qcore-audit/src/compose.rs` (composition). A checkpointed
`quirin-audit current` run analyses independent compile configurations, seals each
unit's rows and tallies with an identity, and publishes the report by folding
the groups' results. Three facts are proved.

* **Reuse is recomputation.** A group's stored rows are reused only when the
  stored identity (the run identity and every unit's fingerprint) equals the
  identity recomputed now. Under the premise that a unit's row is determined by
  the run identity and its own fingerprint (`Determined`), a reused result is
  exactly the recomputed one, for any checkpoint the route sealed — whatever
  earlier environment and units it was sealed from.
* **Composition is the whole run.** The whole run folds every unit's row into
  one publication (rows appended, tallies added, sources united). Folding each
  group separately and then folding the groups' publications gives the same
  publication, for any split of the unit list into consecutive groups: the fold
  is a monoid homomorphism from lists.
* **Resume, then compose, is the whole run.** The two together: a run whose
  groups were each reused or recomputed publishes exactly the whole run's
  publication.

A composition is admitted only when every checkpoint names the same run and
their plan positions are exactly the plan's; `compose_admits_one_run` and
`compose_covers_the_plan` state what an admitted composition establishes.

Deliberately **not** claimed here: that SHA-256 detects tampering (the Rust
reader checks every section's digest, and a checkpoint that fails any check is
treated as absent, which this model covers as `none`); that the Rust
fingerprint captures every input of a unit's analysis (the `Determined`
premise — the independence audit in `docs/reports/w45-audit-checkpoints/`
argues it input by input, and the corpus measurements test it); and that the
Rust renderer prints the folded publication byte for byte (the byte-identity
tests and the corpus comparison do). -/

namespace QCore.Checkpoints

/-! ## Identity-guarded reuse -/

/-- A run environment's identity, a unit's fingerprint and its analysis. The
environment, the unit inputs, fingerprints, rows and run identities are
abstract. -/
structure Analysis (Env Input Fp Row RunId : Type) where
  runId : Env → RunId
  fingerprint : Env → Input → Fp
  analyze : Env → Input → Row

variable {Env Input Fp Row RunId : Type}

/-- The premise every reuse rests on: a unit's row is determined by the run
identity and the unit's own fingerprint. -/
def Determined (a : Analysis Env Input Fp Row RunId) : Prop :=
  ∀ e e' i i', a.runId e = a.runId e' → a.fingerprint e i = a.fingerprint e' i' →
    a.analyze e i = a.analyze e' i'

/-- A sealed group: the identity it was computed under and its rows. -/
structure Checkpoint (Fp Row RunId : Type) where
  run : RunId
  units : List Fp
  rows : List Row

/-- The checkpoint a group's computation seals. -/
def sealGroup (a : Analysis Env Input Fp Row RunId) (e : Env) (g : List Input) :
    Checkpoint Fp Row RunId :=
  { run := a.runId e, units := g.map (a.fingerprint e), rows := g.map (a.analyze e) }

/-- The group's rows, computed now. -/
def recompute (a : Analysis Env Input Fp Row RunId) (e : Env) (g : List Input) : List Row :=
  g.map (a.analyze e)

/-- A checkpoint was sealed by the route, from some environment and units. -/
def Sealed (a : Analysis Env Input Fp Row RunId) (c : Checkpoint Fp Row RunId) : Prop :=
  ∃ e g, c = sealGroup a e g

/-- Resume a group: reuse a stored checkpoint whose identity equals the one
recomputed now; otherwise (no checkpoint, a refused one, or another identity)
recompute. -/
def resume [DecidableEq Fp] [DecidableEq RunId] (a : Analysis Env Input Fp Row RunId)
    (e : Env) (g : List Input) : Option (Checkpoint Fp Row RunId) → List Row
  | none => recompute a e g
  | some c =>
    if c.run = a.runId e ∧ c.units = g.map (a.fingerprint e) then c.rows else recompute a e g

/-- Equal fingerprints unit by unit, under one run identity, give equal rows
unit by unit. -/
theorem rows_eq_of_fingerprints_eq {a : Analysis Env Input Fp Row RunId}
    (determined : Determined a) {e e' : Env} (same : a.runId e = a.runId e') :
    ∀ (g g' : List Input), g.map (a.fingerprint e) = g'.map (a.fingerprint e') →
      g.map (a.analyze e) = g'.map (a.analyze e')
  | [], [], _ => rfl
  | [], _ :: _, h => by simp at h
  | _ :: _, [], h => by simp at h
  | i :: g, i' :: g', h => by
    simp only [List.map_cons, List.cons.injEq] at h ⊢
    exact ⟨determined e e' i i' same h.1, rows_eq_of_fingerprints_eq determined same g g' h.2⟩

/-- **A reused group result equals the recomputed one** whenever the identity
is equal, for any checkpoint the route sealed. -/
theorem reused_eq_recomputed [DecidableEq Fp] [DecidableEq RunId]
    {a : Analysis Env Input Fp Row RunId} (determined : Determined a)
    {c : Checkpoint Fp Row RunId} (sealed : Sealed a c) (e : Env) (g : List Input) :
    resume a e g (some c) = recompute a e g := by
  obtain ⟨e₀, g₀, rfl⟩ := sealed
  simp only [resume]
  by_cases h : (sealGroup a e₀ g₀).run = a.runId e ∧
      (sealGroup a e₀ g₀).units = g.map (a.fingerprint e)
  · rw [ite_eq_left h]
    exact rows_eq_of_fingerprints_eq determined h.1 g₀ g h.2
  · rw [ite_eq_right h]

/-- Resume is recomputation for every stored state the route can present: no
checkpoint, or a sealed one. -/
theorem resume_eq_recompute [DecidableEq Fp] [DecidableEq RunId]
    {a : Analysis Env Input Fp Row RunId} (determined : Determined a)
    (e : Env) (g : List Input) (stored : Option (Checkpoint Fp Row RunId))
    (sealed : ∀ c, stored = some c → Sealed a c) :
    resume a e g stored = recompute a e g := by
  cases stored with
  | none => rfl
  | some c => exact reused_eq_recomputed determined (sealed c rfl) e g

/-- Without the premise, reuse can differ: a stale checkpoint under an equal
identity is reused as stored. This is why the identity must capture every
input (the premise), not merely some. -/
theorem reuse_takes_the_stored_rows [DecidableEq Fp] [DecidableEq RunId]
    (a : Analysis Env Input Fp Row RunId) (e : Env) (g : List Input)
    (c : Checkpoint Fp Row RunId) (run : c.run = a.runId e)
    (units : c.units = g.map (a.fingerprint e)) :
    resume a e g (some c) = c.rows := by
  simp [resume, run, units]

/-! ## Composition -/

/-- What the whole run folds from its rows: the rows in order, a tally
(verdict counts, per-category diagnostic counts and the like are sums), and
the sources the rows stand for (a union: the report counts distinct source
files). -/
structure Publication (Row Src : Type) where
  rows : List Row
  tally : Nat
  sources : Src → Bool

variable {Src : Type}

namespace Publication

/-- Nothing published. -/
def empty : Publication Row Src := ⟨[], 0, fun _ => false⟩

/-- One publication after another. -/
def join (p q : Publication Row Src) : Publication Row Src :=
  ⟨p.rows ++ q.rows, p.tally + q.tally, fun s => p.sources s || q.sources s⟩

theorem empty_join (p : Publication Row Src) : join empty p = p := by
  cases p
  simp [join, empty]

theorem join_empty (p : Publication Row Src) : join p empty = p := by
  cases p
  simp [join, empty]

theorem join_assoc (p q r : Publication Row Src) :
    join (join p q) r = join p (join q r) := by
  cases p; cases q; cases r
  simp only [join, List.append_assoc, Nat.add_assoc, Publication.mk.injEq, true_and]
  funext s
  exact Bool.or_assoc _ _ _

end Publication

/-- The whole run's fold over its rows, given each row's tally and source. -/
def publish [DecidableEq Src] (tally : Row → Nat) (source : Row → Src) :
    List Row → Publication Row Src
  | [] => Publication.empty
  | r :: rs => Publication.join ⟨[r], tally r, fun s => decide (s = source r)⟩
      (publish tally source rs)

theorem publish_append [DecidableEq Src] (tally : Row → Nat) (source : Row → Src) :
    ∀ (xs ys : List Row),
      publish tally source (xs ++ ys) =
        Publication.join (publish tally source xs) (publish tally source ys)
  | [], ys => by simp [publish, Publication.empty_join]
  | x :: xs, ys => by
    simp only [List.cons_append, publish]
    rw [publish_append tally source xs ys, Publication.join_assoc]

/-- Composition: each group's publication, folded in plan order. -/
def composeGroups [DecidableEq Src] (tally : Row → Nat) (source : Row → Src) :
    List (List Row) → Publication Row Src
  | [] => Publication.empty
  | g :: gs => Publication.join (publish tally source g) (composeGroups tally source gs)

/-- **Composition is the whole run**: for any split of the rows into
consecutive groups, folding the groups' publications is folding the rows. -/
theorem compose_eq_whole [DecidableEq Src] (tally : Row → Nat) (source : Row → Src) :
    ∀ (gs : List (List Row)), composeGroups tally source gs = publish tally source gs.flatten
  | [] => rfl
  | g :: gs => by
    simp only [composeGroups, List.flatten_cons]
    rw [compose_eq_whole tally source gs, publish_append]

/-- A group's rows are its own units' rows: no unit reads another's analysis
(the independence premise the audit checks input by input). Then the groups'
rows, flattened, are the whole run's rows. -/
theorem group_rows_flatten (f : Input → Row) :
    ∀ (gs : List (List Input)), (gs.map (List.map f)).flatten = gs.flatten.map f
  | [] => rfl
  | g :: gs => by
    rw [List.map_cons, List.flatten_cons, List.flatten_cons, List.map_append,
      group_rows_flatten f gs]

/-- **Resume, then compose, is the whole run.** Each group reused or
recomputed under identity-guarded resume, each published, and the groups'
publications folded in plan order, give the whole run's publication over every
unit. -/
theorem resume_then_compose_eq_whole [DecidableEq Fp] [DecidableEq RunId] [DecidableEq Src]
    {a : Analysis Env Input Fp Row RunId} (determined : Determined a)
    (tally : Row → Nat) (source : Row → Src) (e : Env) (gs : List (List Input))
    (stored : List Input → Option (Checkpoint Fp Row RunId))
    (sealed : ∀ g c, stored g = some c → Sealed a c) :
    composeGroups tally source (gs.map fun g => resume a e g (stored g)) =
      publish tally source (gs.flatten.map (a.analyze e)) := by
  have resumed : (gs.map fun g => resume a e g (stored g)) = gs.map (List.map (a.analyze e)) :=
    List.map_congr_left fun g _ => resume_eq_recompute determined e g (stored g) (sealed g)
  rw [resumed, compose_eq_whole, group_rows_flatten]

/-- A singleton checkpoint is enough to publish one configuration; later
completed configurations may be stored independently while ordered publication
waits for an earlier unit. The wait changes neither the rows nor their fold. -/
theorem per_unit_resume_then_compose_eq_whole [DecidableEq Fp] [DecidableEq RunId]
    [DecidableEq Src] {a : Analysis Env Input Fp Row RunId} (determined : Determined a)
    (tally : Row → Nat) (source : Row → Src) (e : Env) (units : List Input)
    (stored : List Input → Option (Checkpoint Fp Row RunId))
    (sealed : ∀ g c, stored g = some c → Sealed a c) :
    composeGroups tally source
      (units.map fun unit => resume a e [unit] (stored [unit])) =
      publish tally source (units.map (a.analyze e)) := by
  have h := resume_then_compose_eq_whole determined tally source e
    (units.map fun unit => [unit]) stored sealed
  have flat (xs : List Input) : (xs.map fun unit => [unit]).flatten = xs := by
    induction xs with
    | nil => rfl
    | cons unit rest ih => simp [ih]
  simpa [List.map_map, Function.comp_def, flat] using h

/-! ## Admission -/

/-- Every checkpoint of a composition names the same run. -/
def OneRun (cs : List (Nat × Checkpoint Fp Row RunId)) : Prop :=
  ∀ c ∈ cs, ∀ d ∈ cs, c.2.run = d.2.run

instance [DecidableEq RunId] (cs : List (Nat × Checkpoint Fp Row RunId)) :
    Decidable (OneRun cs) := by
  unfold OneRun
  infer_instance

/-- A composition's inputs: each checkpoint with its plan position. It is
admitted only when there is at least one checkpoint, the positions are exactly
the plan's `0, …, n-1` in order (no group missing, none twice), and every
checkpoint names one run; otherwise it refuses. -/
def compose? [DecidableEq RunId] [DecidableEq Src] (tally : Row → Nat) (source : Row → Src)
    (n : Nat) (cs : List (Nat × Checkpoint Fp Row RunId)) : Option (Publication Row Src) :=
  if cs ≠ [] ∧ cs.map Prod.fst = List.range n ∧ OneRun cs then
    some (composeGroups tally source (cs.map fun c => c.2.rows))
  else none

/-- An admitted composition names one run. -/
theorem compose_admits_one_run [DecidableEq RunId] [DecidableEq Src]
    {tally : Row → Nat} {source : Row → Src} {n : Nat}
    {cs : List (Nat × Checkpoint Fp Row RunId)} {p : Publication Row Src}
    (admitted : compose? tally source n cs = some p) : OneRun cs := by
  unfold compose? at admitted
  by_cases h : cs ≠ [] ∧ cs.map Prod.fst = List.range n ∧ OneRun cs
  · exact h.2.2
  · rw [ite_eq_right h] at admitted
    cases admitted

/-- An admitted composition covers the plan: no group missing, none twice. -/
theorem compose_covers_the_plan [DecidableEq RunId] [DecidableEq Src]
    {tally : Row → Nat} {source : Row → Src} {n : Nat}
    {cs : List (Nat × Checkpoint Fp Row RunId)} {p : Publication Row Src}
    (admitted : compose? tally source n cs = some p) : cs.map Prod.fst = List.range n := by
  unfold compose? at admitted
  by_cases h : cs ≠ [] ∧ cs.map Prod.fst = List.range n ∧ OneRun cs
  · exact h.2.1
  · rw [ite_eq_right h] at admitted
    cases admitted

/-- The checkpoints one run seals, one per group at its plan position, are
admitted, and their composition is the whole run over every unit. -/
theorem one_run_composes_to_the_whole [DecidableEq RunId] [DecidableEq Src]
    (a : Analysis Env Input Fp Row RunId) (tally : Row → Nat) (source : Row → Src) (e : Env)
    (n : Nat) (cs : List (Nat × Checkpoint Fp Row RunId)) (groups : List (List Input))
    (nonempty : cs ≠ []) (positions : cs.map Prod.fst = List.range n)
    (sealedHere : cs.map Prod.snd = groups.map (sealGroup a e)) :
    compose? tally source n cs =
      some (publish tally source (groups.flatten.map (a.analyze e))) := by
  have runs : ∀ c ∈ cs, c.2.run = a.runId e := by
    intro c hc
    have member : c.2 ∈ groups.map (sealGroup a e) := by
      rw [← sealedHere]
      exact List.mem_map_of_mem hc
    obtain ⟨g, _, sealed⟩ := List.mem_map.mp member
    rw [← sealed]
    rfl
  have one : OneRun cs := fun c hc d hd => by rw [runs c hc, runs d hd]
  have rows : (cs.map fun c => c.2.rows) = groups.map (List.map (a.analyze e)) := by
    have : (cs.map fun c => c.2.rows) = (cs.map Prod.snd).map Checkpoint.rows := by
      rw [List.map_map]
      rfl
    rw [this, sealedHere, List.map_map]
    rfl
  unfold compose?
  rw [ite_eq_left ⟨nonempty, positions, one⟩, rows, compose_eq_whole, group_rows_flatten]

end QCore.Checkpoints
