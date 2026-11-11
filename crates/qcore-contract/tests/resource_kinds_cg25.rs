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

//! Card **CG25-RESOURCE-KINDS** — the `dynamic_library` kind and the in-place
//! `regex` kind, loader side (contract spec §C04.6, §C04.6.1).
//!
//! Each admitted placement loads to exactly the kinds it spells; every other
//! placement rejects the **whole** module with its own message.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::{
    ResourceCarrier, ResourceKind, ResourceParameterMode, ResourceReturnMode,
};

fn module(signature: &str, clauses: &str) -> String {
    format!(
        "module contract;\nextern fn k{signature}\ncontract {{\n    \
         requires resource_contract_v1;\n{clauses}}};\n"
    )
}

fn load(source: &str) -> Result<qcore_schemas::contract::ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("k.qv".to_owned(), source.to_owned())]))
}

#[test]
fn every_kind_has_one_spelling_and_one_carrier() {
    let table = ResourceKind::ALL.map(|kind| (kind.spelling(), kind.carrier()));
    assert_eq!(
        table,
        [
            ("heap", ResourceCarrier::Pointer),
            ("descriptor", ResourceCarrier::Integer),
            ("stream", ResourceCarrier::Pointer),
            ("directory", ResourceCarrier::Pointer),
            ("dynamic_library", ResourceCarrier::Pointer),
            ("regex", ResourceCarrier::Storage),
            // Card REFCOUNT-RESOURCE-KIND (§C04.17).
            ("reference", ResourceCarrier::Pointer),
        ]
    );
    for kind in ResourceKind::ALL {
        assert_eq!(ResourceKind::from_spelling(kind.spelling()), Some(kind));
        assert_eq!(
            kind.is_in_place(),
            kind.carrier() == ResourceCarrier::Storage
        );
        assert_eq!(
            kind.is_integer(),
            kind.carrier() == ResourceCarrier::Integer
        );
    }
}

#[test]
fn the_library_kind_loads_on_an_owned_result_and_a_release() {
    let open = load(&module(
        "(p: ptr<const, u8>) -> ptr<mut, u8>",
        "    ensures normal resource_borrowed(p);\n    \
         ensures normal resource_return_owned(nullable);\n    \
         ensures normal resource_return_kind(dynamic_library);\n",
    ))
    .expect("an owned library handle loads");
    let contract = open.contract("k").expect("declared");
    assert_eq!(
        contract.result,
        ResourceReturnMode::Owned { nullable: true }
    );
    assert_eq!(contract.result_kind, ResourceKind::DynamicLibrary);
    let close = load(&module(
        "(h: ptr<mut, u8>) -> i32",
        "    ensures normal resource_released(h, nonnull);\n    \
         ensures normal resource_kind(h, dynamic_library);\n    \
         ensures normal resource_return_none();\n",
    ))
    .expect("a library release loads");
    assert_eq!(
        close.contract("k").expect("declared").parameter_kind(0),
        ResourceKind::DynamicLibrary
    );
}

#[test]
fn the_in_place_kind_loads_on_its_three_positions() {
    let init = load(&module(
        "(o: ptr<mut, u8>, s: ptr<const, u8>) -> i32",
        "    ensures normal resource_out(o, nullable);\n    \
         ensures normal resource_kind(o, regex);\n    \
         ensures normal resource_borrowed(s);\n    \
         ensures normal resource_return_none();\n",
    ))
    .expect("an in-place initialiser loads");
    let contract = init.contract("k").expect("declared");
    assert_eq!(
        contract.parameters[0],
        ResourceParameterMode::Out { nullable: true }
    );
    assert_eq!(contract.parameter_kind(0), ResourceKind::Regex);
    for (signature, clause, mode) in [
        (
            "(o: ptr<mut, u8>) -> unit",
            "resource_released(o, nonnull)",
            ResourceParameterMode::Released { nullable: false },
        ),
        (
            "(o: ptr<const, u8>) -> i32",
            "resource_borrowed(o)",
            ResourceParameterMode::Borrowed,
        ),
    ] {
        let loaded = load(&module(
            signature,
            &format!(
                "    ensures normal {clause};\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n"
            ),
        ))
        .unwrap_or_else(|error| panic!("{clause}: {error}"));
        let contract = loaded.contract("k").expect("declared");
        assert_eq!(contract.parameters[0], mode);
        assert_eq!(contract.parameter_kind(0), ResourceKind::Regex);
    }
}

#[test]
fn every_other_placement_rejects_the_module() {
    let rejected = [
        (
            "a library handle on a borrowed parameter",
            module(
                "(h: ptr<mut, u8>) -> i32",
                "    ensures normal resource_borrowed(h);\n    \
                 ensures normal resource_kind(h, dynamic_library);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a kind names a custody-bearing parameter, not a borrowed one",
        ),
        (
            "a library handle on an integer",
            module(
                "(h: i32) -> i32",
                "    ensures normal resource_released(h, valid);\n    \
                 ensures normal resource_kind(h, dynamic_library);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a dynamic_library position spells its nullability `nullable` or `nonnull`",
        ),
        (
            "an infallible in-place initialiser",
            module(
                "(o: ptr<mut, u8>) -> i32",
                "    ensures normal resource_out(o, nonnull);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex initialiser fails by its status: spell it `nullable`",
        ),
        (
            "an in-place initialiser over a pointer slot",
            module(
                "(o: ptr<mut, ptr<mut, u8>>) -> i32",
                "    ensures normal resource_out(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex position names the object",
        ),
        (
            "an in-place initialiser over a const object",
            module(
                "(o: ptr<const, u8>) -> i32",
                "    ensures normal resource_out(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex initialiser writes the object",
        ),
        (
            "an in-place initialiser without its status",
            module(
                "(o: ptr<mut, u8>) -> unit",
                "    ensures normal resource_out(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "an in-place initialiser returns its `i32` status",
        ),
        (
            "an in-place initialiser that also returns custody",
            module(
                "(o: ptr<mut, u8>) -> ptr<mut, u8>",
                "    ensures normal resource_out(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_owned(nullable);\n",
            ),
            "an in-place initialiser returns its `i32` status",
        ),
        (
            "an in-place initialiser that moves other custody",
            module(
                "(o: ptr<mut, u8>, p: ptr<mut, u8>) -> i32",
                "    ensures normal resource_out(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_released(p, nullable);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "an in-place initialiser moves no other custody",
        ),
        (
            "two in-place initialisers",
            module(
                "(o: ptr<mut, u8>, q: ptr<mut, u8>) -> i32",
                "    ensures normal resource_out(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_out(q, nullable);\n    \
                 ensures normal resource_kind(q, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "one status witnesses at most one in-place initialisation",
        ),
        (
            "a nullable in-place finaliser",
            module(
                "(o: ptr<mut, u8>) -> unit",
                "    ensures normal resource_released(o, nullable);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex finaliser names an object: spell it `nonnull`",
        ),
        (
            "an owned in-place parameter",
            module(
                "(o: ptr<mut, u8>) -> unit",
                "    ensures normal resource_owned(o, nonnull);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex object stays in caller storage",
        ),
        (
            "a consumed in-place parameter",
            module(
                "(o: ptr<mut, u8>) -> unit",
                "    ensures normal resource_consumed(o, nonnull);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex object stays in caller storage",
        ),
        (
            "an in-place result",
            module(
                "() -> ptr<mut, u8>",
                "    ensures normal resource_return_owned(nullable);\n    \
                 ensures normal resource_return_kind(regex);\n",
            ),
            "a regex object is initialised in caller storage, never returned",
        ),
        (
            "an in-place object on an integer",
            module(
                "(o: i32) -> unit",
                "    ensures normal resource_borrowed(o);\n    \
                 ensures normal resource_kind(o, regex);\n    \
                 ensures normal resource_return_none();\n",
            ),
            "a regex position names the object",
        ),
        (
            "a generic handle kind stays undeclared",
            module(
                "() -> ptr<mut, u8>",
                "    ensures normal resource_return_owned(nonnull);\n    \
                 ensures normal resource_return_kind(handle);\n",
            ),
            "unsupported resource kind handle",
        ),
    ];
    for (label, source, expected) in rejected {
        let error = load(&source).expect_err(label);
        assert!(error.contains(expected), "{label}: {error}\n{source}");
    }
}
