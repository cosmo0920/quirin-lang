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

//! Sealed source projections for two bounded ChunkIO teardown families.
//!
//! This module converts imported typed paths into the portable teardown
//! vocabulary so a separate process can recompute the structural reading those
//! paths perform. It establishes no custody, no allocation provenance, no ring
//! membership and no exclusivity: those remain the pinned contracts' declared
//! premises, evaluated by the trusted core. A projection that would need any
//! construct outside the vocabulary keeps its function Unknown; there is no
//! lossy encoding and no fallback.
//!
//! Calls are admitted only against an explicit closed callee table. The unlink
//! family admits exactly the co-transported helper (matched on both name and
//! signature) and the destructor family admits exactly `free`; every other
//! callee refuses the projection, so no opaque call vocabulary is opened.

use serde::Serialize;

use qcore_schemas::teardown_summary::{
    TEARDOWN_SOURCE_INPUT_VERSION, TeardownExpressionRecord, TeardownFamilyInput,
    TeardownFunctionRecord, TeardownOperationRecord, TeardownPathRecord, TeardownSourceInput,
    TeardownValueRecord,
};

use crate::boolean_source::{SourceInputIdentity, input_identity};
use crate::clang::ClangIdentity;
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_source::{
    DEFAULT_SOURCE_RULES, SourceExpression, SourceImportProfile, SourceOperation, SourceResult,
    SourceSummary, SourceValue, import_source_functions,
};

/// One admitted callee: exact source name and exact declared C signature.
type Callee<'a> = (&'a str, &'a str);

/// Constructed only by the importer below, so a wire record can never be
/// promoted into a source-checked import.
///
/// ```compile_fail
/// use qcore_c_gateway::teardown_source::ListUnlinkSourceImport;
/// let _: ListUnlinkSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ListUnlinkSourceImport {
    entry: SourceSummary,
    helper: SourceSummary,
    #[serde(skip)]
    identity: SourceInputIdentity,
    #[serde(skip)]
    input: TeardownSourceInput,
}

impl ListUnlinkSourceImport {
    /// The trusted typed summary of the unlink entry.
    pub fn entry(&self) -> &SourceSummary {
        &self.entry
    }

    /// The trusted typed summary of the single helper the entry calls.
    pub fn helper(&self) -> &SourceSummary {
        &self.helper
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.identity
    }

    /// The portable projection of both bodies, without any derived reading.
    pub fn source_input(&self) -> TeardownSourceInput {
        self.input.clone()
    }

    pub fn function(&self) -> &str {
        self.entry.function()
    }

    pub fn source_revision(&self) -> &str {
        self.entry.source_revision()
    }

    pub fn node_c_type(&self) -> &str {
        match &self.input.family {
            TeardownFamilyInput::ListUnlink { node_c_type, .. } => node_c_type,
            TeardownFamilyInput::FieldDestructor { .. } => unreachable!("unlink import family"),
        }
    }
}

/// Constructed only by the importer below.
///
/// ```compile_fail
/// use qcore_c_gateway::teardown_source::FieldDestructorSourceImport;
/// let _: FieldDestructorSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldDestructorSourceImport {
    destructor: SourceSummary,
    #[serde(skip)]
    identity: SourceInputIdentity,
    #[serde(skip)]
    input: TeardownSourceInput,
}

impl FieldDestructorSourceImport {
    pub fn summary(&self) -> &SourceSummary {
        &self.destructor
    }

    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.identity
    }

    pub fn source_input(&self) -> TeardownSourceInput {
        self.input.clone()
    }

    pub fn function(&self) -> &str {
        self.destructor.function()
    }

    pub fn source_revision(&self) -> &str {
        self.destructor.source_revision()
    }

    pub fn root_c_type(&self) -> &str {
        match &self.input.family {
            TeardownFamilyInput::FieldDestructor { root_c_type, .. } => root_c_type,
            TeardownFamilyInput::ListUnlink { .. } => unreachable!("destructor import family"),
        }
    }

    /// The owned-field inventory the caller declared for this projection. The
    /// trusted core refuses a task whose contract declares a different one.
    pub fn declared_owned_fields(&self) -> &[String] {
        match &self.input.family {
            TeardownFamilyInput::FieldDestructor {
                declared_owned_fields,
                ..
            } => declared_owned_fields,
            TeardownFamilyInput::ListUnlink { .. } => unreachable!("destructor import family"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ListUnlinkSourceResult {
    Import(Box<ListUnlinkSourceImport>),
    Unknown { function: String, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum FieldDestructorSourceResult {
    Import(Box<FieldDestructorSourceImport>),
    Unknown { function: String, reason: String },
}

fn value(value: &SourceValue) -> Result<TeardownValueRecord, String> {
    let expression = match &value.expression {
        SourceExpression::Parameter { index } => TeardownExpressionRecord::Parameter {
            index: u32::try_from(*index).map_err(|_| "parameter index out of range")?,
        },
        SourceExpression::Symbol { operation } => TeardownExpressionRecord::Symbol {
            operation: u32::try_from(*operation).map_err(|_| "symbol index out of range")?,
        },
        SourceExpression::Integer { value } => TeardownExpressionRecord::Integer {
            value: value.clone(),
        },
        SourceExpression::Field { base, name } => TeardownExpressionRecord::Field {
            base: Box::new(self::value(base)?),
            name: name.clone(),
        },
        SourceExpression::Cast { cast, value } => TeardownExpressionRecord::Cast {
            cast: cast.clone(),
            value: Box::new(self::value(value)?),
        },
        SourceExpression::Unary { operator, value } => TeardownExpressionRecord::Unary {
            operator: operator.clone(),
            value: Box::new(self::value(value)?),
        },
        SourceExpression::Binary {
            operator,
            left,
            right,
        } => TeardownExpressionRecord::Binary {
            operator: operator.clone(),
            left: Box::new(self::value(left)?),
            right: Box::new(self::value(right)?),
        },
        _ => return Err("teardown projection needs the bounded value vocabulary".into()),
    };
    Ok(TeardownValueRecord {
        c_type: value.c_type.clone(),
        expression,
    })
}

/// Project every complete path of one summary. Scan loops, local starts,
/// value returns, terminations and every callee outside `callees` stay outside
/// the vocabulary, so their functions keep the existing `Unknown` result.
fn project(
    summary: &SourceSummary,
    callees: &[Callee<'_>],
) -> Result<TeardownFunctionRecord, String> {
    if summary.has_scan_loop() {
        return Err("teardown projection does not represent scan loops".into());
    }
    if summary.paths().is_empty() {
        return Err("teardown projection requires at least one complete source path".into());
    }
    let mut paths = Vec::new();
    for path in summary.paths() {
        let mut operations = Vec::new();
        for operation in &path.operations {
            operations.push(match operation {
                SourceOperation::Read { place } => TeardownOperationRecord::Read {
                    place: value(place)?,
                },
                SourceOperation::Write {
                    place,
                    value: written,
                } => TeardownOperationRecord::Write {
                    place: value(place)?,
                    value: value(written)?,
                },
                SourceOperation::LocalWrite {
                    local,
                    value: written,
                } => TeardownOperationRecord::LocalWrite {
                    local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                    value: value(written)?,
                },
                SourceOperation::LocalEnd { local } => TeardownOperationRecord::LocalEnd {
                    local: u32::try_from(*local).map_err(|_| "local index out of range")?,
                },
                SourceOperation::Guard { condition, taken } => TeardownOperationRecord::Guard {
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
                            "teardown projection admits no call to `{function}`"
                        ));
                    }
                    TeardownOperationRecord::Call {
                        function: function.clone(),
                        signature: signature.clone(),
                        arguments: arguments
                            .iter()
                            .map(value)
                            .collect::<Result<Vec<_>, String>>()?,
                        result_type: result_type.clone(),
                    }
                }
                SourceOperation::Return => TeardownOperationRecord::Return,
                _ => {
                    return Err("teardown projection needs the bounded operation vocabulary".into());
                }
            });
        }
        paths.push(TeardownPathRecord { operations });
    }
    Ok(TeardownFunctionRecord {
        function: summary.function().into(),
        source_revision: summary.source_revision().into(),
        signature: summary.signature().into(),
        paths,
        profile_assumptions: summary.profile_assumptions().clone(),
    })
}

fn capture(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<(ClangIdentity, SourceInputIdentity), String> {
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
    Ok((compiler, identity))
}

/// Import an unlink entry with the complete body of the single helper it
/// calls, and project both. The revision is a caller assertion, exactly as in
/// the existing typed import; the outer audit must bind the actual checkout.
/// Function selection, the record inventory and the node C type are explicit
/// inputs, never inferred from a proposal.
pub fn import_list_unlink_sources(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    entry_function: &str,
    helper_function: &str,
    records: &[&str],
    node_c_type: &str,
) -> Result<ListUnlinkSourceResult, String> {
    if entry_function.is_empty() || helper_function.is_empty() || node_c_type.is_empty() {
        return Err("unlink import needs explicit functions and a node C type".into());
    }
    if entry_function == helper_function {
        return Err("unlink entry and helper must be distinct functions".into());
    }
    let (compiler, identity) = capture(unit, source_revision)?;
    let imported = import_source_functions(
        unit,
        source_revision,
        &[entry_function, helper_function],
        &SourceImportProfile {
            records,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let result = match imported.as_slice() {
        [SourceResult::Summary(entry), SourceResult::Summary(helper)] => {
            match project_unlink(entry, helper, node_c_type) {
                Ok(input) => ListUnlinkSourceResult::Import(Box::new(ListUnlinkSourceImport {
                    entry: entry.clone(),
                    helper: helper.clone(),
                    identity,
                    input,
                })),
                Err(reason) => ListUnlinkSourceResult::Unknown {
                    function: entry.function().into(),
                    reason,
                },
            }
        }
        _ => ListUnlinkSourceResult::Unknown {
            function: entry_function.into(),
            reason: format!("unlink source import incomplete: {imported:?}"),
        },
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during unlink source import".into());
    }
    Ok(result)
}

fn project_unlink(
    entry: &SourceSummary,
    helper: &SourceSummary,
    node_c_type: &str,
) -> Result<TeardownSourceInput, String> {
    // The importer returns names in lexical order, so the two summaries are
    // reassociated by their declared signatures rather than by position.
    let entry_signature = format!("void ({node_c_type})");
    let helper_signature = format!("void ({node_c_type}, {node_c_type})");
    let (entry, helper) = if entry.signature() == entry_signature {
        (entry, helper)
    } else if helper.signature() == entry_signature {
        (helper, entry)
    } else {
        return Err("neither imported unlink body has the declared entry signature".into());
    };
    if helper.signature() != helper_signature || entry.function() == helper.function() {
        return Err("unlink helper signature or identity mismatch".into());
    }
    Ok(TeardownSourceInput {
        schema_version: TEARDOWN_SOURCE_INPUT_VERSION,
        family: TeardownFamilyInput::ListUnlink {
            entry: project(entry, &[(helper.function(), helper.signature())])?,
            helper: project(helper, &[])?,
            node_c_type: node_c_type.into(),
        },
    })
}

/// The single libc callee the destructor family admits.
const FREE_CALLEE: Callee<'static> = ("free", "void (void *)");

/// Explicit selection inputs for one nullable-field destructor import. Every
/// field is a caller assertion, never something inferred from a proposal.
pub struct FieldDestructorSelection<'a> {
    pub function: &'a str,
    pub records: &'a [&'a str],
    pub signature_constraints: &'a [(&'a str, &'a str, &'a [&'a str])],
    /// Functions whose calls use the bounded call vocabulary, exactly as the
    /// in-process route passes them.
    pub restricted_calls: &'a [&'a str],
    pub root_c_type: &'a str,
    /// The inventory the pinned contract declares. The trusted core refuses a
    /// task whose contract does not declare exactly this set.
    pub declared_owned_fields: &'a [&'a str],
}

/// Import one nullable-field destructor and project it.
pub fn import_field_destructor_sources(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &FieldDestructorSelection<'_>,
) -> Result<FieldDestructorSourceResult, String> {
    let FieldDestructorSelection {
        function,
        records,
        signature_constraints,
        restricted_calls,
        root_c_type,
        declared_owned_fields,
    } = *selection;
    if function.is_empty() || root_c_type.is_empty() || declared_owned_fields.is_empty() {
        return Err(
            "destructor import needs a function, a root C type and a field inventory".into(),
        );
    }
    let (compiler, identity) = capture(unit, source_revision)?;
    let imported = import_source_functions(
        unit,
        source_revision,
        &[function],
        &SourceImportProfile {
            records,
            sizeof_types: &[],
            signature_constraints,
            restricted_calls,
        },
    )?;
    let result = match imported.as_slice() {
        [SourceResult::Summary(summary)] => {
            match project(summary, &[FREE_CALLEE]).and_then(|destructor| {
                if summary.signature() != format!("void ({root_c_type})") {
                    return Err("destructor signature does not match the supplied root".into());
                }
                Ok(TeardownSourceInput {
                    schema_version: TEARDOWN_SOURCE_INPUT_VERSION,
                    family: TeardownFamilyInput::FieldDestructor {
                        destructor,
                        root_c_type: root_c_type.into(),
                        declared_owned_fields: declared_owned_fields
                            .iter()
                            .map(|field| (*field).to_owned())
                            .collect(),
                    },
                })
            }) {
                Ok(input) => {
                    FieldDestructorSourceResult::Import(Box::new(FieldDestructorSourceImport {
                        destructor: summary.clone(),
                        identity,
                        input,
                    }))
                }
                Err(reason) => FieldDestructorSourceResult::Unknown {
                    function: summary.function().into(),
                    reason,
                },
            }
        }
        [SourceResult::Unknown { function, reason }] => FieldDestructorSourceResult::Unknown {
            function: function.clone(),
            reason: reason.clone(),
        },
        _ => FieldDestructorSourceResult::Unknown {
            function: function.into(),
            reason: format!("destructor source import incomplete: {imported:?}"),
        },
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during destructor source import".into());
    }
    Ok(result)
}
