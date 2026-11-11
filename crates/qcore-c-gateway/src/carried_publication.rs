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

//! Card **MONKEY-VHOST-PROPERTY-CUSTODY** (gateway rule 12d): a publication
//! carries the owner's field-linked children into the container, and an
//! unlink of the owner hands them back.
//!
//! Rule 10d discharges a body-acquired object linked into a caller's (or a
//! static) container. Before this card an object that *owned* something
//! through a field link (O00 blocker B08: `ha->name = mk_string_dup(v)`) was
//! published with its children withdrawn (an unresolved effect, the call
//! refused), because an unlink of the owner could not hand them back: the
//! Monkey `mk_vhost_set_property` `Name` branch,
//!
//! ```c
//! ha = mk_mem_alloc(sizeof(struct mk_vhost_alias));
//! if (!ha) return -1;
//! ha->name = mk_string_dup(v);
//! ha->len  = strlen(v);
//! mk_list_add(&ha->_head, &vh->server_names);
//! ```
//!
//! was `Unknown` for that reason alone once `mk_string_dup` is declared.
//!
//! The rule decides each child the publication reaches through a field link
//! ([`publish_step`]):
//!
//! * a child that is exactly live, linked on every path (not a partial link,
//!   rule 11c), with no unresolved effect, not itself published, and owning
//!   no links or contained children of its own is **carried**: it becomes
//!   `ESCAPED` with its owner, which the container now holds;
//! * any other child **refuses**, an untracked one included (it may be a
//!   caller's object the store reached, whose custody no rule moves), and the
//!   publication keeps rule 10d's withdrawal byte for byte.
//!
//! An owner with contained children (a `resource_stored` relation) keeps the
//! withdrawal as well. The field links themselves are kept: they are what an
//! unlink reads.
//!
//! An unlink of a published owner decides each child it links
//! ([`unlink_step`]): a must-linked child that is exactly `ESCAPED` is handed
//! back (`LIVE`), exactly as the owner is; a child with an `ESCAPED` bit that
//! is partial or of mixed status refuses the unlink; any other child is left
//! as it is (it was never carried). Handing back is sound because nothing
//! else can move a carried child: a store into a published owner's member
//! refuses (its owner is not live, O00 B08), a release or consumption of an
//! escaped identity is a violation candidate, and a return of one is refused
//! or a violation. Releasing the owner after the unlink does not cascade
//! (P-B08-SHALLOW, unchanged), so a child still held at an exit is reported
//! as a leak.
//!
//! The Lean model is `QCore.CarriedPublication`
//! (`proof/QCore/CarriedPublication.lean`); [`PUBLISH_VECTORS`] and
//! [`UNLINK_VECTORS`] are shared with it.

use crate::legacy_c::{ESCAPED, LIVE};

/// What an owner's publication does to one child it links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublishStep {
    /// Exactly live, linked on every path, unencumbered: escapes with its
    /// owner.
    Carry,
    /// Anything else: the publication keeps rule 10d's withdrawal.
    Refuse,
}

impl PublishStep {
    #[cfg(test)]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Carry => "carry",
            Self::Refuse => "refuse",
        }
    }
}

/// The facts the flow holds about one child an owner links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChildFacts {
    /// The link may not hold on every path (rule 11c's partial link).
    pub(crate) partial: bool,
    /// The child's joined status bits; `None` when the flow tracks none.
    pub(crate) status: Option<u8>,
    /// The child carries an unresolved effect.
    pub(crate) unresolved: bool,
    /// The child is itself published into a container.
    pub(crate) published: bool,
    /// The child owns field links or contained children of its own.
    pub(crate) owns: bool,
}

/// The decision for one child an owner's publication reaches
/// (`QCore.CarriedPublication.publishStep`).
pub(crate) const fn publish_step(child: ChildFacts) -> PublishStep {
    match child.status {
        Some(LIVE) if !child.partial && !child.unresolved && !child.published && !child.owns => {
            PublishStep::Carry
        }
        _ => PublishStep::Refuse,
    }
}

/// What an unlink of a published owner does to one child it links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnlinkStep {
    /// Not carried (untracked, or never escaped with the owner): unchanged.
    Keep,
    /// Carried: handed back to the body with its owner.
    Restore,
    /// Possibly carried on some paths only, or of mixed status: the unlink
    /// refuses.
    Refuse,
}

impl UnlinkStep {
    #[cfg(test)]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Restore => "restore",
            Self::Refuse => "refuse",
        }
    }
}

/// The decision for one child an unlink of its published owner reaches
/// (`QCore.CarriedPublication.unlinkStep`).
pub(crate) const fn unlink_step(partial: bool, status: Option<u8>) -> UnlinkStep {
    match status {
        Some(ESCAPED) if !partial => UnlinkStep::Restore,
        Some(bits) if bits & ESCAPED != 0 => UnlinkStep::Refuse,
        _ => UnlinkStep::Keep,
    }
}

/// `(partial, tracked, live, released, escaped, contained, unresolved,
/// published, owns)` and the step, shared with
/// `QCore.CarriedPublication.publishVectors`.
#[cfg(test)]
pub(crate) type PublishVector = (
    (bool, bool, bool, bool, bool, bool, bool, bool, bool),
    &'static str,
);

#[cfg(test)]
pub(crate) const PUBLISH_VECTORS: &[PublishVector] = &[
    // Untracked: no rule says whose it is, so the withdrawal stands.
    (
        (
            false, false, false, false, false, false, false, false, false,
        ),
        "refuse",
    ),
    (
        (true, false, false, false, false, false, true, true, true),
        "refuse",
    ),
    // The `Name` branch: exactly live, must-linked, unencumbered.
    (
        (false, true, true, false, false, false, false, false, false),
        "carry",
    ),
    // Each encumbrance alone refuses.
    (
        (true, true, true, false, false, false, false, false, false),
        "refuse",
    ),
    (
        (false, true, true, false, false, false, true, false, false),
        "refuse",
    ),
    (
        (false, true, true, false, false, false, false, true, false),
        "refuse",
    ),
    (
        (false, true, true, false, false, false, false, false, true),
        "refuse",
    ),
    // Any status but exactly live refuses.
    (
        (false, true, true, true, false, false, false, false, false),
        "refuse",
    ),
    (
        (false, true, true, false, true, false, false, false, false),
        "refuse",
    ),
    (
        (false, true, true, false, false, true, false, false, false),
        "refuse",
    ),
    (
        (false, true, false, true, false, false, false, false, false),
        "refuse",
    ),
    (
        (false, true, false, false, true, false, false, false, false),
        "refuse",
    ),
    (
        (false, true, false, false, false, true, false, false, false),
        "refuse",
    ),
    (
        (false, true, false, false, false, false, false, false, false),
        "refuse",
    ),
];

/// `(partial, tracked, live, released, escaped)` and the step, shared with
/// `QCore.CarriedPublication.unlinkVectors`.
#[cfg(test)]
pub(crate) type UnlinkVector = ((bool, bool, bool, bool, bool), &'static str);

#[cfg(test)]
pub(crate) const UNLINK_VECTORS: &[UnlinkVector] = &[
    ((false, false, false, false, false), "keep"),
    ((true, false, false, false, false), "keep"),
    ((false, true, false, false, true), "restore"),
    ((true, true, false, false, true), "refuse"),
    ((false, true, true, false, true), "refuse"),
    ((false, true, false, true, true), "refuse"),
    ((false, true, true, false, false), "keep"),
    ((true, true, true, false, false), "keep"),
    ((false, true, false, true, false), "keep"),
];

#[cfg(test)]
#[path = "../tests/unit/carried_publication.rs"]
mod tests;
