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

//! Actual guarded production metadata remains covered by normal text/site pins.
use qcore_audit::{
    contract_revision::{ContractLock, PinStanding, pin_standing},
    contract_tip::TipResolver,
};
use qcore_schemas::contract_revision::PinMismatchReason;
use std::{collections::BTreeMap, fs, path::Path};

#[test]
fn actual_guarded_source_requires_current_contract_and_all_original_reviewed_sites() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plan: qcore_contract::guarded_metadata_bridge::Plan = serde_json::from_slice(
        &fs::read(root.join("docs/reports/guarded-contract-adoption/adoption-plan.json")).unwrap(),
    )
    .unwrap();
    let actual = fs::read_to_string(root.join(&plan.module)).unwrap();
    let source = BTreeMap::from([(plan.module.clone(), actual.clone())]);
    let old = BTreeMap::from([(
        plan.module.clone(),
        actual.replacen(
            &qcore_contract::guarded_metadata_bridge::render(&plan.metadata).unwrap(),
            "",
            1,
        ),
    )]);
    let new_hash = qcore_contract::contract_revision::declaration_digests(&source).unwrap()
        [&plan.declaration]
        .1
        .clone();
    let old_hash = qcore_contract::contract_revision::declaration_digests(&old).unwrap()
        [&plan.declaration]
        .1
        .clone();
    assert_ne!(new_hash, old_hash);
    let lock: ContractLock = serde_json::from_slice(
        &fs::read(root.join("providers/qcore-provider-librdkafka/contracts.lock")).unwrap(),
    )
    .unwrap();
    let entry = lock
        .pinned
        .iter()
        .find(|e| e.function == plan.declaration)
        .unwrap();
    assert_eq!(entry.contract_sha256, new_hash);
    for identity in &entry.identities {
        let good = pin_standing(
            Some(&lock),
            &plan.declaration,
            &new_hash,
            &identity.sites,
            &entry.module,
            &mut TipResolver::default(),
        );
        assert!(matches!(good, PinStanding::InForce { .. }));
        for text in [&old_hash, &"0".repeat(64)] {
            assert!(matches!(
                pin_standing(
                    Some(&lock),
                    &plan.declaration,
                    text,
                    &identity.sites,
                    &entry.module,
                    &mut TipResolver::default()
                ),
                PinStanding::Mismatch {
                    reason: PinMismatchReason::ContractChanged,
                    ..
                }
            ));
        }
        let mut missing = identity.sites.clone();
        missing.pop();
        assert!(matches!(
            pin_standing(
                Some(&lock),
                &plan.declaration,
                &new_hash,
                &missing,
                &entry.module,
                &mut TipResolver::default()
            ),
            PinStanding::Mismatch { .. }
        ));
        let mut changed = identity.sites.clone();
        changed[0].declaration_sha256 = "0".repeat(64);
        assert!(matches!(
            pin_standing(
                Some(&lock),
                &plan.declaration,
                &new_hash,
                &changed,
                &entry.module,
                &mut TipResolver::default()
            ),
            PinStanding::Mismatch { .. }
        ));
    }
    assert!(matches!(
        pin_standing(
            None,
            &plan.declaration,
            &new_hash,
            &entry.identities[0].sites,
            &entry.module,
            &mut TipResolver::default()
        ),
        PinStanding::Mismatch {
            reason: PinMismatchReason::NoLock,
            ..
        }
    ));
}
