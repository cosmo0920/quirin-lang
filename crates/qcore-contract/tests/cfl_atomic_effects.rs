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

use qcore_frontend::frontend::{SourceContractClauseKind as K, parse_source_program};
use std::collections::BTreeMap;
#[test]
fn atomic_backend_model_retains_typed_cases_without_installing_a_summary() {
    let text = include_str!("../../../providers/quirin-provider-cfl/contracts/cfl/atomic.qv");
    let parsed = parse_source_program(text).unwrap();
    assert_eq!(parsed.declarations.len(), 4);
    let mut effects = Vec::new();
    for declaration in parsed.declarations {
        for clause in &declaration.value.function_contract.unwrap().clauses {
            match &clause.value.kind {
                K::QuantCondition(qcore_schemas::quant_condition::Selector::Entry) => {
                    qcore_frontend::quant_proposition::parse(&clause.value.expression_source)
                        .unwrap();
                }
                K::QuantEffect(_) => {
                    effects.push(
                        qcore_frontend::quant_effect::parse(&clause.value.expression_source)
                            .unwrap(),
                    );
                }
                other => panic!("unexpected nonformal atomic clause: {other:?}"),
            }
        }
    }
    assert_eq!(effects.len(), 5);
    let success = effects.iter().find(|e| e.id == "cas_success").unwrap();
    let failure = effects.iter().find(|e| e.id == "cas_failure").unwrap();
    assert_eq!(success.reads, success.writes);
    assert!(failure.writes.is_empty());
    assert_ne!(success.guard, failure.guard);
    let sources = BTreeMap::from([("cfl/atomic.qv".into(), text.into())]);
    assert!(qcore_contract::contract_loader::load_resource_contract_sources(&sources).is_err());
    assert!(qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).is_err());
}
