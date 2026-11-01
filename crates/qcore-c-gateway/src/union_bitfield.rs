//! Card **CG13-UNION-AND-BITFIELD-PLACES** (gateway rule 10w): a place step
//! that selects a union member or a bit-field, read from the Clang AST only.
//!
//! ## Unions
//!
//! Every member of one union object occupies the same storage, so which
//! member is *live* is not the object's declared type. This module admits a
//! union member step in the typed access-path reader
//! ([`crate::typed_access_path::Reader::path`]) and in the common typed place
//! ([`crate::typed_place`], additively: [`MemberStorage`] on each step) and
//! decides the step with [`decide`]:
//!
//! * a **scalar read** is custody-neutral, as every scalar read is: whatever
//!   member was stored last, the value carries no custody;
//! * a **pointer-value read** (the final value, or a pointer loaded to read
//!   through) is P2's target-free borrowed alias, unless the function stores
//!   through, or takes the address of, *another* member of a union of the same
//!   record, or writes a whole object of the union's type: that is type
//!   punning (M22), and the read is refused ([`punned`]). A store made by a
//!   callee is not visible here; it is the stated premise
//!   `union-live-member-view`, and a punned pointer so read still carries no
//!   custody;
//! * a **scalar store** through a member of a union that can hold a pointer
//!   (any member not transitively a builtin scalar, an enumeration or an array
//!   of them) overwrites the representation of its pointer-valued siblings.
//!   It carries W43's refused-write frame, which poisons every binding a store
//!   could have rewritten, and with it the tracked identity of any
//!   pointer-valued member of the same object. A store through a
//!   **custody-free** union (every member scalar) is frame-neutral;
//! * a **pointer-valued store** through a union member is refused: a custody
//!   pointer stored in overlapping storage has no fixed view, and no contract
//!   fixes the view here. Card CG11 (rule 10v) owns pointer-valued stores and
//!   classifies their values; one through a union member reaches
//!   [`crate::typed_access_path::Reader::path`] as `Access::PointerStore` and
//!   is refused here whatever its value.
//!
//! No route binds a pointer place through a union member (the refusals above
//! and CG11's), so a union pointer member never carries a tracked identity or
//! a field link; the frame is what keeps that true after a sibling store.
//!
//! ## Bit-fields
//!
//! A bit-field occupies bits of a **storage unit**. C11 makes a maximal run of
//! adjacent non-zero-width bit-fields one memory location (C11 3.14), so the
//! unit a bit-field belongs to is read from its record's declaration order
//! ([`RecordFacts::units`]): never from a guessed layout. A bit-field is never
//! a pointer; a read is a custody-neutral scalar read of its unit and a store
//! is a read-modify-write of the whole unit, which holds only bit-fields, so
//! it rewrites no pointer and is frame-neutral. Two bit-fields of one unit are
//! one memory location: their bit ranges being disjoint does not make their
//! footprints disjoint. When the elaborator refuses an unordered pair of two
//! bit-field places of one object, [`shared_unit_refusal`] asks the core model
//! ([`qcore_core::disjoint_footprint`]) with a layout contract read from
//! Clang's own record layout (`-fdump-record-layouts-complete`), and names the
//! refusal with the core's `SharedBitfieldUnit` (Q661). A union bit-field, a
//! bit-field whose width Clang did not give, and every other unlisted form is
//! refused.
//!
//! The Lean model is `QCore.UnionBitfieldPlace`; its vectors are this
//! module's [`DECISION_VECTORS`].

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::typed_access_path::{FieldFacts, qual};
use crate::typed_c_ast::{children, kind};

/// How a member step occupies storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemberStorage {
    /// A member of a `struct`: its own storage.
    Ordinary,
    /// A member of a `union`: storage shared with every sibling.
    Union,
    /// A bit-field of a `struct` whose storage unit is known.
    Bitfield,
    /// A bit-field of a `union` (refused).
    UnionBitfield,
    /// A union member or bit-field whose facts Clang did not give: a width,
    /// a unit or an owning union (refused).
    Unread,
}

impl MemberStorage {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::Union => "union",
            Self::Bitfield => "bitfield",
            Self::UnionBitfield => "union-bitfield",
            Self::Unread => "unread",
        }
    }
}

/// The access a place makes through one member step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    ScalarRead,
    /// A pointer value read from the member's storage: the final value, or a
    /// pointer the path then reads through.
    PointerRead,
    ScalarStore,
    /// A pointer-valued store (card CG11's, rule 10v), which reaches the
    /// reader only for a value CG11 classified neutral.
    PointerStore,
}

impl Access {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::ScalarRead => "scalar-read",
            Self::PointerRead => "pointer-read",
            Self::ScalarStore => "scalar-store",
            Self::PointerStore => "pointer-store",
        }
    }
}

/// The decision on one member step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Not this rule's step.
    Ordinary,
    /// Custody-neutral, no frame.
    Neutral,
    /// Admitted with W43's refused-write frame.
    Framed,
    /// A target-free borrowed alias (P2).
    Borrow,
    /// A bit-field read of its storage unit.
    UnitRead,
    /// A bit-field store: a read-modify-write of its storage unit.
    UnitWrite,
    /// A pointer read through a union the function also stores through
    /// another member of.
    RefusedPunned,
    /// A pointer-valued store through a union member.
    RefusedPointerStore,
    /// Any other form.
    RefusedUnlisted,
}

impl Outcome {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::Neutral => "neutral",
            Self::Framed => "framed",
            Self::Borrow => "borrow",
            Self::UnitRead => "unit-read",
            Self::UnitWrite => "unit-write",
            Self::RefusedPunned => "refused-punned",
            Self::RefusedPointerStore => "refused-pointer-store",
            Self::RefusedUnlisted => "refused-unlisted",
        }
    }

    pub(crate) fn admits(self) -> bool {
        !matches!(
            self,
            Self::RefusedPunned | Self::RefusedPointerStore | Self::RefusedUnlisted
        )
    }
}

/// The decision table (`QCore.UnionBitfieldPlace.decide`): a pure function
/// of the step's storage, the access, whether the function puns the union
/// and whether the union is custody-free.
pub(crate) fn decide(
    storage: MemberStorage,
    access: Access,
    punned: bool,
    custody_free: bool,
) -> Outcome {
    match (storage, access) {
        (MemberStorage::Ordinary, _) => Outcome::Ordinary,
        (MemberStorage::Union, Access::ScalarRead) => Outcome::Neutral,
        (MemberStorage::Union, Access::PointerRead) if punned => Outcome::RefusedPunned,
        (MemberStorage::Union, Access::PointerRead) => Outcome::Borrow,
        (MemberStorage::Union, Access::ScalarStore) if custody_free => Outcome::Neutral,
        (MemberStorage::Union, Access::ScalarStore) => Outcome::Framed,
        (MemberStorage::Union, Access::PointerStore) => Outcome::RefusedPointerStore,
        (MemberStorage::Bitfield, Access::ScalarRead) => Outcome::UnitRead,
        (MemberStorage::Bitfield, Access::ScalarStore) => Outcome::UnitWrite,
        _ => Outcome::RefusedUnlisted,
    }
}

/// The decision vectors shared with `proof/QCore/UnionBitfieldPlace.lean`
/// (`decisionVectors`): storage, access, punned, custody-free, outcome.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str, bool, bool, &str)] = &[
    ("ordinary", "scalar-store", false, false, "ordinary"),
    ("union", "scalar-read", false, false, "neutral"),
    ("union", "scalar-read", true, false, "neutral"),
    ("union", "pointer-read", false, false, "borrow"),
    ("union", "pointer-read", true, false, "refused-punned"),
    ("union", "pointer-read", true, true, "refused-punned"),
    ("union", "scalar-store", false, false, "framed"),
    ("union", "scalar-store", false, true, "neutral"),
    (
        "union",
        "pointer-store",
        false,
        false,
        "refused-pointer-store",
    ),
    (
        "union",
        "pointer-store",
        false,
        true,
        "refused-pointer-store",
    ),
    ("bitfield", "scalar-read", false, false, "unit-read"),
    ("bitfield", "scalar-store", false, false, "unit-write"),
    ("bitfield", "pointer-read", false, false, "refused-unlisted"),
    (
        "bitfield",
        "pointer-store",
        false,
        false,
        "refused-unlisted",
    ),
    (
        "union-bitfield",
        "scalar-read",
        false,
        true,
        "refused-unlisted",
    ),
    (
        "union-bitfield",
        "scalar-store",
        false,
        true,
        "refused-unlisted",
    ),
    ("unread", "scalar-read", false, false, "refused-unlisted"),
    ("unread", "scalar-store", false, false, "refused-unlisted"),
];

/// One bit-field storage unit: a maximal run of adjacent non-zero-width
/// bit-fields of one complete `struct`, in declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BitfieldUnit {
    /// The owning `RecordDecl` id.
    pub(crate) record: String,
    /// The unit's ordinal among the record's units.
    pub(crate) ordinal: usize,
    /// The unit's members' `FieldDecl` ids, in declaration order.
    pub(crate) members: Vec<String>,
}

/// Record facts this rule reads from the unit's declarations.
#[derive(Clone, Debug, Default)]
pub(crate) struct RecordFacts {
    /// A union member's `FieldDecl` id → its union's `RecordDecl` id.
    pub(crate) union_owner: BTreeMap<String, String>,
    /// Union `RecordDecl` ids none of whose members can hold a pointer.
    pub(crate) custody_free: BTreeSet<String>,
    /// A bit-field's `FieldDecl` id → its storage unit; `None` when the width
    /// is unread or the owner is a union.
    pub(crate) units: BTreeMap<String, Option<BitfieldUnit>>,
    /// A named `struct`'s `RecordDecl` id → its tag, for the layout reader.
    pub(crate) names: BTreeMap<String, String>,
    /// A bit-field's or union member's `FieldDecl` id → its name (empty for
    /// an unnamed one).
    pub(crate) field_names: BTreeMap<String, String>,
    /// Card CG18-FOOTPRINT-CONNECTION (rule 11k): every member of a complete
    /// `struct`, `FieldDecl` id → (the struct's `RecordDecl` id, the member's
    /// name, empty for an unnamed one).
    pub(crate) struct_members: BTreeMap<String, (String, String)>,
    /// Card MK-EVENT-FIRST-MEMBER-CELL (rule 38 (1)): a named `struct`'s tag
    /// → one entry per complete definition, its first member's name (empty
    /// for an unnamed one) and declared type (desugared), `None` when it has
    /// no member. A reader admits only a tag with exactly one entry.
    pub(crate) first_members: BTreeMap<String, Vec<Option<(String, String)>>>,
}

impl RecordFacts {
    /// Collect the facts of every complete record under `node`.
    pub(crate) fn collect(node: &Value, facts: &mut Self) {
        if kind(node) == "RecordDecl" && node["completeDefinition"].as_bool() == Some(true) {
            facts.record(node);
        }
        for child in children(node) {
            Self::collect(child, facts);
        }
    }

    fn record(&mut self, record: &Value) {
        let Some(id) = record["id"].as_str() else {
            return;
        };
        let tag = record["tagUsed"].as_str();
        if tag == Some("struct")
            && let Some(name) = record["name"].as_str().filter(|name| !name.is_empty())
        {
            self.names.insert(id.to_owned(), name.to_owned());
        }
        if tag == Some("struct")
            && let Some(name) = record["name"].as_str().filter(|name| !name.is_empty())
        {
            let first = children(record)
                .iter()
                .find(|child| kind(child) == "FieldDecl")
                .map(|field| {
                    let declared = field["type"]["desugaredQualType"]
                        .as_str()
                        .or_else(|| field["type"]["qualType"].as_str())
                        .unwrap_or("");
                    (
                        field["name"].as_str().unwrap_or("").to_owned(),
                        declared.to_owned(),
                    )
                });
            self.first_members
                .entry(name.to_owned())
                .or_default()
                .push(first);
        }
        if tag == Some("struct") {
            for field in children(record)
                .iter()
                .filter(|child| kind(child) == "FieldDecl")
            {
                if let Some(field_id) = field["id"].as_str() {
                    self.struct_members.insert(
                        field_id.to_owned(),
                        (
                            id.to_owned(),
                            field["name"].as_str().unwrap_or("").to_owned(),
                        ),
                    );
                }
            }
        }
        if tag == Some("union") {
            for field in children(record)
                .iter()
                .filter(|child| kind(child) == "FieldDecl")
            {
                if let Some(field_id) = field["id"].as_str() {
                    self.union_owner.insert(field_id.to_owned(), id.to_owned());
                    self.field_names.insert(
                        field_id.to_owned(),
                        field["name"].as_str().unwrap_or("").to_owned(),
                    );
                }
            }
            if custody_free(record) {
                self.custody_free.insert(id.to_owned());
            }
        }
        let mut run: Vec<String> = Vec::new();
        let mut ordinal = 0;
        let mut close =
            |run: &mut Vec<String>, units: &mut BTreeMap<String, Option<BitfieldUnit>>| {
                if run.is_empty() {
                    return;
                }
                let unit = BitfieldUnit {
                    record: id.to_owned(),
                    ordinal,
                    members: std::mem::take(run),
                };
                ordinal += 1;
                for member in &unit.members {
                    units.insert(member.clone(), Some(unit.clone()));
                }
            };
        for field in children(record)
            .iter()
            .filter(|child| kind(child) == "FieldDecl")
        {
            let Some(field_id) = field["id"].as_str() else {
                continue;
            };
            if field["isBitfield"].as_bool() != Some(true) {
                close(&mut run, &mut self.units);
                continue;
            }
            self.field_names.insert(
                field_id.to_owned(),
                field["name"].as_str().unwrap_or("").to_owned(),
            );
            let width = bitfield_width(field);
            if tag != Some("struct") || width.is_none() {
                close(&mut run, &mut self.units);
                self.units.insert(field_id.to_owned(), None);
                continue;
            }
            if width == Some(0) {
                // A zero-width bit-field ends the memory location.
                close(&mut run, &mut self.units);
                continue;
            }
            run.push(field_id.to_owned());
        }
        close(&mut run, &mut self.units);
    }

    /// The storage of the member a `MemberExpr` selects.
    pub(crate) fn storage(&self, facts: &FieldFacts, field: &str) -> MemberStorage {
        match (facts.union_owner, facts.bitfield) {
            (false, false) => MemberStorage::Ordinary,
            (true, false) if self.union_owner.contains_key(field) => MemberStorage::Union,
            (true, false) => MemberStorage::Unread,
            (true, true) => MemberStorage::UnionBitfield,
            (false, true) => match self.units.get(field) {
                Some(Some(_)) => MemberStorage::Bitfield,
                _ => MemberStorage::Unread,
            },
        }
    }
}

/// The width Clang gives a bit-field (its `ConstantExpr` value).
fn bitfield_width(field: &Value) -> Option<u64> {
    let [width] = children(field) else {
        return None;
    };
    if kind(width) != "ConstantExpr" {
        return None;
    }
    width["value"].as_str()?.parse().ok()
}

/// Whether no member of `record` can hold a pointer: every member is a
/// builtin scalar, an enumeration or an array of them, or a record defined
/// inline right before it whose members are all such.
fn custody_free(record: &Value) -> bool {
    let mut previous: Option<&Value> = None;
    for child in children(record) {
        match kind(child) {
            "RecordDecl" => previous = Some(child),
            "FieldDecl" => {
                let Some(ty) = qual(child) else {
                    return false;
                };
                let inline = previous.take().filter(|nested| {
                    nested["completeDefinition"].as_bool() == Some(true)
                        && (ty.contains("(anonymous")
                            || ty.contains("(unnamed")
                            || nested["name"].as_str().is_some_and(|name| {
                                !name.is_empty()
                                    && (ty == format!("struct {name}")
                                        || ty == format!("union {name}"))
                            }))
                });
                if let Some(nested) = inline {
                    if !custody_free(nested) {
                        return false;
                    }
                } else if !scalar_type(ty) {
                    return false;
                }
            }
            _ => previous = None,
        }
    }
    true
}

/// A builtin arithmetic type, an `enum TAG`, or an array of either.
fn scalar_type(text: &str) -> bool {
    let base = text.split('[').next().unwrap_or(text);
    let words = base.split_whitespace().collect::<Vec<_>>();
    if let ["enum", tag] | ["const", "enum", tag] = words.as_slice() {
        return !tag.starts_with('(');
    }
    let mut any = false;
    for word in words {
        match word {
            "const" => {}
            "signed" | "unsigned" | "short" | "long" | "char" | "int" | "float" | "double"
            | "_Bool" | "bool" | "__int128" => any = true,
            _ => return false,
        }
    }
    any
}

/// Whether `function` stores through, or takes the address of, a member of
/// the union `record` other than `member`, or writes or addresses a whole
/// object whose type spells `union_type`. Assignments, compound assignments,
/// increments and decrements are stores; `&` is an address a callee may store
/// through.
pub(crate) fn punned(
    function: &Value,
    facts: &RecordFacts,
    record: &str,
    member: &str,
    union_type: &str,
) -> bool {
    let mut found = false;
    visit(function, &mut |node| {
        if found {
            return;
        }
        let written = match kind(node) {
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => children(node).first(),
            "CompoundAssignOperator" => children(node).first(),
            "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--" | "&")) => {
                children(node).first()
            }
            _ => None,
        };
        if let Some(place) = written
            && writes_other_member(place, facts, record, member, union_type)
        {
            found = true;
        }
    });
    found
}

fn writes_other_member(
    mut place: &Value,
    facts: &RecordFacts,
    record: &str,
    member: &str,
    union_type: &str,
) -> bool {
    let mut outer = true;
    loop {
        if outer && qual(place) == Some(union_type) {
            return true;
        }
        match kind(place) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => {}
            "MemberExpr" => {
                outer = false;
                if let Some(field) = place["referencedMemberDecl"].as_str()
                    && field != member
                    && facts.union_owner.get(field).map(String::as_str) == Some(record)
                {
                    return true;
                }
            }
            "ArraySubscriptExpr" | "UnaryOperator" => outer = false,
            _ => return false,
        }
        let Some(inner) = children(place).first() else {
            return false;
        };
        place = inner;
    }
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// Whether a subscript's base is a pointer value read from memory (a hop),
/// rather than an array decaying to its first element.
pub(crate) fn subscript_hop(base: &Value) -> bool {
    let mut node = base;
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" => match node["castKind"].as_str() {
                Some("NoOp") => {}
                Some("ArrayToPointerDecay") => return false,
                _ => return true,
            },
            _ => return true,
        }
        let Some(inner) = children(node).first() else {
            return true;
        };
        node = inner;
    }
}

/// One non-ordinary member step of a path, in evaluation order, with what
/// follows it.
#[derive(Clone, Debug)]
pub(crate) struct MemberStep {
    pub(crate) storage: MemberStorage,
    pub(crate) field: String,
    /// The union's record id, for a union step.
    pub(crate) record: Option<String>,
    /// The Clang type of the union object the step selects from.
    pub(crate) union_type: String,
    /// Whether the path reads a pointer value from memory after this step.
    pub(crate) hop_after: bool,
}

/// The decision on every non-ordinary step of one path, or the first
/// refusal. `final_access` is the path's own access. Returns whether a store
/// needs the refused-write frame. Parity runs record every decision
/// (`position: "union-bitfield"`) with the path's node offset.
pub(crate) fn decide_path(
    steps: &[MemberStep],
    final_access: Access,
    facts: &RecordFacts,
    function: Option<&Value>,
    node: &Value,
) -> Result<bool, Outcome> {
    let mut framed = false;
    let mut decided = Vec::new();
    let mut refusal = None;
    for step in steps {
        let access = if step.hop_after {
            Access::PointerRead
        } else {
            final_access
        };
        let (punned, custody_free) = match (&step.record, step.storage) {
            (Some(record), MemberStorage::Union) => {
                let punned = access == Access::PointerRead
                    && function.is_none_or(|function| {
                        punned(function, facts, record, &step.field, &step.union_type)
                    });
                (punned, facts.custody_free.contains(record))
            }
            _ => (false, false),
        };
        let outcome = decide(step.storage, access, punned, custody_free);
        decided.push((step.storage, access, outcome));
        match outcome {
            Outcome::Framed => framed = true,
            outcome if !outcome.admits() => {
                refusal = Some(outcome);
                break;
            }
            _ => {}
        }
    }
    if crate::typed_access_path::parity_enabled() {
        crate::typed_access_path::record_row(serde_json::json!({
            "position": "union-bitfield",
            "function": function.and_then(|function| function["name"].as_str()),
            "offset": crate::typed_access_path::offset(&node["range"]["begin"]),
            "steps": decided.iter().map(|(storage, access, outcome)| serde_json::json!({
                "storage": storage.code(),
                "access": access.code(),
                "outcome": outcome.code(),
            })).collect::<Vec<_>>(),
            "admitted": refusal.is_none(),
            "framed": framed,
        }));
    }
    match refusal {
        Some(outcome) => Err(outcome),
        None => Ok(framed),
    }
}

// ---------------------------------------------------------------------------
// The bit-field storage-unit connection to the core footprint model.
// ---------------------------------------------------------------------------

/// Clang's layout of one record's direct bit-fields, read from its own
/// `-fdump-record-layouts-complete` output: name → (byte offset, first bit,
/// last bit), and the record's size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClangLayout {
    pub(crate) size: u32,
    pub(crate) bitfields: BTreeMap<String, (u32, u32, u32)>,
}

/// Read the layout of `struct NAME` from Clang's record layout dump. `None`
/// unless exactly one dump names the record and every direct bit-field line
/// parses.
pub(crate) fn parse_layout(dump: &str, name: &str) -> Option<ClangLayout> {
    let header = format!("| struct {name}");
    let mut found = None;
    for section in dump.split("*** Dumping AST Record Layout").skip(1) {
        let mut lines = section.lines().filter(|line| !line.trim().is_empty());
        let Some(first) = lines.next() else {
            continue;
        };
        if first
            .trim_end()
            .split_once('|')
            .map(|(_, rest)| rest.trim())
            != Some(header.trim_start_matches("| "))
        {
            continue;
        }
        if found.is_some() {
            return None;
        }
        let mut layout = ClangLayout {
            size: 0,
            bitfields: BTreeMap::new(),
        };
        for line in lines {
            let (offset, rest) = line.split_once('|')?;
            let rest = rest.strip_prefix(' ')?;
            if let Some(size) = rest.trim().strip_prefix("[sizeof=") {
                layout.size = size.split(',').next()?.parse().ok()?;
                break;
            }
            // A direct member is indented by exactly two spaces.
            let direct = rest.starts_with("  ") && !rest.starts_with("   ");
            let Some((byte, bits)) = offset.trim().split_once(':') else {
                continue;
            };
            if !direct {
                continue;
            }
            let (first_bit, last_bit) = bits.split_once('-')?;
            let field = rest.split_whitespace().last()?;
            layout.bitfields.insert(
                field.to_owned(),
                (
                    byte.parse().ok()?,
                    first_bit.parse().ok()?,
                    last_bit.parse().ok()?,
                ),
            );
        }
        found = Some(layout);
    }
    found.filter(|layout| layout.size > 0)
}

/// The core layout contract of a record's bit-field units, and the core
/// field id of each member: every unit's bytes are the bytes its members'
/// bits occupy in Clang's layout, and each member keeps its bit range within
/// them. `None` when a member is missing from the layout.
pub(crate) fn layout_contract(
    facts: &RecordFacts,
    record: &str,
    layout: &ClangLayout,
) -> Option<(
    qcore_core::disjoint_footprint::LayoutContract,
    BTreeMap<String, u32>,
)> {
    use qcore_core::disjoint_footprint::{FieldLayout, FieldStorage, LayoutContract};
    let mut units = facts
        .units
        .values()
        .flatten()
        .filter(|unit| unit.record == record)
        .collect::<Vec<_>>();
    units.sort_by_key(|unit| unit.ordinal);
    units.dedup_by_key(|unit| unit.ordinal);
    let mut fields = Vec::new();
    let mut ids = BTreeMap::new();
    for unit in units {
        let mut bits = Vec::new();
        for member in &unit.members {
            let name = facts.field_names.get(member)?;
            let &(byte, first, last) = layout.bitfields.get(name)?;
            if name.is_empty() || last < first {
                return None;
            }
            bits.push((member, byte * 8 + first, byte * 8 + last));
        }
        let start = bits.iter().map(|(_, low, _)| low / 8).min()?;
        let end = bits.iter().map(|(_, _, high)| high / 8 + 1).max()?;
        for (member, low, high) in bits {
            let id = u32::try_from(ids.len()).ok()?;
            ids.insert(member.clone(), id);
            fields.push(FieldLayout {
                field: id,
                byte_offset: start,
                byte_size: end - start,
                storage: FieldStorage::BitfieldUnit {
                    unit: u32::try_from(unit.ordinal).ok()?,
                    bit_offset: low - start * 8,
                    bit_width: high - low + 1,
                },
            });
        }
    }
    Some((
        LayoutContract {
            record: 0,
            size: layout.size,
            fields,
        },
        ids,
    ))
}

/// The core model's answer on two bit-fields of one object: `Some(unit)`
/// when it refuses the pair as one storage unit (Q661).
pub(crate) fn core_shared_unit(
    facts: &RecordFacts,
    record: &str,
    layout: &ClangLayout,
    left: &str,
    right: &str,
) -> Option<u32> {
    use qcore_core::disjoint_footprint::{
        ElementShape, FootprintDiagnostic, FootprintEvent, FootprintProgram,
        check_disjoint_footprint_program,
    };
    let (contract, ids) = layout_contract(facts, record, layout)?;
    let program = FootprintProgram {
        layout_contracts: vec![contract],
        events: vec![
            FootprintEvent::Allocate {
                alloc: 1,
                generation: 1,
                shape: ElementShape::Record { record: 0 },
                count: 1,
            },
            FootprintEvent::DeriveSlice {
                handle: 10,
                alloc: 1,
                generation: 1,
                elem_offset: 0,
                elem_count: 1,
                declared_bytes: None,
            },
            FootprintEvent::DeriveField {
                handle: 20,
                base: 10,
                field: *ids.get(left)?,
            },
            FootprintEvent::DeriveField {
                handle: 21,
                base: 10,
                field: *ids.get(right)?,
            },
            FootprintEvent::AssertDisjoint {
                left: 20,
                right: 21,
            },
        ],
    };
    match check_disjoint_footprint_program(&program) {
        Err(diagnostics) => match diagnostics.as_slice() {
            [FootprintDiagnostic::SharedBitfieldUnit { unit, .. }] => Some(*unit),
            _ => None,
        },
        Ok(_) => None,
    }
}

/// Card CG13 hook for the elaborator: an unordered pair `one`/`other` the
/// elaborator refused as not proved disjoint, when both are bit-field places
/// of one object in one storage unit by the core model, is named as such.
/// `node` is the full expression; `None` keeps the elaborator's refusal.
pub(crate) fn shared_unit_refusal(
    node: &Value,
    facts: &RecordFacts,
    one: &str,
    other: &str,
    layout_dump: &dyn Fn() -> Option<String>,
) -> Option<u32> {
    let left = bitfield_place(node, one)?;
    let right = bitfield_place(node, other)?;
    if left.0 != right.0 || left.1 == right.1 {
        return None;
    }
    let unit_left = facts.units.get(&left.1)?.as_ref()?;
    let unit_right = facts.units.get(&right.1)?.as_ref()?;
    if unit_left.record != unit_right.record {
        return None;
    }
    let name = facts.names.get(&unit_left.record)?;
    let layout = parse_layout(&layout_dump()?, name)?;
    core_shared_unit(facts, &unit_left.record, &layout, &left.1, &right.1)
}

/// The object spelling and bit-field `FieldDecl` id of the place spelled
/// `spelling` in `node`, when its outermost step is a member selection.
fn bitfield_place(node: &Value, spelling: &str) -> Option<(String, String)> {
    let mut found = None;
    let mut ambiguous = false;
    visit(node, &mut |candidate| {
        if kind(candidate) == "MemberExpr"
            && crate::elaboration::path_place(candidate)
                .is_ok_and(|place| place.spelling() == spelling)
        {
            let field = candidate["referencedMemberDecl"]
                .as_str()
                .map(str::to_owned);
            let object = children(candidate)
                .first()
                .and_then(|base| crate::elaboration::path_place(base).ok())
                .map(|place| place.spelling().to_owned());
            match (field, object) {
                (Some(field), Some(object)) => {
                    if found
                        .as_ref()
                        .is_some_and(|seen| *seen != (object.clone(), field.clone()))
                    {
                        ambiguous = true;
                    }
                    found = Some((object, field));
                }
                _ => ambiguous = true,
            }
        }
    });
    if ambiguous { None } else { found }
}

#[cfg(test)]
#[path = "../tests/unit/union_bitfield.rs"]
mod tests;
