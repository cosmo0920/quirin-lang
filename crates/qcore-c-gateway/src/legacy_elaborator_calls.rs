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

//! Card **CG05-ELABORATOR-CALL-FOOTPRINTS** (gateway rule 11n): the
//! elaborator's own call events carry their contract footprint, and a
//! sequenced piece holding calls is lowered through card CG03's one
//! application point (route G).
//!
//! # Where
//!
//! Only inside card CG02's statement route (rule 10s): the whole statement
//! is elaborated with the call footprints in force
//! ([`Parser::elaboration_context_with_calls`]), and a refused piece whose
//! graph holds a call is lowered here
//! ([`Parser::footprint_call_events`]) where CG02's lowering refused every
//! call (`call … unsequenced with another event (CG05)`). Every older route of
//! the statement and of each piece is asked first (hard invariant 3).
//!
//! # The order
//!
//! A call event's footprint is read exactly as card CG05's nested-call reading
//! reads a call of its tree (`nested_call_effects::call_footprint`): the
//! contract in force, an identity at a `Borrowed` formal read and at any
//! other formal written, `&x` and an array decay a write of the object's
//! storage, an owned result a written fresh identity. Every other event's
//! footprint is `elaboration::event_footprint`'s: a scalar local's read or
//! write touches its own storage only, which overlaps no identity when the
//! local's address is never taken. `elaborate` then asks
//! `EvalGraph::check_with_footprints`: a call pair commutes only by its
//! footprints, every other pair exactly as CG18's `check_with`.
//!
//! # The lowering
//!
//! The admitted graph is lowered in its event order, which reaches one state
//! in every order C permits (`QCore.ElaboratorCallFootprints`): each read or
//! write as CG02 lowers it, each call by `Parser::instantiate_call` with use
//! `Tested` (an operator or a store consumes its value; it must own nothing),
//! its scalar actuals at `Borrowed` formals data (their reads lowered as
//! events) and its pointer actuals the classifier's own substitution. The
//! contract applied must be the one the footprint was read from.
//!
//! # What stays refused, by name
//!
//! A call with no footprint (no contract in force, an indirect callee, a
//! container effect, a reallocated result, an arity mismatch), a call whose
//! actuals carry effects (CG03's), a pair whose footprints do not commute,
//! and every refusal of the application point.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};

use super::{LegacyInstruction, Parser, Token};
use crate::call_instantiation::ResultUse;
use crate::elaboration::{Elaboration, Event, Refusal};
use crate::nested_call_effects::{CallFacts, Facts};
use crate::typed_access_path::{FieldFacts, StatementIndex, StatementPlan, is_pointer};
use crate::typed_c_ast::children;

/// The contract facts the elaborator's call footprints are read from: the
/// unit's contracts in force and declared data tails, its record fields and
/// the function's aliasing (CG02's `aliasing`).
pub(super) struct ElaboratorFacts<'a> {
    pub(super) contracts: &'a ResourceContracts,
    pub(super) tails: &'a BTreeSet<String>,
    pub(super) fields: &'a BTreeMap<String, Option<FieldFacts>>,
    pub(super) automatic: &'a BTreeSet<String>,
    pub(super) taken: &'a BTreeSet<String>,
}

impl Facts for ElaboratorFacts<'_> {
    fn contract(&self, callee: &str) -> Option<CallFacts> {
        let contract = self.contracts.contract(callee)?;
        Some(CallFacts {
            modes: contract.parameters.clone(),
            result: contract.result,
            tail: self.tails.contains(callee),
            container: contract.has_container_effect(),
        })
    }

    fn unaliased(&self, decl: &str) -> bool {
        self.automatic.contains(decl) && !self.taken.contains(decl)
    }

    fn fields(&self) -> &BTreeMap<String, Option<FieldFacts>> {
        self.fields
    }
}

/// The contracts the G context reads call footprints from.
#[derive(Clone, Copy)]
pub(super) struct CallContracts<'a> {
    pub(super) contracts: &'a ResourceContracts,
    pub(super) tails: &'a BTreeSet<String>,
}

impl Parser<'_> {
    /// The contracts this route's footprints are read from.
    pub(super) fn call_contracts(&self) -> CallContracts<'_> {
        CallContracts {
            contracts: self.contracts,
            tails: &self.variadic_tails,
        }
    }

    /// Lower a refused piece whose elaborated graph holds a call: `Ok(None)`
    /// when it holds none (CG02's lowering decides it), a refusal when a call
    /// cannot be lowered here, else every event in the graph's order.
    pub(super) fn footprint_call_events(
        &mut self,
        elaborated: &Elaboration<StatementPlan>,
        index: &StatementIndex,
        line: usize,
    ) -> Result<Option<Vec<LegacyInstruction>>, Refusal> {
        let events = &elaborated.graph.events;
        if !events
            .iter()
            .any(|event| matches!(event, Event::Call { .. }))
        {
            return Ok(None);
        }
        // Every call carries its footprint and reads only (its effectful
        // actuals are CG03's).
        for (id, event) in events.iter().enumerate() {
            let Event::Call { callee } = event else {
                continue;
            };
            if !elaborated.call_footprints.contains_key(&id) {
                return Err(Refusal::UnorderedCall(callee.clone()));
            }
            if elaborated.actuals.get(&id).is_some_and(|actuals| {
                actuals
                    .iter()
                    .any(|&actual| !matches!(events[actual], Event::Read(_)))
            }) {
                return Err(Refusal::EffectfulActual(callee.clone()));
            }
        }
        let saved = self.probe_state();
        let mut instructions = Vec::new();
        for (id, event) in events.iter().enumerate() {
            let step = match event {
                Event::Call { callee } => self.footprint_call(elaborated, index, id, callee, line),
                _ => self
                    .lower_event(elaborated, id)
                    .map(|instruction| instruction.into_iter().collect()),
            };
            match step {
                Ok(lowered) => instructions.extend(lowered),
                Err(refusal) => {
                    self.restore_probe_state(&saved);
                    return Err(refusal);
                }
            }
        }
        Ok(Some(instructions))
    }

    /// One call event, instantiated through the one application point with
    /// use `Tested`.
    fn footprint_call(
        &mut self,
        elaborated: &Elaboration<StatementPlan>,
        index: &StatementIndex,
        id: usize,
        callee: &str,
        line: usize,
    ) -> Result<Vec<LegacyInstruction>, Refusal> {
        let declined = Refusal::CallLowering;
        let node = elaborated
            .call_nodes
            .get(&id)
            .ok_or_else(|| Refusal::UnorderedCall(callee.to_owned()))?;
        let contract = self
            .contracts
            .contract(callee)
            .cloned()
            .ok_or_else(|| Refusal::UnorderedCall(callee.to_owned()))?;
        let actuals = children(node).get(1..).unwrap_or_default();
        let tail = self.variadic_tails.contains(callee);
        if actuals.len() != contract.parameters.len()
            && !(tail && actuals.len() > contract.parameters.len())
        {
            return Err(declined(format!(
                "call `{callee}`: actuals do not match the formals"
            )));
        }
        let at = |text: &str| Token {
            text: text.to_owned(),
            line,
        };
        // The call's tokens with each scalar actual at a `Borrowed` formal (or
        // in a declared data tail) a neutral literal: its reads were lowered
        // as events. A pointer actual keeps its spelling, so the classifier
        // substitutes it.
        let mut tokens = vec![at(callee), at("(")];
        for (formal, actual) in actuals.iter().enumerate() {
            if formal > 0 {
                tokens.push(at(","));
            }
            let borrowed = contract
                .parameters
                .get(formal)
                .is_none_or(|mode| matches!(mode, ResourceParameterMode::Borrowed));
            if borrowed && !is_pointer(actual) {
                tokens.push(at("0"));
            } else {
                let texts = index
                    .node_tokens(actual)
                    .ok_or(Refusal::Unlisted("an actual without tokens"))?;
                tokens.extend(texts.into_iter().map(|text| Token { text, line }));
            }
        }
        tokens.push(at(")"));
        let sequence = self
            .instantiate_call(callee, &tokens, None, ResultUse::Tested)
            .map_err(|refusal| declined(format!("call `{callee}`: {refusal:?}")))?;
        // The contract applied is the one the footprint was read from.
        match sequence.last() {
            Some(LegacyInstruction::ContractCall {
                contract: applied, ..
            }) if applied.parameters == contract.parameters
                && applied.result == contract.result => {}
            _ => {
                return Err(declined(format!(
                    "call `{callee}`: the contract applied is not the one read"
                )));
            }
        }
        Ok(sequence)
    }
}
