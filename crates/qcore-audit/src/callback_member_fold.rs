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

//! Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (gateway rule 12n; contract
//! spec §C04.19.3): the run's closure of every function-pointer member a
//! member invocation names, folded over every unit after the units are
//! analysed and before anything is published.
//!
//! A unit decides a member invocation over the stores it sees itself
//! ([`qcore_c_gateway::callback_member_closure`]). The fold reads every
//! analysed unit's facts and computes the run's closure; a `Safe` or `Pending`
//! row whose dependencies name a declaration that invokes a member is kept
//! only when the run's closure of that member is closed and contained in the
//! unit's own (so every candidate the run can store was one the unit decided);
//! otherwise it is demoted to `Unknown` with the reason
//! `callback-member-open`. A unit that recorded no facts opens every member
//! (premise `P-CG22-MEMBER-CLOSED-WORLD`: the analysed units are every unit
//! that can store into it). Card CALLBACK-MEMBER-DATA-ONLY-UNITS: a data-only
//! unit records the facts read from its AST ([`data_only_facts`]), so only a
//! unit whose AST is not read opens a member. The fold never promotes a row.
//!
//! Card **OP-CONSTRUCTOR-DECLARATIONS** (rules 12o, 13w, 14e): a unit reads
//! a constructor's stated result state only where its own stores keep it; the
//! fold reads every analysed unit's classified stores
//! ([`qcore_c_gateway::constructed_state::stability`]) and demotes a `Safe`
//! or `Pending` row whose dependencies name a constructor stating a state on
//! a member the run's stores do not keep (`constructed-state-open`); a unit
//! that recorded no facts opens every stated member (premise
//! `P-OPCTOR-STORES-SEEN`).

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::callback_member_closure::{Closure, MemberFacts, closure};
use qcore_c_gateway::constructed_state::{StateFacts, stability, stated_keys};
use qcore_schemas::callback::CallbackClause;
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};

/// The sidecar key of the fold's section.
pub const SECTION_KEY: &str = "callback_members";

/// The assumptions key naming the member-invoking declarations, recorded
/// before any unit (a checkpoint composer refuses a run that has any unless
/// its checkpoints carry the run's facts, card MEMBER-PREPASS-OTHER-ROUTES).
pub const DECLARATIONS_KEY: &str = "callback_member_declarations";

/// What the section means, carried in it.
pub const MEANING: &str = "the run's closure of every function-pointer member a resource_callback_member_invoked clause names (rule 12n): over every analysed unit's Clang stores into the member, a designator joins the set, a null pointer adds nothing, a read of another such member adds its set, a formal of a function whose address no unit takes adds its actuals at every call, and anything else opens it; a unit that recorded no facts opens it (premise P-CG22-MEMBER-CLOSED-WORLD). `demoted` lists the Safe or Pending rows whose unit decided a member over a set the run's closure is not contained in, now Unknown (callback-member-open). `members_over_recorded_units` is the same closure over the units that did record facts, and `units_without_facts_named` names the others (evidence only). Evidence and a demotion only: no row is promoted";

/// What the result-state part of the section means (card
/// OP-CONSTRUCTOR-DECLARATIONS).
pub const CONSTRUCTED_MEANING: &str = "the run's stability of every member a constructor's stated result state names (rules 12o, 13w, 14e): every store into the member in every analysed unit is classified (a store into an object its function obtained from an owned-result call only or an automatic object, a null pointer into a function-pointer member, a constant flag-or, a case-guarded rewrite); anything else, a whole-object write of the record, or a unit that recorded no facts (premise P-OPCTOR-STORES-SEEN) leaves it not kept. `demoted` lists the Safe or Pending rows resting on a constructor whose stated member the run does not keep, now Unknown (constructed-state-open). Evidence and a demotion only: no row is promoted";

/// Every declaration of `contracts` that invokes a member, with the members.
#[must_use]
pub fn member_declarations(contracts: &ResourceContracts) -> BTreeMap<String, BTreeSet<String>> {
    contracts
        .retaining()
        .iter()
        .filter_map(|(name, declared)| {
            let members = declared
                .callbacks
                .iter()
                .filter_map(|clause| match clause {
                    CallbackClause::MemberInvoked { member, .. } => Some(member.spelling()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            (!members.is_empty()).then(|| (name.clone(), members))
        })
        .collect()
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (rule 12n; user decisions Q2,
/// D1): every declared registration into a parameter's member (each call a
/// store of its callback actual) and every stated function default of a
/// constructor's result (a store of the function), for the run's closure.
#[must_use]
pub fn declared_stores(
    contracts: &ResourceContracts,
) -> Vec<qcore_c_gateway::member_run_facts::DeclaredStore> {
    use qcore_c_gateway::member_run_facts::DeclaredStore;
    use qcore_schemas::constructed_state::MemberValue;
    let mut out = Vec::new();
    for (name, declared) in contracts.retaining() {
        for clause in &declared.callbacks {
            if let CallbackClause::MemberRegistered {
                member, callback, ..
            } = clause
            {
                out.push(DeclaredStore::Parameter {
                    member: member.spelling(),
                    function: name.clone(),
                    index: *callback,
                });
            }
        }
        for state in &declared.constructed.members {
            if let MemberValue::Function { function } = &state.value {
                out.push(DeclaredStore::Default {
                    member: state.member.spelling(),
                    function: function.clone(),
                });
            }
        }
    }
    out.sort();
    out
}

/// Card **CALLBACK-MEMBER-DATA-ONLY-UNITS** (rule 12n): the facts of a unit
/// the gateway does not analyse, read from its whole Clang AST
/// ([`qcore_c_gateway::callback_member_closure::read_unit`]) over its
/// preprocessed text; `None` (every member open) when it has none (its
/// preprocessing failed) or the AST is not read.
pub(crate) fn data_only_facts(
    preprocessed: Option<&str>,
    unit: &qcore_c_gateway::legacy_c::C11TranslationUnit<'_>,
) -> Option<MemberFacts> {
    qcore_c_gateway::callback_member_closure::read_unit(preprocessed?, unit).ok()
}

/// Card **OP-CONSTRUCTOR-DECLARATIONS**: a data-only unit's classified
/// stores into the members `contracts`' result states name, read from its
/// whole Clang AST; `None` (every stated member open) when it has none or the
/// AST is not read, and when no declaration states a result state.
pub(crate) fn data_only_state(
    preprocessed: Option<&str>,
    unit: &qcore_c_gateway::legacy_c::C11TranslationUnit<'_>,
    contracts: &ResourceContracts,
) -> Option<StateFacts> {
    let keys = stated_keys(
        contracts
            .retaining()
            .values()
            .map(|declared| &declared.constructed),
    );
    if keys.is_empty() {
        return None;
    }
    qcore_c_gateway::constructed_state::read_unit(preprocessed?, unit, &keys).ok()
}

/// Card OP-CONSTRUCTOR-DECLARATIONS: every declaration of `contracts`
/// that states a result state, with the members it states (and whether each
/// is a discriminant).
#[must_use]
pub fn constructed_declarations(
    contracts: &ResourceContracts,
) -> BTreeMap<String, BTreeMap<String, bool>> {
    contracts
        .retaining()
        .iter()
        .filter(|(_, declared)| !declared.constructed.is_empty())
        .map(|(name, declared)| {
            let mut members = stated_keys([&declared.constructed])
                .into_iter()
                .map(|key| (key, false))
                .collect::<BTreeMap<_, _>>();
            if let Some(set) = &declared.constructed.discriminant {
                members.insert(set.member.spelling(), true);
            }
            (name.clone(), members)
        })
        .filter(|(_, members)| !members.is_empty())
        .collect()
}

/// Card OP-CONSTRUCTOR-DECLARATIONS: every `(library, file, function)` the
/// fold's section lists as demoted (rule 12n's and the result states').
#[must_use]
pub fn demoted_functions(section: &Value) -> BTreeSet<(String, String, String)> {
    section["demoted"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            section["constructed_states"]["demoted"]
                .as_array()
                .into_iter()
                .flatten(),
        )
        .map(|entry| {
            let text = |key: &str| entry[key].as_str().unwrap_or("").to_owned();
            (text("library"), text("file"), text("function"))
        })
        .collect()
}

/// Card OP-CONSTRUCTOR-DECLARATIONS: the property rows (null, bounds) of a
/// demoted function read `Unknown` custody, and their combined verdict is
/// recomputed with it. Never promotes: only a `Safe` or `Pending` custody
/// column moves.
pub fn demote_property_rows(
    null: &mut [Value],
    bounds: &mut [Value],
    demoted: &BTreeSet<(String, String, String)>,
) {
    let key = |row: &Value| {
        let text = |key: &str| row[key].as_str().unwrap_or("").to_owned();
        (text("library"), text("file"), text("function"))
    };
    let moves = |row: &Value| {
        demoted.contains(&key(row)) && matches!(row["custody"].as_str(), Some("Safe" | "Pending"))
    };
    for row in null.iter_mut().filter(|row| moves(row)) {
        use qcore_c_gateway::null_deref::{NullVerdict, Request, compose};
        let verdict = match row["null"]["verdict"].as_str() {
            Some("Safe") => Some(NullVerdict::Safe),
            Some("Pending") => Some(NullVerdict::Pending),
            Some("Unsafe") => Some(NullVerdict::Unsafe),
            Some(_) => Some(NullVerdict::Unknown),
            None => None,
        };
        row["custody"] = json!("Unknown");
        let combined = compose(
            Request {
                custody: true,
                null: true,
            },
            Some(NullVerdict::Unknown),
            verdict,
        );
        row["combined"] = json!(combined.as_str());
    }
    for row in bounds.iter_mut().filter(|row| moves(row)) {
        use qcore_c_gateway::index_extent::{BoundsVerdict, Request, compose};
        let verdict = match row["bounds"]["verdict"].as_str() {
            Some("Safe") => Some(BoundsVerdict::Safe),
            Some("Pending") => Some(BoundsVerdict::Pending),
            Some("Unsafe") => Some(BoundsVerdict::Unsafe),
            Some(_) => Some(BoundsVerdict::Unknown),
            None => None,
        };
        row["custody"] = json!("Unknown");
        let combined = compose(
            Request {
                custody: true,
                bounds: true,
            },
            Some(BoundsVerdict::Unknown),
            verdict,
        );
        row["combined"] = json!(combined.as_str());
    }
}

/// One unit's recorded facts (card MEMBER-PREPASS-OTHER-ROUTES: carried by
/// a checkpoint, so serialisable).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnitFacts {
    pub library: String,
    pub file: String,
    pub facts: Option<MemberFacts>,
    /// Card OP-CONSTRUCTOR-DECLARATIONS: the unit's classified stores into
    /// the stated members.
    pub state: Option<StateFacts>,
}

fn closure_value(result: &Result<BTreeSet<String>, BTreeSet<String>>) -> Value {
    match result {
        Ok(names) => json!({"closed": true, "candidates": names}),
        Err(open) => json!({"closed": false, "open": open}),
    }
}

/// Card MEMBER-PREPASS-OTHER-ROUTES: the inverse of [`closure_value`], for a
/// closure a checkpoint carries.
///
/// # Errors
/// A value [`closure_value`] does not write.
pub fn closure_from_value(value: &Value) -> Result<Closure, String> {
    let names = |key: &str| {
        value[key]
            .as_array()
            .ok_or_else(|| format!("a carried closure has no `{key}` list"))?
            .iter()
            .map(|name| {
                name.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "a carried closure names a non-text entry".to_owned())
            })
            .collect::<Result<BTreeSet<_>, _>>()
    };
    match value["closed"].as_bool() {
        Some(true) => Ok(Ok(names("candidates")?)),
        Some(false) => Ok(Err(names("open")?)),
        None => Err("a carried closure has no `closed` flag".into()),
    }
}

/// Card MEMBER-PREPASS-OTHER-ROUTES: everything the fold reads of the
/// contracts, so a checkpoint composer (which loads none) folds as the run
/// does: the member-invoking declarations with their members, the
/// declarations stating a result state with their members, and the
/// functions whose declared result is owned.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoldEnvironment {
    pub declarations: BTreeMap<String, BTreeSet<String>>,
    pub constructed: BTreeMap<String, BTreeMap<String, bool>>,
    pub owned: BTreeSet<String>,
}

impl FoldEnvironment {
    /// The environment of `contracts`.
    #[must_use]
    pub fn of(contracts: &ResourceContracts) -> Self {
        let owned = |result: qcore_schemas::contract::ResourceReturnMode| {
            matches!(
                result,
                qcore_schemas::contract::ResourceReturnMode::Owned { .. }
            )
        };
        Self {
            declarations: member_declarations(contracts),
            constructed: constructed_declarations(contracts),
            owned: contracts
                .functions()
                .iter()
                .filter(|(_, contract)| owned(contract.result))
                .map(|(name, _)| name.clone())
                .chain(
                    contracts
                        .retaining()
                        .iter()
                        .filter(|(_, declared)| owned(declared.contract.result))
                        .map(|(name, _)| name.clone()),
                )
                .collect(),
        }
    }

    /// Whether the fold has nothing to do.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty() && self.constructed.is_empty()
    }

    /// Every invoked member.
    #[must_use]
    pub fn members(&self) -> BTreeSet<String> {
        self.declarations.values().flatten().cloned().collect()
    }

    /// The run's closure of every invoked member, read from the pre-pass:
    /// what the run's decisions and its fold read of the run's facts.
    #[must_use]
    pub fn run_closures(
        &self,
        prepass: &qcore_c_gateway::member_run_facts::RunMemberFacts,
    ) -> BTreeMap<String, Closure> {
        self.members()
            .into_iter()
            .map(|member| {
                let closure = prepass.closure(&member);
                (member, closure)
            })
            .collect()
    }
}

/// The run closures as a checkpoint carries them.
#[must_use]
pub fn closures_value(closures: &BTreeMap<String, Closure>) -> Value {
    json!(
        closures
            .iter()
            .map(|(member, closure)| (member.clone(), closure_value(closure)))
            .collect::<BTreeMap<_, _>>()
    )
}

/// The inverse of [`closures_value`].
///
/// # Errors
/// A value [`closures_value`] does not write.
pub fn closures_from_value(value: &Value) -> Result<BTreeMap<String, Closure>, String> {
    value
        .as_object()
        .ok_or("the carried closures are not an object")?
        .iter()
        .map(|(member, closure)| Ok((member.clone(), closure_from_value(closure)?)))
        .collect()
}

/// Fold the run: demote rows of `units` (the report's unit rows, in the
/// order `facts` was recorded) and return the section, `Value::Null` when no
/// declaration invokes a member.
pub fn fold(
    units: &mut [Value],
    facts: &[UnitFacts],
    contracts: &ResourceContracts,
    diagnostic_counts: &mut BTreeMap<String, usize>,
) -> Value {
    fold_with(units, facts, contracts, diagnostic_counts, None)
}

/// [`fold`], with the run's facts the pre-pass read (card
/// OP-APP-RK-CALLBACKS-VIA-POLL-CB, user decision R1): every unit decided a
/// member over that run's set itself, so the run's closure is the pre-pass's
/// and a row is kept exactly when it is closed. Still demote-only.
pub fn fold_with(
    units: &mut [Value],
    facts: &[UnitFacts],
    contracts: &ResourceContracts,
    diagnostic_counts: &mut BTreeMap<String, usize>,
    prepass: Option<&qcore_c_gateway::member_run_facts::RunMemberFacts>,
) -> Value {
    let environment = FoldEnvironment::of(contracts);
    let run = prepass.map(|prepass| environment.run_closures(prepass));
    fold_in(&environment, units, facts, diagnostic_counts, run.as_ref())
}

/// Card MEMBER-PREPASS-OTHER-ROUTES: [`fold_with`] over what it reads of the
/// contracts ([`FoldEnvironment`]) and of the pre-pass (the run's closure of
/// each invoked member), so a checkpoint composer folds exactly as the run
/// does from what the checkpoints carry. A member the carried closures do not
/// name reads open.
pub fn fold_in(
    environment: &FoldEnvironment,
    units: &mut [Value],
    facts: &[UnitFacts],
    diagnostic_counts: &mut BTreeMap<String, usize>,
    run_closures: Option<&BTreeMap<String, Closure>>,
) -> Value {
    let declarations = &environment.declarations;
    let constructed = &environment.constructed;
    if declarations.is_empty() && constructed.is_empty() {
        return Value::Null;
    }
    let carried = |member: &str| -> Closure {
        run_closures
            .and_then(|closures| closures.get(member).cloned())
            .unwrap_or_else(|| {
                Err(BTreeSet::from([
                    "the run's facts carry no closure of it".to_owned()
                ]))
            })
    };
    let members = declarations
        .values()
        .flatten()
        .cloned()
        .collect::<BTreeSet<_>>();
    let recorded = facts
        .iter()
        .filter_map(|unit| unit.facts.as_ref())
        .collect::<Vec<_>>();
    // Pair each unit row with the facts its projection recorded, in order; a
    // row with none (a data-only unit, a unit the gateway did not analyse)
    // recorded no facts.
    let mut paired: Vec<Option<&UnitFacts>> = Vec::with_capacity(units.len());
    let mut next = 0usize;
    for row in units.iter() {
        let unit = facts.get(next).filter(|unit| {
            row["library"].as_str() == Some(unit.library.as_str())
                && row["file"].as_str() == Some(unit.file.as_str())
        });
        if unit.is_some() {
            next += 1;
        }
        paired.push(unit);
    }
    let missing = paired
        .iter()
        .filter(|unit| unit.is_none_or(|unit| unit.facts.is_none()))
        .count()
        + facts.len().saturating_sub(next);
    // Card OP-CONSTRUCTOR-DECLARATIONS: the run's stability of every stated
    // member, over every unit row (a row with no recorded state facts opens
    // it).
    let owned = |name: &str| environment.owned.contains(name);
    let state_units = paired
        .iter()
        .map(|unit| unit.and_then(|unit| unit.state.as_ref()))
        .collect::<Vec<_>>();
    let mut state_stability: BTreeMap<String, Result<(), BTreeSet<String>>> = BTreeMap::new();
    for members in constructed.values() {
        for (member, discriminant) in members {
            state_stability.entry(member.clone()).or_insert_with(|| {
                stability(&state_units, member, *discriminant, &owned).map(|_| ())
            });
        }
    }
    // The units with no facts, named for the section.
    let without = units
        .iter()
        .zip(&paired)
        .filter(|(_, unit)| unit.is_none_or(|unit| unit.facts.is_none()))
        .map(|(row, _)| {
            format!(
                "{}/{}",
                row["library"].as_str().unwrap_or("?"),
                row["file"].as_str().unwrap_or("?")
            )
        })
        .collect::<Vec<_>>();
    // Evidence only: the closure over the units that did record facts.
    let recorded_only = members
        .iter()
        .map(|member| (member.clone(), closure_value(&closure(&recorded, member))))
        .collect::<BTreeMap<_, _>>();
    let run = members
        .iter()
        .map(|member| {
            // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (R1): the pre-pass's set,
            // which every unit decided over.
            if run_closures.is_some() {
                return (member.clone(), carried(member));
            }
            let mut result = closure(&recorded, member);
            if missing > 0 {
                let reason = format!("{missing} analysed unit(s) recorded no facts");
                result = Err(match result {
                    Ok(_) => BTreeSet::from([reason]),
                    Err(mut open) => {
                        open.insert(reason);
                        open
                    }
                });
            }
            (member.clone(), result)
        })
        .collect::<BTreeMap<_, _>>();
    let mut demoted = Vec::new();
    let mut state_demoted = Vec::new();
    for (row, unit) in units.iter_mut().zip(paired.iter().copied()) {
        let own_facts = unit.and_then(|unit| unit.facts.as_ref());
        let (library, file) = (
            row["library"].as_str().unwrap_or("").to_owned(),
            row["file"].as_str().unwrap_or("").to_owned(),
        );
        let mut demoted_here = 0usize;
        for function in row["functions"].as_array_mut().into_iter().flatten() {
            if !matches!(function["status"].as_str(), Some("Safe" | "Pending")) {
                continue;
            }
            let relied = function["contract_dependencies"]["declarations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|name| {
                    declarations
                        .get(name)
                        .map(|members| (name.to_owned(), members))
                })
                .collect::<Vec<_>>();
            // Card OP-CONSTRUCTOR-DECLARATIONS: a row resting on a
            // constructor's stated state keeps it only when the run's stores
            // keep every member it states.
            let opened = function["contract_dependencies"]["declarations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|name| {
                    constructed
                        .get(name)
                        .map(|members| (name.to_owned(), members))
                })
                .flat_map(|(name, members)| {
                    members
                        .keys()
                        .filter(|member| state_stability.get(*member).is_some_and(Result::is_err))
                        .map(move |member| (name.clone(), member.clone()))
                })
                .collect::<Vec<_>>();
            if let Some((declaration, member)) = opened.first()
                && function["status"].as_str() != Some("Unknown")
            {
                function["status"] = json!("Unknown");
                let description = format!(
                    "call to `{declaration}`: the run's stores into `{member}` do not keep the \
                     stated result state (constructed-state-open)"
                );
                if let Some(diagnostics) = function["diagnostics"].as_array_mut() {
                    diagnostics.push(json!({
                        "line": 0,
                        "origin": Value::Null,
                        "category": "unsupported",
                        "description": description,
                    }));
                }
                *diagnostic_counts.entry("unsupported".into()).or_default() += 1;
                demoted_here += 1;
                state_demoted.push(json!({
                    "library": library,
                    "file": file,
                    "function": function["function"],
                    "declaration": declaration,
                    "member": member,
                }));
            }
            for (declaration, members) in relied {
                for member in members {
                    // With the pre-pass, the unit decided over the run's set
                    // itself: kept exactly when that set is closed.
                    let own = match run_closures {
                        Some(_) => Some(carried(member)),
                        None => own_facts.map(|facts| closure(&[facts], member)),
                    };
                    let kept = matches!(
                        (&own, &run[member]),
                        (Some(Ok(own)), Ok(all)) if all.is_subset(own)
                    );
                    if kept || function["status"].as_str() == Some("Unknown") {
                        continue;
                    }
                    function["status"] = json!("Unknown");
                    let description = format!(
                        "call to `{declaration}`: the run's stores into `{member}` are not \
                         the closed set this unit decided (callback-member-open)"
                    );
                    if let Some(diagnostics) = function["diagnostics"].as_array_mut() {
                        diagnostics.push(json!({
                            "line": 0,
                            "origin": Value::Null,
                            "category": "unsupported",
                            "description": description,
                        }));
                    }
                    *diagnostic_counts.entry("unsupported".into()).or_default() += 1;
                    demoted_here += 1;
                    demoted.push(json!({
                        "library": library,
                        "file": file,
                        "function": function["function"],
                        "declaration": declaration,
                        "member": member,
                    }));
                }
            }
        }
        if demoted_here > 0 {
            let count = row["diagnostic_counts"]["unsupported"]
                .as_u64()
                .unwrap_or(0);
            row["diagnostic_counts"]["unsupported"] = json!(count + demoted_here as u64);
            if matches!(row["status"].as_str(), Some("Safe" | "Pending")) {
                row["status"] = json!("Unknown");
                row["reason"] = json!(
                    "a member invocation's closed set is not the run's (callback-member-open)"
                );
                row["reason_code"] = crate::report::reason_code(row["reason"].as_str());
            }
        }
    }
    let mut section = json!({
        "meaning": MEANING,
        "declarations": declarations,
        "members": run.iter().map(|(member, result)| (member.clone(), closure_value(result))).collect::<BTreeMap<_, _>>(),
        "units_without_facts": missing,
        "units_without_facts_named": without,
        "members_over_recorded_units": recorded_only,
        "demoted": demoted,
    });
    // Card OP-CONSTRUCTOR-DECLARATIONS: present only when a declaration
    // states a result state.
    if !constructed.is_empty() {
        section["constructed_states"] = json!({
            "meaning": CONSTRUCTED_MEANING,
            "declarations": constructed,
            "members": state_stability
                .iter()
                .map(|(member, result)| {
                    let value = match result {
                        Ok(()) => json!({"kept": true}),
                        Err(open) => json!({"kept": false, "open": open}),
                    };
                    (member.clone(), value)
                })
                .collect::<BTreeMap<_, _>>(),
            "demoted": state_demoted,
        });
    }
    section
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contracts() -> ResourceContracts {
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "fold.qv".to_owned(),
            "module contract;
extern fn destroy(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nonnull);
    ensures normal resource_callback_member_invoked(o, op.cb, _, o);
    ensures normal resource_return_none();
};
"
            .to_owned(),
        )]))
        .expect("loads")
    }

    fn facts(names: &[&str]) -> MemberFacts {
        let mut facts = MemberFacts::default();
        facts.stores.insert(
            "op.cb".to_owned(),
            names
                .iter()
                .map(
                    |name| qcore_c_gateway::callback_member_closure::Flow::Function {
                        name: (*name).to_owned(),
                    },
                )
                .collect(),
        );
        facts
    }

    fn row(file: &str, status: &str, declarations: &[&str]) -> Value {
        json!({"library": "l", "file": file, "status": status, "reason": null,
               "diagnostic_counts": {},
               "functions": [{"function": "f", "status": status, "diagnostics": [],
                              "contract_dependencies": {"declarations": declarations}}]})
    }

    fn unit(file: &str, facts: Option<MemberFacts>) -> UnitFacts {
        UnitFacts {
            library: "l".into(),
            file: file.into(),
            facts,
            state: None,
        }
    }

    /// Card OP-CONSTRUCTOR-DECLARATIONS: a demoted function's property rows
    /// read `Unknown` custody, and their combined verdict follows; other rows
    /// and an `Unknown` custody column are untouched.
    #[test]
    fn a_demoted_function_demotes_its_property_rows() {
        let demoted = BTreeSet::from([("l".to_owned(), "a.c".to_owned(), "f".to_owned())]);
        let mut null = vec![
            json!({"library": "l", "file": "a.c", "function": "f", "custody": "Safe",
                   "null": {"verdict": "Safe"}, "combined": "Safe"}),
            json!({"library": "l", "file": "a.c", "function": "g", "custody": "Safe",
                   "null": {"verdict": "Safe"}, "combined": "Safe"}),
        ];
        let mut bounds = vec![
            json!({"library": "l", "file": "a.c", "function": "f", "custody": "Pending",
                   "bounds": {"verdict": "Safe"}, "combined": "Pending"}),
        ];
        demote_property_rows(&mut null, &mut bounds, &demoted);
        assert_eq!(null[0]["custody"], "Unknown");
        assert_eq!(null[0]["combined"], "Unknown");
        assert_eq!(null[1]["combined"], "Safe");
        assert_eq!(bounds[0]["custody"], "Unknown");
        assert_eq!(bounds[0]["combined"], "Unknown");
        let section = json!({"demoted": [{"library": "l", "file": "a.c", "function": "f"}],
                             "constructed_states": {"demoted": [
                                 {"library": "l", "file": "b.c", "function": "h"}]}});
        assert_eq!(demoted_functions(&section).len(), 2);
    }

    #[test]
    fn a_row_is_kept_only_when_the_run_set_is_the_units() {
        let mut counts = BTreeMap::new();
        // Unit a decided over {h}; the run adds unit b's {g}: demoted.
        let mut units = vec![row("a.c", "Safe", &["destroy"]), row("b.c", "Unknown", &[])];
        let section = fold(
            &mut units,
            &[
                unit("a.c", Some(facts(&["h"]))),
                unit("b.c", Some(facts(&["g"]))),
            ],
            &contracts(),
            &mut counts,
        );
        assert_eq!(units[0]["status"], "Unknown");
        assert_eq!(units[0]["functions"][0]["status"], "Unknown");
        assert_eq!(section["demoted"].as_array().map(Vec::len), Some(1));
        assert_eq!(counts["unsupported"], 1);
        // The run's set is the unit's own: kept.
        let mut units = vec![row("a.c", "Safe", &["destroy"]), row("b.c", "Unknown", &[])];
        fold(
            &mut units,
            &[
                unit("a.c", Some(facts(&["h"]))),
                unit("b.c", Some(facts(&["h"]))),
            ],
            &contracts(),
            &mut BTreeMap::new(),
        );
        assert_eq!(units[0]["status"], "Safe");
        // A unit with no facts opens every member: demoted.
        let mut units = vec![row("a.c", "Safe", &["destroy"]), row("b.c", "Unknown", &[])];
        fold(
            &mut units,
            &[unit("a.c", Some(facts(&["h"]))), unit("b.c", None)],
            &contracts(),
            &mut BTreeMap::new(),
        );
        assert_eq!(units[0]["status"], "Unknown");
        // A row resting on no member-invoking declaration is never touched.
        let mut units = vec![row("a.c", "Safe", &["other"])];
        fold(
            &mut units,
            &[unit("a.c", None)],
            &contracts(),
            &mut BTreeMap::new(),
        );
        assert_eq!(units[0]["status"], "Safe");
    }

    /// `RUN_VECTORS` (`QCore.OpCallbacks.runVectors`): each unit's reading of
    /// the member (`None` unread, `Some(None)` open, `Some(Some(names))` read
    /// with those stores, `Some(Some(&[]))` read with none) and the run's
    /// closure (`None` open).
    #[allow(clippy::type_complexity)]
    const RUN_VECTORS: &[(&[Option<Option<&[&str]>>], Option<&[&str]>)] = &[
        (&[], Some(&[])),
        (&[Some(Some(&["a"])), Some(Some(&[]))], Some(&["a"])),
        (&[Some(Some(&["a"])), Some(Some(&["b"]))], Some(&["a", "b"])),
        (&[Some(Some(&["a"])), None], None),
        (&[Some(Some(&[])), None, Some(Some(&["b"]))], None),
        (&[Some(Some(&["a"])), Some(None)], None),
    ];

    #[test]
    fn run_vectors_hold() {
        for (readings, expected) in RUN_VECTORS {
            let mut rows = Vec::new();
            let mut recorded = Vec::new();
            for (index, reading) in readings.iter().enumerate() {
                let file = format!("u{index}.c");
                rows.push(row(&file, "Unknown", &[]));
                let facts = reading.map(|reading| match reading {
                    Some(names) => facts(names),
                    None => {
                        let mut open = MemberFacts::default();
                        open.stores.insert(
                            "op.cb".to_owned(),
                            BTreeSet::from([
                                qcore_c_gateway::callback_member_closure::Flow::Open {
                                    why: "open".to_owned(),
                                },
                            ]),
                        );
                        open
                    }
                });
                recorded.push(unit(&file, facts));
            }
            let section = fold(&mut rows, &recorded, &contracts(), &mut BTreeMap::new());
            let member = &section["members"]["op.cb"];
            let got = (member["closed"] == true).then(|| {
                member["candidates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|name| name.as_str().unwrap().to_owned())
                    .collect::<Vec<_>>()
            });
            let expected =
                expected.map(|names| names.iter().map(|name| (*name).to_owned()).collect());
            assert_eq!(got, expected, "{readings:?}");
        }
        assert_eq!(RUN_VECTORS.len(), 6);
    }

    fn clang_available() -> bool {
        match qcore_c_gateway::clang_command() {
            Ok(_) => true,
            Err(reason) => {
                assert!(
                    std::env::var_os("QCORE_CLANG").is_none(),
                    "explicit QCORE_CLANG configuration is invalid: {reason}"
                );
                eprintln!("skipping Clang-dependent data-only member test: {reason}");
                false
            }
        }
    }

    /// Card CALLBACK-MEMBER-DATA-ONLY-UNITS: project `units` (file, source)
    /// as the audit does and fold the run; the `op.cb` closure.
    fn project_and_fold(units: &[(&str, &str)]) -> Value {
        use qcore_c_gateway::legacy_c::{C11TranslationUnit, enumerate_source_definitions};
        let contracts = contracts();
        let mut report = crate::report::AuditReport::new("test");
        let mut counts = crate::projection::AuditCounts::default();
        for (file, source) in units {
            let unit = C11TranslationUnit {
                source,
                ..Default::default()
            };
            let inventory = enumerate_source_definitions(&unit);
            let mut out = String::new();
            crate::projection::project_unit_into(
                "l",
                file,
                &unit,
                &inventory,
                &contracts,
                &mut report,
                &mut counts,
                &mut out,
            )
            .expect("projects");
        }
        let (section, _, _) = report.fold_callback_members(&contracts);
        section
    }

    const RECORD: &str = "struct op { int kind; int (*cb)(void *); };\n";

    #[test]
    fn a_data_only_unit_is_read_from_its_ast() {
        if !clang_available() {
            return;
        }
        let analysed = format!(
            "{RECORD}int h(void *);\nvoid destroy(struct op *o);\n\
             void set(struct op *o) {{ o->cb = h; }}\n\
             void drop(struct op *o) {{ destroy(o); }}\n"
        );
        // A data-only unit whose designated initializer stores into the
        // member: its candidate joins the run's closed set.
        let storing = format!("{RECORD}int g(void *);\nconst struct op tmpl = {{ .cb = g }};\n");
        let section = project_and_fold(&[("a.c", &analysed), ("d.c", &storing)]);
        assert_eq!(section["units_without_facts"], 0, "{section}");
        assert_eq!(
            section["members"]["op.cb"],
            json!({"closed": true, "candidates": ["g", "h"]})
        );
        // A data-only unit with no store into the member closes: the run's
        // set is the analysed unit's own.
        let silent = format!("{RECORD}int x = 1;\nconst struct op none = {{ 3 }};\n");
        let section = project_and_fold(&[("a.c", &analysed), ("d.c", &silent)]);
        assert_eq!(
            section["members"]["op.cb"],
            json!({"closed": true, "candidates": ["h"]})
        );
        // An empty unit (the measured `ctr_utils.c`) closes too.
        let section = project_and_fold(&[("a.c", &analysed), ("e.c", "")]);
        assert_eq!(
            section["members"]["op.cb"],
            json!({"closed": true, "candidates": ["h"]})
        );
        // A data-only unit whose AST is not read (preprocessing fails) records
        // no facts and keeps the member open.
        let unreadable = format!("{RECORD}#error unreadable\n");
        let section = project_and_fold(&[("a.c", &analysed), ("f.c", &unreadable)]);
        assert_eq!(section["units_without_facts"], 1, "{section}");
        assert_eq!(section["members"]["op.cb"]["closed"], false);
        assert_eq!(
            section["units_without_facts_named"],
            json!(["l/f.c"]),
            "{section}"
        );
    }
}
