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

//! `source.record.v1` — the first source-level aggregate type (card
//! `AGGREGATES-V1`).
//!
//! # What this model is, stated before anything else
//!
//! A **record** is a named type with a fixed, ordered list of distinctly named
//! fields, each of an existing scalar type. A program declares one, constructs a
//! value of it from check-time-known field values, and reads a field back as an
//! ordinary integer. That is the whole of v1.
//!
//! # The field-custody decision, and the card that superseded half of it
//!
//! **A record of scalar fields is a plain copyable value and carries no
//! custody at all.** It contributes no [`crate::ownership::OwnershipOp`], it
//! opens no obligation, it appears in no ledger, and a name bound to one is
//! refused at every custody position by the admitting route's *existing* "this
//! is not a buffer" rule rather than by anything here.
//!
//! That was card `AGGREGATES-V1`'s decision, measured in
//! `docs/reports/aggregates-v1/README.md` §3 against the cycle-3 ownership IR
//! as it then was: an owner was a **plain name** with no notion of a path, so
//! `p.buf` was not something `check_ownership` could be asked about, and an
//! owned field needed four rules none of which existed — a field-path owner
//! vocabulary, a partial-move rule, a drop-through-aggregate rule and a leak
//! rule that sees into the aggregate. Card `AGGREGATE-OWNED-FIELD` (wave 39)
//! landed those four rules **in the IR first** ([`crate::ownership::Place`],
//! [`crate::ownership::OwnershipOp::Compose`] and its siblings) and only then
//! widened this model: a field may now be declared [`FieldType::Bytes`], a
//! construction supplying a `source.buffer.v1` acquisition at such a field
//! **moves** it in, and the record is then a custody shape — addressed field by
//! field, dropped as a whole, never copied. A record declaring no such field is
//! exactly what it was. The view-field step (`AGGREGATE-VIEW-FIELD`) stays a
//! named follow-on with its missing piece stated.
//!
//! # Construction is total, and what that buys
//!
//! Card `W48-QR-USABLE-RECORDS` added the field write
//! ([`RecordValue::write_path`]: one field replaced, nothing else) and the
//! record field ([`FieldType::Record`], a record held by value, the
//! declaration graph acyclic — [`admit_graph`]). Neither weakens what follows:
//! a write replaces a value the construction supplied, so every declared
//! path still has one. The construction is *total* — every declared field
//! is supplied exactly once, checked here — so a constructed value has a value
//! for every field it can be asked for, and [`RecordValue::project`] is total on
//! the declared field names. That totality is what makes the field-access rule a
//! theorem rather than a partial function with an error arm, and it is the
//! reason the refusals below are all about the *construction*.
//!
//! # Distinctness is the property, and it is decided here
//!
//! CT09's scenario is "distinct ordinary struct fields". Two fields of one
//! record are distinct because [`admit_type`] refuses a repeated field name
//! outright, and because a construction binds each declared name to its own
//! supplied value. Reading two different fields of one value therefore yields
//! the two values supplied for them — never one of them twice — which is what
//! the emitted C's two distinct members have to reproduce and what the byte
//! comparison against a native run actually checks.
//!
//! # Diagnostics, and why this module allocates no code
//!
//! Under the `Q-NAMESPACE-DECISION` ice box no card allocates new `Q` codes, so
//! every refusal below lands on a **condition of the admitting route's existing
//! band** — `Q051`–`Q057`, the `StandardBuffer` pass — widened and stated in
//! §L21 and in the card's report rather than taken silently. [`code`] is the one
//! place that mapping lives.
//!
//! [`code`]: RecordRefusal::code

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::scalar_types::ScalarType;

/// The largest number of fields a record this model admits may declare.
///
/// Bounded for the reason [`crate::std_buffer::MAX_EXTENT`] is: every field of
/// every constructed value is published in the certificate and emitted as a
/// distinct C member, so a record this model cannot enumerate exactly is not
/// modelled at all. A larger declaration is refused by name rather than
/// truncated.
pub const MAX_FIELDS: usize = 32;

/// The type of a record field.
///
/// Three constructors. A **scalar** field is one type of the §G02t fixed-width
/// table (card `W48-QR-SCALAR-TYPES-R3`; before it, `i32` only): a field read
/// binds a value of exactly that type, which the admitting route then checks
/// at every position it reaches — `i32` is the one its indices, stored bytes
/// and loop bounds take. `bytes` (card `AGGREGATE-OWNED-FIELD`) is the route's
/// **owned byte buffer** — spelled with the acquisition's own reserved word
/// rather than the task route's `Buffer`, because that word names a different
/// resource (`qcore_buffer_new`'s four-byte payload) with a different model,
/// and a field type that spelled one while meaning the other would be a
/// collision by design. A **record** field (card `W48-QR-USABLE-RECORDS`)
/// holds another declared record by value. Widening this enum further is
/// `AGGREGATE-FIELD-TYPES`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldType {
    /// A §G02t scalar, held by value.
    Scalar(ScalarType),
    /// An owned `source.buffer.v1` buffer, moved in at construction.
    Bytes,
    /// Another record type this module declares, held by value (card
    /// `W48-QR-USABLE-RECORDS`, which lifted `AGGREGATE-NESTED`). A record of
    /// scalars held here is copied in; one with an owned field anywhere below
    /// it is **moved** in, its owned leaves becoming the outer record's.
    Record(String),
}

impl FieldType {
    /// The `i32` field every record declared before card
    /// `W48-QR-SCALAR-TYPES-R3` has.
    pub const I32: Self = Self::Scalar(ScalarType::I32);

    /// The source spelling this type is declared with.
    #[must_use]
    pub fn spelling(&self) -> &str {
        match self {
            Self::Scalar(ty) => ty.name(),
            Self::Bytes => "bytes",
            Self::Record(name) => name,
        }
    }

    /// The declared spelling, or `None` for a word this model gives no type.
    /// A record type is not recognised here — which words name one is the
    /// module's question ([`Self::from_spelling_in`]).
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        match word {
            "bytes" => Some(Self::Bytes),
            _ => ScalarType::parse(word).map(Self::Scalar),
        }
    }

    /// [`Self::from_spelling`], plus a record type the module declares (card
    /// `W48-QR-USABLE-RECORDS`). A scalar or `bytes` spelling keeps its
    /// meaning even if a record were so named, because those words were
    /// field types first.
    #[must_use]
    pub fn from_spelling_in(word: &str, declared_records: &BTreeSet<String>) -> Option<Self> {
        Self::from_spelling(word).or_else(|| {
            declared_records
                .contains(word)
                .then(|| Self::Record(word.to_owned()))
        })
    }

    /// The scalar type of a scalar field, `None` for an owned or record one.
    #[must_use]
    pub fn scalar(&self) -> Option<ScalarType> {
        match self {
            Self::Scalar(ty) => Some(*ty),
            Self::Bytes | Self::Record(_) => None,
        }
    }

    /// The record type of a record field.
    #[must_use]
    pub fn record(&self) -> Option<&str> {
        match self {
            Self::Record(name) => Some(name),
            Self::Scalar(_) | Self::Bytes => None,
        }
    }

    /// Whether a field of this type **directly** owns a resource. A record
    /// field owns through its own fields; [`owned_leaves`] is the question
    /// that sees through it.
    #[must_use]
    pub fn is_owned(&self) -> bool {
        matches!(self, Self::Bytes)
    }
}

/// The spelling a `source.string.v1` borrowed view would be written with at a
/// field position (card `OWNED-FIELD-ROUTE-REACH`).
///
/// One word, deliberately. The view has no written type name anywhere in this
/// revision — it is produced by `borrow_utf8(…)` and by a quoted literal, never
/// declared — so this constant is not "the" name of the type but the spelling a
/// reader would reach for, and its whole job is to make `AGGREGATE-VIEW-FIELD`
/// name itself at the one place the boundary is met.
pub const VIEW_FIELD_SPELLING: &str = "string";

/// Why a field-type spelling is not a field type — the **by-name**
/// classification (card `OWNED-FIELD-ROUTE-REACH`, wave 41).
///
/// Two answers, in one place so the reader and any later route cannot
/// disagree about which residual a spelling meets: a view spelling is
/// `AGGREGATE-VIEW-FIELD`, and everything else is the generic
/// `AGGREGATE-FIELD-TYPES` this model always reported. Both are `Q051`.
/// Card `W48-QR-USABLE-RECORDS` removed the third answer: a record type this
/// module declares is a field type ([`FieldType::Record`]), so
/// `AGGREGATE-NESTED`'s refusal is lifted rather than renamed, and
/// `declared_records` is read only so that a spelling is never classified
/// as unknown when it names one.
#[must_use]
pub fn field_type_refusal(
    type_name: &str,
    field: &str,
    spelling: &str,
    declared_records: &BTreeSet<String>,
) -> RecordRefusal {
    debug_assert!(
        !declared_records.contains(spelling),
        "a declared record type is a field type"
    );
    if spelling == VIEW_FIELD_SPELLING {
        return RecordRefusal::ViewFieldType {
            type_name: type_name.to_owned(),
            field: field.to_owned(),
            spelling: spelling.to_owned(),
        };
    }
    RecordRefusal::UnknownFieldType {
        type_name: type_name.to_owned(),
        field: field.to_owned(),
        spelling: spelling.to_owned(),
    }
}

/// The deepest nesting of record fields this model admits (card
/// `W48-QR-USABLE-RECORDS`): a record holding a record holding … at most this
/// many records deep, the outermost counted. Bounded for [`MAX_FIELDS`]'s
/// reason — every leaf of every value is published and emitted — and refused
/// by name beyond it rather than truncated.
pub const MAX_NESTING: usize = 8;

/// The most scalar and owned **leaves** one record type may hold once its
/// record fields are expanded (card `W48-QR-USABLE-RECORDS`). [`MAX_FIELDS`]
/// bounds one level; this bounds the tree.
pub const MAX_LEAVES: usize = 256;

/// The owned **leaves** of `type_name` — every `bytes` field at any depth, as
/// a path from the record down — in declaration order at every level,
/// depth-first (card `W48-QR-USABLE-RECORDS`). This is the order a
/// whole-record `drop` releases in, and the order the ownership IR's
/// aggregate lists; a one-level record's list is
/// [`RecordType::owned_field_names`], one-element paths.
///
/// `types` resolves a record name; an unresolvable one and a cycle contribute
/// nothing (both are refused at type formation, [`admit_types`], so a module
/// that reaches this function has neither).
#[must_use]
pub fn owned_leaves<'a>(
    types: &impl Fn(&str) -> Option<&'a RecordType>,
    type_name: &str,
) -> Vec<Vec<String>> {
    fn walk<'a>(
        types: &impl Fn(&str) -> Option<&'a RecordType>,
        type_name: &str,
        prefix: &mut Vec<String>,
        depth: usize,
        into: &mut Vec<Vec<String>>,
    ) {
        if depth > MAX_NESTING {
            return;
        }
        let Some(declared) = types(type_name) else {
            return;
        };
        for field in &declared.fields {
            prefix.push(field.name.clone());
            match &field.field_type {
                FieldType::Bytes => into.push(prefix.clone()),
                FieldType::Record(inner) => walk(types, inner, prefix, depth + 1, into),
                FieldType::Scalar(_) => {}
            }
            prefix.pop();
        }
    }
    let mut leaves = Vec::new();
    walk(types, type_name, &mut Vec::new(), 1, &mut leaves);
    leaves
}

/// Whether a value of `type_name` owns anything — an owned field at any depth
/// (card `W48-QR-USABLE-RECORDS`). The one question that decides whether a
/// record is a custody shape or a plain value; for a record declaring no
/// record field it is [`RecordType::has_owned_field`].
#[must_use]
pub fn owns<'a>(types: &impl Fn(&str) -> Option<&'a RecordType>, type_name: &str) -> bool {
    !owned_leaves(types, type_name).is_empty()
}

/// The declared type at `path` below a value of `type_name` (card
/// `W48-QR-USABLE-RECORDS`): every component but the last must name a field
/// whose type is a record.
///
/// # Errors
///
/// [`RecordRefusal::NoSuchField`] for a component the type at that point does
/// not declare, and [`RecordRefusal::PathThroughNonRecord`] for a component
/// reached through a field that is a scalar or a buffer.
pub fn field_type_at<'a>(
    types: &impl Fn(&str) -> Option<&'a RecordType>,
    name: &str,
    type_name: &str,
    path: &[String],
) -> Result<FieldType, RecordRefusal> {
    let mut current = type_name.to_owned();
    for (depth, component) in path.iter().enumerate() {
        let Some(declared) = types(&current) else {
            return Err(RecordRefusal::UnknownType { type_name: current });
        };
        let Some(field_type) = declared.field(component) else {
            return Err(RecordRefusal::NoSuchField {
                name: spell_path(name, &path[..depth]),
                type_name: current,
                field: component.clone(),
            });
        };
        if depth + 1 == path.len() {
            return Ok(field_type);
        }
        match field_type {
            FieldType::Record(inner) => current = inner,
            other => {
                return Err(RecordRefusal::PathThroughNonRecord {
                    path: spell_path(name, &path[..=depth]),
                    declared: other.spelling().to_owned(),
                    next: path[depth + 1].clone(),
                });
            }
        }
    }
    Err(RecordRefusal::NotARecord {
        name: name.to_owned(),
    })
}

/// `NAME.F1.….Fn`, the spelling a diagnostic and a certificate name a field
/// path by; `NAME` itself for the empty path.
#[must_use]
pub fn spell_path(name: &str, path: &[String]) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{name}.{}", path.join("."))
    }
}

/// The value supplied for — and held by — one field.
///
/// An `i32` is the integer itself, the one kind every record declared before
/// card `W48-QR-SCALAR-TYPES-R3` holds. A value of any other §G02t type is
/// [`Self::Typed`]: the type and the number (`bool` is `0`/`1`), so a value
/// never travels without the type that says what it is, and one type has one
/// representation. An owned field holds the **name** of the buffer binding
/// moved into it at construction; the custody of that move is the ownership
/// IR's ([`crate::ownership::OwnershipOp::Compose`]), and the admitting route
/// carries the buffer's extent and bytes beside this model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
    Scalar(i32),
    /// A value of a §G02t type other than `i32` (card `W48-QR-SCALAR-TYPES-R3`).
    Typed {
        ty: ScalarType,
        value: i128,
    },
    Bytes {
        moved_from: String,
    },
    /// A record held by a record field (card `W48-QR-USABLE-RECORDS`): the
    /// whole value, copied or moved in at construction.
    Record(RecordValue),
}

impl FieldValue {
    /// The scalar type of this value, `None` for an owned buffer or a record.
    #[must_use]
    pub fn scalar_type(&self) -> Option<ScalarType> {
        match self {
            Self::Scalar(_) => Some(ScalarType::I32),
            Self::Typed { ty, .. } => Some(*ty),
            Self::Bytes { .. } | Self::Record(_) => None,
        }
    }
}

/// One declared field: a name and a type, in declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordField {
    pub name: String,
    pub field_type: FieldType,
}

/// A declared record type.
///
/// The field list is ordered and the order is load-bearing in exactly one
/// place — the emitted C's member order — and in no other: construction is by
/// name and projection is by name, so no rule of this model depends on a
/// position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordType {
    pub name: String,
    pub fields: Vec<RecordField>,
}

impl RecordType {
    /// The declared type of `field`, or `None` if this type declares no such
    /// field.
    #[must_use]
    pub fn field(&self, field: &str) -> Option<FieldType> {
        self.fields
            .iter()
            .find(|declared| declared.name == field)
            .map(|declared| declared.field_type.clone())
    }

    /// The declared field names, in declaration order.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|field| field.name.as_str())
    }

    /// The declared **owned** field names, in declaration order (card
    /// `AGGREGATE-OWNED-FIELD`). This order is the one a whole-record `drop`
    /// releases in, published by the certificate and walked by the emitter,
    /// so it is read from the declaration in exactly one place.
    pub fn owned_field_names(&self) -> impl Iterator<Item = &str> {
        self.fields
            .iter()
            .filter(|field| field.field_type.is_owned())
            .map(|field| field.name.as_str())
    }

    /// Whether this type declares any owned field — the one question that
    /// decides whether a value of it is a custody shape or a plain value.
    #[must_use]
    pub fn has_owned_field(&self) -> bool {
        self.fields.iter().any(|field| field.field_type.is_owned())
    }
}

/// A constructed record value: the value supplied for each declared field.
///
/// Held by field **name** rather than by position, so nothing downstream can
/// read one field's value as another's by miscounting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordValue {
    pub type_name: String,
    pub fields: BTreeMap<String, FieldValue>,
}

impl RecordValue {
    /// Read one field.
    ///
    /// Total on the declared field names of the type this value was constructed
    /// for, because [`construct`] refuses a construction that does not supply
    /// every one of them exactly once.
    #[must_use]
    pub fn project(&self, field: &str) -> Option<&FieldValue> {
        self.fields.get(field)
    }

    /// Read one **`i32`** field: `None` for a field the type does not declare,
    /// one of another scalar type, or one that owns a buffer.
    #[must_use]
    pub fn scalar(&self, field: &str) -> Option<i32> {
        match self.fields.get(field) {
            Some(FieldValue::Scalar(value)) => Some(*value),
            _ => None,
        }
    }

    /// Read one field of a §G02t type other than `i32` (card
    /// `W48-QR-SCALAR-TYPES-R3`): its type and number.
    #[must_use]
    pub fn typed(&self, field: &str) -> Option<(ScalarType, i128)> {
        match self.fields.get(field) {
            Some(FieldValue::Typed { ty, value }) => Some((*ty, *value)),
            _ => None,
        }
    }

    /// **Projection along a path** (card `W48-QR-USABLE-RECORDS`): the value
    /// at `a.b.c`, each component but the last a record field. Total on the
    /// declared paths of the type this value was constructed for, because
    /// construction is total at every level; `None` for any other path.
    #[must_use]
    pub fn project_path(&self, path: &[String]) -> Option<&FieldValue> {
        let (first, rest) = path.split_first()?;
        let value = self.fields.get(first)?;
        if rest.is_empty() {
            return Some(value);
        }
        match value {
            FieldValue::Record(inner) => inner.project_path(rest),
            _ => None,
        }
    }

    /// **The field-write rule** (card `W48-QR-USABLE-RECORDS`): replace the
    /// value at `path` and nothing else. Returns the value the path held, or
    /// `None` — changing nothing — when the path is not a declared path of
    /// this value. `QCore.UsableRecords.write_changes_exactly_that_field` and
    /// `QCore.UsableRecords.write_leaves_every_other_path` are the Lean
    /// statements.
    pub fn write_path(&mut self, path: &[String], value: FieldValue) -> Option<FieldValue> {
        let (first, rest) = path.split_first()?;
        let slot = self.fields.get_mut(first)?;
        if rest.is_empty() {
            return Some(std::mem::replace(slot, value));
        }
        match slot {
            FieldValue::Record(inner) => inner.write_path(rest, value),
            _ => None,
        }
    }
}

/// Every way this model refuses.
///
/// **No variant allocates a diagnostic code.** Each maps onto a condition the
/// admitting route's band already names — see [`RecordRefusal::code`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordRefusal {
    /// A record type declaring no field at all.
    Empty { type_name: String },
    /// Two fields of one type sharing a name. **The distinctness rule**, and the
    /// reason this is a refusal rather than a resolution: which of two values a
    /// read returns would otherwise be decided by the order two readers happen
    /// to walk the list in.
    DuplicateField { type_name: String, field: String },
    /// More fields than this model enumerates.
    TooManyFields { type_name: String, count: usize },
    /// A field declared with a word this model gives no type.
    UnknownFieldType {
        type_name: String,
        field: String,
        spelling: String,
    },
    /// A field declared with the **view** spelling (card
    /// `OWNED-FIELD-ROUTE-REACH`, wave 41).
    ///
    /// Split out of [`Self::UnknownFieldType`] so the boundary
    /// `AGGREGATE-VIEW-FIELD` names **itself** where it is met. Before this
    /// card the shape refused with the generic message, which named
    /// `AGGREGATE-FIELD-TYPES` — a *different* residual — so the boundary was
    /// absence rather than refusal and a later card lifting it would have found
    /// nothing to supersede and no message pinned. Same code (`Q051`), same
    /// refused set: a condition extension, not a new allocation.
    ViewFieldType {
        type_name: String,
        field: String,
        spelling: String,
    },
    /// A record type that contains itself, directly or through other record
    /// fields (card `W48-QR-USABLE-RECORDS`): **the termination argument of
    /// the declaration graph**. Card `OWNED-FIELD-ROUTE-REACH` refused every
    /// record field by name (`AGGREGATE-NESTED`); the lift admits a record
    /// field exactly when the graph of record fields is acyclic, so every
    /// value is a finite tree and every walk over one ends. `cycle` names the
    /// types around the loop, starting and ending at `type_name`.
    RecursiveFieldType {
        type_name: String,
        field: String,
        cycle: Vec<String>,
    },
    /// A record nested more than [`MAX_NESTING`] deep (card
    /// `W48-QR-USABLE-RECORDS`).
    NestingTooDeep { type_name: String, depth: usize },
    /// A record whose expanded tree has more than [`MAX_LEAVES`] leaves (card
    /// `W48-QR-USABLE-RECORDS`).
    TooManyLeaves { type_name: String, count: usize },
    /// A field path continuing through a field that is not a record (card
    /// `W48-QR-USABLE-RECORDS`): `p.x.y` where `p.x` is a scalar or a buffer.
    PathThroughNonRecord {
        path: String,
        declared: String,
        next: String,
    },
    /// A record field supplied something other than a record of its declared
    /// type (card `W48-QR-USABLE-RECORDS`): a scalar, a buffer, or a record of
    /// another type. `supplied` names the other record type when it is one.
    FieldTakesARecord {
        type_name: String,
        field: String,
        declared: String,
        supplied: Option<String>,
    },
    /// A write naming a field whose type is a record (card
    /// `W48-QR-USABLE-RECORDS`): this revision writes a scalar field, or
    /// refills an owned one, and a whole sub-record is written field by field
    /// (`W48-QR-RECORD-FIELD-WHOLE-WRITE`).
    WriteOfRecordField { path: String, declared: String },
    /// A construction naming a type this module does not declare.
    UnknownType { type_name: String },
    /// A construction supplying a field the type does not declare.
    UndeclaredField { type_name: String, field: String },
    /// A construction supplying one field twice.
    RepeatedField { type_name: String, field: String },
    /// A construction leaving a declared field unsupplied. **The totality
    /// rule.**
    MissingField { type_name: String, field: String },
    /// A field read naming something this entry constructed no record for.
    NotARecord { name: String },
    /// A field read naming a field the record's type does not declare.
    NoSuchField {
        name: String,
        type_name: String,
        field: String,
    },
    /// A record at a position that takes an integer — an index, a stored byte,
    /// a rendering operand, a loop bound.
    RecordAtScalarPosition { name: String },
    /// A construction supplying an integer at a field declared `bytes` (card
    /// `AGGREGATE-OWNED-FIELD`): the field takes an owned buffer this entry
    /// acquired, moved in.
    FieldTakesABuffer { type_name: String, field: String },
    /// A construction supplying a buffer at a field declared `i32`. Reached
    /// from the admitting route only through its own crossing rule (`Q055`,
    /// unchanged); this is the model's total statement of the same fault.
    FieldTakesAScalar { type_name: String, field: String },
    /// A construction supplying a value of one §G02t type at a field declared
    /// another (card `W48-QR-SCALAR-TYPES-R3`). There is no implicit
    /// conversion anywhere, so a `u8` name at an `i32` field is refused rather
    /// than widened; the reader writes `as` where they mean one.
    FieldTakesAType {
        type_name: String,
        field: String,
        declared: ScalarType,
        supplied: ScalarType,
    },
    /// A construction supplying a literal that is not a value of its field's
    /// type (card `W48-QR-SCALAR-TYPES-R3`): outside the type's range, the
    /// minimum included, or a number at a `bool` field.
    FieldLiteralOutOfRange {
        type_name: String,
        field: String,
        declared: ScalarType,
        literal: String,
    },
    /// A field declared `isize` or `usize` in a module checked with no
    /// recorded target (card `W48-QR-SCALAR-TYPES-R3`): the width is never
    /// guessed.
    FieldNeedsRecordedTarget {
        type_name: String,
        field: String,
        declared: ScalarType,
    },
    /// A custody position — a store, a load, an extent read, a release —
    /// naming a field that is a scalar, not a buffer.
    FieldIsNotABuffer {
        name: String,
        type_name: String,
        field: String,
        /// The field's declared type's spelling (card
        /// `W48-QR-SCALAR-TYPES-R3`; before it, always `i32`; since card
        /// `W48-QR-USABLE-RECORDS` a scalar's or a record's name).
        declared: String,
    },
    /// A read of a whole sub-record with an owned field below it (card
    /// `W48-QR-USABLE-RECORDS`): that would move a whole aggregate out of
    /// another, which this revision does not spell; its fields are read one by
    /// one (`W48-QR-NESTED-OWNED-SUBRECORD-MOVE`).
    SubRecordOwns { path: String, declared: String },
}

impl RecordRefusal {
    /// The admitting route's condition this refusal lands on.
    ///
    /// The `StandardBuffer` band, widened rather than extended: `Q051` is that
    /// pass's **shape** condition, `Q052` its "a name that is not what this
    /// position takes" condition, and `Q053` its **range** condition. Card
    /// `AGGREGATES-V1` allocates no code, on the `SOCKET-EVENT-COMPOSE` /
    /// `SOCKET-INET-ADDRESSES` precedent and under the `Q-NAMESPACE-DECISION`
    /// ice box.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            // Shape: what a well-formed declaration and a well-formed
            // construction are.
            Self::Empty { .. }
            | Self::DuplicateField { .. }
            | Self::UnknownFieldType { .. }
            // Card `OWNED-FIELD-ROUTE-REACH`: the two boundaries that now name
            // themselves are the same shape condition, refused at the same
            // code — what changed is the message, not the allocation.
            | Self::ViewFieldType { .. }
            // Card `W48-QR-USABLE-RECORDS`: a record that contains itself is
            // not a type, and a whole sub-record write is not a statement this
            // revision spells — both the band's shape condition.
            | Self::RecursiveFieldType { .. }
            | Self::WriteOfRecordField { .. }
            | Self::SubRecordOwns { .. }
            | Self::UndeclaredField { .. }
            | Self::RepeatedField { .. }
            | Self::MissingField { .. } => "Q051",
            // A name at a position that is not what the position takes.
            Self::UnknownType { .. }
            | Self::NotARecord { .. }
            | Self::NoSuchField { .. }
            | Self::RecordAtScalarPosition { .. }
            // Card `AGGREGATE-OWNED-FIELD`: an integer where an owned field
            // takes a buffer, and a scalar field where a custody position
            // takes a buffer — both "a name that is not what this position
            // takes", `Q052`'s existing condition widened.
            | Self::FieldTakesABuffer { .. }
            | Self::FieldIsNotABuffer { .. }
            // Card `W48-QR-SCALAR-TYPES-R3`: a value of another type at a
            // field is "a name that is not what this position takes", the
            // same condition, widened.
            | Self::FieldTakesAType { .. }
            // Card `W48-QR-USABLE-RECORDS`: a path through a field that has
            // no fields, and a record field given something else, are the
            // same condition.
            | Self::PathThroughNonRecord { .. }
            | Self::FieldTakesARecord { .. } => "Q052",
            // Card `W48-QR-SCALAR-TYPES-R3`: a literal outside its field's
            // type is the band's **range** condition, widened.
            Self::FieldLiteralOutOfRange { .. } => "Q053",
            // Card `W48-QR-SCALAR-TYPES-R3`: a target-width field without a
            // recorded target is a declaration this module cannot state, the
            // band's **shape** condition, widened.
            Self::FieldNeedsRecordedTarget { .. } => "Q051",
            // A buffer at a scalar field is the route's crossing condition,
            // `Q055`, which the route reports first; the model's own statement
            // of it lands on the same code so one fault has one code.
            Self::FieldTakesAScalar { .. } => "Q055",
            // A range this model does not enumerate.
            Self::TooManyFields { .. }
            | Self::NestingTooDeep { .. }
            | Self::TooManyLeaves { .. } => "Q053",
        }
    }
}

impl fmt::Display for RecordRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { type_name } => write!(
                f,
                "`{type_name}` declares no field; `source.record.v1` models a record as a fixed \
                 list of distinctly named fields and a list of none names nothing to read"
            ),
            Self::DuplicateField { type_name, field } => write!(
                f,
                "`{type_name}` declares `{field}` twice; `source.record.v1` refuses a repeated \
                 field name rather than deciding which of two declarations a read answers to — \
                 the fields of a record are distinct, and that is the property this type has"
            ),
            Self::TooManyFields { type_name, count } => write!(
                f,
                "`{type_name}` declares {count} fields; `source.record.v1` models 1 to \
                 {MAX_FIELDS}, and a larger declaration is refused rather than truncated"
            ),
            Self::UnknownFieldType {
                type_name,
                field,
                spelling,
            } => write!(
                f,
                "`{type_name}.{field}` is declared `{spelling}`, which is not a field type \
                 `source.record.v1` models; a field is a fixed-width scalar (bool, i8, i16, \
                 i32, i64, isize, u8, u16, u32, u64, usize) or `bytes` (an owned buffer, moved \
                 in at construction), and any other type is `AGGREGATE-FIELD-TYPES`"
            ),
            Self::ViewFieldType {
                type_name,
                field,
                spelling,
            } => write!(
                f,
                "`{type_name}.{field}` is declared `{spelling}`, a `source.string.v1` view, and \
                 `source.record.v1` models no view field: a view is `{{storage, offset, len}}` \
                 borrowed from a buffer for an exact lexical window, and an aggregate has \
                 nowhere to carry that window's end — the field would outlive the borrow the \
                 window was computed for. This is `AGGREGATE-VIEW-FIELD`, and it is refused by \
                 name rather than left to the generic field-type message"
            ),
            Self::RecursiveFieldType {
                type_name,
                field,
                cycle,
            } => write!(
                f,
                "`{type_name}.{field}` makes `{type_name}` contain itself ({}); a record field \
                 holds a whole value, so a record that contains itself would be an infinite \
                 value — the record fields of a module form an acyclic graph",
                cycle.join(" -> ")
            ),
            Self::NestingTooDeep { type_name, depth } => write!(
                f,
                "`{type_name}` nests records {depth} deep; `source.record.v1` models at most \
                 {MAX_NESTING} levels, and a deeper declaration is refused rather than truncated"
            ),
            Self::TooManyLeaves { type_name, count } => write!(
                f,
                "`{type_name}` holds {count} fields once its record fields are expanded; \
                 `source.record.v1` models at most {MAX_LEAVES}, and a larger tree is refused \
                 rather than truncated"
            ),
            Self::PathThroughNonRecord {
                path,
                declared,
                next,
            } => write!(
                f,
                "`{path}` is declared `{declared}`, which has no fields, so `{path}.{next}` names \
                 nothing; a field path continues only through a field whose type is a record"
            ),
            Self::FieldTakesARecord {
                type_name,
                field,
                declared,
                supplied,
            } => match supplied {
                Some(supplied) => write!(
                    f,
                    "`{type_name}.{field}` is declared `{declared}` and is supplied a \
                     `{supplied}`; a record field takes a record of exactly its declared type"
                ),
                None => write!(
                    f,
                    "`{type_name}.{field}` is declared `{declared}` and takes a `{declared}` \
                     record this entry bound; a literal, a scalar or a buffer is not one"
                ),
            },
            Self::SubRecordOwns { path, declared } => write!(
                f,
                "`{path}` is a `{declared}`, which holds an owned field; reading it whole would \
                 move an aggregate out of another, which this revision does not spell, so read \
                 its fields one by one (`W48-QR-NESTED-OWNED-SUBRECORD-MOVE`)"
            ),
            Self::WriteOfRecordField { path, declared } => write!(
                f,
                "`{path}` is declared `{declared}`, a record; a write names a scalar field (or \
                 refills an owned one), so write the fields of `{path}` one by one \
                 (`W48-QR-RECORD-FIELD-WHOLE-WRITE`)"
            ),
            Self::UnknownType { type_name } => write!(
                f,
                "`{type_name}` is not a record type this module declares; `source.record.v1` \
                 constructs a value of a type declared in the same file"
            ),
            Self::UndeclaredField { type_name, field } => write!(
                f,
                "`{type_name}` declares no field `{field}`; a construction supplies exactly the \
                 declared fields, so a name that is not one of them is refused rather than ignored"
            ),
            Self::RepeatedField { type_name, field } => write!(
                f,
                "this `{type_name}` construction supplies `{field}` twice; `source.record.v1` \
                 refuses a repeated field rather than deciding which of two values survives"
            ),
            Self::MissingField { type_name, field } => write!(
                f,
                "this `{type_name}` construction supplies no `{field}`; `source.record.v1` \
                 constructs a record only from a complete field list, so every field of a \
                 constructed value has a value a reader can predict"
            ),
            Self::NotARecord { name } => write!(
                f,
                "`{name}` is not a record this entry constructed; `source.record.v1` reads a field \
                 only of a value constructed in the same entry"
            ),
            Self::NoSuchField {
                name,
                type_name,
                field,
            } => write!(
                f,
                "`{name}` is a `{type_name}`, which declares no field `{field}`"
            ),
            Self::RecordAtScalarPosition { name } => write!(
                f,
                "`{name}` is a record and this position takes an integer; `source.record.v1` \
                 carries no arithmetic on an aggregate, so read the field you mean first \
                 (`let v = {name}.FIELD;`)"
            ),
            Self::FieldTakesABuffer { type_name, field } => write!(
                f,
                "`{type_name}.{field}` is declared `bytes` and takes an owned buffer this entry \
                 acquired, which the construction moves in; an integer, a scalar name, a record or \
                 a lend is not one"
            ),
            Self::FieldTakesAScalar { type_name, field } => write!(
                f,
                "`{type_name}.{field}` is declared `i32` and takes an integer; a buffer cannot \
                 cross into a scalar field"
            ),
            Self::FieldTakesAType {
                type_name,
                field,
                declared,
                supplied,
            } => write!(
                f,
                "`{type_name}.{field}` is declared `{declared}` and is supplied a `{supplied}`; \
                 there is no implicit conversion, so bind `let v = NAME as {declared};` first \
                 where the number is meant at the field's type"
            ),
            Self::FieldLiteralOutOfRange {
                type_name,
                field,
                declared,
                literal,
            } => write!(
                f,
                "`{type_name}.{field}` is declared `{declared}` and `{literal}` is not a value \
                 of `{declared}`; a field literal is range-checked against its field's type, \
                 the minimum included, and refused rather than truncated"
            ),
            Self::FieldNeedsRecordedTarget {
                type_name,
                field,
                declared,
            } => write!(
                f,
                "`{type_name}.{field}` is declared `{declared}`, and `isize` and `usize` take \
                 their width from a recorded target, none of which was supplied; the width is \
                 never guessed (supply one with `--recorded-target FILE`)"
            ),
            Self::FieldIsNotABuffer {
                name,
                type_name,
                field,
                declared,
            } => write!(
                f,
                "`{name}.{field}` is a `{type_name}` field declared `{declared}`, not a buffer; a \
                 store, a load, an extent read and a release name a field declared `bytes`"
            ),
        }
    }
}

/// **Type formation.** Whether this declaration is a record type at all.
///
/// Three conditions, each one of them a refusal this model owns: a record
/// declares at least one field, no two fields share a name, and the list is one
/// this model enumerates. The first failure is reported; a declaration is
/// refused either way, and one report names the one thing to change.
pub fn admit_type(declared: &RecordType) -> Result<(), RecordRefusal> {
    if declared.fields.is_empty() {
        return Err(RecordRefusal::Empty {
            type_name: declared.name.clone(),
        });
    }
    if declared.fields.len() > MAX_FIELDS {
        return Err(RecordRefusal::TooManyFields {
            type_name: declared.name.clone(),
            count: declared.fields.len(),
        });
    }
    for (position, field) in declared.fields.iter().enumerate() {
        if declared.fields[..position]
            .iter()
            .any(|earlier| earlier.name == field.name)
        {
            return Err(RecordRefusal::DuplicateField {
                type_name: declared.name.clone(),
                field: field.name.clone(),
            });
        }
    }
    Ok(())
}

/// **Construction totality.** Build a value of `declared` from `supplied`.
///
/// Refuses a field the type does not declare, a value of the other kind than
/// the field is declared (card `AGGREGATE-OWNED-FIELD`), a field supplied
/// twice, and a declared field left unsupplied — in that order, so a
/// construction that is wrong in two ways names the one a reader reaches
/// first. The result is a value with a value for **every** declared field,
/// which is what makes [`RecordValue::project`] total on those names.
pub fn construct(
    declared: &RecordType,
    supplied: &[(String, FieldValue)],
) -> Result<RecordValue, RecordRefusal> {
    let mut fields: BTreeMap<String, FieldValue> = BTreeMap::new();
    for (name, value) in supplied {
        let Some(field_type) = declared.field(name) else {
            return Err(RecordRefusal::UndeclaredField {
                type_name: declared.name.clone(),
                field: name.clone(),
            });
        };
        match (field_type, value) {
            (FieldType::Bytes, FieldValue::Bytes { .. }) => {}
            (
                FieldType::Bytes,
                FieldValue::Scalar(_) | FieldValue::Typed { .. } | FieldValue::Record(_),
            ) => {
                return Err(RecordRefusal::FieldTakesABuffer {
                    type_name: declared.name.clone(),
                    field: name.clone(),
                });
            }
            (FieldType::Scalar(_), FieldValue::Bytes { .. }) => {
                return Err(RecordRefusal::FieldTakesAScalar {
                    type_name: declared.name.clone(),
                    field: name.clone(),
                });
            }
            // Card `W48-QR-USABLE-RECORDS`: a record field takes a record of
            // exactly its declared type, and nothing else takes a record.
            (FieldType::Record(expected), FieldValue::Record(supplied))
                if supplied.type_name == expected => {}
            (FieldType::Record(expected), other) => {
                return Err(RecordRefusal::FieldTakesARecord {
                    type_name: declared.name.clone(),
                    field: name.clone(),
                    declared: expected,
                    supplied: match other {
                        FieldValue::Record(supplied) => Some(supplied.type_name.clone()),
                        _ => None,
                    },
                });
            }
            (FieldType::Scalar(_), FieldValue::Record(supplied)) => {
                return Err(RecordRefusal::RecordAtScalarPosition {
                    name: supplied.type_name.clone(),
                });
            }
            // Card `W48-QR-SCALAR-TYPES-R3`: the value's type is the field's,
            // exactly. An `i32` is held as `Scalar` and every other type as
            // `Typed`, so one type has one representation and the check is an
            // equality; a `Typed` spelling of `i32` is refused as not that
            // representation.
            (FieldType::Scalar(expected), value) => {
                let supplied = value
                    .scalar_type()
                    .expect("a scalar or typed value has a scalar type");
                let canonical =
                    !matches!(value, FieldValue::Typed { ty, .. } if *ty == ScalarType::I32);
                if supplied != expected || !canonical {
                    return Err(RecordRefusal::FieldTakesAType {
                        type_name: declared.name.clone(),
                        field: name.clone(),
                        declared: expected,
                        supplied,
                    });
                }
            }
        }
        if fields.insert(name.clone(), value.clone()).is_some() {
            return Err(RecordRefusal::RepeatedField {
                type_name: declared.name.clone(),
                field: name.clone(),
            });
        }
    }
    for field in &declared.fields {
        if !fields.contains_key(&field.name) {
            return Err(RecordRefusal::MissingField {
                type_name: declared.name.clone(),
                field: field.name.clone(),
            });
        }
    }
    Ok(RecordValue {
        type_name: declared.name.clone(),
        fields,
    })
}

/// The declared types of one module, by name.
///
/// Built once, total-or-reject: a module carrying a declaration [`admit_type`]
/// refuses has no table at all, so no later rule can read a type this model did
/// not admit.
pub fn admit_types(declared: &[RecordType]) -> Result<BTreeMap<String, RecordType>, RecordRefusal> {
    let mut types = BTreeMap::new();
    for record in declared {
        admit_type(record)?;
        types.insert(record.name.clone(), record.clone());
    }
    admit_graph(declared, &types)?;
    Ok(types)
}

/// **The termination argument** (card `W48-QR-USABLE-RECORDS`): the graph
/// whose edges are record fields is acyclic, no chain of record fields is
/// longer than [`MAX_NESTING`], and no type's expanded tree has more than
/// [`MAX_LEAVES`] leaves. With the graph acyclic every value is a finite tree
/// and every walk over one — construction, projection, the drop-through, the
/// emitted C's member order — ends. The Lean model's values are finite trees
/// by construction (`QCore.UsableRecords.Val`); the graph check itself is
/// pinned by `compiler/tests/owned_field_route_reach.rs` and
/// `compiler/tests/usable_records.rs`. Checked in declaration order, so the first
/// offending declaration is the one reported. A record field naming a type
/// the module does not declare cannot reach here: the reader classifies the
/// spelling against the module's declared names.
///
/// # Errors
///
/// The first cycle, the first over-deep type, or the first over-large one.
pub fn admit_graph(
    declared: &[RecordType],
    types: &BTreeMap<String, RecordType>,
) -> Result<(), RecordRefusal> {
    /// The depth of `name`'s tree, or the cycle through `name` if `stack`
    /// already holds it.
    fn depth(
        types: &BTreeMap<String, RecordType>,
        name: &str,
        stack: &mut Vec<String>,
    ) -> Result<usize, RecordRefusal> {
        let Some(declared) = types.get(name) else {
            return Ok(0);
        };
        stack.push(name.to_owned());
        let mut deepest = 0;
        for field in &declared.fields {
            let FieldType::Record(inner) = &field.field_type else {
                continue;
            };
            if let Some(at) = stack.iter().position(|entry| entry == inner) {
                // The field that closes the loop, on the type that holds it.
                let mut cycle = stack[at..].to_vec();
                cycle.push(inner.clone());
                return Err(RecordRefusal::RecursiveFieldType {
                    type_name: name.to_owned(),
                    field: field.name.clone(),
                    cycle,
                });
            }
            if stack.len() > MAX_NESTING {
                return Err(RecordRefusal::NestingTooDeep {
                    type_name: stack[0].clone(),
                    depth: stack.len() + 1,
                });
            }
            deepest = deepest.max(depth(types, inner, stack)?);
        }
        stack.pop();
        Ok(deepest + 1)
    }
    fn leaves(types: &BTreeMap<String, RecordType>, name: &str) -> usize {
        types.get(name).map_or(0, |declared| {
            declared
                .fields
                .iter()
                .map(|field| match &field.field_type {
                    FieldType::Record(inner) => leaves(types, inner).max(1),
                    _ => 1,
                })
                .fold(0usize, usize::saturating_add)
        })
    }
    for name in declared.iter().map(|record| &record.name) {
        let deep = depth(types, name, &mut Vec::new())?;
        if deep > MAX_NESTING {
            return Err(RecordRefusal::NestingTooDeep {
                type_name: name.clone(),
                depth: deep,
            });
        }
        let count = leaves(types, name);
        if count > MAX_LEAVES {
            return Err(RecordRefusal::TooManyLeaves {
                type_name: name.clone(),
                count,
            });
        }
    }
    Ok(())
}
