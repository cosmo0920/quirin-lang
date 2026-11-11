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

use super::*;
use qcore_c_gateway::typed_source_nodes::{NODE_TABLE, facts};

/// CG01 (rule 10r): the wrapper projection classifies every CG01 node, as
/// strongly as each node's facts allow and no more: exactly the nodes with no
/// fault case and no store are custody-neutral; the rest are obligations the
/// projection records and never discharges.
#[test]
fn monkey_wrapper_admission_classifies_every_cg01_node() {
    MONKEY_WRAPPER_ADMISSION.validate().unwrap();
    let rows = MONKEY_WRAPPER_ADMISSION.rows();
    assert_eq!(rows.len(), NODE_TABLE.len());
    for (spelling, class) in rows {
        let facts = facts(spelling).unwrap();
        let neutral = facts.faults.is_empty() && !facts.store;
        assert_eq!(
            *class,
            if neutral {
                Classification::CustodyNeutral
            } else {
                Classification::ObligationBearing
            },
            "{spelling}"
        );
    }
    // The callbacks keep the legacy admission: nothing is admitted.
    assert!(NodeAdmission::LEGACY.admit("binary ^").is_err());
}

/// CG09 (rule 11l): the wrapper projection admits exactly the acyclic control
/// forms, each lowered into guarded paths of the existing vocabulary; no
/// admission can list a back-edge form, and the callbacks list nothing.
#[test]
fn monkey_wrapper_control_admits_exactly_the_acyclic_forms() {
    use qcore_c_gateway::typed_source::CONTROL_TABLE;
    MONKEY_WRAPPER_CONTROL.validate().unwrap();
    let acyclic = CONTROL_TABLE
        .iter()
        .filter(|facts| !facts.back_edge)
        .map(|facts| facts.spelling)
        .collect::<Vec<_>>();
    assert_eq!(MONKEY_WRAPPER_CONTROL.rows(), acyclic.as_slice());
    for facts in CONTROL_TABLE.iter().filter(|facts| facts.back_edge) {
        assert!(MONKEY_WRAPPER_CONTROL.admit(facts.spelling).is_err());
    }
    assert!(ControlAdmission::LEGACY.admit("switch").is_err());
}
