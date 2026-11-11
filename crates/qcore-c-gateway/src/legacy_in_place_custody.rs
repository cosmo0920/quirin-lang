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

//! Card **CG25-RESOURCE-KINDS** (gateway rule 11v): in-place resource kinds
//! in route G.
//!
//! An in-place kind (`regex`, contract spec §C04.6.1) is initialised into an
//! object the caller owns: `regcomp(&re, …)` creates an obligation that lives
//! in `re`'s storage, `regfree(&re)` discharges it, and `regexec(&re, …)` uses
//! it. Nothing is returned; the initialiser's `int` result is its **status**,
//! zero on success and nonzero on failure, so the kind's failure value (its
//! sentinel) is carried by the status, not by the storage. This module holds
//! the pieces the rest of the gateway consults, behind thin hooks in
//! `legacy_c.rs` that name the card:
//!
//! * [`InPlaceNames`]: the names of one body passed as `&NAME` to a declared
//!   in-place initialiser. Empty in every environment that declares no
//!   in-place kind, which makes every rule here the identity there.
//! * [`Parser::in_place_actuals`]: an in-place-kinded actual must be `&NAME`
//!   for `NAME` one automatic object of this function (one Clang `VarDecl`,
//!   no storage class, not a pointer or array); its place is `NAME`, so the
//!   obligation is bound to the storage. Any other actual refuses.
//! * [`Parser::in_place_guard`]: a statement that mentions a tracked object is
//!   admitted only as a contracted call of a **declared** callee whose every
//!   mention of the object is a whole `&NAME` actual at an in-place position
//!   of the object's kind, or as the object's own declaration. Every other
//!   route (a struct copy, `memset(&re, …)` at an unkinded borrowed position,
//!   a member read, an undeclared callee) treats the storage as data, and is
//!   refused.
//! * **Status witnesses** ([`record_witness`], [`forget_witnesses`],
//!   [`witnessed_storage`], [`join_witnesses`]): the plain name an initialiser's
//!   status is assigned to witnesses its site until the name is written. A null
//!   test of the witness (`if (r)`, `if (r != 0)`, `if (!r)`, `r == 0`) refines
//!   the storage's binding: the zero edge is success (the obligation is
//!   certain), the nonzero edge is failure (no obligation). Joined by
//!   intersection, so a witness one arriving path lost refines nothing.
//! * [`verify_cfg`]: after the CFG is built, every call node with an in-place
//!   position must carry that position's place; a route that lowered one
//!   without it is refused, so no in-place effect is ever silently dropped.
//!
//! Everything here refuses rather than approximates.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{
    AbstractState, LegacyCfg, LegacyInstruction, Parser, ResourceContracts, ResourceKind,
    ResourceParameterMode, Token, call_arguments, is_identifier, strip_parentheses,
};
use crate::typed_c_ast::{children, kind};
use qcore_schemas::contract::ResourceFunctionContract;

/// The refusal of an in-place actual that is not `&NAME` of one automatic
/// object.
pub(crate) const ACTUAL_REFUSAL: &str = "in-place resource object is not the address of one automatic object of this function (rule 11v)";

/// The refusal of an initialiser whose status no plain name holds, or is
/// assigned to a name the body takes the address of.
pub(crate) const WITNESS_REFUSAL: &str =
    "in-place initialiser status not held by a plain, not address-taken name (rule 11v)";

/// The names of one body that may hold a tracked in-place object, with the
/// kind of the initialiser that names them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InPlaceNames {
    kinds: BTreeMap<String, BTreeSet<ResourceKind>>,
    /// Every name the body takes the address of: a status assigned to one
    /// can be written through a pointer, so it witnesses nothing.
    address_taken: BTreeSet<String>,
}

impl InPlaceNames {
    /// Every `NAME` passed as a whole `&NAME` actual at an in-place
    /// initialiser position of a contract in force, anywhere in `body` (the
    /// tokens between the braces). Flow-insensitive: a larger set only refuses
    /// more.
    pub(crate) fn read(body: &[&str], contracts: &ResourceContracts) -> Self {
        let mut kinds: BTreeMap<String, BTreeSet<ResourceKind>> = BTreeMap::new();
        let any_in_place = contracts.functions().values().any(has_in_place_position);
        if !any_in_place {
            return Self::default();
        }
        let address_taken = body
            .windows(2)
            .filter(|pair| pair[0] == "&" && is_identifier(pair[1]))
            .map(|pair| pair[1].to_owned())
            .collect();
        for (index, pair) in body.windows(2).enumerate() {
            if pair[1] != "(" || !is_identifier(pair[0]) {
                continue;
            }
            let Some(contract) = contracts.contract(pair[0]) else {
                continue;
            };
            if !has_in_place_position(contract) {
                continue;
            }
            let Some(arguments) = call_arguments(body, index) else {
                continue;
            };
            for (position, argument) in arguments.iter().enumerate() {
                let kind = contract.parameter_kind(position);
                if !kind.is_in_place()
                    || !matches!(
                        contract.parameters.get(position),
                        Some(ResourceParameterMode::Out { .. })
                    )
                {
                    continue;
                }
                if let Some(name) = address_of_name(argument) {
                    kinds.entry(name.to_owned()).or_default().insert(kind);
                }
            }
        }
        Self {
            kinds,
            address_taken,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.kinds.contains_key(name)
    }

    /// The one kind `name` is initialised as, when there is exactly one.
    pub(crate) fn kind(&self, name: &str) -> Option<ResourceKind> {
        match self
            .kinds
            .get(name)
            .map(|kinds| kinds.iter().collect::<Vec<_>>())
        {
            Some(kinds) if kinds.len() == 1 => Some(*kinds[0]),
            _ => None,
        }
    }
}

/// Whether a contract has any in-place-kinded position.
pub(crate) fn has_in_place_position(contract: &ResourceFunctionContract) -> bool {
    contract
        .parameter_kinds
        .values()
        .any(|kind| kind.is_in_place())
}

/// `NAME` when `argument` is `& NAME`, possibly parenthesised.
pub(crate) fn address_of_name<'a>(argument: &'a [&'a str]) -> Option<&'a str> {
    match strip_parentheses(argument) {
        ["&", name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => Some(name),
        ["&", "(", name, ")"] if is_identifier(name) => Some(name),
        _ => None,
    }
}

/// Whether `node` is the one automatic object named `name` this function
/// declares: exactly one `VarDecl` of that name (and no formal of it), with no
/// storage class, whose type is neither a pointer nor an array. Shared with
/// card CALLEE-RETENTION-CLASS1's frame holder (gateway rule 12t).
pub(crate) fn one_automatic_object(function: &Value, name: &str) -> bool {
    fn walk<'a>(node: &'a Value, name: &str, found: &mut Vec<&'a Value>) {
        if matches!(kind(node), "VarDecl" | "ParmVarDecl") && node["name"].as_str() == Some(name) {
            found.push(node);
        }
        for child in children(node) {
            walk(child, name, found);
        }
    }
    let mut found = Vec::new();
    walk(function, name, &mut found);
    let [declaration] = found.as_slice() else {
        return false;
    };
    let ty = declaration["type"]["qualType"].as_str().unwrap_or("*");
    kind(declaration) == "VarDecl"
        && crate::storage_duration::storage_of(declaration, true)
            == Some(crate::storage_duration::Storage::Automatic)
        && declaration["storageClass"].as_str().is_none()
        && !ty.contains('*')
        && !ty.contains('[')
        && !ty.contains("volatile")
}

impl Parser<'_> {
    /// Hook (rule 11v): give every in-place-kinded position of one contracted
    /// call its place. `Err` carries the refusal; the call is then lowered to
    /// nothing else.
    pub(super) fn in_place_actuals(
        &self,
        contract: &ResourceFunctionContract,
        raw_arguments: Option<&Vec<&[&str]>>,
        destination: Option<&str>,
        arguments: &mut [Option<String>],
    ) -> Result<(), String> {
        if !has_in_place_position(contract) {
            return Ok(());
        }
        let initialiser = contract
            .parameters
            .iter()
            .enumerate()
            .any(|(position, mode)| {
                matches!(mode, ResourceParameterMode::Out { .. })
                    && contract.parameter_kind(position).is_in_place()
            });
        // The status is the kind's sentinel: an initialiser whose status no
        // plain name holds (discarded, or consumed by an enclosing expression
        // or condition) leaves the obligation undecidable on every path, and
        // a status written through a pointer would witness a stale value.
        if initialiser
            && destination.is_none_or(|name| {
                !is_identifier(name) || self.in_place_names.address_taken.contains(name)
            })
        {
            return Err(WITNESS_REFUSAL.to_owned());
        }
        for (position, slot) in arguments.iter_mut().enumerate() {
            let kind = contract.parameter_kind(position);
            if !kind.is_in_place() {
                continue;
            }
            let raw = raw_arguments.and_then(|arguments| arguments.get(position));
            let name = raw
                .and_then(|argument| address_of_name(argument))
                .filter(|name| self.in_place_names.kind(name) == Some(kind))
                .filter(|name| {
                    self.typed_positions_index()
                        .is_some_and(|index| one_automatic_object(&index.function, name))
                });
            if let Some(name) = name {
                *slot = Some(name.to_owned());
                continue;
            }
            // A use of an object this body does not track (`regexec(preg, …)`
            // through a formal) keeps the plain borrowed reading: no
            // obligation of this body is touched. An initialiser or a
            // finaliser of one would create or discharge an obligation no
            // place carries, so it refuses, as does any actual naming a
            // tracked object in another form.
            let untracked_use = matches!(
                contract.parameters.get(position),
                Some(ResourceParameterMode::Borrowed)
            ) && raw.is_some_and(|argument| {
                !argument
                    .iter()
                    .any(|text| self.in_place_names.contains(text))
            });
            if !untracked_use {
                return Err(ACTUAL_REFUSAL.to_owned());
            }
        }
        Ok(())
    }

    /// Hook (rule 11v): whether `argument` is `&NAME` of a tracked object at
    /// an in-place position of that object's kind. The formal-address gate
    /// leaves such an actual to [`Self::in_place_actuals`], which decides it.
    pub(super) fn in_place_address(
        &self,
        contract: &ResourceFunctionContract,
        position: usize,
        argument: &[&str],
    ) -> bool {
        let kind = contract.parameter_kind(position);
        kind.is_in_place()
            && address_of_name(argument)
                .is_some_and(|name| self.in_place_names.kind(name) == Some(kind))
    }

    /// Hook (rule 11v): the guard over one complete statement's
    /// classification. With no in-place name in the body it is the identity.
    pub(super) fn in_place_guard(
        &mut self,
        tokens: &[Token],
        instruction: LegacyInstruction,
        saved: &super::probe_state::ParserProbeState,
    ) -> LegacyInstruction {
        if self.in_place_names.is_empty()
            || matches!(instruction, LegacyInstruction::Unknown { .. })
        {
            return instruction;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let mentioned = mentions(&texts, &self.in_place_names);
        let Some(name) = mentioned.first().copied() else {
            return instruction;
        };
        if declares_only(&texts, &mentioned) || self.admits(&texts, &instruction, &mentioned) {
            return instruction;
        }
        self.restore_probe_state(saved);
        LegacyInstruction::Unknown {
            description: format!(
                "tracked in-place object `{name}` used outside a declared in-place position is outside the resource projection (rule 11v)"
            ),
            writes: self.refused_writes(&texts),
        }
    }

    /// Whether `instruction` states the effect of every mention exactly: a
    /// contracted call of a declared callee, with no nested call, whose every
    /// mention of a tracked name is a whole `&NAME` actual bound at an
    /// in-place position of that name's kind.
    fn admits(&self, texts: &[&str], instruction: &LegacyInstruction, mentioned: &[&str]) -> bool {
        let LegacyInstruction::ContractCall {
            function,
            arguments,
            contract,
            nested,
            ..
        } = instruction
        else {
            return false;
        };
        if !nested.is_empty() || !self.declared_callees.contains(function) {
            return false;
        }
        mentioned.iter().all(|name| {
            let positions = arguments
                .iter()
                .enumerate()
                .filter(|(position, argument)| {
                    argument.as_deref() == Some(*name)
                        && Some(contract.parameter_kind(*position))
                            == self.in_place_names.kind(name)
                })
                .count();
            positions > 0 && positions == occurrences(texts, name)
        })
    }
}

/// The tracked names `texts` mentions as variables (a member name after `.`
/// or `->` is not one), in order of first mention.
fn mentions<'a>(texts: &[&'a str], names: &InPlaceNames) -> Vec<&'a str> {
    let mut found = Vec::new();
    for (index, text) in texts.iter().enumerate() {
        if names.contains(text) && !after_member(texts, index) && !found.contains(text) {
            found.push(*text);
        }
    }
    found
}

fn after_member(texts: &[&str], index: usize) -> bool {
    index >= 1
        && (texts[index - 1] == "."
            || (texts[index - 1] == ">" && index >= 2 && texts[index - 2] == "-"))
}

/// How many times `name` occurs in `texts` as a variable.
fn occurrences(texts: &[&str], name: &str) -> usize {
    texts
        .iter()
        .enumerate()
        .filter(|(index, text)| **text == name && !after_member(texts, *index))
        .count()
}

/// Whether the statement is a declaration with no initializer whose only
/// tracked mentions are its declarators (`regex_t re;`): it reads nothing.
fn declares_only(texts: &[&str], mentioned: &[&str]) -> bool {
    if texts
        .iter()
        .any(|text| matches!(*text, "=" | "(" | "[" | "&" | "." | ">"))
    {
        return false;
    }
    let Some(first) = texts.first() else {
        return false;
    };
    let declaration = super::DECLARATION_SPECIFIERS.contains(first)
        || (is_identifier(first) && texts.get(1).is_some_and(|second| is_identifier(second)));
    declaration
        && mentioned
            .iter()
            .all(|name| texts.last() == Some(name) || texts.contains(&","))
        && !texts.contains(&"static")
        && !texts.contains(&"extern")
}

/// Card CG25: record the status witness of an in-place initialiser lowered
/// as `DESTINATION = INIT(&NAME, …)`. The site is the call's one produced
/// site (an initialiser has no owned result and one in-place out position).
pub(super) fn record_witness(instruction: &LegacyInstruction, state: &mut AbstractState) {
    let LegacyInstruction::ContractCall {
        destination: Some(destination),
        contract,
        arguments,
        produced_sites,
        ..
    } = instruction
    else {
        return;
    };
    if !is_identifier(destination) {
        return;
    }
    let initialiser = contract
        .parameters
        .iter()
        .enumerate()
        .find(|(position, mode)| {
            matches!(mode, ResourceParameterMode::Out { .. })
                && contract.parameter_kind(*position).is_in_place()
        })
        .map(|(position, _)| position);
    let (Some(position), [site]) = (initialiser, produced_sites.as_slice()) else {
        return;
    };
    let Some(storage) = arguments.get(position).and_then(Option::as_deref) else {
        return;
    };
    state
        .status_witnesses
        .insert(destination.clone(), (*site, storage.to_owned()));
}

/// Card CG25: a witness whose name the instruction writes witnesses nothing
/// any more. Asked for every instruction, before [`record_witness`].
pub(super) fn forget_witnesses(
    instruction: &LegacyInstruction,
    state: &mut AbstractState,
    writes: impl Fn(&LegacyInstruction, &str) -> bool,
) {
    if state.status_witnesses.is_empty() {
        return;
    }
    state
        .status_witnesses
        .retain(|name, _| !writes(instruction, name));
}

/// Card CG25: the storage whose initialisation `source` still witnesses, when
/// the storage is bound to exactly that one site.
pub(super) fn witnessed_storage(state: &AbstractState, source: &str) -> Option<String> {
    let (site, storage) = state.status_witnesses.get(source)?;
    let pointer = state.pointers.get(storage)?;
    (pointer.derivation.is_identity()
        && !pointer.unknown
        && pointer.targets.len() == 1
        && pointer.targets.contains(site))
    .then(|| storage.clone())
}

/// Card CG25: witnesses survive a join only where both sides agree.
pub(crate) fn join_witnesses(
    current: &mut BTreeMap<String, (usize, String)>,
    incoming: &BTreeMap<String, (usize, String)>,
) {
    current.retain(|name, witness| incoming.get(name) == Some(witness));
}

/// Card CG25: every call node that has an in-place position must carry that
/// position's place. A node a route lowered without it would drop the
/// effect, so it is refused instead (`Unsupported`, the path is unmodelled).
pub(crate) fn verify_cfg(cfg: &mut LegacyCfg) {
    for node in &mut cfg.nodes {
        let LegacyInstruction::ContractCall {
            arguments,
            contract,
            nested,
            ..
        } = &node.instruction
        else {
            continue;
        };
        let complete = |arguments: &[Option<String>], contract: &ResourceFunctionContract| {
            (0..contract.parameters.len()).all(|position| {
                !contract.parameter_kind(position).is_in_place()
                    || matches!(
                        contract.parameters.get(position),
                        Some(ResourceParameterMode::Borrowed)
                    )
                    || arguments.get(position).is_some_and(Option::is_some)
            })
        };
        let nested_in_place = nested
            .iter()
            .any(|nested| has_in_place_position(&nested.contract));
        if nested_in_place || !complete(arguments, contract) {
            node.instruction = LegacyInstruction::Unknown {
                description: ACTUAL_REFUSAL.to_owned(),
                writes: super::RefusedWrites::Unbounded,
            };
        }
    }
}
