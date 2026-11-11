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

use qcore_c_gateway::legacy_c::{C11TranslationUnit, ResourceContracts};
use qcore_c_gateway::null_deref::{NullUnitReport, analyze_null_c11};
use qcore_contract::bounds_contract::BoundsContracts;
use qcore_contract::contract_loader::load_resource_contract_sources;
use std::collections::BTreeMap;

fn result(checked: bool, definition: &str, caller: &str) -> NullUnitReport {
    let marker = if checked {
        "requires resource_body_checked;"
    } else {
        ""
    };
    let contract = format!(
        r#"module contract;
extern fn make() -> ptr<mut, u8> contract {{
 requires resource_contract_v1;
 {marker}
 ensures normal resource_return_owned(nonnull);
}};
"#
    );
    let contracts: ResourceContracts =
        load_resource_contract_sources(&BTreeMap::from([("make.qv".into(), contract)])).unwrap();
    let source =
        format!("struct item {{ int value; }}; struct item *make(void); {definition} {caller}");
    analyze_null_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        &["use".into()],
        &contracts,
        &BoundsContracts::default(),
    )
    .unwrap()
}
const CALLER: &str = "int use(void) { return make()->value; }";
#[test]
fn a_checked_result_without_a_body_is_not_a_null_fact() {
    let report = result(true, "", CALLER);
    assert_ne!(report.functions[0].verdict, "Safe", "{report:#?}");
}
#[test]
fn an_unchecked_nonnull_contract_keeps_its_declared_assumption() {
    let report = result(false, "", CALLER);
    assert_eq!(report.functions[0].verdict, "Safe", "{report:#?}");
}
#[test]
fn a_checked_result_can_still_follow_its_independent_source_summary() {
    let report = result(
        true,
        "struct item item; struct item *make(void) { return &item; }",
        CALLER,
    );
    assert_eq!(report.functions[0].verdict, "Safe", "{report:#?}");
}
#[test]
fn a_checked_but_null_returning_body_is_never_a_nonnull_fact() {
    let report = result(true, "struct item *make(void) { return 0; }", CALLER);
    assert_ne!(report.functions[0].verdict, "Safe", "{report:#?}");
}
#[test]
fn a_caller_can_guard_an_unestablished_checked_result() {
    let report = result(
        true,
        "",
        "int use(void) { struct item *p=make(); if (!p) return 0; return p->value; }",
    );
    assert_eq!(report.functions[0].verdict, "Safe", "{report:#?}");
}
#[test]
fn the_resource_identity_binds_checked_result_metadata() {
    let checked = result(true, "", CALLER);
    let declared = result(false, "", CALLER);
    assert_ne!(
        checked.resource_contracts_sha256,
        declared.resource_contracts_sha256
    );
}
