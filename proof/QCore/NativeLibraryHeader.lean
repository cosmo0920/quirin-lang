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

import QCore.NativeLibraryContract
import QCore.TypedefProjection

/-! Card `NATIVE-LIBRARY-HEADER`: the **exported-header level** of
`source.native-library.v1` — reading the library's real `.h` instead of
believing the manifest's transcription of it.

Cards `UNSAFE-LINKAGE-ARTIFACT` and `NATIVE-LIBRARY-CONTRACT-SLICE` built a
manifest whose entries carry the exported header declaration *as the author
transcribed it*, and nothing compared that transcription against the header the
library ships. Since wave 20 it is a transcription that **pointers** rest on.
This module models what a *bound* header means.

The agreement relation is **imported**, not restated:
`QCore.DeclaredAbiAgreement.admits` is the relation card LOADER-ABI's gateway
reader compares with, and `the_agreement_relation_is_the_declared_abi_one` says
in the model what the crate layering stops the Rust from saying by sharing code
— `qcore-core` has no dependencies at all, and the gateway's reader sits above
the contract loader, so this card's reader is necessarily a second
implementation of one projection. The typedef targets are
`QCore.TypedefProjection`'s, likewise imported, so the two shapes cards
`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` and `ABI-TYPEDEF-ADDRESS-OF-MEMBER` taught
that reader are the same two shapes here.

What is established:

* **the level is opt-in and cannot reach a manifest that did not opt in** — a
  manifest naming no header decides alike whatever text is supplied;
* **the totality is one-directional** — a bound symbol the header does not
  declare refuses, and a declaration the manifest does not name changes nothing
  (stated over *arbitrary* readings of the unnamed symbols, so it is a claim
  about the gate rather than about a particular header);
* **a disagreement refuses and never withdraws** — the weaker admissible reading
  is the manifest without a header, exhibited beside the refusal so the claim is
  not a statement about an empty set;
* **binding a header only removes admissions**, with a witness that it removes
  at least one, so monotonicity is not vacuous;
* **the digest gate decides before the text is read**, so a substituted header
  that happens to agree is still refused;
* **read and unspellable are different facts** — `fnptr`, `f32` and `f64` are
  §C04.1 spellings a manifest entry cannot write, and collapsing them into
  "unreadable" would report a vocabulary gap as a reader gap;
* **the `HEADER-STDINT-PRELUDE` is C11's own reading**, and every name in it is
  spellable in the manifest vocabulary.

What is **not** established, here or anywhere: that the supplied text is the
header the reference names, that the library conforms to its declarations, or
that the linker used the artifact. Those are premises. The manifest's concrete
syntax and its total-or-reject text reader are a Rust boundary and are tested
there; what is modelled here is what a *read* header means. -/

namespace QCore.NativeLibraryHeader

open QCore.DeclaredAbiAgreement (AbiType Projection admits)
open QCore.NativeLibrary (HeaderIdentity)

/-! ## The manifest's vocabulary inside the loader's -/

/-- Whether a loader spelling is one a manifest entry can write.

The manifest's ABI table is `qcore_core::extern_call::AbiType`, which §C04.1's
`fnptr`, `f32` and `f64` are **not** in. A header type that projects to one of
those is *read*; what is missing is a manifest spelling for it.

Card W44-LIBC-RESIDUALS added `va_list` to §C04.1's parameter vocabulary. The
manifest table has no spelling for it either, so it is the fourth entry here.
(This header reader has no builtin arm and never produces it; the entry keeps
the model's vocabulary equal to the Rust table rather than calling `va_list`
spellable by default.) -/
def manifestSpellable : AbiType → Bool
  | .f32 | .f64 | .fnptr | .vaList => false
  | _ => true

/-- The four spellings §C04.1 has and a manifest entry does not. -/
def unspellableSpellings : List AbiType := [.f32, .f64, .fnptr, .vaList]

theorem the_manifest_vocabulary_excludes_exactly_the_unspellable_spellings (t : AbiType) :
    manifestSpellable t = false ↔ t ∈ unspellableSpellings := by
  cases t <;> simp [manifestSpellable, unspellableSpellings]

/-- A projection every one of whose readings is outside the manifest
vocabulary. `none` — unreadable — is deliberately not this: the two facts have
different follow-ons and are reported apart. -/
def isUnspellable (p : Projection) : Bool :=
  match p with
  | none => false
  | some ts => !ts.isEmpty && ts.all (fun t => !manifestSpellable t)

/-! ## What a read header says about one symbol -/

/-- One declaration this reader committed to the shape of. -/
structure Declaration where
  result : Projection
  parameters : List Projection
  /-- The declaration ends in `...`. -/
  variadic : Bool
  /-- The parameter list is empty, which specifies nothing (C11 6.7.6.3p14). -/
  unspecified : Bool
  /-- The declaration carries `static`, so the symbol has internal linkage and is
  not an exported symbol of the artifact at all. -/
  internal : Bool
  deriving DecidableEq, Repr

/-- What the reader found for one named symbol. Five constructors and no
catch-all: an absent symbol, a conditional-only one and a macro say different
things about the header, and each is reported as itself. -/
inductive Reading where
  | declared (d : Declaration)
  | conditionalOnly
  | macroDefined
  | ambiguous
  | absent
  deriving DecidableEq, Repr

/-- One manifest entry, at the part this gate reads: the symbol and the
declaration the manifest transcribed for it. Both entry kinds are compared
identically — what a manifest transcribes is an exported header declaration
whichever keyword introduced it. -/
structure Entry where
  symbol : String
  parameters : List AbiType
  result : AbiType
  deriving DecidableEq, Repr

/-! ## The named refusals -/

/-- Why a header binding does not load. The vocabulary is closed. -/
inductive Refusal where
  | notSupplied
  | digestMismatch
  | undeclaredSymbol
  | conditionalDeclaration
  | macroSymbol
  | internalDeclaration
  | unspecifiedParameters
  | variadicDeclaration
  | ambiguousDeclaration
  | unreadablePosition
  | unspellablePosition
  deriving DecidableEq, BEq, Repr

def allRefusals : List Refusal :=
  [.notSupplied, .digestMismatch, .undeclaredSymbol, .conditionalDeclaration,
   .macroSymbol, .internalDeclaration, .unspecifiedParameters,
   .variadicDeclaration, .ambiguousDeclaration, .unreadablePosition,
   .unspellablePosition]

/-- What a header binding can conclude. A **refusal** says this level could not
be applied; a **disagreement** says it was applied and the manifest lost. Both
refuse the manifest, and they are apart because they are different diagnostics
about different things. -/
inductive Failure where
  | refused (r : Refusal)
  | disagreement
  deriving DecidableEq, Repr

/-! ## The comparison -/

/-- One type position: the header's projection against the manifest's spelling.

The order is the implementation's — unreadable, then unspellable, then the
agreement relation — so a position that is not read at all is never reported as
a disagreement about a reading it does not have. -/
def positionOutcome (p : Projection) (t : AbiType) : Option Failure :=
  if p.isNone then some (.refused .unreadablePosition)
  else if isUnspellable p then some (.refused .unspellablePosition)
  else if admits p t then none
  else some .disagreement

/-- **The agreement relation is the one `QCore.DeclaredAbiAgreement` models.**

Not a comment: the two readers are separated by the crate layering, and this is
the statement that they compare by one relation rather than two that could
drift. A position agrees exactly when the imported `admits` holds of it. -/
theorem the_agreement_relation_is_the_declared_abi_one (p : Projection) (t : AbiType)
    (readable : p.isNone = false) (spellable : isUnspellable p = false) :
    positionOutcome p t = none ↔ admits p t = true := by
  simp [positionOutcome, readable, spellable]

/-- And where it does not hold, the outcome is a disagreement rather than a
refusal: the position was read, in a vocabulary the manifest can write, and the
two readings differ. -/
theorem a_read_position_that_does_not_admit_disagrees (p : Projection) (t : AbiType)
    (readable : p.isNone = false) (spellable : isUnspellable p = false)
    (differs : admits p t = false) :
    positionOutcome p t = some .disagreement := by
  simp [positionOutcome, readable, spellable, differs]

/-- Compare the parameter vectors position by position. -/
def compareParameters : List AbiType → List Projection → Option Failure
  | [], [] => none
  | t :: ts, p :: ps =>
      match positionOutcome p t with
      | some failure => some failure
      | none => compareParameters ts ps
  | _, _ => some .disagreement

/-- The gate for one bound symbol.

Shape facts first — a symbol that is not an exported declaration at all is not
compared against anything — then arity, then the parameters in order, then the
result. -/
def checkReading (e : Entry) (r : Reading) : Option Failure :=
  match r with
  | .absent => some (.refused .undeclaredSymbol)
  | .conditionalOnly => some (.refused .conditionalDeclaration)
  | .macroDefined => some (.refused .macroSymbol)
  | .ambiguous => some (.refused .ambiguousDeclaration)
  | .declared d =>
      if d.internal then some (.refused .internalDeclaration)
      else if d.variadic then some (.refused .variadicDeclaration)
      else if d.unspecified then some (.refused .unspecifiedParameters)
      else if e.parameters.length ≠ d.parameters.length then some .disagreement
      else
        match compareParameters e.parameters d.parameters with
        | some failure => some failure
        | none => positionOutcome d.result e.result

/-! ## The binding -/

/-- One supplied header text, as this gate reads it: the reference it was
resolved from, the digest **measured over it**, and what it declares for each
symbol.

The reading is a total function of the symbol because the Rust reader returns a
row for every symbol it was asked about, including `absent`. -/
structure Source where
  reference : String
  digest : String
  reading : String → Reading

/-- The whole header binding of one manifest.

`header?` is `none` for a manifest that names no header — every manifest of the
two cards below this one — and that case decides `none` whatever is supplied. -/
def checkBinding (header? : Option HeaderIdentity) (source? : Option Source)
    (entries : List Entry) : Option Failure :=
  match header? with
  | none => none
  | some header =>
    match source? with
    | none => some (.refused .notSupplied)
    | some source =>
      if source.reference ≠ header.reference then some (.refused .notSupplied)
      else if source.digest ≠ header.digest then some (.refused .digestMismatch)
      else entries.findSome? (fun e => checkReading e (source.reading e.symbol))

/-! ## Opt-in: the level cannot reach a manifest that did not opt in -/

/-- A manifest naming no header decides alike **whatever** is supplied —
including a text that contradicts every entry it has. This is what makes the
level unable to refuse a program that predates it. -/
theorem a_manifest_naming_no_header_is_unchanged (source? : Option Source)
    (entries : List Entry) : checkBinding none source? entries = none := rfl

/-- And a manifest that *does* name one is not vacuous: there is a binding this
gate refuses. -/
theorem the_level_is_not_a_no_op :
    checkBinding (some ⟨"h", "d"⟩) none [] = some (.refused .notSupplied) := rfl

/-! ## The digest gate -/

theorem a_mismatched_digest_refuses (header : HeaderIdentity) (source : Source)
    (reference : source.reference = header.reference)
    (digest : source.digest ≠ header.digest) (entries : List Entry) :
    checkBinding (some header) (some source) entries = some (.refused .digestMismatch) := by
  simp [checkBinding, reference, digest]

/-- **The digest decides before any declaration is read.** A substituted header
that agrees with the manifest in every declaration is still refused, which is
the whole content of verifying an identity rather than pinning it. -/
theorem a_substituted_header_that_agrees_is_still_refused
    (header : HeaderIdentity) (source : Source)
    (reference : source.reference = header.reference)
    (digest : source.digest ≠ header.digest) (entries : List Entry)
    (_agreeing : ∀ e ∈ entries, checkReading e (source.reading e.symbol) = none) :
    checkBinding (some header) (some source) entries = some (.refused .digestMismatch) := by
  simp [checkBinding, reference, digest]

/-- A header nobody supplied refuses rather than being dropped: a binding this
driver cannot apply must not become silence about the build. -/
theorem an_unsupplied_header_refuses (header : HeaderIdentity) (entries : List Entry) :
    checkBinding (some header) none entries = some (.refused .notSupplied) := rfl

/-- The reference is compared **exactly** — constraint 11's no-normalisation
discipline applied to a third kind of spelled name. -/
theorem a_reference_that_is_not_the_one_named_is_not_supplied
    (header : HeaderIdentity) (source : Source)
    (elsewhere : source.reference ≠ header.reference) (entries : List Entry) :
    checkBinding (some header) (some source) entries = some (.refused .notSupplied) := by
  simp [checkBinding, elsewhere]

/-! ## Totality, one-directional -/

theorem a_bound_symbol_the_header_does_not_declare_refuses (e : Entry) :
    checkReading e .absent = some (.refused .undeclaredSymbol) := rfl

/-- **A declaration the manifest does not name changes nothing.** Stated over
two sources that agree on the named symbols and are arbitrary elsewhere, so it
is a claim about the gate rather than about one header: a header is the
library's whole exported surface, and the manifest is the selection.

This is deliberately the *opposite* of the contract slice's both-directions
totality. A contract module in a manifest's slice is authored for this library,
so a declaration in it that no entry binds makes the binding partial; a header
is authored without reference to any manifest. -/
private theorem findSome?_congr {α β : Type} (f g : α → Option β) :
    ∀ l : List α, (∀ a ∈ l, f a = g a) → l.findSome? f = l.findSome? g
  | [], _ => rfl
  | a :: as, h => by
      have head : f a = g a := h a (by simp)
      have tail := findSome?_congr f g as (fun x hx => h x (by simp [hx]))
      simp only [List.findSome?, head]
      cases g a <;> simp [tail]

theorem unnamed_declarations_are_not_read (header : HeaderIdentity)
    (left right : Source) (entries : List Entry)
    (reference : left.reference = right.reference) (digest : left.digest = right.digest)
    (named : ∀ e ∈ entries, left.reading e.symbol = right.reading e.symbol) :
    checkBinding (some header) (some left) entries
      = checkBinding (some header) (some right) entries := by
  simp only [checkBinding, reference, digest]
  split
  · rfl
  · split
    · rfl
    · exact findSome?_congr _ _ entries (fun e he => by rw [named e he])

/-! ## A disagreement refuses; it never withdraws -/

/-- **The weaker admissible reading is admitted**, which is what makes the
refusal below a decision instead of a default. -/
theorem the_weaker_reading_is_admitted (source? : Option Source) (entries : List Entry) :
    checkBinding none source? entries = none := rfl

/-- And a contradicting header refuses rather than withdrawing to it. The two
halves stand side by side: the same entries, admitted with no header bound and
refused with one.

The rule is card `NATIVE-LIBRARY-CONTRACT-SLICE`'s — *withdraw only where the
weaker admissible reading refuses* — and here it is accepted, so withdrawing
would reward a contradiction with a premise that says less about the build. -/
theorem a_contradicting_header_refuses_and_the_weaker_reading_is_admitted :
    let e : Entry := ⟨"k", [], .i32⟩
    let d : Declaration := ⟨some [.unit], [], false, false, false⟩
    let s : Source := ⟨"h", "d", fun _ => .declared d⟩
    checkBinding (some ⟨"h", "d"⟩) (some s) [e] = some .disagreement ∧
      checkBinding none (some s) [e] = none := ⟨rfl, rfl⟩

/-- **Binding a header only removes admissions.** The headerless reading admits
everything this one does, over every manifest and every supplied text. -/
theorem a_header_binding_only_removes_admissions (header? : Option HeaderIdentity)
    (source? : Option Source) (entries : List Entry)
    (_admitted : checkBinding header? source? entries = none) :
    checkBinding none source? entries = none := rfl

/-- …and it removes at least one, so the monotonicity above is not a statement
about an empty set. -/
theorem a_header_binding_removes_something :
    let e : Entry := ⟨"k", [], .i32⟩
    let s : Source := ⟨"h", "d", fun _ => .absent⟩
    checkBinding (some ⟨"h", "d"⟩) (some s) [e] ≠ none ∧
      checkBinding none (some s) [e] = none := by
  exact ⟨by decide, rfl⟩

/-! ## Read, and unspellable: two different facts -/

/-- An unspellable position never agrees, for **any** manifest spelling. A
manifest cannot reach a function pointer or a floating type by writing something
else at that position. -/
theorem an_unspellable_position_never_agrees (p : Projection) (t : AbiType)
    (unspellable : isUnspellable p = true) :
    positionOutcome p t = some (.refused .unspellablePosition) := by
  have readable : p.isNone = false := by
    cases p with
    | none => simp [isUnspellable] at unspellable
    | some => rfl
  simp [positionOutcome, readable, unspellable]

/-- And it is reported apart from an unreadable one. Collapsing the two would
report a vocabulary gap as a reader gap, and they have different follow-ons. -/
theorem unreadable_and_unspellable_are_different_refusals (t : AbiType) :
    positionOutcome none t = some (.refused .unreadablePosition) ∧
      positionOutcome (some [.fnptr]) t = some (.refused .unspellablePosition) := by
  refine ⟨rfl, ?_⟩
  cases t <;> rfl

/-! ## The typedef shapes, imported from `QCore.TypedefProjection` -/

open QCore.TypedefProjection (Target project)

/-- A **function-pointer typedef** — the shape card
`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught the gateway reader — is read here and
is unspellable in a manifest. That is `msgpack_packer_init`'s third parameter,
measured. -/
theorem a_function_pointer_typedef_is_read_and_unspellable :
    isUnspellable (project .functionPointer 0 false) = true := by
  decide

/-- An **aggregate defined by its own typedef** under one pointer level — the
other shape that card taught — is read *and* spellable, so the declaration
beside the one above projects and agrees. The two shapes are the two halves of
one measured declaration. -/
theorem an_aggregate_pointer_typedef_is_read_and_spellable :
    isUnspellable (project (.aggregate 0) 1 false) = false ∧
      positionOutcome (project (.aggregate 0) 1 false) .ptrMut = none := by
  refine ⟨by decide, by decide⟩

/-- An aggregate **value** is unreadable rather than unspellable: this
vocabulary has no spelling for one, and the reader does not commit. -/
theorem an_aggregate_value_is_unreadable (t : AbiType) :
    positionOutcome (project (.aggregate 0) 0 false) t
      = some (.refused .unreadablePosition) := rfl

/-! ## The `HEADER-STDINT-PRELUDE` -/

/-- The typedef names C11 itself fixes, with the readings the standard fixes
them to — the one place this reader commits to a name it did not see defined.

Two classes. The **exact-width** names (C11 7.20.1.1) are specified to be
exactly `N` bits, so their reading rests on the standard alone. The
pointer- and size-carrying names (7.20.1.4, 7.19) are *some* integer type of
sufficient width, which the standard does not fix; their reading is the LP64
one, and LP64 is not assumed here — it is the manifest's own declared
`data_model lp64` premise. -/
def standardTypedef : String → Option (List AbiType)
  | "int8_t" => some [.i8]
  | "int16_t" => some [.i16]
  | "int32_t" => some [.i32]
  | "int64_t" => some [.i64, .isize]
  | "uint8_t" => some [.u8]
  | "uint16_t" => some [.u16]
  | "uint32_t" => some [.u32]
  | "uint64_t" => some [.u64, .usize]
  | "intptr_t" => some [.i64, .isize]
  | "uintptr_t" => some [.u64, .usize]
  | "ptrdiff_t" => some [.i64, .isize]
  | "size_t" => some [.u64, .usize]
  | _ => none

/-- Every prelude name the standard fixes **exactly** reads as exactly one
spelling. -/
theorem the_exact_width_names_are_single_readings :
    standardTypedef "int8_t" = some [.i8] ∧ standardTypedef "uint8_t" = some [.u8] ∧
      standardTypedef "int32_t" = some [.i32] ∧ standardTypedef "uint32_t" = some [.u32] :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- And every name whose width the standard leaves open reads as the **pair** the
declared LP64 premise fixes — the same set the gateway reader gives `long` and
`unsigned long`, so the two compare at one granularity. -/
theorem the_open_width_names_are_the_lp64_pairs :
    standardTypedef "size_t" = some [.u64, .usize] ∧
      standardTypedef "uintptr_t" = some [.u64, .usize] ∧
      standardTypedef "intptr_t" = some [.i64, .isize] ∧
      standardTypedef "ptrdiff_t" = some [.i64, .isize] :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- A name the standard leaves open in width is **absent**, and therefore
unreadable. The prelude is what C11 fixes and not a convenience list. -/
theorem a_name_the_standard_leaves_open_is_absent :
    standardTypedef "intmax_t" = none ∧ standardTypedef "int_least32_t" = none ∧
      standardTypedef "ssize_t" = none ∧ standardTypedef "wchar_t" = none :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- Every prelude reading is spellable in the manifest vocabulary — which is why
the prelude is what makes the level decide anything at all: a header writes
exactly these names, and a manifest entry writes exactly their spellings. -/
theorem every_prelude_reading_is_spellable (name : String) (spellings : List AbiType)
    (present : standardTypedef name = some spellings) :
    isUnspellable (some spellings) = false := by
  unfold standardTypedef at present
  split at present
  all_goals
    first
      | (injection present with equal; subst equal; decide)
      | simp at present

/-! ## Refusal completeness over the shape vocabulary -/

/-- Each shape a header can present for a bound symbol refuses by its own name.
No shape falls through to a disagreement, and none is silently admitted. -/
theorem every_non_declaration_shape_refuses_by_name (e : Entry) :
    checkReading e .absent = some (.refused .undeclaredSymbol) ∧
      checkReading e .conditionalOnly = some (.refused .conditionalDeclaration) ∧
      checkReading e .macroDefined = some (.refused .macroSymbol) ∧
      checkReading e .ambiguous = some (.refused .ambiguousDeclaration) :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- And each shape a declaration can carry that is not comparable refuses by its
own name, in the implementation's order. -/
theorem every_incomparable_declaration_refuses_by_name (e : Entry)
    (result : Projection) (parameters : List Projection) :
    checkReading e (.declared ⟨result, parameters, false, false, true⟩)
        = some (.refused .internalDeclaration) ∧
      checkReading e (.declared ⟨result, parameters, true, false, false⟩)
        = some (.refused .variadicDeclaration) ∧
      checkReading e (.declared ⟨result, parameters, false, true, false⟩)
        = some (.refused .unspecifiedParameters) :=
  ⟨rfl, rfl, rfl⟩

/-- An arity difference is a **disagreement**, not a refusal: both sides were
read and they say different things. -/
theorem an_arity_difference_disagrees (e : Entry) (result : Projection)
    (parameters : List Projection)
    (differs : e.parameters.length ≠ parameters.length) :
    checkReading e (.declared ⟨result, parameters, false, false, false⟩)
      = some .disagreement := by
  simp [checkReading, differs]

/-- `static` is a statement about the **artifact**, not a reader boundary: a
symbol with internal linkage is not an exported symbol, so binding one to an
artifact identity would be a false statement. It refuses even where every
projection agrees. -/
theorem an_internal_declaration_refuses_even_when_it_agrees :
    let e : Entry := ⟨"k", [], .i32⟩
    checkReading e (.declared ⟨some [.i32], [], false, false, true⟩)
        = some (.refused .internalDeclaration) ∧
      checkReading e (.declared ⟨some [.i32], [], false, false, false⟩) = none :=
  ⟨rfl, rfl⟩

/-! ## Non-vacuity

`checkBinding` is a pure function of the header identity, source and entries.
That type-level restriction is not an independent certificate-replay proof. -/

/-- A complete agreeing binding, so nothing above is vacuous. -/
theorem an_agreeing_binding_is_admitted :
    let e : Entry := ⟨"k", [.ptrConst, .u32], .i32⟩
    let d : Declaration := ⟨some [.i32], [some [.ptrConst], some [.u32]], false, false, false⟩
    let s : Source := ⟨"h", "d", fun _ => .declared d⟩
    checkBinding (some ⟨"h", "d"⟩) (some s) [e] = none := by
  decide

end QCore.NativeLibraryHeader
