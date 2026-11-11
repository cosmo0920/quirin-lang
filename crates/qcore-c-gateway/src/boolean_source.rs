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

//! Bounded, source-derived contracts for single-chunk Boolean field functions.
//!
//! Clang's typed AST is the input boundary. Every statement and expression must
//! belong to the supported grammar before either initial Boolean state is
//! evaluated. Calls need independently source-validated constant-field helper
//! bodies. Other calls, pointer aliases, loops, and fallthrough remain Unknown. A summary describes a borrowed live chunk, not complete teardown or
//! C memory safety. In particular, Fluent Bit's release_lock clears `busy`; it
//! does not unlock the underlying ChunkIO chunk.

use crate::typed_c_ast::{ast_declarations, function_ast};
use qcore_contract::profile_loader::LoadedProfile;
use qcore_schemas::profile::{BooleanFieldModel, OperationKind};
use qcore_schemas::reason::ReasonCategory;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::legacy_c::{
    C11TranslationUnit, enumerate_source_definitions, validate_unrestricted_c11,
};

use qcore_schemas::chunk_summary::{
    BOOLEAN_FIELD_SOURCE_INPUT_VERSION, BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION,
    BooleanFieldExpression, BooleanFieldResult, BooleanFieldSourceInput, BooleanFieldStatement,
    ChunkFrameSummaryRecord, ChunkSourceSummaryRecord,
};
pub use qcore_schemas::chunk_summary::{ChunkSourceField, ChunkSourcePath};

/// Importer-captured provenance for one validated preprocessing snapshot.
///
/// This records the trusted Clang description and exact input/configuration
/// digests; it is not an executable attestation or a Git checkout proof. The
/// caller must bind the declared revision to its repository separately. The
/// compiler executable/environment are trusted and must stay stable during use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceInputIdentity {
    raw_source_digest: String,
    preprocessed_source_digest: String,
    configuration_digest: String,
    dialect: String,
    source_contract_digest: String,
    profile_digest: Option<String>,
    model_library: Option<String>,
    source_revision: String,
    compiler: CompilerIdentity,
    helpers: BTreeMap<String, SourceInputIdentity>,
}

/// Historical name of the record above, retained for existing callers. The
/// record was never Boolean-specific; the counter importer captures the same
/// provenance for its own typed path projection.
pub type BooleanSourceIdentity = SourceInputIdentity;

impl SourceInputIdentity {
    pub fn raw_source_digest(&self) -> &str {
        &self.raw_source_digest
    }
    pub fn preprocessed_source_digest(&self) -> &str {
        &self.preprocessed_source_digest
    }
    pub fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }
    pub fn dialect(&self) -> &str {
        &self.dialect
    }
    pub fn source_contract_digest(&self) -> &str {
        &self.source_contract_digest
    }
    pub fn profile_digest(&self) -> Option<&str> {
        self.profile_digest.as_deref()
    }
    /// Unique loaded library owning the selected ABI/field model. Helpers have
    /// no field-model owner and report `None`.
    pub fn model_library(&self) -> Option<&str> {
        self.model_library.as_deref()
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn compiler_version(&self) -> &str {
        self.compiler.version()
    }
    pub fn compiler_target(&self) -> &str {
        self.compiler.target()
    }
    pub fn helpers(&self) -> &BTreeMap<String, SourceInputIdentity> {
        &self.helpers
    }
}

// The compiler description is shared with the configured audit's compile-task
// identity. Field names, order and serialization are unchanged by that reuse.
use crate::clang::ClangIdentity as CompilerIdentity;

pub(crate) fn input_identity(
    unit: &C11TranslationUnit<'_>,
    preprocessed: &str,
    revision: &str,
    rules: &str,
    profile_digest: Option<String>,
    helpers: &[ChunkFrameSummary],
    compiler: CompilerIdentity,
) -> Result<SourceInputIdentity, String> {
    let scope = match unit.function_scope {
        crate::legacy_c::LegacyFunctionScope::PreprocessedTranslationUnit => {
            "preprocessed_translation_unit"
        }
        crate::legacy_c::LegacyFunctionScope::MainSourceDefinitions => "main_source_definitions",
    };
    let mut configuration = serde_json::to_vec(&(
        unit.dialect.clang_argument(),
        &unit.include_paths,
        &unit.definitions,
        scope,
        std::env::current_dir().map_err(|error| error.to_string())?,
        &compiler,
    ))
    .map_err(|error| error.to_string())?;
    // Keep historical identities byte-for-byte when no ordered options exist.
    // New configurations bind the typed sequence, including include classes.
    if !unit.preprocessor_options.is_empty() {
        configuration.extend_from_slice(b"\nqcore.ordered-preprocessor.v1:");
        configuration.extend(
            serde_json::to_vec(&unit.preprocessor_options).map_err(|error| error.to_string())?,
        );
    }
    // The directory above is the gateway process's, which is what the syntax
    // and typed-AST phases read preprocessed text from. A declared file makes
    // preprocessing run somewhere else, on a named input, so bind that too:
    // the recorded identity must name every path the tools actually saw. Units
    // transported through standard input keep their historical identity bytes.
    if let crate::legacy_c::CSourceInput::DeclaredFile { spelled, directory } = &unit.input {
        configuration.extend_from_slice(b"\nqcore.declared-input.v1:");
        configuration
            .extend(serde_json::to_vec(&(spelled, directory)).map_err(|error| error.to_string())?);
    }
    let mut dependencies = BTreeMap::new();
    for helper in helpers {
        if dependencies
            .insert(
                helper.function().to_owned(),
                helper.input_identity().clone(),
            )
            .is_some()
        {
            return Err("duplicate helper identity".into());
        }
    }
    Ok(SourceInputIdentity {
        raw_source_digest: qcore_schemas::sha256_hex(unit.source.as_bytes()),
        preprocessed_source_digest: qcore_schemas::sha256_hex(preprocessed.as_bytes()),
        configuration_digest: qcore_schemas::sha256_hex(&configuration),
        dialect: unit
            .dialect
            .clang_argument()
            .trim_start_matches("-std=")
            .into(),
        source_contract_digest: qcore_schemas::sha256_hex(rules.as_bytes()),
        profile_digest,
        model_library: None,
        source_revision: revision.to_owned(),
        compiler,
        helpers: dependencies,
    })
}

/// Constructed only by the typed source importer. The schema record alone
/// cannot be promoted into this source-checked wrapper.
///
/// ```compile_fail
/// use qcore_c_gateway::boolean_source::ChunkSourceSummary;
/// use qcore_schemas::chunk_summary::ChunkSourceSummaryRecord;
/// fn forge(record: ChunkSourceSummaryRecord) -> ChunkSourceSummary { record.into() }
/// ```
///
/// ```compile_fail
/// use qcore_c_gateway::boolean_source::ChunkSourceSummary;
/// let _: ChunkSourceSummary = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ChunkSourceSummary(
    ChunkSourceSummaryRecord,
    #[serde(skip)] SourceInputIdentity,
    #[serde(skip)] BooleanFieldStatement,
);

impl std::ops::Deref for ChunkSourceSummary {
    type Target = ChunkSourceSummaryRecord;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ChunkSourceSummary {
    /// Captured typed statement program; no expected execution paths are sent.
    pub fn source_input(&self) -> BooleanFieldSourceInput {
        BooleanFieldSourceInput {
            result_kind: self.result_kind,
            schema_version: match self.result_kind {
                BooleanFieldResult::I32 => BOOLEAN_FIELD_SOURCE_INPUT_VERSION,
                BooleanFieldResult::Unit => BOOLEAN_FIELD_UNIT_SOURCE_INPUT_VERSION,
            },
            function: self.function.clone(),
            source_revision: self.source_revision.clone(),
            field: self.field,
            program: self.2.clone(),
            dependencies: self.dependencies.clone(),
            profile_assumptions: self.profile_assumptions.clone(),
        }
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.1
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }

    pub fn field(&self) -> ChunkSourceField {
        self.field
    }

    pub fn paths(&self) -> &[ChunkSourcePath; 2] {
        &self.paths
    }

    pub fn dependencies(&self) -> &BTreeMap<String, String> {
        &self.dependencies
    }

    pub fn profile_assumptions(&self) -> &BTreeMap<String, String> {
        &self.profile_assumptions
    }
}

/// A checked, nonrecursive helper with one borrowed record pointer and only
/// constant writes to ordinary int fields outside its protected model fields.
/// Source text is retained; the audit must establish cross-unit ABI identity.
/// Ordered field/type vectors do not establish recursive offsets or alignment.
///
/// ```compile_fail
/// use qcore_c_gateway::boolean_source::ChunkFrameSummary;
/// use qcore_schemas::chunk_summary::ChunkFrameSummaryRecord;
/// fn forge(record: ChunkFrameSummaryRecord) -> ChunkFrameSummary { record.into() }
/// ```
///
/// ```compile_fail
/// use qcore_c_gateway::boolean_source::ChunkFrameSummary;
/// let _: ChunkFrameSummary = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ChunkFrameSummary(
    ChunkFrameSummaryRecord,
    #[serde(skip)] SourceInputIdentity,
    #[serde(skip)] FrameBinding,
);

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrameBinding {
    record: String,
    protected_fields: BTreeSet<String>,
    profile: Option<(String, String)>,
}

impl std::ops::Deref for ChunkFrameSummary {
    type Target = ChunkFrameSummaryRecord;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ChunkFrameSummary {
    /// Exact C record name checked against the helper's pointer parameter.
    pub fn record(&self) -> &str {
        &self.2.record
    }

    /// Fields the imported helper is intrinsically forbidden to write.
    pub fn protected_fields(&self) -> &BTreeSet<String> {
        &self.2.protected_fields
    }

    /// Identity captured by the importer, never supplied by a schema record.
    pub fn input_identity(&self) -> &SourceInputIdentity {
        &self.1
    }

    /// The declared profile selection this helper was imported under: the
    /// model library owning the record's Boolean models, and the digest of the
    /// loaded profile manifest. A legacy import carries no profile and reports
    /// `None`.
    ///
    /// Neither value appears in `SourceInputIdentity` — a helper has no
    /// field-model owner, so `model_library` and `profile_digest` are `None`
    /// there — and neither is serialized with the summary. A verification task
    /// that binds this helper must therefore bind this pair separately, or two
    /// different profile selections over one body would record one identity.
    pub fn profile_binding(&self) -> Option<(&str, &str)> {
        self.2
            .profile
            .as_ref()
            .map(|(library, digest)| (library.as_str(), digest.as_str()))
    }

    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn writes(&self) -> &[(String, i32)] {
        &self.writes
    }
}

fn record_layout(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    record_name: &str,
) -> Result<Vec<(String, String)>, String> {
    let declarations = ast_declarations(source, unit, record_name)?;
    let exact: Vec<_> = declarations
        .iter()
        .filter(|node| {
            kind(node) == "RecordDecl"
                && node["name"].as_str() == Some(record_name)
                && node["completeDefinition"] == true
                && node["tagUsed"] == "struct"
        })
        .collect();
    let [record] = exact.as_slice() else {
        return Err("expected one ordinary declared record".into());
    };
    children(record)
        .iter()
        .map(|field| {
            if kind(field) != "FieldDecl"
                || field["isBitfield"] == true
                || !children(field).is_empty()
            {
                return Err("chunk dependency layout has unmodeled attributes or fields".into());
            }
            Ok((
                field["name"].as_str().ok_or("unnamed chunk field")?.into(),
                field["type"]["qualType"]
                    .as_str()
                    .ok_or("missing chunk field type")?
                    .into(),
            ))
        })
        .collect()
}

/// Import the existing bounded constant-field helper grammar. The caller
/// supplies the expected revision and acceptance-rule text explicitly. Revision
/// equality checks this declaration only; it does not establish checkout identity.
pub fn import_chunk_frame_helper(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    expected_revision: &str,
    function: &str,
    source_contract_rules: &str,
) -> Result<ChunkFrameSummary, String> {
    import_frame_helper(
        unit,
        revision,
        expected_revision,
        function,
        "cio_chunk",
        &BTreeSet::from(["busy".into(), "lock".into()]),
        None,
        source_contract_rules,
    )
}

/// Import a helper against an explicit declared record model. Every Boolean
/// field modeled for this record in the full profile is protected, including
/// fields belonging to libraries not selected for the eventual caller import.
/// Profile selection is retained inside the opaque helper; the parent import
/// rechecks it and serializes its existing complete profile identity.
pub fn import_chunk_frame_helper_with_profile(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    function: &str,
    profile: &LoadedProfile,
    model_library: &str,
    record: &str,
    source_contract_rules: &str,
) -> Result<ChunkFrameSummary, String> {
    let library = profile
        .library(model_library)
        .ok_or("unknown helper model library")?;
    let models = library
        .boolean_field_models
        .iter()
        .filter(|model| model.record == record)
        .collect::<Vec<_>>();
    if models.is_empty() {
        return Err("helper record has no selected declared Boolean model".into());
    }
    for model in models {
        field_kind(model)?;
    }
    let pin = profile
        .pin_for(model_library)
        .ok_or("missing helper source pin")?;
    let protected = protected_fields(profile, record);
    let digest = qcore_schemas::sha256_hex(
        &serde_json::to_vec(profile.manifest()).map_err(|e| e.to_string())?,
    );
    import_frame_helper(
        unit,
        revision,
        &pin.git_commit,
        function,
        record,
        &protected,
        Some((model_library.into(), digest)),
        source_contract_rules,
    )
}

fn protected_fields(profile: &LoadedProfile, record: &str) -> BTreeSet<String> {
    profile
        .manifest()
        .libraries
        .values()
        .flat_map(|library| &library.boolean_field_models)
        .filter(|model| model.record == record)
        .map(|model| model.field.clone())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn import_frame_helper(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    expected_revision: &str,
    function: &str,
    record: &str,
    protected: &BTreeSet<String>,
    profile_binding: Option<(String, String)>,
    source_contract_rules: &str,
) -> Result<ChunkFrameSummary, String> {
    if revision != expected_revision {
        return Err("chunk helper source revision mismatch".into());
    }
    let compiler = CompilerIdentity::capture()?;
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let ast = function_ast(&source, unit, function)?;
    if ast["type"]["qualType"] != format!("void (struct {record} *)") {
        return Err("helper requires one declared record pointer and void result".into());
    }
    let [parameter, body] = children(&ast) else {
        return Err("unmodeled helper parameters or attributes".into());
    };
    if kind(parameter) != "ParmVarDecl"
        || !children(parameter).is_empty()
        || kind(body) != "CompoundStmt"
    {
        return Err("unmodeled helper definition".into());
    }
    let layout = record_layout(&source, unit, record)?;
    let id = parameter["id"]
        .as_str()
        .ok_or("missing helper parameter identity")?;
    let mut writes = Vec::new();
    for statement in children(body) {
        let [place, expression] = children(statement) else {
            return Err("helper requires direct constant field assignments".into());
        };
        let field = place["name"]
            .as_str()
            .ok_or("helper requires a named field")?;
        if kind(statement) != "BinaryOperator" || statement["opcode"] != "=" {
            return Err("helper requires a direct assignment".into());
        }
        if protected.contains(field) || !layout.contains(&(field.into(), "int".into())) {
            return Err(
                "helper must write only ordinary int fields outside the declared Boolean models"
                    .into(),
            );
        }
        qcore_contract::contract_observation::check_contract_facts(
            source_contract_rules,
            "frame_write",
            &[
                ("lifecycle_field", i128::from(protected.contains(field))),
                (
                    "ordinary_int_field",
                    i128::from(layout.contains(&(field.into(), "int".into()))),
                ),
            ],
        )?;
        let mut importer = Importer {
            result_kind: BooleanFieldResult::Unit,
            parameter_id: id,
            record_name: record,
            field_name: field,
            used_field: false,
            helpers: &BTreeMap::new(),
            dependencies: BTreeMap::new(),
        };
        if !importer.field(place) {
            return Err("helper field is not bound to its borrowed parameter".into());
        }
        let expr = importer.expression(expression, 0)?;
        // Constant syntax only: checking two sample values would not establish
        // independence from arbitrary initial error/metadata fields.
        fn constant(expr: &Expression) -> bool {
            match expr {
                Expression::Integer(_) => true,
                Expression::Negate(value) => constant(value),
                _ => false,
            }
        }
        if !constant(&expr) {
            return Err("helper assignment is not a constant".into());
        }
        writes.push((field.into(), expr.evaluate(false)?));
    }
    if writes.is_empty() {
        return Err("helper has no checked field writes".into());
    }
    let identity = input_identity(
        unit,
        &source,
        revision,
        source_contract_rules,
        None,
        &[],
        compiler.clone(),
    )?;
    if CompilerIdentity::capture()? != compiler {
        return Err("Clang identity changed during Boolean source import".into());
    }
    Ok(ChunkFrameSummary(
        ChunkFrameSummaryRecord {
            function: function.into(),
            source,
            layout,
            writes,
        },
        identity,
        FrameBinding {
            record: record.into(),
            protected_fields: protected.clone(),
            profile: profile_binding,
        },
    ))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ChunkSourceResult {
    /// A complete field-effect summary, not a Safe translation-unit verdict.
    Summary(Box<ChunkSourceSummary>),
    Unknown {
        function: String,
        reason: String,
        /// The closed §L21.1 category of `reason` (card REASON-CODES-B2).
        ///
        /// This *refines* the row: `function` and `reason` keep their exact
        /// bytes, and every consumer that already read the free text is
        /// unaffected. It is fail-closed in the two directions the vocabulary
        /// requires — a reason the closed vocabulary does not recognise carries
        /// `None`, never a defaulted category, and `None` asserts nothing, so
        /// an unrecognised row can never be read as a statement about the
        /// analysed source. Consumers that used to re-derive the category from
        /// the free text now read it from here, so this enum stops being a
        /// second, uncarried reason vocabulary.
        reason_code: Option<ReasonCategory>,
    },
}

impl ChunkSourceResult {
    /// Build an `Unknown` row, classifying its reason under the closed
    /// vocabulary. This is the only way an `Unknown` row is minted in
    /// production, so the category can never disagree with the text beside it.
    pub fn unknown(function: impl Into<String>, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let reason_code = ReasonCategory::classify(&reason).ok();
        Self::Unknown {
            function: function.into(),
            reason,
            reason_code,
        }
    }

    /// The closed category of this row's reason, or `None` for a summary row
    /// and for a reason the closed vocabulary does not recognise.
    pub fn reason_code(&self) -> Option<ReasonCategory> {
        match self {
            Self::Summary(_) => None,
            Self::Unknown { reason_code, .. } => *reason_code,
        }
    }
}

/// Import selected single-field models from explicit declared profile data.
/// Source-contract acceptance rules are supplied by the trusted caller; their
/// identity and the profile must be bound by the outer audit/replay boundary. The revision
/// is a caller assertion, as in the compatibility API; the outer audit must bind
/// actual checkout/build identity. Missing models never gain an implicit route.
/// The model/assumption mapping is part of every returned bounded summary.
pub fn import_chunk_source_with_profile(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
    profile: &LoadedProfile,
    libraries: &[&str],
    helpers: &[ChunkFrameSummary],
    source_contract_rules: &str,
) -> Result<Vec<ChunkSourceResult>, String> {
    let mut models = BTreeMap::new();
    let mut model_libraries = BTreeMap::new();
    for name in libraries {
        let library = profile
            .library(name)
            .ok_or_else(|| format!("unknown field-model library `{name}`"))?;
        let pin = profile
            .pin_for(name)
            .ok_or("missing field-model source pin")?;
        if source_revision != pin.git_commit {
            return Err("ChunkIO source revision does not match the pinned revision".into());
        }
        for model in &library.boolean_field_models {
            field_kind(model)?;
            // The supported function ABI has one record pointer. Multiple field
            // routes for that record are ambiguous until a richer importer exists.
            for result in ["int", "void"] {
                let abi = format!("{result} (struct {} *)", model.record);
                if models.insert(abi.clone(), model).is_some() {
                    return Err("ambiguous Boolean field models for one record ABI".into());
                }
                model_libraries.insert(abi, (*name).to_owned());
            }
        }
    }
    let compiler = CompilerIdentity::capture()?;
    let source = validate_unrestricted_c11(unit).map_err(|error| error.to_string())?;
    let mut checked_helpers = BTreeMap::new();
    if !helpers.is_empty() {
        let profile_digest = qcore_schemas::sha256_hex(
            &serde_json::to_vec(profile.manifest()).map_err(|e| e.to_string())?,
        );
        for helper in helpers {
            let record = &helper.2.record;
            let abi = format!("int (struct {record} *)");
            if !models.contains_key(&abi) {
                return Err("helper record is not a selected Boolean model".into());
            }
            if let Some((library, digest)) = &helper.2.profile
                && (model_libraries.get(&abi) != Some(library)
                    || digest != &profile_digest
                    || helper.input_identity().source_revision() != source_revision
                    || helper.input_identity().dialect()
                        != unit.dialect.clang_argument().trim_start_matches("-std=")
                    || helper.1.compiler != compiler
                    || helper.input_identity().source_contract_digest()
                        != qcore_schemas::sha256_hex(source_contract_rules.as_bytes()))
            {
                return Err("helper source/build/profile selection mismatch".into());
            }
            let protected = protected_fields(profile, record);
            if helper
                .writes
                .iter()
                .any(|(field, _)| protected.contains(field))
            {
                return Err("helper writes a declared Boolean model field".into());
            }
            let layout = record_layout(&source, unit, record)?;
            let declarations = ast_declarations(&source, unit, &helper.function)?;
            let exact: Vec<_> = declarations
                .iter()
                .filter(|node| kind(node) == "FunctionDecl" && node["name"] == helper.function)
                .collect();
            let external_prototypes = exact.iter().all(|decl| {
                decl["storageClass"] != "static"
                    && decl["type"]["qualType"] == format!("void (struct {record} *)")
                    && children(decl)
                        .iter()
                        .all(|child| kind(child) == "ParmVarDecl")
            });
            if exact.is_empty()
                || !external_prototypes
                || helper.layout != layout
                || checked_helpers.contains_key(&helper.function)
            {
                return Err("helper declaration, structural layout, or uniqueness mismatch".into());
            }
            qcore_contract::contract_observation::check_contract_facts(
                source_contract_rules,
                "frame_declaration",
                &[
                    ("declaration_count", exact.len() as i128),
                    ("all_external_prototypes", i128::from(external_prototypes)),
                ],
            )?;
            qcore_contract::contract_observation::check_contract_facts(
                source_contract_rules,
                "frame_dependency",
                &[
                    ("layout_matches", i128::from(helper.layout == layout)),
                    (
                        "duplicate",
                        i128::from(checked_helpers.contains_key(&helper.function)),
                    ),
                ],
            )?;
            checked_helpers.insert(helper.function.clone(), helper);
        }
    }
    let layouts = models
        .iter()
        .map(|(abi, model)| {
            (
                abi.as_str(),
                validate_field_layout(&source, unit, &model.record, &model.field),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let identity = input_identity(
        unit,
        &source,
        source_revision,
        source_contract_rules,
        Some(qcore_schemas::sha256_hex(
            &serde_json::to_vec(&(profile.manifest(), libraries))
                .map_err(|error| error.to_string())?,
        )),
        helpers,
        compiler.clone(),
    )?;
    let mut results = Vec::new();
    // Unit E01-F1: the imported denominator is the enumerated main-source
    // definition set, so a definition produced only by macro expansion reaches
    // the typed importer instead of silently carrying neither a contract nor an
    // Unknown row. Nothing here reads a K&R body: such a definition has no
    // typed AST entry under this importer's lookup and returns `Unknown` with
    // its own reason, which is the honest outcome, not a new capability.
    for function in enumerate_source_definitions(unit).definitions().to_vec() {
        let result = function_ast(&source, unit, &function).and_then(|ast| {
            let abi = ast["type"]["qualType"].as_str().unwrap_or("");
            let model = models
                .get(abi)
                .ok_or("outside single borrowed chunk / int-or-void-result source profile")?;
            let mut identity = identity.clone();
            identity.model_library = Some(model_libraries[abi].clone());
            let mut summary = derive_summary(
                &ast,
                &function,
                source_revision,
                &checked_helpers,
                model,
                &identity,
            )?;
            layouts[abi].clone()?;
            summary.0.profile_assumptions = profile.assumptions();
            if summary.result_kind == BooleanFieldResult::Unit {
                summary
                    .0
                    .profile_assumptions
                    .insert("boolean_field_record".into(), model.record.clone());
                summary
                    .0
                    .profile_assumptions
                    .insert("boolean_field_name".into(), model.field.clone());
            }
            Ok(summary)
        });
        results.push(match result {
            Ok(summary) => ChunkSourceResult::Summary(Box::new(summary)),
            Err(reason) => ChunkSourceResult::unknown(function, reason),
        });
    }
    if CompilerIdentity::capture()? != compiler {
        return Err("Clang identity changed during Boolean source import".into());
    }
    Ok(results)
}

fn validate_field_layout(
    source: &str,
    unit: &C11TranslationUnit<'_>,
    record: &str,
    field: &str,
) -> Result<(), String> {
    let definitions = ast_declarations(source, unit, record)?;
    let records = definitions
        .iter()
        .filter(|node| {
            kind(node) == "RecordDecl"
                && node["name"].as_str() == Some(record)
                && node["completeDefinition"] == true
                && node["tagUsed"] == "struct"
        })
        .collect::<Vec<_>>();
    let [record] = records.as_slice() else {
        return Err("expected one complete chunk record definition".into());
    };
    let fields = children(record)
        .iter()
        .filter(|node| kind(node) == "FieldDecl" && node["name"].as_str() == Some(field))
        .collect::<Vec<_>>();
    let [field] = fields.as_slice() else {
        return Err("expected one direct lifecycle field".into());
    };
    if field["type"]["qualType"] != "int" || field["isBitfield"] == true {
        return Err("lifecycle field must be an ordinary non-volatile int, not a bitfield".into());
    }
    Ok(())
}

fn kind(node: &Value) -> &str {
    node["kind"].as_str().unwrap_or("")
}

fn children(node: &Value) -> &[Value] {
    node["inner"].as_array().map_or(&[], Vec::as_slice)
}

#[derive(Clone, Debug)]
enum Expression {
    Integer(i32),
    Field,
    Not(Box<Self>),
    Negate(Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),
}

impl Expression {
    fn record(&self) -> BooleanFieldExpression {
        use BooleanFieldExpression as R;
        match self {
            Self::Integer(value) => R::Integer(*value),
            Self::Field => R::Field,
            Self::Not(value) => R::Not(Box::new(value.record())),
            Self::Negate(value) => R::Negate(Box::new(value.record())),
            Self::Equal(left, right) => R::Equal(Box::new(left.record()), Box::new(right.record())),
            Self::NotEqual(left, right) => {
                R::NotEqual(Box::new(left.record()), Box::new(right.record()))
            }
        }
    }

    fn evaluate(&self, field: bool) -> Result<i32, String> {
        Ok(match self {
            Self::Integer(value) => *value,
            Self::Field => i32::from(field),
            Self::Not(value) => i32::from(value.evaluate(field)? == 0),
            Self::Negate(value) => value
                .evaluate(field)?
                .checked_neg()
                .ok_or("integer overflow in source expression")?,
            Self::Equal(left, right) => i32::from(left.evaluate(field)? == right.evaluate(field)?),
            Self::NotEqual(left, right) => {
                i32::from(left.evaluate(field)? != right.evaluate(field)?)
            }
        })
    }
}

#[derive(Clone, Debug)]
enum Statement {
    Block(Vec<Self>),
    If(Expression, Box<Self>, Option<Box<Self>>),
    Assign(Expression),
    Return(Expression),
    ReturnUnit,
    FrameWrites(Vec<(String, i32)>),
}

impl Statement {
    fn record(&self) -> BooleanFieldStatement {
        use BooleanFieldStatement as R;
        match self {
            Self::Block(statements) => R::Block(statements.iter().map(Self::record).collect()),
            Self::If(condition, yes, no) => R::If(
                condition.record(),
                Box::new(yes.record()),
                no.as_ref().map(|value| Box::new(value.record())),
            ),
            Self::Assign(value) => R::Assign(value.record()),
            Self::Return(value) => R::Return(value.record()),
            Self::ReturnUnit => R::ReturnUnit,
            Self::FrameWrites(writes) => R::FrameWrites(writes.clone()),
        }
    }

    fn execute(&self, path: &mut ChunkSourcePath) -> Result<Option<i32>, String> {
        match self {
            Self::Block(statements) => {
                for statement in statements {
                    if let Some(value) = statement.execute(path)? {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            }
            Self::If(condition, yes, no) => {
                let taken = condition.evaluate(path.final_value)? != 0;
                path.branches.push(taken);
                if taken {
                    yes.execute(path)
                } else if let Some(no) = no {
                    no.execute(path)
                } else {
                    Ok(None)
                }
            }
            Self::Assign(value) => {
                let value = value.evaluate(path.final_value)?;
                if !matches!(value, 0 | 1) {
                    return Err("field assignment leaves the Boolean lifecycle domain".into());
                }
                let value = value != 0;
                path.writes.push(value);
                path.final_value = value;
                Ok(None)
            }
            Self::Return(value) => value.evaluate(path.final_value).map(Some),
            Self::ReturnUnit => Ok(Some(0)),
            Self::FrameWrites(writes) => {
                path.frame_writes.extend(writes.clone());
                Ok(None)
            }
        }
    }
}

struct Importer<'a> {
    result_kind: BooleanFieldResult,
    parameter_id: &'a str,
    record_name: &'a str,
    field_name: &'a str,
    used_field: bool,
    helpers: &'a BTreeMap<String, &'a ChunkFrameSummary>,
    dependencies: BTreeMap<String, String>,
}

impl Importer<'_> {
    fn field(&mut self, node: &Value) -> bool {
        let inner = children(node);
        if kind(node) != "MemberExpr"
            || node["name"].as_str() != Some(self.field_name)
            || node["isArrow"].as_bool() != Some(true)
            || node["type"]["qualType"].as_str() != Some("int")
            || inner.len() != 1
        {
            return false;
        }
        let mut base = &inner[0];
        if kind(base) == "ImplicitCastExpr"
            && base["castKind"] == "LValueToRValue"
            && children(base).len() == 1
        {
            base = &children(base)[0];
        }
        let accepted = kind(base) == "DeclRefExpr"
            && base["referencedDecl"]["kind"] == "ParmVarDecl"
            && base["referencedDecl"]["id"].as_str() == Some(self.parameter_id);
        self.used_field |= accepted;
        accepted
    }

    fn expression(&mut self, node: &Value, depth: usize) -> Result<Expression, String> {
        if depth > 64 {
            return Err("source expression exceeds the import depth limit".into());
        }
        let inner = children(node);
        match (kind(node), inner) {
            ("IntegerLiteral", []) if node["type"]["qualType"] == "int" => node["value"]
                .as_str()
                .and_then(|value| value.parse().ok())
                .map(Expression::Integer)
                .ok_or_else(|| "unsupported integer literal".into()),
            ("ImplicitCastExpr", [value]) if node["castKind"] == "LValueToRValue" => {
                self.expression(value, depth + 1)
            }
            ("ParenExpr", [value]) => self.expression(value, depth + 1),
            ("MemberExpr", _) if self.field(node) => Ok(Expression::Field),
            ("UnaryOperator", [value]) if node["opcode"] == "!" => Ok(Expression::Not(Box::new(
                self.expression(value, depth + 1)?,
            ))),
            ("UnaryOperator", [value]) if node["opcode"] == "-" => Ok(Expression::Negate(
                Box::new(self.expression(value, depth + 1)?),
            )),
            ("BinaryOperator", [left, right])
                if node["opcode"] == "==" || node["opcode"] == "!=" =>
            {
                let left = Box::new(self.expression(left, depth + 1)?);
                let right = Box::new(self.expression(right, depth + 1)?);
                Ok(if node["opcode"] == "==" {
                    Expression::Equal(left, right)
                } else {
                    Expression::NotEqual(left, right)
                })
            }
            _ => Err(format!("unmodeled expression {}", kind(node))),
        }
    }

    fn statement(&mut self, node: &Value, depth: usize) -> Result<Statement, String> {
        if depth > 64 {
            return Err("source statement exceeds the import depth limit".into());
        }
        match (kind(node), children(node)) {
            ("CallExpr", [callee, argument]) => {
                let [reference] = children(callee) else {
                    return Err("unmodeled helper callee".into());
                };
                let [parameter] = children(argument) else {
                    return Err("helper requires direct borrowed parameter".into());
                };
                let name = reference["referencedDecl"]["name"]
                    .as_str()
                    .ok_or("missing helper identity")?;
                if kind(callee) != "ImplicitCastExpr"
                    || callee["castKind"] != "FunctionToPointerDecay"
                    || kind(reference) != "DeclRefExpr"
                    || reference["referencedDecl"]["kind"] != "FunctionDecl"
                    || reference["type"]["qualType"]
                        != format!("void (struct {} *)", self.record_name)
                    || kind(argument) != "ImplicitCastExpr"
                    || argument["castKind"] != "LValueToRValue"
                    || kind(parameter) != "DeclRefExpr"
                    || parameter["referencedDecl"]["kind"] != "ParmVarDecl"
                    || parameter["referencedDecl"]["id"].as_str() != Some(self.parameter_id)
                {
                    return Err("helper signature or parameter binding mismatch".into());
                }
                let helper = self
                    .helpers
                    .get(name)
                    .ok_or("call lacks source-validated frame helper")?;
                if helper.2.record != self.record_name {
                    return Err("helper record differs from caller model record".into());
                }
                self.dependencies.insert(name.into(), helper.source.clone());
                Ok(Statement::FrameWrites(helper.writes.clone()))
            }
            ("CompoundStmt", inner) => inner
                .iter()
                .map(|node| self.statement(node, depth + 1))
                .collect::<Result<_, _>>()
                .map(Statement::Block),
            ("IfStmt", [condition, yes]) if node["hasElse"] != true => Ok(Statement::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                None,
            )),
            ("IfStmt", [condition, yes, no]) if node["hasElse"] == true => Ok(Statement::If(
                self.expression(condition, depth + 1)?,
                Box::new(self.statement(yes, depth + 1)?),
                Some(Box::new(self.statement(no, depth + 1)?)),
            )),
            ("BinaryOperator", [place, value]) if node["opcode"] == "=" && self.field(place) => {
                Ok(Statement::Assign(self.expression(value, depth + 1)?))
            }
            ("ReturnStmt", [value]) if self.result_kind == BooleanFieldResult::I32 => {
                Ok(Statement::Return(self.expression(value, depth + 1)?))
            }
            ("ReturnStmt", []) if self.result_kind == BooleanFieldResult::Unit => {
                Ok(Statement::ReturnUnit)
            }
            _ => Err(format!("unmodeled statement {}", kind(node))),
        }
    }
}

fn field_kind(model: &BooleanFieldModel) -> Result<ChunkSourceField, String> {
    match (model.set_operation, model.clear_operation) {
        (OperationKind::SetBusy, OperationKind::SetBusy) => Ok(ChunkSourceField::Busy),
        (OperationKind::Lock, OperationKind::Unlock) => Ok(ChunkSourceField::Locked),
        _ => Err("unsupported Boolean field operation mapping".into()),
    }
}

fn derive_summary(
    ast: &Value,
    function: &str,
    source_revision: &str,
    helpers: &BTreeMap<String, &ChunkFrameSummary>,
    model: &BooleanFieldModel,
    identity: &SourceInputIdentity,
) -> Result<ChunkSourceSummary, String> {
    let field = field_kind(model)?;
    let field_name = model.field.as_str();
    // Do not silently discard attributes, nested declarations, extra parameters,
    // or other definition children that can change execution semantics.
    let [parameter, body] = children(ast) else {
        return Err("unmodeled function parameters or attributes".into());
    };
    if kind(parameter) != "ParmVarDecl" || kind(body) != "CompoundStmt" {
        return Err("unmodeled function definition".into());
    }
    let result_kind = if ast["type"]["qualType"] == format!("void (struct {} *)", model.record) {
        BooleanFieldResult::Unit
    } else if ast["type"]["qualType"] == format!("int (struct {} *)", model.record) {
        BooleanFieldResult::I32
    } else {
        return Err("unsupported Boolean field result ABI".into());
    };
    let mut importer = Importer {
        result_kind,
        parameter_id: parameter["id"]
            .as_str()
            .ok_or("missing parameter identity")?,
        record_name: &model.record,
        field_name,
        used_field: false,
        helpers,
        dependencies: BTreeMap::new(),
    };
    let statement = importer.statement(body, 0)?;
    if !importer.used_field {
        return Err("function has no modeled chunk field access".into());
    }
    let evaluate = |initial_value| {
        let mut path = ChunkSourcePath {
            initial_value,
            final_value: initial_value,
            return_value: 0,
            branches: Vec::new(),
            writes: Vec::new(),
            frame_writes: Vec::new(),
        };
        path.return_value = match (result_kind, statement.execute(&mut path)?) {
            (BooleanFieldResult::Unit, value) => value.unwrap_or(0),
            (BooleanFieldResult::I32, Some(value)) => value,
            (BooleanFieldResult::I32, None) => {
                return Err("source path falls through without an explicit return".into());
            }
        };
        Ok::<_, String>(path)
    };
    Ok(ChunkSourceSummary(
        ChunkSourceSummaryRecord {
            result_kind,
            function: function.to_owned(),
            source_revision: source_revision.to_owned(),
            field,
            paths: [evaluate(false)?, evaluate(true)?],
            dependencies: importer.dependencies,
            profile_assumptions: BTreeMap::new(),
        },
        identity.clone(),
        statement.record(),
    ))
}
