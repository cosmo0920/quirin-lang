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

//! Card W43-DESCRIPTOR-RESOURCE-TRACKING — the kind clauses of resource
//! vocabulary version 1 (contract spec §C04.6), loader side.
//!
//! Every admitted placement loads to exactly the kinds it spells, a module
//! with no kind clause loads to heap everywhere, and every other placement
//! rejects the **whole** module with its own message: nothing is approximated.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{ResourceKind, ResourceParameterMode, ResourceReturnMode};

fn module(signature: &str, clauses: &str) -> String {
    format!(
        "module contract;\nextern fn k{signature}\ncontract {{\n    \
         requires resource_contract_v1;\n{clauses}}};\n"
    )
}

fn load(source: &str) -> Result<qcore_schemas::contract::ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), source.to_owned())]))
}

#[test]
fn every_kind_loads_where_it_may_stand() {
    let admitted = [
        (
            "(p: ptr<const, u8>) -> i32",
            "    ensures normal resource_borrowed(p);\n    \
             ensures normal resource_return_owned(sentinel);\n    \
             ensures normal resource_return_kind(descriptor);\n",
            None,
            ResourceKind::Descriptor,
        ),
        (
            "(fd: i32) -> i32",
            "    ensures normal resource_released(fd, valid);\n    \
             ensures normal resource_kind(fd, descriptor);\n    \
             ensures normal resource_return_none();\n",
            Some((
                ResourceParameterMode::Released { nullable: false },
                ResourceKind::Descriptor,
            )),
            ResourceKind::Heap,
        ),
        (
            "(fd: i32) -> unit",
            "    ensures normal resource_consumed(fd, sentinel);\n    \
             ensures normal resource_kind(fd, descriptor);\n    \
             ensures normal resource_return_none();\n",
            Some((
                ResourceParameterMode::Consumed { nullable: true },
                ResourceKind::Descriptor,
            )),
            ResourceKind::Heap,
        ),
        (
            "(fd: i32) -> unit",
            "    ensures normal resource_owned(fd, valid);\n    \
             ensures normal resource_kind(fd, descriptor);\n    \
             ensures normal resource_return_none();\n",
            Some((
                ResourceParameterMode::Owned { nullable: false },
                ResourceKind::Descriptor,
            )),
            ResourceKind::Heap,
        ),
        (
            "(p: ptr<mut, u8>) -> i32",
            "    ensures normal resource_released(p, nonnull);\n    \
             ensures normal resource_kind(p, stream);\n    \
             ensures normal resource_return_none();\n",
            Some((
                ResourceParameterMode::Released { nullable: false },
                ResourceKind::Stream,
            )),
            ResourceKind::Heap,
        ),
        (
            "(out: ptr<mut, ptr<mut, u8>>) -> i32",
            "    ensures normal resource_out(out, nullable);\n    \
             ensures normal resource_kind(out, directory);\n    \
             ensures normal resource_return_none();\n",
            Some((
                ResourceParameterMode::Out { nullable: true },
                ResourceKind::Directory,
            )),
            ResourceKind::Heap,
        ),
    ];
    for (signature, clauses, parameter, result_kind) in admitted {
        let source = module(signature, clauses);
        let contracts = load(&source).unwrap_or_else(|error| panic!("{error}\n{source}"));
        let contract = contracts.contract("k").expect("declared");
        assert_eq!(contract.result_kind, result_kind, "{source}");
        if let Some((mode, kind)) = parameter {
            assert_eq!(contract.parameters[0], mode, "{source}");
            assert_eq!(contract.parameter_kind(0), kind, "{source}");
        }
        assert!(contract.has_non_heap_kind(), "{source}");
    }
}

/// A module without a kind clause means the heap everywhere, and its contract
/// is equal to — and serializes exactly as — the pre-card one.
#[test]
fn an_unkinded_module_is_the_heap_and_serializes_as_before() {
    let source = module(
        "(p: ptr<mut, u8>) -> unit",
        "    ensures normal resource_released(p, nullable);\n    \
         ensures normal resource_return_none();\n",
    );
    let contracts = load(&source).expect("loads");
    let contract = contracts.contract("k").expect("declared");
    assert!(!contract.has_non_heap_kind());
    assert_eq!(
        *contract,
        qcore_schemas::contract::ResourceFunctionContract::new(
            [ResourceParameterMode::Released { nullable: true }],
            ResourceReturnMode::None,
        )
    );
    let json = serde_json::to_string(contract).expect("serializes");
    assert_eq!(
        json,
        r#"{"parameters":[{"released":{"nullable":true}}],"result":"none"}"#
    );
    assert_eq!(
        format!("{contract:?}"),
        "ResourceFunctionContract { parameters: [Released { nullable: true }], result: None }"
    );
}

#[test]
fn every_other_placement_rejects_the_module() {
    let rejected = [
        (
            "a kind on a borrowed parameter",
            module(
                "(fd: i32) -> unit",
                "    ensures normal resource_borrowed(fd);\n    \
                 ensures normal resource_kind(fd, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a kind names a custody-bearing parameter, not a borrowed one",
        ),
        (
            "the heap spelled out",
            module(
                "(p: ptr<mut, u8>) -> unit",
                "    ensures normal resource_released(p, nullable);\n    \
                 ensures normal resource_kind(p, heap);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "the heap kind is spelled by omitting the kind clause",
        ),
        (
            "an unknown kind",
            module(
                "(p: ptr<mut, u8>) -> unit",
                "    ensures normal resource_released(p, nullable);\n    \
                 ensures normal resource_kind(p, file_tree);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "unsupported resource kind file_tree",
        ),
        (
            "a descriptor spelled with pointer nullability",
            module(
                "(fd: i32) -> unit",
                "    ensures normal resource_released(fd, nonnull);\n    \
                 ensures normal resource_kind(fd, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a descriptor position spells its validity `sentinel` or `valid`",
        ),
        (
            "a heap pointer spelled with descriptor validity",
            module(
                "(p: ptr<mut, u8>) -> unit",
                "    ensures normal resource_released(p, valid);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a heap position spells its nullability `nullable` or `nonnull`",
        ),
        (
            "a stream spelled with descriptor validity",
            module(
                "(p: ptr<mut, u8>) -> unit",
                "    ensures normal resource_released(p, sentinel);\n    \
                 ensures normal resource_kind(p, stream);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a stream position spells its nullability `nullable` or `nonnull`",
        ),
        (
            "a descriptor on a pointer",
            module(
                "(p: ptr<mut, u8>) -> unit",
                "    ensures normal resource_released(p, valid);\n    \
                 ensures normal resource_kind(p, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a descriptor position requires `i32`",
        ),
        (
            "a descriptor on a wider integer",
            module(
                "(fd: i64) -> unit",
                "    ensures normal resource_released(fd, valid);\n    \
                 ensures normal resource_kind(fd, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a descriptor position requires `i32`",
        ),
        (
            "a stream on an integer",
            module(
                "(fd: i32) -> unit",
                "    ensures normal resource_released(fd, nonnull);\n    \
                 ensures normal resource_kind(fd, stream);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a stream position requires a pointer type",
        ),
        (
            "a descriptor out slot",
            module(
                "(out: ptr<mut, ptr<mut, u8>>) -> unit",
                "    ensures normal resource_out(out, valid);\n    \
                 ensures normal resource_kind(out, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a descriptor has no out slot in this vocabulary",
        ),
        (
            "a kinded stored parameter",
            module(
                "(owner: ptr<mut, u8>, fd: i32) -> unit",
                "    ensures normal resource_owned(owner, nonnull);\n    \
                 ensures normal resource_stored(fd, owner, valid);\n    \
                 ensures normal resource_kind(fd, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a stored or removed parameter is heap only",
        ),
        (
            "a result kind on no result",
            module(
                "() -> i32",
                "    ensures normal resource_return_none();\n    \
                 ensures normal resource_return_kind(descriptor);\n",
            ),
            "a result kind requires an owned result",
        ),
        (
            "a result kind on a reallocation",
            module(
                "(p: ptr<mut, u8>) -> ptr<mut, u8>",
                "    ensures normal resource_owned(p, nonnull);\n    \
                 ensures normal resource_return_reallocated(p, nullable);\n    \
                 ensures normal resource_return_kind(stream);\n",
            ),
            "a result kind requires an owned result",
        ),
        (
            "a duplicate kind",
            module(
                "(fd: i32) -> unit",
                "    ensures normal resource_released(fd, valid);\n    \
                 ensures normal resource_kind(fd, descriptor);\n    \
                 ensures normal resource_kind(fd, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "duplicate kind for fd",
        ),
        (
            "a duplicate result kind",
            module(
                "() -> i32",
                "    ensures normal resource_return_owned(sentinel);\n    \
                 ensures normal resource_return_kind(descriptor);\n    \
                 ensures normal resource_return_kind(descriptor);\n",
            ),
            "duplicate result kind",
        ),
        (
            "a kind naming no parameter",
            module(
                "(fd: i32) -> unit",
                "    ensures normal resource_released(fd, valid);\n    \
                 ensures normal resource_kind(other, descriptor);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "unknown parameter other",
        ),
        (
            "an integer custody mode without a kind (the pre-card refusal)",
            module(
                "(fd: i32) -> unit",
                "    ensures normal resource_released(fd, nullable);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "custody mode requires a pointer parameter",
        ),
        (
            "an integer owned result without a kind (the pre-card refusal)",
            module(
                "() -> i32",
                "    ensures normal resource_return_owned(nullable);\n",
            ),
            "owned result requires a pointer return type",
        ),
        (
            "a sentinel result without a kind",
            module(
                "() -> i32",
                "    ensures normal resource_return_owned(sentinel);\n",
            ),
            "a heap position spells its nullability `nullable` or `nonnull`",
        ),
    ];
    for (label, source, expected) in rejected {
        let error = load(&source).expect_err(label);
        assert!(error.contains(expected), "{label}: {error}\n{source}");
    }
}
