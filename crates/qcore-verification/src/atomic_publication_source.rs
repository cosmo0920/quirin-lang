//! Composition of closed C source extraction with the core publication model.

pub use qcore_c_gateway::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationSourceProfile,
};
use qcore_c_gateway::atomic_publication_source::{
    AtomicPublicationOrder, AtomicPublicationSourceCertificate, AtomicPublicationSourceRefusal,
    extract_atomic_publication_source,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicPublicationRefusal {
    Source(AtomicPublicationSourceRefusal),
    Model(qcore_core::AtomicPublicationRefusal),
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
    Ok(AtomicPublicationCertificate {
        property: "successful release/acquire publication orders one payload write before one payload read",
        semantic_profile: "c11-bounded-atomic-publication-v1",
        scope: source.scope,
        evidence_version: "ct41-sync-model-v1",
        assumptions: vec![
            "the source certificate was derived from the full typed translation-unit AST",
            "the publisher invocation and successful consumer invocation bounds are caller-declared premises",
            "the Rust weak-memory checker corresponds to the audited Lean abstraction by review",
        ],
        source,
        model,
    })
}
