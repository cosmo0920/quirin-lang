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

//! Compilation-database ingestion, the inherited bounded option adapter, and
//! the compile-task identity that binds one accepted C import configuration.
use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CCompilerDialect, CFrontendFlag, CPreprocessorOption, CSourceInput,
    LegacyFunctionScope, LegacySafetyCertificate,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct CompileEntry {
    pub directory: PathBuf,
    pub command: String,
    pub file: PathBuf,
}

/// Two compilation records describe the same compile task only when their
/// working directory and complete command agree exactly.
///
/// Real CMake databases carry several distinct commands for one source file
/// (static and shared targets, per-target export macros). Those are different
/// configurations that can preprocess to different code, so a caller must keep
/// them apart or reject; silently choosing one would let a convenient build
/// stand in for the one under audit. Textually different spellings of the same
/// command are treated as distinct, which over-rejects rather than under-rejects.
pub fn same_configuration(left: &CompileEntry, right: &CompileEntry) -> bool {
    left.directory == right.directory && left.command == right.command
}

/// Require exactly one distinct configuration for a selected source file.
pub fn select_unique_configuration<'a>(
    entries: &'a [CompileEntry],
    file: &Path,
) -> Result<&'a CompileEntry, String> {
    let distinct = select_configurations(entries, file)?;
    match distinct.as_slice() {
        [] => Err(format!(
            "compilation database contains no command for {}",
            file.display()
        )),
        [entry] => Ok(entry),
        many => Err(format!(
            "compilation database contains {} distinct configurations for {}; the audit will not choose one",
            many.len(),
            file.display()
        )),
    }
}

/// Every distinct configuration of `file` in the database, in database
/// order (card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: a pin-time derivation
/// reads each one, so no configuration is chosen over another).
///
/// # Errors
///
/// Two spellings of the file (`input-path-alias`).
pub fn select_configurations<'a>(
    entries: &'a [CompileEntry],
    file: &Path,
) -> Result<Vec<&'a CompileEntry>, String> {
    let mut distinct = Vec::<&CompileEntry>::new();
    // Card AUDIT-CANONICAL-PATHS: records match by located path, and two
    // spellings of the file are refused (`input-path-alias`) rather than
    // merged by `same_configuration` below.
    for entry in crate::input_paths::select_exact(entries, file)? {
        if !distinct.iter().any(|kept| same_configuration(kept, entry)) {
            distinct.push(entry);
        }
    }
    Ok(distinct)
}

/// Schema of the recorded compile-task identity.
pub const COMPILE_TASK_IDENTITY_VERSION: u32 = 1;

/// Complete, ordered description of one accepted compile task.
///
/// It binds the Clang that actually performs the analysis (program, version and
/// default target), the driver the database declared, the compile working
/// directory, the main source path, the complete ordered argument vector, the
/// dialect, the ordered transported preprocessing options and legacy include/
/// definition fields, the analyzed function scope, the raw source bytes, the
/// effective preprocessing input, and the caller's named dependency digests
/// (contract manifest, source profile, compilation database, source rules).
///
/// Deliberate exclusions, recorded in `docs/reports/remaining-g01/README.md`:
/// individual header files are not enumerated, only bound by their effect on
/// the preprocessed token stream, which `-E -P` strips of line markers; a
/// distinct header file with identical relevant contents is therefore the same
/// task. The declared driver is recorded but never checked for equivalence with
/// the Clang that ran, except where `compiler_options` already demands the
/// configured Clang for explicit frontend build flags.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CompileTaskIdentity {
    schema_version: u32,
    compiler: ClangIdentity,
    declared_driver: String,
    directory: PathBuf,
    file: PathBuf,
    arguments: Vec<String>,
    dialect: &'static str,
    function_scope: &'static str,
    /// How the primary input actually reached Clang. `directory`, `file` and
    /// `arguments` above describe the compile record; this describes what the
    /// gateway did with it. Without it, a unit transported through standard
    /// input — where file-name macros expand to `<stdin>` and quoted includes
    /// resolve elsewhere — would record the same identity as one Clang read
    /// from the declared path.
    source_input: CSourceInput,
    preprocessor_options: Vec<CPreprocessorOption>,
    include_paths: Vec<String>,
    definitions: Vec<String>,
    raw_source_sha256: String,
    preprocessed_source_sha256: Option<String>,
    preprocessing_error: Option<String>,
    dependencies: BTreeMap<String, String>,
    /// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the digest of the dialect's
    /// registered AST differences, for a dialect that registers any (C23,
    /// C2y). Absent otherwise, so no earlier identity's digest moves.
    #[serde(skip_serializing_if = "Option::is_none")]
    dialect_differences_sha256: Option<String>,
}

/// The effective preprocessing input, or the rejection that replaced it.
/// A rejected task keeps an identity so its Unknown result stays attributable.
#[derive(Clone, Copy, Debug)]
pub enum EffectiveInput<'a> {
    Accepted(&'a str),
    Rejected(&'a str),
}

fn dialect_name(dialect: CCompilerDialect) -> &'static str {
    dialect.std_name()
}

fn scope_name(scope: LegacyFunctionScope) -> &'static str {
    match scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => "preprocessed_translation_unit",
        LegacyFunctionScope::MainSourceDefinitions => "main_source_definitions",
    }
}

/// Bind one accepted compile task to everything the audit read to analyze it.
///
/// The entry must already have passed [`compiler_options`]: every argument is
/// then either transported into `unit` or structurally accounted for, so the
/// recorded argument vector is complete rather than a filtered projection.
pub fn compile_task_identity(
    entry: &CompileEntry,
    unit: &C11TranslationUnit<'_>,
    input: EffectiveInput<'_>,
    compiler: &ClangIdentity,
    dependencies: BTreeMap<String, String>,
) -> Result<CompileTaskIdentity, String> {
    let arguments = shell_words(&entry.command)?;
    let declared_driver = arguments
        .first()
        .filter(|driver| !driver.is_empty())
        .ok_or("empty compiler executable")?
        .clone();
    let (preprocessed_source_sha256, preprocessing_error) = match input {
        EffectiveInput::Accepted(text) => (Some(qcore_schemas::sha256_hex(text.as_bytes())), None),
        EffectiveInput::Rejected(error) => (None, Some(error.to_owned())),
    };
    Ok(CompileTaskIdentity {
        schema_version: COMPILE_TASK_IDENTITY_VERSION,
        compiler: compiler.clone(),
        declared_driver,
        directory: entry.directory.clone(),
        file: entry.file.clone(),
        arguments,
        dialect: dialect_name(unit.dialect),
        function_scope: scope_name(unit.function_scope),
        source_input: unit.input.clone(),
        preprocessor_options: unit.preprocessor_options.clone(),
        include_paths: unit.include_paths.clone(),
        definitions: unit.definitions.clone(),
        raw_source_sha256: qcore_schemas::sha256_hex(unit.source.as_bytes()),
        preprocessed_source_sha256,
        preprocessing_error,
        dependencies,
        dialect_differences_sha256:
            qcore_c_gateway::c_frontend_identity::dialect_differences_sha256(unit.dialect),
    })
}

impl CompileTaskIdentity {
    /// One digest over every bound component. Evidence recorded under a digest
    /// belongs to that task alone; any bound input that changes changes it.
    pub fn digest(&self) -> Result<String, String> {
        Ok(qcore_schemas::sha256_hex(
            &serde_json::to_vec(self).map_err(|error| error.to_string())?,
        ))
    }

    pub fn raw_source_sha256(&self) -> &str {
        &self.raw_source_sha256
    }

    pub fn preprocessed_source_sha256(&self) -> Option<&str> {
        self.preprocessed_source_sha256.as_deref()
    }

    pub fn dependencies(&self) -> &BTreeMap<String, String> {
        &self.dependencies
    }

    pub fn dependency(&self, name: &str) -> Option<&str> {
        self.dependencies.get(name).map(String::as_str)
    }

    /// Refuse an incomplete task identity at a certificate admission boundary.
    ///
    /// Ordinary report generation also records rejected preprocessing tasks,
    /// but a `Safe` certificate can only be bound to an accepted task whose
    /// compiler, target, build request, effective input and two audit trust
    /// dependencies are all present. Equality between two incomplete records
    /// is not provenance.
    pub fn validate_for_certificate(&self) -> Result<(), String> {
        let is_sha256 = |digest: &str| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        };
        if self.schema_version != COMPILE_TASK_IDENTITY_VERSION {
            return Err("unsupported compile-task identity version".into());
        }
        if self.compiler.program().is_empty()
            || self.compiler.version().is_empty()
            || self.compiler.target().is_empty()
        {
            return Err(
                "compile-task identity has incomplete compiler or target provenance".into(),
            );
        }
        if self.declared_driver.is_empty()
            || self.directory.as_os_str().is_empty()
            || self.file.as_os_str().is_empty()
            || self.arguments.is_empty()
        {
            return Err("compile-task identity has incomplete build provenance".into());
        }
        if !is_sha256(&self.raw_source_sha256)
            || self
                .preprocessed_source_sha256
                .as_deref()
                .is_none_or(|digest| !is_sha256(digest))
            || self.preprocessing_error.is_some()
        {
            return Err("compile-task identity has no accepted effective input provenance".into());
        }
        for required in ["resource_contract_manifest", "source_profile"] {
            let digest = self.dependencies.get(required).ok_or_else(|| {
                format!("compile-task identity is missing `{required}` provenance")
            })?;
            if !is_sha256(digest) {
                return Err(format!(
                    "compile-task identity has malformed `{required}` provenance"
                ));
            }
        }
        if self
            .dependencies
            .iter()
            .any(|(name, digest)| name.is_empty() || digest.is_empty())
        {
            return Err("compile-task identity has an empty dependency provenance field".into());
        }
        Ok(())
    }

    /// Check that this otherwise complete task identity describes the gateway
    /// request and retained artifacts inside `certificate`.
    pub fn validate_certificate_binding(
        &self,
        certificate: &LegacySafetyCertificate,
    ) -> Result<(), String> {
        self.validate_for_certificate()?;
        let input = &certificate.target_scope.input_identity;
        let request = &certificate.target_scope.request;
        if self.raw_source_sha256 != input.raw_source_sha256
            || self.preprocessed_source_sha256.as_deref()
                != Some(input.preprocessed_source_sha256.as_str())
            || self.dialect != dialect_name(certificate.dialect)
            || self.function_scope != scope_name(certificate.function_scope)
            || self.source_input != request.input
            || self.preprocessor_options != request.preprocessor_options
            || self.include_paths != request.include_paths
            || self.definitions != request.definitions
        {
            return Err(
                "compile-task identity does not describe the certificate gateway request and artifacts"
                    .into(),
            );
        }
        Ok(())
    }

    /// Check an expected task against the prepared request and artifacts it is
    /// meant to authorize, before looking at any presented certificate.
    pub fn validate_prepared_binding(
        &self,
        prepared: &qcore_c_gateway::legacy_c::PreparedTranslationUnit,
    ) -> Result<(), String> {
        self.validate_for_certificate()?;
        let preprocessed = prepared
            .preprocessed_source()
            .map_err(|error| error.to_string())?;
        let preprocessed_sha256 = qcore_schemas::sha256_hex(preprocessed.as_bytes());
        if self.raw_source_sha256 != qcore_schemas::sha256_hex(prepared.raw_source().as_bytes())
            || self.preprocessed_source_sha256.as_deref() != Some(preprocessed_sha256.as_str())
            || self.dialect != dialect_name(prepared.dialect())
            || self.function_scope != scope_name(prepared.function_scope())
            || &self.source_input != prepared.input()
            || self.preprocessor_options != prepared.preprocessor_options()
            || self.include_paths != prepared.include_paths()
            || self.definitions != prepared.definitions()
        {
            return Err(
                "compile-task identity does not describe the expected prepared request and artifacts"
                    .into(),
            );
        }
        Ok(())
    }
}

pub struct CompilerOptions {
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    pub dialect: CCompilerDialect,
    pub preprocessor_options: Vec<CPreprocessorOption>,
    /// How the gateway must hand this entry's source to Clang.
    pub input: CSourceInput,
}

pub fn compiler_options_compatibility(entry: &CompileEntry) -> Result<CompilerOptions, String> {
    let words = shell_words(&entry.command)?;
    // The gateway preprocesses through stdin, so Clang cannot infer the normal
    // quoted-include search directory from the source filename.  Preserve that
    // part of the compilation-database contract explicitly.
    let mut include_paths = entry
        .file
        .parent()
        .map(|path| vec![path.to_string_lossy().into_owned()])
        .unwrap_or_default();
    let mut definitions = Vec::new();
    let mut dialect = CCompilerDialect::Gnu17;
    let mut cursor = 1;
    while cursor < words.len() {
        let word = &words[cursor];
        if word == "-I" || word == "-isystem" {
            cursor += 1;
            let path = words.get(cursor).ok_or("missing compiler include path")?;
            include_paths.push(resolve_path(&entry.directory, path));
        } else if let Some(path) = word.strip_prefix("-I") {
            include_paths.push(resolve_path(&entry.directory, path));
        } else if word == "-D" {
            cursor += 1;
            let definition = words.get(cursor).ok_or("missing compiler definition")?;
            if !definition.contains('$') {
                definitions.push(definition.clone());
            }
        } else if let Some(definition) = word.strip_prefix("-D") {
            if !definition.contains('$') {
                definitions.push(definition.to_owned());
            }
        } else if let Some(standard) = word.strip_prefix("-std=") {
            dialect = match standard {
                "c11" => CCompilerDialect::C11,
                "gnu99" => CCompilerDialect::Gnu99,
                "gnu11" => CCompilerDialect::Gnu11,
                "gnu17" => CCompilerDialect::Gnu17,
                other => return Err(format!("unsupported compilation dialect `{other}`")),
            };
        }
        cursor += 1;
    }
    Ok(CompilerOptions {
        include_paths,
        definitions,
        dialect,
        preprocessor_options: Vec::new(),
        // The pinned Cycle 19 route keeps its archived standard-input
        // projection, including `<stdin>` file names. It is a declared
        // assumption of those artifacts, not an equivalence claim.
        input: CSourceInput::HistoricalStdin,
    })
}

fn resolve_path(directory: &Path, path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_string_lossy().into_owned()
    } else {
        directory.join(path).to_string_lossy().into_owned()
    }
}

/// Whether two spellings name one existing file: equal, or both resolving to
/// one canonical path. Shared with the member pre-pass's own-input test (card
/// AUDIT-CANONICAL-PATHS), so both find a unit's input word the same way.
pub(crate) fn same_existing_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

pub fn shell_words(command: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut characters = command.chars().peekable();
    while let Some(character) = characters.next() {
        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                } else {
                    word.push(character);
                }
            }
            Some('"') => match character {
                '"' => quote = None,
                '\\' => match characters.peek().copied() {
                    Some(next @ ('$' | '`' | '"' | '\\' | '\n')) => {
                        characters.next();
                        if next != '\n' {
                            word.push(next);
                        }
                    }
                    Some(_) => word.push('\\'),
                    None => return Err("unterminated quoting in compile command".into()),
                },
                '$' | '`' => return Err("shell expansion in compile command is unsupported".into()),
                _ => word.push(character),
            },
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    started = true;
                }
                '\\' => {
                    let next = characters
                        .next()
                        .ok_or("unterminated quoting in compile command")?;
                    if next != '\n' {
                        word.push(next);
                        started = true;
                    }
                }
                '$' | '`' | ';' | '|' | '&' | '<' | '>' | '(' | ')' | '*' | '?' | '[' => {
                    return Err("shell expression in compile command is unsupported".into());
                }
                value if value.is_whitespace() => {
                    if started {
                        words.push(std::mem::take(&mut word));
                        started = false;
                    }
                }
                _ => {
                    word.push(character);
                    started = true;
                }
            },
            _ => unreachable!(),
        }
    }
    if quote.is_some() {
        return Err("unterminated quoting in compile command".into());
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

/// Read actual compilation-database records atomically, rather than aligning
/// independent textual searches for similarly named fields.
pub fn compile_entries(database: &str) -> Result<Vec<CompileEntry>, String> {
    #[derive(serde::Deserialize)]
    struct Record {
        directory: PathBuf,
        file: PathBuf,
        command: Option<String>,
        arguments: Option<Vec<String>>,
    }
    let records: Vec<Record> = serde_json::from_str(database).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for record in records {
        if !record.directory.is_absolute() {
            return Err("compilation directory must be absolute".into());
        }
        let command =
            match (record.command, record.arguments) {
                (Some(command), None) => command,
                (None, Some(arguments)) if !arguments.is_empty() => arguments
                    .into_iter()
                    .map(|word| format!("'{}'", word.replace('\'', "'\\''")))
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => return Err(
                    "each compile record must carry exactly one nonempty command or arguments list"
                        .into(),
                ),
            };
        if shell_words(&command)?.is_empty() {
            return Err("empty compile command".into());
        }
        let file = if record.file.is_absolute() {
            record.file
        } else {
            record.directory.join(record.file)
        };
        entries.push(CompileEntry {
            directory: record.directory,
            file,
            command,
        });
    }
    Ok(entries)
}

// Resolve names without executing the compile-database executable. Only the
// gateway's validated Clang command is probed, by its existing discovery API.
fn executable_path(program: &str, directory: &Path) -> Result<PathBuf, String> {
    let path = Path::new(program);
    let selected = if path.is_absolute() {
        path.to_owned()
    } else if path.components().count() > 1 {
        directory.join(path)
    } else {
        let search = std::env::var_os("PATH").ok_or("PATH is unavailable")?;
        std::env::split_paths(&search)
            .flat_map(|search_directory| {
                let search_directory = if search_directory.is_absolute() {
                    search_directory
                } else {
                    directory.join(search_directory)
                };
                let mut candidates = vec![search_directory.join(path)];
                if cfg!(windows) && path.extension().is_none() {
                    candidates.push(search_directory.join(format!("{program}.exe")));
                }
                candidates
            })
            .find(|path| {
                #[cfg(unix)]
                let executable = {
                    use std::os::unix::fs::PermissionsExt;
                    path.metadata()
                        .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
                };
                #[cfg(not(unix))]
                let executable = true;
                path.is_file() && executable
            })
            .ok_or_else(|| format!("compiler executable `{program}` cannot be resolved"))?
    };
    selected.canonicalize().map_err(|error| error.to_string())
}

/// Check driver identity without executing the program named by the database.
pub fn require_clang_driver(entry: &CompileEntry) -> Result<(), String> {
    let words = shell_words(&entry.command)?;
    let driver = words
        .first()
        .filter(|word| !word.is_empty())
        .ok_or("empty compiler executable")?;
    let name = Path::new(driver)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.contains("++") || name.starts_with("clang-cl") {
        return Err(
            "frontend build flags require the Clang C driver, not a C++ or CL driver".into(),
        );
    }
    let command = qcore_c_gateway::clang_command()?;
    let actual = command
        .get_program()
        .to_str()
        .ok_or("non-UTF-8 Clang executable")?;
    let actual = executable_path(actual, &std::env::current_dir().map_err(|e| e.to_string())?)?;
    if executable_path(driver, &entry.directory)? != actual {
        return Err(format!(
            "audit requires the configured Clang executable ({}); compilation database selects {driver}. Prepare an explicit Clang build with quirin-audit prepare and run quirin-audit current over it (prepare --run chains both); GCC compiler equivalence is unproven",
            actual.display()
        ));
    }
    Ok(())
}

// The shell parser has already rejected active shell expansion. Dollars inside
// one quoted C string are literal macro bytes, not a request to evaluate make
// or shell expressions. Retain the complete string, including doubled dollars.
fn literal_string_definition(value: &str) -> bool {
    let Some((_, replacement)) = value.split_once('=') else {
        return false;
    };
    let Some(interior) = replacement
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
    else {
        return false;
    };
    let mut escaped = false;
    for character in interior.chars() {
        if matches!(character, '\n' | '\r') {
            return false;
        }
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return false;
        }
    }
    !escaped
}

/// The complete table of build options this gateway transports verbatim into
/// preprocessing, syntax and the typed AST.
///
/// This is an allowlist keyed on the exact spelling: an option absent from it
/// is refused by [`compiler_options`], never dropped. Every entry has a
/// recorded preservation argument in `docs/reports/remaining-g03-f/README.md`
/// (measured `-E -P` text and `-dM` macro-set effects). Adding a row means
/// supplying that argument, not widening a default.
fn frontend_flag(word: &str) -> Option<CFrontendFlag> {
    use qcore_c_gateway::legacy_c::COptimizationLevel as Level;
    Some(match word {
        "-Wall" => CFrontendFlag::AllWarnings,
        "-Wextra" => CFrontendFlag::ExtraWarnings,
        "-g" => CFrontendFlag::DebugInformation,
        "-fPIC" => CFrontendFlag::PositionIndependentCode,
        "-Wa,--noexecstack" => CFrontendFlag::AssemblerNoExecStack,
        "-Wstrict-aliasing" => CFrontendFlag::StrictAliasingWarnings,
        "-Wstrict-prototypes" => CFrontendFlag::StrictPrototypes,
        "-Wundef" => CFrontendFlag::UndefinedIdentifierWarnings,
        "-Wmissing-prototypes" => CFrontendFlag::MissingPrototypes,
        "-Winit-self" => CFrontendFlag::InitSelf,
        "-Wmissing-field-initializers" => CFrontendFlag::MissingFieldInitializers,
        "-Wdeclaration-after-statement" => CFrontendFlag::DeclarationAfterStatement,
        "-Waddress" => CFrontendFlag::AddressComparisons,
        "-Woverride-init" => CFrontendFlag::OverrideInit,
        "-Wwrite-strings" => CFrontendFlag::WriteStrings,
        "-Wvla" => CFrontendFlag::VariableLengthArrays,
        "-Wno-unused-parameter" => CFrontendFlag::NoUnusedParameter,
        "-Wno-unused-function" => CFrontendFlag::NoUnusedFunction,
        "-Wno-pragmas" => CFrontendFlag::NoPragmas,
        "-Wno-void-pointer-to-enum-cast" => CFrontendFlag::NoVoidPointerToEnumCast,
        "-fno-strict-aliasing" => CFrontendFlag::NoStrictAliasing,
        "-ffunction-sections" => CFrontendFlag::FunctionSections,
        "-fdata-sections" => CFrontendFlag::DataSections,
        "-fsanitize=address,undefined" => CFrontendFlag::AddressUndefinedSanitizers,
        // A level is preprocessing-visible, so it is named exactly. Bare `-O`,
        // `-Ofast` and every other spelling stay refused rather than being
        // folded onto a neighbouring level.
        "-O0" => CFrontendFlag::Optimization(Level::None),
        "-O1" => CFrontendFlag::Optimization(Level::Level1),
        "-O2" => CFrontendFlag::Optimization(Level::Level2),
        "-O3" => CFrontendFlag::Optimization(Level::Level3),
        "-Os" => CFrontendFlag::Optimization(Level::Size),
        "-Oz" => CFrontendFlag::Optimization(Level::MinimumSize),
        "-Og" => CFrontendFlag::Optimization(Level::Debug),
        _ => return None,
    })
}

/// Keep only the compile records whose declared object output starts with an
/// explicitly configured prefix.
///
/// A multi-target CMake project records one command per (source, target) pair;
/// the commands differ only in `-o`, but the audit must not pick between them
/// on its own — that silent choice is exactly the defect recorded in
/// `docs/reports/remaining-g01/README.md`. This is the explicit alternative:
/// the audit configuration names the target's object directory and everything
/// else is discarded before selection. It narrows the database; it never merges
/// two configurations, so [`select_unique_configuration`] and
/// `select_profile_units` keep refusing any residual ambiguity.
pub fn select_object_prefix(
    entries: Vec<CompileEntry>,
    prefix: &str,
) -> Result<Vec<CompileEntry>, String> {
    if prefix.trim().is_empty() || prefix.starts_with('-') || prefix.contains('$') {
        return Err("compile-target object prefix must be a nonempty plain path fragment".into());
    }
    let mut kept = Vec::new();
    for entry in entries {
        let words = shell_words(&entry.command)?;
        // Only the two-word `-o PATH` form is recognised. A record whose output
        // this cannot read does not match, so it is dropped rather than kept on
        // an assumption about its target.
        let output = words
            .windows(2)
            .find(|pair| pair[0] == "-o")
            .map(|pair| pair[1].clone());
        if output.is_some_and(|output| output.starts_with(prefix)) {
            kept.push(entry);
        }
    }
    if kept.is_empty() {
        return Err(format!(
            "compile-target object prefix `{prefix}` selects no compilation record"
        ));
    }
    Ok(kept)
}

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the explicit target selections
/// of one command, each at most once, admitted together.
#[derive(Default)]
struct TargetSelection {
    identity: qcore_c_gateway::c_frontend_identity::CTargetIdentity,
}

impl TargetSelection {
    fn select_triple(&mut self, triple: &str) -> Result<(), String> {
        if self.identity.triple.is_some() {
            return Err(
                "unsupported compilation target: the command selects a target twice".into(),
            );
        }
        self.identity.triple = Some(qcore_c_gateway::c_frontend_identity::CTargetTriple::parse(
            triple,
        )?);
        Ok(())
    }

    fn select_abi(&mut self, abi: &str) -> Result<(), String> {
        if self.identity.abi.is_some() {
            return Err("unsupported compilation ABI: the command selects an ABI twice".into());
        }
        self.identity.abi = Some(qcore_c_gateway::c_frontend_identity::CAbiName::parse(abi)?);
        Ok(())
    }

    fn select_architecture(&mut self, architecture: &str) -> Result<(), String> {
        // Several `-arch` words keep their existing treatment; each must be
        // an LP64 architecture.
        let mut single = qcore_c_gateway::c_frontend_identity::CTargetIdentity {
            architecture: Some(architecture.to_owned()),
            ..Default::default()
        };
        single.admit()?;
        if self.identity.architecture.is_none() {
            self.identity.architecture = single.architecture.take();
        }
        Ok(())
    }

    /// Admit the selections; the `--target` and `-mabi` flags to transport
    /// (`-arch` is transported where it is read, as before).
    fn admit(self) -> Result<Vec<CFrontendFlag>, String> {
        self.identity.admit()?;
        let mut flags = Vec::new();
        if let Some(triple) = self.identity.triple {
            flags.push(CFrontendFlag::Target(triple));
        }
        if let Some(abi) = self.identity.abi {
            flags.push(CFrontendFlag::Abi(abi));
        }
        Ok(flags)
    }
}

/// The configurable driver accepts only options whose preprocessing effects
/// this gateway can preserve. Existing pinned adapters retain their explicitly
/// bounded historical projection via `compiler_options_compatibility`.
pub fn compiler_options(entry: &CompileEntry) -> Result<CompilerOptions, String> {
    if !entry.directory.is_absolute() || !entry.file.is_absolute() {
        return Err("compile directory and source must be absolute".into());
    }
    let words = shell_words(&entry.command)?;
    if words.first().is_none_or(String::is_empty) {
        return Err("empty compiler executable".into());
    }
    // Clang is given this entry's own input word, run from this entry's own
    // directory, so `__FILE__` and the includer-directory rule for quoted
    // includes answer exactly as they do for the recorded compile command. The
    // gateway no longer approximates the includer directory with `-iquote`:
    // that synthesized path loses to whatever directory a standard-input run
    // happens to start in.
    let mut options = CompilerOptions {
        include_paths: Vec::new(),
        definitions: Vec::new(),
        dialect: CCompilerDialect::Gnu17,
        preprocessor_options: Vec::new(),
        input: CSourceInput::AnonymousText,
    };
    let mut cursor = 1;
    let mut spelled_input = None;
    let mut inputs = 0;
    let mut compile = 0;
    let mut outputs = 0;
    let mut dependency_modes = 0;
    let mut dependency_outputs = 0;
    let mut explicit_dialect = false;
    let mut frontend_flags = false;
    let mut target = TargetSelection::default();
    while cursor < words.len() {
        let word = &words[cursor];
        let split = ["-isystem", "-iquote", "-I", "-D", "-U"]
            .into_iter()
            .find_map(|flag| word.strip_prefix(flag).map(|suffix| (flag, suffix)));
        if let Some((flag, suffix)) = split {
            let value = if suffix.is_empty() {
                cursor += 1;
                words
                    .get(cursor)
                    .ok_or("missing compile option argument")?
                    .as_str()
            } else {
                suffix
            };
            if value.is_empty()
                || (value.contains('$') && !(flag == "-D" && literal_string_definition(value)))
                || (flag == "-I" && value == "-")
            {
                return Err("empty or unexpanded compiler option is unsupported".into());
            }
            if matches!(flag, "-I" | "-isystem" | "-iquote") && value.starts_with('=') {
                return Err("sysroot-relative include paths are unsupported".into());
            }
            let option = match flag {
                "-I" => CPreprocessorOption::Include(resolve_path(&entry.directory, value)),
                "-isystem" => {
                    CPreprocessorOption::SystemInclude(resolve_path(&entry.directory, value))
                }
                "-iquote" => {
                    CPreprocessorOption::QuoteInclude(resolve_path(&entry.directory, value))
                }
                "-D" => CPreprocessorOption::Define(value.into()),
                "-U" => CPreprocessorOption::Undefine(value.into()),
                _ => unreachable!(),
            };
            options.preprocessor_options.push(option);
        } else if let Some(flag) = frontend_flag(word) {
            frontend_flags = true;
            options
                .preprocessor_options
                .push(CPreprocessorOption::FrontendFlag(flag));
        } else if let Some(prefix) = word.strip_prefix("-Wundef-prefix=") {
            if prefix.is_empty() || prefix.contains('$') {
                return Err("invalid -Wundef-prefix argument".into());
            }
            frontend_flags = true;
            options
                .preprocessor_options
                .push(CPreprocessorOption::FrontendFlag(
                    CFrontendFlag::UndefPrefix(prefix.into()),
                ));
        } else if word == "-arch" {
            cursor += 1;
            let architecture = words.get(cursor).ok_or("missing -arch argument")?;
            if architecture.is_empty()
                || architecture.starts_with('-')
                || architecture.contains('$')
            {
                return Err("invalid -arch argument".into());
            }
            frontend_flags = true;
            target.select_architecture(architecture)?;
            options
                .preprocessor_options
                .push(CPreprocessorOption::FrontendFlag(
                    CFrontendFlag::Architecture(architecture.clone()),
                ));
        } else if let Some(standard) = word.strip_prefix("-std=") {
            explicit_dialect = true;
            // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the exact names of
            // the seven dialect identities; aliases stay refused.
            options.dialect = CCompilerDialect::from_std_name(standard)
                .ok_or_else(|| format!("unsupported compilation dialect `{standard}`"))?;
        } else if let Some(triple) = word.strip_prefix("--target=") {
            // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): a structured target.
            target.select_triple(triple)?;
        } else if word == "-target" {
            cursor += 1;
            let triple = words.get(cursor).ok_or("missing -target argument")?;
            target.select_triple(triple)?;
        } else if let Some(abi) = word.strip_prefix("-mabi=") {
            target.select_abi(abi)?;
        } else if word == "-MMD" {
            // Dependency-file emission is an output action, like -o. Keep it
            // in the compile-task identity but do not overwrite build outputs
            // during the gateway's preprocessing and syntax-only phases.
            dependency_modes += 1;
        } else if word == "-MF" {
            cursor += 1;
            let value = words
                .get(cursor)
                .ok_or("missing dependency output argument")?;
            if value.is_empty() || value.starts_with('-') || value.contains('$') {
                return Err("invalid dependency output argument".into());
            }
            dependency_outputs += 1;
        } else if word == "-o" {
            cursor += 1;
            let value = words.get(cursor).ok_or("missing compile output argument")?;
            if value.is_empty() || value.starts_with('-') || value.contains('$') {
                return Err("invalid compile output argument".into());
            }
            outputs += 1;
        } else if word == "-c" {
            compile += 1;
        } else if !word.starts_with('-')
            && same_existing_path(
                Path::new(&resolve_path(&entry.directory, word)),
                &entry.file,
            )
        {
            inputs += 1;
            spelled_input = Some(word.clone());
        } else {
            return Err(format!(
                "compiler option `{word}` cannot be preserved by the bounded gateway"
            ));
        }
        cursor += 1;
    }
    if inputs != 1 || compile != 1 || outputs > 1 {
        return Err(
            "compile command requires one -c, one source input and at most one output".into(),
        );
    }
    if !matches!((dependency_modes, dependency_outputs), (0, 0) | (1, 1)) {
        return Err("dependency outputs require exactly one paired -MMD and -MF".into());
    }
    options.input = CSourceInput::DeclaredFile {
        spelled: spelled_input.ok_or("compile command has no recorded source input")?,
        directory: entry.directory.clone(),
    };
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the explicit selections are
    // admitted together, under the readers' LP64 premise, and transported.
    let target_flags = target.admit()?;
    if !target_flags.is_empty() {
        frontend_flags = true;
        options.preprocessor_options.extend(
            target_flags
                .into_iter()
                .map(CPreprocessorOption::FrontendFlag),
        );
    }
    if frontend_flags {
        if !explicit_dialect {
            return Err("frontend build flags require an explicit supported -std= dialect".into());
        }
        require_clang_driver(entry)?;
    }
    Ok(options)
}
