// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Typed direct scalar-field access classification for the resource projection.
//!
//! The resource importer rejects any statement whose *tokens* contain `->`, a
//! non-numeric `.`, `[`, `]` or a unary `*`
//! ([`crate::legacy_c`]'s `contains_unmodeled_access`). That gate is lexical: it
//! carries no declared type, so it cannot separate a scalar field store from a
//! pointer store, a union write or an address escape. Everything it rejects
//! becomes `Unknown`, which poisons the enclosing function's resource state.
//!
//! This module supplies the missing *typed* premise from the C declarations of
//! the same preprocessed translation unit the importer is already reading, and
//! admits one conjunctive whitelist of accesses as resource-projection no-ops:
//!
//! 1. the statement contains no call and no braces (checked by the caller);
//! 2. it contains no `&`, `*`, `[` or `]` token at all, so no address-of, no
//!    dereference and no subscript can occur;
//! 3. every member access is exactly `base -> field` or `base . field` where
//!    `base` is a direct identifier naming a formal or local of the enclosing
//!    function whose *declared* type is, respectively, a single-star pointer to
//!    a complete record or that record itself;
//! 4. the record is a `struct`, is defined exactly once in the translation
//!    unit, and every one of its member declarations was read successfully;
//! 5. the accessed field's *declared* type is a non-pointer, non-array
//!    arithmetic or enumeration type carrying neither `volatile` nor `_Atomic`;
//! 6. the access does not continue (`a->b->c`, `a->b.c`, `a->b[i]`, `a->b(…)`
//!    all reject);
//! 7. every remaining identifier in the statement is an enumeration constant or
//!    a variable of the same function with a declared scalar type — which
//!    rejects casts, type names and every pointer-valued operand;
//! 8. the base is a name this function never binds to a resource identity: it
//!    is not a formal the projection protects, is never assigned, incremented
//!    or address-taken, and never appears in any call's argument list.
//!
//! Anything else keeps the existing lexical rejection and its existing
//! diagnostic. No library, record, field or function name appears in any rule
//! below: admission is decided only by declared C types and expression shape.
//!
//! **Soundness.** A read or write of a non-pointer arithmetic or enumeration
//! member of a non-union record can neither create, destroy, alias nor transfer
//! a tracked resource identity, so it is custody-neutral and the statement is a
//! no-op for this projection. Unions are excluded because a scalar store can
//! overwrite an overlapping pointer member; `volatile` and `_Atomic` because
//! they carry side effects the projection does not model; address-of because it
//! escapes a sub-object the projection cannot name.
//!
//! Custody neutrality alone would not be enough. A neutral read through a
//! pointer this function already released would turn a program the projection
//! cannot judge into a `Safe` one, which is exactly the fail-closed pin
//! `unmodeled_memory_reads_do_not_become_neutral_effects` protects. Premise 8
//! closes that: a base the projection never binds carries no released, escaped
//! or reallocated identity for this classification to erase. Whether the
//! *caller* released it is outside this analyzer's model at every other site
//! too, and is not claimed here.
//!
//! **Premises.** Two, both explicit. (a) The declarations read here are the ones
//! in force at the access — the reader refuses the whole function whenever it
//! meets a declaration it cannot read, so a declaration it did not understand
//! cannot silently shadow one it did. (b) The program respects C effective
//! types: storage declared as a record is not accessed through an incompatible
//! record type whose overlapping member is a pointer. Neither premise is proved
//! here.

use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Card BASE-TYPE-READER-CENSUS (wave 37): note sinks, and why they are shaped
// this way.
//
// The census must not become a **second** reader of declared types — that is
// the mistake card `C-GATEWAY-INDIRECT-CENSUS` made with source text and that
// `C-GATEWAY-INDIRECT-CENSUS-2` exists to avoid. So the cause of every
// `DeclaredType::Opaque` answer is recorded **by the reader, at the return that
// produces it**, and carried out to the census rather than re-derived.
//
// With the `indirect-census` feature off, `OpaqueNote` is `()`, `DeclCensus` is
// a unit struct, and each `note_*!` macro expands to a borrow of its sink with
// the cause tokens dropped before type checking — so no cause value is
// constructed, no map is allocated and no branch is taken. The reader below is
// one function either way.

#[cfg(feature = "indirect-census")]
pub(crate) type OpaqueNote = crate::indirect_census::OpaqueNote;
/// With the feature off the sink carries nothing. It is a newtype rather than
/// `()` so that `OpaqueNote::default()` is an ordinary value construction at
/// every call site under both configurations, and neither configuration needs a
/// lint exemption to say the same thing.
#[cfg(not(feature = "indirect-census"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct OpaqueNote(());

#[cfg(feature = "indirect-census")]
pub(crate) type DeclCensus = crate::indirect_census::DeclCensus;
#[cfg(not(feature = "indirect-census"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct DeclCensus(());

/// Record the specifier-level cause of one `Opaque` answer.
#[cfg(feature = "indirect-census")]
macro_rules! note_opaque {
    ($note:expr, $cause:expr) => {
        *$note = Some($cause)
    };
}
#[cfg(not(feature = "indirect-census"))]
macro_rules! note_opaque {
    ($note:expr, $cause:expr) => {
        let _ = &$note;
    };
}

/// Record the rule that refused a whole function's declarations. First writer
/// wins: the reader stops at its first refusal, so that is the rule to report.
#[cfg(feature = "indirect-census")]
macro_rules! note_refusal {
    ($census:expr, $cause:expr) => {
        if $census.refusal.is_none() {
            $census.refusal = Some($cause);
        }
    };
}
#[cfg(not(feature = "indirect-census"))]
macro_rules! note_refusal {
    ($census:expr, $cause:expr) => {
        let _ = &$census;
    };
}

/// Attach a specifier-level cause to the name it made `Opaque`.
#[cfg(feature = "indirect-census")]
macro_rules! note_name_opaque {
    ($census:expr, $name:expr, $note:expr) => {
        if let Some(cause) = $note.clone() {
            $census.opaque.insert($name.to_owned(), cause);
        }
    };
}
#[cfg(not(feature = "indirect-census"))]
macro_rules! note_name_opaque {
    ($census:expr, $name:expr, $note:expr) => {
        let _ = &$census;
    };
}

/// A declared C type, as far as this reader is willing to commit to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeclaredType {
    /// Non-pointer, non-array arithmetic or enumeration type, without
    /// `volatile` or `_Atomic`.
    Scalar,
    /// The named record type itself, with no pointer or array modifier.
    Record(String),
    /// Exactly one `*` applied to the named record type.
    RecordPointer(String),
    /// Anything this reader will not commit to. Never admitted.
    Opaque,
}

#[derive(Clone, Debug, Default)]
struct RecordInfo {
    definitions: usize,
    /// `union` tags are recorded so that they can be refused, never admitted.
    union_tag: bool,
    /// Every member declaration of the single definition was read.
    complete: bool,
    scalar_fields: BTreeSet<String>,
    /// Members whose declared type is a scalar *or* a pointer of any depth,
    /// with no array modifier and no rejecting qualifier. Reading one of these
    /// yields a value, never an aggregate sub-object. Used only by the borrowed
    /// actual rule in [`crate::borrowed_actual`], never by the scalar no-op
    /// rule: a pointer member may not be *written* under this classification.
    borrowable_fields: BTreeSet<String>,
    /// Members declared *exactly* `struct TAG * name` — one star, a `struct`
    /// specifier, no array modifier and no rejecting qualifier — mapped to
    /// `TAG`. Only these members may be *continued through* by the bounded
    /// access chain of [`crate::borrowed_actual`]; every other member ends the
    /// chain or refuses it. A `union`, a typedef this reader will not resolve,
    /// a multi-star member and a record-valued member are all absent here by
    /// construction, so a chain can never cross one.
    record_pointer_fields: BTreeMap<String, String>,
    /// Members declared with *exactly one* `*` over a specifier list this
    /// reader resolves to a `struct` tag or to an arithmetic/enumeration type,
    /// with no array modifier and no rejecting qualifier. Such a member holds
    /// exactly one pointer value, so a store into it replaces exactly one
    /// identity. `void *`, multi-star, function-pointer, array, bitfield and
    /// unresolved-typedef members are absent by construction. Used only by the
    /// aggregate-store rule in [`crate::aggregate_store`].
    single_pointer_fields: BTreeSet<String>,
    /// Members declared **exactly** `void * name` — the specifier list is the
    /// single token `void`, exactly one star, no array modifier and no
    /// rejecting qualifier (card ALIAS-AGGREGATE). Such a member holds exactly
    /// one pointer value, so a store into it replaces exactly one identity,
    /// but its declared type names no pointee, so no *read* of it can be
    /// matched against a callee's declared parameter type. Every typedef this
    /// reader will not resolve, `const void *`, `void **` and every other
    /// opaque spelling are absent by construction: only the literal one-token
    /// `void` specifier list is admitted here. Used only by the aggregate-store
    /// rule in [`crate::aggregate_store`], and there only on the *store* side.
    void_pointer_fields: BTreeSet<String>,
    /// Members declared with **no** `*`, no array modifier and no rejecting
    /// qualifier, over a specifier list this reader committed to as a
    /// **non-pointer object type** ([`TypedAccessModel::is_non_pointer_object`]).
    /// Such a member is a sub-object of the record that can never itself *be* a
    /// pointer this projection links, so its *address* denotes storage disjoint
    /// from every linkable member. Pointer members (including pointer
    /// typedefs), arrays, bitfields and every typedef this reader will not
    /// resolve are absent by construction. Used only by the member-address rule
    /// in [`crate::aggregate_store`].
    ///
    /// Card **ABI-TYPEDEF-ADDRESS-OF-MEMBER** widened the specifier side of
    /// this set from `struct TAG`/arithmetic to every non-pointer object type
    /// the reader commits to — a `union TAG` member and a member whose typedef
    /// *defines* its own `struct`/`union`/`enum` join it. The disjointness
    /// argument reads only "this member is not a pointer", never the member's
    /// own shape, so it is indifferent to which of those the member is.
    non_pointer_fields: BTreeSet<String>,
    /// Card **FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS**: the body was read past
    /// at least one function-pointer member declaration, which joins no class.
    /// Rule 5g's flow-refusing scalar member store keeps its base domain of
    /// records read with every member (see [`scalar_member_store_base`]).
    function_pointer_members: bool,
}

/// What one `typedef` declaration of this unit names, as far as this reader is
/// willing to commit to it (card **ABI-TYPEDEF-ADDRESS-OF-MEMBER**).
///
/// Before that card the table held only the specifier-list reading, and every
/// other shape — an aggregate the typedef itself *defines* above all — recorded
/// the absent reading `None`. Recording the aggregate case distinctly keeps
/// [`TypedAccessModel::classify_specifiers`] answering exactly what it answered
/// before (an aggregate definition is [`DeclaredType::Opaque`], as an
/// unresolved name was), while letting the *one* premise that needs no more
/// than "this is an object type and it is not a pointer" —
/// [`TypedAccessModel::is_non_pointer_object`] — commit to it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TypedefTarget {
    /// The specifier tokens the typedef expands to, with no declarator at all.
    /// Exactly the reading this table held before the card above.
    Specifiers(Vec<String>),
    /// A `struct`, `union` or `enum` the typedef declaration itself defines,
    /// with the declarator exactly the typedef name — `typedef union { … } t;`
    /// and `typedef struct tag { … } t;`.
    ///
    /// The payload is the definition's *head* — the tag keyword, and the tag
    /// when the definition carries one (`"union"`, `"struct msgpack_packer"`).
    /// It is carried so that two disagreeing declarations of one name still
    /// disagree under this enum's `PartialEq`, and it is the only part of the
    /// definition read: a member of such a type is not a pointer whatever its
    /// body holds.
    Aggregate(String),
}

/// The member classifications one readable record body yields.
#[derive(Clone, Debug, Default)]
struct RecordMembers {
    scalar_fields: BTreeSet<String>,
    borrowable_fields: BTreeSet<String>,
    record_pointer_fields: BTreeMap<String, String>,
    single_pointer_fields: BTreeSet<String>,
    void_pointer_fields: BTreeSet<String>,
    non_pointer_fields: BTreeSet<String>,
    function_pointer_members: bool,
}

/// Declaration facts of one preprocessed translation unit.
#[derive(Clone, Debug, Default)]
pub(crate) struct TypedAccessModel {
    records: BTreeMap<String, RecordInfo>,
    /// `None` marks a name this reader refuses to resolve.
    typedefs: BTreeMap<String, Option<TypedefTarget>>,
    enum_constants: BTreeSet<String>,
    /// Function names this unit declares, mapped to whether **every**
    /// declaration of that name the reader read returns exactly one `*` over a
    /// specifier list resolving to [`DeclaredType::Scalar`] (card
    /// **CONSTRUCTOR-RESIDUALS**). `false` is absorbing: a second, disagreeing
    /// or unreadable declaration of the same name retires the fact rather than
    /// competing with it, and a name absent here was never read at all. Used
    /// only by the dereference-of-a-call-result rule in
    /// [`crate::variadic_nested`].
    scalar_pointer_results: BTreeMap<String, bool>,
    /// Card **BASE-TYPE-READER-CENSUS**: names this unit declares at **file
    /// scope**, read by the same declaration reader the function bodies use.
    ///
    /// Census-only, and approximate **in the refusing direction**: a file-scope
    /// declaration this reader will not read is absent here, so the census
    /// reports its name `Unknown` rather than claiming a declaration it did not
    /// see. Nothing consults this table but the census.
    #[cfg(feature = "indirect-census")]
    census_file_scope: BTreeSet<String>,
}

/// Declared types of one function's formals and locals.
///
/// `None` means the reader met a declaration it could not read and therefore
/// refuses every access in that function.
#[derive(Clone, Debug, Default)]
pub(crate) struct FunctionTypes {
    variables: Option<BTreeMap<String, DeclaredType>>,
    /// Names this function may bind to a resource identity. An
    /// over-approximation: rejecting a name that could not in fact be bound
    /// only narrows the whitelist.
    bound: BTreeSet<String>,
    /// Card **W43-INDIRECT-ACCESS-GENERAL**: the names whose one level of
    /// indirection designates a scalar object — `SCALAR * name` or
    /// `SCALAR name [ N ]` — and `None` for a name declared more than once with
    /// different answers. A side table: no [`DeclaredType`] answer changes.
    elements: BTreeMap<String, Option<ScalarElement>>,
    /// Card **FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS**: the parameter list
    /// carries a function-pointer formal, which the reader now reads as an
    /// `Opaque` name instead of refusing the function. Rule 5g's
    /// flow-refusing store keeps its domain of functions read before that
    /// (see [`scalar_member_store_base`]).
    function_pointer_formals: bool,
    /// Card **BASE-TYPE-READER-CENSUS**: the rule that refused this function's
    /// declarations, and the specifier-level cause of every name the reader
    /// answered `Opaque` for. Written by the reader at the return that decided
    /// it; read only by the census. Absent with the feature off.
    #[cfg(feature = "indirect-census")]
    pub(crate) census: DeclCensus,
}

/// Arithmetic type keywords. `void`, `_Complex` and `_Imaginary` are absent by
/// decision: they are not admitted as scalar field or operand types.
const ARITHMETIC_KEYWORDS: &[&str] = &[
    "char", "short", "int", "long", "signed", "unsigned", "float", "double", "_Bool",
];

/// Qualifiers and storage classes that do not change the admitted class.
/// `volatile` and `_Atomic` are deliberately absent: they reject.
const IGNORED_SPECIFIERS: &[&str] = &[
    "const",
    "restrict",
    "__restrict",
    "__restrict__",
    "static",
    "register",
    "extern",
    "auto",
    "inline",
    "_Noreturn",
    "_Thread_local",
    "__extension__",
    "__inline",
    "__inline__",
];

const REJECTING_QUALIFIERS: &[&str] = &["volatile", "_Atomic"];

/// Declaration suffixes that carry no type and may stand between a
/// declarator's parameter list and its `;` or body.
const DECLARATION_SUFFIXES: &[&str] = &["__attribute__", "__attribute", "__asm__", "asm", "__asm"];

/// Keywords that can never name a declarator.
const RESERVED_WORDS: &[&str] = &[
    "auto",
    "break",
    "case",
    "char",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "register",
    "restrict",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "struct",
    "switch",
    "typedef",
    "union",
    "unsigned",
    "void",
    "volatile",
    "while",
    "_Alignas",
    "_Alignof",
    "_Atomic",
    "_Bool",
    "_Complex",
    "_Generic",
    "_Imaginary",
    "_Noreturn",
    "_Static_assert",
    "_Thread_local",
    "__extension__",
    "__inline",
    "__inline__",
    "__restrict",
    "__restrict__",
    "__typeof",
    "__typeof__",
    "typeof",
];

/// Punctuation admitted in a classified statement. `&`, `*`, `[`, `]`, `{`,
/// `}`, `,` and `;` are absent by decision; each would introduce an address,
/// an indirection, a subscript, an aggregate or a sequence this rule does not
/// model. `.` is absent because it is only ever consumed by an access match.
const ADMITTED_PUNCTUATION: &[&str] = &[
    "(", ")", "=", "+", "-", "/", "%", "<", ">", "!", "~", "^", "|", "?", ":",
];

/// Statement-token scan bound. A declaration candidate longer than this is
/// refused rather than read.
const DECLARATION_SCAN_LIMIT: usize = 4096;

/// Typedef resolution depth bound.
const TYPEDEF_DEPTH_LIMIT: usize = 8;

pub(crate) fn is_name(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && !RESERVED_WORDS.contains(&text)
}

fn is_numeric_literal(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// Read one `typedef` declaration that **defines** the aggregate it names
/// (card **ABI-TYPEDEF-ADDRESS-OF-MEMBER**).
///
/// `slice` is the token run strictly between `typedef` and the `;` that ends
/// the declaration, whose last token the caller has already checked is a name.
/// The shape recognised is exactly
///
/// ```text
/// struct|union|enum [TAG] { … } NAME
/// ```
///
/// with the matching `}` immediately before `NAME`, so nothing — no `*`, no
/// `[`, no second declarator — stands between the aggregate and the declared
/// name. A `typedef struct { … } *p;`, a `typedef struct { … } a, *b;` and a
/// `typedef struct { … } a[4];` are all refused here and keep the absent
/// reading they had.
///
/// The body is **not** read: this reading commits only to "the name denotes an
/// object type with no pointer level", which is all
/// [`TypedAccessModel::is_non_pointer_object`] consumes.
fn read_aggregate_definition(slice: &[&str]) -> Option<TypedefTarget> {
    let keyword = *slice.first()?;
    if !matches!(keyword, "struct" | "union" | "enum") {
        return None;
    }
    let (head, open) = match slice.get(1) {
        Some(&"{") => (keyword.to_owned(), 1usize),
        Some(text) if is_name(text) && slice.get(2) == Some(&"{") => {
            (format!("{keyword} {text}"), 2)
        }
        _ => return None,
    };
    // The matching `}` must be the token immediately before the declared name,
    // which is the last token of the slice.
    if matching_close(slice, open, "{", "}")? + 2 != slice.len() {
        return None;
    }
    Some(TypedefTarget::Aggregate(head))
}

/// Card **FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS**: the name one declaration
/// group declares when it is exactly a function-pointer declarator,
/// `SPECIFIERS ( * NAME ) ( PARAMETERS )`.
///
/// The specifier run is non-empty and holds only names and `*`; the
/// declarator is one `*` and one name in parentheses (no array, no
/// qualifier, no second level); the parameter list is balanced, closes the
/// group, and holds no `{`, `}`, `;`, `=` or `:`. Every other shape answers
/// `None`, so a caller that refused a parenthesised declaration before still
/// refuses everything this does not name.
fn function_pointer_declarator<'a>(group: &[&'a str]) -> Option<&'a str> {
    let open = group.iter().position(|text| *text == "(")?;
    // Specifiers are identifier tokens (keywords such as `void`, `struct`,
    // `const` included) and `*`.
    let identifier = |text: &str| {
        text.as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    };
    if open == 0
        || group[..open]
            .iter()
            .any(|text| !(identifier(text) || *text == "*"))
    {
        return None;
    }
    let ["*", name, ")", "("] = group.get(open + 1..open + 5)? else {
        return None;
    };
    if !is_name(name) || RESERVED_WORDS.contains(name) {
        return None;
    }
    let close = matching_close(group, open + 4, "(", ")")?;
    if close + 1 != group.len()
        || group[open + 5..close]
            .iter()
            .any(|text| matches!(*text, "{" | "}" | ";" | "=" | ":"))
    {
        return None;
    }
    Some(name)
}

/// One parameter list's declaration groups, split at the commas outside
/// parentheses, so a function-pointer formal's own parameter list stays in
/// its group. Without parentheses this is the plain comma split.
fn parameter_groups<'s, 'a>(parameters: &'s [&'a str]) -> Vec<&'s [&'a str]> {
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, text) in parameters.iter().enumerate() {
        match *text {
            "(" => depth += 1,
            ")" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                groups.push(&parameters[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    groups.push(&parameters[start..]);
    groups
}

/// The token on which [`TypedAccessModel::read_record_body`] refuses a whole
/// record body, if any: a `{`, `}`, `:` or `=` anywhere, or a parenthesis in
/// a member declaration that is not exactly a function-pointer declarator
/// ([`function_pointer_declarator`]). Shared with the record-body census so
/// that its `refused_on` is the reader's own answer.
pub(crate) fn record_body_refusal<'a>(body: &[&'a str]) -> Option<&'a str> {
    if let Some(offending) = body
        .iter()
        .find(|text| matches!(**text, "{" | "}" | ":" | "="))
    {
        return Some(offending);
    }
    body.split(|text| *text == ";")
        .filter(|member| member.iter().any(|text| matches!(*text, "(" | ")")))
        .find(|member| function_pointer_declarator(member).is_none())
        .and_then(|member| {
            member
                .iter()
                .find(|text| matches!(**text, "(" | ")"))
                .copied()
        })
}

fn matching_close(texts: &[&str], open_index: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open_index) {
        if *text == open {
            depth += 1;
        } else if *text == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

impl TypedAccessModel {
    /// Read every record, typedef and enumeration constant declared in one
    /// preprocessed translation unit.
    pub(crate) fn read(texts: &[&str]) -> Self {
        let mut model = Self::default();
        model.read_typedefs(texts);
        model.read_records(texts);
        model.read_enum_constants(texts);
        model.read_scalar_pointer_results(texts);
        #[cfg(feature = "indirect-census")]
        {
            model.census_file_scope = file_scope_names(&model, texts);
        }
        model
    }

    /// Whether this unit's declaration reader read `name` as a typedef name,
    /// resolved or not (integration of W44-COLLECTION-ALLOCATION onto
    /// W44-INDIRECT-CALL-ARGUMENT-ACCESS: a `sizeof` operand name is a type
    /// only when it is one of these or a tag).
    pub(crate) fn declares_typedef(&self, name: &str) -> bool {
        self.typedefs.contains_key(name)
    }

    /// The sole named struct a direct defining typedef denotes. This is a
    /// narrow source-frame view, not a change to the shared classifier: that
    /// classifier deliberately keeps defining aggregates opaque. Anonymous,
    /// union, pointer, conflicting aliases and incomplete/ambiguous tags refuse.
    pub(crate) fn defining_struct_alias(&self, alias: &str) -> Option<&str> {
        let Some(Some(TypedefTarget::Aggregate(head))) = self.typedefs.get(alias) else {
            return None;
        };
        let mut words = head.split_ascii_whitespace();
        if words.next()? != "struct" {
            return None;
        }
        let tag = words.next()?;
        if words.next().is_some() {
            return None;
        }
        let record = self.records.get(tag)?;
        (record.complete && record.definitions == 1 && !record.union_tag).then_some(tag)
    }

    /// Whether this unit declares `name` at file scope, as far as the same
    /// declaration reader will read it. Card **BASE-TYPE-READER-CENSUS**;
    /// census-only.
    #[cfg(feature = "indirect-census")]
    pub(crate) fn census_declares_at_file_scope(&self, name: &str) -> bool {
        self.census_file_scope.contains(name)
    }

    /// Read the return type of every function declaration or definition this
    /// reader can commit to (card **CONSTRUCTOR-RESIDUALS**).
    ///
    /// The shape read is `PREFIX NAME ( … ) ;` or `PREFIX NAME ( … ) {`, with
    /// only type-free attribute or assembler suffixes in between — the same
    /// declarator shape [`crate::legacy_declared_abi`] reads for the ABI
    /// comparison, except that the *pointee* is classified here instead of
    /// being projected away. `PREFIX` is walked backwards from the name over
    /// stars, qualifiers, storage classes, tag keywords and typedef names, and
    /// an empty prefix means "not a declaration" — which is exactly what
    /// separates `f(x);` from `int *f(int);`.
    ///
    /// Everything the reader will not commit to records `false`, never absence:
    /// a `typedef`, a prefix whose stars are not exactly one, a specifier list
    /// classifying to anything but [`DeclaredType::Scalar`], a K&R definition
    /// (whose parameter list is followed by a declaration run rather than `;`
    /// or `{`), and a second declaration of the same name that disagrees. A
    /// name that occurs only as a call is never recorded at all, so the
    /// admission that reads this map refuses it.
    fn read_scalar_pointer_results(&mut self, texts: &[&str]) {
        for index in 0..texts.len() {
            if !is_name(texts[index]) || texts.get(index + 1) != Some(&"(") {
                continue;
            }
            let Some(close) = matching_close(texts, index + 1, "(", ")") else {
                continue;
            };
            let mut after = close + 1;
            while let Some(introducer) = texts.get(after) {
                if !DECLARATION_SUFFIXES.contains(introducer) || texts.get(after + 1) != Some(&"(")
                {
                    break;
                }
                let Some(suffix_close) = matching_close(texts, after + 1, "(", ")") else {
                    break;
                };
                after = suffix_close + 1;
            }
            if !matches!(texts.get(after), Some(&";") | Some(&"{")) {
                continue;
            }
            let mut start = index;
            while start > 0 {
                let previous = texts[start - 1];
                if previous == "*"
                    || IGNORED_SPECIFIERS.contains(&previous)
                    || REJECTING_QUALIFIERS.contains(&previous)
                    || matches!(previous, "struct" | "union" | "enum" | "typedef")
                    || (is_name(previous) && !RESERVED_WORDS.contains(&previous))
                    || ARITHMETIC_KEYWORDS.contains(&previous)
                    || previous == "void"
                {
                    start -= 1;
                    continue;
                }
                break;
            }
            if start == index {
                continue;
            }
            // A declaration begins a statement. Without this the reader would
            // mistake `return * f ( ) ;` and `x = a * f ( ) ;` for declarations
            // of `f`, and — because `false` is absorbing — would retire the
            // real declaration's fact from somewhere else in the unit. Both
            // directions stay fail-closed either way; this only keeps the
            // reader from losing a fact it did read.
            if start > 0 && !matches!(texts[start - 1], ";" | "{" | "}") {
                continue;
            }
            let prefix = &texts[start..index];
            let stars = prefix.iter().rev().take_while(|text| **text == "*").count();
            let specifiers = &prefix[..prefix.len() - stars];
            if specifiers.is_empty() {
                continue;
            }
            let scalar = stars == 1
                && !specifiers.contains(&"typedef")
                && !specifiers.contains(&"*")
                && matches!(
                    self.classify_specifiers(specifiers, 0),
                    DeclaredType::Scalar
                );
            self.scalar_pointer_results
                .entry(texts[index].to_owned())
                .and_modify(|agreed| *agreed &= scalar)
                .or_insert(scalar);
        }
    }

    /// Whether every declaration of `function` in this unit returns exactly one
    /// `*` over a scalar type, and there was at least one.
    pub(crate) fn scalar_pointer_result(&self, function: &str) -> bool {
        self.scalar_pointer_results
            .get(function)
            .copied()
            .unwrap_or(false)
    }

    /// Whether `specifiers` name an **object type that is not a pointer type**
    /// (card **ABI-TYPEDEF-ADDRESS-OF-MEMBER**).
    ///
    /// This is deliberately weaker than [`Self::classify_specifiers`] and is
    /// consulted by exactly one premise: a member declared with no `*` over
    /// such a specifier list is a sub-object of its record, so its *address*
    /// denotes storage C's object model keeps disjoint from every sibling
    /// member. That argument needs to know only that the member is not itself a
    /// pointer; it reads nothing about the member's own shape, size or
    /// contents, so an aggregate, a `union` and an `enum` all satisfy it as
    /// exactly as an `int` does.
    ///
    /// What it still refuses, each fail-closed:
    ///
    /// * a **pointer typedef** — `typedef struct s *p; … p m;` declares a
    ///   member with no `*` that *is* a pointer, and `&x->m` would then be the
    ///   `struct s **` laundering shape the member-address rule exists to
    ///   refuse. [`Self::read_typedefs`] records such a typedef unresolved, and
    ///   an unresolved name is refused here;
    /// * a **function-pointer typedef**, an **array typedef** and a
    ///   **multi-declarator typedef** — unresolved for the same reason;
    /// * a name typedef'd twice to disagreeing targets — unresolved;
    /// * a name with no typedef in this unit at all;
    /// * `void`, which is not an object type.
    fn is_non_pointer_object(&self, specifiers: &[&str], depth: usize) -> bool {
        if specifiers
            .iter()
            .any(|text| REJECTING_QUALIFIERS.contains(text))
        {
            return false;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        match core.as_slice() {
            // A tagged aggregate. `struct TAG` already classified `Record`;
            // `union TAG` and `enum TAG` are the members this widening adds,
            // and both are object types with no pointer level.
            ["struct", tag] | ["union", tag] | ["enum", tag] if is_name(tag) => true,
            ["enum"] => true,
            [name] if is_name(name) => {
                if depth >= TYPEDEF_DEPTH_LIMIT {
                    return false;
                }
                match self.typedefs.get(*name) {
                    // The head is read rather than assumed: only the three tag
                    // keywords name an aggregate, and nothing else may reach
                    // this arm.
                    Some(Some(TypedefTarget::Aggregate(head))) => matches!(
                        head.split(' ').next(),
                        Some("struct") | Some("union") | Some("enum")
                    ),
                    Some(Some(TypedefTarget::Specifiers(target))) => {
                        let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                        self.is_non_pointer_object(&target, depth + 1)
                    }
                    _ => false,
                }
            }
            rest if !rest.is_empty()
                && rest.iter().all(|text| ARITHMETIC_KEYWORDS.contains(text)) =>
            {
                true
            }
            _ => false,
        }
    }

    fn read_typedefs(&mut self, texts: &[&str]) {
        for index in 0..texts.len() {
            if texts[index] != "typedef" {
                continue;
            }
            let Some(end) = statement_end(texts, index) else {
                continue;
            };
            let slice = &texts[index + 1..end];
            let Some(last) = slice.last() else { continue };
            if !is_name(last) {
                // A declarator this reader does not read. Nothing is recorded,
                // so every use of the declared name stays unresolved.
                continue;
            }
            let specifiers = &slice[..slice.len() - 1];
            let resolved = if slice
                .iter()
                .any(|text| matches!(*text, "{" | "}" | "(" | ")" | "[" | "]" | "*" | ","))
            {
                // Card ABI-TYPEDEF-ADDRESS-OF-MEMBER: exactly one of the shapes
                // this branch used to discard is read — an aggregate the
                // typedef itself defines, whose declarator is the bare typedef
                // name. Every other shape (a pointer typedef, a
                // function-pointer typedef, an array typedef, a
                // multi-declarator typedef, an aggregate under a `*`) still
                // records the absent reading, exactly as before.
                read_aggregate_definition(slice)
            } else {
                Some(TypedefTarget::Specifiers(
                    specifiers.iter().map(|text| (*text).to_owned()).collect(),
                ))
            };
            match self.typedefs.get(*last) {
                Some(existing) if *existing == resolved => {}
                Some(_) => {
                    self.typedefs.insert((*last).to_owned(), None);
                }
                None => {
                    self.typedefs.insert((*last).to_owned(), resolved);
                }
            }
        }
    }

    fn read_records(&mut self, texts: &[&str]) {
        for index in 0..texts.len() {
            let tag_kind = texts[index];
            if tag_kind != "struct" && tag_kind != "union" {
                continue;
            }
            let Some(tag) = texts.get(index + 1).copied().filter(|tag| is_name(tag)) else {
                continue;
            };
            if texts.get(index + 2) != Some(&"{") {
                continue;
            }
            let Some(close) = matching_close(texts, index + 2, "{", "}") else {
                continue;
            };
            let entry = self.records.entry(tag.to_owned()).or_default();
            entry.definitions += 1;
            if tag_kind == "union" {
                entry.union_tag = true;
            }
            if entry.definitions > 1 {
                // The tag is ambiguous in this translation unit; refuse it.
                entry.complete = false;
                entry.scalar_fields.clear();
                entry.borrowable_fields.clear();
                entry.record_pointer_fields.clear();
                entry.single_pointer_fields.clear();
                entry.void_pointer_fields.clear();
                entry.non_pointer_fields.clear();
                entry.function_pointer_members = false;
                continue;
            }
            if entry.union_tag {
                continue;
            }
            let body = texts[index + 3..close].to_vec();
            // Card C-GATEWAY-INDIRECT-CENSUS-2: measurement only, compiled out
            // with the feature off, and taken before the reading rather than
            // after, so a refused body is seen at all.
            #[cfg(feature = "indirect-census")]
            crate::indirect_census::record_body(tag, &body);
            if let Some(members) = self.read_record_body(&body) {
                let entry = self.records.entry(tag.to_owned()).or_default();
                entry.complete = true;
                entry.scalar_fields = members.scalar_fields;
                entry.borrowable_fields = members.borrowable_fields;
                entry.record_pointer_fields = members.record_pointer_fields;
                entry.single_pointer_fields = members.single_pointer_fields;
                entry.void_pointer_fields = members.void_pointer_fields;
                entry.non_pointer_fields = members.non_pointer_fields;
                entry.function_pointer_members = members.function_pointer_members;
            }
        }
    }

    /// Read every member declaration of one record body, or refuse the record.
    ///
    /// The result is all-or-nothing on purpose: a body carrying a bitfield, a
    /// nested anonymous aggregate or any declarator this reader cannot resolve
    /// yields `None`, and the whole record is then inadmissible. Reading only
    /// the members it understands would risk attributing a scalar type to a
    /// name it had actually misparsed.
    ///
    /// Card **FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS** (ledger
    /// `RECORD-BODY-TOTAL-OR-REJECT`): the one exception is a member
    /// declaration that is exactly a function-pointer declarator
    /// ([`function_pointer_declarator`]). It is parsed whole — its name and
    /// its extent are certain, and no other member's tokens are inside it —
    /// and joins **no** class, so every access of it keeps its refusal; every
    /// other member is classified exactly as in the same body without it. A
    /// function pointer holds a code address, never an object the projection
    /// tracks, and a store into it stays refused (it is in no store class).
    fn read_record_body(&self, body: &[&str]) -> Option<RecordMembers> {
        if record_body_refusal(body).is_some() {
            return None;
        }
        let mut scalar_fields = BTreeSet::new();
        let mut borrowable_fields = BTreeSet::new();
        let mut record_pointer_fields = BTreeMap::new();
        let mut single_pointer_fields = BTreeSet::new();
        let mut void_pointer_fields = BTreeSet::new();
        let mut non_pointer_fields = BTreeSet::new();
        let mut function_pointer_members = false;
        for member in body.split(|text| *text == ";") {
            if member.is_empty() {
                continue;
            }
            // A parenthesised member passed `record_body_refusal`, so it is
            // exactly one function-pointer declarator: it joins no class.
            if member.iter().any(|text| matches!(*text, "(" | ")")) {
                function_pointer_members = true;
                continue;
            }
            let mut groups = member.split(|text| *text == ",");
            let first = groups.next()?;
            let (specifiers, first_declarator) = split_declarator(first)?;
            let mut declarators = vec![first_declarator];
            for group in groups {
                declarators.push(read_declarator(group)?);
            }
            let qualified = specifiers
                .iter()
                .any(|text| REJECTING_QUALIFIERS.contains(text));
            let scalar_specifiers =
                self.classify_specifiers(&specifiers, 0) == DeclaredType::Scalar;
            for declarator in declarators {
                let name = declarator.name?;
                if declarator.array {
                    continue;
                }
                // O04-B09B10: a member with no `*` is a sub-object of this
                // record. Its address denotes storage disjoint from every other
                // member, so handing it to a borrowing callee cannot disturb a
                // linked pointer member.
                //
                // Card ABI-TYPEDEF-ADDRESS-OF-MEMBER: the specifier premise is
                // [`Self::is_non_pointer_object`] rather than the
                // `Record`/`Scalar` classification, because the disjointness
                // argument reads only "this member is not a pointer". A
                // `union`-typed member and a member whose typedef defines its
                // own aggregate join the set; a pointer typedef, an array
                // member, a `volatile`/`_Atomic` member and every typedef this
                // reader will not resolve stay out of it.
                if declarator.stars == 0 && !qualified && self.is_non_pointer_object(&specifiers, 0)
                {
                    non_pointer_fields.insert(name.clone());
                }
                if declarator.stars == 0 && scalar_specifiers {
                    scalar_fields.insert(name.clone());
                    borrowable_fields.insert(name);
                    continue;
                }
                // A pointer member of any pointee type is readable as a value.
                // The member's own declaration must not be `volatile`/`_Atomic`.
                if declarator.stars >= 1 && !qualified {
                    // Exactly `struct TAG *` is additionally continuable: the
                    // access chain may follow it into `TAG`. `declared_type`
                    // commits to `RecordPointer` only for one star over a
                    // `struct` specifier, so unions, typedefs it will not
                    // resolve and deeper pointers are excluded here.
                    if let DeclaredType::RecordPointer(pointee) =
                        self.declared_type(&specifiers, declarator.stars, false)
                    {
                        record_pointer_fields.insert(name.clone(), pointee);
                    }
                    // Exactly one star over a specifier list this reader
                    // resolves. `void *` and every typedef it will not resolve
                    // classify `Opaque` and are excluded here.
                    if declarator.stars == 1
                        && matches!(
                            self.classify_specifiers(&specifiers, 0),
                            DeclaredType::Record(_) | DeclaredType::Scalar
                        )
                    {
                        single_pointer_fields.insert(name.clone());
                    }
                    // Card ALIAS-AGGREGATE: exactly one star over the literal
                    // one-token `void` specifier list. It is disjoint from
                    // `single_pointer_fields` by construction — `void`
                    // classifies `Opaque`, never `Record`/`Scalar` — and from
                    // `record_pointer_fields` for the same reason. `void **`,
                    // `const void *` and every typedef spelling are excluded.
                    if declarator.stars == 1 && !qualified && specifiers == ["void"] {
                        void_pointer_fields.insert(name.clone());
                    }
                    borrowable_fields.insert(name);
                }
            }
        }
        Some(RecordMembers {
            scalar_fields,
            borrowable_fields,
            record_pointer_fields,
            single_pointer_fields,
            void_pointer_fields,
            non_pointer_fields,
            function_pointer_members,
        })
    }

    fn read_enum_constants(&mut self, texts: &[&str]) {
        for index in 0..texts.len() {
            if texts[index] != "enum" {
                continue;
            }
            let open = if texts.get(index + 1) == Some(&"{") {
                index + 1
            } else if texts.get(index + 2) == Some(&"{") {
                index + 2
            } else {
                continue;
            };
            let Some(close) = matching_close(texts, open, "{", "}") else {
                continue;
            };
            let mut expect_name = true;
            let mut depth = 0usize;
            for text in &texts[open + 1..close] {
                match *text {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth = depth.saturating_sub(1),
                    "," if depth == 0 => expect_name = true,
                    name if depth == 0 && expect_name => {
                        if is_name(name) {
                            self.enum_constants.insert(name.to_owned());
                        }
                        expect_name = false;
                    }
                    _ => expect_name = false,
                }
            }
        }
    }

    fn classify_specifiers(&self, specifiers: &[&str], depth: usize) -> DeclaredType {
        self.classify_specifiers_noting(specifiers, depth, &mut OpaqueNote::default())
    }

    /// The classifier itself. `note` receives the cause of an `Opaque` answer
    /// and is `()` with the census feature off (card
    /// **BASE-TYPE-READER-CENSUS**); the answers are otherwise exactly those
    /// the wrapper above has always given.
    #[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
    fn classify_specifiers_noting(
        &self,
        specifiers: &[&str],
        depth: usize,
        note: &mut OpaqueNote,
    ) -> DeclaredType {
        if specifiers
            .iter()
            .any(|text| REJECTING_QUALIFIERS.contains(text))
        {
            note_opaque!(
                note,
                crate::indirect_census::OpaqueCause::RejectingQualifier
            );
            return DeclaredType::Opaque;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        match core.as_slice() {
            [] => {
                note_opaque!(note, crate::indirect_census::OpaqueCause::NoSpecifiers);
                DeclaredType::Opaque
            }
            ["struct", tag] if is_name(tag) => DeclaredType::Record((*tag).to_owned()),
            // A union member may overlap a pointer; never admitted.
            ["union", _] | ["union"] => {
                note_opaque!(note, crate::indirect_census::OpaqueCause::UnionSpecifier);
                DeclaredType::Opaque
            }
            ["struct"] => {
                note_opaque!(note, crate::indirect_census::OpaqueCause::BareTagKeyword);
                DeclaredType::Opaque
            }
            ["enum", tag] if is_name(tag) => DeclaredType::Scalar,
            ["enum"] => DeclaredType::Scalar,
            [name] if is_name(name) => {
                if depth >= TYPEDEF_DEPTH_LIMIT {
                    note_opaque!(note, crate::indirect_census::OpaqueCause::TypedefDepthLimit);
                    return DeclaredType::Opaque;
                }
                match self.typedefs.get(*name) {
                    Some(Some(TypedefTarget::Specifiers(target))) => {
                        let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                        self.classify_specifiers_noting(&target, depth + 1, note)
                    }
                    // Card ABI-TYPEDEF-ADDRESS-OF-MEMBER records an aggregate a
                    // typedef defines, and this classifier deliberately answers
                    // for it exactly what it answered when the reading was
                    // absent: `Opaque`. Every premise that reads this function
                    // — the scalar no-op rule, the borrowable/single-pointer/
                    // `void *` member sets, the chain walker, the declared
                    // types of locals — therefore decides byte-identically to
                    // the tree before that card. `is_non_pointer_object`
                    // reads its object classification; the separate bounded
                    // `defining_struct_alias` frame view reads its named tag
                    // without changing this shared opaque answer.
                    Some(Some(TypedefTarget::Aggregate(head))) => {
                        note_opaque!(
                            note,
                            crate::indirect_census::OpaqueCause::TypedefDefinesAggregate(
                                head.clone()
                            )
                        );
                        DeclaredType::Opaque
                    }
                    Some(None) => {
                        note_opaque!(
                            note,
                            crate::indirect_census::OpaqueCause::TypedefDisagreeing
                        );
                        DeclaredType::Opaque
                    }
                    None => {
                        note_opaque!(
                            note,
                            crate::indirect_census::OpaqueCause::TypedefUnknown((*name).to_owned())
                        );
                        DeclaredType::Opaque
                    }
                }
            }
            rest if rest.iter().all(|text| ARITHMETIC_KEYWORDS.contains(text)) => {
                DeclaredType::Scalar
            }
            _ => {
                note_opaque!(
                    note,
                    crate::indirect_census::OpaqueCause::UnreadableSpecifiers
                );
                DeclaredType::Opaque
            }
        }
    }

    /// A scalar type name with no declarator or expression syntax. Used by
    /// the fixed type-query rule; an array typedef remains opaque here.
    pub(crate) fn scalar_type_name(&self, specifiers: &[&str]) -> bool {
        !specifiers.is_empty()
            && specifiers.iter().all(|text| {
                text.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            })
            && self.classify_specifiers(specifiers, 0) == DeclaredType::Scalar
    }

    fn declared_type(&self, specifiers: &[&str], stars: usize, array: bool) -> DeclaredType {
        self.declared_type_noting(specifiers, stars, array, &mut OpaqueNote::default())
    }

    /// The declarator-level reader. `note` receives the *innermost* cause of an
    /// `Opaque` answer — a specifier the classifier refused where there is one,
    /// and otherwise the declarator fact (array, pointer depth) that made a
    /// committed specifier list opaque. Card **BASE-TYPE-READER-CENSUS**; the
    /// answers are exactly the wrapper's.
    #[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
    fn declared_type_noting(
        &self,
        specifiers: &[&str],
        stars: usize,
        array: bool,
        note: &mut OpaqueNote,
    ) -> DeclaredType {
        if array {
            note_opaque!(note, crate::indirect_census::OpaqueCause::ArrayDeclarator);
            return DeclaredType::Opaque;
        }
        match (self.classify_specifiers_noting(specifiers, 0, note), stars) {
            (kind, 0) => kind,
            (DeclaredType::Record(tag), 1) => DeclaredType::RecordPointer(tag),
            // The specifier list was refused; `note` already carries that
            // cause, which is the one an arbiter would have to answer.
            (DeclaredType::Opaque, _) => DeclaredType::Opaque,
            (DeclaredType::Scalar, _) => {
                note_opaque!(note, crate::indirect_census::OpaqueCause::PointerToScalar);
                DeclaredType::Opaque
            }
            _ => {
                note_opaque!(
                    note,
                    crate::indirect_census::OpaqueCause::PointerDepthAboveOne
                );
                DeclaredType::Opaque
            }
        }
    }

    /// [`Self::scalar_field`] over a record read with **every** member, as the
    /// reader read records before card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS:
    /// false for a record read past a function-pointer member.
    pub(crate) fn scalar_field_of_a_fully_read_record(&self, tag: &str, field: &str) -> bool {
        self.scalar_field(tag, field)
            && self
                .records
                .get(tag)
                .is_some_and(|record| !record.function_pointer_members)
    }

    pub(crate) fn scalar_field(&self, tag: &str, field: &str) -> bool {
        self.records.get(tag).is_some_and(|record| {
            record.complete
                && !record.union_tag
                && record.definitions == 1
                && record.scalar_fields.contains(field)
        })
    }

    /// Whether `tag`'s single complete non-union definition declares `field` as
    /// a scalar or as a pointer with no array modifier and no rejecting
    /// qualifier. Reading such a member yields a value, never an aggregate.
    /// Whether `name` is an enumeration constant of this translation unit.
    pub(crate) fn is_enum_constant(&self, name: &str) -> bool {
        self.enum_constants.contains(name)
    }

    pub(crate) fn borrowable_field(&self, tag: &str, field: &str) -> bool {
        self.records.get(tag).is_some_and(|record| {
            record.complete
                && !record.union_tag
                && record.definitions == 1
                && record.borrowable_fields.contains(field)
        })
    }

    /// The record tag `tag`'s member `field` points at, when that member is
    /// declared *exactly* `struct POINTEE *` in `tag`'s single complete
    /// non-union definition. `None` for every other member, so a bounded access
    /// chain can be continued only through a member this reader fully typed.
    ///
    /// The pointee record itself is **not** checked here: the chain walker
    /// re-enters this model at the next link, so an incomplete, ambiguous or
    /// `union` pointee refuses there.
    /// Whether `tag` has exactly one definition in this translation unit, it
    /// is a `struct` rather than a `union`, and every one of its member
    /// declarations was read.
    pub(crate) fn record_definition_is_complete(&self, tag: &str) -> bool {
        self.records
            .get(tag)
            .is_some_and(|record| record.complete && !record.union_tag && record.definitions == 1)
    }

    /// Whether `tag`'s single complete non-union definition declares `field`
    /// with exactly one `*` over a specifier list this reader resolved. Such a
    /// member holds exactly one pointer value.
    pub(crate) fn single_pointer_field(&self, tag: &str, field: &str) -> bool {
        self.records.get(tag).is_some_and(|record| {
            record.complete
                && !record.union_tag
                && record.definitions == 1
                && record.single_pointer_fields.contains(field)
        })
    }

    /// Whether `tag`'s single complete non-union definition declares `field`
    /// **exactly** `void * field` (card ALIAS-AGGREGATE).
    ///
    /// Such a member holds one pointer value, so a *store* into it replaces
    /// exactly one identity and the aggregate-store rule may link it. It is
    /// never readable back: `void *` names no pointee, so nothing can match it
    /// against a callee's declared parameter type, and
    /// [`crate::aggregate_store::bound_read_actuals`] refuses it explicitly.
    pub(crate) fn void_pointer_field(&self, tag: &str, field: &str) -> bool {
        self.records.get(tag).is_some_and(|record| {
            record.complete
                && !record.union_tag
                && record.definitions == 1
                && record.void_pointer_fields.contains(field)
        })
    }

    /// Whether `tag`'s single complete non-union definition declares `field`
    /// with **no** `*`, no array modifier and no rejecting qualifier, over a
    /// specifier list this reader resolved. Taking the address of such a member
    /// yields a pointer into storage that C's object model keeps disjoint from
    /// every other member of the same `struct`, so no member this projection
    /// links can be reached through it (card O04-B09B10).
    pub(crate) fn addressable_field(&self, tag: &str, field: &str) -> bool {
        self.records.get(tag).is_some_and(|record| {
            record.complete
                && !record.union_tag
                && record.definitions == 1
                && record.non_pointer_fields.contains(field)
        })
    }

    /// Card **W45-SUB-OBJECT-STORES**: the `struct` tag a specifier list
    /// names, read through this unit's typedefs, including a typedef that
    /// itself defines `struct TAG { … }` (which [`Self::classify_specifiers`]
    /// deliberately answers `Opaque`). A side reading consulted only by
    /// `legacy_c::sub_object_store`: no [`DeclaredType`] answer changes.
    /// `volatile`/`_Atomic`, `union`, an untagged aggregate and every typedef
    /// this reader does not resolve answer `None`.
    pub(crate) fn record_tag(&self, specifiers: &[&str]) -> Option<String> {
        self.record_tag_at(specifiers, 0)
    }

    fn record_tag_at(&self, specifiers: &[&str], depth: usize) -> Option<String> {
        if depth >= TYPEDEF_DEPTH_LIMIT
            || specifiers
                .iter()
                .any(|text| REJECTING_QUALIFIERS.contains(text) || *text == "typedef")
        {
            return None;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        match core.as_slice() {
            ["struct", tag] if is_name(tag) => Some((*tag).to_owned()),
            [name] if is_name(name) => match self.typedefs.get(*name)? {
                Some(TypedefTarget::Specifiers(target)) => {
                    let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                    self.record_tag_at(&target, depth + 1)
                }
                Some(TypedefTarget::Aggregate(head)) => {
                    match head.split(' ').collect::<Vec<_>>()[..] {
                        ["struct", tag] if is_name(tag) => Some(tag.to_owned()),
                        _ => None,
                    }
                }
                None => None,
            },
            _ => None,
        }
    }

    pub(crate) fn record_pointer_field(&self, tag: &str, field: &str) -> Option<&str> {
        self.records
            .get(tag)
            .filter(|record| record.complete && !record.union_tag && record.definitions == 1)
            .and_then(|record| record.record_pointer_fields.get(field))
            .map(String::as_str)
    }
}

#[derive(Clone, Debug, Default)]
struct Declarator {
    name: Option<String>,
    stars: usize,
    array: bool,
    /// Card **W43-INDIRECT-ACCESS-GENERAL**: how many trailing `[ … ]` groups
    /// the declarator carries. Read only by [`scalar_element`].
    dimensions: usize,
}

/// Split one declaration group into its specifiers and its first declarator.
fn split_declarator<'a>(group: &[&'a str]) -> Option<(Vec<&'a str>, Declarator)> {
    let declarator = read_declarator(group)?;
    let mut rest = group;
    if declarator.array {
        while rest.last() == Some(&"]") {
            let open = rest.iter().rposition(|text| *text == "[")?;
            rest = &rest[..open];
        }
    }
    if declarator.name.is_some() {
        rest = &rest[..rest.len().checked_sub(1)?];
    }
    for _ in 0..declarator.stars {
        rest = &rest[..rest.len().checked_sub(1)?];
    }
    Some((rest.to_vec(), declarator))
}

/// Read one declarator suffix: trailing array brackets, a name, and the stars
/// that precede it. An abstract declarator (`const char *`) has no name.
fn read_declarator(group: &[&str]) -> Option<Declarator> {
    let mut rest = group;
    let mut array = false;
    let mut dimensions = 0usize;
    while rest.last() == Some(&"]") {
        let open = rest.iter().rposition(|text| *text == "[")?;
        rest = &rest[..open];
        array = true;
        dimensions += 1;
    }
    if rest.is_empty() {
        return None;
    }
    let mut declarator = Declarator {
        array,
        dimensions,
        ..Declarator::default()
    };
    let last = *rest.last()?;
    if is_name(last) {
        // `struct tag` with no declarator names the tag, not a member.
        if matches!(
            rest.get(rest.len().wrapping_sub(2)),
            Some(&"struct") | Some(&"union") | Some(&"enum")
        ) {
            return Some(declarator);
        }
        declarator.name = Some(last.to_owned());
        rest = &rest[..rest.len() - 1];
    }
    while rest.last() == Some(&"*") {
        declarator.stars += 1;
        rest = &rest[..rest.len() - 1];
    }
    Some(declarator)
}

/// The index of the `;` that ends the construct starting at `start`, or the
/// index at which an unmatched closing token ends it.
pub(crate) fn statement_end(texts: &[&str], start: usize) -> Option<usize> {
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut braces = 0usize;
    for (offset, text) in texts.iter().enumerate().skip(start) {
        if offset - start > DECLARATION_SCAN_LIMIT {
            return None;
        }
        match *text {
            "(" => parentheses += 1,
            "[" => brackets += 1,
            "{" => braces += 1,
            ")" => {
                if parentheses == 0 {
                    return Some(offset);
                }
                parentheses -= 1;
            }
            "]" => {
                if brackets == 0 {
                    return Some(offset);
                }
                brackets -= 1;
            }
            "}" => {
                if braces == 0 {
                    return Some(offset);
                }
                braces -= 1;
            }
            ";" if parentheses == 0 && brackets == 0 && braces == 0 => return Some(offset),
            _ => {}
        }
    }
    None
}

/// Whether the tokens at `start` could begin a declaration. Every C
/// declaration that *names* a variable matches this shape, which is what lets
/// an unreadable declaration refuse the whole function instead of silently
/// leaving a shadowed name typed by an outer declaration.
pub(crate) fn is_declaration_candidate(texts: &[&str], start: usize) -> bool {
    let Some(first) = texts.get(start) else {
        return false;
    };
    if RESERVED_WORDS.contains(first) {
        return matches!(
            *first,
            "struct"
                | "union"
                | "enum"
                | "const"
                | "volatile"
                | "restrict"
                | "__restrict"
                | "__restrict__"
                | "static"
                | "register"
                | "extern"
                | "auto"
                | "inline"
                | "_Atomic"
                | "_Thread_local"
                | "char"
                | "short"
                | "int"
                | "long"
                | "signed"
                | "unsigned"
                | "float"
                | "double"
                | "void"
                | "_Bool"
                | "_Complex"
                | "_Imaginary"
        );
    }
    if !is_name(first) {
        return false;
    }
    matches!(texts.get(start + 1), Some(next) if is_name(next) || *next == "*")
}

impl FunctionTypes {
    /// Read the declared types of one function's formals and locals.
    ///
    /// `parameters` is the token slice between the definition's parentheses and
    /// `body` the slice between its braces. Any declaration this reader cannot
    /// read refuses the whole function.
    pub(crate) fn read(
        model: &TypedAccessModel,
        parameters: &[&str],
        body: &[&str],
        protected_formals: &BTreeSet<String>,
    ) -> Self {
        let bound = bound_names(body, protected_formals);
        let function_pointer_formals = parameters.contains(&"(");
        let mut variables = BTreeMap::new();
        let mut elements = BTreeMap::new();
        let mut census = DeclCensus::default();
        if !read_parameters(
            model,
            parameters,
            &mut variables,
            &mut elements,
            &mut census,
        ) {
            return Self::build(None, bound, BTreeMap::new(), false, census);
        }
        if !read_locals(model, body, &mut variables, &mut elements, &mut census) {
            return Self::build(None, bound, BTreeMap::new(), false, census);
        }
        Self::build(
            Some(variables),
            bound,
            elements,
            function_pointer_formals,
            census,
        )
    }

    /// Assemble one reading. Split out so the census sink is carried in exactly
    /// one place and the three exits above stay identical with the feature off.
    #[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
    fn build(
        variables: Option<BTreeMap<String, DeclaredType>>,
        bound: BTreeSet<String>,
        elements: BTreeMap<String, Option<ScalarElement>>,
        function_pointer_formals: bool,
        census: DeclCensus,
    ) -> Self {
        Self {
            variables,
            bound,
            elements,
            function_pointer_formals,
            #[cfg(feature = "indirect-census")]
            census,
        }
    }

    fn declared(&self, name: &str) -> Option<&DeclaredType> {
        self.variables.as_ref()?.get(name)
    }

    /// The declared type of one name, or `None` when the reader refused this
    /// function's declarations or does not know the name.
    pub(crate) fn declared_type(&self, name: &str) -> Option<&DeclaredType> {
        self.declared(name)
    }

    /// Card **W43-INDIRECT-ACCESS-GENERAL**: what one level of indirection
    /// through `name` designates, when it is a scalar object: `name` is
    /// declared exactly once (per answer) as `SCALAR * name` or
    /// `SCALAR name [ N ]`. `None` when the reader refused this function, the
    /// name is undeclared, or any declaration of it answers otherwise.
    pub(crate) fn scalar_element(&self, name: &str) -> Option<ScalarElement> {
        self.variables.as_ref()?;
        self.elements.get(name).copied().flatten()
    }

    /// Whether `name` is a declared scalar variable of this function.
    pub(crate) fn declared_is_scalar(&self, name: &str) -> bool {
        self.declared(name) == Some(&DeclaredType::Scalar)
    }

    /// Card **W43-INDEX-TRAVERSAL**: what kind of single-level pointer object
    /// `name` is, when the reader committed to one: `SCALAR * name`,
    /// `SCALAR name [ N ]` or `struct T * name`. `None` for every other
    /// declaration, an undeclared name and an unreadable function.
    pub(crate) fn pointer_kind(&self, name: &str) -> Option<PointerKind> {
        match self.scalar_element(name) {
            Some(ScalarElement::Pointer) => return Some(PointerKind::Scalar),
            Some(ScalarElement::Array) => return Some(PointerKind::Array),
            None => {}
        }
        matches!(self.declared(name), Some(DeclaredType::RecordPointer(_)))
            .then_some(PointerKind::Record)
    }

    /// Card **W43-INDEX-TRAVERSAL**: every name this function declares as a
    /// scalar, and every name it declares as a single-level pointer object
    /// ([`Self::pointer_kind`]). Both empty when the reader refused the
    /// function, so no transfer rule keyed on them can fire there.
    pub(crate) fn traversal_names(&self) -> (BTreeSet<String>, BTreeSet<String>) {
        let Some(variables) = self.variables.as_ref() else {
            return (BTreeSet::new(), BTreeSet::new());
        };
        let scalars = variables
            .iter()
            .filter(|(_, declared)| **declared == DeclaredType::Scalar)
            .map(|(name, _)| name.clone())
            .collect();
        let pointers = variables
            .keys()
            .chain(self.elements.keys())
            .filter(|name| self.pointer_kind(name).is_some())
            .cloned()
            .collect();
        (scalars, pointers)
    }

    /// Whether every declaration of this function was read. When false, no
    /// typed rule may admit anything in it.
    pub(crate) fn is_readable(&self) -> bool {
        self.variables.is_some()
    }

    /// Whether this function may bind `name` to a resource identity.
    ///
    /// The over-approximation [`bound_names`] computes, read out so that a rule
    /// living in another module can require C02's premise 8 — a base a body may
    /// bind can hold a released, escaped or reallocated identity, and a read
    /// through it is not something any of these rules may neutralize. Card
    /// REACH-GATE reads it for the access chains it admits in an expression
    /// position; every link of such a chain hangs off a base this answers
    /// `false` for.
    pub(crate) fn may_bind(&self, name: &str) -> bool {
        self.bound.contains(name)
    }

    /// Every name this function declares as exactly `struct TAG *`, paired with
    /// `TAG`.
    ///
    /// Empty when the reader refused this function's declarations, so a rule
    /// reading it can establish no type sharing there. Used only by
    /// [`crate::may_alias`], which relates *two* declared names of the same
    /// record-pointer type; a name whose type this reader will not commit to is
    /// absent, and therefore relates to nothing.
    pub(crate) fn record_pointer_names(&self) -> Vec<(&str, &str)> {
        self.variables
            .as_ref()
            .into_iter()
            .flatten()
            .filter_map(|(name, declared)| match declared {
                DeclaredType::RecordPointer(tag) => Some((name.as_str(), tag.as_str())),
                _ => None,
            })
            .collect()
    }
}

/// Compound-assignment operators, tokenized one character at a time. `==`,
/// `!=`, `<=` and `>=` are deliberately absent: a comparison does not write.
const COMPOUND_ASSIGNMENT_OPERATORS: &[&str] = &["+", "-", "*", "/", "%", "&", "|", "^"];

/// Whether the name at `index` is written, incremented or address-taken there.
fn is_written_at(body: &[&str], index: usize) -> bool {
    let next = body.get(index + 1).copied();
    let after = body.get(index + 2).copied();
    // `x = …`, but not `x == …`.
    if next == Some("=") && after != Some("=") {
        return true;
    }
    // `x += …` and its siblings.
    if next.is_some_and(|operator| COMPOUND_ASSIGNMENT_OPERATORS.contains(&operator))
        && after == Some("=")
        && next != Some("+")
        && next != Some("-")
    {
        return true;
    }
    // `x++`, `x--`, and `x += 1` / `x -= 1` share their first two tokens with
    // the increment forms, so both are treated as writes.
    if matches!(
        (next, after),
        (Some("+"), Some("+")) | (Some("-"), Some("-"))
    ) || (matches!(next, Some("+") | Some("-")) && after == Some("="))
    {
        return true;
    }
    // `x <<= …` and `x >>= …`.
    if matches!(
        (next, after),
        (Some("<"), Some("<")) | (Some(">"), Some(">"))
    ) && body.get(index + 3).copied() == Some("=")
    {
        return true;
    }
    // `++x`, `--x`, `&x`.
    let previous = index.checked_sub(1).and_then(|at| body.get(at)).copied();
    let before = index.checked_sub(2).and_then(|at| body.get(at)).copied();
    previous == Some("&")
        || matches!(
            (before, previous),
            (Some("+"), Some("+")) | (Some("-"), Some("-"))
        )
}

/// Over-approximate the names this function may bind to a resource identity.
///
/// A name is bound when it is written (`x = …`, `x op= …`, `x++`, `--x`), when
/// its address is taken, or when it appears anywhere in a call's argument list,
/// since a contracted call can release, consume or reallocate through it. A
/// formal the projection already protects is bound at entry. This is a syntactic
/// over-approximation on purpose: naming too many bases only rejects more.
pub(crate) fn bound_names(body: &[&str], protected_formals: &BTreeSet<String>) -> BTreeSet<String> {
    // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): a static-duration
    // declaration's initialization is no write of the body.
    let masked = crate::storage_duration::mask_static_declarators(body);
    let body = masked.as_slice();
    let mut bound = protected_formals.clone();
    let mut argument_depth = 0usize;
    let mut call_closes: BTreeSet<usize> = BTreeSet::new();
    for index in 0..body.len() {
        if call_closes.remove(&index) {
            argument_depth = argument_depth.saturating_sub(1);
        }
        let text = body[index];
        if !is_name(text) {
            continue;
        }
        if body.get(index + 1) == Some(&"(") {
            if let Some(close) = matching_close(body, index + 1, "(", ")") {
                argument_depth += 1;
                call_closes.insert(close);
            } else {
                // An unbalanced call list: refuse every name after it.
                argument_depth += 1;
            }
            continue;
        }
        if argument_depth > 0 {
            bound.insert(text.to_owned());
            continue;
        }
        if is_written_at(body, index) {
            bound.insert(text.to_owned());
        }
    }
    // Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (rule 10o): a parenthesised
    // destination or operand (`(x) = …`, `(x)++`, `&(x)`, the spelling of a
    // macro-expanded write) is a write the adjacency test above does not see.
    bound.extend(crate::lexical_nested_assignment::written_names(body));
    bound
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**: what one level of indirection through
/// a declared name designates, when that is a scalar object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarElement {
    /// `SCALAR * name`: exactly one `*` over a specifier list the classifier
    /// resolves to [`DeclaredType::Scalar`], no array modifier.
    Pointer,
    /// `SCALAR name [ N ]`: exactly one array dimension over such a list and no
    /// `*`. A formal spelled this way is a pointer (C11 6.7.6.3p7); a local is
    /// an automatic array. Either way one subscript designates one scalar.
    Array,
}

/// Card **W43-INDEX-TRAVERSAL**: the single-level pointer objects whose
/// arithmetic, copies and null tests the traversal rules read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerKind {
    /// `SCALAR * name`.
    Scalar,
    /// `SCALAR name [ N ]`: an array local or an array-spelled formal. It is
    /// never stepped in place (an array local is not an lvalue that `++`
    /// accepts, and the rule does not distinguish the formal).
    Array,
    /// `struct T * name`.
    Record,
}

/// The [`ScalarElement`] one declarator gives its name, if any. `volatile` and
/// `_Atomic` specifier lists are never `Scalar`, so neither is ever an element.
fn scalar_element(
    model: &TypedAccessModel,
    specifiers: &[&str],
    declarator: &Declarator,
) -> Option<ScalarElement> {
    if model.classify_specifiers(specifiers, 0) != DeclaredType::Scalar {
        return None;
    }
    match (declarator.stars, declarator.dimensions) {
        (1, 0) => Some(ScalarElement::Pointer),
        (0, 1) => Some(ScalarElement::Array),
        _ => None,
    }
}

/// Record one declaration's element answer. A second declaration of the same
/// name with a different answer (a shadowing redeclaration) makes the name
/// unusable, exactly as [`record_variable`] does for the declared type.
fn record_element(
    elements: &mut BTreeMap<String, Option<ScalarElement>>,
    name: &str,
    element: Option<ScalarElement>,
) {
    match elements.get(name) {
        Some(existing) if *existing != element => {
            elements.insert(name.to_owned(), None);
        }
        Some(_) => {}
        None => {
            elements.insert(name.to_owned(), element);
        }
    }
}

#[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
fn record_variable(
    variables: &mut BTreeMap<String, DeclaredType>,
    name: String,
    kind: DeclaredType,
    census: &mut DeclCensus,
) {
    match variables.get(&name) {
        // A redeclaration in an inner scope with a different declared type
        // makes the name unusable rather than silently keeping either type.
        Some(existing) if *existing != kind => {
            note_name_opaque!(
                census,
                name,
                Some(crate::indirect_census::OpaqueCause::ShadowedByDifferentType)
            );
            variables.insert(name, DeclaredType::Opaque);
        }
        Some(_) => {}
        None => {
            variables.insert(name, kind);
        }
    }
}

#[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
fn read_parameters(
    model: &TypedAccessModel,
    parameters: &[&str],
    variables: &mut BTreeMap<String, DeclaredType>,
    elements: &mut BTreeMap<String, Option<ScalarElement>>,
    census: &mut DeclCensus,
) -> bool {
    // Card CG26-VARIADICS (rule 11w): a variadic definition's trailing
    // `, ...` declares no object, so the named parameters before it are the
    // whole declared set. The tokenizer delivers the ellipsis as three `.`
    // tokens; any other `.` still refuses below.
    let parameters = crate::variadic_tail::named_parameters(parameters);
    if let Some(offending) = parameters
        .iter()
        .find(|text| matches!(**text, "{" | "}" | "." | ";" | "="))
    {
        note_refusal!(
            census,
            crate::indirect_census::DeclRefusal::ParameterListToken((*offending).to_owned())
        );
        return false;
    }
    if parameters.is_empty() || parameters == ["void"] {
        return true;
    }
    for group in parameter_groups(parameters) {
        // Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: a function-pointer
        // formal `R ( * cb ) ( P )` declares `cb` with no committed type
        // (`Opaque`, which no typed rule admits) instead of refusing every
        // declaration of the function. Any other parenthesised parameter
        // still refuses them all, as before.
        if group.iter().any(|text| matches!(*text, "(" | ")")) {
            let Some(name) = function_pointer_declarator(group) else {
                let offending = group
                    .iter()
                    .find(|text| matches!(**text, "(" | ")"))
                    .copied()
                    .unwrap_or("(");
                note_refusal!(
                    census,
                    crate::indirect_census::DeclRefusal::ParameterListToken(offending.to_owned())
                );
                return false;
            };
            note_name_opaque!(
                census,
                name,
                Some(crate::indirect_census::OpaqueCause::FunctionPointerDeclarator)
            );
            record_element(elements, name, None);
            record_variable(variables, name.to_owned(), DeclaredType::Opaque, census);
            continue;
        }
        let Some((specifiers, declarator)) = split_declarator(group) else {
            note_refusal!(
                census,
                crate::indirect_census::DeclRefusal::ParameterDeclarator
            );
            return false;
        };
        let element = scalar_element(model, &specifiers, &declarator);
        let Some(name) = declarator.name else {
            continue;
        };
        let mut note = OpaqueNote::default();
        let kind =
            model.declared_type_noting(&specifiers, declarator.stars, declarator.array, &mut note);
        note_name_opaque!(census, name, note);
        record_element(elements, &name, element);
        record_variable(variables, name, kind, census);
    }
    true
}

#[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
fn read_locals(
    model: &TypedAccessModel,
    body: &[&str],
    variables: &mut BTreeMap<String, DeclaredType>,
    elements: &mut BTreeMap<String, Option<ScalarElement>>,
    census: &mut DeclCensus,
) -> bool {
    // Card W45-FB-BACKLOG-CUSTODY: the declaration inside an element
    // statement expression (`container_of`) is skipped, not read.
    let skipped = crate::legacy_c::list_iteration::element_expression_spans(body);
    for start in 0..body.len() {
        if start > 0 && !matches!(body[start - 1], "{" | "}" | ";" | "(" | ":") {
            continue;
        }
        if skipped
            .iter()
            .any(|(open, close)| *open < start && start <= *close)
        {
            continue;
        }
        if !is_declaration_candidate(body, start) {
            continue;
        }
        let Some(end) = statement_end(body, start) else {
            note_refusal!(
                census,
                crate::indirect_census::DeclRefusal::LocalStatementEnd
            );
            return false;
        };
        if !read_declaration(model, &body[start..end], variables, elements, census) {
            return false;
        }
    }
    true
}

/// Plain-name provenance cannot distinguish shadowed bindings. Reuse the
/// existing total-or-reject declaration reader, counting declaration runs
/// separately so same-type shadowing is excluded too. This admits no access.
pub(crate) fn unique_local_names(model: &TypedAccessModel, body: &[&str]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut repeated = BTreeSet::new();
    // Card W45-FB-BACKLOG-CUSTODY: as in `read_locals`.
    let skipped = crate::legacy_c::list_iteration::element_expression_spans(body);
    for start in 0..body.len() {
        if start > 0 && !matches!(body[start - 1], "{" | "}" | ";" | "(" | ":") {
            continue;
        }
        if skipped
            .iter()
            .any(|(open, close)| *open < start && start <= *close)
        {
            continue;
        }
        if !is_declaration_candidate(body, start) {
            continue;
        }
        let Some(end) = statement_end(body, start) else {
            return BTreeSet::new();
        };
        let mut names = BTreeMap::new();
        if !read_declaration(
            model,
            &body[start..end],
            &mut names,
            &mut BTreeMap::new(),
            &mut DeclCensus::default(),
        ) {
            return BTreeSet::new();
        }
        for name in names.into_keys() {
            if !seen.insert(name.clone()) {
                repeated.insert(name);
            }
        }
    }
    seen.retain(|name| !repeated.contains(name));
    seen
}

#[cfg_attr(not(feature = "indirect-census"), allow(unused_variables))]
fn read_declaration(
    model: &TypedAccessModel,
    declaration: &[&str],
    variables: &mut BTreeMap<String, DeclaredType>,
    elements: &mut BTreeMap<String, Option<ScalarElement>>,
    census: &mut DeclCensus,
) -> bool {
    // Initializers are dropped before the declarator is read: an initializer
    // may contain calls, aggregates and commas that say nothing about the
    // declared type. What remains must be a declarator this reader understands.
    let mut groups = split_top_level(declaration, ",").into_iter();
    let Some(first) = groups.next() else {
        note_refusal!(
            census,
            crate::indirect_census::DeclRefusal::LocalEmptyDeclaration
        );
        return false;
    };
    let first = truncate_at_initializer(&first);
    if let Some(offending) = unreadable_declarator_token(&first) {
        note_refusal!(
            census,
            crate::indirect_census::DeclRefusal::LocalDeclaratorToken(offending.to_owned())
        );
        return false;
    }
    let Some((specifiers, declarator)) = split_declarator(&first) else {
        note_refusal!(
            census,
            crate::indirect_census::DeclRefusal::LocalSplitDeclarator
        );
        return false;
    };
    let mut declarators = vec![declarator];
    for group in groups {
        let group = truncate_at_initializer(&group);
        if let Some(offending) = unreadable_declarator_token(&group) {
            note_refusal!(
                census,
                crate::indirect_census::DeclRefusal::LocalDeclaratorToken(offending.to_owned())
            );
            return false;
        }
        let Some(declarator) = read_declarator(&group) else {
            note_refusal!(
                census,
                crate::indirect_census::DeclRefusal::LocalReadDeclarator
            );
            return false;
        };
        declarators.push(declarator);
    }
    for declarator in declarators {
        let element = scalar_element(model, &specifiers, &declarator);
        let Some(name) = declarator.name else {
            continue;
        };
        let mut note = OpaqueNote::default();
        let kind =
            model.declared_type_noting(&specifiers, declarator.stars, declarator.array, &mut note);
        note_name_opaque!(census, name, note);
        record_element(elements, &name, element);
        record_variable(variables, name, kind, census);
    }
    true
}

/// Every name this unit declares at brace depth 0, read by the **same**
/// declaration reader `read_locals` uses. Card **BASE-TYPE-READER-CENSUS**.
///
/// A function *definition*'s declarator carries `(`, so `readable_declarator`
/// refuses it and no function name enters this set — which is what makes it a
/// file-scope **object** table. A declaration the reader will not read is
/// simply absent, so the census under-reports rather than over-reports.
#[cfg(feature = "indirect-census")]
fn file_scope_names(model: &TypedAccessModel, texts: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    for start in 0..texts.len() {
        match texts[start] {
            "{" => {
                depth += 1;
                continue;
            }
            "}" => {
                depth = depth.saturating_sub(1);
                continue;
            }
            _ => {}
        }
        if depth != 0 {
            continue;
        }
        if start > 0 && !matches!(texts[start - 1], "{" | "}" | ";") {
            continue;
        }
        if !is_declaration_candidate(texts, start) {
            continue;
        }
        let Some(end) = statement_end(texts, start) else {
            continue;
        };
        let mut found = BTreeMap::new();
        if read_declaration(
            model,
            &texts[start..end],
            &mut found,
            &mut BTreeMap::new(),
            &mut DeclCensus::default(),
        ) {
            names.extend(found.into_keys());
        }
    }
    names
}

/// The first token that makes a declarator unreadable, if any: a nested
/// declarator, a function parameter list, a bitfield width or an aggregate.
/// A declarator this answers `Some` for refuses its whole function.
///
/// Card **BASE-TYPE-READER-CENSUS** replaced the predicate `readable_declarator`
/// — which was `self.is_none()` — with this, because the census reports *which*
/// token refused and a second reader answering that question could disagree
/// with the first. The refusal set is unchanged.
fn unreadable_declarator_token<'a>(group: &[&'a str]) -> Option<&'a str> {
    group
        .iter()
        .copied()
        .find(|text| matches!(*text, "{" | "}" | "(" | ")" | ":"))
}

fn truncate_at_initializer<'a>(group: &[&'a str]) -> Vec<&'a str> {
    let mut depth = 0usize;
    for (index, text) in group.iter().enumerate() {
        match *text {
            "[" | "(" | "{" => depth += 1,
            "]" | ")" | "}" => depth = depth.saturating_sub(1),
            "=" if depth == 0 => return group[..index].to_vec(),
            _ => {}
        }
    }
    group.to_vec()
}

fn split_top_level<'a>(texts: &[&'a str], separator: &str) -> Vec<Vec<&'a str>> {
    let mut groups = vec![Vec::new()];
    let mut depth = 0usize;
    for text in texts {
        match *text {
            "[" | "(" | "{" => depth += 1,
            "]" | ")" | "}" => depth = depth.saturating_sub(1),
            _ => {}
        }
        if *text == separator && depth == 0 {
            groups.push(Vec::new());
            continue;
        }
        if let Some(last) = groups.last_mut() {
            last.push(*text);
        }
    }
    groups
}

/// Pointer bases dereferenced by the older direct scalar-field route.  The
/// existing total statement reader decides admission first; this walk only
/// recovers the exact `BASE -> FIELD` bases it accepted so flow can check their
/// alias and liveness obligations.  A direct record value's `BASE.FIELD` has
/// no pointer dereference and contributes no base.
pub(crate) fn scalar_field_noop_pointer_bases(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<Vec<String>> {
    (scalar_statement_accesses(texts, model, types)? > 0).then(|| {
        let mut bases = Vec::new();
        for window in texts.windows(4) {
            let [base, "-", ">", _field] = window else {
                continue;
            };
            if matches!(types.declared(base), Some(DeclaredType::RecordPointer(_)))
                && !bases.iter().any(|seen| seen == base)
            {
                bases.push((*base).to_owned());
            }
        }
        bases
    })
}

/// The base of one exactly-shaped scalar member store through a record pointer
/// this function may bind to a tracked identity.
///
/// This is deliberately disjoint from [`scalar_field_noop_pointer_bases`]'s premise 8.
/// That rule may erase an access only when the base can never carry custody;
/// this rule admits a bound base only as an explicit instruction whose
/// flow-sensitive transfer rechecks the base's live identity.  The syntax is
/// exactly `BASE -> FIELD = RHS`: one selection, one simple assignment, a
/// scalar field, and a scalar-only right-hand side with no access of its own.
/// Calls, member chains, subscripts, dereferences, pointer fields and a second
/// assignment all remain refused.
pub(crate) fn scalar_member_store_base<'a>(
    texts: &'a [&'a str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<&'a str> {
    let [base, "-", ">", field, "=", right @ ..] = texts else {
        return None;
    };
    if !is_name(base) || !is_name(field) || !types.may_bind(base) || types.function_pointer_formals
    {
        return None;
    }
    let Some(DeclaredType::RecordPointer(tag)) = types.declared_type(base) else {
        return None;
    };
    // Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: this route lowers to an
    // instruction whose flow check refuses a base that is not one settled
    // identity, and it runs before the statement's refusal reaches the later
    // fallbacks (the Clang-typed statement reading, rule 10k). A record the
    // reader now reads past a function-pointer member stays outside it, so
    // every statement keeps the route order it had: the reader repair only
    // admits, it never hands a decided statement to a refusing route.
    if !model.scalar_field_of_a_fully_read_record(tag, field)
        || right.is_empty()
        || right.contains(&"=")
        || right
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
        || scalar_statement_accesses(right, model, types) != Some(0)
    {
        return None;
    }
    Some(base)
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**: what one admitted typed scalar
/// access statement asks of the flow analysis.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TypedScalarPlan {
    /// Every name the statement dereferences — `*B`, `B[I]` or `B->F` — in
    /// first-occurrence order, without repeats. Each must pass the dereference
    /// obligation at the statement.
    pub(crate) bases: Vec<String>,
    /// The subset of `bases` read through `->`, whose C02-P-ALIAS may-alias
    /// obligation travels with the instruction exactly as a modeled read's does.
    pub(crate) record_bases: Vec<String>,
    /// The declared scalar variable the statement writes, when its destination
    /// is a plain name rather than an access.
    pub(crate) destination: Option<String>,
    /// Whether the statement writes through `*B` or `B[I]`. Such a write may
    /// reach any byte of the object `B` points into — a character-typed one
    /// can rewrite a pointer object's representation without undefined
    /// behaviour — so the flow analysis bounds its effect on bindings exactly
    /// as the refusal it replaces did. A member store `B->F` or `S.F` writes
    /// one scalar member and nothing else.
    pub(crate) writes_through_pointer: bool,
}

impl TypedScalarPlan {
    fn add_base(&mut self, base: &str, record: bool) {
        if !self.bases.iter().any(|known| known == base) {
            self.bases.push(base.to_owned());
        }
        if record && !self.record_bases.iter().any(|known| known == base) {
            self.record_bases.push(base.to_owned());
        }
    }
}

/// One access in operand position: the index just past it, the name it
/// dereferences (none for `S . F`) and whether that name is a record pointer.
struct TypedAccess<'a> {
    end: usize,
    base: Option<&'a str>,
    record: bool,
}

impl TypedAccess<'_> {
    /// `*B` or `B[I]`: an access through a scalar pointer or array, as opposed
    /// to a member selection.
    fn through_pointer(&self) -> bool {
        self.base.is_some() && !self.record
    }
}

/// Whether the token at `at` continues an access into something this rule
/// does not admit: another member, a subscript or a call.
fn continues_access(texts: &[&str], at: usize) -> bool {
    match texts.get(at).copied() {
        Some(".") | Some("[") | Some("(") => true,
        Some("-") => texts.get(at + 1) == Some(&">"),
        _ => false,
    }
}

/// Parse one typed scalar access at `index` (card
/// **W43-INDIRECT-ACCESS-GENERAL**). Every premise is a declaration fact this
/// function's reader committed to; nothing is inferred from spelling alone.
fn typed_access<'a>(
    texts: &[&'a str],
    index: usize,
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<TypedAccess<'a>> {
    let text = *texts.get(index)?;
    if text == "*" {
        let name = *texts.get(index + 1)?;
        if !is_name(name) || types.scalar_element(name) != Some(ScalarElement::Pointer) {
            return None;
        }
        let end = index + 2;
        // `* p ++` increments the pointer, not the object: never an access here.
        if continues_access(texts, end)
            || matches!(
                (texts.get(end).copied(), texts.get(end + 1).copied()),
                (Some("+"), Some("+")) | (Some("-"), Some("-"))
            )
        {
            return None;
        }
        return Some(TypedAccess {
            end,
            base: Some(name),
            record: false,
        });
    }
    if !is_name(text) {
        return None;
    }
    match (texts.get(index + 1).copied(), texts.get(index + 2).copied()) {
        (Some("-"), Some(">")) => {
            let field = *texts.get(index + 3)?;
            let Some(DeclaredType::RecordPointer(tag)) = types.declared(text) else {
                return None;
            };
            if !is_name(field) || !model.scalar_field(tag, field) {
                return None;
            }
            let end = index + 4;
            (!continues_access(texts, end)).then_some(TypedAccess {
                end,
                base: Some(text),
                record: true,
            })
        }
        (Some("."), _) => {
            let field = *texts.get(index + 2)?;
            let Some(DeclaredType::Record(tag)) = types.declared(text) else {
                return None;
            };
            if !is_name(field) || !model.scalar_field(tag, field) {
                return None;
            }
            let end = index + 3;
            (!continues_access(texts, end)).then_some(TypedAccess {
                end,
                base: None,
                record: false,
            })
        }
        (Some("["), _) => {
            types.scalar_element(text)?;
            let close = matching_close(texts, index + 1, "[", "]")?;
            let subscript = &texts[index + 2..close];
            // Card W43-INDEX-TRAVERSAL, a fail-closed repair: the index must
            // not write. `scalar_statement_accesses` admits `+`, `-` and `=`
            // as punctuation, so `out[m++] = c` was admitted with `m`'s old
            // branch fact kept, and a later `if (m)` could prune the branch C
            // takes. An index that increments, decrements or assigns now
            // refuses here; `crate::index_traversal::stepped_access` admits
            // the `[ X ++ ]` shape with the forget it needs.
            if subscript.is_empty()
                || scalar_statement_accesses(subscript, model, types) != Some(0)
                || subscript.contains(&"=")
                || subscript
                    .windows(2)
                    .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
            {
                return None;
            }
            let end = close + 1;
            (!continues_access(texts, end)).then_some(TypedAccess {
                end,
                base: Some(text),
                record: false,
            })
        }
        _ => None,
    }
}

/// Walk one scalar expression that may carry typed scalar accesses, adding
/// their bases to `plan`. Returns the number of accesses, or `None` on any
/// token outside the rule: a call, `&` in operand position, `,`, a lone `=`,
/// an increment, a pointer-valued name, or an unbalanced parenthesis.
fn typed_scalar_expression(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    plan: &mut TypedScalarPlan,
) -> Option<usize> {
    typed_operand_expression(texts, model, types, plan, false, &no_extension)
}

/// Card **W44-INDIRECT-CHAIN-ACCESS**: an opt-in reader for one access the
/// single-selection reader [`typed_access`] refuses, consulted by the walker
/// at every operand position and by the destination reader. Given the whole
/// run and a position it returns the index just past an access it admits, or
/// `None`. It records nothing here: the caller that supplied it rereads the
/// admitted run for the obligations its accesses carry
/// ([`crate::indirect_chain_access`]). Every pre-existing entry point passes
/// [`no_extension`], so each of their answers is unchanged.
pub(crate) type AccessExtension<'e> = dyn Fn(&[&str], usize) -> Option<usize> + 'e;

/// The extension every pre-existing entry point passes: it admits nothing.
fn no_extension(_: &[&str], _: usize) -> Option<usize> {
    None
}

/// The walker behind [`typed_scalar_expression`] and
/// [`typed_scalar_condition`]. `pointer_operands` additionally admits, as a
/// whole operand, a declared single-level pointer name ([`PointerKind`]) and
/// the null constant `( void * ) 0`: a **condition** compares or tests such a
/// value and discards it into a control-flow decision, which is exactly what
/// the lexical predicate already admits in `p < end` or `p != 0`. It reads the
/// pointer's value, never its object, so it carries no obligation. It is off
/// for every statement value, which must stay scalar.
fn typed_operand_expression(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    plan: &mut TypedScalarPlan,
    pointer_operands: bool,
    extension: &AccessExtension<'_>,
) -> Option<usize> {
    let mut accesses = 0usize;
    let mut expect_operand = true;
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < texts.len() {
        let text = texts[index];
        if expect_operand {
            if pointer_operands && texts[index..].starts_with(&["(", "void", "*", ")"]) {
                // The null constant's cast; its operand `0` follows.
                index += 4;
                continue;
            }
            // Card W44-INDIRECT-CHAIN-ACCESS: an access the opt-in extension
            // admits (tried first, so `( * p ) . f -> g` is not read as a
            // parenthesised operand). With `no_extension` this never fires.
            if let Some(end) = extension(texts, index) {
                accesses += 1;
                index = end;
                expect_operand = false;
                continue;
            }
            if text == "(" {
                let close = matching_close(texts, index, "(", ")")?;
                let inner = &texts[index + 1..close];
                // A cast to a scalar type name: its operand follows.
                if !inner.is_empty() && model.scalar_type_name(inner) {
                    index = close + 1;
                    continue;
                }
                depth += 1;
                index += 1;
                continue;
            }
            if matches!(text, "-" | "+" | "!" | "~") {
                if matches!(text, "-" | "+") && texts.get(index + 1) == Some(&text) {
                    return None;
                }
                index += 1;
                continue;
            }
            if is_numeric_literal(text) || text == "__qcore_literal__" {
                expect_operand = false;
                index += 1;
                continue;
            }
            if let Some(access) = typed_access(texts, index, model, types) {
                accesses += 1;
                if let Some(base) = access.base {
                    plan.add_base(base, access.record);
                }
                index = access.end;
                expect_operand = false;
                continue;
            }
            if is_name(text)
                && texts.get(index + 1) != Some(&"(")
                && (model.enum_constants.contains(text)
                    || types.declared(text) == Some(&DeclaredType::Scalar)
                    || (pointer_operands && types.pointer_kind(text).is_some()))
            {
                expect_operand = false;
                index += 1;
                continue;
            }
            return None;
        }
        if text == ")" {
            depth = depth.checked_sub(1)?;
            index += 1;
            continue;
        }
        match (text, texts.get(index + 1).copied().unwrap_or("")) {
            ("+", "+") | ("-", "-") => return None,
            ("=", "=")
            | ("!", "=")
            | ("<", "=")
            | (">", "=")
            | ("<", "<")
            | (">", ">")
            | ("&", "&")
            | ("|", "|") => {
                index += 2;
                expect_operand = true;
                continue;
            }
            _ => {}
        }
        if matches!(
            text,
            "+" | "-" | "*" | "/" | "%" | "<" | ">" | "^" | "|" | "&" | "?" | ":"
        ) {
            index += 1;
            expect_operand = true;
            continue;
        }
        return None;
    }
    (!expect_operand && depth == 0).then_some(accesses)
}

/// Strip parentheses that enclose the whole run.
fn strip_enclosing<'a, 'b>(mut texts: &'b [&'a str]) -> &'b [&'a str] {
    while texts.len() >= 2
        && texts[0] == "("
        && matching_close(texts, 0, "(", ")") == Some(texts.len() - 1)
    {
        texts = &texts[1..texts.len() - 1];
    }
    texts
}

/// Exactly one typed access spanning `texts`, after enclosing parentheses.
/// `dereferencing` additionally requires a base: an increment is admitted only
/// through a pointer, while an assignment may also store into `S . F`.
fn single_typed_access(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    plan: &mut TypedScalarPlan,
    dereferencing: bool,
    extension: &AccessExtension<'_>,
) -> Option<()> {
    let texts = strip_enclosing(texts);
    let Some(access) = typed_access(texts, 0, model, types) else {
        // Card W44-INDIRECT-CHAIN-ACCESS: a whole-run access the opt-in
        // extension admits. It selects a scalar member, so it writes one
        // scalar and never through `*B`/`B[I]`.
        if extension(texts, 0) != Some(texts.len()) {
            return None;
        }
        plan.writes_through_pointer = false;
        return Some(());
    };
    if access.end != texts.len() || (dereferencing && access.base.is_none()) {
        return None;
    }
    if let Some(base) = access.base {
        plan.add_base(base, access.record);
    }
    plan.writes_through_pointer = access.through_pointer();
    Some(())
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**: a complete statement whose every
/// memory access is a typed scalar access and whose destination and value are
/// scalar. The statement can neither acquire, release, transfer nor rebind a
/// tracked resource identity; what remains is the dereference obligation of
/// each base, which the flow analysis checks.
///
/// Admitted forms, over the statement's complete token text:
///
/// * `ACCESS = E` and `ACCESS op= E` (every compound operator, `<<=` and `>>=`
///   included), where `ACCESS` is `*B`, `B[I]`, `B->F` or `S.F`;
/// * `++ACCESS`, `--ACCESS`, `ACCESS++` and `ACCESS--` through a pointer,
///   never `*B++`;
/// * `X = E`, `X op= E` and `SCALAR_TYPE X = E` with `X` a declared scalar and
///   `E` carrying at least one access (`S.F` included).
///
/// `E` is built from numeric and character literals, declared scalars,
/// enumeration constants, casts to scalar type names and typed accesses, with
/// no call, no unary `&`, no comma, no nested assignment and no increment. An
/// access is `*B` (`B` declared `SCALAR *`), `B[I]` (`B` declared `SCALAR *` or
/// `SCALAR [N]`, `I` a scalar-only index), `B->F` (`B` declared `struct T *`,
/// `F` a scalar member of a completely read `struct T`) or `S.F` (`S` declared
/// `struct T`).
pub(crate) fn typed_scalar_statement(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<TypedScalarPlan> {
    typed_scalar_statement_with(texts, model, types, &no_extension)
}

/// Card **W44-INDIRECT-CHAIN-ACCESS**: [`typed_scalar_statement`] with an
/// opt-in reader for accesses the single-selection reader refuses, admitted
/// wherever a typed access is (destination, increment operand, value).
pub(crate) fn typed_scalar_statement_with(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    extension: &AccessExtension<'_>,
) -> Option<TypedScalarPlan> {
    types.variables.as_ref()?;
    if texts.is_empty()
        || texts
            .iter()
            .any(|text| matches!(*text, "{" | "}" | ";" | ","))
    {
        return None;
    }
    let mut plan = TypedScalarPlan::default();
    let Some((equal, operator)) = statement_assignment(texts) else {
        let body = match texts {
            ["+", "+", rest @ ..] | ["-", "-", rest @ ..] => rest,
            [rest @ .., "+", "+"] | [rest @ .., "-", "-"] => {
                // `* p ++` is `* (p ++)`: it increments the pointer.
                if rest.first() == Some(&"*") {
                    return None;
                }
                rest
            }
            _ => return None,
        };
        single_typed_access(body, model, types, &mut plan, true, extension)?;
        return Some(plan);
    };
    let left = &texts[..equal - operator];
    let right = &texts[equal + 1..];
    let lhs_access = single_typed_access(left, model, types, &mut plan, false, extension).is_some();
    if !lhs_access {
        plan = TypedScalarPlan::default();
        let (&name, specifiers) = left.split_last()?;
        if !is_name(name) || types.declared(name) != Some(&DeclaredType::Scalar) {
            return None;
        }
        if !specifiers.is_empty() && (operator != 0 || !model.scalar_type_name(specifiers)) {
            return None;
        }
        plan.destination = Some(name.to_owned());
    }
    let accesses = typed_operand_expression(right, model, types, &mut plan, false, extension)?;
    (lhs_access || accesses > 0).then_some(plan)
}

/// Card **W43-INDEX-TRAVERSAL**: a complete controlling expression (of an
/// `if`, `while`, `for`, `do`-`while` or `switch`) whose every memory access is
/// a typed scalar access, and which therefore reads memory and decides a branch
/// without writing, calling or binding anything.
///
/// Built by the same walker as a statement's value ([`typed_scalar_expression`])
/// with one widening that only a condition may have: a declared single-level
/// pointer name and the null constant are admitted as whole operands, so
/// `p != 0 && *p != ':'` and `p < end && *p` read like the lexical
/// predicate's `p < end`. No call, brace, comma, unary `&`, assignment or
/// increment is admitted, and at least one access is required (a condition
/// with none is the lexical predicate's, not this rule's). Every base the plan
/// names keeps its dereference obligation, which the flow analysis checks at
/// the header **even for an operand a short-circuit could skip**: requiring an
/// obligation that was not needed can only refuse more.
pub(crate) fn typed_scalar_condition(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<TypedScalarPlan> {
    typed_scalar_condition_with(texts, model, types, &no_extension)
}

/// Card **W44-INDIRECT-CHAIN-ACCESS**: [`typed_scalar_condition`] with the
/// same opt-in reader as [`typed_scalar_statement_with`].
pub(crate) fn typed_scalar_condition_with(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
    extension: &AccessExtension<'_>,
) -> Option<TypedScalarPlan> {
    types.variables.as_ref()?;
    if texts.is_empty()
        || texts
            .iter()
            .any(|text| matches!(*text, "{" | "}" | ";" | ","))
    {
        return None;
    }
    let mut plan = TypedScalarPlan::default();
    let accesses = typed_operand_expression(texts, model, types, &mut plan, true, extension)?;
    (accesses > 0).then_some(plan)
}

/// Card **W43-INDEX-TRAVERSAL**: whether `texts` is a scalar value with no
/// memory access at all — declared scalars, enumeration constants, numeric and
/// character literals, casts to scalar type names and arithmetic. The offset
/// of a pointer step must be one.
pub(crate) fn is_scalar_value_expression(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    types.variables.is_some()
        && !texts.is_empty()
        && !texts
            .iter()
            .any(|text| matches!(*text, "{" | "}" | ";" | ","))
        && typed_scalar_expression(texts, model, types, &mut TypedScalarPlan::default()) == Some(0)
}

/// Card **W44-INDIRECT-CALL-ARGUMENT-ACCESS**: one whole call actual that is a
/// scalar value carrying at least one typed scalar access — the value walker
/// of [`typed_scalar_statement`], unchanged, over the actual alone. `None` for
/// an unreadable function, an empty actual, a `{ } ; ,`, a call, a unary `&`,
/// an increment, an assignment, a pointer-valued name or no access at all.
/// The plan's bases carry exactly the typed route's dereference obligation.
pub(crate) fn typed_scalar_value(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<TypedScalarPlan> {
    types.variables.as_ref()?;
    if texts.is_empty()
        || texts
            .iter()
            .any(|text| matches!(*text, "{" | "}" | ";" | ","))
    {
        return None;
    }
    let mut plan = TypedScalarPlan::default();
    let accesses = typed_scalar_expression(texts, model, types, &mut plan)?;
    (accesses > 0).then_some(plan)
}

/// The statement's own assignment: the index of its top-level `=` and the
/// length of the compound operator before it (0 for `=`, 1 for `+=`, 2 for
/// `<<=`). `None` when there is no assignment at depth 0 or the first `=` at
/// depth 0 belongs to a comparison.
fn statement_assignment(texts: &[&str]) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" if depth == 0 => {
                let next = texts.get(index + 1).copied();
                let previous = index.checked_sub(1).map(|at| texts[at]);
                if next == Some("=") || matches!(previous, Some("=") | Some("!")) {
                    return None;
                }
                return match previous {
                    Some("<") | Some(">") => {
                        let before = index.checked_sub(2).map(|at| texts[at]);
                        (before == previous).then_some((index, 2))
                    }
                    Some("+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") => Some((index, 1)),
                    _ => Some((index, 0)),
                };
            }
            _ => {}
        }
    }
    None
}

/// Whether a statement operates only on declared scalar variables, enumeration
/// constants and numeric literals, with no member access at all.
///
/// This is the zero-access case of the same walker. A statement whose every
/// identifier is a *declared scalar* of this function (or an enumeration
/// constant), carrying no call, no `&`, no `*`, no subscript and no aggregate,
/// can neither create, destroy, alias nor transfer a tracked resource identity:
/// this projection tracks pointer identities only, and a scalar variable is
/// never bound to one. Without this rule the importer models `flag = 1;` as an
/// alias from an unbound place and reports an unresolved alias, which poisons
/// every path through it. O04 needs it because the selected caller's failure
/// arms are reached through exactly such scalar bookkeeping.
///
/// It is deliberately separate from [`scalar_field_noop_pointer_bases`], whose contract
/// (at least one admitted member access) is unchanged.
pub(crate) fn is_scalar_only_noop(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    // An empty statement is already a no-op for the importer; requiring at
    // least one token keeps this rule from claiming credit for that path.
    !texts.is_empty() && scalar_statement_accesses(texts, model, types) == Some(0)
}

/// Whether an assignment into a declared scalar variable has a right-hand side
/// that is itself an admitted scalar expression.
///
/// This is [`is_scalar_only_noop`] for the *declaration* form `int flag =
/// FLAG_OFF;`, whose leading type specifiers the statement walker refuses. The
/// destination must be a declared scalar of this function, so the store can
/// neither create nor discard a tracked identity, and the right-hand side must
/// pass the same walker as any other scalar-only statement.
pub(crate) fn is_scalar_initializer(
    destination: &str,
    right: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    types.declared(destination) == Some(&DeclaredType::Scalar)
        && !right.is_empty()
        && scalar_statement_accesses(right, model, types) == Some(0)
}

/// Whether an *expression* operand run is admitted by the same walker, with any
/// number of admitted direct member accesses including none.
///
/// Card W4-EXPR-POS reads the six expression positions of `legacy_c` through
/// this entry point (see [`crate::neutral_position`]). It is the union of
/// [`scalar_field_noop_pointer_bases`] and [`is_scalar_only_noop`]'s conditions minus their
/// disjoint access-count requirements: both counts are custody-neutral by the
/// same argument, and an expression position — unlike a statement — has no
/// reason to distinguish them. The empty run is admitted, so that an expression
/// consisting of exactly one admitted neutral call leaves nothing to reject;
/// the caller refuses an empty *expression* before ever reaching here.
pub(crate) fn is_scalar_operand_expression(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    scalar_statement_accesses(texts, model, types).is_some()
}

/// Whether one token is a punctuation mark this project's operand rules admit.
///
/// Exactly [`ADMITTED_PUNCTUATION`], read out so that card REACH-GATE's chain
/// walker in [`crate::neutral_position`] uses the *same* set rather than a copy
/// of it: `&`, `*`, `[`, `]`, `{`, `}`, `,` and `;` are absent there, so an
/// address, an indirection, a subscript, an aggregate and the comma operator
/// each refuse in both walkers by one decision, not two.
pub(crate) fn is_admitted_operand_punctuation(text: &str) -> bool {
    ADMITTED_PUNCTUATION.contains(&text)
}

/// Whether `text` is a numeric literal, read out for the same reason as
/// [`is_admitted_operand_punctuation`].
pub(crate) fn is_admitted_numeric_literal(text: &str) -> bool {
    is_numeric_literal(text)
}

/// The shared walker: `Some(count)` when every token of the statement is
/// admitted, counting admitted direct member accesses; `None` on any refusal.
fn scalar_statement_accesses(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<usize> {
    types.variables.as_ref()?;
    if texts
        .iter()
        .any(|text| matches!(*text, "&" | "*" | "[" | "]" | "{" | "}" | "," | ";"))
    {
        return None;
    }
    let mut accesses = 0usize;
    let mut cursor = 0usize;
    while cursor < texts.len() {
        let text = texts[cursor];
        if is_name(text) {
            let arrow = texts.get(cursor + 1) == Some(&"-") && texts.get(cursor + 2) == Some(&">");
            let dot = texts.get(cursor + 1) == Some(&".");
            if arrow || dot {
                let field_index = if arrow { cursor + 3 } else { cursor + 2 };
                let field = texts
                    .get(field_index)
                    .copied()
                    .filter(|field| is_name(field))?;
                // Premise 8: a base the projection may bind can hold a
                // released, escaped or reallocated identity, and a read
                // through it is not something this rule may neutralize.
                if types.bound.contains(text) {
                    return None;
                }
                let tag = match (types.declared(text), arrow) {
                    (Some(DeclaredType::RecordPointer(tag)), true) => tag,
                    (Some(DeclaredType::Record(tag)), false) => tag,
                    _ => return None,
                };
                if !model.scalar_field(tag, field) {
                    return None;
                }
                // The access must not continue into another member, a
                // subscript or a call.
                match texts.get(field_index + 1) {
                    Some(&".") | Some(&"[") | Some(&"(") => return None,
                    Some(&"-") if texts.get(field_index + 2) == Some(&">") => return None,
                    _ => {}
                }
                accesses += 1;
                cursor = field_index + 1;
                continue;
            }
            // A remaining identifier must be an enumeration constant or a
            // declared scalar variable. Type names, casts and every
            // pointer-valued operand reject here.
            if model.enum_constants.contains(text) {
                cursor += 1;
                continue;
            }
            if types.declared(text) != Some(&DeclaredType::Scalar) {
                return None;
            }
            cursor += 1;
            continue;
        }
        if is_numeric_literal(text) {
            cursor += 1;
            continue;
        }
        if text == "-" && texts.get(cursor + 1) == Some(&">") {
            // An arrow whose base this reader did not admit.
            return None;
        }
        if !ADMITTED_PUNCTUATION.contains(&text) {
            return None;
        }
        cursor += 1;
    }
    Some(accesses)
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the integer class of one
/// specifier list — signedness and conversion rank (`char` 1, `short` 2,
/// `int` 3, `long` 4, `long long` 5) — resolved through this unit's typedefs
/// exactly as [`TypedAccessModel::classify_specifiers`] resolves them.
///
/// `None` for everything else: a floating, `_Bool`, enumeration, pointer,
/// record, `volatile` or `_Atomic` type, a plain `char` (whose signedness is
/// implementation-defined), and every name this reader does not resolve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IntegerClass {
    pub(crate) signed: bool,
    pub(crate) rank: u8,
}

impl TypedAccessModel {
    pub(crate) fn integer_class(&self, specifiers: &[&str]) -> Option<IntegerClass> {
        self.integer_class_at(specifiers, 0)
    }

    fn integer_class_at(&self, specifiers: &[&str], depth: usize) -> Option<IntegerClass> {
        if specifiers
            .iter()
            .any(|text| REJECTING_QUALIFIERS.contains(text))
        {
            return None;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        if let [name] = core.as_slice()
            && is_name(name)
            && !ARITHMETIC_KEYWORDS.contains(name)
        {
            if depth >= TYPEDEF_DEPTH_LIMIT {
                return None;
            }
            return match self.typedefs.get(*name) {
                Some(Some(TypedefTarget::Specifiers(target))) => {
                    let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                    self.integer_class_at(&target, depth + 1)
                }
                _ => None,
            };
        }
        if core.is_empty()
            || !core.iter().all(|text| {
                matches!(
                    *text,
                    "char" | "short" | "int" | "long" | "signed" | "unsigned"
                )
            })
        {
            return None;
        }
        let count = |word: &str| core.iter().filter(|text| **text == word).count();
        let unsigned = count("unsigned");
        let signed = count("signed");
        if unsigned + signed > 1 {
            return None;
        }
        let rank = match (count("char"), count("short"), count("long"), count("int")) {
            (1, 0, 0, 0) if unsigned + signed == 1 => 1,
            (0, 1, 0, 0 | 1) => 2,
            (0, 0, 0, 0 | 1) => 3,
            (0, 0, 1, 0 | 1) => 4,
            (0, 0, 2, 0 | 1) => 5,
            _ => return None,
        };
        Some(IntegerClass {
            signed: unsigned == 0,
            rank,
        })
    }
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: one declarator of a function's
/// formals or locals, as read by the same total-or-reject reader
/// [`FunctionTypes::read`] uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeclaratorFact {
    pub(crate) specifiers: Vec<String>,
    pub(crate) name: String,
    pub(crate) stars: usize,
    pub(crate) dimensions: usize,
}

/// Every named declarator of one function's parameter list and body, or
/// `None` exactly where [`FunctionTypes::read`] would refuse the function.
/// It walks the same statements with the same helpers; it classifies
/// nothing, so no existing answer of this module changes.
pub(crate) fn declarator_facts(parameters: &[&str], body: &[&str]) -> Option<Vec<DeclaratorFact>> {
    let mut facts = Vec::new();
    let mut push = |specifiers: &[&str], declarator: Declarator| {
        if let Some(name) = declarator.name {
            facts.push(DeclaratorFact {
                specifiers: specifiers.iter().map(|text| (*text).to_owned()).collect(),
                name,
                stars: declarator.stars,
                dimensions: declarator.dimensions,
            });
        }
    };
    if parameters
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "." | ";" | "="))
    {
        return None;
    }
    if !(parameters.is_empty() || parameters == ["void"]) {
        for group in parameters.split(|text| *text == ",") {
            let (specifiers, declarator) = split_declarator(group)?;
            push(&specifiers, declarator);
        }
    }
    for start in 0..body.len() {
        if start > 0 && !matches!(body[start - 1], "{" | "}" | ";" | "(" | ":") {
            continue;
        }
        if !is_declaration_candidate(body, start) {
            continue;
        }
        let end = statement_end(body, start)?;
        let mut groups = split_top_level(&body[start..end], ",").into_iter();
        let first = truncate_at_initializer(&groups.next()?);
        if unreadable_declarator_token(&first).is_some() {
            return None;
        }
        let (specifiers, declarator) = split_declarator(&first)?;
        push(&specifiers, declarator);
        for group in groups {
            let group = truncate_at_initializer(&group);
            if unreadable_declarator_token(&group).is_some() {
                return None;
            }
            push(&specifiers, read_declarator(&group)?);
        }
    }
    Some(facts)
}

/// Card **W44-COLLECTION-ALLOCATION**: whether a specifier list names a type
/// that cannot be variably modified, read through this unit's typedefs.
///
/// Only an ordinary identifier with block or prototype scope may have a
/// variably modified type (C11 6.7.6.2p2), and a member may not (6.7.2.1p9).
/// So an arithmetic type, `void`, a tagged `struct`/`union`/`enum`, and a
/// typedef resolving to one of these — including a typedef that defines an
/// aggregate — never is. [`TypedAccessModel::read_typedefs`] reads every
/// typedef of the unit, block-scope ones included, and records an array,
/// pointer, function or disagreeing typedef unresolved, so a block-scope VLA
/// typedef (and anything defined through one) answers `false` here, as does
/// every name the reader does not resolve. `volatile`, `_Atomic` and a
/// `typedef` storage class are refused to stay inside the reader's own
/// committed forms. Consulted only by [`crate::sizeof_operand`].
impl TypedAccessModel {
    pub(crate) fn never_variably_modified(&self, specifiers: &[&str]) -> bool {
        self.never_variably_modified_at(specifiers, 0)
    }

    fn never_variably_modified_at(&self, specifiers: &[&str], depth: usize) -> bool {
        if specifiers
            .iter()
            .any(|text| REJECTING_QUALIFIERS.contains(text) || *text == "typedef")
        {
            return false;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        match core.as_slice() {
            ["struct" | "union" | "enum", tag] => is_name(tag),
            [name] if is_name(name) => {
                if depth >= TYPEDEF_DEPTH_LIMIT {
                    return false;
                }
                match self.typedefs.get(*name) {
                    Some(Some(TypedefTarget::Specifiers(target))) => {
                        let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                        self.never_variably_modified_at(&target, depth + 1)
                    }
                    Some(Some(TypedefTarget::Aggregate(head))) => matches!(
                        head.split(' ').next(),
                        Some("struct") | Some("union") | Some("enum")
                    ),
                    _ => false,
                }
            }
            rest => {
                !rest.is_empty()
                    && rest
                        .iter()
                        .all(|text| ARITHMETIC_KEYWORDS.contains(text) || *text == "void")
            }
        }
    }
}

/// Card **W45-RETURN-EXPRESSION-FAMILIES**: whether `specifiers` name a
/// **scalar value type** — an arithmetic or enumeration type, or a typedef
/// this reader resolves to one — with no pointer level. It is
/// [`TypedAccessModel::classify_specifiers`]' `Scalar` class plus one reading
/// that class deliberately leaves `Opaque`: an enumeration a typedef itself
/// defines (`typedef enum { … } rd_kafka_resp_err_t;`), whose head is read
/// rather than assumed. A `struct`/`union` a typedef defines, a pointer
/// typedef, an unresolved or disagreeing name, `void`, `volatile` and
/// `_Atomic` all answer `false`. Consulted only by
/// [`crate::legacy_c::return_family`], for the declared result of the
/// function whose `return` it decides.
impl TypedAccessModel {
    pub(crate) fn is_scalar_value_type(&self, specifiers: &[&str]) -> bool {
        self.scalar_value_type_at(specifiers, 0)
    }

    fn scalar_value_type_at(&self, specifiers: &[&str], depth: usize) -> bool {
        if self.classify_specifiers(specifiers, depth) == DeclaredType::Scalar {
            return true;
        }
        if specifiers
            .iter()
            .any(|text| REJECTING_QUALIFIERS.contains(text) || *text == "typedef")
        {
            return false;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        let [name] = core.as_slice() else {
            return false;
        };
        if !is_name(name) || depth >= TYPEDEF_DEPTH_LIMIT {
            return false;
        }
        match self.typedefs.get(*name) {
            Some(Some(TypedefTarget::Aggregate(head))) => head.split(' ').next() == Some("enum"),
            Some(Some(TypedefTarget::Specifiers(target))) => {
                let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                self.scalar_value_type_at(&target, depth + 1)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Split like the importer's own tokenizer: identifiers, numbers and one
    /// character per punctuation token, so `->` is `-` followed by `>`.
    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphabetic() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else if byte.is_ascii_digit() {
                let start = cursor;
                while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    fn model(source: &str) -> TypedAccessModel {
        let tokens = texts(source);
        TypedAccessModel::read(&tokens)
    }

    /// Card **ALIAS-AGGREGATE**: the opaque-member class admits the literal
    /// one-token `void` specifier list with exactly one star, and nothing else.
    /// Every neighbouring spelling is asserted refused in the same record, so
    /// the class cannot widen by accident.
    #[test]
    fn only_the_exact_void_star_spelling_is_an_opaque_pointer_member() {
        let model = model(
            "typedef void * handle ; \
             struct r { void * plain ; const void * ro ; volatile void * vol ; \
             void * * deep ; void * arr [ 4 ] ; handle aliased ; \
             struct q * typed ; int flag ; } ;",
        );
        assert!(model.void_pointer_field("r", "plain"));
        for field in ["ro", "vol", "deep", "arr", "aliased", "typed", "flag"] {
            assert!(!model.void_pointer_field("r", field), "{field}");
        }
        // The three classes are disjoint on every member of the record.
        for field in [
            "plain", "ro", "vol", "deep", "arr", "aliased", "typed", "flag",
        ] {
            let classes = usize::from(model.scalar_field("r", field))
                + usize::from(model.single_pointer_field("r", field))
                + usize::from(model.void_pointer_field("r", field));
            assert!(classes <= 1, "{field} is in {classes} classes");
        }
        assert!(model.single_pointer_field("r", "typed"));
        assert!(model.scalar_field("r", "flag"));
    }

    /// An opaque member of a record this reader refuses — a `union`, a
    /// multiply-defined tag or an incomplete body — is not admitted either.
    #[test]
    fn an_opaque_member_of_a_refused_record_is_not_admitted() {
        assert!(!model("union r { void * plain ; } ;").void_pointer_field("r", "plain"));
        assert!(
            !model("struct r { void * plain ; } ; struct r { void * plain ; } ;")
                .void_pointer_field("r", "plain")
        );
        assert!(
            !model("struct r { void * plain ; int bits : 3 ; } ;").void_pointer_field("r", "plain")
        );
    }

    #[test]
    fn a_scalar_member_of_a_complete_struct_is_read() {
        let model = model("struct r { int a ; struct q * p ; char b ; } ;");
        assert!(model.scalar_field("r", "a"));
        assert!(model.scalar_field("r", "b"));
        assert!(!model.scalar_field("r", "p"));
    }

    #[test]
    fn a_union_tag_is_never_admitted() {
        let model = model("union u { int a ; void * p ; } ;");
        assert!(!model.scalar_field("u", "a"));
    }

    #[test]
    fn a_bitfield_refuses_the_whole_record() {
        let model = model("struct r { int a ; unsigned b : 1 ; } ;");
        assert!(!model.scalar_field("r", "a"));
    }

    #[test]
    fn a_nested_anonymous_aggregate_refuses_the_whole_record() {
        let model = model("struct r { int a ; union { int x ; void * p ; } ; } ;");
        assert!(!model.scalar_field("r", "a"));
    }

    /// Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS (amends the former
    /// `a_function_pointer_member_refuses_the_whole_record`): a member that is
    /// exactly a function-pointer declarator joins no class, and every other
    /// member is read as in the same record without it.
    #[test]
    fn a_function_pointer_member_joins_no_class_and_the_rest_is_read() {
        let with = model(
            "struct r { int a ; void ( * f ) ( int , void ( * g ) ( void ) ) ; \
             struct q * p ; void * v ; struct q s ; } ;",
        );
        let without = model("struct r { int a ; struct q * p ; void * v ; struct q s ; } ;");
        for field in ["a", "p", "v", "s", "f", "g"] {
            assert_eq!(
                with.scalar_field("r", field),
                without.scalar_field("r", field),
                "{field}"
            );
            assert_eq!(
                with.borrowable_field("r", field),
                without.borrowable_field("r", field),
                "{field}"
            );
            assert_eq!(
                with.single_pointer_field("r", field),
                without.single_pointer_field("r", field),
                "{field}"
            );
            assert_eq!(
                with.void_pointer_field("r", field),
                without.void_pointer_field("r", field),
                "{field}"
            );
            assert_eq!(
                with.addressable_field("r", field),
                without.addressable_field("r", field),
                "{field}"
            );
            assert_eq!(
                with.record_pointer_field("r", field),
                without.record_pointer_field("r", field),
                "{field}"
            );
        }
        assert!(with.scalar_field("r", "a"));
        assert_eq!(with.record_pointer_field("r", "p"), Some("q"));
        assert!(with.record_definition_is_complete("r"));
        // Rule 5g's flow-refusing store keeps its domain of records read with
        // every member.
        assert!(!with.scalar_field_of_a_fully_read_record("r", "a"));
        assert!(without.scalar_field_of_a_fully_read_record("r", "a"));
        // The function pointer itself is in no class: every access keeps its
        // refusal.
        assert!(!with.borrowable_field("r", "f"));
        assert!(!with.addressable_field("r", "f"));
    }

    /// Every other parenthesised member declaration still refuses the whole
    /// record, as does any `{`, `}`, `:` or `=` beside a function pointer.
    #[test]
    fn any_other_parenthesised_member_still_refuses_the_whole_record() {
        for member in [
            "int ( * f [ 2 ] ) ( void ) ;",
            "int ( * * f ) ( void ) ;",
            "int ( * const f ) ( void ) ;",
            "int ( * f ) ( void ) , b ;",
            "int ( f ) ;",
            "int ( * f ) ;",
            "int ( * f ) ( void ) [ 2 ] ;",
            "int x __attribute__ ( ( aligned ( 8 ) ) ) ;",
            "void ( * f ) ( int ) ; unsigned b : 1 ;",
            "void ( * f ) ( int ) ; union { int x ; } ;",
        ] {
            let model = model(&format!("struct r {{ int a ; {member} }} ;"));
            assert!(!model.scalar_field("r", "a"), "{member}");
            assert!(!model.record_definition_is_complete("r"), "{member}");
        }
    }

    /// The census's `refused_on` is the reader's own answer.
    #[test]
    fn the_record_body_refusal_is_the_readers_answer() {
        let body = |source: &'static str| texts(source);
        assert_eq!(
            record_body_refusal(&body("int a ; void ( * f ) ( int ) ;")),
            None
        );
        assert_eq!(
            record_body_refusal(&body("int a ; int ( * f [ 2 ] ) ( void ) ;")),
            Some("(")
        );
        assert_eq!(
            record_body_refusal(&body("void ( * f ) ( int ) ; int b : 1 ;")),
            Some(":")
        );
    }

    /// A function-pointer formal is a name of no committed type; the
    /// function's other declarations are read.
    #[test]
    fn a_function_pointer_formal_is_opaque_and_the_function_is_read() {
        let model = model("struct r { int a ; } ;");
        let types = types(
            &model,
            "struct r * p , void ( * cb ) ( struct r * , int ( * g ) ( void ) ) , int n",
            "int t ; t = n ; p -> a = t ;",
        );
        assert!(types.is_readable());
        assert_eq!(types.declared("cb"), Some(&DeclaredType::Opaque));
        assert_eq!(types.scalar_element("cb"), None);
        assert_eq!(
            types.declared("p"),
            Some(&DeclaredType::RecordPointer("r".into()))
        );
        assert_eq!(types.declared("n"), Some(&DeclaredType::Scalar));
        assert_eq!(types.declared("t"), Some(&DeclaredType::Scalar));
        // The inner parameter name of the function pointer declares nothing.
        assert_eq!(types.declared("g"), None);
    }

    /// Rule 5g's flow-refusing store keeps its domain: a function read past a
    /// function-pointer formal, or a record read past a function-pointer
    /// member, is outside it, so the statement keeps its base route order.
    #[test]
    fn rule_5g_keeps_its_domain_of_fully_read_declarations() {
        let model = model("struct r { int a ; } ; struct s { int a ; void ( * f ) ( void ) ; } ;");
        let statement = texts("p -> a = 1");
        let plain = types(&model, "struct r * p", "p = g ( ) ; p -> a = 1 ;");
        assert!(plain.may_bind("p"));
        assert_eq!(
            scalar_member_store_base(&statement, &model, &plain),
            Some("p")
        );
        let with_formal = types(
            &model,
            "struct r * p , void ( * cb ) ( void )",
            "p = g ( ) ; p -> a = 1 ;",
        );
        assert!(with_formal.is_readable());
        assert_eq!(
            scalar_member_store_base(&statement, &model, &with_formal),
            None
        );
        let with_member = types(&model, "struct s * p", "p = g ( ) ; p -> a = 1 ;");
        assert!(model.scalar_field("s", "a"));
        assert_eq!(
            scalar_member_store_base(&statement, &model, &with_member),
            None
        );
    }

    /// Every other parenthesised formal still refuses the function.
    #[test]
    fn any_other_parenthesised_formal_still_refuses_the_function() {
        let model = model("struct r { int a ; } ;");
        for parameters in [
            "struct r * p , int ( * cb [ 2 ] ) ( void )",
            "struct r * p , int ( * * cb ) ( void )",
            "struct r * p , int ( cb )",
            "struct r * p , int ( * cb ) ( int x = 1 )",
        ] {
            let types = types(&model, parameters, "p -> a = 1 ;");
            assert!(!types.is_readable(), "{parameters}");
        }
    }

    #[test]
    fn a_volatile_or_atomic_member_is_not_scalar() {
        let model = model("struct r { volatile int a ; _Atomic int b ; int c ; } ;");
        assert!(!model.scalar_field("r", "a"));
        assert!(!model.scalar_field("r", "b"));
        assert!(model.scalar_field("r", "c"));
    }

    #[test]
    fn an_array_member_is_not_scalar() {
        let model = model("struct r { char n [ 8 ] ; int a ; } ;");
        assert!(!model.scalar_field("r", "n"));
        assert!(model.scalar_field("r", "a"));
    }

    #[test]
    fn two_definitions_of_one_tag_refuse_it() {
        let model = model("struct r { int a ; } ; struct r { int a ; } ;");
        assert!(!model.scalar_field("r", "a"));
    }

    #[test]
    fn a_scalar_typedef_resolves_but_a_pointer_typedef_does_not() {
        let model = model(
            "typedef unsigned long u64 ; typedef u64 alias ; typedef void * handle ; \
             struct r { alias a ; handle h ; } ;",
        );
        assert!(model.scalar_field("r", "a"));
        assert!(!model.scalar_field("r", "h"));
    }

    #[test]
    fn enumeration_constants_are_collected() {
        let model = model("enum e { E_A , E_B = 3 , E_C } ;");
        assert!(model.enum_constants.contains("E_A"));
        assert!(model.enum_constants.contains("E_B"));
        assert!(model.enum_constants.contains("E_C"));
    }

    fn types(model: &TypedAccessModel, parameters: &str, body: &str) -> FunctionTypes {
        FunctionTypes::read(
            model,
            &texts(parameters),
            &texts(body),
            &BTreeSet::from(["protected".to_owned()]),
        )
    }

    #[test]
    fn a_pointer_formal_and_a_scalar_local_are_typed() {
        let model = model("struct r { int a ; } ;");
        let types = types(&model, "struct r * p , int n", "int t ; t = n ;");
        assert_eq!(
            types.declared("p"),
            Some(&DeclaredType::RecordPointer("r".into()))
        );
        assert_eq!(types.declared("n"), Some(&DeclaredType::Scalar));
        assert_eq!(types.declared("t"), Some(&DeclaredType::Scalar));
    }

    #[test]
    fn an_unreadable_local_declaration_refuses_the_whole_function() {
        let model = model("struct r { int a ; } ;");
        let types = types(
            &model,
            "struct r * p",
            "void ( * cb ) ( int ) ; p -> a = 1 ;",
        );
        assert!(types.variables.is_none());
    }

    #[test]
    fn a_shadowing_redeclaration_of_a_different_type_makes_the_name_unusable() {
        let model = model("struct r { int a ; } ; struct s { void * a ; } ;");
        let types = types(&model, "struct r * p", "{ struct s * p ; }");
        assert_eq!(types.declared("p"), Some(&DeclaredType::Opaque));
    }

    #[test]
    fn a_for_loop_declaration_is_read_like_any_other() {
        let model = model("struct r { int a ; } ;");
        let types = types(
            &model,
            "struct r * p",
            "for ( int i = 0 ; i < 2 ; i ++ ) { }",
        );
        assert_eq!(types.declared("i"), Some(&DeclaredType::Scalar));
    }

    fn admits(records: &str, parameters: &str, body: &str, statement: &str) -> bool {
        let model = model(records);
        let types = types(&model, parameters, body);
        scalar_field_noop_pointer_bases(&texts(statement), &model, &types).is_some()
    }

    const R: &str = "struct r { int a ; int b ; struct q * p ; } ; struct q { int a ; } ;";

    #[test]
    fn a_direct_scalar_field_store_is_admitted() {
        assert!(admits(R, "struct r * s , int n", "", "s -> a = 1"));
        assert!(admits(R, "struct r * s , int n", "", "s -> a = n"));
        assert!(admits(R, "struct r * s , int n", "", "s -> a = s -> b + 1"));
    }

    #[test]
    fn a_direct_record_value_field_store_is_admitted() {
        assert!(admits(R, "int n", "struct r v ;", "v . a = n"));
    }

    #[test]
    fn a_pointer_field_a_chain_and_a_subscript_are_refused() {
        assert!(!admits(R, "struct r * s", "", "s -> p = 0"));
        assert!(!admits(R, "struct r * s", "", "s -> p -> a = 1"));
        assert!(!admits(R, "struct r * s , int i", "", "s -> a = i [ 0 ]"));
    }

    #[test]
    fn an_address_escape_and_a_dereference_are_refused() {
        assert!(!admits(R, "struct r * s , int * o", "", "o = & s -> a"));
        assert!(!admits(R, "struct r * s , int * o", "", "* o = s -> a"));
    }

    #[test]
    fn a_cast_operand_is_refused() {
        assert!(!admits(
            R,
            "struct r * s , int n",
            "",
            "s -> a = ( long ) n"
        ));
    }

    #[test]
    fn a_statement_with_no_access_is_not_this_rule() {
        assert!(!admits(R, "int n", "", "n = 1"));
    }

    #[test]
    fn an_undeclared_base_is_refused() {
        assert!(!admits(R, "int n", "", "s -> a = 1"));
    }

    #[test]
    fn a_compound_assignment_and_a_scalar_destination_are_admitted() {
        assert!(admits(R, "struct r * s", "", "s -> a += 1"));
        assert!(admits(R, "struct r * s , int n", "", "n = s -> a"));
    }

    #[test]
    fn a_base_the_projection_may_bind_is_refused() {
        // Written, address-taken, incremented, or handed to a call: each is a
        // way the projection can come to hold a released identity for the name.
        for body in [
            "s = 0 ;",
            "t = & s ;",
            "s ++ ;",
            "s -= 1 ;",
            "release ( s ) ;",
            "release ( wrap ( s ) ) ;",
        ] {
            assert!(
                !admits(R, "struct r * s", body, "s -> a = 1"),
                "a base bound by `{body}` must be refused"
            );
        }
        // A protected formal is bound at entry by the contract in force.
        let model = model(R);
        let types = FunctionTypes::read(
            &model,
            &texts("struct r * protected"),
            &texts(""),
            &BTreeSet::from(["protected".to_owned()]),
        );
        assert!(
            scalar_field_noop_pointer_bases(&texts("protected -> a = 1"), &model, &types).is_none()
        );
    }

    #[test]
    fn a_bound_base_has_a_separate_exact_scalar_store_route() {
        let model = model(R);
        let types = FunctionTypes::read(
            &model,
            &texts("struct r * s , int n"),
            &texts("s = make ( ) ;"),
            &BTreeSet::new(),
        );
        assert_eq!(
            scalar_member_store_base(&texts("s -> a = n + 1"), &model, &types),
            Some("s")
        );
        for refused in [
            "s -> p = n",
            "s -> a += n",
            "s -> a = s -> a",
            "s -> a = n [ 0 ]",
            "s -> a = * n",
            "s -> a = n ++",
            "s -> a = ( n = 1 )",
        ] {
            assert_eq!(
                scalar_member_store_base(&texts(refused), &model, &types),
                None,
                "refused: {refused}"
            );
        }
    }

    #[test]
    fn a_comparison_does_not_bind_the_base() {
        // `!=`, `==`, `<=` and `>=` are reads, not writes: the common
        // null-guard shape must stay admissible.
        for body in ["if ( s != 0 ) { }", "if ( s == 0 ) { }"] {
            assert!(
                admits(R, "struct r * s", body, "s -> a = 1"),
                "a comparison in `{body}` must not bind the base"
            );
        }
    }

    /// Card **CONSTRUCTOR-RESIDUALS**: the scalar-pointer return class admits a
    /// single `*` over a specifier list resolving to `Scalar`, and nothing else.
    /// Every neighbouring spelling is asserted refused in the same unit, so the
    /// class cannot widen by accident.
    #[test]
    fn only_a_single_star_scalar_return_is_a_scalar_pointer_result() {
        let model = model(
            "typedef int alias ; typedef int ( * fnptr ) ( void ) ; \
             struct q { int f ; } ; \
             int * plain ( void ) ; alias * through ( void ) ; \
             int * * deep ( void ) ; struct q * rec ( void ) ; void * opaque ( void ) ; \
             volatile int * hot ( void ) ; int flat ( void ) ; \
             fnptr * unresolved ( void ) ; enum e * tag ( void ) ;",
        );
        for admitted in ["plain", "through", "tag"] {
            assert!(model.scalar_pointer_result(admitted), "{admitted}");
        }
        for refused in [
            "deep",
            "rec",
            "opaque",
            "hot",
            "flat",
            "unresolved",
            "absent",
        ] {
            assert!(!model.scalar_pointer_result(refused), "{refused}");
        }
    }

    /// A second, disagreeing declaration retires the fact rather than competing
    /// with it. Two disagreeing declarations are not writable in valid C, so
    /// this direction is pinned here rather than in the Clang-backed family.
    #[test]
    fn a_disagreeing_second_declaration_retires_the_scalar_pointer_result() {
        assert!(model("int * f ( void ) ;").scalar_pointer_result("f"));
        assert!(!model("int * f ( void ) ; struct q * f ( void ) ;").scalar_pointer_result("f"));
        assert!(!model("struct q * f ( void ) ; int * f ( void ) ;").scalar_pointer_result("f"));
    }

    /// The reader's precision guard: an expression carrying the declarator
    /// shape is not a declaration, so it neither records nor retires a fact.
    #[test]
    fn an_expression_with_the_declarator_shape_does_not_retire_the_fact() {
        for expression in [
            "int * f ( void ) ; int g ( void ) { return * f ( ) ; }",
            "int * f ( void ) ; int a ; int g ( void ) { int x ; x = a * f ( ) ; return x ; }",
        ] {
            assert!(model(expression).scalar_pointer_result("f"), "{expression}");
        }
    }

    /// Attribute suffixes stand between the parameter list and the `;` in
    /// glibc's own declaration of `__errno_location`, after preprocessing.
    #[test]
    fn an_attribute_suffix_does_not_hide_the_declaration() {
        let model = model(
            "extern int * __errno_location ( void ) \
             __attribute__ ( ( __nothrow__ , __leaf__ ) ) __attribute__ ( ( __const__ ) ) ;",
        );
        assert!(model.scalar_pointer_result("__errno_location"));
    }

    #[test]
    fn an_arrow_on_a_record_value_and_a_dot_on_a_pointer_are_refused() {
        assert!(!admits(R, "int n", "struct r v ;", "v -> a = n"));
        assert!(!admits(R, "struct r * s , int n", "", "s . a = n"));
    }
}
