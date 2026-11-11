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

//! Card **NULL-SAFETY-PROPERTY** (gateway rule 14n, USER DECISION
//! 2026-10-10, decision 1): the custody hook of the moved NULL question.
//!
//! A typed scalar access dereferences its bases and its chains' roots, and
//! the dereference obligation (W43-INDIRECT-ACCESS-GENERAL, W44-INDIRECT-
//! CHAIN-ACCESS) refuses a **possibly-null** binding. Decision 1 moves that
//! question to the null-dereference property: the dereference is the check
//! there. So a binding that is **target-free, unknown, possibly null and the
//! identity derivation** in a state that is not widened (exactly what a
//! pointer-valued path read and rule 14b's interior alias bind) is read
//! **non-null at this dereference**, and every other part of the obligation
//! is decided exactly as for a non-null binding: a bound root still needs
//! rule 11d's per-path premise, a tracked, mixed, widened or literal-NULL
//! binding still refuses. The reading is recorded as the named premise
//! `null-deref:<function>:<line>:<variable>` ([`crate::null_premise`]) on
//! the summary and in the consultation log, so the verdict lists it.
//!
//! Nothing else changes: the binding itself stays possibly null (a release,
//! a publication or a custody position through it decides as before, rule
//! 14b's `the_alias_is_never_known_non_null`), and a store through it keeps
//! W42's refusal of a store base that is not one live identity.

use std::collections::BTreeSet;

use crate::bound_untracked::{BindingClass, Premise, decide, rooted_at};
use crate::indirect_chain_access::{AccessChain, ChainView, LinkBinding, chain_resolves};

/// Whether `base`'s binding is the target-free, possibly-null borrowed alias
/// decision 1 reads non-null at a dereference. `QCore.NullDeref.premised`.
pub(super) fn premised(state: &super::AbstractState, base: &str) -> bool {
    state.pointers.get(base).is_some_and(|pointer| {
        crate::null_premise::premised_shape(
            pointer.targets.is_empty(),
            pointer.unknown,
            pointer.may_be_null,
            pointer.derivation.is_identity(),
            state.widened,
        )
    })
}

/// The dereference obligation of a scalar access's base, with decision 1's
/// reading: the base resolves as before, or it is premised (and the premise
/// is noted).
pub(super) fn base_resolves(
    function: &str,
    line: usize,
    state: &super::AbstractState,
    base: &str,
) -> bool {
    if super::indirect_scalar_base_resolves(state, base) {
        return true;
    }
    if premised(state, base) {
        crate::null_premise::note(function, line, base);
        return true;
    }
    false
}

/// The state read with one premised root non-null.
struct Premised<'a> {
    state: &'a super::AbstractState,
    root: &'a str,
}

impl ChainView for Premised<'_> {
    fn binding(&self, place: &str) -> Option<LinkBinding<'_>> {
        let mut binding = self.state.binding(place)?;
        if place == self.root {
            binding.may_be_null = false;
        }
        Some(binding)
    }

    fn exactly_live(&self, site: usize) -> bool {
        self.state.exactly_live(site)
    }

    fn field_link(&self, owner: usize, field: &str) -> Option<&BTreeSet<usize>> {
        self.state.field_link(owner, field)
    }

    fn bound_untracked(&self, root: &str) -> bool {
        if root != self.root {
            return self.state.bound_untracked(root);
        }
        decide(
            true,
            BindingClass::TargetFree,
            self.state
                .pointers
                .keys()
                .any(|place| rooted_at(place, root)),
            self.state.unmodeled,
        ) == Premise::Holds
    }
}

/// The chain obligation with decision 1's reading of a premised root.
pub(super) fn chain_resolves_premised(
    function: &str,
    line: usize,
    state: &super::AbstractState,
    chain: &AccessChain,
) -> Result<(), String> {
    let refusal = match chain_resolves(state, chain) {
        Ok(()) => return Ok(()),
        Err(refusal) => refusal,
    };
    if chain.dereferenced_root
        && premised(state, &chain.root)
        && chain_resolves(
            &Premised {
                state,
                root: &chain.root,
            },
            chain,
        )
        .is_ok()
    {
        crate::null_premise::note(function, line, &chain.root);
        return Ok(());
    }
    Err(refusal)
}
