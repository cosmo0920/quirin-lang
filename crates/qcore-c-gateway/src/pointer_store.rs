//! Card **CG11-POINTER-VALUED-STORES** (gateway rule 10v): a pointer-valued
//! store `P = V` into a typed place, read from the Clang AST, admitted only
//! where no custody moves.
//!
//! ## What was refused
//!
//! W46's typed statement reader stores only scalars (its `path` requires a
//! scalar place), CG10's typed place keeps the pointer-valued store refused by
//! name, and W45's sub-object store reads a lexically resolved single member of
//! a record whose every member its reader resolved. So `s->in.p = NULL`,
//! `(&cc->cc_entries)->tqh_last = &(&cc->cc_entries)->tqh_first` (the
//! `TAILQ_INIT` expansion) and `s->cb = f` kept the gate's refusal.
//!
//! ## Four events, kept apart
//!
//! 1. **The value store** writes the place. Its per-hop obligations are the
//!    place's own: [`crate::typed_access_path::Reader::path`] reads the
//!    destination exactly as it reads a scalar store's (W46's never-bound root,
//!    or CG10's tracked-root chain), and the flow checks them unchanged.
//! 2. **Alias addition and overwrite.** Every binding of a place rooted at the
//!    destination's root is forgotten after the store: its old fact no longer
//!    describes the object. An admitted value adds no alias the projection
//!    tracks.
//! 3. **The old value's obligation.** The store is admitted only when the
//!    destination's owner, the object the member lives in, is *untracked*
//!    ([`crate::indirect_chain_access::chain_owner`]: no reaching path bound
//!    it, so it holds no field link), and no binding of a place rooted at the
//!    destination carries a target or is unknown. A tracked owner may hold an
//!    owning field: its store refuses, because only O04-B08's field-link store
//!    rewrites one. The old value of a slot in an untracked object is not this
//!    body's obligation: premise **P-CG11-CALLER-SLOT**, stated, not proved.
//! 4. **Escape into the owner.** Only values that carry no identity are
//!    admitted ([`ValueClass::neutral`]): the null constant, a string literal,
//!    a function designator, a name of function-pointer type the flow proves
//!    bound to no identity, and the address of the destination's own object
//!    (the root itself or a place in it read through no pointer but the
//!    root's). Nothing escapes, so no custody moves.
//!
//! Everything else refuses, and the refusal is the gate's own, byte for byte:
//! a formal's object pointer (example C, `s->p = q`: no contract or tracked
//! state gives `*s`'s owner or bounds `*q`'s lifetime by it, and no summary
//! mode states the transfer), a local's pointer (it may carry an identity),
//! a pointer read, a call, arithmetic, another object's address, a cast, and
//! every destination CG10's place grammar does not list.
//!
//! ## Summaries
//!
//! A body with an admitted pointer store publishes no derived summary and
//! establishes no specification ([`has_pointer_store`], read by the W4 gate):
//! a caller would read `Borrowed` for the formal whose object the body
//! overwrote. While a summary is derived, a formal is bound to its caller's
//! site, so its object is a tracked owner and the store refuses anyway; the
//! gate states the decision rather than relying on that.
//!
//! The Lean model is `QCore.PointerStore`; its decision vectors are this
//! module's [`DECISION_VECTORS`].

use std::cell::RefCell;
use std::collections::BTreeMap;

use serde_json::Value;

use crate::indirect_chain_access::AccessChain;
use crate::typed_access_path::{FieldFacts, Reader, is_pointer_type, qual};
use crate::typed_c_ast::{children, kind};
use crate::typed_place::{Decision as PlaceDecision, RootKind, TypedPlace, decide as decide_place};

/// The stored value's class, read from the Clang AST only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueClass {
    /// A null pointer constant.
    Null,
    /// A string literal (static storage duration).
    StringLiteral,
    /// A function designator or its address.
    Function,
    /// A parameter or variable of function-pointer type; the flow must find
    /// it bound to no identity.
    FunctionPointerName,
    /// The destination's root pointer itself (`s->self = s`).
    SelfAddress,
    /// The address of a place in the destination's own object.
    InteriorAddress,
    /// A formal's object pointer.
    FormalPointer,
    /// Any other named object pointer.
    LocalPointer,
    /// A pointer value read from memory.
    PointerRead,
    /// A call.
    Call,
    /// Pointer arithmetic.
    Arithmetic,
    /// The address of another object.
    OtherAddress,
    /// Anything else (a cast, a conditional, a compound literal, …).
    Other,
}

impl ValueClass {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::StringLiteral => "string",
            Self::Function => "function",
            Self::FunctionPointerName => "function-pointer-name",
            Self::SelfAddress => "self-address",
            Self::InteriorAddress => "interior-address",
            Self::FormalPointer => "formal-pointer",
            Self::LocalPointer => "local-pointer",
            Self::PointerRead => "pointer-read",
            Self::Call => "call",
            Self::Arithmetic => "arithmetic",
            Self::OtherAddress => "other-address",
            Self::Other => "other",
        }
    }

    /// The classes whose value denotes no object with custody.
    pub(crate) fn neutral(self) -> bool {
        matches!(
            self,
            Self::Null
                | Self::StringLiteral
                | Self::Function
                | Self::FunctionPointerName
                | Self::SelfAddress
                | Self::InteriorAddress
        )
    }
}

/// The destination's owner, as the flow reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnerClass {
    Untracked,
    Tracked,
    Unresolved,
}

impl OwnerClass {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Untracked => "untracked",
            Self::Tracked => "tracked",
            Self::Unresolved => "unresolved",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    /// A formal's object pointer: the owner premise is missing (example C).
    FormalValue,
    /// A named value that is or may be an identity.
    NamedValue,
    /// A value this rule does not read as custody-neutral.
    ValueShape,
    /// The owner is one tracked identity: the member may own its old value.
    OwnedField,
    /// The destination's chain obligation does not resolve.
    UnresolvedOwner,
}

impl Decision {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::FormalValue => "formal-value",
            Self::NamedValue => "named-value",
            Self::ValueShape => "value-shape",
            Self::OwnedField => "owned-field",
            Self::UnresolvedOwner => "unresolved-owner",
        }
    }
}

/// The decision: a pure function of the value's class, whether the flow
/// found an identity in a named value, and the owner. `QCore.PointerStore.decideStore`
/// is the same function.
pub(crate) fn decide(value: ValueClass, carries: bool, owner: OwnerClass) -> Decision {
    match value {
        ValueClass::FormalPointer => Decision::FormalValue,
        ValueClass::LocalPointer => Decision::NamedValue,
        value if !value.neutral() => Decision::ValueShape,
        _ if carries => Decision::NamedValue,
        _ => match owner {
            OwnerClass::Untracked => Decision::Admit,
            OwnerClass::Tracked => Decision::OwnedField,
            OwnerClass::Unresolved => Decision::UnresolvedOwner,
        },
    }
}

/// The decision vectors shared with `proof/QCore/PointerStore.lean`
/// (`decisionVectors`): a statement spelling, its value class, whether the
/// named value carries an identity, the owner and the decision. The Rust tests
/// bind each spelling to Clang's reading and to the route's outcome.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str, bool, &str, &str)] = &[
    ("s->p = 0", "null", false, "untracked", "admit"),
    ("s->in.p = 0", "null", false, "untracked", "admit"),
    ("s->name = \"x\"", "string", false, "untracked", "admit"),
    ("s->cb = f", "function", false, "untracked", "admit"),
    ("s->cb = &f", "function", false, "untracked", "admit"),
    (
        "s->cb = g",
        "function-pointer-name",
        false,
        "untracked",
        "admit",
    ),
    ("s->self = s", "self-address", false, "untracked", "admit"),
    (
        "s->pp = &s->p",
        "interior-address",
        false,
        "untracked",
        "admit",
    ),
    (
        "s->p = q",
        "formal-pointer",
        false,
        "untracked",
        "formal-value",
    ),
    (
        "s->p = l",
        "local-pointer",
        false,
        "untracked",
        "named-value",
    ),
    (
        "s->p = t->p",
        "pointer-read",
        false,
        "untracked",
        "value-shape",
    ),
    ("s->p = cg_get()", "call", false, "untracked", "value-shape"),
    (
        "s->name = s->name + 1",
        "arithmetic",
        false,
        "untracked",
        "value-shape",
    ),
    (
        "s->pp = &t->p",
        "other-address",
        false,
        "untracked",
        "value-shape",
    ),
    ("o->p = 0", "null", false, "tracked", "owned-field"),
    (
        "o->self = o",
        "self-address",
        false,
        "tracked",
        "owned-field",
    ),
    ("d->p = 0", "null", false, "unresolved", "unresolved-owner"),
];

/// The admitted store the flow decides ([`LegacyInstruction::PointerStore`]).
///
/// [`LegacyInstruction::PointerStore`]: crate::legacy_c::LegacyInstruction::PointerStore
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointerStore {
    /// The destination's root name.
    pub root: String,
    /// The destination, spelled from its root (diagnostics and parity).
    pub place: String,
    /// The destination's chain obligation; `None` for a member of a
    /// never-bound record object of this frame selected with `.` only.
    pub owner: Option<AccessChain>,
    /// The value's (neutral) class.
    pub value: &'static str,
    /// The named value of a function-pointer name, decided by the flow.
    pub name: Option<String>,
}

/// Whether `node` (a left-hand side) is a place of pointer type this rule
/// reads: a member, subscript or indirection whose type is an object or
/// function pointer.
pub(crate) fn is_pointer_place(node: &Value) -> bool {
    let Some(bare) = crate::typed_access_path::strip(node) else {
        return false;
    };
    let access = matches!(kind(bare), "MemberExpr" | "ArraySubscriptExpr")
        || (kind(bare) == "UnaryOperator" && bare["opcode"].as_str() == Some("*"));
    access && qual(bare).is_some_and(pointer_type)
}

/// An object pointer (W46's positive test) or a function pointer
/// `R (*)(A…)`, neither qualified `volatile`/`_Atomic` nor a union's.
pub(crate) fn pointer_type(text: &str) -> bool {
    is_pointer_type(text) || function_pointer_type(text)
}

fn function_pointer_type(text: &str) -> bool {
    let text = text.trim();
    text.contains("(*)(")
        && text.ends_with(')')
        && !text.contains("volatile")
        && !text.contains("_Atomic")
        && !text.contains("union ")
}

/// Parentheses and the listed implicit conversions only.
fn peel<'a>(mut node: &'a Value, casts: &[&str]) -> Option<(&'a Value, Vec<&'a str>)> {
    let mut seen = Vec::new();
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr" => {
                let cast = node["castKind"].as_str()?;
                if !casts.contains(&cast) {
                    return Some((node, seen));
                }
                seen.push(cast);
            }
            _ => return Some((node, seen)),
        }
        node = children(node).first()?;
    }
}

/// The value's class and, for an address, the place node whose address is
/// taken, or for a named value its name.
pub(crate) struct ValueReading<'a> {
    pub(crate) class: ValueClass,
    /// A named value's spelling and declaration id.
    pub(crate) name: Option<(String, String)>,
    pub(crate) address: Option<&'a Value>,
}

impl<'a> ValueReading<'a> {
    fn of(class: ValueClass) -> Self {
        Self {
            class,
            name: None,
            address: None,
        }
    }
}

/// Classify the stored value `value` for the destination place.
pub(crate) fn classify_value<'a>(
    value: &'a Value,
    destination: &TypedPlace,
    fields: &BTreeMap<String, Option<FieldFacts>>,
) -> ValueReading<'a> {
    // The null pointer constant: an integer zero (or `__null`) converted to a
    // pointer, through parentheses, `(void *)` casts and qualification.
    if let Some((inner, casts)) = peel(value, &["NullToPointer", "BitCast", "NoOp"])
        && ((kind(inner) == "IntegerLiteral" && inner["value"].as_str() == Some("0"))
            || kind(inner) == "GNUNullExpr")
        && casts.contains(&"NullToPointer")
    {
        return ValueReading::of(ValueClass::Null);
    }
    // A string literal decays to its first character's address.
    if let Some((inner, casts)) = peel(value, &["ArrayToPointerDecay", "NoOp", "BitCast"])
        && kind(inner) == "StringLiteral"
        && casts.contains(&"ArrayToPointerDecay")
    {
        return ValueReading::of(ValueClass::StringLiteral);
    }
    let Some((bare, casts)) = peel(value, &["FunctionToPointerDecay", "LValueToRValue", "NoOp"])
    else {
        return ValueReading::of(ValueClass::Other);
    };
    match kind(bare) {
        "DeclRefExpr" => {
            let decl = &bare["referencedDecl"];
            match decl["kind"].as_str() {
                Some("FunctionDecl") if casts.contains(&"FunctionToPointerDecay") => {
                    ValueReading::of(ValueClass::Function)
                }
                Some("ParmVarDecl" | "VarDecl") if casts.contains(&"LValueToRValue") => {
                    let Some(ty) = qual(bare) else {
                        return ValueReading::of(ValueClass::Other);
                    };
                    let name = decl["name"]
                        .as_str()
                        .zip(decl["id"].as_str())
                        .map(|(name, id)| (name.to_owned(), id.to_owned()));
                    if function_pointer_type(ty) {
                        ValueReading {
                            class: ValueClass::FunctionPointerName,
                            name,
                            address: None,
                        }
                    } else if !is_pointer_type(ty) {
                        ValueReading::of(ValueClass::Other)
                    } else if decl["id"].as_str() == Some(destination.decl.as_str())
                        && destination.root == RootKind::Pointer
                        && root_only(destination)
                    {
                        ValueReading::of(ValueClass::SelfAddress)
                    } else if decl["kind"].as_str() == Some("ParmVarDecl") {
                        ValueReading::of(ValueClass::FormalPointer)
                    } else {
                        ValueReading::of(ValueClass::LocalPointer)
                    }
                }
                _ => ValueReading::of(ValueClass::Other),
            }
        }
        "UnaryOperator" if bare["opcode"].as_str() == Some("&") && casts.is_empty() => {
            let Some(operand) = children(bare).first() else {
                return ValueReading::of(ValueClass::Other);
            };
            if let Some((target, _)) = peel(operand, &[])
                && kind(target) == "DeclRefExpr"
            {
                let decl = &target["referencedDecl"];
                if decl["kind"].as_str() == Some("FunctionDecl") {
                    return ValueReading::of(ValueClass::Function);
                }
                // `head.next = &head`: the record root's own address.
                if decl["id"].as_str() == Some(destination.decl.as_str())
                    && destination.root == RootKind::Record
                    && root_only(destination)
                {
                    return ValueReading::of(ValueClass::SelfAddress);
                }
                return ValueReading::of(ValueClass::OtherAddress);
            }
            match crate::typed_place::read_place(operand, fields) {
                Ok(place) if same_object(destination, &place) => ValueReading {
                    class: ValueClass::InteriorAddress,
                    name: None,
                    address: Some(operand),
                },
                _ => ValueReading::of(ValueClass::OtherAddress),
            }
        }
        "MemberExpr" | "ArraySubscriptExpr" => ValueReading::of(ValueClass::PointerRead),
        "UnaryOperator" if bare["opcode"].as_str() == Some("*") => {
            ValueReading::of(ValueClass::PointerRead)
        }
        "CallExpr" => ValueReading::of(ValueClass::Call),
        "BinaryOperator" if matches!(bare["opcode"].as_str(), Some("+" | "-")) => {
            ValueReading::of(ValueClass::Arithmetic)
        }
        _ => ValueReading::of(ValueClass::Other),
    }
}

/// A place reads no pointer but its root's: a pointer root whose only hop is
/// the root (CG10's `root-only`), or a record root with no hop at all.
fn root_only(place: &TypedPlace) -> bool {
    let kinds = place.steps.iter().map(|step| step.kind).collect::<Vec<_>>();
    match place.root {
        RootKind::Pointer => decide_place(place.root, &kinds) == PlaceDecision::RootOnly,
        RootKind::Record => kinds.iter().all(|step| !step.hop()),
        RootKind::Array => false,
    }
}

/// Both places lie in one object: the same root declaration, and neither
/// reads a pointer past the root.
fn same_object(destination: &TypedPlace, value: &TypedPlace) -> bool {
    destination.decl == value.decl
        && destination.root == value.root
        && root_only(destination)
        && root_only(value)
}

/// The name the flow keys a value by denotes one object in `function`: every
/// declaration of `name` there is the one with `id`, and every reference to
/// the spelling refers to it.
fn unique_name(function: &Value, name: &str, id: &str) -> bool {
    let mut unique = true;
    visit(function, &mut |node| match kind(node) {
        "ParmVarDecl" | "VarDecl" if node["name"].as_str() == Some(name) => {
            unique &= node["id"].as_str() == Some(id);
        }
        "DeclRefExpr" if node["referencedDecl"]["name"].as_str() == Some(name) => {
            unique &= node["referencedDecl"]["id"].as_str() == Some(id);
        }
        _ => {}
    });
    unique
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// The spelling of a place from its root (`s->in.p`, `head.next`, `s[]`).
fn spell(place: &TypedPlace) -> String {
    let mut spelled = place.name.clone();
    for step in &place.steps {
        match step.kind {
            crate::typed_place::StepKind::Arrow => {
                spelled.push_str("->");
                spelled.push_str(&step.member);
            }
            crate::typed_place::StepKind::Dot => {
                spelled.push('.');
                spelled.push_str(&step.member);
            }
            crate::typed_place::StepKind::Index { .. } => spelled.push_str("[]"),
            crate::typed_place::StepKind::Deref { .. } => spelled.insert(0, '*'),
        }
    }
    spelled
}

/// The reader hook (rule 10v): `place = value` where `place` has pointer type.
/// `Some(())` when the store is read as this rule's, with the destination's
/// obligations (and an address value's) added to `reader`'s plan; `None`
/// keeps the refusal.
pub(crate) fn read_store(reader: &mut Reader<'_>, place: &Value, value: &Value) -> Option<()> {
    let destination = match crate::typed_place::read_place(place, reader.fields()) {
        Ok(destination) => destination,
        Err(_) => {
            record(None, None, false);
            return None;
        }
    };
    let reading = classify_value(value, &destination, reader.fields());
    let admitted = reading.class.neutral()
        && reading
            .name
            .as_ref()
            .is_none_or(|(name, id)| reader.function().is_some_and(|f| unique_name(f, name, id)));
    record(Some(&destination), Some(reading.class), admitted);
    if !admitted {
        return None;
    }
    let before = reader.plan.chains.len();
    reader.pointer_store_target = true;
    reader.path(place, true, false)?;
    let owner = match reader.plan.chains.len() - before {
        0 => None,
        1 => reader.plan.chains.last().cloned(),
        _ => return None,
    };
    if owner.is_none() && destination.root != RootKind::Record {
        // A pointer root always yields its chain; anything else is not a
        // form this rule decides.
        return None;
    }
    if let Some(address) = reading.address {
        reader.address_target = true;
        reader.path(address, false, false)?;
    }
    reader.plan.pointer_store = Some(PointerStore {
        root: destination.name.clone(),
        place: spell(&destination),
        owner,
        value: reading.class.code(),
        name: reading.name.map(|(name, _)| name),
    });
    Some(())
}

/// Whether `cfg` holds an admitted pointer store. Such a body publishes no
/// summary and establishes no specification (see the module header).
pub(crate) fn has_pointer_store(cfg: &crate::legacy_c::LegacyCfg) -> bool {
    cfg.nodes.iter().any(|node| {
        matches!(
            node.instruction,
            crate::legacy_c::LegacyInstruction::PointerStore { .. }
        )
    })
}

thread_local! {
    /// Parity only: the classifications made since the last record.
    static DECISIONS: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
}

fn record(place: Option<&TypedPlace>, value: Option<ValueClass>, admitted: bool) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    let row = serde_json::json!({
        "place": place.map(spell),
        "root": place.map(|place| place.root.code()),
        "value": value.map(ValueClass::code),
        "admitted": admitted,
    });
    DECISIONS.with(|decisions| decisions.borrow_mut().push(row));
}

/// Parity only: one `pointer-store` row for the classifications the last
/// statement consultation made.
pub(crate) fn record_decisions(function: &str, line: usize, texts: &[&str]) {
    let decisions = DECISIONS.with(|decisions| std::mem::take(&mut *decisions.borrow_mut()));
    if decisions.is_empty() || !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "function": function,
        "line": line,
        "texts": texts,
        "position": "pointer-store",
        "classifications": decisions,
    }));
}

/// Parity only: the flow's decision on one admitted classification.
pub(crate) fn record_flow(
    function: &str,
    line: usize,
    store: &PointerStore,
    owner: OwnerClass,
    decision: Decision,
) {
    if !crate::typed_access_path::parity_enabled() {
        return;
    }
    crate::typed_access_path::record_row(serde_json::json!({
        "function": function,
        "line": line,
        "position": "pointer-store-flow",
        "place": store.place,
        "value": store.value,
        "owner": owner.code(),
        "decision": decision.code(),
    }));
}

#[cfg(test)]
#[path = "../tests/unit/pointer_store.rs"]
mod tests;
