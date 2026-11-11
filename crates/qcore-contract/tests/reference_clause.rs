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

//! Card REFCOUNT-RESOURCE-KIND: the reference clauses (contract
//! specification §C04.17) of resource vocabulary version 1, read
//! total-or-reject (grammar G05).

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::reference_count::{ReferenceClause, ReferenceOperation};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "references.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

const REFERENCES: &str = "
extern fn keep(func: ptr<const, u8>, line: i32, p: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(func);
    ensures normal resource_borrowed(line);
    ensures normal resource_borrowed(p);
    ensures normal resource_reference_kept(p);
    ensures normal resource_return_alias(p, nonnull);
};

extern fn keep_void(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_reference_kept(p);
    ensures normal resource_return_none();
};

extern fn put(p: ptr<mut, u8>, disable: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_borrowed(disable);
    ensures normal resource_reference_dropped(p);
    ensures normal resource_return_none();
};
";

#[test]
fn reference_clauses_load_withheld() {
    let contracts = load(REFERENCES).expect("loads");
    for name in ["keep", "keep_void", "put"] {
        assert!(contracts.contract(name).is_none(), "{name} is withheld");
    }
    let keep = contracts.retaining_contract("keep").expect("withheld");
    assert_eq!(
        keep.references,
        [ReferenceClause {
            parameter: 2,
            operation: ReferenceOperation::Kept
        }]
    );
    assert!(!keep.retains());
    assert!(!keep.references_withdrawn);
    let put = contracts.retaining_contract("put").expect("withheld");
    assert_eq!(
        put.references,
        [ReferenceClause {
            parameter: 0,
            operation: ReferenceOperation::Dropped
        }]
    );
    check_source_assumptions(&contracts).expect("the retained texts reload");
}

fn declaration(parameters: &str, clauses: &str) -> String {
    format!(
        "extern fn k({parameters}) -> unit\ncontract {{\n    requires resource_contract_v1;\n{clauses}}};\n"
    )
}

#[test]
fn every_misplaced_reference_clause_rejects_the_module() {
    let cases = [
        // An owned parameter.
        (
            "p: ptr<mut, u8>",
            "    ensures normal resource_owned(p, nonnull);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_return_none();\n",
        ),
        // A scalar.
        (
            "p: i32",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_dropped(p);\n    ensures normal resource_return_none();\n",
        ),
        // A pointer to a pointer.
        (
            "p: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_return_none();\n",
        ),
        // Two reference clauses.
        (
            "p: ptr<mut, u8>, q: ptr<mut, u8>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(q);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_reference_dropped(q);\n    ensures normal resource_return_none();\n",
        ),
        // An unknown parameter.
        (
            "p: ptr<mut, u8>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_kept(q);\n    ensures normal resource_return_none();\n",
        ),
        // A retained parameter.
        (
            "p: ptr<mut, u8>, h: ptr<mut, u8>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(h);\n    ensures normal resource_retained_by(p, h);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_return_none();\n",
        ),
        // Beside a variadic tail.
        (
            "p: ptr<mut, u8>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_dropped(p);\n    ensures normal resource_variadic_data_tail();\n    ensures normal resource_return_none();\n",
        ),
    ];
    for (parameters, clauses) in cases {
        let text = declaration(parameters, clauses);
        assert!(load(&text).is_err(), "{text}");
    }
}

#[test]
fn a_reference_result_is_the_alias_or_none() {
    // A drop that returns its argument.
    let drop_alias = "extern fn k(p: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_dropped(p);\n    ensures normal resource_return_alias(p, nonnull);\n};\n";
    assert!(load(drop_alias).is_err());
    // A keep returning a nullable alias, or an alias of another parameter.
    let nullable = "extern fn k(p: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_return_alias(p, nullable);\n};\n";
    assert!(load(nullable).is_err());
    let other = "extern fn k(p: ptr<mut, u8>, q: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(q);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_return_alias(q, nonnull);\n};\n";
    assert!(load(other).is_err());
    // A keep returning fresh custody.
    let owned = "extern fn k(p: ptr<mut, u8>) -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_return_owned(nonnull);\n};\n";
    assert!(load(owned).is_err());
}

#[test]
fn the_reference_kind_is_spelled_only_on_an_owned_result() {
    // A parameter position never spells it.
    let parameter = declaration(
        "p: ptr<mut, u8>",
        "    ensures normal resource_released(p, nonnull);\n    ensures normal resource_kind(p, reference);\n    ensures normal resource_return_none();\n",
    );
    let error = load(&parameter).expect_err("rejected");
    assert!(error.contains("resource_reference_kept"), "{error}");
    // A reference-returning call: an owned result of the reference kind.
    let result = "extern fn k() -> ptr<mut, u8>\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_return_owned(nullable);\n    ensures normal resource_return_kind(reference);\n};\n";
    let contracts = load(result).expect("an owned reference result loads");
    let contract = contracts.contract("k").expect("in the function map");
    assert_eq!(
        contract.result_kind,
        qcore_schemas::contract::ResourceKind::Reference
    );
    // On any other result it rejects the module.
    let none = "extern fn k() -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_return_none();\n    ensures normal resource_return_kind(reference);\n};\n";
    assert!(load(none).is_err());
}
