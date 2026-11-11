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

namespace QCore.TeardownAdmission

/-! Admission of a transported teardown proposal (card `P04-NEXT`).

Model of `qcore_verification::teardown_source`'s two `WitnessChecker::check`
bodies. The point of the model is that **agreement discharges nothing**: the
trusted core evaluates the pinned contract over its own import, and a proposal
is admitted only when it *equals* the reading the core computed for itself.
Everything the core cannot decide — ring membership, exclusive link access,
allocation provenance, field disjointness, the `free`/libc binding — stays a
declared premise of the pinned contract and is outside every statement here. -/

inductive Family where
  | listUnlink
  | fieldDestructor
  deriving DecidableEq, Repr

/-- An abstract structural reading. `content` stands for the computed cases:
the unlink's per-alias-class link writes, or the destructor's per-nullable-state
release order. Nothing here interprets it; only equality matters. -/
structure Reading where
  family : Family
  content : Nat
  deriving DecidableEq, Repr

/-- The core's own derived task. `digest` is its recorded identity. -/
structure Task where
  digest : Nat
  family : Family
  deriving DecidableEq, Repr

structure Witness where
  taskDigest : Nat
  proposal : Reading
  deriving DecidableEq, Repr

/-- What the core does on its own: evaluate the pinned contract over its own
import. `none` is a refusal; `some r` is the reading the core computed. -/
abbrev CoreEvaluation := Task → Option Reading

/-- The implemented admission rule, in the order the Rust body checks it. -/
def admits (evaluate : CoreEvaluation) (own presented : Task) (w : Witness) : Bool :=
  presented == own &&
  w.taskDigest == own.digest &&
  w.proposal.family == own.family &&
  (match evaluate own with
   | none => false
   | some reading => w.proposal == reading)

/-! ### The core's own evaluation is what decides -/

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
  obtain ⟨_, proposal⟩ := admitted
  exact proposal

/-- Two admitted witnesses cannot disagree: admission is a function of the
core's own evaluation, so the proposal is determined. -/
theorem admitted_witnesses_never_disagree {evaluate : CoreEvaluation} {own p q : Task}
    {v w : Witness} (first : admits evaluate own p v = true)
    (second : admits evaluate own q w = true) :
    v.proposal = w.proposal := by
  obtain ⟨reading, evaluated⟩ := an_admitted_witness_has_a_core_evaluation first
  rw [only_the_core_reading_is_admitted evaluated first,
    only_the_core_reading_is_admitted evaluated second]

/-! ### Identity binding -/

/-- A witness bound to another task digest is refused before replay. -/
theorem a_cross_task_witness_is_refused {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} (bound_elsewhere : w.taskDigest ≠ own.digest) :
    admits evaluate own presented w = false := by
  simp [admits, bound_elsewhere]

/-- A task the checker did not derive is refused whatever the witness says. -/
theorem a_foreign_task_is_refused {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} (different : presented ≠ own) :
    admits evaluate own presented w = false := by
  simp [admits, different]

/-- One family's proposal never satisfies the other family's checker, even at
the right digest and with a successful evaluation. -/
theorem a_cross_family_proposal_is_refused {evaluate : CoreEvaluation} {own presented : Task}
    {w : Witness} (other : w.proposal.family ≠ own.family) :
    admits evaluate own presented w = false := by
  simp [admits, other]

/-- Admission implies all three identity facts hold together. -/
theorem an_admitted_witness_is_bound_to_its_own_task {evaluate : CoreEvaluation}
    {own presented : Task} {w : Witness} (admitted : admits evaluate own presented w = true) :
    presented = own ∧ w.taskDigest = own.digest ∧ w.proposal.family = own.family := by
  unfold admits at admitted
  cases h : evaluate own with
  | none => rw [h] at admitted; simp at admitted
  | some reading =>
    rw [h] at admitted
    simp at admitted
    obtain ⟨⟨⟨task, digest⟩, family⟩, _⟩ := admitted
    exact ⟨task, digest, family⟩

/-- The rule is not vacuously false: the core's own reading is admitted. -/
theorem the_core_reading_is_admitted (own : Task) (reading : Reading)
    (family_matches : reading.family = own.family) :
    admits (fun _ => some reading) own own ⟨own.digest, reading⟩ = true := by
  simp [admits, family_matches]

/-- Changing what the core computes retires every previously admitted witness:
a reading is admitted for exactly one evaluation outcome. -/
theorem a_changed_core_reading_retires_the_witness {own : Task} {w : Witness}
    {before after : Reading} (moved : before ≠ after)
    (admitted : admits (fun _ => some before) own own w = true) :
    admits (fun _ => some after) own own w = false := by
  have proposal : w.proposal = before := only_the_core_reading_is_admitted rfl admitted
  unfold admits
  simp [proposal, moved]

/-! ### The two-summary question

The unlink family transports two bodies. The recorded identity is a function of
*both*, so a helper substitution is a different question rather than a reuse of
the entry's evidence. `digest` is any injective recorder; the Rust route uses
SHA-256 over the complete projection plus the helper's own identity key. -/

structure UnlinkQuestion where
  entry : Nat
  helper : Nat
  contract : Nat
  deriving DecidableEq, Repr

theorem a_changed_helper_changes_the_question {digest : UnlinkQuestion → Nat}
    (injective : Function.Injective digest) {q r : UnlinkQuestion}
    (different_helper : q.helper ≠ r.helper) :
    digest q ≠ digest r := by
  intro equal
  exact different_helper (congrArg UnlinkQuestion.helper (injective equal))

theorem a_changed_entry_changes_the_question {digest : UnlinkQuestion → Nat}
    (injective : Function.Injective digest) {q r : UnlinkQuestion}
    (different_entry : q.entry ≠ r.entry) :
    digest q ≠ digest r := by
  intro equal
  exact different_entry (congrArg UnlinkQuestion.entry (injective equal))

theorem a_changed_contract_changes_the_question {digest : UnlinkQuestion → Nat}
    (injective : Function.Injective digest) {q r : UnlinkQuestion}
    (different_contract : q.contract ≠ r.contract) :
    digest q ≠ digest r := by
  intro equal
  exact different_contract (congrArg UnlinkQuestion.contract (injective equal))

/-- Consequence: a witness earned against one helper body is refused by the
task that transports another, in the direction that matters (no reuse). -/
theorem an_entry_witness_does_not_survive_a_helper_substitution
    {digest : UnlinkQuestion → Nat} (injective : Function.Injective digest)
    {q r : UnlinkQuestion} (different_helper : q.helper ≠ r.helper)
    (evaluate : CoreEvaluation) (proposal : Reading) :
    admits evaluate ⟨digest r, Family.listUnlink⟩ ⟨digest r, Family.listUnlink⟩
      ⟨digest q, proposal⟩ = false :=
  a_cross_task_witness_is_refused (a_changed_helper_changes_the_question injective different_helper)

/-! ### The declared owned-field inventory

The destructor family's projection carries the inventory the pinned contract
declares, because the inventory selects which nullable states are enumerated.
The core re-reads that inventory from the contract and refuses any other set,
so a producer cannot choose a smaller domain to answer over. -/

structure Field where
  name : String
  parent : Option String
  deriving DecidableEq, Repr

/-- A nullable state is realizable when every live field is declared and every
live child's declared parent is live too: a null parent has no independently
live child storage. -/
def realizable (fields : List Field) (live : List String) : Bool :=
  live.all fun n =>
    match fields.find? (fun f => f.name == n) with
    | none => false
    | some f =>
      match f.parent with
      | none => true
      | some p => live.contains p

def subsets : List String → List (List String)
  | [] => [[]]
  | x :: xs =>
    let rest := subsets xs
    rest ++ rest.map (fun s => x :: s)

/-- The enumeration domain: every realizable nullable state of the inventory. -/
def states (fields : List Field) : List (List String) :=
  (subsets (fields.map Field.name)).filter (realizable fields)

/-- The memory-backend inventory `providers/qcore-provider-chunkio/contracts/chunkio/memory_backend.qv`
declares. -/
def memfsFields : List Field :=
  [ ⟨"backend", none⟩,
    ⟨"backend__buf_data", some "backend"⟩,
    ⟨"backend__meta_data", some "backend"⟩,
    ⟨"backend__name", some "backend"⟩ ]

/-- The measured domain: four declared fields, nine realizable states — the
same nine cases the Rust certificate and the transported reading carry. -/
theorem the_declared_inventory_has_nine_realizable_states :
    (states memfsFields).length = 9 := by decide

/-- Dropping one declared field is a strictly smaller question, so a proposal
computed over it can never stand in for the full one. -/
theorem dropping_a_declared_field_shrinks_the_question :
    (states (memfsFields.filter (fun f => f.name != "backend__name"))).length = 5 := by decide

theorem the_empty_state_is_always_realizable (fields : List Field) :
    realizable fields [] = true := by simp [realizable]

theorem a_state_naming_an_undeclared_field_is_unrealizable :
    realizable memfsFields ["absent"] = false := by decide

theorem a_child_without_its_parent_is_unrealizable :
    realizable memfsFields ["backend__name"] = false := by decide

theorem the_full_inventory_is_realizable :
    realizable memfsFields (memfsFields.map Field.name) = true := by decide

/-- The inventory cross-check the core performs before a task exists. -/
def inventoryAccepted (declared projected : List String) : Bool := declared == projected

theorem only_the_contracts_own_inventory_is_accepted {declared projected : List String}
    (different : declared ≠ projected) :
    inventoryAccepted declared projected = false := by
  simp [inventoryAccepted, different]

theorem the_contracts_own_inventory_is_accepted (declared : List String) :
    inventoryAccepted declared declared = true := by simp [inventoryAccepted]

/-! ### Both families are inhabited

Neither checker is vacuously refusing, and neither can answer for the other. -/

theorem the_unlink_family_admits_a_reading :
    admits (fun _ => some ⟨Family.listUnlink, 1⟩) ⟨7, Family.listUnlink⟩
      ⟨7, Family.listUnlink⟩ ⟨7, ⟨Family.listUnlink, 1⟩⟩ = true := by decide

theorem the_destructor_family_admits_a_reading :
    admits (fun _ => some ⟨Family.fieldDestructor, 9⟩) ⟨7, Family.fieldDestructor⟩
      ⟨7, Family.fieldDestructor⟩ ⟨7, ⟨Family.fieldDestructor, 9⟩⟩ = true := by decide

theorem a_destructor_reading_never_satisfies_the_unlink_checker :
    admits (fun _ => some ⟨Family.listUnlink, 1⟩) ⟨7, Family.listUnlink⟩
      ⟨7, Family.listUnlink⟩ ⟨7, ⟨Family.fieldDestructor, 9⟩⟩ = false := by decide

theorem an_unlink_reading_never_satisfies_the_destructor_checker :
    admits (fun _ => some ⟨Family.fieldDestructor, 9⟩) ⟨7, Family.fieldDestructor⟩
      ⟨7, Family.fieldDestructor⟩ ⟨7, ⟨Family.listUnlink, 1⟩⟩ = false := by decide

end QCore.TeardownAdmission
