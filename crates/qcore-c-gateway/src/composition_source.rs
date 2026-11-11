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
//! `providers/qcore-provider-chunkio/contracts/chunkio/unmap_wrapper.qv` declares no callee vocabulary,
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
    COMPOSITION_SOURCE_INPUT_VERSION, CalleeIdentityRecord, CompositionFamilyInput,
    CompositionPrerequisiteSet, CompositionSourceInput, DerivedUnitRecord, FileCloseDeleteInput,
    MemoryCloseCalleeRecord,
};

use crate::boolean_source::{SourceInputIdentity, input_identity};
use crate::clang::ClangIdentity;
use crate::legacy_c::{
    C11TranslationUnit, derived_source_text, derived_translation_unit, validate_unrestricted_c11,
};
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

/// The `cio_file_close` wrapper's closed callee table: exactly the five frames
/// the trusted composition's own replay admits, matched on **both** name and
/// declared C signature. Two of them — `munmap_file` and
/// `cio_file_native_delete` — are themselves separately checked capabilities,
/// which is what makes this wrapper a two-level composition.
pub const FILE_CLOSE_CALLEES: [(&str, &str); 5] = [
    ("munmap_file", "int (struct cio_ctx *, struct cio_chunk *)"),
    ("cio_file_native_close", "int (struct cio_file *)"),
    ("cio_file_native_delete", "int (struct cio_file *)"),
    (
        "cio_log_print",
        "void (void *, int, const char *, int, const char *, ...)",
    ),
    ("free", "void (void *)"),
];

/// Which bounded composition question a body is projected for. The family is a
/// caller selection, recorded on the import and bound into the verification
/// task; the two `FileClose` inhabitants are separate because the deleting entry
/// point answers a strictly larger question under its own semantic profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionFamily {
    UnmapWrapper,
    FileClose,
    FileCloseDeleting,
    /// The first family whose bodies are imported from **derived** text: the
    /// base unit plus a constant probe recipe. See [`MemoryCloseSelection`].
    MemoryClose,
    /// The filesystem arm of the same wrapper, imported from a **second**
    /// derived text under a second recipe. See
    /// [`FilesystemChunkCloseSelection`].
    FilesystemChunkClose,
    /// The symbolic other-backend arm of the same wrapper. It imports **no body
    /// at all**: its whole question is the filesystem question, verbatim. See
    /// [`UnsupportedCloseSelection`].
    UnsupportedClose,
}

impl CompositionFamily {
    #[must_use]
    pub const fn is_file_close(&self) -> bool {
        matches!(self, Self::FileClose | Self::FileCloseDeleting)
    }

    /// Whether this family imports from text derived from the unit under
    /// analysis rather than from the unit itself.
    #[must_use]
    pub const fn is_derived(&self) -> bool {
        matches!(self, Self::MemoryClose | Self::FilesystemChunkClose)
    }

    /// Whether this family imports no body of its own. Exactly one family does:
    /// the symbolic arm reads the wrapper and the probe out of its nested
    /// question rather than importing either, so restating one here could name a
    /// different body than the nested question does.
    #[must_use]
    pub const fn is_body_less(&self) -> bool {
        matches!(self, Self::UnsupportedClose)
    }
}

/// The memory-close family's selection: the derivation recipe and the four
/// prerequisite callee identities the wrapper is allowed to resolve.
///
/// The recipe is a **caller constant**, supplied by the trusted decision
/// procedure's own contract selection. This importer builds the derived text
/// itself from the base unit and that recipe; no derived text is ever accepted
/// from anywhere.
pub struct MemoryCloseSelection<'a> {
    pub recipe: &'a str,
    pub probe_function: &'a str,
    /// The four prerequisite bodies' `(name, declared signature)` pairs, in the
    /// order the composition names them: frame reset, backend destructor, list
    /// unlink, counter.
    pub callees: MemoryCloseCalleeRecord,
    /// The two callees the composition admits that no prerequisite supplies:
    /// the libc release frame and the filesystem dispatch arm.
    pub fixed_callees: [(&'a str, &'a str); 2],
}

/// The nested unmap composition's own two bodies and record profile, selected
/// for a `file_close` import. The nested *question* is rebuilt from them
/// verbatim, so a producer must recompute the whole nested composed reading.
pub struct NestedUnmapSelection<'a> {
    pub wrapper: &'a str,
    pub native: &'a str,
    pub native_profile: CleanupProfileRecord,
}

/// The guarded-delete body, record profile and declared eligibility field the
/// deleting arm additionally needs.
pub struct FileCloseDeleteSelection<'a> {
    pub function: &'a str,
    pub profile: CleanupProfileRecord,
    pub mapping_field: &'a str,
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
    /// The nested composition's own two bodies, on the `file_close` families.
    nested_wrapper: Option<SourceSummary>,
    nested_native: Option<SourceSummary>,
    /// The guarded-delete body, on the deleting family only.
    delete: Option<SourceSummary>,
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

    /// The nested composition's wrapper body (`munmap_file`), on the
    /// `file_close` families. The core requires it to be the body the nested
    /// composition task itself imported.
    pub fn nested_wrapper_summary(&self) -> Option<&SourceSummary> {
        self.nested_wrapper.as_ref()
    }

    /// The nested composition's prerequisite body (`cio_file_native_unmap`).
    pub fn nested_native_summary(&self) -> Option<&SourceSummary> {
        self.nested_native.as_ref()
    }

    /// The guarded-delete body, on the deleting family only.
    pub fn delete_summary(&self) -> Option<&SourceSummary> {
        self.delete.as_ref()
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
        // CG01 literals (rule 10r): this consumer imports through the legacy
        // admission, so they cannot reach it; were they to, they are refused.
        SourceExpression::Floating { .. } | SourceExpression::Character { .. } => {
            return Err(
                "composition projection does not classify CG01 floating/character literals".into(),
            );
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
    void_return: bool,
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
                // A value-free return is admitted only for a body the caller
                // declared `void`-returning — the `cio_file_close` wrapper. Every
                // value-returning body on this route keeps the existing refusal,
                // so no family's vocabulary widened by accident.
                SourceOperation::Return if void_return => CleanupReplayOperationRecord::Return,
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
    pub prerequisites: CompositionPrerequisiteSet,
    /// The nested composition's own bodies. Required by the `file_close`
    /// families and refused for the leaf family, in both directions.
    pub nested: Option<NestedUnmapSelection<'a>>,
    /// The guarded-delete body. Required by the deleting family and refused
    /// otherwise, in both directions.
    pub delete: Option<FileCloseDeleteSelection<'a>>,
}

/// The profile-declared callee table one native cleanup body is admitted
/// against, exactly as on the cleanup-replay route.
fn native_callees(profile: &CleanupProfileRecord) -> [(&str, &str); 3] {
    [
        (profile.syscall.as_str(), profile.syscall_signature.as_str()),
        (profile.errno_function.as_str(), "int *(void)"),
        (
            profile.report_function.as_str(),
            "int (int, const char *, int)",
        ),
    ]
}

fn profile_is_complete(profile: &CleanupProfileRecord) -> bool {
    !profile.fields.is_empty()
        && !profile.argument_fields.is_empty()
        && !profile.syscall.is_empty()
        && !profile.errno_function.is_empty()
        && !profile.report_function.is_empty()
}

/// Import one composition family's bodies and project every one of them. The
/// revision is a caller assertion, exactly as in the existing typed import; the
/// outer audit must bind the actual checkout.
///
/// The `file_close` families import **five** bodies at once — the wrapper, the
/// native close, the nested composition's wrapper and native, and (on the
/// deleting arm) the guarded-delete body — because the producer must recompute
/// the whole nested composed reading, not merely a body replay, before it can
/// answer this question.
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
        nested,
        delete,
    } = selection;
    if wrapper.is_empty() || native.is_empty() || records.is_empty() {
        return Err("composition import needs both functions and a record inventory".into());
    }
    if wrapper == native {
        return Err("composition import needs two distinct functions".into());
    }
    if !prerequisites.is_complete() {
        return Err("composition import needs complete prerequisite identities".into());
    }
    if !prerequisites.are_distinct() {
        return Err("composition import needs distinct prerequisite identities".into());
    }
    if native_profile.function != *native {
        return Err("composition prerequisite profile names a different function".into());
    }
    if !profile_is_complete(native_profile) {
        return Err("composition prerequisite profile is incomplete".into());
    }
    // The selected shape and the named prerequisite set must agree, in both
    // directions, before anything is imported.
    match (family, prerequisites) {
        (CompositionFamily::UnmapWrapper, CompositionPrerequisiteSet::UnmapWrapper(_)) => (),
        (
            CompositionFamily::FileClose | CompositionFamily::FileCloseDeleting,
            CompositionPrerequisiteSet::FileClose(_),
        ) => (),
        // The derived family has its own importer, because it must build its
        // own text before it can import anything. Refused here rather than
        // silently imported from the declared file.
        (CompositionFamily::MemoryClose | CompositionFamily::FilesystemChunkClose, _) => {
            return Err(
                "the derived-text families are imported from derived text by their own importers"
                    .into(),
            );
        }
        // The body-less family has its own importer too, for the opposite
        // reason: it has no body to import, and a wrapper selected here would be
        // a *second* naming of the body its nested question already carries.
        (CompositionFamily::UnsupportedClose, _) => {
            return Err(
                "the composed-only family imports no body of its own and has its own importer"
                    .into(),
            );
        }
        _ => return Err("composition family does not match its prerequisite set".into()),
    }
    let deleting = matches!(family, CompositionFamily::FileCloseDeleting);
    if deleting
        != prerequisites
            .file_close()
            .is_some_and(|set| set.native_delete.is_some())
    {
        return Err(
            "the deleting composition family and its delete prerequisite must agree".into(),
        );
    }
    if deleting != delete.is_some() {
        return Err("the deleting composition family and its delete body must agree".into());
    }
    if family.is_file_close() != nested.is_some() {
        return Err("only a file_close composition carries a nested composition".into());
    }
    let mut functions: Vec<&str> = vec![wrapper, native];
    if let Some(nested) = nested {
        if nested.wrapper.is_empty() || nested.native.is_empty() {
            return Err("the nested composition needs both of its functions".into());
        }
        if nested.native_profile.function != nested.native {
            return Err("nested composition profile names a different function".into());
        }
        if !profile_is_complete(&nested.native_profile) {
            return Err("nested composition profile is incomplete".into());
        }
        functions.push(nested.wrapper);
        functions.push(nested.native);
    }
    if let Some(delete) = delete {
        if delete.function.is_empty() || delete.mapping_field.is_empty() {
            return Err("the guarded-delete selection is incomplete".into());
        }
        if delete.profile.function != delete.function {
            return Err("guarded-delete profile names a different function".into());
        }
        if !profile_is_complete(&delete.profile) {
            return Err("guarded-delete profile is incomplete".into());
        }
        if !delete
            .profile
            .fields
            .iter()
            .any(|field| field.name == delete.mapping_field)
        {
            return Err("the declared eligibility field is not a profile member".into());
        }
        functions.push(delete.function);
    }
    let mut distinct = functions.clone();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() != functions.len() {
        return Err("composition import needs distinct functions".into());
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
        &functions,
        &SourceImportProfile {
            records,
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let mut summaries = Vec::new();
    let mut unknown = None;
    for result in &imported {
        match result {
            SourceResult::Summary(summary) => summaries.push(summary.clone()),
            SourceResult::Unknown { function, reason } => {
                if unknown.is_none() {
                    unknown = Some((function.clone(), reason.clone()));
                }
            }
        }
    }
    let result = if let Some((function, reason)) = unknown {
        CompositionSourceResult::Unknown { function, reason }
    } else if summaries.len() != functions.len() {
        CompositionSourceResult::Unknown {
            function: (*wrapper).into(),
            reason: format!("composition source import incomplete: {imported:?}"),
        }
    } else {
        let wrapper_summary = summaries[0].clone();
        let native_summary = summaries[1].clone();
        let wrapper_admitted: Vec<(&str, &str)> = match family {
            CompositionFamily::UnmapWrapper => UNMAP_WRAPPER_CALLEES.to_vec(),
            CompositionFamily::FileClose | CompositionFamily::FileCloseDeleting => {
                FILE_CLOSE_CALLEES.to_vec()
            }
            // Refused above, before any import.
            CompositionFamily::MemoryClose
            | CompositionFamily::FilesystemChunkClose
            | CompositionFamily::UnsupportedClose => Vec::new(),
        };
        // Only the `void` file_close wrapper may end a path without a value.
        let void_wrapper = family.is_file_close();
        let projected = project_family(
            family,
            &wrapper_summary,
            &wrapper_admitted,
            void_wrapper,
            &native_summary,
            native_profile,
            nested.as_ref(),
            delete.as_ref(),
            &summaries,
        );
        match projected {
            Ok(family_input) => {
                if family_input.semantic_profile() != expected_profile(family) {
                    return Err("composition family input answers another semantic profile".into());
                }
                CompositionSourceResult::Import(Box::new(CompositionSourceImport {
                    wrapper: wrapper_summary,
                    native: native_summary,
                    nested_wrapper: nested.as_ref().map(|_| summaries[2].clone()),
                    nested_native: nested.as_ref().map(|_| summaries[3].clone()),
                    delete: delete.as_ref().map(|_| summaries[4].clone()),
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
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during composition source import".into());
    }
    Ok(result)
}

/// Project the selected family's whole question. For a `file_close` family the
/// nested composition question is rebuilt verbatim, so the transported value is
/// the same one the leaf composition's own projection carries.
#[allow(clippy::too_many_arguments)]
fn project_family(
    family: &CompositionFamily,
    wrapper_summary: &SourceSummary,
    wrapper_admitted: &[(&str, &str)],
    void_wrapper: bool,
    native_summary: &SourceSummary,
    native_profile: &CleanupProfileRecord,
    nested: Option<&NestedUnmapSelection<'_>>,
    delete: Option<&FileCloseDeleteSelection<'_>>,
    summaries: &[SourceSummary],
) -> Result<CompositionFamilyInput, String> {
    let wrapper_body = project(wrapper_summary, wrapper_admitted, void_wrapper)?;
    let native_body = project(native_summary, &native_callees(native_profile), false)?;
    match family {
        CompositionFamily::UnmapWrapper => Ok(CompositionFamilyInput::UnmapWrapper {
            wrapper: wrapper_body,
            native: native_body,
            native_profile: native_profile.clone(),
        }),
        CompositionFamily::MemoryClose
        | CompositionFamily::FilesystemChunkClose
        | CompositionFamily::UnsupportedClose => Err(
            "the derived-text and composed-only families are not projected by this importer".into(),
        ),
        CompositionFamily::FileClose | CompositionFamily::FileCloseDeleting => {
            let nested = nested.ok_or("missing nested composition selection")?;
            let nested_wrapper = project(&summaries[2], &UNMAP_WRAPPER_CALLEES, false)?;
            let nested_native = project(
                &summaries[3],
                &native_callees(&nested.native_profile),
                false,
            )?;
            let delete_input = match delete {
                None => None,
                Some(delete) => Some(Box::new(FileCloseDeleteInput {
                    body: project(&summaries[4], &native_callees(&delete.profile), false)?,
                    profile: delete.profile.clone(),
                    mapping_field: delete.mapping_field.to_owned(),
                })),
            };
            Ok(CompositionFamilyInput::FileClose {
                wrapper: wrapper_body,
                unmap: Box::new(CompositionFamilyInput::UnmapWrapper {
                    wrapper: nested_wrapper,
                    native: nested_native,
                    native_profile: nested.native_profile.clone(),
                }),
                close: native_body,
                close_profile: native_profile.clone(),
                delete: delete_input,
            })
        }
    }
}

/// The semantic profile a selected family must answer under, so the projected
/// shape and the caller's selection cannot disagree.
const fn expected_profile(family: &CompositionFamily) -> &'static str {
    match family {
        CompositionFamily::UnmapWrapper => {
            qcore_schemas::composition_summary::UNMAP_WRAPPER_SEMANTIC_PROFILE
        }
        CompositionFamily::FileClose => {
            qcore_schemas::composition_summary::FILE_CLOSE_SEMANTIC_PROFILE
        }
        CompositionFamily::FileCloseDeleting => {
            qcore_schemas::composition_summary::FILE_CLOSE_DELETE_SEMANTIC_PROFILE
        }
        CompositionFamily::MemoryClose => {
            qcore_schemas::composition_summary::MEMORY_CLOSE_SEMANTIC_PROFILE
        }
        CompositionFamily::FilesystemChunkClose => {
            qcore_schemas::composition_summary::FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE
        }
        CompositionFamily::UnsupportedClose => {
            qcore_schemas::composition_summary::UNSUPPORTED_CLOSE_SEMANTIC_PROFILE
        }
    }
}

// ---------------------------------------------------------------------------
// The memory-close family: the one composition imported from derived text.
// ---------------------------------------------------------------------------

/// A sealed memory-close import. Constructed only by the importer below, so a
/// wire record can never be promoted into a source-checked import.
///
/// It records **two** identities, not one, and that separation is the point of
/// the card. The derived unit's identity is the identity of the text the bodies
/// actually came from; the base unit's identity is the identity of the file the
/// audit selected. `CSourceInput::for_derived_text` collapses a declared file
/// to an anonymous text, so the derived identity records neither the spelled
/// input nor the directory — two base units with equal bytes and different
/// declared identities produce byte-identical derived identities. A dependent
/// that bound only the derived identity would treat them as one task.
///
/// ```compile_fail
/// use qcore_c_gateway::composition_source::MemoryCloseSourceImport;
/// let _: MemoryCloseSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryCloseSourceImport {
    wrapper: SourceSummary,
    probe: SourceSummary,
    #[serde(skip)]
    base_identity: SourceInputIdentity,
    #[serde(skip)]
    derived_identity: SourceInputIdentity,
    #[serde(skip)]
    derivation: DerivedUnitRecord,
    #[serde(skip)]
    callees: MemoryCloseCalleeRecord,
    #[serde(skip)]
    input: CompositionSourceInput,
}

impl MemoryCloseSourceImport {
    /// The trusted typed summary of the wrapper body, imported from the derived
    /// unit.
    pub fn wrapper_summary(&self) -> &SourceSummary {
        &self.wrapper
    }

    /// The trusted typed summary of the probe body the recipe defines.
    pub fn probe_summary(&self) -> &SourceSummary {
        &self.probe
    }

    /// The identity of the **base** unit the audit selected. Bound separately
    /// by the dependent task, because the derived identity cannot stand in for
    /// it.
    pub fn base_identity(&self) -> &SourceInputIdentity {
        &self.base_identity
    }

    /// The identity of the **derived** unit the bodies were imported from.
    pub fn derived_identity(&self) -> &SourceInputIdentity {
        &self.derived_identity
    }

    /// The derivation, as this importer measured it. The trusted core rebuilds
    /// the same record from its own base unit and its own recipe and refuses a
    /// transported record that is not exactly this.
    pub fn derivation(&self) -> &DerivedUnitRecord {
        &self.derivation
    }

    pub fn callees(&self) -> &MemoryCloseCalleeRecord {
        &self.callees
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
pub enum MemoryCloseSourceResult {
    Import(Box<MemoryCloseSourceImport>),
    Unknown { function: String, reason: String },
}

/// Explicit selection inputs for one memory-close composition import.
pub struct MemoryCloseImportSelection<'a> {
    /// The wrapper body the composition is about.
    pub wrapper: &'a str,
    pub records: &'a [&'a str],
    /// Lookup keys, never conclusions.
    pub prerequisites: CompositionPrerequisiteSet,
    pub memory_close: MemoryCloseSelection<'a>,
}

/// Import the memory-close family's two bodies **from text this importer
/// derives** and project both.
///
/// The caller supplies the base unit and the recipe; the derived text is built
/// here, by the one shared helper the trusted decision procedure also uses, so
/// the two cannot derive different bytes from the same inputs. No caller — and
/// certainly no proposal — supplies derived text.
pub fn import_memory_close_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &MemoryCloseImportSelection<'_>,
) -> Result<MemoryCloseSourceResult, String> {
    let MemoryCloseImportSelection {
        wrapper,
        records,
        prerequisites,
        memory_close,
    } = selection;
    if wrapper.is_empty() || records.is_empty() {
        return Err("memory close import needs a wrapper and a record inventory".into());
    }
    if memory_close.recipe.is_empty() || memory_close.probe_function.is_empty() {
        return Err("memory close import needs a complete derivation recipe".into());
    }
    if *wrapper == memory_close.probe_function {
        return Err("memory close import needs two distinct functions".into());
    }
    if !prerequisites.is_complete() {
        return Err("memory close import needs complete prerequisite identities".into());
    }
    if !prerequisites.are_distinct() {
        return Err("memory close import needs distinct prerequisite identities".into());
    }
    if prerequisites.memory_close().is_none() {
        return Err("memory close family does not match its prerequisite set".into());
    }
    if !memory_close.callees.is_complete() || !memory_close.callees.are_distinct() {
        return Err("memory close import needs four distinct prerequisite callees".into());
    }
    if memory_close
        .fixed_callees
        .iter()
        .any(|(name, signature)| name.is_empty() || signature.is_empty())
    {
        return Err("memory close import needs complete fixed callee identities".into());
    }
    let compiler = ClangIdentity::capture()?;
    // The base unit's own identity: the file the audit selected.
    let base_preprocessed = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let base_identity = input_identity(
        unit,
        &base_preprocessed,
        source_revision,
        DEFAULT_SOURCE_RULES,
        None,
        &[],
        compiler.clone(),
    )?;
    // The derived unit: the base text plus the caller's recipe, analysed under
    // every one of the base unit's declared options and under the input
    // identity `for_derived_text` leaves it with.
    let combined = derived_source_text(unit.source, memory_close.recipe);
    let derived_unit = derived_translation_unit(unit, &combined);
    let derived_preprocessed =
        validate_unrestricted_c11(&derived_unit).map_err(|error| error.to_string())?;
    let derived_identity = input_identity(
        &derived_unit,
        &derived_preprocessed,
        source_revision,
        DEFAULT_SOURCE_RULES,
        None,
        &[],
        compiler.clone(),
    )?;
    let derivation = DerivedUnitRecord {
        recipe: memory_close.recipe.to_owned(),
        recipe_function: memory_close.probe_function.to_owned(),
        base_source_digest: base_identity.raw_source_digest().to_owned(),
        derived_source_digest: derived_identity.raw_source_digest().to_owned(),
        base_input_class: unit.input.class().to_owned(),
        derived_input_class: derived_unit.input.class().to_owned(),
    };
    if !derivation.is_complete() || !derivation.is_a_derivation() {
        return Err("the memory close recipe derived no new text".into());
    }
    let imported = import_source_functions(
        &derived_unit,
        source_revision,
        &[wrapper, memory_close.probe_function],
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
            SourceResult::Summary(probe_summary),
        ] => {
            let mut admitted: Vec<(&str, &str)> = memory_close
                .fixed_callees
                .iter()
                .map(|(name, signature)| (*name, *signature))
                .collect();
            for callee in [
                &memory_close.callees.reset,
                &memory_close.callees.backend,
                &memory_close.callees.unlink,
            ] {
                admitted.push((callee.function.as_str(), callee.signature.as_str()));
            }
            // The counter frame's call-site prototype, derived under the
            // trusted composition's own rule rather than assumed equal to the
            // definition's.
            let counter_call =
                counter_call_identity(wrapper_summary, &memory_close.callees.counter);
            if let Some((function, signature)) = &counter_call {
                admitted.push((function.as_str(), signature.as_str()));
            }
            // The wrapper returns `void`, so its complete paths end without a
            // value, exactly as the `cio_file_close` wrapper's do.
            match (
                project(wrapper_summary, &admitted, true),
                project(probe_summary, &[], false),
            ) {
                (Ok(wrapper_body), Ok(probe_body)) => {
                    let family = CompositionFamilyInput::MemoryClose {
                        wrapper: wrapper_body,
                        probe: probe_body,
                        derivation: derivation.clone(),
                        callees: memory_close.callees.clone(),
                    };
                    if family.semantic_profile()
                        != qcore_schemas::composition_summary::MEMORY_CLOSE_SEMANTIC_PROFILE
                    {
                        return Err(
                            "memory close family input answers another semantic profile".into()
                        );
                    }
                    MemoryCloseSourceResult::Import(Box::new(MemoryCloseSourceImport {
                        wrapper: wrapper_summary.clone(),
                        probe: probe_summary.clone(),
                        base_identity,
                        derived_identity,
                        derivation,
                        callees: memory_close.callees.clone(),
                        input: CompositionSourceInput {
                            schema_version: COMPOSITION_SOURCE_INPUT_VERSION,
                            prerequisites: prerequisites.clone(),
                            family,
                        },
                    }))
                }
                (Err(reason), _) | (_, Err(reason)) => MemoryCloseSourceResult::Unknown {
                    function: wrapper_summary.function().into(),
                    reason,
                },
            }
        }
        _ => {
            let (function, reason) = imported
                .iter()
                .find_map(|result| match result {
                    SourceResult::Unknown { function, reason } => {
                        Some((function.clone(), reason.clone()))
                    }
                    SourceResult::Summary(_) => None,
                })
                .unwrap_or_else(|| {
                    (
                        (*wrapper).into(),
                        format!("memory close source import incomplete: {imported:?}"),
                    )
                });
            MemoryCloseSourceResult::Unknown { function, reason }
        }
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during memory close source import".into());
    }
    Ok(result)
}

/// The counter frame's **call-site** prototype, admitted only when it satisfies
/// the trusted composition's own rule against the prerequisite's declared
/// signature: the same name, the same declared argument list, and a result type
/// equal to the prerequisite's declared return type.
///
/// A C typedef may be spelled differently at a call site than at the
/// definition — the pinned ChunkIO counters return a `size_t` typedef — which is
/// exactly why the trusted replay compares those three things rather than the
/// whole signature string. Deriving the admitted pair from the body under the
/// core's own rule buys no authority: the core re-runs its own replay over its
/// own import regardless, and a call that fails the rule has no projection at
/// all.
fn counter_call_identity(
    wrapper: &SourceSummary,
    counter: &CalleeIdentityRecord,
) -> Option<(String, String)> {
    let (declared_result, declared_arguments) = counter.signature.split_once('(')?;
    let declared_result = declared_result.trim();
    for path in wrapper.paths() {
        for operation in &path.operations {
            let SourceOperation::Call {
                function,
                signature,
                result_type,
                ..
            } = operation
            else {
                continue;
            };
            if function != &counter.function || result_type != declared_result {
                continue;
            }
            if signature.split_once('(').map(|(_, rest)| rest) != Some(declared_arguments) {
                continue;
            }
            return Some((function.clone(), signature.clone()));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The filesystem-chunk-close family: a second derived unit in the same question.
// ---------------------------------------------------------------------------

/// Constructed only by [`import_filesystem_chunk_close_source`], so a wire
/// record can never be promoted into a source-checked import.
///
/// This family imports exactly **one** body of its own — the filesystem
/// discriminant probe, from a unit this importer derives under its own recipe.
/// The wrapper it composes over is the nested memory-close question's, which
/// this import carries verbatim rather than re-importing, so the two halves of
/// one question cannot name two different bodies.
///
/// ```compile_fail
/// use qcore_c_gateway::composition_source::FilesystemChunkCloseSourceImport;
/// let _: FilesystemChunkCloseSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FilesystemChunkCloseSourceImport {
    probe: SourceSummary,
    #[serde(skip)]
    base_identity: SourceInputIdentity,
    #[serde(skip)]
    derived_identity: SourceInputIdentity,
    #[serde(skip)]
    derivation: DerivedUnitRecord,
    #[serde(skip)]
    input: CompositionSourceInput,
}

impl FilesystemChunkCloseSourceImport {
    /// The trusted typed summary of the probe body this family's recipe defines.
    pub fn probe_summary(&self) -> &SourceSummary {
        &self.probe
    }

    /// The identity of the **base** unit the audit selected — the same unit the
    /// nested memory-close question derives from.
    pub fn base_identity(&self) -> &SourceInputIdentity {
        &self.base_identity
    }

    /// The identity of the unit **this family's** probe was imported from.
    pub fn derived_identity(&self) -> &SourceInputIdentity {
        &self.derived_identity
    }

    /// The derivation, as this importer measured it. The trusted core rebuilds
    /// the same record from its own base unit and its own recipe and refuses a
    /// transported record that is not exactly this.
    pub fn derivation(&self) -> &DerivedUnitRecord {
        &self.derivation
    }

    /// The portable projection, without any derived reading at any level.
    pub fn source_input(&self) -> CompositionSourceInput {
        self.input.clone()
    }

    /// The wrapper this family composes over: the nested memory-close
    /// question's own body record.
    pub fn wrapper_record(&self) -> &CleanupReplayFunctionRecord {
        self.input.family.wrapper()
    }

    pub fn function(&self) -> &str {
        &self.wrapper_record().function
    }

    pub fn source_revision(&self) -> &str {
        self.probe.source_revision()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum FilesystemChunkCloseSourceResult {
    Import(Box<FilesystemChunkCloseSourceImport>),
    Unknown { function: String, reason: String },
}

/// The filesystem family's own selection: its probe recipe, and the two nested
/// questions its two composition prerequisites imported.
///
/// The two nested questions are **not** re-derived here. They are the values
/// the core's own nested imports produced, and the trusted task requires each
/// to be exactly the family input its own nested composition context imported —
/// which is the same tie `P04-COMPOSE-A` established for its one nested
/// question, applied twice.
pub struct FilesystemChunkCloseSelection<'a> {
    pub recipe: &'a str,
    pub probe_function: &'a str,
    /// The nested memory-close question, verbatim.
    pub memory: CompositionFamilyInput,
    /// The nested deleting `cio_file_close` question, verbatim.
    pub backend: CompositionFamilyInput,
    /// Lookup keys, never conclusions.
    pub prerequisites: CompositionPrerequisiteSet,
}

/// Import the filesystem family's one body **from text this importer derives**
/// and project it.
///
/// The caller supplies the base unit and the recipe; the derived text is built
/// here by the one shared helper the trusted decision procedure also uses, so
/// the two cannot derive different bytes from the same inputs. No caller — and
/// certainly no proposal — supplies derived text.
pub fn import_filesystem_chunk_close_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &FilesystemChunkCloseSelection<'_>,
) -> Result<FilesystemChunkCloseSourceResult, String> {
    let FilesystemChunkCloseSelection {
        recipe,
        probe_function,
        memory,
        backend,
        prerequisites,
    } = selection;
    if recipe.is_empty() || probe_function.is_empty() {
        return Err("filesystem chunk close import needs a complete derivation recipe".into());
    }
    if !prerequisites.is_complete() {
        return Err("filesystem chunk close import needs complete prerequisite identities".into());
    }
    if !prerequisites.are_distinct() {
        return Err("filesystem chunk close import needs distinct prerequisite identities".into());
    }
    if prerequisites.filesystem_chunk_close().is_none() {
        return Err("filesystem chunk close family does not match its prerequisite set".into());
    }
    // Both nested questions must be the families this route's prerequisites
    // answer, before anything is imported.
    if !matches!(memory, CompositionFamilyInput::MemoryClose { .. }) {
        return Err("the filesystem question's memory half is not a memory-close question".into());
    }
    if !matches!(
        backend,
        CompositionFamilyInput::FileClose {
            delete: Some(_),
            ..
        }
    ) {
        return Err(
            "the filesystem question's backend half is not a deleting file_close question".into(),
        );
    }
    // The two derivations must be two *different* recipes over one base text.
    // A filesystem probe derived by the memory recipe would read the memory
    // discriminant and the two arms would collapse.
    let memory_derivation = memory
        .derivation()
        .ok_or("the nested memory-close question carries no derivation")?;
    if memory_derivation.recipe == *recipe || memory_derivation.recipe_function == *probe_function {
        return Err("the two derived units must be derived by two different recipes".into());
    }
    let compiler = ClangIdentity::capture()?;
    let base_preprocessed = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let base_identity = input_identity(
        unit,
        &base_preprocessed,
        source_revision,
        DEFAULT_SOURCE_RULES,
        None,
        &[],
        compiler.clone(),
    )?;
    if base_identity.raw_source_digest() != memory_derivation.base_source_digest {
        return Err(
            "the nested memory-close question derives from another base text than this one".into(),
        );
    }
    let combined = derived_source_text(unit.source, recipe);
    let derived_unit = derived_translation_unit(unit, &combined);
    let derived_preprocessed =
        validate_unrestricted_c11(&derived_unit).map_err(|error| error.to_string())?;
    let derived_identity = input_identity(
        &derived_unit,
        &derived_preprocessed,
        source_revision,
        DEFAULT_SOURCE_RULES,
        None,
        &[],
        compiler.clone(),
    )?;
    let derivation = DerivedUnitRecord {
        recipe: (*recipe).to_owned(),
        recipe_function: (*probe_function).to_owned(),
        base_source_digest: base_identity.raw_source_digest().to_owned(),
        derived_source_digest: derived_identity.raw_source_digest().to_owned(),
        base_input_class: unit.input.class().to_owned(),
        derived_input_class: derived_unit.input.class().to_owned(),
    };
    if !derivation.is_complete() || !derivation.is_a_derivation() {
        return Err("the filesystem chunk close recipe derived no new text".into());
    }
    if derivation.derived_source_digest == memory_derivation.derived_source_digest {
        return Err("the two recipes derived the same text".into());
    }
    let imported = import_source_functions(
        &derived_unit,
        source_revision,
        &[probe_function],
        &SourceImportProfile {
            records: &[],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let result = match imported.as_slice() {
        [SourceResult::Summary(probe_summary)] => match project(probe_summary, &[], false) {
            Ok(probe_body) => {
                let family = CompositionFamilyInput::FilesystemChunkClose {
                    memory: Box::new(memory.clone()),
                    backend: Box::new(backend.clone()),
                    probe: probe_body,
                    derivation: derivation.clone(),
                };
                if family.semantic_profile()
                    != qcore_schemas::composition_summary::FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE
                {
                    return Err(
                        "filesystem chunk close family input answers another semantic profile"
                            .into(),
                    );
                }
                if !family.matches(prerequisites) {
                    return Err(
                        "filesystem chunk close family and prerequisite set disagree".into(),
                    );
                }
                FilesystemChunkCloseSourceResult::Import(Box::new(
                    FilesystemChunkCloseSourceImport {
                        probe: probe_summary.clone(),
                        base_identity,
                        derived_identity,
                        derivation,
                        input: CompositionSourceInput {
                            schema_version: COMPOSITION_SOURCE_INPUT_VERSION,
                            prerequisites: prerequisites.clone(),
                            family,
                        },
                    },
                ))
            }
            Err(reason) => FilesystemChunkCloseSourceResult::Unknown {
                function: probe_summary.function().into(),
                reason,
            },
        },
        _ => {
            let (function, reason) = imported
                .iter()
                .find_map(|result| match result {
                    SourceResult::Unknown { function, reason } => {
                        Some((function.clone(), reason.clone()))
                    }
                    SourceResult::Summary(_) => None,
                })
                .unwrap_or_else(|| {
                    (
                        (*probe_function).into(),
                        format!("filesystem chunk close source import incomplete: {imported:?}"),
                    )
                });
            FilesystemChunkCloseSourceResult::Unknown { function, reason }
        }
    };
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during filesystem chunk close source import".into());
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// The symbolic other-backend family: a question with no body of its own.
// ---------------------------------------------------------------------------

/// Constructed only by [`import_unsupported_close_source`], so a wire record can
/// never be promoted into a source-checked import.
///
/// This family imports **no body at all**. Both bodies its reading needs — the
/// `cio_chunk_close` wrapper and the filesystem discriminant probe — are already
/// inside the nested filesystem question, which this import carries verbatim.
/// Importing either again here would let one question name two bodies, exactly
/// the failure the filesystem family avoided by delegating its wrapper.
///
/// What the import does capture is the **base unit's own identity**, because the
/// route still has to tie its task to a translation unit and its dependent core
/// still has to check that the nested question was checked over that same text.
///
/// ```compile_fail
/// use qcore_c_gateway::composition_source::UnsupportedCloseSourceImport;
/// let _: UnsupportedCloseSourceImport = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnsupportedCloseSourceImport {
    #[serde(skip)]
    base_identity: SourceInputIdentity,
    #[serde(skip)]
    input: CompositionSourceInput,
}

impl UnsupportedCloseSourceImport {
    /// The identity of the **base** unit the audit selected — the unit the
    /// nested filesystem question's two derivations both derive from.
    pub fn base_identity(&self) -> &SourceInputIdentity {
        &self.base_identity
    }

    /// The portable projection, without any derived reading at any level.
    pub fn source_input(&self) -> CompositionSourceInput {
        self.input.clone()
    }

    /// The wrapper this family composes over: the nested memory-close question's
    /// own body record, reached two levels down.
    pub fn wrapper_record(&self) -> &CleanupReplayFunctionRecord {
        self.input.family.wrapper()
    }

    /// The nested filesystem question, verbatim.
    pub fn nested_question(&self) -> &CompositionFamilyInput {
        let [nested] = self.input.family.nested()[..] else {
            unreachable!("the symbolic family nests exactly one question");
        };
        nested
    }

    pub fn function(&self) -> &str {
        &self.wrapper_record().function
    }

    pub fn source_revision(&self) -> &str {
        &self.wrapper_record().source_revision
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum UnsupportedCloseSourceResult {
    Import(Box<UnsupportedCloseSourceImport>),
    Unknown { function: String, reason: String },
}

/// The symbolic family's own selection: the one nested question its single
/// composition prerequisite imported, and that prerequisite's lookup key.
///
/// The nested question is **not** re-derived here. It is the value the core's
/// own filesystem import produced, and the trusted task requires it to be
/// exactly the family input that context imported.
pub struct UnsupportedCloseSelection<'a> {
    /// The nested filesystem question, verbatim.
    pub filesystem: CompositionFamilyInput,
    /// A lookup key, never a conclusion.
    pub prerequisites: &'a CompositionPrerequisiteSet,
}

/// Project the symbolic family's question. Nothing is imported: the base unit is
/// preprocessed only so its identity can be captured and tied to the nested
/// question's own derivations.
pub fn import_unsupported_close_source(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    selection: &UnsupportedCloseSelection<'_>,
) -> Result<UnsupportedCloseSourceResult, String> {
    let UnsupportedCloseSelection {
        filesystem,
        prerequisites,
    } = selection;
    if source_revision.is_empty() {
        return Err("unsupported close import needs a declared source revision".into());
    }
    if !prerequisites.is_complete() {
        return Err("unsupported close import needs a complete prerequisite identity".into());
    }
    if prerequisites.unsupported_close().is_none() {
        return Err("unsupported close family does not match its prerequisite set".into());
    }
    let CompositionFamilyInput::FilesystemChunkClose {
        memory,
        derivation: filesystem_derivation,
        ..
    } = filesystem
    else {
        return Err(
            "the symbolic question's nested half is not a filesystem chunk close question".into(),
        );
    };
    let memory_derivation = memory
        .derivation()
        .ok_or("the nested memory-close question carries no derivation")?;
    // The nested question was derived under two recipes over one base text. This
    // route imports neither text, so what it can check is that the base text is
    // the unit it was handed — and it checks it against **both** derivations,
    // not one, so a nested question assembled from two different units cannot
    // pass by matching on its filesystem half alone.
    let compiler = ClangIdentity::capture()?;
    let base_preprocessed = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let base_identity = input_identity(
        unit,
        &base_preprocessed,
        source_revision,
        DEFAULT_SOURCE_RULES,
        None,
        &[],
        compiler.clone(),
    )?;
    if filesystem_derivation.base_source_digest != base_identity.raw_source_digest()
        || memory_derivation.base_source_digest != base_identity.raw_source_digest()
    {
        return Err(
            "the nested filesystem question derives from another base text than this one".into(),
        );
    }
    let family = CompositionFamilyInput::UnsupportedClose {
        filesystem: Box::new((*filesystem).clone()),
    };
    if family.semantic_profile()
        != qcore_schemas::composition_summary::UNSUPPORTED_CLOSE_SEMANTIC_PROFILE
    {
        return Err("unsupported close family input answers another semantic profile".into());
    }
    if !family.matches(prerequisites) {
        return Err("unsupported close family and prerequisite set disagree".into());
    }
    if family.wrapper().paths.is_empty() {
        return Err("unsupported close question carries no complete wrapper path".into());
    }
    let result = UnsupportedCloseSourceResult::Import(Box::new(UnsupportedCloseSourceImport {
        base_identity,
        input: CompositionSourceInput {
            schema_version: COMPOSITION_SOURCE_INPUT_VERSION,
            prerequisites: (*prerequisites).clone(),
            family,
        },
    }));
    if ClangIdentity::capture()? != compiler {
        return Err("Clang identity changed during unsupported close source import".into());
    }
    Ok(result)
}

/// Build the four prerequisite callee identities from the bodies the core's own
/// prerequisite tasks checked. A caller cannot invent one: three of the four
/// pairs are read off a summary the core imported, and the fourth is the frame
/// helper's function with the prototype the composition's own replay requires.
#[must_use]
pub fn memory_close_callees(
    reset: (&str, &str),
    backend: &SourceSummary,
    unlink: &SourceSummary,
    counter: &SourceSummary,
) -> MemoryCloseCalleeRecord {
    let identity = |function: &str, signature: &str| CalleeIdentityRecord {
        function: function.to_owned(),
        signature: signature.to_owned(),
    };
    MemoryCloseCalleeRecord {
        reset: identity(reset.0, reset.1),
        backend: identity(backend.function(), backend.signature()),
        unlink: identity(unlink.function(), unlink.signature()),
        counter: identity(counter.function(), counter.signature()),
    }
}
