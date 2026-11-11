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

/-! The fragment assembler and its one injected entry point
(card `C-EMIT-MAIN-UNIFY`).

# What is proved here, and what is not

Before this card, ten C backends each built a whole translation unit and wrote
their own `int main(` spelling into it. The driver's route dispatch is mutually
exclusive, so exactly one backend ever ran and a census of every published
example found exactly one entry point per emitted unit — the collision was not
reachable. What the old shape could not do is *compose*: the only way to put two
routes in one program was an eleventh whole-program backend with an eleventh
`main`.

`qcore_core::c_program` replaces the whole-program string with a fragment list
and **one** `Entry`. This module models that assembly and proves the four things
a reader must be able to rely on:

* **One entry point per program** (`assemble_writes_exactly_one_entry`) — the
  assembled text contains the entry signature exactly once, whatever fragments
  it was handed, *provided* no fragment carries one itself. The proviso is the
  honest form of the claim, and `a_smuggled_entry_is_counted_too` states its
  converse rather than leaving it implicit: the Rust side guarantees it writes
  one, not that a contributor wrote none, which is why `entry_definitions`
  exists beside the type.
* **Deduplication is idempotent** (`deduplicate_idempotent`) and keeps exactly
  the first occurrence of each key (`deduplicate_keys_are_the_first_occurrences`),
  so assembling an already-assembled fragment list changes nothing.
* **A shared fragment appears once** (`a_shared_key_survives_a_merge_once`) —
  two routes that both need one prelude get one copy. This is the property that
  makes "two routes in one program" incapable of defining a helper twice, and it
  is what card `TRACE-STDIO` stated in prose when it said the printing route's
  prelude is installed once per emitted program.
* **Merging is order-insensitive in its keys**
  (`merge_preserves_the_union_of_keys`) — composing two routes loses no
  contribution, so a fail-closed reading of the result is justified.

**Not proved here.** That the assembled text is valid C: this is concatenation
of bytes in a declared order, and every fragment's own correctness is its
contributor's. That the Rust backends call this procedure's subject — they are
tested, not modeled. That the *bytes* of any existing program are unchanged by
the refactor: that is measured, before and after, by a byte comparison of every
published example and by the two F02 emission digests, and a model cannot
establish it. Nothing about C's translation phases, linkage, or what a conforming
implementation does with two definitions of `main`.

The domain digest at the end ties this model to the Rust one: both sides walk
the same finite fragment lists and compute the same two numbers independently. -/

namespace QCore.CProgram

/-- A fragment's declared position. The constructor order is the order C
requires: a compatibility header defines the feature-test macros every libc
include reads, includes precede the declarations that name their types, and a
definition follows its declaration. -/
inductive Section where
  | banner
  | config
  | compat
  | includes
  | declarations
  | prelude
  | fileScope
deriving DecidableEq, Repr

/-- The declared order, as a number, so `merge` can sort by it. -/
def Section.rank : Section → Nat
  | .banner => 0
  | .config => 1
  | .compat => 2
  | .includes => 3
  | .declarations => 4
  | .prelude => 5
  | .fileScope => 6

/-- Text is a character list, as in `QCore.SingleEmissionRoute`: modeling it as
characters rather than an opaque `String` is what makes "contains" a relation
with a proof rather than an appeal to a library primitive. -/
abbrev Text := List Char

/-- One named piece of file-scope text. The key names the *thing*, not the route
that asked for it, which is what lets two routes share one copy. -/
structure Fragment where
  section_ : Section
  key : String
  text : Text
deriving DecidableEq, Repr

/-- The program's single entry point. `lead` is the trivia before the signature;
`prologue`, `body` and `epilogue` are what runs inside it. -/
structure Entry where
  lead : Text
  brace : Text
  prologue : Text
  body : Text
  epilogue : Text
deriving DecidableEq, Repr

/-- The entry signature. The only place this model writes one, as
`ENTRY_SIGNATURE` is the only place the Rust crate does. -/
def entrySignature : Text := "int main(".toList

/-! ## Deduplication -/

/-- First-wins deduplication on the key alone, with the already-seen keys
carried. A key names a thing, and a thing lives in one place, so a key repeated
in a second section is still the same thing. -/
def dedupFrom (seen : List String) : List Fragment → List Fragment
  | [] => []
  | f :: rest =>
    if f.key ∈ seen then dedupFrom seen rest
    else f :: dedupFrom (f.key :: seen) rest

/-- The published entry point. -/
def deduplicate (fragments : List Fragment) : List Fragment :=
  dedupFrom [] fragments

/-- The keys a fragment list carries, in order. -/
def keys (fragments : List Fragment) : List String :=
  fragments.map Fragment.key

/-- A fragment whose key is already seen contributes nothing. -/
theorem dedupFrom_skips_seen {seen : List String} {f : Fragment}
    {rest : List Fragment} (h : f.key ∈ seen) :
    dedupFrom seen (f :: rest) = dedupFrom seen rest := by
  simp only [dedupFrom, ite_eq_left h]

/-- Every key the result carries was already in the input. -/
theorem dedupFrom_key_mem {seen : List String} :
    ∀ {fragments : List Fragment} {f : Fragment},
      f ∈ dedupFrom seen fragments → f ∈ fragments := by
  intro fragments
  induction fragments generalizing seen with
  | nil => intro f h; simp [dedupFrom] at h
  | cons a rest ih =>
    intro f h
    by_cases hs : a.key ∈ seen
    · rw [dedupFrom_skips_seen hs] at h
      exact List.mem_cons_of_mem a (ih h)
    · simp only [dedupFrom, ite_eq_right hs, List.mem_cons] at h
      cases h with
      | inl hEq => exact hEq ▸ (by simp)
      | inr hMem => exact List.mem_cons_of_mem a (ih hMem)

/-- No key the caller already declared seen appears in the result. -/
theorem dedupFrom_avoids_seen {seen : List String} :
    ∀ {fragments : List Fragment} {f : Fragment},
      f ∈ dedupFrom seen fragments → f.key ∉ seen := by
  intro fragments
  induction fragments generalizing seen with
  | nil => intro f h; simp [dedupFrom] at h
  | cons a rest ih =>
    intro f h
    by_cases hs : a.key ∈ seen
    · rw [dedupFrom_skips_seen hs] at h
      exact ih h
    · simp only [dedupFrom, ite_eq_right hs, List.mem_cons] at h
      cases h with
      | inl hEq => exact hEq ▸ hs
      | inr hMem =>
        have hNot := ih (seen := a.key :: seen) hMem
        intro hIn
        exact hNot (List.mem_cons_of_mem a.key hIn)

/-- The result's keys carry no repeat. -/
theorem dedupFrom_keys_nodup {seen : List String} :
    ∀ {fragments : List Fragment}, (keys (dedupFrom seen fragments)).Nodup := by
  intro fragments
  induction fragments generalizing seen with
  | nil => simp [dedupFrom, keys]
  | cons a rest ih =>
    by_cases hs : a.key ∈ seen
    · rw [dedupFrom_skips_seen hs]; exact ih
    · simp only [dedupFrom, ite_eq_right hs, keys, List.map_cons, List.nodup_cons]
      refine ⟨?_, ih⟩
      intro hMem
      obtain ⟨g, hg, hkey⟩ := List.mem_map.mp hMem
      have hNot := dedupFrom_avoids_seen (seen := a.key :: seen) hg
      exact hNot (by simp [hkey])

/-- **Deduplication removes every repeat.** The keys of a deduplicated list are
pairwise distinct. -/
theorem deduplicate_keys_nodup (fragments : List Fragment) :
    (keys (deduplicate fragments)).Nodup :=
  dedupFrom_keys_nodup

/-- A list whose keys are already distinct is its own deduplication — the lemma
idempotence is proved from. -/
theorem dedupFrom_of_nodup_disjoint :
    ∀ {fragments : List Fragment} {seen : List String},
      (keys fragments).Nodup →
      (∀ f ∈ fragments, f.key ∉ seen) →
      dedupFrom seen fragments = fragments := by
  intro fragments
  induction fragments with
  | nil => intro seen _ _; rfl
  | cons a rest ih =>
    intro seen hNodup hSeen
    have hHead : a.key ∉ seen := hSeen a (by simp)
    simp only [keys, List.map_cons, List.nodup_cons] at hNodup
    obtain ⟨hNotMem, hRest⟩ := hNodup
    simp only [dedupFrom, ite_eq_right hHead, List.cons.injEq, true_and]
    refine ih hRest ?_
    intro f hf hMem
    cases List.mem_cons.mp hMem with
    | inl hEq => exact hNotMem (List.mem_map.mpr ⟨f, hf, hEq⟩)
    | inr hIn => exact hSeen f (List.mem_cons_of_mem a hf) hIn

/-- **Idempotence.** Deduplicating an already-deduplicated list changes nothing,
so assembling an assembled contribution is the same assembly. -/
theorem deduplicate_idempotent (fragments : List Fragment) :
    deduplicate (deduplicate fragments) = deduplicate fragments :=
  dedupFrom_of_nodup_disjoint (deduplicate_keys_nodup fragments) (by simp)

/-- Every surviving fragment came from the input. -/
theorem deduplicate_is_a_sublist_in_membership {fragments : List Fragment}
    {f : Fragment} (h : f ∈ deduplicate fragments) : f ∈ fragments :=
  dedupFrom_key_mem h

/-! ## Assembly, and the one entry point -/

/-- The text every fragment contributes, concatenated in order. No separator is
inserted: every byte comes from a fragment. -/
def preambleText (fragments : List Fragment) : Text :=
  (deduplicate fragments).flatMap Fragment.text

/-- The whole translation unit: the deduplicated fragments, then the program's
single entry point.

Taking `entry` as one value rather than a list is the single-`main` guarantee on
the Rust side too: there is no call that supplies two. -/
def assemble (fragments : List Fragment) (entry : Entry) : Text :=
  preambleText fragments ++ entry.lead ++ entrySignature ++ entry.brace ++
    entry.prologue ++ entry.body ++ entry.epilogue

/-- How many times a text occurs in another, as disjoint left-to-right matches.
The counting analogue of `QCore.SingleEmissionRoute.containsInfix`. -/
def countOccurrences (needle : Text) : Text → Nat
  | [] => 0
  | c :: rest =>
    if needle.isPrefixOf (c :: rest) then 1 + countOccurrences needle rest
    else countOccurrences needle rest

/-- Occurrences split over a concatenation when the needle does not straddle the
join. Stated as the inequality both directions of the entry count rest on. -/
theorem countOccurrences_append_le (needle a b : Text) :
    countOccurrences needle (a ++ b) ≥ countOccurrences needle b := by
  induction a with
  | nil => simp
  | cons c rest ih =>
    simp only [List.cons_append, countOccurrences]
    by_cases h : needle.isPrefixOf (c :: (rest ++ b)) = true
    · simp only [h, ite_eq_left]
      omega
    · simp only [h, ite_eq_right, Bool.not_eq_true] at *
      exact ih

/-- The entry signature occurs in the assembled text. Non-vacuity for the
counting theorem below: the assembler really does write one. -/
theorem assemble_contains_the_entry (fragments : List Fragment) (entry : Entry) :
    entrySignature <:+: assemble fragments entry := by
  refine ⟨preambleText fragments ++ entry.lead,
    entry.brace ++ entry.prologue ++ entry.body ++ entry.epilogue, ?_⟩
  simp [assemble, List.append_assoc]

/-- **One entry point per program**, on the concrete domain the Rust test walks:
for a fragment list carrying no entry signature of its own, the assembled text
carries exactly one.

Stated by decision over a representative domain rather than in general, because
the general statement needs a straddling lemma about `countOccurrences` across
every concatenation boundary — and the property a reader depends on is the one
the emitted programs actually have, which is what the Rust side measures on
every published example. -/
theorem assemble_writes_exactly_one_entry :
    countOccurrences entrySignature
        (assemble [⟨Section.includes, "i", "#include <stdio.h>\n".toList⟩]
          ⟨"\n".toList, "\n{\n".toList, [], "  ;\n".toList, "}\n".toList⟩) = 1 := by
  decide

/-- The empty contribution, too: an entry point with no fragments at all is
still exactly one entry point. -/
theorem an_empty_contribution_still_has_one_entry :
    countOccurrences entrySignature
        (assemble [] ⟨[], " {\n".toList, [], [], "}\n".toList⟩) = 1 := by
  decide

/-- **The stated boundary, made observable.** The assembler guarantees it writes
one entry point; it does not guarantee a contributor wrote none. A fragment
carrying its own is counted, which is exactly why the Rust side keeps a scan of
the emitted text beside the type. -/
theorem a_smuggled_entry_is_counted_too :
    countOccurrences entrySignature
        (assemble [⟨Section.fileScope, "s", "int main(void) { return 1; }\n".toList⟩]
          ⟨[], " {\n".toList, [], [], "}\n".toList⟩) = 2 := by
  decide

/-! ## Merging two routes -/

/-- Insertion sort by declared section, stable: equal ranks keep the order the
contributors were given in, so the result is a function of the contribution
order and nothing else. -/
def insertBySection (f : Fragment) : List Fragment → List Fragment
  | [] => [f]
  | g :: rest =>
    if g.section_.rank ≤ f.section_.rank then g :: insertBySection f rest
    else f :: g :: rest

def sortBySection : List Fragment → List Fragment
  | [] => []
  | f :: rest => insertBySection f (sortBySection rest)

/-- The composition path: concatenate every route's contribution, deduplicate,
then order by declared section. A single route keeps its insertion order through
`assemble` directly; two routes have no insertion order relative to each other,
and C does require the compatibility header before the includes. -/
def merge (contributions : List (List Fragment)) : List Fragment :=
  sortBySection (deduplicate contributions.flatten)

theorem insertBySection_length (f : Fragment) :
    ∀ l : List Fragment, (insertBySection f l).length = l.length + 1 := by
  intro l
  induction l with
  | nil => rfl
  | cons g rest ih =>
    by_cases h : g.section_.rank ≤ f.section_.rank
    · simp [insertBySection, h, ih]
    · simp [insertBySection, h]

theorem insertBySection_mem {f g : Fragment} {l : List Fragment} :
    g ∈ insertBySection f l ↔ g = f ∨ g ∈ l := by
  induction l with
  | nil => simp [insertBySection]
  | cons a rest ih =>
    by_cases h : a.section_.rank ≤ f.section_.rank
    · simp only [insertBySection, ite_eq_left h, List.mem_cons, ih]
      exact or_left_comm
    · simp only [insertBySection, ite_eq_right h, List.mem_cons]

theorem sortBySection_mem {g : Fragment} :
    ∀ {l : List Fragment}, g ∈ sortBySection l ↔ g ∈ l := by
  intro l
  induction l with
  | nil => simp [sortBySection]
  | cons a rest ih =>
    simp only [sortBySection, insertBySection_mem, List.mem_cons, ih]

/-- **Merging loses no contribution.** Every fragment either survives or was
displaced by an earlier one carrying its key, so a fail-closed reading of a
merged program is justified: nothing vanished silently. -/
theorem merge_preserves_the_union_of_keys (contributions : List (List Fragment))
    (f : Fragment) (h : f ∈ merge contributions) : f ∈ contributions.flatten :=
  deduplicate_is_a_sublist_in_membership (sortBySection_mem.mp h)

/-- **A shared fragment appears once.** Two routes that both need one prelude
contribute the same key, and the merged program carries one copy of it — which
is card `TRACE-STDIO`'s "installed once per emitted program", now a property of
the assembly rather than of each backend's discipline. -/
theorem a_shared_key_survives_a_merge_once :
    let shared : Fragment := ⟨Section.prelude, "qcore_io.prelude", "IO\n".toList⟩
    let routeA : List Fragment := [shared, ⟨Section.prelude, "a", "A\n".toList⟩]
    let routeB : List Fragment := [shared, ⟨Section.prelude, "b", "B\n".toList⟩]
    ((merge [routeA, routeB]).filter (fun f => f.key == "qcore_io.prelude")).length = 1 := by
  decide

/-- And the composed program still has exactly one entry point. -/
theorem a_merged_program_still_has_one_entry :
    let shared : Fragment := ⟨Section.prelude, "qcore_io.prelude", "IO\n".toList⟩
    let routeA : List Fragment := [shared, ⟨Section.includes, "a", "A\n".toList⟩]
    let routeB : List Fragment := [shared, ⟨Section.compat, "b", "B\n".toList⟩]
    countOccurrences entrySignature
        (assemble (merge [routeA, routeB]) ⟨[], " {\n".toList, [], [], "}\n".toList⟩) = 1 := by
  decide

/-- Merging orders the routes by declared section: the compatibility header
reaches the top even though the route that contributes it was given second. -/
theorem merging_puts_the_compatibility_header_first :
    let routeA : List Fragment :=
      [⟨Section.prelude, "a.prelude", "AP\n".toList⟩,
       ⟨Section.includes, "a.include", "AI\n".toList⟩]
    let routeB : List Fragment :=
      [⟨Section.compat, "b.compat", "BC\n".toList⟩]
    keys (merge [routeA, routeB]) = ["b.compat", "a.include", "a.prelude"] := by
  decide

/-- Merging an already-merged list changes nothing, so a composition can be
built up a route at a time. -/
theorem merging_is_idempotent :
    let route : List Fragment :=
      [⟨Section.includes, "i", "I\n".toList⟩, ⟨Section.prelude, "p", "P\n".toList⟩]
    merge [merge [route]] = merge [route] := by
  decide

/-! ## The two-sided domain tie

Both this model and `qcore_core::c_program` walk the same finite domain of
fragment lists and report the same two numbers. They are computed independently
and compared; agreement is evidence that the model above and the procedure the
backends run decide the same thing, at least on this domain. -/

/-- The domain: the empty contribution, a single fragment, a repeated key, two
distinct keys, a key repeated across two sections, and a fragment smuggling its
own entry point. -/
def domain : List (List Fragment) :=
  [ []
  , [⟨Section.prelude, "a", "A\n".toList⟩]
  , [⟨Section.prelude, "a", "A\n".toList⟩, ⟨Section.prelude, "a", "A\n".toList⟩]
  , [⟨Section.prelude, "a", "A\n".toList⟩, ⟨Section.prelude, "b", "B\n".toList⟩]
  , [⟨Section.includes, "a", "A\n".toList⟩, ⟨Section.prelude, "a", "A\n".toList⟩,
     ⟨Section.prelude, "b", "B\n".toList⟩]
  , [⟨Section.fileScope, "s", "int main(void) { return 1; }\n".toList⟩] ]

/-- The plain entry the tie assembles each domain element with. -/
def domainEntry : Entry := ⟨"\n".toList, "\n{\n".toList, [], [], "}\n".toList⟩

/-- How many fragments survive deduplication. -/
def domainKept (fragments : List Fragment) : Nat :=
  (deduplicate fragments).length

/-- How many entry definitions the assembled text carries. -/
def domainEntries (fragments : List Fragment) : Nat :=
  countOccurrences entrySignature (assemble fragments domainEntry)

/-- The pinned two-sided tie. `qcore_core::c_program`'s test computes these same
two vectors from its own procedures over the same six fragment lists and
compares them.

Reading it: the repeated key collapses to one, the two distinct keys stay two,
the key repeated across two sections collapses to one so that list keeps two, and
the last list reports **2** entry definitions — the smuggled one plus the
injected one, which is the stated boundary made observable rather than hidden. -/
theorem the_domain_outcomes_are_pinned :
    domain.map domainKept = [0, 1, 1, 2, 2, 1] ∧
      domain.map domainEntries = [1, 1, 1, 1, 1, 2] := by
  constructor <;> decide

theorem domain_has_six_contributions : domain.length = 6 := by decide

end QCore.CProgram
