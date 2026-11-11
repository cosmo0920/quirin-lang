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

namespace QCore.CallUseAdmission

/-! Admission of a transported call-result def/use proposal (card `P04-NEXT-AB`).

Model of the third transported source vocabulary and of
`qcore_verification::call_use_source`'s `WitnessChecker::check` body.

Two things are modelled, and nothing else:

1. **The vocabulary is closed.** A source operation outside the listed forms,
   and a call to a callee outside the family's closed table, have no
   projection at all — so the function keeps its existing `Unknown`. There is
   no lossy encoding and no fallback.
2. **Agreement discharges nothing.** The trusted core evaluates the pinned
   contract over its own import, and a proposal is admitted only when it
   *equals* the reading the core computed for itself.

Everything the core cannot decide — successful OS cleanup, descriptor and
mapping custody, the `close`/`munmap`/`unlink` ABI bindings — stays a declared
premise of the pinned contract and is outside every statement here. A recorded
call is a recorded call. -/

/-! ### The closed operation vocabulary -/

/-- An operation the projection can represent. `deps` abstracts the set of
earlier operation indices this operation's own operands depend on; the model
does not interpret expressions, only that dependency set. -/
inductive Op where
  | read (deps : List Nat)
  | write (deps : List Nat)
  | localStart
  | localWrite (deps : List Nat)
  | localEnd
  | guard (deps : List Nat)
  | call (callee : Nat) (deps : List Nat)
  | returnValue (deps : List Nat)
  | ret
  deriving DecidableEq, Repr

/-- What an importer may encounter. The last three constructors stand for the
forms the projector refuses **by name**: structured loops, a declared
nonreturning termination, and an expression form outside the closed expression
vocabulary (a subscript, an address-taken stack slot, a sizeof operand, an
aggregate initializer, a loop index, a callee designator). -/
inductive SourceOp where
  | read (deps : List Nat)
  | write (deps : List Nat)
  | localStart
  | localWrite (deps : List Nat)
  | localEnd
  | guard (deps : List Nat)
  | call (callee : Nat) (deps : List Nat)
  | returnValue (deps : List Nat)
  | ret
  | scanLoop
  | terminate
  | outsideExpressionVocabulary (deps : List Nat)
  deriving DecidableEq, Repr

/-- Project one operation against the family's closed callee table. -/
def projectOp (admitted : List Nat) : SourceOp → Option Op
  | .read d => some (.read d)
  | .write d => some (.write d)
  | .localStart => some .localStart
  | .localWrite d => some (.localWrite d)
  | .localEnd => some .localEnd
  | .guard d => some (.guard d)
  | .call c d => if admitted.contains c then some (.call c d) else none
  | .returnValue d => some (.returnValue d)
  | .ret => some .ret
  | .scanLoop => none
  | .terminate => none
  | .outsideExpressionVocabulary _ => none

/-- Project a whole path. One refused operation refuses the whole projection;
there is no partial encoding. -/
def project (admitted : List Nat) : List SourceOp → Option (List Op)
  | [] => some []
  | op :: rest =>
    match projectOp admitted op, project admitted rest with
    | some o, some os => some (o :: os)
    | _, _ => none

/-! ### The def/use reading -/

/-- One call and every later operation of the same path that observes its
result. `guardUses` is the subset of `uses` that are guard conditions. -/
structure CallUse where
  operation : Nat
  callee : Nat
  uses : List Nat
  guardUses : List Nat
  deriving DecidableEq, Repr

def deps : Op → List Nat
  | .read d => d
  | .write d => d
  | .localStart => []
  | .localWrite d => d
  | .localEnd => []
  | .guard d => d
  | .call _ d => d
  | .returnValue d => d
  | .ret => []

def isGuard : Op → Bool
  | .guard _ => true
  | _ => false

/-- One step of the reading, exactly in the order the producer and the core
both perform it: record uses of already-open calls first, then open the call
this operation may itself be. -/
def step (calls : List CallUse) (i : Nat) (op : Op) : List CallUse :=
  let observed := calls.map (fun c =>
    if (deps op).contains c.operation then
      { c with
        uses := c.uses ++ [i],
        guardUses := if isGuard op then c.guardUses ++ [i] else c.guardUses }
    else c)
  match op with
  | .call callee _ =>
    observed ++ [{ operation := i, callee := callee, uses := [], guardUses := [] }]
  | _ => observed

def go : Nat → List CallUse → List Op → List CallUse
  | _, acc, [] => acc
  | i, acc, op :: rest => go (i + 1) (step acc i op) rest

/-- The reading of one complete path. -/
def readPath (ops : List Op) : List CallUse := go 0 [] ops

/-! ### The vocabulary is closed: refusals -/

theorem a_scan_loop_has_no_projection (admitted : List Nat) :
    projectOp admitted .scanLoop = none := rfl

theorem a_termination_has_no_projection (admitted : List Nat) :
    projectOp admitted .terminate = none := rfl

theorem an_expression_outside_the_vocabulary_has_no_projection
    (admitted : List Nat) (d : List Nat) :
    projectOp admitted (.outsideExpressionVocabulary d) = none := rfl

/-- A callee outside the closed table refuses the operation. -/
theorem an_unadmitted_callee_has_no_projection {admitted : List Nat} {c : Nat} {d : List Nat}
    (outside : c ∉ admitted) :
    projectOp admitted (.call c d) = none := by
  simp [projectOp, outside]

/-- One refused operation refuses the whole path, so a body outside the
vocabulary keeps its existing `Unknown` rather than being partially encoded. -/
theorem a_refused_operation_refuses_the_path {admitted : List Nat} {op : SourceOp}
    {rest : List SourceOp} (refused : projectOp admitted op = none) :
    project admitted (op :: rest) = none := by
  simp [project, refused]

theorem a_refused_tail_refuses_the_path {admitted : List Nat} {op : SourceOp}
    {rest : List SourceOp} (refused : project admitted rest = none) :
    project admitted (op :: rest) = none := by
  cases h : projectOp admitted op <;> simp [project, h, refused]

/-- A path containing a scan loop anywhere has no projection. -/
theorem a_path_with_a_scan_loop_has_no_projection (admitted : List Nat) :
    ∀ pre post : List SourceOp,
      project admitted (pre ++ .scanLoop :: post) = none := by
  intro pre
  induction pre with
  | nil => intro post; exact a_refused_operation_refuses_the_path rfl
  | cons op rest ih =>
    intro post
    exact a_refused_tail_refuses_the_path (ih post)

/-! ### The closed callee table is part of the question -/

/-- Widening the table can only admit more: the table is monotone, so a
narrower table never accepts a body a wider one refuses. -/
theorem a_narrower_table_never_admits_more {narrow wide : List Nat} {op : SourceOp} {o : Op}
    (contained : ∀ c, c ∈ narrow → c ∈ wide)
    (admitted : projectOp narrow op = some o) :
    projectOp wide op = some o := by
  cases op <;> simp_all [projectOp]

/-- The table is load-bearing, not decorative: there is a body one table
projects and another refuses. -/
theorem the_table_decides_a_real_body :
    project [7] [.call 7 []] = some [.call 7 []] ∧ project [] [.call 7 []] = none := by
  constructor <;> rfl

/-! ### Invariants of the reading -/

/-- Every guarded use is a use. Established as an invariant of the fold, so it
holds for every path in the vocabulary rather than for sampled ones. -/
theorem guard_uses_are_uses :
    ∀ (ops : List Op) (i : Nat) (acc : List CallUse),
      (∀ c ∈ acc, ∀ u ∈ c.guardUses, u ∈ c.uses) →
      ∀ c ∈ go i acc ops, ∀ u ∈ c.guardUses, u ∈ c.uses := by
  intro ops
  induction ops with
  | nil => intro i acc inv c hc u hu; exact inv c hc u hu
  | cons op rest ih =>
    intro i acc inv
    refine ih (i + 1) (step acc i op) ?_
    intro c hc u hu
    unfold step at hc
    have observed :
        ∀ c ∈ acc.map (fun c =>
              if (deps op).contains c.operation then
                { c with
                  uses := c.uses ++ [i],
                  guardUses := if isGuard op then c.guardUses ++ [i] else c.guardUses }
              else c),
          ∀ u ∈ c.guardUses, u ∈ c.uses := by
      intro c hc u hu
      simp only [List.mem_map] at hc
      obtain ⟨d, hd, rfl⟩ := hc
      by_cases hdep : (deps op).contains d.operation
      · simp only [hdep, ite_true] at hu ⊢
        by_cases hg : isGuard op
        · simp only [hg, ite_true] at hu
          simp only [List.mem_append, List.mem_singleton] at hu ⊢
          rcases hu with hu | hu
          · exact Or.inl (inv d hd u hu)
          · exact Or.inr hu
        · simp only [hg] at hu
          simp only [List.mem_append, List.mem_singleton]
          exact Or.inl (inv d hd u hu)
      · simp only [hdep] at hu ⊢
        exact inv d hd u hu
    cases op with
    | call callee d =>
      simp only at hc
      rcases List.mem_append.mp hc with h | h
      · exact observed c h u hu
      · simp only [List.mem_singleton] at h
        subst h
        simp at hu
    | _ => exact observed c hc u hu

/-- Applied at the entry point: every reading of every path in the vocabulary
satisfies the invariant. -/
theorem every_reading_has_guard_uses_inside_uses (ops : List Op) :
    ∀ c ∈ readPath ops, ∀ u ∈ c.guardUses, u ∈ c.uses := by
  refine guard_uses_are_uses ops 0 [] ?_
  intro c hc
  simp at hc

/-- A call-free path reads as no calls, so a producer cannot report a call the
transported path does not contain. -/
theorem a_call_free_path_reads_as_no_calls :
    ∀ (ops : List Op) (i : Nat) (acc : List CallUse),
      (∀ op ∈ ops, ∀ c d, op ≠ .call c d) → (go i acc ops).length = acc.length := by
  intro ops
  induction ops with
  | nil => intro i acc _; rfl
  | cons op rest ih =>
    intro i acc callFree
    have tail : ∀ o ∈ rest, ∀ c d, o ≠ .call c d := by
      intro o ho; exact callFree o (List.mem_cons_of_mem op ho)
    have stepLen : (step acc i op).length = acc.length := by
      cases op with
      | call c d =>
        exact absurd rfl (callFree (.call c d) (List.mem_cons_self ..) c d)
      | _ => simp [step]
    have unfolded : go i acc (op :: rest) = go (i + 1) (step acc i op) rest := rfl
    rw [unfolded, ih (i + 1) (step acc i op) tail, stepLen]

/-! ### Non-vacuity: the reading really is computed -/

/-- The wrapper shape: four calls, none of whose results is observed. -/
theorem an_ignored_result_reads_as_ignored :
    readPath [.read [], .call 9 [0], .ret] =
      [{ operation := 1, callee := 9, uses := [], guardUses := [] }] := by
  rfl

/-- The native shape: one call, stored into a local and then guarded on. The
stored use and the guarded use are both recorded, and the guarded one is the
subset. -/
theorem a_stored_and_guarded_result_reads_as_both :
    readPath [.read [], .call 9 [0], .localWrite [1], .guard [1], .ret] =
      [{ operation := 1, callee := 9, uses := [2, 3], guardUses := [3] }] := by
  rfl

/-- Dropping the guard changes the reading rather than normalizing it away, so
the core's comparison and its own contract evaluation both see the change. -/
theorem dropping_the_guard_changes_the_reading :
    readPath [.read [], .call 9 [0], .localWrite [1], .ret] ≠
      readPath [.read [], .call 9 [0], .localWrite [1], .guard [1], .ret] := by
  decide

/-! ### Admission -/

/-- An abstract structural reading of one body: the per-path def/use lists.
Nothing here interprets it; only equality matters. -/
structure Reading where
  content : List (List CallUse)
  deriving DecidableEq, Repr

/-- The core's own derived task. `digest` is its recorded identity, which binds
the projection, the pinned contract, the selected policy and the closed callee
table the projection was taken under. -/
structure Task where
  digest : Nat
  deriving DecidableEq, Repr

structure Witness where
  taskDigest : Nat
  proposal : Reading
  deriving DecidableEq, Repr

/-- What the core does on its own: evaluate the pinned contract over its own
import and read the certificate it produced. `none` is a refusal. -/
abbrev CoreEvaluation := Task → Option Reading

/-- The implemented admission rule, in the order the Rust body checks it. -/
def admits (evaluate : CoreEvaluation) (own presented : Task) (w : Witness) : Bool :=
  presented == own &&
  w.taskDigest == own.digest &&
  (match evaluate own with
   | none => false
   | some reading => w.proposal == reading)

/-- Agreement alone discharges nothing: when the core's own contract evaluation
refuses, no witness whatsoever is admitted. -/
theorem a_refusing_core_admits_nothing {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} (refused : evaluate own = none) :
    admits evaluate own presented w = false := by
  simp [admits, refused]

/-- Every admission is backed by a successful evaluation the core ran itself. -/
theorem an_admitted_witness_has_a_core_evaluation {evaluate : CoreEvaluation}
    {own presented : Task} {w : Witness} (admitted : admits evaluate own presented w = true) :
    ∃ reading, evaluate own = some reading := by
  unfold admits at admitted
  cases h : evaluate own with
  | none => rw [h] at admitted; simp at admitted
  | some reading => exact ⟨reading, rfl⟩

/-- An admitted proposal is exactly the reading the core computed; a producer
never contributes content, only agreement. -/
theorem only_the_core_reading_is_admitted {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} {reading : Reading} (evaluated : evaluate own = some reading)
    (admitted : admits evaluate own presented w = true) :
    w.proposal = reading := by
  unfold admits at admitted
  rw [evaluated] at admitted
  simp at admitted
  exact admitted.2

/-- Two admitted witnesses cannot disagree. -/
theorem admitted_witnesses_never_disagree {evaluate : CoreEvaluation} {own presented : Task}
    {v w : Witness} (a : admits evaluate own presented v = true)
    (b : admits evaluate own presented w = true) :
    v.proposal = w.proposal := by
  obtain ⟨reading, evaluated⟩ := an_admitted_witness_has_a_core_evaluation a
  rw [only_the_core_reading_is_admitted evaluated a,
      only_the_core_reading_is_admitted evaluated b]

/-- Changing what the core computes retires the witness. -/
theorem a_changed_core_reading_retires_the_witness {evaluate : CoreEvaluation}
    {own presented : Task} {w : Witness} {before after : Reading}
    (evaluated : evaluate own = some before) (admitted : admits evaluate own presented w = true)
    (moved : after ≠ before) :
    w.proposal ≠ after := by
  rw [only_the_core_reading_is_admitted evaluated admitted]
  exact fun h => moved h.symm

/-- A witness bound to another task is refused before any replay. -/
theorem a_cross_task_witness_is_refused {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} (other : w.taskDigest ≠ own.digest) :
    admits evaluate own presented w = false := by
  simp [admits, other]

/-- A task the checker did not derive is refused even with a real witness. -/
theorem a_foreign_task_is_refused {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} (foreign : presented ≠ own) :
    admits evaluate own presented w = false := by
  simp [admits, foreign]

/-- Non-vacuity: the core's own reading is admitted. -/
theorem the_core_reading_is_admitted {evaluate : CoreEvaluation} {own : Task} {reading : Reading}
    (evaluated : evaluate own = some reading) :
    admits evaluate own own { taskDigest := own.digest, proposal := reading } = true := by
  simp [admits, evaluated]

/-! ### The dependent-capability boundary

A dependent capability never receives a prerequisite *certificate*: it receives
only an identity triple, and the trusted core re-derives the prerequisite task
from its own import and re-runs the prerequisite's own evaluation. Admission of
the dependent therefore cannot be bought by anything a producer sends. -/

structure Prerequisite where
  semanticProfile : Nat
  taskDigest : Nat
  obligationId : Nat
  deriving DecidableEq, Repr

/-- The identity the core derives for itself from its own task. -/
def derivedIdentity (profile : Nat) (own : Task) (obligation : Nat) : Prerequisite :=
  { semanticProfile := profile, taskDigest := own.digest, obligationId := obligation }

/-- Re-validation: the claimed identity must be the one the core derives, and
the prerequisite's own evaluation must then succeed. Nothing the claim carries
is read as a conclusion. -/
def revalidate (evaluate : CoreEvaluation) (profile : Nat) (own : Task) (obligation : Nat)
    (claimed : Prerequisite) : Option Reading :=
  if claimed == derivedIdentity profile own obligation then evaluate own else none

/-- A claimed prerequisite that is not the core's own is refused. -/
theorem a_foreign_prerequisite_is_refused {evaluate : CoreEvaluation} {profile : Nat}
    {own : Task} {obligation : Nat} {claimed : Prerequisite}
    (foreign : claimed ≠ derivedIdentity profile own obligation) :
    revalidate evaluate profile own obligation claimed = none := by
  simp [revalidate, foreign]

/-- Re-validation always re-runs the prerequisite's own evaluation: a
prerequisite whose contract evaluation refuses can never be established, no
matter what identity is claimed. -/
theorem a_refusing_prerequisite_is_never_established {evaluate : CoreEvaluation} {profile : Nat}
    {own : Task} {obligation : Nat} {claimed : Prerequisite}
    (refused : evaluate own = none) :
    revalidate evaluate profile own obligation claimed = none := by
  unfold revalidate
  split <;> simp [refused]

/-- Every field of the identity is load-bearing: a differing profile, digest or
obligation identifier each refuse. -/
theorem every_prerequisite_field_is_load_bearing {evaluate : CoreEvaluation} {profile : Nat}
    {own : Task} {obligation : Nat} {claimed : Prerequisite}
    (established : ∃ r, revalidate evaluate profile own obligation claimed = some r) :
    claimed.semanticProfile = profile ∧ claimed.taskDigest = own.digest ∧
      claimed.obligationId = obligation := by
  obtain ⟨r, hr⟩ := established
  unfold revalidate at hr
  split at hr
  · next h =>
    have : claimed = derivedIdentity profile own obligation := by simpa using h
    subst this
    exact ⟨rfl, rfl, rfl⟩
  · simp at hr

/-- Non-vacuity: the core's own derived identity is established when the
prerequisite's own evaluation succeeds. -/
theorem the_derived_prerequisite_is_established {evaluate : CoreEvaluation} {profile : Nat}
    {own : Task} {obligation : Nat} {reading : Reading} (evaluated : evaluate own = some reading) :
    revalidate evaluate profile own obligation (derivedIdentity profile own obligation)
      = some reading := by
  simp [revalidate, evaluated]

/-- A prerequisite established for one task never establishes another: two
tasks with different digests have different identities. -/
theorem a_prerequisite_does_not_cross_tasks {profile : Nat} {a b : Task} {obligation : Nat}
    (different : a.digest ≠ b.digest) :
    derivedIdentity profile a obligation ≠ derivedIdentity profile b obligation := by
  simp [derivedIdentity, different]

end QCore.CallUseAdmission
