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

//! Card **UNTRACKED-BASE-NULL-JOIN-DISCHARGE** (gateway rule 83): a binding
//! that is the null constant on some reaching path is never rule 14n's
//! premised alias.
//!
//! Rule 14n reads a target-free, `unknown`, possibly-null identity binding
//! non-null at a dereference under the named premise
//! `null-deref:F:LINE:VAR`, and never premises a *known* binding: the null
//! constant (`p = 0`, lowered to `Clear`: no target, known, possibly null)
//! refuses, so `const struct s *p = 0; return p->n + 1;` is `Unknown`
//! (`unknown_alias:p`). The pointer join forgets that knowledge. Where one
//! path binds `p` to the null constant and the other leaves it unbound (a
//! copy of an unbound formal takes rule 62's custody-free arm) or binds it to
//! a pointer read, [`super::AbstractState::join`] makes the joined binding
//! `unknown` and possibly null, which is exactly the premised shape. So
//! `const struct s *p = 0; if (f) p = f; return p->n + 1;` was `Safe` under
//! a premise the function's own null constant refutes on one path.
//!
//! This module keeps that knowledge beside the bindings, in
//! [`super::AbstractState::null_on_path`]: the names that may hold a
//! definitely-null binding on some path reaching this point.
//!
//! * **Join** ([`join`]): the union of both sides' sets, plus every name
//!   either side binds definitely null, restricted to names the joined state
//!   still binds.
//! * **Transfer** ([`transfer`]): a rebinding of a name removes it (the new
//!   value is the statement's own: an acquisition, a call result, a pointer
//!   read, a copy, the null constant again); a copy of a name in the set
//!   carries it to the destination.
//! * **Reading** ([`refuses_premise`]): rule 14n's premise refuses a name in
//!   the set, so the base and chain obligations refuse it (`unknown_alias`),
//!   as they refuse the definitely-null binding itself.
//!
//! Nothing else reads the set. A binding no reaching path held as the null
//! constant, a null test's non-null arm (its binding is not possibly null,
//! so it needs no premise), every tracked binding and every reading other
//! than the premise decide exactly as before, so the change only refuses
//! more (`QCore.NullJoinDischarge`).

use std::collections::BTreeSet;

use super::{AbstractPointer, AbstractState, LegacyInstruction};

/// The null constant's binding: no target, known, possibly null. The shape
/// [`super::Derivation::join`] already calls definitely null.
/// `QCore.NullJoinDischarge.definitelyNull`.
pub(super) fn definitely_null(pointer: &AbstractPointer) -> bool {
    pointer.targets.is_empty() && !pointer.unknown && pointer.may_be_null
}

/// The join hook, run after the pointer join: `before` is the receiving side
/// as it was, `incoming` the other side, `state` the joined result.
pub(super) fn join(state: &mut AbstractState, before: &AbstractState, incoming: &AbstractState) {
    let mut names = before
        .null_on_path
        .union(&incoming.null_on_path)
        .cloned()
        .collect::<BTreeSet<_>>();
    for side in [before, incoming] {
        names.extend(
            side.pointers
                .iter()
                .filter(|(_, pointer)| definitely_null(pointer))
                .map(|(name, _)| name.clone()),
        );
    }
    names.retain(|name| state.pointers.contains_key(name));
    state.null_on_path = names;
}

/// The transfer hook, run after a node's transfer: `before` is the set on
/// entry to the node, `output` the node's output state.
pub(super) fn transfer(
    instruction: &LegacyInstruction,
    before: &BTreeSet<String>,
    output: &mut AbstractState,
) {
    let mut names = before.clone();
    for name in super::rebound_names(instruction) {
        names.remove(name);
    }
    if let LegacyInstruction::Alias {
        destination,
        source,
    } = instruction
    {
        // `rebound_names` keeps plain names only; a copy into a member
        // place rebinds that place too.
        names.remove(destination);
        if before.contains(source) {
            names.insert(destination.clone());
        }
    }
    names.retain(|name| output.pointers.contains_key(name));
    output.null_on_path = names;
}

/// Whether rule 14n's premise must refuse `base` here: some reaching path
/// held it as the null constant. `QCore.NullJoinDischarge.refusesPremise`.
pub(crate) fn refuses_premise(state: &AbstractState, base: &str) -> bool {
    state.null_on_path.contains(base)
}

#[cfg(test)]
#[path = "../tests/unit/legacy_null_join.rs"]
mod tests;
