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

//! Generic loop summaries and resource-ledger morphisms.
//! Native conversion interprets a supplied execution record; it does not prove
//! that the record came from source or a particular native execution.

use crate::native_concurrency::{NativeExecutionCertificate, NativeProgram};
use crate::resource_flow::{Flow, FlowError};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopBoundEvidence {
    /// A supplied exact-count premise. The producer must establish this count;
    /// this model API does not derive source control-flow or identity evidence.
    Exact(u64),
    /// Any count from zero through this maximum may execute.
    AtMost(u64),
    /// A termination proof supplied a maximum, not an exact execution count.
    Terminating(u64),
    /// Abstract interpretation widened to this checked maximum iteration count.
    Widened(u64),
    /// No finite resource summary is claimed.
    Unbounded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopSummaryCertificate {
    pub body: Flow,
    pub evidence: LoopBoundEvidence,
    pub summary: Flow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopSummaryError {
    Unbounded,
    NonExactCustody,
    Arithmetic(FlowError),
    InvalidCertificate,
}

pub fn summarize_general_loop(
    body: &Flow,
    evidence: LoopBoundEvidence,
) -> Result<LoopSummaryCertificate, LoopSummaryError> {
    let summary = match evidence {
        LoopBoundEvidence::Exact(iterations) => body.repeat_exact(iterations),
        LoopBoundEvidence::AtMost(0)
        | LoopBoundEvidence::Terminating(0)
        | LoopBoundEvidence::Widened(0) => Ok(Flow::zero()),
        LoopBoundEvidence::AtMost(bound)
        | LoopBoundEvidence::Terminating(bound)
        | LoopBoundEvidence::Widened(bound) => {
            if !body.net().is_zero() {
                // One exact-net Flow cannot cover both zero and one iteration.
                return Err(LoopSummaryError::NonExactCustody);
            }
            body.bounded_zero_net_loop(bound)
        }
        LoopBoundEvidence::Unbounded => return Err(LoopSummaryError::Unbounded),
    }
    .map_err(LoopSummaryError::Arithmetic)?;
    Ok(LoopSummaryCertificate {
        body: body.clone(),
        evidence,
        summary,
    })
}

pub fn check_loop_summary(certificate: &LoopSummaryCertificate) -> Result<(), LoopSummaryError> {
    let expected = summarize_general_loop(&certificate.body, certificate.evidence)?;
    if expected.summary == certificate.summary {
        Ok(())
    } else {
        Err(LoopSummaryError::InvalidCertificate)
    }
}

pub type Ledger = BTreeMap<String, i128>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerMorphism {
    pub name: String,
    pub source: Ledger,
    pub target: Ledger,
    pub delta: Ledger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoryError {
    NegativeLedger { morphism: String, axis: String },
    InvalidDelta { morphism: String, axis: String },
    EndpointMismatch { left: String, right: String },
    EmptyAccount,
    InvalidLoopSummary,
}

impl LedgerMorphism {
    pub fn check(&self) -> Result<(), CategoryError> {
        let axes = self
            .source
            .keys()
            .chain(self.target.keys())
            .chain(self.delta.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for axis in axes {
            let source = self.source.get(&axis).copied().unwrap_or(0);
            let target = self.target.get(&axis).copied().unwrap_or(0);
            let delta = self.delta.get(&axis).copied().unwrap_or(0);
            if source < 0 || target < 0 {
                return Err(CategoryError::NegativeLedger {
                    morphism: self.name.clone(),
                    axis,
                });
            }
            if source.checked_add(delta) != Some(target) {
                return Err(CategoryError::InvalidDelta {
                    morphism: self.name.clone(),
                    axis,
                });
            }
        }
        Ok(())
    }

    pub fn then(&self, next: &Self) -> Result<Self, CategoryError> {
        self.check()?;
        next.check()?;
        if self.target != next.source {
            return Err(CategoryError::EndpointMismatch {
                left: self.name.clone(),
                right: next.name.clone(),
            });
        }
        let mut delta = self.delta.clone();
        for (axis, amount) in &next.delta {
            *delta.entry(axis.clone()).or_default() += amount;
        }
        let result = Self {
            name: format!("{};{}", self.name, next.name),
            source: self.source.clone(),
            target: next.target.clone(),
            delta,
        };
        result.check()?;
        Ok(result)
    }
}

/// Interpret the executable loop resource ledger as a morphism.
pub fn loop_ledger_morphism(
    certificate: &LoopSummaryCertificate,
) -> Result<LedgerMorphism, CategoryError> {
    check_loop_summary(certificate).map_err(|_| CategoryError::InvalidLoopSummary)?;
    let source = certificate
        .summary
        .required_entry()
        .iter()
        .map(|(axis, amount)| (axis.to_owned(), i128::from(amount)))
        .collect::<Ledger>();
    let mut target = source.clone();
    let mut delta = Ledger::new();
    for (axis, amount) in certificate.summary.net().iter() {
        delta.insert(axis.to_owned(), amount);
        *target.entry(axis.to_owned()).or_default() += amount;
    }
    let morphism = LedgerMorphism {
        name: "loop".into(),
        source,
        target,
        delta,
    };
    morphism.check()?;
    Ok(morphism)
}

/// Interpret native custody cleanup as a resource-ledger morphism.
pub fn native_ledger_morphism(
    program: &NativeProgram,
    execution: &NativeExecutionCertificate,
) -> Result<LedgerMorphism, CategoryError> {
    let source = program
        .initial_custody
        .keys()
        .map(|resource| (format!("custody:{resource}"), 1))
        .collect::<Ledger>();
    let target = program
        .initial_custody
        .keys()
        .filter(|resource| !execution.destroyed_resources.contains(resource))
        .map(|resource| (format!("custody:{resource}"), 1))
        .collect::<Ledger>();
    let delta = program
        .initial_custody
        .keys()
        .filter(|resource| execution.destroyed_resources.contains(resource))
        .map(|resource| (format!("custody:{resource}"), -1))
        .collect::<Ledger>();
    let morphism = LedgerMorphism {
        name: "native-cleanup".into(),
        source,
        target,
        delta,
    };
    morphism.check()?;
    Ok(morphism)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoricalCertificate {
    /// Resource-like ledgers share the same checked morphism law.
    pub resource_morphisms: Vec<LedgerMorphism>,
    /// Relations whose semantics do not form additive resource ledgers.
    pub deliberately_separate: BTreeSet<String>,
}

pub fn check_categorical_account(account: &CategoricalCertificate) -> Result<(), CategoryError> {
    if account.resource_morphisms.is_empty() || account.deliberately_separate.is_empty() {
        return Err(CategoryError::EmptyAccount);
    }
    for morphism in &account.resource_morphisms {
        morphism.check()?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/resource_accounting.rs"]
mod tests;
