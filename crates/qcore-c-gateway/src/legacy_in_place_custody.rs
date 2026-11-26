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
//! * **Every mention, whatever the route** (card CG25-VOID-CAST-GUARD-ESCAPE,
//!   rule 11v (6)): [`InPlaceNames::body_mentions`] reads every mention of a
//!   tracked object in the body's token stream once, before any route, and
//!   [`verify_mentions`] refuses the function at its entry when one is not
//!   the object's declarator, a `sizeof` operand, or a whole custody actual
//!   at a position of its kind, or when a custody actual's call reached no
//!   node with the object's place. The statement guard sees only the first
//!   classification; this sees every fallback, condition and return route,
//!   and a by-value actual too, because it reads names off the tokens rather
//!   than off the nodes.
//! * **Automatic thread keys** (card CG25-THREAD-KEY-AUTOMATIC-REVISIT, rule
//!   11v (7)): `pthread_key_create(&k, 0)` into the one automatic key object
//!   of the function is tracked like a `regex` object, with
//!   `pthread_key_delete(k)` and `pthread_getspecific(k)` as by-value
//!   custody actuals. [`report_uncertified_key_uses`] admits a by-value use
//!   only where the settled state certifies the key; a released key is the
//!   `use_after_close` candidate.
//! * **Every in-place kind, certified and one object** (card
//!   CG25-IN-PLACE-REGEX, rule 74i): [`report_uncertified_key_uses`] reads a
//!   use of every in-place kind (`regexec(&re, …)` too), and every in-place
//!   actual must name the one automatic object every reference of the name
//!   designates (`Parser::automatic_object`), not only a key's.
//!
//! Everything here refuses rather than approximates.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{
    AbstractState, LegacyCfg, LegacyCfgNode, LegacyInstruction, Parser, RefusedWrites,
    ResourceContracts, ResourceKind, ResourceParameterMode, Token, call_argument_spans,
    call_arguments, is_identifier, strip_parentheses,
};
use crate::typed_c_ast::{children, kind};
use qcore_schemas::contract::ResourceFunctionContract;

/// The refusal of an in-place actual that is not `&NAME` of one automatic
/// object.
pub(crate) const ACTUAL_REFUSAL: &str = "in-place resource object is not the address of one automatic object of this function (rule 11v)";

/// Card CURRENT-CONTEXT-NEW-TRIAGE (D3): the refusal of a by-value finaliser
/// of a key held by static storage. Its creation bound no frame obligation
/// (P-THREAD-KEY-STATIC), and whether another frame still needs it is outside
/// this one.
pub(crate) const STATIC_KEY_RELEASE_REFUSAL: &str = "finaliser of a thread key held by static storage: the holder's lifetime is outside this frame (rule 11v)";

/// Card CURRENT-CONTEXT-NEW-TRIAGE (D3): the refusal of a `thread_key`
/// creation into anything but an object of static storage duration. D3
/// refused automatic keys too: a by-value use (`(void)
/// pthread_getspecific(k)`) reached a statement route that read the key as
/// data. Rule 11v (6) now refuses every such mention, so card
/// CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)) tracks the one automatic
/// key object of a function (`Parser::automatic_key`). This refusal now
/// covers every other non-static storage: a member, a formal, an object
/// reached through a pointer, and a name that also designates a file-scope
/// object. Its text is unchanged; every object it names is still not of
/// static storage duration.
pub(crate) const THREAD_KEY_STORAGE_REFUSAL: &str =
    "thread key created into an object that is not of static storage duration (rule 11v (5))";

/// The refusal of an initialiser whose status no plain name holds, or is
/// assigned to a name the body takes the address of.
pub(crate) const WITNESS_REFUSAL: &str =
    "in-place initialiser status not held by a plain, not address-taken name (rule 11v)";

/// The names of one body that may hold a tracked in-place object, with the
/// kind of the initialiser that names them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InPlaceNames {
    kinds: BTreeMap<String, BTreeSet<ResourceKind>>,
    /// Card CURRENT-CONTEXT-NEW-TRIAGE (D3): every unshadowed object of static
    /// storage duration (file scope or block `static`, never thread-local)
    /// passed as `&NAME` to a `thread_key` initialiser. Its key is held by
    /// static storage (P-THREAD-KEY-STATIC): no frame obligation is bound, so
    /// it is not a tracked name.
    static_keys: BTreeSet<String>,
    /// Every name of an unshadowed static-duration object (not thread-local)
    /// this body may name, for the by-value finaliser's refusal.
    static_objects: BTreeSet<String>,
    /// Every name the body takes the address of: a status assigned to one
    /// can be written through a pointer, so it witnesses nothing.
    address_taken: BTreeSet<String>,
}

impl InPlaceNames {
    /// Every `NAME` passed as a whole `&NAME` actual at an in-place
    /// initialiser position of a contract in force, anywhere in `body` (the
    /// tokens between the braces). Flow-insensitive: a larger set only refuses
    /// more.
    pub(crate) fn read(
        body: &[&str],
        contracts: &ResourceContracts,
        static_objects: &BTreeSet<String>,
    ) -> Self {
        let mut kinds: BTreeMap<String, BTreeSet<ResourceKind>> = BTreeMap::new();
        let mut static_keys = BTreeSet::new();
        let any_in_place = contracts.functions().values().any(has_in_place_position);
        if !any_in_place {
            return Self::default();
        }
        // Rule 74e: a name after an odd run of `&` (`p && name` takes no
        // address).
        let address_taken = (1..body.len())
            .filter(|&at| {
                is_identifier(body[at]) && crate::ampersand_runs::ends_in_address(body, at)
            })
            .map(|at| body[at].to_owned())
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
                    if kind == ResourceKind::ThreadKey && static_objects.contains(name) {
                        // Card CURRENT-CONTEXT-NEW-TRIAGE (D3): a key created
                        // into static storage is the static holder's.
                        static_keys.insert(name.to_owned());
                    } else {
                        // Card CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v
                        // (7)): any other `&NAME` key is tracked like a
                        // `regex` object; `in_place_actuals` admits only one
                        // automatic object every reference of the body names.
                        kinds.entry(name.to_owned()).or_default().insert(kind);
                    }
                }
            }
        }
        // A name the body also initialises another way stays tracked, never
        // held by static storage.
        static_keys.retain(|name: &String| !kinds.contains_key(name));
        Self {
            kinds,
            static_keys,
            static_objects: static_objects.clone(),
            address_taken,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Card CURRENT-CONTEXT-NEW-TRIAGE (D3): whether `name` is a static
    /// object a `thread_key` initialiser of this body creates a key into.
    pub(crate) fn static_key(&self, name: &str) -> bool {
        self.static_keys.contains(name)
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

    /// Card CG25-VOID-CAST-GUARD-ESCAPE (rule 11v (6)): every mention of a
    /// tracked object in `body` (the tokens between the function's braces, as
    /// the parser reads them), read once and before any route.
    ///
    /// A mention is admitted only as one of three things:
    ///
    /// * the object's declarator, by the statement guard's own
    ///   `declares_only` over the enclosing statement;
    /// * the operand of `sizeof` spelled `sizeof NAME` or `sizeof ( NAME )`,
    ///   which C does not evaluate (C11 6.5.3.4p2; a tracked object is never
    ///   an array, so never a VLA);
    /// * a **custody actual**: a whole `&NAME` actual (or `NAME` itself at a
    ///   by-value finaliser or use of a by-value kind) of a direct call to a
    ///   declared callee with a contract in force, at a position of the
    ///   object's kind.
    ///
    /// The first other mention is recorded as stray, whatever its statement,
    /// condition or return is and whichever route would lower it.
    pub(crate) fn body_mentions(
        &self,
        body: &[Token],
        contracts: &ResourceContracts,
        declared_callees: &BTreeSet<String>,
    ) -> BodyMentions {
        let mut mentions = BodyMentions::default();
        if self.kinds.is_empty() {
            return mentions;
        }
        let texts = body
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let mut custody_indices = BTreeSet::new();
        for index in 0..texts.len().saturating_sub(1) {
            let callee = texts[index];
            if texts[index + 1] != "("
                || !is_identifier(callee)
                || after_member(&texts, index)
                || !declared_callees.contains(callee)
            {
                continue;
            }
            let Some(contract) = contracts.contract(callee) else {
                continue;
            };
            let Some(spans) = call_argument_spans(&texts, index) else {
                continue;
            };
            for (position, (start, end)) in spans.into_iter().enumerate() {
                let kind = contract.parameter_kind(position);
                if !kind.is_in_place() {
                    continue;
                }
                let argument = &texts[start..end];
                let by_value = kind.finalised_by_value()
                    && !matches!(
                        contract.parameters.get(position),
                        Some(ResourceParameterMode::Out { .. })
                    );
                // Card CG25-THREAD-KEY-AUTOMATIC-REVISIT: a by-value position
                // names the object by itself and an address position by
                // `&NAME`; neither spelling is custody at the other.
                let name = if by_value {
                    match strip_parentheses(argument) {
                        [name] if is_identifier(name) => Some(*name),
                        _ => None,
                    }
                } else {
                    address_of_name(argument)
                };
                let Some(name) = name.filter(|name| self.kind(name) == Some(kind)) else {
                    continue;
                };
                let Some(at) = (start..end).find(|at| texts[*at] == name) else {
                    continue;
                };
                custody_indices.insert(at);
                let entry = mentions
                    .custody
                    .entry((callee.to_owned(), position, name.to_owned()))
                    .or_insert((0, body[at].line));
                entry.0 += 1;
            }
        }
        for (index, text) in texts.iter().enumerate() {
            if !self.contains(text) || after_member(&texts, index) {
                continue;
            }
            if custody_indices.contains(&index)
                || sizeof_operand(&texts, index)
                || declarator(&texts, index)
            {
                continue;
            }
            mentions.stray = Some((body[index].line, (*text).to_owned()));
            break;
        }
        mentions
    }
}

/// Card CG25-VOID-CAST-GUARD-ESCAPE (rule 11v (6)): what
/// [`InPlaceNames::body_mentions`] read of one body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BodyMentions {
    /// The first mention of a tracked object that is not admitted: its line
    /// and the object's name.
    stray: Option<(usize, String)>,
    /// Every custody actual, keyed by callee, position and object: how many
    /// times the body spells it, and the line of the first.
    custody: BTreeMap<(String, usize, String), (usize, usize)>,
}

/// Whether the mention at `index` is the operand of `sizeof NAME` or
/// `sizeof ( NAME )`.
fn sizeof_operand(texts: &[&str], index: usize) -> bool {
    (index >= 1 && texts[index - 1] == "sizeof")
        || (index >= 2
            && texts[index - 2] == "sizeof"
            && texts[index - 1] == "("
            && texts.get(index + 1) == Some(&")"))
}

/// Whether the mention at `index` is a declarator of a statement the
/// statement guard reads as a declaration that reads nothing
/// ([`declares_only`]): the statement runs from the token after the nearest
/// `;`, `{` or `}` before `index` to the nearest `;` after it.
fn declarator(texts: &[&str], index: usize) -> bool {
    let start = texts[..index]
        .iter()
        .rposition(|text| matches!(*text, ";" | "{" | "}"))
        .map_or(0, |at| at + 1);
    let Some(end) = texts[index..].iter().position(|text| *text == ";") else {
        return false;
    };
    let statement = &texts[start..index + end];
    // A statement keyword followed by the name (`return re;`, `else re;`)
    // reads the object; only a declaration declares it.
    let keyword = statement.first().is_some_and(|first| {
        matches!(
            *first,
            "return"
                | "goto"
                | "case"
                | "default"
                | "else"
                | "do"
                | "if"
                | "while"
                | "for"
                | "switch"
                | "break"
                | "continue"
                | "sizeof"
                | "typedef"
        )
    });
    !keyword && declares_only(statement, &[texts[index]])
}

/// Card CG25-VOID-CAST-GUARD-ESCAPE: the refusal of a stray mention.
fn stray_refusal(name: &str) -> String {
    format!(
        "tracked in-place object `{name}` named outside its declarator, a `sizeof` operand or a declared in-place position (rule 11v (6))"
    )
}

/// Card CG25-VOID-CAST-GUARD-ESCAPE: the refusal of a custody actual whose
/// call no route lowered with the object's place.
fn uncarried_refusal(callee: &str, name: &str) -> String {
    format!("in-place actual `{name}` of `{callee}` is carried by no call node (rule 11v (6))")
}

/// Card CURRENT-CONTEXT-NEW-TRIAGE (D3): whether `instruction` is this
/// module's guard refusal (a tracked in-place object named outside a declared
/// in-place position). No later statement route may re-admit it: a route that
/// re-lowers the statement (`(void) f(obj)` through the cast route, for
/// instance) would read tracked storage as data, which rule 11v (4) forbids.
pub(crate) fn is_guard_refusal(instruction: &LegacyInstruction) -> bool {
    matches!(
        instruction,
        LegacyInstruction::Unknown { description, .. }
            if description.starts_with("tracked in-place object `")
                && description.ends_with("(rule 11v)")
    )
}

/// Card CURRENT-CONTEXT-NEW-TRIAGE (D3): whether `body` calls a declared
/// callee with a `thread_key` position (an initialiser, which the static-key
/// reading needs the unit's static objects for, or a finaliser, whose refusal
/// of a static key names it).
pub(crate) fn calls_thread_key_initialiser(body: &[&str], contracts: &ResourceContracts) -> bool {
    body.windows(2).any(|pair| {
        pair[1] == "("
            && is_identifier(pair[0])
            && contracts.contract(pair[0]).is_some_and(|contract| {
                // Card CG25-THREAD-KEY-AUTOMATIC-REVISIT: a by-value use
                // (`pthread_getspecific`) needs no static objects.
                contract.parameter_kinds.iter().any(|(position, kind)| {
                    *kind == ResourceKind::ThreadKey
                        && !matches!(
                            contract.parameters.get(*position),
                            Some(ResourceParameterMode::Borrowed)
                        )
                })
            })
    })
}

/// Whether a contract has any in-place-kinded position.
pub(crate) fn has_in_place_position(contract: &ResourceFunctionContract) -> bool {
    contract
        .parameter_kinds
        .values()
        .any(|kind| kind.is_in_place())
}

/// Card CG25-THREAD-KEY-AUTOMATIC-REVISIT: whether a contract has an
/// in-place position other than a by-value use: an initialiser, a finaliser,
/// or a use that names the object by address.
fn has_in_place_custody_position(contract: &ResourceFunctionContract) -> bool {
    contract.parameter_kinds.iter().any(|(position, kind)| {
        kind.is_in_place()
            && !(kind.finalised_by_value()
                && matches!(
                    contract.parameters.get(*position),
                    Some(ResourceParameterMode::Borrowed)
                ))
    })
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
/// card CALLEE-RETENTION-CLASS1's frame holder (gateway rule 12t) and the
/// callback registry's frame holder. It counts declarations only: every
/// caller also requires [`every_reference_is`] (rules 74i, 74o), or a
/// block-scope local shadowing a file-scope object the body names is read as
/// one object.
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

/// Card CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)): whether every
/// `DeclRefExpr` naming `name` in `function` refers to the one `VarDecl` of
/// that name the function declares. A reference to a file-scope object the
/// local shadows (before its declarator, or after its block closes) is
/// another object, so the name is then not one key.
pub(crate) fn every_reference_is(function: &Value, name: &str) -> bool {
    fn walk<'a>(
        node: &'a Value,
        name: &str,
        declarations: &mut Vec<&'a str>,
        references: &mut Vec<Option<&'a str>>,
    ) {
        if kind(node) == "VarDecl" && node["name"].as_str() == Some(name) {
            declarations.push(node["id"].as_str().unwrap_or(""));
        }
        if kind(node) == "DeclRefExpr" && node["referencedDecl"]["name"].as_str() == Some(name) {
            references.push(node["referencedDecl"]["id"].as_str());
        }
        for child in children(node) {
            walk(child, name, declarations, references);
        }
    }
    let mut declarations = Vec::new();
    let mut references = Vec::new();
    walk(function, name, &mut declarations, &mut references);
    let [declaration] = declarations.as_slice() else {
        return false;
    };
    !declaration.is_empty()
        && references
            .iter()
            .all(|reference| *reference == Some(*declaration))
}

impl Parser<'_> {
    /// Hook (rule 11v): give every in-place-kinded position of one contracted
    /// call its place. `Err` carries the refusal; the call is then lowered to
    /// nothing else.
    ///
    /// `Ok` carries the initialiser positions whose object is held by static
    /// storage (card CURRENT-CONTEXT-NEW-TRIAGE, D3: a `thread_key` created
    /// into `&G`, `G` of static storage duration). The caller reads each as a
    /// borrowed position: the key is the static holder's
    /// (P-THREAD-KEY-STATIC), so the call binds no frame obligation and needs
    /// no status witness.
    pub(super) fn in_place_actuals(
        &self,
        contract: &ResourceFunctionContract,
        raw_arguments: Option<&Vec<&[&str]>>,
        destination: Option<&str>,
        arguments: &mut [Option<String>],
    ) -> Result<Vec<usize>, String> {
        if !has_in_place_position(contract) {
            return Ok(Vec::new());
        }
        let static_held = (0..contract.parameters.len())
            .filter(|position| {
                contract.parameter_kind(*position) == ResourceKind::ThreadKey
                    && matches!(
                        contract.parameters.get(*position),
                        Some(ResourceParameterMode::Out { .. })
                    )
                    && raw_arguments
                        .and_then(|arguments| arguments.get(*position))
                        .and_then(|argument| address_of_name(argument))
                        .is_some_and(|name| self.in_place_names.static_key(name))
            })
            .collect::<Vec<_>>();
        // Card CURRENT-CONTEXT-NEW-TRIAGE (D3), narrowed by card
        // CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)): a key created
        // anywhere but into static storage or into one automatic key object
        // of this function refuses.
        if (0..contract.parameters.len()).any(|position| {
            contract.parameter_kind(position) == ResourceKind::ThreadKey
                && matches!(
                    contract.parameters.get(position),
                    Some(ResourceParameterMode::Out { .. })
                )
                && !static_held.contains(&position)
                && !raw_arguments
                    .and_then(|arguments| arguments.get(position))
                    .and_then(|argument| address_of_name(argument))
                    .is_some_and(|name| self.automatic_key(name))
        }) {
            return Err(THREAD_KEY_STORAGE_REFUSAL.to_owned());
        }
        let initialiser = contract
            .parameters
            .iter()
            .enumerate()
            .any(|(position, mode)| {
                matches!(mode, ResourceParameterMode::Out { .. })
                    && contract.parameter_kind(position).is_in_place()
                    && !static_held.contains(&position)
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
            if static_held.contains(&position) {
                *slot = None;
                continue;
            }
            let raw = raw_arguments.and_then(|arguments| arguments.get(position));
            // Card CURRENT-CONTEXT-NEW-TRIAGE (D3): a by-value finaliser or use
            // (`pthread_key_delete(k)`, `pthread_getspecific(k)`) names the
            // object by itself. The place is `k` when `k` is the one automatic
            // key object of this function (card
            // CG25-THREAD-KEY-AUTOMATIC-REVISIT, [`Self::automatic_key`]); a
            // static key's finaliser refuses (its holder's lifetime is outside
            // the frame). `&k` at a by-value position is never its place.
            let by_value = kind.finalised_by_value()
                && !matches!(
                    contract.parameters.get(position),
                    Some(ResourceParameterMode::Out { .. })
                );
            if by_value {
                let named = raw
                    .map(|argument| strip_parentheses(argument))
                    .and_then(|argument| match argument {
                        [name] if is_identifier(name) => Some(*name),
                        _ => None,
                    });
                if let Some(name) = named
                    .filter(|name| self.in_place_names.kind(name) == Some(kind))
                    .filter(|name| self.automatic_key(name))
                {
                    *slot = Some(name.to_owned());
                    continue;
                }
                let released = matches!(
                    contract.parameters.get(position),
                    Some(ResourceParameterMode::Released { .. })
                );
                if released
                    && named.is_some_and(|name| {
                        self.in_place_names.static_objects.contains(name)
                            && !self.in_place_names.contains(name)
                    })
                {
                    return Err(STATIC_KEY_RELEASE_REFUSAL.to_owned());
                }
            }
            let name = raw
                .filter(|_| !by_value)
                .and_then(|argument| address_of_name(argument))
                .filter(|name| self.in_place_names.kind(name) == Some(kind))
                // Card CG25-IN-PLACE-REGEX (rule 74i): every in-place kind,
                // not only `thread_key`, needs the one automatic object every
                // reference of the name designates.
                .filter(|name| self.automatic_object(name));
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
        Ok(static_held)
    }

    /// Card CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)): whether `name`
    /// is a tracked `thread_key` object that is the one automatic object of
    /// this function by that name ([`one_automatic_object`]) and that every
    /// reference to `name` in the function designates
    /// ([`every_reference_is`]), so no mention of the name can be a
    /// file-scope object the local shadows.
    fn automatic_key(&self, name: &str) -> bool {
        self.in_place_names.kind(name) == Some(ResourceKind::ThreadKey)
            && self.automatic_object(name)
    }

    /// Card CG25-IN-PLACE-REGEX (rule 74i): whether `name` is the one
    /// automatic object of this function by that name
    /// ([`one_automatic_object`]) and every reference to `name` in the
    /// function designates it ([`every_reference_is`]). Rule 11v (7) asked
    /// this of a `thread_key` only; a block-scope `regex_t g` that shadows a
    /// file-scope `g` the body also names is two objects behind one name.
    fn automatic_object(&self, name: &str) -> bool {
        self.typed_positions_index().is_some_and(|index| {
            one_automatic_object(&index.function, name) && every_reference_is(&index.function, name)
        })
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
        // Card CG25-THREAD-KEY-AUTOMATIC-REVISIT: a nested by-value use
        // (`free(pthread_getspecific(k))`) carries no place. When it names a
        // tracked key, rule 11v (6)'s carried count refuses the function;
        // otherwise it keeps the plain borrowed reading it had before the use
        // was kinded. Every other nested in-place position is refused here.
        let nested_in_place = nested
            .iter()
            .any(|nested| has_in_place_custody_position(&nested.contract));
        if nested_in_place || !complete(arguments, contract) {
            node.instruction = LegacyInstruction::Unknown {
                description: ACTUAL_REFUSAL.to_owned(),
                writes: super::RefusedWrites::Unbounded,
            };
        }
    }
}

/// Card CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)): the refusal of a
/// by-value use of a tracked key its creation status has not certified. Card
/// CG25-IN-PLACE-REGEX (rule 74i): every other in-place kind's refusal.
fn uncertified_use_refusal(name: &str, kind: ResourceKind) -> String {
    if kind == ResourceKind::ThreadKey {
        format!("use of thread key `{name}` not certified by its creation status (rule 11v (7))")
    } else {
        format!(
            "use of in-place object `{name}` not certified by its initialiser status (rule 74i)"
        )
    }
}

/// Card CG25-THREAD-KEY-AUTOMATIC-REVISIT (rule 11v (7)), generalised by card
/// CG25-IN-PLACE-REGEX (rule 74i) to **every in-place kind**: a use of a
/// tracked object (`pthread_getspecific(k)`, `regexec(&re, …)`,
/// `regerror(r, &re, …)`) is admitted only where the object is certainly
/// initialised: read from the settled incoming states, as
/// `report_kind_violations` reads its candidates. The tracked objects are the
/// places this CFG's own in-place initialisers bind (a static-held key
/// creation's position is borrowed, so it binds none).
///
/// At such a use the binding of the object must be an identity of exactly
/// one site, not unknown, not possibly failed (`may_be_null` or a nullable
/// site), and the site must be live. A site released on a reaching path is
/// `report_kind_violations`' `use_after_close` candidate and is left to it;
/// every other uncertain state (before the initialiser, before the status
/// test, on the failure edge, a join of those) refuses. POSIX leaves a use of
/// a key value not obtained from `pthread_key_create` undefined, and the
/// contents of a `regex_t` whose `regcomp` failed undefined. The check reads
/// no callee by name. Card CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE (rule
/// 74n) lets the contract tell `regerror`, which POSIX defines on the failed
/// object, from `regexec`: a use declared `resource_kind_failure_use` is also
/// admitted where [`failure_use_admitted`] holds (the failure edge, before
/// the test). Every other use reads as above.
pub(super) fn report_uncertified_key_uses(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    diagnostics: &mut BTreeSet<super::LegacyDiagnostic>,
) {
    let in_place_positions = |contract: &ResourceFunctionContract, mode_is_use: bool| {
        (0..contract.parameters.len())
            .filter(move |position| {
                let kind = contract.parameter_kind(*position);
                let mode = contract.parameters.get(*position);
                kind.is_in_place()
                    && if mode_is_use {
                        matches!(mode, Some(ResourceParameterMode::Borrowed))
                    } else {
                        matches!(mode, Some(ResourceParameterMode::Out { .. }))
                    }
            })
            .collect::<Vec<_>>()
    };
    let mut tracked = BTreeSet::new();
    for node in &cfg.nodes {
        if let LegacyInstruction::ContractCall {
            arguments,
            contract,
            ..
        } = &node.instruction
        {
            for position in in_place_positions(contract, false) {
                if let Some(place) = arguments.get(position).and_then(Option::as_deref) {
                    tracked.insert(place.to_owned());
                }
            }
        }
    }
    if tracked.is_empty() {
        return;
    }
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let LegacyInstruction::ContractCall {
            arguments,
            contract,
            ..
        } = &node.instruction
        else {
            continue;
        };
        let Some(state) = incoming.get(node_id).and_then(Option::as_ref) else {
            continue;
        };
        for position in in_place_positions(contract, true) {
            let Some(place) = arguments
                .get(position)
                .and_then(Option::as_deref)
                .filter(|place| tracked.contains(*place))
            else {
                continue;
            };
            let certified = state.pointers.get(place).is_some_and(|pointer| {
                pointer.derivation.is_identity()
                    && !pointer.unknown
                    && !pointer.may_be_null
                    && pointer.targets.len() == 1
                    && pointer.targets.iter().all(|site| {
                        !state.nullable_sites.contains(site)
                            && state
                                .resources
                                .get(site)
                                .is_some_and(|status| status & (super::LIVE | super::RELEASED) != 0)
                    })
            });
            if !certified
                && !(contract.is_failure_use(position) && failure_use_admitted(state, place))
            {
                diagnostics.insert(super::LegacyDiagnostic {
                    function: cfg.function.clone(),
                    line: node.line,
                    kind: super::LegacyDiagnosticKind::Unsupported {
                        description: uncertified_use_refusal(
                            place,
                            contract.parameter_kind(position),
                        ),
                    },
                });
            }
        }
    }
}

/// Card CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE (rule 74n): whether a use
/// declared `resource_kind_failure_use` (`regerror(r, &re, …)`) is admitted
/// where rule 74i's certification does not hold. The use is defined on the
/// object its latest initialiser left, whether that initialiser succeeded or
/// failed, so it is admitted when, on every reaching path, `place` is bound
/// to that initialiser's outcome and to nothing else:
///
/// * the binding exists on every path (a path that reaches before any
///   initialiser joins to `unknown`), is an identity and is not `unknown`;
/// * it names at most one site, which is live and released on no reaching
///   path (a site only possibly failed, before the status test, is live);
/// * a binding with no site is the failure edge's (rule 11v's nonzero edge
///   removes the site and leaves the binding definitely null), and only that.
///
/// A released site is not admitted here: it keeps rule 74i's certified
/// reading, `report_kind_violations`' `use_after_close` candidate. Before the
/// initialiser there is no binding. Every other state refuses as before.
fn failure_use_admitted(state: &AbstractState, place: &str) -> bool {
    let Some(pointer) = state.pointers.get(place) else {
        return false;
    };
    pointer.derivation.is_identity()
        && !pointer.unknown
        && pointer.targets.len() <= 1
        && (!pointer.targets.is_empty() || pointer.may_be_null)
        && pointer.targets.iter().all(|site| {
            state
                .resources
                .get(site)
                .is_some_and(|status| status & super::LIVE != 0 && status & super::RELEASED == 0)
        })
}

/// Card CG25-VOID-CAST-GUARD-ESCAPE (rule 11v (6)): refuse the function at its
/// entry when [`InPlaceNames::body_mentions`] found a stray mention, or when a
/// custody actual is carried by fewer call nodes than the body spells it (a
/// route lowered its call without the place, or not at all). Asked last,
/// over the finished CFG (after [`verify_cfg`] and every later pass); the
/// identity on a body that tracks nothing.
///
/// The refusal is a new node before the old entry, so it lies on every path
/// and no instruction the routes produced is replaced. Whichever route lowered
/// a statement, condition or return, an admitted function therefore never
/// reads tracked storage as data and never drops a custody effect.
pub(crate) fn verify_mentions(cfg: &mut LegacyCfg, mentions: &BodyMentions) {
    let refusal = mentions
        .stray
        .as_ref()
        .map(|(line, name)| (*line, stray_refusal(name)))
        .or_else(|| {
            mentions
                .custody
                .iter()
                .find(|((callee, position, name), (spelled, _))| {
                    carried(cfg, callee, *position, name) < *spelled
                })
                .map(|((callee, _, name), (_, line))| (*line, uncarried_refusal(callee, name)))
        });
    let Some((line, description)) = refusal else {
        return;
    };
    let entry = cfg.nodes.len();
    cfg.nodes.push(LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction: LegacyInstruction::Unknown {
            description,
            writes: RefusedWrites::Unbounded,
        },
        successors: vec![cfg.entry],
        line,
        is_exit: false,
        loop_exit_edge: None,
    });
    cfg.entry = entry;
}

/// How many call nodes of `callee` carry `name`'s place at `position`.
fn carried(cfg: &LegacyCfg, callee: &str, position: usize, name: &str) -> usize {
    cfg.nodes
        .iter()
        .filter(|node| {
            matches!(
                &node.instruction,
                LegacyInstruction::ContractCall { function, arguments, .. }
                    if function == callee
                        && arguments.get(position).and_then(Option::as_deref) == Some(name)
            )
        })
        .count()
}

/// Card CG25-VOID-CAST-GUARD-ESCAPE: the body reading over the libc
/// declarations, including the by-value `thread_key` case D3's node-level
/// check could not see (no place is recorded for a by-value scalar actual).
/// The names are built directly here; `tests/thread_key_in_place.rs` reads
/// real bodies.
#[cfg(test)]
mod mention_tests {
    use super::*;
    use qcore_contract::contract_loader::load_resource_contract_sources;

    const REGEX: &str =
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/regex.qv");
    const THREADS: &str =
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/threads.qv");

    fn contracts() -> ResourceContracts {
        load_resource_contract_sources(&BTreeMap::from([
            ("libc/regex.qv".to_owned(), REGEX.to_owned()),
            ("libc/threads.qv".to_owned(), THREADS.to_owned()),
        ]))
        .expect("the libc modules load")
    }

    fn tracked(name: &str, kind: ResourceKind) -> InPlaceNames {
        InPlaceNames {
            kinds: BTreeMap::from([(name.to_owned(), BTreeSet::from([kind]))]),
            ..InPlaceNames::default()
        }
    }

    /// Space-separated tokens, one line per `;`-terminated statement.
    fn tokens(text: &str) -> Vec<Token> {
        let mut line = 1;
        text.split_whitespace()
            .map(|text| {
                let token = Token {
                    text: text.to_owned(),
                    line,
                };
                if text == ";" {
                    line += 1;
                }
                token
            })
            .collect()
    }

    fn read(names: &InPlaceNames, body: &str) -> BodyMentions {
        let declared = [
            "regcomp",
            "regexec",
            "regerror",
            "regfree",
            "pthread_key_create",
            "pthread_key_delete",
            "pthread_getspecific",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        names.body_mentions(&tokens(body), &contracts(), &declared)
    }

    const REGEX_MATCH: &str = "regex_t re ; int r ; r = regcomp ( & re , pattern , 1 ) ; \
         if ( r ) { if ( errstr ) regerror ( r , & re , errstr , size ) ; return 0 ; } \
         r = regexec ( & re , str , 0 , 0 , 0 ) != 1 ; regfree ( & re ) ; return r ;";

    #[test]
    fn the_regex_match_body_names_only_custody_actuals() {
        let mentions = read(&tracked("re", ResourceKind::Regex), REGEX_MATCH);
        assert_eq!(mentions.stray, None);
        assert_eq!(
            mentions.custody.keys().cloned().collect::<Vec<_>>(),
            [
                ("regcomp".to_owned(), 0, "re".to_owned()),
                ("regerror".to_owned(), 1, "re".to_owned()),
                ("regexec".to_owned(), 0, "re".to_owned()),
                ("regfree".to_owned(), 0, "re".to_owned()),
            ]
        );
    }

    #[test]
    fn every_data_spelling_is_stray() {
        let names = tracked("re", ResourceKind::Regex);
        for (statement, line) in [
            ("( void ) re . re_nsub ;", 8),
            ("if ( re . re_nsub ) r = 2 ;", 8),
            ("return ( int ) re . re_nsub ;", 8),
            ("else re ;", 8),
            (
                "r = regerror ( r , & re , 0 , 0 ) + ( int ) sizeof ( re . re_nsub ) ;",
                8,
            ),
        ] {
            let body = REGEX_MATCH.replace("return r ;", &format!("{statement} return r ;"));
            assert_eq!(
                read(&names, &body).stray,
                Some((line, "re".to_owned())),
                "{statement}"
            );
        }
        // An unevaluated operand reads nothing.
        let body = REGEX_MATCH.replace("return r ;", "r = sizeof re + sizeof ( re ) ; return r ;");
        assert_eq!(read(&names, &body).stray, None);
    }

    /// D3's escape, after card CG25-THREAD-KEY-AUTOMATIC-REVISIT kinded the
    /// getter's key: a whole by-value `key` at the getter is custody (whether
    /// a node carries it is `verify_mentions`' check, which refuses the cast
    /// route's place-less lowering); `&key` or `key + 0` there is stray, as
    /// is `&key` at the by-value delete.
    #[test]
    fn a_by_value_getter_of_a_tracked_key_is_custody_only_whole() {
        let names = tracked("key", ResourceKind::ThreadKey);
        let body = "pthread_key_t key ; int r ; r = pthread_key_create ( & key , 0 ) ; \
             if ( r ) return ; pthread_key_delete ( key ) ;";
        let mentions = read(&names, body);
        assert_eq!(mentions.stray, None);
        assert!(mentions.custody.contains_key(&(
            "pthread_key_delete".to_owned(),
            0,
            "key".to_owned()
        )));
        let cast = format!("{body} ( void ) pthread_getspecific ( key ) ;");
        let mentions = read(&names, &cast);
        assert_eq!(mentions.stray, None);
        assert!(mentions.custody.contains_key(&(
            "pthread_getspecific".to_owned(),
            0,
            "key".to_owned()
        )));
        for stray in [
            "( void ) pthread_getspecific ( & key ) ;",
            "( void ) pthread_getspecific ( key + 0 ) ;",
            "pthread_key_delete ( & key ) ;",
        ] {
            let body = format!("{body} {stray}");
            assert_eq!(
                read(&names, &body).stray,
                Some((6, "key".to_owned())),
                "{stray}"
            );
        }
    }

    #[test]
    fn a_body_that_tracks_nothing_reads_nothing() {
        let mentions = read(&InPlaceNames::default(), REGEX_MATCH);
        assert_eq!(mentions, BodyMentions::default());
    }
}
