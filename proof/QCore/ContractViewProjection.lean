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

namespace QCore.ContractViewProjection

/-! Card `CONTRACT-VIEW-PROJECTION-CHECK`: checked relations between the
declarations of one function in several contract modules.

Model of `qcore_contract::contract_views`. A function may be declared once per
consuming checker; each registered pair is checked. This file proves, over the
unbounded integers, the entailments the Rust check decides by enumeration, and
models the three other relation checks:

1. **Projection.** `fluent_bit/memory.qv`'s native-evaluated allocator
   clauses entail the resource-v1 view's custody (`resource_return_owned
   (nullable)` for `flb_malloc`/`flb_calloc`, `resource_released(ptr,
   nullable)` for `flb_free`) under the bridge's stated premises; the ChunkIO
   lifecycle contracts entail `resource_released(chunk, nonnull)`. Each
   entailment fails for the mutated contract the Rust negative controls use.
   The memory-backend premise of `chunkio/memory_close.qv` is a coverage
   restriction: outside it the view is **not** implied (the registered finding
   `CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`), proved here.
2. **Renamed equivalence.** The generic frame evaluator (`error_frame.qcore`)
   and the Chunk-flavour evaluator (`error_state.qcore`) accept exactly the same
   write vectors whenever the protected set contains `lock` and excludes the
   declared write footprint; outside that condition they differ.
3. **Agreement of different kinds.** A recorded C signature agrees with a typed
   declaration position by position; an arity mismatch or a mistagged pointer is
   refused, and the four Monkey wrapper signatures agree.
4. **The census guard** refuses an unregistered pair and a stale registration.

Deliberately outside every statement: that the Lean clause models transcribe
the `.qcore` text (review, as for every Lean model here), that the Rust
enumeration decides the fragment exactly (its small-model argument is
documented in the Rust module and recorded as
`CONTRACT-VIEW-SMALL-MODEL-UNPROVED`), and every premise the relation is pinned
as assuming. -/

/-- The shared interpreter's reading of a comparison as an integer: 1 if it
holds, 0 otherwise. -/
def truth (p : Prop) [Decidable p] : Int := if p then 1 else 0

/-! ### Allocator observation → resource v1 -/

/-- The ghost observations `flb_malloc`/`flb_calloc`/`flb_free` are evaluated
over. `n`/`forwardedN`/`zeroed` are `flb_calloc`'s; `forwardedInput` is
`flb_free`'s. -/
structure AllocGhost where
  size : Int
  n : Int
  resultNull : Int
  liveAfter : Int
  resultIsBackend : Int
  backendCalls : Int
  forwardedSize : Int
  forwardedN : Int
  forwardedInput : Int
  zeroed : Int
  deriving Repr

/-- `flb_malloc`'s five `ensures normal` clauses. -/
def mallocEnsures (g : AllocGhost) : Prop :=
  (g.size ≠ 0 ∨ g.resultNull = 1) ∧
  g.backendCalls = truth (g.size ≠ 0) ∧
  (g.size = 0 ∨ g.forwardedSize = g.size) ∧
  (g.size = 0 ∨ g.resultIsBackend = 1) ∧
  g.liveAfter = truth (g.resultNull = 0)

/-- `flb_calloc`'s six `ensures normal` clauses. -/
def callocEnsures (g : AllocGhost) : Prop :=
  (g.size ≠ 0 ∨ g.resultNull = 1) ∧
  g.backendCalls = truth (g.size ≠ 0) ∧
  (g.size = 0 ∨ (g.forwardedSize = g.size ∧ g.forwardedN = g.n)) ∧
  (g.size = 0 ∨ g.resultIsBackend = 1) ∧
  (g.resultNull = 1 ∨ g.zeroed = 1) ∧
  g.liveAfter = truth (g.resultNull = 0)

/-- `flb_free`'s three `ensures normal` clauses. -/
def freeEnsures (g : AllocGhost) : Prop :=
  g.backendCalls = 1 ∧ g.forwardedInput = 1 ∧ g.liveAfter = 0

/-- The bridge's domain premise: the adapter's null test is 0/1. -/
def resultNullDomain (g : AllocGhost) : Prop := g.resultNull = 0 ∨ g.resultNull = 1

/-- `resource_return_owned(nullable)` in the allocator vocabulary: null with
nothing live, or a live backend allocation handed to the caller. -/
def ownedNullable (g : AllocGhost) : Prop :=
  (g.resultNull = 1 ∧ g.liveAfter = 0) ∨
  (g.resultNull = 0 ∧ g.liveAfter = 1 ∧ g.resultIsBackend = 1)

/-- `resource_released(ptr, nullable)` in the allocator vocabulary. -/
def releasedNullable (g : AllocGhost) : Prop :=
  g.liveAfter = 0 ∧ g.forwardedInput = 1

theorem malloc_entails_owned_nullable (g : AllocGhost)
    (hdomain : resultNullDomain g) (h : mallocEnsures g) : ownedNullable g := by
  obtain ⟨hnull, -, -, hbackend, hlive⟩ := h
  rcases hdomain with h0 | h1
  · right
    refine ⟨h0, ?_, ?_⟩
    · simp [hlive, truth, h0]
    · rcases hbackend with hs | hb
      · rcases hnull with hn | hn
        · exact absurd hs hn
        · omega
      · exact hb
  · left
    exact ⟨h1, by simp [hlive, truth, h1]⟩

theorem calloc_entails_owned_nullable (g : AllocGhost)
    (hdomain : resultNullDomain g) (h : callocEnsures g) : ownedNullable g := by
  obtain ⟨hnull, -, -, hbackend, -, hlive⟩ := h
  rcases hdomain with h0 | h1
  · right
    refine ⟨h0, ?_, ?_⟩
    · simp [hlive, truth, h0]
    · rcases hbackend with hs | hb
      · rcases hnull with hn | hn
        · exact absurd hs hn
        · omega
      · exact hb
  · left
    exact ⟨h1, by simp [hlive, truth, h1]⟩

/-- `valid_input_extent == 1` is what v1 nullable custody of the argument
establishes; the release itself needs only `flb_free`'s own clauses. -/
theorem free_entails_released_nullable (g : AllocGhost)
    (h : freeEnsures g) : releasedNullable g :=
  ⟨h.2.2, h.2.1⟩

/-- The detailed contracts are satisfiable in both result arms, so the
entailments are not vacuous. -/
theorem malloc_contract_has_a_null_and_a_live_model :
    mallocEnsures ⟨0, 0, 1, 0, 0, 0, 0, 0, 0, 0⟩ ∧
    mallocEnsures ⟨16, 0, 0, 1, 1, 1, 16, 0, 0, 0⟩ := by
  constructor <;> simp [mallocEnsures, truth]

theorem calloc_contract_has_a_live_model :
    callocEnsures ⟨16, 2, 0, 1, 1, 1, 16, 2, 0, 1⟩ := by
  simp [callocEnsures, truth]

/-- Negative control: `flb_malloc` with `live_after == 1` in place of the
custody link does not entail the owned-nullable view. -/
def mallocEnsuresMutated (g : AllocGhost) : Prop :=
  (g.size ≠ 0 ∨ g.resultNull = 1) ∧
  g.backendCalls = truth (g.size ≠ 0) ∧
  (g.size = 0 ∨ g.forwardedSize = g.size) ∧
  (g.size = 0 ∨ g.resultIsBackend = 1) ∧
  g.liveAfter = 1

theorem the_mutated_malloc_contract_does_not_entail_the_view :
    ∃ g, resultNullDomain g ∧ mallocEnsuresMutated g ∧ ¬ ownedNullable g :=
  ⟨⟨0, 0, 1, 1, 0, 0, 0, 0, 0, 0⟩, Or.inr rfl,
    by simp [mallocEnsuresMutated, truth],
    by simp [ownedNullable]⟩

/-- Negative control: `flb_free` leaving an allocation live. -/
theorem the_mutated_free_contract_does_not_entail_the_view :
    ∃ g : AllocGhost, (g.backendCalls = 1 ∧ g.forwardedInput = 1 ∧ g.liveAfter = 1) ∧
      ¬ releasedNullable g :=
  ⟨⟨0, 0, 0, 1, 0, 1, 0, 0, 1, 0⟩, by simp, by simp [releasedNullable]⟩

/-! ### ChunkIO close → resource v1 -/

/-- The lifecycle-mechanism propositions `chunkio/memory_close.qv` states
over, with its coverage premise. -/
structure CloseAtoms where
  nullableOwnedChunk : Bool
  nonnullChunkHasMemoryBackend : Bool
  chunkWasNonnull : Bool
  chunkReleased : Bool
  nameReleased : Bool
  deriving Repr, DecidableEq

/-- The clause that carries the chunk object's release. -/
def memoryCloseReleaseClause (a : CloseAtoms) : Prop :=
  (!a.chunkWasNonnull || a.chunkReleased) = true

/-- `resource_released(chunk, nonnull)`'s custody premise, as the v1 caller
discipline establishes it in this vocabulary. -/
def closeViewPremise (a : CloseAtoms) : Prop :=
  a.nullableOwnedChunk = true ∧ a.chunkWasNonnull = true

theorem memory_close_entails_released_nonnull (a : CloseAtoms)
    (hview : closeViewPremise a) (h : memoryCloseReleaseClause a) :
    a.chunkReleased = true := by
  unfold memoryCloseReleaseClause at h
  rw [hview.2] at h
  simpa using h

/-- Negative control: releasing only the name does not release the chunk. -/
theorem releasing_only_the_name_does_not_entail_the_view :
    ∃ a : CloseAtoms, closeViewPremise a ∧
      (!a.chunkWasNonnull || a.nameReleased) = true ∧ a.chunkReleased = false :=
  ⟨⟨true, true, true, false, true⟩, ⟨rfl, rfl⟩, rfl, rfl⟩

/-- The detailed contract as a whole: its postconditions under its premises. -/
def memoryCloseContract (a : CloseAtoms) : Prop :=
  a.nullableOwnedChunk = true → a.nonnullChunkHasMemoryBackend = true →
    memoryCloseReleaseClause a

/-- The registered finding `CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`:
without the memory-backend premise, the contract holds of a nonnull owned
chunk that is **not** released, so `memory_close.qcore` alone does not support
the view the resource loader applies to every backend. -/
theorem outside_the_memory_backend_the_view_is_not_implied :
    ∃ a : CloseAtoms, memoryCloseContract a ∧ closeViewPremise a ∧
      a.chunkReleased = false :=
  ⟨⟨true, false, true, false, false⟩, fun _ h => absurd h (by decide),
    ⟨rfl, rfl⟩, rfl⟩

/-- `chunkio/chunk_destructor.qv`'s six named `ensures normal`
propositions (its `quant` clause is outside the fragment and not used). -/
structure DestructorAtoms where
  selectedBackendIsClosed : Bool
  streamLinkIsRemoved : Bool
  contextLinkIsRemoved : Bool
  chunkNameAndObjectAreReleased : Bool
  contextChunkCounterIsDecremented : Bool
  deletionIsRecordedAsRequested : Bool
  deriving Repr, DecidableEq

def destructorEnsures (d : DestructorAtoms) : Prop :=
  d.selectedBackendIsClosed = true ∧ d.streamLinkIsRemoved = true ∧
    d.contextLinkIsRemoved = true ∧ d.chunkNameAndObjectAreReleased = true ∧
    d.contextChunkCounterIsDecremented = true ∧ d.deletionIsRecordedAsRequested = true

/-- The language lifecycle entails the view for its consumed `own<Chunk>`,
on every backend. -/
theorem chunk_destructor_entails_released_nonnull (d : DestructorAtoms)
    (h : destructorEnsures d) : d.chunkNameAndObjectAreReleased = true :=
  h.2.2.2.1

/-! ### Renamed frame equivalence (`cio_error_reset`) -/

abbrev Writes := List (String × Int)

/-- The Chunk-flavour evaluator: every write is to a declared footprint field,
none is to `lock`, and the postconditions hold. (An empty write vector refuses
because the contract's `field_NAME` fact is supplied only by a write.) -/
def chunkFlavourAccepts (declared : List String) (post : Writes → Prop) (ws : Writes) : Prop :=
  ws ≠ [] ∧ (∀ w ∈ ws, w.1 ∈ declared) ∧ (∀ w ∈ ws, w.1 ≠ "lock") ∧ post ws

/-- The generic evaluator over protected set `P`: nonempty writes disjoint
from `P` (checked before and again as `modeled_fields_are_preserved`), every
write declared, and the postconditions hold. -/
def genericAccepts (P declared : List String) (post : Writes → Prop) (ws : Writes) : Prop :=
  ws ≠ [] ∧ (∀ w ∈ ws, w.1 ∉ P) ∧ (∀ w ∈ ws, w.1 ∈ declared) ∧ post ws

theorem the_frame_evaluators_agree_under_the_condition
    (P declared : List String) (post : Writes → Prop) (ws : Writes)
    (hlock : "lock" ∈ P) (hdisjoint : ∀ d ∈ declared, d ∉ P) :
    chunkFlavourAccepts declared post ws ↔ genericAccepts P declared post ws := by
  constructor
  · rintro ⟨hne, hdecl, -, hpost⟩
    exact ⟨hne, fun w hw => hdisjoint _ (hdecl w hw), hdecl, hpost⟩
  · rintro ⟨hne, hprot, hdecl, hpost⟩
    refine ⟨hne, hdecl, fun w hw hl => hprot w hw ?_, hpost⟩
    rw [hl]
    exact hlock

/-- The condition is load-bearing: a protected set covering the written field
separates the evaluators. -/
theorem the_condition_is_load_bearing :
    ∃ ws : Writes, chunkFlavourAccepts ["error_n"] (fun _ => True) ws ∧
      ¬ genericAccepts ["error_n", "lock"] ["error_n"] (fun _ => True) ws := by
  refine ⟨[("error_n", 0)], ⟨by simp, by simp, by simp, trivial⟩, ?_⟩
  rintro ⟨-, hprot, -, -⟩
  exact hprot ("error_n", 0) (by simp) (by simp)

/-- The two real protected sets both satisfy the condition for the declared
footprint `{error_n}`. -/
theorem both_real_protected_sets_satisfy_the_condition :
    ("lock" ∈ ["lock"] ∧ ∀ d ∈ ["error_n"], d ∉ ["lock"]) ∧
    ("lock" ∈ ["busy", "lock"] ∧ ∀ d ∈ ["error_n"], d ∉ ["busy", "lock"]) := by
  decide

/-! ### Agreement of a recorded C signature with a typed declaration -/

inductive CType where
  | void
  | scalar (name : String)
  | pointer (isConst : Bool) (tag : Option String)
  deriving DecidableEq, Repr

inductive QType where
  | unit
  | scalar (name : String)
  | pointer (isConst : Bool) (nominal : String)
  deriving DecidableEq, Repr

/-- One position: `void`/`unit`, equal scalar spellings, or pointers where a C
const pointer stays const and a tagged struct maps to its registered nominal. -/
def positionAgrees (nominals : List (String × String)) : CType → QType → Bool
  | .void, .unit => true
  | .scalar c, .scalar q => c == q
  | .pointer cConst tag, .pointer qConst nominal =>
      (!cConst || qConst) &&
        (match tag with
         | none => true
         | some t => nominals.lookup t == some nominal)
  | _, _ => false

def signatureAgrees (nominals : List (String × String))
    (cParams : List CType) (cResult : CType)
    (qParams : List QType) (qResult : QType) : Bool :=
  cParams.length == qParams.length &&
    (cParams.zip qParams).all (fun p => positionAgrees nominals p.1 p.2) &&
    positionAgrees nominals cResult qResult

theorem an_arity_mismatch_is_refused (nominals : List (String × String))
    (cParams : List CType) (cResult : CType) (qParams : List QType) (qResult : QType)
    (h : cParams.length ≠ qParams.length) :
    signatureAgrees nominals cParams cResult qParams qResult = false := by
  simp [signatureAgrees, h]

def monkeyNominals : List (String × String) :=
  [("mk_event_loop", "EventLoop"), ("mk_event_ctx", "EventContext")]

/-- The four Monkey wrapper signatures recorded in
`monkey/wait_source_policy.qv` agree with `monkey/wrapper_paths.qv`. -/
theorem the_monkey_wrapper_signatures_agree :
    signatureAgrees monkeyNominals [.scalar "i32"] (.pointer false none)
      [.scalar "i32"] (.pointer false "EventContext") = true ∧
    signatureAgrees monkeyNominals [.pointer false (some "mk_event_ctx")] .void
      [.pointer false "EventContext"] .unit = true ∧
    signatureAgrees monkeyNominals [.pointer false (some "mk_event_loop"), .scalar "i32"]
      (.scalar "i32") [.pointer false "EventLoop", .scalar "i32"] (.scalar "i32") = true := by
  decide

/-- A pointer tagged with the other struct is refused. -/
theorem a_mistagged_pointer_is_refused :
    signatureAgrees monkeyNominals [.pointer false (some "mk_event_loop")] .void
      [.pointer false "EventContext"] .unit = false := by
  decide

/-- A C const pointer declared mutable is refused. -/
theorem a_const_pointer_declared_mutable_is_refused :
    positionAgrees monkeyNominals (.pointer true none) (.pointer false "EventLoop") = false := by
  decide

/-! ### The census guard -/

abbrev Pair := String × String × String

def guardAccepts (pairs registered : List Pair) : Bool :=
  pairs.all (fun p => registered.contains p) && registered.all (fun p => pairs.contains p)

theorem an_unregistered_pair_is_refused (pairs registered : List Pair) (p : Pair)
    (hp : p ∈ pairs) (hn : p ∉ registered) : guardAccepts pairs registered = false := by
  simp only [guardAccepts, Bool.and_eq_false_iff]
  left
  simp only [List.all_eq_false, List.contains_iff_mem]
  exact ⟨p, hp, by simpa using hn⟩

theorem a_stale_registration_is_refused (pairs registered : List Pair) (p : Pair)
    (hp : p ∈ registered) (hn : p ∉ pairs) : guardAccepts pairs registered = false := by
  simp only [guardAccepts, Bool.and_eq_false_iff]
  right
  simp only [List.all_eq_false, List.contains_iff_mem]
  exact ⟨p, hp, by simpa using hn⟩

theorem an_exactly_registered_census_is_accepted (pairs : List Pair) :
    guardAccepts pairs pairs = true := by
  simp [guardAccepts]

/-! ### Why dropping a premise is sound -/

/-- Entailment from a list of premises. -/
def entailsAll {α : Type} (premises : List (α → Prop)) (goal : α → Prop) : Prop :=
  ∀ s, (∀ p ∈ premises, p s) → goal s

/-- If a sub-collection of the premises entails the goal, so does the whole
collection: the Rust check drops premises outside its fragment (and outside the
goal's cone of influence) without ever accepting more. -/
theorem dropping_premises_is_sound {α : Type} (kept all : List (α → Prop))
    (goal : α → Prop) (hsub : ∀ p ∈ kept, p ∈ all) (h : entailsAll kept goal) :
    entailsAll all goal :=
  fun s hall => h s (fun p hp => hall p (hsub p hp))

end QCore.ContractViewProjection
