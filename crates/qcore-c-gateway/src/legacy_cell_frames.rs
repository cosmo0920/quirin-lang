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

//! Card **CELL-PROTOCOL-RELEASE-WINDOW-FRAMES** (gateway rule 38g, grammar
//! G05k, contract specification §C04.6.4): the route-G host of callee frames
//! (`rule_38_descriptor_cells::frame`).
//!
//! USER DECISION 2026-11-04 (verbatim, on the options (a) callee frame
//! declarations, (b) a member-actual narrowing, (c) keep the refusal): "(a)
//! Callee frames".
//!
//! Two hooks, each the identity in an environment that declares no frame:
//!
//! 1. [`settle_frames`] (after the parser builds a body's CFG): for every
//!    contracted callee the body calls (nested calls included) with a
//!    declared frame over a record (`ResourceContracts::cell_frames`), the
//!    frame's standing in this unit, read from Clang's AST of the unit
//!    ([`read_body`]) by the package's [`frame::body_check`]. It travels on
//!    the CFG (`LegacyCfg::cell_frames`), so certificate rederivation, which
//!    re-parses the same source under the same environment, reproduces it.
//! 2. [`frame_held_call`] / [`frame_hold_description`]: the refusal of a call
//!    a declared, unestablished frame holds for review
//!    (`awaiting-human-review`); `legacy_verdict::is_held_call` reads it as
//!    `Pending`.
//!
//! The reading itself (rule 38g, in rule 38c's `touch`) is
//! `legacy_descriptor_cells.rs`'s: only where a call would refuse a record
//! that does not keep its protocol.
//!
//! **What the body reader reads** ([`read_body`]). Every member access with
//! its record type's tag (`p->m`, `s.m`), every direct callee, and, as
//! unread constructs that leave the frame unestablished: a call through a
//! pointer, a pointer reinterpreted as anything but a record or `void`
//! pointer, an integer read as a pointer, a whole-record read or store, a
//! scalar read through a pointer (`*p`, `p[i]` not over a named array),
//! `offsetof`, inline assembly, `__builtin_bit_cast` and atomic builtins.
//! Premises stated, not proved (trust boundary 28): Clang's AST of the
//! configured unit; C11 6.5p7 (an `int` cell is read or written only through
//! an lvalue that names its member, a whole-record access, or a reinterpreted
//! pointer, each of which the reader sees) and 6.5.6p8 (pointer arithmetic
//! stays inside the object it points into).

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::ResourceContracts;
use rule_38_descriptor_cells::frame::{self, Body, Standing};
use serde_json::Value;

use super::{AWAITING_HUMAN_REVIEW, LegacyCfg, LegacyInstruction};
use crate::typed_c_ast::{children, kind};

/// A declared frame's standing in this unit, as the CFG carries it (an
/// undeclared frame is absent from the table).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum FrameStanding {
    Checked,
    Accepted,
    Declared,
    Refuted,
}

impl FrameStanding {
    #[must_use]
    pub const fn package(self) -> Standing {
        match self {
            Self::Checked => Standing::Checked,
            Self::Accepted => Standing::Accepted,
            Self::Declared => Standing::Declared,
            Self::Refuted => Standing::Refuted,
        }
    }

    const fn from_package(standing: Standing) -> Option<Self> {
        match standing {
            Standing::Checked => Some(Self::Checked),
            Standing::Accepted => Some(Self::Accepted),
            Standing::Declared => Some(Self::Declared),
            Standing::Refuted => Some(Self::Refuted),
            Standing::Absent => None,
        }
    }
}

/// What a body's CFG carries for rule 38g.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CellFrames {
    /// Per callee, per record, the standing of the callee's declared frame.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub standings: BTreeMap<String, BTreeMap<String, FrameStanding>>,
    /// Per record with a declared protocol, every cell path its protocols
    /// state (`ch_events[0]`, `event.fd`): a call handed only these numbers
    /// of the record is rule 38c's close, handed no pointer.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cells: BTreeMap<String, BTreeSet<String>>,
}

impl CellFrames {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.standings.is_empty() && self.cells.is_empty()
    }

    /// Fold another half of one body (`legacy_recursion`).
    pub fn extend(&mut self, other: Self) {
        for (callee, records) in other.standings {
            self.standings.entry(callee).or_default().extend(records);
        }
        for (record, cells) in other.cells {
            self.cells.entry(record).or_default().extend(cells);
        }
    }
}

/// The standing of a call running `callees` over `record`'s protocol: the
/// weakest of theirs, a callee without a declared frame being absent.
#[must_use]
pub fn call_standing<'a>(
    frames: &CellFrames,
    callees: impl IntoIterator<Item = &'a str>,
    record: &str,
) -> Standing {
    frame::join(callees.into_iter().map(|callee| {
        frames
            .standings
            .get(callee)
            .and_then(|records| records.get(record))
            .map_or(Standing::Absent, |standing| standing.package())
    }))
}

/// Whether a path of `cfg` can hold a protocol fact: a guard test, a
/// protocol store or close, or a cell-bearing call (the only places one is
/// introduced). Every other CFG carries no frame table, so it serializes and
/// reads Clang's AST as before.
fn may_hold_a_fact(cfg: &LegacyCfg) -> bool {
    !cfg.cell_tests.is_empty()
        || cfg.nodes.iter().any(|node| match &node.instruction {
            LegacyInstruction::CellProtocolStep { .. } => true,
            LegacyInstruction::ContractCall { retention, .. } => {
                retention.as_ref().is_some_and(|r| r.cells.is_some())
            }
            _ => false,
        })
}

/// Hook 1: every declared protocol's cells, and the standing of every
/// declared frame of a callee `cfg` calls, for a CFG that can hold a fact.
pub(super) fn settle_frames(cfg: &mut LegacyCfg, contracts: &ResourceContracts) {
    if !may_hold_a_fact(cfg) {
        return;
    }
    let specs = super::descriptor_cells_route::protocol_specs(contracts);
    for (record, specs) in &specs {
        cfg.cell_frames.cells.insert(
            record.clone(),
            specs
                .iter()
                .flat_map(|spec| spec.cells.iter().map(|(cell, _)| cell.clone()))
                .collect(),
        );
    }
    let declared = contracts.cell_frames();
    if declared.is_empty() {
        return;
    }
    let mut callees = BTreeSet::new();
    for node in &cfg.nodes {
        if let LegacyInstruction::ContractCall {
            function, nested, ..
        } = &node.instruction
        {
            callees.insert(function.clone());
            callees.extend(nested.iter().map(|call| call.function.clone()));
        }
    }
    let accepted = contracts.cell_frames_accepted();
    for (record, framed) in &declared {
        let protocols = specs
            .get(record)
            .map(|specs| specs.iter().map(|spec| spec.package()).collect::<Vec<_>>())
            .unwrap_or_default();
        let members = frame::protocol_members(&protocols);
        for callee in callees.iter().filter(|callee| framed.contains(*callee)) {
            let check = frame::body_check(&unit_body, callee, record, &members);
            if let Some(standing) =
                FrameStanding::from_package(frame::standing(true, check, accepted))
            {
                cfg.cell_frames
                    .standings
                    .entry(callee.clone())
                    .or_default()
                    .insert(record.clone(), standing);
            }
        }
    }
}

/// One body of the analysed unit, as the frame check reads it; `None` when
/// the unit holds no single definition of `name` or Clang did not read it.
fn unit_body(name: &str) -> Option<Body> {
    crate::typed_access_path::with_indirect_call_unit(|unit| unit.function(name).map(read_body))
        .flatten()
}

/// The fixed tail of a frame hold's refusal.
const HOLD_TAIL: &str = " rests on its declared frame, which awaits human review";

/// Hook 2: the refusal of a call held by a declared frame of `callee` over
/// `record` that this unit does not establish.
#[must_use]
pub fn frame_hold_description(callee: &str, record: &str) -> String {
    format!(
        "call to `{callee}` reaching a `struct {record}` that does not keep its cell protocol{HOLD_TAIL} ({AWAITING_HUMAN_REVIEW})"
    )
}

/// The callee a frame hold's refusal names, if `description` is one.
#[must_use]
pub fn frame_held_call(description: &str) -> Option<&str> {
    let rest = description.strip_prefix("call to `")?;
    let (callee, rest) = rest.split_once("` reaching a `struct ")?;
    let (record, tail) = rest.split_once("` that does not keep its cell protocol")?;
    let expected = format!("{HOLD_TAIL} ({AWAITING_HUMAN_REVIEW})");
    (tail == expected
        && !callee.is_empty()
        && !record.is_empty()
        && !callee.contains('`')
        && !record.contains('`'))
    .then_some(callee)
}

/// The type Clang gives `node`, desugared where it says.
fn type_of(node: &Value) -> Option<&str> {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
}

fn without_qualifiers(mut ty: &str) -> &str {
    loop {
        let trimmed = ty.trim();
        let next = ["const ", "volatile ", "restrict ", "_Atomic "]
            .iter()
            .find_map(|q| trimmed.strip_prefix(q));
        match next {
            Some(rest) => ty = rest,
            None => return trimmed,
        }
    }
}

/// The record tag a member access reads: the base's type, through its
/// pointer for `->`.
fn record_tag(ty: &str, arrow: bool) -> Option<String> {
    let ty = if arrow {
        let star = ty.rfind('*')?;
        &ty[..star]
    } else {
        ty
    };
    let ty = without_qualifiers(ty);
    if ty.contains('*') || ty.contains('[') || ty.contains('(') {
        return None;
    }
    ty.strip_prefix("struct ")
        .or_else(|| ty.strip_prefix("union "))
        .map(|tag| tag.trim().to_owned())
        .filter(|tag| !tag.is_empty() && !tag.contains(' '))
}

/// A record (or union) value, not a pointer or an array.
fn is_record_value(ty: &str) -> bool {
    let ty = without_qualifiers(ty);
    (ty.starts_with("struct ") || ty.starts_with("union "))
        && !ty.contains('*')
        && !ty.contains('[')
}

/// A scalar value: neither a pointer, a record, an array nor a function.
fn is_scalar_value(ty: &str) -> bool {
    let ty = without_qualifiers(ty);
    !ty.contains('*')
        && !ty.contains('[')
        && !ty.contains('(')
        && !ty.starts_with("struct ")
        && !ty.starts_with("union ")
        && ty != "void"
}

/// A pointer to a record, a union or `void`.
fn points_to_record_or_void(ty: &str) -> bool {
    let Some(star) = ty.rfind('*') else {
        return false;
    };
    let pointee = without_qualifiers(&ty[..star]);
    pointee == "void"
        || ((pointee.starts_with("struct ") || pointee.starts_with("union "))
            && !pointee.contains('*'))
}

/// The function a call's callee expression names directly.
fn direct_callee(node: &Value) -> Option<String> {
    match kind(node) {
        "DeclRefExpr" => (node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
            .then(|| node["referencedDecl"]["name"].as_str().map(str::to_owned))
            .flatten(),
        "ImplicitCastExpr" | "ParenExpr" => children(node).first().and_then(direct_callee),
        _ => None,
    }
}

/// Read one function definition's body for the frame check.
#[must_use]
pub fn read_body(definition: &Value) -> Body {
    let mut body = Body::default();
    for child in children(definition) {
        if kind(child) != "ParmVarDecl" {
            walk(child, &mut body);
        }
    }
    body
}

fn walk(node: &Value, body: &mut Body) {
    let ty = type_of(node).unwrap_or("");
    match kind(node) {
        "MemberExpr" => {
            let arrow = node["isArrow"].as_bool().unwrap_or(false);
            let base = children(node).first();
            match (
                base.and_then(type_of)
                    .and_then(|base_type| record_tag(base_type, arrow)),
                node["name"].as_str(),
            ) {
                (Some(tag), Some(member)) => body.accesses.push((tag, member.to_owned())),
                _ => body
                    .unread
                    .push("a member of a record type not read".to_owned()),
            }
        }
        "CallExpr" => {
            let parts = children(node);
            match parts.first().and_then(direct_callee) {
                Some(callee) => body.calls.push(callee),
                None => body.unread.push("a call through a pointer".to_owned()),
            }
            for argument in parts.iter().skip(1) {
                walk(argument, body);
            }
            return;
        }
        "ImplicitCastExpr" | "CStyleCastExpr" => match node["castKind"].as_str() {
            Some("BitCast") if !points_to_record_or_void(ty) => {
                body.unread.push("a pointer reinterpreted".to_owned());
            }
            Some("IntegralToPointer") => {
                body.unread.push("an integer read as a pointer".to_owned());
            }
            Some("LValueToRValue") if is_record_value(ty) => {
                body.unread.push("a whole-record read".to_owned());
            }
            _ => {}
        },
        "BinaryOperator" | "CompoundAssignOperator" if is_record_value(ty) => {
            body.unread.push("a whole-record store".to_owned());
        }
        "UnaryOperator" if node["opcode"].as_str() == Some("*") && is_scalar_value(ty) => {
            body.unread
                .push("a scalar read through a pointer".to_owned());
        }
        "ArraySubscriptExpr" if is_scalar_value(ty) => {
            let named_array = children(node).first().is_some_and(|base| {
                let inner = if kind(base) == "ImplicitCastExpr"
                    && base["castKind"].as_str() == Some("ArrayToPointerDecay")
                {
                    children(base).first()
                } else {
                    None
                };
                inner.is_some_and(|inner| {
                    matches!(kind(inner), "MemberExpr" | "DeclRefExpr")
                        && type_of(inner).is_some_and(|t| t.contains('['))
                })
            });
            if !named_array {
                body.unread
                    .push("an element read through a pointer".to_owned());
            }
        }
        "OffsetOfExpr" | "GCCAsmStmt" | "MSAsmStmt" | "BuiltinBitCastExpr" | "AtomicExpr" => {
            body.unread.push(kind(node).to_owned());
        }
        _ => {}
    }
    for child in children(node) {
        walk(child, body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_reads_back_and_nothing_else_does() {
        let held = frame_hold_description("mk_list_del", "flb_input_instance");
        assert_eq!(frame_held_call(&held), Some("mk_list_del"));
        assert!(held.ends_with("(awaiting-human-review)"));
        assert_eq!(
            frame_held_call(&held.replace("awaits human review", "is checked")),
            None
        );
        assert_eq!(
            frame_held_call(&super::super::hold_description("mk_list_del")),
            None
        );
    }

    #[test]
    fn the_type_readers() {
        assert_eq!(
            record_tag("struct flb_input_instance *", true).as_deref(),
            Some("flb_input_instance")
        );
        assert_eq!(
            record_tag("struct mk_list *const", true).as_deref(),
            Some("mk_list")
        );
        assert_eq!(
            record_tag("const struct mk_event", false).as_deref(),
            Some("mk_event")
        );
        assert_eq!(record_tag("struct mk_list **", true), None);
        assert!(is_record_value("struct flb_output_instance"));
        assert!(!is_record_value("struct flb_output_instance *"));
        assert!(is_scalar_value("int"));
        assert!(!is_scalar_value("struct mk_list *"));
        assert!(points_to_record_or_void("void *"));
        assert!(points_to_record_or_void("struct mk_list *"));
        assert!(!points_to_record_or_void("char *"));
        assert!(!points_to_record_or_void("int *"));
    }
}
