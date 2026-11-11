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

import Std

namespace QCore.ContractInherit

/-! Rule model for contract inclusion and inheritance (card CONTRACT-INHERIT).

`qcore_contract::contract_inclusion` resolves `#include "crate:module";`
against a published load path and elaborates `contract inherit [NS::]NAME;`
into the parent's clauses under a positional renaming;
`qcore_contract::contract_loader::load_published_resource_contracts` then
requires the resource-v1 reading of every inherited declaration to equal its
parent's. This module models:

* the load path: a crate includes only from itself or a declared dependency,
  and only published modules; a crate without dependencies includes nothing
  foreign; duplicate crate names never resolve by order;
* namespaces: a namespace is derived from the crate name (one of exactly
  two admitted prefixes removed — `quirin-provider-`, and since card
  CONTRACT-INHERIT-QCORE-PROVIDER-NAMING `qcore-provider-` — `-` read as
  `_`), contains no `-`, derives equally under either prefix, and — under the
  load-path validity check — names exactly one crate, so one remainder under
  both prefixes is refused;
* include and inheritance cycles: a depth-first walk with a visiting stack
  refuses every cycle reachable from its start, and fuel exhaustion is itself
  a refusal, so the walk is total-or-refuse by construction;
* parent lookup: an unqualified parent resolves only among own declarations,
  a qualified one only among the directly included modules of the named
  crate, and more than one candidate is refused, never resolved by order;
* positional substitution: the elaborated clauses are exactly the parent's
  clauses under the renaming, and under the capture side condition every
  clause means the same thing to a reading that resolves names by parameter
  lookup, so the inherited contract is not weaker than its parent;
* why the consumer gate is needed: a reading that treats an argument
  position structurally can diverge under the same renaming, and the gate
  admits only an equal reading;
* (card CONTRACT-INHERIT-ADOPTION) the load path built from Cargo metadata:
  an include follows a declared Cargo edge to a load-path crate or stays in
  its own crate, a changed published parent is refused by its pin, and a
  schema-2 resource manifest's path entries load exactly as schema 1.

Words and names are modelled as natural numbers (interned identifiers).
Correspondence with the Rust implementation is maintained by review; it is
not an extraction theorem (constraint 28). -/

abbrev Name := Nat
abbrev Word := Nat

inductive Refusal where
  | malformed
  | extension
  | unreachableCrate
  | unpublished
  | includeCycle
  | unknownParent
  | ambiguousParent
  | signature
  | inheritCycle
  | capture
  | invalidLoadPath
  | notInheritable
  | namespaceInvalid
  | qualifiedRequired
  | unknownNamespace
  | sameName
  | consumerDisagrees
  | exhausted
  deriving DecidableEq, Repr

/-! ## 1. Load path and publication -/

structure Crate where
  name : Name
  ns : Name
  deps : List Name
  published : List Name
  deriving DecidableEq, Repr

/-- The unique crate with this name; none or several is a refusal. -/
def findCrate (crates : List Crate) (name : Name) : Except Refusal Crate :=
  match crates.filter (fun c => c.name == name) with
  | [c] => .ok c
  | [] => .error .unreachableCrate
  | _ :: _ :: _ => .error .invalidLoadPath

/-- Resolve `#include "crate:module"` written in a module of `fromCrate`. -/
def resolveInclude (crates : List Crate) (fromCrate : Name) (target : Name × Name) :
    Except Refusal (Name × Name) :=
  match findCrate crates fromCrate with
  | .error e => .error e
  | .ok src =>
    if target.1 = fromCrate ∨ target.1 ∈ src.deps then
      match findCrate crates target.1 with
      | .error e => .error e
      | .ok tgt => if target.2 ∈ tgt.published then .ok target else .error .unpublished
    else .error .unreachableCrate

theorem findCrate_sound {crates : List Crate} {name : Name} {c : Crate}
    (h : findCrate crates name = .ok c) :
    c ∈ crates ∧ c.name = name ∧ crates.filter (fun c => c.name == name) = [c] := by
  unfold findCrate at h
  split at h
  · rename_i c' hfilter
    cases h
    have hmem : c ∈ crates.filter (fun c => c.name == name) := by
      rw [hfilter]; exact List.mem_singleton_self c
    rw [List.mem_filter] at hmem
    exact ⟨hmem.1, by simpa using hmem.2, hfilter⟩
  · cases h
  · cases h

/-- Two crates with one name never resolve by order. -/
theorem findCrate_duplicate_refused {crates : List Crate} {name : Name}
    {a b : Crate} {rest : List Crate}
    (h : crates.filter (fun c => c.name == name) = a :: b :: rest) :
    findCrate crates name = .error .invalidLoadPath := by
  unfold findCrate; rw [h]

theorem resolveInclude_sound {crates : List Crate} {fromCrate : Name} {target r : Name × Name}
    (h : resolveInclude crates fromCrate target = .ok r) :
    r = target ∧
    (∃ src, findCrate crates fromCrate = .ok src ∧ (target.1 = fromCrate ∨ target.1 ∈ src.deps)) ∧
    (∃ tgt, findCrate crates target.1 = .ok tgt ∧ target.2 ∈ tgt.published) := by
  unfold resolveInclude at h
  split at h
  · cases h
  · rename_i src hsrc
    split at h
    · rename_i hreach
      split at h
      · cases h
      · rename_i tgt htgt
        split at h
        · rename_i hpub
          cases h
          exact ⟨rfl, ⟨src, hsrc, hreach⟩, ⟨tgt, htgt, hpub⟩⟩
        · cases h
    · cases h

/-- A crate that declares no dependency includes nothing from another crate
(`quirin-provider-libc` today). -/
theorem no_dependency_includes_nothing_foreign {crates : List Crate} {fromCrate : Name}
    {src : Crate} {target : Name × Name}
    (hsrc : findCrate crates fromCrate = .ok src) (hdeps : src.deps = [])
    (hother : target.1 ≠ fromCrate) :
    resolveInclude crates fromCrate target = .error .unreachableCrate := by
  unfold resolveInclude
  rw [hsrc]
  simp only
  have : ¬ (target.1 = fromCrate ∨ target.1 ∈ src.deps) := by
    rw [hdeps]; simp [hother]
  simp [this]

/-- An unpublished module is refused even from the crate's own modules. -/
theorem unpublished_is_refused {crates : List Crate} {fromCrate : Name} {src tgt : Crate}
    {target : Name × Name}
    (hsrc : findCrate crates fromCrate = .ok src)
    (hreach : target.1 = fromCrate ∨ target.1 ∈ src.deps)
    (htgt : findCrate crates target.1 = .ok tgt) (hpub : target.2 ∉ tgt.published) :
    resolveInclude crates fromCrate target = .error .unpublished := by
  unfold resolveInclude
  simp [hsrc, hreach, htgt, hpub]

/-! ## 2. Namespaces

Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING (the user's option B): exactly two
package-name prefixes are admitted, `quirin-provider-` (the data-only contract
crates) and `qcore-provider-` (the executable provider crates). A namespace is
always the package name without its prefix, `-` read as `_`; there is no other
prefix and no freely recorded namespace. The prefixes are spelled as explicit
character lists so that the kernel can evaluate them. -/

/-- The data-only contract crates' prefix, `quirin-provider-`. -/
def cratePrefix : List Char :=
  ['q', 'u', 'i', 'r', 'i', 'n', '-', 'p', 'r', 'o', 'v', 'i', 'd', 'e', 'r', '-']

/-- The executable provider crates' prefix, `qcore-provider-`. -/
def executablePrefix : List Char :=
  ['q', 'c', 'o', 'r', 'e', '-', 'p', 'r', 'o', 'v', 'i', 'd', 'e', 'r', '-']

/-- Exactly the admitted prefixes, in the order the derivation tries them. -/
def cratePrefixes : List (List Char) := [cratePrefix, executablePrefix]

def dashToUnderscore (c : Char) : Char := if c = '-' then '_' else c

/-- The remainder after the first listed prefix the name carries. -/
def stripPrefixes : List (List Char) → List Char → Option (List Char)
  | [], _ => none
  | p :: ps, name => if p.isPrefixOf name = true then some (name.drop p.length)
    else stripPrefixes ps name

/-- `quirin-provider-fluent-bit` ↦ `fluent_bit`, `qcore-provider-librdkafka` ↦
`librdkafka`; `none` without an admitted prefix or with an empty remainder.
(The Rust derivation additionally requires an identifier shape; that
refinement only refuses more.) -/
def deriveNamespace (name : List Char) : Option (List Char) :=
  match stripPrefixes cratePrefixes name with
  | none => none
  | some rest =>
    match rest.map dashToUnderscore with
    | [] => none
    | c :: cs => some (c :: cs)

theorem stripPrefixes_sound :
    ∀ {ps : List (List Char)} {name rest : List Char},
      stripPrefixes ps name = some rest →
        ∃ p ∈ ps, p.isPrefixOf name = true ∧ rest = name.drop p.length
  | [], _, _, h => by cases h
  | p :: ps, name, rest, h => by
    unfold stripPrefixes at h
    split at h
    · rename_i hp
      cases h
      exact ⟨p, List.mem_cons_self .., hp, rfl⟩
    · obtain ⟨q, hq, hpre, hrest⟩ := stripPrefixes_sound h
      exact ⟨q, List.mem_cons_of_mem _ hq, hpre, hrest⟩

/-- A derived namespace comes from a name that carries one of exactly the two
admitted prefixes. -/
theorem deriveNamespace_requires_prefix {name ns : List Char}
    (h : deriveNamespace name = some ns) :
    ∃ p ∈ cratePrefixes, p.isPrefixOf name = true := by
  unfold deriveNamespace at h
  split at h
  · cases h
  · rename_i rest hstrip
    obtain ⟨p, hp, hpre, _⟩ := stripPrefixes_sound hstrip
    exact ⟨p, hp, hpre⟩

theorem dashToUnderscore_ne_dash (c : Char) : dashToUnderscore c ≠ '-' := by
  unfold dashToUnderscore
  split
  · decide
  · assumption

/-- A derived namespace never contains `-`, so it is an identifier-shaped
qualifier rather than a package name. -/
theorem deriveNamespace_has_no_dash {name ns : List Char}
    (h : deriveNamespace name = some ns) : '-' ∉ ns := by
  unfold deriveNamespace at h
  split at h
  · cases h
  · split at h
    · cases h
    · rename_i c cs hmap
      cases h
      intro hmem
      rw [← hmap] at hmem
      rw [List.mem_map] at hmem
      obtain ⟨x, _, hx⟩ := hmem
      exact dashToUnderscore_ne_dash x hx

theorem isPrefixOf_append_self : ∀ (p r : List Char), p.isPrefixOf (p ++ r) = true
  | [], _ => rfl
  | a :: as, r => by
    simp [isPrefixOf_append_self as r]

/-- The two prefixes exclude each other: no name carries both, so which prefix
a name carries is never a matter of order. -/
theorem prefixes_exclusive (r : List Char) :
    cratePrefix.isPrefixOf (executablePrefix ++ r) = false ∧
      executablePrefix.isPrefixOf (cratePrefix ++ r) = false := by
  constructor <;> rfl

/-- A `quirin-provider-` name derives its remainder, `-` read as `_`. -/
theorem deriveNamespace_data {r : List Char} (hr : r ≠ []) :
    deriveNamespace (cratePrefix ++ r) = some (r.map dashToUnderscore) := by
  have hstrip : stripPrefixes cratePrefixes (cratePrefix ++ r) = some r := by
    simp [cratePrefixes, stripPrefixes, isPrefixOf_append_self]
  unfold deriveNamespace
  rw [hstrip]
  cases r with
  | nil => exact absurd rfl hr
  | cons c cs => rfl

/-- A `qcore-provider-` name derives its remainder the same way (option B):
`qcore-provider-librdkafka` ↦ `librdkafka`. -/
theorem deriveNamespace_executable {r : List Char} (hr : r ≠ []) :
    deriveNamespace (executablePrefix ++ r) = some (r.map dashToUnderscore) := by
  have hstrip : stripPrefixes cratePrefixes (executablePrefix ++ r) = some r := by
    simp [cratePrefixes, stripPrefixes, (prefixes_exclusive r).1, isPrefixOf_append_self]
  unfold deriveNamespace
  rw [hstrip]
  cases r with
  | nil => exact absurd rfl hr
  | cons c cs => rfl

/-- One remainder under both prefixes derives one namespace
(`quirin-provider-libc` and `qcore-provider-libc` both derive `libc`); the
load-path check below therefore refuses such a pair. -/
theorem same_remainder_same_namespace (r : List Char) :
    deriveNamespace (cratePrefix ++ r) = deriveNamespace (executablePrefix ++ r) := by
  cases r with
  | nil => rfl
  | cons c cs =>
    rw [deriveNamespace_data (List.cons_ne_nil c cs),
      deriveNamespace_executable (List.cons_ne_nil c cs)]

/-- A name with neither admitted prefix derives nothing. -/
theorem deriveNamespace_other_prefix_none {name : List Char}
    (h1 : cratePrefix.isPrefixOf name = false) (h2 : executablePrefix.isPrefixOf name = false) :
    deriveNamespace name = none := by
  unfold deriveNamespace
  simp [cratePrefixes, stripPrefixes, h1, h2]

/-- The explicit spellings are the package-name prefixes as strings. -/
theorem cratePrefix_spelling : cratePrefix = "quirin-provider-".toList := by decide

theorem executablePrefix_spelling : executablePrefix = "qcore-provider-".toList := by decide

/-- Evaluated on every crate on the real load path (card
CONTRACT-INHERIT-QCORE-PROVIDER-NAMING): the eleven package names derive the
eleven recorded namespaces, the Monkey contracts' owner is `libevent`, a name
with another prefix derives nothing, and `libc` under both prefixes collides. -/
theorem real_namespaces :
    deriveNamespace "quirin-provider-libc".toList = some "libc".toList ∧
    deriveNamespace "quirin-provider-cfl".toList = some "cfl".toList ∧
    deriveNamespace "quirin-provider-mpack".toList = some "mpack".toList ∧
    deriveNamespace "quirin-provider-msgpack".toList = some "msgpack".toList ∧
    deriveNamespace "quirin-provider-fluent-bit".toList = some "fluent_bit".toList ∧
    deriveNamespace "quirin-provider-shared".toList = some "shared".toList ∧
    deriveNamespace "qcore-provider-chunkio".toList = some "chunkio".toList ∧
    deriveNamespace "qcore-provider-cmetrics".toList = some "cmetrics".toList ∧
    deriveNamespace "qcore-provider-ctraces".toList = some "ctraces".toList ∧
    deriveNamespace "qcore-provider-libevent".toList = some "libevent".toList ∧
    deriveNamespace "qcore-provider-librdkafka".toList = some "librdkafka".toList ∧
    deriveNamespace "other-provider-libc".toList = none ∧
    deriveNamespace "quirin-provider-libc".toList =
      deriveNamespace "qcore-provider-libc".toList := by
  decide

/-- The load-path check on namespaces: every recorded namespace is the derived
one, and no two crates share one. -/
def namespacesValid (derive : Name → Option Name) (crates : List Crate) : Bool :=
  crates.all (fun c => derive c.name == some c.ns) &&
    decide (crates.map (·.ns)).Nodup

theorem nodup_map_injective {α β : Type} [DecidableEq β] {f : α → β} :
    ∀ {l : List α} {a b : α}, (l.map f).Nodup → a ∈ l → b ∈ l → f a = f b → a = b
  | [], _, _, _, ha, _, _ => by cases ha
  | x :: xs, a, b, hnd, ha, hb, hf => by
    rw [List.map_cons, List.nodup_cons] at hnd
    obtain ⟨hx, hxs⟩ := hnd
    rw [List.mem_cons] at ha hb
    rcases ha with rfl | ha <;> rcases hb with rfl | hb
    · rfl
    · exact absurd (hf ▸ List.mem_map_of_mem hb) hx
    · exact absurd (hf.symm ▸ List.mem_map_of_mem ha) hx
    · exact nodup_map_injective hxs ha hb hf

/-- Under the load-path check, a namespace names exactly one crate. -/
theorem namespace_names_one_crate {derive : Name → Option Name} {crates : List Crate}
    (hvalid : namespacesValid derive crates = true) {a b : Crate}
    (ha : a ∈ crates) (hb : b ∈ crates) (hns : a.ns = b.ns) : a = b := by
  unfold namespacesValid at hvalid
  rw [Bool.and_eq_true, decide_eq_true_iff] at hvalid
  exact nodup_map_injective hvalid.2 ha hb hns

theorem namespacesValid_records_derived {derive : Name → Option Name} {crates : List Crate}
    (hvalid : namespacesValid derive crates = true) {c : Crate} (hc : c ∈ crates) :
    derive c.name = some c.ns := by
  unfold namespacesValid at hvalid
  rw [Bool.and_eq_true, List.all_eq_true] at hvalid
  simpa using hvalid.1 c hc

/-- Two distinct crates that derive one namespace — for example the same
remainder under both prefixes — fail the load-path check (`QI017`), whatever
the derivation. -/
theorem shared_namespace_refused {derive : Name → Option Name} {crates : List Crate}
    {a b : Crate} (ha : a ∈ crates) (hb : b ∈ crates) (hne : a ≠ b) (hns : a.ns = b.ns) :
    namespacesValid derive crates = false := by
  cases hvalid : namespacesValid derive crates with
  | false => rfl
  | true => exact absurd (namespace_names_one_crate hvalid ha hb hns) hne

/-! ## 3. Walks with a visiting stack: include and inheritance cycles -/

def allOk {α : Type} (f : α → Except Refusal Unit) : List α → Except Refusal Unit
  | [] => .ok ()
  | x :: xs =>
    match f x with
    | .ok () => allOk f xs
    | .error e => .error e

theorem allOk_each {α : Type} {f : α → Except Refusal Unit} :
    ∀ {xs : List α}, allOk f xs = .ok () → ∀ x ∈ xs, f x = .ok ()
  | [], _, x, hx => by cases hx
  | y :: ys, h, x, hx => by
    unfold allOk at h
    split at h
    · rename_i hy
      rw [List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact hy
      · exact allOk_each h x hx
    · cases h

/-- Depth-first elaboration order over `children` (included modules, or the
parent of an inheriting declaration). Revisiting the stack is `cycle`; fuel
exhaustion is also a refusal, so the walk is total-or-refuse by
construction. -/
def walk {α : Type} [DecidableEq α] (children : α → List α) (cycle : Refusal) :
    Nat → List α → α → Except Refusal Unit
  | 0, _, _ => .error .exhausted
  | fuel + 1, visiting, a =>
    if a ∈ visiting then .error cycle
    else allOk (walk children cycle fuel (a :: visiting)) (children a)

inductive Reaches {α : Type} (children : α → List α) : α → α → Prop
  | step {a b : α} : b ∈ children a → Reaches children a b
  | trans {a b c : α} : b ∈ children a → Reaches children b c → Reaches children a c

theorem walk_visited {α : Type} [DecidableEq α] {children : α → List α} {cycle : Refusal}
    {fuel : Nat} {visiting : List α} {a : α} (h : a ∈ visiting) :
    walk children cycle fuel visiting a ≠ .ok () := by
  cases fuel with
  | zero => simp [walk]
  | succ n => simp [walk, h]

theorem walk_refuses_reach_into_visiting {α : Type} [DecidableEq α]
    {children : α → List α} {cycle : Refusal} {a c : α}
    (hr : Reaches children a c) :
    ∀ {fuel : Nat} {visiting : List α}, c ∈ visiting →
      walk children cycle fuel visiting a ≠ .ok () := by
  induction hr with
  | @step a b hb =>
    intro fuel visiting hc
    cases fuel with
    | zero => simp [walk]
    | succ n =>
      unfold walk
      split
      · simp
      · intro hall
        have := allOk_each hall b hb
        exact walk_visited (List.mem_cons_of_mem a hc) this
  | @trans a b c hb _ ih =>
    intro fuel visiting hc
    cases fuel with
    | zero => simp [walk]
    | succ n =>
      unfold walk
      split
      · simp
      · intro hall
        have := allOk_each hall b hb
        exact ih (List.mem_cons_of_mem a hc) this

/-- Every cycle through the start refuses, whatever the fuel. -/
theorem walk_refuses_cycle {α : Type} [DecidableEq α] {children : α → List α}
    {cycle : Refusal} {a : α} (hr : Reaches children a a) (fuel : Nat) (visiting : List α) :
    walk children cycle fuel visiting a ≠ .ok () := by
  cases fuel with
  | zero => simp [walk]
  | succ n =>
    unfold walk
    split
    · simp
    · intro hall
      cases hr with
      | step hb =>
        exact walk_visited (List.mem_cons_self) (allOk_each hall a hb)
      | trans hb hrest =>
        rename_i b
        exact walk_refuses_reach_into_visiting hrest (List.mem_cons_self)
          (allOk_each hall b hb)

/-- A successful walk certifies that no cycle passes through its start. -/
theorem walk_ok_is_acyclic {α : Type} [DecidableEq α] {children : α → List α}
    {cycle : Refusal} {fuel : Nat} {a : α}
    (h : walk children cycle fuel [] a = .ok ()) : ¬ Reaches children a a :=
  fun hr => walk_refuses_cycle hr fuel [] h

/-- Inheritance chains are the walk over the single-parent relation; a
declaration inheriting itself is the one-step cycle. -/
def parentChildren {α : Type} (parentOf : α → Option α) (a : α) : List α :=
  (parentOf a).toList

theorem self_inheritance_refused {α : Type} [DecidableEq α] {parentOf : α → Option α}
    {a : α} (h : parentOf a = some a) (fuel : Nat) :
    walk (parentChildren parentOf) .inheritCycle fuel [] a ≠ .ok () :=
  walk_refuses_cycle (Reaches.step (by simp [parentChildren, h])) fuel []

/-! ## 4. Declarations, positional renaming and meaning -/

structure Clause where
  /-- The introducer and selector words (`ensures normal`, `error(E)`). -/
  intro : List Word
  /-- The expression's word tokens. -/
  body : List Word
  deriving DecidableEq, Repr

structure Decl where
  name : Name
  params : List Word
  types : List Name
  result : Name
  clauses : List Clause
  inheritable : Bool
  deriving DecidableEq, Repr

def indexOf? : List Word → Word → Option Nat
  | [], _ => none
  | x :: xs, w => if x = w then some 0 else (indexOf? xs w).map (· + 1)

theorem indexOf?_some {L : List Word} {w : Word} {i : Nat} :
    indexOf? L w = some i → ∃ h : i < L.length, L[i] = w := by
  induction L generalizing i with
  | nil => simp [indexOf?]
  | cons x xs ih =>
    intro h
    unfold indexOf? at h
    split at h
    · rename_i hx
      cases h
      exact ⟨by simp, by simpa using hx⟩
    · cases hi : indexOf? xs w with
      | none => rw [hi] at h; cases h
      | some j =>
        rw [hi] at h
        cases h
        obtain ⟨hj, hget⟩ := ih hi
        exact ⟨by simp; omega, by simpa using hget⟩

theorem indexOf?_none {L : List Word} {w : Word} : indexOf? L w = none ↔ w ∉ L := by
  induction L with
  | nil => simp [indexOf?]
  | cons x xs ih =>
    unfold indexOf?
    split
    · rename_i hx
      simp [hx]
    · rename_i hx
      rw [Option.map_eq_none_iff, ih]
      simp only [List.mem_cons, not_or]
      constructor
      · intro h; exact ⟨fun e => hx e.symm, h⟩
      · intro h; exact h.2

theorem indexOf?_nodup {L : List Word} (hnd : L.Nodup) :
    ∀ {i : Nat} (h : i < L.length), indexOf? L L[i] = some i := by
  induction L with
  | nil => intro i h; simp at h
  | cons x xs ih =>
    intro i h
    rw [List.nodup_cons] at hnd
    cases i with
    | zero => simp [indexOf?]
    | succ j =>
      have hj : j < xs.length := by simp at h; omega
      unfold indexOf?
      have hne : ¬ x = (x :: xs)[j + 1] := by
        intro e
        apply hnd.1
        rw [e]
        simp
      simp only [hne, ↓reduceIte]
      simp only [List.getElem_cons_succ]
      rw [ih hnd.2 hj]
      rfl

/-- The positional renaming σ: the `i`-th parent parameter becomes the `i`-th
child parameter; every other word is unchanged. -/
def renameWord (P C : List Word) (w : Word) : Word :=
  match indexOf? P w with
  | some i => C.getD i w
  | none => w

def renameClause (P C : List Word) (cl : Clause) : Clause :=
  { cl with body := cl.body.map (renameWord P C) }

/-- The capture side condition: no child parameter occurs in a parent clause as
a word that is not a parent parameter, and no parameter name is a selector
word. -/
def captures (P C : List Word) (cls : List Clause) : Bool :=
  cls.any (fun cl =>
    cl.body.any (fun w => !(P.contains w) && C.contains w) ||
      cl.intro.any (fun w => P.contains w || C.contains w))

inductive Atom where
  | param (i : Nat)
  | word (w : Word)
  deriving DecidableEq, Repr

/-- A reading that resolves each word by looking it up in the declaration's
parameter list: a parameter reference is its position, anything else is the
word itself. -/
def meaningWord (names : List Word) (w : Word) : Atom :=
  match indexOf? names w with
  | some i => .param i
  | none => .word w

def meaning (names : List Word) (cl : Clause) : List Word × List Atom :=
  (cl.intro, cl.body.map (meaningWord names))

theorem rename_word_meaning {P C : List Word} (hC : C.Nodup) (hlen : P.length = C.length)
    {w : Word} (hcap : (!(P.contains w) && C.contains w) = false) :
    meaningWord C (renameWord P C w) = meaningWord P w := by
  unfold renameWord meaningWord
  cases hP : indexOf? P w with
  | some i =>
    obtain ⟨hi, _⟩ := indexOf?_some hP
    have hiC : i < C.length := hlen ▸ hi
    simp only
    have hget : C.getD i w = C[i] := by
      simp [List.getD, hiC]
    rw [hget, indexOf?_nodup hC hiC]
  | none =>
    simp only
    have hnotP : w ∉ P := indexOf?_none.mp hP
    have hnotC : w ∉ C := by
      intro hmem
      have hPc : P.contains w = false := by simpa using hnotP
      have hCc : C.contains w = true := by simpa using hmem
      rw [hPc, hCc] at hcap
      simp at hcap
    rw [indexOf?_none.mpr hnotC]

/-- Under the side conditions, the renamed clause means exactly what the
parent clause means to a name-resolving reading. -/
theorem rename_preserves_meaning {P C : List Word} (hC : C.Nodup)
    (hlen : P.length = C.length) {cl : Clause} (hcap : captures P C [cl] = false) :
    meaning C (renameClause P C cl) = meaning P cl := by
  unfold meaning renameClause
  simp only [List.map_map]
  unfold captures at hcap
  simp only [List.any_cons, List.any_nil, Bool.or_false, Bool.or_eq_false_iff] at hcap
  obtain ⟨hbody, _⟩ := hcap
  congr 1
  apply List.map_congr_left
  intro w hw
  simp only [Function.comp]
  apply rename_word_meaning hC hlen
  rw [List.any_eq_false] at hbody
  simpa using hbody w hw

theorem captures_each {P C : List Word} {cls : List Clause} (h : captures P C cls = false)
    {cl : Clause} (hcl : cl ∈ cls) : captures P C [cl] = false := by
  unfold captures at h ⊢
  rw [List.any_eq_false] at h
  simpa using h cl hcl

theorem rename_clauses_preserve_meaning {P C : List Word} (hC : C.Nodup)
    (hlen : P.length = C.length) {cls : List Clause} (hcap : captures P C cls = false) :
    (cls.map (renameClause P C)).map (meaning C) = cls.map (meaning P) := by
  rw [List.map_map]
  apply List.map_congr_left
  intro cl hcl
  exact rename_preserves_meaning hC hlen (captures_each hcap hcl)

/-! ## 5. Parent lookup and one inheritance step -/

structure Module where
  /-- The namespace of the crate owning the module. -/
  crateNs : Name
  decls : List Decl
  deriving DecidableEq, Repr

inductive Parent where
  | own (name : Name)
  | qualified (ns name : Name)
  deriving DecidableEq, Repr

def unique (candidates : List Decl) : Except Refusal Decl :=
  match candidates with
  | [d] => .ok d
  | [] => .error .unknownParent
  | _ :: _ :: _ => .error .ambiguousParent

def lookup (own : List Decl) (direct : List Module) : Parent → Except Refusal Decl
  | .own n =>
    if own.filter (fun d => d.name == n) = [] then
      if direct.any (fun m => m.decls.any (fun d => d.name == n)) then
        .error .qualifiedRequired
      else .error .unknownParent
    else unique (own.filter (fun d => d.name == n))
  | .qualified ns n =>
    if direct.filter (fun m => m.crateNs == ns) = [] then .error .unknownNamespace
    else unique (((direct.filter (fun m => m.crateNs == ns)).flatMap (·.decls)).filter
      (fun d => d.name == n))

theorem unique_sound {cs : List Decl} {d : Decl} (h : unique cs = .ok d) : cs = [d] := by
  unfold unique at h
  split at h
  · cases h; rfl
  · cases h
  · cases h

theorem unique_ambiguous {a b : Decl} {rest : List Decl} :
    unique (a :: b :: rest) = .error .ambiguousParent := rfl

/-- An unqualified parent is always an own declaration, and the only one. -/
theorem lookup_own_sound {own : List Decl} {direct : List Module} {n : Name} {d : Decl}
    (h : lookup own direct (.own n) = .ok d) :
    d ∈ own ∧ d.name = n ∧ own.filter (fun d => d.name == n) = [d] := by
  unfold lookup at h
  by_cases hf : own.filter (fun d => d.name == n) = []
  · by_cases ha : direct.any (fun m => m.decls.any (fun d => d.name == n)) = true <;>
      simp [hf, ha] at h
  · simp only [hf, ↓reduceIte] at h
    have hfilter := unique_sound h
    have hmem : d ∈ own.filter (fun d => d.name == n) := by rw [hfilter]; simp
    rw [List.mem_filter] at hmem
    exact ⟨hmem.1, by simpa using hmem.2, hfilter⟩

/-- An unqualified name that only an included module declares is refused with
the qualified spelling required, never resolved into the included module. -/
theorem unqualified_included_only_refused {own : List Decl} {direct : List Module} {n : Name}
    (hown : own.filter (fun d => d.name == n) = [])
    {m : Module} (hm : m ∈ direct) {d : Decl} (hd : d ∈ m.decls) (hn : d.name = n) :
    lookup own direct (.own n) = .error .qualifiedRequired := by
  have hany : direct.any (fun m => m.decls.any (fun d => d.name == n)) = true := by
    rw [List.any_eq_true]
    exact ⟨m, hm, by rw [List.any_eq_true]; exact ⟨d, hd, by simpa using hn⟩⟩
  unfold lookup
  simp only [hown, hany, ↓reduceIte]

/-- A qualified parent comes from a directly included module of the crate the
namespace names, and is the only candidate there. -/
theorem lookup_qualified_sound {own : List Decl} {direct : List Module} {ns n : Name}
    {d : Decl} (h : lookup own direct (.qualified ns n) = .ok d) :
    ∃ m ∈ direct, m.crateNs = ns ∧ d ∈ m.decls ∧ d.name = n := by
  unfold lookup at h
  by_cases hs : direct.filter (fun m => m.crateNs == ns) = []
  · simp [hs] at h
  · simp only [hs, ↓reduceIte] at h
    have hcs := unique_sound h
    have hmem : d ∈ ((direct.filter (fun m => m.crateNs == ns)).flatMap (·.decls)).filter
        (fun d => d.name == n) := by
      rw [hcs]; simp
    rw [List.mem_filter, List.mem_flatMap] at hmem
    obtain ⟨⟨m, hm, hdm⟩, hname⟩ := hmem
    rw [List.mem_filter] at hm
    exact ⟨m, hm.1, by simpa using hm.2, hdm, by simpa using hname⟩

/-- Two candidates in the qualified scope are refused, never resolved by
order (two published modules of one crate declaring the parent). -/
theorem lookup_qualified_ambiguous {own : List Decl} {direct : List Module} {ns n : Name}
    (hne : direct.filter (fun m => m.crateNs == ns) ≠ []) {a b : Decl} {rest : List Decl}
    (h : ((direct.filter (fun m => m.crateNs == ns)).flatMap (·.decls)).filter
      (fun d => d.name == n) = a :: b :: rest) :
    lookup own direct (.qualified ns n) = .error .ambiguousParent := by
  unfold lookup
  simp only [hne, ↓reduceIte, h]
  rfl

/-- A namespace naming no directly included crate is refused. -/
theorem lookup_unknown_namespace {own : List Decl} {direct : List Module} {ns n : Name}
    (h : direct.filter (fun m => m.crateNs == ns) = []) :
    lookup own direct (.qualified ns n) = .error .unknownNamespace := by
  unfold lookup
  simp only [h, ↓reduceIte]

structure Site where
  child : Decl
  parent : Parent
  /-- Clauses written after `inherit PARENT`; none are admitted. -/
  extension : List Clause
  deriving DecidableEq, Repr

def parentName : Parent → Name
  | .own n => n
  | .qualified _ n => n

/-- A child named like its qualified parent (refused in this version). -/
def namedLikeQualifiedParent (site : Site) : Bool :=
  match site.parent with
  | .qualified _ n => site.child.name == n
  | .own _ => false

/-- One inheritance step: the child's clauses, or a refusal. -/
def resolveInherit (own : List Decl) (direct : List Module) (site : Site) :
    Except Refusal (List Clause) :=
  if site.extension ≠ [] then .error .extension
  else if namedLikeQualifiedParent site then .error .sameName
  else
    match lookup own direct site.parent with
    | .error e => .error e
    | .ok p =>
      if p.inheritable = false then .error .notInheritable
      else if ¬ (site.child.types = p.types ∧ site.child.result = p.result ∧
          p.params.length = site.child.params.length) then .error .signature
      else if ¬ (p.params.Nodup ∧ site.child.params.Nodup) then .error .malformed
      else if captures p.params site.child.params p.clauses = true then .error .capture
      else .ok (p.clauses.map (renameClause p.params site.child.params))

theorem extension_refused {own : List Decl} {direct : List Module} {site : Site}
    (h : site.extension ≠ []) : resolveInherit own direct site = .error .extension := by
  unfold resolveInherit
  simp [h]

theorem same_name_refused {own : List Decl} {direct : List Module} {site : Site} {ns : Name}
    (hext : site.extension = []) (hp : site.parent = .qualified ns site.child.name) :
    resolveInherit own direct site = .error .sameName := by
  unfold resolveInherit namedLikeQualifiedParent
  simp [hext, hp]

/-- The elaborated clauses are exactly the parent's clauses under the
positional renaming, the parent is the one the lookup found, and the
signatures agree. -/
theorem elaboration_exact {own : List Decl} {direct : List Module} {site : Site}
    {cls : List Clause} (h : resolveInherit own direct site = .ok cls) :
    ∃ p, lookup own direct site.parent = .ok p ∧
      site.child.types = p.types ∧ site.child.result = p.result ∧
      cls = p.clauses.map (renameClause p.params site.child.params) := by
  unfold resolveInherit at h
  split at h
  · cases h
  split at h
  · cases h
  split at h
  · cases h
  rename_i p hp
  split at h
  · cases h
  split at h
  · cases h
  rename_i hsig
  split at h
  · cases h
  split at h
  · cases h
  cases h
  have hsig' := Classical.not_not.mp hsig
  exact ⟨p, hp, hsig'.1, hsig'.2.1, rfl⟩

/-- An inherited contract is not weaker than its parent: under a
name-resolving reading, every parent clause's meaning is a child clause's
meaning, position for position. -/
theorem inherited_is_not_weaker {own : List Decl} {direct : List Module} {site : Site}
    {cls : List Clause} (h : resolveInherit own direct site = .ok cls) :
    ∃ p, lookup own direct site.parent = .ok p ∧
      cls.map (meaning site.child.params) = p.clauses.map (meaning p.params) ∧
      ∀ cl ∈ p.clauses, meaning p.params cl ∈ cls.map (meaning site.child.params) := by
  unfold resolveInherit at h
  split at h
  · cases h
  split at h
  · cases h
  split at h
  · cases h
  rename_i p hp
  split at h
  · cases h
  split at h
  · cases h
  rename_i hsig
  split at h
  · cases h
  rename_i hnd
  split at h
  · cases h
  rename_i hcap
  cases h
  have hsig' := Classical.not_not.mp hsig
  have hnd' := Classical.not_not.mp hnd
  have hcap' : captures p.params site.child.params p.clauses = false := by
    simpa using hcap
  have heq := rename_clauses_preserve_meaning hnd'.2 hsig'.2.2 hcap'
  refine ⟨p, hp, heq, ?_⟩
  intro cl hcl
  rw [heq]
  exact List.mem_map_of_mem hcl

/-- Transitivity: a chain of two admitted steps preserves the root's
meaning. -/
theorem transitive_inheritance_preserves_meaning {A B C : List Word}
    {root : List Clause} (hB : B.Nodup) (hC : C.Nodup)
    (hAB : A.length = B.length) (hBC : B.length = C.length)
    (hcap1 : captures A B root = false)
    (hcap2 : captures B C (root.map (renameClause A B)) = false) :
    ((root.map (renameClause A B)).map (renameClause B C)).map (meaning C) =
      root.map (meaning A) := by
  rw [rename_clauses_preserve_meaning hC hBC hcap2]
  exact rename_clauses_preserve_meaning hB hAB hcap1

/-! ## 6. Why the consumer gate is needed -/

/-- A structural reading of a two-argument predicate: the first argument is a
parameter reference, the second is read as a keyword *without* a parameter
lookup — the resource-v1 reading of `resource_owned(p, nullable)`. -/
def structuralReading (names : List Word) (cl : Clause) : Option (Nat × Word) :=
  match cl.body with
  | [x, k] => (indexOf? names x).map (fun i => (i, k))
  | _ => none

/-- With a parent parameter interned as the keyword `nullable` (1) and a child
parameter as `nonnull` (2): the renaming passes the capture check and
preserves the name-resolving meaning, yet the structural reading flips the
nullability. The v1 loader therefore compares its own reading of child and
parent and refuses a difference. -/
theorem positional_reading_can_diverge :
    let P : List Word := [1, 3]
    let C : List Word := [2, 4]
    let cl : Clause := { intro := [], body := [3, 1] }
    captures P C [cl] = false ∧
      meaning C (renameClause P C cl) = meaning P cl ∧
      structuralReading P cl ≠ structuralReading C (renameClause P C cl) := by
  decide

/-- The consumer gate: admit an inherited declaration only when the
consumer's own reading of the elaborated child equals its reading of the
parent. -/
def consumerGate {β : Type} [DecidableEq β] (read : List Word → List Clause → β)
    (parent child : List Word) (parentClauses childClauses : List Clause) :
    Except Refusal Unit :=
  if read child childClauses = read parent parentClauses then .ok ()
  else .error .consumerDisagrees

theorem consumerGate_sound {β : Type} [DecidableEq β] {read : List Word → List Clause → β}
    {P C : List Word} {pc cc : List Clause}
    (h : consumerGate read P C pc cc = .ok ()) : read C cc = read P pc := by
  unfold consumerGate at h
  split at h
  · assumption
  · cases h

/-! ## 7. The load path built from real metadata (card CONTRACT-INHERIT-ADOPTION)

`providers/contracts.py` builds each load-path crate from `cargo metadata`
(name, declared normal dependencies) and the crate's recorded constants;
`qcore_contract::load_path_manifest` reads the result as data, refusing a
published module whose text is not its pinned digest; and a schema-2 resource
manifest mixes path entries with `crate:module` references. -/

/-- A Cargo package as `cargo metadata --no-deps` reports it. -/
structure CargoPackage where
  name : Name
  deps : List Name
  deriving DecidableEq, Repr

/-- The contract-use edges of a package: its declared dependencies that are
themselves load-path crates. Nothing else becomes an edge. -/
def contractDeps (onPath : List Name) (p : CargoPackage) : List Name :=
  p.deps.filter (fun d => decide (d ∈ onPath))

/-- The load-path crate the generator writes for a package. -/
def crateOf (onPath : List Name) (p : CargoPackage) (ns : Name) (published : List Name) :
    Crate :=
  { name := p.name, ns := ns, deps := contractDeps onPath p, published := published }

theorem contractDeps_sound {onPath : List Name} {p : CargoPackage} {d : Name}
    (h : d ∈ contractDeps onPath p) : d ∈ p.deps ∧ d ∈ onPath := by
  unfold contractDeps at h
  rw [List.mem_filter] at h
  exact ⟨h.1, of_decide_eq_true h.2⟩

/-- No edge is invented: an include that resolves from a crate built from
Cargo metadata names the crate itself or a crate the package declares as a
dependency, which is itself on the load path. -/
theorem include_follows_a_cargo_edge {crates : List Crate} {onPath : List Name}
    {p : CargoPackage} {ns : Name} {published : List Name} {target r : Name × Name}
    (hsrc : findCrate crates p.name = .ok (crateOf onPath p ns published))
    (h : resolveInclude crates p.name target = .ok r) :
    target.1 = p.name ∨ (target.1 ∈ p.deps ∧ target.1 ∈ onPath) := by
  obtain ⟨_, ⟨src, hfind, hreach⟩, _⟩ := resolveInclude_sound h
  rw [hsrc] at hfind
  cases hfind
  rcases hreach with hself | hdep
  · exact Or.inl hself
  · exact Or.inr (contractDeps_sound hdep)

/-- A package without a declared dependency on a load-path crate yields a
crate that includes nothing foreign. -/
theorem no_cargo_edge_includes_nothing_foreign {crates : List Crate} {onPath : List Name}
    {p : CargoPackage} {ns : Name} {published : List Name} {target : Name × Name}
    (hsrc : findCrate crates p.name = .ok (crateOf onPath p ns published))
    (hnone : ∀ d ∈ p.deps, d ∉ onPath) (hother : target.1 ≠ p.name) :
    resolveInclude crates p.name target = .error .unreachableCrate := by
  apply no_dependency_includes_nothing_foreign hsrc _ hother
  show contractDeps onPath p = []
  unfold contractDeps
  rw [List.filter_eq_nil_iff]
  intro d hd
  simpa using hnone d hd

/-- A published module with its pinned digest and the digest of the text
read from disk (digests modelled as naturals). -/
structure PinnedModule where
  module : Name
  pin : Nat
  onDisk : Nat
  deriving DecidableEq, Repr

/-- The manifest reader's pin check: every published module's text must be
its pin, or the whole load path is refused. -/
def admitPinned (modules : List PinnedModule) : Except Refusal (List Name) :=
  if modules.all (fun m => m.onDisk == m.pin) then .ok (modules.map (·.module))
  else .error .invalidLoadPath

theorem admitPinned_sound {modules : List PinnedModule} {names : List Name}
    (h : admitPinned modules = .ok names) :
    names = modules.map (·.module) ∧ ∀ m ∈ modules, m.onDisk = m.pin := by
  unfold admitPinned at h
  split at h
  · rename_i hall
    cases h
    refine ⟨rfl, fun m hm => ?_⟩
    have := List.all_eq_true.mp hall m hm
    simpa using this
  · cases h

/-- A parent that changed without its pin being regenerated is refused, so
no inheriting contract can silently take a different parent text. -/
theorem changed_parent_refused {modules : List PinnedModule} {m : PinnedModule}
    (hm : m ∈ modules) (hchanged : m.onDisk ≠ m.pin) :
    admitPinned modules = .error .invalidLoadPath := by
  unfold admitPinned
  have : modules.all (fun m => m.onDisk == m.pin) = false := by
    rw [List.all_eq_false]
    exact ⟨m, hm, by simpa using hchanged⟩
  simp [this]

/-- One entry of a schema-2 resource manifest. -/
inductive ManifestEntry where
  | path (file : Name)
  | ref (crateName module : Name)
  deriving DecidableEq, Repr

def pathsOf : List ManifestEntry → List Name
  | [] => []
  | .path f :: rest => f :: pathsOf rest
  | .ref _ _ :: rest => pathsOf rest

def refsOf : List ManifestEntry → List (Name × Name)
  | [] => []
  | .path _ :: rest => refsOf rest
  | .ref c m :: rest => (c, m) :: refsOf rest

/-- Schema 1 loads its files; schema 2 loads its path entries the same way
and elaborates its references on the load path, in one load. -/
def selectSchema1 {α : Type} (loadFiles : List Name → α) (files : List Name) : α :=
  loadFiles files

def selectSchema2 {α : Type} (loadFiles : List Name → α)
    (loadMixed : List Name → List (Name × Name) → α) (entries : List ManifestEntry) : α :=
  if refsOf entries = [] then loadFiles (pathsOf entries)
  else loadMixed (pathsOf entries) (refsOf entries)

theorem pathsOf_map_path (files : List Name) : pathsOf (files.map .path) = files := by
  induction files with
  | nil => rfl
  | cons f rest ih => simp [pathsOf, ih]

theorem refsOf_map_path (files : List Name) : refsOf (files.map .path) = [] := by
  induction files with
  | nil => rfl
  | cons f rest ih => simp [refsOf, ih]

/-- Path entries resolve identically: a schema-2 manifest whose entries are
the schema-1 manifest's files, in order, loads exactly what schema 1 loads. -/
theorem schema_two_paths_are_schema_one {α : Type} (loadFiles : List Name → α)
    (loadMixed : List Name → List (Name × Name) → α) (files : List Name) :
    selectSchema2 loadFiles loadMixed (files.map .path) = selectSchema1 loadFiles files := by
  simp [selectSchema2, selectSchema1, refsOf_map_path, pathsOf_map_path]

end QCore.ContractInherit
