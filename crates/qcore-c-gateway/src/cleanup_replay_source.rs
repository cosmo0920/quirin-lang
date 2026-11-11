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

//! Sealed source projection for the two bounded ChunkIO cleanup-outcome
//! families, in the symbolic-replay vocabulary.
//!
//! This module converts one imported typed body, plus the record profile the
//! pinned contract declares, into the portable cleanup-replay vocabulary so a
//! separate process can recompute the symbolic replay those paths perform. It
//! establishes no custody, no OS cleanup success, no descriptor, mapping or
//! file provenance and no discharge: a recorded `close`, `munmap` or `unlink`
//! call is not a released descriptor, a torn-down mapping or a deleted file.
//! Those remain the pinned contract's declared premises, evaluated by the
//! trusted core.
//!
//! A projection that would need any construct outside the vocabulary keeps its
//! function `Unknown`; there is no lossy encoding and no fallback. Calls are
//! admitted only against the **profile's own three declared callees** — the
//! syscall, the errno accessor and the diagnostic helper — so no general call
//! vocabulary is opened here, and the closed table is contract-derived rather
//! than caller-asserted. A bare `Return`, a scan loop and a nonreturning
//! termination have no representation at all, because the replay requires a
//! terminal `ReturnValue` on every complete path.

use serde::Serialize;

use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SOURCE_INPUT_VERSION, CleanupProfileRecord, CleanupReplayExpressionRecord,
    CleanupReplayFamilyInput, CleanupReplayFunctionRecord, CleanupReplayOperationRecord,
    CleanupReplayPathRecord, CleanupReplaySourceInput, CleanupReplayValueRecord,
};

use crate::boolean_source::{SourceInputIdentity, input_identity};
use crate::clang::ClangIdentity;
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_source::{
    DEFAULT_SOURCE_RULES, SourceExpression, SourceImportProfile, SourceOperation, SourceResult,
    SourceSummary, SourceValue, import_source_functions,
};

/// Which of the two sampled prestate domains a body is projected for. The
/// family is a caller selection, recorded on the import and bound into the
/// verification task; the core refuses a task whose family is not the one its
/// own policy decides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupReplayFamily {
    /// Four sampled `(entry, syscall outcome)` prestates.
    NativeCleanupOutcome,
    /// Five sampled eligibility/outcome prestates, with a second eligibility
    /// field that is not a profile member.
    GuardedDelete { mapping_field: String },
}

/// Constructed only by the importer below, so a wire record can never be
/// promoted into a source-checked import.
///
/// ```compile_fail
/// use qcore_c_gateway::cleanup_replay_source::CleanupReplaySourceImport;
/// let _: CleanupReplaySourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanupReplaySourceImport {
    body: SourceSummary,
    #[serde(skip)]
    identity: SourceInputIdentity,
    #[serde(skip)]
    family: CleanupReplayFamily,
    #[serde(skip)]
    profile: CleanupProfileRecord,
    #[serde(skip)]
    input: CleanupReplaySourceInput,
}

impl CleanupReplaySourceImport {
    /// The trusted typed summary of the imported body.
    pub fn summary(&self) -> &SourceSummary {
        &self.body
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.identity
    }

    /// Which sampled prestate domain this projection answers over.
    pub fn family(&self) -> &CleanupReplayFamily {
        &self.family
    }

    /// The record profile the projection was taken under. The trusted core
    /// rebuilds this from its own pinned contract's declared vocabulary and
    /// refuses a projection whose profile is not exactly that.
    pub fn profile(&self) -> &CleanupProfileRecord {
        &self.profile
    }

    /// The portable projection of the body, without any derived reading.
    pub fn source_input(&self) -> CleanupReplaySourceInput {
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
pub enum CleanupReplaySourceResult {
    Import(Box<CleanupReplaySourceImport>),
    Unknown { function: String, reason: String },
}

fn value(value: &SourceValue) -> Result<CleanupReplayValueRecord, String> {
    let expression = match &value.expression {
        SourceExpression::Parameter { index } => CleanupReplayExpressionRecord::Parameter {
            index: u32::try_from(*index).map_err(|_| "parameter index out of range")?,
        },
        SourceExpression::Symbol { operation } => CleanupReplayExpressionRecord::Symbol {
            operation: u32::try_from(*operation).map_err(|_| "symbol index out of range")?,
        },
        SourceExpression::Integer { value } => CleanupReplayExpressionRecord::Integer {
            value: value.clone(),
        },
        SourceExpression::String { value } => CleanupReplayExpressionRecord::String {
            value: value.clone(),
        },
        SourceExpression::Field { base, name } => CleanupReplayExpressionRecord::Field {
            base: Box::new(self::value(base)?),
            name: name.clone(),
        },
        SourceExpression::Cast { cast, value } => CleanupReplayExpressionRecord::Cast {
            cast: cast.clone(),
            value: Box::new(self::value(value)?),
        },
        SourceExpression::Unary { operator, value } => CleanupReplayExpressionRecord::Unary {
            operator: operator.clone(),
            value: Box::new(self::value(value)?),
        },
        SourceExpression::Binary {
            operator,
            left,
            right,
        } => CleanupReplayExpressionRecord::Binary {
            operator: operator.clone(),
            left: Box::new(self::value(left)?),
            right: Box::new(self::value(right)?),
        },
        SourceExpression::LocalStorage { .. } => {
            return Err(
                "cleanup replay projection does not represent address-taken stack slots".into(),
            );
        }
        SourceExpression::LoopIndex { .. } => {
            return Err(
                "cleanup replay projection does not represent loop induction values".into(),
            );
        }
        SourceExpression::SizeOf { .. } => {
            return Err("cleanup replay projection does not represent sizeof operands".into());
        }
        SourceExpression::Aggregate { .. } => {
            return Err(
                "cleanup replay projection does not represent aggregate initializers".into(),
            );
        }
        SourceExpression::Index { .. } => {
            return Err("cleanup replay projection does not represent subscripts".into());
        }
        SourceExpression::Function { .. } => {
            return Err("cleanup replay projection does not represent callee designators".into());
        }
        // CG01 literals (rule 10r): this consumer imports through the legacy
        // admission, so they cannot reach it; were they to, they are refused.
        SourceExpression::Floating { .. } | SourceExpression::Character { .. } => {
            return Err(
                "cleanup replay projection does not classify CG01 floating/character literals"
                    .into(),
            );
        }
    };
    Ok(CleanupReplayValueRecord {
        c_type: value.c_type.clone(),
        expression,
    })
}

/// Project every complete path of one summary. Scan loops, terminations, bare
/// returns and every callee outside the profile's own three declared names
/// stay outside the vocabulary, so their functions keep the existing
/// `Unknown`.
fn project(
    summary: &SourceSummary,
    profile: &CleanupProfileRecord,
) -> Result<CleanupReplayFunctionRecord, String> {
    if summary.has_scan_loop() {
        return Err("cleanup replay projection does not represent scan loops".into());
    }
    if summary.paths().is_empty() {
        return Err("cleanup replay projection requires at least one complete source path".into());
    }
    // The closed callee table is the profile's own declared names, so it is
    // contract-derived rather than a caller assertion.
    let admitted = [
        profile.syscall.as_str(),
        profile.errno_function.as_str(),
        profile.report_function.as_str(),
    ];
    let mut paths = Vec::new();
    for path in summary.paths() {
        let mut operations = Vec::new();
        for operation in &path.operations {
            operations.push(match operation {
                SourceOperation::Read { place } => CleanupReplayOperationRecord::Read {
                    place: value(place)?,
                },
                SourceOperation::Write {
                    place,
                    value: written,
                } => CleanupReplayOperationRecord::Write {
                    place: value(place)?,
                    value: value(written)?,
                },
                SourceOperation::LocalStart { local, c_type } => {
                    CleanupReplayOperationRecord::LocalStart {
                        local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                        c_type: c_type.clone(),
                    }
                }
                SourceOperation::LocalWrite {
                    local,
                    value: written,
                } => CleanupReplayOperationRecord::LocalWrite {
                    local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                    value: value(written)?,
                },
                SourceOperation::LocalEnd { local } => CleanupReplayOperationRecord::LocalEnd {
                    local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                },
                SourceOperation::Guard { condition, taken } => {
                    CleanupReplayOperationRecord::Guard {
                        condition: value(condition)?,
                        taken: *taken,
                    }
                }
                SourceOperation::Call {
                    function,
                    signature,
                    arguments,
                    result_type,
                } => {
                    if !admitted.contains(&function.as_str()) {
                        return Err(format!(
                            "cleanup replay projection admits no call to `{function}`"
                        ));
                    }
                    CleanupReplayOperationRecord::Call {
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
                    CleanupReplayOperationRecord::ReturnValue {
                        value: value(returned)?,
                    }
                }
                SourceOperation::Return => {
                    return Err(
                        "cleanup replay projection does not represent value-free returns".into(),
                    );
                }
                SourceOperation::ScanLoop { .. } => {
                    return Err("cleanup replay projection does not represent scan loops".into());
                }
                SourceOperation::Terminate => {
                    return Err(
                        "cleanup replay projection does not represent nonreturning terminations"
                            .into(),
                    );
                }
            });
        }
        paths.push(CleanupReplayPathRecord { operations });
    }
    Ok(CleanupReplayFunctionRecord {
        function: summary.function().into(),
        source_revision: summary.source_revision().into(),
        signature: summary.signature().into(),
        paths,
        profile_assumptions: summary.profile_assumptions().clone(),
    })
}

/// Explicit selection inputs for one cleanup-replay import. Every field is a
/// caller assertion, never something inferred from a proposal — and the two
/// that carry authority (the profile and the prerequisite triple) are checked
/// by the trusted core against its own contract and its own re-derivation
/// before any task exists.
pub struct CleanupReplaySelection<'a> {
    pub function: &'a str,
    pub records: &'a [&'a str],
    pub family: CleanupReplayFamily,
    pub profile: CleanupProfileRecord,
    /// The call-result def/use prerequisite this reading depends on. A lookup
    /// key, never a conclusion.
    pub prerequisite: PrerequisiteIdentity,
}

/// Import one cleanup-outcome body and project it. The revision is a caller
/// assertion, exactly as in the existing typed import; the outer audit must
/// bind the actual checkout.
pub fn import_cleanup_replay_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &CleanupReplaySelection<'_>,
) -> Result<CleanupReplaySourceResult, String> {
    let CleanupReplaySelection {
        function,
        records,
        family,
        profile,
        prerequisite,
    } = selection;
    if function.is_empty() || records.is_empty() {
        return Err("cleanup replay import needs a function and a record inventory".into());
    }
    if !prerequisite.is_complete() {
        return Err("cleanup replay import needs a complete prerequisite identity".into());
    }
    if profile.function != *function {
        return Err("cleanup replay profile names a different function".into());
    }
    if profile.fields.is_empty()
        || profile.argument_fields.is_empty()
        || profile.syscall.is_empty()
        || profile.errno_function.is_empty()
        || profile.report_function.is_empty()
    {
        return Err("cleanup replay profile is incomplete".into());
    }
    if let CleanupReplayFamily::GuardedDelete { mapping_field } = family
        && (mapping_field.is_empty()
            || !profile.fields.iter().any(|f| f.name == *mapping_field)
            || *mapping_field == profile.presence_field)
    {
        return Err("cleanup replay mapping field is not a distinct declared field".into());
    }
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
        [SourceResult::Summary(summary)] => match project(summary, profile) {
            Ok(body) => {
                let family_input = match family {
                    CleanupReplayFamily::NativeCleanupOutcome => {
                        CleanupReplayFamilyInput::NativeCleanupOutcome {
                            body,
                            profile: profile.clone(),
                        }
                    }
                    CleanupReplayFamily::GuardedDelete { mapping_field } => {
                        CleanupReplayFamilyInput::GuardedDelete {
                            body,
                            profile: profile.clone(),
                            mapping_field: mapping_field.clone(),
                        }
                    }
                };
                CleanupReplaySourceResult::Import(Box::new(CleanupReplaySourceImport {
                    body: summary.clone(),
                    identity,
                    family: family.clone(),
                    profile: profile.clone(),
                    input: CleanupReplaySourceInput {
                        schema_version: CLEANUP_REPLAY_SOURCE_INPUT_VERSION,
                        prerequisite: prerequisite.clone(),
                        family: family_input,
                    },
                }))
            }
            Err(reason) => CleanupReplaySourceResult::Unknown {
                function: summary.function().into(),
                reason,
            },
        },
        [SourceResult::Unknown { function, reason }] => CleanupReplaySourceResult::Unknown {
            function: function.clone(),
            reason: reason.clone(),
        },
        _ => CleanupReplaySourceResult::Unknown {
            function: (*function).into(),
            reason: format!("cleanup replay source import incomplete: {imported:?}"),
        },
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during cleanup replay source import".into());
    }
    Ok(result)
}
