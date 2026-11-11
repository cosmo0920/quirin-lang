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

fn problem() -> Vec<InferenceConstraint> {
    vec![
        InferenceConstraint::UsageAtLeast {
            binding: "x".into(),
            amount: 1,
        },
        InferenceConstraint::UsageAtLeast {
            binding: "x".into(),
            amount: 3,
        },
        InferenceConstraint::UsageAtMost {
            binding: "x".into(),
            amount: 8,
        },
        InferenceConstraint::Effect { name: "Io".into() },
        InferenceConstraint::ResourcePeakAtLeast {
            axis: "memory".into(),
            amount: 16,
        },
        InferenceConstraint::ResourceTotalAtLeast {
            axis: "memory".into(),
            amount: 24,
        },
        InferenceConstraint::Owns { identity: 7 },
        InferenceConstraint::PathIs {
            predicate: "ready".into(),
            value: true,
        },
    ]
}

#[test]
fn infers_and_independently_checks_the_principal_product() {
    let certificate = infer_principal(&problem()).unwrap();
    check_inference_certificate(&problem(), &certificate).unwrap();
    assert_eq!(
        certificate.principal.usage["x"],
        UsageInterval {
            lower: 3,
            upper: Some(8)
        }
    );
    assert_eq!(certificate.principal.resource_peak["memory"], 16);
}

#[test]
fn normalization_makes_principal_results_permutation_unique() {
    let mut reversed = problem();
    reversed.reverse();
    assert_eq!(infer_principal(&problem()), infer_principal(&reversed));
}

#[test]
fn rejects_unsatisfiable_and_ambiguous_inputs() {
    assert!(matches!(
        infer_principal(&[
            InferenceConstraint::UsageAtLeast {
                binding: "x".into(),
                amount: 2
            },
            InferenceConstraint::UsageAtMost {
                binding: "x".into(),
                amount: 1
            }
        ]),
        Err(InferenceError::UnsatisfiableUsage { .. })
    ));
    assert!(matches!(
        infer_principal(&[InferenceConstraint::EitherEffect {
            left: "Io".into(),
            right: "Net".into()
        }]),
        Err(InferenceError::AmbiguousDisjunction { .. })
    ));
}

#[test]
fn rejects_a_satisfying_but_nonprincipal_certificate() {
    let mut certificate = infer_principal(&problem()).unwrap();
    certificate
        .principal
        .resource_peak
        .insert("memory".into(), 32);
    assert_eq!(
        check_inference_certificate(&problem(), &certificate),
        Err(InferenceError::NonPrincipalSolution)
    );
}
