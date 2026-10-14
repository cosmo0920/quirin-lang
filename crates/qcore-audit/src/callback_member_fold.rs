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

use qcore_c_gateway::callback_member_closure::{MemberFacts, closure};
use qcore_c_gateway::constructed_state::{StateFacts, stability, stated_keys};
use qcore_schemas::callback::CallbackClause;
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};

/// The sidecar key of the fold's section.
pub const SECTION_KEY: &str = "callback_members";

/// The assumptions key naming the member-invoking declarations, recorded
/// before any unit (a checkpoint composer refuses a run that has any).
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

/// One unit's recorded facts.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    let constructed = constructed_declarations(contracts);
    if declarations.is_empty() && constructed.is_empty() {
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
    // Card OP-CONSTRUCTOR-DECLARATIONS: the run's stability of every stated
    // member, over every unit row (a row with no recorded state facts opens
    // it).
    let owned = |name: &str| {
        let owned = |result: qcore_schemas::contract::ResourceReturnMode| {
            matches!(result, qcore_schemas::contract::ResourceReturnMode::Owned { .. })
        };
        contracts
            .contract(name)
            .is_some_and(|contract| owned(contract.result))
            || contracts
                .retaining_contract(name)
                .is_some_and(|declared| owned(declared.contract.result))
    };
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
                .filter_map(|name| constructed.get(name).map(|members| (name.to_owned(), members)))
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
