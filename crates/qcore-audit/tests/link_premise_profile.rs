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

//! Card **CG22-CALLBACK-CANDIDATE-LINK-PREMISE**: the `twelve-home` evaluation
//! profile states rule 4c's link premise for the eight-home homes plus four
//! external comparators. No analyzer code changed; these tests pin the
//! profile's stated map and show, through rule 4c's own selection
//! (`LinkMap::select`) over that committed map, that a comparator handed to a
//! synchronous invocation is in force only when the map names it with the
//! home unit that exports it:
//!
//! * **accepting:** `rd_strcmp3`, listed with home `librdkafka/rdstring.c` and
//!   exported from that unit, decides the invocation; the caller is `Safe` and
//!   its certificate replays the home;
//! * **refusing:** a comparator the map does not name (`rd_kafka_group_member_cmp`,
//!   exported Safe from its own unit) is never imported, and its invocation
//!   keeps `callback-uncontracted`; a listed comparator exported from a unit
//!   other than its stated home is not imported either.
//!
//! The comparator bodies and the `c_each` callee are synthetic (the
//! gateway's `callbacks.qv` test assumptions), not the librdkafka sources.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qcore_audit::cross_unit_links::LinkMap;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionScope,
    ResourceContracts, UnrestrictedLegacyResult, analyze_prepared_translation_unit_with_cross_unit,
    check_legacy_safety_certificate, export_cross_unit_homes, prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The four homes the profile adds to the eight-home premise.
const ADDED: [(&str, &str); 4] = [
    (
        "rd_kafka_metadata_broker_cmp",
        "librdkafka/rdkafka_metadata.c",
    ),
    (
        "rd_kafka_metadata_broker_internal_cmp",
        "librdkafka/rdkafka_metadata.c",
    ),
    ("rd_strcmp3", "librdkafka/rdstring.c"),
    (
        "rd_kafka_assignor_topic_cmp",
        "librdkafka/rdkafka_assignor.c",
    ),
];

/// The committed profiles' stated link maps, byte for byte.
const EIGHT_HOME_MAP: &str = include_str!("../../../contract/evaluations/eight-home/link-map.json");
const TWELVE_HOME_MAP: &str =
    include_str!("../../../contract/evaluations/twelve-home/link-map.json");

/// Rule 4c's loader over a profile's committed map (written to a private
/// file, as the route reads it from `QCORE_CROSS_UNIT_LINK_MAP`).
fn profile_map(profile: &str) -> LinkMap {
    let bytes = match profile {
        "eight-home" => EIGHT_HOME_MAP,
        "twelve-home" => TWELVE_HOME_MAP,
        other => panic!("no committed map for {other}"),
    };
    let path: PathBuf = std::env::temp_dir().join(format!(
        "qcore-link-premise-{profile}-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&path, bytes).expect("a private copy of the map");
    let map = LinkMap::load(Path::new(&path)).expect("the profile's stated link map loads");
    std::fs::remove_file(&path).expect("cleanup");
    map
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!(
                "skipping Clang-dependent CG22-CALLBACK-CANDIDATE-LINK-PREMISE test: {reason}"
            );
            false
        }
    }
}

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "callbacks.qv".to_owned(),
        include_str!("../../qcore-c-gateway/tests/fixtures/contracts/callbacks.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

const CALLEE: &str =
    "int c_each(void *reader, int (*cb)(void *, unsigned long, void *), void *context);\n";

/// An identity comparator (reads its formals only) named `name`.
fn comparator(name: &str) -> String {
    format!(
        "int {name}(void *reader, unsigned long index, void *context) \
         {{ return *(int *)context > (int)index; }}\n"
    )
}

#[test]
fn the_profile_map_is_the_eight_home_premise_plus_four_comparator_homes() {
    let eight = profile_map("eight-home");
    let twelve = profile_map("twelve-home");
    assert_eq!(twelve.premise, eight.premise);
    let mut expected = eight.requested();
    expected.extend(ADDED.iter().map(|(name, _)| (*name).to_owned()));
    assert_eq!(twelve.requested(), expected);
    assert_eq!(twelve.requested().len(), 12);
    for name in eight.requested() {
        assert_eq!(twelve.home_for(&name), eight.home_for(&name), "{name}");
    }
    for (name, home) in ADDED {
        assert_eq!(
            eight.home_for(name),
            None,
            "{name} is not an eight-home home"
        );
        assert_eq!(twelve.home_for(name), Some(home), "{name}");
    }
    assert_ne!(twelve.digest, eight.digest);
}

struct Outcome {
    safe: bool,
    refusals: Vec<String>,
}

/// Rule 4c's selection under the committed twelve-home map over `exports`
/// (unit, comparator name), then the caller handing `handed` to `c_each`.
fn run(exports: &[(&str, &str)], handed: &str) -> (BTreeSet<String>, Outcome) {
    let map = profile_map("twelve-home");
    let contracts = environment();
    let mut found = Vec::new();
    let mut definitions = Vec::new();
    for (unit, name) in exports {
        let home = prepare_translation_unit(&C11TranslationUnit {
            source: &comparator(name),
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        // Exported as if requested, so the refusal below is the map's alone.
        let wanted = BTreeSet::from([(*name).to_owned()]);
        let export = export_cross_unit_homes(&home, &contracts, &wanted, &map.digest)
            .expect("home export")
            .pop()
            .expect("a Safe home with a validated summary");
        found.push(((*unit).to_owned(), export));
        definitions.push(((*unit).to_owned(), (*name).to_owned()));
    }
    let (selected, homes) = map.select(found, definitions, &contracts);
    let imported = selected.inferred_imports().keys().cloned().collect();
    let caller_source = format!(
        "{CALLEE}int {handed}(void *reader, unsigned long index, void *context);\n\
         int user(void *reader, int *context) {{ return c_each(reader, {handed}, context); }}\n"
    );
    let caller = prepare_translation_unit(&C11TranslationUnit {
        source: &caller_source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..Default::default()
    });
    let result =
        analyze_prepared_translation_unit_with_cross_unit(&caller, "user", &selected, &homes)
            .expect("caller analysis");
    let outcome = match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).expect("home and caller replay");
            Outcome {
                safe: true,
                refusals: Vec::new(),
            }
        }
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => {
            let function = &summary.functions["user"];
            assert_ne!(function.status, AnalysisStatus::Safe);
            Outcome {
                safe: false,
                refusals: function
                    .diagnostics
                    .iter()
                    .filter_map(|diagnostic| match &diagnostic.kind {
                        LegacyDiagnosticKind::Unsupported { description } => {
                            Some(description.clone())
                        }
                        _ => None,
                    })
                    .collect(),
            }
        }
        other => panic!("expected an analysable caller: {other:?}"),
    };
    (imported, outcome)
}

const EXPORTS: [(&str, &str); 2] = [
    ("librdkafka/rdstring.c", "rd_strcmp3"),
    ("librdkafka/rdkafka_cgrp.c", "rd_kafka_group_member_cmp"),
];

#[test]
fn a_comparator_listed_with_its_home_is_in_force() {
    if !clang_available() {
        return;
    }
    let (imported, outcome) = run(&EXPORTS, "rd_strcmp3");
    assert_eq!(imported, BTreeSet::from(["rd_strcmp3".to_owned()]));
    assert!(outcome.safe, "{:?}", outcome.refusals);
}

#[test]
fn a_comparator_outside_the_stated_map_stays_refused() {
    if !clang_available() {
        return;
    }
    let (imported, outcome) = run(&EXPORTS, "rd_kafka_group_member_cmp");
    assert!(!imported.contains("rd_kafka_group_member_cmp"));
    assert!(!outcome.safe);
    assert!(
        outcome
            .refusals
            .iter()
            .any(|refusal| refusal.contains("callback-uncontracted")),
        "{:?}",
        outcome.refusals
    );
}

#[test]
fn a_listed_comparator_exported_from_another_unit_stays_refused() {
    if !clang_available() {
        return;
    }
    let (imported, outcome) = run(
        &[("librdkafka/rdkafka_metadata.c", "rd_strcmp3")],
        "rd_strcmp3",
    );
    assert!(imported.is_empty());
    assert!(!outcome.safe);
    assert!(
        outcome
            .refusals
            .iter()
            .any(|refusal| refusal.contains("callback-uncontracted")),
        "{:?}",
        outcome.refusals
    );
}
