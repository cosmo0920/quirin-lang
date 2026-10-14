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
//! that can store into it). The fold never promotes a row.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::callback_member_closure::{MemberFacts, closure};
use qcore_schemas::callback::CallbackClause;
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};

/// The sidecar key of the fold's section.
pub const SECTION_KEY: &str = "callback_members";

/// The assumptions key naming the member-invoking declarations, recorded
/// before any unit (a checkpoint composer refuses a run that has any).
pub const DECLARATIONS_KEY: &str = "callback_member_declarations";

/// What the section means, carried in it.
pub const MEANING: &str = "the run's closure of every function-pointer member a resource_callback_member_invoked clause names (rule 12n): over every analysed unit's Clang stores into the member, a designator joins the set, a null pointer adds nothing, a read of another such member adds its set, a formal of a function whose address no unit takes adds its actuals at every call, and anything else opens it; a unit that recorded no facts opens it (premise P-CG22-MEMBER-CLOSED-WORLD). `demoted` lists the Safe or Pending rows whose unit decided a member over a set the run's closure is not contained in, now Unknown (callback-member-open). Evidence and a demotion only: no row is promoted";

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

/// One unit's recorded facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitFacts {
    pub library: String,
    pub file: String,
    pub facts: Option<MemberFacts>,
}

fn closure_value(result: &Result<BTreeSet<String>, BTreeSet<String>>) -> Value {
    match result {
        Ok(names) => json!({"closed": true, "candidates": names}),
        Err(open) => json!({"closed": false, "open": open}),
    }
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
    let declarations = member_declarations(contracts);
    if declarations.is_empty() {
        return Value::Null;
    }
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
    let run = members
        .iter()
        .map(|member| {
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
    for (row, unit) in units.iter_mut().zip(paired) {
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
            for (declaration, members) in relied {
                for member in members {
                    let own = own_facts.map(|facts| closure(&[facts], member));
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
    json!({
        "meaning": MEANING,
        "declarations": declarations,
        "members": run.iter().map(|(member, result)| (member.clone(), closure_value(result))).collect::<BTreeMap<_, _>>(),
        "units_without_facts": missing,
        "demoted": demoted,
    })
}
