//! Card ARM-OBLIGATIONS: the carriage of a rule package admission's
//! obligations (`QCore.ArmObligations`). Card ARM-OBLIGATIONS-UNIT-GRAIN:
//! its row grain.

use std::collections::{BTreeMap, BTreeSet};

use qcore_gateway_rule_api::{Admission, Answer, Obligation};

use super::*;

#[path = "../../../../tests/support/tracked_files.rs"]
mod tracked;
use crate::legacy_c::{
    AnalysisStatus, LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionSummary,
};

fn admission(obligations: Vec<Obligation>) -> Admission<u8> {
    Admission {
        lowering: 7,
        obligations,
    }
}

fn premise(name: &str) -> Obligation {
    Obligation::Premise(name.to_owned())
}

fn open(key: &str) -> Obligation {
    Obligation::Open(key.to_owned())
}

fn carriage(rule: &str, name: &str) -> Carriage {
    Carriage {
        rule: rule.to_owned(),
        name: name.to_owned(),
    }
}

/// Every subset of a small obligation alphabet, as the model's
/// `the_carriage_preserves_every_obligation` quantifies.
fn every_obligation_list() -> Vec<Vec<Obligation>> {
    let alphabet = [premise("p"), premise("q"), open("k")];
    (0..1u8 << alphabet.len())
        .map(|mask| {
            alphabet
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, obligation)| obligation.clone())
                .collect()
        })
        .collect()
}

#[test]
fn an_admission_without_obligations_is_its_lowering_anywhere() {
    assert_eq!(carry("35", admission(Vec::new())), Some(7));
    let frame = Frame::open();
    assert_eq!(carry("35", admission(Vec::new())), Some(7));
    assert!(frame.close().0.is_empty());
}

#[test]
fn with_no_unit_frame_an_obligation_cannot_be_carried_and_is_refused() {
    for obligations in every_obligation_list() {
        let empty = obligations.is_empty();
        assert_eq!(carry("37", admission(obligations)).is_some(), empty);
    }
    assert_eq!(carry_answer("37", Answer::<u8>::Refuse), None);
}

#[test]
fn the_carriage_preserves_every_obligation() {
    for obligations in every_obligation_list() {
        let frame = Frame::open();
        assert_eq!(carry("37", admission(obligations.clone())), Some(7));
        let (frame, _) = frame.close();
        // A host's admission has no established attribution: unit grain.
        assert!(frame.declarations.is_empty());
        let carried = frame.unit;
        assert_eq!(carried, Carried::of("37", &obligations));
        for obligation in &obligations {
            match obligation {
                Obligation::Premise(name) => {
                    assert!(carried.premises.contains(&carriage("37", name)));
                }
                Obligation::Open(key) => assert!(carried.open.contains(&carriage("37", key))),
            }
        }
    }
}

#[test]
fn a_nested_frame_hands_what_it_carried_to_the_enclosing_one() {
    let outer = Frame::open();
    assert_eq!(carry("35", admission(vec![premise("outer")])), Some(7));
    {
        let inner = Frame::open();
        assert_eq!(carry("37", admission(vec![open("inner")])), Some(7));
        inner.carry_declarations(BTreeMap::from([(
            "spec".to_owned(),
            Carried::of("37", &[premise("attributed")]),
        )]));
        // The inner frame carries what was carried while it was open.
        let (carried, _) = inner.close();
        assert_eq!(carried.unit.open, BTreeSet::from([carriage("37", "inner")]));
        assert!(carried.unit.premises.is_empty());
        assert_eq!(
            carried.declarations["spec"].premises,
            BTreeSet::from([carriage("37", "attributed")])
        );
    }
    {
        // Dropped without closing (an analysis that returned early).
        let _dropped = Frame::open();
        assert_eq!(carry("33", admission(vec![premise("dropped")])), Some(7));
    }
    outer.carry_declarations(BTreeMap::from([(
        "spec".to_owned(),
        Carried::of("36", &[premise("held")]),
    )]));
    let (carried, _) = outer.close();
    assert_eq!(
        carried.all().premises,
        BTreeSet::from([
            carriage("33", "dropped"),
            carriage("35", "outer"),
            carriage("36", "held"),
            carriage("37", "attributed"),
        ])
    );
    assert_eq!(
        carried.all().open,
        BTreeSet::from([carriage("37", "inner")])
    );
    // Card ARM-OBLIGATIONS-UNIT-GRAIN: which enclosing rows read a nested
    // analysis is not attributed, so everything a nested frame carried, its
    // attributed declarations included, reaches the enclosing frame at unit
    // grain; the enclosing frame's own declaration stays attributed.
    assert_eq!(
        carried.unit.premises,
        BTreeSet::from([
            carriage("33", "dropped"),
            carriage("35", "outer"),
            carriage("37", "attributed"),
        ])
    );
    assert_eq!(
        carried.declarations,
        BTreeMap::from([("spec".to_owned(), Carried::of("36", &[premise("held")]))])
    );
    assert_eq!(carry("35", admission(vec![premise("after")])), None);
}

#[test]
fn the_consultation_carries_the_winners_obligations_or_ends_the_table() {
    let entries = [
        ("33", Answer::Refuse),
        ("37", Answer::Admit(admission(vec![premise("p")]))),
    ];
    let ask = |_: usize, entry: &(&str, Answer<u8>)| entry.1.clone();
    assert_eq!(consult(&entries, |entry| entry.0, |_| true, ask), None);
    let frame = Frame::open();
    assert_eq!(
        consult(&entries, |entry| entry.0, |_| true, ask),
        Some(Decided {
            index: 1,
            lowering: 7
        })
    );
    assert_eq!(
        frame.close().0.unit.premises,
        BTreeSet::from([carriage("37", "p")])
    );
    let held = consult_held(&entries, |entry| entry.0, |_| true, ask).expect("admitted");
    assert_eq!(held.obligations, [premise("p")]);
    assert_eq!(held.carried.premises, BTreeSet::from([carriage("37", "p")]));
}

fn row(status: AnalysisStatus, diagnostics: Vec<LegacyDiagnostic>) -> LegacyFunctionSummary {
    LegacyFunctionSummary {
        function: "f".into(),
        status,
        diagnostics,
        fixed_point_steps: 0,
        cfg: crate::legacy_c::LegacyCfg {
            function: "f".into(),
            entry: 0,
            nodes: Vec::new(),
            publications: Default::default(),
            conditional_names: BTreeSet::new(),
            tie_names: BTreeSet::new(),
            traversal: Default::default(),
        },
        exits: Vec::new(),
        counted_users: Vec::new(),
        refutations: Vec::new(),
        null_premises: Vec::new(),
        lifetime_effects: BTreeSet::new(),
        rule_obligations: Carried::default(),
    }
}

#[test]
fn a_premise_reaches_the_row_and_leaves_its_verdict() {
    let mut safe = row(AnalysisStatus::Safe, Vec::new());
    let carried = Carried::of("37", &[premise("asserted_allocation")]);
    apply(&mut safe, &carried, 3);
    assert_eq!(safe.status, AnalysisStatus::Safe);
    assert!(safe.diagnostics.is_empty());
    assert_eq!(
        safe.rule_obligations.premises,
        BTreeSet::from([carriage("37", "asserted_allocation")])
    );
}

#[test]
fn an_open_obligation_keeps_the_row_out_of_safe() {
    let carried = Carried::of("37", &[premise("p"), open("k")]);
    let mut safe = row(AnalysisStatus::Safe, Vec::new());
    apply(&mut safe, &carried, 3);
    assert_eq!(safe.status, AnalysisStatus::Unknown);
    assert_eq!(safe.rule_obligations, carried);
    assert_eq!(
        safe.diagnostics,
        [LegacyDiagnostic {
            function: "f".into(),
            line: 3,
            kind: LegacyDiagnosticKind::Unsupported {
                description: open_description(&carriage("37", "k")),
            },
        }]
    );
    let leak = LegacyDiagnostic {
        function: "f".into(),
        line: 2,
        kind: LegacyDiagnosticKind::Leak {
            acquisition_line: 1,
        },
    };
    let mut unsafe_row = row(AnalysisStatus::Unsafe, vec![leak]);
    apply(&mut unsafe_row, &carried, 3);
    assert_eq!(unsafe_row.status, AnalysisStatus::Unsafe, "never better");
}

/// A whole unit's analysis: what its frame carries reaches every row, an
/// open obligation keeps every row and the unit out of `Safe`, and a unit
/// whose admissions carry nothing is analysed exactly as before.
#[test]
fn a_units_rows_carry_its_admissions_obligations() {
    let source = "int f(void) { return 0; }\nint g(void) { return f(); }\n";
    let contracts = crate::legacy_c::ResourceContracts::default();
    let analyse = |injected: Option<Carried>| {
        INJECTED.with(|slot| *slot.borrow_mut() = injected);
        let summary = crate::legacy_c::analyze_legacy_c_translation_unit(source, "g", &contracts)
            .expect("analysed");
        INJECTED.with(|slot| assert!(slot.borrow().is_none(), "the unit's frame took it"));
        summary
    };
    let plain = analyse(None);
    assert_eq!(plain.status, AnalysisStatus::Safe, "{plain:#?}");
    assert_eq!(plain.functions.len(), 2);
    assert!(
        plain
            .functions
            .values()
            .all(|row| row.rule_obligations.is_empty())
    );

    let premised = analyse(Some(Carried::of("37", &[premise("asserted_allocation")])));
    assert_eq!(premised.status, AnalysisStatus::Safe);
    for row in premised.functions.values() {
        assert_eq!(row.status, AnalysisStatus::Safe);
        assert_eq!(
            row.rule_obligations.premises,
            BTreeSet::from([carriage("37", "asserted_allocation")]),
            "{}",
            row.function
        );
    }
    let mut stripped = premised.clone();
    for row in stripped.functions.values_mut() {
        row.rule_obligations = Carried::default();
    }
    assert_eq!(
        stripped, plain,
        "a premise moves nothing but the assumptions"
    );

    let open = analyse(Some(Carried::of("37", &[open("k")])));
    assert_eq!(open.status, AnalysisStatus::Unknown, "in the denominator");
    for row in open.functions.values() {
        assert_eq!(row.status, AnalysisStatus::Unknown, "{}", row.function);
        assert!(row.diagnostics.iter().any(|diagnostic| matches!(
            &diagnostic.kind,
            LegacyDiagnosticKind::Unsupported { description }
                if *description == open_description(&carriage("37", "k"))
        )));
    }
}

/// Card ARM-OBLIGATIONS-UNIT-GRAIN: a whole unit's analysis with an
/// admission attributed to the declaration `alloc`: the rows whose
/// derivation reaches it carry it (`alloc` itself, its caller `wrap` and,
/// through `wrap`, `top`), the row that does not (`other`) does not; an
/// admission without established attribution still reaches every row; and
/// the carriage is the same with the audit's recording open.
#[test]
fn a_declarations_admission_reaches_exactly_the_rows_that_reach_it() {
    let source = "int alloc(int n) { return n; }\nint wrap(int n) { return alloc(n); }\n\
                  int top(void) { return wrap(1); }\nint other(void) { return 0; }\n";
    let contracts = crate::legacy_c::ResourceContracts::default();
    let analyse = |unit: Option<Carried>, declared: Option<Carried>| {
        INJECTED.with(|slot| *slot.borrow_mut() = unit);
        INJECTED_DECLARATIONS.with(|slot| {
            *slot.borrow_mut() = declared
                .map(|carried| BTreeMap::from([("alloc".to_owned(), carried)]))
                .unwrap_or_default();
        });
        crate::legacy_c::analyze_legacy_c_translation_unit(source, "top", &contracts)
            .expect("analysed")
    };
    let plain = analyse(None, None);
    assert_eq!(plain.status, AnalysisStatus::Safe, "{plain:#?}");
    assert_eq!(plain.functions.len(), 4);
    let resting = ["alloc", "top", "wrap"];

    let premised = analyse(None, Some(Carried::of("37", &[premise("p")])));
    for (name, row) in &premised.functions {
        let expected = if resting.contains(&name.as_str()) {
            Carried::of("37", &[premise("p")])
        } else {
            Carried::default()
        };
        assert_eq!(row.rule_obligations, expected, "{name}");
        assert_eq!(row.status, AnalysisStatus::Safe, "{name}");
    }
    let mut stripped = premised.clone();
    for row in stripped.functions.values_mut() {
        row.rule_obligations = Carried::default();
    }
    assert_eq!(
        stripped, plain,
        "a premise moves nothing but the assumptions"
    );

    // Recording independence: the certificate recheck reruns the analysis
    // outside the audit's recording and must reproduce every row.
    let recording = crate::contract_dependencies::Recording::start();
    let recorded = analyse(None, Some(Carried::of("37", &[premise("p")])));
    let _record = recording.finish();
    assert_eq!(recorded, premised);

    // An open obligation keeps exactly the resting rows, and so the unit, in
    // the denominator; the row that never rested on it stays `Safe`.
    let open_unit = analyse(None, Some(Carried::of("37", &[open("k")])));
    assert_eq!(open_unit.status, AnalysisStatus::Unknown);
    for (name, row) in &open_unit.functions {
        let rests = resting.contains(&name.as_str());
        let expected = if rests {
            AnalysisStatus::Unknown
        } else {
            AnalysisStatus::Safe
        };
        assert_eq!(row.status, expected, "{name}");
        assert_eq!(row.rule_obligations.open.is_empty(), !rests, "{name}");
    }

    // Attribution not established: unit grain, beside the attributed one.
    let both = analyse(
        Some(Carried::of("35", &[premise("u")])),
        Some(Carried::of("37", &[premise("p")])),
    );
    for (name, row) in &both.functions {
        let mut expected = Carried::of("35", &[premise("u")]);
        if resting.contains(&name.as_str()) {
            expected.extend(Carried::of("37", &[premise("p")]));
        }
        assert_eq!(row.rule_obligations, expected, "{name}");
    }
}

#[test]
fn a_rows_set_is_a_subset_of_the_units_and_unit_entries_reach_every_row() {
    let carried = FrameCarriage {
        unit: Carried::of("35", &[premise("u")]),
        declarations: BTreeMap::from([
            ("a".to_owned(), Carried::of("37", &[premise("p")])),
            ("b".to_owned(), Carried::of("37", &[open("k")])),
        ]),
    };
    let all = carried.all();
    for reached in [&[][..], &["a"], &["b"], &["a", "b"]] {
        let row = carried.row(|declaration| reached.contains(&declaration));
        assert!(row.premises.is_subset(&all.premises));
        assert!(row.open.is_subset(&all.open));
        assert!(row.premises.contains(&carriage("35", "u")), "{reached:?}");
        assert_eq!(
            row.premises.contains(&carriage("37", "p")),
            reached.contains(&"a")
        );
        assert_eq!(
            row.open.contains(&carriage("37", "k")),
            reached.contains(&"b")
        );
    }
    // Attribution unknown for every declaration: the unit grain.
    assert_eq!(carried.row(|_| true), all);
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

/// `contract_dependencies::attribution::reaching` over hand-built notes: the
/// transitive callee, a relied-on callback candidate the body need not spell,
/// a minted spelling read back to its declaration, a row's applied callee,
/// and a function with work but no recorded body (every lookup counts).
#[test]
fn the_reaching_closure_follows_every_source_of_a_consultation() {
    use crate::contract_dependencies::attribution::{Attribution, reaching};
    use qcore_schemas::contract_consultation::Consulted;
    let minted = format!("{}s", crate::relational_post::CLASS_PREFIX);
    let looked_up = |found: &[&str], relied: &[&str]| Consulted {
        found: names(found),
        relied: names(relied),
        ..Consulted::default()
    };
    let attribution = Attribution {
        work: BTreeMap::from([
            ("a".to_owned(), Consulted::default()),
            ("b".to_owned(), Consulted::default()),
            ("cb_user".to_owned(), looked_up(&[], &["callback"])),
            ("callback".to_owned(), Consulted::default()),
            ("unbodied".to_owned(), looked_up(&["s"], &[])),
            ("filtered".to_owned(), looked_up(&["s"], &[])),
            ("minted_user".to_owned(), Consulted::default()),
            ("s".to_owned(), Consulted::default()),
        ]),
        bodies: BTreeMap::from([
            ("a".to_owned(), names(&["b", "x"])),
            ("b".to_owned(), names(&["s"])),
            ("cb_user".to_owned(), names(&["x"])),
            ("callback".to_owned(), names(&["s"])),
            ("filtered".to_owned(), names(&["x"])),
            ("minted_user".to_owned(), names(&[minted.as_str()])),
            ("s".to_owned(), names(&[])),
        ]),
        relied: BTreeMap::new(),
    };
    let rows = BTreeMap::from([
        ("row_callee".to_owned(), names(&["s"])),
        ("lonely".to_owned(), names(&["x"])),
    ]);
    let environment = crate::legacy_c::ResourceContracts::default();
    let reached = reaching(&attribution, &rows, &environment, &names(&["s", "t"]));
    assert_eq!(
        reached["s"],
        names(&[
            "a",
            "b",
            "callback",
            "cb_user",
            "minted_user",
            "row_callee",
            "s",
            "unbodied"
        ])
    );
    // Rule 13f's filter: a lookup the body does not spell is a whole-unit
    // pass's, not the function's.
    assert!(!reached["s"].contains("filtered"));
    assert!(reached["t"].is_empty());

    // A relied-on note survives a declining probe's rollback of the log.
    let mut rolled_back = attribution.clone();
    if let Some(work) = rolled_back.work.get_mut("cb_user") {
        work.relied.clear();
    }
    rolled_back
        .relied
        .insert("cb_user".to_owned(), names(&["callback"]));
    let reached = reaching(&rolled_back, &rows, &environment, &names(&["s"]));
    assert!(reached["s"].contains("cb_user"));
}

/// The attribution notes come from the analysis itself, in every run: a
/// unit frame records each function's work whether or not the audit's
/// recording is open, and `relied` reaches every function whose work is
/// running, past a rollback of the consultation log.
#[test]
fn the_attribution_is_noted_without_a_recording() {
    let frame = Frame::open();
    crate::contract_dependencies::within("outer", || {
        crate::contract_dependencies::within("inner", || {
            let saved = qcore_schemas::contract_consultation::snapshot();
            crate::contract_dependencies::relied("candidate");
            qcore_schemas::contract_consultation::restore(&saved);
        });
    });
    let (_, attribution) = frame.close();
    assert_eq!(attribution.relied["outer"], names(&["candidate"]));
    assert_eq!(attribution.relied["inner"], names(&["candidate"]));
    assert!(attribution.work.contains_key("outer"));
    assert!(attribution.work.contains_key("inner"));
    assert!(!crate::contract_dependencies::attribution::open());
}

#[test]
fn an_empty_carriage_leaves_the_row_untouched() {
    let mut safe = row(AnalysisStatus::Safe, Vec::new());
    let before = safe.clone();
    apply(&mut safe, &Carried::default(), 3);
    assert_eq!(safe, before);
}

/// `QCore.ArmObligations.applyVectors`: (verdict, an open obligation carried,
/// the row's verdict after the carriage).
const APPLY_VECTORS: &[(&str, bool, &str)] = &[
    ("safe", false, "safe"),
    ("safe", true, "unknown"),
    ("pending", false, "pending"),
    ("pending", true, "unknown"),
    ("unknown", false, "unknown"),
    ("unknown", true, "unknown"),
    ("unsafe", false, "unsafe"),
    ("unsafe", true, "unsafe"),
];

const LEAN: &str = include_str!("../../../../proof/QCore/ArmObligations.lean");

/// The Lean spelling of a verdict constructor (`unsafe` is a keyword).
fn lean_name(verdict: &str) -> String {
    if verdict == "unsafe" {
        "\u{ab}unsafe\u{bb}".to_owned()
    } else {
        verdict.to_owned()
    }
}

#[test]
fn the_apply_vectors_are_the_lean_vectors() {
    let header = "def applyVectors : List (Verdict \u{d7} Bool \u{d7} Verdict) := [";
    let start = LEAN.find(header).expect("the Lean vector list") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the list's end");
    let lean = body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.trim_start_matches(", ")
                .trim_end_matches(',')
                .to_owned()
        })
        .collect::<Vec<_>>();
    let rust = APPLY_VECTORS
        .iter()
        .map(|(before, open, after)| {
            format!("(.{}, {open}, .{})", lean_name(before), lean_name(after))
        })
        .collect::<Vec<_>>();
    assert_eq!(lean, rust);
}

#[test]
fn the_rust_carriage_is_the_apply_vectors() {
    let leak = || LegacyDiagnostic {
        function: "f".into(),
        line: 2,
        kind: LegacyDiagnosticKind::Leak {
            acquisition_line: 1,
        },
    };
    let held = || LegacyDiagnostic {
        function: "f".into(),
        line: 2,
        kind: LegacyDiagnosticKind::Unsupported {
            description: crate::legacy_c::review_hold::hold_description("g"),
        },
    };
    let unknown = || LegacyDiagnostic {
        function: "f".into(),
        line: 2,
        kind: LegacyDiagnosticKind::Unsupported {
            description: "u".into(),
        },
    };
    for (before, has_open, after) in APPLY_VECTORS {
        let (status, diagnostics) = match *before {
            "safe" => (AnalysisStatus::Safe, Vec::new()),
            "pending" => (AnalysisStatus::Pending, vec![held()]),
            "unknown" => (AnalysisStatus::Unknown, vec![unknown()]),
            _ => (AnalysisStatus::Unsafe, vec![leak()]),
        };
        let mut summary = row(status, diagnostics);
        assert_eq!(
            crate::legacy_c::verdict::classify(&summary.diagnostics),
            summary.status,
            "{before}"
        );
        let carried = if *has_open {
            Carried::of("37", &[open("k")])
        } else {
            Carried::of("37", &[premise("p")])
        };
        apply(&mut summary, &carried, 3);
        let expected = match *after {
            "safe" => AnalysisStatus::Safe,
            "pending" => AnalysisStatus::Pending,
            "unknown" => AnalysisStatus::Unknown,
            _ => AnalysisStatus::Unsafe,
        };
        assert_eq!(summary.status, expected, "{before} {has_open}");
    }
}

/// The ways a host could read an admission without its obligations: the
/// rule API's own consultation and an admission's fields.
const FORBIDDEN: &[&str] = &[
    "qcore_gateway_rule_api::consult",
    "answer::consult",
    "qcore_gateway_rule_api::*",
    "into_admission(",
    ".admission.lowering",
    ".admission.obligations",
    "Answer::Admit(Admission",
];

/// A host spelling that reads an admission outside this module, with the
/// file and the forbidden form. `use … consult` is matched separately,
/// because the API's `consult` is imported inside a braced list.
fn bypasses(path: &str, text: &str) -> Vec<(String, &'static str)> {
    // Code only: a comment may name the API's consultation.
    let text = text
        .lines()
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .collect::<Vec<_>>()
        .join("\n");
    let mut found = Vec::new();
    for form in FORBIDDEN {
        if text.contains(form) {
            found.push((path.to_owned(), *form));
        }
    }
    let import = "use qcore_gateway_rule_api::";
    for (at, _) in text.match_indices(import) {
        let rest = &text[at + import.len()..];
        let statement = &rest[..rest.find(';').unwrap_or(rest.len())];
        if statement
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == "consult")
        {
            found.push((path.to_owned(), "use qcore_gateway_rule_api::consult"));
        }
    }
    found
}

/// A dropped obligation is impossible: no gateway source but this module
/// consults a package or reads an admission (mutation-style: the scan below
/// is shown to catch every bypass first).
#[test]
fn no_host_reads_an_admission_without_carrying_its_obligations() {
    let mutants = [
        "use qcore_gateway_rule_api::{Answer, consult};\nfn f() { consult(&[1], |_| true, |_, _| Answer::admit(())); }",
        "fn f() { let _ = qcore_gateway_rule_api::consult(&[1], |_| true, |_, _| todo!()); }",
        "use qcore_gateway_rule_api::*;",
        "use qcore_gateway_rule_api::answer::consult as ask;",
        "#[cfg(test)]\nuse qcore_gateway_rule_api::{\n    Answer,\n    consult as c,\n};",
        "fn f(a: Answer<u8>) -> Option<u8> { a.into_admission().map(|a| a.lowering) }",
        "fn f(d: Consulted<u8>) -> u8 { d.admission.lowering }",
        "fn f(d: Consulted<u8>) -> bool { d.admission.obligations.is_empty() }",
        "fn f(a: Answer<u8>) { if let Answer::Admit(Admission { lowering, .. }) = a {} }",
    ];
    for mutant in mutants {
        assert!(!bypasses("mutant.rs", mutant).is_empty(), "{mutant}");
    }
    // Rule 29: the tracked sources, never a walk of the working tree.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = tracked::tracked_files(&src)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect::<Vec<_>>();
    assert!(files.len() > 100, "the scan reads the crate");
    let mut found = Vec::new();
    for path in files {
        if path
            .file_name()
            .is_some_and(|name| name == "arm_obligations.rs")
        {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a source");
        found.extend(bypasses(&path.display().to_string(), &text));
    }
    assert!(found.is_empty(), "{found:#?}");
}
