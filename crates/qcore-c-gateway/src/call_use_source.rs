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

//! Sealed source projection for the bounded ChunkIO filesystem cleanup-attempt
//! family, in the call-result def/use vocabulary.
//!
//! This module converts one imported typed body into the portable call-use
//! vocabulary so a separate process can recompute the def/use reading those
//! paths perform. It establishes no custody, no OS cleanup success, no
//! descriptor or mapping provenance and no discharge: a recorded `close` call
//! is not a closed descriptor. Those remain the pinned contract's declared
//! premises, evaluated by the trusted core.
//!
//! A projection that would need any construct outside the vocabulary keeps its
//! function `Unknown`; there is no lossy encoding and no fallback. Calls are
//! admitted only against an explicit closed callee table matched on both name
//! and declared C signature — the table is a caller input, recorded on the
//! import and bound into the verification task, so the core refuses a task
//! whose projection was taken under a different table.

use serde::Serialize;

use qcore_schemas::call_use_summary::{
    CALL_USE_SOURCE_INPUT_VERSION, CallUseExpressionRecord, CallUseFamilyInput,
    CallUseFunctionRecord, CallUseOperationRecord, CallUsePathRecord, CallUseSourceInput,
    CallUseValueRecord,
};

use crate::boolean_source::{SourceInputIdentity, input_identity};
use crate::clang::ClangIdentity;
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_source::{
    DEFAULT_SOURCE_RULES, SourceExpression, SourceImportProfile, SourceOperation, SourceResult,
    SourceSummary, SourceValue, import_source_functions,
};

/// One admitted callee: exact source name and exact declared C signature.
pub type AdmittedCallee = (String, String);

/// Constructed only by the importer below, so a wire record can never be
/// promoted into a source-checked import.
///
/// ```compile_fail
/// use qcore_c_gateway::call_use_source::FileAttemptSourceImport;
/// let _: FileAttemptSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileAttemptSourceImport {
    body: SourceSummary,
    #[serde(skip)]
    identity: SourceInputIdentity,
    #[serde(skip)]
    admitted_callees: Vec<AdmittedCallee>,
    #[serde(skip)]
    input: CallUseSourceInput,
}

impl FileAttemptSourceImport {
    /// The trusted typed summary of the imported body.
    pub fn summary(&self) -> &SourceSummary {
        &self.body
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.identity
    }

    /// The closed callee table this projection was taken under. It is part of
    /// the question: the trusted core refuses a task whose import used a table
    /// other than the one its own policy selects.
    pub fn admitted_callees(&self) -> &[AdmittedCallee] {
        &self.admitted_callees
    }

    /// The portable projection of the body, without any derived reading.
    pub fn source_input(&self) -> CallUseSourceInput {
        self.input.clone()
    }

    pub fn function(&self) -> &str {
        self.body.function()
    }

    pub fn source_revision(&self) -> &str {
        self.body.source_revision()
    }

    pub fn signature(&self) -> &str {
        self.body.signature()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum FileAttemptSourceResult {
    Import(Box<FileAttemptSourceImport>),
    Unknown { function: String, reason: String },
}

fn value(value: &SourceValue) -> Result<CallUseValueRecord, String> {
    let expression = match &value.expression {
        SourceExpression::Parameter { index } => CallUseExpressionRecord::Parameter {
            index: u32::try_from(*index).map_err(|_| "parameter index out of range")?,
        },
        SourceExpression::Symbol { operation } => CallUseExpressionRecord::Symbol {
            operation: u32::try_from(*operation).map_err(|_| "symbol index out of range")?,
        },
        SourceExpression::Integer { value } => CallUseExpressionRecord::Integer {
            value: value.clone(),
        },
        SourceExpression::String { value } => CallUseExpressionRecord::String {
            value: value.clone(),
        },
        SourceExpression::Field { base, name } => CallUseExpressionRecord::Field {
            base: Box::new(self::value(base)?),
            name: name.clone(),
        },
        SourceExpression::Cast { cast, value } => CallUseExpressionRecord::Cast {
            cast: cast.clone(),
            value: Box::new(self::value(value)?),
        },
        SourceExpression::Unary { operator, value } => CallUseExpressionRecord::Unary {
            operator: operator.clone(),
            value: Box::new(self::value(value)?),
        },
        SourceExpression::Binary {
            operator,
            left,
            right,
        } => CallUseExpressionRecord::Binary {
            operator: operator.clone(),
            left: Box::new(self::value(left)?),
            right: Box::new(self::value(right)?),
        },
        SourceExpression::LocalStorage { .. } => {
            return Err("call-use projection does not represent address-taken stack slots".into());
        }
        SourceExpression::LoopIndex { .. } => {
            return Err("call-use projection does not represent loop induction values".into());
        }
        SourceExpression::SizeOf { .. } => {
            return Err("call-use projection does not represent sizeof operands".into());
        }
        SourceExpression::Aggregate { .. } => {
            return Err("call-use projection does not represent aggregate initializers".into());
        }
        SourceExpression::Index { .. } => {
            return Err("call-use projection does not represent subscripts".into());
        }
        SourceExpression::Function { .. } => {
            return Err("call-use projection does not represent callee designators".into());
        }
        // CG01 literals (rule 10r): this consumer imports through the legacy
        // admission, so they cannot reach it; were they to, they are refused.
        SourceExpression::Floating { .. } | SourceExpression::Character { .. } => {
            return Err(
                "call-use projection does not classify CG01 floating/character literals".into(),
            );
        }
    };
    Ok(CallUseValueRecord {
        c_type: value.c_type.clone(),
        expression,
    })
}

/// Project every complete path of one summary. Scan loops, terminations and
/// every callee outside `callees` stay outside the vocabulary, so their
/// functions keep the existing `Unknown` result.
fn project(
    summary: &SourceSummary,
    callees: &[AdmittedCallee],
) -> Result<CallUseFunctionRecord, String> {
    if summary.has_scan_loop() {
        return Err("call-use projection does not represent scan loops".into());
    }
    if summary.paths().is_empty() {
        return Err("call-use projection requires at least one complete source path".into());
    }
    let mut paths = Vec::new();
    for path in summary.paths() {
        let mut operations = Vec::new();
        for operation in &path.operations {
            operations.push(match operation {
                SourceOperation::Read { place } => CallUseOperationRecord::Read {
                    place: value(place)?,
                },
                SourceOperation::Write {
                    place,
                    value: written,
                } => CallUseOperationRecord::Write {
                    place: value(place)?,
                    value: value(written)?,
                },
                SourceOperation::LocalStart { local, c_type } => {
                    CallUseOperationRecord::LocalStart {
                        local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                        c_type: c_type.clone(),
                    }
                }
                SourceOperation::LocalWrite {
                    local,
                    value: written,
                } => CallUseOperationRecord::LocalWrite {
                    local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                    value: value(written)?,
                },
                SourceOperation::LocalEnd { local } => CallUseOperationRecord::LocalEnd {
                    local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                },
                SourceOperation::Guard { condition, taken } => CallUseOperationRecord::Guard {
                    condition: value(condition)?,
                    taken: *taken,
                },
                SourceOperation::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    if !callees
                        .iter()
                        .any(|(name, declared)| function == name && signature == declared)
                    {
                        return Err(format!(
                            "call-use projection admits no call to `{function}`"
                        ));
                    }
                    CallUseOperationRecord::Call {
                        function: function.clone(),
                        signature: signature.clone(),
                        arguments: arguments
                            .iter()
                            .map(value)
                            .collect::<Result<Vec<_>, String>>()?,
                        result_type: result_type.clone(),
                    }
                }
                SourceOperation::ReturnValue { value: returned } => {
                    CallUseOperationRecord::ReturnValue {
                        value: value(returned)?,
                    }
                }
                SourceOperation::Return => CallUseOperationRecord::Return,
                SourceOperation::ScanLoop { .. } => {
                    return Err("call-use projection does not represent scan loops".into());
                }
                SourceOperation::Terminate => {
                    return Err(
                        "call-use projection does not represent nonreturning terminations".into(),
                    );
                }
            });
        }
        paths.push(CallUsePathRecord { operations });
    }
    Ok(CallUseFunctionRecord {
        function: summary.function().into(),
        source_revision: summary.source_revision().into(),
        signature: summary.signature().into(),
        paths,
        profile_assumptions: summary.profile_assumptions().clone(),
    })
}

/// Explicit selection inputs for one cleanup-attempt import. Every field is a
/// caller assertion, never something inferred from a proposal.
pub struct FileAttemptSelection<'a> {
    pub function: &'a str,
    pub records: &'a [&'a str],
    /// The closed callee table, matched on name and declared C signature.
    pub admitted_callees: &'a [(&'a str, &'a str)],
}

/// Import one filesystem cleanup-attempt body and project it. The revision is
/// a caller assertion, exactly as in the existing typed import; the outer
/// audit must bind the actual checkout.
pub fn import_file_attempt_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &FileAttemptSelection<'_>,
) -> Result<FileAttemptSourceResult, String> {
    let FileAttemptSelection {
        function,
        records,
        admitted_callees,
    } = *selection;
    if function.is_empty() || admitted_callees.is_empty() {
        return Err("attempt import needs a function and a nonempty admitted callee table".into());
    }
    let mut callees: Vec<AdmittedCallee> = Vec::new();
    for (name, signature) in admitted_callees {
        if name.is_empty() || signature.is_empty() {
            return Err("admitted callee table has an empty name or signature".into());
        }
        let entry = ((*name).to_owned(), (*signature).to_owned());
        if callees.contains(&entry) {
            return Err("admitted callee table repeats an entry".into());
        }
        callees.push(entry);
    }
    // The table is order-insensitive as a question, so it is recorded in a
    // canonical order; two callers naming the same table derive one task.
    callees.sort();
    let compiler = ClangIdentity::capture()?;
    let preprocessed = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let identity = input_identity(
        unit,
        &preprocessed,
        source_revision,
        DEFAULT_SOURCE_RULES,
        None,
        &[],
        compiler.clone(),
    )?;
    let imported = import_source_functions(
        unit,
        source_revision,
        &[function],
        &SourceImportProfile {
            records,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let result = match imported.as_slice() {
        [SourceResult::Summary(summary)] => match project(summary, &callees) {
            Ok(body) => FileAttemptSourceResult::Import(Box::new(FileAttemptSourceImport {
                body: summary.clone(),
                identity,
                admitted_callees: callees,
                input: CallUseSourceInput {
                    schema_version: CALL_USE_SOURCE_INPUT_VERSION,
                    family: CallUseFamilyInput::FilesystemAttempt { body },
                },
            })),
            Err(reason) => FileAttemptSourceResult::Unknown {
                function: summary.function().into(),
                reason,
            },
        },
        [SourceResult::Unknown { function, reason }] => FileAttemptSourceResult::Unknown {
            function: function.clone(),
            reason: reason.clone(),
        },
        _ => FileAttemptSourceResult::Unknown {
            function: function.into(),
            reason: format!("attempt source import incomplete: {imported:?}"),
        },
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during attempt source import".into());
    }
    Ok(result)
}
