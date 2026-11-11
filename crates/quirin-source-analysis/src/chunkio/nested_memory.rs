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

//! Bind the checked context/stream traversal and allocation-release chain to
//! concrete memory-backed model identities. Caller C provenance remains a premise.
use super::{
    PINNED_FLUENT_BIT_REVISION,
    memory_close::MemoryCloseCertificate,
    parent_teardown::{ParentKind, ParentSource},
    traversal::{TraversalKind, TraversalSource},
};
use crate::chunk_lifecycle::{ChunkOperation, execute_chunk_prefix};
use serde::Serialize;
use std::collections::BTreeSet;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/nested_memory_teardown.qv"
);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NestedMemorySource {
    chunks: TraversalSource,
    streams: TraversalSource,
    stream_parent: ParentSource,
    context_parent: ParentSource,
    memory: MemoryCloseCertificate,
    contract_source: String,
}

/// The caller supplies actual ring order and its C-to-model identity binding.
/// Set equality alone does not establish native pointer provenance or order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StreamRing {
    pub stream: u32,
    pub chunks: Vec<u32>,
}

pub fn check_nested_memory_source(
    chunks: &TraversalSource,
    streams: &TraversalSource,
    stream_parent: &ParentSource,
    context_parent: &ParentSource,
    memory: &MemoryCloseCertificate,
) -> Result<NestedMemorySource, String> {
    if chunks.kind() != TraversalKind::Chunks
        || streams.kind() != TraversalKind::Streams
        || stream_parent.kind() != ParentKind::Stream
        || context_parent.kind() != ParentKind::Context
    {
        return Err("nested teardown dependency roles mismatch".into());
    }
    for source in [
        chunks.source(),
        streams.source(),
        stream_parent.source(),
        context_parent.source(),
    ] {
        if source.source_revision() != PINNED_FLUENT_BIT_REVISION {
            return Err("nested teardown dependency revision mismatch".into());
        }
    }
    // Includes the backend, reset, list unlink and total counter source checks.
    let memory = memory.recheck()?;
    check_policy(CONTRACT)?;
    Ok(NestedMemorySource {
        chunks: chunks.clone(),
        streams: streams.clone(),
        stream_parent: stream_parent.clone(),
        context_parent: context_parent.clone(),
        memory,
        contract_source: CONTRACT.into(),
    })
}

impl NestedMemorySource {
    pub(super) fn recheck(&self) -> Result<Self, String> {
        check_nested_memory_source(
            &self.chunks,
            &self.streams,
            &self.stream_parent,
            &self.context_parent,
            &self.memory,
        )
    }
    pub(super) fn memory(&self) -> &MemoryCloseCertificate {
        &self.memory
    }
    /// Return source-bound teardown operations, checked against an existing
    /// model execution prefix. This is conditional source/model composition,
    /// not a native allocation-provenance or whole-program Safe certificate.
    /// The full program must still pass check_chunk_program for terminal safety.
    pub fn instantiate(
        &self,
        prefix: &[ChunkOperation],
        context: Option<u32>,
        rings: &[StreamRing],
    ) -> Result<Vec<ChunkOperation>, String> {
        if prefix.len() > 4096
            || rings.len() > 512
            || rings.iter().map(|r| r.chunks.len()).sum::<usize>() > 512
        {
            return Err("nested teardown exceeds bounded identity profile".into());
        }
        let state = execute_chunk_prefix(prefix)
            .map_err(|e| format!("invalid lifecycle prefix: {e:?}"))?
            .final_state;
        let Some(context) = context else {
            return if rings.is_empty() {
                Ok(vec![])
            } else {
                Err("null context has nonempty ring binding".into())
            };
        };
        let record = state
            .contexts
            .get(&context)
            .filter(|c| c.live)
            .ok_or("nested teardown requires a live context")?;
        let selected: BTreeSet<_> = rings.iter().map(|r| r.stream).collect();
        if selected.len() != rings.len() || selected != record.streams {
            return Err("context ring has missing, duplicate or foreign stream identities".into());
        }
        let mut operations = Vec::new();
        let mut chunks = BTreeSet::new();
        for ring in rings {
            let stream = state
                .streams
                .get(&ring.stream)
                .filter(|s| s.live && s.context == context)
                .ok_or("stream lacks live matching context identity")?;
            let members: BTreeSet<_> = ring.chunks.iter().copied().collect();
            if members.len() != ring.chunks.len() || members != stream.chunks {
                return Err("chunk ring has missing, duplicate or foreign identities".into());
            }
            for &chunk in &ring.chunks {
                if !chunks.insert(chunk) {
                    return Err("child identity occurs in multiple streams".into());
                }
                // In particular, filesystem chunks and outstanding lock/busy/
                // task/route/transaction obligations fail closed here.
                operations.extend(self.memory.instantiate(&state, Some(chunk), false)?);
            }
            // All selected children have already been closed. This operation
            // cannot hide an unchecked nested child because ring membership is exact.
            operations.push(ChunkOperation::DestroyStream {
                stream: ring.stream,
            });
        }
        if record.chunk_total != chunks.len() {
            return Err(
                "context counter does not match the complete child identity inventory".into(),
            );
        }
        operations.push(ChunkOperation::DestroyContext { context });
        let mut combined = prefix.to_vec();
        combined.extend(operations.clone());
        let after = execute_chunk_prefix(&combined)
            .map_err(|e| format!("nested lifecycle discharge failed: {e:?}"))?
            .final_state;
        if after.contexts[&context].live
            || after.contexts[&context].chunk_total != 0
            || rings.iter().any(|r| after.streams[&r.stream].live)
            || chunks.iter().any(|c| after.chunks[c].live)
        {
            return Err("nested teardown left a selected model identity live".into());
        }
        Ok(operations)
    }
}

fn check_policy(contract: &str) -> Result<(), String> {
    crate::contract_observation::check_contract_facts(
        contract,
        "nested_memory_teardown",
        &[
            ("all_component_source_contract_premises", 1),
            (
                "caller_binds_native_allocations_and_ring_order_to_model_identities",
                1,
            ),
            (
                "complete_distinct_live_memory_children_and_exact_context_counter",
                1,
            ),
            (
                "parent_owned_fields_disjoint_from_all_child_and_unrelated_storage",
                1,
            ),
            ("exclusive_access_and_exact_cross_unit_abi_call_bindings", 1),
            ("complete_context_stream_chunk_backend_source_chain", 1),
            ("each_selected_chunk_closed_once_before_its_stream", 1),
            ("all_selected_streams_destroyed_before_context", 1),
            ("filesystem_cleanup_inferred_from_memory_chain", 0),
            ("terminal_safety_requires_complete_program_check", 1),
        ],
    )
}

#[cfg(test)]
#[path = "../../tests/unit/nested_memory_teardown.rs"]
pub(crate) mod tests;
