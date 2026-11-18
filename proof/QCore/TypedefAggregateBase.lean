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

import QCore.AggregateStore

/-!
# A local spelled through a struct alias is the struct pointer (card AG1-INDIRECT-MEMBER-ACCESS, gateway rule 61 (1))

The abstract rule implemented by
`crates/qcore-c-gateway/src/typedef_aggregate_base.rs` (with its hooks in
`typed_field_access.rs` and `aggregate_store.rs`).

The aggregate store of card O04-B08 (`QCore.AggregateStore`) takes, as a
supplied fact, that the base's declared type is exactly `struct TAG *`
(`baseIsRecordPointer`). Its reader supplied that fact only for a keyword
spelling. Rule 61 (1) supplies it also for a local declared `A * NAME` where
every `typedef` declaration of `A` in the unit names one struct `TAG` (a
forward `typedef struct TAG A;`, the defining `typedef struct TAG { … } A;`, or
both). C11 6.7.8p3 makes a typedef name a synonym for its type, so the fact is
then true, and every other premise of the store is decided exactly as for the
keyword spelling.

Proved here:

* **The alias table is sound.** `aliasHead` answers `some t` only when every
  typedef declaration of the name names `struct t`
  (`an_agreeing_alias_names_its_struct_everywhere`), and a name two
  declarations disagree on, or one declaration gives another type, answers
  `none` (`a_disagreeing_alias_names_nothing`,
  `an_other_declaration_names_nothing`).
* **The reading is the declared type.** A local admitted by the decision has
  C type `ptr (structT t)` under the typedef environment
  (`an_admitted_local_is_the_struct_pointer`), the same as its keyword
  spelling; so `QCore.AggregateStore.admitsStore` decides alike over either
  spelling (`the_store_decides_alike_over_either_spelling`).
* **Every fact refuses on its own** (`a_missing_fact_never_admits`), the
  owned binding included (a record this body allocated; an accessor's
  borrowed child is left to the older routes), and the decision is exactly
  the shared vectors (`decision_vectors_hold`).
* **A struct type is never variably modified** (C11 6.7.6.2p2: only an array
  type with a non-constant size, or a type derived from one, is), so a
  `sizeof` operand of such a pointee is unevaluated
  (`a_struct_alias_is_never_variably_modified`).

## What this is not

A model of the lexical reader or of Clang. That the Rust's token tests extract
the declarations faithfully is trust boundary 28; the shared vectors bind the
decision table.
-/

namespace QCore.TypedefAggregateBase

/-! ## Types and typedef declarations -/

/-- The C types this rule distinguishes. -/
inductive CTy where
  | structT (tag : String)
  | ptr (inner : CTy)
  | other
  deriving DecidableEq, Repr

/-- What one `typedef` declaration of a name says it denotes: exactly
    `struct TAG` (forward or defining), or anything else. -/
inductive Target where
  | structTag (tag : String)
  | otherType
  deriving DecidableEq, Repr

/-- One `typedef` declaration: the declared name and its target. -/
structure Decl where
  name : String
  target : Target
  deriving DecidableEq, Repr

/-- The head one declaration contributes: its struct tag, or nothing. -/
def head : Target → Option String
  | .structTag t => some t
  | .otherType => none

/-- The merged table entry, as `note_struct_alias_head` builds it: the first
    declaration sets it, a later one that disagrees clears it for good. `none`
    in the outer option means "not yet seen". -/
def merge (entry : Option (Option String)) (h : Option String) : Option (Option String) :=
  match entry with
  | none => some h
  | some e => if e = h then some e else some none

/-- The table entry of `a` after reading every declaration in order. -/
def entryOf (a : String) : List Decl → Option (Option String)
  | [] => none
  | d :: rest =>
    let e := entryOf a rest
    if d.name = a then merge e (head d.target) else e

/-- The struct tag the alias names, if every declaration agrees on one. -/
def aliasHead (env : List Decl) (a : String) : Option String :=
  match entryOf a env with
  | some (some t) => some t
  | _ => none

/-- The C meaning of a typedef name under the environment (C11 6.7.8p3). -/
def meaning (env : List Decl) (a : String) : Option CTy :=
  (aliasHead env a).map CTy.structT

theorem merge_ne_none (e : Option (Option String)) (h : Option String) : merge e h ≠ none := by
  cases e with
  | none => simp [merge]
  | some e => by_cases hc : e = h <;> simp [merge, hc]

theorem merge_some_some {e : Option (Option String)} {h : Option String} {t : String}
    (hm : merge e h = some (some t)) : h = some t ∧ (e = none ∨ e = some (some t)) := by
  cases e with
  | none =>
    simp [merge] at hm
    exact ⟨hm, Or.inl rfl⟩
  | some e =>
    by_cases hc : e = h
    · subst hc
      simp [merge] at hm
      subst hm
      exact ⟨rfl, Or.inr rfl⟩
    · simp [merge, hc] at hm

/-- A name no declaration names has no entry. -/
theorem entry_none_of_absent (a : String) :
    ∀ l : List Decl, entryOf a l = none → ∀ y ∈ l, y.name ≠ a := by
  intro l
  induction l with
  | nil => intro _ y hy; simp at hy
  | cons z zs ih =>
    intro h y hy hya
    by_cases hz : z.name = a
    · simp only [entryOf, hz, ↓reduceIte] at h
      exact merge_ne_none _ _ h
    · simp only [entryOf, hz, ↓reduceIte] at h
      rcases List.mem_cons.mp hy with hyz | hy'
      · subst hyz; exact hz hya
      · exact ih h y hy' hya

theorem entry_sound (a t : String) :
    ∀ env : List Decl, entryOf a env = some (some t) →
      ∀ d ∈ env, d.name = a → d.target = .structTag t := by
  intro env
  induction env with
  | nil => intro h; simp [entryOf] at h
  | cons d rest ih =>
    intro h d' hd' hname
    by_cases hd : d.name = a
    · simp only [entryOf, hd, ↓reduceIte] at h
      obtain ⟨hh, hrest⟩ := merge_some_some h
      rcases List.mem_cons.mp hd' with hdd | hin
      · subst hdd
        cases ht : d'.target with
        | structTag u => rw [ht] at hh; simp [head] at hh; rw [hh]
        | otherType => rw [ht] at hh; simp [head] at hh
      · rcases hrest with hnone | hsome
        · exact absurd hname (entry_none_of_absent a rest hnone d' hin)
        · exact ih hsome d' hin hname
    · simp only [entryOf, hd, ↓reduceIte] at h
      rcases List.mem_cons.mp hd' with hdd | hin
      · subst hdd; exact absurd hname hd
      · exact ih h d' hin hname

/-- **The alias table is sound**: an answer `t` means every typedef declaration
    of the name names `struct t`. -/
theorem an_agreeing_alias_names_its_struct_everywhere (env : List Decl) (a t : String)
    (h : aliasHead env a = some t) : ∀ d ∈ env, d.name = a → d.target = .structTag t := by
  unfold aliasHead at h
  split at h
  · rename_i t' heq
    simp at h
    subst h
    exact entry_sound a t' env heq
  · simp at h

/-- Two declarations of one name naming different structs answer nothing. -/
theorem a_disagreeing_alias_names_nothing (env : List Decl) (a t u : String) (htu : t ≠ u)
    (ht : ⟨a, .structTag t⟩ ∈ env) (hu : ⟨a, .structTag u⟩ ∈ env) : aliasHead env a = none := by
  cases h : aliasHead env a with
  | none => rfl
  | some v =>
    have s := an_agreeing_alias_names_its_struct_everywhere env a v h
    have h1 := s _ ht rfl
    have h2 := s _ hu rfl
    simp at h1 h2
    exact absurd (h1.trans h2.symm) htu

/-- A declaration giving the name any other type answers nothing. -/
theorem an_other_declaration_names_nothing (env : List Decl) (a : String)
    (h : ⟨a, .otherType⟩ ∈ env) : aliasHead env a = none := by
  cases hh : aliasHead env a with
  | none => rfl
  | some v =>
    have := an_agreeing_alias_names_its_struct_everywhere env a v hh _ h rfl
    simp at this

/-! ## The decision -/

/-- The facts rule 61 (1) decides one local on (`AliasLocalFacts`). -/
structure Facts where
  aliasAgrees : Bool
  recordComplete : Bool
  exactSpelling : Bool
  unique : Bool
  isLocal : Bool
  sharedOpaque : Bool
  /-- The name's one binding is a direct call whose contract in force returns
      an owned result: a record this body allocated, never a borrowed or
      derived one (an accessor's child keeps the older routes). -/
  ownedBinding : Bool
  deriving DecidableEq, Repr

/-- `typedef_aggregate_base::admits`. -/
def admits (f : Facts) : Bool :=
  f.aliasAgrees && f.recordComplete && f.exactSpelling && f.unique && f.isLocal && f.sharedOpaque
    && f.ownedBinding

/-- Every fact refuses on its own. -/
theorem a_missing_fact_never_admits (f : Facts)
    (h : f.aliasAgrees = false ∨ f.recordComplete = false ∨ f.exactSpelling = false ∨
      f.unique = false ∨ f.isLocal = false ∨ f.sharedOpaque = false ∨
      f.ownedBinding = false) :
    admits f = false := by
  rcases h with h | h | h | h | h | h | h <;> simp [admits, h]

/-- The shared vectors (`decision_vectors`): bit `i` of the index is the
    `i`-th fact, admitted exactly at 127. -/
def factsOf (bits : Nat) : Facts :=
  { aliasAgrees := bits % 2 = 1
    recordComplete := bits / 2 % 2 = 1
    exactSpelling := bits / 4 % 2 = 1
    unique := bits / 8 % 2 = 1
    isLocal := bits / 16 % 2 = 1
    sharedOpaque := bits / 32 % 2 = 1
    ownedBinding := bits / 64 % 2 = 1 }

theorem decision_vectors_hold :
    ∀ bits, bits < 128 → admits (factsOf bits) = (bits == 127) := by
  decide

/-- The declared type of a local written `A * NAME`. -/
def localType (env : List Decl) (a : String) : Option CTy :=
  (meaning env a).map CTy.ptr

/-- **The reading is the declared type**: a local the decision admits (its
    alias names `t`) is a `struct t *`, exactly its keyword spelling. -/
theorem an_admitted_local_is_the_struct_pointer (env : List Decl) (a t : String)
    (h : aliasHead env a = some t) : localType env a = some (CTy.ptr (CTy.structT t)) := by
  simp [localType, meaning, h]

/-- What the aggregate store is told about the base: `baseIsRecordPointer` holds
    exactly when the base's type is `ptr (structT _)`. -/
def isRecordPointer : Option CTy → Bool
  | some (CTy.ptr (CTy.structT _)) => true
  | _ => false

/-- **Spelling invariance**: with the base's fact supplied from its C type, the
    store is decided alike for the alias spelling and the keyword spelling. -/
theorem the_store_decides_alike_over_either_spelling (env : List Decl) (a t : String)
    (h : aliasHead env a = some t) (s : QCore.AggregateStore.Store) :
    QCore.AggregateStore.admitsStore { s with baseIsRecordPointer := isRecordPointer (localType env a) }
      = QCore.AggregateStore.admitsStore
          { s with baseIsRecordPointer := isRecordPointer (some (CTy.ptr (CTy.structT t))) } := by
  rw [an_admitted_local_is_the_struct_pointer env a t h]

/-- A refused alias supplies no record-pointer fact, so the store keeps its
    refusal (`QCore.AggregateStore.a_non_record_pointer_base_never_admits_a_store`). -/
theorem a_refused_alias_keeps_the_refusal (env : List Decl) (a : String)
    (h : aliasHead env a = none) (s : QCore.AggregateStore.Store) :
    QCore.AggregateStore.admitsStore { s with baseIsRecordPointer := isRecordPointer (localType env a) }
      = false := by
  apply QCore.AggregateStore.a_non_record_pointer_base_never_admits_a_store
  simp [localType, meaning, h, isRecordPointer]

/-! ## `sizeof` operands -/

/-- Variably modified types (C11 6.7.6.2p2), over this rule's types: none of
    them is an array, so only a pointer to one could be; a struct never is. -/
def variablyModified : CTy → Bool
  | .structT _ => false
  | .ptr inner => variablyModified inner
  | .other => true

/-- A pointee named by an agreeing struct alias is never variably modified, so
    `sizeof (*p)` does not evaluate `*p` (C11 6.5.3.4p2). -/
theorem a_struct_alias_is_never_variably_modified (env : List Decl) (a t : String)
    (h : aliasHead env a = some t) : (meaning env a).map variablyModified = some false := by
  simp [meaning, h, variablyModified]

end QCore.TypedefAggregateBase
