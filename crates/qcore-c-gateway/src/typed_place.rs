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

//! Card **CG10-COMMON-TYPED-PLACE** (gateway rule 10t): one common typed
//! place, read from the Clang AST, and the per-hop obligations of a mixed
//! `->` / `.` / `[]` / `*` path on a **tracked** root.
//!
//! ## The place
//!
//! A [`TypedPlace`] is a root identity followed by steps, each with Clang's
//! resolved type:
//!
//! * the **root** is one `DeclRefExpr` naming a `ParmVarDecl` or `VarDecl`,
//!   identified by its Clang declaration id (DeclID). A call result, a
//!   compound literal, a cast or any other expression is not a root: the
//!   place is refused;
//! * a **step** is a member selection (`->F` or `.F`, identified by its
//!   `FieldDecl` id), a subscript (`[I]`) or an indirection (`*`). A
//!   subscript or indirection records whether its base is a pointer value
//!   read from memory (`LValueToRValue`, a *hop*) or an array decaying to its
//!   first element's address (`ArrayToPointerDecay`, no hop). An arrow is
//!   always a hop.
//!
//! Anything Clang does not give exactly (a missing type, field id or cast
//! kind, a union or bit-field member, a `volatile` or `_Atomic` base, a cast
//! other than those listed, pointer arithmetic, a call) is refused and never
//! guessed ([`Refusal`]).
//!
//! ## The decision on a tracked root ([`decide`])
//!
//! Before this card a mixed path was read only when the custody closure
//! proved its root never bound (card W46-ACCESS-PATH-STATEMENTS). This card
//! admits the corresponding path on any other pointer root, with the
//! obligations the existing chain model already checks
//! ([`crate::indirect_chain_access::chain_resolves`]):
//!
//! * **root only** — the one pointer read through is the root
//!   (`R->in.x`, `R->arr[i]`, `R[i].x`, `(*R).in.x`): the chain has no link,
//!   so the root must be exactly one known, non-null, live identity, the
//!   obligation `R->F` already carries;
//! * **direct member** — the pointers read through are the root and the
//!   value of the direct member place `R->F` (`R->items[i].v`, `*R->p`,
//!   `R->F->in.x`): `F` is the chain's first link, so `R->F` must carry the
//!   admitted store's evidence exactly as `R->F->G` does. A later hop is a
//!   further link: it needs an untracked owner;
//! * anything else is refused: a pointer loaded from a place deeper than
//!   `R->F` (`R->in.p->x`, `R->slots[i]->x`) or from an element or pointee of
//!   the root (`R[i]->x`, `(*R)->x`) has no evidence in the projection, and
//!   a record-object or array root keeps the never-bound premise.
//!
//! A read or a store of a non-pointer value changes no custody, status,
//! marker or field link; a store through a subscript or indirection keeps
//! W43's refused-write frame. A pointer value read through the place is
//! P2's target-free borrowed alias with no custody. A pointer-valued store
//! stays refused (card CG11).
//!
//! ## Identity (I04)
//!
//! [`Occurrence`] tells two candidates with the same tokens on one line
//! apart: the gateway's own token run is the k-th occurrence of its text on
//! its line, and when the line's tokens equal the analysed source's, every
//! candidate lies on that line, the candidates begin at distinct offsets and
//! their number equals the number of occurrences, the k-th candidate in
//! source order is the node. Otherwise the old unique-match refusal stands.
//!
//! The Lean model is `QCore.TypedPlace`; its decision vectors are this
//! module's [`DECISION_VECTORS`].

use std::collections::BTreeMap;

use serde_json::Value;

use crate::indirect_chain_access::AccessChain;
use crate::legacy_c::Token;
use crate::typed_access_path::{FieldFacts, qual};
use crate::typed_c_ast::{children, kind};

/// The root's kind, read from the step that consumes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootKind {
    /// A pointer value: the first step reads through it.
    Pointer,
    /// A record object selected with `.`.
    Record,
    /// An array decaying to its first element.
    Array,
}

/// One step's form. `through_pointer` is a hop: a pointer value read from
/// memory and dereferenced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepKind {
    Arrow,
    Dot,
    Index { through_pointer: bool },
    Deref { through_pointer: bool },
}

impl StepKind {
    pub(crate) fn hop(self) -> bool {
        match self {
            Self::Arrow => true,
            Self::Dot => false,
            Self::Index { through_pointer } | Self::Deref { through_pointer } => through_pointer,
        }
    }

    /// The code the Lean vectors spell.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::Dot => "dot",
            Self::Index {
                through_pointer: true,
            } => "index-pointer",
            Self::Index {
                through_pointer: false,
            } => "index-array",
            Self::Deref {
                through_pointer: true,
            } => "deref-pointer",
            Self::Deref {
                through_pointer: false,
            } => "deref-array",
        }
    }
}

impl RootKind {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Pointer => "pointer",
            Self::Record => "record",
            Self::Array => "array",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaceStep {
    pub(crate) kind: StepKind,
    /// The member name for `->`/`.`, empty otherwise.
    pub(crate) member: String,
    /// The `FieldDecl` id for `->`/`.`, empty otherwise.
    pub(crate) field: String,
    /// Clang's type of the step's result (the place after this step).
    pub(crate) ty: String,
    /// Clang's type of the value or object the step reads through (its view).
    pub(crate) view: String,
    /// Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): how a member step's
    /// member occupies storage; `Ordinary` for every other step.
    pub(crate) storage: crate::union_bitfield::MemberStorage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedPlace {
    /// The root's Clang declaration id.
    pub(crate) decl: String,
    pub(crate) name: String,
    pub(crate) root: RootKind,
    /// `ParmVarDecl` or `VarDecl`.
    pub(crate) decl_kind: String,
    pub(crate) root_type: String,
    /// In evaluation order from the root.
    pub(crate) steps: Vec<PlaceStep>,
}

/// Why the place layer refused. Every unlisted form is one of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A node kind the place grammar does not list.
    Form(String),
    /// A cast kind the place grammar does not list, or a missing one.
    Cast(String),
    /// A member of a union, a bit-field, or a field without complete facts.
    Field,
    /// A missing Clang type or declaration fact.
    Missing,
    /// A `volatile`, `_Atomic` or `union` base.
    Qualifier,
    /// An arrow whose base is not a pointer, or a dot whose base is one.
    Operator,
    /// The root's spelling names another declaration in the function, or the
    /// root is not one parameter or automatic local of this function.
    Identity,
    /// The tracked-root decision refused (see [`Decision`]).
    Decision(Decision),
    /// Card CG13 (rule 10w): a pointer read through after a union-member
    /// step. A pointer member of a union carries no tracked identity, so it
    /// is never a link of the chain model.
    UnionHop,
}

impl Refusal {
    fn code(&self) -> String {
        match self {
            Self::Form(kind) => format!("form:{kind}"),
            Self::Cast(kind) => format!("cast:{kind}"),
            Self::Field => "field".into(),
            Self::Missing => "missing".into(),
            Self::Qualifier => "qualifier".into(),
            Self::Operator => "operator".into(),
            Self::Identity => "identity".into(),
            Self::Decision(decision) => decision.code().into(),
            Self::UnionHop => "union-hop".into(),
        }
    }
}

/// The tracked-root decision over a place's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The only pointer read through is the root: a chain with no link.
    RootOnly,
    /// The root and the direct member place `R->F`: `F` is the first link.
    DirectMember,
    /// A record-object root keeps the never-bound premise.
    RecordRoot,
    /// An array root keeps the never-bound premise.
    ArrayRoot,
    /// A pointer loaded from the root's element or pointee (`R[i]->x`,
    /// `(*R)->x`): the projection binds element places, not these.
    ElementPlace,
    /// A pointer loaded from a place deeper than `R->F`.
    DeeperPlace,
    /// No step reads through the root (not a place this rule reads).
    NoRootHop,
}

impl Decision {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::RootOnly => "root-only",
            Self::DirectMember => "direct-member",
            Self::RecordRoot => "record-root",
            Self::ArrayRoot => "array-root",
            Self::ElementPlace => "element-place",
            Self::DeeperPlace => "deeper-place",
            Self::NoRootHop => "no-root-hop",
        }
    }

    pub(crate) fn admits(self) -> bool {
        matches!(self, Self::RootOnly | Self::DirectMember)
    }
}

/// The tracked-root decision: a pure function of the root's kind and the
/// steps' kinds. `QCore.TypedPlace.decide` is the same function.
pub(crate) fn decide(root: RootKind, steps: &[StepKind]) -> Decision {
    match root {
        RootKind::Record => return Decision::RecordRoot,
        RootKind::Array => return Decision::ArrayRoot,
        RootKind::Pointer => {}
    }
    let hops = steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| step.hop().then_some(index))
        .collect::<Vec<_>>();
    match hops.as_slice() {
        [0] => Decision::RootOnly,
        [0, 1, ..] if steps[0] == StepKind::Arrow => Decision::DirectMember,
        [0, 1, ..] => Decision::ElementPlace,
        [0, _, ..] => Decision::DeeperPlace,
        _ => Decision::NoRootHop,
    }
}

/// The decision vectors shared with `proof/QCore/TypedPlace.lean`
/// (`decisionVectors`): a C spelling, its root kind, its step codes and the
/// decision. The Rust test binds each spelling to Clang's reading of it.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str, &[&str], &str)] = &[
    ("s->a", "pointer", &["arrow"], "root-only"),
    ("s->in.x", "pointer", &["arrow", "dot"], "root-only"),
    (
        "s->arr[i]",
        "pointer",
        &["arrow", "index-array"],
        "root-only",
    ),
    (
        "s->in.arr[i]",
        "pointer",
        &["arrow", "dot", "index-array"],
        "root-only",
    ),
    ("s[i].a", "pointer", &["index-pointer", "dot"], "root-only"),
    (
        "(*s).in.x",
        "pointer",
        &["deref-pointer", "dot", "dot"],
        "root-only",
    ),
    (
        "s->next->a",
        "pointer",
        &["arrow", "arrow"],
        "direct-member",
    ),
    (
        "s->ints[i]",
        "pointer",
        &["arrow", "index-pointer"],
        "direct-member",
    ),
    (
        "*s->ip",
        "pointer",
        &["arrow", "deref-pointer"],
        "direct-member",
    ),
    (
        "s->next->in.x",
        "pointer",
        &["arrow", "arrow", "dot"],
        "direct-member",
    ),
    (
        "s->next->next->a",
        "pointer",
        &["arrow", "arrow", "arrow"],
        "direct-member",
    ),
    (
        "s->in.next->a",
        "pointer",
        &["arrow", "dot", "arrow"],
        "deeper-place",
    ),
    (
        "s->links[i]->a",
        "pointer",
        &["arrow", "index-array", "arrow"],
        "deeper-place",
    ),
    (
        "v[i]->a",
        "pointer",
        &["index-pointer", "arrow"],
        "element-place",
    ),
    (
        "(*v)->a",
        "pointer",
        &["deref-pointer", "arrow"],
        "element-place",
    ),
    ("o.in.x", "record", &["dot", "dot"], "record-root"),
    ("t[i].a", "array", &["index-array", "dot"], "array-root"),
];

/// Read the place `node` designates, from the Clang AST only.
pub(crate) fn read_place(
    node: &Value,
    fields: &BTreeMap<String, Option<FieldFacts>>,
) -> Result<TypedPlace, Refusal> {
    let mut steps = Vec::new();
    let mut cursor = lvalue(node)?;
    let root_kind = loop {
        match kind(cursor) {
            "MemberExpr" => {
                let facts = fields
                    .get(
                        cursor["referencedMemberDecl"]
                            .as_str()
                            .ok_or(Refusal::Missing)?,
                    )
                    .ok_or(Refusal::Field)?
                    .as_ref()
                    .ok_or(Refusal::Field)?;
                // Card CG13 (rule 10w): a union member or a bit-field is a
                // step; its access is decided by `union_bitfield::decide` in
                // the reader that asked, which also requires a bit-field's unit.
                let storage = match (facts.union_owner, facts.bitfield) {
                    (false, false) => crate::union_bitfield::MemberStorage::Ordinary,
                    (true, false) => crate::union_bitfield::MemberStorage::Union,
                    (false, true) => crate::union_bitfield::MemberStorage::Bitfield,
                    (true, true) => return Err(Refusal::Field),
                };
                let base = children(cursor).first().ok_or(Refusal::Missing)?;
                let arrow = cursor["isArrow"].as_bool() == Some(true);
                let (base, arrow) =
                    crate::typed_position_shapes::arrow_through_address(base, arrow);
                let view = qual(base).ok_or(Refusal::Missing)?;
                if (view.contains("union ")
                    && storage != crate::union_bitfield::MemberStorage::Union)
                    || view.contains("volatile")
                    || view.contains("_Atomic")
                {
                    return Err(Refusal::Qualifier);
                }
                let pointer = view.trim_end().ends_with('*');
                if arrow != pointer {
                    return Err(Refusal::Operator);
                }
                steps.push(PlaceStep {
                    kind: if arrow {
                        StepKind::Arrow
                    } else {
                        StepKind::Dot
                    },
                    member: cursor["name"].as_str().ok_or(Refusal::Missing)?.to_owned(),
                    field: cursor["referencedMemberDecl"]
                        .as_str()
                        .ok_or(Refusal::Missing)?
                        .to_owned(),
                    ty: qual(cursor).ok_or(Refusal::Missing)?.to_owned(),
                    view: view.to_owned(),
                    storage,
                });
                if arrow {
                    let (inner, through_pointer) = through(base)?;
                    if !through_pointer {
                        // `array->f` is `array[0].f`: not a form this lists.
                        return Err(Refusal::Cast("ArrayToPointerDecay".into()));
                    }
                    cursor = inner;
                    if kind(cursor) == "DeclRefExpr" {
                        break RootKind::Pointer;
                    }
                } else {
                    cursor = lvalue(base)?;
                    if kind(cursor) == "DeclRefExpr" {
                        break RootKind::Record;
                    }
                }
            }
            "ArraySubscriptExpr" => {
                let pair = children(cursor);
                if pair.len() != 2 {
                    return Err(Refusal::Missing);
                }
                let view = qual(&pair[0]).ok_or(Refusal::Missing)?;
                if view.contains("volatile") || view.contains("_Atomic") {
                    return Err(Refusal::Qualifier);
                }
                let (inner, through_pointer) = through(&pair[0])?;
                steps.push(PlaceStep {
                    kind: StepKind::Index { through_pointer },
                    member: String::new(),
                    field: String::new(),
                    ty: qual(cursor).ok_or(Refusal::Missing)?.to_owned(),
                    view: view.to_owned(),
                    storage: crate::union_bitfield::MemberStorage::Ordinary,
                });
                cursor = inner;
                if kind(cursor) == "DeclRefExpr" {
                    break if through_pointer {
                        RootKind::Pointer
                    } else {
                        RootKind::Array
                    };
                }
            }
            "UnaryOperator" if cursor["opcode"].as_str() == Some("*") => {
                let operand = children(cursor).first().ok_or(Refusal::Missing)?;
                let view = qual(operand).ok_or(Refusal::Missing)?;
                if view.contains("volatile") || view.contains("_Atomic") {
                    return Err(Refusal::Qualifier);
                }
                let (inner, through_pointer) = through(operand)?;
                steps.push(PlaceStep {
                    kind: StepKind::Deref { through_pointer },
                    member: String::new(),
                    field: String::new(),
                    ty: qual(cursor).ok_or(Refusal::Missing)?.to_owned(),
                    view: view.to_owned(),
                    storage: crate::union_bitfield::MemberStorage::Ordinary,
                });
                cursor = inner;
                if kind(cursor) == "DeclRefExpr" {
                    break if through_pointer {
                        RootKind::Pointer
                    } else {
                        RootKind::Array
                    };
                }
            }
            other => return Err(Refusal::Form(other.to_owned())),
        }
    };
    let decl = &cursor["referencedDecl"];
    let decl_kind = decl["kind"].as_str().ok_or(Refusal::Missing)?;
    if !matches!(decl_kind, "VarDecl" | "ParmVarDecl") {
        return Err(Refusal::Form(decl_kind.to_owned()));
    }
    steps.reverse();
    Ok(TypedPlace {
        decl: decl["id"].as_str().ok_or(Refusal::Missing)?.to_owned(),
        name: decl["name"].as_str().ok_or(Refusal::Missing)?.to_owned(),
        root: root_kind,
        decl_kind: decl_kind.to_owned(),
        root_type: qual(cursor).ok_or(Refusal::Missing)?.to_owned(),
        steps,
    })
}

/// An lvalue operand: parentheses only. Any implicit conversion here would
/// make the operand a value, which a place's base never is.
fn lvalue(mut node: &Value) -> Result<&Value, Refusal> {
    loop {
        match kind(node) {
            "ParenExpr" => node = children(node).first().ok_or(Refusal::Missing)?,
            "ImplicitCastExpr" => {
                // The access itself may be read as a value; its place is the
                // operand of that one load or qualification conversion.
                let cast = node["castKind"].as_str().ok_or(Refusal::Missing)?;
                if !matches!(cast, "LValueToRValue" | "NoOp") {
                    return Err(Refusal::Cast(cast.to_owned()));
                }
                node = children(node).first().ok_or(Refusal::Missing)?;
            }
            _ => return Ok(node),
        }
    }
}

/// The base of an arrow, subscript or indirection: `(inner, true)` for a
/// pointer value read from memory, `(inner, false)` for an array decaying to
/// its first element. A qualification conversion and parentheses pass; any
/// other conversion, and a base with neither, is refused.
fn through(mut node: &Value) -> Result<(&Value, bool), Refusal> {
    let mut decided = None;
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" => {
                let cast = node["castKind"].as_str().ok_or(Refusal::Missing)?;
                match (cast, decided) {
                    ("NoOp", _) => {}
                    ("LValueToRValue", None) => decided = Some(true),
                    ("ArrayToPointerDecay", None) => decided = Some(false),
                    _ => return Err(Refusal::Cast(cast.to_owned())),
                }
            }
            _ => {
                let through_pointer =
                    decided.ok_or_else(|| Refusal::Form(format!("base:{}", kind(node))))?;
                return Ok((node, through_pointer));
            }
        }
        node = children(node).first().ok_or(Refusal::Missing)?;
    }
}

/// The root is one parameter or automatic local of `function` and no
/// reference in it names another object of that spelling.
pub(crate) fn root_identity(function: &Value, place: &TypedPlace) -> bool {
    let mut declarations = Vec::new();
    let mut foreign = false;
    visit(function, &mut |node| match kind(node) {
        "ParmVarDecl" | "VarDecl" if node["name"].as_str() == Some(place.name.as_str()) => {
            declarations.push((
                node["id"].as_str().map(str::to_owned),
                node["storageClass"].as_str().map(str::to_owned),
            ));
        }
        "DeclRefExpr"
            if node["referencedDecl"]["name"].as_str() == Some(place.name.as_str())
                && node["referencedDecl"]["id"].as_str() != Some(place.decl.as_str()) =>
        {
            foreign = true;
        }
        _ => {}
    });
    !foreign
        && matches!(declarations.as_slice(),
            [(Some(id), None)] if *id == place.decl)
}

fn visit(node: &Value, f: &mut impl FnMut(&Value)) {
    f(node);
    for child in children(node) {
        visit(child, f);
    }
}

/// The spelling of `steps` from its first member, `in.p` or `items[]`.
fn spell(steps: &[PlaceStep]) -> String {
    let mut spelled = String::new();
    for (index, step) in steps.iter().enumerate() {
        match step.kind {
            StepKind::Arrow if index > 0 => spelled.push_str("->"),
            StepKind::Dot if index > 0 => spelled.push('.'),
            StepKind::Deref { .. } => spelled.insert(0, '*'),
            StepKind::Index { .. } => spelled.push_str("[]"),
            _ => {}
        }
        spelled.push_str(&step.member);
    }
    spelled
}

/// What the tracked-root route admits for one place: its chain obligation
/// and whether a store to it writes through a subscript or indirection.
#[derive(Clone, Debug)]
pub(crate) struct TrackedPlace {
    pub(crate) chain: AccessChain,
    pub(crate) through_element: bool,
}

/// The tracked-root route: the place `node` designates, if its root is a
/// pointer this function declares once and [`decide`] admits its shape.
///
/// Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): the decision is
/// recorded into the consultation's own `decisions`, when its consumer
/// records one.
pub(crate) fn tracked_chain(
    node: &Value,
    fields: &BTreeMap<String, Option<FieldFacts>>,
    function: Option<&Value>,
    decisions: Option<&crate::consultation_decisions::Decisions>,
) -> Result<TrackedPlace, Refusal> {
    let place = read_place(node, fields).inspect_err(|refusal| {
        record_decision(decisions, None, Some(refusal));
    })?;
    let result = tracked_chain_of(&place, function);
    record_decision(decisions, Some(&place), result.as_ref().err());
    result
}

fn tracked_chain_of(place: &TypedPlace, function: Option<&Value>) -> Result<TrackedPlace, Refusal> {
    let kinds = place.steps.iter().map(|step| step.kind).collect::<Vec<_>>();
    let decision = decide(place.root, &kinds);
    if !decision.admits() {
        return Err(Refusal::Decision(decision));
    }
    if !function.is_some_and(|function| root_identity(function, place)) {
        return Err(Refusal::Identity);
    }
    // Card CG13 (rule 10w): no pointer loaded from a union member's storage
    // is a link.
    if let Some(first_union) = place
        .steps
        .iter()
        .position(|step| step.storage == crate::union_bitfield::MemberStorage::Union)
        && kinds[first_union + 1..].iter().any(|step| step.hop())
    {
        return Err(Refusal::UnionHop);
    }
    let hops = kinds
        .iter()
        .enumerate()
        .filter_map(|(index, step)| step.hop().then_some(index))
        .collect::<Vec<_>>();
    // Every hop but the root's is a link: the spelled selection from the
    // previous hop to it (`F`, then `in.p`, …).
    let links = hops
        .windows(2)
        .map(|pair| spell(&place.steps[pair[0]..pair[1]]))
        .collect::<Vec<_>>();
    let tail = hops.last().map_or(0, |&last| last);
    let member = spell(&place.steps[tail..]);
    Ok(TrackedPlace {
        chain: AccessChain {
            root: place.name.clone(),
            dereferenced_root: true,
            // The flow decides: a tracked root must be exactly one live
            // identity; this route never reads the never-bound premise.
            root_never_bound: false,
            // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): or, for an
            // eligible root, an untracked borrowed object on every path.
            root_bound_untracked: function.is_some_and(|function| {
                crate::bound_untracked::eligible_root(function, &place.name, &place.decl)
            }),
            root_untracked_only: false,
            links,
            member,
        },
        through_element: kinds
            .iter()
            .any(|step| matches!(step, StepKind::Index { .. } | StepKind::Deref { .. })),
    })
}

/// Parity only: one place decision, into the consultation's own record
/// (rule 12q). A reader whose consumer records nothing keeps nothing.
fn record_decision(
    decisions: Option<&crate::consultation_decisions::Decisions>,
    place: Option<&TypedPlace>,
    refusal: Option<&Refusal>,
) {
    let Some(decisions) = decisions else {
        return;
    };
    decisions.place(|| {
        serde_json::json!({
            "place": place.map(|place| serde_json::json!({
                "decl": place.decl,
                "name": place.name,
                "decl_kind": place.decl_kind,
                "root": place.root.code(),
                "root_type": place.root_type,
                "steps": place.steps.iter().map(|step| serde_json::json!({
                    "kind": step.kind.code(),
                    "member": step.member,
                    "field": step.field,
                    "type": step.ty,
                    "view": step.view,
                    "storage": step.storage.code(),
                })).collect::<Vec<_>>(),
            })),
            "admitted": refusal.is_none(),
            "refusal": refusal.map(Refusal::code),
        })
    });
}

/// I04: the gateway's own occurrence of a token run on its line.
#[derive(Clone, Debug)]
pub(crate) struct Occurrence {
    /// Which occurrence of the run's text on the line this run is, from 0.
    pub(crate) ordinal: usize,
    /// Every occurrence's count on the line.
    pub(crate) count: usize,
    /// The line's tokens in the gateway's stream.
    pub(crate) line_texts: Vec<String>,
}

/// The occurrence of `run` in `stream` (the parser's token stream). `origin`
/// is the stream index and length of the statement being classified, when
/// there is one: `run` is that statement exactly when it has its length and
/// its tokens (text and line) there. A run that is not (a clause, an operand,
/// a desugared or rewritten token) has no occurrence, and nor has a run that
/// spans lines.
pub(crate) fn occurrence(
    stream: &[Token],
    origin: Option<(usize, usize)>,
    run: &[Token],
) -> Option<Occurrence> {
    let (at, len) = origin?;
    let first = run.first()?;
    let line = first.line;
    if run.len() != len
        || run.iter().any(|token| token.line != line)
        || !stream
            .get(at..at + len)?
            .iter()
            .zip(run)
            .all(|(left, right)| left.text == right.text && left.line == right.line)
        || !stream.windows(2).all(|pair| pair[0].line <= pair[1].line)
    {
        return None;
    }
    let lo = stream.partition_point(|token| token.line < line);
    let hi = stream.partition_point(|token| token.line <= line);
    if !(lo..hi).contains(&at) {
        return None;
    }
    let texts = run
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let line_texts = stream[lo..hi]
        .iter()
        .map(|token| token.text.clone())
        .collect::<Vec<_>>();
    let starts = starts_of(&line_texts, &texts);
    Some(Occurrence {
        ordinal: starts.iter().position(|&start| start == at - lo)?,
        count: starts.len(),
        line_texts,
    })
}

fn starts_of<S: AsRef<str>>(line: &[S], texts: &[&str]) -> Vec<usize> {
    if texts.is_empty() || texts.len() > line.len() {
        return Vec::new();
    }
    (0..=line.len() - texts.len())
        .filter(|&start| {
            line[start..start + texts.len()]
                .iter()
                .zip(texts)
                .all(|(left, right)| left.as_ref() == *right)
        })
        .collect()
}

/// I04: choose among several `candidates` (each already matching `texts` on
/// `line`) by `occurrence`, or refuse. `original_line` is the analysed
/// source's own tokens of `line`; `span` gives a candidate's begin offset and
/// its first and last lines.
pub(crate) fn by_occurrence<'a>(
    candidates: &[&'a Value],
    occurrence: &Occurrence,
    original_line: &[String],
    line: usize,
    span: impl Fn(&Value) -> Option<(usize, usize, usize)>,
) -> Option<&'a Value> {
    if candidates.len() < 2 || occurrence.line_texts != original_line {
        return None;
    }
    let mut spans = candidates
        .iter()
        .map(|node| {
            let (offset, first, last) = span(node)?;
            (first == line && last == line).then_some((offset, *node))
        })
        .collect::<Option<Vec<_>>>()?;
    spans.sort_by_key(|(offset, _)| *offset);
    if spans.windows(2).any(|pair| pair[0].0 == pair[1].0) || spans.len() != occurrence.count {
        return None;
    }
    spans.get(occurrence.ordinal).map(|(_, node)| *node)
}

#[cfg(test)]
#[path = "../tests/unit/typed_place.rs"]
mod tests;
