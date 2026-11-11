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

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CSourceInput, LegacyFunctionScope, ResourceContracts,
    UnrestrictedLegacyResult, analyze_prepared_translation_unit, analyze_unrestricted_c11,
    prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = include_str!("fixtures/contracts/q_base.qv");

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping prepared-TU tests: {reason}");
            false
        }
    }
}

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "q_base.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .unwrap()
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    }
}

fn verdict(result: &UnrestrictedLegacyResult) -> &'static str {
    match result {
        UnrestrictedLegacyResult::Safe(_) => "Safe",
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    }
}

#[test]
fn prepared_analysis_preserves_safe_unsafe_and_unknown_results() {
    if !clang_available() {
        return;
    }
    let cases = [
        ("Safe", "void subject(void) { return; }\n"),
        (
            "Unsafe",
            "void *q_alloc(void); void subject(void) { void *p = q_alloc(); }\n",
        ),
        (
            "Unknown",
            "int opaque(void); void subject(void) { if (opaque()) return; }\n",
        ),
    ];
    for (expected, source) in cases {
        let unit = unit(source);
        let baseline = analyze_unrestricted_c11(&unit, "subject", &contracts()).unwrap();
        let prepared = prepare_translation_unit(&unit);
        let shared = analyze_prepared_translation_unit(&prepared, "subject", &contracts()).unwrap();
        assert_eq!(verdict(&baseline), expected, "baseline case {expected}");
        assert_eq!(
            shared, baseline,
            "prepared case {expected} moved its result"
        );
        assert_eq!(prepared.metrics().clang_invocations(), 3);
    }
}

#[test]
fn one_preparation_shares_exact_forms_inventory_and_macro_header_coordinates() {
    if !clang_available() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "qcore-prepared-tu-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("defs.h"),
        "#define MAKE(NAME) void NAME(void) { return; }\n",
    )
    .unwrap();
    let source = "#include \"defs.h\"\nMAKE(subject)\n";
    let unit = C11TranslationUnit {
        source,
        include_paths: vec![root.to_string_lossy().into_owned()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: CSourceInput::HistoricalStdin,
        ..C11TranslationUnit::default()
    };
    let prepared = prepare_translation_unit(&unit);
    assert_eq!(prepared.inventory().definitions(), ["subject"]);
    assert!(prepared.line_marked_source().is_some());
    assert!(
        prepared
            .preprocessed_source()
            .unwrap()
            .contains("void subject(void)")
    );
    assert_ne!(
        prepared.preprocessed_source().unwrap(),
        prepared.line_marked_source().unwrap(),
        "the marked form must be retained separately, never reconstructed"
    );
    let line = prepared
        .preprocessed_source()
        .unwrap()
        .split('\n')
        .position(|line| line.contains("void subject(void)"))
        .unwrap()
        + 1;
    let origin = prepared.locations().original(line).unwrap();
    assert_eq!(origin.depth, 0, "macro expansion is at its invocation site");
    assert_eq!(origin.line, 2);
    assert_eq!(prepared.metrics().clang_invocations(), 3);
    let _ = std::fs::remove_dir_all(root);
}
