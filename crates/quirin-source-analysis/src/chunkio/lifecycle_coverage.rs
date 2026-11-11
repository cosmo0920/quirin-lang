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

//! The ChunkIO half of the selected-function lifecycle accounting.
//!
//! The decision procedure lives in
//! `qcore_verification::chunk_lifecycle_coverage`. What stays here is which
//! functions the pinned artifact selected, what its projection said about each
//! of them, and how an already-rechecked ChunkIO teardown certificate turns
//! into the model program whose legs that procedure derives.
//!
//! Two boundaries are deliberate.
//!
//! * A lifecycle role is never guessed from a function's name. It arrives with
//!   a rechecked certificate or the function stays `Unclassified`, carrying the
//!   pinned projection's own refusal as its reason.
//! * A checked *field* contract is not a lifecycle result. Rows that carry one
//!   are still `Unclassified`; their reason says so in those words, so the
//!   distinction cannot be lost between this module and a published row.

use std::collections::BTreeMap;

use qcore_verification::chunk_lifecycle_coverage::{
    DeleteOnlyPostState, FunctionIdentity, LifecycleEvidence, LifecycleRole, ModelComposition,
    SourceDerivation,
};
use qcore_verification::memory_close_composition::MemoryCloseCertificate;

use super::PINNED_FLUENT_BIT_REVISION;
use super::nested_memory::{NestedMemorySource, StreamRing};
use super::traversal::{TraversalKind, TraversalSource};
use crate::chunk_lifecycle::{ChunkBackend, ChunkOperation, ChunkProgram, ChunkResidency};

/// The single context/stream/chunk identity triple every composition below is
/// instantiated at. The kernel's identity profile is bounded, so a composition
/// is taken at concrete identities rather than over a symbolic ring.
const CONTEXT: u32 = 0;
const STREAM: u32 = 0;
const CHUNK: u32 = 0;

fn opened(backend: ChunkBackend, residency: ChunkResidency) -> Vec<ChunkOperation> {
    vec![
        ChunkOperation::CreateContext { context: CONTEXT },
        ChunkOperation::CreateStream {
            context: CONTEXT,
            stream: STREAM,
        },
        ChunkOperation::OpenChunk {
            context: CONTEXT,
            stream: STREAM,
            chunk: CHUNK,
            backend,
            residency,
        },
    ]
}

fn state_of(operations: &[ChunkOperation]) -> Result<crate::chunk_lifecycle::ChunkState, String> {
    crate::chunk_lifecycle::execute_chunk_prefix(operations)
        .map(|execution| execution.final_state)
        .map_err(|diagnostics| format!("lifecycle prefix is not executable: {diagnostics:?}"))
}

/// Compose the rechecked null/memory-backed `cio_chunk_close` certificate into
/// the model prefix it actually reaches.
///
/// The certificate supplies the operations; this function only places them
/// after a live context, stream and memory-backed chunk. The context and
/// stream stay live on purpose: `cio_chunk_close` requires them to outlive the
/// chunk, and a required premise is not a discharged obligation.
pub fn compose_memory_close(
    program_digest: impl Into<String>,
    certificate: &MemoryCloseCertificate,
    delete: bool,
) -> Result<ModelComposition, String> {
    ModelComposition::prefix(program_digest, &memory_close_program(certificate, delete)?)
}

/// The model program [`compose_memory_close`] composes, so a caller can bind
/// its digest to the exact operations.
pub fn memory_close_program(
    certificate: &MemoryCloseCertificate,
    delete: bool,
) -> Result<ChunkProgram, String> {
    let mut operations = opened(ChunkBackend::Memory, ChunkResidency::Down);
    let state = state_of(&operations)?;
    operations.extend(certificate.instantiate(&state, Some(CHUNK), delete)?);
    Ok(ChunkProgram { operations })
}

/// A content digest of a model program bound to the unit it was derived from.
/// The operations are rendered with their derived `Debug` form, which names
/// every operation and identity.
pub fn bound_program_digest(unit_sha256: &str, function: &str, program: &ChunkProgram) -> String {
    format!(
        "sha256:{}",
        qcore_schemas::sha256_hex(
            format!("{unit_sha256}\n{function}\n{:?}", program.operations).as_bytes()
        )
    )
}

/// Compose the rechecked nested context → stream → chunk → backend teardown
/// into a terminal model program.
///
/// This is the only shape in the ChunkIO evidence that reaches every model leg.
/// It belongs to the nested destructor chain, not to any single selected
/// `cio_chunk.c` or `flb_input_chunk.c` entry point.
pub fn compose_nested_teardown(
    program_digest: impl Into<String>,
    source: &NestedMemorySource,
) -> Result<ModelComposition, String> {
    let prefix = opened(ChunkBackend::Memory, ChunkResidency::Down);
    let teardown = source.instantiate(
        &prefix,
        Some(CONTEXT),
        &[StreamRing {
            stream: STREAM,
            chunks: vec![CHUNK],
        }],
    )?;
    let mut operations = prefix;
    operations.extend(teardown);
    ModelComposition::terminal(program_digest, &ChunkProgram { operations })
}

/// Compose the rechecked chunk-ring traversal (`cio_chunk_close_stream`) with
/// the rechecked null/memory `cio_chunk_close` certificate over one stream whose
/// ring holds `members` memory-backed chunks.
///
/// Card `W43-CHUNK-SOURCE-PROFILE`. The traversal certificate establishes that
/// the body visits every ring member once with a saved next pointer and passes
/// only the current child, with `delete` zero; the close certificate supplies
/// each member's operations. The ring is the stream's exact model membership, so
/// a member cannot be skipped. The stream and its context stay live: the
/// traversal destroys neither, and that is recorded as retained, not discharged.
pub fn compose_stream_ring_close(
    program_digest: impl Into<String>,
    traversal: &TraversalSource,
    certificate: &MemoryCloseCertificate,
    members: u32,
) -> Result<ModelComposition, String> {
    ModelComposition::prefix(
        program_digest,
        &stream_ring_close_program(traversal, certificate, members)?,
    )
}

/// The model program [`compose_stream_ring_close`] composes.
pub fn stream_ring_close_program(
    traversal: &TraversalSource,
    certificate: &MemoryCloseCertificate,
    members: u32,
) -> Result<ChunkProgram, String> {
    if traversal.kind() != TraversalKind::Chunks {
        return Err("a stream ring close needs the chunk-ring traversal".into());
    }
    if traversal.source().source_revision() != PINNED_FLUENT_BIT_REVISION {
        return Err("chunk-ring traversal revision mismatch".into());
    }
    if members == 0 {
        return Err("a stream ring close over an empty ring discharges nothing".into());
    }
    // Includes the backend, reset, list unlink and total counter source checks.
    let certificate = certificate.recheck()?;
    let mut operations = vec![
        ChunkOperation::CreateContext { context: CONTEXT },
        ChunkOperation::CreateStream {
            context: CONTEXT,
            stream: STREAM,
        },
    ];
    for chunk in 0..members {
        operations.push(ChunkOperation::OpenChunk {
            context: CONTEXT,
            stream: STREAM,
            chunk,
            backend: ChunkBackend::Memory,
            residency: ChunkResidency::Down,
        });
    }
    let state = state_of(&operations)?;
    let ring = state
        .streams
        .get(&STREAM)
        .filter(|stream| stream.live)
        .map(|stream| stream.chunks.clone())
        .ok_or("the ring's stream is not live")?;
    for chunk in ring {
        let current = state_of(&operations)?;
        // `chunk_delete_argument_zero` is a checked traversal fact.
        operations.extend(certificate.instantiate(&current, Some(chunk), false)?);
    }
    Ok(ChunkProgram { operations })
}

/// Compose a `cio_stream_delete`-shaped partial operation: the stream's content
/// is deleted and the stream itself stays live.
pub fn compose_stream_content_delete(
    program_digest: impl Into<String>,
    filesystem_delete_succeeds: bool,
) -> Result<DeleteOnlyPostState, String> {
    let mut operations = opened(ChunkBackend::File, ChunkResidency::Up);
    operations.push(ChunkOperation::DeleteStreamContent {
        stream: STREAM,
        filesystem_delete_succeeds,
    });
    DeleteOnlyPostState::from_program(program_digest, &ChunkProgram { operations })
}

/// What the pinned artifact recorded about one selected function's bounded
/// field contract. Neither arm is a lifecycle result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldContract {
    Checked { states: usize },
    Refused { reason: String, reason_code: String },
}

/// One function the pinned source/build profile selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedFunction {
    pub identity: FunctionIdentity,
    pub field_contract: FieldContract,
}

/// A lifecycle classification the caller obtained from rechecked evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassifiedFunction {
    pub function: String,
    pub role: LifecycleRole,
    pub model: Option<ModelComposition>,
    pub delete_only: Option<DeleteOnlyPostState>,
    pub error_exits_modeled: bool,
    pub wrapper_obligations_discharged: bool,
    /// The pinned unit the composition was derived from and the source paths
    /// it composes (card `W43-CHUNK-SOURCE-PROFILE`).
    pub derivation: Option<SourceDerivation>,
}

/// Turn the pinned selection and the rechecked classifications into the
/// evidence rows the verification adapter decides.
///
/// A classification naming a function outside the selection, naming one twice,
/// or claiming [`LifecycleRole::Unclassified`] is refused: the unclassified
/// arm is this module's own fail-closed default and is not a caller input.
pub fn derive_lifecycle_evidence(
    selected: &[SelectedFunction],
    classified: Vec<ClassifiedFunction>,
) -> Result<Vec<LifecycleEvidence>, String> {
    let mut rows: BTreeMap<String, ClassifiedFunction> = BTreeMap::new();
    for row in classified {
        if row.role == LifecycleRole::Unclassified {
            return Err(format!(
                "`{}` cannot be classified as unclassified; omit it instead",
                row.function
            ));
        }
        if !selected
            .iter()
            .any(|entry| entry.identity.function == row.function)
        {
            return Err(format!(
                "lifecycle classification names unselected function `{}`",
                row.function
            ));
        }
        if rows.insert(row.function.clone(), row).is_some() {
            return Err("lifecycle classification names a function twice".into());
        }
    }
    let mut evidence = Vec::with_capacity(selected.len());
    for entry in selected {
        match rows.remove(&entry.identity.function) {
            Some(row) => evidence.push(LifecycleEvidence {
                identity: entry.identity.clone(),
                role: row.role,
                projection_reason: None,
                model: row.model,
                delete_only: row.delete_only,
                error_exits_modeled: row.error_exits_modeled,
                wrapper_obligations_discharged: row.wrapper_obligations_discharged,
                derivation: row.derivation,
            }),
            None => evidence.push(LifecycleEvidence {
                identity: entry.identity.clone(),
                role: LifecycleRole::Unclassified,
                projection_reason: Some(projection_reason(&entry.field_contract)),
                model: None,
                delete_only: None,
                error_exits_modeled: false,
                wrapper_obligations_discharged: false,
                derivation: None,
            }),
        }
    }
    Ok(evidence)
}

fn projection_reason(contract: &FieldContract) -> String {
    match contract {
        FieldContract::Checked { states } => format!(
            "checked field contract over {states} Boolean field state(s) only; a field contract \
             establishes no lifecycle post-state"
        ),
        FieldContract::Refused {
            reason,
            reason_code,
        } => format!("pinned projection refusal ({reason_code}): {reason}"),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/lifecycle_coverage.rs"]
mod tests;
