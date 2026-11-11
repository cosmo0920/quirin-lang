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

//! Card **B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED** (gateway rule 11c): what a
//! B08 field link that holds on some paths only does to its child.
//!
//! An admitted aggregate field store records a *field link* `owner -> field
//! -> child` (O00 blocker B08). Releasing the owner does not discharge the
//! child (premise P-B08-SHALLOW); escaping the owner carries a live linked
//! child with it. Before this card the join of two paths recorded every child
//! linked on one side only as an unresolved effect, and nothing else: no
//! diagnostic reported it, `report_leaks` skipped the child, and the escape
//! cascade escaped it anyway. `if (n) b->item = it; else k_release(it);
//! k_drop(b);` was `Safe` although the arm that linked `it` leaks it, and
//! `if (n) b->item = it; return b;` was `Safe` although the other arm leaks
//! it.
//!
//! The repair keeps the child's obligation and records instead which links
//! may not hold on every path (*partial* links, [`join_link`]):
//!
//! * the join adds **no** unresolved effect, so a one-sided child the owner's
//!   release does not discharge is reported exactly as in the straight-line
//!   form;
//! * an escape of the owner decides each child it reaches ([`escape_step`]):
//!   it escapes a child only when the child is exactly live and linked on
//!   every path, and refuses (`Unknown`) whenever a possibly-live child is
//!   not carried on every path or its status is mixed;
//! * a read through a partial link refuses (it no longer denotes one
//!   identity on every path);
//! * as a floor for every other rule, a settled exit on a path no refusal
//!   reached that still withholds a possibly-live identity with an
//!   unresolved effect reports a refusal ([`exit_refuses`]).
//!
//! The Lean model is `QCore.FieldLinkJoin` (`proof/QCore/FieldLinkJoin.lean`);
//! [`LINK_VECTORS`], [`ESCAPE_VECTORS`] and [`EXIT_VECTORS`] are shared with it.

use crate::legacy_c::{ESCAPED, LIVE};

/// Whether one path's field records a child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Link {
    /// Not linked on any path reaching here.
    Absent,
    /// Linked on every path reaching here.
    Must,
    /// Linked on some paths reaching here and not on others.
    Partial,
}

impl Link {
    #[cfg(test)]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Must => "must",
            Self::Partial => "partial",
        }
    }
}

/// The join of two paths' links (`QCore.FieldLinkJoin.joinLink`).
pub(crate) const fn join_link(one: Link, other: Link) -> Link {
    match (one, other) {
        (Link::Absent, Link::Absent) => Link::Absent,
        (Link::Must, Link::Must) => Link::Must,
        _ => Link::Partial,
    }
}

/// What an escape of an owner does to one child it reaches through a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EscapeStep {
    /// Untracked, or already escaped on every path: nothing changes.
    Keep,
    /// Exactly live and linked on every path: it escapes with its owner.
    Escape,
    /// Live on no path (released, or contained and so its container's): the
    /// escape owes nothing for it, and the member's hazard is withdrawn as
    /// before (an unresolved effect, no report). A contained child later
    /// detached live meets the settled-exit floor.
    Withdraw,
    /// Possibly live, and either not carried on every path or of mixed
    /// status: the escape refuses (`Unknown`) and the child keeps an
    /// unresolved effect.
    Refuse,
}

impl EscapeStep {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Escape => "escape",
            Self::Withdraw => "withdraw",
            Self::Refuse => "refuse",
        }
    }
}

/// The decision for one child an escape reaches
/// (`QCore.FieldLinkJoin.escapeStep`). `status` is the child's joined status
/// bits, `None` when the flow tracks no status for it.
pub(crate) const fn escape_step(partial: bool, status: Option<u8>) -> EscapeStep {
    match status {
        None => EscapeStep::Keep,
        Some(ESCAPED) => EscapeStep::Keep,
        Some(LIVE) if !partial => EscapeStep::Escape,
        Some(bits) if bits & LIVE != 0 => EscapeStep::Refuse,
        Some(_) => EscapeStep::Withdraw,
    }
}

/// Whether a settled exit refuses: it withholds a possibly-live identity with
/// an unresolved effect (`withheld`) on a path no refusal reached
/// (`modeled`). On an unmodeled path a refusal the path passed already makes
/// the function at least `Unknown` (`QCore.FieldLinkJoin.exitRefuses`).
pub(crate) const fn exit_refuses(withheld: bool, modeled: bool) -> bool {
    withheld && modeled
}

/// The status bits the tables below name.
#[cfg(test)]
pub(crate) const RELEASED_BIT: u8 = crate::legacy_c::RELEASED;
#[cfg(test)]
pub(crate) const CONTAINED_BIT: u8 = crate::legacy_c::CONTAINED;

/// `(one, other, joined)`, shared with `QCore.FieldLinkJoin.linkVectors`.
#[cfg(test)]
pub(crate) const LINK_VECTORS: &[(&str, &str, &str)] = &[
    ("absent", "absent", "absent"),
    ("absent", "must", "partial"),
    ("absent", "partial", "partial"),
    ("must", "absent", "partial"),
    ("must", "must", "must"),
    ("must", "partial", "partial"),
    ("partial", "absent", "partial"),
    ("partial", "must", "partial"),
    ("partial", "partial", "partial"),
];

/// `(partial, tracked, live, released, escaped, contained)` and the step,
/// shared with `QCore.FieldLinkJoin.escapeVectors`.
#[cfg(test)]
pub(crate) type EscapeVector = ((bool, bool, bool, bool, bool, bool), &'static str);

#[cfg(test)]
pub(crate) const ESCAPE_VECTORS: &[EscapeVector] = &[
    ((false, false, false, false, false, false), "keep"),
    ((true, false, false, false, false, false), "keep"),
    ((false, true, true, false, false, false), "escape"),
    ((true, true, true, false, false, false), "refuse"),
    ((false, true, true, true, false, false), "refuse"),
    ((true, true, true, true, false, false), "refuse"),
    ((false, true, true, false, true, false), "refuse"),
    ((false, true, true, false, false, true), "refuse"),
    ((false, true, false, false, true, false), "keep"),
    ((true, true, false, false, true, false), "keep"),
    ((false, true, false, true, false, false), "withdraw"),
    ((true, true, false, true, false, false), "withdraw"),
    ((false, true, false, true, true, false), "withdraw"),
    ((false, true, false, false, false, true), "withdraw"),
    ((true, true, false, false, false, true), "withdraw"),
    ((false, true, false, true, false, true), "withdraw"),
];

/// `(withheld, modeled, refuses)`, shared with
/// `QCore.FieldLinkJoin.exitVectors`.
#[cfg(test)]
pub(crate) const EXIT_VECTORS: &[(bool, bool, bool)] = &[
    (false, false, false),
    (false, true, false),
    (true, false, false),
    (true, true, true),
];

#[cfg(test)]
#[path = "../tests/unit/field_link_join.rs"]
mod tests;
