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
    /// Members declared with **no** `*`, no array modifier and no rejecting
    /// qualifier, over a specifier list this reader resolved to an
    /// arithmetic/enumeration type or to a `struct` tag. Such a member is a
    /// sub-object of the record that can never itself hold a pointer this
    /// projection links, so its *address* denotes storage disjoint from every
    /// linkable member. `union`-typed members, pointer members, arrays,
    /// bitfields and unresolved typedefs are absent by construction. Used only
    /// by the member-address rule in [`crate::aggregate_store`].
    non_pointer_fields: BTreeSet<String>,
}

/// The four member classifications one readable record body yields.
#[derive(Clone, Debug, Default)]
struct RecordMembers {
    scalar_fields: BTreeSet<String>,
    borrowable_fields: BTreeSet<String>,
    record_pointer_fields: BTreeMap<String, String>,
    single_pointer_fields: BTreeSet<String>,
    non_pointer_fields: BTreeSet<String>,
}

/// Declaration facts of one preprocessed translation unit.
#[derive(Clone, Debug, Default)]
pub(crate) struct TypedAccessModel {
    records: BTreeMap<String, RecordInfo>,
    /// `None` marks a name this reader refuses to resolve.
    typedefs: BTreeMap<String, Option<Vec<String>>>,
    enum_constants: BTreeSet<String>,
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
        model
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
                None
            } else {
                Some(specifiers.iter().map(|text| (*text).to_owned()).collect())
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
                entry.non_pointer_fields.clear();
                continue;
            }
            if entry.union_tag {
                continue;
            }
            let body = texts[index + 3..close].to_vec();
            if let Some(members) = self.read_record_body(&body) {
                let entry = self.records.entry(tag.to_owned()).or_default();
                entry.complete = true;
                entry.scalar_fields = members.scalar_fields;
                entry.borrowable_fields = members.borrowable_fields;
                entry.record_pointer_fields = members.record_pointer_fields;
                entry.single_pointer_fields = members.single_pointer_fields;
                entry.non_pointer_fields = members.non_pointer_fields;
            }
        }
    }

    /// Read every member declaration of one record body, or refuse the record.
    ///
    /// The result is all-or-nothing on purpose: a body carrying a bitfield, a
    /// nested anonymous aggregate, a function-pointer member or any declarator
    /// this reader cannot resolve yields `None`, and the whole record is then
    /// inadmissible. Reading only the members it understands would risk
    /// attributing a scalar type to a name it had actually misparsed.
    fn read_record_body(&self, body: &[&str]) -> Option<RecordMembers> {
        if body
            .iter()
            .any(|text| matches!(*text, "{" | "}" | "(" | ")" | ":" | "="))
        {
            return None;
        }
        let mut scalar_fields = BTreeSet::new();
        let mut borrowable_fields = BTreeSet::new();
        let mut record_pointer_fields = BTreeMap::new();
        let mut single_pointer_fields = BTreeSet::new();
        let mut non_pointer_fields = BTreeSet::new();
        for member in body.split(|text| *text == ";") {
            if member.is_empty() {
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
                // linked pointer member. `union` specifiers and every typedef
                // this reader will not resolve classify `Opaque` and are
                // excluded here, exactly as they are elsewhere in this model.
                if declarator.stars == 0
                    && !qualified
                    && matches!(
                        self.classify_specifiers(&specifiers, 0),
                        DeclaredType::Record(_) | DeclaredType::Scalar
                    )
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
                    borrowable_fields.insert(name);
                }
            }
        }
        Some(RecordMembers {
            scalar_fields,
            borrowable_fields,
            record_pointer_fields,
            single_pointer_fields,
            non_pointer_fields,
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
        if specifiers
            .iter()
            .any(|text| REJECTING_QUALIFIERS.contains(text))
        {
            return DeclaredType::Opaque;
        }
        let core = specifiers
            .iter()
            .copied()
            .filter(|text| !IGNORED_SPECIFIERS.contains(text))
            .collect::<Vec<_>>();
        match core.as_slice() {
            [] => DeclaredType::Opaque,
            ["struct", tag] if is_name(tag) => DeclaredType::Record((*tag).to_owned()),
            // A union member may overlap a pointer; never admitted.
            ["union", _] | ["union"] | ["struct"] => DeclaredType::Opaque,
            ["enum", tag] if is_name(tag) => DeclaredType::Scalar,
            ["enum"] => DeclaredType::Scalar,
            [name] if is_name(name) => {
                if depth >= TYPEDEF_DEPTH_LIMIT {
                    return DeclaredType::Opaque;
                }
                match self.typedefs.get(*name) {
                    Some(Some(target)) => {
                        let target = target.iter().map(String::as_str).collect::<Vec<_>>();
                        self.classify_specifiers(&target, depth + 1)
                    }
                    _ => DeclaredType::Opaque,
                }
            }
            rest if rest.iter().all(|text| ARITHMETIC_KEYWORDS.contains(text)) => {
                DeclaredType::Scalar
            }
            _ => DeclaredType::Opaque,
        }
    }

    fn declared_type(&self, specifiers: &[&str], stars: usize, array: bool) -> DeclaredType {
        if array {
            return DeclaredType::Opaque;
        }
        match (self.classify_specifiers(specifiers, 0), stars) {
            (kind, 0) => kind,
            (DeclaredType::Record(tag), 1) => DeclaredType::RecordPointer(tag),
            _ => DeclaredType::Opaque,
        }
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
    while rest.last() == Some(&"]") {
        let open = rest.iter().rposition(|text| *text == "[")?;
        rest = &rest[..open];
        array = true;
    }
    if rest.is_empty() {
        return None;
    }
    let mut declarator = Declarator {
        array,
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
fn statement_end(texts: &[&str], start: usize) -> Option<usize> {
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
fn is_declaration_candidate(texts: &[&str], start: usize) -> bool {
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
        let mut variables = BTreeMap::new();
        if !read_parameters(model, parameters, &mut variables) {
            return Self {
                variables: None,
                bound,
            };
        }
        if !read_locals(model, body, &mut variables) {
            return Self {
                variables: None,
                bound,
            };
        }
        Self {
            variables: Some(variables),
            bound,
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

    /// Whether `name` is a declared scalar variable of this function.
    pub(crate) fn declared_is_scalar(&self, name: &str) -> bool {
        self.declared(name) == Some(&DeclaredType::Scalar)
    }

    /// Whether every declaration of this function was read. When false, no
    /// typed rule may admit anything in it.
    pub(crate) fn is_readable(&self) -> bool {
        self.variables.is_some()
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
fn bound_names(body: &[&str], protected_formals: &BTreeSet<String>) -> BTreeSet<String> {
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
    bound
}

fn record_variable(
    variables: &mut BTreeMap<String, DeclaredType>,
    name: String,
    kind: DeclaredType,
) {
    match variables.get(&name) {
        // A redeclaration in an inner scope with a different declared type
        // makes the name unusable rather than silently keeping either type.
        Some(existing) if *existing != kind => {
            variables.insert(name, DeclaredType::Opaque);
        }
        Some(_) => {}
        None => {
            variables.insert(name, kind);
        }
    }
}

fn read_parameters(
    model: &TypedAccessModel,
    parameters: &[&str],
    variables: &mut BTreeMap<String, DeclaredType>,
) -> bool {
    if parameters
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "." | ";" | "="))
    {
        return false;
    }
    if parameters.is_empty() || parameters == ["void"] {
        return true;
    }
    for group in parameters.split(|text| *text == ",") {
        let Some((specifiers, declarator)) = split_declarator(group) else {
            return false;
        };
        let Some(name) = declarator.name else {
            continue;
        };
        let kind = model.declared_type(&specifiers, declarator.stars, declarator.array);
        record_variable(variables, name, kind);
    }
    true
}

fn read_locals(
    model: &TypedAccessModel,
    body: &[&str],
    variables: &mut BTreeMap<String, DeclaredType>,
) -> bool {
    for start in 0..body.len() {
        if start > 0 && !matches!(body[start - 1], "{" | "}" | ";" | "(" | ":") {
            continue;
        }
        if !is_declaration_candidate(body, start) {
            continue;
        }
        let Some(end) = statement_end(body, start) else {
            return false;
        };
        if !read_declaration(model, &body[start..end], variables) {
            return false;
        }
    }
    true
}

fn read_declaration(
    model: &TypedAccessModel,
    declaration: &[&str],
    variables: &mut BTreeMap<String, DeclaredType>,
) -> bool {
    // Initializers are dropped before the declarator is read: an initializer
    // may contain calls, aggregates and commas that say nothing about the
    // declared type. What remains must be a declarator this reader understands.
    let mut groups = split_top_level(declaration, ",").into_iter();
    let Some(first) = groups.next() else {
        return false;
    };
    let first = truncate_at_initializer(&first);
    if !readable_declarator(&first) {
        return false;
    }
    let Some((specifiers, declarator)) = split_declarator(&first) else {
        return false;
    };
    let mut declarators = vec![declarator];
    for group in groups {
        let group = truncate_at_initializer(&group);
        if !readable_declarator(&group) {
            return false;
        }
        let Some(declarator) = read_declarator(&group) else {
            return false;
        };
        declarators.push(declarator);
    }
    for declarator in declarators {
        let Some(name) = declarator.name else {
            continue;
        };
        let kind = model.declared_type(&specifiers, declarator.stars, declarator.array);
        record_variable(variables, name, kind);
    }
    true
}

/// A declarator carrying a nested declarator, a function parameter list, a
/// bitfield width or an aggregate is not read; it refuses its whole function.
fn readable_declarator(group: &[&str]) -> bool {
    !group
        .iter()
        .any(|text| matches!(*text, "{" | "}" | "(" | ")" | ":"))
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

/// Whether a statement the lexical gate rejected is a typed direct scalar-field
/// access, and therefore custody-neutral for the resource projection.
///
/// The caller must already have established that the statement contains no call
/// and no braces. Every other condition of the whitelist is checked here.
pub(crate) fn is_scalar_field_noop(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> bool {
    matches!(scalar_statement_accesses(texts, model, types), Some(count) if count > 0)
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
/// It is deliberately separate from [`is_scalar_field_noop`], whose contract
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
/// [`is_scalar_field_noop`] and [`is_scalar_only_noop`]'s conditions minus their
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

    #[test]
    fn a_function_pointer_member_refuses_the_whole_record() {
        let model = model("struct r { int a ; void ( * f ) ( int ) ; } ;");
        assert!(!model.scalar_field("r", "a"));
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
        is_scalar_field_noop(&texts(statement), &model, &types)
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
        assert!(!is_scalar_field_noop(
            &texts("protected -> a = 1"),
            &model,
            &types
        ));
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

    #[test]
    fn an_arrow_on_a_record_value_and_a_dot_on_a_pointer_are_refused() {
        assert!(!admits(R, "int n", "struct r v ;", "v -> a = n"));
        assert!(!admits(R, "struct r * s , int n", "", "s . a = n"));
    }
}
