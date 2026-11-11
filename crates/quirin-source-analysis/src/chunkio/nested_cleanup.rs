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

//! Conditional mixed-backend traversal cases. These retain each child's OS
//! residual after stream/context allocation release; no native outcome is inferred.
use super::{
    file_close::ResidualCustody, filesystem_chunk_close::FilesystemChunkCloseEvidence,
    nested_memory::NestedMemorySource, unsupported_close::UnsupportedCloseEvidence,
};
use serde::Serialize;
use std::collections::BTreeSet;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/nested_cleanup.qv"
);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NestedCleanupSource {
    chain: NestedMemorySource,
    filesystem: FilesystemChunkCloseEvidence,
    contract_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unsupported: Option<UnsupportedCloseEvidence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ChildCase {
    Memory {
        chunk: u32,
    },
    Filesystem {
        chunk: u32,
        case: usize,
    },
    /// Conditional live input only: no native constructor is inferred to have
    /// returned a chunk with this unrecognized stream discriminant.
    Unsupported {
        chunk: u32,
        kind: i32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConditionalStream {
    pub stream: u32,
    pub chunks: Vec<ChildCase>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChildResidual {
    pub stream: u32,
    pub chunk: u32,
    pub mapping: ResidualCustody,
    pub descriptor: ResidualCustody,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpaqueChildResidual {
    pub stream: u32,
    pub chunk: u32,
    pub backend_kind: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NestedCleanupCase {
    context: u32,
    selected_streams: Vec<ConditionalStream>,
    released_chunks: Vec<u32>,
    released_streams: Vec<u32>,
    context_released: bool,
    total_counter_after: usize,
    up_counter_after: usize,
    residuals: Vec<ChildResidual>,
    /// Unclassified backend custody includes any nested memory/OS obligations;
    /// absence from the typed OS residual list is not evidence of their absence.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    opaque_residuals: Vec<OpaqueChildResidual>,
}
impl NestedCleanupCase {
    pub fn residuals(&self) -> &[ChildResidual] {
        &self.residuals
    }
    pub fn opaque_residuals(&self) -> &[OpaqueChildResidual] {
        &self.opaque_residuals
    }
    pub fn up_counter_after(&self) -> usize {
        self.up_counter_after
    }
    pub fn released_chunks(&self) -> &[u32] {
        &self.released_chunks
    }
    /// Conditional source gate only. Caller provenance and observed OS outcomes
    /// still need independent binding even when this gate accepts.
    pub fn require_conditional_discharge(&self) -> Result<(), String> {
        if !self.residuals.is_empty()
            || !self.opaque_residuals.is_empty()
            || self.up_counter_after != 0
        {
            Err("Unknown: parent release retains child backend/OS/counter obligations".into())
        } else {
            Ok(())
        }
    }
}

pub fn check_nested_cleanup_source(
    chain: &NestedMemorySource,
    filesystem: &FilesystemChunkCloseEvidence,
) -> Result<NestedCleanupSource, String> {
    let chain = chain.recheck()?;
    let filesystem = filesystem.recheck()?;
    if chain.memory() != filesystem.memory_scope() {
        return Err("nested filesystem and memory dispatch source/dependencies differ".into());
    }
    Ok(NestedCleanupSource {
        chain,
        filesystem,
        contract_source: CONTRACT.into(),
        unsupported: None,
    })
}
impl NestedCleanupSource {
    /// Bind the independently rechecked symbolic dispatch to this exact chain.
    /// The existing constructor remains fail-closed for unsupported child cases.
    pub fn with_unsupported(&self, evidence: &UnsupportedCloseEvidence) -> Result<Self, String> {
        let mut source = check_nested_cleanup_source(&self.chain, &self.filesystem)?;
        let evidence = evidence.recheck()?;
        if evidence.memory_scope() != source.chain.memory() {
            return Err("unsupported child and parent chain source/dependencies differ".into());
        }
        source.unsupported = Some(evidence);
        Ok(source)
    }
    /// Select conditional child source cases, not runtime observations. Each
    /// selected case's entry/frame/outcome premises must be bound externally.
    pub fn project(
        &self,
        context: u32,
        streams: &[ConditionalStream],
        total_entry: usize,
        up_entry: usize,
    ) -> Result<NestedCleanupCase, String> {
        self.project_with_contract(context, streams, total_entry, up_entry, CONTRACT)
    }
    fn project_with_contract(
        &self,
        context: u32,
        streams: &[ConditionalStream],
        total_entry: usize,
        up_entry: usize,
        contract: &str,
    ) -> Result<NestedCleanupCase, String> {
        if streams.len() > 512 || total_entry > 512 {
            return Err("nested cleanup exceeds bounded identity profile".into());
        }
        let mut stream_ids = BTreeSet::new();
        let mut chunk_ids = BTreeSet::new();
        let mut residuals = Vec::new();
        let mut opaque_residuals = Vec::new();
        let mut released_chunks = Vec::new();
        let mut released_streams = Vec::new();
        let mut up = up_entry;
        for stream in streams {
            if !stream_ids.insert(stream.stream) || stream.chunks.len() > 512 {
                return Err("duplicate stream or oversized chunk ring".into());
            }
            for child in &stream.chunks {
                let (chunk, mapping, descriptor, decrements) = match *child {
                    ChildCase::Unsupported { chunk, kind } => {
                        let evidence = self.unsupported.as_ref().ok_or(
                            "unsupported child requires source-bound symbolic close evidence",
                        )?;
                        if evidence.excluded_kinds().contains(&kind) {
                            return Err(
                                "known backend cannot select an unsupported child case".into()
                            );
                        }
                        opaque_residuals.push(OpaqueChildResidual {
                            stream: stream.stream,
                            chunk,
                            backend_kind: kind,
                        });
                        // Only the outer releases/counter decrement are composed.
                        // All backend obligations remain in the opaque inventory.
                        (chunk, ResidualCustody::Absent, ResidualCustody::Absent, 0)
                    }
                    ChildCase::Memory { chunk } => {
                        (chunk, ResidualCustody::Absent, ResidualCustody::Absent, 0)
                    }
                    ChildCase::Filesystem { chunk, case } => {
                        let c = self
                            .filesystem
                            .cases()
                            .get(case)
                            .ok_or("filesystem child case out of range")?;
                        if !c.chunk_present
                            || c.delete
                            || !c.chunk_released
                            || c.total_counter_decrements != 1
                        {
                            return Err("stream destroy requires a nonnull non-deleting complete chunk case".into());
                        }
                        (chunk, c.mapping, c.descriptor, c.up_counter_decrements)
                    }
                };
                if !chunk_ids.insert(chunk) || chunk_ids.len() > total_entry {
                    return Err("duplicate child identity or counter below ring inventory".into());
                }
                up = up
                    .checked_sub(decrements)
                    .ok_or("nested cleanup up-counter underflow")?;
                released_chunks.push(chunk);
                if mapping != ResidualCustody::Absent || descriptor != ResidualCustody::Absent {
                    residuals.push(ChildResidual {
                        stream: stream.stream,
                        chunk,
                        mapping,
                        descriptor,
                    });
                }
            }
            released_streams.push(stream.stream);
        }
        if chunk_ids.len() != total_entry {
            return Err("context total counter differs from complete ring inventory".into());
        }
        crate::contract_observation::check_contract_facts(
            contract,
            "nested_cleanup",
            &[
                (
                    "all_source_component_and_child_case_entry_outcome_premises",
                    1,
                ),
                (
                    "complete_finite_live_reciprocal_rings_and_distinct_owned_provenance",
                    1,
                ),
                (
                    "exclusive_access_and_exact_native_identity_order_abi_call_bindings",
                    1,
                ),
                (
                    "parent_fields_and_saved_successors_disjoint_from_child_releases",
                    1,
                ),
                ("entry_total", total_entry as i128),
                ("released_chunks", released_chunks.len() as i128),
                ("entry_streams", streams.len() as i128),
                ("released_streams", released_streams.len() as i128),
                ("context_released", 1),
                ("total_after", 0),
                ("up_after", up as i128),
                ("entry_up", up_entry as i128),
                ("up_decrements", (up_entry - up) as i128),
                (
                    "residual_child_count",
                    (residuals.len() + opaque_residuals.len()) as i128,
                ),
                (
                    "post_parent_residual_child_count",
                    (residuals.len() + opaque_residuals.len()) as i128,
                ),
                (
                    "conditional_discharge_permitted",
                    i128::from(residuals.is_empty() && opaque_residuals.is_empty() && up == 0),
                ),
                ("backing_deletion_established", 0),
            ],
        )?;
        Ok(NestedCleanupCase {
            context,
            selected_streams: streams.to_vec(),
            released_chunks,
            released_streams,
            context_released: true,
            total_counter_after: 0,
            up_counter_after: up,
            residuals,
            opaque_residuals,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/nested_cleanup.rs"]
mod tests;
