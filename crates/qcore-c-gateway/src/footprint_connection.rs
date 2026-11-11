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

//! Card **CG18-FOOTPRINT-CONNECTION** (gateway rule 11k): source places
//! connected to the core footprint model ([`qcore_core::disjoint_footprint`]).
//!
//! The custody gateway's guarantee is custody, not byte safety, so most of
//! the gap analysis's footprint entries are premises a custody verdict does
//! not rest on. This module does three things and claims nothing else.
//!
//! ## Member pairs (M20/M21, the CG13 amendment)
//!
//! The elaborator refuses an unordered write/write or write/read pair on two
//! paths as not proved disjoint (rule 10s). [`members_disjoint`] admits
//! exactly one kind of such pair: two places that lie, by their own
//! structure, inside two **different members of one struct object**, the same
//! base expression designating that object on both sides. The core model
//! decides it: the two members become a layout contract read from Clang's own
//! record layout (`-fdump-record-layouts-complete`), one allocation generation
//! holds the object, and the pair is admitted only when the core checker
//! admits `AssertDisjoint` on the two field handles (premise
//! `DeclaredRecordLayout`, the layout extraction of trust boundary 28, already
//! stated by rule 10w).
//!
//! * The bases are compared **structurally** (the Clang nodes, not the
//!   place spelling, which erases indices): same kinds, the same referenced
//!   declarations, the same member steps and the same literal values. A write
//!   to anything a base reads is its own unordered pair, decided separately.
//! * Above the divergent member only `.` member steps and parentheses may
//!   occur, so each place is a sub-object of its member's storage. A
//!   subscript, a dereference or an `->` hop above it refuses: its target is
//!   not inside the member, or lies there only under an in-bounds premise
//!   this module does not take.
//! * A union member, an anonymous member, an unnamed record, a bit-field of
//!   an unread unit and two bit-fields of one storage unit refuse (the last
//!   is CG13's `Q661`).
//! * Two different base objects (two pointers, two allocations) refuse: the
//!   core model does not assume two allocation identities disjoint (`Q662`,
//!   M21), and the gateway takes no allocator premise.
//!
//! ## Generations (M20)
//!
//! Rule 11a names three identities per acquisition site: current, previous
//! and the older summary. [`core_identity`] maps each to the core model:
//! current and previous are two distinct allocation identities (one object
//! per execution each, so never two generations of one allocation), the
//! summary has none (it may stand for many objects, so no footprint is
//! derived from it), and a reallocation is the core `Reallocate` of the
//! current identity (every earlier handle stale, `Q652`, as the gateway's
//! `UnknownAlias` on the old pointer). [`replay`] turns a rule-11a event
//! sequence into a core program; its vectors are shared with
//! `QCore.DisjointFootprint`'s generation-agreement section. No liveness
//! check of the custody flow changes.
//!
//! ## Premises (M21, M22, M26)
//!
//! [`BOUNDARY_PREMISES`] names every premise this connection states and which
//! route rests on it; the core certificate carries the same names.

#[cfg(test)]
use std::collections::BTreeMap;

use qcore_core::disjoint_footprint::{
    ElementShape, FieldLayout, FieldStorage, FootprintEvent, FootprintProgram, LayoutContract,
    check_disjoint_footprint_program,
};
use serde_json::Value;

#[cfg(test)]
use crate::loop_generations::{self, Generation};
use crate::typed_c_ast::{children, kind};
use crate::union_bitfield::RecordFacts;
#[cfg(test)]
use qcore_core::disjoint_footprint::AllocationId;

/// The premises this connection names, with the routes that state them
/// (read by the tests and the report; no route consults it).
#[cfg(test)]
pub(crate) const BOUNDARY_PREMISES: &[(&str, &str)] = &[
    (
        "DeclaredRecordLayout",
        "route G member pairs (rule 11k): Clang's record layout is the target's (trust boundary 28)",
    ),
    (
        "P-DISTINCT-ALLOCATIONS-DISJOINT",
        "core only: two allocations declared fresh by an allocator contract; no gateway verdict takes it",
    ),
    (
        "P-EFFECTIVE-TYPE",
        "core views; route G reads a punned scalar load as custody-neutral under the no-undefined-behaviour reading (AOM-POISON-NO-UB) and claims no effective-type result",
    ),
    (
        "P-BYTE-REPRESENTATION",
        "core byte transfers; route G's Borrowed memset/memcpy declarations and W43's character-typed store (constraint 10a's frame)",
    ),
    (
        "P-RESTRICT",
        "no route relies on restrict: route T refuses it, route G ignores the qualifier and derives no disjointness from it",
    ),
];

// ---------------------------------------------------------------------------
// Generations (M20).
// ---------------------------------------------------------------------------

/// The core identity of a rule-11a identity, given the core allocation the
/// site's current and previous generations were assigned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum CoreIdentity {
    /// One object: a core allocation identity.
    Allocation(AllocationId),
    /// The older summary: no core identity, no footprint.
    Summary,
    /// Not yet produced on this path.
    Absent,
}

/// One rule-11a event over acquisition sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum SiteEvent {
    /// The site's node runs: a fresh current generation (shifting a present
    /// one to previous, and a previous one into the summary).
    Acquire(usize),
    /// The current generation is reallocated in place of itself.
    Reallocate(usize),
}

/// A site's current core allocation with its generation, and its previous
/// generation's core allocation.
#[cfg(test)]
type SiteAllocations = (Option<(AllocationId, u32)>, Option<AllocationId>);

/// The replay state: each site's current and previous core allocation.
#[derive(Clone, Debug, Default)]
#[cfg(test)]
pub(crate) struct Replay {
    next: AllocationId,
    sites: BTreeMap<usize, SiteAllocations>,
    pub(crate) program: FootprintProgram,
}

/// Map a rule-11a identity to its core identity in the replay state.
#[cfg(test)]
pub(crate) fn core_identity(replay: &Replay, identity: usize) -> CoreIdentity {
    let site = loop_generations::base(identity);
    let Some((current, previous)) = replay.sites.get(&site) else {
        return CoreIdentity::Absent;
    };
    match loop_generations::generation(identity) {
        Generation::Current => current.map_or(CoreIdentity::Absent, |(alloc, _)| {
            CoreIdentity::Allocation(alloc)
        }),
        Generation::Previous => previous.map_or(CoreIdentity::Absent, CoreIdentity::Allocation),
        Generation::Older => CoreIdentity::Summary,
    }
}

/// The core generation a handle derived now of the current identity names.
#[cfg(test)]
pub(crate) fn current_generation(replay: &Replay, site: usize) -> Option<(AllocationId, u32)> {
    replay.sites.get(&site).and_then(|(current, _)| *current)
}

/// Apply one rule-11a event, appending its core events.
#[cfg(test)]
pub(crate) fn replay(state: &mut Replay, event: SiteEvent) {
    match event {
        SiteEvent::Acquire(site) => {
            state.next += 1;
            let fresh = state.next;
            let entry = state.sites.entry(site).or_default();
            // Shift: current becomes previous; the old previous joins the
            // summary and keeps its core allocation, unnamed.
            entry.1 = entry.0.map(|(alloc, _)| alloc);
            entry.0 = Some((fresh, 1));
            state.program.events.push(FootprintEvent::Allocate {
                alloc: fresh,
                generation: 1,
                shape: ElementShape::Scalar { size: 1 },
                count: 8,
            });
        }
        SiteEvent::Reallocate(site) => {
            if let Some((Some((alloc, generation)), _)) = state.sites.get_mut(&site) {
                *generation += 1;
                state.program.events.push(FootprintEvent::Reallocate {
                    alloc: *alloc,
                    generation: *generation,
                    count: 8,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Member pairs.
// ---------------------------------------------------------------------------

/// One direct member: its name, first byte, and first and last bit when it
/// is a bit-field.
pub(crate) type MemberOffset = (String, u32, Option<(u32, u32)>);

/// Clang's layout of one record's direct members: name → (first byte,
/// `Some((first bit, last bit))` for a bit-field), and the record size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MemberLayout {
    pub(crate) size: u32,
    pub(crate) members: Vec<MemberOffset>,
}

/// Read `struct NAME`'s direct members from Clang's record layout dump.
/// `None` unless exactly one dump names the record and every direct member
/// line parses.
pub(crate) fn parse_member_layout(dump: &str, name: &str) -> Option<MemberLayout> {
    let header = format!("struct {name}");
    let mut found = None;
    for section in dump.split("*** Dumping AST Record Layout").skip(1) {
        let mut lines = section.lines().filter(|line| !line.trim().is_empty());
        let first = lines.next()?;
        if first.split_once('|').map(|(_, rest)| rest.trim()) != Some(header.as_str()) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        let mut layout = MemberLayout::default();
        for line in lines {
            let (offset, rest) = line.split_once('|')?;
            let rest = rest.strip_prefix(' ')?;
            if let Some(size) = rest.trim().strip_prefix("[sizeof=") {
                layout.size = size.split(',').next()?.parse().ok()?;
                break;
            }
            // A direct member is indented by exactly two spaces.
            if !(rest.starts_with("  ") && !rest.starts_with("   ")) {
                continue;
            }
            let field = rest.split_whitespace().last()?.to_owned();
            let offset = offset.trim();
            let (byte, bits) = match offset.split_once(':') {
                Some((byte, bits)) => {
                    let (low, high) = bits.split_once('-')?;
                    (byte, Some((low.parse().ok()?, high.parse().ok()?)))
                }
                None => (offset, None),
            };
            layout.members.push((field, byte.parse().ok()?, bits));
        }
        found = Some(layout);
    }
    found.filter(|layout| layout.size > 0)
}

/// The core field layout of one direct member: an ordinary member spans from
/// its offset to the next member's first byte (or the record's end), which
/// covers its storage and possibly padding (a superset, so disjointness is
/// never over-claimed); a bit-field spans the bytes its bits occupy.
fn field_layout(layout: &MemberLayout, name: &str, field: u32, unit: u32) -> Option<FieldLayout> {
    let &(_, byte, bits) = layout
        .members
        .iter()
        .find(|(member, _, _)| member == name)?;
    match bits {
        None => {
            let end = layout
                .members
                .iter()
                .map(|(_, start, _)| *start)
                .filter(|start| *start > byte)
                .min()
                .unwrap_or(layout.size);
            Some(FieldLayout {
                field,
                byte_offset: byte,
                byte_size: end.checked_sub(byte)?,
                storage: FieldStorage::Addressable,
            })
        }
        Some((first, last)) => {
            let low = byte * 8 + first;
            let high = byte * 8 + last;
            if high < low {
                return None;
            }
            let start = low / 8;
            let end = high / 8 + 1;
            Some(FieldLayout {
                field,
                byte_offset: start,
                byte_size: end - start,
                storage: FieldStorage::BitfieldUnit {
                    unit,
                    bit_offset: low - start * 8,
                    bit_width: high - low + 1,
                },
            })
        }
    }
}

/// The core model's decision on two members of one object of `struct NAME`.
pub(crate) fn core_members_disjoint(
    layout: &MemberLayout,
    left: &str,
    right: &str,
    same_unit: bool,
) -> bool {
    if left == right || same_unit {
        return false;
    }
    let (Some(one), Some(other)) = (
        field_layout(layout, left, 0, 0),
        field_layout(layout, right, 1, 1),
    ) else {
        return false;
    };
    let program = FootprintProgram {
        layout_contracts: vec![LayoutContract {
            record: 0,
            size: layout.size,
            fields: vec![one, other],
        }],
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
                field: 0,
            },
            FootprintEvent::DeriveField {
                handle: 21,
                base: 10,
                field: 1,
            },
            FootprintEvent::AssertDisjoint {
                left: 20,
                right: 21,
            },
        ],
    };
    check_disjoint_footprint_program(&program).is_ok()
}

/// Whether two Clang nodes designate one object in one evaluation: the same
/// structure, declarations, member steps and literal values. Anything else
/// (a call, an increment, an assignment, a string) is never the same.
pub(crate) fn same_designation(left: &Value, right: &Value) -> bool {
    if kind(left) != kind(right) {
        return false;
    }
    let same_children = || {
        let (a, b) = (children(left), children(right));
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same_designation(x, y))
    };
    match kind(left) {
        "ParenExpr" | "ArraySubscriptExpr" => same_children(),
        "ImplicitCastExpr" => left["castKind"] == right["castKind"] && same_children(),
        "MemberExpr" => {
            left["isArrow"] == right["isArrow"]
                && left["referencedMemberDecl"].is_string()
                && left["referencedMemberDecl"] == right["referencedMemberDecl"]
                && same_children()
        }
        "UnaryOperator" => {
            left["opcode"].as_str() == Some("*")
                && right["opcode"].as_str() == Some("*")
                && same_children()
        }
        "DeclRefExpr" => {
            left["referencedDecl"]["id"].is_string()
                && left["referencedDecl"]["id"] == right["referencedDecl"]["id"]
        }
        "IntegerLiteral" => left["value"].is_string() && left["value"] == right["value"],
        _ => false,
    }
}

/// The member steps a place lies inside by its own structure: from the place
/// down through parentheses and `.` steps, and the first `->` step, which
/// ends the walk. Outermost first.
fn enclosing_members(place: &Value) -> Vec<&Value> {
    let mut steps = Vec::new();
    let mut cursor = place;
    loop {
        match kind(cursor) {
            "ParenExpr" => match children(cursor).first() {
                Some(child) => cursor = child,
                None => return steps,
            },
            "MemberExpr" => {
                steps.push(cursor);
                if cursor["isArrow"].as_bool() == Some(true) {
                    return steps;
                }
                match children(cursor).first() {
                    Some(child) => cursor = child,
                    None => return steps,
                }
            }
            _ => return steps,
        }
    }
}

/// Every node under `node` whose place spelling is `spelling`.
fn places<'a>(node: &'a Value, spelling: &str, out: &mut Vec<&'a Value>) {
    if matches!(kind(node), "MemberExpr" | "ParenExpr")
        && crate::elaboration::path_place(node).is_ok_and(|place| place.spelling() == spelling)
    {
        out.push(node);
    }
    for child in children(node) {
        places(child, spelling, out);
    }
}

/// Card CG18 hook for the elaborator: whether the unordered path places
/// `one` and `other` of the full expression `node` lie inside two different
/// members of one struct object, decided by the core model. `false` keeps
/// the elaborator's refusal.
pub(crate) fn members_disjoint(
    node: &Value,
    facts: &RecordFacts,
    one: &str,
    other: &str,
    layout_dump: &dyn Fn() -> Option<String>,
) -> bool {
    let decided = decide_members(node, facts, one, other, layout_dump);
    if crate::typed_access_path::parity_enabled() {
        crate::typed_access_path::record_row(serde_json::json!({
            "position": "footprint-member-pair",
            "offset": crate::typed_access_path::offset(&node["range"]["begin"]),
            "one": one,
            "other": other,
            "decision": match decided {
                Ok(()) => "disjoint",
                Err(reason) => reason,
            },
        }));
    }
    decided.is_ok()
}

fn decide_members(
    node: &Value,
    facts: &RecordFacts,
    one: &str,
    other: &str,
    layout_dump: &dyn Fn() -> Option<String>,
) -> Result<(), &'static str> {
    if one == other {
        return Err("one-spelling");
    }
    let (mut left, mut right) = (Vec::new(), Vec::new());
    places(node, one, &mut left);
    places(node, other, &mut right);
    // The outermost node of each spelling; parentheses inside one place
    // repeat it, so keep the nodes no other listed node contains.
    let (left, right) = (outermost(left), outermost(right));
    let ([left], [right]) = (left.as_slice(), right.as_slice()) else {
        return Err("ambiguous-place");
    };
    for l in enclosing_members(left) {
        for r in enclosing_members(right) {
            let (Some(lb), Some(rb)) = (children(l).first(), children(r).first()) else {
                continue;
            };
            if l["isArrow"] != r["isArrow"] || !same_designation(lb, rb) {
                continue;
            }
            let (Some(lf), Some(rf)) = (
                l["referencedMemberDecl"].as_str(),
                r["referencedMemberDecl"].as_str(),
            ) else {
                return Err("unread-member");
            };
            if lf == rf {
                // One member on both sides: the places may overlap.
                return Err("same-member");
            }
            if facts.union_owner.contains_key(lf) || facts.union_owner.contains_key(rf) {
                return Err("union-member");
            }
            // Both members of one complete `struct`, read from its own
            // declaration (never from the base's type spelling, which a
            // typedef hides).
            let (Some((lrec, ln)), Some((rrec, rn))) =
                (facts.struct_members.get(lf), facts.struct_members.get(rf))
            else {
                return Err("not-a-struct-member");
            };
            if lrec != rrec {
                return Err("two-records");
            }
            if ln.is_empty() || rn.is_empty() {
                return Err("anonymous-member");
            }
            let unit = |field: &str| facts.units.get(field);
            let same_unit = match (unit(lf), unit(rf)) {
                (Some(None), _) | (_, Some(None)) => return Err("unread-unit"),
                (Some(Some(a)), Some(Some(b))) => a.record == b.record && a.ordinal == b.ordinal,
                _ => false,
            };
            if same_unit {
                return Err("shared-unit");
            }
            let Some(name) = facts.names.get(lrec) else {
                return Err("unnamed-record");
            };
            let Some(layout) = layout_dump().and_then(|dump| parse_member_layout(&dump, name))
            else {
                return Err("layout-unread");
            };
            return if core_members_disjoint(&layout, ln, rn, false) {
                Ok(())
            } else {
                Err("core-refused")
            };
        }
    }
    Err("no-common-object")
}

/// The listed nodes no other listed node contains: parentheses inside one
/// place repeat its spelling, so only the outermost node of each place stays.
fn outermost(nodes: Vec<&Value>) -> Vec<&Value> {
    let mut kept: Vec<&Value> = Vec::new();
    for node in nodes {
        if !kept.iter().any(|seen| contains(seen, node)) {
            kept.push(node);
        }
    }
    kept
}

/// Whether `inner` is `outer` or a node under it (by identity).
fn contains(outer: &Value, inner: &Value) -> bool {
    std::ptr::eq(outer, inner) || children(outer).iter().any(|child| contains(child, inner))
}

#[cfg(test)]
#[path = "../tests/unit/footprint_connection.rs"]
mod tests;
