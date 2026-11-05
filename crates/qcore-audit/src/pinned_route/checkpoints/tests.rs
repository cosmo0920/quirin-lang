//! Card CORPUS-GROUP-CHECKPOINTS: the pinned group checkpoint's files,
//! round trip and identity, on fixtures. Evidence item 3 of the card: a
//! changed input of each kind invalidates exactly the groups that read it.

use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_c_gateway::legacy_c::export_cross_unit_homes;
use qcore_schemas::contract_refutation::{ContractRefutation, RefutationKind, RefutationTable};

use super::*;

/// A fresh scratch directory under the system temporary directory, removed
/// on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "qcore-pinned-checkpoints-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn contracts(extra: bool) -> ResourceContracts {
    let mut text = String::from(
        "module contract;\nextern fn keep(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n",
    );
    if extra {
        text.push_str("extern fn other(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n");
    }
    qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "fixture.qv".to_owned(),
        text,
    )]))
    .unwrap()
}

fn run_identity(environment: Vec<EnvironmentValue>) -> PinnedRunIdentity {
    PinnedRunIdentity {
        route: "pinned".into(),
        analyzer: AnalyzerIdentity {
            package: "qcore-audit".into(),
            version: "0".into(),
            build_profile: "release".into(),
            build_commit: Some("c".repeat(40)),
            binary_sha256: "b".repeat(64),
        },
        fluent_bit_root: "/fixture/fluent-bit".into(),
        fluent_bit_revision: "f".repeat(40),
        source_profile_sha256: "p".repeat(64),
        source_contracts: SourceContractsIdentity {
            configuration: "/fixture/source-contracts.json".into(),
            configuration_sha256: "s".repeat(64),
            package: None,
        },
        contract_manifest: "/fixture/manifest.json".into(),
        contract_manifest_sha256: "m".repeat(64),
        locks: vec![LockDigest {
            path: "providers/x/contracts.lock".into(),
            sha256: "l".repeat(64),
        }],
        pin_evaluation_sha256: "t".repeat(64),
        compiler: CompilerIdentity {
            program: "/usr/bin/clang".into(),
            version: "21".into(),
            target: "x86_64-pc-linux-gnu".into(),
        },
        environment,
        member_facts_sha256: Some("a".repeat(64)),
    }
}

fn stored_unit(seed: u64) -> StoredUnit {
    let row = json!({
        "library": "fixture", "file": format!("u{seed}.c"), "status": "Unknown",
        "functions": [{"function": "f", "verdict": "Safe", "ratio": 0.5, "count": seed}],
    });
    StoredUnit {
        output: format!("u{seed}.c: 1 function(s)\n"),
        rows: vec![BinaryValue::from_json(&row).unwrap()],
        diagnostic_counts: vec![CategoryCount {
            category: "custody".into(),
            count: seed,
        }],
        bounds: Vec::new(),
        bounds_evidence: Vec::new(),
        null: vec![
            BinaryValue::from_json(&json!({"function": "f", "null": {"verdict": "Unknown"}}))
                .unwrap(),
        ],
        null_evidence: Vec::new(),
        callback_members: Vec::new(),
        counts: CountsRecord {
            units: 1,
            source_functions: 1,
            projected_functions: 1,
            unknown_units: 1,
            ..CountsRecord::default()
        },
        native_results: Vec::new(),
        teardown_results: vec![
            BinaryValue::from_json(&json!({"kind": "fixture", "seed": seed})).unwrap(),
        ],
    }
}

fn complete_record() -> GroupRecord {
    GroupRecord {
        prelude_output: "fixture prelude\n".into(),
        prelude: Ok(()),
        units: vec![
            Some(UnitRecord::Done(stored_unit(1))),
            Some(UnitRecord::Done(stored_unit(2))),
        ],
        finish: Some((
            "fixture postlude\n".into(),
            Ok(vec![
                BinaryValue::from_json(&json!({"kind": "nested"})).unwrap(),
            ]),
        )),
    }
}

fn identity() -> PinnedIdentity {
    PinnedIdentity {
        run: run_identity(Vec::new()),
        stage: stage_identity(Stage::First, &contracts(false), &[], "n".repeat(64)).unwrap(),
        group: PinnedGroupIdentity {
            label: "fixture".into(),
            source_root: "/fixture/lib".into(),
            units: (1..=2)
                .map(|seed| PinnedUnitFingerprint {
                    file: format!("/fixture/lib/u{seed}.c"),
                    record_sha256: format!("{seed}").repeat(64),
                    raw_source_sha256: "r".repeat(64),
                    preprocessed_sha256: None,
                    preparation: PreparationFacts::NotPrepared,
                })
                .collect(),
        },
    }
}

#[test]
fn a_sealed_group_is_reused_only_under_its_identity_and_refused_when_damaged() {
    let scratch = Scratch::new("seal");
    let path = checkpoint_path(&scratch.0, "fixture", Stage::First);
    assert!(path.ends_with("fixture.first.qo"));
    let identity = identity();
    let record = complete_record();
    let content = seal(&path, &identity, &record, &PinnedProvenance::default()).unwrap();

    // Equal identity: every section restored exactly.
    let (loaded, reused_content) = load(&path, &identity).unwrap();
    assert_eq!(reused_content, content);
    assert_eq!(loaded.prelude_output, record.prelude_output);
    assert!(loaded.complete());
    let stored = |record: &GroupRecord| {
        record
            .units
            .iter()
            .map(|unit| match unit {
                Some(UnitRecord::Done(unit)) => unit.clone(),
                _ => panic!("complete"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(stored(&loaded), stored(&record));
    assert_eq!(
        loaded
            .finish
            .as_ref()
            .map(|(output, result)| (output, result.as_ref().ok())),
        record
            .finish
            .as_ref()
            .map(|(output, result)| (output, result.as_ref().ok()))
    );

    // A provenance-only difference keeps the content digest.
    let again = scratch.0.join("again.qo");
    let other = seal(
        &again,
        &identity,
        &record,
        &PinnedProvenance {
            wall_time_ms: 7,
            jobs: 3,
            process_peak_rss_kib: Some(1),
        },
    )
    .unwrap();
    assert_eq!(other, content);

    // Another identity: recomputed, naming the field.
    let mut changed = identity.clone();
    changed.stage = stage_identity(Stage::First, &contracts(true), &[], "n".repeat(64)).unwrap();
    assert_eq!(
        load(&path, &changed).err().unwrap(),
        "stage.environment_sha256 differs"
    );
    let mut changed = identity.clone();
    changed.run.analyzer.binary_sha256 = "d".repeat(64);
    assert_eq!(
        load(&path, &changed).err().unwrap(),
        "run.analyzer.binary_sha256 differs"
    );

    // Neither reader accepts the other's files.
    let current = CheckpointReader::open(&path).err().unwrap();
    assert!(
        current.contains("unsupported checkpoint schema version 1"),
        "{current}"
    );

    // A damaged file is refused whole and recomputed.
    let bytes = fs::read(&path).unwrap();
    let mut tampered = bytes.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    fs::write(&path, &tampered).unwrap();
    let refused = load(&path, &identity).err().unwrap();
    assert!(refused.starts_with("checkpoint refused:"), "{refused}");
    fs::write(&path, &bytes[..bytes.len() - 3]).unwrap();
    let refused = load(&path, &identity).err().unwrap();
    assert!(refused.contains("truncated"), "{refused}");
    fs::remove_file(&path).unwrap();
    assert_eq!(load(&path, &identity).err().unwrap(), "no checkpoint");

    // An incomplete group is never sealed.
    let mut incomplete = complete_record();
    incomplete.units[1] = Some(UnitRecord::Failed {
        output: String::new(),
        error: "boom".into(),
    });
    assert!(seal(&path, &identity, &incomplete, &PinnedProvenance::default()).is_err());
    assert!(!path.exists());
}

#[test]
fn a_restored_unit_is_the_stored_unit() {
    let unit = stored_unit(3);
    let products = restore_unit("f".repeat(40).as_str(), &unit).unwrap();
    assert_eq!(products.counts.units, 1);
    assert_eq!(products.counts.unknown_units, 1);
    let (again, _) = store_unit(unit.output.clone(), products).unwrap();
    assert_eq!(again, unit);
}

#[test]
fn the_environment_records_selections_and_the_link_map_content() {
    let base = |null: &str, link: &str, jobs: &str| {
        environment_identity(
            [
                ("QCORE_CLANG".to_owned(), "/usr/bin/clang".to_owned()),
                ("QCORE_NULL_PROPERTY".to_owned(), null.to_owned()),
                ("QCORE_OUTER_PROJECTION".to_owned(), format!("{null}.outer")),
                ("QCORE_CROSS_UNIT_LINK_MAP".to_owned(), link.to_owned()),
                ("QCORE_AUDIT_JOBS".to_owned(), jobs.to_owned()),
                ("PATH".to_owned(), "/bin".to_owned()),
            ],
            Some("1".repeat(64).as_str()),
        )
        .unwrap()
    };
    let one = base("/run/one/corpus-null.jsonl", "/repo/a/link-map.json", "3");
    // Output paths, the link map's path and the job count are not inputs.
    assert_eq!(
        one,
        base("/run/two/corpus-null.jsonl", "/repo/b/link-map.json", "16")
    );
    assert_eq!(
        one.iter()
            .map(|value| (value.name.as_str(), value.value.as_str()))
            .collect::<Vec<_>>(),
        [
            ("QCORE_CLANG", "/usr/bin/clang"),
            (
                "QCORE_CROSS_UNIT_LINK_MAP",
                &*format!("sha256:{}", "1".repeat(64))
            ),
            ("QCORE_NULL_PROPERTY", "selected"),
            ("QCORE_OUTER_PROJECTION", "selected"),
        ]
    );
    // Its content is.
    let other = environment_identity(
        [(
            "QCORE_CROSS_UNIT_LINK_MAP".to_owned(),
            "/repo/a/link-map.json".to_owned(),
        )],
        Some("2".repeat(64).as_str()),
    )
    .unwrap();
    assert_ne!(other[0], one[1]);
    // A selection the run did not make is absent, so selecting one changes it.
    let unselected = environment_identity(
        [("QCORE_CLANG".to_owned(), "/usr/bin/clang".to_owned())],
        None,
    )
    .unwrap();
    assert_ne!(unselected, one);
    assert!(
        environment_identity(
            [("QCORE_CROSS_UNIT_LINK_MAP".to_owned(), "/x".to_owned())],
            None
        )
        .is_err()
    );
}

/// The fixture's three library groups: each its own source and header.
struct Corpus {
    root: Scratch,
}

impl Corpus {
    fn new() -> Self {
        let root = Scratch::new("corpus");
        for group in ["alpha", "beta", "gamma"] {
            let directory = root.0.join(group);
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                directory.join(format!("{group}.h")),
                format!("/* {group} header */\nint {group}_value(int x);\n"),
            )
            .unwrap();
            fs::write(
                directory.join(format!("{group}.c")),
                format!("#include \"{group}.h\"\nint {group}_value(int x) {{ return x + 1; }}\n"),
            )
            .unwrap();
        }
        Self { root }
    }

    fn plans(&self) -> Vec<LibraryPlan> {
        ["alpha", "beta", "gamma"]
            .into_iter()
            .map(|group| {
                let directory = self.root.0.join(group);
                LibraryPlan {
                    library: group.into(),
                    source_root: directory.clone(),
                    selected: vec![CompileEntry {
                        directory: directory.clone(),
                        command: format!("cc -I{} -c {}.c", directory.display(), group),
                        file: directory.join(format!("{group}.c")),
                    }],
                }
            })
            .collect()
    }

    fn groups(&self) -> Vec<PinnedGroupIdentity> {
        group_identities(&self.plans(), 2)
            .into_iter()
            .map(Result::unwrap)
            .collect()
    }
}

/// Which groups a stored identity set would recompute against the current
/// one, with the first differing field of each.
fn invalidated(stored: &[PinnedIdentity], current: &[PinnedIdentity]) -> Vec<(String, String)> {
    stored
        .iter()
        .zip(current)
        .filter_map(|(stored, current)| {
            identity_difference(stored, current)
                .unwrap()
                .map(|field| (current.group.label.clone(), field))
        })
        .collect()
}

fn identities(
    run: &PinnedRunIdentity,
    stage: &StageIdentity,
    groups: &[PinnedGroupIdentity],
) -> Vec<PinnedIdentity> {
    groups
        .iter()
        .map(|group| PinnedIdentity {
            run: run.clone(),
            stage: stage.clone(),
            group: group.clone(),
        })
        .collect()
}

fn labels(changes: &[(String, String)]) -> Vec<&str> {
    changes.iter().map(|(label, _)| label.as_str()).collect()
}

#[test]
fn a_changed_input_of_each_kind_invalidates_exactly_the_groups_that_read_it() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let corpus = Corpus::new();
    let base_contracts = contracts(false);
    let bounds = "n".repeat(64);
    let run = run_identity(
        environment_identity(
            [
                ("QCORE_CLANG".to_owned(), "/usr/bin/clang".to_owned()),
                (
                    "QCORE_NULL_PROPERTY".to_owned(),
                    "/run/null.jsonl".to_owned(),
                ),
            ],
            None,
        )
        .unwrap(),
    );
    let first = stage_identity(Stage::First, &base_contracts, &[], bounds.clone()).unwrap();
    let groups = corpus.groups();
    let stored = identities(&run, &first, &groups);
    // Nothing changed: every group is reused.
    assert!(invalidated(&stored, &identities(&run, &first, &corpus.groups())).is_empty());

    // 1. A group's own source: that group alone.
    let beta = corpus.root.0.join("beta/beta.c");
    fs::write(
        &beta,
        "#include \"beta.h\"\nint beta_value(int x) { return x + 2; }\n",
    )
    .unwrap();
    let changes = invalidated(&stored, &identities(&run, &first, &corpus.groups()));
    assert_eq!(labels(&changes), ["beta"]);
    assert!(changes[0].1.starts_with("group.units[0]"), "{changes:?}");
    // A header it includes: that group alone; a comment the preprocessor
    // removes reaches no analysis and invalidates nothing.
    let after_source = corpus.groups();
    let stored_after = identities(&run, &first, &after_source);
    let header = corpus.root.0.join("gamma/gamma.h");
    fs::write(
        &header,
        "/* gamma header, reworded */\nint gamma_value(int x);\n",
    )
    .unwrap();
    assert!(invalidated(&stored_after, &identities(&run, &first, &corpus.groups())).is_empty());
    fs::write(&header, "/* gamma header */\nlong gamma_value(int x);\n").unwrap();
    assert_eq!(
        labels(&invalidated(
            &stored_after,
            &identities(&run, &first, &corpus.groups())
        )),
        ["gamma"]
    );
    // Its compile record: that group alone.
    let mut plans = corpus.plans();
    plans[0].selected[0].command.push_str(" -DFIXTURE=1");
    let recorded = group_identities(&plans, 1)
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let changes = invalidated(
        &identities(&run, &first, &corpus.groups()),
        &identities(&run, &first, &recorded),
    );
    assert_eq!(labels(&changes), ["alpha"]);
    // An unreadable source cannot be fingerprinted: that group is recomputed.
    let mut plans = corpus.plans();
    plans[2].selected[0].file = corpus.root.0.join("gamma/missing.c");
    let missing = group_identities(&plans, 1);
    assert!(missing[0].is_ok() && missing[1].is_ok() && missing[2].is_err());

    let groups = corpus.groups();
    let stored = identities(&run, &first, &groups);
    let all = ["alpha", "beta", "gamma"];

    // 2. The contract environment: every group.
    let changed = stage_identity(Stage::First, &contracts(true), &[], bounds.clone()).unwrap();
    let changes = invalidated(&stored, &identities(&run, &changed, &groups));
    assert_eq!(labels(&changes), all);
    assert!(
        changes
            .iter()
            .all(|(_, field)| field == "stage.environment_sha256")
    );

    // 3. The analyzer build, the pin evaluation, a lock, the source profile:
    //    every group, each named.
    let mut builds = Vec::new();
    let mut changed = run.clone();
    changed.analyzer.binary_sha256 = "e".repeat(64);
    builds.push((changed, "run.analyzer.binary_sha256"));
    let mut changed = run.clone();
    changed.pin_evaluation_sha256 = "u".repeat(64);
    builds.push((changed, "run.pin_evaluation_sha256"));
    let mut changed = run.clone();
    changed.locks[0].sha256 = "k".repeat(64);
    builds.push((changed, "run.locks"));
    let mut changed = run.clone();
    changed.source_profile_sha256 = "q".repeat(64);
    builds.push((changed, "run.source_profile_sha256"));
    let mut changed = run.clone();
    changed.compiler.version = "22".into();
    builds.push((changed, "run.compiler.version"));
    for (changed, field) in builds {
        let changes = invalidated(&stored, &identities(&changed, &first, &groups));
        assert_eq!(labels(&changes), all, "{field}");
        assert!(
            changes.iter().all(|(_, found)| found.starts_with(field)),
            "{field}: {changes:?}"
        );
    }

    // 4. The route's environment: a selection or the link map's content is
    //    every group's input; an output path is no group's.
    let mut moved = run.clone();
    moved.environment = environment_identity(
        [
            ("QCORE_CLANG".to_owned(), "/usr/bin/clang".to_owned()),
            (
                "QCORE_NULL_PROPERTY".to_owned(),
                "/elsewhere/null.jsonl".to_owned(),
            ),
        ],
        None,
    )
    .unwrap();
    assert!(invalidated(&stored, &identities(&moved, &first, &groups)).is_empty());
    for variables in [
        vec![("QCORE_CLANG", "/usr/bin/clang")],
        vec![
            ("QCORE_CLANG", "/usr/bin/clang"),
            ("QCORE_NULL_PROPERTY", "/run/null.jsonl"),
            ("QCORE_OUTER_PROJECTION", "/run/outer.json"),
        ],
        vec![
            ("QCORE_CLANG", "/usr/bin/clang"),
            ("QCORE_NULL_PROPERTY", "/run/null.jsonl"),
            ("QCORE_GATEWAY_RULES_OFF", "12n"),
        ],
        vec![
            ("QCORE_CLANG", "/usr/bin/clang"),
            ("QCORE_NULL_PROPERTY", "/run/null.jsonl"),
            ("QCORE_CROSS_UNIT_LINK_MAP", "/run/link-map.json"),
        ],
    ] {
        let mut changed = run.clone();
        changed.environment = environment_identity(
            variables
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned())),
            Some("1".repeat(64).as_str()),
        )
        .unwrap();
        let changes = invalidated(&stored, &identities(&changed, &first, &groups));
        assert_eq!(labels(&changes), all, "{variables:?}");
        assert!(changes.iter().all(|(_, field)| field == "run.environment"));
    }

    // 5. Whole-run facts. The run's member facts: every group.
    let mut changed = run.clone();
    changed.member_facts_sha256 = Some("9".repeat(64));
    let changes = invalidated(&stored, &identities(&changed, &first, &groups));
    assert_eq!(labels(&changes), all);
    // The refutation table enters the `refuted` stage's environment, never
    // the `first` stage's: a changed table recomputes every group's refuted
    // stage and none of its first stage.
    let table = |unit: &str| {
        RefutationTable::from([(
            "keep".to_owned(),
            vec![ContractRefutation {
                unit: unit.into(),
                kind: RefutationKind::CustodyDuty,
                clause: "resource_borrowed(p)".into(),
                origin: None,
                reason: "fixture".into(),
            }],
        )])
    };
    let refuted = |unit: &str| {
        stage_identity(
            Stage::Refuted,
            &crate::refuted_contracts::with_table(&base_contracts, &table(unit)),
            &[],
            bounds.clone(),
        )
        .unwrap()
    };
    assert_ne!(
        refuted("alpha.c").environment_sha256,
        first.environment_sha256
    );
    let stored_refuted = identities(&run, &refuted("alpha.c"), &groups);
    assert!(
        invalidated(
            &stored_refuted,
            &identities(&run, &refuted("alpha.c"), &groups)
        )
        .is_empty()
    );
    let changes = invalidated(
        &stored_refuted,
        &identities(&run, &refuted("beta.c"), &groups),
    );
    assert_eq!(labels(&changes), all);
    assert!(
        changes
            .iter()
            .all(|(_, field)| field == "stage.environment_sha256")
    );
    assert!(invalidated(&stored, &identities(&run, &first, &groups)).is_empty());
    // The bounds and null environments: every group of the stage.
    let changed = stage_identity(Stage::First, &base_contracts, &[], "o".repeat(64)).unwrap();
    assert_eq!(
        labels(&invalidated(&stored, &identities(&run, &changed, &groups))),
        all
    );
    // The cross-unit homes: every group of the `imported` stage; a changed
    // home body changes the homes every caller is analysed with.
    let home = |body: &str| {
        let prepared = prepare_translation_unit(&C11TranslationUnit {
            source: body,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        export_cross_unit_homes(
            &prepared,
            &base_contracts,
            &std::collections::BTreeSet::from(["callee".to_owned()]),
            &"1".repeat(64),
        )
        .unwrap()
        .into_iter()
        .map(|export| export.home)
        .collect::<Vec<_>>()
    };
    let homes = home("void callee(void) { return; }\n");
    assert_eq!(homes.len(), 1);
    let imported =
        stage_identity(Stage::Imported, &base_contracts, &homes, bounds.clone()).unwrap();
    assert_ne!(imported.homes_sha256, first.homes_sha256);
    let stored_imported = identities(&run, &imported, &groups);
    let rehomed = home("void callee(void) { }\n");
    assert_eq!(rehomed.len(), 1);
    let changed = stage_identity(Stage::Imported, &base_contracts, &rehomed, bounds).unwrap();
    let changes = invalidated(&stored_imported, &identities(&run, &changed, &groups));
    assert_eq!(labels(&changes), all);
    assert!(
        changes
            .iter()
            .all(|(_, field)| field == "stage.homes_sha256")
    );
}

#[test]
fn a_tree_digest_covers_every_file_by_path() {
    let scratch = Scratch::new("tree");
    fs::create_dir_all(scratch.0.join("sub")).unwrap();
    fs::write(scratch.0.join("a.json"), "{}").unwrap();
    fs::write(scratch.0.join("sub/b.json"), "[]").unwrap();
    let one = tree_sha256(&scratch.0).unwrap();
    assert_eq!(one, tree_sha256(&scratch.0).unwrap());
    fs::write(scratch.0.join("sub/b.json"), "[1]").unwrap();
    let two = tree_sha256(&scratch.0).unwrap();
    assert_ne!(one, two);
    fs::rename(scratch.0.join("sub/b.json"), scratch.0.join("sub/c.json")).unwrap();
    assert_ne!(two, tree_sha256(&scratch.0).unwrap());
}
