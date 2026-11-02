//! Card **VERDICT-CONTRACT-DEPENDENCIES** (gateway rules 13f–13h): the
//! per-row dependency set, the body evidence, and the shared vectors binding
//! `proof/QCore/ContractDependencies.lean`.
//!
//! * `outcomeVectors` = [`UnitRecord::body_outcomes`], row for row.
//! * `closureVectors` = [`row_dependencies`] over the Lean `exampleUnit`,
//!   built as a record with real CFG instructions for its applied callees.
//! * A real unit: the set comes from the route's own consultations, a
//!   summary's dependencies propagate to its caller, a declaration its body
//!   validates is `validated` and one it does not is not, and recording
//!   changes no verdict.

use std::collections::BTreeSet;

use qcore_c_gateway::contract_dependencies::{
    BodyOutcome, Recording, UnitRecord, row_dependencies,
};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyCfg, LegacyCfgNode, LegacyFunctionSummary, LegacyInstruction,
    LegacyProgramSummary, ResourceApiKind, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, analyze_legacy_c_translation_unit,
};
use qcore_schemas::contract_consultation::Consulted;

const LEAN: &str = include_str!("../../../proof/QCore/ContractDependencies.lean");

/// The fields of every row of the Lean list `header` opens: quoted strings and
/// bare words, one `Vec` per row (a nested list is one field of its words).
fn lean_rows(header: &str) -> Vec<Vec<String>> {
    let start = LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the Lean list closes");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let line = line.trim_start_matches('(').trim_end_matches(',');
            let line = line.strip_suffix(')').unwrap_or(line);
            let mut fields = Vec::new();
            let mut rest = line;
            while !rest.is_empty() {
                rest = rest.trim_start_matches([',', ' ']);
                if let Some(list) = rest.strip_prefix('[') {
                    let close = list.find(']').expect("a closed list");
                    fields.push(list[..close].replace(['"', ' '], ""));
                    rest = &list[close + 1..];
                } else {
                    let end = rest.find(',').unwrap_or(rest.len());
                    fields.push(rest[..end].trim().trim_matches('"').to_owned());
                    rest = &rest[end..];
                }
            }
            fields
        })
        .collect()
}

fn names(list: &str) -> BTreeSet<String> {
    list.split(',')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn lean_outcome_vectors_match_the_unit_record() {
    let rows = lean_rows("def outcomeVectors : List (Bool × Bool × String) := [");
    assert_eq!(rows.len(), 4);
    for row in rows {
        let (validated, failed) = (row[0] == "true", row[1] == "true");
        let mut record = UnitRecord::default();
        record.examined.insert("d".to_owned());
        if validated {
            record
                .validated
                .insert("d".to_owned(), BTreeSet::from(["W4".to_owned()]));
        }
        if failed {
            record
                .failed
                .insert("d".to_owned(), BTreeSet::from(["conflict".to_owned()]));
        }
        let outcome = record.body_outcomes()["d"];
        assert_eq!(outcome.as_str(), row[2], "{row:?}");
    }
    // Evidence alone, with no examined body, is still an outcome.
    let mut record = UnitRecord::default();
    record
        .validated
        .insert("e".to_owned(), BTreeSet::from(["13c".to_owned()]));
    assert_eq!(record.body_outcomes()["e"], BodyOutcome::Validated);
}

fn call(callee: &str) -> LegacyCfgNode {
    LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction: LegacyInstruction::ContractCall {
            function: callee.to_owned(),
            destination: None,
            arguments: Vec::new(),
            contract: ResourceFunctionContract::default(),
            produced_sites: Vec::new(),
            aggregate_reads: Vec::new(),
            aggregate_consumed: Vec::new(),
            aggregate_addresses: Vec::new(),
            alias_reads: Vec::new(),
            nested: Vec::new(),
            container: None,
            retention: None,
        },
        successors: Vec::new(),
        line: 1,
        is_exit: true,
        loop_exit_edge: None,
    }
}

fn row(function: &str, applied: &[&str]) -> LegacyFunctionSummary {
    LegacyFunctionSummary {
        function: function.to_owned(),
        status: AnalysisStatus::Safe,
        diagnostics: Vec::new(),
        fixed_point_steps: 0,
        cfg: LegacyCfg {
            function: function.to_owned(),
            entry: 0,
            nodes: applied.iter().map(|callee| call(callee)).collect(),
            publications: Default::default(),
            conditional_names: BTreeSet::new(),
            tie_names: BTreeSet::new(),
            traversal: Default::default(),
            cell_tests: Default::default(),
        },
        exits: Vec::new(),
        counted_users: Vec::new(),
        refutations: Vec::new(),
        null_premises: Vec::new(),
        lifetime_effects: BTreeSet::new(),
        rule_obligations: Default::default(),
    }
}

/// The Lean `exampleUnit`, as a record and rows.
fn example() -> (UnitRecord, LegacyProgramSummary, ResourceContracts) {
    let consulted = [("f", vec!["g", "free", "local"]), ("g", vec!["h"])];
    let mut record = UnitRecord::default();
    for (function, spellings) in consulted {
        record.functions.insert(
            function.to_owned(),
            Consulted {
                found: spellings.iter().map(|name| (*name).to_owned()).collect(),
                missed: BTreeSet::new(),
                null_premises: BTreeSet::new(),
                relied: BTreeSet::new(),
            },
        );
        record.bodies.insert(
            function.to_owned(),
            spellings.iter().map(|name| (*name).to_owned()).collect(),
        );
    }
    record.inferred = BTreeSet::from(["g".to_owned(), "h".to_owned()]);
    let applied = [
        ("f", vec![]),
        ("g", vec!["close"]),
        ("h", vec!["fclose"]),
        ("k", vec!["malloc"]),
    ];
    let summary = LegacyProgramSummary {
        entry: "f".to_owned(),
        status: AnalysisStatus::Safe,
        functions: applied
            .iter()
            .map(|(function, callees)| ((*function).to_owned(), row(function, callees)))
            .collect(),
    };
    let environment = ResourceContracts::new([
        ("free", ResourceApiKind::Release),
        ("close", ResourceApiKind::Release),
        ("fclose", ResourceApiKind::Release),
        ("malloc", ResourceApiKind::Acquire),
    ]);
    (record, summary, environment)
}

#[test]
fn lean_closure_vectors_match_row_dependencies() {
    let (record, summary, environment) = example();
    let dependencies = row_dependencies(&record, &summary, &environment);
    let rows = lean_rows("def closureVectors : List (String × List String) := [");
    assert_eq!(rows.len(), 4);
    for row in rows {
        assert_eq!(
            dependencies[&row[0]].declarations,
            names(&row[1]),
            "{}",
            row[0]
        );
        assert!(dependencies[&row[0]].held.is_empty());
        assert!(dependencies[&row[0]].withdrawn.is_empty());
    }
    assert_eq!(
        dependencies["f"].summaries,
        BTreeSet::from(["g".to_owned(), "h".to_owned()])
    );
}

#[test]
fn a_lookup_the_body_does_not_spell_is_not_the_rows() {
    // A pass that scans the whole unit inside `f`'s build asks for another
    // function's callee; `f`'s body does not spell it, so it is not `f`'s.
    let (mut record, summary, environment) = example();
    record
        .functions
        .get_mut("f")
        .expect("f")
        .found
        .insert("malloc".to_owned());
    let dependencies = row_dependencies(&record, &summary, &environment);
    assert!(!dependencies["f"].declarations.contains("malloc"));
    // Spelled in the body, it is.
    record
        .bodies
        .get_mut("f")
        .expect("f")
        .insert("malloc".to_owned());
    let dependencies = row_dependencies(&record, &summary, &environment);
    assert!(dependencies["f"].declarations.contains("malloc"));
}

#[test]
fn a_held_or_withdrawn_spelling_is_named_by_its_standing() {
    let (mut record, summary, mut environment) = example();
    let f = record.functions.get_mut("f").expect("f");
    f.missed.insert("qcore_pin_mismatch__close".to_owned());
    f.missed
        .insert("qcore_abi_withdrawn__mismatch__fclose".to_owned());
    let body = record.bodies.get_mut("f").expect("f");
    body.insert("qcore_pin_mismatch__close".to_owned());
    body.insert("qcore_abi_withdrawn__mismatch__fclose".to_owned());
    environment.insert("unused", ResourceApiKind::Neutral);
    let dependencies = row_dependencies(&record, &summary, &environment);
    assert!(dependencies["f"].held.contains("close"));
    assert!(dependencies["f"].withdrawn.contains("fclose"));
}

const UNIT: &str = "\
void *malloc(unsigned long n);\n\
void free(void *p);\n\
static void drop(void *p) { free(p); }\n\
void owner(void) { void *p = malloc(4); if (!p) return; drop(p); }\n\
void released(void *p) { free(p); }\n\
void unrelated(int x) { int y = x + 1; (void) y; }\n";

fn unit_contracts() -> ResourceContracts {
    let mut contracts = ResourceContracts::new([("free", ResourceApiKind::Release)]);
    contracts.insert_contract(
        "malloc",
        ResourceFunctionContract {
            parameters: vec![ResourceParameterMode::Borrowed],
            result: ResourceReturnMode::Owned { nullable: true },
            ..ResourceFunctionContract::default()
        },
    );
    // A declaration whose body this unit defines and validates (W4).
    contracts.insert("released", ResourceApiKind::Release);
    contracts
}

#[test]
fn the_set_comes_from_the_routes_consultations_and_changes_no_verdict() {
    let contracts = unit_contracts();
    let unrecorded =
        analyze_legacy_c_translation_unit(UNIT, "owner", &contracts).expect("the unit analyses");
    let recording = Recording::start();
    let recorded =
        analyze_legacy_c_translation_unit(UNIT, "owner", &contracts).expect("the unit analyses");
    let record = recording.finish();
    assert_eq!(
        record.primary_source_sha256.as_deref(),
        Some(qcore_schemas::sha256_hex(UNIT.as_bytes()).as_str())
    );
    // Recording changes no row: status, diagnostics, CFG and exits.
    assert_eq!(unrecorded, recorded);
    let dependencies = row_dependencies(&record, &recorded, &contracts);
    let owner = &dependencies["owner"];
    assert_eq!(
        recorded.functions["owner"].status,
        AnalysisStatus::Safe,
        "{:?}",
        recorded.functions["owner"]
    );
    // `owner` applies `malloc` and the in-unit summary of `drop`, whose own
    // consultation of `free` is `owner`'s too.
    assert!(owner.declarations.contains("malloc"), "{owner:?}");
    assert!(owner.declarations.contains("free"), "{owner:?}");
    assert!(owner.summaries.contains("drop"), "{owner:?}");
    assert!(!owner.declarations.contains("released"), "{owner:?}");
    // A function that consults nothing rests on nothing.
    assert!(dependencies["unrelated"].declarations.is_empty());
    // `released`'s own declaration is its entry contract.
    assert!(dependencies["released"].declarations.contains("released"));
    assert!(dependencies["released"].declarations.contains("free"));
    // Its body validates it here; nothing is recorded without a recording.
    assert_eq!(
        record.body_outcomes().get("released"),
        Some(&BodyOutcome::Validated),
        "{record:?}"
    );
    let after = Recording::start().finish();
    assert!(after.functions.is_empty() && after.examined.is_empty());
}

#[test]
fn a_body_that_does_not_validate_its_declaration_is_not_validated() {
    // `released` keeps its argument instead of releasing it: the body's
    // summary conflicts with the declaration, so the unit's outcome fails.
    let unit = "\
void free(void *p);\n\
void *keep;\n\
void released(void *p) { keep = p; }\n";
    let contracts = unit_contracts();
    let recording = Recording::start();
    let summary =
        analyze_legacy_c_translation_unit(unit, "released", &contracts).expect("the unit analyses");
    let record = recording.finish();
    assert_ne!(summary.functions["released"].status, AnalysisStatus::Safe);
    assert_ne!(
        record.body_outcomes().get("released"),
        Some(&BodyOutcome::Validated),
        "{record:?}"
    );
    assert!(record.body_outcomes().contains_key("released"));
}

#[test]
fn recording_twice_on_one_thread_is_independent() {
    let contracts = unit_contracts();
    let first = Recording::start();
    let _ = analyze_legacy_c_translation_unit(UNIT, "owner", &contracts).expect("analyses");
    let first = first.finish();
    let second = Recording::start();
    let _ = analyze_legacy_c_translation_unit(UNIT, "owner", &contracts).expect("analyses");
    let second = second.finish();
    assert_eq!(first, second);
}

#[test]
fn a_declined_probe_leaves_no_consultation_in_the_function_log() {
    // `step` has a two-class return-code disposition (`== 0` writes parameter 0
    // as `Out`, `!= 0` leaves it `Borrowed`). In `if (step(p) == 0)`, the
    // relational condition probe (`relational_condition_plan`) classifies edge
    // 0 (`== 0`) under the minted class contract `qcore_relational_class__0__step`,
    // which consults that minted name in `contracts.functions()`, and then
    // declines when `classify_simple` refuses `p` at the `Out` slot. Restoring
    // the probe state rolls back the minted class consultation from `subject`'s
    // recorded `found` set.
    use qcore_schemas::relational_post::{
        ExitClass, RelationOp, RelationalContract, RelationalOrigin, ReturnRelation,
    };
    let unit = "\
int step(void **p);\n\
void subject(void **p) { if (step(p) == 0) { } }\n";
    let mut contracts = ResourceContracts::default();
    contracts.insert_relational(
        "step",
        RelationalContract {
            origin: RelationalOrigin::Declared,
            classes: vec![
                ExitClass {
                    relation: ReturnRelation::new(RelationOp::Eq, 0),
                    contract: ResourceFunctionContract::new(
                        [ResourceParameterMode::Out { nullable: false }],
                        ResourceReturnMode::None,
                    ),
                },
                ExitClass {
                    relation: ReturnRelation::new(RelationOp::Ne, 0),
                    contract: ResourceFunctionContract::new(
                        [ResourceParameterMode::Borrowed],
                        ResourceReturnMode::None,
                    ),
                },
            ],
            withdrawn: None,
        },
    );
    let recording = Recording::start();
    let summary =
        analyze_legacy_c_translation_unit(unit, "subject", &contracts).expect("the unit analyses");
    let record = recording.finish();
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Unknown);
    assert!(
        !record.functions["subject"]
            .found
            .contains("qcore_relational_class__0__step"),
        "{:?}",
        record.functions["subject"]
    );
    // Non-vacuous: when both edges admit (`if (step(&p) == 0)`), the probe is
    // kept and the minted class consultations stay in `subject`'s `found` set.
    let admitted_unit = "\
int step(void **p);\n\
void free(void *p);\n\
void subject(void) { void *p = 0; if (step(&p) == 0) { free(p); } }\n";
    contracts.insert("free", ResourceApiKind::Release);
    let recording = Recording::start();
    let summary = analyze_legacy_c_translation_unit(admitted_unit, "subject", &contracts)
        .expect("the unit analyses");
    let record = recording.finish();
    assert_eq!(summary.functions["subject"].status, AnalysisStatus::Safe);
    assert!(
        record.functions["subject"]
            .found
            .contains("qcore_relational_class__0__step")
            && record.functions["subject"]
                .found
                .contains("qcore_relational_class__1__step"),
        "{:?}",
        record.functions["subject"]
    );
}

#[test]
fn actual_calls_and_summary_inheritance_lift_exactly_the_consulted_atoms() {
    use qcore_schemas::hypothesis::{Atom, Graph, Proposition};
    use std::collections::BTreeMap;
    let contracts = unit_contracts();
    let recording = Recording::start();
    let summary = analyze_legacy_c_translation_unit(UNIT, "owner", &contracts).unwrap();
    let record = recording.finish();
    let rows = row_dependencies(&record, &summary, &contracts);
    let graphs = ["free", "malloc", "released"]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                Graph {
                    version: 1,
                    atoms: BTreeMap::from([(
                        format!("{name}_atom"),
                        Atom {
                            id: format!("{name}_atom"),
                            proposition: Proposition::Opaque {
                                statement: format!("{name} assumption"),
                            },
                        },
                    )]),
                    premises: BTreeMap::new(),
                    refinements: BTreeMap::new(),
                    requirements: Vec::new(),
                },
            )
        })
        .collect();
    let closure =
        qcore_c_gateway::contract_dependencies::hypothesis_closure(&rows["owner"], &graphs);
    assert_eq!(
        closure.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["free", "malloc"]
    );
    assert_eq!(closure["free"], BTreeSet::from(["free_atom".into()]));
    assert_eq!(closure["malloc"], BTreeSet::from(["malloc_atom".into()]));
    assert!(rows["owner"].summaries.contains("drop"));
    assert!(
        qcore_c_gateway::contract_dependencies::hypothesis_closure(&rows["unrelated"], &graphs)
            .is_empty()
    );
}
