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

//! A schemas-only proposal producer for bounded unsigned counter helpers.
//!
//! This process reads the supplied typed path independently and computes which
//! field the path updates, in which direction, and over which unsigned storage
//! type. It evaluates no contract and receives no expected result. The receiver
//! recomputes the same reading from its own import and evaluates the pinned
//! contract itself before anything is admitted.
use qcore_schemas::SemanticWitness;
use qcore_schemas::counter_summary::{
    CounterContractWitness, CounterExpressionRecord, CounterOperationRecord, CounterSourceInput,
    CounterUpdateRecord, CounterValueRecord,
};
use qcore_schemas::plugin::*;

/// Bounds the supplied projection before any structural reading. These apply
/// to direct Rust callers as well as to transported JSON.
const MAX_OPERATIONS: usize = 64;

fn is_parameter(value: &CounterValueRecord, c_type: &str, index: u32) -> bool {
    value.c_type == c_type && value.expression == CounterExpressionRecord::Parameter { index }
}

fn is_symbol(value: &CounterValueRecord, c_type: &str, operation: u32) -> bool {
    value.c_type == c_type && value.expression == CounterExpressionRecord::Symbol { operation }
}

fn is_literal_one(value: &CounterValueRecord, c_type: &str) -> bool {
    value.c_type == c_type
        && value.expression
            == CounterExpressionRecord::Integer {
                value: "1".to_owned(),
            }
}

/// The unsigned C storage types this reading supports. Anything else keeps the
/// function outside the profile; there is no widening or promotion here.
fn is_supported_counter_type(c_type: &str) -> bool {
    matches!(
        c_type,
        "unsigned int" | "unsigned long" | "unsigned long long"
    )
}

fn derive_update(source: &CounterSourceInput) -> Option<CounterUpdateRecord> {
    if !source.version_matches()
        || source.function.is_empty()
        || source.source_revision.is_empty()
        || source.root_c_type.is_empty()
        || source.operations.len() > MAX_OPERATIONS
    {
        return None;
    }
    // One read of the counter, one unit update written back, one read of the
    // updated value, and that value returned. No other effect may appear.
    let [
        CounterOperationRecord::Read { place },
        CounterOperationRecord::Write {
            place: written,
            value,
        },
        CounterOperationRecord::Read { place: reread },
        CounterOperationRecord::ReturnValue { value: result },
    ] = source.operations.as_slice()
    else {
        return None;
    };
    let CounterExpressionRecord::Field { base, name } = &place.expression else {
        return None;
    };
    let counter_c_type = place.c_type.as_str();
    if !is_supported_counter_type(counter_c_type)
        || name.is_empty()
        || !is_parameter(base, &source.root_c_type, 0)
        || written != place
        || reread != place
        || source.signature != format!("{counter_c_type} ({})", source.root_c_type)
    {
        return None;
    }
    let CounterExpressionRecord::Binary {
        operator,
        left,
        right,
    } = &value.expression
    else {
        return None;
    };
    if value.c_type != place.c_type
        || !is_symbol(left, counter_c_type, 0)
        || !is_literal_one(right, counter_c_type)
    {
        return None;
    }
    let delta = match operator.as_str() {
        "+" => 1,
        "-" => -1,
        _ => return None,
    };
    // Both structural facts are read from the path, never assumed: the result
    // must be the value produced by the read that follows the write, and the
    // update must be the only write in the path.
    let result_is_updated_counter = is_symbol(result, counter_c_type, 2);
    let only_selected_field_written = source
        .operations
        .iter()
        .filter(|operation| matches!(operation, CounterOperationRecord::Write { .. }))
        .count()
        == 1;
    if !result_is_updated_counter || !only_selected_field_written {
        return None;
    }
    Some(CounterUpdateRecord {
        function: source.function.clone(),
        source_revision: source.source_revision.clone(),
        root_c_type: source.root_c_type.clone(),
        counter_c_type: counter_c_type.to_owned(),
        field: name.clone(),
        delta,
        result_is_updated_counter,
        only_selected_field_written,
        profile_assumptions: source.profile_assumptions.clone(),
    })
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || request.target.profile != "qcore.c_counter_contract.v1"
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    let summary = derive_update(request.counter_effect.as_ref()?)?;
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::CounterContract(Box::new(CounterContractWitness {
                task_digest: request.task_digest.clone(),
                summary,
            })),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::counter_summary::COUNTER_SOURCE_INPUT_VERSION;

    fn field(root: &str, name: &str, c_type: &str) -> CounterValueRecord {
        CounterValueRecord {
            c_type: c_type.into(),
            expression: CounterExpressionRecord::Field {
                base: Box::new(CounterValueRecord {
                    c_type: root.into(),
                    expression: CounterExpressionRecord::Parameter { index: 0 },
                }),
                name: name.into(),
            },
        }
    }

    fn symbol(c_type: &str, operation: u32) -> CounterValueRecord {
        CounterValueRecord {
            c_type: c_type.into(),
            expression: CounterExpressionRecord::Symbol { operation },
        }
    }

    fn integer(c_type: &str, value: &str) -> CounterValueRecord {
        CounterValueRecord {
            c_type: c_type.into(),
            expression: CounterExpressionRecord::Integer {
                value: value.into(),
            },
        }
    }

    fn input(operator: &str) -> CounterSourceInput {
        let root = "struct cio_ctx *";
        let ty = "unsigned long";
        let place = field(root, "total_chunks", ty);
        CounterSourceInput {
            schema_version: COUNTER_SOURCE_INPUT_VERSION,
            function: "cio_chunk_counter_total_sub".into(),
            source_revision: "pinned-source".into(),
            signature: format!("{ty} ({root})"),
            root_c_type: root.into(),
            operations: vec![
                CounterOperationRecord::Read {
                    place: place.clone(),
                },
                CounterOperationRecord::Write {
                    place: place.clone(),
                    value: CounterValueRecord {
                        c_type: ty.into(),
                        expression: CounterExpressionRecord::Binary {
                            operator: operator.into(),
                            left: Box::new(symbol(ty, 0)),
                            right: Box::new(integer(ty, "1")),
                        },
                    },
                },
                CounterOperationRecord::Read {
                    place: place.clone(),
                },
                CounterOperationRecord::ReturnValue {
                    value: symbol(ty, 2),
                },
            ],
            profile_assumptions: [("library".into(), "chunkio".into())].into(),
        }
    }

    #[test]
    fn reads_field_direction_and_storage_type_from_the_path() {
        let decrement = derive_update(&input("-")).unwrap();
        assert_eq!(decrement.field, "total_chunks");
        assert_eq!(decrement.delta, -1);
        assert_eq!(decrement.counter_c_type, "unsigned long");
        assert!(decrement.result_is_updated_counter && decrement.only_selected_field_written);
        assert_eq!(derive_update(&input("+")).unwrap().delta, 1);
        assert_eq!(
            decrement.profile_assumptions,
            input("-").profile_assumptions
        );
    }

    #[test]
    fn every_departure_from_the_supported_path_refuses() {
        let ty = "unsigned long";
        let root = "struct cio_ctx *";
        let mutate = |change: &dyn Fn(&mut CounterSourceInput)| {
            let mut source = input("-");
            change(&mut source);
            derive_update(&source)
        };
        // Unsupported direction, storage type, literal, symbol and signature.
        assert!(
            mutate(&|s| s.operations[1] = match &s.operations[1] {
                CounterOperationRecord::Write { place, value } => CounterOperationRecord::Write {
                    place: place.clone(),
                    value: CounterValueRecord {
                        c_type: value.c_type.clone(),
                        expression: CounterExpressionRecord::Binary {
                            operator: "*".into(),
                            left: Box::new(symbol(ty, 0)),
                            right: Box::new(integer(ty, "1")),
                        },
                    },
                },
                other => other.clone(),
            })
            .is_none()
        );
        assert!(
            mutate(&|s| s.operations[0] = CounterOperationRecord::Read {
                place: field(root, "total_chunks", "long"),
            })
            .is_none()
        );
        assert!(
            mutate(&|s| s.operations[3] = CounterOperationRecord::ReturnValue {
                value: symbol(ty, 0),
            })
            .is_none()
        );
        assert!(mutate(&|s| s.signature = format!("{ty} (void)")).is_none());
        assert!(mutate(&|s| s.schema_version = 0).is_none());
        assert!(mutate(&|s| s.function.clear()).is_none());
        assert!(mutate(&|s| s.root_c_type = "struct other *".into()).is_none());
        // A second write, a truncated path and an extra operation all refuse.
        assert!(
            mutate(&|s| {
                let extra = s.operations[1].clone();
                s.operations.push(extra);
            })
            .is_none()
        );
        assert!(mutate(&|s| { s.operations.truncate(3) }).is_none());
        assert!(
            mutate(&|s| s.operations = vec![s.operations[0].clone(); MAX_OPERATIONS + 1]).is_none()
        );
    }
}
