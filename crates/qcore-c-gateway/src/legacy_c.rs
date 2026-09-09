//! Conservative resource analysis for the Cycle 9 annotated legacy-C subset.
//!
//! The parser intentionally accepts more C than the analyzer models.  Every
//! construct outside the supported structured subset becomes an `Unknown`
//! instruction. `Safe` concerns the supported custody projection under its
//! declared resource contracts, not arbitrary C memory safety. Indirect reads,
//! aggregate/indexed accesses and ambiguous expression forms without a model
//! remain unsupported; symbolic resource place spelling is not access proof.

#[path = "legacy_out_alias.rs"]
mod out_alias;

#[path = "legacy_enumeration.rs"]
pub mod enumeration;

pub use enumeration::{
    EnumerationRefusal, SourceDefinitionInventory, UnanalyzableDefinition, UnanalyzableForm,
    enumerate_source_definitions,
};

#[path = "legacy_recursion.rs"]
pub(crate) mod recursion;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

pub type LegacyNodeId = usize;

pub use qcore_schemas::contract::{
    ContractTrust, ResourceApiKind, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
};

use crate::aggregate_store::AggregateStoreBases;
use crate::borrowed_actual::UntrackedBases;
use crate::legacy_summary::{CustodyDerivation, derive_custody_summaries};
use crate::typed_field_access::{FunctionTypes, TypedAccessModel};

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
    /// Bind `destination` to a value this function does not track: the read of
    /// an admitted direct member of an untracked aggregate (C02-P, O04).
    ///
    /// The resulting abstract pointer is exactly the one an [`Self::Alias`]
    /// from an unbound place would produce — unknown and possibly null, so
    /// every downstream use stays fail-closed. The only difference is that no
    /// unresolved-alias diagnostic is reported, because the typed premises of
    /// [`crate::borrowed_actual`] establish that the source place is one this
    /// function never binds. It never creates, releases or transfers custody.
    OpaqueValue {
        destination: String,
    },
    /// A store into an admitted field of an aggregate this function tracks
    /// whose stored value carries no tracked identity (O00 blocker B08): a
    /// scalar expression into a scalar member, or a declared pointer this
    /// function never binds into a single-star pointer member. See
    /// [`crate::aggregate_store`].
    ///
    /// It is *not* a no-op. Storing through an aggregate this function already
    /// released, escaped or never resolved is refused by [`transfer`], and the
    /// store clears whatever field link the member previously carried, so an
    /// overwritten resource is never silently discharged.
    AggregateStore {
        place: String,
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
    Leak {
        acquisition_line: usize,
    },
    DoubleRelease {
        acquisition_line: usize,
    },
    ReleaseAfterEscape {
        acquisition_line: usize,
    },
    CustodyConflict {
        acquisition_line: usize,
    },
    UnknownAlias {
        variable: String,
    },
    Unsupported {
        description: String,
    },
    /// A validated inferred custody summary contradicts the explicit contract
    /// in force for the same function (W4). The explicit contract still wins at
    /// every call site; the disagreement is reported instead of being silently
    /// overridden, and it keeps the defining function `Unknown`.
    SummaryConflict {
        inferred: Box<ResourceFunctionContract>,
        declared: Box<ResourceFunctionContract>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegacyDiagnostic {
    pub function: String,
    pub line: usize,
    pub kind: LegacyDiagnosticKind,
}

/// What one reachable exit of a function leaves behind, per resource identity.
///
/// The function-wide [`AnalysisStatus`] is a join over every path: one
/// unmodeled construct anywhere makes the whole function `Unknown` and says
/// nothing about the paths that *are* modeled. Card O04 needs the finer
/// statement — "on this failure arm the constructed identity is released
/// exactly once" — for a caller whose success exit publishes into aggregates
/// this projection cannot follow. This record supplies it without changing any
/// verdict: it is an additional observation, never an input to a status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyExitResidual {
    pub node: LegacyNodeId,
    /// Input line of the exit statement.
    pub line: usize,
    pub custody: ExitCustody,
    /// Every resource identity this function acquired that is still accounted
    /// for at this exit, in acquisition-site order. Identities the caller owns
    /// (formal parameters under a contract) are excluded: discharging them is
    /// the caller's obligation, not this exit's.
    pub sites: Vec<LegacyResidualSite>,
}

/// One resource identity's disposition at one exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyResidualSite {
    pub site: usize,
    /// Input line of the acquisition, or 0 when it is not a modeled call.
    pub acquisition_line: usize,
    pub disposition: SiteDisposition,
}

/// Whether an exit's custody obligations are settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitCustody {
    /// Every path reaching this exit is fully modeled, and no identity this
    /// function acquired is still live here.
    Discharged,
    /// Every path reaching this exit is fully modeled, and at least one
    /// identity is still live — a residual this analysis does claim.
    Residual,
    /// Some path reaching this exit carries an unmodeled construct, an
    /// unresolved effect or a widened alias set. No custody claim is made, in
    /// either direction.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyFunctionSummary {
    pub function: String,
    pub status: AnalysisStatus,
    pub diagnostics: Vec<LegacyDiagnostic>,
    /// Number of work-list transfers used to reach the CFG fixed point.
    pub fixed_point_steps: usize,
    pub cfg: LegacyCfg,
    /// Per-exit custody residuals, in CFG node order (O04).
    pub exits: Vec<LegacyExitResidual>,
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
    MalformedFunction {
        function: String,
        line: usize,
    },
    ProductionParserUnavailable(String),
    ProductionParserRejected(String),
    /// The unit uses a construct whose meaning depends on the input identity
    /// its declared [`CSourceInput`] cannot supply. Rejected, never guessed.
    UnsupportedInputIdentity(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct C11TranslationUnit<'a> {
    pub source: &'a str,
    pub include_paths: Vec<String>,
    pub definitions: Vec<String>,
    /// Ordered, typed preprocessing options, applied after the legacy fields.
    pub preprocessor_options: Vec<CPreprocessorOption>,
    pub dialect: CCompilerDialect,
    pub function_scope: LegacyFunctionScope,
    /// How Clang is told to read this unit's primary input. See [`CSourceInput`].
    pub input: CSourceInput,
}

/// The declared identity of a translation unit's primary input.
///
/// Preprocessing is not a function of the source text alone: `__FILE__`,
/// `__FILE_NAME__`, `__BASE_FILE__` and quoted includes resolved against the
/// includer's directory all answer to the *input Clang was given*. Feeding text
/// through standard input answers `<stdin>` for the first three and resolves
/// quoted includes against the gateway process's working directory. This type
/// makes that choice explicit so a unit either sees the input identity it
/// declares, or is rejected for a construct the transport cannot answer.
///
/// This records how the input is transported. It is not a claim that the
/// gateway's staged preprocess/syntax/AST pipeline is equivalent to a direct
/// one-shot compilation.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub enum CSourceInput {
    /// A unit with no declared file: the text is piped to Clang as `-`.
    ///
    /// The unit's identity is the text plus its declared options; quoted
    /// includes resolve against the gateway process's recorded working
    /// directory, because an anonymous text has no includer directory of its
    /// own. Constructs that would have to invent a file name are rejected.
    #[default]
    AnonymousText,
    /// A declared file: Clang is run from `directory` and given `spelled`
    /// exactly as a compilation database records the input argument, so
    /// `__FILE__` and quoted includes answer as a direct compile does.
    ///
    /// The declared source bytes must equal the file's bytes; otherwise the
    /// unit is rejected rather than analyzed under a different text.
    DeclaredFile {
        /// The input argument exactly as the recorded compile command spells it.
        spelled: String,
        /// The absolute working directory the recorded compile command declares.
        directory: PathBuf,
    },
    /// The historical Cycle 19 compatibility projection: standard input with
    /// `<stdin>` accepted as the unit's file name.
    ///
    /// Retained only for the pinned compatibility adapter whose archived
    /// artifacts were produced under it. Selecting this is a declared
    /// assumption that the unit's filename-sensitive constructs do not matter
    /// to its projection; it is never an equivalence claim.
    HistoricalStdin,
}

impl CSourceInput {
    /// The input identity for a unit whose text is *derived* from this one, for
    /// example by appending a probe. Derived text is not the declared file, so
    /// a declared identity is dropped rather than reused for different bytes.
    pub fn for_derived_text(&self) -> Self {
        match self {
            Self::HistoricalStdin => Self::HistoricalStdin,
            Self::AnonymousText | Self::DeclaredFile { .. } => Self::AnonymousText,
        }
    }

    /// Whether the transport must reject `<stdin>` file-name expansions.
    fn rejects_stdin_file_names(&self) -> bool {
        matches!(self, Self::AnonymousText)
    }

    /// Whether the transport must reject deferred source-location builtins.
    ///
    /// These survive preprocessing and are resolved by the frontend, which the
    /// gateway always feeds preprocessed text through standard input. Only the
    /// pinned historical route keeps its previous silent behavior.
    fn rejects_source_location_builtins(&self) -> bool {
        !matches!(self, Self::HistoricalStdin)
    }
}

/// Bounded preprocessing transport. These are arguments, never shell commands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CPreprocessorOption {
    Include(String),
    SystemInclude(String),
    QuoteInclude(String),
    Define(String),
    Undefine(String),
    FrontendFlag(CFrontendFlag),
}

/// One `-O` optimization level, spelled exactly as the build spelled it.
///
/// An optimization level is *not* a code-generation-only request: Clang's
/// preprocessor defines `__OPTIMIZE__` (and `__OPTIMIZE_SIZE__` for the size
/// levels) and drops `__NO_INLINE__` whenever a level above `-O0` is selected,
/// and glibc headers branch on those macros. Dropping the build's level
/// therefore analyses a *different* translation unit; the measurement backing
/// this is recorded in `docs/reports/remaining-g03-f/README.md`. Levels are a
/// closed set: a level this enum cannot name is refused, never approximated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum COptimizationLevel {
    /// `-O0`
    None,
    /// `-O1`
    Level1,
    /// `-O2`
    Level2,
    /// `-O3`
    Level3,
    /// `-Os`
    Size,
    /// `-Oz`
    MinimumSize,
    /// `-Og`
    Debug,
}

impl COptimizationLevel {
    /// The build spelling this level was admitted from, reproduced verbatim.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::None => "-O0",
            Self::Level1 => "-O1",
            Self::Level2 => "-O2",
            Self::Level3 => "-O3",
            Self::Size => "-Os",
            Self::MinimumSize => "-Oz",
            Self::Debug => "-Og",
        }
    }
}

/// Explicit build flags admitted to Clang preprocessing, syntax and typed AST.
/// Assembler requests are retained but no object/stack property is certified.
///
/// Every member is admitted because its effect on what this gateway reads —
/// the preprocessed token stream, the syntax phase and the typed AST — has a
/// recorded preservation argument, and it is reproduced verbatim on all three
/// Clang invocations by [`CFrontendFlag::append_to`]. A build option with no
/// such argument stays refused by the callers that build this type; there is no
/// catch-all member and no "ignore unknown flag" path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CFrontendFlag {
    AllWarnings,
    ExtraWarnings,
    /// Warn when an undefined preprocessor identifier starts with this prefix.
    UndefPrefix(String),
    DebugInformation,
    PositionIndependentCode,
    AssemblerNoExecStack,
    /// Select one Apple Clang architecture using its two-argument `-arch` form.
    Architecture(String),
    // --- Diagnostic-only warning selections -------------------------------
    // Each of the members below selects or deselects one Clang warning group.
    // Warning selection is measured to leave the preprocessed token stream
    // byte-identical and the predefined macro set unchanged, so transporting
    // one preserves the recorded compile command without changing the analysed
    // program. None of them is `-Werror`: no warning becomes a hard error, so
    // an admitted unit cannot be turned into a refusal by its warning set.
    /// `-Wstrict-aliasing`
    StrictAliasingWarnings,
    /// `-Wstrict-prototypes`
    StrictPrototypes,
    /// `-Wundef`
    UndefinedIdentifierWarnings,
    /// `-Wmissing-prototypes`
    MissingPrototypes,
    /// `-Winit-self`
    InitSelf,
    /// `-Wmissing-field-initializers`
    MissingFieldInitializers,
    /// `-Wdeclaration-after-statement`
    DeclarationAfterStatement,
    /// `-Waddress`
    AddressComparisons,
    /// `-Woverride-init`
    OverrideInit,
    /// `-Wwrite-strings`
    WriteStrings,
    /// `-Wvla`
    VariableLengthArrays,
    /// `-Wno-unused-parameter`
    NoUnusedParameter,
    /// `-Wno-unused-function`
    NoUnusedFunction,
    /// `-Wno-pragmas`
    NoPragmas,
    /// `-Wno-void-pointer-to-enum-cast`
    NoVoidPointerToEnumCast,
    // --- Code generation ---------------------------------------------------
    /// `-fno-strict-aliasing`: relaxes the aliasing rule the optimizer may
    /// assume. Measured to change neither the predefined macros nor the
    /// preprocessed text; it is transported so the recorded command is
    /// reproduced, and, exactly like [`Self::AssemblerNoExecStack`], no object
    /// or aliasing property is certified by admitting it.
    NoStrictAliasing,
    /// `-O0` … `-Og`: transported because the level is visible to the
    /// preprocessor (see [`COptimizationLevel`]).
    Optimization(COptimizationLevel),
}

impl CFrontendFlag {
    fn append_to(&self, command: &mut std::process::Command) {
        match self {
            Self::AllWarnings => command.arg("-Wall"),
            Self::ExtraWarnings => command.arg("-Wextra"),
            Self::UndefPrefix(prefix) => command.arg(format!("-Wundef-prefix={prefix}")),
            Self::DebugInformation => command.arg("-g"),
            Self::PositionIndependentCode => command.arg("-fPIC"),
            Self::AssemblerNoExecStack => command.arg("-Wa,--noexecstack"),
            Self::Architecture(architecture) => command.args(["-arch", architecture]),
            Self::StrictAliasingWarnings => command.arg("-Wstrict-aliasing"),
            Self::StrictPrototypes => command.arg("-Wstrict-prototypes"),
            Self::UndefinedIdentifierWarnings => command.arg("-Wundef"),
            Self::MissingPrototypes => command.arg("-Wmissing-prototypes"),
            Self::InitSelf => command.arg("-Winit-self"),
            Self::MissingFieldInitializers => command.arg("-Wmissing-field-initializers"),
            Self::DeclarationAfterStatement => command.arg("-Wdeclaration-after-statement"),
            Self::AddressComparisons => command.arg("-Waddress"),
            Self::OverrideInit => command.arg("-Woverride-init"),
            Self::WriteStrings => command.arg("-Wwrite-strings"),
            Self::VariableLengthArrays => command.arg("-Wvla"),
            Self::NoUnusedParameter => command.arg("-Wno-unused-parameter"),
            Self::NoUnusedFunction => command.arg("-Wno-unused-function"),
            Self::NoPragmas => command.arg("-Wno-pragmas"),
            Self::NoVoidPointerToEnumCast => command.arg("-Wno-void-pointer-to-enum-cast"),
            Self::NoStrictAliasing => command.arg("-fno-strict-aliasing"),
            Self::Optimization(level) => command.arg(level.spelling()),
        };
    }
}

impl C11TranslationUnit<'_> {
    pub(crate) fn append_frontend_flags(&self, command: &mut std::process::Command) {
        for option in &self.preprocessor_options {
            if let CPreprocessorOption::FrontendFlag(flag) = option {
                flag.append_to(command);
            }
        }
    }

    /// Append the primary input for the phase that reads the original source,
    /// and report whether the source text must also be piped to standard input.
    ///
    /// A declared file is checked against its own bytes first: analyzing text
    /// that differs from the file Clang reads would make the recorded input
    /// identity a different unit from the effective one.
    pub(crate) fn append_primary_input(
        &self,
        command: &mut std::process::Command,
    ) -> Result<bool, LegacyCError> {
        match &self.input {
            CSourceInput::AnonymousText | CSourceInput::HistoricalStdin => {
                command.arg("-");
                Ok(true)
            }
            CSourceInput::DeclaredFile { spelled, directory } => {
                if !directory.is_absolute() {
                    return Err(LegacyCError::UnsupportedInputIdentity(
                        "declared input directory must be absolute".into(),
                    ));
                }
                if spelled.is_empty() || spelled.starts_with('-') {
                    return Err(LegacyCError::UnsupportedInputIdentity(format!(
                        "declared input `{spelled}` is not a source path argument"
                    )));
                }
                let path = directory.join(spelled);
                let bytes = std::fs::read(&path).map_err(|error| {
                    LegacyCError::UnsupportedInputIdentity(format!(
                        "declared input `{}` is unreadable: {error}",
                        path.display()
                    ))
                })?;
                if bytes != self.source.as_bytes() {
                    return Err(LegacyCError::UnsupportedInputIdentity(format!(
                        "declared input `{}` does not hold the declared source bytes",
                        path.display()
                    )));
                }
                command.current_dir(directory).arg(spelled);
                Ok(false)
            }
        }
    }
}

/// Constructs whose expansion is decided by the input Clang was given.
///
/// The scan runs over preprocessed output, so a construct reaching the unit
/// through an included header is seen exactly like one spelled in the source.
fn reject_filename_sensitive_constructs(
    preprocessed: &str,
    input: &CSourceInput,
) -> Result<(), LegacyCError> {
    let bytes = preprocessed.as_bytes();
    let mut cursor = 0;
    let mut line = 1usize;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'\n' {
            line += 1;
            cursor += 1;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                line += usize::from(bytes[cursor] == b'\n');
                cursor += 1;
            }
            cursor = (cursor + 2).min(bytes.len());
        } else if byte == b'"' || byte == b'\'' {
            let quote = byte;
            let start = cursor + 1;
            cursor += 1;
            while cursor < bytes.len() {
                if bytes[cursor] == b'\\' {
                    cursor = (cursor + 2).min(bytes.len());
                } else if bytes[cursor] == quote {
                    break;
                } else {
                    line += usize::from(bytes[cursor] == b'\n');
                    cursor += 1;
                }
            }
            // `__FILE__`, `__FILE_NAME__` and `__BASE_FILE__` all expand to the
            // input's name. Standard input names it `<stdin>`, which is not the
            // unit's file, so the expansion is a construct without an answer.
            if input.rejects_stdin_file_names()
                && &preprocessed[start..cursor.min(bytes.len())] == "<stdin>"
            {
                return Err(LegacyCError::UnsupportedInputIdentity(format!(
                    "preprocessed line {line}: a source file name macro \
                     (`__FILE__`, `__FILE_NAME__` or `__BASE_FILE__`) expanded to \
                     `<stdin>` because this unit declares no input file"
                )));
            }
            cursor = (cursor + 1).min(bytes.len());
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            let name = &preprocessed[start..cursor];
            // These survive preprocessing and are answered by the frontend,
            // which always reads preprocessed text from standard input here.
            if input.rejects_source_location_builtins()
                && matches!(name, "__builtin_FILE" | "__builtin_FILE_NAME")
            {
                return Err(LegacyCError::UnsupportedInputIdentity(format!(
                    "preprocessed line {line}: `{name}` is answered after \
                     preprocessing, where this gateway reads standard input"
                )));
            }
        } else {
            cursor += 1;
        }
    }
    Ok(())
}

impl CPreprocessorOption {
    fn append_to(&self, command: &mut std::process::Command) {
        let (flag, value) = match self {
            Self::Include(value) => ("-I", value),
            Self::SystemInclude(value) => ("-isystem", value),
            Self::QuoteInclude(value) => ("-iquote", value),
            Self::Define(value) => ("-D", value),
            Self::Undefine(value) => ("-U", value),
            Self::FrontendFlag(flag) => {
                flag.append_to(command);
                return;
            }
        };
        command.args([flag, value]);
    }
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
    pub(crate) fn clang_argument(self) -> &'static str {
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
    /// Interprocedural custody summaries the analysis relied on (W4). Only
    /// [`ContractTrust::InferredValidated`] records appear here; declared
    /// contracts already travel in `contracts`. The recheck re-derives this
    /// list independently and refuses a certificate that does not reproduce it.
    pub summaries: Vec<ResourceSummaryRecord>,
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
                    "could not project body for C function `{function}` at input line {line}"
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
            Self::UnsupportedInputIdentity(message) => {
                write!(
                    f,
                    "translation unit input identity cannot answer a \
                     filename-sensitive construct: {message}"
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
    Ok(analyze_cfg(build_function_cfg(
        tokens, function, contracts,
    )?))
}

/// Gate the conservative importer with Clang's production C11 parser.  The
/// typed AST accepted by Clang is still deliberately projected into the small
/// resource language below; any projection gap remains `Unknown`.
pub fn analyze_legacy_c11(
    source: &str,
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let mut child = crate::clang::command()
        .map_err(LegacyCError::ProductionParserUnavailable)?
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
    let inventory = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(enumerate_source_definitions(unit)),
    };
    analyze_unrestricted_c11_with_inventory(unit, entry, contracts, inventory.as_ref())
}

/// [`analyze_unrestricted_c11`] over an inventory the caller already
/// enumerated (unit **E01-F1**).
///
/// A driver that reports the denominator must enumerate it anyway, and the
/// enumeration costs one Clang preprocessing; this entry point lets the two
/// share it, and — more importantly — guarantees the analyzer's selection is
/// *the same inventory the report publishes*, rather than a second, possibly
/// divergent, computation of it.
///
/// `inventory` is `None` exactly when the unit's scope is
/// [`LegacyFunctionScope::PreprocessedTranslationUnit`], where there is no
/// main-source restriction to apply.
pub fn analyze_unrestricted_c11_with_inventory(
    unit: &C11TranslationUnit<'_>,
    entry: &str,
    contracts: &ResourceContracts,
    inventory: Option<&SourceDefinitionInventory>,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let source_functions = match unit.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => Some(inventory.map_or_else(
            || enumerate_source_definitions(unit).definitions().to_vec(),
            |inventory| inventory.definitions().to_vec(),
        )),
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
    let summaries = analysis.summaries;
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
            reason: unresolved_projection_reason(&summary),
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
                summaries,
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

fn unresolved_projection_reason(summary: &LegacyProgramSummary) -> String {
    let mut unsupported = 0;
    let mut aliases = 0;
    let mut conflicts = 0;
    for diagnostic in summary
        .functions
        .values()
        .flat_map(|function| &function.diagnostics)
    {
        match diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { .. } => unsupported += 1,
            LegacyDiagnosticKind::UnknownAlias { .. } => aliases += 1,
            LegacyDiagnosticKind::SummaryConflict { .. } => conflicts += 1,
            _ => {}
        }
    }
    format!(
        "resource projection unresolved: {unsupported} unsupported operation(s), \
         {aliases} unresolved alias(es), {conflicts} custody summary conflict(s)"
    )
}

/// Preprocess and syntax-check an unrestricted C translation unit without
/// requiring it to contain an executable function body.
pub fn validate_unrestricted_c11(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    let mut command = crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
    command.args(["-x", "c", unit.dialect.clang_argument(), "-E", "-P"]);
    for include in &unit.include_paths {
        command.arg(format!("-I{include}"));
    }
    for definition in &unit.definitions {
        command.arg(format!("-D{definition}"));
    }
    for option in &unit.preprocessor_options {
        option.append_to(&mut command);
    }
    let piped = unit.append_primary_input(&mut command)?;
    let mut child = command
        .stdin(if piped { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if piped {
        child
            .stdin
            .take()
            .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
            .write_all(unit.source.as_bytes())
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyCError::ProductionParserRejected(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let preprocessed_source = String::from_utf8_lossy(&output.stdout).into_owned();
    reject_filename_sensitive_constructs(&preprocessed_source, &unit.input)?;
    let mut syntax_command =
        crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
    unit.append_frontend_flags(&mut syntax_command);
    let mut syntax = syntax_command
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
    crate::contract_loader::check_source_assumptions(&certificate.contracts)?;
    let selected = match certificate.function_scope {
        LegacyFunctionScope::PreprocessedTranslationUnit => None,
        LegacyFunctionScope::MainSourceDefinitions => {
            Some(certificate.checked_functions.as_slice())
        }
    };
    let recomputed = analyze_legacy_c_translation_unit_selected_partial(
        &certificate.preprocessed_source,
        &certificate.entry,
        &certificate.contracts,
        selected,
    )
    .map_err(|error| format!("certificate source cannot be reanalyzed: {error}"))?;
    if let Some(error) = recomputed.projection_failures.first() {
        return Err(format!("certificate source cannot be reanalyzed: {error}"));
    }
    // The recheck re-derives the interprocedural custody summaries from the
    // preprocessed source and the declared contracts alone. A certificate that
    // leans on a summary this pass cannot reproduce fails containment; a
    // summary is never taken on the certificate's word.
    if recomputed.summaries != certificate.summaries {
        return Err("inferred custody summaries do not match the preprocessed source".into());
    }
    if certificate
        .summaries
        .iter()
        .any(|record| record.trust != ContractTrust::InferredValidated)
    {
        return Err("certificate carries a custody summary that was not validated".into());
    }
    if recomputed.summary != certificate.summary {
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

pub(crate) struct PartialTranslationUnitAnalysis {
    pub(crate) summary: LegacyProgramSummary,
    pub(crate) projection_failures: Vec<LegacyCError>,
    /// Validated interprocedural custody summaries used at call sites.
    pub(crate) summaries: Vec<ResourceSummaryRecord>,
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
    // A retained callee summary does not prove neutral parameter/return
    // custody. Explicit trusted contracts are preserved; a call is modeled only
    // through such a contract or through an inferred summary that this pass
    // validated by re-running the intraprocedural analyzer against it (W4).
    // Everything else stays an uncontracted call and therefore `Unknown`.
    let derivation = derive_custody_summaries(&tokens, &names, contracts);
    let mut functions = BTreeMap::new();
    let mut projection_failures = Vec::new();
    for name in names {
        match analyze_defined_function(&tokens, &name, &derivation) {
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
        summaries: derivation.records(),
    })
}

/// Analyze one defined function under the derived environment.
///
/// The entry state comes from the contract in force for the function itself —
/// explicit if one was declared, otherwise the validated inferred summary — so
/// that a callee's own body is checked against the same custody interface its
/// callers use. If the body does not validate against that interface the
/// function falls back to the contract-free analysis, which makes no custody
/// assumption about the formals at all.
fn analyze_defined_function(
    tokens: &[Token],
    function: &str,
    derivation: &CustodyDerivation,
) -> Result<LegacyFunctionSummary, LegacyCError> {
    let cfg = match derivation.contextual_cfg(function) {
        Some(cfg) => cfg.clone(),
        None => build_function_cfg(tokens, function, derivation.environment())?,
    };
    let mut summary = match derivation.entry_contract(function) {
        Some(contract) => match function_parameters(tokens, function) {
            Some(parameters) => match EntryCustody::build(&cfg, &parameters, contract) {
                Some(entry) => {
                    let (candidate, observations) = analyze_cfg_with_entry(cfg.clone(), &entry);
                    if crate::legacy_summary::observations_match_contract(
                        contract,
                        &entry,
                        &candidate,
                        &observations,
                    ) {
                        candidate
                    } else {
                        analyze_cfg(cfg)
                    }
                }
                None => analyze_cfg(cfg),
            },
            None => analyze_cfg(cfg),
        },
        None => analyze_cfg(cfg),
    };
    if let Some((inferred, declared)) = derivation.conflict(function) {
        let line = find_function_body(tokens, function).map_or(0, |(_, line)| line);
        summary.diagnostics.push(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::SummaryConflict {
                inferred: Box::new(inferred.clone()),
                declared: Box::new(declared.clone()),
            },
        });
        summary.diagnostics.sort();
        summary.diagnostics.dedup();
        summary.status = classify_diagnostics(&summary.diagnostics);
        // A summary that contradicts the declared contract invalidates every
        // per-exit claim derived under it, including the exits the conflicting
        // interface never reached.
        for exit in &mut summary.exits {
            exit.custody = ExitCustody::Unknown;
        }
    }
    Ok(summary)
}

/// Parse one function body into its resource CFG under `contracts`.
pub(crate) fn build_function_cfg(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Result<LegacyCfg, LegacyCError> {
    let normalized = out_alias::normalize(tokens, function, contracts);
    let tokens = normalized.as_deref().unwrap_or(tokens);
    let (body_start, line) = find_function_body(tokens, function)
        .ok_or_else(|| LegacyCError::FunctionNotFound(function.to_owned()))?;
    let parameters = function_parameters(tokens, function).unwrap_or_default();
    let formal_parameters = parameters.iter().flatten().cloned().collect();
    let protected_formals =
        protected_resource_formals(tokens, body_start, function, &parameters, contracts);
    let (typed_model, typed_types, typed_bases, store_bases) = typed_access_facts(
        tokens,
        function,
        body_start,
        &parameters,
        &protected_formals,
        contracts,
    );
    let mut parser = Parser {
        tokens,
        cursor: body_start,
        contracts,
        next_site: 0,
        formal_parameters,
        protected_formals,
        typed_model,
        typed_types,
        typed_bases,
        store_bases,
    };
    let statements =
        parser
            .parse_block()
            .map_err(|failure_line| LegacyCError::MalformedFunction {
                function: function.to_owned(),
                line: failure_line.max(line),
            })?;
    Ok(CfgBuilder::build(function, &statements))
}

/// Read the declaration facts the typed direct scalar-field whitelist needs.
///
/// Both the record inventory and the function's variable types come from the
/// same preprocessed token stream the importer is already projecting, so the
/// classification is reproduced exactly by W4 summary derivation and by
/// [`check_legacy_safety_certificate`], which hold only that stream. When the
/// function's declarator list cannot be read, [`FunctionTypes::default`]
/// refuses every access in it and the lexical gate governs unchanged.
fn typed_access_facts(
    tokens: &[Token],
    function: &str,
    body_start: usize,
    parameters: &[Option<String>],
    protected_formals: &BTreeSet<String>,
    contracts: &ResourceContracts,
) -> (
    TypedAccessModel,
    FunctionTypes,
    UntrackedBases,
    AggregateStoreBases,
) {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let model = TypedAccessModel::read(&texts);
    let body = match (
        function_parameter_range(tokens, function, body_start),
        matching(tokens, body_start, "{", "}"),
    ) {
        (Some((open, close)), Some(body_end)) => Some((open, close, body_end)),
        _ => None,
    };
    let Some((open, close, body_end)) = body else {
        return (
            model,
            FunctionTypes::default(),
            UntrackedBases::default(),
            AggregateStoreBases::default(),
        );
    };
    let types = FunctionTypes::read(
        &model,
        &texts[open + 1..close],
        &texts[body_start + 1..body_end],
        protected_formals,
    );
    // O04/C02-P: the admissible access bases of this body, read from the same
    // token stream, so that W4 derivation, W4 validation and certificate
    // rederivation all repeat the classification from the certificate's own
    // content. See [`crate::borrowed_actual`].
    let bases = crate::borrowed_actual::untracked_bases(
        &texts[body_start + 1..body_end],
        protected_formals,
        contracts,
        &model,
        &types,
    );
    // O00 blocker B08: the local aggregate pointers a store may target, read
    // from the same token stream for the same reason.
    let formals = parameters
        .iter()
        .flatten()
        .cloned()
        .collect::<BTreeSet<_>>();
    let stores = crate::aggregate_store::store_bases(
        &texts[body_start + 1..body_end],
        &formals,
        contracts,
        &model,
        &types,
    );
    (model, types, bases, stores)
}

/// The parenthesis indices of the definition whose body starts at `body_start`.
fn function_parameter_range(
    tokens: &[Token],
    function: &str,
    body_start: usize,
) -> Option<(usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let Some(close) = matching(tokens, index + 1, "(", ")") else {
            continue;
        };
        if close + 1 == body_start {
            return Some((index + 1, close));
        }
    }
    None
}

// Discover identities requiring stable bindings before parsing expressions.
// This is only a conservative candidate inventory, never an inferred effect:
// direct *formal writes, forwarded Out calls, and non-borrowed resource calls
// are still checked by CFG re-analysis. Declared modes also protect formals.
fn protected_resource_formals(
    tokens: &[Token],
    body_start: usize,
    function: &str,
    parameters: &[Option<String>],
    contracts: &ResourceContracts,
) -> BTreeSet<String> {
    let formals = parameters
        .iter()
        .flatten()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut protected = BTreeSet::new();
    if let Some(contract) = contracts.contract(function) {
        for (parameter, mode) in parameters.iter().zip(&contract.parameters) {
            if !matches!(mode, ResourceParameterMode::Borrowed)
                && let Some(name) = parameter
            {
                protected.insert(name.clone());
            }
        }
    }
    let mut depth = 0usize;
    let body_end = tokens
        .iter()
        .enumerate()
        .skip(body_start)
        .find_map(|(index, token)| {
            match token.text.as_str() {
                "{" => depth += 1,
                "}" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
            None
        })
        .unwrap_or(tokens.len());
    let body = tokens[body_start..body_end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    for (index, pair) in body.windows(2).enumerate() {
        if pair[0] == "*" && formals.contains(pair[1]) {
            protected.insert(pair[1].to_owned());
        }
        if pair[1] != "(" {
            continue;
        }
        let Some(contract) = contracts.contract(pair[0]) else {
            continue;
        };
        let Some(arguments) = call_arguments(&body, index) else {
            continue;
        };
        for (mode, argument) in contract.parameters.iter().zip(arguments) {
            if matches!(mode, ResourceParameterMode::Borrowed) {
                continue;
            }
            let Some(place) = contract_argument_place(argument, *mode) else {
                continue;
            };
            let name = place.strip_prefix('*').unwrap_or(&place);
            if is_identifier(name) {
                protected.insert(name.to_owned());
            }
        }
    }
    // Propagate protection, never custody, across direct lexical aliases.
    // The closure is deliberately flow-insensitive: mutation before an alias
    // declaration must not be hidden by the later resource call on that alias.
    let mut aliases = Vec::new();
    let mut start = 0usize;
    for (end, token) in body.iter().enumerate() {
        if !matches!(*token, ";" | "{" | "}") {
            continue;
        }
        let statement = &body[start..end];
        start = end + 1;
        let Some(equal) = statement.iter().position(|token| *token == "=") else {
            continue;
        };
        let Some(destination) = direct_assignment_destination(&statement[..equal]) else {
            continue;
        };
        let Some(source) = canonical_place(&statement[equal + 1..]) else {
            continue;
        };
        if is_identifier(&destination) && is_identifier(&source) {
            aliases.push((destination, source));
        }
    }
    loop {
        let before = protected.len();
        for (destination, source) in &aliases {
            if protected.contains(destination) || protected.contains(source) {
                protected.insert(destination.clone());
                protected.insert(source.clone());
            }
        }
        if before == protected.len() {
            break;
        }
    }
    protected
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    text: String,
    line: usize,
}

pub(crate) fn tokenize(source: &str) -> Vec<Token> {
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

/// Declaration specifiers that can end a parameter declarator without naming
/// it. If one of these is the last token, the parameter is unnamed and the body
/// cannot refer to it.
const DECLARATION_SPECIFIERS: &[&str] = &[
    "void",
    "char",
    "short",
    "int",
    "long",
    "float",
    "double",
    "signed",
    "unsigned",
    "_Bool",
    "_Complex",
    "_Imaginary",
    "const",
    "volatile",
    "restrict",
    "__restrict",
    "__restrict__",
    "struct",
    "union",
    "enum",
    "static",
    "register",
    "inline",
    "_Atomic",
];

/// Formal parameter names of a defined function, in declaration order.
///
/// `Some(vec![Some(name), None, ..])` names each formal the body can refer to;
/// an inner `None` is an unnamed formal. The whole result is `None` whenever
/// the declarator list cannot be read *unambiguously* — variadic lists,
/// function-pointer parameters, K&R-style single-identifier declarators,
/// unnamed tagged types, and duplicate names all reject the function outright.
/// Misreading which declarator carries a name would attach custody to the wrong
/// argument, so this parser refuses everything it cannot pin down.
pub(crate) fn function_parameters(tokens: &[Token], function: &str) -> Option<Vec<Option<String>>> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) != Some("{") {
            continue;
        }
        return parse_parameter_declarators(&tokens[index + 2..close]);
    }
    None
}

fn parse_parameter_declarators(tokens: &[Token]) -> Option<Vec<Option<String>>> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    // `(`/`)` mark a function-pointer or nested declarator; `.` is the first
    // token of a variadic `...`. Neither is read here.
    if texts
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "."))
    {
        return None;
    }
    if texts.is_empty() {
        return Some(Vec::new());
    }
    let groups = texts.split(|text| *text == ",").collect::<Vec<_>>();
    if groups.len() == 1 && groups[0] == ["void"] {
        return Some(Vec::new());
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for group in groups {
        let mut group = group;
        // Drop array declarator suffixes: `char buf[16]` names `buf`.
        while group.last() == Some(&"]") {
            let open = group.iter().rposition(|text| *text == "[")?;
            group = &group[..open];
        }
        if group.is_empty() {
            return None;
        }
        let last = *group.last()?;
        if !is_identifier(last) {
            // An abstract declarator such as `const char *`: unnamed.
            names.push(None);
            continue;
        }
        if DECLARATION_SPECIFIERS.contains(&last) {
            names.push(None);
            continue;
        }
        if group.len() < 2 {
            // `f(n)` (K&R) and `f(size_t)` are indistinguishable here.
            return None;
        }
        if matches!(group[group.len() - 2], "struct" | "union" | "enum") {
            // `struct foo` without a declarator: the identifier is the tag.
            names.push(None);
            continue;
        }
        if !seen.insert(last.to_owned()) {
            return None;
        }
        names.push(Some(last.to_owned()));
    }
    Some(names)
}

/// Names in `candidates` that the body of `function` calls directly.
pub(crate) fn direct_calls(
    tokens: &[Token],
    function: &str,
    candidates: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut called = BTreeSet::new();
    let Some((body_start, _)) = find_function_body(tokens, function) else {
        return called;
    };
    let Some(body_end) = matching(tokens, body_start, "{", "}") else {
        return called;
    };
    for index in body_start..body_end {
        if tokens.get(index + 1).map(|token| token.text.as_str()) != Some("(") {
            continue;
        }
        let name = &tokens[index].text;
        if candidates.contains(name) {
            called.insert(name.clone());
        }
    }
    called
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
    formal_parameters: BTreeSet<String>,
    protected_formals: BTreeSet<String>,
    /// Declaration facts of the enclosing translation unit and function, used
    /// only to admit the typed direct scalar-field whitelist of
    /// [`crate::typed_field_access`]. Everything outside that whitelist keeps
    /// the lexical rejection.
    typed_model: TypedAccessModel,
    typed_types: FunctionTypes,
    /// Access bases this function never binds to a resource identity, used only
    /// to admit the borrowed pointer-field actuals of
    /// [`crate::borrowed_actual`]. Everything outside that whitelist keeps the
    /// lexical rejection.
    typed_bases: UntrackedBases,
    /// Local aggregate pointers whose single-selection fields a store may
    /// target (O00 blocker B08). Everything outside that whitelist keeps the
    /// lexical rejection. See [`crate::aggregate_store`].
    store_bases: AggregateStoreBases,
}

impl Parser<'_> {
    /// The null test a branch condition expresses, including the one
    /// aggregate-field form card O04-B08 admits.
    ///
    /// The general refusal of `->` in a condition is unchanged: only a null
    /// test whose *entire* operand is an admitted single-selection **pointer**
    /// field of an admitted aggregate base is read here, and only when the
    /// condition carries no call, no braces and no other unmodeled access.
    fn null_condition(&self, condition: &[Token]) -> Option<PointerNullCondition> {
        if let Some(existing) = pointer_null_condition(condition) {
            return Some(existing);
        }
        if contains_call(condition) || contains_braces(condition) {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        self.aggregate_null_condition(&texts)
    }

    fn aggregate_null_condition(&self, texts: &[&str]) -> Option<PointerNullCondition> {
        let texts = strip_parentheses(texts);
        if let Some(rest) = texts.strip_prefix(&["!"]) {
            let mut condition = self.aggregate_null_condition(rest)?;
            condition.true_is_null = !condition.true_is_null;
            return Some(condition);
        }
        if let Some(place) = self.admitted_field_operand(texts) {
            return Some(PointerNullCondition {
                source: place,
                true_is_null: false,
            });
        }
        for index in 0..texts.len().saturating_sub(1) {
            if !matches!(texts[index], "=" | "!") || texts[index + 1] != "=" {
                continue;
            }
            let left = &texts[..index];
            let right = &texts[index + 2..];
            let source = if is_null_constant(left) {
                self.admitted_field_operand(right)?
            } else if is_null_constant(right) {
                self.admitted_field_operand(left)?
            } else {
                return None;
            };
            return Some(PointerNullCondition {
                source,
                true_is_null: texts[index] == "=",
            });
        }
        None
    }

    /// The place named by an operand that is exactly `BASE - > FIELD` on an
    /// admitted base whose member is a single-star pointer.
    fn admitted_field_operand(&self, texts: &[&str]) -> Option<String> {
        let texts = strip_parentheses(texts);
        let [base, "-", ">", field] = texts else {
            return None;
        };
        let store = crate::aggregate_store::admitted_field(
            base,
            field,
            &self.store_bases,
            &self.typed_model,
            &self.typed_types,
        )?;
        store.pointer_field.then_some(store.place)
    }

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
                    condition_supported: supported_neutral_expression(&condition),
                    null_condition: self.null_condition(&condition),
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
                    condition_supported: supported_neutral_expression(&condition),
                    null_condition: self.null_condition(&condition),
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
                    condition_supported: supported_neutral_expression(&condition),
                    null_condition: self.null_condition(&condition),
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
                    condition_supported: supported_neutral_expression(&condition),
                    null_condition: self.null_condition(&condition),
                    body,
                })
            }
            Some("switch") => {
                self.cursor += 1;
                let expression = self.take_parenthesized()?;
                let body = self.parse_statement_body()?;
                Ok(Statement::Switch {
                    line,
                    expression_supported: supported_neutral_expression(&expression),
                    body,
                })
            }
            Some("case") => {
                self.cursor += 1;
                let expression = self.consume_to_colon()?;
                Ok(Statement::Case {
                    line,
                    expression_supported: supported_neutral_expression(&expression),
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
                    expression_supported: supported_neutral_expression(&expression),
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
        let mut delimiters = Vec::new();
        while self.cursor < self.tokens.len() {
            match self.text() {
                Some("(") => delimiters.push(")"),
                Some("[") => delimiters.push("]"),
                Some("{") => delimiters.push("}"),
                Some(close @ (")" | "]" | "}")) => {
                    if delimiters.pop() != Some(close) {
                        return Err(line);
                    }
                }
                Some(";") if delimiters.is_empty() => {
                    let result = self.tokens[start..self.cursor].to_vec();
                    self.cursor += 1;
                    return Ok(result);
                }
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
        // Retain the enclosing function and subsequent cleanup, but never
        // mistake nested initializers or GNU statement expressions for a
        // single modeled call/assignment. Their custody effects are unknown.
        if contains_braces(tokens) {
            return LegacyInstruction::Unknown {
                description: "braced initializer, expression, or declaration is not modeled".into(),
            };
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let assignment = texts.iter().position(|text| *text == "=");
        let mentions_formal = texts
            .iter()
            .any(|name| self.protected_formals.contains(*name));
        let has_increment = texts
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]));
        let mut expression_depth = 0usize;
        let nested_assignment = texts.iter().any(|text| match *text {
            "(" | "[" => {
                expression_depth += 1;
                false
            }
            ")" | "]" => {
                expression_depth = expression_depth.saturating_sub(1);
                false
            }
            "=" => expression_depth > 0,
            _ => false,
        });
        if mentions_formal && (has_increment || nested_assignment) {
            return LegacyInstruction::Unknown {
                description: "unmodeled expression may mutate a formal resource identity".into(),
            };
        }
        // Resource places currently use names, without lexical binding IDs.
        // A local declaration must not reuse a formal's entry custody or Out
        // slot. Conservatively reject ambiguous declaration prefixes too;
        // initializer references to a formal remain outside this prefix.
        let declaration = &texts[..assignment.unwrap_or(texts.len())];
        let declaration_like = declaration.first().is_some_and(|first| {
            DECLARATION_SPECIFIERS.contains(first)
                || (is_identifier(first)
                    && declaration.get(1).is_some_and(|second| {
                        is_identifier(second)
                            || *second == "*"
                            || (*second == "(" && declaration.get(2) == Some(&"*"))
                    }))
        });
        let mut nesting = 0usize;
        let mut initializer = false;
        let mut may_shadow_formal = false;
        if declaration_like {
            for text in &texts {
                match *text {
                    "(" | "[" => nesting += 1,
                    ")" | "]" => nesting = nesting.saturating_sub(1),
                    "=" if nesting == 0 => initializer = true,
                    "," if nesting == 0 => initializer = false,
                    name if !initializer && self.formal_parameters.contains(name) => {
                        may_shadow_formal = true;
                    }
                    _ => {}
                }
            }
        }
        if may_shadow_formal {
            return LegacyInstruction::Unknown {
                description: "local declaration may shadow a formal resource identity".into(),
            };
        }
        // One typed exception to the lexical indirect-access gate below: a
        // direct scalar-field access whose declared types make it
        // custody-neutral is a no-op for this projection. The whitelist is
        // conjunctive and every one of its premises is checked against
        // declarations read from this same translation unit; see
        // [`crate::typed_field_access`]. It runs after the formal-mutation and
        // shadowing gates, which still govern, and never for a statement
        // carrying a call, whose own rules continue to apply.
        if !contains_call(tokens)
            && crate::typed_field_access::is_scalar_field_noop(
                &texts,
                &self.typed_model,
                &self.typed_types,
            )
        {
            return LegacyInstruction::NoOp;
        }
        // O04/C02-P: the borrowed pointer-field actuals of a contracted call,
        // and an assignment whose whole right-hand side is one admitted direct
        // member read. Both are *reads*; the destination side of the lexical
        // gate below is untouched, which is what keeps such a place from ever
        // being bound. See [`crate::borrowed_actual`].
        let borrowed_actuals = crate::borrowed_actual::borrowed_actual_spans(
            &texts,
            self.contracts,
            &self.typed_model,
            &self.typed_types,
            &self.typed_bases,
        );
        if let Some(equal) = assignment
            && !contains_call(tokens)
            && let Some(destination) = direct_assignment_destination(&texts[..equal])
            && !destination.contains(['.', '[', '*'])
            && !destination.contains("->")
            && crate::borrowed_actual::is_opaque_field_read(
                &texts[equal + 1..],
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
            )
        {
            return LegacyInstruction::OpaqueValue { destination };
        }
        // O00 blocker B08: one destination shape leaves the lexical gate — a
        // single `->` selection naming an admitted member of a local aggregate
        // pointer this body acquired. Every premise is typed and checked
        // against declarations from this same token stream; see
        // [`crate::aggregate_store`]. The *value* side is classified below and
        // the aggregate's liveness is required flow-sensitively by [`transfer`].
        let admitted_store = assignment.and_then(|equal| {
            let store = crate::aggregate_store::admitted_store(
                &texts[..equal],
                &self.store_bases,
                &self.typed_model,
                &self.typed_types,
            )?;
            // The importer's own canonical place must be exactly the admitted
            // one, or the destination this statement writes is not the one the
            // premises were checked against.
            (direct_assignment_destination(&texts[..equal]).as_deref() == Some(&store.place))
                .then_some(store)
        });
        // Explicit Out stores remain a separate modeled instruction. Reading
        // through a pointer, member or subscript has no heap/liveness model and
        // must never disappear into scalar NoOp or borrowed-call argument erasure.
        let unsupported_access = if let Some(equal) = assignment {
            let destination = direct_assignment_destination(&texts[..equal]);
            (admitted_store.is_none()
                && destination.as_deref().is_some_and(|place| {
                    place.contains('.') || place.contains('[') || place.contains("->")
                }))
                || contains_unmodeled_access(&outside_spans(&texts, &borrowed_actuals, equal + 1))
        } else if declaration_like
            && texts
                .first()
                .is_some_and(|first| DECLARATION_SPECIFIERS.contains(first))
        {
            // Declarator stars do not evaluate a pointer. A plain object
            // declaration with constant array bounds and no initializer
            // acquires nothing and evaluates nothing, so it is custody-neutral;
            // every other bracketed or dotted declaration still needs an
            // importer we do not have.
            (texts.contains(&"[") && !constant_array_declaration(&texts)) || texts.contains(&".")
        } else {
            contains_unmodeled_access(&outside_spans(&texts, &borrowed_actuals, 0))
        };
        if unsupported_access {
            return LegacyInstruction::Unknown {
                description: "indirect memory access is outside the resource projection".into(),
            };
        }
        let call = find_call(&texts);

        if let Some((call_index, function)) = call {
            if matches!(
                function,
                "sizeof" | "_Alignof" | "alignof" | "typeof" | "__typeof" | "__typeof__"
            ) {
                return LegacyInstruction::Unknown {
                    description: format!("C operator `{function}` requires type/operand analysis"),
                };
            }
            if let Some(contract) = self.contracts.contract(function).cloned() {
                let mut call_depth = 0usize;
                let call_end =
                    texts
                        .iter()
                        .enumerate()
                        .skip(call_index + 1)
                        .find_map(|(index, text)| {
                            match *text {
                                "(" => call_depth += 1,
                                ")" => {
                                    call_depth = call_depth.saturating_sub(1);
                                    if call_depth == 0 {
                                        return Some(index);
                                    }
                                }
                                _ => {}
                            }
                            None
                        });
                let direct_call = call_index == 0
                    || assignment.is_some_and(|equal| {
                        call_index == equal + 1
                            && direct_assignment_destination(&texts[..equal]).is_some()
                    });
                // O00 blocker B08, two refinements that fire **only** for a
                // destination [`crate::aggregate_store`] admitted, so no other
                // statement in the corpus can move:
                //
                // 1. The aggregate's own name is the modeled destination, not
                //    "a formal embedded in an unmodeled expression". Its
                //    liveness and single identity are required flow-sensitively
                //    by [`transfer`], which is a stronger check than the
                //    syntactic one this gate performs. A protected name in the
                //    *right-hand side* still refuses.
                // 2. One leading parenthesized cast between `=` and the call
                //    does not break the direct flow of the call's result into
                //    the member: it converts the result's declared type, and
                //    this projection tracks identities, not types. The call
                //    must still be the whole remainder of the statement.
                let (mentions_formal, direct_call) = match (&admitted_store, assignment) {
                    (Some(_), Some(equal)) => (
                        texts[equal + 1..]
                            .iter()
                            .any(|name| self.protected_formals.contains(*name)),
                        call_index == equal + 1 || is_leading_cast(&texts[equal + 1..call_index]),
                    ),
                    _ => (mentions_formal, direct_call),
                };
                if mentions_formal && (!direct_call || call_end != Some(texts.len() - 1)) {
                    return LegacyInstruction::Unknown {
                        description:
                            "resource call with a formal embedded in an unmodeled expression".into(),
                    };
                }
                let destination =
                    assignment.and_then(|equal| direct_assignment_destination(&texts[..equal]));
                // A call result carrying custody may only be stored into a
                // member declared with exactly one `*`; a scalar member cannot
                // hold an identity, and no other member shape is admitted.
                if let Some(store) = &admitted_store
                    && !matches!(contract.result, ResourceReturnMode::None)
                    && !store.pointer_field
                {
                    return LegacyInstruction::Unknown {
                        description: "indirect memory access is outside the resource projection"
                            .into(),
                    };
                }
                if !matches!(contract.result, ResourceReturnMode::None) && destination.is_none() {
                    return LegacyInstruction::Unknown {
                        description: format!("discarded owned result of resource API `{function}`"),
                    };
                }
                let raw_arguments = call_arguments(&texts, call_index);
                if raw_arguments.as_ref().is_some_and(|arguments| {
                    contract
                        .parameters
                        .iter()
                        .zip(arguments)
                        .any(|(mode, argument)| {
                            ((!matches!(mode, ResourceParameterMode::Out { .. })
                                && argument.contains(&"&"))
                                || (matches!(mode, ResourceParameterMode::Borrowed)
                                    && canonical_place(argument).is_none()))
                                && argument
                                    .iter()
                                    .any(|name| self.protected_formals.contains(*name))
                        })
                }) {
                    return LegacyInstruction::Unknown {
                        description: "call has an unmodeled formal address or borrowed expression"
                            .into(),
                    };
                }
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

        // O04: a statement over declared scalars, enumeration constants and
        // numeric literals alone is custody-neutral — this projection tracks
        // pointer identities, and a declared scalar is never bound to one. It
        // is consulted only where the existing modeling would otherwise report
        // an unresolved alias or refuse the statement, so every value fact the
        // importer already derives (`flag = 0` binding a definitely-null place,
        // and every variable-to-variable alias) is preserved exactly.
        let scalar_only = !contains_call(tokens)
            && crate::typed_field_access::is_scalar_only_noop(
                &texts,
                &self.typed_model,
                &self.typed_types,
            );
        if let Some(store) = &admitted_store
            && let Some(equal) = assignment
        {
            let right = &texts[equal + 1..];
            let Some(value) = crate::aggregate_store::classify_value(
                store,
                right,
                &self.typed_model,
                &self.typed_types,
                &self.typed_bases,
                is_null_constant,
            ) else {
                // A value shape the premises do not cover keeps the existing
                // refusal and the existing diagnostic.
                return LegacyInstruction::Unknown {
                    description: "indirect memory access is outside the resource projection".into(),
                };
            };
            return match value {
                crate::aggregate_store::StoredValue::Untracked => {
                    LegacyInstruction::AggregateStore {
                        place: store.place.clone(),
                    }
                }
                crate::aggregate_store::StoredValue::Null => LegacyInstruction::Clear {
                    destination: store.place.clone(),
                },
                crate::aggregate_store::StoredValue::Tracked(source) => LegacyInstruction::Alias {
                    destination: store.place.clone(),
                    source,
                },
            };
        }
        if let Some(equal) = assignment {
            let Some(destination) = direct_assignment_destination(&texts[..equal]) else {
                if scalar_only {
                    return LegacyInstruction::NoOp;
                }
                return LegacyInstruction::Unknown {
                    description: "unrecognized assignment destination".to_owned(),
                };
            };
            let right = &texts[equal + 1..];
            if is_null_constant(right) {
                return LegacyInstruction::Clear {
                    destination: destination.to_owned(),
                };
            }
            // The declaration form `int flag = FLAG_OFF;`, whose leading type
            // specifiers the statement walker refuses.
            let scalar_only = scalar_only
                || (!contains_call(tokens)
                    && crate::typed_field_access::is_scalar_initializer(
                        &destination,
                        right,
                        &self.typed_model,
                        &self.typed_types,
                    ));
            if let Some(source) = canonical_place(right) {
                // An enumeration constant names no place this projection can
                // ever bind, so aliasing it only reports an unresolved alias.
                if scalar_only && self.typed_model.is_enum_constant(&source) {
                    return LegacyInstruction::NoOp;
                }
                return LegacyInstruction::Alias {
                    destination: destination.to_owned(),
                    source,
                };
            }
            if scalar_only {
                return LegacyInstruction::NoOp;
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

/// Whether `tokens` is exactly one balanced parenthesized group carrying no
/// call, no assignment and no comma — the shape of a cast applied to the value
/// that follows it. An empty slice is not a cast; nor is anything with a
/// second top-level group.
fn is_leading_cast(tokens: &[&str]) -> bool {
    if tokens.first() != Some(&"(") || tokens.last() != Some(&")") || tokens.len() < 3 {
        return false;
    }
    let mut depth = 0usize;
    for (index, text) in tokens.iter().enumerate() {
        match *text {
            "(" => depth += 1,
            ")" => {
                depth = match depth.checked_sub(1) {
                    Some(depth) => depth,
                    None => return false,
                };
                if depth == 0 && index + 1 != tokens.len() {
                    return false;
                }
            }
            "=" | "," | "[" | "]" | "{" | "}" | ";" => return false,
            _ => {}
        }
    }
    depth == 0 && find_call(tokens).is_none()
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

/// The tokens of `texts[from..]` that no admitted borrowed pointer-field
/// actual covers.
///
/// Every admitted span is a *whole* call argument, so deleting it leaves the
/// surrounding punctuation adjacent (`( ,` or `( )`) and can never manufacture
/// a unary `*` or a member `.` out of neighbours that did not have one.
fn outside_spans<'a>(texts: &[&'a str], spans: &[(usize, usize)], from: usize) -> Vec<&'a str> {
    texts
        .iter()
        .enumerate()
        .skip(from)
        .filter(|(index, _)| {
            !spans
                .iter()
                .any(|(start, end)| index >= start && index < end)
        })
        .map(|(_, text)| *text)
        .collect()
}

/// Whether a declaration statement's every bracket group is an integer constant
/// bound (or an empty `[]`), so that the declaration allocates automatic
/// storage and evaluates nothing. A variable-length bound, a designator or a
/// function declarator is refused.
fn constant_array_declaration(texts: &[&str]) -> bool {
    if texts
        .iter()
        .any(|text| matches!(*text, "(" | ")" | "{" | "}" | "="))
    {
        return false;
    }
    let mut cursor = 0usize;
    while cursor < texts.len() {
        if texts[cursor] != "[" {
            cursor += 1;
            continue;
        }
        let Some(close) = texts.iter().skip(cursor).position(|text| *text == "]") else {
            return false;
        };
        let bound = &texts[cursor + 1..cursor + close];
        if !bound
            .iter()
            .all(|text| text.bytes().all(|byte| byte.is_ascii_alphanumeric()) && is_numeric(text))
        {
            return false;
        }
        cursor += close + 1;
    }
    true
}

fn is_numeric(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_digit)
}

/// This is a lexical exclusion boundary, not a proof of C memory safety.
/// Only multiplication with a plain left operand is identifiable here; a star
/// after a closing parenthesis may follow a typedef cast and remains unknown.
fn contains_unmodeled_access(texts: &[&str]) -> bool {
    texts.windows(2).any(|pair| pair == ["-", ">"])
        || texts.iter().enumerate().any(|(index, text)| match *text {
            "[" | "]" => true,
            "." => {
                !texts
                    .get(index.wrapping_sub(1))
                    .is_some_and(|left| left.bytes().all(|byte| byte.is_ascii_digit()))
                    || !texts
                        .get(index + 1)
                        .is_some_and(|right| right.bytes().all(|byte| byte.is_ascii_digit()))
            }
            "*" => !texts.get(index.wrapping_sub(1)).is_some_and(|left| {
                (is_identifier(left) || left.bytes().all(|byte| byte.is_ascii_digit()))
                    && !matches!(
                        *left,
                        "sizeof"
                            | "_Alignof"
                            | "alignof"
                            | "__extension__"
                            | "__real__"
                            | "__real"
                            | "__imag__"
                            | "__imag"
                    )
            }),
            _ => false,
        })
}

fn supported_neutral_expression(tokens: &[Token]) -> bool {
    !contains_call(tokens)
        && !contains_braces(tokens)
        && !contains_unmodeled_access(
            &tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
        )
}

fn contains_braces(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.text.as_str(), "{" | "}"))
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
    if !supported_neutral_expression(tokens) {
        return None;
    }
    null_condition_text(&texts)
}

fn null_condition_text(texts: &[&str]) -> Option<PointerNullCondition> {
    let texts = strip_parentheses(texts);
    if let Some(rest) = texts.strip_prefix(&["!"]) {
        let mut condition = null_condition_text(rest)?;
        condition.true_is_null = !condition.true_is_null;
        return Some(condition);
    }
    if let Some(source) = canonical_place(texts) {
        return Some(PointerNullCondition {
            source,
            true_is_null: false,
        });
    }
    let mut depth = 0usize;
    for index in 0..texts.len().saturating_sub(1) {
        match texts[index] {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" | "!" if depth == 0 && texts[index + 1] == "=" => {
                let left = &texts[..index];
                let right = &texts[index + 2..];
                let source = if is_null_constant(left) {
                    canonical_place(right)?
                } else if is_null_constant(right) {
                    canonical_place(left)?
                } else {
                    return None;
                };
                return Some(PointerNullCondition {
                    source,
                    true_is_null: texts[index] == "=",
                });
            }
            _ => {}
        }
    }
    None
}

// Only erase parentheses that enclose the complete expression. In particular,
// `(p) + offset` must not become the resource place `p`.
fn strip_parentheses<'a>(mut tokens: &'a [&'a str]) -> &'a [&'a str] {
    while tokens.first() == Some(&"(") && tokens.last() == Some(&")") {
        let mut depth = 0usize;
        let mut closes_at_end = false;
        for (index, token) in tokens.iter().enumerate() {
            match *token {
                "(" => depth += 1,
                ")" => {
                    let Some(next) = depth.checked_sub(1) else {
                        return tokens;
                    };
                    depth = next;
                    if depth == 0 {
                        closes_at_end = index == tokens.len() - 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            break;
        }
        tokens = &tokens[1..tokens.len() - 1];
    }
    tokens
}

fn is_null_constant(tokens: &[&str]) -> bool {
    let tokens = strip_parentheses(tokens);
    matches!(tokens, ["NULL" | "nullptr" | "0"])
        || tokens
            .strip_prefix(&["(", "void", "*", ")"])
            .is_some_and(|rest| strip_parentheses(rest) == ["0"])
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
        // Retain direct borrowed names for the enclosing Out-slot identity
        // guard; the callee's borrowed resource effect still consumes nothing.
        ResourceParameterMode::Borrowed => canonical_place(argument),
        ResourceParameterMode::Out { .. } => {
            let argument = strip_parentheses(argument);
            if argument.first() == Some(&"&") {
                canonical_place(&argument[1..])
            } else {
                // A bare pointer may forward the current function's out slot.
                // Naming *formal is not permission to write it: transfer must
                // find that exact slot sanctioned and its pointer unchanged.
                match argument {
                    [name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
                        Some(format!("*{name}"))
                    }
                    _ => None,
                }
            }
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
        // A single dereference of a plain identifier is the only indirect
        // destination this importer names: `*out = …` becomes the place
        // `*out`. It is *not* modeled here — [`transfer`] rejects it unless the
        // analysis in force sanctions `*out` as an out-parameter slot — but
        // naming it is what lets an out-parameter contract describe the store.
        // Anything deeper (`*p->field`, `**p`, `*(p + 1)`) stays unnamed.
        return match tokens {
            ["*", name] if is_identifier(name) && !matches!(*name, "NULL" | "nullptr") => {
                Some(format!("*{name}"))
            }
            _ => None,
        };
    }
    (0..tokens.len()).find_map(|start| canonical_place(&tokens[start..]))
}

/// Whether `place` denotes a store through a pointer rather than a variable.
/// Such a place is modeled only when the contract in force declares it an
/// out-parameter slot; see [`EntryCustody::out_slots`].
fn is_indirect_place(place: &str) -> bool {
    place.starts_with('*')
}

fn canonical_place(tokens: &[&str]) -> Option<String> {
    let tokens = strip_parentheses(tokens);
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
                        description:
                            "unmodeled call, aggregate or indirect access in return expression"
                                .to_owned(),
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
                        description:
                            "unmodeled call, aggregate or indirect access in branch condition"
                                .to_owned(),
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
                        description:
                            "unmodeled call, aggregate or indirect access in loop condition"
                                .to_owned(),
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
                    "unmodeled call, aggregate or indirect access in for-loop condition",
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
                    "unmodeled call, aggregate or indirect access in do-while condition",
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
                        description:
                            "unmodeled call, aggregate or indirect access in switch expression"
                                .to_owned(),
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
                        description:
                            "unmodeled call, aggregate or indirect access in case expression"
                                .to_owned(),
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
    /// O00 blocker B08: `owner site -> field name -> child sites` recorded by
    /// an admitted aggregate field store. Deliberately separate from
    /// `contained`: escaping the owner escapes its live linked children, while
    /// *releasing* the owner does not cascade, because a plain C field store
    /// carries no declared destructor obligation (premise P-B08-SHALLOW).
    field_links: BTreeMap<usize, BTreeMap<String, BTreeSet<usize>>>,
    /// Custody retained after a rejected effect; its residual is not a proven
    /// leak because the unmodeled action may have discharged it.
    unresolved_effect_sites: BTreeSet<usize>,
    widened: bool,
    /// Whether some instruction on *this* path was refused or reported an
    /// unresolved effect (O04). It is the per-path counterpart of the
    /// function-wide `Unknown` status: a path carrying it can support no
    /// custody claim at the exits it reaches. Joined by disjunction, so it is
    /// monotone and the work list still terminates.
    unmodeled: bool,
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
        self.unresolved_effect_sites
            .extend(incoming.unresolved_effect_sites.iter().copied());
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
        self.join_field_links(incoming);
        self.widened |= incoming.widened;
        self.unmodeled |= incoming.unmodeled;
        *self != before
    }

    /// Join the B08 field-link relation.
    ///
    /// The union alone would be unsound in the escaping direction: a child
    /// linked on one path only would be carried away by an escape of the
    /// aggregate on a path that never stored it, hiding a leak. Every child in
    /// the symmetric difference is therefore recorded as an unresolved effect,
    /// which makes its disposition `Mixed` and the exit `Unknown`. The union
    /// and `unresolved_effect_sites` both only grow, so the work list still
    /// terminates.
    fn join_field_links(&mut self, incoming: &Self) {
        let owners = self
            .field_links
            .keys()
            .chain(incoming.field_links.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        for owner in owners {
            let mine = self.field_links.get(&owner).cloned().unwrap_or_default();
            let theirs = incoming
                .field_links
                .get(&owner)
                .cloned()
                .unwrap_or_default();
            let fields = mine
                .keys()
                .chain(theirs.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut joined: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
            for field in fields {
                let left = mine.get(&field).cloned().unwrap_or_default();
                let right = theirs.get(&field).cloned().unwrap_or_default();
                if left != right {
                    self.unresolved_effect_sites
                        .extend(left.symmetric_difference(&right).copied());
                }
                let union = left.union(&right).copied().collect::<BTreeSet<_>>();
                if !union.is_empty() {
                    joined.insert(field, union);
                }
            }
            if joined.is_empty() {
                self.field_links.remove(&owner);
            } else {
                self.field_links.insert(owner, joined);
            }
        }
    }
}

/// Custody the caller hands to a function's formal parameters, derived from the
/// contract in force for that function.
///
/// Parameter resources are *caller-owned*: they are seeded live at entry and
/// exempt from exit-leak reporting, because discharging them is the caller's
/// obligation, not the callee's. Their exit disposition is what the summary
/// claims about the call, and [`crate::legacy_summary`] checks that claim.
#[derive(Clone, Debug, Default)]
pub(crate) struct EntryCustody {
    state: AbstractState,
    /// Abstract site bound to each formal parameter, when the contract gives
    /// that parameter observable custody. `None` for `Borrowed`/`Out` modes.
    pub(crate) parameter_sites: Vec<Option<usize>>,
    /// Place a formal declared `Out` writes through, as `*name`. Nothing is
    /// bound at entry — the callee *initializes* the slot — but naming it is
    /// what makes `*name = …` a modeled store instead of an unmodeled one.
    pub(crate) out_slots: Vec<Option<String>>,
    /// Whether the contract's result may be null. A body may return a
    /// possibly-null place only when its own contract says the caller has to
    /// null-check the result; otherwise the escape reports an unresolved alias.
    pub(crate) result_nullable: bool,
}

impl EntryCustody {
    /// Build the entry state a contract prescribes, or `None` when the
    /// parameter list and the contract cannot be lined up exactly.
    pub(crate) fn build(
        cfg: &LegacyCfg,
        parameters: &[Option<String>],
        contract: &ResourceFunctionContract,
    ) -> Option<Self> {
        if contract.parameters.len() != parameters.len() {
            return None;
        }
        let base = first_free_site(cfg);
        let mut custody = Self {
            state: AbstractState::default(),
            parameter_sites: vec![None; parameters.len()],
            out_slots: vec![None; parameters.len()],
            result_nullable: match contract.result {
                ResourceReturnMode::None => false,
                ResourceReturnMode::Owned { nullable }
                | ResourceReturnMode::Reallocated { nullable, .. } => nullable,
            },
        };
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            let nullable = match mode {
                // Nothing is bound: the body may not treat the formal as a
                // resource it already owns.
                ResourceParameterMode::Borrowed => continue,
                // An `Out` formal is *written* by the callee. Nothing is bound
                // at entry either, but the slot it writes through is named so
                // the store can be modeled and observed at every exit.
                ResourceParameterMode::Out { .. } => {
                    let name = parameters[index].as_ref()?;
                    custody.out_slots[index] = Some(format!("*{name}"));
                    continue;
                }
                ResourceParameterMode::Owned { nullable }
                | ResourceParameterMode::Consumed { nullable }
                | ResourceParameterMode::Released { nullable }
                | ResourceParameterMode::Stored { nullable, .. }
                | ResourceParameterMode::Removed { nullable, .. } => nullable,
            };
            let name = parameters[index].as_ref()?;
            let site = base.checked_add(index)?;
            custody.state.resources.insert(site, LIVE);
            custody.state.pointers.insert(
                name.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([site]),
                    unknown: false,
                    may_be_null: nullable,
                },
            );
            if nullable {
                custody.state.nullable_sites.insert(site);
            }
            custody.parameter_sites[index] = Some(site);
        }
        // A Removed formal is supplied inside its owner's aggregate, not in
        // direct custody. Bind every formal first so argument order cannot
        // change the owner identity. Unsupported owner shapes fail closed.
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            if let ResourceParameterMode::Removed {
                owner_parameter, ..
            } = mode
            {
                if index == owner_parameter
                    || !matches!(
                        contract.parameters.get(owner_parameter),
                        Some(ResourceParameterMode::Owned { nullable: false })
                    )
                {
                    return None;
                }
                let child = custody.parameter_sites[index]?;
                let owner = custody
                    .parameter_sites
                    .get(owner_parameter)
                    .copied()
                    .flatten()?;
                custody.state.resources.insert(child, CONTAINED);
                custody
                    .state
                    .contained
                    .entry(owner)
                    .or_default()
                    .insert(child);
            }
        }
        Some(custody)
    }

    /// Sites the caller — not this body — is responsible for discharging at a
    /// given exit: every formal the contract bound, plus whatever each out
    /// slot holds there, because an `Out` contract hands that resource back.
    ///
    /// The out part is read from the exit state rather than fixed up front, so
    /// a resource overwritten in the slot (`*out = p; *out = q;`) loses its
    /// exemption and is reported as the leak it is.
    fn exempt_sites(&self, state: &AbstractState) -> BTreeSet<usize> {
        let mut sites = self
            .parameter_sites
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        for place in self.out_slots.iter().flatten() {
            if let Some(pointer) = state.pointers.get(place) {
                sites.extend(pointer.targets.iter().copied());
            }
        }
        sites
    }

    /// Places a store may legitimately target in this analysis.
    fn sanctioned_places(&self) -> BTreeSet<&str> {
        self.out_slots
            .iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }
}

/// The first abstract site that no acquisition in `cfg` already uses.
fn first_free_site(cfg: &LegacyCfg) -> usize {
    cfg.nodes
        .iter()
        .flat_map(|node| match &node.instruction {
            LegacyInstruction::Acquire { site, .. } => vec![*site],
            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),
            _ => Vec::new(),
        })
        .max()
        .map_or(0, |site| site.saturating_add(1))
}

/// What one reachable exit of a function does to the custody it was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExitObservation {
    /// Disposition of each bound formal parameter site at this exit.
    pub(crate) parameters: Vec<Option<SiteDisposition>>,
    pub(crate) returned: ReturnedValue,
    /// What each declared out slot holds at this exit.
    pub(crate) out_slots: Vec<Option<OutSlotValue>>,
    /// Containment whose *owner* is a site the caller holds — a formal, or a
    /// resource handed back through an out slot. A summary may only claim such
    /// an attachment through an explicit `Stored` mode; anything else here is
    /// a transfer invisible at the call site, and refutes the summary.
    pub(crate) caller_containment: BTreeMap<usize, BTreeSet<usize>>,
    /// Reallocation replacements that this exit left unresolved.
    pub(crate) pending_replacements: BTreeMap<usize, BTreeSet<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteDisposition {
    Live,
    Released,
    Escaped,
    Contained,
    /// The site left the state on this path (a refuted nullable acquisition).
    Absent,
    /// Several dispositions joined; no single custody claim describes it.
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReturnedValue {
    /// The exit returns no modeled resource place (including a null constant).
    None,
    Site {
        site: usize,
        nullable: bool,
        disposition: SiteDisposition,
    },
    /// The returned place denotes more than one site, or an unknown one.
    Ambiguous,
}

/// What an out slot holds at one exit. `Unwritten` is deliberately distinct
/// from `Null`: a slot the callee never stored to leaves the caller's variable
/// as it was, which no `Out` mode describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutSlotValue {
    /// The slot was never stored to on this path.
    Unwritten,
    /// The slot definitely holds a null pointer.
    Null,
    Site {
        site: usize,
        nullable: bool,
        disposition: SiteDisposition,
    },
    /// The slot denotes more than one site, or an unknown one.
    Ambiguous,
}

fn observe_out_slot(state: &AbstractState, place: &str) -> OutSlotValue {
    match state.pointers.get(place) {
        None => OutSlotValue::Unwritten,
        Some(pointer) if pointer.unknown || pointer.targets.len() > 1 => OutSlotValue::Ambiguous,
        Some(pointer) => match pointer.targets.first().copied() {
            None => OutSlotValue::Null,
            Some(site) => OutSlotValue::Site {
                site,
                nullable: pointer.may_be_null || state.nullable_sites.contains(&site),
                disposition: site_disposition(state, site),
            },
        },
    }
}

fn site_disposition(state: &AbstractState, site: usize) -> SiteDisposition {
    if state.unresolved_effect_sites.contains(&site) {
        return SiteDisposition::Mixed;
    }
    match state.resources.get(&site).copied() {
        None => SiteDisposition::Absent,
        Some(LIVE) => SiteDisposition::Live,
        Some(RELEASED) => SiteDisposition::Released,
        Some(ESCAPED) => SiteDisposition::Escaped,
        Some(CONTAINED) => SiteDisposition::Contained,
        Some(_) => SiteDisposition::Mixed,
    }
}

/// Containment a caller could observe: the declared `Stored` relation, plus
/// every B08 field link. A field link whose owner the caller holds is a
/// transfer invisible at the call site, so it must reach
/// [`crate::legacy_summary`]'s observation check and refute any summary that
/// does not declare it. The store admission already refuses a caller-held
/// aggregate, so this is defence in depth rather than a reachable case.
fn caller_visible_containment(state: &AbstractState) -> BTreeMap<usize, BTreeSet<usize>> {
    let mut containment = state.contained.clone();
    for (owner, fields) in &state.field_links {
        let children = containment.entry(*owner).or_default();
        for linked in fields.values() {
            children.extend(linked.iter().copied());
        }
    }
    containment
}

fn observe_exit(
    node: &LegacyCfgNode,
    state: &AbstractState,
    entry: &EntryCustody,
) -> ExitObservation {
    let returned = match &node.instruction {
        LegacyInstruction::EscapeIfOwned { source } | LegacyInstruction::Escape { source } => {
            match state.pointers.get(source) {
                None => ReturnedValue::None,
                Some(pointer) if pointer.unknown || pointer.targets.len() > 1 => {
                    ReturnedValue::Ambiguous
                }
                Some(pointer) => match pointer.targets.first().copied() {
                    // A definitely-null place returns no resource.
                    None => ReturnedValue::None,
                    Some(site) => ReturnedValue::Site {
                        site,
                        nullable: pointer.may_be_null || state.nullable_sites.contains(&site),
                        disposition: site_disposition(state, site),
                    },
                },
            }
        }
        _ => ReturnedValue::None,
    };
    let exempt = entry.exempt_sites(state);
    ExitObservation {
        parameters: entry
            .parameter_sites
            .iter()
            .map(|site| site.map(|site| site_disposition(state, site)))
            .collect(),
        returned,
        out_slots: entry
            .out_slots
            .iter()
            .map(|place| place.as_deref().map(|place| observe_out_slot(state, place)))
            .collect(),
        caller_containment: caller_visible_containment(state)
            .iter()
            .filter(|(owner, children)| exempt.contains(owner) && !children.is_empty())
            .map(|(owner, children)| (*owner, children.clone()))
            .collect(),
        pending_replacements: state.conditional_replacements.clone(),
    }
}

fn classify_diagnostics(diagnostics: &[LegacyDiagnostic]) -> AnalysisStatus {
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
            LegacyDiagnosticKind::UnknownAlias { .. }
                | LegacyDiagnosticKind::Unsupported { .. }
                | LegacyDiagnosticKind::SummaryConflict { .. }
        )
    });
    if unsafe_found {
        AnalysisStatus::Unsafe
    } else if unknown_found {
        AnalysisStatus::Unknown
    } else {
        AnalysisStatus::Safe
    }
}

/// The parts of the contract in force that the transfer function consults.
///
/// Both fields default to the most conservative setting, which is what the
/// contract-free analysis uses: no store through a pointer is modeled, and a
/// returned place may not be null. A contract only ever *adds* permissions
/// here, and every permission it adds is re-checked at the exits.
#[derive(Default)]
struct TransferContext<'a> {
    /// Out-parameter slots the contract in force declares. A store to any
    /// other indirect place stays unmodeled.
    sanctioned: BTreeSet<&'a str>,
    /// Whether the contract's result mode obliges callers to null-check.
    result_nullable: bool,
    /// Sites the *caller* holds at entry. An aggregate store never targets one
    /// of these: a transfer into a caller-owned aggregate is invisible at the
    /// call site and is refused rather than modeled (O00 blocker B08).
    caller_sites: BTreeSet<usize>,
}

fn analyze_cfg(cfg: LegacyCfg) -> LegacyFunctionSummary {
    analyze_cfg_with_entry(cfg, &EntryCustody::default()).0
}

pub(crate) fn analyze_cfg_with_entry(
    cfg: LegacyCfg,
    entry_custody: &EntryCustody,
) -> (LegacyFunctionSummary, Vec<ExitObservation>) {
    let context = TransferContext {
        sanctioned: entry_custody.sanctioned_places(),
        result_nullable: entry_custody.result_nullable,
        caller_sites: entry_custody
            .parameter_sites
            .iter()
            .flatten()
            .copied()
            .collect(),
    };
    let mut incoming = vec![None::<AbstractState>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(entry_custody.state.clone());
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
        let mut local = BTreeSet::new();
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &context,
            &mut local,
        );
        // A path that reached a refusal supports no per-exit custody claim.
        // Recorded before the local diagnostics are merged, so the flag tracks
        // *this* node rather than anything reported elsewhere in the function.
        output.unmodeled |= local.iter().any(is_path_poisoning);
        diagnostics.append(&mut local);
        if node.is_exit {
            let exempt = entry_custody.exempt_sites(&output);
            report_leaks(
                &cfg.function,
                node.line,
                &output,
                &acquisition_lines,
                &exempt,
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
                        edge_output.unmodeled = true;
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

    // Read the settled exit states once the work list is stable, so that a
    // summary describes the joined behavior of every reachable return rather
    // than an intermediate iterate. Diagnostics are already recorded above;
    // this pass only observes.
    let mut observations = Vec::new();
    let mut exits = Vec::new();
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        if !node.is_exit {
            continue;
        }
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let mut ignored = BTreeSet::new();
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &context,
            &mut ignored,
        );
        output.unmodeled |= ignored.iter().any(is_path_poisoning);
        observations.push(observe_exit(node, &output, entry_custody));
        exits.push(observe_residual(
            node_id,
            node,
            &output,
            &acquisition_lines,
            entry_custody,
        ));
    }

    let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    let status = classify_diagnostics(&diagnostics);
    (
        LegacyFunctionSummary {
            function: cfg.function.clone(),
            status,
            diagnostics,
            fixed_point_steps: steps,
            cfg,
            exits,
        },
        observations,
    )
}

/// Whether a diagnostic means the path that produced it can support no custody
/// claim. `Leak` is deliberately absent: it is reported *at* an exit by
/// [`report_leaks`], and it is exactly the residual this ledger describes
/// rather than a reason to refuse describing it.
fn is_path_poisoning(diagnostic: &LegacyDiagnostic) -> bool {
    matches!(
        diagnostic.kind,
        LegacyDiagnosticKind::Unsupported { .. }
            | LegacyDiagnosticKind::UnknownAlias { .. }
            | LegacyDiagnosticKind::SummaryConflict { .. }
            | LegacyDiagnosticKind::DoubleRelease { .. }
            | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
            | LegacyDiagnosticKind::CustodyConflict { .. }
    )
}

/// Read one exit's custody residual out of its settled state.
fn observe_residual(
    node_id: LegacyNodeId,
    node: &LegacyCfgNode,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    entry: &EntryCustody,
) -> LegacyExitResidual {
    let exempt = entry.exempt_sites(state);
    let sites = state
        .resources
        .keys()
        .copied()
        .filter(|site| !exempt.contains(site))
        .map(|site| LegacyResidualSite {
            site,
            acquisition_line: acquisition_line(acquisition_lines, site),
            disposition: site_disposition(state, site),
        })
        .collect::<Vec<_>>();
    let indeterminate = state.unmodeled
        || state.widened
        || sites
            .iter()
            .any(|site| site.disposition == SiteDisposition::Mixed);
    let custody = if indeterminate {
        ExitCustody::Unknown
    } else if sites
        .iter()
        .any(|site| site.disposition == SiteDisposition::Live)
    {
        ExitCustody::Residual
    } else {
        ExitCustody::Discharged
    };
    LegacyExitResidual {
        node: node_id,
        line: node.line,
        custody,
        sites,
    }
}

/// Report an indirect store the contract in force does not sanction, and
/// poison the state exactly as an unmodeled construct would. The importer
/// *names* `*out` so an out-parameter contract can describe it; without such a
/// contract the store is as opaque as it was before out slots existed.
fn reject_unsanctioned_store(
    function: &str,
    line: usize,
    place: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("store through `{place}` without an out-parameter contract"),
        },
    });
    for pointer in state.pointers.values_mut() {
        pointer.unknown = true;
        pointer.may_be_null = true;
    }
}

fn transfer(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    // A slot name denotes the caller's original pointer only while that
    // formal is unchanged. Tracking its reassignment would need pointer-to-
    // pointer alias analysis; reject it rather than writing a fictitious slot.
    let destination = match &node.instruction {
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::OpaqueValue { destination }
        | LegacyInstruction::Clear { destination } => Some(destination.as_str()),
        LegacyInstruction::ContractCall { destination, .. } => destination.as_deref(),
        _ => None,
    };
    let changes_slot_identity = |place: &str| {
        context
            .sanctioned
            .iter()
            .any(|slot| slot.strip_prefix('*') == Some(place))
    };
    let overwritten = destination.is_some_and(changes_slot_identity)
        || match &node.instruction {
            LegacyInstruction::ContractCall {
                arguments,
                contract,
                ..
            } => contract
                .parameters
                .iter()
                .zip(arguments)
                .any(|(mode, argument)| {
                    matches!(mode, ResourceParameterMode::Out { .. })
                        && argument.as_deref().is_some_and(changes_slot_identity)
                }),
            _ => false,
        };
    if overwritten {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: "reassignment of an out-parameter pointer".to_owned(),
            },
        });
        for pointer in state.pointers.values_mut() {
            pointer.unknown = true;
            pointer.may_be_null = true;
        }
        return state;
    }
    // A store through a pointer is modeled only where the contract in force
    // declares that slot an out parameter. `ContractCall` handles its own
    // destination below, because a call whose result carries no custody never
    // touched the slot in the first place.
    if let LegacyInstruction::Alias {
        destination,
        source: _,
    }
    | LegacyInstruction::Clear { destination } = &node.instruction
        && is_indirect_place(destination)
        && !context.sanctioned.contains(destination.as_str())
    {
        reject_unsanctioned_store(function, node.line, destination, &mut state, diagnostics);
        return state;
    }
    // Forwarded Out arguments denote indirect writes just like *out = value.
    // A local alias or an unbound pointer must never gain a fabricated slot.
    if let LegacyInstruction::ContractCall {
        arguments,
        contract,
        ..
    } = &node.instruction
    {
        for (mode, argument) in contract.parameters.iter().zip(arguments) {
            if !matches!(mode, ResourceParameterMode::Out { .. })
                && argument.as_deref().is_some_and(changes_slot_identity)
            {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: "out-slot pointer passed without a modeled Out effect".into(),
                    },
                });
                for pointer in state.pointers.values_mut() {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
                return state;
            }
            if matches!(mode, ResourceParameterMode::Out { .. })
                && let Some(place) = argument
                && is_indirect_place(place)
                && !context.sanctioned.contains(place.as_str())
            {
                reject_unsanctioned_store(function, node.line, place, &mut state, diagnostics);
                return state;
            }
        }
    }
    // O00 blocker B08: every store into an admitted aggregate field requires
    // its aggregate to resolve, here and now, to exactly one live identity
    // *this* body acquired. A released, escaped, refuted, ambiguous, nullable
    // or caller-held aggregate refuses and poisons the path; this is the C00
    // premise in its store direction, checked flow-sensitively rather than
    // syntactically.
    let stored_field = match written_field_place(&node.instruction) {
        None => None,
        Some(place) => {
            let owner = crate::aggregate_store::place_owner(place).unwrap_or_default();
            match resolved_aggregate_owner(&state, owner, &context.caller_sites) {
                Some(site) => {
                    let field = crate::aggregate_store::place_field(place).unwrap_or_default();
                    Some((site, field.to_owned(), place.to_owned()))
                }
                None => {
                    diagnostics.insert(LegacyDiagnostic {
                        function: function.to_owned(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::UnknownAlias {
                            variable: owner.to_owned(),
                        },
                    });
                    for pointer in state.pointers.values_mut() {
                        pointer.unknown = true;
                        pointer.may_be_null = true;
                    }
                    return state;
                }
            }
        }
    };
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
                    // Returning a possibly-null place is an unresolved alias
                    // unless this function's own result mode already obliges
                    // its callers to null-check what they get back.
                    context.result_nullable,
                );
            }
        }
        LegacyInstruction::ContractCall {
            function: called,
            destination,
            arguments,
            contract,
            produced_sites,
        } => {
            let unsanctioned = destination
                .as_deref()
                .filter(|place| is_indirect_place(place) && !context.sanctioned.contains(place));
            match unsanctioned {
                // A result that carries no custody never wrote the slot, so
                // the call's parameter effects still apply and the unmodeled
                // destination is simply ignored — the pre-out-slot behavior.
                Some(_) if matches!(contract.result, ResourceReturnMode::None) => {
                    apply_contract_call(
                        function,
                        node.line,
                        called,
                        None,
                        arguments,
                        contract,
                        produced_sites,
                        &mut state,
                        acquisition_lines,
                        diagnostics,
                    );
                }
                // Custody would be created in a place this analysis cannot
                // account for: refuse the whole call.
                Some(_) => {
                    diagnostics.insert(LegacyDiagnostic {
                        function: function.to_owned(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::Unsupported {
                            description: format!(
                                "discarded owned result of resource API `{called}`"
                            ),
                        },
                    });
                    for pointer in state.pointers.values_mut() {
                        pointer.unknown = true;
                        pointer.may_be_null = true;
                    }
                }
                None => apply_contract_call(
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
            }
        }
        LegacyInstruction::OpaqueValue { destination } => {
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                    may_be_null: true,
                },
            );
        }
        LegacyInstruction::AggregateStore { place } => {
            // The stored value carries no tracked identity, so the member binds
            // nothing. Whatever it previously linked is unlinked below, which
            // is what makes an overwritten resource visible as a residual.
            state.pointers.remove(place);
        }
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
    if let Some((owner_site, field, place)) = stored_field {
        let children = state
            .pointers
            .get(&place)
            .map(|pointer| pointer.targets.clone())
            .unwrap_or_default();
        let entry = state.field_links.entry(owner_site).or_default();
        if children.is_empty() {
            entry.remove(&field);
        } else {
            entry.insert(field, children);
        }
        if entry.is_empty() {
            state.field_links.remove(&owner_site);
        }
    }
    state
}

/// The admitted aggregate field place an instruction stores into, if any.
fn written_field_place(instruction: &LegacyInstruction) -> Option<&str> {
    let place = match instruction {
        LegacyInstruction::AggregateStore { place } => place.as_str(),
        LegacyInstruction::Alias { destination, .. } | LegacyInstruction::Clear { destination } => {
            destination.as_str()
        }
        LegacyInstruction::ContractCall { destination, .. } => destination.as_deref()?,
        _ => return None,
    };
    crate::aggregate_store::is_field_place(place).then_some(place)
}

/// The single live identity an admitted aggregate base denotes here, when the
/// projection can name exactly one and this body acquired it.
fn resolved_aggregate_owner(
    state: &AbstractState,
    owner: &str,
    caller_sites: &BTreeSet<usize>,
) -> Option<usize> {
    let pointer = state.pointers.get(owner)?;
    if pointer.unknown || pointer.may_be_null || pointer.targets.len() != 1 {
        return None;
    }
    let site = pointer.targets.first().copied()?;
    if caller_sites.contains(&site)
        || state.nullable_sites.contains(&site)
        || state.unresolved_effect_sites.contains(&site)
    {
        return None;
    }
    (state.resources.get(&site) == Some(&LIVE)).then_some(site)
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
    let unresolved = pointer.unknown
        || pointer.targets.len() > 1
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null));
    if unresolved {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    let unresolved_nullability = !nullable
        && pointer
            .targets
            .iter()
            .any(|site| state.nullable_sites.contains(site));
    if unresolved_nullability {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!("nullable resource `{source}` used by a non-null contract"),
            },
        });
    }
    if unresolved || unresolved_nullability {
        // One source action cannot discharge every possible selected target.
        // Preserve the residual obligations when target identity is unknown.
        mark_unresolved_effect(state, &pointer);
        return;
    }
    if !cascade_is_unambiguous(state, &pointer.targets) {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        mark_unresolved_effect(state, &pointer);
        return;
    }
    for site in pointer.targets {
        let status = state.resources.get(&site).copied().unwrap_or(0);
        if escape && status != LIVE {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
            continue;
        }
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

fn mark_unresolved_effect(state: &mut AbstractState, pointer: &AbstractPointer) {
    let mut pending = if pointer.unknown {
        state.resources.keys().copied().collect::<BTreeSet<_>>()
    } else {
        pointer.targets.clone()
    };
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if visited.insert(site) {
            state.unresolved_effect_sites.insert(site);
            if let Some(children) = state.contained.get(&site) {
                pending.extend(children);
            }
            if let Some(fields) = state.field_links.get(&site) {
                pending.extend(fields.values().flatten().copied());
            }
        }
    }
}

fn cascade_is_unambiguous(state: &AbstractState, roots: &BTreeSet<usize>) -> bool {
    let mut pending = roots.clone();
    let mut visited = BTreeSet::new();
    while let Some(owner) = pending.pop_first() {
        if !visited.insert(owner) {
            return false;
        }
        for child in state.contained.get(&owner).into_iter().flatten() {
            if state.resources.get(child) != Some(&CONTAINED)
                || state
                    .contained
                    .iter()
                    .any(|(other, children)| *other != owner && children.contains(child))
            {
                return false;
            }
            pending.insert(*child);
        }
    }
    true
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
    if escape {
        cascade_field_links_on_escape(state, owner);
    }
}

/// B08: publishing an aggregate publishes the resources its admitted members
/// link, so a live linked child escapes with its owner rather than being
/// reported as a leak. A child in any other state is left exactly as it is and
/// recorded as an unresolved effect: a member still pointing at a released
/// identity when its aggregate escapes is a hazard this projection will not
/// decide in either direction.
///
/// **Releasing** an owner deliberately does not reach this function
/// (premise P-B08-SHALLOW): a declared `resource_released` release names one
/// identity, and a plain C field store carries no declared destructor
/// obligation, so a child still live there stays live and is reported.
fn cascade_field_links_on_escape(state: &mut AbstractState, owner: usize) {
    let mut pending = state
        .field_links
        .remove(&owner)
        .into_iter()
        .flat_map(|fields| fields.into_values().flatten())
        .collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if !visited.insert(site) {
            continue;
        }
        if let Some(fields) = state.field_links.remove(&site) {
            pending.extend(fields.into_values().flatten());
        }
        if let Some(children) = state.contained.remove(&site) {
            pending.extend(children);
        }
        match state.resources.get(&site).copied() {
            Some(LIVE) => {
                state.resources.insert(site, ESCAPED);
            }
            Some(ESCAPED) | None => {}
            Some(_) => {
                unresolved.insert(site);
            }
        }
    }
    state.unresolved_effect_sites.extend(unresolved);
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
    // Reallocation consumes at most one proven live original on success.
    // A null old value carries no custody only when its input contract permits it.
    // Validate it before any other parameter effect or result allocation.
    if let ResourceReturnMode::Reallocated { parameter, .. } = contract.result {
        let input_nullable = matches!(
            contract.parameters.get(parameter),
            Some(ResourceParameterMode::Owned { nullable: true })
        );
        let source = arguments.get(parameter).and_then(Option::as_deref);
        let old = source
            .and_then(|source| state.pointers.get(source))
            .cloned();
        let valid = old.as_ref().is_some_and(|old| {
            !old.unknown
                && old.targets.len() <= 1
                && (input_nullable || (old.targets.len() == 1 && !old.may_be_null))
                && old.targets.iter().all(|site| {
                    state.resources.get(site) == Some(&LIVE)
                        && (input_nullable || !state.nullable_sites.contains(site))
                        && !state.unresolved_effect_sites.contains(site)
                })
        });
        if !valid {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::UnknownAlias {
                    variable: source.unwrap_or(called).to_owned(),
                },
            });
            mark_unresolved_effect(
                state,
                &old.unwrap_or(AbstractPointer {
                    unknown: true,
                    ..Default::default()
                }),
            );
            return;
        }
    }
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
        ResourceReturnMode::None => {
            // The result carries no custody, so the destination holds no
            // resource afterwards. Dropping the binding — rather than leaving
            // the previous one in place — keeps a stale alias from covering for
            // a resource the assignment actually dropped.
            if let Some(destination) = destination {
                state.pointers.remove(destination);
            }
        }
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
            let input_nullable = matches!(
                contract.parameters.get(parameter),
                Some(ResourceParameterMode::Owned { nullable: true })
            );
            if old.unknown || old.targets.len() > 1 || (old.targets.is_empty() && !input_nullable) {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                mark_unresolved_effect(state, &old);
                return;
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
    // Acquisition sites may repeat in loops; retain prior uncertainty rather
    // than assuming this abstract site is a fresh runtime generation.
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
    let owner_live = require_live_resource(function, line, owner, false, state, diagnostics);
    let child_live = require_live_resource(function, line, source, nullable, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        return;
    };
    if !owner_live || !child_live {
        mark_unresolved_effect(state, &children);
        return;
    }
    if owners.unknown || children.unknown || owners.targets.len() != 1 || children.targets.len() > 1
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        mark_unresolved_effect(state, &children);
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
    let owner_live = require_live_resource(function, line, owner, false, state, diagnostics);
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
    if !owner_live
        || owners.unknown
        || children.unknown
        || owners.targets.len() != 1
        // A call detaches one child, not every possible target of an alias
        // joined from different paths. Their correlation is not represented.
        || children.targets.len() > 1
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
        let containing_owners = state
            .contained
            .iter()
            .filter_map(|(owner, children)| children.contains(&child).then_some(*owner))
            .collect::<BTreeSet<_>>();
        let status = state.resources.get(&child).copied().unwrap_or(0);
        // Every represented path must still own this exact child through the
        // owner. A joined CONTAINED|LIVE state includes an already-detached
        // path; removing the CONTAINED bit would erase its repeated removal.
        // The union of two different possible owners is not proof that the
        // requested owner contains the child on every represented path.
        if containing_owners != BTreeSet::from([owner_site]) || status != CONTAINED {
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
) -> bool {
    let Some(pointer) = state.pointers.get(source) else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return false;
    };
    let invalid = pointer.unknown
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null))
        || (!nullable
            && pointer
                .targets
                .iter()
                .any(|site| state.nullable_sites.contains(site)))
        || pointer.targets.iter().any(|site| {
            state.resources.get(site) != Some(&LIVE) || state.unresolved_effect_sites.contains(site)
        });
    if invalid {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    !invalid
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
        // The null alternative has no child at this site. Remove precisely
        // its incoming aggregate edges, retaining every sibling obligation.
        state.contained.retain(|_, children| {
            children.remove(&site);
            !children.is_empty()
        });
        state.field_links.retain(|_, fields| {
            fields.retain(|_, children| {
                children.remove(&site);
                !children.is_empty()
            });
            !fields.is_empty()
        });
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
    // Caller-owned parameter resources are discharged by the caller; a live
    // formal at exit is the interface, not a leak.
    exempt: &BTreeSet<usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (&site, &status) in &state.resources {
        if status & LIVE != 0
            && !exempt.contains(&site)
            && !state.unresolved_effect_sites.contains(&site)
        {
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
