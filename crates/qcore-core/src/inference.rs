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

//! Cycle 17 principal inference for the decidable checker fragment.
//!
//! The solver emits a canonical certificate.  `check_inference_certificate`
//! does not call the solver: it checks the normalized constraint stream, every
//! obligation, and the least lower/greatest upper construction directly.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InferenceConstraint {
    UsageAtLeast {
        binding: String,
        amount: u64,
    },
    UsageAtMost {
        binding: String,
        amount: u64,
    },
    Effect {
        name: String,
    },
    ResourcePeakAtLeast {
        axis: String,
        amount: u64,
    },
    ResourceTotalAtLeast {
        axis: String,
        amount: u64,
    },
    Owns {
        identity: u64,
    },
    PathIs {
        predicate: String,
        value: bool,
    },
    /// Disjunction is deliberately outside the principal fragment.
    EitherEffect {
        left: String,
        right: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageInterval {
    pub lower: u64,
    pub upper: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrincipalSolution {
    pub usage: BTreeMap<String, UsageInterval>,
    pub effects: BTreeSet<String>,
    pub resource_peak: BTreeMap<String, u64>,
    pub resource_total: BTreeMap<String, u64>,
    pub owned: BTreeSet<u64>,
    pub paths: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InferenceCertificate {
    pub normal_form: Vec<InferenceConstraint>,
    pub principal: PrincipalSolution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceError {
    UnsatisfiableUsage {
        binding: String,
        lower: u64,
        upper: u64,
    },
    UnsatisfiablePath {
        predicate: String,
    },
    AmbiguousDisjunction {
        left: String,
        right: String,
    },
    NonCanonicalConstraints,
    ConstraintNotSatisfied(InferenceConstraint),
    NonPrincipalSolution,
}

impl fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for InferenceError {}

fn normalize(constraints: &[InferenceConstraint]) -> Vec<InferenceConstraint> {
    constraints
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Solve the finite conjunction fragment and return its canonical least
/// solution.  Upper usage bounds are contravariant: the principal interval has
/// the greatest allowed upper bound and the least required lower bound.
pub fn infer_principal(
    constraints: &[InferenceConstraint],
) -> Result<InferenceCertificate, InferenceError> {
    let normal_form = normalize(constraints);
    let principal = canonical_solution(&normal_form)?;
    Ok(InferenceCertificate {
        normal_form,
        principal,
    })
}

fn canonical_solution(
    constraints: &[InferenceConstraint],
) -> Result<PrincipalSolution, InferenceError> {
    let mut solution = PrincipalSolution::default();
    let mut lowers = BTreeMap::<String, u64>::new();
    let mut uppers = BTreeMap::<String, u64>::new();
    for constraint in constraints {
        match constraint {
            InferenceConstraint::UsageAtLeast { binding, amount } => {
                lowers
                    .entry(binding.clone())
                    .and_modify(|current| *current = (*current).max(*amount))
                    .or_insert(*amount);
            }
            InferenceConstraint::UsageAtMost { binding, amount } => {
                uppers
                    .entry(binding.clone())
                    .and_modify(|current| *current = (*current).min(*amount))
                    .or_insert(*amount);
            }
            InferenceConstraint::Effect { name } => {
                solution.effects.insert(name.clone());
            }
            InferenceConstraint::ResourcePeakAtLeast { axis, amount } => {
                insert_max(&mut solution.resource_peak, axis, *amount);
            }
            InferenceConstraint::ResourceTotalAtLeast { axis, amount } => {
                insert_max(&mut solution.resource_total, axis, *amount);
            }
            InferenceConstraint::Owns { identity } => {
                solution.owned.insert(*identity);
            }
            InferenceConstraint::PathIs { predicate, value } => {
                if solution
                    .paths
                    .insert(predicate.clone(), *value)
                    .is_some_and(|old| old != *value)
                {
                    return Err(InferenceError::UnsatisfiablePath {
                        predicate: predicate.clone(),
                    });
                }
            }
            InferenceConstraint::EitherEffect { left, right } => {
                return Err(InferenceError::AmbiguousDisjunction {
                    left: left.clone(),
                    right: right.clone(),
                });
            }
        }
    }
    let bindings = lowers
        .keys()
        .chain(uppers.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for binding in bindings {
        let lower = lowers.get(&binding).copied().unwrap_or(0);
        let upper = uppers.get(&binding).copied();
        if upper.is_some_and(|limit| lower > limit) {
            return Err(InferenceError::UnsatisfiableUsage {
                binding,
                lower,
                upper: upper.expect("upper was tested"),
            });
        }
        solution
            .usage
            .insert(binding, UsageInterval { lower, upper });
    }
    Ok(solution)
}

fn insert_max(map: &mut BTreeMap<String, u64>, key: &str, amount: u64) {
    map.entry(key.to_owned())
        .and_modify(|current| *current = (*current).max(amount))
        .or_insert(amount);
}

/// Independently check a solver-produced principal certificate.
pub fn check_inference_certificate(
    constraints: &[InferenceConstraint],
    certificate: &InferenceCertificate,
) -> Result<(), InferenceError> {
    let expected_normal = normalize(constraints);
    if certificate.normal_form != expected_normal {
        return Err(InferenceError::NonCanonicalConstraints);
    }
    for constraint in &certificate.normal_form {
        if !satisfies(&certificate.principal, constraint) {
            return Err(InferenceError::ConstraintNotSatisfied(constraint.clone()));
        }
    }
    // This reconstructs the extrema directly; it intentionally does not call
    // `infer_principal`, so the certificate checker remains a separate gate.
    if certificate.principal != canonical_solution(&certificate.normal_form)? {
        return Err(InferenceError::NonPrincipalSolution);
    }
    Ok(())
}

fn satisfies(solution: &PrincipalSolution, constraint: &InferenceConstraint) -> bool {
    match constraint {
        InferenceConstraint::UsageAtLeast { binding, amount } => solution
            .usage
            .get(binding)
            .is_some_and(|interval| interval.lower >= *amount),
        InferenceConstraint::UsageAtMost { binding, amount } => solution
            .usage
            .get(binding)
            .and_then(|interval| interval.upper)
            .is_some_and(|upper| upper <= *amount),
        InferenceConstraint::Effect { name } => solution.effects.contains(name),
        InferenceConstraint::ResourcePeakAtLeast { axis, amount } => {
            solution.resource_peak.get(axis).copied().unwrap_or(0) >= *amount
        }
        InferenceConstraint::ResourceTotalAtLeast { axis, amount } => {
            solution.resource_total.get(axis).copied().unwrap_or(0) >= *amount
        }
        InferenceConstraint::Owns { identity } => solution.owned.contains(identity),
        InferenceConstraint::PathIs { predicate, value } => {
            solution.paths.get(predicate) == Some(value)
        }
        InferenceConstraint::EitherEffect { .. } => false,
    }
}

#[cfg(test)]
#[path = "../tests/unit/inference.rs"]
mod tests;
