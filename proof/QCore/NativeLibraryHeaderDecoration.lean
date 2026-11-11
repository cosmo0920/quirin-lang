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

import QCore.NativeLibraryHeader

/-! Card `NATIVE-LIBRARY-HEADER-DECORATION`: making a **decorated** exported
header readable without creating an unchecked premise about its text.

Card `NATIVE-LIBRARY-HEADER`'s census measured **0 of 423** Libevent
declarations inside its subset, for one reason: every exported declaration is
written `EVENT2_EXPORT_SYMBOL struct event_base *event_base_new(void);`, and
that macro's `#define` lives in a header the reader did not follow. The obvious
repair — a manifest field listing the names to erase — was named there and
declined, because it *converts a reader boundary into an unchecked premise about
the header's own text*, which is precisely what that level exists to remove.

This module models the derived route. The split it rests on is the whole card:

* a manifest declares **which hash-identified texts may be read**, and says
  nothing whatever about what they contain;
* the reader **derives** from those texts' own `#define`s which names are
  erasable declaration decorations, evaluating **no condition**;
* the vocabulary of erasable bodies is the reader's own, fixed and published.

What is established here:

* **a group with no `#else` never establishes definedness**, and neither does one
  with an arm that does not define — so a derivation is never taken on a
  configuration the reader cannot see;
* **every definition must be erasable for the name to be**, and a
  function-like definition or an `#undef` anywhere withdraws it;
* **erasing the name is expanding it**: the erasure vocabulary deletes a
  decoration whether it arrived as a derived macro name or was written out in
  full, so the two readings are equal;
* **a name not derived is not erased**, so the declaration keeps the refusal
  card `NATIVE-LIBRARY-HEADER` already gave it — the derivation can only widen
  the *readable* set, never the *admitted* one;
* **the texts read are exactly the declared ones**: an undeclared text refuses,
  a declared text whose digest differs refuses, a declared text nothing reaches
  refuses, and an `#include` nobody declared is not followed;
* **declaring no text is card `NATIVE-LIBRARY-HEADER`'s reading**, unchanged.

What is **not** established, and is stated rather than smoothed over: that each
name in the erasable vocabulary really is a decoration contributing no type, no
parameter and no linkage name is a reading of **C**, published in
`ERASABLE_ATTRIBUTES` and auditable there, not a theorem. It is the same kind of
commitment as `HEADER-STDINT-PRELUDE`'s reading of C11 7.20, and the model takes
it as the classification of a token rather than deriving it. The concrete
lexical layer — splicing, guard removal, tokenising — is a Rust boundary tested
there. -/

namespace QCore.NativeLibraryHeaderDecoration

/-! ## The token stream, classified

A token is *code* or *decoration*. "Decoration" is the reader's published
vocabulary: an attribute list every one of whose names is in
`ERASABLE_ATTRIBUTES`, a `__declspec` of an export/import, the bare `__global`
keyword, or a macro name the derivation below established. Which real C tokens
fall in that class is the reading recorded in the Rust module; what this model
is about is what follows *once* a token is in it. -/

inductive Tok where
  /-- A token that contributes to the declaration: a specifier, a declarator
  piece, a name. -/
  | code (spelling : String)
  /-- A token this reader erases, of either origin. -/
  | decoration (spelling : String)
  deriving DecidableEq, Repr

/-- Whether a token survives erasure. -/
def survives : Tok → Bool
  | .code _ => true
  | .decoration _ => false

/-- The reading of a token stream: the decorations are gone and nothing else
moved. -/
def erase (ts : List Tok) : List Tok := ts.filter survives

theorem erase_append (xs ys : List Tok) : erase (xs ++ ys) = erase xs ++ erase ys := by
  simp [erase, List.filter_append]

/-- **Erasure deletes exactly the decorations.** -/
theorem erase_keeps_exactly_the_code (ts : List Tok) (t : Tok) :
    t ∈ erase ts ↔ (t ∈ ts ∧ survives t = true) := by
  simp [erase, List.mem_filter]

/-- A body every token of which this reader erases. -/
def erasableBody (b : List Tok) : Bool := b.all (fun t => !survives t)

theorem an_empty_body_is_erasable : erasableBody [] = true := by
  simp [erasableBody]

/-- **Erasing a decoration's name is expanding it.**

The derivation's whole justification, and it is an equality rather than an
appeal: a body all of whose tokens are erased contributes nothing to the reading,
so splicing it in where the macro name stood leaves the reading exactly where
deleting the name does. The Rust side makes this literal by erasing the written
form and the derived name with the **one** vocabulary. -/
theorem expanding_then_erasing_equals_erasing
    (xs b ys : List Tok) (h : erasableBody b = true) :
    erase (xs ++ b ++ ys) = erase (xs ++ ys) := by
  have hb : erase b = [] := by
    simp [erase, erasableBody, List.all_eq_true] at h ⊢
    exact fun t ht => by simpa using h t ht
  simp [erase_append, hb]

/-- The same statement with the macro name in place, which is the form the
reader actually takes: a derived name is a decoration token. -/
theorem erasing_the_name_is_expanding_it
    (xs b ys : List Tok) (name : String) (h : erasableBody b = true) :
    erase (xs ++ [Tok.decoration name] ++ ys) = erase (xs ++ b ++ ys) := by
  rw [expanding_then_erasing_equals_erasing xs b ys h]
  simp [erase, survives]

/-- **A name the derivation does not establish is not erased.** Its token is
`code`, so it stands in the specifier prefix and the declaration refuses exactly
as it did before this card — the derivation can only widen what is *readable*,
never what is *admitted*. -/
theorem an_underived_name_survives (xs ys : List Tok) (name : String) :
    erase (xs ++ [Tok.code name] ++ ys) = erase xs ++ [Tok.code name] ++ erase ys := by
  simp [erase, List.filter_append, List.filter_cons, survives]

/-! ## The conditional structure, and definedness on every path

No condition is evaluated and none is guessed. A region is a tree of directive
groups; a group establishes definedness only when it has an `#else` — so that no
implicit empty arm escapes — and every one of its arms establishes it. -/

mutual

/-- One region of a spliced text, at the granularity the derivation reads. -/
inductive Region where
  /-- `#define NAME …`, with whether it is function-like and whether its body is
  erasable. -/
  | define (name : String) (functionLike : Bool) (erasable : Bool)
  /-- `#undef NAME`. -/
  | undef (name : String)
  /-- C tokens, which establish nothing about a macro. -/
  | text
  /-- One region after another. -/
  | seq (left right : Region)
  /-- `#if … #elif … #else … #endif`, with its arms and whether it wrote an
  `#else`. -/
  | group (arms : Arms) (hasElse : Bool)
  deriving Repr

/-- The arms of one conditional group. -/
inductive Arms where
  | nil
  | cons (head : Region) (tail : Arms)
  deriving Repr

end

mutual

/-- Whether `name` is defined on **every** path through this region. -/
def definedOnEveryPath : Region → String → Bool
  | .define n _ _, name => n == name
  | .undef _, _ => false
  | .text, _ => false
  | .seq l r, name => definedOnEveryPath l name || definedOnEveryPath r name
  | .group arms hasElse, name => hasElse && everyArmDefines arms name

/-- Whether every arm of a group defines `name` on every path through it. -/
def everyArmDefines : Arms → String → Bool
  | .nil, _ => true
  | .cons head tail, name => definedOnEveryPath head name && everyArmDefines tail name

end

mutual

/-- Whether every `#define` of `name` in this region is object-like and has an
erasable body. A region defining it nowhere satisfies this vacuously; the
definedness predicate above is what stops that being enough. -/
def everyDefinitionErasable : Region → String → Bool
  | .define n functionLike erasable, name =>
      if n == name then !functionLike && erasable else true
  | .undef _, _ => true
  | .text, _ => true
  | .seq l r, name => everyDefinitionErasable l name && everyDefinitionErasable r name
  | .group arms _, name => everyArmDefinitionErasable arms name

def everyArmDefinitionErasable : Arms → String → Bool
  | .nil, _ => true
  | .cons head tail, name =>
      everyDefinitionErasable head name && everyArmDefinitionErasable tail name

end

mutual

/-- Whether `name` is `#undef`ed anywhere. This reader tracks no scope for a
macro, so the conservative reading is the refusing one. -/
def everUndefined : Region → String → Bool
  | .define _ _ _, _ => false
  | .undef n, name => n == name
  | .text, _ => false
  | .seq l r, name => everUndefined l name || everUndefined r name
  | .group arms _, name => someArmUndefines arms name

def someArmUndefines : Arms → String → Bool
  | .nil, _ => false
  | .cons head tail, name => everUndefined head name || someArmUndefines tail name

end

/-- **The derivation.** A name is an erasable declaration decoration exactly
when every definition of it is object-like with an erasable body, it is never
`#undef`ed, and it is defined on every path through the text preceding its first
use.

Every one of the three is read out of the supplied texts. None of them is a
manifest statement, and none of them evaluates a condition. -/
def derivable (r : Region) (name : String) : Bool :=
  everyDefinitionErasable r name && !everUndefined r name && definedOnEveryPath r name

/-! ### What the derivation refuses -/

/-- **A group with no `#else` never establishes definedness.** The arm where no
condition holds is implicit and defines nothing, and the reader will not supply
it from a configuration it cannot see. -/
theorem a_group_with_no_else_establishes_nothing (arms : Arms) (name : String) :
    definedOnEveryPath (.group arms false) name = false := by
  simp [definedOnEveryPath]

theorem a_group_with_no_else_is_never_derivable (arms : Arms) (name : String) :
    derivable (.group arms false) name = false := by
  simp [derivable, definedOnEveryPath]

/-- **An arm that does not define withdraws the whole group**, wherever it
stands, because the reader decides no condition and so cannot know which arm a
consumer takes. -/
theorem an_arm_that_does_not_define_withdraws_the_group
    (head : Region) (tail : Arms) (hasElse : Bool) (name : String)
    (h : definedOnEveryPath head name = false) :
    definedOnEveryPath (.group (.cons head tail) hasElse) name = false := by
  simp [definedOnEveryPath, everyArmDefines, h]

/-- **A function-like definition is never derivable.** Its expansion depends on
its arguments at each use site, and this reader evaluates no expansion. -/
theorem a_function_like_definition_is_not_derivable (name : String) (erasable : Bool) :
    derivable (.define name true erasable) name = false := by
  simp [derivable, everyDefinitionErasable]

/-- **A body outside the vocabulary is never derivable.** -/
theorem an_unerasable_body_is_not_derivable (name : String) :
    derivable (.define name false false) name = false := by
  simp [derivable, everyDefinitionErasable]

/-- **One unerasable definition withdraws the derivation**, even beside
definitions that are erasable — which is the shape a real visibility header has,
one name defined once per conditional arm. -/
theorem one_unerasable_definition_withdraws_the_derivation
    (left right : Region) (name : String)
    (h : everyDefinitionErasable left name = false) :
    derivable (.seq left right) name = false := by
  simp [derivable, everyDefinitionErasable, h]

/-- **An `#undef` anywhere withdraws the derivation.** -/
theorem an_undefined_name_is_not_derivable (left right : Region) (name : String)
    (h : everUndefined left name = true) :
    derivable (.seq left right) name = false := by
  simp [derivable, everUndefined, h]

/-- **The derivation is not vacuous.** The real shape — one name, defined in
every arm of a group that writes an `#else`, each body erasable — is derived. -/
theorem the_real_visibility_shape_is_derivable (name : String) :
    derivable
      (.group (.cons (.define name false true) (.cons (.define name false true) .nil)) true)
      name = true := by
  simp [derivable, definedOnEveryPath, everyArmDefines, everyDefinitionErasable,
    everyArmDefinitionErasable, everUndefined, someArmUndefines]

/-- **And it is a function of the text alone.** Two readings of the same region
derive the same names; there is no manifest input to this decision, which is the
whole difference between this card and the declared erasure list card
`NATIVE-LIBRARY-HEADER` declined. -/
theorem the_derivation_is_a_function_of_the_text
    (r r' : Region) (name : String) (h : r = r') :
    derivable r name = derivable r' name := by
  rw [h]

/-! ## The declared texts

A manifest names texts; it says nothing about them. The reading is total over
the declared set in both directions: a declared text must be supplied, must
measure the declared digest, and must be reached; and once a header is bound, a
text nobody declared is refused rather than read. An `#include` the manifest does
not declare is **not** followed, which is card `NATIVE-LIBRARY-HEADER`'s
behaviour unchanged and is what keeps the level usable on a header that includes
`<stdint.h>`. -/

/-- One declared text: the `#include` operand it stands for, the reference it is
read from, and its digest. -/
structure IncludeIdentity where
  spelling : String
  reference : String
  digest : String
  deriving DecidableEq, Repr

/-- Why a declared-text binding does not load. The vocabulary is closed. -/
inductive IncludeRefusal where
  | notSupplied
  | digestMismatch
  | notReached
  | sourceNotDeclared
  deriving DecidableEq, BEq, Repr

def allIncludeRefusals : List IncludeRefusal :=
  [.notSupplied, .digestMismatch, .notReached, .sourceNotDeclared]

/-- One supplied text: the reference it arrived under and the digest measured
over its bytes. -/
structure Supplied where
  reference : String
  measuredDigest : String
  deriving DecidableEq, Repr

/-- Resolve one declared text against what was supplied, or refuse by name. -/
def resolve (declared : IncludeIdentity) (sources : List Supplied) :
    Except IncludeRefusal Supplied :=
  match sources.find? (fun s => s.reference == declared.reference) with
  | none => .error .notSupplied
  | some supplied =>
      if supplied.measuredDigest == declared.digest then .ok supplied
      else .error .digestMismatch

/-- The whole binding: every declared text resolves, and every one is reached by
an `#include` of its spelling. -/
def bindIncludes (declared : List IncludeIdentity) (sources : List Supplied)
    (reached : List String) : Except IncludeRefusal Unit :=
  match declared with
  | [] => .ok ()
  | d :: rest =>
      match resolve d sources with
      | .error e => .error e
      | .ok _ =>
          if reached.contains d.spelling then bindIncludes rest sources reached
          else .error .notReached

/-- **A declared text that is not supplied refuses.** -/
theorem an_unsupplied_declared_text_refuses
    (d : IncludeIdentity) (rest : List IncludeIdentity) (sources : List Supplied)
    (reached : List String)
    (h : sources.find? (fun s => s.reference == d.reference) = none) :
    bindIncludes (d :: rest) sources reached = .error .notSupplied := by
  simp [bindIncludes, resolve, h]

/-- **A declared text whose digest is not the declared one refuses**, and it
refuses *before* the text is read — so a substituted text that would have been
read the same way is still refused. That is the whole content of verifying an
identity, and it is card `NATIVE-LIBRARY-HEADER` §3's argument applied to a
second kind of text. -/
theorem a_substituted_declared_text_refuses
    (d : IncludeIdentity) (rest : List IncludeIdentity) (sources : List Supplied)
    (reached : List String) (supplied : Supplied)
    (hfind : sources.find? (fun s => s.reference == d.reference) = some supplied)
    (hdigest : supplied.measuredDigest ≠ d.digest) :
    bindIncludes (d :: rest) sources reached = .error .digestMismatch := by
  simp [bindIncludes, resolve, hfind, beq_iff_eq, hdigest]

/-- **A declared text no `#include` reaches refuses.** A declared text nothing
reads is a statement with no effect, and this level's content is that its
statements have effects. -/
theorem an_unreached_declared_text_refuses
    (d : IncludeIdentity) (rest : List IncludeIdentity) (sources : List Supplied)
    (reached : List String) (supplied : Supplied)
    (hfind : sources.find? (fun s => s.reference == d.reference) = some supplied)
    (hdigest : supplied.measuredDigest = d.digest)
    (hreach : d.spelling ∉ reached) :
    bindIncludes (d :: rest) sources reached = .error .notReached := by
  simp [bindIncludes, resolve, hfind, hdigest, hreach]

/-- Whether every supplied text is one some manifest declares. Gated on a header
being bound, so a selection naming none is unchanged. -/
def everySourceDeclared (sources : List Supplied) (declared : List String) : Bool :=
  sources.all (fun s => declared.contains s.reference)

/-- **A text supplied for a reference nobody declares refuses.** The texts a
decision rests on are exactly the declared ones, so an extra one is refused
rather than dropped on the floor. -/
theorem an_undeclared_source_refuses
    (s : Supplied) (rest : List Supplied) (declared : List String)
    (h : s.reference ∉ declared) :
    everySourceDeclared (s :: rest) declared = false := by
  simp [everySourceDeclared, h]

/-- **Declaring no text is card `NATIVE-LIBRARY-HEADER`'s reading.** The
binding succeeds over any supplied texts and any reached spellings, so nothing
about the wave-22 world moves. -/
theorem declaring_no_text_binds_trivially (sources : List Supplied) (reached : List String) :
    bindIncludes [] sources reached = .ok () := by
  simp [bindIncludes]

/-- And the accepted configuration is reachable: one declared text, supplied at
its declared digest and reached. -/
theorem a_declared_supplied_and_reached_text_binds
    (d : IncludeIdentity) (sources : List Supplied) (reached : List String)
    (supplied : Supplied)
    (hfind : sources.find? (fun s => s.reference == d.reference) = some supplied)
    (hdigest : supplied.measuredDigest = d.digest)
    (hreach : d.spelling ∈ reached) :
    bindIncludes [d] sources reached = .ok () := by
  simp [bindIncludes, resolve, hfind, hdigest, hreach]

/-! ## The level is still only able to refuse more about a *declaration*

The derivation widens what this reader can **read**. It does not widen what the
comparison **admits**: a read declaration is compared against the manifest's
transcription by card `NATIVE-LIBRARY-HEADER`'s unchanged relation, and a
disagreement still refuses. The statement below is that the erasure cannot turn
a disagreement into an agreement, because it deletes no token the comparison
reads. -/

/-- **Erasure deletes no code token**, so every token the comparison reads is
still there and in order. -/
theorem erasure_preserves_every_code_token (ts : List Tok) (spelling : String) :
    (Tok.code spelling ∈ erase ts) ↔ (Tok.code spelling ∈ ts) := by
  simp [erase, List.mem_filter, survives]

/-- **Erasure is idempotent**: reading a stream that has already been erased
changes nothing, so the decision does not depend on how many times the
vocabulary was applied. -/
theorem erasure_is_idempotent (ts : List Tok) : erase (erase ts) = erase ts := by
  simp [erase, List.filter_filter, Bool.and_self]

end QCore.NativeLibraryHeaderDecoration
