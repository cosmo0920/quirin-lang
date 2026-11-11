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

//! Card **CG10-BOUND-UNTRACKED-ROOTS** (gateway rule 11d): the flow side of
//! the per-path bound-untracked root premise. The decision is
//! [`crate::bound_untracked::decide`]; this adapter reads its four facts off
//! one [`super::AbstractState`] and records the census.
//!
//! * the binding class of the root ([`binding_class`]);
//! * whether a place rooted at it is bound;
//! * whether some reaching path was refused or left an unresolved effect
//!   (`unmodeled`, joined by disjunction).
//!
//! Parity only, [`record_census`] reads the settled incoming state of every
//! node whose chains carry a root the never-bound premise does not cover
//! (`position: "bound-untracked-root"`). It decides nothing.

use serde_json::json;

use crate::bound_untracked::{BindingClass, Premise, decide, rooted_at};
use crate::indirect_chain_access::{AccessChain, chain_owner};
use crate::typed_access_path::{parity_enabled, record_row};

/// The class of `root`'s binding in `state`.
pub(super) fn binding_class(state: &super::AbstractState, root: &str) -> BindingClass {
    let Some(pointer) = state.pointers.get(root) else {
        return BindingClass::Absent;
    };
    if !pointer.targets.is_empty() {
        BindingClass::Tracked
    } else if pointer.may_be_null {
        BindingClass::Nullable
    } else if pointer.unknown
        && !state.widened
        && !matches!(pointer.derivation, super::Derivation::Mixed)
    {
        BindingClass::TargetFree
    } else {
        BindingClass::Unresolved
    }
}

/// The premise for an eligible or ineligible `root` in `state`.
pub(super) fn premise(state: &super::AbstractState, root: &str, eligible: bool) -> Premise {
    decide(
        eligible,
        binding_class(state, root),
        state.pointers.keys().any(|place| rooted_at(place, root)),
        state.unmodeled,
    )
}

/// The chains of one node that carry a dereferenced root the never-bound
/// premise does not cover.
fn chains(instruction: &super::LegacyInstruction) -> Vec<&AccessChain> {
    let all: Vec<&AccessChain> = match instruction {
        super::LegacyInstruction::IndirectScalarAccess { chains, .. } => chains.iter().collect(),
        super::LegacyInstruction::PointerStore { store, .. } => store.owner.iter().collect(),
        _ => Vec::new(),
    };
    all.into_iter()
        .filter(|chain| chain.dereferenced_root && !chain.root_never_bound)
        .collect()
}

/// Parity only: one row per settled node and chain root the never-bound
/// premise does not cover, with the premise's facts and answer and the
/// chain's answer with and without it.
pub(super) fn record_census(cfg: &super::LegacyCfg, incoming: &[Option<super::AbstractState>]) {
    if !parity_enabled() {
        return;
    }
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        for chain in chains(&node.instruction) {
            let answer = premise(state, &chain.root, chain.root_bound_untracked);
            let without = chain_owner(&Without(state), chain).is_ok();
            let with = chain_owner(&With(state), chain).is_ok();
            record_row(json!({
                "function": cfg.function,
                "line": node.line,
                "texts": [],
                "position": "bound-untracked-root",
                "root": chain.root,
                "links": chain.links,
                "member": chain.member,
                "eligible": chain.root_bound_untracked,
                "binding": binding_class(state, &chain.root).code(),
                "rooted": state.pointers.keys().any(|place| rooted_at(place, &chain.root)),
                "path_refused": state.unmodeled,
                "premise": answer.code(),
                "store": matches!(node.instruction, super::LegacyInstruction::PointerStore { .. }),
                "resolves_without": without,
                "resolves_with": with,
                "old_route": "flow",
                "old_admitted": without,
                "new_admitted": with,
                "disagreement": without != with,
            }));
        }
    }
}

/// Parity only: why the lexical closure binds each name of `function` it
/// binds (`position: "bound-untracked-closure"`), for the census's classes.
pub(super) fn record_closure(
    function: &str,
    body: &[&str],
    protected_formals: &std::collections::BTreeSet<String>,
    contracts: &qcore_schemas::contract::ResourceContracts,
    variadic_tails: &std::collections::BTreeSet<String>,
    bases: &crate::borrowed_actual::UntrackedBases,
) {
    if !parity_enabled() {
        return;
    }
    let reasons = crate::borrowed_actual::closure_reasons(
        body,
        protected_formals,
        contracts,
        variadic_tails,
        bases,
    );
    record_row(json!({
        "function": function,
        "line": 0,
        "texts": [],
        "position": "bound-untracked-closure",
        "reasons": reasons,
        "old_route": "closure",
        "old_admitted": true,
        "new_admitted": true,
        "disagreement": false,
    }));
}

/// The state read without the premise (every other answer of the view).
struct Without<'a>(&'a super::AbstractState);

/// The state read with the premise.
struct With<'a>(&'a super::AbstractState);

impl crate::indirect_chain_access::ChainView for Without<'_> {
    fn binding(&self, place: &str) -> Option<crate::indirect_chain_access::LinkBinding<'_>> {
        self.0.binding(place)
    }
    fn exactly_live(&self, site: usize) -> bool {
        self.0.exactly_live(site)
    }
    fn field_link(&self, owner: usize, field: &str) -> Option<&std::collections::BTreeSet<usize>> {
        self.0.field_link(owner, field)
    }
}

impl crate::indirect_chain_access::ChainView for With<'_> {
    fn binding(&self, place: &str) -> Option<crate::indirect_chain_access::LinkBinding<'_>> {
        self.0.binding(place)
    }
    fn exactly_live(&self, site: usize) -> bool {
        self.0.exactly_live(site)
    }
    fn field_link(&self, owner: usize, field: &str) -> Option<&std::collections::BTreeSet<usize>> {
        self.0.field_link(owner, field)
    }
    fn bound_untracked(&self, root: &str) -> bool {
        premise(self.0, root, true) == Premise::Holds
    }
}
