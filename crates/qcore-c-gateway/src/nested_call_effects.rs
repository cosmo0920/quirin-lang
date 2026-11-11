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

//! Card **CG05-NESTED-CALL-EFFECTS** (gateway rule 10y): the non-identity
//! effects of nested and sibling calls (Clang gap entries E10, E11 and E12),
//! read from Clang's typed AST.
//!
//! # The reading
//!
//! [`read`] reads one call whose actuals hold calls into a **call tree**:
//! every call, its contract facts, and each actual as one of three forms.
//!
//! * **Plain** ([`Actual::Plain`]): an actual holding no evaluated call. The
//!   classifier substitutes it exactly as in a statement.
//! * **Bound** ([`Actual::Bound`]): the whole actual is one pointer-valued call,
//!   through parentheses and value-preserving pointer conversions only. Its
//!   value is bound to a reserved place (`__qcore_nested_value_N`), exactly as
//!   `T = g(p); f(T);` for a fresh `T` (the desugaring card
//!   C-GATEWAY-RETURN-CALL-W4 made for `return f(a);`): the inner call's
//!   contract is applied whole, and its result, with its identity and alias
//!   state, is the outer actual. The use is `Passed`.
//! * **Data** ([`Actual::Data`]): a scalar value holding calls
//!   (`f(g(p) + 1)`, `f(g(p))` with a scalar `g`). Its non-call part is read by
//!   card CG03's data reading with each call standing for a scalar; the value
//!   is data, so it may stand only at a `Borrowed` formal or a declared
//!   variadic data tail. A call whose value is the whole actual is `Passed`;
//!   one whose value an operator consumes is `Tested` (CG03's vocabulary).
//!
//! A call inside a call's actuals is read the same way, at any depth
//! (`f(g(h(p)))`), and its effects apply **inside-out**: C sequences every
//! evaluation of a call's actuals, calls included, before the call's entry
//! (C11 6.5.2.2p10).
//!
//! # The order (E12)
//!
//! The actuals of one call are unsequenced; a call inside one of them is
//! indeterminately sequenced with every evaluation of the others. [`read`]
//! builds card CG02's [`EvalGraph`] for the tree: each call is an event
//! sequenced after every event of its actuals, and the actuals of each call
//! are one unsequenced group. Every event carries a contract footprint
//! ([`Footprint`], rule 10y's cells) and the graph must pass
//! [`EvalGraph::check_footprints`]: every unordered pair must commute by its
//! footprints. A call's footprint is read from its contract and actuals:
//!
//! * an identity at a `Borrowed` formal is **read**; at any other formal it is
//!   **written** (released, consumed, stored, removed, owned across);
//! * `&x` of a pointer object is a write of `x`'s storage (an `Out` slot, or a
//!   callee that may store through the address);
//! * an owned result is a written **fresh** identity.
//!
//! What a call does beyond its contract is not read: its effect on the
//! abstract state is its contract's (constraint 4's trust), exactly as for a
//! call standing alone.
//!
//! A read of a pointer object reads its storage and the identity it denotes; a
//! scalar read is custody-neutral. Two identity cells always overlap (names
//! may alias: identity disjointness is not proved here), so two unordered calls
//! that touch identities commute only when neither writes one. No order C
//! leaves unspecified is ever chosen: the route lowers the tree in the graph's
//! event order only because an admitted graph reaches one state in every
//! permitted order (`QCore.NestedCallEffects.every_interleaving_runs_alike`).
//!
//! # What stays refused, by name
//!
//! An indirect callee (CG22), a callee with no contract in force, a container
//! effect, a reallocated result, a scalar owned result (a descriptor: the
//! descriptor route's), a data actual at a custody-taking formal, a data actual
//! CG03's data reading refuses (a place, pointer arithmetic, an effectful
//! form, a conditional, a cast), an arity mismatch without a declared tail,
//! and every unordered pair whose footprints do not commute. The Lean model is
//! `QCore.NestedCallEffects`, bound by the shared vectors
//! [`OVERLAP_VECTORS`] and [`ORDER_VECTORS`].

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};
use serde_json::{Value, json};

use crate::call_instantiation::{Actual as Substituted, ResultUse, read_actual};
use crate::elaboration::{
    Cell, EvalGraph, Event, EventId, Footprint, Place, Refusal as OrderRefusal, direct_callee,
    strip_parens,
};
use crate::typed_access_path::{FieldFacts, is_pointer, qual};
use crate::typed_c_ast::{children, kind};

/// The reserved place a bound nested result is written to. Reserved to the
/// implementation (C11 7.1.3) and checked unspelled before any admission.
pub(crate) const NESTED_VALUE_PREFIX: &str = "__qcore_nested_value_";

/// The contract facts of one callee, as the application point will apply them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CallFacts {
    pub(crate) modes: Vec<ResourceParameterMode>,
    pub(crate) result: ResourceReturnMode,
    /// A declared variadic data tail.
    pub(crate) tail: bool,
    /// A declared container effect (not listed here).
    pub(crate) container: bool,
}

/// What the reading asks of the route.
pub(crate) trait Facts {
    /// The contract in force for `callee`.
    fn contract(&self, callee: &str) -> Option<CallFacts>;
    /// An automatic object whose address is never taken.
    fn unaliased(&self, decl: &str) -> bool;
    /// The unit's record field facts (CG10's place reader).
    fn fields(&self) -> &BTreeMap<String, Option<FieldFacts>>;
}

/// One call of the tree.
#[derive(Clone, Debug)]
pub(crate) struct Call<'a> {
    pub(crate) callee: String,
    pub(crate) facts: CallFacts,
    /// This call's event in [`Reading::graph`].
    pub(crate) event: EventId,
    pub(crate) actuals: Vec<Actual<'a>>,
    /// The use C makes of this call's value (the root's is the consumer's).
    pub(crate) use_: Option<ResultUse>,
}

/// One actual of a call.
#[derive(Clone, Debug)]
pub(crate) enum Actual<'a> {
    /// No evaluated call: the classifier substitutes it.
    Plain(&'a Value),
    /// The whole actual is one pointer-valued call, bound to a reserved place.
    Bound(Box<Call<'a>>),
    /// A scalar value holding calls: data at a `Borrowed` formal.
    Data { calls: Vec<Call<'a>> },
}

/// A named refusal of the reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A form this card does not list, with the card that owns it.
    Form(&'static str, &'static str),
    /// A callee with no contract in force.
    Uncontracted(String),
    /// A contract clause this card does not compose.
    Clause(String, &'static str),
    /// Actuals do not match the declared formals.
    Arity(String),
    /// Data at a custody-taking formal.
    DataAtCustody(String),
    /// CG03's data reading refused the actual.
    Data(String),
    /// The order model refused the graph.
    Order(OrderRefusal),
    /// No call nested at all: not this card's statement.
    NotNested,
}

impl Refusal {
    pub(crate) fn reason(&self) -> String {
        match self {
            Self::Form(form, owner) => format!("{form} ({owner})"),
            Self::Uncontracted(callee) => {
                format!("nested call `{callee}` has no contract in force")
            }
            Self::Clause(callee, clause) => format!("call `{callee}`: {clause} (not composed)"),
            Self::Arity(callee) => format!("call `{callee}`: actuals do not match the formals"),
            Self::DataAtCustody(callee) => {
                format!("call `{callee}`: a value holding calls at a custody-taking formal")
            }
            Self::Data(reason) => format!("data actual: {reason}"),
            Self::Order(refusal) => refusal.reason(),
            Self::NotNested => "no nested call".to_owned(),
        }
    }
}

/// The reading of one call statement's call tree.
#[derive(Clone, Debug)]
pub(crate) struct Reading<'a> {
    pub(crate) root: Call<'a>,
    /// The order-checked graph (read by the tests).
    #[allow(dead_code, reason = "read by the tests")]
    pub(crate) graph: EvalGraph,
    /// The reserved place each bound call's value is written to, by event.
    pub(crate) places: BTreeMap<EventId, String>,
}

/// A `_Generic` selection's selected association, by Clang's own `selected`
/// mark: only it is evaluated (C11 6.5.1.1), never the controlling expression.
fn selected_association(node: &Value) -> Option<&Value> {
    children(node)
        .iter()
        .find(|association| association["selected"].as_bool() == Some(true))
        .and_then(|association| children(association).last())
}

/// Whether a node's evaluation holds a call (a type query's operand is not
/// evaluated; a selection evaluates its selected association only, and one
/// without a selected association counts as holding a call, so that the data
/// reading refuses it).
pub(crate) fn holds_call(node: &Value) -> bool {
    match kind(node) {
        "CallExpr" => true,
        "UnaryExprOrTypeTraitExpr" => false,
        "GenericSelectionExpr" => selected_association(node).is_none_or(holds_call),
        _ => children(node).iter().any(holds_call),
    }
}

/// The call an actual *is*, through parentheses and value-preserving pointer
/// conversions (a load never applies to a call's value).
pub(crate) fn whole_call(node: &Value) -> Option<&Value> {
    let mut cursor = strip_parens(node);
    loop {
        match kind(cursor) {
            "CallExpr" => return Some(cursor),
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(cursor["castKind"].as_str(), Some("NoOp" | "BitCast"))
                    && is_pointer(cursor) =>
            {
                cursor = strip_parens(children(cursor).first()?);
            }
            "GenericSelectionExpr" => cursor = strip_parens(selected_association(cursor)?),
            _ => return None,
        }
    }
}

/// The scalar call a data actual *is*: through parentheses and scalar
/// conversions of its value.
fn whole_scalar_call(node: &Value) -> Option<&Value> {
    let mut cursor = strip_parens(node);
    loop {
        match kind(cursor) {
            "CallExpr" => return Some(cursor),
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(
                    cursor["castKind"].as_str(),
                    Some(
                        "NoOp"
                            | "IntegralCast"
                            | "IntegralToFloating"
                            | "FloatingCast"
                            | "IntegralToBoolean"
                    )
                ) =>
            {
                cursor = strip_parens(children(cursor).first()?);
            }
            _ => return None,
        }
    }
}

/// A copy of `node` with every evaluated call replaced by a scalar literal of
/// its type, so that CG03's data reading reads the rest.
fn calls_as_literals(node: &Value) -> Value {
    match kind(node) {
        "CallExpr" => json!({
            "kind": "IntegerLiteral",
            "type": { "qualType": "int" },
            "value": "0",
        }),
        "UnaryExprOrTypeTraitExpr" => node.clone(),
        "GenericSelectionExpr" => match selected_association(node) {
            Some(selected) => calls_as_literals(selected),
            None => node.clone(),
        },
        _ => {
            let mut copy = node.clone();
            if let Some(inner) = copy.get_mut("inner").and_then(Value::as_array_mut) {
                for child in inner.iter_mut() {
                    *child = calls_as_literals(child);
                }
            }
            copy
        }
    }
}

/// The evaluated calls of a data expression, outermost first, in source
/// order (a call's own actuals are read by [`Reader::call`]).
fn evaluated_calls<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    match kind(node) {
        "CallExpr" => out.push(node),
        "UnaryExprOrTypeTraitExpr" => {}
        "GenericSelectionExpr" => {
            if let Some(selected) = selected_association(node) {
                evaluated_calls(selected, out);
            }
        }
        _ => {
            for child in children(node) {
                evaluated_calls(child, out);
            }
        }
    }
}

struct Reader<'f> {
    facts: &'f dyn Facts,
    graph: EvalGraph,
    footprints: BTreeMap<EventId, Footprint>,
    bound: usize,
    /// The bound place of each bound call event.
    places: BTreeMap<EventId, String>,
}

/// Read the call `node` (a `CallExpr`) as a nested-call tree and check its
/// order. `Refusal::NotNested` when none of its actuals holds a call.
pub(crate) fn read<'a>(
    node: &'a Value,
    facts: &dyn Facts,
    use_: ResultUse,
) -> Result<Reading<'a>, Refusal> {
    let call = strip_parens(node);
    if kind(call) != "CallExpr" {
        return Err(Refusal::Form("not a call", "CG03"));
    }
    if !children(call).iter().skip(1).any(holds_call) {
        return Err(Refusal::NotNested);
    }
    let mut reader = Reader {
        facts,
        graph: EvalGraph::default(),
        footprints: BTreeMap::new(),
        bound: 0,
        places: BTreeMap::new(),
    };
    let (mut root, _) = reader.call(call)?;
    root.use_ = Some(use_);
    reader
        .graph
        .check_footprints(&reader.footprints, &|decl| facts.unaliased(decl))
        .map_err(Refusal::Order)?;
    Ok(Reading {
        root,
        graph: reader.graph,
        places: reader.places,
    })
}

/// The reading of a scalar value holding calls that is not itself one call
/// (`n = g(p) + 1`, `return h(p) * k`): its calls, inside-out, under the same
/// order check. Every call is `Tested` (an operator consumes its value) unless
/// its value is the whole expression.
#[derive(Clone, Debug)]
pub(crate) struct DataReading<'a> {
    pub(crate) calls: Vec<Call<'a>>,
    pub(crate) places: BTreeMap<EventId, String>,
}

impl DataReading<'_> {
    /// Every call in its lowering order.
    pub(crate) fn post_order(&self) -> Vec<&Call<'_>> {
        self.calls.iter().flat_map(Call::post_order).collect()
    }
}

/// Read a scalar value holding calls; `Refusal::NotNested` when it holds none.
pub(crate) fn read_data<'a>(
    node: &'a Value,
    facts: &dyn Facts,
) -> Result<DataReading<'a>, Refusal> {
    if !holds_call(node) {
        return Err(Refusal::NotNested);
    }
    let mut reader = Reader {
        facts,
        graph: EvalGraph::default(),
        footprints: BTreeMap::new(),
        bound: 0,
        places: BTreeMap::new(),
    };
    let mut addressed = Vec::new();
    let (calls, _) = reader.data(node, &mut addressed)?;
    if !addressed.is_empty() {
        return Err(Refusal::Data(
            "an address outside the calls' actuals".into(),
        ));
    }
    reader
        .graph
        .check_footprints(&reader.footprints, &|decl| facts.unaliased(decl))
        .map_err(Refusal::Order)?;
    Ok(DataReading {
        calls,
        places: reader.places,
    })
}

/// Card **CG06-VALUE-CONNECTIVE-RESIDUALS** (gateway rule 11m): this card's
/// reading of whole **operands** into one footprinted [`EvalGraph`], for a
/// consumer that composes several operands (a value region and its siblings)
/// under [`EvalGraph::check_footprints`].
///
/// Every evaluated call of an operand, at any depth, is read by
/// [`Reader::call`] exactly as a nested call is (same refusals, same
/// footprints, each call sequenced after its actuals' events), and the
/// operand's non-call part contributes its named reads ([`Reader::reads`]).
/// An address the non-call part takes is a value, not an effect, and adds no
/// cell. Nothing about the order between operands is decided here: the
/// consumer adds only the sequenced-before edges C guarantees, and
/// [`Operands::check`] is `check_footprints` unchanged.
pub(crate) struct Operands<'f> {
    reader: Reader<'f>,
}

impl<'f> Operands<'f> {
    pub(crate) fn new(facts: &'f dyn Facts) -> Self {
        Self {
            reader: Reader {
                facts,
                graph: EvalGraph::default(),
                footprints: BTreeMap::new(),
                bound: 0,
                places: BTreeMap::new(),
            },
        }
    }

    /// Read one evaluated operand; answer its events.
    pub(crate) fn operand(&mut self, node: &Value) -> Result<Vec<EventId>, Refusal> {
        let mut calls = Vec::new();
        evaluated_calls(node, &mut calls);
        let mut events = Vec::new();
        for call in calls {
            let (_, inner) = self.reader.call(call)?;
            events.extend(inner);
        }
        let mut addressed = Vec::new();
        events.extend(self.reader.reads(&calls_as_literals(node), &mut addressed));
        Ok(events)
    }

    /// A control event (`Branch`, `Merge`): it touches no cell, so its
    /// footprint is empty.
    pub(crate) fn control(&mut self, event: Event) -> EventId {
        self.reader.push(event, Footprint::default())
    }

    /// `before` are each sequenced before `after` (an edge C guarantees).
    pub(crate) fn sequence(&mut self, before: &[EventId], after: EventId) {
        for &event in before {
            self.reader.graph.before.push((event, after));
        }
    }

    /// The graph, for inspection by the tests.
    #[cfg(test)]
    pub(crate) fn graph(&self) -> &EvalGraph {
        &self.reader.graph
    }

    /// [`EvalGraph::check_footprints`] over every event read.
    pub(crate) fn check(&self) -> Result<(), OrderRefusal> {
        let facts = self.reader.facts;
        self.reader
            .graph
            .check_footprints(&self.reader.footprints, &|decl| facts.unaliased(decl))
    }
}

/// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the contract footprint of
/// one direct call none of whose actuals holds a call, read exactly as [`read`]
/// reads a call of the tree (its contract in force, each actual's cell, `&x`
/// and array decays, a stored or removed resource's owner, an owned result's
/// fresh identity). Its fresh cell, if any, is `Cell::Fresh(0)`: the
/// elaborator keys it to its own call event. Every refusal of the reading (no
/// contract, a container effect, a reallocated result, an arity mismatch, an
/// indirect callee, a nested call) answers no footprint.
pub(crate) fn call_footprint(node: &Value, facts: &dyn Facts) -> Result<Footprint, Refusal> {
    let call = strip_parens(node);
    if kind(call) != "CallExpr" {
        return Err(Refusal::Form("not a call", "CG03"));
    }
    if children(call).iter().skip(1).any(holds_call) {
        return Err(Refusal::Form("nested call", "CG05"));
    }
    let mut reader = Reader {
        facts,
        graph: EvalGraph::default(),
        footprints: BTreeMap::new(),
        bound: 0,
        places: BTreeMap::new(),
    };
    let (read, _) = reader.call(call)?;
    let mut footprint = reader.footprints.remove(&read.event).unwrap_or_default();
    for cell in footprint
        .reads
        .iter_mut()
        .chain(footprint.writes.iter_mut())
    {
        if let Cell::Fresh(_) = cell {
            *cell = Cell::Fresh(0);
        }
    }
    Ok(footprint)
}

/// The bound place name of the `index`-th bound call of a statement.
pub(crate) fn bound_place(index: usize) -> String {
    format!("{NESTED_VALUE_PREFIX}{index}")
}

impl<'f> Reader<'f> {
    fn push(&mut self, event: Event, footprint: Footprint) -> EventId {
        self.graph.events.push(event);
        let id = self.graph.events.len() - 1;
        self.footprints.insert(id, footprint);
        id
    }

    /// The read events of an actual's evaluated named objects, and the cells a
    /// call taking this actual writes through an address it holds.
    fn reads(&mut self, node: &Value, addressed: &mut Vec<Cell>) -> Vec<EventId> {
        fn object(node: &Value) -> Option<(String, String)> {
            let reference = &node["referencedDecl"];
            matches!(reference["kind"].as_str(), Some("VarDecl" | "ParmVarDecl"))
                .then(|| {
                    Some((
                        reference["id"].as_str()?.to_owned(),
                        reference["name"].as_str()?.to_owned(),
                    ))
                })
                .flatten()
        }
        fn walk(node: &Value, out: &mut Vec<(String, String, bool)>, addressed: &mut Vec<Cell>) {
            match kind(node) {
                "UnaryExprOrTypeTraitExpr" => {}
                "GenericSelectionExpr" => {
                    if let Some(selected) = selected_association(node) {
                        walk(selected, out, addressed);
                    }
                }
                "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                    // The address of an object: a callee may store through it.
                    let operand = children(node).first().map(strip_parens);
                    if let Some(operand) = operand
                        && kind(operand) == "DeclRefExpr"
                        && let Some((decl, _)) = object(operand)
                    {
                        // A scalar object's storage holds no identity.
                        if !crate::typed_access_path::is_scalar(operand) {
                            addressed.push(Cell::Object(decl));
                        }
                    } else if let Some(operand) = operand {
                        // The address of a place: its root is read.
                        walk(operand, out, addressed);
                    }
                }
                "ImplicitCastExpr" if node["castKind"].as_str() == Some("ArrayToPointerDecay") => {
                    // An array object's address: a callee may store into it.
                    match children(node).first().map(strip_parens) {
                        Some(inner) if kind(inner) == "DeclRefExpr" => {
                            if let Some((decl, _)) = object(inner) {
                                addressed.push(Cell::Object(decl));
                            }
                        }
                        Some(inner) => walk(inner, out, addressed),
                        None => {}
                    }
                }
                "DeclRefExpr" => {
                    if let Some((decl, name)) = object(node) {
                        let pointer =
                            qual(node).is_none_or(|ty| ty.contains('*') || ty.contains('['));
                        out.push((decl, name, pointer));
                    }
                }
                _ => {
                    for child in children(node) {
                        walk(child, out, addressed);
                    }
                }
            }
        }
        let mut objects = Vec::new();
        walk(node, &mut objects, addressed);
        objects
            .into_iter()
            .map(|(decl, name, pointer)| {
                let reads = if pointer {
                    vec![Cell::Object(decl.clone()), Cell::Identity(name.clone())]
                } else {
                    Vec::new()
                };
                self.push(
                    Event::Read(Place::Object { decl, name }),
                    Footprint {
                        reads,
                        writes: Vec::new(),
                    },
                )
            })
            .collect()
    }

    /// Read a scalar value holding calls: its calls (each read at any depth)
    /// and its own reads. A call whose value is the whole expression is
    /// `Passed`; one an operator consumes is `Tested`.
    fn data<'a>(
        &mut self,
        node: &'a Value,
        addressed: &mut Vec<Cell>,
    ) -> Result<(Vec<Call<'a>>, Vec<EventId>), Refusal> {
        if is_pointer(node) || qual(node).is_some_and(|ty| ty.contains('*')) {
            return Err(Refusal::Data("a pointer value holding calls (CG16)".into()));
        }
        read_actual(&calls_as_literals(node), self.facts.fields())
            .map_err(|refusal| Refusal::Data(refusal.reason()))
            .and_then(|read| match read {
                Substituted::Data => Ok(()),
                _ => Err(Refusal::Data("not data".into())),
            })?;
        let whole = whole_scalar_call(node);
        let mut inner = Vec::new();
        evaluated_calls(node, &mut inner);
        let mut calls = Vec::with_capacity(inner.len());
        let mut events = Vec::new();
        for call_node in inner {
            let (mut call, events_inner) = self.call(call_node)?;
            call.use_ = Some(
                if whole.is_some_and(|whole| std::ptr::eq(whole, call_node)) {
                    ResultUse::Passed
                } else {
                    ResultUse::Tested
                },
            );
            if !is_pointer(call_node)
                && matches!(call.facts.result, ResourceReturnMode::Owned { .. })
            {
                return Err(Refusal::Clause(
                    call.callee,
                    "an owned scalar result (the descriptor route's)",
                ));
            }
            events.extend(events_inner);
            calls.push(call);
        }
        // The non-call part's own reads.
        events.extend(self.reads(&calls_as_literals(node), addressed));
        Ok((calls, events))
    }

    /// Read one call and every call in its actuals; answer the call and all
    /// its events (its own last).
    fn call<'a>(&mut self, node: &'a Value) -> Result<(Call<'a>, Vec<EventId>), Refusal> {
        let (designator, actuals) = children(node)
            .split_first()
            .ok_or(Refusal::Form("call without a callee", "CG03"))?;
        let callee = direct_callee(designator)
            .ok_or(Refusal::Form("indirect call", "CG22"))?
            .to_owned();
        let facts = self
            .facts
            .contract(&callee)
            .ok_or_else(|| Refusal::Uncontracted(callee.clone()))?;
        if facts.container {
            return Err(Refusal::Clause(callee, "a container effect"));
        }
        if matches!(facts.result, ResourceReturnMode::Reallocated { .. }) {
            return Err(Refusal::Clause(callee, "a reallocated result"));
        }
        if matches!(facts.result, ResourceReturnMode::Removed { .. }) {
            return Err(Refusal::Clause(callee, "a same-item removed result"));
        }
        if actuals.len() != facts.modes.len() && !(facts.tail && actuals.len() > facts.modes.len())
        {
            return Err(Refusal::Arity(callee));
        }
        let mut read = Vec::with_capacity(actuals.len());
        let mut cells: Vec<Option<Cell>> = Vec::with_capacity(actuals.len());
        let mut operands = Vec::with_capacity(actuals.len());
        let mut all = Vec::new();
        let mut footprint = Footprint::default();
        for (formal, actual) in actuals.iter().enumerate() {
            let mode = facts
                .modes
                .get(formal)
                .copied()
                .unwrap_or(ResourceParameterMode::Borrowed);
            let custody = !matches!(mode, ResourceParameterMode::Borrowed);
            let mut events = Vec::new();
            let mut addressed = Vec::new();
            let (form, cell) = if !holds_call(actual) {
                events.extend(self.reads(actual, &mut addressed));
                let cell = match read_actual(actual, self.facts.fields()) {
                    Ok(Substituted::Identity { name, .. }) => Some(Cell::Identity(name)),
                    Ok(Substituted::Address { .. } | Substituted::Data) => None,
                    // A place: the identity its root denotes is read or
                    // written (a place at a custody formal is refused later).
                    Ok(Substituted::Place { .. }) => Some(Cell::Identity(String::new())),
                    // An actual CG03 does not read: the classifier decides it;
                    // its footprint is any identity.
                    Err(_) => Some(Cell::Identity(String::new())),
                };
                (Actual::Plain(actual), cell)
            } else if let Some(inner) = whole_call(actual)
                && is_pointer(inner)
            {
                let (mut call, events_inner) = self.call(inner)?;
                call.use_ = Some(ResultUse::Passed);
                events.extend(events_inner);
                let place = bound_place(self.bound);
                self.bound += 1;
                self.places.insert(call.event, place.clone());
                let cell = match call.facts.result {
                    ResourceReturnMode::Owned { .. } => Cell::Fresh(call.event),
                    _ => Cell::Identity(place),
                };
                (Actual::Bound(Box::new(call)), Some(cell))
            } else {
                if custody {
                    return Err(Refusal::DataAtCustody(callee));
                }
                let (calls, data_events) = self.data(actual, &mut addressed)?;
                events.extend(data_events);
                (Actual::Data { calls }, None)
            };
            if let Some(cell) = &cell {
                if custody {
                    footprint.writes.push(cell.clone());
                } else {
                    footprint.reads.push(cell.clone());
                }
            }
            cells.push(cell);
            footprint.writes.extend(addressed);
            all.extend(events.iter().copied());
            operands.push(events);
            read.push(form);
        }
        // A stored or removed resource moves under its owner: the owner's
        // identity is written too.
        for mode in &facts.modes {
            if let ResourceParameterMode::Stored {
                owner_parameter, ..
            }
            | ResourceParameterMode::Removed {
                owner_parameter, ..
            } = mode
            {
                footprint.writes.push(
                    cells
                        .get(*owner_parameter)
                        .cloned()
                        .flatten()
                        .unwrap_or_else(|| Cell::Identity(String::new())),
                );
            }
        }
        if operands.len() > 1 {
            self.graph.unsequenced.push(operands);
        }
        if matches!(facts.result, ResourceReturnMode::Owned { .. }) {
            // The fresh identity is this event's; its id is the next one.
            footprint.writes.push(Cell::Fresh(self.graph.events.len()));
        }
        let event = self.push(
            Event::Call {
                callee: callee.clone(),
            },
            footprint,
        );
        for &before in &all {
            self.graph.before.push((before, event));
        }
        all.push(event);
        Ok((
            Call {
                callee,
                facts,
                event,
                actuals: read,
                use_: None,
            },
            all,
        ))
    }
}

impl Call<'_> {
    /// Every call of the tree in its lowering order: each call after every call
    /// of its actuals (inside-out), siblings in the graph's event order.
    pub(crate) fn post_order(&self) -> Vec<&Call<'_>> {
        let mut out = Vec::new();
        for actual in &self.actuals {
            match actual {
                Actual::Plain(_) => {}
                Actual::Bound(call) => out.extend(call.post_order()),
                Actual::Data { calls, .. } => {
                    for call in calls {
                        out.extend(call.post_order());
                    }
                }
            }
        }
        out.push(self);
        out
    }

    /// The largest depth of nesting under this call (1 for a leaf call).
    pub(crate) fn depth(&self) -> usize {
        1 + self
            .actuals
            .iter()
            .map(|actual| match actual {
                Actual::Plain(_) => 0,
                Actual::Bound(call) => call.depth(),
                Actual::Data { calls, .. } => calls.iter().map(Call::depth).max().unwrap_or(0),
            })
            .max()
            .unwrap_or(0)
    }
}

/// `(cell, cell, overlap)` over the declaration ids `u` (unaliased) and `a`
/// (address taken), shared with `QCore.NestedCallEffects.overlapVectors`.
#[cfg(test)]
pub(crate) const OVERLAP_VECTORS: &[(&str, &str, bool)] = &[
    ("object:u", "object:u", true),
    ("object:u", "object:a", false),
    ("identity:p", "identity:q", true),
    ("identity:p", "identity:p", true),
    ("fresh:0", "fresh:0", true),
    ("fresh:0", "fresh:1", false),
    ("fresh:0", "identity:p", false),
    ("fresh:0", "object:a", false),
    ("object:u", "identity:p", false),
    ("object:a", "identity:p", true),
];

/// `(C body, admitted)`: the last statement of `subject`, read by [`read`] over
/// the fixture contracts, shared with `QCore.NestedCallEffects.orderVectors`.
#[cfg(test)]
pub(crate) const ORDER_VECTORS: &[(&str, bool)] = &[
    // E10: an inner release, then the outer entry.
    ("void *p = q_alloc(); cg_sink(r_release_code(p));", true),
    // E10: an owned inner result bound and released by the outer call.
    ("void *p = q_alloc(); q_free(r_wrap(p));", true),
    // E11: depth two.
    (
        "void *p = q_alloc(); cg_sink(r_borrow_code(r_wrap(p)));",
        true,
    ),
    // E11: a call inside a larger actual.
    ("void *p = q_alloc(); cg_sink(r_borrow_code(p) + 1);", true),
    // E12: two siblings producing fresh identities from borrowed reads.
    (
        "void *p = q_alloc(); cg_sink2(r_borrow_code(r_wrap(p)), r_borrow_code(r_wrap(p)));",
        true,
    ),
    // E12: a release unordered with a borrow of one identity: refused.
    (
        "void *p = q_alloc(); cg_sink2(r_release_code(p), r_borrow_code(p));",
        false,
    ),
    // E12: two releases of names that may alias: refused.
    (
        "void *p = q_alloc(); void *q = q_alloc(); cg_sink2(r_release_code(p), r_release_code(q));",
        false,
    ),
    // The actual `p` read unordered with a release of `p`: refused.
    ("void *p = q_alloc(); cg_pair(p, r_release_code(p));", false),
];

#[cfg(test)]
#[path = "../tests/unit/nested_call_effects.rs"]
mod tests;
