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

//! Source call-result evidence for filesystem teardown. Passing these policies
//! records ignored/guarded results; it never certifies successful OS cleanup.
use super::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::C11TranslationUnit;
use crate::source_call_use::PathCallUse;
use crate::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use qcore_verification::call_use_contract::{CallResultUsePolicy, check_call_result_use};
use serde::Serialize;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_attempts.qv"
);
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileAttemptEvidence {
    source: SourceSummary,
    calls: Vec<PathCallUse>,
    contract_source: String,
    status: String,
}
impl FileAttemptEvidence {
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub fn calls(&self) -> &[PathCallUse] {
        &self.calls
    }
}

pub fn check_file_attempt_sources(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    native_unix: bool,
) -> Result<Vec<FileAttemptEvidence>, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("filesystem attempt revision mismatch".into());
    }
    let functions: &[&str] = if native_unix {
        &["cio_file_native_close", "cio_file_native_unmap"]
    } else {
        &["cio_file_close", "munmap_file"]
    };
    let summaries = import_source_functions(
        unit,
        revision,
        functions,
        &SourceImportProfile {
            records: &["cio_chunk", "cio_file", "cio_ctx", "cio_stream"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    summaries
        .iter()
        .map(|source| match source {
            SourceResult::Summary(source) => check(source, CONTRACT),
            SourceResult::Unknown { function, reason } => Err(format!("{function}: {reason}")),
        })
        .collect()
}
pub const DELETE_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete_attempt.qv"
);

pub fn check_file_delete_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
) -> Result<FileAttemptEvidence, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("filesystem delete revision mismatch".into());
    }
    let sources = import_source_functions(
        unit,
        revision,
        &["cio_file_native_delete"],
        &SourceImportProfile {
            records: &["cio_file"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    match sources.as_slice() {
        [SourceResult::Summary(source)] => check(source, DELETE_CONTRACT),
        _ => Err(format!("filesystem delete import incomplete: {sources:?}")),
    }
}

/// Which contract policy and callees decide one selected function. This table
/// is the ChunkIO-specific half of the check; the generic decision procedure
/// lives in `qcore_verification::call_use_contract` so the retained in-process
/// route and the moved process route share one implementation.
pub fn policy_for(function: &str) -> Result<CallResultUsePolicy, String> {
    let (policy, first, second, signature) = match function {
        "cio_file_close" => (
            "file_close_result_use",
            "munmap_file",
            Some("cio_file_native_close"),
            "void (struct cio_chunk *, int)",
        ),
        "munmap_file" => (
            "file_unmap_result_use",
            "cio_file_native_unmap",
            None,
            "int (struct cio_ctx *, struct cio_chunk *)",
        ),
        "cio_file_native_close" => (
            "native_close_result_use",
            "close",
            None,
            "int (struct cio_file *)",
        ),
        "cio_file_native_unmap" => (
            "native_unmap_result_use",
            "munmap",
            None,
            "int (struct cio_file *)",
        ),
        "cio_file_native_delete" => (
            "native_delete_result_use",
            "unlink",
            None,
            "int (struct cio_file *)",
        ),
        _ => return Err("unsupported filesystem result-use source".into()),
    };
    Ok(CallResultUsePolicy {
        policy: policy.into(),
        first: first.into(),
        second: second.map(str::to_owned),
        signature: signature.into(),
    })
}

fn check(source: &SourceSummary, contract: &str) -> Result<FileAttemptEvidence, String> {
    let certificate = check_call_result_use(source, contract, &policy_for(source.function())?)?;
    Ok(FileAttemptEvidence {
        source: certificate.source().clone(),
        calls: certificate.calls().to_vec(),
        contract_source: certificate.contract_source().into(),
        status: certificate.status().into(),
    })
}

#[cfg(test)]
#[path = "../../tests/unit/file_attempts.rs"]
mod tests;
