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

//! Card **CG25-RESOURCE-KINDS**: the shared vectors binding
//! `proof/QCore/ResourceKinds.lean` to the Rust kind table and to the libc
//! provider's declarations.
//!
//! * `kindVectors` = `ResourceKind::ALL` read through `spelling`, `carrier`,
//!   `is_integer` and `is_in_place`, row for row.
//! * `protocolVectors` = the acquiring and releasing positions of each new
//!   kind, as `providers/quirin-provider-libc` declares them.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{
    ResourceCarrier, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};

const LEAN: &str = include_str!("../../../proof/QCore/ResourceKinds.lean");
const DLFCN: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/dlfcn.qv");
const REGEX: &str = include_str!("../../../providers/quirin-provider-libc/contracts/libc/regex.qv");

/// The quoted and boolean fields of every row of the Lean list `header`
/// opens, one `Vec` per row.
fn lean_rows(header: &str) -> Vec<Vec<String>> {
    let start = LEAN.find(header).expect("the Lean list exists") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the Lean list closes");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            line.trim_start_matches('(')
                .trim_end_matches(',')
                .trim_end_matches(')')
                .split(',')
                .map(|field| field.trim().trim_matches('"').to_owned())
                .collect()
        })
        .collect()
}

fn carrier_word(carrier: ResourceCarrier) -> &'static str {
    match carrier {
        ResourceCarrier::Pointer => "pointer",
        ResourceCarrier::Integer => "integer",
        ResourceCarrier::Storage => "storage",
    }
}

#[test]
fn cg25_lean_kind_vectors_match_the_rust_table() {
    let rows = lean_rows("def kindVectors : List (String × String × Bool × Bool) := [");
    let rust = ResourceKind::ALL
        .iter()
        .map(|kind| {
            vec![
                kind.spelling().to_owned(),
                carrier_word(kind.carrier()).to_owned(),
                kind.is_integer().to_string(),
                kind.is_in_place().to_string(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn cg25_lean_protocol_vectors_match_the_libc_declarations() {
    let rows =
        lean_rows("def protocolVectors : List (String × String × String × String × String) := [");
    assert_eq!(rows.len(), 2);
    let contracts = load_resource_contract_sources(&BTreeMap::from([
        ("libc/dlfcn.qv".to_owned(), DLFCN.to_owned()),
        ("libc/regex.qv".to_owned(), REGEX.to_owned()),
    ]))
    .expect("the libc modules load");
    for row in rows {
        let [kind, acquire, acquire_position, release, release_position] = row.as_slice() else {
            panic!("malformed row {row:?}");
        };
        let kind = ResourceKind::from_spelling(kind).expect("a declared kind");
        let acquiring = contracts.contract(acquire).expect("declared");
        match acquire_position.as_str() {
            "return_owned" => {
                assert!(matches!(acquiring.result, ResourceReturnMode::Owned { .. }));
                assert_eq!(acquiring.result_kind, kind);
            }
            "out" => {
                assert!(matches!(
                    acquiring.parameters[0],
                    ResourceParameterMode::Out { nullable: true }
                ));
                assert_eq!(acquiring.parameter_kind(0), kind);
                assert!(kind.is_in_place());
            }
            other => panic!("unknown position {other}"),
        }
        let releasing = contracts.contract(release).expect("declared");
        assert_eq!(release_position, "released");
        assert!(matches!(
            releasing.parameters[0],
            ResourceParameterMode::Released { nullable: false }
        ));
        assert_eq!(releasing.parameter_kind(0), kind);
    }
}
