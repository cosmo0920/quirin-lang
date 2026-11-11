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

//! Card CALLEE-RETENTION-CLASS1: the retention clauses of resource
//! vocabulary version 1 (contract specification §C04.14, grammar G05), read
//! total-or-reject, and the withheld declaration's journey through the
//! environment (pin withholding, source recheck).

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use qcore_schemas::contract_revision::PinMismatchReason;
use qcore_schemas::retained_by::{Retention, RetentionHolder};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "retained.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

const KEEP: &str = "
extern fn keep(state: ptr<mut, u8>, src: ptr<const, u8>, n: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(state);
    ensures normal resource_borrowed(src);
    ensures normal resource_borrowed(n);
    ensures normal resource_retained_by(src, state);
    ensures normal resource_holder_reset(state);
    ensures normal resource_return_none();
};

extern fn publish(value: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(value);
    ensures normal resource_retained_by(value, global);
    ensures normal resource_return_none();
};
";

#[test]
fn a_retaining_declaration_loads_and_is_withheld_from_the_function_map() {
    let contracts = load(KEEP).expect("loads");
    assert!(contracts.contract("keep").is_none());
    assert!(contracts.contract("publish").is_none());
    let keep = contracts.retaining_contract("keep").expect("retaining");
    assert_eq!(
        keep.retentions,
        [Retention {
            retained: 1,
            holder: RetentionHolder::Parameter { index: 0 },
            guard: None,
        }]
    );
    assert_eq!(keep.resets, [0]);
    assert!(
        keep.contract
            .parameters
            .iter()
            .all(|mode| *mode == ResourceParameterMode::Borrowed)
    );
    let publish = contracts.retaining_contract("publish").expect("retaining");
    assert_eq!(publish.retentions[0].holder, RetentionHolder::Global);
    check_source_assumptions(&contracts).expect("the retained texts reload to the same tables");
}

#[test]
fn every_malformed_retention_rejects_the_whole_module() {
    let cases = [
        // The retained pointer must stay the caller's: borrowed.
        (
            "ensures normal resource_owned(src, nonnull); ensures normal resource_borrowed(state);",
            "resource_retained_by(src, state)",
        ),
        // A pointer does not retain itself.
        (
            "ensures normal resource_borrowed(src); ensures normal resource_borrowed(state);",
            "resource_retained_by(src, src)",
        ),
        // A scalar is not a holder.
        (
            "ensures normal resource_borrowed(src); ensures normal resource_borrowed(state);",
            "resource_retained_by(src, n)",
        ),
        // An unknown holder.
        (
            "ensures normal resource_borrowed(src); ensures normal resource_borrowed(state);",
            "resource_retained_by(src, nowhere)",
        ),
    ];
    for (modes, clause) in cases {
        let text = format!(
            "extern fn keep(state: ptr<mut, u8>, src: ptr<const, u8>, n: i32) -> i32
contract {{
    requires resource_contract_v1;
    {modes}
    ensures normal resource_borrowed(n);
    ensures normal {clause};
    ensures normal resource_return_none();
}};"
        );
        assert!(load(&text).is_err(), "{clause} with {modes}");
    }
    // A second retention of one pointer.
    assert!(
        load(
            "extern fn keep(state: ptr<mut, u8>, src: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(state);
    ensures normal resource_borrowed(src);
    ensures normal resource_retained_by(src, state);
    ensures normal resource_retained_by(src, global);
    ensures normal resource_return_none();
};"
        )
        .is_err()
    );
    // A parameter named `global` makes the word ambiguous.
    assert!(
        load(
            "extern fn keep(global: ptr<mut, u8>, src: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(global);
    ensures normal resource_borrowed(src);
    ensures normal resource_retained_by(src, global);
    ensures normal resource_return_none();
};"
        )
        .is_err()
    );
    // Never beside a return-code disposition.
    assert!(
        load(
            "extern fn keep(state: ptr<mut, u8>, src: ptr<const, u8>, item: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(state);
    ensures normal resource_borrowed(src);
    ensures normal resource_retained_by(src, state);
    ensures normal resource_on_return(eq, 0, resource_consumed(item, nullable));
    ensures normal resource_on_return(ne, 0, resource_owned(item, nullable));
    ensures normal resource_return_none();
};"
        )
        .is_err()
    );
}

#[test]
fn a_withheld_retaining_declaration_returns_to_its_own_table() {
    let mut contracts = load(KEEP).expect("loads");
    contracts
        .withhold_for_pin_mismatch(
            "keep",
            "retained.qv".to_owned(),
            PinMismatchReason::NoLock,
            None,
            "test".to_owned(),
        )
        .expect("a pinned retaining declaration is withheld");
    assert!(contracts.retaining_contract("keep").is_none());
    assert!(contracts.contract("keep").is_none());
    check_source_assumptions(&contracts).expect("a withheld declaration is still declared");
    let resolved = contracts.with_human_holds_in_force().expect("one hold");
    assert!(resolved.retaining_contract("keep").is_some());
    assert!(
        resolved.contract("keep").is_none(),
        "a retention is never read as a plain borrow"
    );
}
