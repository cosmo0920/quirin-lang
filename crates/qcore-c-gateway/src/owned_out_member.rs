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

//! Card **CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT** (gateway rule 72): an owned
//! value a callee delivers through an out slot whose actual is a member of
//! an object the caller supplied.
//!
//! ## The gap
//!
//! `return cmt_mpack_consume_string_tag(reader, &opts->ns);` hands the callee
//! the address of a member of an object reached from a formal, and the callee
//! stores a fresh allocation through it (`*output_buffer =
//! cfl_sds_create_size(len)`). Contract specification §C04.13 reads an out
//! slot only at an actual `&NAME`, rule W44 reads no address of a
//! pointer-typed member at any position, and the lexical gate refuses the
//! statement (`indirect memory access is outside the resource projection`).
//! Round 4 of CG10-INDIRECT-ACCESS-SOLE-RESIDUALS counted 13 functions that
//! this refusal alone blocks.
//!
//! ## The reading
//!
//! The call is the store `*slot = fresh`, made by the callee: an owned value
//! written into the member path `R->F…`. Where the object holding the member
//! is the caller's (an untracked owner reached from a caller-supplied root,
//! [`caller_roots`]), the delivered custody is that object's: it escapes this
//! frame exactly as a field-linked child escapes with an escaping owner
//! (O04-B08's `cascade_field_links_on_escape`), and the old value of the slot
//! is not this body's obligation (premise **P-CG11-CALLER-SLOT**, rule 10v).
//! So the call is read with the slot as data (the callee's contract under a
//! minted name whose slot position is `Borrowed`, [`minted_contract`]), the
//! slot's chain obligation is kept, and the store is decided on the flow by
//! rule 10v's owner premise ([`decide`]): an untracked owner admits, a tracked
//! (frame-owned) owner, an unresolved one, or a rooted binding that carries a
//! target refuses with the gate's own refusal.
//!
//! The lowering is a [`crate::legacy_c::LegacyInstruction::PointerStore`] of
//! value class [`OWNED_OUT`], so rule 10v's summary gate holds: a body with
//! such a store publishes no derived summary, and no caller reads the
//! delivery as `Borrowed`. Rule 70's retention summary refuses a body with
//! one ([`crate::member_store_retention`]).
//!
//! The Lean model is `QCore.OwnedOutMemberSlot`; [`DECISION_VECTORS`] and
//! [`ROOT_VECTORS`] are shared with it.

use std::collections::BTreeSet;

use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

use crate::indirect_chain_access::AccessChain;
use crate::pointer_store::{Decision, OwnerClass};

/// The value code a rule-72 store carries
/// ([`crate::pointer_store::ValueClass::OwnedOut`]).
pub(crate) const OWNED_OUT: &str = "owned-out";

/// The prefix of the minted name under which a callee's contract is read with
/// its out slot as data.
pub(crate) const MINTED_PREFIX: &str = "qcore_owned_out_member__";

/// The minted name of `callee`.
pub(crate) fn minted_name(callee: &str) -> String {
    format!("{MINTED_PREFIX}{callee}")
}

/// The contract `callee` is read with when its out slot is handed a member
/// path, and that slot's position: `None` unless the contract in force is
///
/// * a plain contract (the function map) with exactly one `Out` position, or
/// * a return-code disposition (§C04.13, not withdrawn) whose classes are
///   `Out` or `Borrowed` (the class leaves the slot unwritten) at exactly one
///   position, `Out` in at least one class, and agree everywhere else;
///
/// and the contract with that position read `Borrowed` is the neutral one:
/// every other formal `Borrowed`, a `None` result, no kind, container,
/// counted-user or link clause. Every class then delivers either fresh
/// custody or nothing through the slot, so where the slot's object is the
/// caller's the classes need no decision: whatever the call returns, the
/// frame owes nothing for the slot.
pub(crate) fn minted_contract(
    contracts: &ResourceContracts,
    callee: &str,
) -> Option<(ResourceFunctionContract, usize)> {
    let neutralised = |contract: &ResourceFunctionContract, slot: usize| {
        let mut minted = contract.clone();
        minted.parameters[slot] = ResourceParameterMode::Borrowed;
        minted
    };
    let (minted, slot) = if let Some(contract) = contracts.functions().get(callee) {
        let slots = contract
            .parameters
            .iter()
            .enumerate()
            .filter(|(_, mode)| matches!(mode, ResourceParameterMode::Out { .. }))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [slot] = slots.as_slice() else {
            return None;
        };
        if !contract.parameter_kind(*slot).is_heap() {
            return None;
        }
        (neutralised(contract, *slot), *slot)
    } else {
        let relational = contracts.relational().get(callee)?;
        if relational.withdrawn.is_some() || relational.classes.len() < 2 {
            return None;
        }
        let arity = relational.classes[0].contract.parameters.len();
        let mut slot = None;
        for index in 0..arity {
            let modes = relational
                .classes
                .iter()
                .map(|class| class.contract.parameters.get(index).copied())
                .collect::<Vec<_>>();
            let any_out = modes
                .iter()
                .any(|mode| matches!(mode, Some(ResourceParameterMode::Out { .. })));
            if !any_out {
                continue;
            }
            let all_slot = modes.iter().all(|mode| {
                matches!(
                    mode,
                    Some(ResourceParameterMode::Out { .. } | ResourceParameterMode::Borrowed)
                )
            });
            if !all_slot || slot.replace(index).is_some() {
                return None;
            }
        }
        let slot = slot?;
        let minted = neutralised(&relational.classes[0].contract, slot);
        if relational.classes.iter().any(|class| {
            class.contract.parameters.len() != arity
                || !class.contract.parameter_kind(slot).is_heap()
                || neutralised(&class.contract, slot) != minted
        }) {
            return None;
        }
        (minted, slot)
    };
    let neutral = ResourceFunctionContract {
        parameters: vec![ResourceParameterMode::Borrowed; minted.parameters.len()],
        result: ResourceReturnMode::None,
        ..ResourceFunctionContract::default()
    };
    (minted == neutral).then_some((minted, slot))
}

/// The environment with every callee the stream names that [`minted_contract`]
/// reads installed under its minted name; `None` (the identity) when there is
/// none.
pub(crate) fn minted_environment(
    tokens: &[crate::legacy_c::Token],
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let mut minted = Vec::new();
    let mut seen = BTreeSet::new();
    for token in tokens {
        if !seen.insert(token.text.as_str()) {
            continue;
        }
        if !contracts.functions().contains_key(&token.text)
            && !contracts.relational().contains_key(&token.text)
        {
            continue;
        }
        if let Some((contract, _)) = minted_contract(contracts, &token.text) {
            minted.push((minted_name(&token.text), contract));
        }
    }
    if minted.is_empty() {
        return None;
    }
    let mut environment = contracts.clone();
    for (name, contract) in minted {
        environment.insert_contract(name, contract);
    }
    Some(environment)
}

/// A slot actual `& R -> F ( -> G )* ( . H )*`, fused: its root, the members
/// selected and then dereferenced (the chain's links) and the final member
/// path (`status.message`). `None` for every other spelling: a first step
/// `.` (a member of this frame's own object), an index, a call, a cast, a
/// parenthesis, a `.` before a later `->`, a `union` or a dereference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    pub(crate) root: String,
    pub(crate) links: Vec<String>,
    pub(crate) member: String,
}

impl Slot {
    /// The member path spelled from its root (`opts->ns`,
    /// `context->span->status.message`).
    pub(crate) fn place(&self) -> String {
        let mut place = self.root.clone();
        for link in &self.links {
            place.push_str("->");
            place.push_str(link);
        }
        place.push_str("->");
        place.push_str(&self.member);
        place
    }

    /// The slot's chain obligation: the root dereferenced, each link read,
    /// the final member's object its owner. `root_bound_untracked` is rule
    /// 11d's eligibility, asked by the flow only where the never-bound premise
    /// fails.
    pub(crate) fn chain(&self, root_never_bound: bool, root_bound_untracked: bool) -> AccessChain {
        AccessChain {
            root: self.root.clone(),
            dereferenced_root: true,
            root_never_bound,
            root_bound_untracked,
            root_untracked_only: false,
            links: self.links.clone(),
            member: self.member.clone(),
        }
    }
}

fn is_identifier(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && text.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// Read a slot actual (fused tokens).
pub(crate) fn read_slot(actual: &[&str]) -> Option<Slot> {
    let ["&", root, rest @ ..] = actual else {
        return None;
    };
    if !is_identifier(root) || rest.is_empty() || rest.len() % 2 != 0 {
        return None;
    }
    let steps = rest
        .chunks(2)
        .map(|pair| match pair {
            [op @ ("->" | "."), member] if is_identifier(member) => Some((*op, *member)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    if steps.first()?.0 != "->" {
        return None;
    }
    // The last `->` step: every step before it is a dereferenced link, every
    // step from it on selects within the final object with `.` only.
    let last_arrow = steps.iter().rposition(|(op, _)| *op == "->")?;
    if steps[..last_arrow].iter().any(|(op, _)| *op == ".") {
        return None;
    }
    let links = steps[..last_arrow]
        .iter()
        .map(|(_, member)| (*member).to_owned())
        .collect();
    let member = steps[last_arrow..]
        .iter()
        .map(|(_, member)| *member)
        .collect::<Vec<_>>()
        .join(".");
    Some(Slot {
        root: (*root).to_owned(),
        links,
        member,
    })
}

const WRITES: [&str; 13] = [
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "++", "--",
];

/// The indices at which `name` is written or has its address taken (fused
/// tokens). A token after `->` or `.` is a member name, never the variable;
/// `&N->F`, `&N.F` and `&N[I]` take a member's address, not `N`'s.
fn writes(body: &[&str], name: &str) -> Vec<usize> {
    body.iter()
        .enumerate()
        .filter(|(index, token)| {
            let previous = index.checked_sub(1).and_then(|at| body.get(at)).copied();
            let next = body.get(index + 1).copied();
            let member = matches!(next, Some("->" | "." | "["));
            **token == name
                && !matches!(previous, Some("->" | "."))
                && (next.is_some_and(|next| WRITES.contains(&next))
                    || matches!(previous, Some("++" | "--"))
                    || (previous == Some("&") && !member))
        })
        .map(|(index, _)| index)
        .collect()
}

/// Whether `value` (an assigned value's tokens, up to its `;`) is a caller
/// root of `roots`: `N`, `( T… * ) N` (one object-pointer cast of identifier
/// type words, no `union`) or `N -> F ( -> G )*`.
fn from_roots(value: &[&str], roots: &BTreeSet<String>) -> bool {
    let value = match value {
        ["(", rest @ ..] => {
            let Some(close) = rest.iter().position(|text| *text == ")") else {
                return false;
            };
            let words = &rest[..close];
            let [types @ .., "*"] = words else {
                return false;
            };
            if types.is_empty()
                || types
                    .iter()
                    .any(|word| !is_identifier(word) || *word == "union")
            {
                return false;
            }
            &rest[close + 1..]
        }
        value => value,
    };
    match value {
        [root, rest @ ..] => {
            roots.contains(*root)
                && rest.len() % 2 == 0
                && rest
                    .chunks(2)
                    .all(|pair| pair[0] == "->" && is_identifier(pair[1]))
        }
        [] => false,
    }
}

/// The caller-supplied roots of one body (fused tokens), premise
/// **P-72-CALLER-OBJECT**: the formals no statement writes, increments or
/// takes the address of; then, to a fixpoint, every other name whose every
/// write is a plain `NAME = VALUE ;` (a declarator's initialiser or a
/// statement), never address-taken or compound-assigned, with `VALUE` a
/// caller root, an object-pointer cast of one, or a member read through one
/// (`map = (struct cmt_map *) context;`, `opts = map->opts;`,
/// `struct ctx *context = ctx;`), and which some write binds. A member path
/// through such a name lies in memory the caller supplied (stated, not
/// proved: a pointer read out of the caller's object designates an object a
/// caller holds, never this frame's storage).
pub(crate) fn caller_roots(body: &[&str], formals: &BTreeSet<String>) -> BTreeSet<String> {
    let mut roots = formals
        .iter()
        .filter(|name| writes(body, name).is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();
    let candidates = body
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            is_identifier(token)
                && !formals.contains(**token)
                && body.get(index + 1) == Some(&"=")
                && !matches!(
                    index.checked_sub(1).and_then(|at| body.get(at)),
                    Some(&("->" | "."))
                )
        })
        .map(|(_, token)| (*token).to_owned())
        .collect::<BTreeSet<_>>();
    loop {
        let mut grown = false;
        for name in &candidates {
            if roots.contains(name) {
                continue;
            }
            let sites = writes(body, name);
            let admitted = !sites.is_empty()
                && sites.iter().all(|&at| {
                    body.get(at + 1) == Some(&"=")
                        && body.get(at.wrapping_sub(1)) != Some(&"&")
                        && body[at + 2..]
                            .iter()
                            .position(|text| matches!(*text, ";" | ","))
                            .is_some_and(|end| {
                                body[at + 2 + end] == ";"
                                    && from_roots(&body[at + 2..at + 2 + end], &roots)
                            })
                });
            if admitted {
                roots.insert(name.clone());
                grown = true;
            }
        }
        if !grown {
            return roots;
        }
    }
}

/// The flow's decision for a rule-72 store: the destination's owner. The old
/// value's premise (no binding rooted at the root carries a target) is
/// rule 10v's, applied by the same flow after this decision.
/// `QCore.OwnedOutMemberSlot.decideDelivery` is the same function.
pub(crate) fn decide(owner: OwnerClass) -> Decision {
    match owner {
        OwnerClass::Untracked => Decision::Admit,
        OwnerClass::Tracked => Decision::OwnedField,
        OwnerClass::Unresolved => Decision::UnresolvedOwner,
    }
}

/// The decision vectors shared with `proof/QCore/OwnedOutMemberSlot.lean`
/// (`decisionVectors`): `(owner, decision)`.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(&str, &str)] = &[
    ("untracked", "admit"),
    ("tracked", "owned-field"),
    ("unresolved", "unresolved-owner"),
];

/// One slot vector: a fused actual, and its root, links and member, or `None`.
#[cfg(test)]
pub(crate) type SlotVector = (
    &'static str,
    Option<(&'static str, &'static [&'static str], &'static str)>,
);

/// The slot vectors shared with `proof/QCore/OwnedOutMemberSlot.lean`
/// (`slotVectors`).
#[cfg(test)]
pub(crate) const ROOT_VECTORS: &[SlotVector] = &[
    ("& opts -> ns", Some(("opts", &[], "ns"))),
    (
        "& context -> scope_span -> instrumentation_scope -> name",
        Some(("context", &["scope_span", "instrumentation_scope"], "name")),
    ),
    (
        "& context -> span -> status . message",
        Some(("context", &["span"], "status.message")),
    ),
    ("& ciov -> iov_base", Some(("ciov", &[], "iov_base"))),
    ("& s", None),
    ("& s . p", None),
    ("& s -> a . b -> c", None),
    ("& s -> p [ 0 ]", None),
    ("& ( s -> p )", None),
    ("s -> p", None),
];

#[cfg(test)]
#[path = "../tests/unit/owned_out_member.rs"]
mod tests;
