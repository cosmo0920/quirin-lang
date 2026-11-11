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

//! Cycle 16 bounded research closure.
//!
//! This module composes, rather than replaces, the independently executable
//! sequential, concurrent, and advanced-control checkers.  A certificate is
//! available only for a closed product program whose target observation
//! validates and whose resource ledgers all terminate without live identities.

use std::collections::BTreeSet;
use std::fmt;

use crate::advanced_control::{
    AdvancedControlError, AdvancedExecution, AdvancedProgram, AdvancedTraceEvent, run_advanced,
};
use crate::concurrency::{
    ConcurrencyCertificate, ConcurrencyDiagnostic, ConcurrentProgram, check_concurrent,
};
use crate::translation_validation::{
    TranslationCertificate, TranslationValidationError, validate_translation,
};
use crate::{SourceProgram, TraceEvent, lower_cleanup_cfg};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchProgram {
    pub sequential: SourceProgram,
    pub c11_stdout: String,
    pub concurrent: ConcurrentProgram,
    pub advanced: AdvancedProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceSafetyCertificate {
    pub sequential_identities: Vec<u64>,
    pub concurrent_identities: Vec<u32>,
    pub advanced_identities: Vec<u32>,
    pub sequential_acquisitions: usize,
    pub concurrent_events: usize,
    pub advanced_acquisitions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchClosureCertificate {
    pub translation: TranslationCertificate,
    pub concurrency: ConcurrencyCertificate,
    pub advanced: AdvancedExecution,
    pub resources: ResourceSafetyCertificate,
    pub progress: Progress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenResearchQuestion {
    PrincipalInference,
    GeneralNonzeroNetLoops,
    UnrestrictedLegacyC,
    CategoricalUnification,
    FullC11WeakMemory,
    NativeConcurrentBackend,
}

pub const PROMOTED_RESEARCH_QUESTIONS: &[OpenResearchQuestion] = &[
    OpenResearchQuestion::PrincipalInference,
    OpenResearchQuestion::GeneralNonzeroNetLoops,
    OpenResearchQuestion::UnrestrictedLegacyC,
    OpenResearchQuestion::CategoricalUnification,
    OpenResearchQuestion::FullC11WeakMemory,
    OpenResearchQuestion::NativeConcurrentBackend,
];

/// Cycle 17 discharges the six questions retained by the Cycle 16 product.
pub const OPEN_RESEARCH_QUESTIONS: &[OpenResearchQuestion] = &[];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResearchClosureError {
    Translation(TranslationValidationError),
    Concurrency(Vec<ConcurrencyDiagnostic>),
    Advanced(AdvancedControlError),
    SequentialLedger,
    ConcurrentLedger,
    AdvancedLedger,
}

impl fmt::Display for ResearchClosureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ResearchClosureError {}

pub fn close_research_program(
    program: &ResearchProgram,
) -> Result<ResearchClosureCertificate, ResearchClosureError> {
    let cfg = lower_cleanup_cfg(&program.sequential).map_err(|error| {
        ResearchClosureError::Translation(TranslationValidationError::Source(error))
    })?;
    let translation = validate_translation(&program.sequential, &cfg, &program.c11_stdout)
        .map_err(ResearchClosureError::Translation)?;
    let concurrency =
        check_concurrent(&program.concurrent).map_err(ResearchClosureError::Concurrency)?;
    let advanced = run_advanced(&program.advanced).map_err(ResearchClosureError::Advanced)?;

    let (sequential_identities, sequential_acquisitions) = check_sequential_ledger(&translation)?;
    let concurrent_identities = check_concurrent_ledger(program, &concurrency)?;
    let (advanced_identities, advanced_acquisitions) = check_advanced_ledger(&advanced)?;
    let resources = ResourceSafetyCertificate {
        sequential_identities,
        concurrent_identities,
        advanced_identities,
        sequential_acquisitions,
        concurrent_events: concurrency.checked_events,
        advanced_acquisitions,
    };
    Ok(ResearchClosureCertificate {
        translation,
        concurrency,
        advanced,
        resources,
        progress: Progress::Terminal,
    })
}

fn check_sequential_ledger(
    translation: &TranslationCertificate,
) -> Result<(Vec<u64>, usize), ResearchClosureError> {
    let mut acquired = BTreeSet::new();
    let mut live = BTreeSet::new();
    let mut completed = BTreeSet::new();
    for event in &translation.source.trace {
        match event {
            TraceEvent::Acquire(resource) => {
                if !acquired.insert(*resource) || !live.insert(*resource) {
                    return Err(ResearchClosureError::SequentialLedger);
                }
            }
            TraceEvent::Release(resource) | TraceEvent::Transfer(resource) => {
                if !live.remove(resource) || !completed.insert(*resource) {
                    return Err(ResearchClosureError::SequentialLedger);
                }
            }
        }
    }
    if !live.is_empty() || acquired != completed {
        return Err(ResearchClosureError::SequentialLedger);
    }
    Ok((acquired.iter().copied().collect(), acquired.len()))
}

fn check_concurrent_ledger(
    program: &ResearchProgram,
    certificate: &ConcurrencyCertificate,
) -> Result<Vec<u32>, ResearchClosureError> {
    let initial = program
        .concurrent
        .initial_custody
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let destroyed = certificate
        .destroyed_resources
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if initial != destroyed || destroyed.len() != certificate.destroyed_resources.len() {
        return Err(ResearchClosureError::ConcurrentLedger);
    }
    Ok(initial.into_iter().collect())
}

fn check_advanced_ledger(
    execution: &AdvancedExecution,
) -> Result<(Vec<u32>, usize), ResearchClosureError> {
    let mut acquired = BTreeSet::new();
    let mut live = BTreeSet::new();
    let mut destroyed = BTreeSet::new();
    for event in &execution.trace {
        match event {
            AdvancedTraceEvent::Acquire(resource) => {
                if !acquired.insert(*resource) || !live.insert(*resource) {
                    return Err(ResearchClosureError::AdvancedLedger);
                }
            }
            AdvancedTraceEvent::Destroy(resource) => {
                if !live.remove(resource) || !destroyed.insert(*resource) {
                    return Err(ResearchClosureError::AdvancedLedger);
                }
            }
            AdvancedTraceEvent::DestructorFailed { .. }
            | AdvancedTraceEvent::CancellationRequested
            | AdvancedTraceEvent::CancellationDelivered
            | AdvancedTraceEvent::HandlerInvoked(_) => {}
        }
    }
    if !live.is_empty() || acquired != destroyed {
        return Err(ResearchClosureError::AdvancedLedger);
    }
    Ok((acquired.iter().copied().collect(), acquired.len()))
}

#[cfg(test)]
#[path = "../tests/unit/research_closure.rs"]
mod tests;
