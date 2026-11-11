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

//! Card **CG11-POINTER-VALUED-STORES** (gateway rule 10v): the flow decision
//! of one admitted pointer store, the `PointerStore` arm of
//! [`super::transfer`]. The classification and the decision table are
//! [`crate::pointer_store`]; the Lean model is `QCore.PointerStore`.
//!
//! Every premise is decided on the state at the node. When one fails the node
//! is exactly its **fallback**, the gate's own refusal, byte for byte (the
//! refusal, its write frame and the refused hand-on rule), so a store that
//! stays refused behaves as it did before this card.
//!
//! 1. **The owner.** The destination's chain resolves
//!    ([`crate::indirect_chain_access::chain_owner`]) with its root *and* the
//!    object the member lives in both untracked. A tracked owner may hold an
//!    owning field (only O04-B08's field-link store rewrites one), and a
//!    tracked root's untracked child is not a caller's object the premise
//!    speaks of: both refuse. A member of a never-bound record object of this
//!    frame, selected with `.` only, carries no chain and is untracked by the
//!    reader's premise.
//! 2. **The value.** A name of function-pointer type must be bound to no
//!    identity on any reaching path (no binding, or a known target-free one).
//! 3. **The old value.** No binding of a place rooted at the destination's
//!    root carries a target or is unknown: the projection holds nothing about
//!    the member's old value that the store could drop.
//! 4. **The store.** The access (the destination's per-hop obligations, and an
//!    address value's) is applied by the ordinary transfer, then every binding
//!    of a place rooted at the root is forgotten: its fact no longer describes
//!    the object. No status, marker, field link or containment changes.

use std::collections::BTreeSet;

use crate::indirect_chain_access::{AccessChain, Owner, chain_owner};
use crate::pointer_store::{Decision, OwnerClass, PointerStore, ValueClass, decide};

/// The owner class of the destination (`QCore.PointerStore.ownerClass`).
fn owner_class(state: &super::AbstractState, store: &PointerStore) -> OwnerClass {
    let Some(chain) = &store.owner else {
        return OwnerClass::Untracked;
    };
    let root = AccessChain {
        links: Vec::new(),
        ..chain.clone()
    };
    match (chain_owner(state, &root), chain_owner(state, chain)) {
        (Ok(Owner::Untracked), Ok(Owner::Untracked)) => OwnerClass::Untracked,
        (Ok(_), Ok(_)) => OwnerClass::Tracked,
        _ => OwnerClass::Unresolved,
    }
}

/// Whether `place` is a place (not the plain name) rooted at `root`.
pub(super) fn rooted_at(place: &str, root: &str) -> bool {
    if place == root {
        return false;
    }
    if crate::may_alias::place_root(place) == Some(root) {
        return true;
    }
    let trimmed = place.trim_start_matches(['*', '(', '&', ' ']);
    let prefixed = trimmed.len() != place.len();
    trimmed.strip_prefix(root).is_some_and(|rest| {
        rest.starts_with("->")
            || rest.starts_with('.')
            || rest.starts_with('[')
            || (prefixed && (rest.is_empty() || rest.starts_with(')')))
    })
}

/// The class a stored value was classified with (the classifier admits only
/// neutral ones).
fn value_class(code: &str) -> ValueClass {
    match code {
        "null" => ValueClass::Null,
        "string" => ValueClass::StringLiteral,
        "function" => ValueClass::Function,
        "function-pointer-name" => ValueClass::FunctionPointerName,
        "self-address" => ValueClass::SelfAddress,
        "interior-address" => ValueClass::InteriorAddress,
        _ => ValueClass::Other,
    }
}

pub(super) fn transfer(
    function: &str,
    node: &super::LegacyCfgNode,
    state: super::AbstractState,
    diagnostics: &mut BTreeSet<super::LegacyDiagnostic>,
    base: &dyn Fn(
        &super::LegacyCfgNode,
        super::AbstractState,
        &mut BTreeSet<super::LegacyDiagnostic>,
    ) -> super::AbstractState,
) -> super::AbstractState {
    let super::LegacyInstruction::PointerStore {
        store,
        access,
        fallback,
    } = &node.instruction
    else {
        return base(node, state, diagnostics);
    };
    // 2. The value.
    let carries = store.name.as_ref().is_some_and(|name| {
        state
            .pointers
            .get(name)
            .is_some_and(|pointer| !pointer.targets.is_empty() || pointer.unknown)
    });
    // 1. The owner.
    let owner = owner_class(&state, store);
    let mut decision = decide(value_class(store.value), carries, owner);
    // 3. The old value.
    if decision == Decision::Admit
        && state.pointers.iter().any(|(place, pointer)| {
            rooted_at(place, &store.root) && (!pointer.targets.is_empty() || pointer.unknown)
        })
    {
        decision = Decision::OwnedField;
    }
    crate::pointer_store::record_flow(function, node.line, store, owner, decision);
    if decision != Decision::Admit {
        return base(&super::with_instruction(node, fallback), state, diagnostics);
    }
    // 4. The store.
    let mut state = base(&super::with_instruction(node, access), state, diagnostics);
    state
        .pointers
        .retain(|place, _| !rooted_at(place, &store.root));
    state
}

/// Card CG11: what either reading of a pointer store may write (its access's
/// frame, or its fallback refusal's).
pub(super) fn readings(
    instruction: &super::LegacyInstruction,
) -> Option<[&super::LegacyInstruction; 2]> {
    match instruction {
        super::LegacyInstruction::PointerStore {
            access, fallback, ..
        } => Some([access.as_ref(), fallback.as_ref()]),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_pointer_store.rs"]
mod tests;
