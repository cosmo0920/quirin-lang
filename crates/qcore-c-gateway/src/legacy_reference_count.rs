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

//! Card **REFCOUNT-RESOURCE-KIND** (gateway rules 13b and 13c): the route-G
//! adapter of [`crate::reference_count`]. Thin hooks, each the identity on
//! every environment that holds no reference declaration:
//!
//! 1. [`custody_contract`] (in `retaining_environment`): a keep is classified
//!    under its custody contract with the result read as `none`; the alias
//!    binding is this route's.
//! 2. [`Parser::reference_operands`] (in `retaining_call`): each reference
//!    actual as spelled, a site reserved for the family a keep may mint, and
//!    the refusal of a declaration rule 13c withdrew.
//! 3. [`apply`] (the transfer of a `ContractCall`, after the retentions):
//!    keeps and drops decided against the state.
//! 4. [`report_exit`] (at every exit, beside rule 12t's): what each family
//!    still owes; `report_leaks` leaves families to it.
//! 5. [`join`] (in `AbstractState::join`).
//! 6. [`reserved_site`] (in the kind table): a minted family is of the
//!    reference kind.
//! 7. [`settle`] (beside rule 13a's, at the three unit entry points): the
//!    body checks of rule 13c.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    ResourceContracts, ResourceFunctionContract, ResourceKind, ResourceReturnMode,
};
use qcore_schemas::reference_count::ReferenceOperation;
use qcore_schemas::retained_by::RetainingContract;

use super::slot_move_route::Invalid;
use super::{
    AbstractPointer, AbstractState, Derivation, LIVE, LegacyDiagnostic, LegacyDiagnosticKind,
    LegacyInstruction, Parser, Token,
};
use crate::reference_count::{
    Binding, DROPPED, DropDecision, ExitOutcome, Family, Held, KeepDecision, UNBALANCED, UNDECIDED,
    WITHDRAWN, decide_drop, decide_exit, decide_keep, owed,
};

/// The largest count a family is tracked to; one more keep makes it
/// undecided, which bounds the work list.
const HELD_BOUND: u32 = 16;

/// One reference operand of a call, in clause order.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceOperand {
    pub operation: ReferenceOperation,
    /// The name the actual denotes (a plain name, or a pointer cast of one),
    /// `None` for any other actual.
    pub name: Option<String>,
    /// The actual as spelled.
    pub text: String,
}

/// Hook 1: the custody contract a reference declaration is classified under.
/// A keep's alias result is read as `none`: the result's binding (the
/// family) is this route's, so the derived-base reading never refuses a
/// keep's cast actual. Everything else is the declaration's.
pub(super) fn custody_contract(declaration: &RetainingContract) -> ResourceFunctionContract {
    let mut contract = declaration.contract.clone();
    if declaration
        .references
        .iter()
        .any(|clause| clause.operation == ReferenceOperation::Kept)
    {
        contract.result = ResourceReturnMode::None;
    }
    contract
}

/// The refusal text of one reference operation.
#[must_use]
pub fn refusal(function: &str, actual: &str, detail: &str, reason: &str) -> String {
    format!("call to `{function}` with `{actual}`: {detail} ({reason})")
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// The name an actual denotes: a plain name, or a pointer cast of one
/// (`( T * ) name`, the type words identifiers, `const`, `struct` and `*`).
fn actual_name(actual: &[&str]) -> Option<String> {
    match actual {
        [name] if is_identifier(name) => Some((*name).to_owned()),
        ["(", rest @ ..] => {
            let close = rest.iter().position(|word| *word == ")")?;
            let (cast, after) = rest.split_at(close);
            let [")", name] = after else {
                return None;
            };
            (cast.last() == Some(&"*")
                && cast.iter().all(|word| *word == "*" || is_identifier(word))
                && is_identifier(name))
            .then(|| (*name).to_owned())
        }
        _ => None,
    }
}

impl Parser<'_> {
    /// Hook 2: the reference operands of a call to `function`, read from its
    /// actuals; `Err` with the refusal when the declaration is withdrawn here.
    pub(super) fn reference_operands(
        &self,
        function: &str,
        declaration: &RetainingContract,
        actual: &dyn Fn(usize) -> Option<Vec<String>>,
    ) -> Result<Vec<ReferenceOperand>, String> {
        if declaration.references_withdrawn {
            return Err(refusal(
                function,
                "?",
                "its body in this unit does not validate the declared reference clause",
                WITHDRAWN,
            ));
        }
        Ok(declaration
            .references
            .iter()
            .map(|clause| {
                let words = actual(clause.parameter);
                let borrowed = words
                    .as_ref()
                    .map(|words| words.iter().map(String::as_str).collect::<Vec<_>>());
                ReferenceOperand {
                    operation: clause.operation,
                    name: borrowed.as_deref().and_then(actual_name),
                    text: words.map_or_else(|| "?".to_owned(), |words| words.join(" ")),
                }
            })
            .collect())
    }

    /// Hook 2: reserve the site a keep may mint its family at.
    pub(super) fn reserve_reference_site(&mut self, instruction: &mut LegacyInstruction) {
        if let LegacyInstruction::ContractCall {
            produced_sites,
            retention: Some(operands),
            ..
        } = instruction
            && operands
                .references
                .iter()
                .any(|operand| operand.operation == ReferenceOperation::Kept)
        {
            produced_sites.push(self.next_site);
            self.next_site += 1;
        }
    }
}

/// Hook 6: the site a keep call reserved, which is of the reference kind.
#[must_use]
pub(super) fn reserved_site(instruction: &LegacyInstruction) -> Option<usize> {
    let LegacyInstruction::ContractCall {
        produced_sites,
        retention: Some(operands),
        ..
    } = instruction
    else {
        return None;
    };
    operands
        .references
        .iter()
        .any(|operand| operand.operation == ReferenceOperation::Kept)
        .then(|| produced_sites.last().copied())
        .flatten()
}

/// Whether a family's site is held by a structure other than names: a field
/// link, a stored containment, a container publication, a holder.
fn linked(state: &AbstractState, site: usize) -> bool {
    state
        .field_links
        .values()
        .any(|fields| fields.values().any(|children| children.contains(&site)))
        || state
            .contained
            .values()
            .any(|children| children.contains(&site))
        || state.published.contains(&site)
        || state
            .retained
            .iter()
            .any(|(object, _)| object.site() == Some(site))
}

/// Whether one of the family's references was handed off at an exit (it
/// escaped, alone or with the owner it is linked into, or was published or
/// stored), `None` when the paths disagree. A field link to an owner that
/// does not escape, and a holder's retention, hand nothing off: the
/// reference is still owed.
fn handed(state: &AbstractState, site: usize) -> Option<bool> {
    match state.resources.get(&site).copied() {
        Some(LIVE) => Some(state.published.contains(&site)),
        Some(super::ESCAPED | super::CONTAINED) => Some(true),
        _ => None,
    }
}

/// What `name` denotes for a reference operation.
fn binding(state: &AbstractState, name: &str) -> (Binding, Option<usize>) {
    match state.invalidated.get(name) {
        Some(Invalid::ReferenceDropped) => return (Binding::Invalid, None),
        Some(Invalid::ReferenceFreed { site }) => return (Binding::Freed, Some(*site)),
        Some(_) => return (Binding::Other, None),
        None => {}
    }
    let Some(pointer) = state.pointers.get(name) else {
        return (Binding::Unbound, None);
    };
    if !pointer.derivation.is_identity() {
        return (Binding::Other, None);
    }
    if pointer.targets.is_empty() {
        // An untracked value (an opaque read); a definitely-NULL binding is
        // not an object.
        return if pointer.unknown {
            (Binding::Unbound, None)
        } else {
            (Binding::Other, None)
        };
    }
    let [site] = pointer.targets.iter().copied().collect::<Vec<_>>()[..] else {
        return (Binding::Other, None);
    };
    if pointer.unknown || pointer.may_be_null || state.unresolved_effect_sites.contains(&site) {
        return (Binding::Other, None);
    }
    let Some(Family { held, .. }) = state.references.get(&site).copied() else {
        return (Binding::Other, None);
    };
    // For a keep or a drop, a family a structure other than names holds (a
    // field link, a container, a holder) counts as handed: a drop of it is
    // refused, as a release while held is (rule 12t).
    match handed(state, site) {
        Some(handed) => (
            Binding::Family {
                held,
                handed: handed || linked(state, site),
            },
            Some(site),
        ),
        None => (
            Binding::Family {
                held: Held::Undecided,
                handed: false,
            },
            Some(site),
        ),
    }
}

fn bind(state: &mut AbstractState, name: &str, site: usize) {
    state.pointers.insert(
        name.to_owned(),
        AbstractPointer {
            targets: BTreeSet::from([site]),
            unknown: false,
            may_be_null: false,
            derivation: Derivation::Identity,
        },
    );
}

fn is_plain(place: &str) -> bool {
    crate::may_alias::place_root(place) == Some(place)
}

#[allow(clippy::too_many_arguments)]
fn refuse(
    function: &str,
    line: usize,
    called: &str,
    text: &str,
    detail: &str,
    reason: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: refusal(called, text, detail, reason),
        },
    });
    state.unmodeled = true;
}

/// Hook 3 (before the call's reads are refused): whether `name` is the
/// actual of one of the call's drops, read past an acquired family's last
/// drop. Rule 13b decides that read itself (an over-drop), so the generic
/// read refusal leaves it alone.
pub(super) fn decides_read(instruction: &LegacyInstruction, name: &str) -> bool {
    let LegacyInstruction::ContractCall {
        retention: Some(operands),
        ..
    } = instruction
    else {
        return false;
    };
    operands.references.iter().any(|operand| {
        operand.operation == ReferenceOperation::Dropped && operand.name.as_deref() == Some(name)
    })
}

/// Hook 3: a declared reference-returning call (`resource_return_owned` with
/// the reference kind) starts an acquired family at the site it created.
pub(super) fn acquired(state: &mut AbstractState, site: usize) {
    let family = if state.references.contains_key(&site) {
        Family {
            held: Held::Undecided,
            acquired: true,
        }
    } else {
        Family::ACQUIRED
    };
    state.references.insert(site, family);
}

/// Hook 3: the call's keeps and drops, decided against `state`.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply(
    function: &str,
    line: usize,
    called: &str,
    operands: &[ReferenceOperand],
    destination: Option<&str>,
    produced_sites: &[usize],
    acquisition_lines: &BTreeMap<usize, usize>,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for operand in operands {
        let (found, site) = operand
            .name
            .as_deref()
            .map_or((Binding::Other, None), |name| binding(state, name));
        // A read of an invalidated name was refused before the call.
        let invalid = matches!(found, Binding::Invalid | Binding::Freed);
        match operand.operation {
            ReferenceOperation::Kept => {
                if let Some(destination) = destination.filter(|place| !is_plain(place)) {
                    refuse(
                        function,
                        line,
                        called,
                        &operand.text,
                        &format!(
                            "the reference is kept into `{destination}`, a place this route does not track"
                        ),
                        UNDECIDED,
                        state,
                        diagnostics,
                    );
                    continue;
                }
                let family = match decide_keep(found) {
                    KeepDecision::Mint => {
                        let (Some(name), Some(&minted)) =
                            (operand.name.as_deref(), produced_sites.last())
                        else {
                            refuse(
                                function,
                                line,
                                called,
                                &operand.text,
                                "the kept object is not a name this route reads",
                                UNDECIDED,
                                state,
                                diagnostics,
                            );
                            continue;
                        };
                        // A site minted again on a later iteration of a loop
                        // while the earlier family is still recorded: the two
                        // generations share it, so its count is undecided.
                        let family = if state.references.contains_key(&minted)
                            || state.resources.contains_key(&minted)
                        {
                            Family {
                                held: Held::Undecided,
                                acquired: false,
                            }
                        } else {
                            Family::KEPT
                        };
                        state.resources.insert(minted, LIVE);
                        state.nullable_sites.remove(&minted);
                        state.references.insert(minted, family);
                        bind(state, name, minted);
                        minted
                    }
                    KeepDecision::Add => {
                        let Some(site) = site else {
                            continue;
                        };
                        if let Some(family) = state.references.get_mut(&site) {
                            family.held = match family.held {
                                Held::Known(held) if held < HELD_BOUND => Held::Known(held + 1),
                                _ => Held::Undecided,
                            };
                        }
                        site
                    }
                    KeepDecision::Refuse(reason) => {
                        if !invalid {
                            refuse(
                                function,
                                line,
                                called,
                                &operand.text,
                                "the kept object is not a name this route reads, or its count is undecided",
                                reason,
                                state,
                                diagnostics,
                            );
                        }
                        continue;
                    }
                };
                if let Some(destination) = destination {
                    bind(state, destination, family);
                }
            }
            ReferenceOperation::Dropped => match decide_drop(found) {
                DropDecision::Decrement => {
                    if let Some(site) = site
                        && let Some(family) = state.references.get_mut(&site)
                        && let Held::Known(held) = family.held
                    {
                        family.held = Held::Known(held - 1);
                    }
                }
                DropDecision::Final => {
                    let Some(site) = site else {
                        continue;
                    };
                    // Possibly final: the object may be freed. The family is
                    // discharged and every name bound to it is invalidated;
                    // an acquired family's names remember it, so a further
                    // drop through one is an over-drop.
                    let acquired = state
                        .references
                        .remove(&site)
                        .is_some_and(|family| family.acquired);
                    let reason = if acquired {
                        Invalid::ReferenceFreed { site }
                    } else {
                        Invalid::ReferenceDropped
                    };
                    state.resources.remove(&site);
                    state.nullable_sites.remove(&site);
                    // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule
                    // 13e): the last reference this function holds on an
                    // object it acquired is dropped, so the object it held
                    // other pointers in is, as far as this function can see,
                    // gone (premise P-RESULT-HOLDER-LAST-DROP): what it
                    // retains is no longer this function's to outlive. A
                    // borrowed family's object lives on with the caller's
                    // reference, so its records stand.
                    if acquired {
                        super::retained_by_route::holder_released(state, site);
                    }
                    let names = state
                        .pointers
                        .iter()
                        .filter(|(_, pointer)| pointer.targets.contains(&site))
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>();
                    for name in names {
                        state.pointers.remove(&name);
                        state.invalidated.insert(name, reason);
                    }
                }
                // A drop past an acquired family's last drop: every reference
                // the function could see is already dropped. A violation
                // candidate on a path that refused nothing, provisional and
                // never an upstream defect; a refusal on any other path.
                DropDecision::OverDrop => {
                    if state.unmodeled {
                        refuse(
                            function,
                            line,
                            called,
                            &operand.text,
                            "a drop past the last drop of a reference this function acquired",
                            DROPPED,
                            state,
                            diagnostics,
                        );
                    } else if let Some(site) = site {
                        diagnostics.insert(LegacyDiagnostic {
                            function: function.to_owned(),
                            line,
                            kind: LegacyDiagnosticKind::DoubleRelease {
                                acquisition_line: super::acquisition_line(acquisition_lines, site),
                            },
                        });
                    }
                }
                DropDecision::Refuse(reason) => {
                    if !invalid {
                        refuse(
                            function,
                            line,
                            called,
                            &operand.text,
                            "a drop of a reference this function does not hold, or whose count is undecided",
                            reason,
                            state,
                            diagnostics,
                        );
                    }
                }
            },
        }
    }
}

/// Hook 4: at an exit, what each family still owes: a leak candidate on a
/// path with no refusal, `reference-unbalanced` on a path with one.
pub(super) fn report_exit(
    function: &str,
    line: usize,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (&site, &Family { held, .. }) in &state.references {
        if state.unresolved_effect_sites.contains(&site) {
            continue;
        }
        let owes = handed(state, site).and_then(|handed| owed(held, handed));
        let kind = match decide_exit(owes, !state.unmodeled) {
            ExitOutcome::Balanced => continue,
            ExitOutcome::Leak => LegacyDiagnosticKind::Leak {
                acquisition_line: super::acquisition_line(acquisition_lines, site),
            },
            ExitOutcome::Refused => LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "a reference taken at line {} may still be owed at this exit ({UNBALANCED})",
                    super::acquisition_line(acquisition_lines, site)
                ),
            },
        };
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind,
        });
    }
}

/// Hook 5: two paths' families, by site; counts that disagree are undecided.
pub(super) fn join(state: &mut AbstractState, incoming: &AbstractState) {
    for (&site, &family) in &incoming.references {
        state
            .references
            .entry(site)
            .and_modify(|current| *current = current.join(family))
            .or_insert(family);
    }
}

/// Hook 6's kind table entry.
pub(super) fn site_kind(instruction: &LegacyInstruction) -> Option<(usize, ResourceKind)> {
    reserved_site(instruction).map(|site| (site, ResourceKind::Reference))
}

/// The callee of the call whose argument list the `(` at `open` opens.
fn callee_of<'a>(body: &[&'a str], open: usize) -> Option<&'a str> {
    open.checked_sub(1).map(|at| body[at]).filter(|word| {
        is_identifier(word)
            && !matches!(
                *word,
                "if" | "while" | "for" | "switch" | "return" | "sizeof"
            )
    })
}

/// The `(` enclosing position `at`, at depth zero to its left.
fn enclosing_open(body: &[&str], at: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..at).rev() {
        match body[index] {
            ")" => depth += 1,
            "(" if depth == 0 => return Some(index),
            "(" => depth -= 1,
            ";" | "{" | "}" if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// Whether the token pair at `at` spells an assignment operator `=` (not
/// `==`, `!=`, `<=`, `>=`, which the lexer emits byte by byte).
fn assignment_at(body: &[&str], at: usize) -> bool {
    body.get(at) == Some(&"=")
        && body.get(at + 1) != Some(&"=")
        && !at
            .checked_sub(1)
            .is_some_and(|before| matches!(body[before], "=" | "!" | "<" | ">"))
}

/// Rule 13c: whether a keep's or a drop's body validates the clause on the
/// parameter `name`. Structural: `name` is never assigned, never has its
/// address taken, is never stored or returned (except a keep's `return
/// name;`), and is mentioned only through member accesses, as a whole actual
/// of a call, or in a null test; the body has no `goto`; and it touches the
/// object (a member access, or a whole actual at the same reference position
/// of a declared callee).
pub(super) fn validated(
    body: &[&str],
    name: &str,
    operation: ReferenceOperation,
    alias: bool,
    contracts: &ResourceContracts,
) -> bool {
    if name.is_empty() || body.contains(&"goto") {
        return false;
    }
    let mut touched = false;
    for (at, word) in body.iter().enumerate() {
        if *word == "return" {
            let end = body[at..]
                .iter()
                .position(|word| *word == ";")
                .map_or(body.len(), |offset| at + offset);
            let mut returned = &body[at + 1..end];
            while returned.len() >= 2 && returned[0] == "(" && returned[returned.len() - 1] == ")" {
                returned = &returned[1..returned.len() - 1];
            }
            let admitted = if operation == ReferenceOperation::Kept && alias {
                returned == [name]
            } else {
                returned.is_empty()
            };
            if !admitted {
                return false;
            }
        }
        if *word != name {
            continue;
        }
        let previous = at.checked_sub(1).map(|before| body[before]);
        let next = body.get(at + 1).copied();
        if next == Some("->") {
            touched = true;
            continue;
        }
        if previous == Some("return") {
            continue;
        }
        if previous == Some("&") || assignment_at(body, at + 1) {
            return false;
        }
        if previous.is_some_and(|_| assignment_at(body, at - 1)) {
            return false;
        }
        // A null test: `!name`, `name == …`, `name != …`, `if (name)`.
        if previous == Some("!")
            || (next == Some("=") && body.get(at + 2) == Some(&"="))
            || (next == Some("!") && body.get(at + 2) == Some(&"="))
        {
            continue;
        }
        if matches!(previous, Some("(" | ",")) && matches!(next, Some(")" | ",")) {
            let Some(open) = enclosing_open(body, at) else {
                return false;
            };
            match callee_of(body, open) {
                // A whole actual of a call; forwarding to the same operation
                // of a declared callee touches the object.
                Some(callee) => {
                    let position = body[open + 1..at]
                        .iter()
                        .fold((0usize, 0usize), |(depth, commas), word| match *word {
                            "(" | "[" | "{" => (depth + 1, commas),
                            ")" | "]" | "}" => (depth.saturating_sub(1), commas),
                            "," if depth == 0 => (depth, commas + 1),
                            _ => (depth, commas),
                        })
                        .1;
                    if contracts
                        .retaining_contract(callee)
                        .is_some_and(|declared| {
                            declared.references.iter().any(|clause| {
                                clause.operation == operation && clause.parameter == position
                            })
                        })
                    {
                        touched = true;
                    }
                    continue;
                }
                // `if (name)`, `while (name)`: a null test.
                None if open
                    .checked_sub(1)
                    .is_some_and(|before| matches!(body[before], "if" | "while")) =>
                {
                    continue;
                }
                None => return false,
            }
        }
        return false;
    }
    touched
}

/// Hook 7 (rule 13c): in a unit that defines a reference declaration's body,
/// the declaration stands only when the body validates it; otherwise it is
/// marked withdrawn there, so every call refuses and no inference reads the
/// body in its place. `None` when nothing is withdrawn.
pub(crate) fn settle(tokens: &[Token], contracts: &ResourceContracts) -> Option<ResourceContracts> {
    // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule 13d): returned
    // references first, then the keeps and drops over that environment.
    let results = super::reference_result_route::settle(tokens, contracts);
    let contracts_after_results = results.as_ref().unwrap_or(contracts);
    settle_clauses(tokens, contracts_after_results).or(results)
}

/// Rule 13c proper: the keep and drop clauses.
fn settle_clauses(tokens: &[Token], contracts: &ResourceContracts) -> Option<ResourceContracts> {
    let mut withdrawn = Vec::new();
    for (name, declaration) in contracts.retaining() {
        if declaration.references.is_empty() || declaration.references_withdrawn {
            continue;
        }
        let Some((parameters, body)) = super::slot_move_route::defined_body(tokens, name) else {
            continue;
        };
        let alias = matches!(
            declaration.contract.result,
            ResourceReturnMode::Derived { .. }
        );
        let valid = declaration.references.iter().all(|clause| {
            validated(
                &body,
                parameters.get(clause.parameter).map_or("", String::as_str),
                clause.operation,
                alias,
                contracts,
            )
        });
        if !valid {
            withdrawn.push((name.clone(), declaration.clone()));
        } else {
            // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g).
            crate::contract_dependencies::validated(name, "13c");
        }
    }
    if withdrawn.is_empty() {
        return None;
    }
    let mut settled = contracts.clone();
    for (name, mut declaration) in withdrawn {
        crate::contract_dependencies::withdrawn(&name, "13c", true);
        declaration.references_withdrawn = true;
        settled.insert_retaining(name, declaration);
    }
    Some(settled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    fn empty() -> ResourceContracts {
        ResourceContracts::default()
    }

    #[test]
    fn the_measured_keep_bodies_validate() {
        let keep0 = words("{ rd_atomic32_add ( & rktp -> rktp_refcnt , 1 ) ; return rktp ; }");
        assert!(validated(
            &keep0,
            "rktp",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
        let q_keep = words(
            "{ mtx_lock ( & rkq -> rkq_lock ) ; rkq -> rkq_refcnt ++ ; \
             mtx_unlock ( & rkq -> rkq_lock ) ; return rkq ; }",
        );
        assert!(validated(
            &q_keep,
            "rkq",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
        let topic_keep = words(
            "{ rd_kafka_lwtopic_t * lrkt ; if ( __builtin_expect ( ! ! ( ( lrkt = \
             rd_kafka_rkt_get_lw ( rkt ) ) != NULL ) , 0 ) ) rd_kafka_lwtopic_keep ( lrkt ) ; \
             else rd_atomic32_add ( & rkt -> rkt_refcnt , 1 ) ; return rkt ; }",
        );
        assert!(validated(
            &topic_keep,
            "rkt",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
    }

    #[test]
    fn the_measured_drop_bodies_validate() {
        let destroy0 = words(
            "{ if ( __builtin_expect ( ! ! ( rd_refcnt_sub0 ( & rktp -> rktp_refcnt ) == 0 ) , 0 ) ) \
             rd_kafka_toppar_destroy_final ( rktp ) ; }",
        );
        assert!(validated(
            &destroy0,
            "rktp",
            ReferenceOperation::Dropped,
            false,
            &empty()
        ));
    }

    #[test]
    fn a_body_that_stores_or_returns_the_object_is_refused() {
        let stores =
            words("{ holder -> p = rktp ; rd_atomic32_add ( & rktp -> n , 1 ) ; return rktp ; }");
        assert!(!validated(
            &stores,
            "rktp",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
        let returns_other = words("{ rd_atomic32_add ( & rktp -> n , 1 ) ; return other ; }");
        assert!(!validated(
            &returns_other,
            "rktp",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
        let rebinds = words("{ rktp = other ; rktp -> n ++ ; return rktp ; }");
        assert!(!validated(
            &rebinds,
            "rktp",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
        let address = words("{ f ( & rktp ) ; rktp -> n -- ; }");
        assert!(!validated(
            &address,
            "rktp",
            ReferenceOperation::Dropped,
            false,
            &empty()
        ));
        let returns = words("{ rktp -> n -- ; return rktp ; }");
        assert!(!validated(
            &returns,
            "rktp",
            ReferenceOperation::Dropped,
            false,
            &empty()
        ));
        let untouched = words("{ return rktp ; }");
        assert!(!validated(
            &untouched,
            "rktp",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
        let jumps = words("{ rktp -> n ++ ; goto out ; out : return rktp ; }");
        assert!(!validated(
            &jumps,
            "rktp",
            ReferenceOperation::Kept,
            true,
            &empty()
        ));
    }

    #[test]
    fn actual_names() {
        assert_eq!(actual_name(&["rktp"]), Some("rktp".to_owned()));
        assert_eq!(
            actual_name(&["(", "rd_kafka_toppar_t", "*", ")", "elem"]),
            Some("elem".to_owned())
        );
        assert_eq!(actual_name(&["rkt", "->", "rkt_p"]), None);
        assert_eq!(actual_name(&["(", "x", ")", "elem"]), None);
    }
}
