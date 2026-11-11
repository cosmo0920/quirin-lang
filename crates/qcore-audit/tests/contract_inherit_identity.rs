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

//! Card CONTRACT-INHERIT: an inherited contract's parent is part of the
//! audit trust-context identity, and an environment that inherited nothing
//! derives exactly the identity it derived before the card.

use std::collections::{BTreeMap, BTreeSet};

use qcore_audit::certificate_context::{AuditContractTrustPolicy, AuditTrustContextIdentity};
use qcore_contract::contract_inclusion::{ContractLoadPath, PublishedContractCrate};
use qcore_contract::contract_loader::{
    PublishedModuleRef, inclusion_provenance, load_published_resource_contracts,
    load_resource_contract_sources,
};
use qcore_schemas::contract::ResourceContracts;

const PARENT: &str = "module contract;\n\
extern fn free(ptr: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(ptr, nullable);\n\
    ensures normal resource_return_none();\n\
};\n";

const CHILD: &str = "module contract;\n\
#include \"quirin-provider-libc:libc/heap.qcore\";\n\
extern fn flb_free(p: ptr<mut, u8>) -> unit\n\
contract inherit libc::free;\n";

const HAND_WRITTEN: &str = "module contract;\n\
extern fn flb_free(p: ptr<mut, u8>) -> unit\n\
contract {\n\
    requires resource_contract_v1;\n\
    ensures normal resource_released(p, nullable);\n\
    ensures normal resource_return_none();\n\
};\n";

const PROFILE: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn inherited(parent: &str) -> ResourceContracts {
    let libc = PublishedContractCrate {
        name: "quirin-provider-libc".into(),
        namespace: "libc".into(),
        version: "0.1.0".into(),
        dependencies: BTreeSet::new(),
        modules: BTreeMap::from([("libc/heap.qcore".into(), parent.into())]),
        published: BTreeSet::from(["libc/heap.qcore".into()]),
    };
    let flb = PublishedContractCrate {
        name: "quirin-provider-fluent-bit".into(),
        namespace: "fluent_bit".into(),
        version: "0.1.0".into(),
        dependencies: BTreeSet::from(["quirin-provider-libc".into()]),
        modules: BTreeMap::from([("fluent_bit/memory.qv".into(), CHILD.into())]),
        published: BTreeSet::new(),
    };
    load_published_resource_contracts(
        &ContractLoadPath::new([libc, flb]).unwrap(),
        &[PublishedModuleRef::new(
            "quirin-provider-fluent-bit",
            "fluent_bit/memory.qv",
        )],
    )
    .unwrap()
}

fn identity(contracts: &ResourceContracts) -> AuditTrustContextIdentity {
    AuditTrustContextIdentity::derive(
        contracts,
        PROFILE,
        AuditContractTrustPolicy::DeclaredSourcesAreTrustedAssumptions,
    )
    .unwrap()
}

#[test]
fn the_parent_text_is_part_of_the_trust_context_identity() {
    let before = inherited(PARENT);
    let after = inherited(&format!("{PARENT}// reviewed\n"));
    // Same modes, same elaborated source; different parent bytes.
    assert_eq!(before.functions(), after.functions());
    assert_eq!(before.source_assumptions(), after.source_assumptions());
    assert_ne!(identity(&before), identity(&after));
    let provenance = inclusion_provenance(&before).unwrap();
    let chain = &provenance["modules"]["quirin-provider-fluent-bit:fluent_bit/memory.qv"]["inherited"]
        ["flb_free"];
    assert_eq!(chain[0]["declaration"], "free");
    assert_eq!(chain[0]["module"]["crate_name"], "quirin-provider-libc");
    assert_eq!(chain[0]["module"]["crate_version"], "0.1.0");
    assert_eq!(chain[0]["module"]["module"], "libc/heap.qcore");
    assert_eq!(
        chain[0]["module"]["sha256"],
        qcore_schemas::sha256_hex(PARENT.as_bytes())
    );
}

#[test]
fn an_environment_without_inclusion_derives_the_identity_it_always_did() {
    let written = load_resource_contract_sources(&BTreeMap::from([(
        "memory.qcore".to_owned(),
        HAND_WRITTEN.to_owned(),
    )]))
    .unwrap();
    assert!(inclusion_provenance(&written).is_none());
    // The serialized environment has exactly the two pre-card fields, so its
    // digest is the pre-card digest.
    let json = serde_json::to_value(&written).unwrap();
    assert_eq!(
        json.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["functions", "source_assumptions"]
    );
    let inherited = inherited(PARENT);
    assert_eq!(written.functions(), inherited.functions());
    assert_ne!(identity(&written), identity(&inherited));
}
