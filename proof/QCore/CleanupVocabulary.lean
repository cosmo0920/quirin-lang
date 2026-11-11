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

/-! Card `P04-B1`: the cleanup-outcome contracts' declared referent vocabulary.

Model of `crates/qcore-verification/src/cleanup_vocabulary.rs`. Two layers are
modelled, because the card's claim has two halves:

* the **spelling layer** — each clause is one C-style identifier built from a
  fixed prefix and a payload, so a declaration cannot be read two ways;
* the **clause layer** — a declaration is a set of structured premises, and
  reading it back yields exactly one profile shape or nothing at all.

What is proved: the prefixes are a prefix code, the type tokens are pairwise
distinct and separator-free, and a name or record tag carrying the separator has
no spelling at all; reading is single-valued and fails closed (no marker, a
repeated marker, a duplicate, an index gap or a missing single-valued item each
yield *no* shape rather than a defaulted one); binding admits exactly the
declared shape; and — over an exhaustively enumerated bounded domain and over
the real pinned unmap shape — spelling round-trips and is injective, so every
profile member, the reset values and the *order* of the syscall arguments
included, is load-bearing.

What is **not** proved here, and stays a Rust boundary: that
`String::starts_with`/`split("__")` implement the segmentation modelled below
(the prefix-code lemmas state what that implementation must satisfy, and
`crates/qcore-verification/tests/cleanup_vocabulary.rs` exercises the real
one); and, decisively, that a declared name is *bound by the pinned C source*.
That second half is not a property of this relation at all — it is decided by
the existing replay in `crates/qcore-verification/src/cleanup_outcome.rs` over
a real Clang import, and is measured, not modelled. Declaring `close`, `munmap`
or `unlink` remains a statement about a name, never about a descriptor, a
mapping or a file.

The exhaustive domain is deliberately small and its size is pinned below; every
claim over it is stated as bounded. -/
namespace QCore.CleanupVocabulary

/-- The C types the family binds: a closed scalar table plus single pointers to
a `struct` tag. Arrays, function pointers, double indirection and typedef
spellings are absent by construction, which is the refusal. -/
inductive CType where
  | int
  | charPtr
  | unsignedLong
  | sizeT
  | voidPtr
  | constCharPtr
  | record (tag : String)
  deriving DecidableEq, Repr

/-- What a successful syscall leaves behind in a field. This is the
custody-bearing half of the inventory. -/
inductive Reset where
  | null
  | integer (value : Int)
  deriving DecidableEq, Repr

/-- Names are abstract so the bounded domain below is decidable in the kernel
without string arithmetic. The spelling layer instantiates `N := String`. -/
structure Field (N : Type) where
  name : N
  type : CType
  reset : Reset
  deriving DecidableEq, Repr

/-- The `CleanupProfile` a contract declares, plus the guarded-delete
eligibility field, which is not a profile member and is bound separately. -/
structure Shape (N : Type) where
  function : N
  rootType : CType
  syscall : N
  signatureResult : CType
  signatureArguments : List CType
  presenceField : N
  fields : List (Field N)
  argumentFields : List N
  mappingField : Option N
  errnoHelper : N
  reportHelper : N
  deriving DecidableEq, Repr

/-- One declared premise. `marker` is the interpreter selector, the analogue of
W1's `resource_contract_v1`. -/
inductive Clause (N : Type) where
  | marker
  | function (name : N)
  | rootType (type : CType)
  | syscall (name : N)
  | signature (result : CType) (arguments : List CType)
  | presence (name : N)
  | field (index : Nat) (name : N) (type : CType) (reset : Reset)
  | argument (index : Nat) (name : N)
  | mapping (name : N)
  | errno (name : N)
  | report (name : N)
  deriving DecidableEq, Repr

/-! ## The spelling layer

An identifier is modelled as its character list, and the separator, prefix and
segment tests are written structurally here rather than taken from `String`, so
the kernel decides them. Two model bounds are stated rather than hidden: an
index and a reset magnitude are single digits (the real inventories have three
fields and reset to `-1`, `0` or `NULL`), and the Rust builds these identifiers
with `String` concatenation, which the theorems below constrain but do not
mechanise. -/

/-- An identifier, as characters. -/
abbrev Ident := List Char

/-- Structural prefix test, so `decide` can reduce it. -/
def isPrefix : Ident → Ident → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs => a == b && isPrefix as bs

/-- Does an identifier carry the `__` separator anywhere? -/
def carriesSeparator : Ident → Bool
  | [] => false
  | [_] => false
  | a :: b :: rest => (a == '_' && b == '_') || carriesSeparator (b :: rest)

/-- The ten fixed prefixes, in the order the reader tries them. -/
def prefixes : List Ident :=
  [ "cleanup_syscall_signature__".toList, "cleanup_argument_field__".toList,
    "cleanup_presence_field__".toList, "cleanup_mapping_field__".toList,
    "cleanup_report_helper__".toList, "cleanup_errno_helper__".toList,
    "cleanup_root_type__".toList, "cleanup_function__".toList,
    "cleanup_syscall__".toList, "cleanup_field__".toList ]

/-- The closed scalar token table. -/
def scalarTokens : List (CType × Ident) :=
  [ (.int, "int".toList), (.charPtr, "char_ptr".toList),
    (.unsignedLong, "unsigned_long".toList), (.sizeT, "size_t".toList),
    (.voidPtr, "void_ptr".toList), (.constCharPtr, "const_char_ptr".toList) ]

/-- A name usable where the payload is `__`-split: nonempty and separator-free.
Verbatim payloads (a function, syscall or helper name) are not restricted,
which is why `__errno_location` is declarable at all. -/
def admissibleSegment (name : String) : Bool :=
  match name.toList with
  | [] => false
  | cs => !carriesSeparator cs

/-- A record tag additionally may not begin with a digit. -/
def admissibleTag (tag : String) : Bool :=
  match tag.toList with
  | [] => false
  | c :: cs => !c.isDigit && !carriesSeparator (c :: cs)

/-- Single-digit indices and reset magnitudes: a model bound, refused rather
than truncated. -/
def digitToken : Nat → Option Ident
  | 0 => some ['0'] | 1 => some ['1'] | 2 => some ['2'] | 3 => some ['3']
  | 4 => some ['4'] | 5 => some ['5'] | 6 => some ['6'] | 7 => some ['7']
  | 8 => some ['8'] | 9 => some ['9'] | _ => none

/-- The identifier token of a C type, total or reject. -/
def typeToken : CType → Option Ident
  | .record tag =>
      if admissibleTag tag then some ("struct_".toList ++ tag.toList ++ "_ptr".toList)
      else none
  | t => (scalarTokens.find? (fun p => p.1 = t)).map Prod.snd

/-- The reset token. Negative values take the `m` spelling so the token stays
an identifier. -/
def resetToken : Reset → Option Ident
  | .null => some "null".toList
  | .integer n =>
      (digitToken n.natAbs).map (fun d =>
        if n < 0 then "integer_m".toList ++ d else "integer_".toList ++ d)

private def sep : Ident := "__".toList

/-- The identifier a clause is spelled by, total or reject. -/
def spellClause : Clause String → Option Ident
  | .marker => some "cleanup_vocabulary_v1".toList
  | .function n =>
      if n.toList.isEmpty then none else some ("cleanup_function__".toList ++ n.toList)
  | .syscall n =>
      if n.toList.isEmpty then none else some ("cleanup_syscall__".toList ++ n.toList)
  | .errno n =>
      if n.toList.isEmpty then none else some ("cleanup_errno_helper__".toList ++ n.toList)
  | .report n =>
      if n.toList.isEmpty then none else some ("cleanup_report_helper__".toList ++ n.toList)
  | .rootType t => (typeToken t).map ("cleanup_root_type__".toList ++ ·)
  | .presence n =>
      if admissibleSegment n then some ("cleanup_presence_field__".toList ++ n.toList)
      else none
  | .mapping n =>
      if admissibleSegment n then some ("cleanup_mapping_field__".toList ++ n.toList)
      else none
  | .argument i n =>
      if admissibleSegment n then
        (digitToken i).map (fun d => "cleanup_argument_field__".toList ++ d ++ sep ++ n.toList)
      else none
  | .field i n t r =>
      if admissibleSegment n then
        match digitToken i, typeToken t, resetToken r with
        | some d, some tt, some rt =>
            some ("cleanup_field__".toList ++ d ++ sep ++ n.toList ++ sep ++ tt ++ sep ++ rt)
        | _, _, _ => none
      else none
  | .signature res args =>
      match (res :: args).foldr
              (fun t acc => match typeToken t, acc with
                            | some tt, some rest => some (tt :: rest)
                            | _, _ => none) (some []) with
      | some tokens =>
          some ("cleanup_syscall_signature__".toList
                ++ (tokens.foldr (fun t acc => if acc.isEmpty then t else t ++ sep ++ acc) []))
      | none => none

/-- No prefix is a prefix of another, so an identifier determines which clause
form produced it. This is what makes the reader's longest-first search a prefix
code decode rather than a heuristic. -/
theorem the_prefixes_are_pairwise_incomparable :
    prefixes.all (fun a => prefixes.all (fun b => a == b || !isPrefix a b)) := by decide +kernel

/-- The ten prefixes are distinct, so the table has no dead row. -/
theorem the_prefixes_are_distinct : prefixes.eraseDups.length = 10 := by decide +kernel

/-- Every prefix ends with the separator, so a payload always begins at a
segment boundary. -/
theorem every_prefix_ends_with_the_separator :
    prefixes.all (fun p => isPrefix sep p.reverse) := by decide +kernel

/-- The scalar tokens are pairwise distinct, so decoding one is unambiguous. -/
theorem the_scalar_tokens_are_pairwise_distinct :
    (scalarTokens.map Prod.snd).eraseDups.length = 6 := by decide +kernel

/-- …and so are the C spellings they decode to. -/
theorem the_scalar_spellings_are_pairwise_distinct :
    (scalarTokens.map Prod.fst).eraseDups.length = 6 := by decide +kernel

/-- Every scalar token is separator-free, so `__` remains an unambiguous
separator inside a field clause or a signature. -/
theorem every_scalar_token_is_separator_free :
    scalarTokens.all (fun p => !carriesSeparator p.2) := by decide +kernel

/-- No scalar token can be confused with a record token. -/
theorem no_scalar_token_looks_like_a_record :
    scalarTokens.all (fun p => !isPrefix "struct_".toList p.2) := by decide +kernel

/-- Every reset and index token is separator-free too. -/
theorem every_reset_and_index_token_is_separator_free :
    ([Reset.null, .integer 0, .integer (-1), .integer 9].all
        (fun r => (resetToken r).all (fun t => !carriesSeparator t)))
      ∧ ([0, 1, 2, 3, 9].all (fun i => (digitToken i).all (fun t => !carriesSeparator t))) := by
  decide +kernel

/-- A field or argument name carrying the separator has no spelling at all: it
is refused rather than split into pieces the reader would guess at. -/
theorem a_name_carrying_the_separator_has_no_spelling :
    spellClause (.field 0 "a__b" .int .null) = none
      ∧ spellClause (.argument 0 "a__b") = none
      ∧ spellClause (.presence "a__b") = none
      ∧ spellClause (.mapping "a__b") = none := by decide +kernel

/-- A record tag carrying the separator likewise has no token, so a record
pointer can never masquerade as two segments. -/
theorem a_record_tag_carrying_the_separator_has_no_token :
    typeToken (.record "cio__file") = none := by decide +kernel

/-- A record tag starting with a digit is refused, so a token can never be read
as an index. -/
theorem a_record_tag_starting_with_a_digit_has_no_token :
    typeToken (.record "0file") = none := by decide +kernel

/-- An empty payload is refused, so a clause never degenerates to its bare
prefix. -/
theorem an_empty_payload_has_no_spelling :
    spellClause (.function "") = none ∧ spellClause (.syscall "") = none
      ∧ spellClause (.errno "") = none ∧ spellClause (.report "") = none
      ∧ spellClause (.presence "") = none := by decide +kernel

/-- An index the model cannot spell as one digit is refused rather than
truncated — the stated model bound, made fail-closed. -/
theorem an_unspellable_index_is_refused :
    spellClause (.argument 10 "a") = none
      ∧ spellClause (.field 10 "a" .int .null) = none := by decide +kernel

/-- Non-vacuity of the verbatim rule: the real errno helper, whose C name
begins with the separator, is declarable exactly as the contract spells it. -/
theorem the_errno_helper_spells_verbatim :
    spellClause (.errno "__errno_location")
      = some "cleanup_errno_helper____errno_location".toList := by decide +kernel

/-- Non-vacuity of the structured rules: the real field, signature and record
clauses spell exactly as the pinned contracts write them. -/
theorem the_pinned_clauses_spell_as_the_contracts_write_them :
    spellClause (.field 1 "alloc_size" .unsignedLong (.integer 0))
        = some "cleanup_field__1__alloc_size__unsigned_long__integer_0".toList
      ∧ spellClause (.field 0 "fd" .int (.integer (-1)))
        = some "cleanup_field__0__fd__int__integer_m1".toList
      ∧ spellClause (.signature .int [.voidPtr, .sizeT])
        = some "cleanup_syscall_signature__int__void_ptr__size_t".toList
      ∧ spellClause (.signature .int [.constCharPtr])
        = some "cleanup_syscall_signature__int__const_char_ptr".toList
      ∧ spellClause (.rootType (.record "cio_file"))
        = some "cleanup_root_type__struct_cio_file_ptr".toList
      ∧ spellClause (.argument 1 "alloc_size")
        = some "cleanup_argument_field__1__alloc_size".toList
      ∧ spellClause (.mapping "map") = some "cleanup_mapping_field__map".toList := by decide +kernel

/-! ## The clause layer -/

variable {N : Type}

private def single : List α → Option α
  | [a] => some a
  | _ => none

private def indexed [DecidableEq α] (n : Nat) (start : Nat)
    (entries : List (Nat × α)) : Option (List α) :=
  match n with
  | 0 => some []
  | n + 1 =>
      match single ((entries.filter (fun p => p.1 = start)).map Prod.snd) with
      | some a => (indexed n (start + 1) entries).map (a :: ·)
      | none => none

/-- Gather an indexed family: exactly one entry per index, contiguous from
zero, in index order. A duplicate index or a gap yields nothing. -/
def gather [DecidableEq α] (entries : List (Nat × α)) : Option (List α) :=
  indexed entries.length 0 entries

section Reader
variable [DecidableEq N]

private def markers (cs : List (Clause N)) : Nat :=
  (cs.filter (· = .marker)).length

private def functions : List (Clause N) → List N
  | [] => []
  | .function n :: rest => n :: functions rest
  | _ :: rest => functions rest

private def rootTypes : List (Clause N) → List CType
  | [] => []
  | .rootType t :: rest => t :: rootTypes rest
  | _ :: rest => rootTypes rest

private def syscalls : List (Clause N) → List N
  | [] => []
  | .syscall n :: rest => n :: syscalls rest
  | _ :: rest => syscalls rest

private def signatures : List (Clause N) → List (CType × List CType)
  | [] => []
  | .signature r a :: rest => (r, a) :: signatures rest
  | _ :: rest => signatures rest

private def presences : List (Clause N) → List N
  | [] => []
  | .presence n :: rest => n :: presences rest
  | _ :: rest => presences rest

private def errnos : List (Clause N) → List N
  | [] => []
  | .errno n :: rest => n :: errnos rest
  | _ :: rest => errnos rest

private def reports : List (Clause N) → List N
  | [] => []
  | .report n :: rest => n :: reports rest
  | _ :: rest => reports rest

private def mappings : List (Clause N) → List N
  | [] => []
  | .mapping n :: rest => n :: mappings rest
  | _ :: rest => mappings rest

private def fieldEntries : List (Clause N) → List (Nat × Field N)
  | [] => []
  | .field i n t r :: rest => (i, ⟨n, t, r⟩) :: fieldEntries rest
  | _ :: rest => fieldEntries rest

private def argumentEntries : List (Clause N) → List (Nat × N)
  | [] => []
  | .argument i n :: rest => (i, n) :: argumentEntries rest
  | _ :: rest => argumentEntries rest

/-- Read a declaration back. Total or reject: the marker must appear exactly
once, every single-valued item exactly once, the indexed families must be
duplicate-free and contiguous, and neither inventory may be empty. -/
def read (cs : List (Clause N)) : Option (Shape N) :=
  if markers cs ≠ 1 then none else
  match single (functions cs), single (rootTypes cs), single (syscalls cs),
        single (signatures cs), single (presences cs), single (errnos cs),
        single (reports cs), gather (fieldEntries cs), gather (argumentEntries cs),
        mappings cs with
  | some function, some rootType, some syscall, some signature, some presence,
    some errno, some report, some (f :: fields), some (a :: arguments), ms =>
      match ms with
      | [] =>
        some { function, rootType, syscall, signatureResult := signature.1,
               signatureArguments := signature.2, presenceField := presence,
               fields := f :: fields, argumentFields := a :: arguments,
               mappingField := none, errnoHelper := errno, reportHelper := report }
      | [m] =>
        some { function, rootType, syscall, signatureResult := signature.1,
               signatureArguments := signature.2, presenceField := presence,
               fields := f :: fields, argumentFields := a :: arguments,
               mappingField := some m, errnoHelper := errno, reportHelper := report }
      | _ => none
  | _, _, _, _, _, _, _, _, _, _ => none

/-- Spell a shape as the declaration it is, in canonical order. The reader is
order-insensitive, so this is one representative of the declarations that read
back to it. -/
def spell (s : Shape N) : List (Clause N) :=
  [ .marker, .function s.function, .rootType s.rootType, .syscall s.syscall,
    .signature s.signatureResult s.signatureArguments,
    .presence s.presenceField ]
  ++ (s.fields.zipIdx.map (fun p => Clause.field p.2 p.1.name p.1.type p.1.reset))
  ++ (s.argumentFields.zipIdx.map (fun p => Clause.argument p.2 p.1))
  ++ (match s.mappingField with | some m => [Clause.mapping m] | none => [])
  ++ [ .errno s.errnoHelper, .report s.reportHelper ]

/-- Binding: a profile is admitted exactly when it *is* the declared shape.
There is no reconciliation step and no default. -/
def bind (cs : List (Clause N)) (s : Shape N) : Bool := decide (read cs = some s)

/-- Reading is single-valued: a declaration never yields two shapes. -/
theorem a_declaration_never_yields_two_shapes (cs : List (Clause N)) (a b : Shape N)
    (ha : read cs = some a) (hb : read cs = some b) : a = b := by
  rw [ha] at hb
  exact Option.some.inj hb

/-- When a declaration cannot be read, *every* profile is refused. This is the
fail-closed half: an unreadable contract never falls back to the caller's
profile. -/
theorem an_unreadable_declaration_refuses_every_profile
    (cs : List (Clause N)) (h : read cs = none) (s : Shape N) : bind cs s = false := by
  simp [bind, h]

/-- Binding admits exactly the declared shape and nothing else. -/
theorem binding_admits_exactly_the_declared_shape (cs : List (Clause N)) (s t : Shape N)
    (h : read cs = some s) : bind cs t = true ↔ t = s := by
  simp [bind, h, eq_comm]

/-- A declaration without the selector reads as nothing: every earlier revision
of these two contracts is refused rather than defaulted. -/
theorem a_declaration_without_the_marker_is_refused (cs : List (Clause N))
    (h : markers cs = 0) : read cs = none := by
  simp [read, h]

/-- A repeated selector is refused too, so the marker cannot be doubled to mask
a malformed body. -/
theorem a_repeated_marker_is_refused (cs : List (Clause N)) (h : markers cs = 2) :
    read cs = none := by
  simp [read, h]

end Reader

/-! ### Bounded round-trip and injectivity

The domain is enumerated exhaustively and its size is pinned, so the claim is
bounded by construction. It exercises both inventories at two lengths, both
reset forms, three types, both mapping states and both argument orders, which
is what makes the ordering and reset rows below non-vacuous. -/

/-- Two abstract names. Strings play no part in the bounded domain, so the
kernel decides it without string arithmetic. -/
inductive Sym where
  | a
  | b
  deriving DecidableEq, Repr

private def sampleFields : List (List (Field Sym)) :=
  [ [⟨.a, .int, .null⟩],
    [⟨.a, .int, .integer (-1)⟩],
    [⟨.b, .charPtr, .integer 0⟩],
    [⟨.a, .int, .null⟩, ⟨.b, .charPtr, .integer 0⟩],
    [⟨.b, .charPtr, .integer 0⟩, ⟨.a, .int, .null⟩] ]

private def sampleArguments : List (List Sym) :=
  [ [.a], [.b], [.a, .b], [.b, .a] ]

/-- The enumerated shape domain. -/
def sampleShapes : List (Shape Sym) :=
  sampleFields.flatMap fun fields =>
    sampleArguments.flatMap fun arguments =>
      [none, some Sym.a].map fun mapping =>
        { function := .a, rootType := .int, syscall := .b,
          signatureResult := .int, signatureArguments := [.voidPtr, .sizeT],
          presenceField := .a, fields, argumentFields := arguments,
          mappingField := mapping, errnoHelper := .a, reportHelper := .b }

/-- The sample domain is not degenerate. -/
theorem the_sample_domain_is_inhabited_and_varied :
    sampleShapes.length = 40
      ∧ sampleShapes.any (fun s => s.mappingField.isSome)
      ∧ sampleShapes.any (fun s => s.mappingField.isNone)
      ∧ sampleShapes.any (fun s => s.fields.length = 2)
      ∧ sampleShapes.any (fun s => s.argumentFields.length = 2) := by decide +kernel

/-- Bounded: over the enumerated domain, a declaration reads back to exactly
the shape that spelled it. -/
theorem spelling_round_trips_on_the_sample_domain :
    sampleShapes.all (fun s => read (spell s) == some s) := by decide +kernel

/-- Bounded: two different shapes never spell the same declaration, so no
profile member is redundant. -/
theorem spelling_is_injective_on_the_sample_domain :
    sampleShapes.all (fun a => sampleShapes.all (fun b =>
      (spell a == spell b) == (a == b))) := by decide +kernel

/-- Bounded, and the sharpest consequence: one declaration binds one profile,
so a producer cannot hand the checker any other. -/
theorem one_declaration_binds_one_profile_on_the_sample_domain :
    sampleShapes.all (fun a => sampleShapes.all (fun b =>
      bind (spell a) b == (a == b))) := by decide +kernel

/-- Dropping the marker from a real declaration refuses it, over the whole
domain. -/
theorem dropping_the_marker_refuses_the_whole_domain :
    sampleShapes.all (fun s =>
      read ((spell s).filter (fun c => c != Clause.marker)) == none) := by decide +kernel

/-- Duplicating any field clause refuses the declaration, over the whole
domain: a duplicated inventory entry is not silently collapsed. -/
theorem duplicating_a_field_clause_refuses_the_whole_domain :
    sampleShapes.all (fun s =>
      match (spell s).find? (fun c => match c with | .field .. => true | _ => false) with
      | some c => read (c :: spell s) == none
      | none => false) := by decide +kernel

/-- Removing the index-zero field clause refuses the declaration over the whole
domain: the inventory may not have a gap, and may not be empty. -/
theorem an_inventory_gap_refuses_the_whole_domain :
    sampleShapes.all (fun s =>
      read ((spell s).filter (fun c =>
        match c with
        | .field i .. => i != 0
        | _ => true)) == none) := by decide +kernel

/-- Two eligibility-field clauses are refused rather than one being preferred. -/
theorem two_mapping_clauses_are_refused_on_the_whole_domain :
    sampleShapes.all (fun s =>
      read (Clause.mapping Sym.a :: Clause.mapping Sym.b :: spell s) == none) := by decide +kernel

/-! ### The load-bearing members of the real unmap policy -/

private def base : Shape String :=
  { function := "cio_file_native_unmap", rootType := .record "cio_file",
    syscall := "munmap", signatureResult := .int,
    signatureArguments := [.voidPtr, .sizeT], presenceField := "map",
    fields := [⟨"map", .charPtr, .null⟩,
               ⟨"alloc_size", .unsignedLong, .integer 0⟩,
               ⟨"map_truncated_warned", .int, .integer 0⟩],
    argumentFields := ["map", "alloc_size"], mappingField := none,
    errnoHelper := "__errno_location", reportHelper := "cio_errno_print" }

/-- The real unmap policy reads back to the profile the checker binds. -/
theorem the_pinned_unmap_shape_reads_back : read (spell base) = some base := by decide +kernel

/-- Argument *order* is load-bearing: swapping the two `munmap` arguments is a
different declaration, which the original refuses. -/
theorem swapping_the_syscall_arguments_is_refused :
    bind (spell base) { base with argumentFields := ["alloc_size", "map"] } = false := by
  decide +kernel

/-- A reset value is load-bearing: what a successful syscall leaves behind
cannot be changed without changing the declaration. -/
theorem changing_a_reset_value_is_refused :
    bind (spell base)
      { base with fields := [⟨"map", .charPtr, .integer 0⟩,
                             ⟨"alloc_size", .unsignedLong, .integer 0⟩,
                             ⟨"map_truncated_warned", .int, .integer 0⟩] } = false := by
  decide +kernel

/-- The inventory cannot be widened or narrowed under the same declaration. -/
theorem resizing_the_inventory_is_refused :
    bind (spell base) { base with fields := base.fields ++ [⟨"extra", .int, .null⟩] } = false
      ∧ bind (spell base) { base with fields := [⟨"map", .charPtr, .null⟩] } = false := by
  decide +kernel

/-- The guarded-delete eligibility field is declared exactly when it is
present: the close/unmap policies cannot declare one and the delete policy
cannot omit one. -/
theorem the_mapping_field_is_declared_exactly_when_present :
    bind (spell base) { base with mappingField := some "map" } = false
      ∧ bind (spell { base with mappingField := some "map" }) base = false := by decide +kernel

/-- A field type is load-bearing, so a declaration cannot retype what the
source really holds. -/
theorem retyping_a_field_is_refused :
    bind (spell base)
      { base with fields := [⟨"map", .voidPtr, .null⟩,
                             ⟨"alloc_size", .unsignedLong, .integer 0⟩,
                             ⟨"map_truncated_warned", .int, .integer 0⟩] } = false := by
  decide +kernel

/-- Renaming the pinned function, the syscall, the record type, the eligibility
field or either helper each changes the declaration. -/
theorem the_named_referents_are_load_bearing :
    bind (spell base) { base with function := "cio_file_native_close" } = false
      ∧ bind (spell base) { base with syscall := "close" } = false
      ∧ bind (spell base) { base with errnoHelper := "errno_location" } = false
      ∧ bind (spell base) { base with reportHelper := "cio_print" } = false
      ∧ bind (spell base) { base with rootType := .record "cio_chunk" } = false
      ∧ bind (spell base) { base with presenceField := "alloc_size" } = false
      ∧ bind (spell base) { base with signatureArguments := [.sizeT, .voidPtr] } = false := by
  decide +kernel

/-- Every clause of the real unmap declaration has a spelling, so the shape is
declarable at all — the totality side of total-or-reject. -/
theorem the_pinned_unmap_declaration_is_spellable :
    (spell base).all (fun c => (spellClause c).isSome) := by decide +kernel

/-- …and those spellings are pairwise distinct thirteen premises, so the
declaration is not a shorter one with collisions. -/
theorem the_pinned_unmap_spellings_are_distinct :
    ((spell base).filterMap spellClause).eraseDups.length = 13
      ∧ ((spell base).filterMap spellClause).length = 13 := by decide +kernel

end QCore.CleanupVocabulary
