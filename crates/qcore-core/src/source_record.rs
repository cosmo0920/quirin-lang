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
//! # The field-custody decision, which is the design
//!
//! **Every field of a v1 record is a scalar, so a record is a plain copyable
//! value and carries no custody at all.** It contributes no
//! [`crate::ownership::OwnershipOp`], it opens no obligation, it appears in no
//! ledger, and a name bound to one is refused at every custody position by the
//! admitting route's *existing* "this is not a buffer" rule rather than by
//! anything here.
//!
//! That is a decision and not an omission, and the measurement behind it is in
//! `docs/reports/aggregates-v1/README.md` §3. An **owned** field — a record
//! holding a `source.buffer.v1` acquisition — is a new custody shape, because
//! the cycle-3 ownership IR addresses an owner by a **plain name** and has no
//! notion of a path: `p.buf` is not something `check_ownership` can be asked
//! about, so an owned field would need a field-path owner vocabulary, a
//! partial-move rule, a drop-through-aggregate rule and a leak rule that sees
//! into the aggregate — four rules, none of which exists. Landing them behind a
//! v1 that cannot even *read* a field would have been the wrong order. The two
//! capabilities are named follow-ons (`AGGREGATE-OWNED-FIELD`,
//! `AGGREGATE-VIEW-FIELD`), each with its missing piece stated.
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
/// One constructor in v1, and deliberately: `i32` is the scalar the admitting
/// route's environment already holds, so a field read binds a value that route
/// can already index with, store, render and carry. Widening this enum is
/// `AGGREGATE-FIELD-TYPES`; widening it to a type that *owns* something is the
/// separate and larger `AGGREGATE-OWNED-FIELD`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldType {
    I32,
}

impl FieldType {
    /// The source spelling this type is declared with.
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            Self::I32 => "i32",
        }
    }

    /// The declared spelling, or `None` for a word this model gives no type.
    #[must_use]
    pub fn from_spelling(word: &str) -> Option<Self> {
        match word {
            "i32" => Some(Self::I32),
            _ => None,
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
            .map(|declared| declared.field_type)
    }

    /// The declared field names, in declaration order.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|field| field.name.as_str())
    }
}

/// A constructed record value: the value supplied for each declared field.
///
/// Held by field **name** rather than by position, so nothing downstream can
/// read one field's value as another's by miscounting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordValue {
    pub type_name: String,
    pub fields: BTreeMap<String, i32>,
}

impl RecordValue {
    /// Read one field.
    ///
    /// Total on the declared field names of the type this value was constructed
    /// for, because [`construct`] refuses a construction that does not supply
    /// every one of them exactly once.
    #[must_use]
    pub fn project(&self, field: &str) -> Option<i32> {
        self.fields.get(field).copied()
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
            | Self::RecordAtScalarPosition { .. } => "Q052",
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
                 `source.record.v1` models; every field of this revision's record is `i32`, and a \
                 field that owns something is a custody shape this model does not carry \
                 (`AGGREGATE-OWNED-FIELD`)"
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
/// Refuses a field the type does not declare, a field supplied twice, and a
/// declared field left unsupplied — in that order, so a construction that is
/// wrong in two ways names the one a reader reaches first. The result is a value
/// with a value for **every** declared field, which is what makes
/// [`RecordValue::project`] total on those names.
pub fn construct(
    declared: &RecordType,
    supplied: &[(String, i32)],
) -> Result<RecordValue, RecordRefusal> {
    let mut fields: BTreeMap<String, i32> = BTreeMap::new();
    for (name, value) in supplied {
        if declared.field(name).is_none() {
            return Err(RecordRefusal::UndeclaredField {
                type_name: declared.name.clone(),
                field: name.clone(),
            });
        }
        if fields.insert(name.clone(), *value).is_some() {
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
