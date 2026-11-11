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

//! Card **W45-FB-BACKLOG-CUSTODY**: checked specifications (contract spec
//! §C06.3, gateway constraint 5l).
//!
//! # Why a declaration is not enough for a body the audit reads
//!
//! A declared contract is applied at every call site whatever the callee's
//! own body proves: the mode gate (constraint 8) withdraws it only when a
//! *validated* W4 summary of the body contradicts it, and a body that does not
//! validate yields no summary, so its declaration stays trusted. For a
//! function the audit analyses, that makes the declaration a premise that
//! replaces the body. A declaration carrying `requires resource_body_checked;`
//! is instead a **specification**:
//!
//! * it is in force — at its own body's entry and at every call site in the
//!   unit — only while this unit defines the body, inside the derivation
//!   scope, outside every recursive component, and that body **establishes**
//!   it: analysed under the specification's entry custody, the body is fully
//!   modelled, reports nothing, and every exit matches every clause
//!   (`legacy_summary::observations_match_contract`, the W4 gate);
//! * otherwise it is **withdrawn** from the unit's environment, exactly as the
//!   ABI gate withdraws a contradicted declaration; callers then see whatever
//!   W4 validates from the body, or an uncontracted call, and the defining
//!   function's row carries [`not_established`], so it is never `Safe`.
//!
//! Settling is a descent: every round removes at least one specification or
//! stops, and a round only ever validates a body against specifications that
//! are themselves still standing. Recursive components are withdrawn before
//! the first round, so no specification validates through itself (constraint
//! 4: no assume-guarantee fixed point above the neutral element). The
//! surviving set is therefore well founded on the acyclic call graph.
//! `QCore.CheckedSpecification` models the rule.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::ResourceContracts;

use super::Token;
use super::declared_modes::ModeGate;

/// Why a checked specification is not in force in this unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Withdrawal {
    /// The environment's retained texts could not be classified.
    Unreadable,
    /// This unit does not define the body inside its derivation scope.
    NoBody,
    /// The body is a member of a recursive component of the unit.
    Recursive,
    /// The body does not establish the specification.
    NotEstablished,
    /// Card RULE37-PILOT: the declaration names a premise a gateway rule
    /// package owns (`asserted_allocation`, rule 37), and that package does
    /// not admit it (switched off), or admits it with an obligation the
    /// declaration does not state.
    PremiseNotAdmitted,
}

impl Withdrawal {
    fn reason(self) -> &'static str {
        match self {
            Self::Unreadable => "its declaring texts could not be classified",
            Self::NoBody => "this unit does not define its body",
            Self::Recursive => "its body is in a recursive component",
            Self::NotEstablished => {
                "its body, analysed under it, is not fully modelled, reports a diagnostic or \
                 leaves an exit that does not match it"
            }
            Self::PremiseNotAdmitted => {
                "the gateway rule owning its named premise does not admit it (switched off)"
            }
        }
    }
}

/// The diagnostic a defining function's row carries when its specification is
/// withdrawn.
#[must_use]
pub fn not_established(function: &str, reason: &str) -> String {
    format!("declared specification of `{function}` is not established: {reason}")
}

/// The outcome of settling one unit.
pub(crate) struct Settled {
    pub(crate) gate: ModeGate,
    /// Every withdrawn specification, with its reason.
    pub(crate) withdrawn: BTreeMap<String, Withdrawal>,
    /// Card ARM-OBLIGATIONS: the obligations of the rule packages' admissions
    /// of the specifications that stand (rule 37's `asserted_allocation`).
    /// Card ARM-OBLIGATIONS-UNIT-GRAIN: per specification, so that the rows
    /// whose derivation reaches its name carry them. A withdrawn
    /// specification's admission has no row resting on it and contributes
    /// nothing.
    pub(crate) carried: BTreeMap<String, crate::arm_obligations::Carried>,
}

/// The obligations the standing specifications' admissions rest on, by
/// specification.
fn standing_obligations(
    admitted: &BTreeMap<String, crate::arm_obligations::Carried>,
    withdrawn: &BTreeMap<String, Withdrawal>,
) -> BTreeMap<String, crate::arm_obligations::Carried> {
    admitted
        .iter()
        .filter(|(name, obligations)| !withdrawn.contains_key(*name) && !obligations.is_empty())
        .map(|(name, obligations)| (name.clone(), obligations.clone()))
        .collect()
}

/// The diagnostic text for a withdrawn specification defined here.
pub(crate) fn diagnostic(
    withdrawn: &BTreeMap<String, Withdrawal>,
    function: &str,
) -> Option<String> {
    withdrawn
        .get(function)
        .map(|withdrawal| not_established(function, withdrawal.reason()))
}

fn reduced(
    contracts: &ResourceContracts,
    withdrawn: &BTreeMap<String, Withdrawal>,
) -> ResourceContracts {
    let mut reduced = ResourceContracts::default();
    for (name, contract) in contracts.functions() {
        if !withdrawn.contains_key(name) {
            reduced.insert_contract(name, contract.clone());
        }
    }
    // Card CALLEE-RETENTION-CLASS1: the dispositions and retaining
    // declarations stay in force.
    reduced.carry_side_declarations(contracts, |_| false);
    reduced.set_source_assumptions(contracts.source_assumptions().clone());
    reduced.carry_cross_unit_imports(contracts);
    reduced
}

/// Names of `scope` that reach themselves through direct calls in the unit.
fn recursive_members(tokens: &[Token], scope: &[String]) -> BTreeSet<String> {
    let candidates = scope.iter().cloned().collect::<BTreeSet<_>>();
    let edges = scope
        .iter()
        .map(|name| (name.clone(), super::direct_calls(tokens, name, &candidates)))
        .collect::<BTreeMap<_, _>>();
    let mut recursive = BTreeSet::new();
    for start in scope {
        let mut seen = BTreeSet::new();
        let mut work = edges
            .get(start)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        while let Some(next) = work.pop() {
            if next == *start {
                recursive.insert(start.clone());
                break;
            }
            if seen.insert(next.clone()) {
                work.extend(edges.get(&next).cloned().unwrap_or_default());
            }
        }
    }
    recursive
}

/// Whether `function`'s body establishes the specification in force for it in
/// `gate`'s derivation.
fn establishes(
    tokens: &[Token],
    function: &str,
    gate: &ModeGate,
    declared: &ResourceContracts,
) -> bool {
    let tokens = gate.tokens.as_deref().unwrap_or(tokens);
    let derivation = &gate.derivation;
    let (Some(contract), Some(specification)) = (
        derivation.entry_contract(function),
        declared.contract(function),
    ) else {
        return false;
    };
    if contract != specification || derivation.conflict(function).is_some() {
        return false;
    }
    let Some(parameters) = super::function_parameters(tokens, function) else {
        return false;
    };
    let cfg = match derivation.contextual_cfg(function) {
        Some(cfg) => cfg.clone(),
        None => match super::build_function_cfg(tokens, function, derivation.environment()) {
            Ok(cfg) => cfg,
            Err(_) => return false,
        },
    };
    let Some(entry) = super::EntryCustody::build(&cfg, &parameters, contract) else {
        return false;
    };
    let (candidate, observations) = super::analyze_cfg_with_entry(cfg, &entry);
    crate::legacy_summary::observations_match_contract(contract, &entry, &candidate, &observations)
}

/// Settle the checked specifications of one unit and return the mode gate
/// over the environment that keeps exactly the established ones.
pub(crate) fn settle(tokens: &[Token], scope: &[String], contracts: &ResourceContracts) -> Settled {
    let checked = match qcore_contract::contract_loader::declared_checked_specifications(contracts)
    {
        Ok(checked) => checked,
        // Unreachable for an environment the loader built from the same texts;
        // the fail-closed answer is to withdraw every declaration.
        Err(_) => {
            let withdrawn = contracts
                .functions()
                .keys()
                .map(|name| (name.clone(), Withdrawal::Unreadable))
                .collect::<BTreeMap<_, _>>();
            let environment = reduced(contracts, &withdrawn);
            return Settled {
                gate: ModeGate::apply(tokens, scope, &environment),
                withdrawn,
                carried: BTreeMap::new(),
            };
        }
    };
    let checked = checked
        .into_iter()
        .filter(|name| contracts.contract(name).is_some())
        .collect::<BTreeSet<_>>();
    if checked.is_empty() {
        return Settled {
            gate: ModeGate::apply(tokens, scope, contracts),
            withdrawn: BTreeMap::new(),
            carried: BTreeMap::new(),
        };
    }
    let mut withdrawn = BTreeMap::new();
    let recursive = recursive_members(tokens, scope);
    for name in &checked {
        if !scope.contains(name) {
            withdrawn.insert(name.clone(), Withdrawal::NoBody);
        } else if recursive.contains(name) {
            withdrawn.insert(name.clone(), Withdrawal::Recursive);
        }
    }
    // Card RULE37-PILOT: a specification naming a premise a rule package owns
    // stands only while that package admits it (rule 37's
    // `asserted_allocation`). Unreadable premises withdraw every checked
    // specification, as an unreadable checked set does.
    // Card ARM-OBLIGATIONS: an admitted one keeps its admission's
    // obligations, carried below while it stands.
    let mut admitted = BTreeMap::new();
    match crate::result_constructor_rule::premise_settlement(contracts, &checked) {
        Some(settlement) => {
            for name in settlement.withdrawn {
                withdrawn
                    .entry(name)
                    .or_insert(Withdrawal::PremiseNotAdmitted);
            }
            admitted = settlement.admitted;
        }
        None => {
            for name in &checked {
                withdrawn
                    .entry(name.clone())
                    .or_insert(Withdrawal::Unreadable);
            }
        }
    }
    // Each round that continues withdraws at least one of `checked`, so
    // `checked.len() + 1` rounds bound the descent. Falling out of the loop
    // re-derives over the final withdrawals, which only grew.
    for _ in 0..=checked.len() {
        let environment = reduced(contracts, &withdrawn);
        let gate = ModeGate::apply(tokens, scope, &environment);
        let failing = checked
            .iter()
            .filter(|name| !withdrawn.contains_key(*name))
            .filter(|name| !establishes(tokens, name, &gate, contracts))
            .cloned()
            .collect::<Vec<_>>();
        if failing.is_empty() {
            let carried = standing_obligations(&admitted, &withdrawn);
            return Settled {
                gate,
                withdrawn,
                carried,
            };
        }
        for name in failing {
            withdrawn.insert(name, Withdrawal::NotEstablished);
        }
    }
    Settled {
        gate: ModeGate::apply(tokens, scope, &reduced(contracts, &withdrawn)),
        carried: standing_obligations(&admitted, &withdrawn),
        withdrawn,
    }
}
