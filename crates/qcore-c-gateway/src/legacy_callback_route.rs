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

use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackHolder, MemberObject};
use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract};
use qcore_schemas::retained_by::RetainingContract;
use serde_json::Value;

use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind, Parser};
use crate::callback_contracts::{
    ARM_UNDECIDED, ARM_UNDECLARED, ARM_WITHDRAWN, CandidateClass, Decision, ELEMENT_UNDECIDED,
    MEMBER_OPEN, REGISTRY_UNDECIDED, RegistryClass, UNDECIDED, WITHDRAWN, classify, decide,
    invocation_detail, refusal, registry_class,
};
use crate::typed_c_ast::{children, kind};
use qcore_schemas::arm_release::{ArmEffect, ArmJoin, join_at};

/// A registry holder's actual, as spelled.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RegistryHolder {
    /// `&L`, `L` an automatic object of this frame named only at registry and
    /// retention holder positions.
    Frame(String),
    /// A plain name.
    Name(String),
    /// The object the call returns.
    Result,
}

/// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 12m): the invocation a
/// candidate is classified at, for the caller branch check.
pub(super) struct HandOffAt<'a> {
    /// The declared invoker (the called declaration).
    pub invoker: &'a str,
    pub invocation: crate::handoff_branch::Invocation,
}

/// The classes of one invocation's candidates, with what the composition of
/// a candidate's own invocations (card HANDOFF-CANDIDATE-OWN-CALLBACKS, rule
/// 13u amended) read.
pub(super) struct Classified {
    pub classes: Vec<(String, CandidateClass)>,
    /// A composed candidate's contract: its custody contract with every
    /// requirement its own invocations put on its formals.
    pub composed: BTreeMap<String, ResourceFunctionContract>,
    /// Why a composed candidate refuses.
    pub refused: BTreeMap<String, String>,
    /// Every declaration a composed candidate's own invocations read (their
    /// candidates, transitively).
    pub inner: BTreeMap<String, BTreeSet<String>>,
}

impl Classified {
    /// The refusal detail of a decision over these classes.
    fn detail(&self, reason: &str, candidate: Option<&str>, actual: &str) -> String {
        let detail = invocation_detail(reason, candidate, actual);
        match candidate.and_then(|name| self.refused.get(name)) {
            Some(why) if reason == crate::candidate_composition::OWN_REFUSED => {
                format!("{detail}: {why}")
            }
            _ => detail,
        }
    }

    /// Card HANDOFF-CANDIDATE-OWN-CALLBACKS: note, for the row, every
    /// declaration a decided invocation over `candidates` read: each
    /// candidate, and a composed candidate's inner candidates.
    fn note_relied(&self, candidates: &BTreeSet<String>) {
        for name in candidates {
            crate::contract_dependencies::relied(name);
            for inner in self.inner.get(name).into_iter().flatten() {
                crate::contract_dependencies::relied(inner);
            }
        }
    }

    /// Every declaration a decided invocation over `candidates` read.
    fn relied(&self, candidates: &BTreeSet<String>) -> BTreeSet<String> {
        let mut out = candidates.clone();
        for name in candidates {
            out.extend(self.inner.get(name).into_iter().flatten().cloned());
        }
        out
    }
}

/// One registry effect of a call, in clause order.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Operands {
    pub uses: Vec<RegistryUse>,
    /// The holder actual's text, per use (for refusals).
    pub texts: Vec<String>,
    /// The call's parameters a callback reads at a `resource_owned(n)`
    /// position, with `n` (every candidate's `n` joined): the classifier
    /// reads each as `resource_owned(n)` on the call itself.
    pub owned: BTreeMap<usize, bool>,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): the
    /// type-switched release, read under the cautious join of its arms.
    pub arms: Option<ArmOperands>,
    /// Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): what a
    /// constructor states of its result.
    pub constructed: Option<super::constructed_route::ConstructedOperands>,
    /// Card OP-CONSTRUCTOR-DECLARATIONS (rules 13w, 14e): member invocations
    /// the unit's closure refuses, read against the object's path state.
    pub member_uses: Vec<super::constructed_route::MemberUse>,
    /// Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): the candidates'
    /// member-path states at each parameter position of a decided
    /// invocation, composed with the arms before them.
    pub member_paths: Vec<super::member_path_route::MemberPathOperands>,
    /// Card FB-TEARDOWN-DESTROY-DECLARATIONS (rule 30 at a direct call): the
    /// states a direct call of a member-path declaration leaves at each
    /// parameter position, each guard decided at the call.
    pub direct_member_paths: Vec<super::member_path_route::DirectOperands>,
}

/// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): the arms of a
/// type-switched release at one call, joined over every arm (the
/// discriminant is never decided at a call: no declared constructor states
/// it, `OP-CONSTRUCTOR-DECLARATIONS`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArmOperands {
    /// The discriminated object's actual, as spelled.
    pub object: Vec<String>,
    /// Every member path some arm releases, with the join of the arms there.
    pub released: BTreeMap<String, ArmJoin>,
    /// Card OP-CONSTRUCTOR-DECLARATIONS (rule 12o): the decided-arm reading,
    /// for an object whose path state decides its discriminant.
    pub decided: super::constructed_route::DecidedArms,
}

/// What a holder keys on in the path state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum RegistryKey {
    Frame(String),
    Site(usize),
}

/// A registration as the path state keeps it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
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

/// Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): whether a declaration
/// carries nothing beside its custody contract, its hand-off and its
/// member-path states (the loader admits nothing else beside them).
fn member_path_plain(declared: &RetainingContract) -> bool {
    declared.retentions.is_empty()
        && declared.resets.is_empty()
        && declared.slot_moves.is_empty()
        && declared.derived_outs.is_empty()
        && declared.references.is_empty()
        && declared.callbacks.is_empty()
        && declared.discriminant.is_none()
        && declared.arms.is_empty()
        && declared.constructed.is_empty()
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
    contracts
        .retaining_contract(callee)
        .is_some_and(|declared| {
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
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("FunctionToPointerDecay") => {
                node = children(node).first()?;
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl") => {
                return node["referencedDecl"]["name"].as_str();
            }
            _ => return None,
        }
    }
}

/// Card OP-CONSTRUCTOR-DECLARATIONS: every call node below `node`.
pub(super) fn calls_of<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    calls(node, out);
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
    /// Every Clang call of `callee` on `line` of the current function is
    /// read; the answer stands only when there is one, or when all agree (rule
    /// 11p's placement: glibc's `__glibc_const_generic` puts two `bsearch`
    /// calls, one per association, on the line the token stream reads as one).
    fn callback_site<T: PartialEq>(
        &self,
        callee: &str,
        line: usize,
        mut read: impl FnMut(&Value, &Value, &crate::indirect_calls::TableFacts) -> T,
    ) -> Option<T> {
        crate::typed_access_path::with_indirect_call_unit(|unit| {
            let function = unit.function(&self.typed_path_function)?;
            let mut found = Vec::new();
            calls(function, &mut found);
            let mut answers = found
                .into_iter()
                .filter(|call| {
                    direct_callee(call) == Some(callee) && unit.node_line(call) == Some(line)
                })
                .map(|call| read(call, function, unit.tables()));
            let first = answers.next()?;
            answers.all(|answer| answer == first).then_some(first)
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
        at: Option<&HandOffAt<'_>>,
    ) -> Classified {
        self.classified_in(candidates, actuals, at, &mut Vec::new())
    }

    /// [`Self::classified`] under the composition's decision stack (the
    /// candidates whose own invocations are being decided).
    fn classified_in(
        &self,
        candidates: &BTreeSet<String>,
        actuals: &[CallbackActual],
        at: Option<&HandOffAt<'_>>,
        stack: &mut Vec<String>,
    ) -> Classified {
        let mut composed = BTreeMap::new();
        let mut refused = BTreeMap::new();
        let mut inner = BTreeMap::new();
        let classes = candidates
            .iter()
            .map(|name| {
                let contract = self.contracts.contract(name);
                // Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 13u amended): a
                // candidate whose declaration carries its own member
                // invocations is the composition of its own clauses, decided
                // here; contracted only when each own invocation decides.
                if contract.is_none()
                    && let Some(declared) = self.contracts.retaining_contract(name)
                    && crate::candidate_composition::composable(declared)
                {
                    let class = match self.composed(name, declared, actuals, stack) {
                        Ok((effective, read)) => {
                            inner.insert(name.clone(), read);
                            let class = match &declared.handoff {
                                Some(handoff) => {
                                    self.handed_with(name, &effective, handoff, actuals, at)
                                }
                                None => classify(Some(&effective), actuals),
                            };
                            composed.insert(name.clone(), effective);
                            class
                        }
                        Err(why) => {
                            refused.insert(name.clone(), why);
                            CandidateClass::OwnInvocationRefused
                        }
                    };
                    return (name.clone(), class);
                }
                // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 12m): a
                // candidate whose declaration hands a formal depending on its
                // return code (withheld from the function map).
                if contract.is_none()
                    && let Some(declared) = self.contracts.retaining_contract(name)
                    && let Some(handoff) = &declared.handoff
                {
                    return (
                        name.clone(),
                        self.handed_class(name, declared, handoff, actuals, at),
                    );
                }
                // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): a candidate
                // stating member-path states and nothing else beside its
                // custody contract reads as that contract; its states are
                // composed at the invocation (`member_path_route`). Withdrawn
                // in this unit, it has no contract in force here.
                if contract.is_none()
                    && let Some(declared) = self.contracts.retaining_contract(name)
                    && !declared.member_paths.is_empty()
                {
                    let class = if declared.member_paths_withdrawn || !member_path_plain(declared) {
                        CandidateClass::Missing
                    } else {
                        classify(Some(&declared.contract), actuals)
                    };
                    return (name.clone(), class);
                }
                // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12m): a
                // candidate that takes the supplied object only on some
                // return codes (a disposition, never in the function map)
                // leaves it with the callee on the others.
                let class = if contract.is_none()
                    && actuals.contains(&CallbackActual::SuppliedOwned)
                    && self.contracts.relational_contract(name).is_some()
                {
                    CandidateClass::KeepsSupplied
                } else {
                    classify(contract, actuals)
                };
                (name.clone(), class)
            })
            .collect();
        Classified {
            classes,
            composed,
            refused,
            inner,
        }
    }

    /// Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 13u amended): decide each
    /// of `name`'s own member invocations as its own body would face it (the
    /// run's set, `name` the invoker), under the decision stack; `Ok` with
    /// its contract strengthened by every requirement they put on its
    /// formals, `Err` naming the first that does not decide. `outer` is the
    /// invocation's actuals: an inner member-path state at a formal it hands
    /// a parameter of the caller is not composed.
    fn composed(
        &self,
        name: &str,
        declared: &RetainingContract,
        outer: &[CallbackActual],
        stack: &mut Vec<String>,
    ) -> Result<(ResourceFunctionContract, BTreeSet<String>), String> {
        if crate::member_run_facts::installed().is_none() {
            return Err(
                "its own member invocations are decided only over the run's set, which this \
                 route does not read (no pre-pass)"
                    .to_owned(),
            );
        }
        crate::candidate_composition::with_stack(stack, name, |stack| {
            let mut effective = declared.contract.clone();
            let mut read = BTreeSet::new();
            for clause in &declared.callbacks {
                let CallbackClause::MemberInvoked {
                    object,
                    member,
                    actuals,
                } = clause
                else {
                    return Err("an own callback clause that is not a member invocation".into());
                };
                let spelled = member.spelling();
                let candidates = self.member_candidates(&spelled).map_err(|why| {
                    format!(
                        "its own invocation of `{spelled}` does not decide: the callbacks stored \
                         there are not a closed set of functions ({why}) ({MEMBER_OPEN})"
                    )
                })?;
                let at = HandOffAt {
                    invoker: name,
                    invocation: crate::handoff_branch::Invocation::Member {
                        object: match object {
                            MemberObject::Parameter { index } => Some(*index),
                            MemberObject::SuppliedOwned => None,
                        },
                        path: member.path.clone(),
                    },
                };
                let inner = self.classified_in(&candidates, actuals, Some(&at), stack);
                if let Decision::Refused(reason, candidate) =
                    crate::callback_contracts::decide_run(Ok(&inner.classes))
                {
                    return Err(format!(
                        "its own invocation of `{spelled}` does not decide: {} ({reason})",
                        inner.detail(reason, candidate.as_deref(), &spelled)
                    ));
                }
                read.extend(inner.relied(&candidates));
                let mut owned = BTreeMap::new();
                self.require(&candidates, actuals, &inner, &mut owned);
                for (index, nullable) in owned {
                    crate::candidate_composition::strengthen(&mut effective, index, nullable);
                }
                let stated = super::member_path_route::operands(
                    self.contracts,
                    &candidates,
                    actuals,
                    &|_| None,
                    &spelled,
                );
                for operands in stated {
                    let Some(CallbackActual::Parameter { index }) = actuals.get(operands.position)
                    else {
                        continue;
                    };
                    if matches!(outer.get(*index), Some(CallbackActual::Parameter { .. })) {
                        return Err(format!(
                            "its own invocation of `{spelled}` states member-path states at its \
                             parameter {index}, which this invocation hands a parameter of the \
                             caller (two levels of rule 30 are not composed)"
                        ));
                    }
                }
            }
            Ok((effective, read))
        })
    }

    /// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 12m): a candidate whose
    /// declaration hands its formal `handoff.parameter` to the callback
    /// depending on its return code. A declaration that carries any other
    /// side clause still reads uncontracted (rule 13u: a retaining
    /// declaration is not in the function map). Otherwise the invoker's
    /// branch is checked ([`crate::handoff_branch::admit`]); admitted, the
    /// candidate is classified by its custody contract with the handed formal
    /// taken at a `supplied_owned` position and kept by the invoker at `_` or
    /// a parameter (whose every admitted path ends inside the invoker).
    fn handed_class(
        &self,
        name: &str,
        declared: &RetainingContract,
        handoff: &qcore_schemas::handoff::HandOff,
        actuals: &[CallbackActual],
        at: Option<&HandOffAt<'_>>,
    ) -> CandidateClass {
        let plain = declared.retentions.is_empty()
            && declared.resets.is_empty()
            && declared.slot_moves.is_empty()
            && declared.derived_outs.is_empty()
            && declared.references.is_empty()
            && declared.callbacks.is_empty()
            && declared.discriminant.is_none()
            && declared.arms.is_empty()
            && declared.constructed.is_empty();
        // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): member-path states
        // beside the hand-off are read at the invocation; withdrawn in this
        // unit, the candidate has no contract in force here.
        if !plain || declared.member_paths_withdrawn {
            return CandidateClass::Missing;
        }
        self.handed_with(name, &declared.contract, handoff, actuals, at)
    }

    /// The caller branch check of a hand-off candidate whose contract (its
    /// declared one, or a composed one) is `contract`: admitted, classified
    /// with the handed formal taken at a `supplied_owned` position.
    fn handed_with(
        &self,
        name: &str,
        contract: &ResourceFunctionContract,
        handoff: &qcore_schemas::handoff::HandOff,
        actuals: &[CallbackActual],
        at: Option<&HandOffAt<'_>>,
    ) -> CandidateClass {
        match self.handoff_admission(name, handoff, actuals, at) {
            Ok(()) => {
                let mut contract = contract.clone();
                if actuals.get(handoff.parameter) == Some(&CallbackActual::SuppliedOwned) {
                    contract.parameters[handoff.parameter] =
                        qcore_schemas::contract::ResourceParameterMode::Consumed {
                            nullable: false,
                        };
                }
                classify(Some(&contract), actuals)
            }
            Err(_) => CandidateClass::HandOffUnbranched,
        }
    }

    /// Why the invoker's branch does not admit `name`'s hand-off (`Ok` when
    /// it does).
    fn handoff_admission(
        &self,
        name: &str,
        handoff: &qcore_schemas::handoff::HandOff,
        actuals: &[CallbackActual],
        at: Option<&HandOffAt<'_>>,
    ) -> Result<(), String> {
        use crate::handoff_branch::{Handed, Query, UnitBodies, admit, resolve};
        let at = at.ok_or("a table dispatch reads no invoker's branch")?;
        let handed = match actuals.get(handoff.parameter) {
            Some(CallbackActual::SuppliedOwned | CallbackActual::Supplied) => Handed::Own,
            Some(CallbackActual::Parameter { index }) => Handed::Parameter(*index),
            _ => return Err("the handed formal has no handed actual".into()),
        };
        let check = |units: &[&crate::callback_member_closure::MemberFacts]| {
            let value = resolve(units, name, &handoff.guard.constant)?;
            admit(
                &UnitBodies(units.to_vec()),
                &Query {
                    invoker: at.invoker,
                    invocation: at.invocation.clone(),
                    position: handoff.parameter,
                    handed,
                    guard: &handoff.guard,
                    value,
                },
            )
        };
        // The run's facts (the pre-pass) where installed; otherwise the
        // analysing unit's own.
        if let Some(run) = crate::member_run_facts::installed() {
            let units = run.units.iter().collect::<Vec<_>>();
            return check(&units);
        }
        crate::typed_access_path::with_indirect_call_unit(|unit| check(&[unit.member_facts()]))
            .unwrap_or_else(|| Err("the unit's AST is not read".into()))
    }

    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the candidates
    /// of `member` over this unit's stores, `Err` with why the unit does not
    /// close it.
    fn member_candidates(&self, member: &str) -> Result<BTreeSet<String>, String> {
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n; user decisions R1,
        // E1): with the run's facts installed by the pre-pass, the unit
        // decides over the run's set, and a closed empty set invokes nothing;
        // a member that is not a function-pointer member of a record the run
        // defines is never decided so.
        if let Some(run) = crate::member_run_facts::installed() {
            run.normalise(member).map_err(|why| {
                format!("`{member}` is not a member the run's records define ({why})")
            })?;
            return run
                .closure(member)
                .map_err(|open| open.into_iter().take(3).collect::<Vec<_>>().join("; "));
        }
        let closure = crate::typed_access_path::with_indirect_call_unit(|unit| {
            crate::callback_member_closure::closure(&[unit.member_facts()], member)
        })
        .ok_or_else(|| "the unit's AST is not read".to_owned())?;
        match closure {
            Ok(names) if names.is_empty() => Err(format!(
                "no store into `{member}` in this unit names a function"
            )),
            Ok(names) => Ok(names),
            Err(open) => Err(open.into_iter().take(3).collect::<Vec<_>>().join("; ")),
        }
    }

    /// The requirements every candidate puts on the call's parameters
    /// ([`crate::callback_contracts::requirements`]), joined into `owned`.
    fn require(
        &self,
        candidates: &BTreeSet<String>,
        actuals: &[CallbackActual],
        classified: &Classified,
        owned: &mut BTreeMap<usize, bool>,
    ) {
        for name in candidates {
            // Card HANDOFF-CANDIDATE-OWN-CALLBACKS: a composed candidate's
            // requirements are its composed contract's. Card
            // SUPPLIED-OWNED-RETURN-CODE-HANDOFF: a hand-off candidate's
            // requirements are its custody contract's.
            let contract = classified.composed.get(name);
            let contract = contract
                .or_else(|| self.contracts.contract(name))
                .or_else(|| {
                    self.contracts
                        .retaining_contract(name)
                        .filter(|declared| {
                            declared.handoff.is_some() || !declared.member_paths.is_empty()
                        })
                        .map(|declared| &declared.contract)
                });
            for (index, nullable) in crate::callback_contracts::requirements(contract, actuals) {
                owned
                    .entry(index)
                    .and_modify(|all| *all &= nullable)
                    .or_insert(nullable);
            }
        }
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
        // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 12m): a declaration
        // that hands a formal to its callback depending on its return code
        // is read as a candidate of a callback clause. Card
        // HANDOFF-DIRECT-CALL-READING (rule 12m amended): a direct call is
        // read only on an edge of the caller's own test of its result, where
        // the calling body's reading admits it; every other direct call
        // refuses.
        if let Some(handoff) = &declaration.handoff
            && !super::handoff_direct_route::edge_in_force(function)
        {
            return Err(self.handoff_direct_refusal(function, handoff));
        }
        // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): member-path states
        // beside a hand-off are read only at a callback invocation, composed
        // with the invoker's arms. Card FB-TEARDOWN-DESTROY-DECLARATIONS: a
        // direct call of any other member-path declaration reads its states
        // once (`member_path_route::apply_direct`); withdrawn in this unit,
        // it has no states in force and refuses.
        if !declaration.member_paths.is_empty() {
            if declaration.handoff.is_some() || !member_path_plain(declaration) {
                return Err(refusal(
                    function,
                    "it states what it leaves at member paths of a formal's object beside a \
                     hand-off, which only a callback invocation composes",
                    super::member_path_route::DIRECT,
                ));
            }
            if declaration.member_paths_withdrawn {
                return Err(refusal(
                    function,
                    "its body in this unit contradicts the member-path states it declares",
                    super::member_path_route::UNSTATED,
                ));
            }
            let decide = |guard: &qcore_schemas::member_path::ParameterGuard| {
                let words = actual(guard.parameter);
                let value = words.as_ref().and_then(|words| {
                    let words = words.iter().map(String::as_str).collect::<Vec<_>>();
                    super::retention_guard::constant_value(
                        self.tokens,
                        &self.typed_path_function,
                        &words,
                    )
                });
                let constant = super::member_path_route::guard_constant(guard, &|name| {
                    crate::typed_access_path::with_indirect_call_unit(|unit| {
                        crate::handoff_branch::resolve_parameter(
                            &[unit.member_facts()],
                            function,
                            guard.parameter,
                            name,
                        )
                        .ok()
                    })
                    .flatten()
                });
                guard.decide(value, constant)
            };
            return Ok(Some(Operands {
                uses: Vec::new(),
                texts: Vec::new(),
                owned: BTreeMap::new(),
                arms: None,
                constructed: None,
                member_uses: Vec::new(),
                member_paths: Vec::new(),
                direct_member_paths: super::member_path_route::direct_operands(
                    &declaration.member_paths,
                    actual,
                    &decide,
                ),
            }));
        }
        if declaration.callbacks_withdrawn {
            return Err(refusal(
                function,
                "its body in this unit does not validate the declared callback clause",
                WITHDRAWN,
            ));
        }
        if declaration.arms_withdrawn {
            return Err(refusal(
                function,
                "its body in this unit does not validate the declared arms",
                ARM_WITHDRAWN,
            ));
        }
        if declaration.callbacks.is_empty()
            && declaration.discriminant.is_none()
            && declaration.constructed.is_empty()
        {
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
                &format!(
                    "the registry holder `{}` is not one this body names",
                    text(index)
                ),
                REGISTRY_UNDECIDED,
            )
        };
        let mut operands = Operands {
            uses: Vec::new(),
            texts: Vec::new(),
            owned: BTreeMap::new(),
            arms: None,
            constructed: None,
            member_uses: Vec::new(),
            member_paths: Vec::new(),
            direct_member_paths: Vec::new(),
        };
        // Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): a
        // constructor's stated result state, unless its body in this unit
        // withdrew it (rule 14e: the result is then tracked, its state
        // undecided).
        if !declaration.constructed.is_empty() && !declaration.constructed_withdrawn {
            operands.constructed = Some(self.constructed_operands(
                function,
                &declaration.constructed,
                line,
                |callback| {
                    self.callback_candidates(function, line, callback, true)
                        .ok()
                },
            ));
        }
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): the
        // discriminant is undecided at every call, so the arms are read under
        // their cautious join; with no `otherwise`, some value has no
        // declared arm, which is never read as neutral.
        if let Some(discriminant) = &declaration.discriminant {
            if !qcore_schemas::arm_release::has_otherwise(&declaration.arms) {
                return Err(refusal(
                    function,
                    &format!(
                        "the discriminant `{}->{}` is not decided here and some value has no \
                         declared arm",
                        text(discriminant.parameter),
                        discriminant.field
                    ),
                    ARM_UNDECLARED,
                ));
            }
            let released = declaration
                .arms
                .iter()
                .filter_map(|arm| match &arm.effect {
                    ArmEffect::Released { path } => Some(path.clone()),
                    ArmEffect::Empty => None,
                })
                .map(|path| {
                    let join = join_at(&declaration.arms, &path);
                    (path, join)
                })
                .collect();
            operands.arms = Some(ArmOperands {
                object: actual(discriminant.parameter).unwrap_or_default(),
                released,
                decided: self.decided_arms(declaration, &discriminant.field),
            });
        }
        for clause in &declaration.callbacks {
            match clause {
                CallbackClause::Invoked { callback, actuals } => {
                    // Card SERVE-FORWARDED-CALLBACK-FORMALS (rule 15w): the
                    // calling function's own callback formal, handed on
                    // exactly as its declaration in force states it, is that
                    // declared invocation, decided at its own callers.
                    if self.forwarded_invocation(declaration, *callback, actual) {
                        continue;
                    }
                    let resolution = self.callback_candidates(function, line, *callback, false);
                    let at = HandOffAt {
                        invoker: function,
                        invocation: crate::handoff_branch::Invocation::Formal(*callback),
                    };
                    let classes = resolution
                        .as_ref()
                        .map(|candidates| self.classified(candidates, actuals, Some(&at)));
                    let decision = decide(
                        classes
                            .as_ref()
                            .map(|classes| classes.classes.as_slice())
                            .map_err(|_| ()),
                        false,
                    );
                    if let Decision::Refused(reason, candidate) = decision {
                        let detail = match (&resolution, &classes) {
                            (Err(why), _) if reason == UNDECIDED => format!(
                                "the callback `{}` is not a closed set of functions ({why})",
                                text(*callback)
                            ),
                            (_, Ok(classes)) => {
                                classes.detail(reason, candidate.as_deref(), &text(*callback))
                            }
                            _ => invocation_detail(reason, candidate.as_deref(), &text(*callback)),
                        };
                        return Err(refusal(function, &detail, reason));
                    }
                    if let (Ok(candidates), Ok(classes)) = (&resolution, &classes) {
                        classes.note_relied(candidates);
                        self.require(candidates, actuals, classes, &mut operands.owned);
                        operands
                            .member_paths
                            .extend(super::member_path_route::operands(
                                self.contracts,
                                candidates,
                                actuals,
                                actual,
                                &text(*callback),
                            ));
                    }
                }
                CallbackClause::Table { table, actuals } => {
                    let resolution = self.table_candidates(function, line, *table);
                    let classes = resolution
                        .as_ref()
                        .map(|candidates| self.classified(candidates, actuals, None));
                    let decision = decide(
                        classes
                            .as_ref()
                            .map(|classes| classes.classes.as_slice())
                            .map_err(|_| ()),
                        false,
                    );
                    if let Decision::Refused(reason, candidate) = decision {
                        let detail = match (&resolution, &classes) {
                            (Err(why), _) if reason == UNDECIDED => format!(
                                "the handlers of `{}` are not a closed set of functions ({why})",
                                text(*table)
                            ),
                            (_, Ok(classes)) => {
                                classes.detail(reason, candidate.as_deref(), &text(*table))
                            }
                            _ => invocation_detail(reason, candidate.as_deref(), &text(*table)),
                        };
                        return Err(refusal(function, &detail, reason));
                    }
                    if let (Ok(candidates), Ok(classes)) = (&resolution, &classes) {
                        classes.note_relied(candidates);
                        self.require(candidates, actuals, classes, &mut operands.owned);
                    }
                }
                CallbackClause::Registered {
                    callback,
                    holder: at,
                } => {
                    let candidates = self
                        .callback_candidates(function, line, *callback, true)
                        .map_err(|why| {
                            refusal(
                                function,
                                &format!(
                                    "the callback `{}` is not a closed set of functions ({why})",
                                    text(*callback)
                                ),
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
                    operands
                        .uses
                        .push(RegistryUse::ReleaseIfConstructed { holder });
                }
                // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, Q2): a
                // setter's registration changes nothing on the caller's
                // state; its actual is a store into the member for the run's
                // closure (read from the call's facts), and every invocation
                // of the member is decided where it is made.
                CallbackClause::MemberRegistered { .. } => {}
                // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the
                // candidates are the unit's closed set of stores into the
                // member (the run's fold checks the unit saw them all), then
                // decided as rule 13u's.
                CallbackClause::MemberInvoked {
                    object,
                    member,
                    actuals,
                } => {
                    let spelled = member.spelling();
                    // Card OP-CONSTRUCTOR-DECLARATIONS (rules 13w, 14e): a
                    // refusal over the unit's closure is deferred to the
                    // transfer when the object may carry a constructor's
                    // decided state (a stated member the unit's stores keep,
                    // stored nowhere in this function); with no such state
                    // there, the same refusal is made there.
                    let deferred = |description: String| {
                        let MemberObject::Parameter { index } = object else {
                            return None;
                        };
                        let words = actual(*index)?;
                        let [name] = words.as_slice() else {
                            return None;
                        };
                        (is_identifier(name)
                            && self.states_member(&spelled)
                            && self.usable(&spelled, false).is_some())
                        .then(|| super::constructed_route::MemberUse {
                            object: name.clone(),
                            key: spelled.clone(),
                            actuals: actuals.clone(),
                            refusal: description,
                            member_paths: self
                                .contracts
                                .retaining()
                                .iter()
                                .filter(|(name, declared)| {
                                    !declared.member_paths.is_empty()
                                        && !declared.member_paths_withdrawn
                                        && declared.handoff.is_none()
                                        && member_path_plain(declared)
                                        && self.contracts.contract(name).is_none()
                                })
                                .map(|(name, declared)| {
                                    (
                                        name.clone(),
                                        (
                                            declared.contract.clone(),
                                            super::member_path_route::invocation_states(
                                                &declared.member_paths,
                                            ),
                                        ),
                                    )
                                })
                                .collect(),
                        })
                    };
                    let candidates = match self.member_candidates(&spelled) {
                        Ok(candidates) => candidates,
                        Err(why) => {
                            let description = refusal(
                                function,
                                &format!(
                                    "the callbacks stored in `{spelled}` are not a closed set of \
                                     functions ({why})"
                                ),
                                MEMBER_OPEN,
                            );
                            match deferred(description.clone()) {
                                Some(use_) => {
                                    operands.member_uses.push(use_);
                                    continue;
                                }
                                None => return Err(description),
                            }
                        }
                    };
                    let at = HandOffAt {
                        invoker: function,
                        invocation: crate::handoff_branch::Invocation::Member {
                            object: match object {
                                MemberObject::Parameter { index } => Some(*index),
                                MemberObject::SuppliedOwned => None,
                            },
                            path: member.path.clone(),
                        },
                    };
                    let classes = self.classified(&candidates, actuals, Some(&at));
                    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (E1): an empty set
                    // reaches here only from the run's closed set (a unit's own
                    // empty set is refused above), and invokes nothing.
                    let decision = if crate::member_run_facts::installed().is_some() {
                        crate::callback_contracts::decide_run(Ok(&classes.classes))
                    } else {
                        decide(Ok(&classes.classes), false)
                    };
                    if let Decision::Refused(reason, candidate) = decision {
                        let description = refusal(
                            function,
                            &classes.detail(reason, candidate.as_deref(), &spelled),
                            reason,
                        );
                        match deferred(description.clone()) {
                            Some(use_) => {
                                operands.member_uses.push(use_);
                                continue;
                            }
                            None => return Err(description),
                        }
                    }
                    classes.note_relied(&candidates);
                    self.require(&candidates, actuals, &classes, &mut operands.owned);
                    operands
                        .member_paths
                        .extend(super::member_path_route::operands(
                            self.contracts,
                            &candidates,
                            actuals,
                            actual,
                            &spelled,
                        ));
                }
            }
        }
        Ok((!operands.uses.is_empty()
            || !operands.owned.is_empty()
            || operands.arms.is_some()
            || operands.constructed.is_some()
            || !operands.member_uses.is_empty()
            || !operands.member_paths.is_empty())
        .then_some(operands))
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
    // Card OP-CONSTRUCTOR-DECLARATIONS (rules 13w, 14e): a member invocation
    // the closure refused reads the object's path state, before the call's
    // own effects.
    for use_ in &operands.member_uses {
        match super::constructed_route::decide_member(state, use_) {
            Ok(decided) => {
                // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): the decided
                // candidates' member-path states, composed with the arms.
                for (position, handed) in use_.actuals.iter().enumerate() {
                    let CallbackActual::Parameter { .. } = handed else {
                        continue;
                    };
                    let candidates = decided
                        .iter()
                        .map(|(name, states)| {
                            (
                                name.clone(),
                                states
                                    .iter()
                                    .filter(|state| state.parameter == position)
                                    .cloned()
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>();
                    if candidates.iter().all(|(_, states)| states.is_empty()) {
                        continue;
                    }
                    super::member_path_route::apply(
                        function,
                        line,
                        callee,
                        &super::member_path_route::MemberPathOperands {
                            object: vec![use_.object.clone()],
                            position,
                            candidates,
                            member: use_.key.clone(),
                        },
                        operands.arms.as_ref(),
                        state,
                        diagnostics,
                    );
                }
            }
            Err(super::constructed_route::MemberRefusal::Undecided) => {
                refuse(function, line, use_.refusal.clone(), state, diagnostics);
            }
            Err(super::constructed_route::MemberRefusal::Decided(reason, candidate)) => refuse(
                function,
                line,
                refusal(
                    callee,
                    &invocation_detail(reason, candidate.as_deref(), &use_.key),
                    reason,
                ),
                state,
                diagnostics,
            ),
        }
    }
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): the callbacks run
    // before the release's arms; a linked path some candidate states is
    // decided by their composition with the arms, every other path by the
    // arms as before.
    // Card FB-TEARDOWN-DESTROY-DECLARATIONS (rule 30 at a direct call).
    for direct in &operands.direct_member_paths {
        super::member_path_route::apply_direct(function, line, callee, direct, state, diagnostics);
    }
    for member_paths in &operands.member_paths {
        super::member_path_route::apply(
            function,
            line,
            callee,
            member_paths,
            operands.arms.as_ref(),
            state,
            diagnostics,
        );
    }
    if let Some(arms) = &operands.arms {
        apply_arms(function, line, callee, arms, state, diagnostics);
    }
    // Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): the stated
    // result state goes onto the tracked result.
    if let Some(constructed) = &operands.constructed
        && let Some(RegistryKey::Site(site)) = key_of(state, &RegistryHolder::Result, destination)
    {
        super::constructed_route::record(state, site, constructed);
    }
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
                if candidates.iter().any(|(_, contract)| {
                    !crate::callback_contracts::requirements(contract.as_ref(), actuals).is_empty()
                }) {
                    // A registered callback that requires the caller's actual
                    // live is not decided here (the conservative reading).
                    refuse(
                        function,
                        line,
                        refusal(
                            callee,
                            &format!("a callback `{text}` keeps requires a caller's object live"),
                            crate::callback_contracts::EFFECTFUL,
                        ),
                        state,
                        diagnostics,
                    );
                    continue;
                }
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
                        [name] if is_identifier(name) => Some(
                            super::retained_by_route::RetainedSpelling::Name(name.clone()),
                        ),
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
                super::retained_by_route::holder_ended(state, &by);
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

/// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): after the declared
/// release of the discriminated object, each object a member of it links is
/// released when every arm releases that member, refused when only some do,
/// and left as a plain release leaves it (premise P-B08-SHALLOW) when none
/// does.
fn apply_arms(
    function: &str,
    line: usize,
    callee: &str,
    arms: &ArmOperands,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let [name] = arms.object.as_slice() else {
        return;
    };
    let Some(pointer) = state.pointers.get(name).cloned() else {
        return;
    };
    for site in &pointer.targets {
        let Some(fields) = state.field_links.get(site).cloned() else {
            continue;
        };
        for (field, children) in fields {
            // Card OP-CONSTRUCTOR-DECLARATIONS (rule 12o): a decided
            // discriminant selects exactly its reachable values' arms.
            let join = match super::constructed_route::decided_arm_join(
                state,
                name,
                &arms.decided,
                &field,
            ) {
                Some(Some(join)) => join,
                Some(None) => {
                    refuse(
                        function,
                        line,
                        refusal(
                            callee,
                            &format!(
                                "the decided discriminant of `{name}` reaches a value with no \
                                 declared arm"
                            ),
                            ARM_UNDECLARED,
                        ),
                        state,
                        diagnostics,
                    );
                    continue;
                }
                None => arms.released.get(&field).copied().unwrap_or(ArmJoin::None),
            };
            match join {
                ArmJoin::None => {}
                ArmJoin::Some => refuse(
                    function,
                    line,
                    refusal(
                        callee,
                        &format!(
                            "`{name}->{field}` links an object some arms release and others do not"
                        ),
                        ARM_UNDECIDED,
                    ),
                    state,
                    diagnostics,
                ),
                ArmJoin::Every => {
                    let single = !pointer.unknown && pointer.targets.len() == 1;
                    let live = children.iter().all(|child| {
                        state.resources.get(child).copied().unwrap_or(0) & super::LIVE != 0
                    });
                    if !single || !live {
                        refuse(
                            function,
                            line,
                            refusal(
                                callee,
                                &format!(
                                    "`{name}->{field}` does not link one live object every arm \
                                     releases"
                                ),
                                ARM_UNDECIDED,
                            ),
                            state,
                            diagnostics,
                        );
                        continue;
                    }
                    for child in &children {
                        let status = state.resources.get(child).copied().unwrap_or(0);
                        state.resources.insert(
                            *child,
                            (status & !(super::LIVE | super::CONTAINED)) | super::RELEASED,
                        );
                    }
                    if let Some(links) = state.field_links.get_mut(site) {
                        links.remove(&field);
                        if links.is_empty() {
                            state.field_links.remove(site);
                        }
                    }
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

/// The parameter names of `function`'s definition, read as rule 12w's
/// `definition` reads them, except that an inline function-pointer
/// declarator `RET (*NAME)(…)` names `NAME` (`rd_list_init`'s `free_cb`);
/// any other parenthesised declarator names nothing, which the check refuses.
pub(super) fn parameter_names(tokens: &[super::Token], function: &str) -> Option<Vec<String>> {
    let (open, _) = super::find_function_body(tokens, function)?;
    let close = open.checked_sub(1)?;
    let mut depth = 0usize;
    let mut start = None;
    for at in (0..=close).rev() {
        match tokens[at].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    start = Some(at);
                    break;
                }
            }
            _ => {}
        }
    }
    let start = start?;
    let words = tokens[start + 1..close]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let mut names = Vec::new();
    let mut group: Vec<&str> = Vec::new();
    let mut nested = 0usize;
    let finish = |group: &[&str]| -> String {
        if !group.contains(&"(") {
            return group
                .last()
                .filter(|word| is_identifier(word))
                .map_or_else(String::new, |word| (*word).to_owned());
        }
        // `RET ( * NAME ) ( … )`: exactly one starred name in the first group.
        match group.iter().position(|word| *word == "(") {
            Some(at)
                if group.get(at + 1) == Some(&"*")
                    && group.get(at + 2).is_some_and(|word| is_identifier(word))
                    && group.get(at + 3) == Some(&")")
                    && group.get(at + 4) == Some(&"(") =>
            {
                group[at + 2].to_owned()
            }
            _ => String::new(),
        }
    };
    for word in words {
        match word {
            "(" | "[" => nested += 1,
            ")" | "]" => nested = nested.saturating_sub(1),
            "," if nested == 0 => {
                names.push(finish(&group));
                group.clear();
                continue;
            }
            _ => {}
        }
        group.push(word);
    }
    if !group.is_empty() {
        names.push(finish(&group));
    }
    Some(names)
}

/// Whether the formal `name` is rebound or has its address taken in `body`.
pub(super) fn rebound(body: &[&str], name: &str) -> bool {
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
                    "=" | "+="
                        | "-="
                        | "*="
                        | "/="
                        | "%="
                        | "&="
                        | "|="
                        | "^="
                        | "<<="
                        | ">>="
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
    let Some(name) = parameters.get(callback).filter(|name| !name.is_empty()) else {
        // A callback formal whose name the reader does not read is never
        // validated vacuously.
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
                CallbackActual::Parameter { index } => {
                    parameters.get(*index).is_some_and(|formal| {
                        !formal.is_empty()
                            && argument == [formal.as_str()]
                            && !rebound(body, formal)
                    })
                }
                CallbackActual::Supplied | CallbackActual::SuppliedOwned => {
                    !argument.contains(&name.as_str())
                }
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
fn registration_validates(
    body: &[&str],
    parameters: &[String],
    callback: usize,
    holder: usize,
) -> bool {
    let (Some(name), Some(holder)) = (
        parameters.get(callback).filter(|name| !name.is_empty()),
        parameters.get(holder).filter(|name| !name.is_empty()),
    ) else {
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
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the unit's member
    // facts, the run's closure evidence, whenever a member invocation is in
    // the environment.
    if contracts.retaining().values().any(|declared| {
        declared
            .callbacks
            .iter()
            .any(|clause| matches!(clause, CallbackClause::MemberInvoked { .. }))
    }) && let Some(facts) =
        crate::typed_access_path::with_indirect_call_unit(|unit| unit.member_facts().clone())
    {
        crate::contract_dependencies::member_evidence(facts);
    }
    // Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): the unit's
    // classified stores into the stated members, the run's stability
    // evidence, whenever a result state is in the environment.
    let stated = crate::constructed_state::stated_keys(
        contracts
            .retaining()
            .values()
            .map(|declared| &declared.constructed),
    );
    if !stated.is_empty()
        && let Some(facts) = crate::typed_access_path::with_indirect_call_unit(|unit| {
            unit.state_facts(&stated).as_ref().clone()
        })
    {
        crate::contract_dependencies::state_evidence(facts);
    }
    // Card OP-CONSTRUCTOR-DECLARATIONS (rule 14e): a stated `absent` member
    // stands in a unit that defines the constructor only when its Clang body
    // validates the stated result state; otherwise the state is withdrawn
    // there (the result stays tracked, its state undecided).
    let mut constructed_withdrawn = Vec::new();
    for (name, declaration) in contracts.retaining() {
        if !declaration.constructed.states_absent()
            || declaration.constructed_withdrawn
            || super::slot_move_route::defined_body(tokens, name).is_none()
        {
            continue;
        }
        let checked = crate::typed_access_path::with_indirect_call_unit(|unit| {
            unit.function(name).map(|function| {
                crate::constructed_state::result_state_validates(
                    function,
                    &declaration.constructed,
                    unit.member_facts().enumerators(),
                )
            })
        })
        .flatten()
        .unwrap_or_else(|| Err("the body's Clang AST is not read".to_owned()));
        if checked.is_ok() {
            crate::contract_dependencies::validated(name, "14e");
        } else {
            constructed_withdrawn.push(name.clone());
        }
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12o): arms stand in a
    // unit that defines the body only when its Clang body validates them.
    let mut arms_withdrawn = Vec::new();
    for (name, declaration) in contracts.retaining() {
        let Some(discriminant) = &declaration.discriminant else {
            continue;
        };
        if declaration.arms_withdrawn
            || super::slot_move_route::defined_body(tokens, name).is_none()
        {
            continue;
        }
        let checked = crate::typed_access_path::with_indirect_call_unit(|unit| {
            unit.function(name).map(|function| {
                crate::arm_release_check::validates(
                    function,
                    discriminant,
                    &declaration.arms,
                    unit.member_facts().enumerators(),
                )
            })
        })
        .flatten()
        .unwrap_or_else(|| Err("the body's Clang AST is not read".to_owned()));
        if checked.is_ok() {
            crate::contract_dependencies::validated(name, "12o");
        } else {
            arms_withdrawn.push((name.clone(), declaration.clone()));
        }
    }
    // Card SERVE-FORWARDED-CALLBACK-FORMALS (rule 15w): a synchronous
    // invocation also stands on a body whose every mention of the callback
    // formal is an exact forward to a callee whose declaration is in force
    // here. A callee whose body this unit defines is in force only once its
    // own clauses stand, so the standing set is the least fixed point: a
    // declaration whose only support would be itself (a recursive forward)
    // never stands, and nothing stands on a withdrawn callee.
    struct Checked<'a> {
        name: &'a str,
        declaration: &'a RetainingContract,
        clauses: Vec<&'a CallbackClause>,
        body: Vec<&'a str>,
        parameters: Vec<String>,
    }
    let mut defined = BTreeSet::new();
    let mut pending = Vec::new();
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
        let Some((_, body)) = super::slot_move_route::defined_body(tokens, name) else {
            continue;
        };
        defined.insert(name.as_str());
        let Some(parameters) = parameter_names(tokens, name) else {
            continue;
        };
        pending.push(Checked {
            name,
            declaration,
            clauses: checked,
            body,
            parameters,
        });
    }
    // One clause, rule 13y's own check (reading no other declaration), or,
    // where `forward`, for a synchronous invocation, rule 15w's forward over
    // `in_force`.
    fn clause_stands<'c>(
        entry: &Checked<'_>,
        clause: &CallbackClause,
        forward: bool,
        in_force: &dyn Fn(&str) -> Option<&'c RetainingContract>,
    ) -> bool {
        match clause {
            CallbackClause::Invoked { callback, actuals } => {
                invocation_validates(&entry.body, &entry.parameters, *callback, actuals)
                    || forward
                        && super::callback_forward::forward_validates(
                            &entry.body,
                            &entry.parameters,
                            *callback,
                            actuals,
                            in_force,
                        )
            }
            CallbackClause::Registered {
                callback,
                holder: CallbackHolder::Parameter { index },
            } => registration_validates(&entry.body, &entry.parameters, *callback, *index),
            _ => false,
        }
    }
    let mut standing = BTreeSet::new();
    loop {
        let mut grew = false;
        for entry in &pending {
            if standing.contains(entry.name) {
                continue;
            }
            let in_force = |callee: &str| {
                if defined.contains(callee) && !standing.contains(callee) {
                    return None;
                }
                contracts.retaining_contract(callee)
            };
            if entry
                .clauses
                .iter()
                .all(|clause| clause_stands(entry, clause, true, &in_force))
            {
                standing.insert(entry.name);
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let mut withdrawn = Vec::new();
    for entry in &pending {
        if !standing.contains(entry.name) {
            withdrawn.push((entry.name.to_owned(), entry.declaration.clone()));
        } else if entry
            .clauses
            .iter()
            .all(|clause| clause_stands(entry, clause, false, &|_| None))
        {
            crate::contract_dependencies::validated(entry.name, "13y");
        }
        // A clause standing by a forward rests on the callee's declaration:
        // no body evidence is recorded for it, so the declaration stays a
        // declared assumption (rule 13i).
    }
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (rule 12n, USER DECISION
    // 2026-10-16, "Result + list"): a registration forwarded through an id
    // stands in a unit that defines the setter only where its body (or the
    // one helper it forwards to) creates one record, links it into the list,
    // stores the callback into the member and returns the record's id;
    // otherwise the declaration is withdrawn there.
    for (name, declaration) in contracts.retaining() {
        if declaration.callbacks_withdrawn
            || withdrawn.iter().any(|(withdrawn, _)| withdrawn == name)
        {
            continue;
        }
        let forwarded = declaration
            .callbacks
            .iter()
            .filter_map(|clause| match clause {
                CallbackClause::MemberRegistered {
                    object,
                    member,
                    callback,
                    by_id: Some(link),
                } => Some((*object, member, *callback, &link.list)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if forwarded.is_empty() {
            continue;
        }
        let defined = |function: &str| -> Option<crate::registration_by_id::Defined> {
            let (_, body) = super::slot_move_route::defined_body(tokens, function)?;
            let parameters = parameter_names(tokens, function)?;
            Some((
                parameters,
                body.iter().map(|word| (*word).to_owned()).collect(),
            ))
        };
        let Some((parameters, body)) = defined(name) else {
            continue;
        };
        let allocates = |callee: &str| {
            contracts.functions().get(callee).is_some_and(|contract| {
                matches!(
                    contract.result,
                    qcore_schemas::contract::ResourceReturnMode::Owned { .. }
                )
            })
        };
        let links = |callee: &str| {
            contracts.functions().get(callee).is_some_and(|contract| {
                contract.container_effects.get(&0)
                    == Some(&qcore_schemas::contract::ContainerEffect::Published {
                        container_parameter: 1,
                    })
            })
        };
        let unit = crate::registration_by_id::Unit {
            defined: &defined,
            allocates: &allocates,
            links: &links,
        };
        let stands = forwarded.iter().all(|(object, member, callback, list)| {
            crate::registration_by_id::validates(
                &body,
                &parameters,
                *callback,
                *object,
                list,
                member,
                &unit,
            )
            .is_ok()
        });
        if stands {
            crate::contract_dependencies::validated(name, "12n");
        } else {
            withdrawn.push((name.clone(), declaration.clone()));
        }
    }
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE (rule 30): member-path states
    // stand in a unit that defines the body unless its Clang body contradicts
    // them (withdrawn there); validated where the body itself establishes
    // every stated reading, a declared assumption otherwise.
    let mut member_paths_withdrawn = Vec::new();
    for (name, declaration) in contracts.retaining() {
        if declaration.member_paths.is_empty()
            || declaration.member_paths_withdrawn
            || super::slot_move_route::defined_body(tokens, name).is_none()
        {
            continue;
        }
        // Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: the reading follows
        // the unit's static helpers (`unit.function`).
        let checked = crate::typed_access_path::with_indirect_call_unit(|unit| {
            unit.function(name).map(|function| {
                crate::member_path_check::validates(
                    function,
                    &declaration.member_paths,
                    &|node| unit.member_facts().record_of_node(node, true),
                    &|callee| unit.function(callee),
                )
            })
        })
        .flatten();
        match checked {
            Some(Ok(crate::member_path_check::Standing::Validated)) => {
                crate::contract_dependencies::validated(name, "30");
            }
            Some(Err(_)) => member_paths_withdrawn.push(name.clone()),
            // A body the check does not read, or whose reading leaves a
            // stated path to a call it does not follow: a declared assumption.
            Some(Ok(crate::member_path_check::Standing::Assumption(_))) | None => {}
        }
    }
    if withdrawn.is_empty()
        && arms_withdrawn.is_empty()
        && constructed_withdrawn.is_empty()
        && member_paths_withdrawn.is_empty()
    {
        return None;
    }
    let mut settled = contracts.clone();
    for (name, mut declaration) in withdrawn {
        crate::contract_dependencies::withdrawn(&name, "13y", true);
        declaration.callbacks_withdrawn = true;
        settled.insert_retaining(name, declaration);
    }
    for (name, _) in arms_withdrawn {
        crate::contract_dependencies::withdrawn(&name, "12o", true);
        let mut declaration = settled
            .retaining_contract(&name)
            .cloned()
            .expect("a retaining declaration");
        declaration.arms_withdrawn = true;
        settled.insert_retaining(name, declaration);
    }
    for name in constructed_withdrawn {
        crate::contract_dependencies::withdrawn(&name, "14e", true);
        let mut declaration = settled
            .retaining_contract(&name)
            .cloned()
            .expect("a retaining declaration");
        declaration.constructed_withdrawn = true;
        settled.insert_retaining(name, declaration);
    }
    for name in member_paths_withdrawn {
        crate::contract_dependencies::withdrawn(&name, "30", true);
        let mut declaration = settled
            .retaining_contract(&name)
            .cloned()
            .expect("a retaining declaration");
        declaration.member_paths_withdrawn = true;
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
///
/// Card RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN (rule 12n, amended): a
/// body that calls a declaration with a member invocation also gets an edge to
/// every in-unit function of that member's closed set in this unit (the set
/// [`Parser::member_candidates`] decides over; an open set adds none, since
/// the call refuses in every run). Those candidates are stores elsewhere in
/// the unit, never named in the body, so without the edge the caller's
/// summary could be derived before theirs, read them as uncontracted there,
/// and leave a formal it releases through the call unbound (`UnknownAlias` in
/// the reported run, which decides the member exactly as for a local).
pub(crate) fn callback_edges(
    tokens: &[super::Token],
    function: &str,
    candidates: &BTreeSet<String>,
    contracts: &ResourceContracts,
) -> BTreeSet<String> {
    let mut edges = BTreeSet::new();
    // Card OP-CONSTRUCTOR-DECLARATIONS (rule 13w): a constructor registering
    // a callback in its result's member is a callback declaration here too.
    let registers = |declared: &RetainingContract| {
        declared.constructed.members.iter().any(|state| {
            matches!(
                state.value,
                qcore_schemas::constructed_state::MemberValue::Callback { .. }
            )
        })
    };
    if contracts
        .retaining()
        .values()
        .all(|declared| declared.callbacks.is_empty() && !registers(declared))
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
    let mut invokes = false;
    let mut members = BTreeSet::new();
    for (at, token) in body.iter().enumerate() {
        if body.get(at + 1).is_none_or(|next| next.text != "(") {
            continue;
        }
        let Some(declared) = contracts.retaining_contract(&token.text) else {
            continue;
        };
        // Card OP-CONSTRUCTOR-DECLARATIONS (rule 13w): a per-path registered
        // candidate is read at a later member invocation in this body, so it
        // precedes its caller as a synchronous candidate does.
        if registers(declared) {
            invokes = true;
        }
        for clause in &declared.callbacks {
            match clause {
                CallbackClause::Invoked { .. } | CallbackClause::Table { .. } => invokes = true,
                CallbackClause::MemberInvoked { member, .. } => {
                    members.insert(member.spelling());
                }
                _ => {}
            }
        }
    }
    if invokes {
        for (at, token) in body.iter().enumerate() {
            if candidates.contains(&token.text)
                && token.text != function
                && body.get(at + 1).is_none_or(|next| next.text != "(")
            {
                edges.insert(token.text.clone());
            }
        }
    }
    if !members.is_empty() {
        let closed = crate::typed_access_path::with_indirect_call_unit(|unit| {
            members
                .iter()
                .filter_map(|member| {
                    crate::callback_member_closure::closure(&[unit.member_facts()], member).ok()
                })
                .flatten()
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
        edges.extend(
            closed
                .into_iter()
                .filter(|name| candidates.contains(name) && name != function),
        );
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

    fn stream(text: &str) -> Vec<super::super::Token> {
        text.split_whitespace()
            .map(|word| super::super::Token {
                text: word.to_owned(),
                line: 1,
            })
            .collect()
    }

    #[test]
    fn an_inline_function_pointer_parameter_is_named() {
        // `rd_list_init`'s definition (`rdlist.c:53-62`).
        let tokens = stream(
            "rd_list_t * rd_list_init ( rd_list_t * rl , int initial_size , void ( * free_cb ) \
             ( void * ) ) { memset ( rl , 0 , sizeof ( * rl ) ) ; if ( initial_size > 0 ) \
             rd_list_grow ( rl , initial_size ) ; rl -> rl_free_cb = free_cb ; return rl ; }",
        );
        let names = parameter_names(&tokens, "rd_list_init").expect("a definition");
        assert_eq!(names, ["rl", "initial_size", "free_cb"]);
        let (_, body) =
            super::super::slot_move_route::defined_body(&tokens, "rd_list_init").expect("a body");
        assert!(registration_validates(&body, &names, 2, 0));
        // A declarator the reader does not name never validates vacuously.
        let odd = stream("int f ( int ( * ( * g ) ) ( void ) ) { return 0 ; }");
        let names = parameter_names(&odd, "f").expect("a definition");
        assert_eq!(names, [""]);
        assert!(!invocation_validates(
            &words("{ return 0 ; }"),
            &names,
            0,
            &[]
        ));
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
