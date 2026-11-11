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

/-!
# The declared-ABI reader's qualifier-after-the-last-`*` step

Card **W43-LIBC-RESULT-ALIAS**, discharging `W43-LIBC-POINTER-QUALIFIER-READER`.
`QCore.DeclaredAbiAgreement` takes the C side's projection as given, and
`QCore.TypedefProjection` models how a typedef name is followed. This module
models the one step this card changed in
`crates/qcore-c-gateway/src/legacy_declared_abi.rs`: how a **parameter**
declarator's words are read before its pointer depth is counted.

## What was refused, and why it is a reader boundary

`project_declarator` counts only *trailing* `*` words and refuses a declarator
whose head still contains a `*`. A parameter spelled
`const char *__restrict __format` — glibc's `printf`, `memcpy`, `strcpy` and
others — ends in a qualifier, so it has no trailing star and a star in its head,
and it projected to nothing: every such declaration was withdrawn with
`abi-unreadable-declaration` at every call site.

## The rule

A qualifier after a parameter declarator's **last** `*` qualifies the parameter
object itself, and "each parameter declared with qualified type is taken as
having the unqualified version of its declared type" (C11 6.7.6.3p15). The
reader now drops exactly those qualifiers, and nothing else.

## What is proved

* **The widening only adds.** Every parameter the earlier reader projected is
  projected identically, so no comparison that agreed or disagreed before can
  have moved for a declarator that was already read.
* **It reads the unqualified type** — as an equation, for any run of trailing
  qualifiers: C11 6.7.6.3p15 in this vocabulary.
* **It is strict**, with the measured glibc witness `const char *__restrict`,
  which was unreadable and is now `ptr<const, u8>`; and the pointee's
  constness is kept, so it does not become `ptr<mut, u8>`.
* **Exactly one position is widened**: a qualifier *between* two `*` still
  refuses, and a declarator with no `*` is read exactly as before.
* **It is idempotent**, so reading twice decides nothing new.

## Array bounds (card W43-LIBC-RESTRICT-PROTOTYPES)

The second section models the array-parameter step: a bound that is a run of
qualifiers and at most one size word is read (C11 6.7.6.3p7), and the element
declarator gets one more `*` without dropping its own trailing qualifiers. It
proves the array reading equals the adjusted pointer parameter, that every
bound read before is read now, that the reading only adds where the element has
nothing to drop, and that the one narrowing — an array of qualified pointers,
`char *const p[]` — now refuses exactly as `char *const *p` does. glibc
`regexec`'s `regmatch_t __pmatch[__restrict __nmatch]` is the witness.

## What this is not

A model of a **decision**, as its two neighbours are. The word list is the
declarator the reader extracted from the token stream (after the parameter name
and any array suffix are handled); that it is the parameter C declares is the
faithful-extraction premise of trust boundary 28. Pointee readability is
modelled by `QCore.TypedefProjection` and abstracted here. The result declarator
is not widened: 6.7.6.3p15 speaks of parameters, and the Rust applies the step
to parameters only.
-/

namespace QCore.PointerQualifierReader

/-- One word of a parameter declarator. `const` is kept apart from the other
qualifiers (`volatile`, `restrict`, `__restrict`, `__restrict__`) because it is
the one whose position before the stars the projection reads. -/
inductive Word where
  | star
  | const
  | qualifier
  | specifier
  deriving DecidableEq, Repr

/-- The qualifier table `QUALIFIERS`. -/
def isQualifier : Word → Bool
  | .const => true
  | .qualifier => true
  | _ => false

/-- Whether a word list contains a `*`. -/
def hasStar : List Word → Bool
  | [] => false
  | w :: ws => w == .star || hasStar ws

/-- Whether a word list contains `const`. -/
def hasConst : List Word → Bool
  | [] => false
  | w :: ws => w == .const || hasConst ws

/-! A declarator is modelled **reversed**, last word first, because every rule
here reads it from its end. -/

/-- Split off the trailing stars: their number, and the rest (still reversed). -/
def countStars : List Word → Nat × List Word
  | .star :: rest => ((countStars rest).1 + 1, (countStars rest).2)
  | ws => (0, ws)

/-- The earlier reader, `project_declarator`: pointer depth and whether the
pointee is `const`, or `none` when a `*` remains in the head. Depth `0` is a
scalar, whose own reading `project_scalar` owns and this model abstracts. -/
def project (rev : List Word) : Option (Nat × Bool) :=
  let split := countStars rev
  if hasStar split.2 then none
  else if split.1 = 0 then some (0, false)
  else some (split.1, hasConst split.2)

/-- Drop leading (that is, trailing-in-C) qualifiers. -/
def stripQualifiers : List Word → List Word
  | [] => []
  | w :: ws => if isQualifier w then stripQualifiers ws else w :: ws

/-- `drop_parameter_object_qualifiers`: drop the qualifiers after the last `*`,
and only when a `*` is what they follow. -/
def dropObjectQualifiers (rev : List Word) : List Word :=
  match stripQualifiers rev with
  | .star :: rest => .star :: rest
  | _ => rev

/-- The reader after this card, for a parameter. -/
def projectParameter (rev : List Word) : Option (Nat × Bool) :=
  project (dropObjectQualifiers rev)

/-! ### Lemmas -/

theorem stripQualifiers_of_not_qualifier {w : Word} (h : isQualifier w = false)
    (ws : List Word) : stripQualifiers (w :: ws) = w :: ws := by
  simp [stripQualifiers, h]

theorem hasStar_of_strip_star {rev rest : List Word}
    (h : stripQualifiers rev = .star :: rest) : hasStar rev = true := by
  induction rev with
  | nil => simp [stripQualifiers] at h
  | cons w ws ih =>
      cases w <;> simp_all [stripQualifiers, isQualifier, hasStar]

theorem strip_qualifiers_then_star (qs rest : List Word)
    (hq : ∀ w ∈ qs, isQualifier w = true) :
    stripQualifiers (qs ++ .star :: rest) = .star :: rest := by
  induction qs with
  | nil => simp [stripQualifiers, isQualifier]
  | cons q qs ih =>
      have hq1 : isQualifier q = true := hq q (by simp)
      have hrest : ∀ w ∈ qs, isQualifier w = true := fun w hw => hq w (by simp [hw])
      simp [stripQualifiers, hq1, ih hrest]

theorem strip_of_no_star {rev : List Word} (h : hasStar rev = false) :
    ∀ rest, stripQualifiers rev ≠ .star :: rest := by
  intro rest heq
  have := hasStar_of_strip_star heq
  simp_all

/-! ### The widening only adds -/

/-- Pointwise: every parameter the earlier reader projected is projected
identically. -/
theorem the_widening_only_adds (rev : List Word) {x : Nat × Bool}
    (h : project rev = some x) : projectParameter rev = some x := by
  unfold projectParameter dropObjectQualifiers
  cases rev with
  | nil => simp_all [stripQualifiers]
  | cons w ws =>
      cases hw : isQualifier w with
      | false =>
          rw [stripQualifiers_of_not_qualifier hw]
          cases w <;> simp_all
      | true =>
          -- A declarator ending in a qualifier has no trailing star, so the
          -- earlier reader projected it only when it contains no `*` at all.
          have hstar : countStars (w :: ws) = (0, w :: ws) := by
            cases w <;> simp_all [isQualifier, countStars]
          have hno : hasStar (w :: ws) = false := by
            unfold project at h
            rw [hstar] at h
            cases hs : hasStar (w :: ws) <;> simp_all
          cases hsq : stripQualifiers (w :: ws) with
          | nil => simp_all
          | cons v vs =>
              cases v with
              | star => exact absurd hsq (strip_of_no_star hno vs)
              | const => simp_all
              | qualifier => simp_all
              | specifier => simp_all

/-- No reading is lost: a declarator the widened reader refuses was refused
before too. -/
theorem the_widening_loses_no_reading (rev : List Word)
    (h : projectParameter rev = none) : project rev = none := by
  cases hp : project rev with
  | none => rfl
  | some x =>
      have := the_widening_only_adds rev hp
      simp_all

/-! ### It reads the unqualified type -/

/-- C11 6.7.6.3p15 in this vocabulary: a run of qualifiers after the last `*`
is read exactly as its absence. -/
theorem the_widening_reads_the_unqualified_type (qs rest : List Word)
    (hq : ∀ w ∈ qs, isQualifier w = true) :
    projectParameter (qs ++ .star :: rest) = project (.star :: rest) := by
  unfold projectParameter dropObjectQualifiers
  rw [strip_qualifiers_then_star qs rest hq]

/-- A declarator with no `*` is read exactly as before. -/
theorem a_declarator_without_a_star_is_unchanged (rev : List Word)
    (h : hasStar rev = false) : projectParameter rev = project rev := by
  unfold projectParameter dropObjectQualifiers
  cases hs : stripQualifiers rev with
  | nil => rfl
  | cons v vs =>
      cases v with
      | star => exact absurd hs (strip_of_no_star h vs)
      | const => rfl
      | qualifier => rfl
      | specifier => rfl

/-- Reading twice decides nothing new. -/
theorem drop_is_idempotent (rev : List Word) :
    dropObjectQualifiers (dropObjectQualifiers rev) = dropObjectQualifiers rev := by
  unfold dropObjectQualifiers
  cases hs : stripQualifiers rev with
  | nil => simp [hs]
  | cons v vs =>
      cases v with
      | star => simp [stripQualifiers, isQualifier]
      | const => simp [hs]
      | qualifier => simp [hs]
      | specifier => simp [hs]

/-! ### Exactly one position is widened -/

/-- A qualifier *between* two `*` qualifies an inner pointer, which this
vocabulary cannot spell: still refused, whatever follows the last `*`. -/
theorem a_qualifier_between_two_stars_still_refuses (qs rest : List Word) {q : Word}
    (hqs : ∀ w ∈ qs, isQualifier w = true) (hq : isQualifier q = true)
    (hrest : hasStar rest = true) :
    projectParameter (qs ++ .star :: q :: rest) = none := by
  rw [the_widening_reads_the_unqualified_type qs (q :: rest) hqs]
  have hc : countStars (.star :: q :: rest) = (1, q :: rest) := by
    cases q <;> simp_all [isQualifier, countStars]
  unfold project
  rw [hc]
  simp [hasStar, hrest]

/-! ### Witnesses -/

/-- glibc's `const char *__restrict __format`, name dropped, reversed. -/
def glibcFormat : List Word := [.qualifier, .star, .specifier, .const]

/-- It was unreadable: the measured `abi-unreadable-declaration`. -/
theorem the_glibc_format_parameter_was_unreadable : project glibcFormat = none := by
  decide

/-- And it is now `ptr<const, u8>`: one level over a `const` pointee. -/
theorem the_glibc_format_parameter_is_now_read :
    projectParameter glibcFormat = some (1, true) := by
  decide

/-- The widening is strict. -/
theorem the_widening_is_strict :
    ∃ rev, project rev = none ∧ projectParameter rev ≠ none :=
  ⟨glibcFormat, by decide, by decide⟩

/-- The pointee's constness survives: dropping the object's qualifier does not
turn `const char *__restrict` into `ptr<mut, u8>`. -/
theorem the_pointee_constness_is_kept :
    projectParameter glibcFormat ≠ some (1, false) := by
  decide

/-- `void *const p`: a constant pointer to mutable bytes is `ptr<mut, u8>`. -/
theorem a_const_pointer_to_mutable_bytes_is_mutable :
    projectParameter [.const, .star, .specifier] = some (1, false) := by
  decide

/-- `void *__restrict *__restrict p`: refused, as the Rust table pins. -/
theorem a_restrict_between_two_stars_refuses :
    projectParameter [.qualifier, .star, .qualifier, .star, .specifier] = none := by
  decide

/-! ## Array bounds (card W43-LIBC-RESTRICT-PROTOTYPES)

C11 6.7.6.3p7 adjusts a parameter declared "array of `T`" to "qualified pointer
to `T`, where the type qualifiers (if any) are those specified within the `[`
and `]`", and 6.7.6.3p15 then drops those qualifiers. So `T a[restrict n]`
is `T *restrict a`, which is `T *a`. glibc's `regexec` spells its match array
`regmatch_t __pmatch[__restrict __nmatch]` after preprocessing, and the reader
refused every bound but an empty or a plain decimal one.

The Rust (`array_bound_is_read`, `project_parameter`) now reads a bound that is
a run of qualifiers followed by at most one size word. The element declarator
(the words before the `[`, parameter name dropped) is then read with one more
`*` and **without** `dropObjectQualifiers`: a qualifier at the end of the
element — `char *const p[]` — qualifies the inner pointer of `char *const *`.
The card W43-LIBC-RESULT-ALIAS reader applied `dropObjectQualifiers` to the
element before adding the star, which read that shape as a plain `char **`;
this card narrows it back to the refusal the pointer spelling gets. Both
directions are stated below. -/

/-- One word inside an array parameter's `[` `]`. -/
inductive BoundWord where
  | qualifier
  | constant
  | name
  | staticKw
  | star
  | other
  deriving DecidableEq, Repr

def isBoundQualifier : BoundWord → Bool
  | .qualifier => true
  | _ => false

/-- A size word the reader accepts: a decimal constant or an identifier. -/
def isSizeWord : BoundWord → Bool
  | .constant => true
  | .name => true
  | _ => false

/-- Drop the leading run of qualifiers. -/
def dropBoundQualifiers : List BoundWord → List BoundWord
  | [] => []
  | w :: ws => if isBoundQualifier w then dropBoundQualifiers ws else w :: ws

/-- The bounds the reader read before this card: empty, or one decimal
constant. -/
def earlierBoundRead : List BoundWord → Bool
  | [] => true
  | [.constant] => true
  | _ => false

/-- `array_bound_is_read`. -/
def boundRead (b : List BoundWord) : Bool :=
  match dropBoundQualifiers b with
  | [] => true
  | [w] => isSizeWord w
  | _ => false

/-- The W43-LIBC-RESULT-ALIAS reader of an array parameter: element declarator
(reversed), qualifiers after its last `*` dropped, then one more `*`. -/
def projectArrayBefore (elem : List Word) (b : List BoundWord) : Option (Nat × Bool) :=
  if earlierBoundRead b then project (.star :: dropObjectQualifiers elem) else none

/-- The reader after this card. -/
def projectArray (elem : List Word) (b : List BoundWord) : Option (Nat × Bool) :=
  if boundRead b then project (.star :: elem) else none

/-- The bound's qualifiers, as the qualifiers after the adjusted pointer's `*`. -/
def boundQualifiers (b : List BoundWord) : List Word :=
  (b.takeWhile isBoundQualifier).map (fun _ => Word.qualifier)

theorem boundQualifiers_are_qualifiers (b : List BoundWord) :
    ∀ w ∈ boundQualifiers b, isQualifier w = true := by
  intro w hw
  unfold boundQualifiers at hw
  simp only [List.mem_map] at hw
  obtain ⟨_, _, rfl⟩ := hw
  rfl

/-- **C11 6.7.6.3p7 then p15, as an equation.** A read array parameter is
read exactly as the pointer parameter carrying the bound's qualifiers after its
`*`, which is read as the unqualified pointer. -/
theorem the_array_parameter_is_the_adjusted_pointer (elem : List Word) (b : List BoundWord)
    (h : boundRead b = true) :
    projectArray elem b = projectParameter (boundQualifiers b ++ .star :: elem) := by
  rw [the_widening_reads_the_unqualified_type _ _ (boundQualifiers_are_qualifiers b)]
  simp [projectArray, h]

/-- Every bound read before is read now. -/
theorem a_bound_read_before_is_read_now (b : List BoundWord)
    (h : earlierBoundRead b = true) : boundRead b = true := by
  cases b with
  | nil => rfl
  | cons w ws =>
      cases w <;> cases ws <;>
        simp_all [earlierBoundRead, boundRead, dropBoundQualifiers, isBoundQualifier, isSizeWord]

/-- **Conservativity where the element has nothing to drop**: for an element
declarator `dropObjectQualifiers` leaves unchanged — every element in the
pinned tables and every measured glibc prototype — any array parameter read
before is read identically. -/
theorem the_array_widening_only_adds (elem : List Word) (b : List BoundWord)
    (helem : dropObjectQualifiers elem = elem) {x : Nat × Bool}
    (h : projectArrayBefore elem b = some x) : projectArray elem b = some x := by
  unfold projectArrayBefore at h
  cases hb : earlierBoundRead b with
  | false => simp [hb] at h
  | true =>
      simp only [hb, helem, ite_true] at h
      simp [projectArray, a_bound_read_before_is_read_now b hb, h]

theorem drop_changes_only_through_a_star {elem : List Word}
    (h : dropObjectQualifiers elem ≠ elem) : ∃ rest, stripQualifiers elem = .star :: rest := by
  unfold dropObjectQualifiers at h
  cases hs : stripQualifiers elem with
  | nil => simp [hs] at h
  | cons v vs =>
      cases v with
      | star => exact ⟨vs, rfl⟩
      | const => simp [hs] at h
      | qualifier => simp [hs] at h
      | specifier => simp [hs] at h

theorem head_qualifier_of_drop_changes {elem : List Word}
    (h : dropObjectQualifiers elem ≠ elem) :
    ∃ q ws, elem = q :: ws ∧ isQualifier q = true ∧ hasStar elem = true := by
  obtain ⟨rest, hs⟩ := drop_changes_only_through_a_star h
  have hstar := hasStar_of_strip_star hs
  cases elem with
  | nil => simp [stripQualifiers] at hs
  | cons q ws =>
      cases hq : isQualifier q with
      | true => exact ⟨q, ws, rfl, hq, hstar⟩
      | false =>
          exfalso
          apply h
          unfold dropObjectQualifiers
          rw [stripQualifiers_of_not_qualifier hq]
          cases q <;> first | rfl | simp [isQualifier] at hq

/-- **The narrowing, and exactly where it is.** Where the earlier reader's
`dropObjectQualifiers` changed the element, the element ends in a qualifier
after a `*` (`char *const p[]`), and the array parameter now refuses — the
same refusal the pointer spelling `char *const *p` gets. -/
theorem an_array_of_qualified_pointers_refuses (elem : List Word) (b : List BoundWord)
    (h : dropObjectQualifiers elem ≠ elem) : projectArray elem b = none := by
  obtain ⟨q, ws, rfl, hq, hstar⟩ := head_qualifier_of_drop_changes h
  have hc : countStars (.star :: q :: ws) = (1, q :: ws) := by
    cases q <;> simp_all [isQualifier, countStars]
  unfold projectArray
  cases boundRead b <;> simp [project, hc, hstar]

/-- And that refusal agrees with the pointer spelling of the same type. -/
theorem the_array_and_pointer_spellings_agree_on_it :
    projectArray [.const, .star, .specifier] [] =
      projectParameter [.star, .const, .star, .specifier] := by
  decide

/-- The earlier reader read that shape as a plain two-level pointer. -/
theorem the_earlier_reader_read_it_as_two_levels :
    projectArrayBefore [.const, .star, .specifier] [] = some (2, false) := by
  decide

/-! ### What still refuses -/

theorem a_static_bound_still_refuses : boundRead [.staticKw, .constant] = false := by decide

theorem a_star_bound_still_refuses : boundRead [.star] = false := by decide

theorem a_qualifier_after_the_size_refuses : boundRead [.constant, .qualifier] = false := by
  decide

theorem a_multi_word_size_refuses : boundRead [.qualifier, .name, .other, .constant] = false := by
  decide

/-! ### The measured witness -/

/-- `regmatch_t __pmatch[__restrict __nmatch]`: the element is one type word
(`regmatch_t`, an aggregate typedef), name dropped. -/
def regexecMatch : List Word := [.specifier]

def regexecBound : List BoundWord := [.qualifier, .name]

theorem the_regexec_match_array_was_unreadable :
    projectArrayBefore regexecMatch regexecBound = none := by
  decide

theorem the_regexec_match_array_is_now_read :
    projectArray regexecMatch regexecBound = some (1, false) := by
  decide

/-- The element's constness survives the adjustment:
`const char p[__restrict]` is `ptr<const, u8>`. -/
theorem an_array_element_constness_is_kept :
    projectArray [.specifier, .const] [.qualifier] = some (1, true) := by
  decide

end QCore.PointerQualifierReader
