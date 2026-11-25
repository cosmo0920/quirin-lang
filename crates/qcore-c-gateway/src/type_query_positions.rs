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

//! Card **K2-NO-TYPED-ACCESS** (gateway rule 79): two positions where a
//! `sizeof` type query was read as an access it is not.
//!
//! USER DECISION 2026-11-25 (verbatim): "Build the 2 rows". Every condition
//! and named refusal below is a formula of the C04.18.9 typed rendering over
//! the K2 terms (`qcore_contract::premise_formula::RENDERED_TERMS`, block
//! `K2`), checked by the premise-formula validator ([`validated`]); the prose
//! beside each is a gloss.
//!
//! ## (1) A connective condition with a type query
//!
//! `if ((sizeof(reg_t) == 8) && (tableType != byU16))`: rule 5h's leaf reader
//! refuses the `sizeof` leaf and the elaborator's condition lowering
//! (rule 10s) lowers no `Branch`/`Merge` of the connective's value region
//! (rule 10z). The condition is admitted, as one branch that refines neither
//! edge and lowers no instruction, when every formula of [`CONDITIONS`]
//! holds:
//!
//! | id | formula |
//! |---|---|
//! | `k2_region_site` | `((#k2_older_refused(#k2_condition) && #k2_region(#k2_condition)) && #k2_scalar_result(#k2_condition))` |
//! | `k2_type_query_present` | `exists (#q: k2_query) { #k2_query_of(#q,#k2_condition) }` |
//! | `k2_queries_unevaluated` | `forall (#q: k2_query) { (#k2_query_of(#q,#k2_condition) => (#k2_type_operand(#q) && !#k2_variably_modified(#q))) }` |
//! | `k2_events_inert` | `forall (#e: k2_event) { (#k2_event_of(#e,#k2_condition) => (#k2_branch_event(#e) \|\| (#k2_object_read(#e) && #k2_unaliased(#e)))) }` |
//!
//! *Soundness.* A type query of a type that is not variably modified is not
//! evaluated (C11 6.5.3.4p2): it reads, writes and calls nothing. Every other
//! event of the condition, on every arm, is a `Branch` or `Merge` (no
//! storage) or the read of an automatic object through its own name whose
//! address is never taken: no lvalue but its name designates it, so the read
//! is no dereference, needs no non-null or live obligation, and changes no
//! abstract state. Whichever arm C evaluates, the abstract state after the
//! condition is the state before it and no obligation arises, so lowering the
//! condition as no instruction followed by a branch that refines neither edge
//! is the condition on both edges (`QCore.TypeQueryPositions`,
//! `an_admitted_condition_is_the_identity_on_every_path`,
//! `an_admitted_condition_owes_nothing`). Not refining an edge keeps both
//! successors reachable: it can lose precision, never a path.
//!
//! ## (2) `NAME * sizeof` begins no declaration
//!
//! `size = sizeof(*list) + (capacity * sizeof(T *));`: the total-or-reject
//! declaration reader (`typed_field_access::read_locals`) tries every run
//! after `(` as a declaration, reads `capacity * sizeof ( T * )` as one, fails,
//! and refuses the whole function's declared types; the elaborator then
//! refuses the store into `size` ("write to an object the gateway does not
//! read as a scalar local"). [`product_run`] answers the K2 formula
//!
//! | id | formula |
//! |---|---|
//! | `k2_product_run_is_no_declaration` | `(#k2_name_star_sizeof(#k2_run_at) => !#k2_declaration(#k2_run_at))` |
//!
//! and `is_declaration_candidate` drops such a run. *Soundness.* A
//! declaration that names a variable puts an identifier after the `*`; the
//! keyword `sizeof` is no identifier (C11 6.4.1p2, 6.7.6), so no declaration
//! is lost and every reader of the candidate keeps its total-or-reject
//! premise (`QCore.TypeQueryPositions.a_product_run_is_no_declaration`). What
//! the statement then means is decided by the routes that already read it:
//! the elaborator's `sizeof(*p)` is its type-query rule (unevaluated unless
//! variably modified, card CG08).
//!
//! ## Named refusals ([`Reason`])
//!
//! Each refusal is the formula [`Reason::formula`] names; the condition then
//! keeps the position's older refusal, byte for byte.
//!
//! The Lean model is `QCore.TypeQueryPositions`; [`DECISION_VECTORS`] and the
//! formula table are shared with it.

use std::sync::OnceLock;

use serde_json::Value;

use crate::elaboration::{Elaboration, Event, Place};

#[allow(dead_code, reason = "read by the unit tests and the Lean binding")]
/// The rule.
pub(crate) const RULE: &str = "79";

/// The admission conditions of (1), and (2)'s fact: id and formula.
pub(crate) const CONDITIONS: &[(&str, &str)] = &[
    (
        "k2_region_site",
        "((#k2_older_refused(#k2_condition) && #k2_region(#k2_condition)) && #k2_scalar_result(#k2_condition))",
    ),
    (
        "k2_type_query_present",
        "exists (#q: k2_query) { #k2_query_of(#q,#k2_condition) }",
    ),
    (
        "k2_queries_unevaluated",
        "forall (#q: k2_query) { (#k2_query_of(#q,#k2_condition) => (#k2_type_operand(#q) && !#k2_variably_modified(#q))) }",
    ),
    (
        "k2_events_inert",
        "forall (#e: k2_event) { (#k2_event_of(#e,#k2_condition) => (#k2_branch_event(#e) || (#k2_object_read(#e) && #k2_unaliased(#e)))) }",
    ),
    (
        "k2_product_run_is_no_declaration",
        "(#k2_name_star_sizeof(#k2_run_at) => !#k2_declaration(#k2_run_at))",
    ),
];

/// Why a region condition is not read by rule 79. Each reason is a formula
/// ([`Self::formula`]) whose truth refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Reason {
    /// The condition holds no type query.
    NoTypeQuery,
    /// A type query's operand is an expression, or its type may be variably
    /// modified.
    EvaluatedQuery,
    /// An event is a call, a store, a path read, a lifetime boundary, or the
    /// read of an object that is static, global or address-taken.
    Event,
}

impl Reason {
    pub(crate) const ALL: [Self; 3] = [Self::NoTypeQuery, Self::EvaluatedQuery, Self::Event];

    /// The reason's id.
    #[must_use]
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::NoTypeQuery => "k2_refuse_no_type_query",
            Self::EvaluatedQuery => "k2_refuse_evaluated_query",
            Self::Event => "k2_refuse_event",
        }
    }

    /// The formula whose truth is the refusal: the negation of one condition.
    #[must_use]
    pub(crate) fn formula(self) -> &'static str {
        match self {
            Self::NoTypeQuery => "forall (#q: k2_query) { !#k2_query_of(#q,#k2_condition) }",
            Self::EvaluatedQuery => {
                "exists (#q: k2_query) { (#k2_query_of(#q,#k2_condition) && (!#k2_type_operand(#q) || #k2_variably_modified(#q))) }"
            }
            Self::Event => {
                "exists (#e: k2_event) { (#k2_event_of(#e,#k2_condition) && (!#k2_branch_event(#e) && (!#k2_object_read(#e) || !#k2_unaliased(#e)))) }"
            }
        }
    }

    /// The condition of [`CONDITIONS`] this reason negates.
    #[must_use]
    #[allow(dead_code, reason = "read by the unit tests and the Lean binding")]
    pub(crate) fn condition(self) -> &'static str {
        match self {
            Self::NoTypeQuery => "k2_type_query_present",
            Self::EvaluatedQuery => "k2_queries_unevaluated",
            Self::Event => "k2_events_inert",
        }
    }
}

/// One type query of the condition: whether its operand is a type name, and
/// whether that type may be variably modified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Query {
    pub(crate) type_operand: bool,
    pub(crate) variably_modified: bool,
}

/// One event of the condition's elaboration, as the conditions read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventClass {
    /// A region's `Branch` or `Merge`.
    Branch,
    /// The read of a named object; `unaliased` is `Context::unaliased`.
    ObjectRead { unaliased: bool },
    /// Anything else: a call, a store, a path read, a lifetime boundary.
    Other,
}

/// The facts of one region condition (`k2_region_site` holds where the hook
/// asks: every older route refused, a region, a non-pointer result).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Facts {
    pub(crate) queries: Vec<Query>,
    pub(crate) events: Vec<EventClass>,
}

/// The decision: `Ok` admits; `Err` names the first failing condition, in the
/// order of [`CONDITIONS`].
pub(crate) fn decide(facts: &Facts) -> Result<(), Reason> {
    if facts.queries.is_empty() {
        return Err(Reason::NoTypeQuery);
    }
    if facts
        .queries
        .iter()
        .any(|q| !q.type_operand || q.variably_modified)
    {
        return Err(Reason::EvaluatedQuery);
    }
    if facts.events.iter().any(|event| {
        !matches!(
            event,
            EventClass::Branch | EventClass::ObjectRead { unaliased: true }
        )
    }) {
        return Err(Reason::Event);
    }
    Ok(())
}

/// The facts of a condition's Clang node and its elaboration.
pub(crate) fn facts<F>(
    node: &Value,
    elaborated: &Elaboration<F>,
    unaliased: &dyn Fn(&str) -> bool,
) -> Facts {
    let mut queries = Vec::new();
    collect_queries(node, &mut queries);
    let events = elaborated
        .graph
        .events
        .iter()
        .map(|event| match event {
            Event::Branch | Event::Merge => EventClass::Branch,
            Event::Read(Place::Object { decl, .. }) => EventClass::ObjectRead {
                unaliased: unaliased(decl),
            },
            _ => EventClass::Other,
        })
        .collect();
    Facts { queries, events }
}

pub(crate) fn collect_queries(node: &Value, queries: &mut Vec<Query>) {
    if node["kind"].as_str() == Some("UnaryExprOrTypeTraitExpr") {
        let inner = node["inner"].as_array().map_or(0, Vec::len);
        let spelled = |key: &str| node["argType"][key].as_str().unwrap_or_default();
        let type_operand = inner == 0 && node["argType"]["qualType"].is_string();
        queries.push(Query {
            type_operand,
            // A conservative reading of "variably modified": any array
            // declarator in Clang's spelling of the type or its desugaring.
            variably_modified: !type_operand
                || spelled("qualType").contains('[')
                || spelled("desugaredQualType").contains('['),
        });
    }
    if let Some(children) = node["inner"].as_array() {
        for child in children {
            collect_queries(child, queries);
        }
    }
}

/// Whether the run at `start` is `NAME * sizeof`, which begins no
/// declaration (`k2_product_run_is_no_declaration`).
pub(crate) fn product_run(texts: &[&str], start: usize) -> bool {
    matches!(
        (texts.get(start), texts.get(start + 1), texts.get(start + 2)),
        (Some(name), Some(&"*"), Some(&"sizeof")) if crate::typed_field_access::is_name(name)
    )
}

/// Every formula of the module: the conditions, then the refusals.
pub(crate) fn formulas() -> Vec<(&'static str, &'static str)> {
    CONDITIONS
        .iter()
        .copied()
        .chain(Reason::ALL.iter().map(|r| (r.id(), r.formula())))
        .collect()
}

/// Every formula of the module is the canonical printing of a C04.18.9
/// typed-rendering formula over registered terms (the premise-formula
/// validator, `guarded_proposition`). Read once.
pub(crate) fn validated() -> Result<(), String> {
    static VALIDATED: OnceLock<Result<(), String>> = OnceLock::new();
    VALIDATED
        .get_or_init(|| {
            for (id, formula) in formulas() {
                qcore_contract::premise_formula::guarded_proposition(id, formula)?;
            }
            Ok(())
        })
        .clone()
}

#[allow(dead_code, reason = "read by the unit tests and the Lean binding")]
/// The decision vectors shared with `QCore.TypeQueryPositions.decisionVectors`:
/// (name, the one fact that differs from an admitted condition, the answer).
pub(crate) const DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("admitted", "none", "admit"),
    ("no_query", "no type query", "k2_refuse_no_type_query"),
    (
        "expression_operand",
        "an expression operand",
        "k2_refuse_evaluated_query",
    ),
    (
        "vla_operand",
        "a variably modified operand",
        "k2_refuse_evaluated_query",
    ),
    ("call_event", "a call event", "k2_refuse_event"),
    (
        "aliased_read",
        "an address-taken object read",
        "k2_refuse_event",
    ),
];

#[allow(dead_code, reason = "read by the unit tests and the Lean binding")]
/// The facts of one decision vector.
pub(crate) fn vector_facts(differs: &str) -> Facts {
    let query = Query {
        type_operand: true,
        variably_modified: false,
    };
    let mut facts = Facts {
        queries: vec![query],
        events: vec![
            EventClass::ObjectRead { unaliased: true },
            EventClass::Branch,
            EventClass::ObjectRead { unaliased: true },
            EventClass::Branch,
        ],
    };
    match differs {
        "no type query" => facts.queries.clear(),
        "an expression operand" => {
            facts.queries[0] = Query {
                type_operand: false,
                variably_modified: true,
            };
        }
        "a variably modified operand" => facts.queries[0].variably_modified = true,
        "a call event" => facts.events[2] = EventClass::Other,
        "an address-taken object read" => {
            facts.events[2] = EventClass::ObjectRead { unaliased: false };
        }
        _ => {}
    }
    facts
}

#[allow(dead_code, reason = "read by the unit tests and the Lean binding")]
/// The answer code of a decision.
pub(crate) fn code(decision: Result<(), Reason>) -> &'static str {
    match decision {
        Ok(()) => "admit",
        Err(reason) => reason.id(),
    }
}
