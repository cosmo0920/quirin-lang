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

//! Card RULE33-PILOT: rule 33's run-time switch through the public gateway
//! entry point, read from the process environment as an analyzer run reads
//! it. The switch is read once per process, so each switched case runs this
//! binary again as a child with `QCORE_GATEWAY_RULES_OFF` set (`child_*` tests
//! do nothing unless `RULE33_SWITCH_CHILD` names them).
//!
//! Fixtures: the retained cJSON wrapper frames of
//! `detach_wrapper_composition.rs` under the production cJSON module, and a
//! decimal-constant caller. With nothing off both array wrappers and the
//! caller are `Safe`; with 33 off they keep their pre-rule-33 refusals (the
//! nested lookup in `F`'s return, the nested `F` call in `G`, an uncontracted
//! call in the caller), and no row of the unit is `Safe` that is not `Safe`
//! with rule 33 on, so off never widens.

use std::collections::BTreeSet;
use std::process::Command;

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts,
};

const CHILD: &str = "RULE33_SWITCH_CHILD";

const FRAME: &str = include_str!("fixtures/detach-wrapper-composition/cjson_wrapper_frame.h");
const PRODUCTION_MODULE: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const HELPERS: &str = r#"module contract;
extern fn dt_alloc() -> ptr<mut, u8> contract {
 requires resource_contract_v1; ensures normal resource_return_owned(nonnull);
};
extern fn dt_free(p: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none();
};
extern fn dt_store(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> unit contract {
 requires resource_contract_v1; ensures normal resource_owned(parent, nonnull);
 ensures normal resource_stored(item, parent, nonnull); ensures normal resource_return_none();
};
"#;
const DECLARATIONS: &str = "struct cJSON *dt_alloc(void); void dt_free(struct cJSON*); void dt_store(struct cJSON*,struct cJSON*);";
const CALLER: &str = "struct cJSON *p=dt_alloc(); struct cJSON *i=dt_alloc(); dt_store(p,i); p->child=i; cJSON_DeleteItemFromArray(p,0); dt_free(p);";
const ARRAY_WRAPPERS: [&str; 2] = ["cJSON_DetachItemFromArray", "cJSON_DeleteItemFromArray"];

/// The rows rule 33 makes `Safe` in this unit.
const ADMITTED_BY_RULE_33: [&str; 3] = [
    "cJSON_DetachItemFromArray",
    "cJSON_DeleteItemFromArray",
    "subject",
];

/// Every row that is `Safe` in this unit with rule 33 on: rule 33's three and
/// the lookups, which no rule-33 reading decides.
const SAFE_WITH_RULE_33: [&str; 7] = [
    "cJSON_DetachItemFromArray",
    "cJSON_DeleteItemFromArray",
    "subject",
    "cJSON_GetArrayItem",
    "cJSON_GetObjectItem",
    "cJSON_GetObjectItemCaseSensitive",
    "get_array_item",
];

/// The pre-rule-33 refusals (rule 33's report, BASE column): `F`'s return
/// nests the lookup, `G` nests `F`, and the caller calls an uncontracted
/// function.
const PRE_RULE_33_REFUSALS: [(&str, &str); 3] = [
    ("cJSON_DetachItemFromArray", "get_array_item"),
    ("cJSON_DeleteItemFromArray", "cJSON_DetachItemFromArray"),
    ("subject", "cJSON_DeleteItemFromArray"),
];

fn environment() -> ResourceContracts {
    qcore_contract::contract_loader::load_resource_contract_sources(
        &std::collections::BTreeMap::from([
            ("cjson.qv".to_owned(), PRODUCTION_MODULE.to_owned()),
            ("helpers.qv".to_owned(), HELPERS.to_owned()),
        ]),
    )
    .unwrap()
}

fn program() -> LegacyProgramSummary {
    let source = format!("{FRAME}\n{DECLARATIONS}\nvoid subject(void) {{ {CALLER} }}");
    qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit(&source, "subject", &environment())
        .unwrap()
}

fn leaves(function: &LegacyFunctionSummary) -> Vec<String> {
    function
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn safe_rows(program: &LegacyProgramSummary) -> BTreeSet<&str> {
    program
        .functions
        .iter()
        .filter(|(_, row)| row.status == AnalysisStatus::Safe)
        .map(|(name, _)| name.as_str())
        .collect()
}

fn run_child(test: &str, list: &str) {
    let status = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env(gateway_rules::RULES_OFF_VARIABLE, list)
        .status()
        .expect("the child test runs");
    assert!(status.success(), "{test} with {list:?}: {status}");
}

fn is_child(test: &str) -> bool {
    std::env::var(CHILD).is_ok_and(|named| named == test)
}

fn assert_rule_33_admits(program: &LegacyProgramSummary) {
    for function in ADMITTED_BY_RULE_33 {
        let row = &program.functions[function];
        assert_eq!(row.status, AnalysisStatus::Safe, "{function}: {row:#?}");
        assert!(row.diagnostics.is_empty(), "{function}");
    }
    // The rows run the frame's own constituent calls.
    let callees =
        qcore_c_gateway::contract_dependencies::cfg_callees(&program.functions["subject"].cfg);
    for callee in [
        "get_array_item",
        "cJSON_DetachItemViaPointer",
        "cJSON_Delete",
    ] {
        assert!(callees.contains(callee), "{callees:?}");
    }
}

#[test]
fn with_nothing_off_rule_33_admits_the_frames_and_the_caller() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    assert!(gateway_rules::effective_off().is_empty());
    let program = program();
    assert_rule_33_admits(&program);
    assert_eq!(
        safe_rows(&program),
        BTreeSet::from(SAFE_WITH_RULE_33),
        "the unit's Safe rows with rule 33 on"
    );
}

#[test]
fn switching_rule_33_off_keeps_the_refusals() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    run_child("child_rule_33_off", "33");
}

#[test]
fn child_rule_33_off() {
    if !is_child("child_rule_33_off") {
        return;
    }
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["33"]
    );
    let program = program();
    for (function, callee) in PRE_RULE_33_REFUSALS {
        let row = &program.functions[function];
        assert_eq!(row.status, AnalysisStatus::Unknown, "{function}: {row:#?}");
        let leaves = leaves(row);
        assert!(
            leaves.iter().any(|leaf| leaf.contains(callee)),
            "{function} keeps its pre-rule-33 refusal naming `{callee}`: {leaves:?}"
        );
    }
    // Nothing is expanded: the caller's CFG calls the frame itself.
    let callees =
        qcore_c_gateway::contract_dependencies::cfg_callees(&program.functions["subject"].cfg);
    assert!(
        !callees.contains("cJSON_DetachItemViaPointer"),
        "{callees:?}"
    );
    // Off never widens: every row `Safe` with rule 33 off is `Safe` with it on.
    let off = safe_rows(&program);
    assert!(
        off.is_subset(&BTreeSet::from(SAFE_WITH_RULE_33)),
        "rows Safe only with rule 33 off: {off:?}"
    );
    let on = BTreeSet::from(SAFE_WITH_RULE_33);
    let moved = on.difference(&off).copied().collect::<BTreeSet<_>>();
    assert_eq!(
        moved,
        BTreeSet::from(ADMITTED_BY_RULE_33),
        "exactly rule 33's rows leave Safe"
    );
}

#[test]
fn switching_another_rule_off_leaves_rule_33_admitting() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    run_child("child_other_rule_off", "35");
}

#[test]
fn child_other_rule_off() {
    if !is_child("child_other_rule_off") {
        return;
    }
    assert_rule_33_admits(&program());
    for wrapper in ARRAY_WRAPPERS {
        assert!(program().functions.contains_key(wrapper));
    }
}
