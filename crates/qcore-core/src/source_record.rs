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
//! # Immutable by construction, and what that buys
//!
//! A v1 record is **constructed once and read**; there is no field write
//! (`AGGREGATE-FIELD-WRITE`). The construction is *total* — every declared field
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

use std::collections::BTreeMap;
use std::fmt;

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
/// Two constructors. `i32` is the scalar the admitting route's environment
/// already holds, so a field read binds a value that route can already index
/// with, store, render and carry. `bytes` (card `AGGREGATE-OWNED-FIELD`) is the
/// route's **owned byte buffer** — spelled with the acquisition's own reserved
/// word rather than the task route's `Buffer`, because that word names a
/// different resource (`qcore_buffer_new`'s four-byte payload) with a different
/// model, and a field type that spelled one while meaning the other would be a
/// collision by design. Widening this enum further is `AGGREGATE-FIELD-TYPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldType {
    I32,
    /// An owned `source.buffer.v1` buffer, moved in at construction.
    Bytes,
}

impl FieldType {
    /// The source spelling this type is declared with.
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::Bytes => "bytes",
        }
    }

    /// The declared spelling, or `None` for a word this model gives no type.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        match word {
            "i32" => Some(Self::I32),
            "bytes" => Some(Self::Bytes),
            _ => None,
        }
    }

    /// Whether a field of this type owns a resource.
    #[must_use]
    pub fn is_owned(self) -> bool {
        matches!(self, Self::Bytes)
    }
}

/// The value supplied for — and held by — one field.
///
/// A scalar is the integer itself. An owned field holds the **name** of the
/// buffer binding moved into it at construction; the custody of that move is
/// the ownership IR's ([`crate::ownership::OwnershipOp::Compose`]), and the
/// admitting route carries the buffer's extent and bytes beside this model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
    Scalar(i32),
    Bytes { moved_from: String },
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
            .map(|declared| declared.field_type)
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

    /// Read one **scalar** field: `None` for a field the type does not declare
    /// or one that owns a buffer.
    #[must_use]
    pub fn scalar(&self, field: &str) -> Option<i32> {
        match self.fields.get(field) {
            Some(FieldValue::Scalar(value)) => Some(*value),
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
    /// A custody position — a store, a load, an extent read, a release —
    /// naming a field that is a scalar, not a buffer.
    FieldIsNotABuffer {
        name: String,
        type_name: String,
        field: String,
    },
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
            | Self::FieldIsNotABuffer { .. } => "Q052",
            // A buffer at a scalar field is the route's crossing condition,
            // `Q055`, which the route reports first; the model's own statement
            // of it lands on the same code so one fault has one code.
            Self::FieldTakesAScalar { .. } => "Q055",
            // A range this model does not enumerate.
            Self::TooManyFields { .. } => "Q053",
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
                 `source.record.v1` models; a field is `i32` or `bytes` (an owned buffer, moved \
                 in at construction), and any other type is `AGGREGATE-FIELD-TYPES`"
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
            Self::FieldIsNotABuffer {
                name,
                type_name,
                field,
            } => write!(
                f,
                "`{name}.{field}` is a `{type_name}` field declared `i32`, not a buffer; a store, \
                 a load, an extent read and a release name a field declared `bytes`"
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
            (FieldType::I32, FieldValue::Scalar(_)) | (FieldType::Bytes, FieldValue::Bytes { .. }) => {}
            (FieldType::Bytes, FieldValue::Scalar(_)) => {
                return Err(RecordRefusal::FieldTakesABuffer {
                    type_name: declared.name.clone(),
                    field: name.clone(),
                });
            }
            (FieldType::I32, FieldValue::Bytes { .. }) => {
                return Err(RecordRefusal::FieldTakesAScalar {
                    type_name: declared.name.clone(),
                    field: name.clone(),
                });
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
    Ok(types)
}
