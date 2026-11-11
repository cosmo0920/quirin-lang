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

//! Card **CG17-LAYOUT-DERIVED-EXTENTS** (gateway rule 11u): layout facts
//! derived from Clang's own layout values, never re-derived on the host.
//!
//! Four facts, each a pure decision here with its Clang reader beside it, and
//! the route-G adapter in `legacy_c/legacy_layout_extents.rs`:
//!
//! * **Variable-length arrays** (M12, [`vla_decision`]). A declaration of one
//!   automatic array of a builtin scalar element type whose bound is a
//!   variable expression evaluates the bound **once**, before the array's
//!   lifetime starts, and carries three numeric obligations this card never
//!   discharges (CG20 owns numbers): the bound's arithmetic is defined, the
//!   bound is positive (C11 6.7.6.2p5) and the byte size is representable.
//!   The lifetime ends at the block's exit, so an address of the array that
//!   may leave the block (any use but an element access or a `sizeof`
//!   operand) is never admitted: such a declaration keeps its refusal.
//! * **Flexible array members** (M13, [`flexible_extent`]). The remaining
//!   extent of `T data[]` is `(size - offsetof(data)) / sizeof(T)` elements,
//!   and only when the allocation size is established. No current contract
//!   establishes one (the extent vocabulary is CG16's, rule 11s), so every
//!   corpus access reads `size-not-established`; the fact is an input to
//!   CG16's bounds, never a custody fact.
//! * **Alignment** (M19, [`alignment`]). A conversion of a derived address
//!   (pointer arithmetic, or `&` of an object) to a pointer whose referenced
//!   type needs alignment `r` is undefined unless the address is aligned
//!   (C11 6.3.2.3p7). The obligation is *aligned* when the base's guaranteed
//!   alignment and the offset are multiples of `r`, *misaligned* when the base
//!   alignment is and a constant offset is not, and *undecided* otherwise.
//!   The published property `layout.alignment.v1` ([`alignment_verdict`]) is
//!   `Safe` only when every obligation is aligned; a misaligned obligation is
//!   a provisional candidate (no reachability validation) and joins to
//!   `Unknown`. A pointer value's guaranteed alignment is its pointee's
//!   (`P-CG17-POINTER-ALIGNED`, C11 6.3.2.3p7, stated, not proved).
//! * **container_of** (E18, [`container_of`]). `(T *)((char *)p -
//!   offsetof(T, m))` is the enclosing object only when `p` is established as
//!   pointing at member `m` of a `T` object ([`member_provenance`]): `&x->m`
//!   written in the operand, or a pointer local whose only write is such an
//!   initializer while `x` is never written. Every other pointer is refused.
//!
//! Custody is unchanged by the extent and alignment facts: they are separate
//! properties, reported apart (projection coverage is not safety coverage).
//! `QCore.LayoutExtents` states the decisions; the vectors below are bound to
//! it by `tests/unit/layout_extents.rs`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::elaboration::strip_parens;
use crate::typed_c_ast::{children, kind};

// ---------------------------------------------------------------------------
// Flexible array members
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexRefusal {
    SizeNotEstablished,
    BelowMember,
    ZeroWidth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlexExtent {
    Elements(u128),
    Refused(FlexRefusal),
}

/// `QCore.LayoutExtents.flexibleExtent`: the allocation size (if
/// established), the member's byte offset and the element width, all Clang's.
pub fn flexible_extent(size: Option<u128>, offset: u128, width: u128) -> FlexExtent {
    let Some(size) = size else {
        return FlexExtent::Refused(FlexRefusal::SizeNotEstablished);
    };
    if width == 0 {
        FlexExtent::Refused(FlexRefusal::ZeroWidth)
    } else if size < offset {
        FlexExtent::Refused(FlexRefusal::BelowMember)
    } else {
        FlexExtent::Elements((size - offset) / width)
    }
}

/// `QCore.LayoutExtents.flexVectors`.
#[cfg(test)]
pub(crate) const FLEX_VECTORS: &[(Option<u128>, u128, u128, FlexExtent)] = &[
    (Some(20), 4, 4, FlexExtent::Elements(4)),
    (Some(22), 4, 4, FlexExtent::Elements(4)),
    (Some(4), 4, 4, FlexExtent::Elements(0)),
    (Some(3), 4, 4, FlexExtent::Refused(FlexRefusal::BelowMember)),
    (Some(40), 8, 0, FlexExtent::Refused(FlexRefusal::ZeroWidth)),
    (
        None,
        4,
        4,
        FlexExtent::Refused(FlexRefusal::SizeNotEstablished),
    ),
    (Some(24), 8, 8, FlexExtent::Elements(2)),
];

// ---------------------------------------------------------------------------
// Variable-length arrays
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundClass {
    ReadsOnly,
    Effect,
    Unread,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VlaUse {
    Element,
    SizeQuery,
    Address,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VlaDecision {
    Admitted,
    ElementNotScalar,
    BoundEffect,
    BoundUnread,
    AddressEscapes,
}

impl VlaDecision {
    pub fn code(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::ElementNotScalar => "element-not-scalar",
            Self::BoundEffect => "bound-effect",
            Self::BoundUnread => "bound-unread",
            Self::AddressEscapes => "address-escapes",
        }
    }
}

/// `QCore.LayoutExtents.vlaDecision`.
pub fn vla_decision(scalar_element: bool, bound: BoundClass, uses: &[VlaUse]) -> VlaDecision {
    if !scalar_element {
        VlaDecision::ElementNotScalar
    } else if bound == BoundClass::Effect {
        VlaDecision::BoundEffect
    } else if bound == BoundClass::Unread {
        VlaDecision::BoundUnread
    } else if uses.contains(&VlaUse::Address) {
        VlaDecision::AddressEscapes
    } else {
        VlaDecision::Admitted
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericObligation {
    BoundDefined,
    BoundPositive,
    SizeRepresentable,
}

impl NumericObligation {
    pub fn code(self) -> &'static str {
        match self {
            Self::BoundDefined => "bound-defined",
            Self::BoundPositive => "bound-positive",
            Self::SizeRepresentable => "size-representable",
        }
    }
}

/// `QCore.LayoutExtents.obligationsOf`: never discharged here (CG20's).
pub fn obligations_of(decision: VlaDecision) -> &'static [NumericObligation] {
    match decision {
        VlaDecision::Admitted => &[
            NumericObligation::BoundDefined,
            NumericObligation::BoundPositive,
            NumericObligation::SizeRepresentable,
        ],
        _ => &[],
    }
}

/// `QCore.LayoutExtents.vlaVectors`.
#[cfg(test)]
pub(crate) const VLA_VECTORS: &[(bool, BoundClass, &[VlaUse], VlaDecision)] = &[
    (true, BoundClass::ReadsOnly, &[], VlaDecision::Admitted),
    (
        true,
        BoundClass::ReadsOnly,
        &[VlaUse::Element, VlaUse::SizeQuery],
        VlaDecision::Admitted,
    ),
    (
        true,
        BoundClass::ReadsOnly,
        &[VlaUse::Element, VlaUse::Address],
        VlaDecision::AddressEscapes,
    ),
    (
        true,
        BoundClass::Effect,
        &[VlaUse::Element],
        VlaDecision::BoundEffect,
    ),
    (true, BoundClass::Unread, &[], VlaDecision::BoundUnread),
    (
        false,
        BoundClass::ReadsOnly,
        &[VlaUse::Element],
        VlaDecision::ElementNotScalar,
    ),
];

/// Builtin scalar element spellings a VLA may hold (no typedef, no pointer,
/// no record: an element that may carry an identity is not this card's).
fn builtin_scalar(text: &str) -> bool {
    let words = text.split_whitespace().collect::<Vec<_>>();
    !words.is_empty()
        && words.iter().all(|word| {
            matches!(
                *word,
                "const"
                    | "signed"
                    | "unsigned"
                    | "short"
                    | "long"
                    | "char"
                    | "int"
                    | "float"
                    | "double"
                    | "_Bool"
            )
        })
        && words.iter().any(|word| *word != "const")
}

/// One VLA declaration read from Clang.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VlaFact {
    pub name: String,
    pub element: String,
    pub bound: String,
    pub bound_class: BoundClass,
    pub uses: Vec<VlaUse>,
    pub decision: VlaDecision,
}

/// Split `ELEM[BOUND]` (one bracket pair, at the end) of a Clang type text.
fn vla_type(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let open = text.find('[')?;
    if !text.ends_with(']') || text[open + 1..].contains('[') {
        return None;
    }
    Some((text[..open].trim(), text[open + 1..text.len() - 1].trim()))
}

/// The VLA a `DeclStmt` declares, with its decision; `None` when the node is
/// not one declaration of one automatic, variably modified array object.
pub fn vla_declaration(statement: &Value, function: &Value) -> Option<VlaFact> {
    let statement = strip_parens(statement);
    if kind(statement) != "DeclStmt" {
        return None;
    }
    let [declaration] = children(statement) else {
        return None;
    };
    if kind(declaration) != "VarDecl"
        || !children(declaration).is_empty()
        || declaration["storageClass"].as_str().is_some()
    {
        return None;
    }
    let text = declaration["type"]["qualType"].as_str()?;
    if !crate::cast_representation::variably_modified(text) {
        return None;
    }
    let name = declaration["name"].as_str()?.to_owned();
    let id = declaration["id"].as_str()?;
    let (element, bound) = vla_type(text).unwrap_or_default();
    let declared_once = declarations_named(function, &name) == 1;
    let bound_class = if declared_once {
        bound_class(bound, function)
    } else {
        BoundClass::Unread
    };
    let uses = vla_uses(function, id);
    let decision = vla_decision(builtin_scalar(element), bound_class, &uses);
    Some(VlaFact {
        name,
        element: element.to_owned(),
        bound: bound.to_owned(),
        bound_class,
        uses,
        decision,
    })
}

fn declarations_named(node: &Value, name: &str) -> usize {
    let own = usize::from(
        matches!(kind(node), "VarDecl" | "ParmVarDecl") && node["name"].as_str() == Some(name),
    );
    own + children(node)
        .iter()
        .map(|child| declarations_named(child, name))
        .sum::<usize>()
}

fn declaration_type<'v>(node: &'v Value, name: &str) -> Option<&'v Value> {
    if matches!(kind(node), "VarDecl" | "ParmVarDecl") && node["name"].as_str() == Some(name) {
        return Some(node);
    }
    children(node)
        .iter()
        .find_map(|child| declaration_type(child, name))
}

/// The bound's evaluation class, from Clang's printed bound (the declared
/// type's text): reads only when every token is an integer literal, one of
/// `+ - * ( )`, or a name declared exactly once in the function as a
/// non-`volatile` integer parameter or local. A call, a store, an increment
/// or anything else unread is never `ReadsOnly`.
fn bound_class(bound: &str, function: &Value) -> BoundClass {
    let tokens = crate::legacy_c::tokenize(bound);
    if tokens.is_empty() {
        return BoundClass::Unread;
    }
    let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    if texts
        .windows(2)
        .any(|pair| crate::legacy_c::is_identifier(pair[0]) && pair[1] == "(")
        || texts.iter().any(|text| matches!(*text, "=" | "++" | "--"))
        || texts
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
    {
        return BoundClass::Effect;
    }
    for text in &texts {
        let readable = match *text {
            "+" | "-" | "*" | "(" | ")" => true,
            literal if literal.bytes().all(|byte| byte.is_ascii_digit()) => true,
            name if crate::legacy_c::is_identifier(name) => {
                declarations_named(function, name) == 1
                    && declaration_type(function, name).is_some_and(|declaration| {
                        let ty = declaration["type"]["desugaredQualType"]
                            .as_str()
                            .or_else(|| declaration["type"]["qualType"].as_str())
                            .unwrap_or("");
                        builtin_scalar(ty)
                            && !ty.contains("float")
                            && !ty.contains("double")
                            && declaration["storageClass"].as_str().is_none()
                    })
            }
            _ => false,
        };
        if !readable {
            return BoundClass::Unread;
        }
    }
    BoundClass::ReadsOnly
}

/// Every use of the array object `id` in the function, classified.
fn vla_uses(function: &Value, id: &str) -> Vec<VlaUse> {
    fn walk<'v>(node: &'v Value, id: &str, parents: &mut Vec<&'v Value>, out: &mut Vec<VlaUse>) {
        if kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id) {
            out.push(classify(parents));
        }
        parents.push(node);
        for child in children(node) {
            walk(child, id, parents, out);
        }
        parents.pop();
    }
    fn classify(parents: &[&Value]) -> VlaUse {
        // The nearest ancestors, parentheses skipped.
        let mut chain = parents
            .iter()
            .rev()
            .filter(|node| kind(node) != "ParenExpr");
        match chain.next() {
            Some(parent) if kind(parent) == "UnaryExprOrTypeTraitExpr" => VlaUse::SizeQuery,
            Some(parent)
                if kind(parent) == "ImplicitCastExpr"
                    && parent["castKind"].as_str() == Some("ArrayToPointerDecay") =>
            {
                match chain.next() {
                    Some(subscript) if kind(subscript) == "ArraySubscriptExpr" => {
                        match chain.next() {
                            Some(outer)
                                if kind(outer) == "UnaryOperator"
                                    && outer["opcode"].as_str() == Some("&") =>
                            {
                                VlaUse::Address
                            }
                            Some(_) => VlaUse::Element,
                            None => VlaUse::Address,
                        }
                    }
                    _ => VlaUse::Address,
                }
            }
            _ => VlaUse::Address,
        }
    }
    let mut out = Vec::new();
    walk(function, id, &mut Vec::new(), &mut out);
    out
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offset {
    Const(i128),
    Scaled(u64),
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Aligned,
    Misaligned,
    Undecided,
}

impl Alignment {
    pub fn code(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Misaligned => "misaligned",
            Self::Undecided => "undecided",
        }
    }
}

/// `QCore.LayoutExtents.alignment`: the referenced type's alignment, the
/// base's guaranteed alignment and the offset, all in bytes.
pub fn alignment(required: u64, base: u64, offset: Offset) -> Alignment {
    if required == 0 || !base.is_multiple_of(required) {
        return Alignment::Undecided;
    }
    match offset {
        Offset::Const(k) => {
            if k.rem_euclid(i128::from(required)) == 0 {
                Alignment::Aligned
            } else {
                Alignment::Misaligned
            }
        }
        Offset::Scaled(stride) if stride.is_multiple_of(required) => Alignment::Aligned,
        Offset::Scaled(_) | Offset::Unknown => Alignment::Undecided,
    }
}

/// The published property's verdict class (`layout.alignment.v1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyVerdict {
    Safe,
    Unknown,
}

/// `QCore.LayoutExtents.alignmentVerdict`.
pub fn alignment_verdict(obligations: &[Alignment]) -> PropertyVerdict {
    if obligations.iter().all(|o| *o == Alignment::Aligned) {
        PropertyVerdict::Safe
    } else {
        PropertyVerdict::Unknown
    }
}

/// `QCore.LayoutExtents.alignmentVectors`.
#[cfg(test)]
pub(crate) const ALIGNMENT_VECTORS: &[(u64, u64, Offset, Alignment)] = &[
    (4, 1, Offset::Const(1), Alignment::Undecided),
    (4, 4, Offset::Const(1), Alignment::Misaligned),
    (4, 4, Offset::Const(8), Alignment::Aligned),
    (4, 8, Offset::Const(-4), Alignment::Aligned),
    (4, 4, Offset::Scaled(4), Alignment::Aligned),
    (8, 4, Offset::Scaled(8), Alignment::Undecided),
    (4, 4, Offset::Scaled(2), Alignment::Undecided),
    (4, 4, Offset::Unknown, Alignment::Undecided),
    (1, 1, Offset::Const(3), Alignment::Aligned),
    (0, 4, Offset::Const(0), Alignment::Undecided),
];

/// The pointee text of a pointer type text (`int *` → `int`).
fn pointee(text: &str) -> Option<&str> {
    // The pointer's own qualifiers (`T *const`, `T *restrict`) first.
    let mut text = text.trim();
    while let Some(rest) = text
        .strip_suffix("const")
        .or_else(|| text.strip_suffix("restrict"))
        .filter(|rest| rest.ends_with([' ', '*']))
    {
        text = rest.trim_end();
    }
    let stripped = text.strip_suffix('*')?.trim_end();
    (!stripped.is_empty() && !stripped.contains('(')).then_some(stripped)
}

fn type_text(node: &Value) -> Option<&str> {
    node["type"]["qualType"].as_str()
}

/// Character and `void` referents need no alignment beyond one byte.
fn byte_aligned(text: &str) -> bool {
    let words = text
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "volatile" | "restrict"))
        .collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["char"] | ["signed", "char"] | ["unsigned", "char"] | ["void"]
    )
}

/// How the base address's guaranteed alignment is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseAddress {
    /// A pointer value of type `T *` (`P-CG17-POINTER-ALIGNED`): `_Alignof(T)`.
    PointerValue(String),
    /// `&x`: `_Alignof` of the object's type.
    Object(String),
    Unknown,
}

/// The offset term in elements of `element` (`sizeof(element)` bytes each).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OffsetTerm {
    Const { elements: i128, element: String },
    Scaled { element: String },
    Unknown,
}

/// One conversion of a derived address to an object pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedCast {
    pub begin: Option<usize>,
    pub target: String,
    pub base: BaseAddress,
    pub offset: OffsetTerm,
}

/// The pointer value under value-preserving pointer conversions, and the
/// base address it names.
fn base_address(node: &Value) -> BaseAddress {
    let mut node = strip_parens(node);
    loop {
        match kind(node) {
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(node["castKind"].as_str(), Some("BitCast" | "NoOp")) =>
            {
                // A conversion keeps the address, so the inner value's
                // alignment knowledge stands.
                let Some(inner) = children(node).first() else {
                    return BaseAddress::Unknown;
                };
                node = strip_parens(inner);
            }
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("LValueToRValue") => {
                let Some(inner) = children(node).first().map(strip_parens) else {
                    return BaseAddress::Unknown;
                };
                if kind(inner) != "DeclRefExpr" {
                    return BaseAddress::Unknown;
                }
                return type_text(inner)
                    .and_then(pointee)
                    .map_or(BaseAddress::Unknown, |p| {
                        BaseAddress::PointerValue(p.into())
                    });
            }
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                let Some(object) = children(node).first().map(strip_parens) else {
                    return BaseAddress::Unknown;
                };
                if kind(object) != "DeclRefExpr" {
                    return BaseAddress::Unknown;
                }
                return type_text(object)
                    .map_or(BaseAddress::Unknown, |t| BaseAddress::Object(t.into()));
            }
            _ => return BaseAddress::Unknown,
        }
    }
}

/// The derived address a conversion's operand forms: pointer arithmetic
/// `B ± k` over a base, or `&x` itself.
fn derived(operand: &Value) -> Option<(BaseAddress, OffsetTerm)> {
    // Value-preserving pointer conversions keep the address.
    let mut operand = strip_parens(operand);
    while matches!(kind(operand), "CStyleCastExpr" | "ImplicitCastExpr")
        && matches!(operand["castKind"].as_str(), Some("BitCast" | "NoOp"))
    {
        operand = strip_parens(children(operand).first()?);
    }
    match kind(operand) {
        "BinaryOperator" if matches!(operand["opcode"].as_str(), Some("+" | "-")) => {
            let [left, right] = children(operand) else {
                return None;
            };
            let (pointer, index, negative) = if pointee(type_text(left)?).is_some() {
                (left, right, operand["opcode"].as_str() == Some("-"))
            } else if operand["opcode"].as_str() == Some("+")
                && pointee(type_text(right)?).is_some()
            {
                (right, left, false)
            } else {
                return None;
            };
            let element = pointee(type_text(pointer)?)?.to_owned();
            let index = strip_parens(index);
            let offset = if kind(index) == "IntegerLiteral" {
                match index["value"].as_str().and_then(|v| v.parse::<i128>().ok()) {
                    Some(value) => OffsetTerm::Const {
                        elements: if negative { -value } else { value },
                        element,
                    },
                    None => OffsetTerm::Unknown,
                }
            } else {
                OffsetTerm::Scaled { element }
            };
            Some((base_address(pointer), offset))
        }
        "UnaryOperator" if operand["opcode"].as_str() == Some("&") => Some((
            base_address(operand),
            OffsetTerm::Const {
                elements: 0,
                element: "char".into(),
            },
        )),
        _ => None,
    }
}

/// Every conversion of a derived address to an object pointer whose
/// referent needs more than byte alignment, in the function's order.
pub fn derived_casts(function: &Value) -> Vec<DerivedCast> {
    fn walk(node: &Value, out: &mut Vec<DerivedCast>) {
        if matches!(kind(node), "CStyleCastExpr" | "ImplicitCastExpr")
            && node["castKind"].as_str() == Some("BitCast")
            && let Some(target) = type_text(node).and_then(pointee)
            && !byte_aligned(target)
            && let Some(operand) = children(node).first()
            && let Some((base, offset)) = derived(operand)
        {
            out.push(DerivedCast {
                begin: crate::typed_access_path::offset(&node["range"]["begin"]),
                target: target.to_owned(),
                base,
                offset,
            });
        }
        for child in children(node) {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(function, &mut out);
    out
}

/// The Clang constant expressions a cast's decision needs.
pub fn cast_queries(cast: &DerivedCast) -> Vec<String> {
    let mut queries = vec![format!("_Alignof({})", cast.target)];
    match &cast.base {
        BaseAddress::PointerValue(t) | BaseAddress::Object(t) if !byte_aligned(t) => {
            queries.push(format!("_Alignof({t})"));
        }
        _ => {}
    }
    match &cast.offset {
        OffsetTerm::Const { element, .. } | OffsetTerm::Scaled { element }
            if !byte_aligned(element) =>
        {
            queries.push(format!("sizeof({element})"));
        }
        _ => {}
    }
    queries
}

/// A cast's alignment obligation, with Clang's numbers (`numbers` answers a
/// query of [`cast_queries`], `None` when Clang gave none).
pub fn decide_cast(
    cast: &DerivedCast,
    numbers: &dyn Fn(&str) -> Option<i128>,
) -> (Option<u64>, Option<u64>, Offset, Alignment) {
    let size_of = |t: &str| -> Option<i128> {
        if byte_aligned(t) {
            Some(1)
        } else {
            numbers(&format!("sizeof({t})"))
        }
    };
    let align_of = |t: &str| -> Option<u64> {
        if byte_aligned(t) {
            Some(1)
        } else {
            numbers(&format!("_Alignof({t})")).and_then(|v| u64::try_from(v).ok())
        }
    };
    let required = align_of(&cast.target);
    let base = match &cast.base {
        BaseAddress::PointerValue(t) | BaseAddress::Object(t) => align_of(t),
        BaseAddress::Unknown => Some(1),
    };
    let offset = match &cast.offset {
        OffsetTerm::Const { elements, element } => size_of(element)
            .and_then(|size| elements.checked_mul(size))
            .map_or(Offset::Unknown, Offset::Const),
        OffsetTerm::Scaled { element } => size_of(element)
            .and_then(|size| u64::try_from(size).ok())
            .map_or(Offset::Unknown, Offset::Scaled),
        OffsetTerm::Unknown => Offset::Unknown,
    };
    let decision = match (required, base) {
        (Some(required), Some(base)) => alignment(required, base, offset),
        _ => Alignment::Undecided,
    };
    (required, base, offset, decision)
}

// ---------------------------------------------------------------------------
// container_of
// ---------------------------------------------------------------------------

/// What the gateway establishes about `container_of`'s pointer operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provenance<B, R, F> {
    MemberOf { base: B, record: R, path: Vec<F> },
    Unestablished(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerDecision<B> {
    Enclosing(B),
    Unestablished(&'static str),
    RecordMismatch,
    MemberMismatch,
}

impl<B> ContainerDecision<B> {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Enclosing(_) => "enclosing",
            Self::Unestablished(_) => "unestablished",
            Self::RecordMismatch => "record-mismatch",
            Self::MemberMismatch => "member-mismatch",
        }
    }
}

/// `QCore.LayoutExtents.containerOf`.
pub fn container_of<B: Clone, R: PartialEq, F: PartialEq>(
    provenance: &Provenance<B, R, F>,
    record: &R,
    path: &[F],
) -> ContainerDecision<B> {
    match provenance {
        Provenance::Unestablished(reason) => ContainerDecision::Unestablished(reason),
        Provenance::MemberOf {
            base,
            record: r,
            path: q,
        } => {
            if r != record {
                ContainerDecision::RecordMismatch
            } else if q.as_slice() != path {
                ContainerDecision::MemberMismatch
            } else {
                ContainerDecision::Enclosing(base.clone())
            }
        }
    }
}

/// `QCore.LayoutExtents.containerVectors` (Lean's `Nat` ids as `u32`).
#[cfg(test)]
pub(crate) type ContainerVector = (
    Provenance<u32, u32, u32>,
    u32,
    &'static [u32],
    ContainerDecision<u32>,
);

#[cfg(test)]
pub(crate) fn container_vectors() -> Vec<ContainerVector> {
    let member = |base, record, path: &[u32]| Provenance::MemberOf {
        base,
        record,
        path: path.to_vec(),
    };
    vec![
        (member(7, 1, &[2]), 1, &[2], ContainerDecision::Enclosing(7)),
        (
            member(7, 1, &[2, 3]),
            1,
            &[2, 3],
            ContainerDecision::Enclosing(7),
        ),
        (
            member(7, 1, &[2]),
            1,
            &[3],
            ContainerDecision::MemberMismatch,
        ),
        (
            member(7, 1, &[2]),
            4,
            &[2],
            ContainerDecision::RecordMismatch,
        ),
        (
            Provenance::Unestablished("unestablished"),
            1,
            &[2],
            ContainerDecision::Unestablished("unestablished"),
        ),
    ]
}

/// A `container_of` expression read from Clang: the record the result points
/// to, the pointer operand, and the record and member path the subtracted
/// offset names.
#[derive(Clone, Debug)]
pub struct ContainerShape<'v> {
    pub target: String,
    pub pointer: &'v Value,
    pub offset_record: String,
    pub offset_path: Vec<String>,
}

/// `(T *)((char *)P - OFFSET)` through parentheses, with `OFFSET` an
/// `offsetof(T, m.n…)` (read from its own tokens, `tokens_of`) or the
/// null-pointer form `(integer) &((T *) 0)->m.n…`.
pub fn container_shape<'v>(
    node: &'v Value,
    tokens_of: &dyn Fn(&Value) -> Option<Vec<String>>,
) -> Option<ContainerShape<'v>> {
    let cast = strip_parens(node);
    if !matches!(kind(cast), "CStyleCastExpr" | "ImplicitCastExpr")
        || cast["castKind"].as_str() != Some("BitCast")
    {
        return None;
    }
    let target = pointee(type_text(cast)?)?.to_owned();
    let difference = strip_parens(children(cast).first()?);
    if kind(difference) != "BinaryOperator" || difference["opcode"].as_str() != Some("-") {
        return None;
    }
    let [bytes, offset] = children(difference) else {
        return None;
    };
    if !byte_aligned(pointee(type_text(difference)?)?) {
        return None;
    }
    let bytes = strip_parens(bytes);
    if !matches!(kind(bytes), "CStyleCastExpr" | "ImplicitCastExpr")
        || !matches!(bytes["castKind"].as_str(), Some("BitCast" | "NoOp"))
    {
        return None;
    }
    let pointer = children(bytes).first()?;
    let (offset_record, offset_path) = offset_designator(offset, tokens_of)?;
    Some(ContainerShape {
        target,
        pointer,
        offset_record,
        offset_path,
    })
}

/// The record and member path an `offsetof` operand names.
fn offset_designator(
    node: &Value,
    tokens_of: &dyn Fn(&Value) -> Option<Vec<String>>,
) -> Option<(String, Vec<String>)> {
    let mut node = strip_parens(node);
    while kind(node) == "ImplicitCastExpr" && node["castKind"].as_str() == Some("IntegralCast") {
        node = strip_parens(children(node).first()?);
    }
    match kind(node) {
        "OffsetOfExpr" => {
            let tokens = tokens_of(node)?;
            let texts = tokens.iter().map(String::as_str).collect::<Vec<_>>();
            let ["__builtin_offsetof", "(", inner @ .., ")"] = texts.as_slice() else {
                return None;
            };
            let comma = inner.iter().position(|text| *text == ",")?;
            let (record, designator) = (&inner[..comma], &inner[comma + 1..]);
            if record.is_empty() || record.iter().any(|t| matches!(*t, "(" | ")" | "," | "*")) {
                return None;
            }
            let mut path = Vec::new();
            for (position, text) in designator.iter().enumerate() {
                if position % 2 == 0 {
                    if !crate::legacy_c::is_identifier(text) {
                        return None;
                    }
                    path.push((*text).to_owned());
                } else if *text != "." {
                    return None;
                }
            }
            (designator.len() % 2 == 1).then(|| (record.join(" "), path))
        }
        "CStyleCastExpr" | "ImplicitCastExpr"
            if node["castKind"].as_str() == Some("PointerToIntegral") =>
        {
            let address = strip_parens(children(node).first()?);
            if kind(address) != "UnaryOperator" || address["opcode"].as_str() != Some("&") {
                return None;
            }
            let (root, path) = member_path(children(address).first()?)?;
            let root = strip_parens(root);
            if !matches!(kind(root), "CStyleCastExpr" | "ImplicitCastExpr")
                || root["castKind"].as_str() != Some("NullToPointer")
            {
                return None;
            }
            Some((pointee(type_text(root)?)?.to_owned(), path))
        }
        _ => None,
    }
}

/// `R->m.n…` (one arrow, at the root, then `.` steps): the root pointer
/// expression and the member names. Anonymous members and any other step
/// refuse.
fn member_path(node: &Value) -> Option<(&Value, Vec<String>)> {
    let mut node = strip_parens(node);
    let mut path = Vec::new();
    loop {
        if kind(node) != "MemberExpr" {
            return None;
        }
        let name = node["name"].as_str().filter(|name| !name.is_empty())?;
        path.push(name.to_owned());
        let inner = strip_parens(children(node).first()?);
        if node["isArrow"].as_bool() == Some(true) {
            path.reverse();
            return Some((inner, path));
        }
        node = inner;
    }
}

/// Whether the declaration `id` is written anywhere in `function` other
/// than by its own initializer, or has its address taken.
fn written_or_addressed(function: &Value, id: &str) -> bool {
    fn names(node: &Value, id: &str) -> bool {
        let node = strip_parens(node);
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
    }
    fn walk(node: &Value, id: &str) -> bool {
        let hit = match kind(node) {
            "BinaryOperator" | "CompoundAssignOperator"
                if kind(node) == "CompoundAssignOperator"
                    || node["opcode"].as_str() == Some("=") =>
            {
                children(node).first().is_some_and(|left| names(left, id))
            }
            "UnaryOperator" => {
                matches!(node["opcode"].as_str(), Some("&" | "++" | "--"))
                    && children(node)
                        .first()
                        .is_some_and(|operand| names(operand, id))
            }
            _ => false,
        };
        hit || children(node).iter().any(|child| walk(child, id))
    }
    walk(function, id)
}

fn declaration_by_id<'v>(node: &'v Value, id: &str) -> Option<&'v Value> {
    if matches!(kind(node), "VarDecl" | "ParmVarDecl") && node["id"].as_str() == Some(id) {
        return Some(node);
    }
    children(node)
        .iter()
        .find_map(|child| declaration_by_id(child, id))
}

/// The base object a member address `&x->m…` names: `x` a parameter or
/// automatic local pointer, read once here.
fn member_address(node: &Value) -> Option<(String, String, String, Vec<String>)> {
    let mut node = strip_parens(node);
    while matches!(kind(node), "ImplicitCastExpr" | "CStyleCastExpr")
        && matches!(node["castKind"].as_str(), Some("NoOp" | "BitCast"))
    {
        node = strip_parens(children(node).first()?);
    }
    if kind(node) != "UnaryOperator" || node["opcode"].as_str() != Some("&") {
        return None;
    }
    let (root, path) = member_path(children(node).first()?)?;
    let root = strip_parens(root);
    if kind(root) != "ImplicitCastExpr" || root["castKind"].as_str() != Some("LValueToRValue") {
        return None;
    }
    let name = strip_parens(children(root).first()?);
    if kind(name) != "DeclRefExpr"
        || !matches!(
            name["referencedDecl"]["kind"].as_str(),
            Some("ParmVarDecl" | "VarDecl")
        )
    {
        return None;
    }
    let record = pointee(type_text(name)?)?.to_owned();
    Some((
        name["referencedDecl"]["id"].as_str()?.to_owned(),
        name["referencedDecl"]["name"].as_str()?.to_owned(),
        record,
        path,
    ))
}

/// The provenance of `container_of`'s pointer operand: established only
/// for `&x->m…` written in the operand, or a pointer local whose only write
/// is such an initializer while neither it nor `x` is otherwise written or
/// addressed. `B` is `(declaration id, name)` of `x`.
pub fn member_provenance(
    pointer: &Value,
    function: &Value,
) -> Provenance<(String, String), String, String> {
    if let Some((id, name, record, path)) = member_address(pointer) {
        return Provenance::MemberOf {
            base: (id, name),
            record,
            path,
        };
    }
    let mut node = strip_parens(pointer);
    if kind(node) == "ImplicitCastExpr" && node["castKind"].as_str() == Some("LValueToRValue") {
        let Some(inner) = children(node).first() else {
            return Provenance::Unestablished("unread-pointer");
        };
        node = strip_parens(inner);
    } else {
        return Provenance::Unestablished("unread-pointer");
    }
    if kind(node) != "DeclRefExpr" || node["referencedDecl"]["kind"].as_str() != Some("VarDecl") {
        return Provenance::Unestablished("not-a-member-address");
    }
    let Some(local) = node["referencedDecl"]["id"]
        .as_str()
        .and_then(|id| declaration_by_id(function, id))
    else {
        return Provenance::Unestablished("not-a-member-address");
    };
    if local["storageClass"].as_str().is_some() {
        return Provenance::Unestablished("not-automatic");
    }
    let Some(initializer) = children(local).first() else {
        return Provenance::Unestablished("not-a-member-address");
    };
    let Some((id, name, record, path)) = member_address(initializer) else {
        return Provenance::Unestablished("not-a-member-address");
    };
    // The respelled name must denote the one declaration the initializer
    // read wherever the `container_of` stands.
    if declarations_named(function, &name) != 1 {
        return Provenance::Unestablished("shadowed");
    }
    if written_or_addressed(function, local["id"].as_str().unwrap_or_default())
        || written_or_addressed(function, &id)
    {
        return Provenance::Unestablished("written");
    }
    Provenance::MemberOf {
        base: (id, name),
        record,
        path,
    }
}

// ---------------------------------------------------------------------------
// Flexible array member accesses
// ---------------------------------------------------------------------------

/// One element access through a flexible array member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexibleAccess {
    pub begin: Option<usize>,
    pub record: String,
    pub member: String,
    pub element: String,
}

/// Every `R->data` (or `r.data`) of an incomplete array member decayed for
/// an element access, in the function's order.
pub fn flexible_accesses(function: &Value) -> Vec<FlexibleAccess> {
    fn walk(node: &Value, out: &mut Vec<FlexibleAccess>) {
        if kind(node) == "MemberExpr"
            && let Some(text) = type_text(node)
            && let Some(element) = text.trim().strip_suffix("[]")
            && let Some(base) = children(node).first()
            && let Some(record_text) = type_text(base)
        {
            let record = if node["isArrow"].as_bool() == Some(true) {
                pointee(record_text).map(str::to_owned)
            } else {
                Some(record_text.trim().to_owned())
            };
            if let (Some(record), Some(member)) = (record, node["name"].as_str()) {
                out.push(FlexibleAccess {
                    begin: crate::typed_access_path::offset(&node["range"]["begin"]),
                    record,
                    member: member.to_owned(),
                    element: element.trim().to_owned(),
                });
            }
        }
        for child in children(node) {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(function, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Clang's numbers
// ---------------------------------------------------------------------------

const PROBE: &str = "__qcore_cg17_value_";

/// Clang's own evaluation of each integer constant expression, in one probe
/// (rule 11b's `clang_constant`, batched): an enumerator per expression at
/// the end of the unit. Any Clang error (a block-scope type, an incomplete
/// one) answers nothing for every expression: the caller's obligations stay
/// undecided.
pub fn clang_constants(
    source: &str,
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
    expressions: &[String],
) -> BTreeMap<String, i128> {
    if expressions.is_empty() {
        return BTreeMap::new();
    }
    let enumerators = expressions
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{PROBE}{i} = ({e})"))
        .collect::<Vec<_>>()
        .join(", ");
    let probe = format!("{source}\nenum __qcore_cg17_probe {{ {enumerators} }};\n");
    let Ok(declarations) = crate::typed_c_ast::ast_declarations(&probe, unit, PROBE) else {
        return BTreeMap::new();
    };
    fn collect(node: &Value, out: &mut BTreeMap<usize, i128>) {
        if kind(node) == "EnumConstantDecl"
            && let Some(index) = node["name"]
                .as_str()
                .and_then(|name| name.strip_prefix(PROBE))
                .and_then(|index| index.parse::<usize>().ok())
        {
            let mut value = children(node).first();
            while let Some(v) = value
                && kind(v) == "ImplicitCastExpr"
            {
                value = children(v).first();
            }
            if let Some(v) = value
                && kind(v) == "ConstantExpr"
                && let Some(number) = v["value"].as_str().and_then(|s| s.parse().ok())
            {
                out.insert(index, number);
            }
        }
        for child in children(node) {
            collect(child, out);
        }
    }
    let mut by_index = BTreeMap::new();
    for declaration in &declarations {
        collect(declaration, &mut by_index);
    }
    by_index
        .into_iter()
        .filter_map(|(i, v)| expressions.get(i).map(|e| (e.clone(), v)))
        .collect()
}

// ---------------------------------------------------------------------------
// The published report
// ---------------------------------------------------------------------------

/// The published alignment property's identity.
pub const ALIGNMENT_PROPERTY: &str = "layout.alignment.v1";
/// The layout facts' evidence version.
pub const EVIDENCE_VERSION: &str = "cg17-layout-extents.v1";
/// The premise every pointer-value base alignment rests on.
pub const POINTER_ALIGNED_PREMISE: &str = "P-CG17-POINTER-ALIGNED";

/// One alignment obligation with Clang's numbers and its decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlignmentObligation {
    pub line: usize,
    pub target: String,
    pub required: Option<u64>,
    pub base: Option<u64>,
    pub offset: Offset,
    pub decision: Alignment,
}

/// One flexible-array access with its member offset, element width and
/// remaining extent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlexibleFact {
    pub line: usize,
    pub record: String,
    pub member: String,
    pub offset: Option<u128>,
    pub width: Option<u128>,
    pub extent: FlexExtent,
}

/// The layout facts and the alignment property of one function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutReport {
    pub function: String,
    pub property: &'static str,
    pub evidence_version: &'static str,
    /// Premises the verdict rests on (named only when used).
    pub assumptions: Vec<&'static str>,
    pub alignment: Vec<AlignmentObligation>,
    pub verdict: PropertyVerdict,
    pub flexible: Vec<FlexibleFact>,
    pub vlas: Vec<VlaFact>,
}

fn line_of(source: &str, offset: Option<usize>) -> usize {
    offset.map_or(0, |offset| {
        source
            .as_bytes()
            .iter()
            .take(offset.min(source.len()))
            .filter(|byte| **byte == b'\n')
            .count()
            + 1
    })
}

/// The layout report of `function`, with an allocation size for each
/// flexible access (`allocation`: none is established by any current
/// contract, CG16's vocabulary).
pub fn layout_report(
    unit: &crate::legacy_c::C11TranslationUnit<'_>,
    function: &str,
    allocation: &dyn Fn(&FlexibleAccess) -> Option<u128>,
) -> Result<LayoutReport, String> {
    let ast = crate::typed_c_ast::function_ast(unit.source, unit, function)?;
    let casts = derived_casts(&ast);
    let flexible = flexible_accesses(&ast);
    let mut queries = casts.iter().flat_map(cast_queries).collect::<Vec<_>>();
    for access in &flexible {
        queries.push(format!(
            "__builtin_offsetof({}, {})",
            access.record, access.member
        ));
        queries.push(format!("sizeof({})", access.element));
    }
    queries.sort();
    queries.dedup();
    let numbers = clang_constants(unit.source, unit, &queries);
    let lookup = |query: &str| numbers.get(query).copied();
    let mut assumptions = Vec::new();
    let alignment = casts
        .iter()
        .map(|cast| {
            if matches!(cast.base, BaseAddress::PointerValue(_))
                && !assumptions.contains(&POINTER_ALIGNED_PREMISE)
            {
                assumptions.push(POINTER_ALIGNED_PREMISE);
            }
            let (required, base, offset, decision) = decide_cast(cast, &lookup);
            AlignmentObligation {
                line: line_of(unit.source, cast.begin),
                target: cast.target.clone(),
                required,
                base,
                offset,
                decision,
            }
        })
        .collect::<Vec<_>>();
    let verdict = alignment_verdict(
        &alignment
            .iter()
            .map(|obligation| obligation.decision)
            .collect::<Vec<_>>(),
    );
    let flexible = flexible
        .iter()
        .map(|access| {
            let offset = lookup(&format!(
                "__builtin_offsetof({}, {})",
                access.record, access.member
            ))
            .and_then(|v| u128::try_from(v).ok());
            let width =
                lookup(&format!("sizeof({})", access.element)).and_then(|v| u128::try_from(v).ok());
            let extent = match (offset, width) {
                (Some(offset), Some(width)) => flexible_extent(allocation(access), offset, width),
                _ => FlexExtent::Refused(FlexRefusal::SizeNotEstablished),
            };
            FlexibleFact {
                line: line_of(unit.source, access.begin),
                record: access.record.clone(),
                member: access.member.clone(),
                offset,
                width,
                extent,
            }
        })
        .collect();
    let mut vlas = Vec::new();
    collect_vlas(&ast, &ast, &mut vlas);
    Ok(LayoutReport {
        function: function.to_owned(),
        property: ALIGNMENT_PROPERTY,
        evidence_version: EVIDENCE_VERSION,
        assumptions,
        alignment,
        verdict,
        flexible,
        vlas,
    })
}

fn collect_vlas(node: &Value, function: &Value, out: &mut Vec<VlaFact>) {
    if let Some(fact) = vla_declaration(node, function) {
        out.push(fact);
    }
    for child in children(node) {
        collect_vlas(child, function, out);
    }
}

/// Every VLA declaration of a function (the census).
pub(crate) fn vla_facts(function: &Value) -> Vec<VlaFact> {
    let mut out = Vec::new();
    collect_vlas(function, function, &mut out);
    out
}

#[cfg(test)]
#[path = "../tests/unit/layout_extents.rs"]
mod tests;
