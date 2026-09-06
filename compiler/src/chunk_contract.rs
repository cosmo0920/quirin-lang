//! Source-checked compatibility boundary for the standalone contract evaluator.
//! Raw schema records yield policy evaluations only; these entry points require
//! opaque summaries constructed by the typed C importer.

use crate::verifier_plugins::chunkio::{ChunkFrameSummary, ChunkSourceSummary};
use qcore_contract::chunk_contract::{
    ChunkContractEvaluation, evaluate_chunk_frame_contract, evaluate_chunk_source_contract,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkContractCertificate(ChunkContractEvaluation);

impl ChunkContractCertificate {
    pub fn function(&self) -> &str {
        self.0.function()
    }

    pub fn contract_source(&self) -> &str {
        self.0.contract_source()
    }

    pub fn checked_states(&self) -> usize {
        self.0.checked_states()
    }

    pub fn dependency_sources(&self) -> &std::collections::BTreeMap<String, String> {
        self.0.dependency_sources()
    }
}

impl std::ops::Deref for ChunkContractCertificate {
    type Target = ChunkContractEvaluation;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn check_chunk_frame_contract(
    summary: &ChunkFrameSummary,
    source: &str,
) -> Result<usize, String> {
    evaluate_chunk_frame_contract(summary, source)
}

pub fn check_chunk_source_contract(
    summary: &ChunkSourceSummary,
    source: &str,
) -> Result<ChunkContractCertificate, String> {
    evaluate_chunk_source_contract(summary, source).map(ChunkContractCertificate)
}

#[cfg(test)]
#[path = "../tests/unit/chunk_contract.rs"]
mod tests;
