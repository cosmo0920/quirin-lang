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

//! Composition of closed C source extraction with the core publication model.

pub use qcore_c_gateway::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationSourceProfile,
};
use qcore_c_gateway::atomic_publication_source::{
    AtomicPublicationOrder, AtomicPublicationSourceCertificate, AtomicPublicationSourceRefusal,
    extract_atomic_publication_source,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_core::execution_coverage::{
    CoverageBounds, CoverageEvidence, GuardedInstruction, Instruction, ProgramFragment,
    ProgramVerdict, check_program_executions,
};
use qcore_core::{
    AtomicPublicationCertificate as CoreCertificate, AtomicPublicationProgram, MemoryOrder,
    check_atomic_publication,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomicPublicationCertificate {
    pub property: &'static str,
    pub semantic_profile: &'static str,
    pub scope: &'static str,
    pub evidence_version: &'static str,
    pub assumptions: Vec<&'static str>,
    pub source: AtomicPublicationSourceCertificate,
    pub model: CoreCertificate,
    /// Card CG29: the extracted shape checked over every candidate
    /// execution (`publication_fragment`).
    pub execution_coverage: CoverageEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicPublicationRefusal {
    Source(AtomicPublicationSourceRefusal),
    Model(qcore_core::AtomicPublicationRefusal),
    /// Card CG29: the extracted shape was not shown race-free over every
    /// candidate execution (a racing execution, or an exceeded bound).
    ExecutionCoverage(Box<ProgramVerdict>),
}

#[must_use]
fn core_order(order: AtomicPublicationOrder) -> MemoryOrder {
    match order {
        AtomicPublicationOrder::Relaxed => MemoryOrder::Relaxed,
        AtomicPublicationOrder::Acquire => MemoryOrder::Acquire,
        AtomicPublicationOrder::Release => MemoryOrder::Release,
        AtomicPublicationOrder::SequentiallyConsistent => MemoryOrder::SequentiallyConsistent,
    }
}

pub fn check_atomic_publication_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &AtomicPublicationSourceProfile<'_>,
    bounds: AtomicPublicationExecutionBounds,
) -> Result<AtomicPublicationCertificate, AtomicPublicationRefusal> {
    let source = extract_atomic_publication_source(unit, source_revision, profile, bounds)
        .map_err(AtomicPublicationRefusal::Source)?;
    let model = check_atomic_publication(&AtomicPublicationProgram {
        flag_location: 0,
        payload_location: 1,
        initial_value: source.initial_value,
        published_value: source.published_value,
        observed_value: source.successful_observation,
        store_order: core_order(source.store_order),
        load_order: core_order(source.load_order),
    })
    .map_err(AtomicPublicationRefusal::Model)?;
    let execution_coverage =
        match check_program_executions(&publication_fragment(&source), PUBLICATION_COVERAGE_BOUNDS)
        {
            ProgramVerdict::RaceFree(evidence) => evidence,
            other => return Err(AtomicPublicationRefusal::ExecutionCoverage(Box::new(other))),
        };
    Ok(AtomicPublicationCertificate {
        property: "successful release/acquire publication orders one payload write before one payload read",
        semantic_profile: "c11-bounded-atomic-publication-v1",
        scope: source.scope,
        evidence_version: "ct41-sync-model-v1",
        assumptions: covered_assumptions(),
        source,
        model,
        execution_coverage,
    })
}

/// Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z): the
/// bounds of the extracted shape, the ones `ALL_EXECUTIONS_ASSUMPTION`
/// states. Threads: the publisher and the consumer. Events: the flag's
/// initialisation, two spawns and two instructions per thread. Values: the
/// initial and the published value. Candidates: at most 2 paths, 2
/// modification orders and 2 S orders.
pub const PUBLICATION_COVERAGE_BOUNDS: CoverageBounds = CoverageBounds {
    threads: 2,
    events: 7,
    values: 2,
    candidates: 16,
};

/// The extracted shape as a fragment: the flag (location 0) initialised,
/// the publisher's payload write (location 1) and release store, the
/// consumer's load guarding its payload read (the extractor admits the use
/// only under `seen == READY`, or after a `seen != READY` early return).
#[must_use]
pub fn publication_fragment(source: &AtomicPublicationSourceCertificate) -> ProgramFragment {
    ProgramFragment {
        atomic_locations: vec![(0, source.initial_value)],
        threads: vec![
            vec![
                GuardedInstruction::always(Instruction::NonAtomicWrite { location: 1 }),
                GuardedInstruction::always(Instruction::Store {
                    location: 0,
                    value: source.published_value,
                    order: core_order(source.store_order),
                }),
            ],
            vec![
                GuardedInstruction::always(Instruction::Load {
                    location: 0,
                    order: core_order(source.load_order),
                    register: 0,
                }),
                GuardedInstruction::when(
                    0,
                    source.successful_observation,
                    Instruction::NonAtomicRead { location: 1 },
                ),
            ],
        ],
    }
}

/// The assumptions an `Unknown` publication verdict carries
/// (`QCore.AtomicSourceRoute.publicationVerdict`): card CG19's single-graph
/// assumption (W07; no liveness, W11).
fn publication_assumptions() -> Vec<&'static str> {
    vec![
        "the source certificate was derived from the full typed translation-unit AST",
        "the publisher invocation and successful consumer invocation bounds are caller-declared premises",
        "the Rust weak-memory checker corresponds to the audited Lean abstraction by review",
        qcore_c_gateway::atomic_source_route::SINGLE_GRAPH_ASSUMPTION,
    ]
}

/// The assumptions a `Safe` publication verdict carries: card CG29 replaces
/// the single-graph assumption with the all-executions one, because the
/// shape was also checked over every candidate execution.
fn covered_assumptions() -> Vec<&'static str> {
    vec![
        "the source certificate was derived from the full typed translation-unit AST",
        "the publisher invocation and successful consumer invocation bounds are caller-declared premises",
        "the Rust weak-memory checker corresponds to the audited Lean abstraction by review",
        qcore_c_gateway::atomic_source_route::ALL_EXECUTIONS_ASSUMPTION,
    ]
}

/// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the production gate of
/// the composed judgment. `Safe` exactly when the source extraction and the
/// core model both accept and (card CG29, rule 11z) the extracted shape is
/// race-free over every candidate execution; every refusal is `Unknown`
/// with its reason. `Safe` carries the all-executions assumption, `Unknown`
/// the single-graph one (`QCore.AtomicSourceRoute.publicationVerdict`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicPublicationVerdict {
    Safe(Box<AtomicPublicationCertificate>),
    Unknown {
        refusal: AtomicPublicationRefusal,
        assumptions: Vec<&'static str>,
    },
}

impl AtomicPublicationVerdict {
    #[must_use]
    pub fn status(&self) -> &'static str {
        match self {
            Self::Safe(_) => "Safe",
            Self::Unknown { .. } => "Unknown",
        }
    }

    #[must_use]
    pub fn assumptions(&self) -> &[&'static str] {
        match self {
            Self::Safe(certificate) => &certificate.assumptions,
            Self::Unknown { assumptions, .. } => assumptions,
        }
    }
}

pub fn atomic_publication_verdict(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &AtomicPublicationSourceProfile<'_>,
    bounds: AtomicPublicationExecutionBounds,
) -> AtomicPublicationVerdict {
    match check_atomic_publication_source(unit, source_revision, profile, bounds) {
        Ok(certificate) => AtomicPublicationVerdict::Safe(Box::new(certificate)),
        Err(refusal) => AtomicPublicationVerdict::Unknown {
            refusal,
            assumptions: publication_assumptions(),
        },
    }
}
