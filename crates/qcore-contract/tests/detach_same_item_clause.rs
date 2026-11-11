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

use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceReturnMode;
use std::collections::BTreeMap;

const CONTRACT: &str = r#"module contract;
extern fn detach(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> ptr<mut, u8>
contract {
 requires resource_contract_v1;
 ensures normal resource_owned(parent, nonnull);
 ensures normal resource_removed(item, parent, nullable);
 ensures normal resource_return_removed(item, nullable);
};"#;
fn load(text: &str) -> Result<qcore_schemas::contract::ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("detach.qv".to_owned(), text.to_owned())]))
}
#[test]
fn removed_result_carries_the_existing_item_not_fresh_custody() {
    let environment = load(CONTRACT).unwrap();
    assert_eq!(
        environment.contract("detach").unwrap().result,
        ResourceReturnMode::Removed {
            parameter: 1,
            nullable: true
        }
    );
}
#[test]
fn unpaired_item_bad_owner_and_nullability_mismatch_refuse_the_module() {
    for text in [
        CONTRACT.replace(
            "resource_removed(item, parent, nullable)",
            "resource_borrowed(item)",
        ),
        CONTRACT.replace(
            "resource_owned(parent, nonnull)",
            "resource_owned(parent, nullable)",
        ),
        CONTRACT.replace(
            "resource_return_removed(item, nullable)",
            "resource_return_removed(item, nonnull)",
        ),
        CONTRACT.replace(
            "resource_return_removed(item, nullable)",
            "resource_return_removed(parent, nullable)",
        ),
        CONTRACT.replace("item: ptr<mut, u8>", "item: ptr<const, u8>"),
        CONTRACT.replace("-> ptr<mut, u8>", "-> i32"),
        CONTRACT.replace("ensures normal resource_owned(parent, nonnull);", "ensures normal resource_owned(parent, nonnull); ensures normal resource_kind(parent, stream);"),
        CONTRACT.replace(
            "resource_removed(item, parent, nullable)",
            "resource_removed(item, item, nullable)",
        ),
    ] {
        assert!(load(&text).is_err(), "{text}");
    }
}

#[test]
fn removed_result_cannot_erase_conditional_or_registered_lifetime_effects() {
    for text in [
        CONTRACT.replace(
            " requires resource_contract_v1;",
            " requires resource_contract_v1; quant #threaded = (#parent.threaded != 0); quant #threaded in { requires resource_premise(detach_guard, \"The detach guard holds\"); }: [[local]];",
        ),
        CONTRACT.replace(
            " requires resource_contract_v1;",
            " requires resource_contract_v1; quant registered #entry: detach_node.next |-> detach_node.parent;",
        ),
    ] {
        let error = load(&text).unwrap_err();
        assert!(error.contains("local-condition-contract"), "{error}\n{text}");
    }
}
