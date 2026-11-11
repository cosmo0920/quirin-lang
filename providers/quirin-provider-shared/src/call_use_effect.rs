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

//! A schemas-only proposal producer for the bounded ChunkIO filesystem
//! cleanup-attempt family.
//!
//! This process reads the supplied typed paths independently and computes what
//! they do: for each complete path, every call in operation order, and every
//! later operation of that path which observes the call's result — separating
//! the subset that are guard conditions. It evaluates no contract, applies no
//! policy, and receives no expected result: the request carries paths and
//! nothing else. The receiver recomputes the same def/use reading from its own
//! import and evaluates the pinned contract itself before anything is
//! admitted.
//!
//! Nothing here observes or claims OS behaviour. A `close` operation in a path
//! is a recorded call; whether a descriptor was released is not a question this
//! process can even express.
use qcore_schemas::SemanticWitness;
use qcore_schemas::call_use_summary::{
    CallResultUseRecord, CallResultUseWitness, CallUseExpressionRecord, CallUseFamilyInput,
    CallUseFunctionRecord, CallUseOperationRecord, CallUseProposalRecord, CallUseRecord,
    CallUseSourceInput, CallUseValueRecord, PathCallUseRecord,
};
use qcore_schemas::plugin::*;
use std::collections::BTreeSet;

/// Bounds applied to a supplied projection before any structural reading.
/// These apply to direct Rust callers as well as to transported JSON.
const MAX_PATHS: usize = 64;
const MAX_OPERATIONS: usize = 512;
/// Depth cap on the expression tree walked for symbol dependencies. The
/// importer's own vocabulary is shallow; a deeper tree is declined rather than
/// recursed into, so a hostile projection cannot exhaust the stack.
const MAX_EXPRESSION_DEPTH: u32 = 32;

fn bounded(function: &CallUseFunctionRecord) -> bool {
    !function.function.is_empty()
        && !function.source_revision.is_empty()
        && !function.signature.is_empty()
        && !function.paths.is_empty()
        && function.paths.len() <= MAX_PATHS
        && function
            .paths
            .iter()
            .all(|path| !path.operations.is_empty() && path.operations.len() <= MAX_OPERATIONS)
}

/// Collect the operation indices this value depends on. Returns `None` when
/// the expression is deeper than the cap, so the whole projection is declined
/// rather than partially read.
fn symbols(value: &CallUseValueRecord, depth: u32, out: &mut BTreeSet<u32>) -> Option<()> {
    if depth > MAX_EXPRESSION_DEPTH {
        return None;
    }
    match &value.expression {
        CallUseExpressionRecord::Symbol { operation } => {
            out.insert(*operation);
        }
        CallUseExpressionRecord::Cast { value, .. }
        | CallUseExpressionRecord::Unary { value, .. } => symbols(value, depth + 1, out)?,
        CallUseExpressionRecord::Field { base, .. } => symbols(base, depth + 1, out)?,
        CallUseExpressionRecord::Binary { left, right, .. } => {
            symbols(left, depth + 1, out)?;
            symbols(right, depth + 1, out)?;
        }
        CallUseExpressionRecord::Parameter { .. }
        | CallUseExpressionRecord::Integer { .. }
        | CallUseExpressionRecord::String { .. } => (),
    }
    Some(())
}

/// The def/use reading of one complete path.
///
/// A use is any later operation of the same path whose own operands depend on
/// the call's produced value: storing it, passing it, returning it, computing
/// with it or guarding on it. A guard use is additionally recorded. Reloads
/// through memory are not inferred to retain a stored result's identity, which
/// matches the receiver's own analyzer.
fn read_path(operations: &[CallUseOperationRecord]) -> Option<Vec<CallUseRecord>> {
    let mut calls: Vec<CallUseRecord> = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let index = u32::try_from(index).ok()?;
        let mut used = BTreeSet::new();
        let guard = matches!(operation, CallUseOperationRecord::Guard { .. });
        match operation {
            CallUseOperationRecord::Read { place } => symbols(place, 0, &mut used)?,
            CallUseOperationRecord::Write { place, value } => {
                symbols(place, 0, &mut used)?;
                symbols(value, 0, &mut used)?;
            }
            CallUseOperationRecord::LocalWrite { value, .. }
            | CallUseOperationRecord::ReturnValue { value } => symbols(value, 0, &mut used)?,
            CallUseOperationRecord::Guard { condition, .. } => symbols(condition, 0, &mut used)?,
            CallUseOperationRecord::Call { arguments, .. } => {
                for value in arguments {
                    symbols(value, 0, &mut used)?;
                }
            }
            CallUseOperationRecord::LocalStart { .. }
            | CallUseOperationRecord::LocalEnd { .. }
            | CallUseOperationRecord::Return => (),
        }
        for call in &mut calls {
            if used.contains(&call.operation) {
                call.uses.push(index);
                if guard {
                    call.guard_uses.push(index);
                }
            }
        }
        if let CallUseOperationRecord::Call {
            function,
            result_type,
            ..
        } = operation
        {
            if function.is_empty() {
                return None;
            }
            calls.push(CallUseRecord {
                operation: index,
                function: function.clone(),
                result_type: result_type.clone(),
                uses: Vec::new(),
                guard_uses: Vec::new(),
            });
        }
    }
    Some(calls)
}

fn derive_attempt(body: &CallUseFunctionRecord) -> Option<CallResultUseRecord> {
    if !bounded(body) {
        return None;
    }
    let mut paths = Vec::new();
    for (index, path) in body.paths.iter().enumerate() {
        paths.push(PathCallUseRecord {
            path: u32::try_from(index).ok()?,
            calls: read_path(&path.operations)?,
        });
    }
    Some(CallResultUseRecord {
        function: body.function.clone(),
        source_revision: body.source_revision.clone(),
        signature: body.signature.clone(),
        paths,
        profile_assumptions: body.profile_assumptions.clone(),
    })
}

fn derive(source: &CallUseSourceInput) -> Option<CallUseProposalRecord> {
    if !source.version_matches() {
        return None;
    }
    match &source.family {
        CallUseFamilyInput::FilesystemAttempt { body } => {
            derive_attempt(body).map(CallUseProposalRecord::FilesystemAttempt)
        }
    }
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::ResourceContracts
        || request.target.profile != "qcore.c_call_result_use.v1"
        || !matches!(
            request.target.dialect.as_str(),
            "c11" | "gnu99" | "gnu11" | "gnu17"
        )
    {
        return None;
    }
    let summary = derive(request.call_use_source.as_ref()?)?;
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::CallResultUse(Box::new(CallResultUseWitness {
                task_digest: request.task_digest.clone(),
                summary,
            })),
        }),
    })
}

#[cfg(test)]
mod tests;
