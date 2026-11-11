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

//! Card GATEWAY-RULE-API: the run-time rule switch `QCORE_GATEWAY_RULES_OFF`
//! through the public gateway entry point, read from the process environment
//! exactly as an analyzer run reads it.
//!
//! The switch is read once per process, so each switched case runs this test
//! binary again as a child with the variable set (`child_*` tests do nothing
//! unless `GATEWAY_RULE_SWITCHES_CHILD` names them). The fixtures are rule 35's
//! (`literal_decay_readings.rs`): with nothing off they are `Safe`; with rule
//! 35 off both arms keep their old refusals; with an unknown id in the list
//! the gateway refuses the list and every switchable rule is off, so the
//! fixtures stay refused, never `Safe`.

use std::collections::BTreeMap;
use std::process::Command;

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CHILD: &str = "GATEWAY_RULE_SWITCHES_CHILD";

const CONTRACTS: &str = r#"module contract;

extern fn sl_read(p: ptr<const, u8>, n: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
"#;

const DECLARATIONS: &str = "int sl_read(const void *p, int n);\n";

/// Rule 35, arm 1: a literal read in a condition.
const LITERAL: &str = "int f(int n) {\n if (!*\"lit\") return 1;\n return n;\n}";
/// Rule 35, arm 2: a scalar array member decayed at a borrowed actual.
const DECAY: &str = "struct st { unsigned v; unsigned mem32[4]; unsigned memsize; };\n\
                     int f(const struct st *s) {\n return sl_read(s->mem32, s->memsize);\n}";

const INDIRECT: &str = "indirect memory access is outside the resource projection";
const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "sl.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent GATEWAY-RULE-API test: {reason}");
            false
        }
    }
}

/// `None` for `Safe`, the function's refusal leaves otherwise.
fn analyze(body: &str) -> Option<Vec<String>> {
    let source = format!("{DECLARATIONS}{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "f",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    match result {
        UnrestrictedLegacyResult::Safe(_) => None,
        UnrestrictedLegacyResult::Unsafe(_) => Some(vec!["Unsafe".into()]),
        UnrestrictedLegacyResult::Unknown { partial, .. } => Some(
            partial
                .and_then(|summary| summary.functions.get("f").cloned())
                .map(|function| {
                    function
                        .diagnostics
                        .iter()
                        .map(|diagnostic| match &diagnostic.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                description.clone()
                            }
                            other => format!("{other:?}"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        ),
    }
}

/// Run `test` of this binary as a child with `QCORE_GATEWAY_RULES_OFF=list`.
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

#[test]
fn with_nothing_off_rule_35_admits_both_fixtures() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    assert_eq!(gateway_rules::check().map(|off| off.is_empty()), Ok(true));
    assert!(gateway_rules::effective_off().is_empty());
    assert_eq!(analyze(LITERAL), None);
    assert_eq!(analyze(DECAY), None);
}

#[test]
fn switching_rule_35_off_restores_both_refusals() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_rule_35_off", "35");
    run_child("child_rule_35_off", " 35 ");
}

#[test]
fn child_rule_35_off() {
    if !is_child("child_rule_35_off") {
        return;
    }
    let off = gateway_rules::check().expect("35 is registered");
    assert_eq!(off.ids().iter().collect::<Vec<_>>(), ["35"]);
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["35"]
    );
    let literal = analyze(LITERAL).expect("refused with 35 off");
    assert!(literal.iter().any(|leaf| leaf == BRANCH), "{literal:?}");
    let decay = analyze(DECAY).expect("refused with 35 off");
    assert!(decay.iter().any(|leaf| leaf == INDIRECT), "{decay:?}");
}

#[test]
fn switching_another_rule_off_leaves_rule_35_admitting() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_other_rule_off", "12f,10s");
}

#[test]
fn child_other_rule_off() {
    if !is_child("child_other_rule_off") {
        return;
    }
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["10s", "12f"]
    );
    assert_eq!(analyze(LITERAL), None);
    assert_eq!(analyze(DECAY), None);
}

#[test]
fn an_unknown_id_is_refused_and_every_rule_is_off() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_unknown_id", "35,no-such-rule");
}

#[test]
fn child_unknown_id() {
    if !is_child("child_unknown_id") {
        return;
    }
    let refusal = gateway_rules::check().expect_err("an unknown id is refused");
    assert!(refusal.to_string().contains("no-such-rule"), "{refusal}");
    let off = gateway_rules::effective_off();
    assert!(off.contains("35") && off.contains("10s") && off.contains("14c"));
    assert!(
        analyze(LITERAL).is_some(),
        "never Safe under a refused list"
    );
    assert!(analyze(DECAY).is_some(), "never Safe under a refused list");
}
