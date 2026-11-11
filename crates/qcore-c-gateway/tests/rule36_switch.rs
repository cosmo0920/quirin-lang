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

//! Card RULE36-PILOT: rule 36's switch through the public gateway entry
//! point, read from the process environment exactly as an analyzer run reads
//! it (`QCORE_GATEWAY_RULES_OFF`, read once per process, so each switched case
//! runs this test binary again as a child; `child_*` tests do nothing unless
//! `RULE36_SWITCH_CHILD` names them).
//!
//! The fixtures are rule 36's (`derived_result_returned.rs`) over the same
//! synthetic trusted `d_*` contracts. With nothing off the exact borrowed-formal
//! returns certify their `Derived(Exact)` summaries. With rule 36 off each
//! keeps its pre-rule-36 refusal: the shape card W48-BU-CMT-MAP-METRIC-GET's
//! commit `0faab87fa` removed from `mixed_exits_are_refused` ("the formal
//! itself on one path": the wrapper refuses the return expression and its
//! caller sees an uncontracted call), and a bare `return p;` keeps the
//! live-entry route's neutral summary instead of `Derived(Exact)`. Off never
//! certifies what on refuses.

use std::collections::BTreeMap;
use std::process::Command;

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, DerivedExtent, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, ResourceReturnMode, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CHILD: &str = "RULE36_SWITCH_CHILD";

const CONTRACTS: &str = include_str!("fixtures/contracts/derived_result_generic.qv");

const DECLARATIONS: &str = "\
char *d_alloc(__SIZE_TYPE__ n); \
void d_free(void *p); \
void d_consume(void *p); \
__SIZE_TYPE__ d_len(const char *s); \
char *d_chr(const char *s, int c); \
void *d_copy(void *restrict dst, const void *restrict src, __SIZE_TYPE__ n);";

/// The pre-rule-36 refused shape: the formal itself on one path.
const ONE_PATH: &str = "char *wrel(char *d, int n) { if (n) { return d; } \
                        return d_copy(d, \"x\", 1); } \
                        void w(char *s) { char *p; p = wrel(s, 1); d_len(p); }";
/// The bare identity.
const IDENTITY: &str = "char *identity(char *p) { return p; } \
                        void w(void) { char *p=d_alloc(8); if(!p)return; \
                        const char *q=identity(p); if(q) d_len(q); d_free(p); }";

const RETURN_REFUSAL: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";
const UNCONTRACTED: &str = "unsupported:call to uncontracted function";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "derived_result_generic.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the derived-result fixture loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent RULE36-PILOT test: {reason}");
            false
        }
    }
}

fn analyze(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS} {body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

/// The status and sorted diagnostic leaves of `function`.
fn outcome(result: &UnrestrictedLegacyResult, function: &str) -> (String, Vec<String>) {
    let summary = program(result)
        .functions
        .get(function)
        .unwrap_or_else(|| panic!("missing function `{function}`"));
    let mut keys = summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (format!("{:?}", summary.status), keys)
}

/// The derived extent `function`'s certified summary returns, if any.
fn exact_summary(result: &UnrestrictedLegacyResult, function: &str) -> Option<DerivedExtent> {
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        return None;
    };
    certificate
        .summaries
        .iter()
        .find(|record| record.function == function)
        .and_then(|record| match record.contract.result {
            ResourceReturnMode::Derived { extent, .. } => Some(extent),
            _ => None,
        })
}

/// The result mode of `function`'s certified summary, if the unit is `Safe`.
fn certified_result(
    result: &UnrestrictedLegacyResult,
    function: &str,
) -> Option<ResourceReturnMode> {
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        return None;
    };
    certificate
        .summaries
        .iter()
        .find(|record| record.function == function)
        .map(|record| record.contract.result)
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

#[test]
fn with_nothing_off_rule_36_certifies_both_fixtures() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    assert_eq!(gateway_rules::check().map(|off| off.is_empty()), Ok(true));
    let one_path = analyze(ONE_PATH);
    assert_eq!(exact_summary(&one_path, "wrel"), Some(DerivedExtent::Exact));
    assert_eq!(outcome(&one_path, "w").0, "Safe");
    let identity = analyze(IDENTITY);
    assert_eq!(
        exact_summary(&identity, "identity"),
        Some(DerivedExtent::Exact)
    );
    assert_eq!(outcome(&identity, "w").0, "Safe");
}

#[test]
fn switching_rule_36_off_restores_the_pre_rule_36_refusals() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_rule_36_off", "36");
    run_child("child_rule_36_off", " 36 ");
    run_child("child_rule_36_off", "35,36");
}

#[test]
fn child_rule_36_off() {
    if !is_child("child_rule_36_off") {
        return;
    }
    let off = gateway_rules::check().expect("36 is registered");
    assert!(off.ids().contains("36"));
    assert!(gateway_rules::effective_off().contains("36"));
    let one_path = analyze(ONE_PATH);
    let (status, keys) = outcome(&one_path, "wrel");
    assert_ne!(status, "Safe");
    assert_eq!(keys, [RETURN_REFUSAL]);
    let (status, keys) = outcome(&one_path, "w");
    assert_eq!(status, "Unknown");
    assert!(
        keys.iter()
            .any(|key| key.starts_with(UNCONTRACTED) && key.contains("wrel")),
        "{keys:?}"
    );
    // Before rule 36 the bare identity kept the live-entry route's neutral
    // summary (no derived result), and its caller was decided on that.
    let identity = analyze(IDENTITY);
    assert_eq!(exact_summary(&identity, "identity"), None);
    assert_eq!(
        certified_result(&identity, "identity"),
        Some(ResourceReturnMode::None)
    );
}

#[test]
fn switching_another_rule_off_leaves_rule_36_certifying() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_other_rule_off", "35,12f");
}

#[test]
fn child_other_rule_off() {
    if !is_child("child_other_rule_off") {
        return;
    }
    assert!(!gateway_rules::effective_off().contains("36"));
    let identity = analyze(IDENTITY);
    assert_eq!(
        exact_summary(&identity, "identity"),
        Some(DerivedExtent::Exact)
    );
}
