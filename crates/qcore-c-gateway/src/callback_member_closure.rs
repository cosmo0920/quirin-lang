//! Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (gateway rule 12n; contract
//! spec §C04.19.3): the closure of a function-pointer member's stores, the
//! candidate set of `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)`.
//!
//! USER DECISION 2026-10-14 (clause 3): the dotted spelling `TAG.FIELD`, the
//! closure run-wide over Clang's stores, premise `P-CG22-MEMBER-CLOSED-WORLD`.
//!
//! # Facts (one unit)
//!
//! [`MemberFacts::collect`] reads every top-level declaration of a unit's
//! Clang AST once ([`crate::typed_access_path`]'s retained declarations):
//!
//! * every store into a function-pointer member (`X->F = V`, `X.a.b = V`),
//!   keyed `TAG.PATH` (the record the path is read from), with what `V` is
//!   ([`Flow`]): a function designator, a null pointer constant, a read of
//!   another such member, a formal of the enclosing function, or anything
//!   else (open, with why); a member whose address is taken is open;
//! * every function-pointer actual of a direct call, keyed by callee and
//!   position, the same way (so a formal's values are its callers' actuals);
//! * every function named other than as a direct callee (its address may
//!   escape, so its formals may be reached by a call no fact records);
//! * the functions the unit defines with internal linkage;
//! * every initializer list of a record the unit defines once at file scope,
//!   mapped onto its fields (card CALLBACK-MEMBER-DATA-ONLY-UNITS): each
//!   function-pointer field's entry is a store into `TAG.FIELD`, an implicit
//!   entry a null pointer;
//! * every record whose function-pointer members an initializer list that
//!   cannot be mapped so, a whole-object assignment from anything but an
//!   object of the same record, or a `memcpy`-family call may write (the
//!   record is open), with each write's target ([`OpaqueTarget`], card
//!   UNION-OPAQUE-RECORD-WRITES-BY-TYPE): a union holding the record reads
//!   the write by the record's placements only where the write may lie in
//!   the union; a write into the union is an open store at its own place.
//!
//! A unit the gateway does not analyse (a data-only unit) is read the same
//! way by [`read_unit`] (card CALLBACK-MEMBER-DATA-ONLY-UNITS), so every unit
//! of the run contributes facts read from its AST; a unit whose AST is not
//! read contributes none, which opens every member.
//!
//! # Closure (any set of units)
//!
//! [`closure`] is the least set of designators the facts let flow into a
//! member: a designator joins it, a null pointer adds nothing, a member read
//! adds that member's set, a formal of a function whose address is never
//! taken adds its actuals at every call the facts record (an internal
//! function's only in its own unit), and anything else opens it. Over one
//! unit it is that unit's view (rule 12n's in-unit decision); over every
//! analysed unit it is the run's (the fold in `qcore-audit`, which demotes a
//! row whose in-unit set the run's does not contain).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// Card **RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION** (rule 12n): the separator
/// between a union's key and the arm path of a place inside it
/// (`rd_kafka_op_s.rko_u.*@leaders.cb`). A union key without it (`U.*`) is
/// the whole union, unplaced.
pub const ARM: char = '@';

/// A union key's two parts: the union (`T.path.*` or `U.*`) and the arm path
/// when the key names a place inside it. `None` for a key that is not a
/// union's.
#[must_use]
pub fn union_parts(key: &str) -> Option<(&str, Option<&str>)> {
    match key.split_once(ARM) {
        Some((union, arm)) if union.ends_with(".*") => Some((union, Some(arm))),
        Some(_) => None,
        None => key.ends_with(".*").then_some((key, None)),
    }
}

fn is_union_key(key: &str) -> bool {
    union_parts(key).is_some()
}

/// A byte range `(offset, size)` within a union.
pub type Range = (u64, u64);

fn overlaps(left: &BTreeSet<Range>, right: &BTreeSet<Range>) -> bool {
    left.iter().any(|(a, n)| {
        right
            .iter()
            .any(|(b, m)| *a < b.saturating_add(*m) && *b < a.saturating_add(*n))
    })
}

/// Whether two places may share a byte: only two known, disjoint sets of
/// ranges are apart.
fn meet(left: Option<BTreeSet<Range>>, right: Option<BTreeSet<Range>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => overlaps(&left, &right),
        _ => true,
    }
}

/// Card **RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION** (`QCore.UnionArmLayout`
/// `placesAlias`): whether two places of one union alias: the same arm path,
/// the whole union on either side, or bytes the layout does not keep apart.
#[must_use]
pub fn places_alias(
    arm: Option<&str>,
    other: Option<&str>,
    ranges: Option<BTreeSet<Range>>,
    other_ranges: Option<BTreeSet<Range>>,
) -> bool {
    match (arm, other) {
        (Some(arm), Some(other)) => arm == other || meet(ranges, other_ranges),
        _ => true,
    }
}

/// `LAYOUT_VECTORS` (`QCore.UnionArmLayout.layoutVectors`): two places of a
/// union (`None`: the whole union), their ranges (`None`: not given) and
/// whether they alias.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub(crate) const LAYOUT_VECTORS: &[(
    Option<&str>,
    Option<&str>,
    Option<&[Range]>,
    Option<&[Range]>,
    bool,
)] = &[
    (Some("a.cb"), Some("a.cb"), None, None, true),
    (
        Some("a.cb"),
        Some("b.cb"),
        Some(&[(0, 8)]),
        Some(&[(32, 8)]),
        false,
    ),
    (
        Some("a.cb"),
        Some("b.cb"),
        Some(&[(0, 8)]),
        Some(&[(4, 8)]),
        true,
    ),
    (Some("a.cb"), Some("b.cb"), None, Some(&[(32, 8)]), true),
    (None, Some("b.cb"), Some(&[(0, 8)]), Some(&[(32, 8)]), true),
    (
        Some("a.cb"),
        Some("b.cb"),
        Some(&[(0, 8)]),
        Some(&[(8, 8)]),
        false,
    ),
    (
        Some("a"),
        Some("b"),
        Some(&[(0, 8), (40, 8)]),
        Some(&[(44, 4)]),
        true,
    ),
];

/// Card **UNION-OPAQUE-RECORD-WRITES-BY-TYPE** (`QCore.UnionArmLayout`
/// `TargetClass`): an opaque write's target, as the decision for one union
/// reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetClass {
    /// No target is read: a dereference, an array element, a pointer.
    Untargeted,
    /// A place of the union read (decided by its own bytes, as a store).
    SameUnion,
    /// A place of another union.
    OtherUnion,
    /// A separate object (`whole`: the object itself, no member path).
    Separate { whole: bool },
    /// A member path below a pointer to a named record.
    Below,
}

/// Card **UNION-OPAQUE-RECORD-WRITES-BY-TYPE** (`QCore.UnionArmLayout.mayLie`):
/// whether an opaque write of a record the union holds may lie in the
/// union's bytes at an unstated position, so that the record's placements
/// decide (the reading by type). `root_held`: the union holds the record
/// the member path starts from; `path_clear`: the tables read the path as
/// passing through no union. A write into the union itself is decided by
/// its own bytes (an open store at its place), never here.
#[must_use]
pub fn opaque_write_may_lie(class: TargetClass, root_held: bool, path_clear: bool) -> bool {
    match class {
        TargetClass::Untargeted | TargetClass::OtherUnion => true,
        TargetClass::SameUnion => false,
        TargetClass::Separate { whole } => !whole && !path_clear,
        TargetClass::Below => root_held || !path_clear,
    }
}

/// `OPAQUE_VECTORS` (`QCore.UnionArmLayout.opaqueVectors`): a target class,
/// `root_held`, `path_clear`, and whether the write may lie in the union.
#[cfg(test)]
pub(crate) const OPAQUE_VECTORS: &[(TargetClass, bool, bool, bool)] = &[
    (TargetClass::Untargeted, false, true, true),
    (TargetClass::SameUnion, true, false, false),
    (TargetClass::OtherUnion, false, true, true),
    (TargetClass::Separate { whole: true }, false, false, false),
    (TargetClass::Separate { whole: false }, false, true, false),
    (TargetClass::Separate { whole: false }, false, false, true),
    (TargetClass::Below, false, true, false),
    (TargetClass::Below, true, true, true),
    (TargetClass::Below, false, false, true),
];

/// Card **RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION** (rule 12n, layout route):
/// Clang's layout of one union as one unit compiled it, read by
/// [`MemberFacts::probe_layouts`]. Every range is relative to the union's
/// first byte. An entry that is absent, or `None`, is unknown: it is never
/// read as disjoint from anything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionLayout {
    /// Each arm path the unit probed (every path its keys name, every
    /// function-pointer member and every record held by value reached
    /// through the union's anonymous members): its range.
    pub paths: BTreeMap<String, Range>,
    /// Each named record held by value inside the union (transitively): its
    /// placements (`None` when one cannot be placed: an array of records, an
    /// unnamed member on the way, a record the unit does not define).
    pub records: BTreeMap<String, Option<BTreeSet<Range>>>,
    /// Each function-pointer member of such a record, by its normal key
    /// (`R.path`): where it lies in the union.
    pub members: BTreeMap<String, Option<BTreeSet<Range>>>,
}

/// The expressions one layout probe asks Clang for.
#[derive(Default)]
struct LayoutProbe {
    expressions: BTreeSet<String>,
}

impl LayoutProbe {
    fn want(&mut self, expression: String) -> String {
        self.expressions.insert(expression.clone());
        expression
    }
}

/// A record placement to probe: its offset, the field's size and the
/// record's own size (`None` when the record cannot be spelled), all as
/// expressions; `None` when an unnamed member lies on the way.
type Placement = Option<(String, String, Option<String>)>;

/// One union's probe: offsets are from the root record's first byte (the
/// union's own offset is `base`).
#[derive(Default)]
struct UnionPlan {
    base: String,
    paths: BTreeMap<String, (String, String)>,
    held: Vec<(String, Placement)>,
    records: BTreeMap<String, Option<RecordPlan>>,
}

/// A record held by value: its function-pointer members (by the key a store
/// into them has) and the records it holds, from its own first byte.
#[derive(Clone, Default)]
struct RecordPlan {
    members: BTreeMap<String, Option<(String, String)>>,
    held: Vec<(String, Placement)>,
}

/// Merge placements: an unknown placement stays unknown.
fn add_ranges(
    map: &mut BTreeMap<String, Option<BTreeSet<Range>>>,
    key: &str,
    ranges: Option<BTreeSet<Range>>,
) {
    match (map.get_mut(key), ranges) {
        (None, ranges) => {
            map.insert(key.to_owned(), ranges);
        }
        (Some(slot), None) => *slot = None,
        (Some(Some(known)), Some(ranges)) => known.extend(ranges),
        (Some(None), Some(_)) => {}
    }
}

impl UnionPlan {
    /// The layout Clang's values give; `None` when the union's own offset
    /// is not given.
    fn layout(&self, value: &dyn Fn(&str) -> Option<u64>) -> Option<UnionLayout> {
        let base = value(&self.base)?;
        let mut layout = UnionLayout::default();
        for (arm, (offset, size)) in &self.paths {
            if let (Some(offset), Some(size)) = (value(offset), value(size))
                && let Some(offset) = offset.checked_sub(base)
            {
                layout.paths.insert(arm.clone(), (offset, size));
            }
        }
        // A placement: the field's offset, when the field is exactly one
        // record (an array of records is not placed).
        let place = |placement: &Placement, from: u64| -> Option<(u64, u64)> {
            let (offset, size, record_size) = placement.as_ref()?;
            let (offset, size) = (value(offset)?, value(size)?);
            (value(record_size.as_ref()?)? == size).then_some(())?;
            Some((from.checked_add(offset)?, size))
        };
        let mut work: Vec<(String, Option<BTreeSet<Range>>, usize)> = Vec::new();
        for (record, placement) in &self.held {
            let ranges = place(placement, 0).and_then(|(offset, size)| {
                Some(BTreeSet::from([(offset.checked_sub(base)?, size)]))
            });
            work.push((record.clone(), ranges, 0));
        }
        while let Some((record, ranges, depth)) = work.pop() {
            add_ranges(&mut layout.records, &record, ranges.clone());
            let Some(Some(plan)) = self.records.get(&record) else {
                continue;
            };
            for (member, spec) in &plan.members {
                let member_ranges = ranges.as_ref().and_then(|ranges| {
                    let (offset, size) = spec.as_ref()?;
                    let (offset, size) = (value(offset)?, value(size)?);
                    ranges
                        .iter()
                        .map(|(at, _)| Some((at.checked_add(offset)?, size)))
                        .collect::<Option<BTreeSet<_>>>()
                });
                add_ranges(&mut layout.members, member, member_ranges);
            }
            if depth > 16 {
                continue;
            }
            for (held, placement) in &plan.held {
                let held_ranges = ranges.as_ref().and_then(|ranges| {
                    ranges
                        .iter()
                        .map(|(at, _)| place(placement, *at))
                        .collect::<Option<BTreeSet<_>>>()
                });
                work.push((held.clone(), held_ranges, depth + 1));
            }
        }
        Some(layout)
    }
}

/// What a stored or handed value is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "flow")]
pub enum Flow {
    /// A function designator.
    Function { name: String },
    /// A null pointer constant: no callback.
    Null,
    /// A read of a function-pointer member, `TAG.PATH`.
    Member { member: String },
    /// A formal of the enclosing function.
    Formal { function: String, index: usize },
    /// Anything else.
    Open { why: String },
}

/// Card **UNION-OPAQUE-RECORD-WRITES-BY-TYPE** (rule 12n): the target of an
/// opaque write of a record (a whole-object assignment from a call or
/// another type, a `memcpy`-family call into it, an initializer list that
/// cannot be mapped), read from its target expression as an assignment
/// store's place is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "target")]
pub enum OpaqueTarget {
    /// A place a union holds: its key (`U.*@arm.path`, or the whole union
    /// `U.*`). The write is also an open store at that key, so its own
    /// bytes decide which of the union's places it meets.
    Union { key: String },
    /// A separate object: a local, a global or a by-value formal (whose
    /// type is `record`, when named), or a member path (`path`) rooted at
    /// one. The object itself lies in no union's bytes; a member path in it
    /// does only where the path meets a union the tables read.
    Separate {
        record: Option<String>,
        path: Vec<String>,
    },
    /// A member path (`path`, from the record) below a pointer to the named
    /// `record`: it lies in a union's bytes only where the union holds the
    /// record by value, or where the path meets a union the tables read.
    Below { record: String, path: Vec<String> },
    /// Anything else (a dereference, an array element, a pointer handed to
    /// a copier, an initializer list): it may lie anywhere, keyed by type.
    Untargeted,
}

/// One unit's facts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberFacts {
    /// S3 source facts, read only when a selected declaration carries a
    /// lifetime protocol or scoped prerequisites. Historical environments
    /// without S3 keep their serialized facts unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime_source: Option<crate::loop_lifetime_source::SourceFacts>,
    /// `TAG.PATH` (or `?.NAME` when the record is not read) -> stored values.
    pub stores: BTreeMap<String, BTreeSet<Flow>>,
    /// Callee -> position -> function-pointer actuals.
    pub calls: BTreeMap<String, BTreeMap<usize, BTreeSet<Flow>>>,
    /// Functions named other than as a direct callee.
    pub address_taken: BTreeSet<String>,
    /// Functions this unit defines with internal linkage.
    pub internal: BTreeSet<String>,
    /// Records whose function-pointer members something opaque may write,
    /// with why.
    pub opened: BTreeMap<String, String>,
    /// Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE (rule 12n): where each
    /// opaque write of a record in [`Self::opened`] lands, by its target
    /// expression. A record in `opened` without an entry (facts written
    /// before the card) is read as untargeted: it may lie in any union.
    #[serde(default)]
    pub opened_targets: BTreeMap<String, BTreeSet<OpaqueTarget>>,
    /// Typedef names of function types and of pointers to them (build state).
    #[serde(skip)]
    function_typedefs: BTreeSet<String>,
    #[serde(skip)]
    pointer_typedefs: BTreeSet<String>,
    /// Typedef name -> the record it names (build state).
    #[serde(skip)]
    record_typedefs: BTreeMap<String, String>,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): every enumerator's
    /// value by declaration id, for the arm check's case labels (not
    /// evidence).
    #[serde(skip)]
    enumerators: BTreeMap<String, i64>,
    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS (rule 12n): each record the unit
    /// defines once at file scope, with its fields in initializer order;
    /// `None` when its fields cannot be mapped (an anonymous member, a second
    /// different definition, a definition inside a function) (build state).
    #[serde(skip)]
    record_fields: BTreeMap<String, Option<RecordShape>>,
    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q3): each record the
    /// unit defines at file scope, as member keys read it (`None`: defined
    /// differently twice in the unit).
    #[serde(default)]
    pub records: BTreeMap<String, Option<RecordKeys>>,
    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (rule 12n, layout route):
    /// Clang's layout of each union this unit's keys name, by the union's
    /// key (`T.path.*` or `U.*`); read only by [`read_unit`] (the pre-pass
    /// and data-only units). A union without one is unplaced.
    #[serde(default)]
    pub union_layouts: BTreeMap<String, UnionLayout>,
    /// The union tags the unit defines (build state).
    #[serde(skip)]
    unions: BTreeSet<String>,
    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 12m): each function
    /// definition's caller-branch reading
    /// ([`crate::handoff_branch::function_handling`]).
    #[serde(default)]
    pub handling: BTreeMap<String, crate::handoff_branch::FunctionHandling>,
    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): each declared
    /// function's result type as Clang spells it.
    #[serde(default)]
    pub result_types: BTreeMap<String, String>,
    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): each enumerator,
    /// by the spelling of every type that names its enumeration (`enum TAG`,
    /// a typedef name), with its value as Clang read the unit.
    #[serde(default)]
    pub typed_constants: BTreeMap<String, BTreeMap<String, i64>>,
    /// Card FB-TEARDOWN-DESTROY-DECLARATIONS (rule 30, decision 2): each
    /// declared function's parameter types as Clang spells them, for an `@`
    /// constant of a parameter guard (rule 13l, resolved in the guarded
    /// parameter's own enumeration). Omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameter_types: BTreeMap<String, Vec<String>>,
    /// The typedef declarations that name an enumeration (build state).
    #[serde(skip)]
    enum_typedefs: BTreeSet<String>,
    /// Each enumeration's enumerators, by its declaration id (build state).
    #[serde(skip)]
    enum_members: BTreeMap<String, Vec<(String, i64)>>,
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (rule 12n, Q3): a record's
/// fields as member keys read them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordKeys {
    pub union: bool,
    pub fields: BTreeMap<String, FieldKey>,
}

/// What one field holds, for member keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "field")]
pub enum FieldKey {
    /// A function pointer.
    Function,
    /// A named record by value (or an array of them).
    Named { record: String },
    /// An anonymous record by value.
    Inline { keys: Box<RecordKeys> },
    /// Anything else.
    Plain,
}

/// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a record's fields as an initializer
/// list names them (unnamed bit-fields take no initializer).
#[derive(Clone, Debug, PartialEq, Eq)]
struct RecordShape {
    union: bool,
    fields: Vec<FieldShape>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldShape {
    name: String,
    function_pointer: bool,
}

const COPIERS: [&str; 6] = [
    "memcpy",
    "memmove",
    "bcopy",
    "__builtin_memcpy",
    "__builtin_memmove",
    "__builtin___memcpy_chk",
];

fn type_text(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or("")
}

fn qual_text(node: &Value) -> &str {
    node["type"]["qualType"].as_str().unwrap_or("")
}

/// `struct NAME` / `union NAME` in a type spelling.
fn spelled_record(text: &str) -> Option<String> {
    for keyword in ["struct ", "union "] {
        if let Some(at) = text.find(keyword) {
            let name: String = text[at + keyword.len()..]
                .chars()
                .take_while(|ch| *ch == '_' || ch.is_ascii_alphanumeric())
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: whether a node's type is an
/// anonymous record. Clang spells it `struct T::(unnamed at ...)` in its
/// desugared form, which [`spelled_record`] reads as the holder `T`.
fn anonymous_record(node: &Value) -> bool {
    [type_text(node), qual_text(node)].iter().any(|text| {
        let bare = strip_qualifiers(text);
        (bare.starts_with("struct ") || bare.starts_with("union "))
            && (bare.contains("(unnamed") || bare.contains("(anonymous"))
    })
}

/// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: callees that fill their
/// destination with one byte value (scalar data, never a function
/// designator): an address handed to one is not a type-punned escape.
const FILLERS: [&str; 6] = [
    "memset",
    "__builtin_memset",
    "__builtin___memset_chk",
    "bzero",
    "explicit_bzero",
    "__explicit_bzero_chk",
];

fn strip_qualifiers(text: &str) -> &str {
    let mut text = text.trim();
    loop {
        let before = text;
        for prefix in ["const ", "volatile ", "restrict "] {
            text = text.strip_prefix(prefix).unwrap_or(text).trim();
        }
        for suffix in [" const", " volatile", " restrict"] {
            text = text.strip_suffix(suffix).unwrap_or(text).trim();
        }
        if before == text {
            return text;
        }
    }
}

/// Card LOG-PRINT-BODY-CONTRACTS (rule 12n, a fail-closed repair): whether a
/// type spelling is a pointer to a function, `R (*)(P...)`. Clang keeps a
/// declarator's parentheses as sugar, so `void (*log_cb)` (chunkio's
/// `cio_set_log_callback` formal, a `void *`) is spelled `void (*)` and an
/// array pointer `int (*)[4]`; neither is a function pointer. Reading one as
/// such recorded a `void *` formal as a function-pointer formal, whose values
/// are only its callers' function-pointer actuals, so a store of it closed a
/// member over values a `void *` actual never contributes. Only a parameter
/// list after `(*)` makes a function pointer.
pub(crate) fn spells_function_pointer(text: &str) -> bool {
    text.contains("(*)(")
}

/// Whether a type spelling is a function type `R (P...)` (a typedef of one):
/// a parameter list and no pointer declarator, so neither `void (*)` nor
/// `int (*)[4]`.
fn spells_function_type(text: &str) -> bool {
    !text.contains("(*)") && text.trim_end().ends_with(')')
}

pub(crate) fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

pub(crate) fn null_constant(node: &Value) -> bool {
    let node = strip(node);
    match kind(node) {
        "ImplicitValueInitExpr" | "GNUNullExpr" => true,
        "IntegerLiteral" => node["value"].as_str() == Some("0"),
        "CStyleCastExpr" => children(node).first().is_some_and(null_constant),
        _ => false,
    }
}

pub(crate) fn designator(node: &Value) -> Option<&str> {
    let node = strip(node);
    (kind(node) == "DeclRefExpr" && node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
        .then(|| node["referencedDecl"]["name"].as_str())
        .flatten()
}

/// The direct callee of a call node, when it is a function designator.
pub(crate) fn direct_callee(call: &Value) -> Option<&str> {
    children(call).first().and_then(designator)
}

/// The enclosing function while walking a body.
struct Enclosing<'a> {
    name: &'a str,
    parameters: Vec<&'a str>,
}

impl MemberFacts {
    /// Whether a type spelling is a function pointer (directly, or a pointer
    /// to a function typedef, or a typedef of a function pointer).
    pub(crate) fn function_pointer(&self, node: &Value) -> bool {
        for text in [type_text(node), qual_text(node)] {
            if spells_function_pointer(text) {
                return true;
            }
            let bare = strip_qualifiers(text);
            if let Some(pointee) = bare.strip_suffix('*') {
                let pointee = strip_qualifiers(pointee);
                if self.function_typedefs.contains(pointee) && !pointee.contains('*') {
                    return true;
                }
            } else if self.pointer_typedefs.contains(bare) {
                return true;
            }
        }
        false
    }

    /// The record a type spelling names, through typedefs.
    fn record_of(&self, text: &str, pointee: bool) -> Option<String> {
        let mut bare = strip_qualifiers(text);
        if pointee {
            bare = strip_qualifiers(bare.strip_suffix('*')?);
            if bare.contains('*') {
                return None;
            }
        } else if bare.contains('*') {
            return None;
        }
        spelled_record(bare).or_else(|| self.record_typedefs.get(bare).cloned())
    }

    pub(crate) fn record_of_node(&self, node: &Value, pointee: bool) -> Option<String> {
        self.record_of(type_text(node), pointee)
            .or_else(|| self.record_of(qual_text(node), pointee))
    }

    /// `TAG.PATH` of a member expression, `?.NAME` when its record is not
    /// read.
    pub(crate) fn member_key(&self, member: &Value) -> String {
        let mut segments = Vec::new();
        let mut node = member;
        let tag = loop {
            let Some(name) = node["name"].as_str() else {
                break None;
            };
            segments.push(name.to_owned());
            let Some(base) = children(node).first() else {
                break None;
            };
            if node["isArrow"].as_bool() == Some(true) {
                break self.record_of_node(strip(base), true);
            }
            let base = strip(base);
            if kind(base) == "MemberExpr" {
                node = base;
                continue;
            }
            break self.record_of_node(base, false);
        };
        segments.reverse();
        match tag {
            Some(tag) => format!("{tag}.{}", segments.join(".")),
            None => format!("?.{}", segments.last().cloned().unwrap_or_default()),
        }
    }

    fn flows(&self, node: &Value, enclosing: Option<&Enclosing<'_>>) -> BTreeSet<Flow> {
        let node = strip(node);
        if let Some(name) = designator(node) {
            return BTreeSet::from([Flow::Function {
                name: name.to_owned(),
            }]);
        }
        if null_constant(node) {
            return BTreeSet::from([Flow::Null]);
        }
        match kind(node) {
            // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q3): keyed by its
            // innermost named record.
            "MemberExpr" => BTreeSet::from([Flow::Member {
                member: self.callback_key(node),
            }]),
            // A formal of another type (a `void *` a caller may hand any
            // function pointer through) is open: its calls record no actual.
            "DeclRefExpr"
                if node["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl")
                    && !self.function_pointer(node) =>
            {
                BTreeSet::from([Flow::Open {
                    why: "a formal that is not a function pointer".to_owned(),
                }])
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl") => {
                let id = node["referencedDecl"]["id"].as_str().unwrap_or("");
                match enclosing.and_then(|enclosing| {
                    enclosing
                        .parameters
                        .iter()
                        .position(|parameter| *parameter == id)
                        .map(|index| (enclosing.name, index))
                }) {
                    Some((function, index)) => BTreeSet::from([Flow::Formal {
                        function: function.to_owned(),
                        index,
                    }]),
                    None => BTreeSet::from([Flow::Open {
                        why: "an unread formal".to_owned(),
                    }]),
                }
            }
            "ConditionalOperator" => {
                let arms = children(node);
                let mut out = BTreeSet::new();
                for arm in arms.iter().skip(1) {
                    out.extend(self.flows(arm, enclosing));
                }
                if arms.len() != 3 {
                    out.insert(Flow::Open {
                        why: "a conditional".to_owned(),
                    });
                }
                out
            }
            "DeclRefExpr" => BTreeSet::from([Flow::Open {
                why: "a variable".to_owned(),
            }]),
            other => BTreeSet::from([Flow::Open {
                why: format!("a `{other}`"),
            }]),
        }
    }

    /// Whether an initializer list holds a function-pointer value that is not
    /// null.
    fn initializes_function_pointer(&self, node: &Value) -> bool {
        children(node).iter().any(|child| {
            let inner = strip(child);
            if kind(inner) == "InitListExpr" {
                return self.initializes_function_pointer(inner);
            }
            designator(inner).is_some() || (self.function_pointer(child) && !null_constant(child))
        })
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: whether an initializer entry for
    /// a field that is not a function pointer carries no callback: an implicit
    /// value, a string literal, a nested list whose entries carry none, or a
    /// scalar that is neither a function nor a function pointer. A record or
    /// array value (a copy that may carry function pointers) carries one.
    fn carries_no_callback(&self, node: &Value) -> bool {
        let inner = strip(node);
        match kind(inner) {
            "ImplicitValueInitExpr" | "StringLiteral" => true,
            "InitListExpr" => children(inner)
                .iter()
                .all(|child| self.carries_no_callback(child)),
            _ => {
                designator(inner).is_none()
                    && (null_constant(node) || !self.function_pointer(node))
                    && !self.function_pointer(inner)
                    && self.record_of_node(inner, false).is_none()
                    && !type_text(inner).contains('[')
            }
        }
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS (rule 12n): map an initializer
    /// list of `record` onto its fields: each function-pointer field's entry
    /// is a store of that value into `TAG.FIELD` (an implicit entry is a null
    /// pointer). `false`, recording nothing, when the list cannot be mapped
    /// exactly (the record's fields are unknown or ambiguous, the entries do
    /// not match them one for one, a union names no field, or another
    /// field's entry may carry a callback); the caller then opens the record.
    fn map_initializer(
        &mut self,
        record: &str,
        node: &Value,
        enclosing: Option<&Enclosing<'_>>,
    ) -> bool {
        let Some(Some(shape)) = self.record_fields.get(record) else {
            return false;
        };
        let entries = children(node);
        let pairs: Vec<(&FieldShape, &Value)> = if shape.union {
            let Some(name) = node["field"]["name"].as_str() else {
                return false;
            };
            let ([entry], Some(field)) = (
                entries,
                shape.fields.iter().find(|field| field.name == name),
            ) else {
                return false;
            };
            vec![(field, entry)]
        } else {
            if entries.len() != shape.fields.len() {
                return false;
            }
            shape.fields.iter().zip(entries).collect()
        };
        let mut stores = Vec::new();
        for (field, entry) in pairs {
            if field.function_pointer {
                stores.push((
                    format!("{record}.{}", field.name),
                    self.flows(entry, enclosing),
                ));
            } else if !self.carries_no_callback(entry) {
                return false;
            }
        }
        for (key, flows) in stores {
            self.stores.entry(key).or_default().extend(flows);
        }
        true
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a record definition's fields in
    /// initializer order, `None` when they cannot be mapped.
    fn record_shape(&self, node: &Value) -> Option<RecordShape> {
        let mut fields = Vec::new();
        for child in children(node) {
            if kind(child) != "FieldDecl" {
                continue;
            }
            match child["name"].as_str().filter(|name| !name.is_empty()) {
                // An unnamed bit-field takes no initializer.
                None if child["isBitfield"].as_bool() == Some(true) => {}
                // An anonymous structure or union member.
                None => return None,
                Some(name) => fields.push(FieldShape {
                    name: name.to_owned(),
                    function_pointer: self.function_pointer(child),
                }),
            }
        }
        Some(RecordShape {
            union: node["tagUsed"].as_str() == Some("union"),
            fields,
        })
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: every file-scope record
    /// definition (nested ones included), each tag once.
    fn collect_records(&mut self, node: &Value) {
        if kind(node) != "RecordDecl" {
            return;
        }
        for child in children(node) {
            self.collect_records(child);
        }
        if node["completeDefinition"].as_bool() != Some(true) {
            return;
        }
        let Some(name) = node["name"].as_str().filter(|name| !name.is_empty()) else {
            return;
        };
        let shape = self.record_shape(node);
        let entry = self
            .record_fields
            .entry(name.to_owned())
            .or_insert(shape.clone());
        if *entry != shape {
            *entry = None;
        }
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q3): the record as
        // member keys read it.
        let keys = self.record_keys(node);
        if keys.union {
            self.unions.insert(name.to_owned());
        }
        let entry = self
            .records
            .entry(name.to_owned())
            .or_insert(Some(keys.clone()));
        if entry.as_ref() != Some(&keys) {
            *entry = None;
        }
    }

    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q3): a record
    /// definition's fields as member keys read them. An anonymous record a
    /// field names is read inline from the record declaration Clang puts just
    /// before it.
    fn record_keys(&self, node: &Value) -> RecordKeys {
        let mut keys = RecordKeys {
            union: node["tagUsed"].as_str() == Some("union"),
            fields: BTreeMap::new(),
        };
        let mut anonymous: Option<&Value> = None;
        for child in children(node) {
            match kind(child) {
                "RecordDecl" => {
                    anonymous = (child["name"].as_str().is_none_or(str::is_empty)
                        && child["completeDefinition"].as_bool() == Some(true))
                    .then_some(child);
                }
                "FieldDecl" => {
                    let name = child["name"].as_str().unwrap_or("").to_owned();
                    let text = type_text(child);
                    let field = if self.function_pointer(child) {
                        FieldKey::Function
                    } else if text.contains("(unnamed") || text.contains("(anonymous") {
                        match anonymous {
                            Some(record) if !text.contains('*') && !text.contains('[') => {
                                FieldKey::Inline {
                                    keys: Box::new(self.record_keys(record)),
                                }
                            }
                            _ => FieldKey::Plain,
                        }
                    } else if text.contains('*') {
                        FieldKey::Plain
                    } else {
                        // A record by value, or an array of them: a store
                        // through it is keyed by that record (`a[i].cb` reads
                        // its base's record).
                        let element = text.split('[').next().unwrap_or(text);
                        match spelled_record(strip_qualifiers(element)).or_else(|| {
                            self.record_typedefs.get(strip_qualifiers(element)).cloned()
                        }) {
                            Some(record) => FieldKey::Named { record },
                            None => FieldKey::Plain,
                        }
                    };
                    keys.fields.insert(name, field);
                    anonymous = None;
                }
                _ => {}
            }
        }
        keys
    }

    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: open a record for an opaque
    /// whole-object write (a `memcpy`-family call, an assignment from another
    /// type or a call). It replaces an initializer-list reason, which opens
    /// the record's own members but not a member of a record it embeds, so
    /// the stronger reason is the one a holder's member reads.
    ///
    /// Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: the write's target is kept
    /// beside it ([`Self::opened_targets`]).
    fn open_opaquely(&mut self, record: String, why: String, target: OpaqueTarget) {
        self.opened_targets
            .entry(record.clone())
            .or_default()
            .insert(target);
        self.opened
            .entry(record)
            .and_modify(|earlier| {
                if earlier == "an initializer list" {
                    earlier.clone_from(&why);
                }
            })
            .or_insert(why);
    }

    /// Card **UNION-OPAQUE-RECORD-WRITES-BY-TYPE** (rule 12n): the target of
    /// an opaque write into `place`, read as an assignment store's place is
    /// (`callback_key`): a place a union holds keeps its union key and arm
    /// path; a by-value variable or formal, or a member path rooted at one,
    /// is a separate object; a member path below a pointer to a named record
    /// keeps the record and the path; anything else is untargeted.
    fn opaque_target(&self, place: &Value) -> OpaqueTarget {
        let place = strip(place);
        if let Some(key) = self.union_place(place) {
            return OpaqueTarget::Union { key };
        }
        let mut path = Vec::new();
        let mut node = place;
        loop {
            match kind(node) {
                "MemberExpr" => {
                    let (Some(name), Some(base)) = (node["name"].as_str(), children(node).first())
                    else {
                        return OpaqueTarget::Untargeted;
                    };
                    if name.is_empty() {
                        return OpaqueTarget::Untargeted;
                    }
                    path.push(name.to_owned());
                    let base = strip(base);
                    if node["isArrow"].as_bool() == Some(true) {
                        path.reverse();
                        return match self.record_of_node(base, true) {
                            Some(record) => OpaqueTarget::Below { record, path },
                            None => OpaqueTarget::Untargeted,
                        };
                    }
                    node = base;
                }
                "DeclRefExpr"
                    if matches!(
                        node["referencedDecl"]["kind"].as_str(),
                        Some("VarDecl" | "ParmVarDecl")
                    ) && !type_text(node).contains('*') =>
                {
                    path.reverse();
                    return OpaqueTarget::Separate {
                        record: self.record_of_node(node, false),
                        path,
                    };
                }
                _ => return OpaqueTarget::Untargeted,
            }
        }
    }

    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: whether a node's type (or its
    /// pointee's) is a union.
    fn union_node(&self, node: &Value, pointee: bool) -> bool {
        [type_text(node), qual_text(node)].into_iter().any(|text| {
            let mut bare = strip_qualifiers(text);
            if pointee {
                match bare.strip_suffix('*') {
                    Some(inner) => bare = strip_qualifiers(inner),
                    None => return false,
                }
            }
            bare.starts_with("union ")
                || self
                    .record_typedefs
                    .get(bare)
                    .is_some_and(|record| self.unions.contains(record))
        })
    }

    /// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (gateway rule 12n, Q3): the
    /// key of a function-pointer member expression for the callback closure,
    /// by its **innermost named record**: `rk->rk_conf.dr_msg_cb` and
    /// `conf->dr_msg_cb` are both `rd_kafka_conf_s.dr_msg_cb`. A member read
    /// from a union object is keyed by the union with `*` (`T.u.*`, or
    /// `U.*` for a union record), so the union's members share their stores.
    /// `?.NAME` when the record is not read.
    pub(crate) fn callback_key(&self, member: &Value) -> String {
        let mut chain: Vec<(&str, &Value, bool)> = Vec::new();
        let mut node = member;
        loop {
            let Some(name) = node["name"].as_str() else {
                return "?.".to_owned();
            };
            let Some(base) = children(node).first() else {
                return format!("?.{name}");
            };
            let base = strip(base);
            let arrow = node["isArrow"].as_bool() == Some(true);
            chain.push((name, base, arrow));
            if !arrow && kind(base) == "MemberExpr" {
                node = base;
                continue;
            }
            break;
        }
        chain.reverse();
        let (first, root, arrow) = chain[0];
        let Some(mut tag) = self.record_of_node(root, arrow) else {
            let last = chain.last().map_or(first, |(name, _, _)| name);
            return format!("?.{last}");
        };
        // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (rule 12n): a member read
        // through a union keeps its arm path beside the union's key
        // (`U.*@arm.field`), so the run's layout can place it; without a
        // layout the key aliases every member of the union, as before.
        let arm = |from: usize| {
            chain[from..]
                .iter()
                .map(|(name, _, _)| *name)
                .collect::<Vec<_>>()
                .join(".")
        };
        if self.union_node(root, arrow) {
            return format!("{tag}.*{ARM}{}", arm(0));
        }
        let mut path: Vec<&str> = Vec::new();
        for (index, (name, base, _)) in chain.iter().enumerate() {
            if index > 0 {
                // `base` is the previous member's value: a record object.
                // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: an anonymous
                // record (Clang's `struct T::(unnamed at ...)`) names no
                // record: it was read as its holder `T`, so an anonymous
                // union was keyed `T.*` (the holder's whole record) and a
                // place below an anonymous structure lost its path. It is
                // now read under its path, as the tables normalise it.
                let named = (!anonymous_record(base))
                    .then(|| self.record_of_node(base, false))
                    .flatten();
                if self.union_node(base, false) {
                    // A named union is keyed by itself (`U.*`), as the
                    // tables normalise it; an anonymous one under its path.
                    if let Some(record) = named {
                        return format!("{record}.*{ARM}{}", arm(index));
                    }
                    path.push("*");
                    return format!("{tag}.{}{ARM}{}", path.join("."), arm(index));
                }
                if let Some(record) = named {
                    tag = record;
                    path.clear();
                }
            }
            path.push(name);
        }
        format!("{tag}.{}", path.join("."))
    }

    /// Card **RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION** (rule 12n): the key of
    /// a place that is a union object or lies inside one: `U.*@arm.path` for
    /// a place inside a union, `U.*` (or `T.path.*`) for the union object
    /// itself; `None` for a place no union holds, or one whose record is not
    /// read.
    fn union_place(&self, place: &Value) -> Option<String> {
        let place = strip(place);
        if kind(place) != "MemberExpr" {
            return self
                .union_node(place, false)
                .then(|| self.record_of_node(place, false))
                .flatten()
                .map(|union| format!("{union}.*"));
        }
        let key = self.callback_key(place);
        if key.starts_with("?.") {
            return None;
        }
        if union_parts(&key).is_some() {
            return Some(key);
        }
        if self.union_node(place, false) {
            let named = (!anonymous_record(place))
                .then(|| self.record_of_node(place, false))
                .flatten();
            return Some(match named {
                Some(union) => format!("{union}.*"),
                None => format!("{key}.*"),
            });
        }
        None
    }

    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: the place an address
    /// expression names (`&P`, or an array `P` decayed to a pointer).
    fn addressed(node: &Value) -> Option<&Value> {
        let mut node = node;
        loop {
            match kind(node) {
                "ParenExpr" => node = children(node).first()?,
                "ImplicitCastExpr" if node["castKind"].as_str() == Some("ArrayToPointerDecay") => {
                    return children(node).first();
                }
                "ImplicitCastExpr" | "CStyleCastExpr" => node = children(node).first()?,
                "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                    return children(node).first();
                }
                _ => return None,
            }
        }
    }

    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: open a whole union, written
    /// opaquely (unplaced: it aliases every member of the union).
    fn open_union(&mut self, key: &str, why: &str) {
        let union = union_parts(key).map_or(key, |(union, _)| union).to_owned();
        self.stores.entry(union).or_default().insert(Flow::Open {
            why: why.to_owned(),
        });
    }

    /// Card **RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION** (rule 12n, layout
    /// route): read Clang's layout of every union this unit's keys name, in
    /// one enumerator probe over the unit's own text
    /// (`layout_extents::clang_constants`: `__builtin_offsetof` and `sizeof`
    /// as Clang evaluates them for the unit's target). For each union: the
    /// range of every arm path its keys name, of every function-pointer
    /// member and record held by value reached through its anonymous
    /// members, and, through every record so held (transitively), the
    /// placements of that record and of its function-pointer members. A
    /// probe that fails leaves the union without a layout (unplaced); a value
    /// Clang does not give, an unnamed member, or a field whose size is not
    /// its record's (an array) leaves that entry unknown.
    pub fn probe_layouts(&mut self, source: &str, unit: &crate::legacy_c::C11TranslationUnit<'_>) {
        // The probe's own names must not be ones the source could forge.
        if source.contains("__qcore_cg17") {
            return;
        }
        let mut unions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut note = |key: &str| {
            if let Some((union, arm)) = union_parts(key) {
                let arms = unions.entry(union.to_owned()).or_default();
                if let Some(arm) = arm {
                    arms.insert(arm.to_owned());
                }
            }
        };
        let member_flows = |flows: &BTreeSet<Flow>| {
            flows
                .iter()
                .filter_map(|flow| match flow {
                    Flow::Member { member } => Some(member.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        for (key, flows) in &self.stores {
            note(key);
            for member in member_flows(flows) {
                note(&member);
            }
        }
        for positions in self.calls.values() {
            for flows in positions.values() {
                for member in member_flows(flows) {
                    note(&member);
                }
            }
        }
        if unions.is_empty() {
            return;
        }
        let mut probe = LayoutProbe::default();
        let mut plans = Vec::new();
        for (union, arms) in &unions {
            if let Some(plan) = self.plan_union(union, arms, &mut probe) {
                plans.push((union.clone(), plan));
            }
        }
        if probe.expressions.is_empty() {
            return;
        }
        let expressions = probe.expressions.iter().cloned().collect::<Vec<_>>();
        let values = crate::layout_extents::clang_constants(source, unit, &expressions);
        if values.is_empty() {
            return;
        }
        let value = |expression: &str| -> Option<u64> {
            values
                .get(expression)
                .and_then(|value| u64::try_from(*value).ok())
        };
        for (union, plan) in plans {
            if let Some(layout) = plan.layout(&value) {
                self.union_layouts.insert(union, layout);
            }
        }
    }

    /// The probe plan of one union key (`T.path.*` or `U.*`): `None` when
    /// the unit does not define its record readably.
    fn plan_union(
        &self,
        union: &str,
        arms: &BTreeSet<String>,
        probe: &mut LayoutProbe,
    ) -> Option<UnionPlan> {
        let stem = union.strip_suffix(".*")?;
        let mut segments = stem.split('.');
        let root = segments.next()?.to_owned();
        let path = segments.map(str::to_owned).collect::<Vec<_>>();
        let root_keys = self.records.get(&root)?.as_ref()?;
        // The union's own keys: the root record itself, or an anonymous
        // union reached through anonymous members.
        let mut keys = root_keys;
        for segment in &path {
            keys = match keys.fields.get(segment) {
                Some(FieldKey::Inline { keys }) => keys,
                _ => return None,
            };
        }
        if !keys.union || path.iter().any(String::is_empty) {
            return None;
        }
        let ty = self.spelled_type(&root)?;
        let prefix = path.join(".");
        let full = |arm: &str| match (prefix.is_empty(), arm.is_empty()) {
            (true, _) => arm.to_owned(),
            (false, true) => prefix.clone(),
            (false, false) => format!("{prefix}.{arm}"),
        };
        let offset = |arm: &str| {
            let full = full(arm);
            if full.is_empty() {
                "0".to_owned()
            } else {
                format!("__builtin_offsetof({ty}, {full})")
            }
        };
        let size = |arm: &str| {
            let full = full(arm);
            if full.is_empty() {
                format!("sizeof({ty})")
            } else {
                format!("sizeof((({ty} *)0)->{full})")
            }
        };
        let mut plan = UnionPlan {
            base: probe.want(offset("")),
            ..UnionPlan::default()
        };
        for arm in arms {
            if arm.split('.').any(str::is_empty) {
                continue;
            }
            plan.paths.insert(
                arm.clone(),
                (probe.want(offset(arm)), probe.want(size(arm))),
            );
        }
        // The union's function-pointer members and records held by value,
        // through its anonymous members (an unnamed member cannot be spelled
        // in a probe: what lies below it stays unknown).
        let mut pending = vec![(String::new(), keys, false)];
        while let Some((at, keys, unnamed)) = pending.pop() {
            for (name, field) in &keys.fields {
                let here = if at.is_empty() {
                    name.clone()
                } else {
                    format!("{at}.{name}")
                };
                let unnamed = unnamed || name.is_empty();
                match field {
                    FieldKey::Function if !unnamed => {
                        plan.paths.insert(
                            here.clone(),
                            (probe.want(offset(&here)), probe.want(size(&here))),
                        );
                    }
                    FieldKey::Inline { keys } => pending.push((here, keys, unnamed)),
                    FieldKey::Named { record } => {
                        let placement = (!unnamed).then(|| {
                            (
                                probe.want(offset(&here)),
                                probe.want(size(&here)),
                                self.spelled_type(record)
                                    .map(|ty| probe.want(format!("sizeof({ty})"))),
                            )
                        });
                        plan.held.push((record.clone(), placement));
                        self.plan_record(record, probe, &mut plan, 0);
                    }
                    FieldKey::Function | FieldKey::Plain => {}
                }
            }
        }
        Some(plan)
    }

    /// `struct TAG` or `union TAG` for a record the unit defines readably.
    fn spelled_type(&self, tag: &str) -> Option<String> {
        let keys = self.records.get(tag)?.as_ref()?;
        Some(format!(
            "{} {tag}",
            if keys.union { "union" } else { "struct" }
        ))
    }

    /// The probe plan of a record held by value: its function-pointer
    /// members and the records it holds by value, relative to itself.
    fn plan_record(
        &self,
        record: &str,
        probe: &mut LayoutProbe,
        plan: &mut UnionPlan,
        depth: usize,
    ) {
        if depth > 16 || plan.records.contains_key(record) {
            return;
        }
        let Some(ty) = self.spelled_type(record) else {
            plan.records.insert(record.to_owned(), None);
            return;
        };
        let keys = self
            .records
            .get(record)
            .cloned()
            .flatten()
            .unwrap_or_default();
        let mut inner = RecordPlan::default();
        let mut nested = Vec::new();
        // Each entry: the path from the record, the keys there, whether an
        // unnamed member lies on the way, and the union key and the path
        // where the first union on the way starts (a member below it is
        // keyed `U.*@arm`, as `callback_key` keys a store into it).
        let top_union = keys.union.then(|| (format!("{record}.*"), String::new()));
        let mut pending = vec![(String::new(), keys, false, top_union)];
        while let Some((at, keys, unnamed, union)) = pending.pop() {
            for (name, field) in &keys.fields {
                let here = if at.is_empty() {
                    name.clone()
                } else {
                    format!("{at}.{name}")
                };
                let unnamed = unnamed || name.is_empty();
                match field {
                    FieldKey::Function if !unnamed => {
                        let key = match &union {
                            None => format!("{record}.{here}"),
                            Some((key, start)) => {
                                let arm = if start.is_empty() {
                                    here.clone()
                                } else {
                                    here[start.len() + 1..].to_owned()
                                };
                                format!("{key}{ARM}{arm}")
                            }
                        };
                        inner.members.insert(
                            key,
                            Some((
                                probe.want(format!("__builtin_offsetof({ty}, {here})")),
                                probe.want(format!("sizeof((({ty} *)0)->{here})")),
                            )),
                        );
                    }
                    FieldKey::Function => {}
                    FieldKey::Inline { keys: inline } => {
                        let union = union.clone().or_else(|| {
                            inline
                                .union
                                .then(|| (format!("{record}.{here}.*"), here.clone()))
                        });
                        pending.push((here, (**inline).clone(), unnamed, union));
                    }
                    FieldKey::Named { record: held } => {
                        let placement = (!unnamed).then(|| {
                            (
                                probe.want(format!("__builtin_offsetof({ty}, {here})")),
                                probe.want(format!("sizeof((({ty} *)0)->{here})")),
                                self.spelled_type(held)
                                    .map(|ty| probe.want(format!("sizeof({ty})"))),
                            )
                        });
                        inner.held.push((held.clone(), placement));
                        nested.push(held.clone());
                    }
                    FieldKey::Plain => {}
                }
            }
        }
        plan.records.insert(record.to_owned(), Some(inner));
        for held in nested {
            self.plan_record(&held, probe, plan, depth + 1);
        }
    }

    /// The value of the enumerator declared with `id`, as Clang read the
    /// unit (implicit values counted on from the previous enumerator).
    #[must_use]
    pub fn enumerator(&self, id: &str) -> Option<i64> {
        self.enumerators.get(id).copied()
    }

    /// Every enumerator's value by declaration id.
    #[must_use]
    pub fn enumerators(&self) -> &BTreeMap<String, i64> {
        &self.enumerators
    }

    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rules 12m, 13l): the
    /// enumerations by the types that name them, every function's result
    /// type, and every definition's caller-branch reading.
    fn collect_handoff(&mut self, node: &Value) {
        fn enum_type(node: &Value) -> Option<&str> {
            if kind(node) == "EnumType" {
                return node["decl"]["id"].as_str();
            }
            children(node).iter().find_map(enum_type)
        }
        match kind(node) {
            "EnumDecl" => {
                let members = children(node)
                    .iter()
                    .filter(|constant| kind(constant) == "EnumConstantDecl")
                    .filter_map(|constant| {
                        let value = self.enumerators.get(constant["id"].as_str()?)?;
                        Some((constant["name"].as_str()?.to_owned(), *value))
                    })
                    .collect::<Vec<_>>();
                if let Some(tag) = node["name"].as_str().filter(|tag| !tag.is_empty()) {
                    for (name, value) in &members {
                        self.typed_constants
                            .entry(name.clone())
                            .or_default()
                            .insert(format!("enum {tag}"), *value);
                    }
                }
                if let Some(id) = node["id"].as_str() {
                    self.enum_members.insert(id.to_owned(), members);
                }
            }
            "TypedefDecl" => {
                for child in children(node) {
                    if kind(child) == "EnumDecl" {
                        self.collect_handoff(child);
                    }
                }
                if let (Some(name), Some(id), Some(target)) =
                    (node["name"].as_str(), node["id"].as_str(), enum_type(node))
                    && let Some(members) = self.enum_members.get(target).cloned()
                {
                    self.enum_typedefs.insert(id.to_owned());
                    for (constant, value) in members {
                        self.typed_constants
                            .entry(constant)
                            .or_default()
                            .insert(name.to_owned(), value);
                    }
                }
            }
            "FunctionDecl" => {
                let Some(name) = node["name"].as_str() else {
                    return;
                };
                if let Some(result) = node["type"]["qualType"]
                    .as_str()
                    .and_then(|text| text.split_once(" ("))
                    .map(|(result, _)| result.trim())
                {
                    self.result_types
                        .entry(name.to_owned())
                        .or_insert_with(|| result.to_owned());
                }
                if let Some(parameters) = node["type"]["qualType"]
                    .as_str()
                    .and_then(function_type_parameters)
                {
                    self.parameter_types
                        .entry(name.to_owned())
                        .or_insert(parameters);
                }
                if !self.handling.contains_key(name)
                    && let Some(handling) = crate::handoff_branch::function_handling(
                        node,
                        &self.enumerators,
                        &self.enum_typedefs,
                        &|parameter| self.function_pointer(parameter),
                    )
                {
                    self.handling.insert(name.to_owned(), handling);
                }
            }
            _ => {}
        }
    }

    fn collect_enumerators(&mut self, node: &Value) {
        if kind(node) == "EnumDecl" {
            let mut next = Some(0i64);
            for constant in children(node) {
                if kind(constant) != "EnumConstantDecl" {
                    continue;
                }
                let value = match children(constant).first() {
                    Some(init) => crate::arm_release_check::fold_with(init, &self.enumerators),
                    None => next,
                };
                if let (Some(value), Some(id)) = (value, constant["id"].as_str()) {
                    self.enumerators.insert(id.to_owned(), value);
                }
                next = value.and_then(|value| value.checked_add(1));
            }
        }
        for child in children(node) {
            if matches!(kind(child), "EnumDecl" | "TypedefDecl" | "RecordDecl") {
                self.collect_enumerators(child);
            }
        }
    }

    /// Read one top-level declaration.
    pub fn collect(&mut self, node: &Value) {
        self.collect_enumerators(node);
        self.collect_records(node);
        self.collect_handoff(node);
        match kind(node) {
            "TypedefDecl" => {
                let Some(name) = node["name"].as_str() else {
                    return;
                };
                let text = qual_text(node);
                let desugared = type_text(node);
                if spells_function_pointer(text) || spells_function_pointer(desugared) {
                    self.pointer_typedefs.insert(name.to_owned());
                } else if spells_function_type(text) || spells_function_type(desugared) {
                    self.function_typedefs.insert(name.to_owned());
                } else if let Some(record) = spelled_record(strip_qualifiers(desugared))
                    .filter(|_| !desugared.contains('*'))
                    .or_else(|| {
                        let bare = strip_qualifiers(text);
                        (!bare.contains('*'))
                            .then(|| self.record_typedefs.get(bare).cloned())
                            .flatten()
                    })
                {
                    self.record_typedefs.insert(name.to_owned(), record);
                }
            }
            "FunctionDecl" => {
                let Some(name) = node["name"].as_str() else {
                    return;
                };
                let Some(body) = children(node)
                    .iter()
                    .find(|child| kind(child) == "CompoundStmt")
                else {
                    return;
                };
                if node["storageClass"].as_str() == Some("static") {
                    self.internal.insert(name.to_owned());
                }
                let parameters = children(node)
                    .iter()
                    .filter(|child| kind(child) == "ParmVarDecl")
                    .map(|child| child["id"].as_str().unwrap_or(""))
                    .collect();
                let enclosing = Enclosing { name, parameters };
                self.walk(body, Some(&enclosing));
            }
            _ => self.walk(node, None),
        }
    }

    fn walk(&mut self, node: &Value, enclosing: Option<&Enclosing<'_>>) {
        match kind(node) {
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let [left, right] = children(node) else {
                    return;
                };
                let place = strip(left);
                if kind(place) == "MemberExpr" && self.function_pointer(place) {
                    let key = self.callback_key(place);
                    let flows = self.flows(right, enclosing);
                    self.stores.entry(key).or_default().extend(flows);
                } else if anonymous_record(place)
                    && let Some(key) = self.union_place(place)
                {
                    // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: an anonymous
                    // record inside a union (an arm) assigned whole writes
                    // exactly its own bytes. A copy of the same type keeps
                    // every byte where it was (values already counted at
                    // their positions); anything else writes those bytes
                    // opaquely. (It was read as a write of the holder's
                    // whole record.)
                    let source = strip(right);
                    if type_text(source) != type_text(place) || kind(source) == "CallExpr" {
                        self.stores.entry(key).or_default().insert(Flow::Open {
                            why: "a whole-object assignment into the union".to_owned(),
                        });
                    }
                } else if let Some(record) = self.record_of_node(place, false) {
                    // A whole-object assignment copies members of the same
                    // record (values already counted); anything else writes
                    // them opaquely.
                    let source = strip(right);
                    if self.record_of_node(source, false).as_deref() != Some(record.as_str())
                        || kind(source) == "CallExpr"
                    {
                        // Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: keyed by
                        // its target too; a place a union holds is also an
                        // open store there, so its own bytes decide.
                        let target = self.opaque_target(place);
                        if let OpaqueTarget::Union { key } = &target {
                            self.stores
                                .entry(key.clone())
                                .or_default()
                                .insert(Flow::Open {
                                    why: "a whole-object assignment into the union".to_owned(),
                                });
                        }
                        self.open_opaquely(record, "a whole-object assignment".to_owned(), target);
                    }
                }
                self.walk(left, enclosing);
                self.walk(right, enclosing);
            }
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                if let Some(operand) = children(node).first() {
                    let operand = strip(operand);
                    if kind(operand) == "MemberExpr" && self.function_pointer(operand) {
                        let key = self.callback_key(operand);
                        self.stores.entry(key).or_default().insert(Flow::Open {
                            why: "its address is taken".to_owned(),
                        });
                    }
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            "CallExpr" => {
                let arguments = children(node);
                if let Some(callee) = direct_callee(node) {
                    for (index, argument) in arguments.iter().skip(1).enumerate() {
                        if self.function_pointer(argument) || designator(argument).is_some() {
                            let flows = self.flows(argument, enclosing);
                            self.calls
                                .entry(callee.to_owned())
                                .or_default()
                                .entry(index)
                                .or_default()
                                .extend(flows);
                        }
                    }
                    if COPIERS.contains(&callee)
                        && let Some(destination) = arguments.get(1)
                        && let Some(record) = self.record_of_node(strip(destination), true)
                    {
                        // Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: the copy's
                        // destination place, when it names one (`&P`, an
                        // array); a pointer is untargeted. A place a union
                        // holds writes the whole union opaquely (below).
                        let target = Self::addressed(destination)
                            .map_or(OpaqueTarget::Untargeted, |place| self.opaque_target(place));
                        self.open_opaquely(record, format!("a `{callee}` into it"), target);
                    }
                    // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: a copy into
                    // a place a union holds may write past the place: the
                    // whole union is written opaquely.
                    if COPIERS.contains(&callee)
                        && let Some(place) = arguments.get(1).and_then(Self::addressed)
                        && let Some(key) = self.union_place(place)
                    {
                        self.open_union(&key, &format!("a `{callee}` into the union"));
                    }
                    // The callee designator is not an address taken.
                    for (index, argument) in arguments.iter().skip(1).enumerate() {
                        // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: a
                        // filler's destination converted to `void *` writes
                        // one byte value only: no escape (its operand is
                        // still read).
                        if index == 0
                            && FILLERS.contains(&callee)
                            && kind(argument) == "ImplicitCastExpr"
                            && argument["castKind"].as_str() == Some("BitCast")
                        {
                            for child in children(argument) {
                                self.walk(child, enclosing);
                            }
                            continue;
                        }
                        self.walk(argument, enclosing);
                    }
                } else {
                    for child in arguments {
                        self.walk(child, enclosing);
                    }
                }
            }
            // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: the address of a
            // place a union holds converted to another pointer type may be
            // written through as any type (type punning): the whole union is
            // written opaquely.
            "CStyleCastExpr" | "ImplicitCastExpr"
                if node["castKind"].as_str() == Some("BitCast") =>
            {
                let mut operand = children(node).first();
                while let Some(inner) = operand
                    && kind(inner) == "ParenExpr"
                {
                    operand = children(inner).first();
                }
                if let Some(inner) = operand
                    && kind(inner) == "UnaryOperator"
                    && inner["opcode"].as_str() == Some("&")
                    && let Some(place) = children(inner).first()
                    && let Some(key) = self.union_place(place)
                {
                    self.open_union(
                        &key,
                        "an address into the union converted to another pointer type",
                    );
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            "DeclRefExpr" => {
                if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
                    && let Some(name) = node["referencedDecl"]["name"].as_str()
                {
                    self.address_taken.insert(name.to_owned());
                }
            }
            // Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a record defined inside a
            // function may reuse a file-scope tag, so no list of that tag is
            // mapped onto fields.
            "RecordDecl" if enclosing.is_some() => {
                if let Some(name) = node["name"].as_str().filter(|name| !name.is_empty()) {
                    self.record_fields.insert(name.to_owned(), None);
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            "InitListExpr" => {
                // Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a list mapped onto its
                // record's fields stores each function-pointer entry;
                // otherwise the record is open, as before.
                if let Some(record) = self.record_of_node(node, false)
                    && self.initializes_function_pointer(node)
                    && !self.map_initializer(&record, node, enclosing)
                {
                    // Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: an initializer
                    // list is not read for its target: untargeted.
                    self.opened_targets
                        .entry(record.clone())
                        .or_default()
                        .insert(OpaqueTarget::Untargeted);
                    self.opened
                        .entry(record)
                        .or_insert_with(|| "an initializer list".to_owned());
                }
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
            _ => {
                for child in children(node) {
                    self.walk(child, enclosing);
                }
            }
        }
    }
}

/// Card **CALLBACK-MEMBER-DATA-ONLY-UNITS** (rule 12n): one unit's facts
/// read from its whole Clang AST, for a unit the gateway does not otherwise
/// analyse (a data-only unit). `source` is the unit's preprocessed text and
/// `unit` gives its dialect and frontend flags: the input an analysed unit's
/// index is read from. Every top-level declaration is read with
/// [`MemberFacts::collect`], so a unit with no store into a member contributes
/// an empty set read from its AST. `Err` when Clang or the JSON fails, or the
/// document is not a translation unit with a declaration array; the caller
/// then records no facts, which opens every member.
pub fn read_unit(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
) -> Result<MemberFacts, String> {
    let mut facts = crate::typed_c_ast::translation_unit_ast_with(source, unit, |bytes| {
        crate::deep_unit_ast::from_slice::<FactsRoot>(bytes)
    })
    .map(|FactsRoot(facts)| facts)?;
    // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: Clang's layout of the
    // unions the unit's keys name (a failed probe leaves them unplaced).
    facts.probe_layouts(source, unit);
    Ok(facts)
}

/// The translation unit, read for its member facts alone.
struct FactsRoot(MemberFacts);

/// The translation unit's declaration array.
struct FactsDeclarations(MemberFacts);

impl<'de> Deserialize<'de> for FactsDeclarations {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Declarations;
        impl<'de> serde::de::Visitor<'de> for Declarations {
            type Value = FactsDeclarations;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit declaration array")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut facts = MemberFacts::default();
                while let Some(node) = seq.next_element::<Value>()? {
                    facts.collect(&node);
                }
                Ok(FactsDeclarations(facts))
            }
        }
        // Anything but an array is refused (serde's default for the other
        // shapes): a unit whose declarations are not read records no facts.
        deserializer.deserialize_seq(Declarations)
    }
}

impl<'de> Deserialize<'de> for FactsRoot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Root;
        impl<'de> serde::de::Visitor<'de> for Root {
            type Value = FactsRoot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Clang translation-unit AST object")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut facts = None;
                let mut root_kind = Value::Null;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        // A duplicate key replaces every earlier set, as the
                        // analysed unit's index reads it.
                        "inner" => facts = Some(map.next_value::<FactsDeclarations>()?.0),
                        "kind" => root_kind = map.next_value::<Value>()?,
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                if root_kind.as_str() != Some("TranslationUnitDecl") {
                    return Err(serde::de::Error::custom(
                        "expected Clang TranslationUnitDecl",
                    ));
                }
                facts
                    .map(FactsRoot)
                    .ok_or_else(|| serde::de::Error::custom("no declaration array"))
            }
        }
        deserializer.deserialize_map(Root)
    }
}

/// The candidates of a member over a set of units: `Ok` the closed set of
/// designators, `Err` every reason it is open.
pub type Closure = Result<BTreeSet<String>, BTreeSet<String>>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Member(String),
    /// A formal: the unit when the function has internal linkage there.
    Formal(Option<usize>, String, usize),
}

fn record_of_key(member: &str) -> &str {
    member.split_once('.').map_or(member, |(tag, _)| tag)
}

fn last_segment(member: &str) -> &str {
    member.rsplit('.').next().unwrap_or(member)
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (gateway rule 12n, Q3): the
/// record tables of a set of units, every reading of each record tag (two
/// units may define one tag differently; a unit that defined it differently
/// twice gives no reading, so nothing is keyed through it).
#[derive(Clone, Debug, Default)]
pub struct Tables {
    records: BTreeMap<String, Vec<RecordKeys>>,
    ambiguous: BTreeSet<String>,
    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: each union's layout over
    /// the units, `None` where two units disagree on a range (the union is
    /// then unplaced).
    layouts: BTreeMap<String, Option<UnionLayout>>,
}

/// Merge one unit's layout of a union into the run's: a range two units give
/// differently leaves the union unplaced; an unknown entry stays unknown.
fn merge_layout(run: &mut Option<UnionLayout>, unit: &UnionLayout) {
    fn merged(layout: &mut UnionLayout, unit: &UnionLayout) -> bool {
        for (arm, range) in &unit.paths {
            if layout.paths.get(arm).is_some_and(|known| known != range) {
                return false;
            }
            layout.paths.insert(arm.clone(), *range);
        }
        for (target, source) in [
            (&mut layout.records, &unit.records),
            (&mut layout.members, &unit.members),
        ] {
            for (key, ranges) in source {
                if let (Some(Some(known)), Some(ranges)) = (target.get(key), ranges)
                    && known != ranges
                {
                    return false;
                }
                add_ranges(target, key, ranges.clone());
            }
        }
        true
    }
    if let Some(layout) = run
        && !merged(layout, unit)
    {
        *run = None;
    }
}

impl Tables {
    /// The tables of `units`.
    #[must_use]
    pub fn of(units: &[&MemberFacts]) -> Self {
        let mut tables = Self::default();
        for facts in units {
            for (union, layout) in &facts.union_layouts {
                match tables.layouts.get_mut(union) {
                    None => {
                        tables.layouts.insert(union.clone(), Some(layout.clone()));
                    }
                    Some(run) => merge_layout(run, layout),
                }
            }
            for (tag, keys) in &facts.records {
                match keys {
                    Some(keys) => {
                        let readings = tables.records.entry(tag.clone()).or_default();
                        if !readings.contains(keys) {
                            readings.push(keys.clone());
                        }
                    }
                    None => {
                        tables.ambiguous.insert(tag.clone());
                    }
                }
            }
        }
        tables
    }

    fn walk(
        &self,
        tag: &str,
        keys: &RecordKeys,
        prefix: &mut Vec<String>,
        rest: &[&str],
        depth: usize,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err(format!("records nest too deeply below `{tag}`"));
        }
        let spell = |prefix: &[String], last: &str| {
            let mut path = prefix.to_vec();
            path.push(last.to_owned());
            format!("{tag}.{}", path.join("."))
        };
        if keys.union {
            // Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: a declared path into a
            // union keeps its arm path (`T.u.*@arm.cb`), as `callback_key`
            // keys a read of the same place, so the run's layout can place
            // it; the whole union only when nothing follows. (It was keyed
            // as the whole union, aliasing every place the union holds.)
            let union = spell(prefix, "*");
            if rest.is_empty() || rest.iter().any(|segment| segment.is_empty()) {
                out.insert(union);
            } else {
                out.insert(format!("{union}{ARM}{}", rest.join(".")));
            }
            return Ok(());
        }
        let Some((segment, more)) = rest.split_first() else {
            return Err(format!("no member path below `{tag}`"));
        };
        let Some(field) = keys.fields.get(*segment) else {
            return Err(format!("`{segment}` is not a member of `{tag}`"));
        };
        if more.is_empty() {
            return match field {
                FieldKey::Function => {
                    out.insert(spell(prefix, segment));
                    Ok(())
                }
                _ => Err(format!("`{tag}.{segment}` is not a function pointer")),
            };
        }
        match field {
            FieldKey::Named { record } => {
                self.walk_record(record, &mut Vec::new(), more, depth + 1, out)
            }
            FieldKey::Inline { keys } => {
                prefix.push((*segment).to_owned());
                let result = self.walk(tag, keys, prefix, more, depth + 1, out);
                prefix.pop();
                result
            }
            FieldKey::Function | FieldKey::Plain => {
                Err(format!("`{tag}.{segment}` is not a record by value"))
            }
        }
    }

    fn walk_record(
        &self,
        tag: &str,
        prefix: &mut Vec<String>,
        rest: &[&str],
        depth: usize,
        out: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if self.ambiguous.contains(tag) {
            return Err(format!("`{tag}` is defined differently within one unit"));
        }
        let readings = self
            .records
            .get(tag)
            .ok_or_else(|| format!("no unit defines `{tag}`"))?;
        for keys in readings {
            self.walk(tag, keys, prefix, rest, depth, out)?;
        }
        Ok(())
    }

    /// The normal forms of a member key `TAG.PATH`: keyed by its innermost
    /// named record, a union's members as `….*`; several where the units
    /// define a record differently. `Err` when the path is not a
    /// function-pointer member of records the units define.
    ///
    /// # Errors
    /// Why the path is not such a member.
    pub fn normalise(&self, key: &str) -> Result<BTreeSet<String>, String> {
        if is_union_key(key) {
            // Already a union's key (with or without its arm path).
            return Ok(BTreeSet::from([key.to_owned()]));
        }
        let (tag, path) = key
            .split_once('.')
            .ok_or_else(|| format!("`{key}` names no member"))?;
        let segments = path.split('.').collect::<Vec<_>>();
        if segments.last() == Some(&"*") {
            // Already a union's key.
            return Ok(BTreeSet::from([key.to_owned()]));
        }
        let mut out = BTreeSet::new();
        self.walk_record(tag, &mut Vec::new(), &segments, 0, &mut out)?;
        Ok(out)
    }

    /// The record keys at a union's normal key (`T.p.*`), every reading.
    fn union_at(&self, key: &str) -> Vec<RecordKeys> {
        let key = union_parts(key).map_or(key, |(union, _)| union);
        let Some((tag, path)) = key.split_once('.') else {
            return Vec::new();
        };
        let segments = path.split('.').collect::<Vec<_>>();
        let Some((&"*", inner)) = segments.split_last() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for keys in self.records.get(tag).into_iter().flatten() {
            let mut at = Some(keys);
            for segment in inner {
                at = match at.and_then(|keys| keys.fields.get(*segment)) {
                    Some(FieldKey::Inline { keys }) => Some(keys),
                    _ => None,
                };
            }
            if let Some(keys) = at {
                out.push(keys.clone());
            }
        }
        out
    }

    /// Every named record a record's fields hold by value, transitively.
    fn embedded(&self, keys: &RecordKeys) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut pending = vec![keys.clone()];
        while let Some(keys) = pending.pop() {
            for field in keys.fields.values() {
                match field {
                    FieldKey::Named { record } => {
                        if out.insert(record.clone()) {
                            pending.extend(self.records.get(record).into_iter().flatten().cloned());
                        }
                    }
                    FieldKey::Inline { keys } => pending.push((**keys).clone()),
                    FieldKey::Function | FieldKey::Plain => {}
                }
            }
        }
        out
    }

    /// Whether a record holds a function pointer, directly or through
    /// records it holds by value (only such a record can carry a store).
    fn carries_function(&self, record: &str) -> bool {
        self.records
            .get(record)
            .into_iter()
            .flatten()
            .any(|keys| !self.names(keys).is_empty())
    }

    /// Every record that holds `record` by value, transitively.
    fn containers(&self, record: &str) -> BTreeSet<String> {
        self.records
            .iter()
            .filter(|(_, readings)| {
                readings
                    .iter()
                    .any(|keys| self.embedded(keys).contains(record))
            })
            .map(|(tag, _)| tag.clone())
            .collect()
    }

    /// Every function-pointer field name below a record's keys, through
    /// embedded records: the names a store into a record that is not read
    /// (`?.NAME`) may use, since only a function-pointer member is a store.
    fn names(&self, keys: &RecordKeys) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut pending = vec![keys.clone()];
        let mut seen = BTreeSet::new();
        while let Some(keys) = pending.pop() {
            for (name, field) in &keys.fields {
                if *field == FieldKey::Function {
                    out.insert(name.clone());
                }
                match field {
                    FieldKey::Named { record } => {
                        if seen.insert(record.clone()) {
                            pending.extend(self.records.get(record).into_iter().flatten().cloned());
                        }
                    }
                    FieldKey::Inline { keys } => pending.push((**keys).clone()),
                    FieldKey::Function | FieldKey::Plain => {}
                }
            }
        }
        out
    }

    /// Whether a store keyed `store` may write the member keyed `member`
    /// (both normal): the same key; a union's key whose members embed the
    /// member's record; or the member is a union's and the store's record is
    /// embedded in it.
    ///
    /// Card **RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION** (layout route): two
    /// places of one union alias when they are the same arm path, when
    /// either is the whole union or not placed, or when Clang's layout puts
    /// them on overlapping bytes; a union's place and a record the union
    /// holds alias unless the layout puts every placement of the record's
    /// member off the place's bytes. Anything the layout does not give is
    /// read as overlapping.
    fn aliases(&self, member: &str, store: &str) -> bool {
        if member == store {
            return true;
        }
        let (in_member, in_store) = (union_parts(member), union_parts(store));
        if let (Some((union, arm)), Some((other, other_arm))) = (in_member, in_store)
            && union == other
        {
            return places_alias(
                arm,
                other_arm,
                arm.and_then(|arm| self.arm_ranges(union, arm)),
                other_arm.and_then(|arm| self.arm_ranges(union, arm)),
            );
        }
        let embeds = |union: &str, record: &str| {
            self.union_at(union)
                .iter()
                .any(|keys| self.embedded(keys).contains(record))
        };
        (in_store.is_some()
            && embeds(store, record_of_key(member))
            && self.meets_held(store, member))
            || (in_member.is_some()
                && embeds(member, record_of_key(store))
                && self.meets_held(member, store))
    }

    /// Card **UNION-OPAQUE-RECORD-WRITES-BY-TYPE** (rule 12n): whether a
    /// member path from `record` passes through no union, as every reading
    /// of the records on the way gives it. A path the tables do not read (a
    /// record no unit defines, one defined differently within a unit, a
    /// segment that is not a field, a step through a field that is not a
    /// record by value, an anonymous member) is never read as avoiding one.
    fn path_avoids_unions(&self, record: &str, path: &[String]) -> bool {
        fn through(tables: &Tables, keys: &RecordKeys, path: &[String], depth: usize) -> bool {
            if keys.union || depth > 64 {
                return false;
            }
            let Some((segment, rest)) = path.split_first() else {
                return true;
            };
            if segment.is_empty() {
                return false;
            }
            match keys.fields.get(segment) {
                None => false,
                Some(FieldKey::Named { record }) if rest.is_empty() => tables
                    .readings(record)
                    .is_some_and(|readings| readings.iter().all(|keys| !keys.union)),
                Some(FieldKey::Named { record }) => {
                    tables.readings(record).is_some_and(|readings| {
                        readings
                            .iter()
                            .all(|keys| through(tables, keys, rest, depth + 1))
                    })
                }
                Some(FieldKey::Inline { keys }) => through(tables, keys, rest, depth + 1),
                Some(FieldKey::Function | FieldKey::Plain) => false,
            }
        }
        self.readings(record)
            .is_some_and(|readings| readings.iter().all(|keys| through(self, keys, path, 0)))
    }

    /// Every reading of a record the units define unambiguously.
    fn readings(&self, record: &str) -> Option<&Vec<RecordKeys>> {
        if self.ambiguous.contains(record) {
            return None;
        }
        self.records
            .get(record)
            .filter(|readings| !readings.is_empty())
    }

    /// Card **UNION-OPAQUE-RECORD-WRITES-BY-TYPE** (rule 12n): whether an
    /// opaque write of a record the union `union` holds, landing at
    /// `target`, may lie in that union's bytes at an unstated position (so
    /// the record's placements decide, as before). A separate object or a
    /// member path below a record the union does not hold, on a path that
    /// meets no union, lies outside it; a write into this union is an open
    /// store at its own place and decided by its bytes there. Everything
    /// else (an untargeted write, a write into another union, a record the
    /// union holds or a path the tables do not read) may.
    fn may_lie_in(&self, union: &str, target: &OpaqueTarget) -> bool {
        match target {
            OpaqueTarget::Untargeted => opaque_write_may_lie(TargetClass::Untargeted, false, false),
            OpaqueTarget::Union { key } => {
                let class = if union_parts(key).map(|(other, _)| other) == Some(union) {
                    TargetClass::SameUnion
                } else {
                    TargetClass::OtherUnion
                };
                opaque_write_may_lie(class, false, false)
            }
            OpaqueTarget::Separate { record, path } => opaque_write_may_lie(
                TargetClass::Separate {
                    whole: path.is_empty(),
                },
                false,
                record
                    .as_deref()
                    .is_some_and(|record| self.path_avoids_unions(record, path)),
            ),
            OpaqueTarget::Below { record, path } => opaque_write_may_lie(
                TargetClass::Below,
                self.union_at(union)
                    .iter()
                    .any(|keys| self.embedded(keys).contains(record)),
                self.path_avoids_unions(record, path),
            ),
        }
    }

    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: the bytes of a union's arm
    /// path, when the run's layout gives them.
    fn arm_ranges(&self, union: &str, arm: &str) -> Option<BTreeSet<Range>> {
        let range = self.layouts.get(union)?.as_ref()?.paths.get(arm)?;
        Some(BTreeSet::from([*range]))
    }

    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: whether a union's place
    /// (`key`) may meet a function-pointer member (`held`, its normal key) of
    /// a record the union holds.
    fn meets_held(&self, key: &str, held: &str) -> bool {
        let Some((union, Some(arm))) = union_parts(key) else {
            return true;
        };
        let ranges = self
            .layouts
            .get(union)
            .and_then(Option::as_ref)
            .and_then(|layout| layout.members.get(held).cloned().flatten());
        meet(self.arm_ranges(union, arm), ranges)
    }

    /// Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: whether a union's place
    /// (`key`) may meet a record the union holds by value (an opaque write
    /// of that record writes its bytes).
    fn meets_record(&self, key: &str, record: &str) -> bool {
        let Some((union, Some(arm))) = union_parts(key) else {
            return true;
        };
        let ranges = self
            .layouts
            .get(union)
            .and_then(Option::as_ref)
            .and_then(|layout| layout.records.get(record).cloned().flatten());
        meet(self.arm_ranges(union, arm), ranges)
    }
}

/// A node's direct contributions: designators, dependencies, why it is open.
type Contribution = (BTreeSet<String>, BTreeSet<Node>, BTreeSet<String>);

/// The units' stores by normal key, with the tables, built once for many
/// closures (the pre-pass's run).
#[derive(Clone, Debug, Default)]
pub struct Prepared {
    pub tables: Tables,
    /// `(unit, normal key, flows)`.
    stores: Vec<(usize, String, BTreeSet<Flow>)>,
}

impl Prepared {
    /// Index `units`.
    #[must_use]
    pub fn of(units: &[&MemberFacts]) -> Self {
        let tables = Tables::of(units);
        let mut stores = Vec::new();
        for (unit, facts) in units.iter().enumerate() {
            for (key, flows) in &facts.stores {
                let normal = if key.starts_with("?.") {
                    BTreeSet::from([key.clone()])
                } else {
                    tables
                        .normalise(key)
                        .unwrap_or_else(|_| BTreeSet::from([key.clone()]))
                };
                for normal in normal {
                    stores.push((unit, normal, flows.clone()));
                }
            }
        }
        Self { tables, stores }
    }
}

/// The closure of `member` (`TAG.PATH`) over `units`.
pub fn closure(units: &[&MemberFacts], member: &str) -> Closure {
    closure_prepared(units, &Prepared::of(units), member).0
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB**: the tokens the closure of
/// `member` over `units` reads (the record tags and member names of every
/// member node and of the records that hold or alias it, and every function
/// a formal node belongs to); the pre-pass reads every unit naming one.
#[must_use]
pub fn closure_tokens(units: &[&MemberFacts], member: &str) -> BTreeSet<String> {
    closure_prepared(units, &Prepared::of(units), member).1
}

/// [`closure`] over an index built once, with the tokens it read.
pub fn closure_prepared(
    units: &[&MemberFacts],
    prepared: &Prepared,
    member: &str,
) -> (Closure, BTreeSet<String>) {
    let tables = &prepared.tables;
    let mut tokens = BTreeSet::new();
    // The direct contributions of one node: designators, dependencies and
    // reasons it is open.
    let direct = |node: &Node, tokens: &mut BTreeSet<String>| -> Contribution {
        let mut names = BTreeSet::new();
        let mut dependencies = BTreeSet::new();
        let mut open = BTreeSet::new();
        let mut notes = BTreeSet::new();
        let mut take = |flows: &BTreeSet<Flow>, unit: usize| {
            for flow in flows {
                match flow {
                    Flow::Function { name } => {
                        names.insert(name.clone());
                    }
                    Flow::Null => {}
                    Flow::Member { member } => {
                        dependencies.insert(Node::Member(member.clone()));
                    }
                    Flow::Formal { function, index } => {
                        let scope = units[unit].internal.contains(function).then_some(unit);
                        dependencies.insert(Node::Formal(scope, function.clone(), *index));
                    }
                    Flow::Open { why } => {
                        open.insert(why.clone());
                    }
                }
            }
        };
        match node {
            Node::Member(member) => {
                if let Some(tag) = member.split('.').next() {
                    tokens.insert(tag.to_owned());
                }
                // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q3): every normal
                // form, the records that hold it (a whole-object write of one
                // writes it), and the names an unread record's store may use.
                let normalised = tables.normalise(member).ok();
                let targets = normalised
                    .clone()
                    .unwrap_or_else(|| BTreeSet::from([member.clone()]));
                let mut records = BTreeSet::new();
                // The records holding the member's record by value: an
                // opaque write of one (a `memcpy`-family call, an assignment
                // from another type) writes the member; an initializer list
                // of one writes its embedded record only through a nested
                // list or a value of that record, which the record's own key
                // already reads, so it does not open the member.
                let mut containers = BTreeSet::new();
                let mut unread_names = BTreeSet::new();
                // Card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: the records
                // whose opaque writes may write the member's bytes (a record
                // a union holds only where the layout lets it meet them).
                let mut noted = BTreeSet::new();
                // Card UNION-OPAQUE-RECORD-WRITES-BY-TYPE: the records a
                // union target holds whose placements meet it, with the
                // unions concerned: an opaque write of one opens the target
                // only where the write may lie in that union (its target).
                let mut held_noted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
                for target in &targets {
                    tokens.insert(record_of_key(target).to_owned());
                    let record = record_of_key(target).to_owned();
                    containers.extend(tables.containers(&record));
                    noted.insert(record.clone());
                    records.insert(record);
                    if let Some((union, _)) = union_parts(target) {
                        for keys in tables.union_at(target) {
                            for held in tables.embedded(&keys) {
                                if tables.meets_record(target, &held) {
                                    held_noted
                                        .entry(held.clone())
                                        .or_default()
                                        .insert(union.to_owned());
                                }
                                records.insert(held);
                            }
                            unread_names.extend(tables.names(&keys));
                        }
                    } else {
                        unread_names.insert(last_segment(target).to_owned());
                    }
                }
                let aliased =
                    |store: &str| targets.iter().any(|target| tables.aliases(target, store));
                for (_, store, _) in &prepared.stores {
                    if is_union_key(store) && aliased(store) {
                        for keys in tables.union_at(store) {
                            unread_names.extend(tables.names(&keys));
                        }
                    }
                }
                // The tokens a unit must name to hold a store the closure
                // reads: a record that can carry one (a whole-object write of
                // a holder of the member writes it), and a function-pointer
                // name an unread record's store may use.
                tokens.extend(
                    records
                        .iter()
                        .chain(&containers)
                        .filter(|record| tables.carries_function(record))
                        .cloned(),
                );
                // A member keyed by named records is written only by a unit
                // that holds their definitions, which name the tags; the
                // names an unread record's store may use are tokens only
                // where the member is not keyed so.
                if normalised.is_none() {
                    tokens.extend(unread_names.iter().cloned());
                }
                for facts in units {
                    for record in &noted {
                        if let Some(why) = facts.opened.get(record) {
                            notes.insert(format!("{why} writes a `struct {record}`"));
                        }
                    }
                    for (record, unions) in &held_noted {
                        if noted.contains(record) {
                            continue;
                        }
                        let Some(why) = facts.opened.get(record) else {
                            continue;
                        };
                        let lies = match facts.opened_targets.get(record) {
                            // Facts without targets: by type, as before.
                            None => true,
                            Some(writes) => writes.iter().any(|write| {
                                unions.iter().any(|union| tables.may_lie_in(union, write))
                            }),
                        };
                        if lies {
                            notes.insert(format!("{why} writes a `struct {record}`"));
                        }
                    }
                    for record in containers.difference(&noted) {
                        if let Some(why) = facts
                            .opened
                            .get(record)
                            .filter(|why| *why != "an initializer list")
                        {
                            notes.insert(format!("{why} writes a `struct {record}`"));
                        }
                    }
                }
                for (unit, store, flows) in &prepared.stores {
                    if let Some(name) = store.strip_prefix("?.") {
                        if unread_names.contains(name) {
                            notes.insert(format!("a store into `{name}` whose record is not read"));
                            take(flows, *unit);
                        }
                    } else if aliased(store) {
                        take(flows, *unit);
                    }
                }
            }
            Node::Formal(scope, function, index) => {
                tokens.insert(function.clone());
                let reached = |unit: usize| match scope {
                    Some(own) => unit == *own,
                    None => !units[unit].internal.contains(function),
                };
                for (unit, facts) in units.iter().enumerate() {
                    if !reached(unit) {
                        continue;
                    }
                    if facts.address_taken.contains(function) {
                        notes.insert(format!("`{function}`'s address is taken"));
                    }
                    if let Some(flows) = facts.calls.get(function).and_then(|at| at.get(index)) {
                        take(flows, unit);
                    }
                }
            }
        }
        open.extend(notes);
        (names, dependencies, open)
    };
    // Discover the reachable graph, then iterate to the least fixed point.
    let mut graph: BTreeMap<Node, Contribution> = BTreeMap::new();
    let mut pending = vec![Node::Member(member.to_owned())];
    while let Some(node) = pending.pop() {
        if graph.contains_key(&node) {
            continue;
        }
        let entry = direct(&node, &mut tokens);
        pending.extend(entry.1.iter().cloned());
        graph.insert(node, entry);
    }
    let mut values: BTreeMap<Node, (BTreeSet<String>, BTreeSet<String>)> = graph
        .iter()
        .map(|(node, (names, _, open))| (node.clone(), (names.clone(), open.clone())))
        .collect();
    loop {
        let mut changed = false;
        for (node, (_, dependencies, _)) in &graph {
            let mut names = values[node].0.clone();
            let mut open = values[node].1.clone();
            for dependency in dependencies {
                names.extend(values[dependency].0.iter().cloned());
                open.extend(values[dependency].1.iter().cloned());
            }
            if (names.len(), open.len()) != (values[node].0.len(), values[node].1.len()) {
                values.insert(node.clone(), (names, open));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let (names, open) = values
        .remove(&Node::Member(member.to_owned()))
        .unwrap_or_default();
    let result = if open.is_empty() {
        Ok(names)
    } else {
        Err(open)
    };
    (result, tokens)
}

/// `CLOSURE_VECTORS` (`QCore.CallbackContracts.closureVectors`): two units'
/// facts in a tiny spelling and the closure of `T.f`: `Some(names)` closed,
/// `None` open. Each fact is `(unit, key, flow)`: a store into `key`, or, for
/// `call:F:i`, an actual of `F` at `i`; `addr:F` marks `F` address-taken and
/// `static:F` internal.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub(crate) const CLOSURE_VECTORS: &[(&[(usize, &str, &str)], Option<&[&str]>)] = &[
    (&[(0, "T.f", "fn:a"), (1, "T.f", "fn:b")], Some(&["a", "b"])),
    (&[(0, "T.f", "null")], Some(&[])),
    (&[(0, "T.f", "open")], None),
    (
        &[(0, "T.f", "formal:reg:0"), (1, "call:reg:0", "fn:c")],
        Some(&["c"]),
    ),
    (
        &[
            (0, "T.f", "formal:reg:0"),
            (1, "call:reg:0", "fn:c"),
            (1, "addr:reg", ""),
        ],
        None,
    ),
    (
        &[
            (0, "T.f", "formal:reg:0"),
            (0, "static:reg", ""),
            (0, "call:reg:0", "fn:d"),
            (1, "call:reg:0", "fn:e"),
        ],
        Some(&["d"]),
    ),
    (
        &[
            (0, "T.f", "member:U.g"),
            (1, "U.g", "fn:h"),
            (0, "U.g", "member:T.f"),
        ],
        Some(&["h"]),
    ),
    (&[(0, "T.f", "fn:a"), (1, "opened:T", "")], None),
];

#[cfg(test)]
pub(crate) fn vector_units(facts: &[(usize, &str, &str)]) -> Vec<MemberFacts> {
    let mut units = vec![MemberFacts::default(), MemberFacts::default()];
    let flow = |text: &str| -> Flow {
        let parts = text.split(':').collect::<Vec<_>>();
        match parts.as_slice() {
            ["fn", name] => Flow::Function {
                name: (*name).to_owned(),
            },
            ["null"] => Flow::Null,
            ["member", member] => Flow::Member {
                member: (*member).to_owned(),
            },
            ["formal", function, index] => Flow::Formal {
                function: (*function).to_owned(),
                index: index.parse().unwrap(),
            },
            _ => Flow::Open {
                why: "open".to_owned(),
            },
        }
    };
    for (unit, key, value) in facts {
        let facts = &mut units[*unit];
        let parts = key.split(':').collect::<Vec<_>>();
        match parts.as_slice() {
            ["call", function, index] => {
                facts
                    .calls
                    .entry((*function).to_owned())
                    .or_default()
                    .entry(index.parse().unwrap())
                    .or_default()
                    .insert(flow(value));
            }
            ["addr", function] => {
                facts.address_taken.insert((*function).to_owned());
            }
            ["static", function] => {
                facts.internal.insert((*function).to_owned());
            }
            ["opened", record] => {
                facts
                    .opened
                    .insert((*record).to_owned(), "opened".to_owned());
            }
            _ => {
                facts
                    .stores
                    .entry((*key).to_owned())
                    .or_default()
                    .insert(flow(value));
            }
        }
    }
    units
}

/// The parameter types of a Clang function type spelling `R (A, B, ...)`,
/// split at top-level commas (`R (void)` and `R ()` have none). `None` for a
/// spelling not of that shape.
#[must_use]
pub fn function_type_parameters(spelling: &str) -> Option<Vec<String>> {
    let open = spelling.find(" (")? + 1;
    let inner = spelling[open..].strip_prefix('(')?.strip_suffix(')')?;
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in inner.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => {
                parts.push(current.trim().to_owned());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    if parts.len() == 1 && parts[0] == "void" {
        parts.clear();
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `KEY_VECTORS` (`QCore.AppRkCallbacks.keyVectors`): a record table
    /// (`(tag, field, record)`: the field holds that record by value), a key
    /// and its normal form. Every other field is a function pointer; an
    /// anonymous record (`sasl`, `oauthbearer`) is inline.
    #[allow(clippy::type_complexity)]
    const KEY_VECTORS: &[(&[(&str, &str, &str)], &str, &str)] = &[
        (
            &[("rd_kafka_s", "rk_conf", "rd_kafka_conf_s")],
            "rd_kafka_s.rk_conf.dr_msg_cb",
            "rd_kafka_conf_s.dr_msg_cb",
        ),
        (
            &[("rd_kafka_s", "rk_conf", "rd_kafka_conf_s")],
            "rd_kafka_conf_s.dr_msg_cb",
            "rd_kafka_conf_s.dr_msg_cb",
        ),
        (
            &[],
            "rd_kafka_conf_s.sasl.oauthbearer.token_refresh_cb",
            "rd_kafka_conf_s.sasl.oauthbearer.token_refresh_cb",
        ),
        (&[("a", "b", "c"), ("c", "d", "e")], "a.b.d.f", "e.f"),
    ];

    fn vector_tables(rows: &[(&str, &str, &str)], key: &str) -> MemberFacts {
        // Every record the key's path names, each field a function pointer
        // unless a row says it holds a record.
        let mut facts = MemberFacts::default();
        let mut tags = rows
            .iter()
            .flat_map(|(t, _, r)| [*t, *r])
            .collect::<Vec<_>>();
        tags.push(key.split('.').next().unwrap());
        for tag in tags {
            let mut keys = RecordKeys::default();
            for segment in key.split('.').skip(1) {
                keys.fields
                    .entry(segment.to_owned())
                    .or_insert(FieldKey::Function);
            }
            for (t, field, record) in rows {
                if *t == tag {
                    keys.fields.insert(
                        (*field).to_owned(),
                        FieldKey::Named {
                            record: (*record).to_owned(),
                        },
                    );
                }
            }
            facts.records.insert(tag.to_owned(), Some(keys));
        }
        // The anonymous records of the third vector.
        if key.contains(".sasl.") {
            let inner = |name: &str, field: FieldKey| RecordKeys {
                union: false,
                fields: BTreeMap::from([(name.to_owned(), field)]),
            };
            let oauth = inner("token_refresh_cb", FieldKey::Function);
            let sasl = inner(
                "oauthbearer",
                FieldKey::Inline {
                    keys: Box::new(oauth),
                },
            );
            facts.records.insert(
                "rd_kafka_conf_s".into(),
                Some(inner(
                    "sasl",
                    FieldKey::Inline {
                        keys: Box::new(sasl),
                    },
                )),
            );
        }
        facts
    }

    /// A development aid: the tables of one preprocessed unit
    /// (`QCORE_DEBUG_UNIT`), the containers and embedded records of
    /// `QCORE_DEBUG_RECORD`.
    #[test]
    #[ignore = "a development aid over a local preprocessed unit"]
    fn debug_tables() {
        let path = std::env::var("QCORE_DEBUG_UNIT").expect("QCORE_DEBUG_UNIT");
        let record = std::env::var("QCORE_DEBUG_RECORD").expect("QCORE_DEBUG_RECORD");
        let text = std::fs::read_to_string(path).unwrap();
        let facts = read_unit(
            &text,
            &crate::legacy_c::C11TranslationUnit {
                source: &text,
                dialect: crate::legacy_c::CCompilerDialect::Gnu11,
                ..Default::default()
            },
        )
        .unwrap();
        let tables = Tables::of(&[&facts]);
        println!("containers {:?}", tables.containers(&record));
        for keys in tables.records.get(&record).into_iter().flatten() {
            println!("embedded {:?}", tables.embedded(keys));
        }
        for (tag, readings) in &tables.records {
            for keys in readings {
                if tables.embedded(keys).contains(&record) {
                    let direct = keys
                        .fields
                        .iter()
                        .filter(|(_, field)| {
                            matches!(field, FieldKey::Named { .. } | FieldKey::Inline { .. })
                        })
                        .collect::<Vec<_>>();
                    println!("{tag} embeds via {direct:?}");
                }
            }
        }
    }

    #[test]
    fn key_vectors_hold() {
        for (rows, key, normal) in KEY_VECTORS {
            let facts = vector_tables(rows, key);
            assert_eq!(
                Tables::of(&[&facts]).normalise(key),
                Ok(BTreeSet::from([(*normal).to_owned()])),
                "{key}"
            );
        }
        assert_eq!(KEY_VECTORS.len(), 4);
    }

    #[test]
    fn layout_vectors_hold() {
        for (arm, other, ranges, other_ranges, expected) in LAYOUT_VECTORS {
            let set =
                |ranges: &Option<&[Range]>| ranges.map(|ranges| ranges.iter().copied().collect());
            assert_eq!(
                places_alias(*arm, *other, set(ranges), set(other_ranges)),
                *expected,
                "{arm:?} {other:?} {ranges:?} {other_ranges:?}"
            );
            // Symmetric.
            assert_eq!(
                places_alias(*other, *arm, set(other_ranges), set(ranges)),
                *expected
            );
        }
        assert_eq!(LAYOUT_VECTORS.len(), 7);
        assert_eq!(
            union_parts("op_s.u.*@late.cb"),
            Some(("op_s.u.*", Some("late.cb")))
        );
        assert_eq!(union_parts("slot_u.*"), Some(("slot_u.*", None)));
        assert_eq!(union_parts("conf_s.log_cb"), None);
        assert_eq!(union_parts("odd@key"), None);
    }

    #[test]
    fn opaque_vectors_hold() {
        for (class, root_held, path_clear, expected) in OPAQUE_VECTORS {
            assert_eq!(
                opaque_write_may_lie(*class, *root_held, *path_clear),
                *expected,
                "{class:?} {root_held} {path_clear}"
            );
        }
        assert_eq!(OPAQUE_VECTORS.len(), 9);
    }

    #[test]
    fn closure_vectors_hold() {
        for (facts, expected) in CLOSURE_VECTORS {
            let units = vector_units(facts);
            let views = units.iter().collect::<Vec<_>>();
            let got = closure(&views, "T.f").ok();
            let expected = expected.map(|names| {
                names
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect::<BTreeSet<_>>()
            });
            assert_eq!(got, expected, "{facts:?}");
        }
        assert_eq!(CLOSURE_VECTORS.len(), 8);
    }

    #[test]
    fn a_unit_sees_a_subset_and_the_run_may_open_it() {
        // Unit 0 alone closes over `a`; the run adds unit 1's `b`.
        let units = vector_units(&[(0, "T.f", "fn:a"), (1, "T.f", "fn:b")]);
        assert_eq!(
            closure(&[&units[0]], "T.f"),
            Ok(BTreeSet::from(["a".to_owned()]))
        );
        let run = closure(&[&units[0], &units[1]], "T.f").unwrap();
        assert!(!run.is_subset(&BTreeSet::from(["a".to_owned()])));
    }

    fn ast(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn facts_read_member_stores_formals_and_calls() {
        let mut facts = MemberFacts::default();
        facts.collect(&ast(
            r#"{"kind":"TypedefDecl","name":"cb_t","type":{"qualType":"int (void *)"}}"#,
        ));
        facts.collect(&ast(
            r#"{"kind":"TypedefDecl","name":"op_t","type":{"desugaredQualType":"struct op_s","qualType":"struct op_s"}}"#,
        ));
        // op_t *new_cb(cb_t *cb) { rko->op_cb = cb; }
        facts.collect(&ast(
            r#"{"kind":"FunctionDecl","name":"new_cb","inner":[
                {"kind":"ParmVarDecl","id":"0x1","name":"cb","type":{"qualType":"cb_t *"}},
                {"kind":"CompoundStmt","inner":[
                  {"kind":"BinaryOperator","opcode":"=","type":{"qualType":"cb_t *"},"inner":[
                    {"kind":"MemberExpr","name":"op_cb","isArrow":true,"type":{"qualType":"cb_t *"},"inner":[
                      {"kind":"ImplicitCastExpr","castKind":"LValueToRValue","type":{"qualType":"op_t *"},"inner":[
                        {"kind":"DeclRefExpr","type":{"qualType":"op_t *"},"referencedDecl":{"kind":"VarDecl","id":"0x9","name":"rko"}}]}]},
                    {"kind":"ImplicitCastExpr","castKind":"LValueToRValue","type":{"qualType":"cb_t *"},"inner":[
                      {"kind":"DeclRefExpr","type":{"qualType":"cb_t *"},"referencedDecl":{"kind":"ParmVarDecl","id":"0x1","name":"cb"}}]}]},
                  {"kind":"CallExpr","type":{"qualType":"op_t *"},"inner":[
                    {"kind":"ImplicitCastExpr","castKind":"FunctionToPointerDecay","inner":[
                      {"kind":"DeclRefExpr","referencedDecl":{"kind":"FunctionDecl","name":"new_cb"}}]},
                    {"kind":"ImplicitCastExpr","castKind":"FunctionToPointerDecay","type":{"qualType":"int (*)(void *)"},"inner":[
                      {"kind":"DeclRefExpr","referencedDecl":{"kind":"FunctionDecl","name":"handler"}}]}]}]}]}"#,
        ));
        assert_eq!(
            facts.stores["op_s.op_cb"],
            BTreeSet::from([Flow::Formal {
                function: "new_cb".into(),
                index: 0
            }])
        );
        assert_eq!(
            facts.calls["new_cb"][&0],
            BTreeSet::from([Flow::Function {
                name: "handler".into()
            }])
        );
        assert!(facts.address_taken.contains("handler"));
        assert!(!facts.address_taken.contains("new_cb"));
        assert_eq!(
            closure(&[&facts], "op_s.op_cb"),
            Ok(BTreeSet::from(["handler".to_owned()]))
        );
    }
}
