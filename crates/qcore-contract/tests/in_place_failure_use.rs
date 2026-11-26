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

//! Card **CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE**, loader side (contract
//! spec §C04.6.1, gateway rule 74n): `resource_kind_failure_use(p, KIND)`
//! states that the in-place use at `p` is defined on an object whose
//! initialiser may have failed.
//!
//! It loads only on a by-address `resource_borrowed` position that carries
//! `resource_kind(p, KIND)` of the same in-place kind; every other placement
//! rejects the **whole** module. A contract without it serializes byte for
//! byte as before.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::contract_loader::load_resource_contract_sources;

const REGEX: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/regex.qv");

fn module(signature: &str, clauses: &str) -> String {
    format!(
        "module contract;\nextern fn k{signature}\ncontract {{\n    \
         requires resource_contract_v1;\n{clauses}}};\n"
    )
}

fn load(source: &str) -> Result<qcore_schemas::contract::ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("k.qv".to_owned(), source.to_owned())]))
}

const USE: &str = "(e: i32, preg: ptr<const, u8>) -> usize";

#[test]
fn the_production_regerror_states_its_failure_use_and_regexec_none() {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "libc/regex.qv".to_owned(),
        REGEX.to_owned(),
    )]))
    .expect("the libc regex module loads");
    let regerror = contracts.contract("regerror").expect("declared");
    assert_eq!(regerror.failure_uses, BTreeSet::from([1]));
    assert!(regerror.is_failure_use(1) && !regerror.is_failure_use(0));
    for name in ["regexec", "regcomp", "regfree"] {
        let contract = contracts.contract(name).expect("declared");
        assert!(contract.failure_uses.is_empty(), "{name}");
        let json = serde_json::to_string(contract).expect("serializes");
        assert!(!json.contains("failure_uses"), "{name}: {json}");
        assert!(!format!("{contract:?}").contains("failure_uses"), "{name}");
    }
    assert!(format!("{regerror:?}").contains("failure_uses: {1}"));
    // No derived summary states it, so a body is never compared with it.
    assert!(regerror.custody_projection().failure_uses.is_empty());
}

#[test]
fn a_failure_use_loads_on_a_kinded_by_address_use() {
    for ty in ["ptr<const, u8>", "ptr<mut, u8>"] {
        let loaded = load(&module(
            &format!("(e: i32, preg: {ty}) -> usize"),
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(preg);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, regex);\n    \
             ensures normal resource_return_none();\n",
        ))
        .unwrap_or_else(|error| panic!("{ty}: {error}"));
        assert_eq!(
            loaded.contract("k").expect("declared").failure_uses,
            BTreeSet::from([1])
        );
    }
}

#[test]
fn every_other_placement_rejects_the_module() {
    let cases: [(&str, &str, &str); 8] = [
        (
            "no kind on the position",
            USE,
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(preg);\n    \
             ensures normal resource_kind_failure_use(preg, regex);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "another in-place kind",
            USE,
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(preg);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, thread_key);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "a kind that is not in place",
            "(e: i32, s: ptr<mut, u8>) -> usize",
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_released(s, nonnull);\n    \
             ensures normal resource_kind(s, stream);\n    \
             ensures normal resource_kind_failure_use(s, stream);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "the initialiser",
            "(preg: ptr<mut, u8>, p: ptr<const, u8>) -> i32",
            "    ensures normal resource_out(preg, nullable);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, regex);\n    \
             ensures normal resource_borrowed(p);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "the finaliser",
            "(preg: ptr<mut, u8>) -> unit",
            "    ensures normal resource_released(preg, nonnull);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, regex);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "a by-value use",
            "(key: u32) -> ptr<mut, u8>",
            "    ensures normal resource_borrowed(key);\n    \
             ensures normal resource_kind(key, thread_key);\n    \
             ensures normal resource_kind_failure_use(key, thread_key);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "a duplicate",
            USE,
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(preg);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, regex);\n    \
             ensures normal resource_return_none();\n",
        ),
        (
            "the spelled-out heap",
            USE,
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(preg);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(preg, heap);\n    \
             ensures normal resource_return_none();\n",
        ),
    ];
    for (case, signature, clauses) in cases {
        let error = load(&module(signature, clauses))
            .err()
            .unwrap_or_else(|| panic!("{case} loaded"));
        let expected = if case == "the spelled-out heap" {
            "heap kind"
        } else {
            "failure"
        };
        assert!(error.contains(expected), "{case}: {error}");
    }
    // A parameter the declaration does not name.
    assert!(
        load(&module(
            USE,
            "    ensures normal resource_borrowed(e);\n    \
             ensures normal resource_borrowed(preg);\n    \
             ensures normal resource_kind(preg, regex);\n    \
             ensures normal resource_kind_failure_use(other, regex);\n    \
             ensures normal resource_return_none();\n",
        ))
        .is_err()
    );
}
