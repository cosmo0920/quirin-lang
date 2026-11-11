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

//! Sealed source projection for bounded unsigned unit-counter helpers.
//!
//! This module converts one imported typed path into the portable counter
//! vocabulary so a separate process can recompute the update the path
//! performs. It establishes no custody, no storage provenance, no exclusivity
//! and no ABI binding: those remain the pinned contract's declared premises,
//! evaluated by the trusted core. A projection that would need any construct
//! outside the vocabulary below keeps its function Unknown; there is no
//! lossy encoding and no fallback.

use std::collections::BTreeMap;

use serde::Serialize;

use qcore_schemas::counter_summary::{
    COUNTER_SOURCE_INPUT_VERSION, CounterExpressionRecord, CounterOperationRecord,
    CounterSourceInput, CounterValueRecord,
};

use crate::boolean_source::{SourceInputIdentity, input_identity};
use crate::clang::ClangIdentity;
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_source::{
    DEFAULT_SOURCE_RULES, SourceExpression, SourceImportProfile, SourceOperation, SourceResult,
    SourceSummary, SourceValue, import_source_functions,
};

/// Constructed only by the importer below, so a wire record can never be
/// promoted into a source-checked import.
///
/// ```compile_fail
/// use qcore_c_gateway::counter_source::CounterSourceImport;
/// let _: CounterSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CounterSourceImport(
    SourceSummary,
    #[serde(skip)] SourceInputIdentity,
    #[serde(skip)] CounterSourceInput,
);

impl CounterSourceImport {
    /// The trusted typed summary this projection was taken from.
    pub fn summary(&self) -> &SourceSummary {
        &self.0
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.1
    }

    /// The portable path projection, without any derived counter result.
    pub fn source_input(&self) -> CounterSourceInput {
        self.2.clone()
    }

    pub fn function(&self) -> &str {
        self.0.function()
    }

    pub fn source_revision(&self) -> &str {
        self.0.source_revision()
    }

    pub fn root_c_type(&self) -> &str {
        &self.2.root_c_type
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum CounterSourceResult {
    Import(Box<CounterSourceImport>),
    Unknown { function: String, reason: String },
}

fn value(value: &SourceValue) -> Result<CounterValueRecord, String> {
    let expression = match &value.expression {
        SourceExpression::Parameter { index } => CounterExpressionRecord::Parameter {
            index: u32::try_from(*index).map_err(|_| "parameter index out of range")?,
        },
        SourceExpression::Symbol { operation } => CounterExpressionRecord::Symbol {
            operation: u32::try_from(*operation).map_err(|_| "symbol index out of range")?,
        },
        SourceExpression::Integer { value } => CounterExpressionRecord::Integer {
            value: value.clone(),
        },
        SourceExpression::Field { base, name } => CounterExpressionRecord::Field {
            base: Box::new(self::value(base)?),
            name: name.clone(),
        },
        SourceExpression::Binary {
            operator,
            left,
            right,
        } => CounterExpressionRecord::Binary {
            operator: operator.clone(),
            left: Box::new(self::value(left)?),
            right: Box::new(self::value(right)?),
        },
        _ => return Err("counter projection needs the bounded value vocabulary".into()),
    };
    Ok(CounterValueRecord {
        c_type: value.c_type.clone(),
        expression,
    })
}

/// Project one complete single-path summary. Calls, guards, loops, local
/// storage and every other operation stay outside the vocabulary, so their
/// functions keep the existing `Unknown` result.
fn project(summary: &SourceSummary, root_c_type: &str) -> Result<CounterSourceInput, String> {
    let [path] = summary.paths() else {
        return Err("counter projection requires exactly one complete source path".into());
    };
    if summary.has_scan_loop() {
        return Err("counter projection does not represent scan loops".into());
    }
    let mut operations = Vec::new();
    for operation in &path.operations {
        operations.push(match operation {
            SourceOperation::Read { place } => CounterOperationRecord::Read {
                place: value(place)?,
            },
            SourceOperation::Write {
                place,
                value: written,
            } => CounterOperationRecord::Write {
                place: value(place)?,
                value: value(written)?,
            },
            SourceOperation::ReturnValue { value: returned } => {
                CounterOperationRecord::ReturnValue {
                    value: value(returned)?,
                }
            }
            _ => return Err("counter projection needs the bounded operation vocabulary".into()),
        });
    }
    Ok(CounterSourceInput {
        schema_version: COUNTER_SOURCE_INPUT_VERSION,
        function: summary.function().into(),
        source_revision: summary.source_revision().into(),
        signature: summary.signature().into(),
        root_c_type: root_c_type.into(),
        operations,
        profile_assumptions: summary.profile_assumptions().clone(),
    })
}

/// Import the named counter helpers and project each one. The revision is a
/// caller assertion, exactly as in the existing typed import; the outer audit
/// must bind the actual checkout. Function selection, the record inventory and
/// the root C type are explicit inputs, never inferred from a proposal.
pub fn import_counter_sources(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    functions: &[&str],
    records: &[&str],
    root_c_type: &str,
) -> Result<Vec<CounterSourceResult>, String> {
    if functions.is_empty() || root_c_type.is_empty() {
        return Err("counter import needs explicit functions and a root C type".into());
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
        functions,
        &SourceImportProfile {
            records,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let mut results = Vec::new();
    for result in &imported {
        results.push(match result {
            SourceResult::Summary(summary) => match project(summary, root_c_type) {
                Ok(input) => CounterSourceResult::Import(Box::new(CounterSourceImport(
                    summary.clone(),
                    identity.clone(),
                    input,
                ))),
                Err(reason) => CounterSourceResult::Unknown {
                    function: summary.function().into(),
                    reason,
                },
            },
            SourceResult::Unknown { function, reason } => CounterSourceResult::Unknown {
                function: function.clone(),
                reason: reason.clone(),
            },
        });
    }
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during counter source import".into());
    }
    Ok(results)
}

/// The declared profile assumptions the projection carried, for reports.
pub fn projected_assumptions(import: &CounterSourceImport) -> &BTreeMap<String, String> {
    &import.2.profile_assumptions
}
