//! Conservative resource analysis for the Cycle 9 annotated legacy-C subset.
//!
//! The parser intentionally accepts more C than the analyzer models.  Every
//! construct outside the supported structured subset becomes an `Unknown`
//! instruction, so an incomplete model can never produce a `Safe` verdict.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

pub type LegacyNodeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceApiKind {
    Acquire,
    Release,
    Escape,
    /// A proven resource-neutral call.  Cycle 13 uses this only for an
    /// internal function whose own summary is retained in the program result.
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceParameterMode {
    /// The call observes the argument without requiring resource custody.
    Borrowed,
    /// The caller must retain a known live resource across the call.
    Owned { nullable: bool },
    /// Custody moves to the callee, without claiming that the callee destroys it.
    Consumed { nullable: bool },
    /// The trusted call destroys the resource.
    Released { nullable: bool },
    /// The call initializes the pointed-to destination with fresh custody.
    Out { nullable: bool },
    /// The callee stores this resource under the resource passed at
    /// `owner_parameter`. Destruction or transfer of the owner cascades to it.
    Stored {
        owner_parameter: usize,
        nullable: bool,
    },
    /// The callee removes this resource from the aggregate passed at
    /// `owner_parameter`, restoring direct custody to the caller.
    Removed {
        owner_parameter: usize,
        nullable: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceReturnMode {
    #[default]
    None,
    Owned {
        nullable: bool,
    },
    /// On success the result replaces `parameter`; on failure the old resource
    /// remains live. A nullable result therefore requires path refinement
    /// before the replacement can be treated as complete.
    Reallocated {
        parameter: usize,
        nullable: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceFunctionContract {
    pub parameters: Vec<ResourceParameterMode>,
    pub result: ResourceReturnMode,
}

impl ResourceFunctionContract {
    #[must_use]
    pub fn new(
        parameters: impl IntoIterator<Item = ResourceParameterMode>,
        result: ResourceReturnMode,
    ) -> Self {
        Self {
            parameters: parameters.into_iter().collect(),
            result,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceContracts(BTreeMap<String, ResourceFunctionContract>);

impl ResourceContracts {
    #[must_use]
    pub fn new(contracts: impl IntoIterator<Item = (impl Into<String>, ResourceApiKind)>) -> Self {
        Self(
            contracts
                .into_iter()
                .map(|(name, kind)| (name.into(), ResourceFunctionContract::from(kind)))
                .collect(),
        )
    }

    pub fn insert(&mut self, name: impl Into<String>, kind: ResourceApiKind) {
        self.0
            .insert(name.into(), ResourceFunctionContract::from(kind));
    }

    pub fn insert_contract(&mut self, name: impl Into<String>, contract: ResourceFunctionContract) {
        self.0.insert(name.into(), contract);
    }

    fn contract(&self, name: &str) -> Option<&ResourceFunctionContract> {
        self.0.get(name)
    }
}

impl From<ResourceApiKind> for ResourceFunctionContract {
    fn from(kind: ResourceApiKind) -> Self {
        match kind {
            ResourceApiKind::Acquire => {
                Self::new([], ResourceReturnMode::Owned { nullable: false })
            }
            ResourceApiKind::Release => Self::new(
                [ResourceParameterMode::Released { nullable: false }],
                ResourceReturnMode::None,
            ),
            ResourceApiKind::Escape => Self::new(
                [ResourceParameterMode::Consumed { nullable: false }],
                ResourceReturnMode::None,
            ),
            ResourceApiKind::Neutral => Self::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyInstruction {
    Acquire {
        destination: String,
        site: usize,
    },
    Alias {
        destination: String,
        source: String,
    },
    Clear {
        destination: String,
    },
    Release {
        source: String,
    },
    Escape {
        source: String,
    },
    /// Transfer a returned local resource when the expression names one.
    /// Non-resource scalar returns remain resource-neutral.
    EscapeIfOwned {
        source: String,
    },
    ContractCall {
        function: String,
        destination: Option<String>,
        arguments: Vec<Option<String>>,
        contract: ResourceFunctionContract,
        produced_sites: Vec<usize>,
    },
    BranchOnNull {
        source: String,
        true_is_null: bool,
    },
    NoOp,
    Unknown {
        description: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCfgNode {
    pub instruction: LegacyInstruction,
    pub successors: Vec<LegacyNodeId>,
    pub line: usize,
    pub is_exit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyCfg {
    pub function: String,
    pub entry: LegacyNodeId,
    pub nodes: Vec<LegacyCfgNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisStatus {
    Safe,
    Unsafe,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegacyDiagnosticKind {
    Leak { acquisition_line: usize },
    DoubleRelease { acquisition_line: usize },
    ReleaseAfterEscape { acquisition_line: usize },
    CustodyConflict { acquisition_line: usize },
    UnknownAlias { variable: String },
    Unsupported { description: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegacyDiagnostic {
    pub function: String,
    pub line: usize,
    pub kind: LegacyDiagnosticKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyFunctionSummary {
    pub function: String,
    pub status: AnalysisStatus,
    pub diagnostics: Vec<LegacyDiagnostic>,
    /// Number of work-list transfers used to reach the CFG fixed point.
    pub fixed_point_steps: usize,
    pub cfg: LegacyCfg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyProgramSummary {
    pub entry: String,
    pub status: AnalysisStatus,
    pub functions: BTreeMap<String, LegacyFunctionSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyCError {
    FunctionNotFound(String),
    MalformedFunction { function: String, line: usize },
    ProductionParserUnavailable(String),
    ProductionParserRejected(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct C11TranslationUnit<'a> {
    pub source: &'a str,
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    pub dialect: CCompilerDialect,
    pub function_scope: LegacyFunctionScope,
}

/// Explicitly gated C dialects accepted by the production-parser gateway.
/// Arbitrary compiler arguments are intentionally not accepted here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CCompilerDialect {
    #[default]
    C11,
    Gnu99,
    Gnu11,
    Gnu17,
}

impl CCompilerDialect {
    fn clang_argument(self) -> &'static str {
        match self {
            Self::C11 => "-std=c11",
            Self::Gnu99 => "-std=gnu99",
            Self::Gnu11 => "-std=gnu11",
            Self::Gnu17 => "-std=gnu17",
        }
    }
}

/// Selects whether header-expanded definitions participate in the proof.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegacyFunctionScope {
    /// Preserve the Cycle 17 behavior and analyze every preprocessed body.
    #[default]
    PreprocessedTranslationUnit,
    /// Analyze only functions whose definitions occur in the input source.
    MainSourceDefinitions,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacySafetyCertificate {
    pub preprocessed_source: String,
    pub entry: String,
    pub contracts: ResourceContracts,
    pub dialect: CCompilerDialect,
    pub function_scope: LegacyFunctionScope,
    pub summary: LegacyProgramSummary,
    pub checked_functions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnrestrictedLegacyResult {
    Safe(LegacySafetyCertificate),
    Unsafe(LegacyProgramSummary),
    Unknown {
        preprocessed_source: String,
        reason: String,
        partial: Option<LegacyProgramSummary>,
    },
}

impl fmt::Display for LegacyCError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FunctionNotFound(function) => write!(f, "C function `{function}` was not found"),
            Self::MalformedFunction { function, line } => {
                write!(
                    f,
                    "malformed body for C function `{function}` at line {line}"
                )
            }
            Self::ProductionParserUnavailable(message) => {
                write!(f, "production C parser is unavailable: {message}")
            }
            Self::ProductionParserRejected(message) => {
                write!(
                    f,
                    "production C parser rejected the translation unit: {message}"
                )
            }
        }
    }
}

impl std::error::Error for LegacyCError {}

/// Parse, import to a resource CFG, convert assignments to SSA-like strong
/// updates, and solve the finite may-alias/resource-state equations.
pub fn analyze_legacy_c(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let tokens = tokenize(source);
    analyze_legacy_c_tokens(&tokens, function, contracts)
}

fn analyze_legacy_c_tokens(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let (body_start, line) = find_function_body(tokens, function)
        .ok_or_else(|| LegacyCError::FunctionNotFound(function.to_owned()))?;
    let mut parser = Parser {
        tokens,
        cursor: body_start,
        contracts,
        next_site: 0,
    };
    let statements =
        parser
            .parse_block()
            .map_err(|failure_line| LegacyCError::MalformedFunction {
                function: function.to_owned(),
                line: failure_line.max(line),
            })?;
    let cfg = CfgBuilder::build(function, &statements);
    Ok(analyze_cfg(cfg))
}

/// Gate the conservative importer with Clang's production C11 parser.  The
/// typed AST accepted by Clang is still deliberately projected into the small
/// resource language below; any projection gap remains `Unknown`.
pub fn analyze_legacy_c11(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let mut child = Command::new("clang")
        .args(["-x", "c", "-std=c11", "-fsyntax-only", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
        .write_all(source.as_bytes())
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    let output = child
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    analyze_legacy_c(source, function, contracts)
}

/// Preprocess a conforming C11 translation unit with Clang, then project it
/// through the conservative resource analyzer.  Projection gaps are an
/// `Unknown` verdict, not a parse failure and never a `Safe` certificate.
pub fn analyze_unrestricted_c11(
    unit: &C11TranslationUnit<'_>,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let source_functions = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(function_names(&tokenize(unit.source))),
    };
    let preprocessed_source = validate_unrestricted_c11(unit)?;
    let analysis = match analyze_legacy_c_translation_unit_selected_partial(
        &preprocessed_source,
        entry,
        contracts,
        source_functions.as_deref(),
    ) {
        Ok(summary) => summary,
        Err(error) => {
            return Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source,
                reason: error.to_string(),
                partial: None,
            });
        }
    };
    let summary = analysis.summary;
    if !analysis.projection_failures.is_empty() {
        let failures = analysis
            .projection_failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason: format!("some source functions could not be projected: {failures}"),
            partial: Some(summary),
        });
    }
    match summary.status {
        AnalysisStatus::Unsafe => Ok(UnrestrictedLegacyResult::Unsafe(summary)),
        AnalysisStatus::Unknown => Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason: "the conservative C11 projection contains an unsupported operation".into(),
            partial: Some(summary),
        }),
        AnalysisStatus::Safe => {
            let checked_functions = summary.functions.keys().cloned().collect::<Vec<_>>();
            let certificate = LegacySafetyCertificate {
                preprocessed_source,
                entry: summary.entry.clone(),
                contracts: contracts.clone(),
                dialect: unit.dialect,
                function_scope: unit.function_scope,
                summary,
                checked_functions,
            };
            check_legacy_safety_certificate(&certificate).map_err(|reason| {
                LegacyCError::ProductionParserRejected(format!(
                    "internal legacy-C certificate check failed: {reason}"
                ))
            })?;
            Ok(UnrestrictedLegacyResult::Safe(certificate))
        }
    }
}

/// Preprocess and syntax-check an unrestricted C translation unit without
/// requiring it to contain an executable function body.
pub fn validate_unrestricted_c11(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    let mut command = Command::new("clang");
    command.args(["-x", "c", unit.dialect.clang_argument(), "-E", "-P"]);
    for include in &unit.include_paths {
        command.arg(format!("-I{include}"));
    }
    for definition in &unit.definitions {
        command.arg(format!("-D{definition}"));
    }
    command.arg("-");
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
        .write_all(unit.source.as_bytes())
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    let output = child
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let preprocessed_source = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut syntax = Command::new("clang")
        .args([
            "-x",
            "c",
            unit.dialect.clang_argument(),
            "-fsyntax-only",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    syntax
        .stdin
        .take()
        .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing syntax stdin".into()))?
        .write_all(preprocessed_source.as_bytes())
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    let syntax_output = syntax
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !syntax_output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&syntax_output.stderr)
                .trim()
                .to_owned(),
        ));
    }
    Ok(preprocessed_source)
}

/// Independent structural check for an unrestricted-C `Safe` result.
pub fn check_legacy_safety_certificate(
    certificate: &LegacySafetyCertificate,
) -> Result<(), String> {
    let selected = match certificate.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => {
            Some(certificate.checked_functions.as_slice())
        }
    };
    let recomputed = analyze_legacy_c_translation_unit_selected(
        &certificate.preprocessed_source,
        &certificate.entry,
        &certificate.contracts,
        selected,
    )
    .map_err(|error| format!("certificate source cannot be reanalyzed: {error}"))?;
    if recomputed != certificate.summary {
        return Err("program summary does not match the preprocessed source".into());
    }
    if certificate.summary.status != AnalysisStatus::Safe {
        return Err("program summary is not Safe".into());
    }
    let names = certificate
        .summary
        .functions
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    if certificate.checked_functions != names {
        return Err("checked function index is not canonical".into());
    }
    for (name, function) in &certificate.summary.functions {
        if function.status != AnalysisStatus::Safe || !function.diagnostics.is_empty() {
            return Err(format!("function `{name}` is not independently Safe"));
        }
        if function
            .cfg
            .nodes
            .iter()
            .any(|node| matches!(node.instruction, LegacyInstruction::Unknown { .. }))
        {
            return Err(format!("function `{name}` retains an unknown instruction"));
        }
    }
    Ok(())
}

/// Analyze every function definition in a translation unit and combine the
/// per-function results into a bounded interprocedural status summary.
/// Internal calls are resource-neutral at the call site, while the callee's
/// own resource behavior is checked separately and contributes to the result.
pub fn analyze_legacy_c_translation_unit(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyProgramSummary, LegacyCError> {
    analyze_legacy_c_translation_unit_selected(source, entry, contracts, None)
}

fn analyze_legacy_c_translation_unit_selected(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
) -> Result<LegacyProgramSummary, LegacyCError> {
    let analysis = analyze_legacy_c_translation_unit_selected_partial(
        source,
        entry,
        contracts,
        selected_functions,
    )?;
    if let Some(error) = analysis.projection_failures.into_iter().next() {
        return Err(error);
    }
    Ok(analysis.summary)
}

struct PartialTranslationUnitAnalysis {
    summary: LegacyProgramSummary,
    projection_failures: Vec<LegacyCError>,
}

fn analyze_legacy_c_translation_unit_selected_partial(
    source: &str,
    entry: &str,
    contracts: &ResourceContracts,
    selected_functions: Option<&[String]>,
) -> Result<PartialTranslationUnitAnalysis, LegacyCError> {
    let tokens = tokenize(source);
    let mut names = function_names(&tokens);
    let requested_was_selected =
        selected_functions.is_some_and(|selected| selected.iter().any(|name| name == entry));
    if let Some(selected) = selected_functions {
        let selected = selected.iter().collect::<BTreeSet<_>>();
        names.retain(|name| selected.contains(name));
    }
    let effective_entry = if names.iter().any(|name| name == entry) {
        entry.to_owned()
    } else if requested_was_selected {
        names
            .first()
            .cloned()
            .ok_or_else(|| LegacyCError::FunctionNotFound(entry.to_owned()))?
    } else {
        return Err(LegacyCError::FunctionNotFound(entry.to_owned()));
    };
    let mut extended = contracts.clone();
    for name in &names {
        extended.insert(name, ResourceApiKind::Neutral);
    }
    let mut functions = BTreeMap::new();
    let mut projection_failures = Vec::new();
    for name in names {
        match analyze_legacy_c_tokens(&tokens, &name, &extended) {
            Ok(summary) => {
                functions.insert(name, summary);
            }
            Err(error @ LegacyCError::MalformedFunction { .. }) => {
                projection_failures.push(error);
            }
            Err(error) => return Err(error),
        }
    }
    let status = if functions
        .values()
        .any(|summary| summary.status == AnalysisStatus::Unsafe)
    {
        AnalysisStatus::Unsafe
    } else if !projection_failures.is_empty()
        || functions
            .values()
            .any(|summary| summary.status == AnalysisStatus::Unknown)
    {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    };
    Ok(PartialTranslationUnitAnalysis {
        summary: LegacyProgramSummary {
            entry: effective_entry,
            status,
            functions,
        },
        projection_failures,
    })
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    line: usize,
}

fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'\n' {
            line += 1;
            cursor += 1;
        } else if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                if bytes[cursor] == b'\n' {
                    line += 1;
                }
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'"' || byte == b'\'' {
            let quote = byte;
            let token_line = line;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else if bytes[cursor] == quote {
                    cursor += 1;
                    break;
                } else {
                    if bytes[cursor] == b'\n' {
                        line += 1;
                    }
                    cursor += 1;
                }
            }
            tokens.push(Token {
                text: "__qcore_literal__".to_owned(),
                line: token_line,
            });
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else if byte.is_ascii_digit() {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_alphanumeric() {
                cursor += 1;
            }
            tokens.push(Token {
                text: source[start..cursor].to_owned(),
                line,
            });
        } else if byte == b'#' {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else {
            tokens.push(Token {
                text: char::from(byte).to_string(),
                line,
            });
            cursor += 1;
        }
    }
    tokens
}

fn find_function_body(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) == Some("{") {
            return Some((close + 1, token.line));
        }
    }
    None
}

fn function_names(tokens: &[Token]) -> Vec<String> {
    let mut names = BTreeSet::new();
    let mut brace_depth = 0usize;
    let mut linkage_closes = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.text == "{" {
            // Raw C sources commonly retain an `extern "C"` wrapper behind
            // `#ifdef __cplusplus`; it is a linkage boundary, not a function.
            if index >= 2
                && tokens[index - 2].text == "extern"
                && tokens[index - 1].text == "__qcore_literal__"
                && let Some(close) = matching(tokens, index, "{", "}")
            {
                linkage_closes.insert(close);
                continue;
            }
            brace_depth += 1;
            continue;
        }
        if token.text == "}" {
            if linkage_closes.remove(&index) {
                continue;
            }
            brace_depth = brace_depth.saturating_sub(1);
            continue;
        }
        if brace_depth != 0 {
            continue;
        }
        if !is_identifier(&token.text)
            || matches!(token.text.as_str(), "if" | "while" | "for" | "switch")
            || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
        {
            continue;
        }
        if let Some(close) = matching(tokens, index + 1, "(", ")")
            && tokens.get(close + 1).map(|next| next.text.as_str()) == Some("{")
        {
            names.insert(token.text.clone());
        }
    }
    names.into_iter().collect()
}

/// Return the function definitions written in a source file before headers
/// and macros are expanded. This is the auditable scope used by
/// [`LegacyFunctionScope::MainSourceDefinitions`].
#[must_use]
pub fn legacy_c_source_functions(source: &str) -> Vec<String> {
    function_names(&tokenize(source))
}

fn matching(tokens: &[Token], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
enum Statement {
    Instruction(LegacyInstruction, usize),
    Block(Vec<Statement>),
    If {
        line: usize,
        condition_supported: bool,
        null_condition: Option<PointerNullCondition>,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    While {
        line: usize,
        condition_supported: bool,
        null_condition: Option<PointerNullCondition>,
        body: Vec<Statement>,
    },
    For {
        line: usize,
        init: Option<Box<(LegacyInstruction, usize)>>,
        condition_always_true: bool,
        condition_supported: bool,
        null_condition: Option<PointerNullCondition>,
        increment: Option<Box<(LegacyInstruction, usize)>>,
        body: Vec<Statement>,
    },
    DoWhile {
        line: usize,
        condition_supported: bool,
        null_condition: Option<PointerNullCondition>,
        body: Vec<Statement>,
    },
    Switch {
        line: usize,
        expression_supported: bool,
        body: Vec<Statement>,
    },
    Case {
        line: usize,
        expression_supported: bool,
        is_default: bool,
    },
    Label {
        line: usize,
        name: String,
    },
    Goto {
        line: usize,
        label: String,
    },
    Break {
        line: usize,
    },
    Continue {
        line: usize,
    },
    Return {
        line: usize,
        expression_supported: bool,
        returned_place: Option<String>,
    },
}

#[derive(Clone, Debug)]
struct PointerNullCondition {
    source: String,
    true_is_null: bool,
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    contracts: &'a ResourceContracts,
    next_site: usize,
}

impl Parser<'_> {
    fn parse_block(&mut self) -> Result<Vec<Statement>, usize> {
        let opening_line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        if self.text() != Some("{") {
            return Err(opening_line);
        }
        self.cursor += 1;
        let mut statements = Vec::new();
        while self.cursor < self.tokens.len() && self.text() != Some("}") {
            statements.push(self.parse_statement()?);
        }
        if self.text() != Some("}") {
            return Err(self.tokens.last().map_or(opening_line, |token| token.line));
        }
        self.cursor += 1;
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, usize> {
        let line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        match self.text() {
            Some("if") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let then_body = self.parse_statement_body()?;
                let else_body = if self.text() == Some("else") {
                    self.cursor += 1;
                    self.parse_statement_body()?
                } else {
                    Vec::new()
                };
                Ok(Statement::If {
                    line,
                    condition_supported: !contains_call(&condition),
                    null_condition: pointer_null_condition(&condition),
                    then_body,
                    else_body,
                })
            }
            Some("while") => {
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                let body = self.parse_statement_body()?;
                Ok(Statement::While {
                    line,
                    condition_supported: !contains_call(&condition),
                    null_condition: pointer_null_condition(&condition),
                    body,
                })
            }
            Some("for") => {
                self.cursor += 1;
                let clauses = self.take_parenthesized()?;
                let Some([init, condition, increment]) = split_for_clauses(&clauses) else {
                    return Ok(Statement::Instruction(
                        LegacyInstruction::Unknown {
                            description: "malformed for-loop clauses".to_owned(),
                        },
                        line,
                    ));
                };
                let body = self.parse_statement_body()?;
                let init =
                    (!init.is_empty()).then(|| Box::new((self.classify_simple(&init), line)));
                let increment = (!increment.is_empty())
                    .then(|| Box::new((self.classify_simple(&increment), line)));
                Ok(Statement::For {
                    line,
                    init,
                    condition_always_true: condition.is_empty(),
                    condition_supported: !contains_call(&condition),
                    null_condition: pointer_null_condition(&condition),
                    increment,
                    body,
                })
            }
            Some("do") => {
                self.cursor += 1;
                let body = self.parse_statement_body()?;
                if self.text() != Some("while") {
                    return Err(line);
                }
                self.cursor += 1;
                let condition = self.take_parenthesized()?;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::DoWhile {
                    line,
                    condition_supported: !contains_call(&condition),
                    null_condition: pointer_null_condition(&condition),
                    body,
                })
            }
            Some("switch") => {
                self.cursor += 1;
                let expression = self.take_parenthesized()?;
                let body = self.parse_statement_body()?;
                Ok(Statement::Switch {
                    line,
                    expression_supported: !contains_call(&expression),
                    body,
                })
            }
            Some("case") => {
                self.cursor += 1;
                let expression = self.consume_to_colon()?;
                Ok(Statement::Case {
                    line,
                    expression_supported: !contains_call(&expression),
                    is_default: false,
                })
            }
            Some("default") => {
                self.cursor += 1;
                if self.text() != Some(":") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Case {
                    line,
                    expression_supported: true,
                    is_default: true,
                })
            }
            Some("goto") => {
                self.cursor += 1;
                let Some(label) = self.text().filter(|text| is_identifier(text)) else {
                    return Err(line);
                };
                let label = label.to_owned();
                self.cursor += 1;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Goto { line, label })
            }
            Some("break") => {
                self.cursor += 1;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Break { line })
            }
            Some("continue") => {
                self.cursor += 1;
                if self.text() != Some(";") {
                    return Err(line);
                }
                self.cursor += 1;
                Ok(Statement::Continue { line })
            }
            Some("return") => {
                self.cursor += 1;
                let expression = self.consume_to_semicolon()?;
                let texts = expression
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<Vec<_>>();
                Ok(Statement::Return {
                    line,
                    expression_supported: !contains_call(&expression),
                    returned_place: canonical_place(&texts),
                })
            }
            Some("{") => Ok(Statement::Block(self.parse_block()?)),
            Some(name)
                if is_identifier(name)
                    && self
                        .tokens
                        .get(self.cursor + 1)
                        .map(|token| token.text.as_str())
                        == Some(":") =>
            {
                let name = name.to_owned();
                self.cursor += 2;
                Ok(Statement::Label { line, name })
            }
            Some(_) => {
                let tokens = self.consume_to_semicolon()?;
                Ok(Statement::Instruction(self.classify_simple(&tokens), line))
            }
            None => Err(line),
        }
    }

    fn parse_statement_body(&mut self) -> Result<Vec<Statement>, usize> {
        if self.text() == Some("{") {
            self.parse_block()
        } else {
            Ok(vec![self.parse_statement()?])
        }
    }

    fn take_parenthesized(&mut self) -> Result<Vec<Token>, usize> {
        let line = self.tokens.get(self.cursor).map_or(1, |token| token.line);
        if self.text() != Some("(") {
            return Err(line);
        }
        let close = matching(self.tokens, self.cursor, "(", ")").ok_or(line)?;
        let result = self.tokens[self.cursor + 1..close].to_vec();
        self.cursor = close + 1;
        Ok(result)
    }

    fn consume_to_semicolon(&mut self) -> Result<Vec<Token>, usize> {
        let start = self.cursor;
        let line = self.tokens.get(start).map_or(1, |token| token.line);
        let mut parentheses = 0usize;
        while self.cursor < self.tokens.len() {
            match self.text() {
                Some("(") => parentheses += 1,
                Some(")") => parentheses = parentheses.saturating_sub(1),
                Some(";") if parentheses == 0 => {
                    let result = self.tokens[start..self.cursor].to_vec();
                    self.cursor += 1;
                    return Ok(result);
                }
                Some("{") | Some("}") if parentheses == 0 => return Err(line),
                _ => {}
            }
            self.cursor += 1;
        }
        Err(line)
    }

    fn consume_to_colon(&mut self) -> Result<Vec<Token>, usize> {
        let start = self.cursor;
        let line = self.tokens.get(start).map_or(1, |token| token.line);
        let mut parentheses = 0usize;
        while self.cursor < self.tokens.len() {
            match self.text() {
                Some("(") => parentheses += 1,
                Some(")") => parentheses = parentheses.saturating_sub(1),
                Some(":") if parentheses == 0 => {
                    let result = self.tokens[start..self.cursor].to_vec();
                    self.cursor += 1;
                    return Ok(result);
                }
                Some("{") | Some("}") | Some(";") if parentheses == 0 => return Err(line),
                _ => {}
            }
            self.cursor += 1;
        }
        Err(line)
    }

    fn classify_simple(&mut self, tokens: &[Token]) -> LegacyInstruction {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let assignment = texts.iter().position(|text| *text == "=");
        let call = find_call(&texts);

        if let Some((call_index, function)) = call {
            if let Some(contract) = self.contracts.contract(function).cloned() {
                let destination =
                    assignment.and_then(|equal| direct_assignment_destination(&texts[..equal]));
                if !matches!(contract.result, ResourceReturnMode::None) && destination.is_none() {
                    return LegacyInstruction::Unknown {
                        description: format!("discarded owned result of resource API `{function}`"),
                    };
                }
                let raw_arguments = call_arguments(&texts, call_index);
                let arguments = contract
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, mode)| {
                        raw_arguments
                            .as_ref()
                            .and_then(|arguments| arguments.get(index))
                            .and_then(|argument| contract_argument_place(argument, *mode))
                    })
                    .collect::<Vec<_>>();
                let produced_count =
                    usize::from(!matches!(contract.result, ResourceReturnMode::None))
                        + contract
                            .parameters
                            .iter()
                            .filter(|mode| matches!(mode, ResourceParameterMode::Out { .. }))
                            .count();
                let produced_sites = (0..produced_count)
                    .map(|_| {
                        let site = self.next_site;
                        self.next_site += 1;
                        site
                    })
                    .collect();
                return LegacyInstruction::ContractCall {
                    function: function.to_owned(),
                    destination,
                    arguments,
                    contract,
                    produced_sites,
                };
            }
            return LegacyInstruction::Unknown {
                description: format!("call to uncontracted function `{function}`"),
            };
        }

        if let Some(equal) = assignment {
            let Some(destination) = direct_assignment_destination(&texts[..equal]) else {
                return LegacyInstruction::Unknown {
                    description: "unrecognized assignment destination".to_owned(),
                };
            };
            let right = &texts[equal + 1..];
            if right.len() == 1 && matches!(right[0], "NULL" | "nullptr" | "0") {
                return LegacyInstruction::Clear {
                    destination: destination.to_owned(),
                };
            }
            if let Some(source) = canonical_place(right) {
                return LegacyInstruction::Alias {
                    destination: destination.to_owned(),
                    source,
                };
            }
            return LegacyInstruction::Unknown {
                description: "unsupported assignment expression".to_owned(),
            };
        }

        LegacyInstruction::NoOp
    }

    fn text(&self) -> Option<&str> {
        self.tokens
            .get(self.cursor)
            .map(|token| token.text.as_str())
    }
}

fn find_call<'a>(tokens: &'a [&'a str]) -> Option<(usize, &'a str)> {
    tokens.windows(2).enumerate().find_map(|(index, pair)| {
        (is_identifier(pair[0]) && pair[1] == "(").then_some((index, pair[0]))
    })
}

fn contains_call(tokens: &[Token]) -> bool {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    find_call(&texts).is_some()
}

fn split_for_clauses(tokens: &[Token]) -> Option<[Vec<Token>; 3]> {
    let mut clauses = vec![Vec::new()];
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for token in tokens {
        match token.text.as_str() {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            ";" if parentheses == 0 && brackets == 0 => clauses.push(Vec::new()),
            _ => clauses.last_mut()?.push(token.clone()),
        }
    }
    clauses.try_into().ok()
}

fn pointer_null_condition(tokens: &[Token]) -> Option<PointerNullCondition> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let is_null = |text: &str| matches!(text, "NULL" | "nullptr" | "0");
    match texts.as_slice() {
        [source] if is_identifier(source) => Some(PointerNullCondition {
            source: (*source).to_owned(),
            true_is_null: false,
        }),
        ["!", source] if is_identifier(source) => Some(PointerNullCondition {
            source: (*source).to_owned(),
            true_is_null: true,
        }),
        [source, "=", "=", null] if is_identifier(source) && is_null(null) => {
            Some(PointerNullCondition {
                source: (*source).to_owned(),
                true_is_null: true,
            })
        }
        [source, "!", "=", null] if is_identifier(source) && is_null(null) => {
            Some(PointerNullCondition {
                source: (*source).to_owned(),
                true_is_null: false,
            })
        }
        [null, "=", "=", source] if is_null(null) && is_identifier(source) => {
            Some(PointerNullCondition {
                source: (*source).to_owned(),
                true_is_null: true,
            })
        }
        [null, "!", "=", source] if is_null(null) && is_identifier(source) => {
            Some(PointerNullCondition {
                source: (*source).to_owned(),
                true_is_null: false,
            })
        }
        _ => None,
    }
}

fn call_arguments<'a>(tokens: &'a [&'a str], call_index: usize) -> Option<Vec<&'a [&'a str]>> {
    let mut arguments = Vec::new();
    let mut start = call_index + 2;
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut cursor = start;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "(" => parentheses += 1,
            ")" if parentheses == 0 && brackets == 0 => {
                if cursor > start {
                    arguments.push(&tokens[start..cursor]);
                }
                return Some(arguments);
            }
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            "," if parentheses == 0 && brackets == 0 => {
                arguments.push(&tokens[start..cursor]);
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn contract_argument_place(argument: &[&str], mode: ResourceParameterMode) -> Option<String> {
    match mode {
        ResourceParameterMode::Borrowed => None,
        ResourceParameterMode::Out { .. } => {
            (argument.first() == Some(&"&")).then(|| canonical_place(&argument[1..]))?
        }
        ResourceParameterMode::Owned { .. }
        | ResourceParameterMode::Consumed { .. }
        | ResourceParameterMode::Released { .. }
        | ResourceParameterMode::Stored { .. }
        | ResourceParameterMode::Removed { .. } => canonical_place(argument),
    }
}

fn direct_assignment_destination(tokens: &[&str]) -> Option<String> {
    if tokens.first() == Some(&"*") {
        return None;
    }
    (0..tokens.len()).find_map(|start| canonical_place(&tokens[start..]))
}

fn canonical_place(tokens: &[&str]) -> Option<String> {
    let first = tokens.first().copied()?;
    if !is_identifier(first) || matches!(first, "NULL" | "nullptr") {
        return None;
    }
    let mut place = first.to_owned();
    let mut cursor = 1;
    while cursor < tokens.len() {
        match tokens[cursor] {
            "." if tokens
                .get(cursor + 1)
                .is_some_and(|field| is_identifier(field)) =>
            {
                place.push('.');
                place.push_str(tokens[cursor + 1]);
                cursor += 2;
            }
            "-" if tokens.get(cursor + 1) == Some(&">")
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|field| is_identifier(field)) =>
            {
                place.push_str("->");
                place.push_str(tokens[cursor + 2]);
                cursor += 3;
            }
            "[" if tokens
                .get(cursor + 1)
                .is_some_and(|index| index.bytes().all(|byte| byte.is_ascii_digit()))
                && tokens.get(cursor + 2) == Some(&"]") =>
            {
                place.push('[');
                place.push_str(tokens[cursor + 1]);
                place.push(']');
                cursor += 3;
            }
            _ => return None,
        }
    }
    Some(place)
}

fn is_identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
}

struct CfgBuilder {
    nodes: Vec<LegacyCfgNode>,
    labels: BTreeMap<String, LegacyNodeId>,
    pending_gotos: Vec<(LegacyNodeId, String, LegacyNodeId)>,
    break_targets: Vec<LegacyNodeId>,
    continue_targets: Vec<LegacyNodeId>,
    switch_cases: Vec<SwitchTargets>,
}

#[derive(Default)]
struct SwitchTargets {
    entries: Vec<LegacyNodeId>,
    has_default: bool,
}

impl CfgBuilder {
    fn build(function: &str, statements: &[Statement]) -> LegacyCfg {
        let mut builder = Self {
            nodes: Vec::new(),
            labels: BTreeMap::new(),
            pending_gotos: Vec::new(),
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            switch_cases: Vec::new(),
        };
        let exit = builder.push(LegacyInstruction::NoOp, Vec::new(), 0, true);
        let entry = builder.compile_sequence(statements, exit);
        for (node, label, fallback) in std::mem::take(&mut builder.pending_gotos) {
            if let Some(target) = builder.labels.get(&label).copied() {
                builder.nodes[node].successors = vec![target];
            } else {
                builder.nodes[node].instruction = LegacyInstruction::Unknown {
                    description: format!("goto to unresolved label `{label}`"),
                };
                builder.nodes[node].successors = vec![fallback];
            }
        }
        LegacyCfg {
            function: function.to_owned(),
            entry,
            nodes: builder.nodes,
        }
    }

    fn compile_sequence(
        &mut self,
        statements: &[Statement],
        continuation: LegacyNodeId,
    ) -> LegacyNodeId {
        statements
            .iter()
            .rev()
            .fold(continuation, |next, statement| {
                self.compile(statement, next)
            })
    }

    fn compile(&mut self, statement: &Statement, continuation: LegacyNodeId) -> LegacyNodeId {
        match statement {
            Statement::Instruction(instruction, line) => {
                self.push(instruction.clone(), vec![continuation], *line, false)
            }
            Statement::Block(statements) => self.compile_sequence(statements, continuation),
            Statement::Return {
                line,
                expression_supported,
                returned_place,
            } => {
                let instruction = if !*expression_supported {
                    LegacyInstruction::Unknown {
                        description: "call in return expression".to_owned(),
                    }
                } else if let Some(source) = returned_place {
                    LegacyInstruction::EscapeIfOwned {
                        source: source.clone(),
                    }
                } else {
                    LegacyInstruction::NoOp
                };
                self.push(instruction, Vec::new(), *line, true)
            }
            Statement::If {
                line,
                condition_supported,
                null_condition,
                then_body,
                else_body,
            } => {
                let then_entry = self.compile_sequence(then_body, continuation);
                let else_entry = self.compile_sequence(else_body, continuation);
                let instruction = if let Some(condition) = null_condition {
                    LegacyInstruction::BranchOnNull {
                        source: condition.source.clone(),
                        true_is_null: condition.true_is_null,
                    }
                } else if *condition_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in branch condition".to_owned(),
                    }
                };
                self.push(instruction, vec![then_entry, else_entry], *line, false)
            }
            Statement::While {
                line,
                condition_supported,
                null_condition,
                body,
            } => {
                let instruction = if let Some(condition) = null_condition {
                    LegacyInstruction::BranchOnNull {
                        source: condition.source.clone(),
                        true_is_null: condition.true_is_null,
                    }
                } else if *condition_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in loop condition".to_owned(),
                    }
                };
                let header = self.push(instruction, Vec::new(), *line, false);
                self.break_targets.push(continuation);
                self.continue_targets.push(header);
                let body_entry = self.compile_sequence(body, header);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = vec![body_entry, continuation];
                header
            }
            Statement::For {
                line,
                init,
                condition_always_true,
                condition_supported,
                null_condition,
                increment,
                body,
            } => {
                let instruction = branch_instruction(
                    *condition_supported,
                    null_condition,
                    "call in for-loop condition",
                );
                let header = self.push(instruction, Vec::new(), *line, false);
                let increment_entry = increment.as_ref().map_or(header, |clause| {
                    let (instruction, line) = clause.as_ref();
                    self.push(instruction.clone(), vec![header], *line, false)
                });
                self.break_targets.push(continuation);
                self.continue_targets.push(increment_entry);
                let body_entry = self.compile_sequence(body, increment_entry);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = if *condition_always_true {
                    vec![body_entry]
                } else {
                    vec![body_entry, continuation]
                };
                init.as_ref().map_or(header, |clause| {
                    let (instruction, line) = clause.as_ref();
                    self.push(instruction.clone(), vec![header], *line, false)
                })
            }
            Statement::DoWhile {
                line,
                condition_supported,
                null_condition,
                body,
            } => {
                let instruction = branch_instruction(
                    *condition_supported,
                    null_condition,
                    "call in do-while condition",
                );
                let header = self.push(instruction, Vec::new(), *line, false);
                self.break_targets.push(continuation);
                self.continue_targets.push(header);
                let body_entry = self.compile_sequence(body, header);
                self.continue_targets.pop();
                self.break_targets.pop();
                self.nodes[header].successors = vec![body_entry, continuation];
                body_entry
            }
            Statement::Switch {
                line,
                expression_supported,
                body,
            } => {
                self.break_targets.push(continuation);
                self.switch_cases.push(SwitchTargets::default());
                let _body_entry = self.compile_sequence(body, continuation);
                let targets = self.switch_cases.pop().expect("switch target scope");
                self.break_targets.pop();
                let mut successors = targets.entries;
                if !targets.has_default || successors.is_empty() {
                    successors.push(continuation);
                }
                successors.sort_unstable();
                successors.dedup();
                let instruction = if *expression_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in switch expression".to_owned(),
                    }
                };
                self.push(instruction, successors, *line, false)
            }
            Statement::Case {
                line,
                expression_supported,
                is_default,
            } => {
                let instruction = if *expression_supported {
                    LegacyInstruction::NoOp
                } else {
                    LegacyInstruction::Unknown {
                        description: "call in case expression".to_owned(),
                    }
                };
                let entry = self.push(instruction, vec![continuation], *line, false);
                if let Some(targets) = self.switch_cases.last_mut() {
                    targets.entries.push(entry);
                    targets.has_default |= *is_default;
                } else {
                    self.nodes[entry].instruction = LegacyInstruction::Unknown {
                        description: "case label outside switch".to_owned(),
                    };
                }
                entry
            }
            Statement::Label { line, name } => {
                let entry = self.push(LegacyInstruction::NoOp, vec![continuation], *line, false);
                if self.labels.insert(name.clone(), entry).is_some() {
                    self.nodes[entry].instruction = LegacyInstruction::Unknown {
                        description: format!("duplicate label `{name}`"),
                    };
                }
                entry
            }
            Statement::Goto { line, label } => {
                let node = self.push(LegacyInstruction::NoOp, Vec::new(), *line, false);
                if let Some(target) = self.labels.get(label).copied() {
                    self.nodes[node].successors = vec![target];
                } else {
                    self.pending_gotos.push((node, label.clone(), continuation));
                }
                node
            }
            Statement::Break { line } => {
                if let Some(target) = self.break_targets.last().copied() {
                    self.push(LegacyInstruction::NoOp, vec![target], *line, false)
                } else {
                    self.push(
                        LegacyInstruction::Unknown {
                            description: "break outside loop or switch".to_owned(),
                        },
                        vec![continuation],
                        *line,
                        false,
                    )
                }
            }
            Statement::Continue { line } => {
                if let Some(target) = self.continue_targets.last().copied() {
                    self.push(LegacyInstruction::NoOp, vec![target], *line, false)
                } else {
                    self.push(
                        LegacyInstruction::Unknown {
                            description: "continue outside loop".to_owned(),
                        },
                        vec![continuation],
                        *line,
                        false,
                    )
                }
            }
        }
    }

    fn push(
        &mut self,
        instruction: LegacyInstruction,
        successors: Vec<LegacyNodeId>,
        line: usize,
        is_exit: bool,
    ) -> LegacyNodeId {
        let id = self.nodes.len();
        self.nodes.push(LegacyCfgNode {
            instruction,
            successors,
            line,
            is_exit,
        });
        id
    }
}

fn branch_instruction(
    condition_supported: bool,
    null_condition: &Option<PointerNullCondition>,
    unsupported_description: &str,
) -> LegacyInstruction {
    if let Some(condition) = null_condition {
        LegacyInstruction::BranchOnNull {
            source: condition.source.clone(),
            true_is_null: condition.true_is_null,
        }
    } else if condition_supported {
        LegacyInstruction::NoOp
    } else {
        LegacyInstruction::Unknown {
            description: unsupported_description.to_owned(),
        }
    }
}

const LIVE: u8 = 1;
const RELEASED: u8 = 2;
const ESCAPED: u8 = 4;
const CONTAINED: u8 = 8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AbstractPointer {
    targets: BTreeSet<usize>,
    unknown: bool,
    may_be_null: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AbstractState {
    pointers: BTreeMap<String, AbstractPointer>,
    resources: BTreeMap<usize, u8>,
    nullable_sites: BTreeSet<usize>,
    conditional_replacements: BTreeMap<usize, BTreeSet<usize>>,
    contained: BTreeMap<usize, BTreeSet<usize>>,
    widened: bool,
}

const MAX_ALIAS_TARGETS: usize = 8;

impl AbstractState {
    fn join(&mut self, incoming: &Self) -> bool {
        let before = self.clone();
        let variables = self
            .pointers
            .keys()
            .chain(incoming.pointers.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for variable in variables {
            match (
                self.pointers.get(&variable).cloned(),
                incoming.pointers.get(&variable),
            ) {
                (Some(mut current), Some(other)) => {
                    current.targets.extend(other.targets.iter().copied());
                    current.unknown |= other.unknown;
                    current.may_be_null |= other.may_be_null;
                    if current.targets.len() > MAX_ALIAS_TARGETS {
                        current.targets.clear();
                        current.unknown = true;
                        self.widened = true;
                    }
                    self.pointers.insert(variable, current);
                }
                (Some(mut current), None) => {
                    current.unknown = true;
                    current.may_be_null = true;
                    self.pointers.insert(variable, current);
                }
                (None, Some(other)) => {
                    let mut joined = other.clone();
                    joined.unknown = true;
                    joined.may_be_null = true;
                    self.pointers.insert(variable, joined);
                }
                (None, None) => {}
            }
        }
        for (&site, &status) in &incoming.resources {
            self.resources
                .entry(site)
                .and_modify(|current| *current |= status)
                .or_insert(status);
        }
        self.nullable_sites
            .extend(incoming.nullable_sites.iter().copied());
        for pointer in self.pointers.values() {
            if pointer.may_be_null {
                self.nullable_sites.extend(pointer.targets.iter().copied());
            }
        }
        for (&site, replaced) in &incoming.conditional_replacements {
            self.conditional_replacements
                .entry(site)
                .or_default()
                .extend(replaced.iter().copied());
        }
        for (&owner, children) in &incoming.contained {
            self.contained
                .entry(owner)
                .or_default()
                .extend(children.iter().copied());
        }
        self.widened |= incoming.widened;
        *self != before
    }
}

fn analyze_cfg(cfg: LegacyCfg) -> LegacyFunctionSummary {
    let mut incoming = vec![None::<AbstractState>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(AbstractState::default());
    let mut work = VecDeque::from([cfg.entry]);
    let mut diagnostics = BTreeSet::new();
    let mut steps = 0usize;
    let acquisition_lines = cfg
        .nodes
        .iter()
        .flat_map(|node| match &node.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![(*site, node.line)],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites
                .iter()
                .map(|site| (*site, node.line))
                .collect(),
            _ => Vec::new(),
        })
        .collect::<BTreeMap<_, _>>();

    while let Some(node_id) = work.pop_front() {
        steps += 1;
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let node = &cfg.nodes[node_id];
        let output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &mut diagnostics,
        );
        if node.is_exit {
            report_leaks(
                &cfg.function,
                node.line,
                &output,
                &acquisition_lines,
                &mut diagnostics,
            );
        }
        for (edge, &successor) in node.successors.iter().enumerate() {
            let mut edge_output = output.clone();
            if let LegacyInstruction::BranchOnNull {
                source,
                true_is_null,
            } = &node.instruction
            {
                let assume_null = if edge == 0 {
                    *true_is_null
                } else {
                    !*true_is_null
                };
                match refine_pointer_null(&mut edge_output, source, assume_null) {
                    PointerRefinement::Infeasible => continue,
                    PointerRefinement::Unknown => {
                        diagnostics.insert(LegacyDiagnostic {
                            function: cfg.function.clone(),
                            line: node.line,
                            kind: LegacyDiagnosticKind::UnknownAlias {
                                variable: source.clone(),
                            },
                        });
                    }
                    PointerRefinement::Refined => {}
                }
            }
            let changed = match &mut incoming[successor] {
                Some(current) => current.join(&edge_output),
                slot @ None => {
                    *slot = Some(edge_output);
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }

    if incoming.iter().flatten().any(|state| state.widened) {
        diagnostics.insert(LegacyDiagnostic {
            function: cfg.function.clone(),
            line: 0,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "alias set exceeded {MAX_ALIAS_TARGETS} targets and was widened"
                ),
            },
        });
    }

    let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    let unsafe_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::Leak { .. }
                | LegacyDiagnosticKind::DoubleRelease { .. }
                | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
                | LegacyDiagnosticKind::CustodyConflict { .. }
        )
    });
    let unknown_found = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::UnknownAlias { .. } | LegacyDiagnosticKind::Unsupported { .. }
        )
    });
    let status = if unsafe_found {
        AnalysisStatus::Unsafe
    } else if unknown_found {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    };
    LegacyFunctionSummary {
        function: cfg.function.clone(),
        status,
        diagnostics,
        fixed_point_steps: steps,
        cfg,
    }
}

fn transfer(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    match &node.instruction {
        LegacyInstruction::Acquire { destination, site } => {
            state.resources.insert(*site, LIVE);
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([*site]),
                    unknown: false,
                    may_be_null: false,
                },
            );
        }
        LegacyInstruction::Alias {
            destination,
            source,
        } => {
            let pointer = state.pointers.get(source).cloned().unwrap_or_else(|| {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.clone(),
                    },
                });
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                    may_be_null: true,
                }
            });
            state.pointers.insert(destination.clone(), pointer);
        }
        LegacyInstruction::Clear { destination } => {
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    may_be_null: true,
                    ..AbstractPointer::default()
                },
            );
        }
        LegacyInstruction::Release { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                false,
                false,
            );
        }
        LegacyInstruction::Escape { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                true,
                false,
            );
        }
        LegacyInstruction::EscapeIfOwned { source } => {
            if state.pointers.contains_key(source) {
                apply_resource_action(
                    function,
                    node.line,
                    source,
                    &mut state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    false,
                );
            }
        }
        LegacyInstruction::ContractCall {
            function: called,
            destination,
            arguments,
            contract,
            produced_sites,
        } => apply_contract_call(
            function,
            node.line,
            called,
            destination.as_deref(),
            arguments,
            contract,
            produced_sites,
            &mut state,
            acquisition_lines,
            diagnostics,
        ),
        LegacyInstruction::BranchOnNull { .. } => {}
        LegacyInstruction::Unknown { description } => {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: description.clone(),
                },
            });
            for pointer in state.pointers.values_mut() {
                pointer.unknown = true;
                pointer.may_be_null = true;
            }
        }
        LegacyInstruction::NoOp => {}
    }
    state
}

#[allow(clippy::too_many_arguments)]
fn apply_resource_action(
    function: &str,
    line: usize,
    source: &str,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    escape: bool,
    nullable: bool,
) {
    let pointer = state
        .pointers
        .get(source)
        .cloned()
        .unwrap_or(AbstractPointer {
            targets: BTreeSet::new(),
            unknown: true,
            may_be_null: true,
        });
    if pointer.unknown || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null)) {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    if !nullable
        && pointer
            .targets
            .iter()
            .any(|site| state.nullable_sites.contains(site))
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!("nullable resource `{source}` used by a non-null contract"),
            },
        });
    }
    for site in pointer.targets {
        let status = state.resources.get(&site).copied().unwrap_or(0);
        if status & CONTAINED != 0 {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
        if escape {
            state
                .resources
                .insert(site, (status & !(LIVE | CONTAINED)) | ESCAPED);
        } else {
            if status & RELEASED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::DoubleRelease {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            if status & ESCAPED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::ReleaseAfterEscape {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            state
                .resources
                .insert(site, (status & !(LIVE | CONTAINED)) | RELEASED);
        }
        cascade_resource_action(state, site, escape);
    }
}

fn cascade_resource_action(state: &mut AbstractState, owner: usize, escape: bool) {
    let mut pending = state.contained.remove(&owner).unwrap_or_default();
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if !visited.insert(site) {
            continue;
        }
        if let Some(children) = state.contained.remove(&site) {
            pending.extend(children);
        }
        let status = state.resources.get(&site).copied().unwrap_or(0);
        let terminal = if escape { ESCAPED } else { RELEASED };
        state
            .resources
            .insert(site, (status & !(LIVE | CONTAINED)) | terminal);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_contract_call(
    function: &str,
    line: usize,
    called: &str,
    destination: Option<&str>,
    arguments: &[Option<String>],
    contract: &ResourceFunctionContract,
    produced_sites: &[usize],
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let mut sites = produced_sites.iter().copied();
    // Apply input custody effects before assigning a return value. This is
    // essential when the destination aliases a consumed/reallocated argument.
    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        let source = arguments.get(index).and_then(Option::as_deref);
        match mode {
            ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => {}
            ResourceParameterMode::Owned { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                require_live_resource(function, line, source, nullable, state, diagnostics);
            }
            ResourceParameterMode::Consumed { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                apply_resource_action(
                    function,
                    line,
                    source,
                    state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    nullable,
                );
            }
            ResourceParameterMode::Released { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                apply_resource_action(
                    function,
                    line,
                    source,
                    state,
                    acquisition_lines,
                    diagnostics,
                    false,
                    nullable,
                );
            }
            ResourceParameterMode::Stored {
                owner_parameter,
                nullable,
            } => {
                let owner = arguments.get(owner_parameter).and_then(Option::as_deref);
                let (Some(source), Some(owner)) = (source, owner) else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                attach_contained_resource(
                    function,
                    line,
                    owner,
                    source,
                    nullable,
                    state,
                    acquisition_lines,
                    diagnostics,
                );
            }
            ResourceParameterMode::Removed {
                owner_parameter,
                nullable,
            } => {
                let owner = arguments.get(owner_parameter).and_then(Option::as_deref);
                let (Some(source), Some(owner)) = (source, owner) else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                detach_contained_resource(
                    function,
                    line,
                    owner,
                    source,
                    nullable,
                    state,
                    acquisition_lines,
                    diagnostics,
                );
            }
        }
    }

    match contract.result {
        ResourceReturnMode::None => {}
        ResourceReturnMode::Owned { nullable } => {
            let (Some(destination), Some(site)) = (destination, sites.next()) else {
                report_unsupported_contract(function, line, called, diagnostics);
                return;
            };
            create_resource(state, destination, site, nullable);
        }
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => {
            let source = arguments.get(parameter).and_then(Option::as_deref);
            let (Some(destination), Some(source), Some(site)) = (destination, source, sites.next())
            else {
                report_unsupported_contract(function, line, called, diagnostics);
                return;
            };
            let Some(old) = state.pointers.get(source).cloned() else {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                return;
            };
            if old.unknown || old.targets.is_empty() {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
            }
            create_resource(state, destination, site, nullable);
            state
                .conditional_replacements
                .insert(site, old.targets.clone());
            if !nullable {
                finalize_replacement_success(state, site);
            }
        }
    }

    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        if let ResourceParameterMode::Out { nullable } = mode {
            let source = arguments.get(index).and_then(Option::as_deref);
            let (Some(destination), Some(site)) = (source, sites.next()) else {
                report_unsupported_contract(function, line, called, diagnostics);
                continue;
            };
            create_resource(state, destination, site, nullable);
        }
    }
}

fn create_resource(state: &mut AbstractState, destination: &str, site: usize, nullable: bool) {
    state.resources.insert(site, LIVE);
    if nullable {
        state.nullable_sites.insert(site);
    } else {
        // Re-executing a non-null acquisition site in a loop is a strong
        // update. A predecessor on which the destination did not yet exist
        // must not permanently poison the fresh result as nullable.
        state.nullable_sites.remove(&site);
    }
    state.pointers.insert(
        destination.to_owned(),
        AbstractPointer {
            targets: BTreeSet::from([site]),
            unknown: false,
            may_be_null: nullable,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn attach_contained_resource(
    function: &str,
    line: usize,
    owner: &str,
    source: &str,
    nullable: bool,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    require_live_resource(function, line, owner, false, state, diagnostics);
    require_live_resource(function, line, source, nullable, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        return;
    };
    if owners.unknown || children.unknown || owners.targets.len() != 1 {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        return;
    }
    let owner_site = *owners.targets.first().expect("one owner target");
    for child in children.targets {
        let status = state.resources.get(&child).copied().unwrap_or(0);
        if status & LIVE == 0 || status & CONTAINED != 0 || child == owner_site {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, child),
                },
            });
            continue;
        }
        state.resources.insert(child, (status & !LIVE) | CONTAINED);
        state.contained.entry(owner_site).or_default().insert(child);
    }
}

#[allow(clippy::too_many_arguments)]
fn detach_contained_resource(
    function: &str,
    line: usize,
    owner: &str,
    source: &str,
    nullable: bool,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    require_live_resource(function, line, owner, false, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return;
    };
    if owners.unknown
        || children.unknown
        || owners.targets.len() != 1
        || (!nullable && (children.targets.is_empty() || children.may_be_null))
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        return;
    }
    let owner_site = *owners.targets.first().expect("one owner target");
    for child in children.targets {
        let attached = state
            .contained
            .get(&owner_site)
            .is_some_and(|contained| contained.contains(&child));
        let status = state.resources.get(&child).copied().unwrap_or(0);
        if !attached || status & CONTAINED == 0 {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, child),
                },
            });
            continue;
        }
        if let Some(contained) = state.contained.get_mut(&owner_site) {
            contained.remove(&child);
            if contained.is_empty() {
                state.contained.remove(&owner_site);
            }
        }
        state.resources.insert(child, (status & !CONTAINED) | LIVE);
    }
}

fn require_live_resource(
    function: &str,
    line: usize,
    source: &str,
    nullable: bool,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let Some(pointer) = state.pointers.get(source) else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return;
    };
    if pointer.unknown
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null))
        || (!nullable
            && pointer
                .targets
                .iter()
                .any(|site| state.nullable_sites.contains(site)))
        || pointer
            .targets
            .iter()
            .any(|site| state.resources.get(site).copied().unwrap_or(0) & LIVE == 0)
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
}

fn report_unsupported_contract(
    function: &str,
    line: usize,
    called: &str,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("arguments do not match resource contract for `{called}`"),
        },
    });
}

fn finalize_replacement_success(state: &mut AbstractState, site: usize) {
    state.nullable_sites.remove(&site);
    if let Some(replaced) = state.conditional_replacements.remove(&site) {
        for old in replaced {
            let status = state.resources.get(&old).copied().unwrap_or(0);
            state.resources.insert(old, (status & !LIVE) | RELEASED);
        }
    }
}

enum PointerRefinement {
    Refined,
    Infeasible,
    Unknown,
}

fn refine_pointer_null(
    state: &mut AbstractState,
    source: &str,
    assume_null: bool,
) -> PointerRefinement {
    let Some(pointer) = state.pointers.get(source).cloned() else {
        // The reduced importer has no scalar type environment. An untracked
        // identifier may be an ordinary integer condition, so leave it alone.
        return PointerRefinement::Refined;
    };
    if pointer.unknown || pointer.targets.len() > 1 {
        return PointerRefinement::Unknown;
    }
    let Some(site) = pointer.targets.iter().next().copied() else {
        return if assume_null {
            PointerRefinement::Refined
        } else {
            PointerRefinement::Infeasible
        };
    };
    if !pointer.may_be_null && !state.nullable_sites.contains(&site) {
        return if assume_null {
            PointerRefinement::Infeasible
        } else {
            PointerRefinement::Refined
        };
    }
    if assume_null {
        state.resources.remove(&site);
        state.nullable_sites.remove(&site);
        state.conditional_replacements.remove(&site);
        for alias in state.pointers.values_mut() {
            if alias.targets.remove(&site) {
                alias.may_be_null = true;
            }
        }
    } else {
        finalize_replacement_success(state, site);
        for alias in state.pointers.values_mut() {
            if alias.targets.contains(&site) {
                alias.may_be_null = false;
            }
        }
    }
    PointerRefinement::Refined
}

fn acquisition_line(acquisition_lines: &BTreeMap<usize, usize>, site: usize) -> usize {
    acquisition_lines.get(&site).copied().unwrap_or(0)
}

fn report_leaks(
    function: &str,
    line: usize,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (&site, &status) in &state.resources {
        if status & LIVE != 0 {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Leak {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_c.rs"]
mod tests;
