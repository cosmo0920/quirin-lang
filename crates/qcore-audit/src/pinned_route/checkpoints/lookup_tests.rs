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

//! Card MEASURE-LOOKUP-RECORD-REUSE: the lookup record and the restricted
//! stage identity, on constructed contract-only diffs.
//!
//! Each scenario seals every group of a stage under a BASE environment, then
//! measures a TIP environment twice: **composed** (each group reused from its
//! BASE file when its identity, computed over the stored lookup records'
//! scope, equals the TIP one; recomputed otherwise) and **full** (every group
//! recomputed). The composed groups' sealed sections must equal the full
//! run's byte for byte. Two consumers are measured: the real pinned unit
//! analysis (`audit_unit`, real Clang), which reads whole tables, and a
//! name-only consumer that reads the real environment only by name.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// A fresh scratch directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "qcore-lookup-reuse-{name}-{}-{}",
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

const KEEP_BORROWED: &str = "extern fn keep(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n";
const KEEP_RELEASED: &str = "extern fn keep(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_released(p, nonnull); ensures normal resource_return_none(); };\n";
const TOUCH: &str = "extern fn touch(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n";
const PROBE: &str = "extern fn probe(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n";

/// The environment the loader builds from these declarations, one module.
fn environment(declarations: &[&str]) -> ResourceContracts {
    let mut text = String::from("module contract;\n");
    for declaration in declarations {
        text.push_str(declaration);
    }
    qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
        "fixture.qv".to_owned(),
        text,
    )]))
    .unwrap()
}

fn run() -> PinnedRunIdentity {
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
        working_directory: "/fixture/work".into(),
        source_profile_sha256: "p".repeat(64),
        source_contracts: SourceContractsIdentity {
            configuration: "/fixture/source-contracts.json".into(),
            configuration_sha256: "s".repeat(64),
            package: None,
        },
        contract_manifest: "/fixture/manifest.json".into(),
        contract_manifest_sha256: "m".repeat(64),
        locks: Vec::new(),
        pin_evaluation_sha256: "t".repeat(64),
        compiler: CompilerIdentity {
            program: "/usr/bin/clang".into(),
            version: "21".into(),
            target: "x86_64-pc-linux-gnu".into(),
        },
        environment: Vec::new(),
        member_facts_sha256: None,
    }
}

/// One consumer: analyse unit `unit` of group `group` under `environment`,
/// returning its stored products and its lookup record.
type Analyse<'a> = dyn Fn(usize, usize, &ResourceContracts) -> (StoredUnit, UnitLookupRecord) + 'a;

/// A group's record from its analysed units.
fn group_record(units: Vec<(StoredUnit, UnitLookupRecord)>) -> GroupRecord {
    let (units, lookups): (Vec<_>, Vec<_>) = units.into_iter().unzip();
    GroupRecord {
        prelude_output: String::new(),
        prelude: Ok(()),
        units: units
            .into_iter()
            .map(|unit| Some(UnitRecord::Done(Box::new(unit))))
            .collect(),
        finish: Some((String::new(), Ok(Vec::new()))),
        lookups,
    }
}

/// The bytes a composed and a full run are compared by: the group's sealed
/// unit and lookup sections.
fn sections(record: &GroupRecord) -> Vec<u8> {
    let units = record
        .units
        .iter()
        .map(|unit| match unit {
            Some(UnitRecord::Done(unit)) => unit.as_ref().clone(),
            _ => panic!("complete"),
        })
        .collect::<Vec<_>>();
    let mut bytes = encode("pinned-units", &UnitsSection { units }).unwrap();
    bytes.extend(
        encode(
            "pinned-lookups",
            &LookupsSection {
                units: record.lookups.clone(),
            },
        )
        .unwrap(),
    );
    bytes
}

struct Outcome {
    /// Per group: reused (with its scope) or why it was recomputed.
    decisions: Vec<Result<String, String>>,
    /// The base seal's scope, per group.
    sealed_scopes: Vec<String>,
}

/// Seal every group under `base`, then measure `tip` composed and full and
/// require the two to be byte-identical.
fn measure(
    groups: &[PinnedGroupIdentity],
    analyse: &Analyse<'_>,
    base: &ResourceContracts,
    tip: &ResourceContracts,
) -> Outcome {
    let scratch = Scratch::new("measure");
    let bounds = "n".repeat(64);
    let identity =
        |group: &PinnedGroupIdentity, environment: &ResourceContracts, scope: Scope<'_>| {
            Ok::<_, String>(PinnedIdentity {
                run: run(),
                stage: stage_identity(Stage::First, environment, scope, &[], bounds.clone())?,
                group: group.clone(),
            })
        };
    let analyse_group = |index: usize, environment: &ResourceContracts| {
        group_record(
            (0..groups[index].units.len())
                .map(|unit| analyse(index, unit, environment))
                .collect(),
        )
    };
    // BASE: every group sealed under its own records' scope.
    let mut sealed_scopes = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let record = analyse_group(index, base);
        let sealed = identity(group, base, sealing_scope(&record.lookups, base)).unwrap();
        sealed_scopes.push(sealed.stage.environment_scope.clone());
        let path = checkpoint_path(&scratch.0, &group.label, Stage::First);
        seal(&path, &sealed, &record, &PinnedProvenance::default()).unwrap();
    }
    // TIP: composed and full.
    let mut decisions = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let path = checkpoint_path(&scratch.0, &group.label, Stage::First);
        let full = analyse_group(index, tip);
        let composed = match load(&path, |scope| identity(group, tip, scope)) {
            Ok((record, _, identity)) => {
                decisions.push(Ok(identity.stage.environment_scope));
                record
            }
            Err(reason) => {
                decisions.push(Err(reason));
                analyse_group(index, tip)
            }
        };
        assert_eq!(
            sections(&composed),
            sections(&full),
            "group {}: the composed run differs from the full run",
            group.label
        );
    }
    Outcome {
        decisions,
        sealed_scopes,
    }
}

fn reused(outcome: &Outcome) -> Vec<bool> {
    outcome.decisions.iter().map(Result::is_ok).collect()
}

// ---------------------------------------------------------------------------
// The name-only consumer
// ---------------------------------------------------------------------------

/// Each group's units, as the names each unit calls.
const CALLS: [&[&[&str]]; 3] = [
    &[&["keep"], &["keep", "malloc"]],
    &[&["touch", "probe"]],
    &[&["free"]],
];

fn named_groups() -> Vec<PinnedGroupIdentity> {
    ["alpha", "beta", "gamma"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| PinnedGroupIdentity {
            label: label.into(),
            source_root: format!("/fixture/{label}"),
            units: (0..CALLS[index].len())
                .map(|unit| PinnedUnitFingerprint {
                    file: format!("/fixture/{label}/u{unit}.c"),
                    record_sha256: format!("{index}{unit}").repeat(32),
                    raw_source_sha256: "r".repeat(64),
                    preprocessed_sha256: None,
                    preparation: PreparationFacts::NotPrepared,
                })
                .collect(),
        })
        .collect()
}

/// A deterministic consumer that reads its (recording) environment by name
/// only: one row per called name, carrying the contract it found.
fn name_only(
    reads_whole: bool,
) -> impl Fn(usize, usize, &ResourceContracts) -> (StoredUnit, UnitLookupRecord) {
    move |group, unit, environment| {
        let session = LookupSession::open(environment);
        let environment = session.environment();
        let mut functions = Vec::new();
        for name in CALLS[group][unit] {
            let contract = environment
                .contract(name)
                .map(|contract| serde_json::to_value(contract).unwrap());
            functions.push(json!({"function": name, "contract": contract}));
        }
        if reads_whole {
            functions.push(json!({"declared": environment.functions().len()}));
        }
        let row = json!({"group": group, "unit": unit, "functions": functions});
        let stored = StoredUnit {
            output: format!("{group}/{unit}\n"),
            rows: vec![BinaryValue::from_json(&row).unwrap()],
            diagnostic_counts: Vec::new(),
            bounds: Vec::new(),
            bounds_evidence: Vec::new(),
            null: Vec::new(),
            null_evidence: Vec::new(),
            callback_members: Vec::new(),
            counts: CountsRecord::default(),
            native_results: Vec::new(),
            teardown_results: Vec::new(),
        };
        (stored, session.record())
    }
}

#[test]
fn a_contract_change_invalidates_exactly_the_group_that_read_the_name() {
    let groups = named_groups();
    let base = environment(&[KEEP_BORROWED, TOUCH]);
    let analyse = name_only(false);

    // Nothing in the environment changed: every group reused, restricted.
    let same = measure(&groups, &analyse, &base, &base);
    assert_eq!(same.sealed_scopes, ["restricted"; 3]);
    assert_eq!(reused(&same), [true, true, true]);

    // A declaration alpha reads (found) changes: alpha alone recomputes.
    let changed = measure(
        &groups,
        &analyse,
        &base,
        &environment(&[KEEP_RELEASED, TOUCH]),
    );
    assert_eq!(reused(&changed), [false, true, true]);
    assert_eq!(
        changed.decisions[0].as_ref().err().unwrap(),
        "stage.environment_sha256 differs"
    );

    // A declaration beta looked up and missed appears: beta alone.
    let added = measure(
        &groups,
        &analyse,
        &base,
        &environment(&[KEEP_BORROWED, TOUCH, PROBE]),
    );
    assert_eq!(reused(&added), [true, false, true]);

    // A declaration nobody reads is added, and the module's text changes:
    // no group read either, so every group is reused.
    let unread = "extern fn unread(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n";
    let other = measure(
        &groups,
        &analyse,
        &base,
        &environment(&[KEEP_BORROWED, TOUCH, unread]),
    );
    assert_eq!(reused(&other), [true, true, true]);

    // A declaration beta reads is removed: beta alone.
    let removed = measure(&groups, &analyse, &base, &environment(&[KEEP_BORROWED]));
    assert_eq!(reused(&removed), [true, false, true]);
}

#[test]
fn a_whole_map_reader_always_reruns_on_any_environment_change() {
    let groups = named_groups();
    let base = environment(&[KEEP_BORROWED, TOUCH]);
    let analyse = name_only(true);
    let same = measure(&groups, &analyse, &base, &base);
    assert_eq!(same.sealed_scopes, ["whole"; 3]);
    // Within one environment the whole digest is equal: reused (schema 1's
    // within-revision resume is unchanged).
    assert_eq!(reused(&same), [true, true, true]);
    let unread = "extern fn unread(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n";
    let other = measure(
        &groups,
        &analyse,
        &base,
        &environment(&[KEEP_BORROWED, TOUCH, unread]),
    );
    assert_eq!(reused(&other), [false, false, false]);
    for decision in &other.decisions {
        assert_eq!(
            decision.as_ref().err().unwrap(),
            "stage.environment_sha256 differs"
        );
    }
}

#[test]
fn a_stored_restricted_claim_the_records_cannot_support_is_refused() {
    let scratch = Scratch::new("claim");
    let groups = named_groups();
    let base = environment(&[KEEP_BORROWED]);
    let (unit, mut lookups) = name_only(false)(2, 0, &base);
    let record = group_record(vec![(unit, lookups.clone())]);
    let identity = PinnedIdentity {
        run: run(),
        stage: stage_identity(
            Stage::First,
            &base,
            sealing_scope(&record.lookups, &base),
            &[],
            "n".repeat(64),
        )
        .unwrap(),
        group: groups[2].clone(),
    };
    assert_eq!(identity.stage.environment_scope, "restricted");
    // A record whose relied candidate was never read by name, or with a
    // whole-map read, cannot carry a restricted claim.
    lookups.relied.insert("unread".into());
    let mut forged = record;
    forged.lookups = vec![lookups];
    let path = checkpoint_path(&scratch.0, "gamma", Stage::First);
    seal(&path, &identity, &forged, &PinnedProvenance::default()).unwrap();
    let refused = load(&path, |_| Ok(identity.clone())).err().unwrap();
    assert!(
        refused.starts_with("checkpoint refused: a restricted stage identity"),
        "{refused}"
    );
    // At seal time a record that does not check under its environment
    // falls back to the whole environment.
    let unchecked = UnitLookupRecord {
        found: BTreeSet::from(["absent".to_owned()]),
        ..UnitLookupRecord::default()
    };
    assert!(matches!(
        sealing_scope(std::slice::from_ref(&unchecked), &base),
        Scope::Whole
    ));
}

// ---------------------------------------------------------------------------
// The real pinned unit analysis
// ---------------------------------------------------------------------------

/// Three library groups of real C units calling `keep`, `touch` and nothing.
struct Corpus {
    root: Scratch,
}

const SOURCES: [(&str, &str); 3] = [
    (
        "alpha",
        "void keep(unsigned char *p);\nvoid alpha(unsigned char *p) { keep(p); }\n",
    ),
    (
        "beta",
        "void touch(unsigned char *p);\nvoid beta(unsigned char *p) { touch(p); }\n",
    ),
    ("gamma", "int gamma(int x) { return x + 1; }\n"),
];

impl Corpus {
    fn new() -> Self {
        let root = Scratch::new("corpus");
        for (label, source) in SOURCES {
            let directory = root.0.join(label);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join(format!("{label}.c")), source).unwrap();
        }
        Self { root }
    }

    fn plans(&self) -> Vec<LibraryPlan> {
        SOURCES
            .into_iter()
            .map(|(label, _)| {
                let directory = self.root.0.join(label);
                LibraryPlan {
                    library: label.into(),
                    source_root: directory.clone(),
                    selected: vec![CompileEntry {
                        directory: directory.clone(),
                        command: format!("cc -c {label}.c"),
                        file: directory.join(format!("{label}.c")),
                    }],
                }
            })
            .collect()
    }
}

#[test]
fn the_real_unit_analysis_reads_whole_tables_so_its_groups_key_on_the_whole_environment() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let corpus = Corpus::new();
    let plans = corpus.plans();
    let groups = group_identities(&plans, 2)
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let source_contracts = super::super::source_contract_configuration().unwrap();
    let contexts = plans
        .iter()
        .map(|plan| prepare_library(plan, &source_contracts, &mut String::new()).unwrap())
        .collect::<Vec<_>>();
    let provider = Mutex::new(());
    let analyse = |group: usize, unit: usize, environment: &ResourceContracts| {
        let session = LookupSession::open(environment);
        let mut output = String::new();
        let products = session
            .scoped(|| {
                audit_unit(
                    &plans[group],
                    &contexts[group],
                    &plans[group].selected[unit],
                    &"f".repeat(40),
                    session.environment(),
                    &[],
                    &source_contracts,
                    &provider,
                    &mut output,
                )
            })
            .unwrap();
        let (stored, _) = store_unit(output, products).unwrap();
        (stored, session.into_handle().record())
    };
    let base = environment(&[KEEP_BORROWED, TOUCH]);

    // Every real unit reads whole tables (the function map, the contract
    // texts its rows' hypothesis context digests, ...), so it seals under
    // the whole environment.
    let (_, record) = analyse(2, 0, &base);
    assert!(record.whole_map_read);
    for accessor in ["functions", "source_assumptions"] {
        assert!(record.whole_map_reads.contains(accessor), "{accessor}");
    }

    // Diff A: a contract file outside the loaded set changes (the
    // environment is byte-identical): every group reused, and the composed
    // run equals the full one.
    let same = measure(
        &groups,
        &analyse,
        &base,
        &environment(&[KEEP_BORROWED, TOUCH]),
    );
    assert_eq!(same.sealed_scopes, ["whole"; 3]);
    assert_eq!(reused(&same), [true, true, true]);

    // Diff B: a declaration only alpha calls changes: every group is a
    // whole-map reader, so every group recomputes (fail-closed), and the
    // composed run equals the full one.
    let changed = measure(
        &groups,
        &analyse,
        &base,
        &environment(&[KEEP_RELEASED, TOUCH]),
    );
    assert_eq!(reused(&changed), [false, false, false]);
}
