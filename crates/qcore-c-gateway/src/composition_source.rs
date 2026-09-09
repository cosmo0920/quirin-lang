//! Sealed source projection for the bounded ChunkIO **composition** family.
//!
//! A composition's evidence is a reading over other capabilities' certificates.
//! This module converts the wrapper body **and** the prerequisite body it calls
//! into the portable composition vocabulary, so a separate process can
//! recompute both readings and compose them itself. It never projects a
//! reading: no case vector, no selected path, no counter figure and no
//! certificate crosses this boundary in either direction.
//!
//! It establishes no custody, no OS cleanup success and no discharge: a
//! recorded `munmap` call is not a torn-down mapping. Those remain the pinned
//! contracts' declared premises, evaluated by the trusted core.
//!
//! A body that would need any construct outside the vocabulary keeps its
//! function `Unknown`; there is no lossy encoding and no fallback. The wrapper's
//! calls are admitted only against this family's own closed
//! (name, signature) table, which mirrors the trusted composition's own call
//! table; the prerequisite body's calls are admitted only against the
//! **profile's own three declared callees**, exactly as on the cleanup-replay
//! route. No general call vocabulary is opened here.
//!
//! The wrapper's closed callee table is a **caller assertion** on this route:
//! `contract/qcore/chunkio/unmap_wrapper.qcore` declares no callee vocabulary,
//! so unlike the cleanup route's contract-derived table this one cannot be read
//! off a declaration. It buys no authority — the trusted core runs its own
//! composition over its own import and its own contracts regardless — and
//! declaring it is recorded as follow-on `P04-COMPOSE-CALLEE-DECL`.

use serde::Serialize;

use qcore_schemas::cleanup_replay_summary::{
    CleanupProfileRecord, CleanupReplayExpressionRecord, CleanupReplayFunctionRecord,
    CleanupReplayOperationRecord, CleanupReplayPathRecord, CleanupReplayValueRecord,
};
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_VERSION, CompositionFamilyInput, CompositionSourceInput,
    UnmapWrapperPrerequisites,
};

use crate::boolean_source::{SourceInputIdentity, input_identity};
use crate::clang::ClangIdentity;
use crate::legacy_c::{C11TranslationUnit, validate_unrestricted_c11};
use crate::typed_source::{
    DEFAULT_SOURCE_RULES, SourceExpression, SourceImportProfile, SourceOperation, SourceResult,
    SourceSummary, SourceValue, import_source_functions,
};

/// The wrapper's closed callee table: exactly the four frames the trusted
/// composition's own replay admits, matched on **both** name and declared C
/// signature. A wrapper calling anything else has no projection.
pub const UNMAP_WRAPPER_CALLEES: [(&str, &str); 4] = [
    ("cio_file_sync", "int (struct cio_chunk *)"),
    (
        "cio_log_print",
        "void (void *, int, const char *, int, const char *, ...)",
    ),
    ("cio_file_native_unmap", "int (struct cio_file *)"),
    (
        "cio_chunk_counter_total_up_sub",
        "size_t (struct cio_ctx *)",
    ),
];

/// Which bounded composition question a body is projected for. One inhabitant;
/// the family is a caller selection, recorded on the import and bound into the
/// verification task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionFamily {
    UnmapWrapper,
}

/// Constructed only by the importer below, so a wire record can never be
/// promoted into a source-checked import.
///
/// ```compile_fail
/// use qcore_c_gateway::composition_source::CompositionSourceImport;
/// let _: CompositionSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompositionSourceImport {
    wrapper: SourceSummary,
    native: SourceSummary,
    #[serde(skip)]
    identity: SourceInputIdentity,
    #[serde(skip)]
    family: CompositionFamily,
    #[serde(skip)]
    native_profile: CleanupProfileRecord,
    #[serde(skip)]
    input: CompositionSourceInput,
}

impl CompositionSourceImport {
    /// The trusted typed summary of the wrapper body.
    pub fn wrapper_summary(&self) -> &SourceSummary {
        &self.wrapper
    }

    /// The trusted typed summary of the prerequisite body the wrapper calls.
    /// The core still re-establishes the prerequisite *task* independently and
    /// requires this summary to be the one that task imported.
    pub fn native_summary(&self) -> &SourceSummary {
        &self.native
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.identity
    }

    pub fn family(&self) -> &CompositionFamily {
        &self.family
    }

    /// The record profile the prerequisite projection was taken under. The
    /// trusted core rebuilds this from its own pinned contract's declared
    /// vocabulary and refuses a projection whose profile is not exactly that.
    pub fn native_profile(&self) -> &CleanupProfileRecord {
        &self.native_profile
    }

    /// The portable projection, without any derived reading.
    pub fn source_input(&self) -> CompositionSourceInput {
        self.input.clone()
    }

    pub fn function(&self) -> &str {
        self.wrapper.function()
    }

    pub fn source_revision(&self) -> &str {
        self.wrapper.source_revision()
    }

    pub fn signature(&self) -> &str {
        self.wrapper.signature()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum CompositionSourceResult {
    Import(Box<CompositionSourceImport>),
    Unknown { function: String, reason: String },
}

/// Project one typed value. Every expression form outside the closed list is
/// refused by name, so its function keeps the existing `Unknown`.
fn value(value: &SourceValue) -> Result<CleanupReplayValueRecord, String> {
    let expression = match &value.expression {
        SourceExpression::Parameter { index } => CleanupReplayExpressionRecord::Parameter {
            index: u32::try_from(*index).map_err(|_| "parameter index out of range")?,
        },
        SourceExpression::Symbol { operation } => CleanupReplayExpressionRecord::Symbol {
            operation: u32::try_from(*operation).map_err(|_| "operation index out of range")?,
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
            return Err("composition projection does not represent address-taken locals".into());
        }
        SourceExpression::LoopIndex { .. } => {
            return Err("composition projection does not represent loop indices".into());
        }
        SourceExpression::SizeOf { .. } => {
            return Err("composition projection does not represent sizeof".into());
        }
        SourceExpression::Aggregate { .. } => {
            return Err("composition projection does not represent aggregate literals".into());
        }
        SourceExpression::Index { .. } => {
            return Err("composition projection does not represent subscripts".into());
        }
        SourceExpression::Function { .. } => {
            return Err("composition projection does not represent callee designators".into());
        }
    };
    Ok(CleanupReplayValueRecord {
        c_type: value.c_type.clone(),
        expression,
    })
}

/// Project every complete path of one summary against an explicit closed callee
/// table. Scan loops, terminations, bare returns and every callee outside the
/// table stay outside the vocabulary, so their functions keep the existing
/// `Unknown`.
fn project(
    summary: &SourceSummary,
    admitted: &[(&str, &str)],
) -> Result<CleanupReplayFunctionRecord, String> {
    if summary.has_scan_loop() {
        return Err("composition projection does not represent scan loops".into());
    }
    if summary.paths().is_empty() {
        return Err("composition projection requires at least one complete source path".into());
    }
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
                    // Matched on both the name and the declared C signature: a
                    // same-named frame with another prototype is not this
                    // family's callee.
                    if !admitted
                        .iter()
                        .any(|(name, declared)| name == function && declared == signature)
                    {
                        return Err(format!(
                            "composition projection admits no call to `{function}` at `{signature}`"
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
                        "composition projection does not represent value-free returns".into(),
                    );
                }
                SourceOperation::ScanLoop { .. } => {
                    return Err("composition projection does not represent scan loops".into());
                }
                SourceOperation::Terminate => {
                    return Err(
                        "composition projection does not represent nonreturning terminations"
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

/// Explicit selection inputs for one composition import. Every field is a
/// caller assertion, never something inferred from a proposal — and the two
/// that carry authority (the prerequisite profile and the three identity
/// triples) are checked by the trusted core against its own contract and its
/// own re-derivations before any task exists.
pub struct CompositionSelection<'a> {
    pub family: CompositionFamily,
    /// The wrapper body the composition is about.
    pub wrapper: &'a str,
    /// The prerequisite body it calls, imported from the same unit.
    pub native: &'a str,
    pub records: &'a [&'a str],
    pub native_profile: CleanupProfileRecord,
    /// Lookup keys, never conclusions.
    pub prerequisites: UnmapWrapperPrerequisites,
}

/// Import one composition family's two bodies and project both. The revision is
/// a caller assertion, exactly as in the existing typed import; the outer audit
/// must bind the actual checkout.
pub fn import_composition_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &CompositionSelection<'_>,
) -> Result<CompositionSourceResult, String> {
    let CompositionSelection {
        family,
        wrapper,
        native,
        records,
        native_profile,
        prerequisites,
    } = selection;
    if wrapper.is_empty() || native.is_empty() || records.is_empty() {
        return Err("composition import needs both functions and a record inventory".into());
    }
    if wrapper == native {
        return Err("composition import needs two distinct functions".into());
    }
    if !prerequisites.is_complete() {
        return Err("composition import needs three complete prerequisite identities".into());
    }
    if !prerequisites.are_distinct() {
        return Err("composition import needs three distinct prerequisite identities".into());
    }
    if native_profile.function != *native {
        return Err("composition prerequisite profile names a different function".into());
    }
    if native_profile.fields.is_empty()
        || native_profile.argument_fields.is_empty()
        || native_profile.syscall.is_empty()
        || native_profile.errno_function.is_empty()
        || native_profile.report_function.is_empty()
    {
        return Err("composition prerequisite profile is incomplete".into());
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
        &[wrapper, native],
        &SourceImportProfile {
            records,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let result = match imported.as_slice() {
        [
            SourceResult::Summary(wrapper_summary),
            SourceResult::Summary(native_summary),
        ] => {
            // The prerequisite body's closed callee table is the profile's own
            // declared names, exactly as on the cleanup-replay route.
            let native_admitted = [
                (native_profile.syscall.as_str(), {
                    native_profile.syscall_signature.as_str()
                }),
                (native_profile.errno_function.as_str(), "int *(void)"),
                (
                    native_profile.report_function.as_str(),
                    "int (int, const char *, int)",
                ),
            ];
            let wrapper_admitted: Vec<(&str, &str)> = match family {
                CompositionFamily::UnmapWrapper => UNMAP_WRAPPER_CALLEES.to_vec(),
            };
            match project(wrapper_summary, &wrapper_admitted)
                .and_then(|body| Ok((body, project(native_summary, &native_admitted)?)))
            {
                Ok((wrapper_body, native_body)) => {
                    let family_input = match family {
                        CompositionFamily::UnmapWrapper => CompositionFamilyInput::UnmapWrapper {
                            wrapper: wrapper_body,
                            native: native_body,
                            native_profile: native_profile.clone(),
                        },
                    };
                    CompositionSourceResult::Import(Box::new(CompositionSourceImport {
                        wrapper: wrapper_summary.clone(),
                        native: native_summary.clone(),
                        identity,
                        family: family.clone(),
                        native_profile: native_profile.clone(),
                        input: CompositionSourceInput {
                            schema_version: COMPOSITION_SOURCE_INPUT_VERSION,
                            prerequisites: prerequisites.clone(),
                            family: family_input,
                        },
                    }))
                }
                Err(reason) => CompositionSourceResult::Unknown {
                    function: wrapper_summary.function().into(),
                    reason,
                },
            }
        }
        [SourceResult::Unknown { function, reason }, ..]
        | [_, SourceResult::Unknown { function, reason }] => CompositionSourceResult::Unknown {
            function: function.clone(),
            reason: reason.clone(),
        },
        _ => CompositionSourceResult::Unknown {
            function: (*wrapper).into(),
            reason: format!("composition source import incomplete: {imported:?}"),
        },
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during composition source import".into());
    }
    Ok(result)
}
