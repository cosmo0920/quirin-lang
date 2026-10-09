//! Card **CG22-CALLBACK-REGISTRY-CONTRACT** (gateway rules 13u–13x): the
//! route-G adapter of [`crate::callback_contracts`]. Thin hooks, each the
//! identity on every environment that holds no callback declaration:
//!
//! 1. [`Parser::callback_operands`] (from `retaining_call`, the classifier's
//!    uncontracted fallback): a synchronous invocation or a table dispatch is
//!    decided here, from Clang's AST and the contracts in force, and either
//!    leaves the call its custody contract or refuses it; a registry clause
//!    is read into operands the transfer decides.
//! 2. [`apply`] (the transfer of a `ContractCall`, after the retentions):
//!    registrations, element additions, registry invocations, element drops,
//!    unregistrations and the flag-conditioned self-release, each against the
//!    path's registrations in force.
//! 3. [`refuse_release_of_registry`] (in `apply_resource_action`): a holder
//!    whose registry still holds elements is released only through the
//!    registry (its elements would be lost).
//! 4. [`join`] (in `AbstractState::join`): a registration not made, or not
//!    made alike, on every arriving path is undecided.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackHolder};
use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract};
use qcore_schemas::retained_by::RetainingContract;
use serde_json::Value;

use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind, Parser};
use crate::callback_contracts::{
    CandidateClass, Decision, ELEMENT_UNDECIDED, REGISTRY_UNDECIDED, RegistryClass, UNDECIDED,
    WITHDRAWN, classify, decide, invocation_detail, refusal, registry_class,
};
use crate::typed_c_ast::{children, kind};

/// A registry holder's actual, as spelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryHolder {
    /// `&L`, `L` an automatic object of this frame named only at registry and
    /// retention holder positions.
    Frame(String),
    /// A plain name.
    Name(String),
    /// The object the call returns.
    Result,
}

/// One registry effect of a call, in clause order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryUse {
    /// `resource_callback_registered(F, H)`: the candidates (empty: no
    /// callback) with the contract each has in force here.
    Register {
        holder: RegistryHolder,
        candidates: Vec<(String, Option<ResourceFunctionContract>)>,
    },
    /// `resource_callback_registry_invoked(H, A…)`.
    Invoke {
        holder: RegistryHolder,
        actuals: Vec<CallbackActual>,
    },
    /// `resource_element_added(E, H)`: the element's actual as spelled.
    Add {
        holder: RegistryHolder,
        element: Vec<String>,
    },
    /// `resource_elements_dropped(H)`.
    Drop { holder: RegistryHolder },
    /// `resource_callback_unregistered(H)`.
    Unregister { holder: RegistryHolder },
    /// `resource_released_if_constructed(H)`.
    ReleaseIfConstructed { holder: RegistryHolder },
}

/// The registry operands of one call, applied by [`apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operands {
    pub uses: Vec<RegistryUse>,
    /// The holder actual's text, per use (for refusals).
    pub texts: Vec<String>,
}

/// What a holder keys on in the path state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegistryKey {
    Frame(String),
    Site(usize),
}

/// A registration as the path state keeps it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Registration {
    /// Made on every arriving path alike.
    InForce {
        candidates: Vec<(String, Option<ResourceFunctionContract>)>,
        /// Made by a constructor (`result` holder).
        constructed: bool,
    },
    /// Not made, or not made alike, on every arriving path.
    Undecided,
}

/// Hook 4: join the registrations of an arriving path.
pub(super) fn join(
    mine: &mut BTreeMap<RegistryKey, Registration>,
    incoming: &BTreeMap<RegistryKey, Registration>,
) {
    if mine.is_empty() && incoming.is_empty() {
        return;
    }
    let keys = mine
        .keys()
        .chain(incoming.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        let agreed = match (mine.get(&key), incoming.get(&key)) {
            (Some(left), Some(right)) if left == right => continue,
            _ => Registration::Undecided,
        };
        mine.insert(key, agreed);
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Whether `callee`'s declaration names a holder at some position (a
/// registry clause or a retention): a call through which a frame holder may
/// be named.
fn names_a_holder(contracts: &ResourceContracts, callee: &str) -> bool {
    contracts.retaining_contract(callee).is_some_and(|declared| {
        declared.retains()
            || declared.callbacks.iter().any(|clause| {
                clause.needs_registration()
                    || matches!(
                        clause,
                        CallbackClause::Registered {
                            holder: CallbackHolder::Parameter { .. },
                            ..
                        }
                    )
            })
    })
}

/// Whether `name` is mentioned in `body` only as the declaration of the
/// object and as whole `&name` actuals of calls to holder-naming callees.
fn only_registry_mentions(body: &[&str], name: &str, contracts: &ResourceContracts) -> bool {
    let mut declarations = 0usize;
    for (index, token) in body.iter().enumerate() {
        if *token != name {
            continue;
        }
        let previous = index.checked_sub(1).map(|at| body[at]);
        let next = body.get(index + 1).copied();
        if matches!(previous, Some("." | "->")) {
            continue;
        }
        if previous == Some("&") && matches!(next, Some("," | ")")) {
            let mut depth = 0usize;
            let mut callee = None;
            for at in (0..index.saturating_sub(1)).rev() {
                match body[at] {
                    ")" => depth += 1,
                    "(" if depth == 0 => {
                        callee = at.checked_sub(1).map(|c| body[c]);
                        break;
                    }
                    "(" => depth -= 1,
                    "," | "&" | "*" | "[" | "]" => {}
                    _ => {}
                }
            }
            if callee.is_some_and(|callee| names_a_holder(contracts, callee)) {
                continue;
            }
            return false;
        }
        if matches!(next, Some(";" | "="))
            && previous.is_some_and(|previous| is_identifier(previous) || previous == "}")
        {
            declarations += 1;
            continue;
        }
        return false;
    }
    declarations == 1
}

fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            "ImplicitCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("NoOp" | "ArrayToPointerDecay" | "BitCast")
                ) =>
            {
                match children(node).first() {
                    Some(inner) => node = inner,
                    None => return node,
                }
            }
            _ => return node,
        }
    }
}

/// The name of a call's direct callee, if it is a function designator.
fn direct_callee(call: &Value) -> Option<&str> {
    let mut node = children(call).first()?;
    loop {
        match kind(node) {
            "ParenExpr" => node = children(node).first()?,
            "ImplicitCastExpr"
                if node["castKind"].as_str() == Some("FunctionToPointerDecay") =>
            {
                node = children(node).first()?;
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl") => {
                return node["referencedDecl"]["name"].as_str();
            }
            _ => return None,
        }
    }
}

fn calls<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if kind(node) == "CallExpr" {
        out.push(node);
    }
    for child in children(node) {
        calls(child, out);
    }
}

fn find_var<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if kind(node) == "VarDecl" && node["id"].as_str() == Some(id) {
        return Some(node);
    }
    children(node).iter().find_map(|child| find_var(child, id))
}

fn references(node: &Value, id: &str) -> usize {
    let own = usize::from(
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id),
    );
    own + children(node)
        .iter()
        .map(|child| references(child, id))
        .sum::<usize>()
}

/// Whether an initializer leaf is a null pointer constant (`NULL`, `0`, a
/// cast of `0`) or an implicit filler: never invoked (premise
/// P-CG22-DEFINED-POINTER-VALUE, a call through a null pointer is undefined).
fn null_leaf(node: &Value) -> bool {
    let node = strip(node);
    match kind(node) {
        "ImplicitValueInitExpr" | "GNUNullExpr" => true,
        "IntegerLiteral" => node["value"].as_str() == Some("0"),
        "ImplicitCastExpr" | "CStyleCastExpr"
            if matches!(
                node["castKind"].as_str(),
                Some("NullToPointer" | "IntegralToPointer" | "NoOp" | "BitCast")
            ) =>
        {
            children(node).first().is_some_and(null_leaf)
        }
        _ => false,
    }
}

/// The function designators of a table's initializer, `Err` when a leaf may
/// hold anything else that could be called.
fn table_designators(init: &Value, out: &mut BTreeSet<String>) -> Result<(), String> {
    let node = strip(init);
    match kind(node) {
        "InitListExpr" => {
            for child in children(node) {
                table_designators(child, out)?;
            }
            if let Some(filler) = node.get("array_filler")
                && !filler.is_null()
            {
                for child in filler.as_array().into_iter().flatten() {
                    table_designators(child, out)?;
                }
            }
            Ok(())
        }
        "ImplicitCastExpr" if node["castKind"].as_str() == Some("FunctionToPointerDecay") => {
            let inner = children(node).first().ok_or("an unread designator")?;
            match direct_name(inner) {
                Some(name) => {
                    out.insert(name.to_owned());
                    Ok(())
                }
                None => Err("an unread designator".into()),
            }
        }
        // A string identifier (`"scale"`) or a scalar member.
        "StringLiteral" | "FloatingLiteral" | "CharacterLiteral" => Ok(()),
        "IntegerLiteral" => Ok(()),
        _ if null_leaf(node) => Ok(()),
        other => Err(format!("a table entry `{other}`")),
    }
}

fn direct_name(node: &Value) -> Option<&str> {
    let node = strip(node);
    (kind(node) == "DeclRefExpr" && node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
        .then(|| node["referencedDecl"]["name"].as_str())
        .flatten()
}

impl Parser<'_> {
    /// The one Clang call node of `callee` on `line` of the current function,
    /// with its function body, read through the unit's AST.
    fn callback_site<T>(
        &self,
        callee: &str,
        line: usize,
        read: impl FnOnce(&Value, &Value, &crate::indirect_calls::TableFacts) -> T,
    ) -> Option<T> {
        crate::typed_access_path::with_indirect_call_unit(|unit| {
            let function = unit.function(&self.typed_path_function)?;
            let mut found = Vec::new();
            calls(function, &mut found);
            let matching = found
                .into_iter()
                .filter(|call| {
                    direct_callee(call) == Some(callee) && unit.node_line(call) == Some(line)
                })
                .collect::<Vec<_>>();
            let [call] = matching.as_slice() else {
                return None;
            };
            Some(read(call, function, unit.tables()))
        })
        .flatten()
    }

    /// The candidates of the callback at argument `position` of the call:
    /// `Ok(empty)` only for a null pointer where `null_is_none`.
    fn callback_candidates(
        &self,
        callee: &str,
        line: usize,
        position: usize,
        null_is_none: bool,
    ) -> Result<BTreeSet<String>, String> {
        self.callback_site(callee, line, |call, function, tables| {
            let argument = children(call)
                .get(position + 1)
                .ok_or_else(|| "an unread actual".to_owned())?;
            if null_is_none && null_leaf(argument) {
                return Ok(BTreeSet::new());
            }
            crate::indirect_calls::resolve(argument, function, tables)
                .candidates
                .map_err(|open| open.describe())
        })
        .unwrap_or_else(|| Err("the call is not placed in Clang's AST".to_owned()))
    }

    /// The candidates of a table actual at argument `position`: every
    /// designator of an automatic array's braced initializer, the array named
    /// only as whole table actuals of declared table dispatchers.
    fn table_candidates(
        &self,
        callee: &str,
        line: usize,
        position: usize,
    ) -> Result<BTreeSet<String>, String> {
        let contracts = self.contracts;
        self.callback_site(callee, line, |call, function, _| {
            let argument = children(call)
                .get(position + 1)
                .ok_or_else(|| "an unread actual".to_owned())?;
            let place = strip(argument);
            if kind(place) != "DeclRefExpr"
                || place["referencedDecl"]["kind"].as_str() != Some("VarDecl")
            {
                return Err("a table that is not a named array".to_owned());
            }
            let id = place["referencedDecl"]["id"].as_str().unwrap_or("");
            let decl = find_var(function, id).ok_or("a table this function does not declare")?;
            let ty = decl["type"]["qualType"].as_str().unwrap_or("");
            if !decl["storageClass"].is_null() || !ty.contains('[') {
                return Err("a table that is not an automatic array".to_owned());
            }
            let init = children(decl)
                .iter()
                .find(|child| kind(strip(child)) == "InitListExpr")
                .ok_or("a table with no braced initializer")?;
            // Every mention is a whole table actual of a declared dispatcher.
            let mut all = Vec::new();
            calls(function, &mut all);
            let mut admitted = 0usize;
            for site in all {
                let Some(name) = direct_callee(site) else {
                    continue;
                };
                let Some(declared) = contracts.retaining_contract(name) else {
                    continue;
                };
                for clause in &declared.callbacks {
                    if let CallbackClause::Table { table, .. } = clause
                        && let Some(actual) = children(site).get(table + 1)
                    {
                        let actual = strip(actual);
                        if kind(actual) == "DeclRefExpr"
                            && actual["referencedDecl"]["id"].as_str() == Some(id)
                        {
                            admitted += 1;
                        }
                    }
                }
            }
            if references(function, id) != admitted {
                return Err("a table the body names elsewhere".to_owned());
            }
            let mut out = BTreeSet::new();
            table_designators(init, &mut out)?;
            if out.is_empty() {
                return Err("a table with no handler".to_owned());
            }
            // A designator a declaration of the function shadows is open.
            Ok(out)
        })
        .unwrap_or_else(|| Err("the call is not placed in Clang's AST".to_owned()))
    }

    fn classified(
        &self,
        candidates: &BTreeSet<String>,
        actuals: &[CallbackActual],
    ) -> Vec<(String, CandidateClass)> {
        candidates
            .iter()
            .map(|name| (name.clone(), classify(self.contracts.contract(name), actuals)))
            .collect()
    }

    /// Hook 1 (rules 13u–13x): decide the synchronous invocations and table
    /// dispatches of a call to `function`, and read its registry clauses.
    /// `Err` with the call's refusal.
    pub(super) fn callback_operands(
        &self,
        function: &str,
        declaration: &RetainingContract,
        line: usize,
        actual: &dyn Fn(usize) -> Option<Vec<String>>,
        frame_holder: &dyn Fn(&str) -> bool,
    ) -> Result<Option<Operands>, String> {
        if declaration.callbacks_withdrawn {
            return Err(refusal(
                function,
                "its body in this unit does not validate the declared callback clause",
                WITHDRAWN,
            ));
        }
        if declaration.callbacks.is_empty() {
            return Ok(None);
        }
        let text = |index: usize| actual(index).map_or_else(|| "?".to_owned(), |w| w.join(" "));
        let holder = |index: usize| -> Option<RegistryHolder> {
            let words = actual(index)?;
            match words.as_slice() {
                [amp, name] if amp == "&" && is_identifier(name) && frame_holder(name) => {
                    Some(RegistryHolder::Frame(name.clone()))
                }
                [name] if is_identifier(name) => Some(RegistryHolder::Name(name.clone())),
                _ => None,
            }
        };
        let undecided_holder = |index: usize| {
            refusal(
                function,
                &format!("the registry holder `{}` is not one this body names", text(index)),
                REGISTRY_UNDECIDED,
            )
        };
        let mut operands = Operands {
            uses: Vec::new(),
            texts: Vec::new(),
        };
        for clause in &declaration.callbacks {
            match clause {
                CallbackClause::Invoked { callback, actuals } => {
                    let resolution = self.callback_candidates(function, line, *callback, false);
                    let classes = resolution
                        .as_ref()
                        .map(|candidates| self.classified(candidates, actuals));
                    let decision = decide(
                        classes.as_ref().map(Vec::as_slice).map_err(|_| ()),
                        false,
                    );
                    if let Decision::Refused(reason, candidate) = decision {
                        return Err(refusal(
                            function,
                            &invocation_detail(reason, candidate.as_deref(), &text(*callback)),
                            reason,
                        ));
                    }
                }
                CallbackClause::Table { table, actuals } => {
                    let resolution = self.table_candidates(function, line, *table);
                    let classes = resolution
                        .as_ref()
                        .map(|candidates| self.classified(candidates, actuals));
                    let decision = decide(
                        classes.as_ref().map(Vec::as_slice).map_err(|_| ()),
                        false,
                    );
                    if let Decision::Refused(reason, candidate) = decision {
                        let detail = match (&resolution, reason) {
                            (Err(why), UNDECIDED) => format!(
                                "the handlers of `{}` are not a closed set of functions ({why})",
                                text(*table)
                            ),
                            _ => invocation_detail(reason, candidate.as_deref(), &text(*table)),
                        };
                        return Err(refusal(function, &detail, reason));
                    }
                }
                CallbackClause::Registered { callback, holder: at } => {
                    let candidates = self
                        .callback_candidates(function, line, *callback, true)
                        .map_err(|_| {
                            refusal(
                                function,
                                &invocation_detail(UNDECIDED, None, &text(*callback)),
                                UNDECIDED,
                            )
                        })?;
                    let holder = match at {
                        CallbackHolder::Result => RegistryHolder::Result,
                        CallbackHolder::Parameter { index } => {
                            holder(*index).ok_or_else(|| undecided_holder(*index))?
                        }
                    };
                    operands.texts.push(match at {
                        CallbackHolder::Result => "the returned object".to_owned(),
                        CallbackHolder::Parameter { index } => text(*index),
                    });
                    operands.uses.push(RegistryUse::Register {
                        holder,
                        candidates: candidates
                            .iter()
                            .map(|name| (name.clone(), self.contracts.contract(name).cloned()))
                            .collect(),
                    });
                }
                CallbackClause::RegistryInvoked {
                    holder: at,
                    actuals,
                } => {
                    let holder = holder(*at).ok_or_else(|| undecided_holder(*at))?;
                    operands.texts.push(text(*at));
                    operands.uses.push(RegistryUse::Invoke {
                        holder,
                        actuals: actuals.clone(),
                    });
                }
                CallbackClause::ElementAdded {
                    element,
                    holder: at,
                } => {
                    let holder = holder(*at).ok_or_else(|| undecided_holder(*at))?;
                    operands.texts.push(text(*at));
                    operands.uses.push(RegistryUse::Add {
                        holder,
                        element: actual(*element).unwrap_or_default(),
                    });
                }
                CallbackClause::ElementsDropped { holder: at } => {
                    let holder = holder(*at).ok_or_else(|| undecided_holder(*at))?;
                    operands.texts.push(text(*at));
                    operands.uses.push(RegistryUse::Drop { holder });
                }
                CallbackClause::Unregistered { holder: at } => {
                    let holder = holder(*at).ok_or_else(|| undecided_holder(*at))?;
                    operands.texts.push(text(*at));
                    operands.uses.push(RegistryUse::Unregister { holder });
                }
                CallbackClause::ReleasedIfConstructed { holder: at } => {
                    let holder = holder(*at).ok_or_else(|| undecided_holder(*at))?;
                    operands.texts.push(text(*at));
                    operands.uses.push(RegistryUse::ReleaseIfConstructed { holder });
                }
            }
        }
        Ok((!operands.uses.is_empty()).then_some(operands))
    }

    /// Whether `name` is a frame registry holder of the calling function.
    pub(super) fn frame_registry_holder(&self, name: &str) -> bool {
        let Some((start, _)) = super::find_function_body(self.tokens, &self.typed_path_function)
        else {
            return false;
        };
        let Some(end) = super::matching(self.tokens, start, "{", "}") else {
            return false;
        };
        let body = self.tokens[start + 1..end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        only_registry_mentions(&body, name, self.contracts)
            && self.typed_positions_index().is_some_and(|index| {
                super::in_place_custody::one_automatic_object(&index.function, name)
            })
    }
}

fn key_of(
    state: &AbstractState,
    holder: &RegistryHolder,
    destination: Option<&str>,
) -> Option<RegistryKey> {
    match holder {
        RegistryHolder::Frame(name) => Some(RegistryKey::Frame(name.clone())),
        RegistryHolder::Name(name) => {
            super::retained_by_route::tracked(state, name).map(RegistryKey::Site)
        }
        RegistryHolder::Result => destination
            .filter(|place| crate::may_alias::place_root(place) == Some(*place))
            .and_then(|place| super::retained_by_route::tracked(state, place))
            .map(RegistryKey::Site),
    }
}

fn refuse(
    function: &str,
    line: usize,
    description: String,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported { description },
    });
    state.unmodeled = true;
}

/// The registry class of a registration's candidates at its element position.
fn class_of(candidates: &[(String, Option<ResourceFunctionContract>)]) -> RegistryClass {
    registry_class(
        &candidates
            .iter()
            .map(|(name, contract)| {
                (
                    name.clone(),
                    classify(contract.as_ref(), &[CallbackActual::Element]),
                )
            })
            .collect::<Vec<_>>(),
    )
}

/// Hook 2 (rules 13w, 13x): the registry effects of one call, in clause order.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply(
    function: &str,
    line: usize,
    called: &str,
    operands: &Operands,
    destination: Option<&str>,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let callee = crate::retained_by::retained_callee(called).unwrap_or(called);
    // The registration a use reads: the one in force before this call (an
    // unregistration does not hide it from the self-release that follows).
    let before = state.registries.clone();
    for (registry_use, text) in operands.uses.iter().zip(&operands.texts) {
        let holder = match registry_use {
            RegistryUse::Register { holder, .. }
            | RegistryUse::Invoke { holder, .. }
            | RegistryUse::Add { holder, .. }
            | RegistryUse::Drop { holder }
            | RegistryUse::Unregister { holder }
            | RegistryUse::ReleaseIfConstructed { holder } => holder,
        };
        let Some(key) = key_of(state, holder, destination) else {
            refuse(
                function,
                line,
                refusal(
                    callee,
                    &format!("the registry holder `{text}` is not one this body tracks"),
                    REGISTRY_UNDECIDED,
                ),
                state,
                diagnostics,
            );
            continue;
        };
        if let RegistryUse::Register { candidates, .. } = registry_use {
            state.registries.insert(
                key,
                Registration::InForce {
                    candidates: candidates.clone(),
                    constructed: matches!(holder, RegistryHolder::Result),
                },
            );
            continue;
        }
        let Some(Registration::InForce {
            candidates,
            constructed,
        }) = before.get(&key).cloned()
        else {
            refuse(
                function,
                line,
                refusal(
                    callee,
                    &format!("no registration is in force at `{text}` on every path"),
                    REGISTRY_UNDECIDED,
                ),
                state,
                diagnostics,
            );
            continue;
        };
        match registry_use {
            RegistryUse::Register { .. } => {}
            RegistryUse::Invoke { actuals, .. } => {
                let classes = candidates
                    .iter()
                    .map(|(name, contract)| (name.clone(), classify(contract.as_ref(), actuals)))
                    .collect::<Vec<_>>();
                match (classes.is_empty(), decide(Ok(classes.as_slice()), true)) {
                    // No callback registered: nothing is called.
                    (true, _) | (false, Decision::Identity) => {}
                    (false, Decision::ReleasesElements) => {
                        if let RegistryKey::Site(site) = key {
                            super::cascade_resource_action(state, site, false);
                        }
                    }
                    (false, Decision::Refused(reason, candidate)) => refuse(
                        function,
                        line,
                        refusal(
                            callee,
                            &invocation_detail(reason, candidate.as_deref(), text),
                            reason,
                        ),
                        state,
                        diagnostics,
                    ),
                }
            }
            RegistryUse::Add { element, .. } => match (class_of(&candidates), &key) {
                (RegistryClass::Releasing, RegistryKey::Site(_)) => {
                    let (RegistryHolder::Name(owner), [child]) = (holder, element.as_slice())
                    else {
                        refuse(
                            function,
                            line,
                            refusal(
                                callee,
                                &format!(
                                    "`{}` is not a name whose custody `{text}` can take",
                                    element.join(" ")
                                ),
                                ELEMENT_UNDECIDED,
                            ),
                            state,
                            diagnostics,
                        );
                        continue;
                    };
                    super::attach_contained_resource(
                        function,
                        line,
                        owner,
                        child,
                        false,
                        state,
                        acquisition_lines,
                        diagnostics,
                    );
                }
                (RegistryClass::None | RegistryClass::Neutral, _) => {
                    let held = match &key {
                        RegistryKey::Frame(name) => crate::retained_by::Holder::Frame(name.clone()),
                        RegistryKey::Site(site) => crate::retained_by::Holder::Tracked(*site),
                    };
                    let spelled = match element.as_slice() {
                        [literal] if literal == "__qcore_literal__" => {
                            Some(super::retained_by_route::RetainedSpelling::Literal)
                        }
                        [name] if is_identifier(name) => {
                            Some(super::retained_by_route::RetainedSpelling::Name(name.clone()))
                        }
                        _ => None,
                    };
                    let retained = spelled
                        .map(|spelled| super::retained_by_route::retained_class(state, &spelled));
                    match retained.map(|retained| crate::retained_by::decide(&retained, &held)) {
                        Some(crate::retained_by::Decision::Outlives) => {}
                        Some(crate::retained_by::Decision::Record(object, by)) => {
                            state.retained.insert((object, by));
                        }
                        _ => refuse(
                            function,
                            line,
                            crate::retained_by::undecided_description(
                                callee,
                                &element.join(" "),
                                text,
                            ),
                            state,
                            diagnostics,
                        ),
                    }
                }
                _ => refuse(
                    function,
                    line,
                    refusal(
                        callee,
                        &format!(
                            "the registry at `{text}` would take `{}`, which this body cannot \
                             hand it",
                            element.join(" ")
                        ),
                        ELEMENT_UNDECIDED,
                    ),
                    state,
                    diagnostics,
                ),
            },
            RegistryUse::Drop { .. } => {
                let by = match &key {
                    RegistryKey::Frame(name) => crate::retained_by::HeldBy::Frame(name.clone()),
                    RegistryKey::Site(site) => crate::retained_by::HeldBy::Site(*site),
                };
                state.retained.retain(|(_, held)| *held != by);
                if let RegistryKey::Site(site) = key
                    && state
                        .contained
                        .get(&site)
                        .is_some_and(|children| !children.is_empty())
                {
                    // Elements whose custody the registry took, dropped
                    // without being released: lost.
                    refuse(
                        function,
                        line,
                        refusal(
                            callee,
                            &format!("elements `{text}` owns are dropped without their release"),
                            ELEMENT_UNDECIDED,
                        ),
                        state,
                        diagnostics,
                    );
                }
            }
            RegistryUse::Unregister { .. } => {
                state.registries.remove(&key);
            }
            RegistryUse::ReleaseIfConstructed { holder } => {
                if constructed
                    && let (RegistryKey::Site(_), RegistryHolder::Name(name)) = (&key, holder)
                {
                    super::apply_resource_action(
                        function,
                        line,
                        name,
                        state,
                        acquisition_lines,
                        diagnostics,
                        false,
                        false,
                    );
                }
            }
        }
    }
}

/// Hook 3: a holder whose registry still holds elements is released only
/// through the registry. `true` when it refused.
pub(super) fn refuse_release_of_registry(
    function: &str,
    line: usize,
    source: &str,
    site: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    let holds = state.registries.contains_key(&RegistryKey::Site(site))
        && state
            .contained
            .get(&site)
            .is_some_and(|children| !children.is_empty());
    if holds {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "`{source}` is released while its registry holds elements it owns \
                     ({ELEMENT_UNDECIDED})"
                ),
            },
        });
    }
    holds
}

/// Whether the formal `name` is rebound or has its address taken in `body`.
fn rebound(body: &[&str], name: &str) -> bool {
    body.iter().enumerate().any(|(at, word)| {
        if *word != name || matches!(at.checked_sub(1).map(|b| body[b]), Some("." | "->")) {
            return false;
        }
        let previous = at.checked_sub(1).map(|b| body[b]);
        let next = body.get(at + 1).copied();
        matches!(previous, Some("&" | "++" | "--"))
            || matches!(
                next,
                Some(
                    "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
                        | "++"
                        | "--"
                )
            )
    })
}

/// Rule 13y: `resource_callback_invoked(F, A…)` stands on a body that names
/// `F` only as the callee of direct calls whose actuals match `A…` position by
/// position (a parameter word: that formal, never rebound; `_`: anything that
/// does not name `F`).
fn invocation_validates(
    body: &[&str],
    parameters: &[String],
    callback: usize,
    actuals: &[CallbackActual],
) -> bool {
    let Some(name) = parameters.get(callback) else {
        return false;
    };
    if rebound(body, name) {
        return false;
    }
    for (at, word) in body.iter().enumerate() {
        if word != name || matches!(at.checked_sub(1).map(|b| body[b]), Some("." | "->")) {
            continue;
        }
        // `F (…)`, or `(*F) (…)` (glibc's inline `bsearch`).
        let call = if body.get(at + 1) == Some(&"(") {
            at
        } else if at >= 2
            && body[at - 2] == "("
            && body[at - 1] == "*"
            && body.get(at + 1) == Some(&")")
            && body.get(at + 2) == Some(&"(")
        {
            at + 1
        } else {
            return false;
        };
        let Some(spans) = super::call_argument_spans(body, call) else {
            return false;
        };
        if spans.len() != actuals.len() {
            return false;
        }
        for (&(start, end), actual) in spans.iter().zip(actuals) {
            let argument = &body[start..end];
            let fits = match actual {
                CallbackActual::Parameter { index } => parameters
                    .get(*index)
                    .is_some_and(|formal| argument == [formal.as_str()] && !rebound(body, formal)),
                CallbackActual::Supplied => !argument.contains(&name.as_str()),
                CallbackActual::Element => false,
            };
            if !fits {
                return false;
            }
        }
    }
    true
}

/// Rule 13y: `resource_callback_registered(F, H)` with `H` a parameter stands
/// on a body that names `F` only as the value of stores `H->M = F;`, at least
/// once, `H` and `F` never rebound.
fn registration_validates(body: &[&str], parameters: &[String], callback: usize, holder: usize) -> bool {
    let (Some(name), Some(holder)) = (parameters.get(callback), parameters.get(holder)) else {
        return false;
    };
    if rebound(body, name) || rebound(body, holder) {
        return false;
    }
    let mut stores = 0usize;
    for (at, word) in body.iter().enumerate() {
        if word != name || matches!(at.checked_sub(1).map(|b| body[b]), Some("." | "->")) {
            continue;
        }
        let store = at >= 4
            && body[at - 4] == holder.as_str()
            && body[at - 3] == "->"
            && is_identifier(body[at - 2])
            && body[at - 1] == "="
            && body.get(at + 1) == Some(&";");
        if !store {
            return false;
        }
        stores += 1;
    }
    stores > 0
}

/// Hook 5 (rule 13y): in a unit that defines a callback declaration's body,
/// the declaration stands only when the body validates each checked callback
/// clause (a synchronous invocation, a registration in a parameter holder);
/// otherwise it is marked withdrawn there, so every call refuses and no
/// inference reads the body in its place. The other callback clauses are
/// declared assumptions. `None` when nothing is withdrawn.
pub(crate) fn settle(
    tokens: &[super::Token],
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let mut withdrawn = Vec::new();
    for (name, declaration) in contracts.retaining() {
        if declaration.callbacks_withdrawn {
            continue;
        }
        let checked = declaration
            .callbacks
            .iter()
            .filter(|clause| {
                matches!(
                    clause,
                    CallbackClause::Invoked { .. }
                        | CallbackClause::Registered {
                            holder: CallbackHolder::Parameter { .. },
                            ..
                        }
                )
            })
            .collect::<Vec<_>>();
        if checked.is_empty() {
            continue;
        }
        let Some((parameters, body)) = super::slot_move_route::defined_body(tokens, name) else {
            continue;
        };
        let valid = checked.iter().all(|clause| match clause {
            CallbackClause::Invoked { callback, actuals } => {
                invocation_validates(&body, &parameters, *callback, actuals)
            }
            CallbackClause::Registered {
                callback,
                holder: CallbackHolder::Parameter { index },
            } => registration_validates(&body, &parameters, *callback, *index),
            _ => false,
        });
        if valid {
            crate::contract_dependencies::validated(name, "13y");
        } else {
            withdrawn.push((name.clone(), declaration.clone()));
        }
    }
    if withdrawn.is_empty() {
        return None;
    }
    let mut settled = contracts.clone();
    for (name, mut declaration) in withdrawn {
        crate::contract_dependencies::withdrawn(&name, "13y", true);
        declaration.callbacks_withdrawn = true;
        settled.insert_retaining(name, declaration);
    }
    Some(settled)
}

/// Rule 13u (W4's order): the in-unit functions `function`'s body names
/// beside a call to a callback-invoking declaration. A designator at a
/// callback position is called during that call, so its summary is derived
/// first, as a direct callee's is. An over-approximation (every candidate the
/// body names once it calls such a declaration), which only orders the
/// derivation; empty for every body that calls none.
pub(crate) fn callback_edges(
    tokens: &[super::Token],
    function: &str,
    candidates: &BTreeSet<String>,
    contracts: &ResourceContracts,
) -> BTreeSet<String> {
    let mut edges = BTreeSet::new();
    if contracts
        .retaining()
        .values()
        .all(|declared| declared.callbacks.is_empty())
    {
        return edges;
    }
    let Some((start, _)) = super::find_function_body(tokens, function) else {
        return edges;
    };
    let Some(end) = super::matching(tokens, start, "{", "}") else {
        return edges;
    };
    let body = &tokens[start + 1..end];
    let invokes = body.iter().enumerate().any(|(at, token)| {
        body.get(at + 1).is_some_and(|next| next.text == "(")
            && contracts
                .retaining_contract(&token.text)
                .is_some_and(|declared| {
                    declared.callbacks.iter().any(|clause| {
                        matches!(
                            clause,
                            CallbackClause::Invoked { .. } | CallbackClause::Table { .. }
                        )
                    })
                })
    });
    if !invokes {
        return edges;
    }
    for (at, token) in body.iter().enumerate() {
        if candidates.contains(&token.text)
            && token.text != function
            && body.get(at + 1).is_none_or(|next| next.text != "(")
        {
            edges.insert(token.text.clone());
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    fn formals(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_measured_invocation_bodies_validate() {
        // `cmt_mpack_unpack_array`'s loop, reduced.
        let body = words(
            "{ uint32_t entry_index ; int result ; for ( entry_index = 0 ; result == 0 ; \
             entry_index ++ ) { result = entry_processor_callback ( reader , entry_index , \
             context ) ; } return result ; }",
        );
        let parameters = formals(&["reader", "entry_processor_callback", "context"]);
        let actuals = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::Supplied,
            CallbackActual::Parameter { index: 2 },
        ];
        assert!(invocation_validates(&body, &parameters, 1, &actuals));
        // glibc's extern inline `bsearch` calls `(*__compar) (__key, __p)`.
        let bsearch = words(
            "{ const void * __p ; int __comparison ; while ( __nmemb ) { __p = ( const void * ) \
             ( ( ( const char * ) __base ) + ( ( __nmemb >> 1 ) * __size ) ) ; __comparison = \
             ( * __compar ) ( __key , __p ) ; if ( __comparison == 0 ) { return ( void * ) __p ; } \
             if ( __comparison > 0 ) { __base = ( ( const char * ) __p ) + __size ; -- __nmemb ; } \
             __nmemb >>= 1 ; } return NULL ; }",
        );
        assert!(invocation_validates(
            &bsearch,
            &formals(&["__key", "__base", "__nmemb", "__size", "__compar"]),
            4,
            &[
                CallbackActual::Parameter { index: 0 },
                CallbackActual::Supplied
            ],
        ));
        // No invocation at all: zero times.
        assert!(invocation_validates(
            &words("{ return 0 ; }"),
            &parameters,
            1,
            &actuals
        ));
    }

    #[test]
    fn a_body_that_keeps_rebinds_or_misplaces_the_callback_does_not_validate() {
        let parameters = formals(&["reader", "cb", "context"]);
        let actuals = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::Supplied,
            CallbackActual::Parameter { index: 2 },
        ];
        for body in [
            // Stored.
            "{ global_cb = cb ; return 0 ; }",
            // Rebound (the default comparator of `rd_kafka_topic_partition_list_sort`).
            "{ if ( ! cb ) cb = other ; return cb ( reader , 0 , context ) ; }",
            // Handed on.
            "{ return forward ( reader , cb , context ) ; }",
            // Another actual at a parameter position.
            "{ return cb ( context , 0 , reader ) ; }",
            // A rebound actual.
            "{ reader = 0 ; return cb ( reader , 0 , context ) ; }",
            // Another arity.
            "{ return cb ( reader , context ) ; }",
        ] {
            assert!(
                !invocation_validates(&words(body), &parameters, 1, &actuals),
                "{body}"
            );
        }
    }

    #[test]
    fn a_registration_validates_only_as_member_stores_into_its_holder() {
        let parameters = formals(&["pk", "data", "callback"]);
        let packer = words("{ pk -> data = data ; pk -> callback = callback ; }");
        assert!(registration_validates(&packer, &parameters, 2, 0));
        for body in [
            "{ pk -> data = data ; }",
            "{ other -> callback = callback ; }",
            "{ pk -> callback = callback ; call ( callback ) ; }",
            "{ pk = 0 ; pk -> callback = callback ; }",
        ] {
            assert!(
                !registration_validates(&words(body), &parameters, 2, 0),
                "{body}"
            );
        }
    }

    #[test]
    fn a_registration_on_some_paths_only_is_undecided() {
        let mut mine = BTreeMap::from([(
            RegistryKey::Frame("l".to_owned()),
            Registration::InForce {
                candidates: Vec::new(),
                constructed: false,
            },
        )]);
        join(&mut mine, &BTreeMap::new());
        assert_eq!(
            mine[&RegistryKey::Frame("l".to_owned())],
            Registration::Undecided
        );
        let mut same = BTreeMap::from([(
            RegistryKey::Site(3),
            Registration::InForce {
                candidates: Vec::new(),
                constructed: true,
            },
        )]);
        let copy = same.clone();
        join(&mut same, &copy);
        assert_eq!(same, copy);
    }
}
