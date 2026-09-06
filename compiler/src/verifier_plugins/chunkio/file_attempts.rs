//! Source call-result evidence for filesystem teardown. Passing these policies
//! records ignored/guarded results; it never certifies successful OS cleanup.
use super::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::C11TranslationUnit;
use crate::source_call_use::{PathCallUse, analyze_source_call_uses};
use crate::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use serde::Serialize;

pub const CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/filesystem_attempts.qcore");
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
fn check(source: &SourceSummary, contract: &str) -> Result<FileAttemptEvidence, String> {
    let calls = analyze_source_call_uses(source)?;
    let (policy, first, second, signature) = match source.function() {
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
        _ => return Err("unsupported filesystem result-use source".into()),
    };
    if source.signature() != signature {
        return Err("filesystem result-use signature mismatch".into());
    }
    let any = calls
        .iter()
        .any(|path| path.calls.iter().any(|call| call.function == first));
    for path in &calls {
        let selected: Vec<_> = path
            .calls
            .iter()
            .filter(|call| call.function == first)
            .collect();
        if selected.iter().any(|call| call.result_type != "int") {
            return Err("cleanup result is outside selected int profile".into());
        }
        let mut facts = vec![
            ("source_has_cleanup_attempts", i128::from(any)),
            ("cleanup_calls", selected.len() as i128),
            (
                "ignored_results",
                selected.iter().filter(|c| c.uses.is_empty()).count() as i128,
            ),
            (
                "guarded_results",
                selected.iter().filter(|c| !c.guard_uses.is_empty()).count() as i128,
            ),
        ];
        if let Some(second) = second {
            let close: Vec<_> = path
                .calls
                .iter()
                .filter(|call| call.function == second)
                .collect();
            if close.iter().any(|call| call.result_type != "int") {
                return Err("native close result is outside int profile".into());
            }
            let free: Vec<_> = path
                .calls
                .iter()
                .filter(|call| call.function == "free")
                .collect();
            facts.extend([
                ("close_calls", close.len() as i128),
                (
                    "ignored_close_results",
                    close.iter().filter(|c| c.uses.is_empty()).count() as i128,
                ),
                ("free_calls", free.len() as i128),
                (
                    "cleanup_order_retained",
                    i128::from(selected.iter().all(|unmap| {
                        close.iter().all(|close| {
                            unmap.operation < close.operation
                                && free.iter().all(|free| close.operation < free.operation)
                        })
                    })),
                ),
            ]);
        }
        crate::contract_observation::check_contract_facts(contract, policy, &facts)?;
    }
    Ok(FileAttemptEvidence{source:source.clone(),calls,contract_source:contract.into(),status:"checked source result-use facts; OS success/discharge and full filesystem custody Unknown".into()})
}

#[cfg(test)]
#[path = "../../../tests/unit/file_attempts.rs"]
mod tests;
