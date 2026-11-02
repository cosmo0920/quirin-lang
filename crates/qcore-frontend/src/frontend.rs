//! Cycle 10 complete source-front-end skeleton.
//!
//! The earlier cycles intentionally exposed their checked IRs as Rust APIs and
//! parsed only dimensions and quantitative axes.  This module is the common
//! source boundary: it recognizes every declaration and statement family in
//! the revision 0.1 grammar, retains byte spans, resolves bindings, and
//! classifies the resulting nodes for the existing semantic passes.  The
//! revision 0.2 concurrency forms are parsed as well, but are deliberately not
//! certified by the sequential Cycle 10 elaborator.

#[path = "frontend_tasks.rs"]
mod tasks;
pub use tasks::{
    ResolvedTaskCapture, ResolvedTaskExpression, ResolvedTaskExpressionKind, ResolvedTaskPlace,
    SourceCaptureMode, SourceClosure, SourceIndexExpression, SourceLeaseBound, SourcePlace,
    SourceShareExpression, SourceTaskCapture, SourceTaskExpression, SourceTaskExpressionKind,
};

#[path = "frontend_result_match.rs"]
mod result_match;
pub use result_match::{
    ResolvedResultMatch, ResolvedResultMatchArm, ResolvedResultMatchScrutinee, ResultPatternKind,
    SourceResultMatch, SourceResultMatchArm, SourceResultPattern,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::lexer::{LexError, Span, Symbol, Token, TokenKind, lex};
use qcore_core::usage::Usage;

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the head word of a named
/// premise clause (`requires resource_premise(NAME, "STATEMENT");`, contract
/// specification C04.18), checked by the contract format in every module.
pub const PREMISE_CLAUSE_WORD: &str = "resource_premise";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<T> {
    pub span: Span,
    pub value: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclarationKind {
    ConcurrencyProfile,
    VerifierScope,
    Dimension,
    Axis,
    TypeAlias,
    /// `type NAME { FIELD: TYPE, … }` — a source-level record type (card
    /// `AGGREGATES-V1`, `source.record.v1`).
    ///
    /// **The head is the already-reserved `type`, so this construct reserves no
    /// word at all.** It is told from [`Self::TypeAlias`] by the one token after
    /// the name — a `{` rather than the `=` an alias requires — and the
    /// constraint-14 census found that three-token shape in **zero** files: all
    /// eleven `type NAME {` occurrences in the tree are `resource type NAME {`,
    /// which the declaration reader claims before it reaches the `type` branch.
    ///
    /// `record` was measured and **rejected** as a spelling: an archived probe
    /// fixture is literally the one line `record R { }` and two archived reports
    /// pin it to a `Q000` refusal, so claiming the word at declaration position
    /// would have falsified a frozen archive's own table (standing constraint
    /// 21) — the `buffer`, `stream` and `len` collision shape exactly.
    Record,
    /// `struct! { type NAME = c"struct TAG" from "HEADER"; fields { … } }` —
    /// an imported C struct (card `W48-QR-C-STRUCT-IMPORT`, `source.ffi.v1`).
    ///
    /// The head is the two tokens `struct` `!` at declaration position. Neither
    /// is a reserved word and none becomes one: `struct` was **census-clean and
    /// deliberately not taken** by `AGGREGATES-V1` (constraint 14), and it is
    /// still not taken — only the sequence `struct !`, which was a parse error
    /// ("expected a QCore declaration") before this card and occurs in no
    /// tracked file (`docs/reports/w48-qr-c-struct-import/census_ffi_spelling.py`).
    /// The declaration's body is read by `crate::source_ffi`; name resolution
    /// never walks declaration tokens, so `c`, `from` and `fields` stay
    /// ordinary identifiers everywhere.
    ImportedStruct,
    ResourceType,
    ErrorType,
    Constant,
    ExternFunction,
    Function,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDeclaration {
    pub kind: DeclarationKind,
    pub name: String,
    pub parameters: Vec<SourceParameter>,
    pub result_type: Option<SourceResultType>,
    pub quant_index_types: Vec<crate::quant_index::SourceQuantIndexType>,
    pub body: Option<SourceBlock>,
    pub function_contract: Option<SourceFunctionContract>,
    pub verifier_scope: Option<VerifierScopeDeclaration>,
    pub tokens: Vec<TokenKind>,
    /// Card `W48-QR-C-STRUCT-IMPORT` (user decision 4): the span of a
    /// `[| quant #c_ffi |]` attribute written immediately before this
    /// declaration. The attribute is not one of [`Self::tokens`] — the
    /// declaration still starts at its own first word — so every reader that
    /// reconstructs a declaration from its tokens reads it exactly as before.
    pub c_ffi: Option<Span>,
}

/// A source-level function contract retained as structured syntax.  Semantic
/// passes can consume these clauses incrementally without reparsing declaration
/// text, while corpus tests can pin the contract surface independently of the
/// executable subset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFunctionContract {
    pub span: Span,
    pub clauses: Vec<Located<SourceContractClause>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceContractClause {
    pub kind: SourceContractClauseKind,
    pub expression_span: Span,
    pub expression_source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContractClauseKind {
    Requires,
    Ensures(SourceContractExitSelector),
    Quant(SourceContractExitSelector),
    /// Card `QUANT-INDEX-CHECKS`: `quant checks P, Idx #i: PREDICATE;`, a
    /// quantified index **check** collocated with the pointer formal `P` it
    /// constrains. It is not an exit selector and carries none: the clause is
    /// a check of the body and an obligation of its callers, never a
    /// postcondition. Only the bounds contract v1 consumer reads it
    /// (`qcore_contract::bounds_contract`); every other consumer rejects or
    /// skips it exactly as it does a `quant` clause. Before this card the
    /// spelling was a parse error (`checks` is no exit selector).
    QuantCheck,
    /// Typed, explicitly parameterized first-order hypothesis; no truth admission.
    QuantProposition,
    /// Explicit entry prerequisite or exit assertion; never implicit evidence.
    QuantCondition(qcore_schemas::quant_condition::Selector),
    /// Checked theorem application syntax; proof and source bindings stay open.
    QuantTheorem,
    /// Conditional own-operation model relation; never an installed call summary.
    QuantEffect(SourceContractExitSelector),
    /// Card `NULL-SAFETY-PROPERTY` (USER DECISION 2026-10-10): `quant
    /// null-deref #B;`, a collocated **check** that the function dereferences
    /// the pointer formal `B`, which must therefore be non-null. Like
    /// [`Self::QuantCheck`] it carries no exit selector: a check of the body
    /// and an obligation of its callers. Only the bounds contract v1 consumer
    /// reads it (`qcore_contract::bounds_contract`); every other consumer
    /// rejects or skips it as it does a `quant` clause. Before this card the
    /// spelling was a parse error (`null` is no exit selector).
    QuantNullDeref,
    /// Card `NULL-DEREF-QTT-GROUNDING` (USER DECISION 2026-10-11, "Two
    /// levels"): `quant deref #B;`, the composite shorthand of a safe
    /// dereference of the pointer formal `B`. The bounds contract v1 loader
    /// elaborates it into its canonical parts (`quant null-deref #B;`, `quant
    /// checks B, Idx #i: #i == 0;`, `quant live #B;`) before any consumer
    /// sees it; every other consumer rejects it. Before this card the spelling
    /// was a parse error (`deref` is no exit selector).
    QuantDeref,
    /// Card `NULL-DEREF-QTT-GROUNDING`: `quant live #B;`, the custody
    /// liveness part of a dereference: the identity behind `B` has not been
    /// released at the use. Admitted by the bounds contract v1 loader as a
    /// checked restatement of the function's own resource v1 declaration;
    /// every other consumer rejects it. Before this card a parse error.
    QuantLive,
    /// Card `OUTER-CONTRACT` (USER DECISIONS 2026-10-17, D1 "B: loader
    /// envelope", D2 "Yes, as proposed"): `quant #outer …;`, a clause of an
    /// outer contract v1 declaration (`requires outer_contract_v1;`): the
    /// contract header `quant #outer [[contract]] NAME, "STATEMENT";`, the
    /// runtime condition `quant #outer where #P in #loaded [[runtime]];` or an
    /// envelope line `quant #outer #loaded: TAG.FIELD |-> #kept;`. The
    /// expression starts at `#outer`. Only the outer contract v1 loader
    /// (`qcore_contract::outer_contract`) reads it; every other consumer
    /// rejects it. Before this card the spelling was a parse error (`#` is no
    /// exit selector).
    QuantOuter,
    /// A read-only contract-local Boolean binding, `quant #C = (EXPR);`.
    QuantConditionBinding,
    /// A scoped conditional prerequisite/protocol block. Inner clauses retain
    /// their own kinds and spans; no local fact is a global assumption.
    QuantConditionBlock {
        condition: String,
        negated: bool,
        clauses: Vec<Located<SourceContractClause>>,
    },
    /// A declared lifetime footprint at a record type, checked by its reader.
    QuantRegistered,
    /// Card MK-EVENT-CELL-CROSS-FUNCTION (USER DECISION 2026-11-02, "B as
    /// recommended"; grammar G05i, gateway rule 38c): `quant cell #B:
    /// TAG.CELL |-> #fresh(descriptor) when #B.GUARD OP V;`, a declared
    /// descriptor-cell protocol at a record type. Read only by the S3
    /// lifetime loader; every other consumer skips or rejects it.
    QuantCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContractExitSelector {
    Normal,
    Error(String),
    Any,
}

/// A source-level selection of verifier providers. The scope is declarative:
/// it grants no authority and is resolved against the registry before a
/// provider is invoked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierScopeDeclaration {
    pub providers: Vec<VerifierProviderUse>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierProviderUse {
    pub span: Span,
    pub plugin_id: String,
    pub version: Option<String>,
    pub profile: String,
    pub capabilities: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTypeKind {
    Atomic,
    Named,
    Tuple,
    Array,
    Slice,
    Pointer,
    Owned,
    Borrow,
    Quant,
    AccessQuant,
    Result,
    Function,
    Never,
    Task,
    Mutex,
}

/// Exact declared result syntax. Retention is structural; only the explicitly
/// recognized executable profile below is admitted by the source driver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceResultType {
    pub kind: SourceTypeKind,
    pub span: Span,
    pub source: String,
    pub tokens: Vec<TokenKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableResultType {
    I32,
    Unit,
}

impl SourceResultType {
    pub fn executable_type(&self) -> Option<ExecutableResultType> {
        match self.tokens.as_slice() {
            [TokenKind::Word(name)] if name == "i32" => Some(ExecutableResultType::I32),
            [TokenKind::Word(name)] if name == "unit" => Some(ExecutableResultType::Unit),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceParameter {
    pub name: Located<String>,
    pub type_kind: SourceTypeKind,
    pub type_span: Span,
    pub type_source: String,
    pub usage: Option<Located<Usage>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceBlock {
    pub span: Span,
    pub statements: Vec<Located<StatementKind>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Let {
        binding: Option<String>,
    },
    LetIndex {
        binding: String,
    },
    Assignment {
        place: Option<String>,
    },
    Expression,
    Drop {
        place: Option<String>,
    },
    Move {
        place: Option<String>,
    },
    Discharge {
        binding: Option<String>,
    },
    Guard,
    /// `if (COND) { … } [ else … ]`, with `else if` flattened: `conditions`
    /// carries one span per `if` header in the chain, and `branches` carries
    /// either `conditions.len()` blocks (no trailing `else`) or one more.
    ///
    /// The condition spans are recorded by card `IF-STATEMENT`; before it the
    /// parser located the header's parentheses and discarded everything between
    /// them, so no pass could read a condition at all. The span is the extent of
    /// the tokens *inside* the parentheses, or of the parentheses themselves
    /// when the condition is empty. Recording a span rather than a parsed
    /// condition keeps the frontend structural: the semantic route reads its own
    /// statement out of a byte-span token window, exactly as every other route
    /// does.
    If {
        conditions: Vec<Span>,
        branches: Vec<SourceBlock>,
    },
    Match {
        arms: Vec<SourceBlock>,
    },
    While {
        body: SourceBlock,
    },
    /// `for NAME in 0..BOUND { … }` — the checked bounded counted loop (card
    /// `LOOP-V1`, `source.loop.v1`).
    ///
    /// The parser records only the name the header **binds**, because that is
    /// the one thing name resolution cannot recover from the token window: the
    /// index is a definition in the body's scope, not a reference to an outer
    /// one. Everything else in the header — the bound, and whether it is a
    /// constant this entry bound — is read out of the token window by
    /// `crate::source_loop`, exactly as every other route reads its own
    /// statements. This variant exists for scoping, not for semantics.
    For {
        binding: String,
        body: SourceBlock,
    },
    /// `[| #unroll, where #bound < N |]` — one statement-adjacent clause list,
    /// licensing the `for` statement that follows it (card
    /// `LOOP-UNROLL-BUDGET`).
    ///
    /// It is its **own statement** rather than a field of [`Self::For`], and
    /// both halves of that are deliberate. A separate statement keeps the
    /// `for` statement's own token window unchanged, so every position
    /// `LOOP-V1` reads by index — the binder at 1, the contextual `in` at 2 —
    /// still holds; and it gives the reader a place to refuse an attribute that
    /// licenses nothing, which is what a dangling attribute and an attribute
    /// before some other statement are.
    ///
    /// The parser records no clause at all: the clause list is read out of the
    /// statement's own byte-span token window by `crate::source_loop`, exactly
    /// as every other route reads its own statements. This variant exists for
    /// *position*, not for semantics.
    LoopAttribute,
    Return,
    Raise,
    Break,
    Continue,
    Defer,
    /// `error region NAME { … } catch (E) [quant open NAME as B] { … } …`
    ///
    /// `blocks[0]` is the region body and `blocks[1..]` are the catch handlers
    /// in written order. `catch_bindings` is parallel to `blocks[1..]`: it
    /// carries the cleanup-capability name a clause's `quant open … as B`
    /// introduces, and `None` for a clause that opens no pack.
    ///
    /// The binding list is recorded here on card `STD-EVENT`'s and card
    /// `LOOP-V1`'s stated rule — the parser records the names a statement
    /// **binds**, because that is the one thing name resolution cannot recover
    /// from a token window, and `discharge B;` inside the handler is an
    /// ordinary reference that must resolve. Everything else about the clause
    /// (which error it catches, which region it opens) is read out of the
    /// statement's own token window by [`crate::source_region`], exactly as
    /// every other route reads its own statements.
    ErrorRegion {
        name: String,
        blocks: Vec<SourceBlock>,
        catch_bindings: Vec<Option<String>>,
    },
    Unsafe {
        body: SourceBlock,
    },
    ThreadScope {
        name: String,
        body: SourceBlock,
    },
    Footprint {
        name: String,
    },
    WithLock {
        guard: String,
        body: SourceBlock,
    },
    /// A `source.event.v1` statement (card `STD-EVENT`).
    ///
    /// The parser records only the names the statement **binds**, because that
    /// is the one thing name resolution cannot recover from the token window:
    /// `endpoints a b;` introduces two endpoint names and `watch e a read;`
    /// introduces one registration name while *referencing* an endpoint. Every
    /// other word in the statement is either a reserved profile keyword or a
    /// reference, and both are handled by the existing machinery.
    ///
    /// The statement's meaning is not decided here. It is read out of the token
    /// window by `crate::source_event`, exactly as every other route reads its
    /// own statements — this variant exists for scoping, not for semantics.
    Event {
        bindings: Vec<String>,
    },
}

impl StatementKind {
    pub fn nested_blocks(&self) -> Vec<&SourceBlock> {
        match self {
            Self::If { branches, .. } | Self::Match { arms: branches } => branches.iter().collect(),
            Self::While { body }
            | Self::For { body, .. }
            | Self::Unsafe { body }
            | Self::ThreadScope { body, .. }
            | Self::WithLock { body, .. } => vec![body],
            Self::ErrorRegion { blocks, .. } => blocks.iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// File/module compilation intent. Non-executable modes check declarations and
/// supported bodies without selecting or synthesizing a process entry point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceModuleMode {
    #[default]
    Executable,
    Library,
    Contract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgram {
    pub mode: SourceModuleMode,
    pub mode_span: Option<Span>,
    imports: Vec<Located<String>>,
    library_bindings: Vec<Located<String>>,
    /// Card `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`: every `alias #NAME =
    /// @C_NAME;` of a contract module, in source order.
    aliases: Vec<Located<SourceAlias>>,
    source: String,
    lexical_tokens: Vec<Token>,
    task_expressions: Vec<SourceTaskExpression>,
    result_matches: Vec<SourceResultMatch>,
    tokens: Vec<TokenKind>,
    pipe_refusals: Vec<crate::pipe::PipeRefusal>,
    /// Card `W48-QR-CLOSURES`: every closure spawn target, in source order.
    closures: Vec<SourceClosure>,
    pub declarations: Vec<Located<SourceDeclaration>>,
}

impl SourceProgram {
    /// Explicit, file-local bundled module imports with their full source spans.
    pub fn imports(&self) -> &[Located<String>] {
        &self.imports
    }

    /// Explicit, file-local `extern library NAME;` declarations with their full
    /// source spans (card `NATIVE-LIBRARY-SOURCE-BINDING`).
    ///
    /// The value is the library name **as this file spelled it**, never
    /// normalised: it is matched against a selected manifest's own `library`
    /// field exactly, on constraint 11's ground — two spellings that normalise
    /// together are two different selections.
    ///
    /// Import-adjacent rather than a declaration: the form is read here, like an
    /// import, precisely because it grants nothing and declares nothing nameable.
    /// It states which *build* this file's extern surface is resolved against, so
    /// it belongs beside the other file-local trust acts and not among the
    /// declarations name resolution walks.
    pub fn library_bindings(&self) -> &[Located<String>] {
        &self.library_bindings
    }

    /// Card `SUPPLIED-OWNED-RETURN-CODE-HANDOFF` (USER DECISION 2026-10-15,
    /// "aliases are allowed"): the module's `alias #NAME = @C_NAME;`
    /// declarations, in source order, each name declared once. Module-local:
    /// the contract loader resolves a clause's `#NAME` against this list
    /// alone, so an alias never reaches another module.
    pub fn aliases(&self) -> &[Located<SourceAlias>] {
        &self.aliases
    }

    /// Whether `span` lies inside one of this file's `extern library NAME;`
    /// declarations.
    ///
    /// Card `NATIVE-LIBRARY-SOURCE-BINDING`'s constraint-14 census outcome. Two
    /// route-selection readers scan the **whole** token stream rather than
    /// statement spans inside a function body — `concurrency_span` and
    /// `source_tasks::has_source_tasks` — and a library named `spawn`, `join`,
    /// `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write` or
    /// `qcore_error_payload` would otherwise select a route from its own name.
    /// A binding's tokens are not a use of anything, so they are excluded, the
    /// same treatment `concurrency_span` already gives a ghost field inside a
    /// declared contract.
    #[must_use]
    pub fn in_library_binding(&self, span: Span) -> bool {
        self.library_bindings
            .iter()
            .any(|binding| binding.span.start <= span.start && span.end <= binding.span.end)
    }

    /// Full match syntax with optional exact builtin Result-pattern recognition.
    pub fn result_matches(&self) -> &[SourceResultMatch] {
        &self.result_matches
    }

    /// Parsed task syntax only; executable checking remains a separate gate.
    pub fn task_expressions(&self) -> &[SourceTaskExpression] {
        &self.task_expressions
    }

    /// Every closure spawn target, in source order (card `W48-QR-CLOSURES`).
    /// Each is also a hidden function declaration named
    /// [`SourceClosure::hidden`].
    pub fn closures(&self) -> &[SourceClosure] {
        &self.closures
    }

    /// The closure whose hidden worker is named `name`, if any.
    pub fn closure(&self, name: &str) -> Option<&SourceClosure> {
        self.closures.iter().find(|closure| closure.hidden == name)
    }

    /// Whether a token at `token` belongs to the header or body of a closure
    /// that the byte window `window` contains without lying inside it (card
    /// `W48-QR-CLOSURES`). Such tokens are the hidden worker's, so a reader of
    /// the enclosing statement's window skips them and reads what the
    /// hand-written twin's statement holds: `spawn SCOPE |CAPTURES|`.
    pub fn outside_closure_window(&self, token: Span, window: Span) -> bool {
        outside_closure_window(self.closures.iter(), token, window)
    }

    /// Where an expression ending at `end` ends as the window `window` reads
    /// it (card `W48-QR-CLOSURES`): a closure spawn's expression ends at the
    /// body's closing brace, but a window that skips the closure's header and
    /// body sees it end at the capture list's closing bar — where its twin's
    /// ends, at the `)`. Every other end is itself.
    pub fn visible_end(&self, window: Span, end: usize) -> usize {
        self.closures
            .iter()
            .find(|closure| {
                closure.lifted.end == end
                    && outside_closure_window(std::iter::once(*closure), closure.lifted, window)
            })
            .map_or(end, |closure| closure.captures_span.end)
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn token_kinds(&self) -> &[TokenKind] {
        &self.tokens
    }

    pub fn lexical_tokens(&self) -> &[Token] {
        &self.lexical_tokens
    }

    /// Pipelines this file's parse refused to desugar (card `PIPE-OPERATOR`).
    /// Raised as diagnostics by `crate::pipe::check_source_operators`, which
    /// the driver runs per input file before name/type checking.
    pub fn pipe_refusals(&self) -> &[crate::pipe::PipeRefusal] {
        &self.pipe_refusals
    }

    pub fn verifier_scopes(&self) -> impl Iterator<Item = (&str, &VerifierScopeDeclaration)> {
        self.declarations.iter().filter_map(|declaration| {
            declaration
                .value
                .verifier_scope
                .as_ref()
                .map(|scope| (declaration.value.name.as_str(), scope))
        })
    }

    /// Round trips compare lexical structure rather than byte offsets.  This
    /// permits callers to format or indent a program without invalidating the
    /// source locations from the original parse.
    pub fn syntax_equivalent(&self, other: &Self) -> bool {
        self.mode == other.mode
            && self.tokens == other.tokens
            && self
                .declarations
                .iter()
                .map(|decl| (decl.value.kind, decl.value.name.as_str()))
                .eq(other
                    .declarations
                    .iter()
                    .map(|decl| (decl.value.kind, decl.value.name.as_str())))
    }
}

impl fmt::Display for SourceProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.source.trim())?;
        if !self.source.trim().is_empty() {
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Card `SUPPLIED-OWNED-RETURN-CODE-HANDOFF` (USER DECISION 2026-10-15): one
/// `alias #NAME = @C_NAME;` of a contract module. `name` is the alias without
/// its `#`, `target` the C name without its `@`; both are single words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAlias {
    pub name: String,
    pub target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceParseError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for SourceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SourceParseError {}

impl From<LexError> for SourceParseError {
    fn from(error: LexError) -> Self {
        Self {
            span: error.span,
            message: error.message,
        }
    }
}

/// Structural metadata for a source-bundle loader. This is deliberately not a
/// SourceProgram: unresolved runtime imports cannot reach semantic consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceImportInventory {
    pub mode: SourceModuleMode,
    pub mode_span: Option<Span>,
    pub runtime_imports: Vec<Located<String>>,
}

pub fn inspect_source_imports(source: &str) -> Result<SourceImportInventory, SourceParseError> {
    let (program, runtime_imports) = SourceParser::new(source)?.parse()?;
    Ok(SourceImportInventory {
        mode: program.mode,
        mode_span: program.mode_span,
        runtime_imports,
    })
}

pub fn parse_source_program(source: &str) -> Result<SourceProgram, SourceParseError> {
    let (program, runtime_imports) = SourceParser::new(source)?.parse()?;
    if let Some(import) = runtime_imports.first() {
        return Err(SourceParseError {
            span: import.span,
            message: "unresolved runtime #import requires source-bundle resolution".into(),
        });
    }
    Ok(program)
}

/// Read the module intent with the same header grammar as the full parser.
/// This does not validate declarations or elaborate contract inclusions.
pub fn parse_source_module_mode(source: &str) -> Result<SourceModuleMode, SourceParseError> {
    SourceParser::new(source)?
        .module_header()
        .map(|(mode, _, _)| mode)
}

struct SourceParser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    matching: BTreeMap<usize, usize>,
    pipe_refusals: Vec<crate::pipe::PipeRefusal>,
    /// Card `W48-QR-CLOSURES`: the lifted closures, keyed by the index of
    /// their `spawn` keyword; filled before the task inventory is read.
    closures: BTreeMap<usize, tasks::LiftedClosure>,
}

/// [`SourceProgram::outside_closure_window`] over any closure list.
fn outside_closure_window<'c>(
    mut closures: impl Iterator<Item = &'c SourceClosure>,
    token: Span,
    window: Span,
) -> bool {
    let within = |outer: Span, inner: Span| outer.start <= inner.start && inner.end <= outer.end;
    closures.any(|closure| within(closure.lifted, token) && !within(closure.span, window))
}

impl<'a> SourceParser<'a> {
    fn new(source: &'a str) -> Result<Self, SourceParseError> {
        let mut tokens = lex(source)?;
        tokens.pop();
        // Card `PIPE-OPERATOR`. The desugaring runs here — after lexing, before
        // the delimiter map and before any parsing — so a synthesized `(`/`)`
        // participates in matching exactly as a written one does, and every
        // later phase sees an ordinary statement. It is total: an ill-formed
        // pipeline is left untouched and refused by `check_source_operators`.
        let (tokens, pipe_refusals) = crate::pipe::desugar_pipes(tokens);
        let mut stack = Vec::<(Symbol, usize)>::new();
        let mut matching = BTreeMap::new();
        for (index, token) in tokens.iter().enumerate() {
            let symbol = match token.kind {
                TokenKind::Symbol(symbol) => symbol,
                _ => continue,
            };
            match symbol {
                Symbol::LeftParen | Symbol::LeftBrace | Symbol::LeftBracket => {
                    stack.push((symbol, index));
                }
                Symbol::RightParen | Symbol::RightBrace | Symbol::RightBracket => {
                    let Some((open, start)) = stack.pop() else {
                        return Err(Self::error_at(token.span, "unmatched closing delimiter"));
                    };
                    if !matches!(
                        (open, symbol),
                        (Symbol::LeftParen, Symbol::RightParen)
                            | (Symbol::LeftBrace, Symbol::RightBrace)
                            | (Symbol::LeftBracket, Symbol::RightBracket)
                    ) {
                        return Err(Self::error_at(token.span, "mismatched closing delimiter"));
                    }
                    matching.insert(start, index);
                    matching.insert(index, start);
                }
                _ => {}
            }
        }
        if let Some((_, start)) = stack.pop() {
            return Err(Self::error_at(
                tokens[start].span,
                "unclosed source delimiter",
            ));
        }
        Ok(Self {
            source,
            tokens,
            matching,
            pipe_refusals,
            closures: BTreeMap::new(),
        })
    }

    fn module_header(&self) -> Result<(SourceModuleMode, Option<Span>, usize), SourceParseError> {
        let mut cursor = 0;
        let mut mode = SourceModuleMode::Executable;
        let mut mode_span = None;
        if self.word(0, "module") {
            mode = if self.word(1, "contract") {
                SourceModuleMode::Contract
            } else if self.word(1, "library") {
                SourceModuleMode::Library
            } else if self.word(1, "executable") {
                SourceModuleMode::Executable
            } else {
                return Err(self.error(1, "expected module contract, library, or executable"));
            };
            if self.tokens.get(2).map(|token| &token.kind)
                != Some(&TokenKind::Symbol(Symbol::Semicolon))
            {
                return Err(self.error(2, "expected semicolon after module mode"));
            }
            mode_span = Some(self.range_span(0, 3));
            cursor = 3;
        }
        Ok((mode, mode_span, cursor))
    }

    /// See [`SourceProgram::outside_closure_window`].
    fn outside_closure_window(&self, token: Span, window: Span) -> bool {
        outside_closure_window(
            self.closures.values().map(|lifted| &lifted.closure),
            token,
            window,
        )
    }

    fn parse(mut self) -> Result<(SourceProgram, Vec<Located<String>>), SourceParseError> {
        let mut declarations = Vec::new();
        let mut imports = Vec::new();
        let mut runtime_imports = Vec::new();
        let mut library_bindings = Vec::new();
        let mut aliases: Vec<Located<SourceAlias>> = Vec::new();
        let (mode, mode_span, mut cursor) = self.module_header()?;
        while cursor < self.tokens.len() {
            let directive = self.symbol(cursor, Symbol::Hash) && self.word(cursor + 1, "import");
            if directive || self.word(cursor, "import") {
                let start = cursor;
                cursor += if directive { 2 } else { 1 };
                if directive
                    && let Some(TokenKind::StringLiteral(path)) =
                        self.tokens.get(cursor).map(|token| &token.kind)
                {
                    // Runtime paths have no escape language. In particular a backslash
                    // cannot be interpreted differently by the lexer and host filesystem.
                    if path.is_empty() || path.chars().any(|ch| ch == '\\' || ch.is_control()) {
                        return Err(self.error(cursor, "runtime import path must be nonempty and contain no escapes or control characters"));
                    }
                    let path = path.clone();
                    cursor += 1;
                    if !self.symbol(cursor, Symbol::Semicolon) {
                        return Err(self.error(cursor, "expected semicolon after runtime import"));
                    }
                    cursor += 1;
                    runtime_imports.push(Located {
                        span: self.range_span(start, cursor),
                        value: path,
                    });
                    continue;
                }
                let mut path = self.identifier(cursor)?.to_owned();
                cursor += 1;
                while self.tokens.get(cursor).map(|token| &token.kind)
                    == Some(&TokenKind::Symbol(Symbol::Dot))
                {
                    cursor += 1;
                    path.push('.');
                    path.push_str(self.identifier(cursor)?);
                    cursor += 1;
                }
                if self.tokens.get(cursor).map(|token| &token.kind)
                    != Some(&TokenKind::Symbol(Symbol::Semicolon))
                {
                    return Err(self.error(cursor, "expected semicolon after module import"));
                }
                cursor += 1;
                imports.push(Located {
                    span: self.range_span(start, cursor),
                    value: path,
                });
                continue;
            }
            // Card `NATIVE-LIBRARY-SOURCE-BINDING`: `extern library NAME;`, the
            // file-scoped selection of the manifest-bound library this file's
            // declared extern surface resolves against.
            //
            // **No new reserved word** (constraint 14). `extern` has introduced
            // a declaration since `source.extern-call.v1`, and `library` has
            // been a source word since `module library;`; what is new is the
            // two-word sequence `extern library` at declaration position, which
            // was a hard parse error before this card ("expected a QCore
            // declaration", since `extern` admitted only `fn` after it). The
            // spelling is the `unsafe extern fn` precedent exactly: an existing
            // word in front of an existing form. `library` stays an ordinary
            // identifier in every other position.
            //
            // Read here rather than in `declaration` because it declares no
            // name: it is a file-local trust act, like an import, and putting it
            // among the declarations would put a manifest identity into the
            // namespace name resolution walks.
            // Card `SUPPLIED-OWNED-RETURN-CODE-HANDOFF` (USER DECISION
            // 2026-10-15, "aliases are allowed"): `alias #NAME = @C_NAME;`, a
            // module-local second spelling of one `@` name. **No new reserved
            // word**: `alias` followed by `#` at declaration position was a
            // parse error before the card ("expected a QCore declaration"),
            // and the census found no `alias` and no `@` in code position in
            // any tracked source file; `alias` stays an ordinary identifier
            // everywhere else. Read here, like an import, because it declares
            // nothing name resolution walks; only a contract module carries
            // one, and a name is declared once.
            if self.word(cursor, "alias") && self.symbol(cursor + 1, Symbol::Hash) {
                let start = cursor;
                // An explicit executable or library header refuses here; a
                // headerless file (the compiler's staged view of a contract
                // module, whose header it blanks) is checked by its consumer
                // against the file's own mode.
                if mode != SourceModuleMode::Contract && mode_span.is_some() {
                    return Err(self.error(
                        cursor,
                        "an `alias` declaration belongs to a contract module",
                    ));
                }
                let name = self.identifier(cursor + 2)?.to_owned();
                if !self.symbol(cursor + 3, Symbol::Equal) {
                    return Err(self.error(cursor + 3, "expected `=` in `alias #NAME = @C_NAME;`"));
                }
                if !self.symbol(cursor + 4, Symbol::At) {
                    return Err(self.error(
                        cursor + 4,
                        "the right-hand side of an alias is an `@` name: `alias #NAME = @C_NAME;`",
                    ));
                }
                let target = self.identifier(cursor + 5)?.to_owned();
                if !self.symbol(cursor + 6, Symbol::Semicolon) {
                    return Err(self.error(cursor + 6, "expected semicolon after alias"));
                }
                cursor += 7;
                if aliases.iter().any(|alias| alias.value.name == name) {
                    return Err(self.error(start + 2, "an alias is declared once per module"));
                }
                aliases.push(Located {
                    span: self.range_span(start, cursor),
                    value: SourceAlias { name, target },
                });
                continue;
            }
            if self.words(cursor, &["extern", "library"]) {
                let start = cursor;
                cursor += 2;
                let name = self.identifier(cursor)?.to_owned();
                cursor += 1;
                if self.tokens.get(cursor).map(|token| &token.kind)
                    != Some(&TokenKind::Symbol(Symbol::Semicolon))
                {
                    return Err(self.error(cursor, "expected semicolon after library binding"));
                }
                cursor += 1;
                library_bindings.push(Located {
                    span: self.range_span(start, cursor),
                    value: name,
                });
                continue;
            }
            // Card `CONTRACT-INHERIT`: an `#include` directive is resolved by
            // the contract loader against a published load path and elaborated
            // away before any consumer parses the module, so reaching it here
            // means nothing resolved it. It stays a parse error — exactly as a
            // top-level `#` was before the card — with a message naming why.
            if self.symbol(cursor, Symbol::Hash) && self.word(cursor + 1, "include") {
                return Err(self.error(
                    cursor,
                    "`#include` is a contract-inclusion directive; only the contract loader \
                     resolves it, against a published contract load path",
                ));
            }
            // Card `W48-QR-C-STRUCT-IMPORT` (user decision 4): the FFI marker
            // `[| quant #c_ffi |]` on the declaration that follows it. Five
            // tokens the lexer already produced (`[|` and `|]` are the
            // quant-index aliases, `#` the attribute marker), claimed only at
            // declaration position, where a `[|` was a parse error before —
            // the census found no `[|` in any tracked source file. The marker
            // marks an `extern fn` and nothing else; whether a module may carry
            // one at all (a contract module, not a `.qr`) is the driver's.
            let marker = self.symbol(cursor, Symbol::QuantOpen)
                && self.word(cursor + 1, "quant")
                && self.symbol(cursor + 2, Symbol::Hash)
                && self.word(cursor + 3, "c_ffi")
                && self.symbol(cursor + 4, Symbol::QuantClose);
            let c_ffi = if marker {
                let span = self.range_span(cursor, cursor + 5);
                cursor += 5;
                if !self.words(cursor, &["extern", "fn"]) {
                    return Err(self.error(
                        cursor,
                        "`[| quant #c_ffi |]` marks an `extern fn` declaration of a contract module",
                    ));
                }
                Some(span)
            } else {
                None
            };
            let (mut declaration, next) = self.declaration(cursor)?;
            declaration.value.c_ffi = c_ffi;
            declarations.push(declaration);
            cursor = next;
        }
        for index in 0..self.tokens.len().saturating_sub(1) {
            if self.symbol(index, Symbol::Hash) && self.word(index + 1, "import") {
                let span = self.tokens[index].span;
                if !imports
                    .iter()
                    .chain(&runtime_imports)
                    .any(|import| import.span.start <= span.start && span.end <= import.span.end)
                {
                    return Err(
                        self.error(index, "#import directives are permitted only at top level")
                    );
                }
            }
        }
        // Card `W48-QR-CLOSURES`: lift every closure into a hidden worker
        // declared immediately before the function that spawns it, before the
        // task and match inventories read any statement.
        let mut closures = BTreeMap::new();
        let mut with_closures = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            if declaration.value.kind == DeclarationKind::Function
                && let Some(body) = &declaration.value.body
            {
                tasks::find_closures(
                    &self,
                    &declaration.value.name,
                    body,
                    &mut with_closures,
                    &mut closures,
                )?;
            }
            with_closures.push(declaration);
        }
        let declarations = with_closures;
        self.closures = closures;
        let task_expressions = tasks::parse_inventory(&self, &declarations)?;
        let result_matches = result_match::parse_inventory(&self, &declarations)?;
        Ok((
            SourceProgram {
                imports,
                library_bindings,
                aliases,
                task_expressions,
                result_matches,
                mode,
                mode_span,
                source: self.source.to_owned(),
                lexical_tokens: self.tokens.clone(),
                tokens: self.tokens.iter().map(|token| token.kind.clone()).collect(),
                pipe_refusals: self.pipe_refusals.clone(),
                closures: self
                    .closures
                    .values()
                    .map(|lifted| lifted.closure.clone())
                    .collect(),
                declarations,
            },
            runtime_imports,
        ))
    }

    fn declaration(
        &self,
        start: usize,
    ) -> Result<(Located<SourceDeclaration>, usize), SourceParseError> {
        let (kind, name_index) = if self.words(start, &["concurrency", "checked"]) {
            (DeclarationKind::ConcurrencyProfile, None)
        } else if self.words(start, &["verify", "scope"]) {
            (DeclarationKind::VerifierScope, Some(start + 2))
        } else if self.word(start, "dimension") {
            (DeclarationKind::Dimension, Some(start + 1))
        } else if self.words(start, &["quant", "axis"]) {
            (DeclarationKind::Axis, Some(start + 2))
        } else if self.words(start, &["resource", "type"]) {
            (DeclarationKind::ResourceType, Some(start + 2))
        } else if self.word(start, "type") {
            // Card `AGGREGATES-V1`: `type NAME {` is a record type and
            // `type NAME = …;` is the alias it always was. The discriminator is
            // the **one token after the name**, which is the narrowest rule that
            // separates them, and it claims a shape that did not parse at all
            // before this card — a bare `type NAME {` fails the alias's own
            // `=` validation. `resource type NAME {` is claimed by the branch
            // above and never reaches here, which is what keeps the eleven
            // resource types of the pinned contract corpus untouched.
            if self.symbol(start + 2, Symbol::LeftBrace) {
                (DeclarationKind::Record, Some(start + 1))
            } else {
                (DeclarationKind::TypeAlias, Some(start + 1))
            }
        } else if self.word(start, "struct") && self.symbol(start + 1, Symbol::Bang) {
            // Card `W48-QR-C-STRUCT-IMPORT`: `struct! { type NAME = …`. The
            // name is the fifth token; the rest of the shape is
            // `crate::source_ffi`'s to read.
            if !self.symbol(start + 2, Symbol::LeftBrace) || !self.word(start + 3, "type") {
                return Err(self.error(
                    start,
                    "expected `struct! { type NAME = c\"struct TAG\" from \"HEADER\"; fields { … } }`",
                ));
            }
            (DeclarationKind::ImportedStruct, Some(start + 4))
        } else if self.word(start, "error") {
            (DeclarationKind::ErrorType, Some(start + 1))
        } else if self.word(start, "const") {
            (DeclarationKind::Constant, Some(start + 1))
        } else if self.words(start, &["unsafe", "extern", "fn"]) {
            // Card `UNSAFE-LINKAGE`: an extern declaration with no contract at
            // all. It keeps `DeclarationKind::ExternFunction`, so no
            // elaboration, name-resolution or typed-AST path changes shape; the
            // waiver is read back from the declaration's own leading token by
            // `source_extern::source_extern_declarations`. `unsafe` was already
            // a reserved source word introducing the `unsafe { … }` statement,
            // so this spelling mints no new reserved word.
            (DeclarationKind::ExternFunction, Some(start + 3))
        } else if self.words(start, &["extern", "fn"]) {
            (DeclarationKind::ExternFunction, Some(start + 2))
        } else if self.word(start, "fn") {
            (DeclarationKind::Function, Some(start + 1))
        } else {
            return Err(self.error(start, "expected a QCore declaration"));
        };

        let name = match name_index {
            Some(index) => self.identifier(index)?.to_owned(),
            None => "checked-concurrency-0.2".to_owned(),
        };
        let (end, body_open) = match kind {
            DeclarationKind::ResourceType
            | DeclarationKind::ErrorType
            | DeclarationKind::Record
            | DeclarationKind::ImportedStruct
            | DeclarationKind::VerifierScope => {
                let open = self.find_symbol(start, Symbol::LeftBrace)?;
                (self.close(open)? + 1, None)
            }
            DeclarationKind::Function => {
                let open = self.function_body(start)?;
                (self.close(open)? + 1, Some(open))
            }
            _ => (self.semicolon(start)? + 1, None),
        };
        let body = body_open.map(|open| self.block(open)).transpose()?;
        let function_contract = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            self.function_contract(start, body_open.unwrap_or(end))?
        } else {
            None
        };
        let verifier_scope = if kind == DeclarationKind::VerifierScope {
            let open = self.find_symbol(start, Symbol::LeftBrace)?;
            Some(self.verifier_scope(open)?)
        } else {
            None
        };
        self.validate_declaration(kind, start, end)?;
        let parameters = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            self.function_parameters(start, end)?
        } else {
            Vec::new()
        };
        let result_type = if matches!(
            kind,
            DeclarationKind::Function | DeclarationKind::ExternFunction
        ) {
            Some(self.function_result_type(start, body_open.unwrap_or(end - 1))?)
        } else {
            None
        };
        let span = self.range_span(start, end);
        let quant_index_types =
            crate::quant_index::parse_quant_index_types(&self.tokens[start..end])?;
        Ok((
            Located {
                span,
                value: SourceDeclaration {
                    kind,
                    name,
                    parameters,
                    result_type,
                    quant_index_types,
                    body,
                    function_contract,
                    verifier_scope,
                    tokens: self.tokens[start..end]
                        .iter()
                        .map(|token| token.kind.clone())
                        .collect(),
                    c_ffi: None,
                },
            },
            end,
        ))
    }

    fn function_result_type(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<SourceResultType, SourceParseError> {
        let params = self.find_symbol(start, Symbol::LeftParen)?;
        let type_start = self.close(params)? + 2; // skip the validated arrow
        let mut cursor = type_start;
        let quantitative_types =
            crate::quant_index::parse_quant_index_types(&self.tokens[type_start..limit])?;
        let mut angle_depth = 0_u32;
        while cursor < limit {
            // Quantitative predicates can contain comparison `>` tokens. Use
            // their parser's span instead of treating comparisons as brackets.
            if let Some(quantitative) = quantitative_types
                .iter()
                .find(|ty| ty.span.start == self.tokens[cursor].span.start)
            {
                while cursor < limit && self.tokens[cursor].span.start < quantitative.span.end {
                    cursor += 1;
                }
                continue;
            }
            if angle_depth == 0 && (self.word(cursor, "effects") || self.word(cursor, "contract")) {
                break;
            }
            if self.symbol(cursor, Symbol::Less) {
                angle_depth += 1;
            } else if self.symbol(cursor, Symbol::Greater) {
                angle_depth = angle_depth
                    .checked_sub(1)
                    .ok_or_else(|| self.error(cursor, "unmatched result type delimiter"))?;
            }
            cursor = self.skip_group(cursor);
        }
        if cursor == type_start || angle_depth != 0 {
            return Err(self.error(
                type_start,
                "function requires a nonempty balanced result type",
            ));
        }
        let span = self.range_span(type_start, cursor);
        Ok(SourceResultType {
            kind: self.type_kind(type_start),
            span,
            source: self.source[span.start..span.end].to_owned(),
            tokens: self.tokens[type_start..cursor]
                .iter()
                .map(|token| token.kind.clone())
                .collect(),
        })
    }

    fn function_contract(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<Option<SourceFunctionContract>, SourceParseError> {
        let params = self.find_symbol(start, Symbol::LeftParen)?;
        let mut cursor = self.close(params)? + 1;
        let mut contract = None;
        while cursor < limit {
            if self.word(cursor, "contract") {
                if contract.is_some() {
                    return Err(self.error(cursor, "function has more than one contract block"));
                }
                // Card `CONTRACT-INHERIT`: `contract inherit NAME;` is resolved
                // and elaborated by the contract loader; unresolved, it is a
                // parse error, as every non-block contract tail was before.
                if self.word(cursor + 1, "inherit") {
                    return Err(self.error(
                        cursor + 1,
                        "`contract inherit NAME;` is resolved only by the contract loader, \
                         against a published contract load path",
                    ));
                }
                if !self.symbol(cursor + 1, Symbol::LeftBrace) {
                    return Err(self.error(cursor + 1, "function contract requires a block"));
                }
                let open = cursor + 1;
                let close = self.close(open)?;
                if close >= limit {
                    return Err(self.error(open, "function contract lies outside its signature"));
                }
                contract = Some(self.contract_block(cursor, open, close, true)?);
                cursor = close + 1;
            } else {
                cursor = self.skip_group(cursor);
            }
        }
        Ok(contract)
    }

    fn contract_block(
        &self,
        contract_start: usize,
        open: usize,
        close: usize,
        allow_local: bool,
    ) -> Result<SourceFunctionContract, SourceParseError> {
        let mut clauses = Vec::new();
        let mut premise_names = BTreeSet::new();
        let mut cursor = open + 1;
        while cursor < close {
            let clause_start = cursor;
            if self.word(cursor, "quant") {
                let negated = self.word(cursor + 1, "not");
                let hash = cursor + if negated { 2 } else { 1 };
                if self.symbol(hash, Symbol::Hash)
                    && !self.word(hash + 1, "outer")
                    && self.word(hash + 2, "in")
                {
                    if !allow_local {
                        return Err(self.error(
                            cursor,
                            "local-condition-nesting: nested local blocks are unsupported",
                        ));
                    }
                    let condition = match self.tokens.get(hash + 1).map(|t| &t.kind) {
                        Some(TokenKind::Word(name)) => name.clone(),
                        _ => {
                            return Err(self.error(
                                hash + 1,
                                "local-condition-name: expected a condition name",
                            ));
                        }
                    };
                    let inner_open = hash + 3;
                    if !self.symbol(inner_open, Symbol::LeftBrace) {
                        return Err(self
                            .error(inner_open, "local-condition-block: expected `{` after `in`"));
                    }
                    let inner_close = self.close(inner_open)?;
                    if inner_close >= close {
                        return Err(self.error(
                            inner_open,
                            "local-condition-block: block exceeds its contract",
                        ));
                    }
                    let suffix = inner_close + 1;
                    if !self.symbol(suffix, Symbol::Colon)
                        || !self.symbol(suffix + 1, Symbol::LeftBracket)
                        || !self.symbol(suffix + 2, Symbol::LeftBracket)
                        || !self.word(suffix + 3, "local")
                        || !self.symbol(suffix + 4, Symbol::RightBracket)
                        || !self.symbol(suffix + 5, Symbol::RightBracket)
                        || !self.symbol(suffix + 6, Symbol::Semicolon)
                    {
                        return Err(
                            self.error(suffix, "local-condition-scope: expected `: [[local]];`")
                        );
                    }
                    let inner = self.contract_block(inner_open, inner_open, inner_close, false)?;
                    if inner.clauses.is_empty()
                        || inner.clauses.iter().any(|c| {
                            !matches!(
                                c.value.kind,
                                SourceContractClauseKind::Requires
                                    | SourceContractClauseKind::QuantRegistered
                            )
                        })
                    {
                        return Err(self.error(inner_open, "local-condition-clause: a nonempty local block contains only prerequisites or lifetime protocols"));
                    }
                    clauses.push(Located {
                        span: self.range_span(clause_start, suffix + 7),
                        value: SourceContractClause {
                            kind: SourceContractClauseKind::QuantConditionBlock {
                                condition,
                                negated,
                                clauses: inner.clauses,
                            },
                            expression_span: self.range_span(hash, suffix + 6),
                            expression_source: self.source[self.range_span(hash, suffix + 6).start
                                ..self.range_span(hash, suffix + 6).end]
                                .trim()
                                .to_owned(),
                        },
                    });
                    cursor = suffix + 7;
                    continue;
                }
            }
            let (kind, expression_start) = if self.word(cursor, "requires") {
                (SourceContractClauseKind::Requires, cursor + 1)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "condition") {
                use qcore_schemas::quant_condition::Selector as C;
                let (selector, next) = if self.word(cursor + 2, "entry") {
                    (C::Entry, cursor + 3)
                } else {
                    let (exit, next) = self.contract_exit_selector(cursor + 2, close)?;
                    (
                        match exit {
                            SourceContractExitSelector::Normal => C::Normal,
                            SourceContractExitSelector::Any => C::Any,
                            SourceContractExitSelector::Error(e) => C::Error(e),
                        },
                        next,
                    )
                };
                (SourceContractClauseKind::QuantCondition(selector), next)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "effect") {
                let (selector, next) = self.contract_exit_selector(cursor + 2, close)?;
                (SourceContractClauseKind::QuantEffect(selector), next)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "theorem") {
                (SourceContractClauseKind::QuantTheorem, cursor + 2)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "proposition") {
                (SourceContractClauseKind::QuantProposition, cursor + 2)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "checks") {
                (SourceContractClauseKind::QuantCheck, cursor + 2)
            } else if self.word(cursor, "quant")
                && self.word(cursor + 1, "null")
                && self.symbol(cursor + 2, Symbol::Minus)
                && self.word(cursor + 3, "deref")
            {
                (SourceContractClauseKind::QuantNullDeref, cursor + 4)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "deref") {
                (SourceContractClauseKind::QuantDeref, cursor + 2)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "registered") {
                (SourceContractClauseKind::QuantRegistered, cursor + 2)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "cell") {
                (SourceContractClauseKind::QuantCell, cursor + 2)
            } else if self.word(cursor, "quant")
                && self.symbol(cursor + 1, Symbol::Hash)
                && self.symbol(cursor + 3, Symbol::Equal)
            {
                (SourceContractClauseKind::QuantConditionBinding, cursor + 1)
            } else if self.word(cursor, "quant") && self.word(cursor + 1, "live") {
                (SourceContractClauseKind::QuantLive, cursor + 2)
            } else if self.word(cursor, "quant")
                && self.symbol(cursor + 1, Symbol::Hash)
                && self.word(cursor + 2, "outer")
            {
                // Card OUTER-CONTRACT: `quant #outer …;`, read whole (from
                // `#outer`) by the outer contract v1 loader alone.
                (SourceContractClauseKind::QuantOuter, cursor + 1)
            } else if self.word(cursor, "quant")
                && self.symbol(cursor + 2, Symbol::Hash)
                && !["normal", "any", "error"]
                    .iter()
                    .any(|selector| self.word(cursor + 1, selector))
            {
                // Card NULL-DEREF-QTT-GROUNDING: `quant WORD #B;` with a word
                // that names no clause is an unknown shorthand (before this
                // card it was the same rejection, as a missing exit selector).
                return Err(self.error(
                    cursor + 1,
                    "quant-shorthand-unknown: the collocated `quant` forms are `quant checks`, \
                     `quant null-deref`, `quant deref` and `quant live`",
                ));
            } else if self.word(cursor, "ensures") || self.word(cursor, "quant") {
                let is_quant = self.word(cursor, "quant");
                let (selector, next) = self.contract_exit_selector(cursor + 1, close)?;
                let kind = if is_quant {
                    SourceContractClauseKind::Quant(selector)
                } else {
                    SourceContractClauseKind::Ensures(selector)
                };
                (kind, next)
            } else {
                return Err(self.error(
                    cursor,
                    "contract clause must begin with `requires`, `ensures`, or `quant`",
                ));
            };
            let semicolon = self.semicolon_before(expression_start, close)?;
            if expression_start == semicolon {
                return Err(self.error(expression_start, "contract clause expression is empty"));
            }
            // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: a named premise is
            // read by the contract format itself, total-or-reject in every
            // vocabulary (resource v1, bounds v1, a model contract).
            if self.word(expression_start, PREMISE_CLAUSE_WORD) {
                self.premise_clause(&kind, expression_start, semicolon, &mut premise_names)?;
            }
            let expression_span = self.range_span(expression_start, semicolon);
            let expression_source = self.source[expression_span.start..expression_span.end]
                .trim()
                .to_owned();
            // Formal formulas own their operators only after their bounded,
            // typed grammar accepts the complete clause. No C fact is inferred.
            let formal = match &kind {
                SourceContractClauseKind::QuantProposition => {
                    crate::quant_proposition::parse(&expression_source).map(|_| ())
                }
                SourceContractClauseKind::QuantCondition(selector) => {
                    crate::quant_condition::parse(selector.clone(), &expression_source).map(|_| ())
                }
                SourceContractClauseKind::QuantEffect(_) => {
                    crate::quant_effect::parse(&expression_source).map(|_| ())
                }
                SourceContractClauseKind::QuantTheorem => {
                    crate::quant_theorem::parse(&expression_source).map(|_| ())
                }
                _ => Ok(()),
            };
            formal.map_err(|error| self.error(expression_start, &error))?;
            clauses.push(Located {
                span: self.range_span(clause_start, semicolon + 1),
                value: SourceContractClause {
                    kind,
                    expression_span,
                    expression_source,
                },
            });
            cursor = semicolon + 1;
        }
        Ok(SourceFunctionContract {
            span: self.range_span(contract_start, close + 1),
            clauses,
        })
    }

    /// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (contract specification
    /// C04.18, USER DECISION 2026-10-15 "Add premise support"): the named
    /// premise is part of the contract format every module parses through,
    /// a resource view's, a bounds view's and a Quirin model contract's
    /// alike. A clause headed `resource_premise` is exactly `requires
    /// resource_premise(NAME, "STATEMENT")`: `NAME` a lowercase identifier
    /// (`[a-z][a-z0-9_]*`), the statement one plain, non-blank string with
    /// no escape and no control character, each name at most once per
    /// declaration. Anything else rejects the module. Where a vocabulary
    /// admits the clause is its loader's (resource v1 anywhere; bounds v1
    /// only beside a `nonnull_member` clause, of any kind since card
    /// NONNULL-MEMBER-UNIFIED-QUALIFIER).
    fn premise_clause(
        &self,
        kind: &SourceContractClauseKind,
        start: usize,
        semicolon: usize,
        names: &mut BTreeSet<String>,
    ) -> Result<(), SourceParseError> {
        let malformed = || {
            self.error(
                start,
                "malformed premise: write `requires resource_premise(NAME, \"STATEMENT\");` \
                 with a lowercase identifier and one plain string",
            )
        };
        if *kind != SourceContractClauseKind::Requires {
            return Err(self.error(
                start,
                "a named premise is a `requires` clause: `requires resource_premise(NAME, \"STATEMENT\");`",
            ));
        }
        let kinds = self.tokens[start..semicolon]
            .iter()
            .map(|token| &token.kind)
            .collect::<Vec<_>>();
        let [
            _,
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(name),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::StringLiteral(statement),
            TokenKind::Symbol(Symbol::RightParen),
        ] = kinds.as_slice()
        else {
            return Err(malformed());
        };
        let identifier = name
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !identifier
            || statement.trim().is_empty()
            || statement.contains('\\')
            || statement.chars().any(char::is_control)
        {
            return Err(malformed());
        }
        if !names.insert(name.clone()) {
            return Err(self.error(
                start,
                format!("duplicate premise `{name}`: a declaration names each premise once"),
            ));
        }
        Ok(())
    }

    fn contract_exit_selector(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(SourceContractExitSelector, usize), SourceParseError> {
        if self.word(start, "normal") {
            return Ok((SourceContractExitSelector::Normal, start + 1));
        }
        if self.word(start, "any") {
            return Ok((SourceContractExitSelector::Any, start + 1));
        }
        if self.word(start, "error") {
            if !self.symbol(start + 1, Symbol::LeftParen) {
                return Err(self.error(start + 1, "error exit selector requires an error type"));
            }
            let close = self.close(start + 1)?;
            if close >= limit || close == start + 2 {
                return Err(self.error(start + 1, "error exit selector requires an error type"));
            }
            let error_span = self.range_span(start + 2, close);
            return Ok((
                SourceContractExitSelector::Error(
                    self.source[error_span.start..error_span.end]
                        .trim()
                        .to_owned(),
                ),
                close + 1,
            ));
        }
        Err(self.error(
            start,
            "contract clause requires `normal`, `error(Type)`, or `any` exit selector",
        ))
    }

    fn verifier_scope(&self, open: usize) -> Result<VerifierScopeDeclaration, SourceParseError> {
        let close = self.close(open)?;
        let mut cursor = open + 1;
        let mut providers = Vec::new();
        let mut seen = BTreeSet::new();
        while cursor < close {
            let entry_start = cursor;
            if !self.word(cursor, "plugin") {
                return Err(self.error(cursor, "verifier scope expects a `plugin` entry"));
            }
            cursor += 1;
            let mut components = Vec::new();
            components.push(self.identifier(cursor)?.to_owned());
            cursor += 1;
            while self.symbol(cursor, Symbol::Dot) {
                cursor += 1;
                components.push(self.identifier(cursor)?.to_owned());
                cursor += 1;
            }
            let plugin_id = components.join(".");
            if components.len() < 2
                || components.iter().any(|component| {
                    component.is_empty()
                        || !component
                            .chars()
                            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
                })
            {
                return Err(self.error(entry_start, "plugin ID must be a lowercase dotted name"));
            }
            let version = if self.word(cursor, "version") {
                cursor += 1;
                let value = match self.tokens.get(cursor).map(|token| &token.kind) {
                    Some(TokenKind::StringLiteral(value)) | Some(TokenKind::Word(value)) => {
                        value.clone()
                    }
                    _ => {
                        return Err(self.error(
                            cursor,
                            "plugin version must be an identifier or string literal",
                        ));
                    }
                };
                cursor += 1;
                Some(value)
            } else {
                None
            };
            if !self.word(cursor, "profile") {
                return Err(self.error(cursor, "plugin entry requires `profile`"));
            }
            cursor += 1;
            let profile = match self.tokens.get(cursor).map(|token| &token.kind) {
                Some(TokenKind::StringLiteral(value)) | Some(TokenKind::Word(value)) => {
                    value.clone()
                }
                _ => return Err(self.error(cursor, "plugin profile must be a name or string")),
            };
            cursor += 1;
            if !self.word(cursor, "capabilities") {
                return Err(self.error(cursor, "plugin entry requires `capabilities`"));
            }
            cursor += 1;
            if !self.symbol(cursor, Symbol::LeftBrace) {
                return Err(self.error(cursor, "plugin capabilities require a set"));
            }
            let capabilities_close = self.close(cursor)?;
            let mut capability_cursor = cursor + 1;
            let mut capabilities = BTreeSet::new();
            while capability_cursor < capabilities_close {
                let capability = self.identifier(capability_cursor)?.to_owned();
                if !capabilities.insert(capability) {
                    return Err(self.error(capability_cursor, "duplicate plugin capability"));
                }
                capability_cursor += 1;
                if capability_cursor < capabilities_close {
                    if !self.symbol(capability_cursor, Symbol::Comma) {
                        return Err(self.error(
                            capability_cursor,
                            "plugin capabilities must be comma-separated",
                        ));
                    }
                    capability_cursor += 1;
                }
            }
            if capabilities.is_empty() {
                return Err(self.error(cursor, "plugin capability set cannot be empty"));
            }
            cursor = capabilities_close + 1;
            if !self.symbol(cursor, Symbol::Semicolon) {
                return Err(self.error(cursor, "plugin entry requires `;`"));
            }
            cursor += 1;
            if !seen.insert((plugin_id.clone(), profile.clone())) {
                return Err(self.error(entry_start, "duplicate plugin/profile entry"));
            }
            providers.push(VerifierProviderUse {
                span: self.range_span(entry_start, cursor),
                plugin_id,
                version,
                profile,
                capabilities,
            });
        }
        if providers.is_empty() {
            return Err(self.error(open, "verifier scope must contain at least one plugin"));
        }
        Ok(VerifierScopeDeclaration { providers })
    }

    fn function_body(&self, start: usize) -> Result<usize, SourceParseError> {
        let params = self.find_symbol(start, Symbol::LeftParen)?;
        let mut cursor = self.close(params)? + 1;
        while cursor < self.tokens.len() {
            if self.symbol(cursor, Symbol::LeftBrace) {
                let close = self.close(cursor)?;
                let next = close + 1;
                if next == self.tokens.len() || self.is_declaration_start(next) {
                    return Ok(cursor);
                }
                cursor = next;
            } else if self.symbol(cursor, Symbol::Semicolon) {
                return Err(self.error(cursor, "function declaration requires a body"));
            } else {
                cursor += 1;
            }
        }
        Err(self.error(start, "function declaration requires a body"))
    }

    fn function_parameters(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<SourceParameter>, SourceParseError> {
        let open = self.find_symbol(start, Symbol::LeftParen)?;
        let close = self.close(open)?;
        if close >= end {
            return Err(self.error(open, "invalid function parameter list"));
        }
        let mut parameters = Vec::new();
        let mut segment = open + 1;
        let mut cursor = segment;
        let mut angle_depth = 0_u32;
        while cursor <= close {
            if let Some(next) = crate::quant_index::quant_type_end(&self.tokens, cursor)? {
                cursor = next;
                continue;
            }
            let separator =
                cursor == close || (angle_depth == 0 && self.symbol(cursor, Symbol::Comma));
            if separator {
                if segment < cursor {
                    parameters.push(self.source_parameter(segment, cursor)?);
                }
                segment = cursor + 1;
                cursor += 1;
                continue;
            }
            if self.symbol(cursor, Symbol::Less) {
                angle_depth += 1;
                cursor += 1;
            } else if self.symbol(cursor, Symbol::Greater) && angle_depth > 0 {
                angle_depth -= 1;
                cursor += 1;
            } else {
                cursor = self.skip_group(cursor);
            }
        }
        Ok(parameters)
    }

    fn source_parameter(
        &self,
        start: usize,
        end: usize,
    ) -> Result<SourceParameter, SourceParseError> {
        let name = self.identifier(start)?.to_owned();
        if !self.symbol(start + 1, Symbol::Colon) {
            return Err(self.error(start + 1, "parameter requires a type annotation"));
        }
        let type_start = start + 2;
        let mut type_end = end;
        let mut cursor = type_start;
        let mut angle_depth = 0_u32;
        while cursor < end {
            if let Some(next) = crate::quant_index::quant_type_end(&self.tokens, cursor)? {
                cursor = next;
                continue;
            }
            if self.symbol(cursor, Symbol::Less) {
                angle_depth += 1;
            } else if self.symbol(cursor, Symbol::Greater) && angle_depth > 0 {
                angle_depth -= 1;
            } else if angle_depth == 0 && (self.word(cursor, "as") || self.word(cursor, "use")) {
                type_end = cursor;
                break;
            }
            cursor = self.skip_group(cursor);
        }
        if type_start >= type_end {
            return Err(self.error(type_start, "parameter type cannot be empty"));
        }
        let type_span = self.range_span(type_start, type_end);
        let mut suffix = type_end;
        if self.word(suffix, "as") {
            if !self.word(suffix + 1, "idx") || suffix + 2 >= end {
                return Err(self.error(suffix, "index alias requires `as idx NAME`"));
            }
            self.identifier(suffix + 2)?;
            suffix += 3;
        }
        let usage = if suffix < end {
            if !self.word(suffix, "use") {
                return Err(self.error(
                    suffix,
                    "expected a usage annotation after the parameter type",
                ));
            }
            Some(Located {
                span: self.range_span(suffix, end),
                value: self.usage_grade(suffix + 1, end)?,
            })
        } else {
            None
        };
        Ok(SourceParameter {
            name: Located {
                span: self.tokens[start].span,
                value: name,
            },
            type_kind: self.type_kind(type_start),
            type_span,
            type_source: self.source[type_span.start..type_span.end].to_owned(),
            usage,
        })
    }

    fn usage_grade(&self, start: usize, end: usize) -> Result<Usage, SourceParseError> {
        let natural = |index: usize| -> Result<u64, SourceParseError> {
            match self.tokens.get(index).map(|token| &token.kind) {
                Some(TokenKind::Integer(value)) => value
                    .parse()
                    .map_err(|_| self.error(index, "usage bound exceeds the supported u64 range")),
                _ => Err(self.error(index, "usage bounds must be natural integer literals")),
            }
        };
        if end == start + 1 {
            return match self.word_value(start) {
                Some("zero") => Ok(Usage::ZERO),
                Some("one") => Ok(Usage::ONE),
                Some("affine") => Ok(Usage::AFFINE),
                Some("relevant") => Ok(Usage::RELEVANT),
                Some("many") => Ok(Usage::MANY),
                Some(_) => Err(self.error(start, "unknown or unsupported symbolic usage grade")),
                None => natural(start).map(|n| Usage::finite(n, n)),
            };
        }
        if end == start + 5
            && self.symbol(start, Symbol::LeftBracket)
            && self.symbol(start + 2, Symbol::Comma)
            && self.symbol(start + 4, Symbol::RightBracket)
        {
            let lower = natural(start + 1)?;
            if self.word(start + 3, "inf") {
                return Ok(Usage::unbounded(lower));
            }
            let upper = natural(start + 3)?;
            if lower > upper {
                return Err(self.error(start, "usage lower bound exceeds its upper bound"));
            }
            return Ok(Usage::finite(lower, upper));
        }
        Err(self.error(
            start,
            "expected a named grade, natural literal, or literal usage interval",
        ))
    }

    fn type_kind(&self, start: usize) -> SourceTypeKind {
        if self.symbol(start, Symbol::LeftParen) {
            return SourceTypeKind::Tuple;
        }
        match self.word_value(start) {
            Some(
                "unit" | "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
                | "isize" | "usize" | "f32" | "f64",
            ) => SourceTypeKind::Atomic,
            Some("array") => SourceTypeKind::Array,
            Some("slice") => SourceTypeKind::Slice,
            Some("ptr") => SourceTypeKind::Pointer,
            Some("own") => SourceTypeKind::Owned,
            Some("borrow") => SourceTypeKind::Borrow,
            Some("quant") if self.word(start + 2, "access") => SourceTypeKind::AccessQuant,
            Some("quant") => SourceTypeKind::Quant,
            Some("result") => SourceTypeKind::Result,
            Some("fn") => SourceTypeKind::Function,
            Some("never") => SourceTypeKind::Never,
            Some("task") => SourceTypeKind::Task,
            Some("mutex") => SourceTypeKind::Mutex,
            _ => SourceTypeKind::Named,
        }
    }

    fn validate_declaration(
        &self,
        kind: DeclarationKind,
        start: usize,
        end: usize,
    ) -> Result<(), SourceParseError> {
        match kind {
            DeclarationKind::ConcurrencyProfile => {
                if end - start != 3 || !self.symbol(end - 1, Symbol::Semicolon) {
                    return Err(self.error(start, "expected `concurrency checked;`"));
                }
            }
            DeclarationKind::VerifierScope => {}
            DeclarationKind::Dimension => {
                if end - start != 7
                    || !self.symbol(start + 2, Symbol::Colon)
                    || !self.word(start + 3, "Idx")
                    || !self.symbol(start + 4, Symbol::Plus)
                    || !self.word(start + 5, "Ordered")
                {
                    return Err(self.error(start, "malformed dimension declaration"));
                }
            }
            DeclarationKind::Axis => {
                if !self.symbol(start + 3, Symbol::Colon)
                    || self
                        .top_level_symbol(start + 4, end, Symbol::Less)
                        .is_none()
                    || !self.symbol(end - 1, Symbol::Semicolon)
                {
                    return Err(self.error(start, "malformed quantitative-axis declaration"));
                }
            }
            DeclarationKind::TypeAlias => {
                if self
                    .top_level_symbol(start + 2, end, Symbol::Equal)
                    .is_none()
                {
                    return Err(self.error(start, "type alias requires `=` and a type"));
                }
            }
            DeclarationKind::Constant => {
                if self
                    .top_level_symbol(start + 2, end, Symbol::Colon)
                    .is_none()
                    || self
                        .top_level_symbol(start + 2, end, Symbol::Equal)
                        .is_none()
                {
                    return Err(self.error(start, "constant requires a type and initializer"));
                }
            }
            DeclarationKind::ExternFunction | DeclarationKind::Function => {
                let params = self.find_symbol(start, Symbol::LeftParen)?;
                let after = self.close(params)? + 1;
                if !self.symbol(after, Symbol::Arrow) {
                    return Err(self.error(after, "function requires `->` return type"));
                }
            }
            DeclarationKind::ResourceType => {
                for member in ["repr", "footprint", "destructor"] {
                    if self.find_word_between(start, end, member).is_none() {
                        return Err(
                            self.error(start, format!("resource type requires `{member}` member"))
                        );
                    }
                }
            }
            DeclarationKind::ErrorType => {}
            // Card `AGGREGATES-V1`: the head is validated by the branch that
            // selected it — a `{` immediately after the name is what made this
            // a record rather than an alias — and the field list's own shape is
            // `crate::source_record`'s to read, exactly as every route reads its
            // own statements out of a byte-span token window. Refusing a
            // malformed field list here would put the rule in a second place.
            DeclarationKind::Record => {}
            // Card `W48-QR-C-STRUCT-IMPORT`: likewise, the head was validated
            // by the branch that selected it and the body is
            // `crate::source_ffi`'s.
            DeclarationKind::ImportedStruct => {}
        }
        Ok(())
    }

    fn block(&self, open: usize) -> Result<SourceBlock, SourceParseError> {
        let close = self.close(open)?;
        let mut cursor = open + 1;
        let mut statements = Vec::new();
        while cursor < close {
            let (statement, next) = self.statement(cursor, close)?;
            if next <= cursor {
                return Err(self.error(cursor, "parser made no progress in block"));
            }
            statements.push(statement);
            cursor = next;
        }
        Ok(SourceBlock {
            span: self.range_span(open, close + 1),
            statements,
        })
    }

    fn statement(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        if self.word(start, "if") {
            return self.if_statement(start);
        }
        if self.word(start, "match") {
            return self.match_statement(start, limit);
        }
        if self.word(start, "while") {
            return self.single_body_statement(start, limit, |body| StatementKind::While { body });
        }
        // Card `LOOP-UNROLL-BUDGET`. `[| #` at a statement head is the
        // unrolling attribute's clause list. The recognition takes **both**
        // tokens on purpose: `[|` alone is the quant-index type's readable
        // opener (`quant[| n where n := 5 |]`, eight code occurrences in the
        // pinned corpus), and requiring the `#` claims only the shape the
        // census found nowhere at all. The list is consumed here and read by
        // `crate::source_loop`; a list that licenses nothing is refused there,
        // at `Q097`, rather than here at `Q000`.
        if self.symbol(start, Symbol::QuantOpen) && self.symbol(start + 1, Symbol::Hash) {
            let mut cursor = start + 1;
            while cursor < limit && !self.symbol(cursor, Symbol::QuantClose) {
                cursor += 1;
            }
            if cursor >= limit {
                // An *unterminated* list is a parse failure, where a
                // *malformed* one is this construct's own refusal: there is no
                // statement here to attribute a diagnostic to.
                return Err(self.error(start, "expected `|]` closing an attribute clause list"));
            }
            return Ok(self.located_statement(start, cursor + 1, StatementKind::LoopAttribute));
        }
        // Card `LOOP-V1`. `for` is census-clean across every `.qcore`/`.qc`
        // file in the tree (zero occurrences outside `//` comments), so this
        // head claims a statement shape that previously did not parse at all.
        if self.word(start, "for") {
            let binding = self.identifier(start + 1)?.to_owned();
            return self
                .single_body_statement(start, limit, |body| StatementKind::For { binding, body });
        }
        if self.words(start, &["error", "region"]) {
            return self.error_region_statement(start, limit);
        }
        if self.word(start, "unsafe") {
            return self.single_body_statement(start, limit, |body| StatementKind::Unsafe { body });
        }
        if self.words(start, &["thread", "scope"]) {
            let name = self.identifier(start + 2)?.to_owned();
            return self.single_body_statement(start, limit, |body| StatementKind::ThreadScope {
                name,
                body,
            });
        }
        if self.words(start, &["with", "lock"]) {
            let body_open = self.last_top_level_block(start, limit)?;
            let guard = self
                .find_word_between(start + 2, body_open, "as")
                .and_then(|index| self.word_value(index + 1))
                .ok_or_else(|| self.error(start, "with lock requires `as` guard binding"))?
                .to_owned();
            let body = self.block(body_open)?;
            let next = self.close(body_open)? + 1;
            return Ok(self.located_statement(
                start,
                next,
                StatementKind::WithLock { guard, body },
            ));
        }

        let semicolon = self.semicolon_before(start, limit)?;
        let next = semicolon + 1;
        let kind = if self.words(start, &["let", "idx"]) {
            StatementKind::LetIndex {
                binding: self.identifier(start + 2)?.to_owned(),
            }
        } else if self.word(start, "let") {
            StatementKind::Let {
                binding: self.pattern_binding(start + 1, semicolon),
            }
        } else if self.word(start, "drop") {
            StatementKind::Drop {
                place: self.root_name(start + 1, semicolon),
            }
        } else if self.word(start, "move") {
            StatementKind::Move {
                place: self.root_name(start + 1, semicolon),
            }
        } else if self.word(start, "discharge") {
            StatementKind::Discharge {
                binding: self.word_value(start + 1).map(ToOwned::to_owned),
            }
        } else if self.word(start, "guard") {
            StatementKind::Guard
        } else if self.word(start, "return") {
            StatementKind::Return
        } else if self.word(start, "raise") {
            StatementKind::Raise
        } else if self.word(start, "break") {
            StatementKind::Break
        } else if self.word(start, "continue") {
            StatementKind::Continue
        } else if self.word(start, "defer") {
            StatementKind::Defer
        } else if self.word(start, "footprint") {
            StatementKind::Footprint {
                name: self.identifier(start + 1)?.to_owned(),
            }
        } else if let Some(bindings) = self.event_statement_bindings(start) {
            StatementKind::Event { bindings }
        } else if let Some(equal) = self.top_level_symbol(start, semicolon, Symbol::Equal) {
            StatementKind::Assignment {
                place: self.root_name(start, equal),
            }
        } else {
            StatementKind::Expression
        };
        Ok(self.located_statement(start, next, kind))
    }

    /// The names a `source.event.v1` statement binds, or `None` when this is not
    /// one of that route's statement heads (card `STD-EVENT`).
    ///
    /// `close D;` is deliberately **absent**: `close` is an existing reserved
    /// word the checked mutex-guard profile already uses at a statement head,
    /// and it binds nothing here anyway, so it stays an ordinary expression
    /// statement whose one word resolves as a reference like any other.
    fn event_statement_bindings(&self, start: usize) -> Option<Vec<String>> {
        match self.word_value(start)? {
            // `endpoints A B;` introduces both names of one pair.
            "endpoints" => Some(
                [self.word_value(start + 1)?, self.word_value(start + 2)?]
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect(),
            ),
            // `watch E D INTEREST;` introduces the registration and references
            // the endpoint.
            "watch" => Some(vec![self.word_value(start + 1)?.to_owned()]),
            // The rest bind nothing: every name they carry is a reference.
            "event" | "unwatch" | "wake" | "dispatch" => Some(Vec::new()),
            _ => None,
        }
    }

    fn if_statement(
        &self,
        start: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        let open = self.find_symbol(start, Symbol::LeftParen)?;
        let close = self.close(open)?;
        // The extent of the tokens between the parentheses. An empty condition
        // has no such tokens, so the span is the parentheses themselves rather
        // than an inverted range; the branch pass refuses it at `Q045`.
        let condition = if close == open + 1 {
            self.range_span(open, close + 1)
        } else {
            self.range_span(open + 1, close)
        };
        let then_open = close + 1;
        if !self.symbol(then_open, Symbol::LeftBrace) {
            return Err(self.error(then_open, "if condition must be followed by a block"));
        }
        let mut conditions = vec![condition];
        let mut branches = vec![self.block(then_open)?];
        let mut next = self.close(then_open)? + 1;
        if self.word(next, "else") {
            if self.symbol(next + 1, Symbol::LeftBrace) {
                branches.push(self.block(next + 1)?);
                next = self.close(next + 1)? + 1;
            } else if self.word(next + 1, "if") {
                let (nested, nested_next) = self.if_statement(next + 1)?;
                if let StatementKind::If {
                    conditions: nested_conditions,
                    branches: nested_branches,
                } = nested.value
                {
                    conditions.extend(nested_conditions);
                    branches.extend(nested_branches);
                }
                next = nested_next;
            } else {
                return Err(self.error(next + 1, "else must be followed by a block or if"));
            }
        }
        Ok(self.located_statement(
            start,
            next,
            StatementKind::If {
                conditions,
                branches,
            },
        ))
    }

    fn match_statement(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        let open = self.last_top_level_block(start, limit)?;
        let close = self.close(open)?;
        let mut arms = Vec::new();
        let mut cursor = open + 1;
        while cursor < close {
            let arrow = self
                .top_level_symbol(cursor, close, Symbol::FatArrow)
                .ok_or_else(|| self.error(cursor, "match arm requires `=>`"))?;
            let body_open = arrow + 1;
            if !self.symbol(body_open, Symbol::LeftBrace) {
                return Err(self.error(body_open, "match arm requires a block"));
            }
            arms.push(self.block(body_open)?);
            cursor = self.close(body_open)? + 1;
        }
        Ok(self.located_statement(start, close + 1, StatementKind::Match { arms }))
    }

    fn single_body_statement<F>(
        &self,
        start: usize,
        limit: usize,
        constructor: F,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError>
    where
        F: FnOnce(SourceBlock) -> StatementKind,
    {
        let open = self.last_top_level_block(start, limit)?;
        let body = self.block(open)?;
        let next = self.close(open)? + 1;
        Ok(self.located_statement(start, next, constructor(body)))
    }

    fn error_region_statement(
        &self,
        start: usize,
        limit: usize,
    ) -> Result<(Located<StatementKind>, usize), SourceParseError> {
        let name = self.identifier(start + 2)?.to_owned();
        let body_open = self.find_symbol(start + 3, Symbol::LeftBrace)?;
        let mut blocks = vec![self.block(body_open)?];
        let mut catch_bindings = Vec::new();
        let mut next = self.close(body_open)? + 1;
        while next < limit && self.word(next, "catch") {
            let pattern_open = next + 1;
            if !self.symbol(pattern_open, Symbol::LeftParen) {
                return Err(self.error(pattern_open, "catch requires a parenthesized pattern"));
            }
            let mut block_open = self.close(pattern_open)? + 1;
            let mut binding = None;
            if self.words(block_open, &["quant", "open"]) {
                // `quant open REGION as BINDING` — five tokens. The binding is
                // the one name the clause introduces, so it is recorded; the
                // region it names is a reference into this statement's own
                // token window and is read there.
                if !self.word(block_open + 3, "as") {
                    return Err(self.error(
                        block_open + 3,
                        "quant open names the cleanup capability it binds: \
                         `quant open REGION as BINDING`",
                    ));
                }
                binding = Some(self.identifier(block_open + 4)?.to_owned());
                block_open += 5;
            }
            if !self.symbol(block_open, Symbol::LeftBrace) {
                return Err(self.error(block_open, "catch requires a block"));
            }
            blocks.push(self.block(block_open)?);
            catch_bindings.push(binding);
            next = self.close(block_open)? + 1;
        }
        if blocks.len() == 1 {
            return Err(self.error(next, "error region requires at least one catch"));
        }
        Ok(self.located_statement(
            start,
            next,
            StatementKind::ErrorRegion {
                name,
                blocks,
                catch_bindings,
            },
        ))
    }

    fn located_statement(
        &self,
        start: usize,
        end: usize,
        value: StatementKind,
    ) -> (Located<StatementKind>, usize) {
        (
            Located {
                span: self.range_span(start, end),
                value,
            },
            end,
        )
    }

    fn last_top_level_block(&self, start: usize, limit: usize) -> Result<usize, SourceParseError> {
        let mut cursor = start;
        let mut candidate = None;
        while cursor < limit {
            if self.symbol(cursor, Symbol::LeftBrace) {
                candidate = Some(cursor);
                cursor = self.close(cursor)? + 1;
                if cursor == limit || self.is_statement_start(cursor) {
                    break;
                }
            } else {
                cursor = self.skip_group(cursor);
            }
        }
        candidate.ok_or_else(|| self.error(start, "structured statement requires a block"))
    }

    fn semicolon(&self, start: usize) -> Result<usize, SourceParseError> {
        self.semicolon_before(start, self.tokens.len())
    }

    fn semicolon_before(&self, start: usize, limit: usize) -> Result<usize, SourceParseError> {
        let mut cursor = start;
        while cursor < limit {
            if self.symbol(cursor, Symbol::Semicolon) {
                return Ok(cursor);
            }
            cursor = self.skip_group(cursor);
        }
        Err(self.error(start, "statement or declaration requires `;`"))
    }

    fn top_level_symbol(&self, start: usize, limit: usize, expected: Symbol) -> Option<usize> {
        let mut cursor = start;
        while cursor < limit {
            if self.symbol(cursor, expected) {
                return Some(cursor);
            }
            cursor = self.skip_group(cursor);
        }
        None
    }

    fn find_symbol(&self, start: usize, expected: Symbol) -> Result<usize, SourceParseError> {
        (start..self.tokens.len())
            .find(|&index| self.symbol(index, expected))
            .ok_or_else(|| self.error(start, format!("expected {expected:?}")))
    }

    fn find_word_between(&self, start: usize, end: usize, expected: &str) -> Option<usize> {
        (start..end).find(|&index| self.word(index, expected))
    }

    fn close(&self, open: usize) -> Result<usize, SourceParseError> {
        self.matching
            .get(&open)
            .copied()
            .ok_or_else(|| self.error(open, "expected a matched delimiter"))
    }

    fn skip_group(&self, cursor: usize) -> usize {
        if matches!(
            self.tokens.get(cursor).map(|token| &token.kind),
            Some(TokenKind::Symbol(
                Symbol::LeftParen | Symbol::LeftBrace | Symbol::LeftBracket
            ))
        ) {
            self.matching
                .get(&cursor)
                .map_or(cursor + 1, |close| close + 1)
        } else {
            cursor + 1
        }
    }

    fn pattern_binding(&self, start: usize, limit: usize) -> Option<String> {
        (start..limit).find_map(|index| {
            self.word_value(index)
                .and_then(|word| (!matches!(word, "_" | "mut")).then(|| word.to_owned()))
        })
    }

    fn root_name(&self, start: usize, limit: usize) -> Option<String> {
        (start..limit).find_map(|index| self.word_value(index).map(ToOwned::to_owned))
    }

    /// The byte extent of a token range.
    ///
    /// Card `PIPE-OPERATOR` generalized this from `first.start .. last.end` to
    /// the minimum start and maximum end over the range. For a range whose
    /// tokens are in source order — every range in a program with no pipe —
    /// the two agree exactly, byte for byte; the pinned goldens are unmoved.
    /// For a desugared pipeline the tokens are a permutation of the statement's
    /// source text, so only the min/max form still covers the operand the
    /// reader wrote, which is what keeps the byte-span statement windows every
    /// route uses over its own tokens correct.
    fn range_span(&self, start: usize, end: usize) -> Span {
        let range = &self.tokens[start..end];
        Span {
            start: range
                .iter()
                .map(|token| token.span.start)
                .min()
                .unwrap_or_else(|| self.tokens[start].span.start),
            end: range
                .iter()
                .map(|token| token.span.end)
                .max()
                .unwrap_or_else(|| self.tokens[end - 1].span.end),
        }
    }

    fn identifier(&self, index: usize) -> Result<&str, SourceParseError> {
        self.word_value(index)
            .ok_or_else(|| self.error(index, "expected identifier"))
    }

    fn word_value(&self, index: usize) -> Option<&str> {
        match self.tokens.get(index).map(|token| &token.kind) {
            Some(TokenKind::Word(word)) => Some(word),
            _ => None,
        }
    }

    fn word(&self, index: usize, expected: &str) -> bool {
        self.word_value(index) == Some(expected)
    }

    fn words(&self, start: usize, expected: &[&str]) -> bool {
        expected
            .iter()
            .enumerate()
            .all(|(offset, word)| self.word(start + offset, word))
    }

    fn symbol(&self, index: usize, expected: Symbol) -> bool {
        matches!(
            self.tokens.get(index).map(|token| &token.kind),
            Some(TokenKind::Symbol(symbol)) if *symbol == expected
        )
    }

    fn is_declaration_start(&self, index: usize) -> bool {
        self.word(index, "dimension")
            || self.word(index, "quant")
            || self.word(index, "type")
            || self.word(index, "resource")
            || self.word(index, "error")
            || self.word(index, "const")
            || self.word(index, "extern")
            // Card `UNSAFE-LINKAGE`: `unsafe extern fn` is a declaration, so a
            // preceding function body ends where it starts. `unsafe` also
            // introduces a statement, but this predicate is consulted only at
            // the token after a *top-level* brace group, where a statement
            // cannot begin.
            || self.word(index, "unsafe")
            || self.word(index, "fn")
            || self.word(index, "concurrency")
            || self.word(index, "verify")
            || self.word(index, "import")
            || (self.symbol(index, Symbol::Hash) && self.word(index + 1, "import"))
            // Card `W48-QR-C-STRUCT-IMPORT`: an imported struct and the FFI
            // marker begin declarations. Both shapes were parse errors after a
            // top-level brace group before the card.
            || (self.word(index, "struct") && self.symbol(index + 1, Symbol::Bang))
            || (self.symbol(index, Symbol::QuantOpen) && self.word(index + 1, "quant"))
    }

    fn is_statement_start(&self, index: usize) -> bool {
        // Card `LOOP-UNROLL-BUDGET`: an unrolling attribute begins a statement,
        // so a structured statement's body ends where one starts. Measured
        // load-bearing for the same reason `for` itself was: without this row
        // `last_top_level_block` runs past an attributed loop that follows a
        // block and claims that loop's body as the earlier statement's.
        // **Both** tokens are required, so the quant-index type's `[|` opener
        // is untouched.
        if self.symbol(index, Symbol::QuantOpen) && self.symbol(index + 1, Symbol::Hash) {
            return true;
        }
        matches!(
            self.word_value(index),
            Some(
                "let"
                    | "drop"
                    | "move"
                    | "discharge"
                    | "guard"
                    | "if"
                    | "match"
                    | "while"
                    // Card `LOOP-V1`. Measured load-bearing: without this row
                    // `last_top_level_block` ran past the first `for`'s body
                    // and claimed a *later* loop's block as it, so a module
                    // with two loops bound the first loop's index over the
                    // second loop's body and refused at `Q013`.
                    | "for"
                    | "return"
                    | "raise"
                    | "break"
                    | "continue"
                    | "defer"
                    | "error"
                    | "unsafe"
                    | "thread"
                    | "footprint"
                    | "with"
                    // Statement leaders of the checked mutex-guard profile.
                    // A structured statement's body ends where the next
                    // statement begins, so every leader must be listed here or
                    // `last_top_level_block` would run past an intervening
                    // statement and claim a later block as the body.
                    | "open"
                    | "close"
                    | "read"
                    | "write"
                    // Card T03-F's additional leaders, listed for the same
                    // reason: a fact assertion, a fact use or an announced
                    // acquisition between two blocks must end the first block.
                    | "assert"
                    | "use"
                    | "wait"
            )
        )
    }

    fn error(&self, index: usize, message: impl Into<String>) -> SourceParseError {
        let span = self.tokens.get(index).map_or(
            Span {
                start: self.source.len(),
                end: self.source.len(),
            },
            |token| token.span,
        );
        Self::error_at(span, message)
    }

    fn error_at(span: Span, message: impl Into<String>) -> SourceParseError {
        SourceParseError {
            span,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticPass {
    NameResolution,
    OrdinaryTypes,
    Index,
    Usage,
    Ownership,
    ResourceFlow,
    TypedErrors,
    ErrorRegion,
    CleanupLowering,
    CEmission,
    Concurrency,
    /// Card `source.extern-call.v1`: admission and bridging of a call to a
    /// declared extern function. Owns `Q022`–`Q027` (spec §L21). It is a
    /// separate pass from `OrdinaryTypes` on purpose: `Q020`/`Q021` refuse
    /// *unenforced source contracts* and keep their exact behavior, while this
    /// pass consumes *explicitly selected declared* contracts.
    ExternCall,
    /// Card `STD-MODULES`: admission of the `std.fmt`, `std.io` and
    /// `std.logger` line-producing statements. Owns `Q028`–`Q031` (spec §L21).
    /// A separate pass from `ExternCall` on purpose: that pass admits calls to
    /// *declared* C symbols under selected contracts, while this one admits a
    /// closed set of statements with no callee and no contract at all.
    StandardIo,
    /// Card `UNSAFE-LINKAGE`: admission of an extern declaration that has **no**
    /// contract, so an external library can be linked without one. Owns
    /// `Q032`–`Q035` (spec §L21). A separate pass from `ExternCall` on purpose:
    /// that pass decides whether a *declared contract* bridges to a call, while
    /// this one decides whether a signature may cross a boundary nothing checks
    /// at all, and records the premise that says so.
    UncheckedLinkage,
    /// Card `PIPE-OPERATOR`: admission of the pipe operator `|>` and of the
    /// claimed logical-operator tokens `|` and `&&`. Owns `Q036`–`Q038`
    /// (spec §L21). It is a separate pass because it runs *before* every other
    /// one: a well-formed pipeline is desugared away at parse time and no
    /// later pass can tell it was written, so this pass exists only to attribute
    /// the refusals of the forms that are not desugared away.
    ///
    /// The band is `Q036`–`Q038` rather than the `Q032`–`Q034` this card first
    /// took: `UNSAFE-LINKAGE` claimed `Q032`–`Q035` independently from the same
    /// base and shipped first, so this unshipped band relocated at integration,
    /// following the wave-3 `T07`/`T09` precedent. The code-to-condition
    /// mapping is unchanged under the shift.
    Pipe,
    /// Card `UNSAFE-LINKAGE-ARTIFACT`: admission of a `source.native-library.v1`
    /// manifest selection and of the manifest-resident waivers it sharpens.
    /// Owns `Q039`–`Q042` (spec §L21), the band
    /// `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §7 reserved for it after
    /// `UNSAFE-LINKAGE` took `Q032`–`Q035` and `PIPE-OPERATOR` `Q036`–`Q038`.
    ///
    /// Card `NATIVE-LIBRARY-CONTRACT-SLICE` adds a **second band**,
    /// `Q047`–`Q050`, for the manifest's contracted entries, through the same
    /// §L21 process. The band is not contiguous with the first because
    /// `Q043`–`Q046` were taken between the two cards (`STD-IO-COMPOSE` and
    /// `IF-STATEMENT`); a pass owning two ranges is the honest record of that
    /// order, and no code moved to make it look tidier. A collision discovered
    /// at integration relocates the *unshipped* band, per the wave-3 `T07`/`T09`
    /// precedent; the code-to-condition mapping is stable under such a shift.
    ///
    /// Card `NATIVE-LIBRARY-HEADER` adds a **third band**, `Q063`–`Q064`, for
    /// the manifest's **exported header** binding — reading the library's real
    /// `.h` instead of believing the manifest's transcription of one. Not
    /// contiguous with either earlier band, for the same reason: `Q050` was
    /// taken between the cards (`COMPOSE-CONCURRENCY`). `Q063` is "the header
    /// binding does not load"; `Q064` is "the manifest and the header
    /// disagree", a **refusal**, deliberately the `Q041` disposition rather than
    /// `Q048`'s, because the weaker admissible reading below a withdrawn header
    /// binding is the manifest *without* a header and that is accepted. A
    /// publication tripwire on the `Q049` model was considered and not
    /// allocated: the header identity is a field of the residence record `Q049`
    /// already requires, so the code would be unreachable.
    ///
    /// Card `NATIVE-LIBRARY-SOURCE-BINDING` adds a **fourth band**,
    /// `Q065`–`Q067`, for the **source file's own** `extern library NAME;`
    /// binding, which stops manifest resolution being by symbol name across the
    /// whole selection. This band *is* contiguous with the third; the pass now
    /// owns four ranges. `Q065` is "this file's binding list does not resolve"
    /// (unknown name, ambiguous name, repeated binding, nothing selected);
    /// `Q066` is "this file declares a symbol a manifest it did not bind binds",
    /// a **refusal** on `Q041`'s ground read from the other side — falling back
    /// to the weaker premise would let a file stop its certificate naming a
    /// build by binding a different library; `Q067` is the publication tripwire
    /// on the `Q049`/`Q035` model, which unlike `Q063`–`Q064`'s declined one is
    /// not structurally impossible, because the binding premise is carried
    /// beside the callee premises rather than inside a record another gate
    /// already demands.
    /// Card `NATIVE-LIBRARY-HEADER-DECORATION` extends `Q063` and allocates
    /// **no new code**: a declared include text that is not supplied, whose
    /// digest does not match or that nothing reaches, a text no manifest
    /// declares, and a decoration macro the reader cannot derive are all the
    /// same condition — the header binding does not load — reached by a new
    /// route. Two names for one fact would be a claim a reader cannot check.
    ///
    /// A separate pass from `UncheckedLinkage` on purpose: that pass decides
    /// whether a *signature* may cross a boundary nothing checks, and its
    /// decision is unchanged here. This one decides whether the **library
    /// manifest** a waiver is resident in loads, binds and agrees — the trust
    /// object is the manifest, not the declaration, and a manifest that does
    /// not load is refused without ever reaching a declaration. It is likewise
    /// separate from `ExternCall`: that pass decides whether a *contract* is
    /// callable, and this one whether a manifest may attribute that contract's
    /// symbol to a named build.
    NativeLibrary,
    /// Card `IF-STATEMENT`: admission of the source branching construct
    /// `source.branch.v1` — `if (COND) { … } else { … }`, its decidable
    /// condition tier and its custody join. Owns `Q044`–`Q046` (spec §L21).
    ///
    /// A separate pass from `CEmission` on purpose: `Q500` refuses a construct
    /// the reduced C11 CFG *cannot encode*, and this construct is encoded — the
    /// typed CFG has carried branch terminators since Cycle 7. What this pass
    /// decides is whether a written branch is inside `source.branch.v1`: the
    /// condition tier, the arm statement subset, and whether the two arms reach
    /// the join in agreeing custody states.
    ///
    /// The band deliberately leaves `Q039`–`Q043` free for the native-library
    /// manifest band that `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` adjustment
    /// A2 reserves as "`Q039` onward". A collision discovered at integration
    /// relocates the *unshipped* band, per the wave-3 `T07`/`T09` precedent the
    /// `Pipe` band above already followed; the code-to-condition mapping is
    /// stable under such a shift.
    Branch,
    /// Card `STD-IO-COMPOSE`: the checked composition of `source.extern-call.v1`
    /// and `source.std-io.v1`. Owns the single code `Q043` (spec §L21), the one
    /// left free between the native-library band `Q039`–`Q042` and the branch
    /// band `Q044`–`Q046`.
    ///
    /// A separate pass from both composed passes on purpose, and deliberately a
    /// *small* one: every refusal either composed route already gives a module
    /// is still given by that route under that route's own code, so this pass
    /// owns only the rules that exist because the two models meet — the
    /// interleaving, the rendering of a value the extern model bound, and the
    /// union of the two derived effect rows. If this pass ever owned a custody
    /// or a rendering rule, the composition would have stopped being a union.
    ComposedStdIo,
    /// Card `COMPOSE-CONCURRENCY`: the checked composition of
    /// `source.std-io.v1` with the **checked task route**. Owns the single code
    /// `Q050` (spec §L21), the first free code above the native-library second
    /// band `Q047`–`Q049`.
    ///
    /// A separate pass from `ComposedStdIo` on purpose. That pass joins two
    /// straight-line models whose numbers are declared at the site; this one
    /// joins a **branching, concurrent** model whose numbers are computed by the
    /// task model at the rendering's own step, and its rules are correspondingly
    /// different: *where* a line may stand (the parent's body, never a worker's,
    /// because a worker's writes are performed by that worker's thread and no
    /// model here orders them), and whether the task checker admits reading the
    /// operand there. Every other refusal a composed task module receives is the
    /// task route's `Q600` or the standard-IO route's `Q028`–`Q031`, at that
    /// route's own site.
    ComposedTasks,
    /// Card `STD-BUFFER`: admission of `source.buffer.v1` — the `std.buffer`
    /// bundled module's owned, extent-carrying byte buffer. Owns `Q051`–`Q057`
    /// (spec §L21), the first free band above `ComposedTasks`'s `Q050`.
    ///
    /// A separate pass from `StandardIo` on purpose: that pass admits a closed
    /// set of statements with **no resource and no custody at all** — its
    /// operand type structurally cannot denote one — while this pass admits a
    /// resource acquisition, an indexed access into it and its release. The two
    /// could not share a band without the code range ceasing to describe what
    /// the pass decides.
    ///
    /// It is likewise separate from `Ownership`, whose `Q103` this pass
    /// deliberately does **not** emit even though every custody refusal on this
    /// route is `qcore_core::ownership`'s: the ownership checker is *inherited*
    /// here rather than run as the sequential passes run it, so a reader
    /// following `Q054` back finds this route's lowering and not the sequential
    /// resource-flow bundle. The refusal is the same one; the attribution says
    /// which route asked.
    ///
    /// A collision discovered at integration relocates this band, per the
    /// wave-3 `T07`/`T09` precedent; the code-to-condition mapping is stable
    /// under such a shift.
    StandardBuffer,
    /// Card `STD-EVENT`: admission of the `std.event` statements
    /// (`source.event.v1`). Owns `Q058`–`Q062` and, since card
    /// `COMPOSE-EVENT-LINES`, the **second band** `Q074` (spec §L21): the
    /// composition with the source line vocabulary and the one value form this
    /// route has. The two bands are not contiguous because `Q063`–`Q064` were
    /// allocated between the cards — the same honest record the `NativeLibrary`
    /// pass keeps of its own ranges — and because the second band was
    /// **relocated at integration** from `Q065` to `Q074`, `Q065`–`Q067` having
    /// been taken by `NATIVE-LIBRARY-SOURCE-BINDING` and `Q068`–`Q073` by
    /// `STRING-SLICE`'s `StandardString` band. That is the wave-3 `T07`/`T09`
    /// precedent: the code-to-condition mapping is stable under such a shift,
    /// which is why the relocation touched numbers and no rule.
    ///
    /// A separate pass from `Concurrency` on purpose. That pass owns the
    /// checked *task* and *mutex* profiles, whose subject is a thread; this one
    /// admits a single-threaded reactor context and the custody edge a
    /// registration puts on a descriptor, and it decides a *determinism*
    /// question — whether a dispatch turn has one well-defined observation —
    /// that no concurrency profile asks.
    ///
    /// The band is `Q058`–`Q062` because `Q050` was the highest code allocated
    /// on the base this card was cut from. A collision discovered at
    /// integration relocates this **unshipped** band, per the wave-3 `T07`/`T09`
    /// precedent that `PIPE-OPERATOR` and `NATIVE-LIBRARY-CONTRACT-SLICE` both
    /// followed; the code-to-condition mapping is stable under such a shift.
    Event,
    /// Card `STRING-SLICE`: admission of `source.string.v1` — the `std.string`
    /// bundled module's immutable UTF-8 `{ptr, len}` **string view**
    /// (`docs/design/STRING_SLICE_V1.md`). Owns `Q068`–`Q073` and, since card
    /// `STRING-SLICE-IO` composed the route with the output modules, the second
    /// band `Q075`–`Q076` (spec §L21).
    ///
    /// The second band is not contiguous with the first because `Q074` was
    /// allocated to `Event` between the two cards — the honest record of the
    /// order they merged in, on the `NativeLibrary` and `Event` precedents, and
    /// no shipped code was moved to make it look tidier. It carries the
    /// composition's **own** two conditions and nothing else: `Q075` the shape
    /// of a rendered-text line, `Q076` a text operand naming something this
    /// entry bound no string for. Every other refusal a rendered string can
    /// receive is an existing one at its existing code — `Q070` for a literal
    /// that does not decode, `Q071` for an inline slice out of range or off a
    /// code-point boundary, `Q072` for a selector applied to a string, `Q073`
    /// for an undeclared derived effect — because the composition added no new
    /// way for any of those to fail.
    ///
    /// **Third band, `Q077`–`Q079`** (card `STRING-CONCAT`): text a program
    /// **computes** rather than writes, and amendment §5's `std.buffer` bridge.
    /// `Q077` the shape or the bound of a construct the band adds — a malformed
    /// `concat(…)`, `borrow_utf8(…)` or `dec(…)` piece, a composition of fewer
    /// than two pieces, an over-long composed payload, a bridge position naming
    /// no buffer, a buffer statement outside the entry, and an embedded
    /// `source.buffer.v1` statement that route's own model refused, carrying
    /// **that model's own message** under this pass's code. `Q078` the borrow
    /// discipline (a store into or release of a buffer inside a live borrow's
    /// window) together with the inherited, unedited ownership checker's own
    /// refusal, carrying that checker's error value. `Q079` the **static UTF-8
    /// decision**: the bytes a `borrow_utf8` would view are not well-formed,
    /// decided at check time out of the buffer's write history. The band is
    /// `Q077`–`Q079` because `STRING-SLICE-IO` returned exactly those three
    /// codes to the pool and the integrator assigned them back to this pass.
    ///
    /// A separate pass from `StandardBuffer` on purpose, and the separation is
    /// the amendment's own §5: that pass admits an **owned, mutable, explicitly
    /// released** `{identity, extent, custody}` and every one of its refusals is
    /// either a custody rule or a rule about writing a byte, while this pass
    /// admits an **immutable view**, and decides two questions no buffer rule
    /// can state: whether bytes are well-formed UTF-8, and whether a slice bound
    /// falls on a code-point boundary. Sharing a band would make the range stop
    /// describing what either pass decides — which is why, since card
    /// `STRING-CONCAT` let one module hold both vocabularies, an embedded buffer
    /// statement's refusal is reported at **this** pass's `Q077` with **that**
    /// pass's message rather than at `Q051`–`Q057`.
    ///
    /// It is likewise separate from `StandardIo`, whose `Q029` this pass does
    /// not reuse for its own `Q073` effect refusal, on §L21's rule that a code
    /// may not be reused by a second semantic pass.
    ///
    /// The band was **allocated as `Q065`–`Q070`** on the base this card was cut
    /// from, where `Q064` was the highest code, and was **relocated to
    /// `Q068`–`Q073` at integration** after card `NATIVE-LIBRARY-SOURCE-BINDING`
    /// merged first and took `Q065`–`Q067` as `NativeLibrary`'s fourth range.
    /// That is the wave-3 `T07`/`T09` precedent applied exactly as it reads: the
    /// band was still unshipped, so it moved rather than the shipped one. The
    /// relocation is a pure renumber — same order, same six conditions, same
    /// meanings — which is what the precedent's "the code-to-condition mapping
    /// is stable under such a shift" is for.
    StandardString,
    /// Card `QUIRIN-SOCKET-V1`: admission of `source.socket.v1` — the
    /// `std.socket` bundled module's checked TCP and UDP surface. Owns
    /// `Q080`–`Q089` (spec §L21), a band assigned by the integrator before the
    /// card started, with `Q075`–`Q079` reserved for a sibling card of the same
    /// wave and never touched here.
    ///
    /// A separate pass from `Event` on purpose. That pass admits a **reactor**:
    /// its subject is readiness, a registration's custody edge on a descriptor,
    /// and whether a dispatch turn has one well-defined observation. This one
    /// admits a **transport**: its subject is the socket's own state machine
    /// (created, bound, listening, connected), the bytes committed between two
    /// sockets of this same program, and whether a receive is satisfied by that
    /// program's own history. Neither range can state the other's conditions,
    /// and the two routes are not composed in v1 (`SOCKET-EVENT-COMPOSE`).
    ///
    /// It is likewise separate from `StandardBuffer`, whose `Q054` custody code
    /// and `Q057` effect code this pass does not reuse for its own `Q083` and
    /// `Q087`, on §L21's rule that a code may not be reused by a second
    /// semantic pass — even where the inherited refusal is literally the same
    /// ownership checker's.
    StandardSocket,
    /// Card `HTTP-QUIRIN-V1`: admission of `source.http.v1` — the `std.http`
    /// bundled module's checked HTTP/1.1 message subset. Owns `Q090`–`Q096`
    /// (spec §L21), a band assigned by the integrator before the card started.
    /// `Q097`–`Q099` of that band are **unused and returned to the pool**, and
    /// `Q077`–`Q079`, returned by `STRING-SLICE-IO`, were deliberately not
    /// reused: this band is contiguous from `Q090`, which keeps the band story
    /// readable.
    ///
    /// A separate pass from `StandardSocket` on purpose. That pass admits a
    /// **transport**: its subject is a socket's state machine, the bytes
    /// committed between two sockets of one program, and whether a receive is
    /// satisfied by that program's own history. This one admits a **message**:
    /// its subject is RFC 9112's own grammar — whether a method is a token,
    /// whether a version is `HTTP/1.1`, whether a request names its `Host`,
    /// whether a declared `Content-Length` is the content's byte count — and not
    /// one of those conditions is expressible in the transport's range. The two
    /// compose rather than overlap: every `Q095` this pass issues carries the
    /// socket checker's own error value, unedited.
    ///
    /// It is likewise separate from `StandardString`, whose `Q070` this pass does
    /// not reuse for a quoted token that fails to decode, on §L21's rule that a
    /// code may not be reused by a second semantic pass.
    StandardHttp,
    /// Card `LOOP-V1`: admission of the checked bounded counted loop
    /// `for NAME in 0..BOUND { … }` (`source.loop.v1`). Owns `Q097`–`Q099`
    /// (spec §L21).
    ///
    /// A separate pass on the `IF-STATEMENT` precedent, and for that card's
    /// reason: the construct is a *control shape*, not a capability of any one
    /// bundled module, and a route admits it the way a route admits `if`. v1
    /// admits it on `source.buffer.v1` only; every other route's refusal is
    /// measured and pinned rather than widened (`LOOP-ROUTE-WIDENING`).
    ///
    /// It is separate from `StandardBuffer` on purpose. That pass decides what a
    /// byte access means; this one decides how many times a body runs and what
    /// one pass through it may do to custody — and a second route admitting the
    /// construct inherits those rules rather than restating them.
    Loop,
    PluginVerification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDiagnostic {
    pub code: &'static str,
    pub pass: SemanticPass,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} at bytes {}..{}",
            self.code, self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SourceDiagnostic {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedBinding {
    pub symbol: SymbolId,
    pub name: String,
    pub span: Span,
    pub type_kind: SourceTypeKind,
    pub usage: Option<Located<Usage>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedStatement {
    /// Structured arm-local names; not evidence of result typing/exhaustiveness.
    pub result_match: Option<ResolvedResultMatch>,
    /// Identity introduced by this statement, after its initializer resolves.
    /// Assignments retain the existing identity and introduce no new binding.
    pub binding: Option<SymbolId>,
    /// Body-local scope/guard identity, including an empty body.
    pub body_binding: Option<SymbolId>,
    /// Role-specific name resolution; no task lifetime or capture authority claim.
    pub task_expressions: Vec<ResolvedTaskExpression>,
    pub span: Span,
    pub pass: SemanticPass,
    pub referenced_symbols: Vec<SymbolId>,
    pub nested: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedSourceDeclaration {
    pub symbol: SymbolId,
    pub kind: DeclarationKind,
    pub name: String,
    pub span: Span,
    pub parameters: Vec<TypedBinding>,
    pub result_type: Option<SourceResultType>,
    pub quant_index_types: Vec<crate::quant_index::ResolvedQuantIndexType>,
    pub function_contract: Option<SourceFunctionContract>,
    pub statements: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedSourceProgram {
    pub mode: SourceModuleMode,
    pub declarations: Vec<TypedSourceDeclaration>,
    pub globals: BTreeMap<String, SymbolId>,
    pub origin: String,
}

/// The names the scopes enclosing `closure` hold at its spawn site (card
/// `W48-QR-CLOSURES`): the enclosing function's parameters, the locals and
/// scope names visible there, the `match` arm binders around it, and — for a
/// closure written inside another closure — the outer closure's own.
fn closure_outer_names(program: &SourceProgram, closure: &SourceClosure) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut current = Some(closure);
    while let Some(closure) = current {
        let position = closure.spawn_keyword.start;
        if let Some(enclosing) = program
            .declarations
            .iter()
            .find(|declaration| declaration.value.name == closure.enclosing)
        {
            names.extend(
                enclosing
                    .value
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.value.clone()),
            );
            if let Some(body) = &enclosing.value.body {
                visible_local_names(body, position, &mut names);
            }
        }
        for syntax in program.result_matches() {
            for arm in &syntax.arms {
                if arm.body.span.start <= position
                    && position < arm.body.span.end
                    && let Some(pattern) = &arm.pattern
                {
                    names.insert(pattern.binding.value.clone());
                }
            }
        }
        current = program.closure(&closure.enclosing);
    }
    names
}

fn visible_local_names(block: &SourceBlock, position: usize, names: &mut BTreeSet<String>) {
    if position < block.span.start || position >= block.span.end {
        return;
    }
    for statement in &block.statements {
        if statement.span.start > position {
            break;
        }
        match &statement.value {
            StatementKind::Let {
                binding: Some(name),
            }
            | StatementKind::LetIndex { binding: name }
            | StatementKind::Discharge {
                binding: Some(name),
            } => {
                names.insert(name.clone());
            }
            StatementKind::WithLock { guard, body }
            | StatementKind::ThreadScope { name: guard, body }
                if position >= body.span.start && position < body.span.end =>
            {
                names.insert(guard.clone());
            }
            _ => {}
        }
        for child in statement.value.nested_blocks() {
            visible_local_names(child, position, names);
        }
    }
}

/// Placement of one verbatim declaration slice inside the reconstructed
/// dimension/axis source, together with that slice's span in the caller's own
/// source. Used only to attribute unconditional declaration diagnostics.
struct IndexSlice {
    start: usize,
    end: usize,
    origin: Span,
}

/// Maps a span in the reconstructed dimension/axis source back to the caller's
/// source. Offsets inside a slice map byte for byte; a span that starts past
/// every slice (end of the reconstructed buffer) is attributed to the last
/// slice, and an empty slice list keeps the previous degenerate span rather
/// than inventing a location.
fn map_index_span(span: Span, slices: &[IndexSlice]) -> Span {
    let Some(slice) = slices
        .iter()
        .find(|slice| slice.start <= span.start && span.start < slice.end)
        .or_else(|| slices.last())
    else {
        return Span { start: 0, end: 0 };
    };
    let start = slice.origin.start
        + span
            .start
            .saturating_sub(slice.start)
            .min(slice.end - slice.start);
    let end = slice.origin.start
        + span
            .end
            .saturating_sub(slice.start)
            .min(slice.end - slice.start);
    Span {
        start,
        end: end.max(start),
    }
}

pub fn elaborate_source(
    program: &SourceProgram,
) -> Result<TypedSourceProgram, Vec<SourceDiagnostic>> {
    crate::module_imports::check_source_imports(program)?;
    let mut diagnostics = Vec::new();
    let mut globals = BTreeMap::new();
    let mut next_symbol = 0_u32;
    for declaration in &program.declarations {
        if matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile | DeclarationKind::VerifierScope
        ) {
            continue;
        }
        if globals.contains_key(&declaration.value.name) {
            diagnostics.push(SourceDiagnostic {
                code: "Q010",
                pass: SemanticPass::NameResolution,
                span: declaration.span,
                message: format!("duplicate declaration `{}`", declaration.value.name),
            });
        } else {
            globals.insert(declaration.value.name.clone(), SymbolId(next_symbol));
            next_symbol += 1;
        }
    }

    let mut declarations = Vec::new();
    let mut index_environment = crate::check::StaticEnvironment::default();
    // Declaration validity is independent of whether a quantitative type uses it.
    {
        // Every slice is a verbatim copy of the original declaration text, so
        // each slice's placement in the reconstructed buffer maps back to the
        // original span byte for byte. Retaining that map is what lets a
        // failure here name the offending declaration instead of reporting the
        // degenerate `0..0` span (which rendered as line 1 of the first input
        // file, and in a multi-file build blamed whichever file was listed
        // first). The accepted/rejected set is unchanged.
        let index_declarations = program
            .declarations
            .iter()
            .filter(|declaration| {
                matches!(
                    declaration.value.kind,
                    DeclarationKind::Dimension | DeclarationKind::Axis
                )
            })
            .collect::<Vec<_>>();
        let mut axis_source = String::new();
        let mut slices: Vec<IndexSlice> = Vec::new();
        for declaration in &index_declarations {
            if !axis_source.is_empty() {
                axis_source.push('\n');
            }
            let start = axis_source.len();
            axis_source.push_str(&program.source[declaration.span.start..declaration.span.end]);
            slices.push(IndexSlice {
                start,
                end: axis_source.len(),
                origin: declaration.span,
            });
        }
        match crate::parser::parse_program(&axis_source) {
            Err(error) => {
                // The parser's offsets address the reconstructed buffer; report
                // the caller's own bytes instead.
                let span = map_index_span(error.span, &slices);
                diagnostics.push(SourceDiagnostic {
                    code: "Q016",
                    pass: SemanticPass::Index,
                    span,
                    message: format!("{} at bytes {}..{}", error.message, span.start, span.end),
                });
            }
            Ok(parsed) => match crate::check::StaticEnvironment::check_located(&parsed) {
                Ok(environment) => index_environment = environment,
                Err((error, index)) => diagnostics.push(SourceDiagnostic {
                    code: "Q016",
                    pass: SemanticPass::Index,
                    span: slices
                        .get(index)
                        .map_or(Span { start: 0, end: 0 }, |slice| slice.origin),
                    message: error.to_string(),
                }),
            },
        }
    }
    for declaration in &program.declarations {
        if matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile | DeclarationKind::VerifierScope
        ) {
            continue;
        }
        let symbol = globals[&declaration.value.name];
        let mut scopes = vec![BTreeMap::<String, SymbolId>::new()];
        let mut parameters = Vec::new();
        // Card `W48-QR-CLOSURES`: a capture list names each place once.
        let closure = program.closure(&declaration.value.name);
        if let Some(closure) = closure {
            let mut seen = BTreeSet::new();
            for capture in &closure.captures {
                let key = tasks::place_key(&capture.value.place.value);
                if !seen.insert(key.clone()) {
                    diagnostics.push(SourceDiagnostic {
                        code: "Q011",
                        pass: SemanticPass::NameResolution,
                        span: capture.span,
                        message: format!(
                            "the closure captures `{key}` twice; a capture list names each place \
                             once"
                        ),
                    });
                }
            }
        }
        for parameter in &declaration.value.parameters {
            if scopes[0].contains_key(&parameter.name.value) {
                diagnostics.push(SourceDiagnostic {
                    code: "Q011",
                    pass: SemanticPass::NameResolution,
                    span: parameter.name.span,
                    message: if closure.is_some() {
                        format!(
                            "two captures of the closure are bound to `{}` inside its body; \
                             name one of them with `as NAME`",
                            parameter.name.value
                        )
                    } else {
                        format!("duplicate parameter `{}`", parameter.name.value)
                    },
                });
                continue;
            }
            let parameter_symbol = SymbolId(next_symbol);
            next_symbol += 1;
            scopes[0].insert(parameter.name.value.clone(), parameter_symbol);
            parameters.push(TypedBinding {
                symbol: parameter_symbol,
                name: parameter.name.value.clone(),
                span: parameter.name.span,
                type_kind: parameter.type_kind,
                usage: parameter.usage.clone(),
            });
        }
        let forbidden_names: BTreeSet<String> = globals
            .keys()
            .cloned()
            .chain(
                declaration
                    .value
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.value.clone()),
            )
            .collect();
        let mut quant_index_types = Vec::new();
        for source_type in &declaration.value.quant_index_types {
            let mut forbidden_names = forbidden_names.clone();
            if let Some(body) = &declaration.value.body {
                visible_local_names(body, source_type.span.start, &mut forbidden_names);
                for syntax in program.result_matches() {
                    for arm in &syntax.arms {
                        if arm.body.span.start <= source_type.span.start
                            && source_type.span.start < arm.body.span.end
                            && let Some(pattern) = &arm.pattern
                        {
                            forbidden_names.insert(pattern.binding.value.clone());
                        }
                    }
                }
            }
            match crate::quant_index::resolve_quant_index_type(
                source_type,
                &index_environment,
                &forbidden_names,
            ) {
                Ok(resolved) => quant_index_types.push(resolved),
                Err(message) => diagnostics.push(SourceDiagnostic {
                    code: "Q016",
                    pass: SemanticPass::Index,
                    span: source_type.span,
                    message,
                }),
            }
        }
        let statements = declaration
            .value
            .body
            .as_ref()
            .map_or_else(Vec::new, |body| {
                elaborate_block(
                    body,
                    &mut scopes,
                    &globals,
                    &mut tasks::ResolutionContext {
                        program,
                        live_scopes: BTreeSet::new(),
                        closure_outer: program
                            .closure(&declaration.value.name)
                            .map(|closure| closure_outer_names(program, closure)),
                    },
                    &mut next_symbol,
                    &mut diagnostics,
                    None,
                )
            });
        declarations.push(TypedSourceDeclaration {
            symbol,
            kind: declaration.value.kind,
            name: declaration.value.name.clone(),
            span: declaration.span,
            parameters,
            result_type: declaration.value.result_type.clone(),
            quant_index_types,
            function_contract: declaration.value.function_contract.clone(),
            statements,
        });
    }
    if diagnostics.is_empty() {
        Ok(TypedSourceProgram {
            mode: program.mode,
            declarations,
            globals,
            origin: program.source.clone(),
        })
    } else {
        Err(diagnostics)
    }
}

fn elaborate_block(
    block: &SourceBlock,
    scopes: &mut Vec<BTreeMap<String, SymbolId>>,
    globals: &BTreeMap<String, SymbolId>,
    context: &mut tasks::ResolutionContext<'_>,
    next_symbol: &mut u32,
    diagnostics: &mut Vec<SourceDiagnostic>,
    body_binding: Option<(&str, bool, SymbolId)>,
) -> Vec<TypedStatement> {
    let mut scope = BTreeMap::new();
    let mut introduced_scope = None;
    if let Some((name, thread_scope, symbol)) = body_binding {
        if thread_scope {
            context.live_scopes.insert(symbol);
            introduced_scope = Some(symbol);
        }
        scope.insert(name.to_owned(), symbol);
    }
    scopes.push(scope);
    let mut typed = Vec::new();
    for statement in &block.statements {
        let (binding, reference, pass) = statement_metadata(&statement.value);
        let mut references = lexical_references(statement, context.program);
        if let Some(reference) = reference {
            references.insert(reference.to_owned());
        }
        let mut referenced_symbols = Vec::new();
        for name in references {
            let resolved = scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(&name))
                .or_else(|| globals.get(&name))
                .copied();
            if let Some(symbol) = resolved {
                referenced_symbols.push(symbol);
            } else if context
                .closure_outer
                .as_ref()
                .is_some_and(|outer| outer.contains(&name))
            {
                // Card `W48-QR-CLOSURES`: no implicit capture. The name is
                // one the reader sees around the closure, and it is not in
                // the closure's scope because the capture list does not
                // name it.
                diagnostics.push(SourceDiagnostic {
                    code: "Q013",
                    pass: SemanticPass::NameResolution,
                    span: statement.span,
                    message: format!(
                        "`{name}` belongs to the scope enclosing this closure, and the closure \
                         does not capture it: a closure sees only the places its capture list \
                         names (`copy {name}`, `move {name}`, `read {name} share …`, \
                         `lend {name} LOW..HIGH`); nothing is captured implicitly"
                    ),
                });
            } else {
                diagnostics.push(SourceDiagnostic {
                    code: "Q013",
                    pass: SemanticPass::NameResolution,
                    span: statement.span,
                    message: format!("unknown name `{name}`"),
                });
            }
        }
        let task_expressions =
            tasks::resolve_tasks(statement.span, scopes, globals, context, diagnostics);
        let mut binding_symbol = None;
        // Card `STD-EVENT`: an endpoint pair introduces **two** names in one
        // statement, which the single-binding tuple above cannot carry. The
        // list is read off the variant rather than added to that tuple, so
        // every existing statement form travels the identical code path with a
        // one-element list.
        let introduced: Vec<&str> = match &statement.value {
            StatementKind::Event { bindings } => bindings.iter().map(String::as_str).collect(),
            _ => binding.into_iter().collect(),
        };
        for binding in introduced {
            let scope = scopes.last_mut().expect("block scope exists");
            if scope.contains_key(binding) {
                diagnostics.push(SourceDiagnostic {
                    code: "Q012",
                    pass: SemanticPass::NameResolution,
                    span: statement.span,
                    message: format!("duplicate local binding `{binding}`"),
                });
            } else {
                let symbol = SymbolId(*next_symbol);
                scope.insert(binding.to_owned(), symbol);
                binding_symbol = binding_symbol.or(Some(symbol));
                *next_symbol += 1;
            }
        }
        let body_binding = match &statement.value {
            StatementKind::WithLock { guard, .. } => Some((guard.as_str(), false)),
            StatementKind::ThreadScope { name, .. } => Some((name.as_str(), true)),
            // Card `LOOP-V1`: the loop index is a binding **of the body's
            // scope**, exactly as a `with lock … as g` guard is. It is therefore
            // not visible after the loop, which is the same rule the checked
            // model applies to a body's bindings and the same rule the emitted
            // C's block scoping applies to its declarations.
            StatementKind::For { binding, .. } => Some((binding.as_str(), false)),
            _ => None,
        }
        .map(|(name, thread_scope)| {
            let symbol = SymbolId(*next_symbol);
            *next_symbol += 1;
            (name, thread_scope, symbol)
        });
        let mut nested = Vec::new();
        let match_syntax = context
            .program
            .result_matches()
            .iter()
            .find(|syntax| syntax.span == statement.span)
            .cloned();
        let resolved_match = if let Some(syntax) = match_syntax {
            // Resolve the scrutinee in the enclosing environment before any arm
            // binder exists. Arm scopes and identities are mutually isolated.
            let scrutinee =
                result_match::resolve_scrutinee(&syntax, &task_expressions, scopes, globals);
            let mut arms = Vec::new();
            for arm in &syntax.arms {
                let arm_binding = arm.pattern.as_ref().map(|pattern| {
                    let symbol = SymbolId(*next_symbol);
                    *next_symbol += 1;
                    (pattern.binding.value.as_str(), false, symbol)
                });
                let statements = elaborate_block(
                    &arm.body,
                    scopes,
                    globals,
                    context,
                    next_symbol,
                    diagnostics,
                    arm_binding,
                );
                let start = nested.len();
                nested.extend(statements);
                let statement_range = start..nested.len();
                arms.push(ResolvedResultMatchArm {
                    syntax: arm.clone(),
                    binding: arm_binding.map(|(_, _, symbol)| symbol),
                    statement_range,
                });
            }
            Some(ResolvedResultMatch {
                syntax,
                scrutinee,
                arms,
            })
        } else if let StatementKind::ErrorRegion {
            blocks,
            catch_bindings,
            ..
        } = &statement.value
        {
            // Card `QUANT-REGION-SOURCE`. An error region's blocks do not share
            // one binding: the body introduces none, and each catch clause
            // introduces exactly the cleanup capability its own
            // `quant open … as B` names — a linear capability of *that*
            // handler, which is why it is scoped to that handler's block and to
            // nothing else. `body_binding` carries one name for one nested
            // block and cannot express that, so the clause list is read off the
            // variant here, exactly as `StatementKind::Event`'s binding list is
            // read off its own variant above.
            for (position, child) in blocks.iter().enumerate() {
                let clause_binding = position
                    .checked_sub(1)
                    .and_then(|clause| catch_bindings.get(clause))
                    .and_then(Option::as_deref)
                    .map(|name| {
                        let symbol = SymbolId(*next_symbol);
                        *next_symbol += 1;
                        (name, false, symbol)
                    });
                nested.extend(elaborate_block(
                    child,
                    scopes,
                    globals,
                    context,
                    next_symbol,
                    diagnostics,
                    clause_binding,
                ));
            }
            None
        } else {
            for child in statement.value.nested_blocks() {
                nested.extend(elaborate_block(
                    child,
                    scopes,
                    globals,
                    context,
                    next_symbol,
                    diagnostics,
                    body_binding,
                ));
            }
            None
        };
        typed.push(TypedStatement {
            result_match: resolved_match,
            binding: binding_symbol,
            body_binding: body_binding.map(|(_, _, symbol)| symbol),
            task_expressions,
            span: statement.span,
            pass,
            referenced_symbols,
            nested,
        });
    }
    scopes.pop();
    if let Some(symbol) = introduced_scope {
        context.live_scopes.remove(&symbol);
    }
    typed
}

fn lexical_references(
    statement: &Located<StatementKind>,
    program: &SourceProgram,
) -> BTreeSet<String> {
    let lexical_tokens = program.lexical_tokens();
    // Card `LOOP-UNROLL-BUDGET`: every word inside `[| #unroll, where #bound <
    // N |]` is a clause word of the attribute grammar and none of them is a
    // reference to a binding, so the list contributes no referenced name at
    // all. That is what keeps `unroll` and `where` out of
    // `is_reserved_source_word` — the constraint-14 census found `where` eight
    // times in code position inside `quant<n where n := 5>` clauses of the
    // pinned corpus, and a global reservation would change how a corpus this
    // card may not touch resolves a name. `bound` is already reserved and is
    // reused rather than respelled.
    if matches!(statement.value, StatementKind::LoopAttribute) {
        return BTreeSet::new();
    }
    // Card `QUANT-REGION-SOURCE`: a region **label** is not a reference. After
    // the truncation below an `error region R { … }` statement would contribute
    // exactly `R`, which names no binding and resolved to `Q013` — which is why
    // no program in this repository could contain an error region at all. The
    // label is the statement's own, `error` and `region` are reserved, and the
    // clause words after the body's `{` are already outside the window, so the
    // whole statement contributes no referenced name. This is `LoopAttribute`'s
    // rule reached by a second statement, and it is what keeps `region` and
    // `catch` from needing any change to name resolution.
    if matches!(statement.value, StatementKind::ErrorRegion { .. }) {
        return BTreeSet::new();
    }
    // Card `W48-QR-CLOSURES`: a closure's header and body are its hidden
    // worker's, resolved in that worker's own scope; the enclosing statement
    // references what its hand-written twin's does — the capture list's
    // places and, below, the worker.
    let mut tokens = lexical_tokens
        .iter()
        .filter(|token| {
            statement.span.start <= token.span.start
                && token.span.end <= statement.span.end
                && !program.outside_closure_window(token.span, statement.span)
        })
        .collect::<Vec<_>>();
    if matches!(
        statement.value,
        StatementKind::If { .. }
            | StatementKind::Match { .. }
            | StatementKind::While { .. }
            | StatementKind::For { .. }
            | StatementKind::ErrorRegion { .. }
            | StatementKind::Unsafe { .. }
            | StatementKind::ThreadScope { .. }
            | StatementKind::WithLock { .. }
    ) && let Some(body) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        tokens.truncate(body);
    }
    if matches!(
        statement.value,
        StatementKind::Let { .. } | StatementKind::LetIndex { .. }
    ) && let Some(equal) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))
    {
        tokens.drain(..=equal);
    }
    if matches!(statement.value, StatementKind::Assignment { .. })
        && let Some(equal) = tokens
            .iter()
            .position(|token| token.kind == TokenKind::Symbol(Symbol::Equal))
    {
        tokens.drain(..=equal);
    }

    // Card `SOCKET-INET-ADDRESSES`: the host half of a declared address is
    // spelled with words — a hexadecimal IPv6 group and every label of a
    // hostname both lex as identifiers — so it is excluded from reference
    // collection by the address reader itself rather than by a shape rule. The
    // **port** stays outside the excluded range and remains a genuine reference.
    let address_host = crate::source_socket::address_host_span(&tokens);
    let mut result = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        let TokenKind::Word(word) = &token.kind else {
            continue;
        };
        if address_host.is_some_and(|(start, end)| start <= index && index < end) {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|index| tokens.get(index));
        let next = tokens.get(index + 1);
        // Header binders are definitions in the body scope, not references.
        // Skip the binder token only: `with lock m as m` still resolves the
        // mutex expression against the enclosing scope.
        let body_binder = match &statement.value {
            // Card `STD-EVENT`: the names an event statement introduces are
            // definitions in this scope, not references to an outer one. They
            // are skipped **by name** rather than by position, because the two
            // binding forms put them at different positions and a binder name
            // is never also a reference within one statement of that grammar.
            StatementKind::Event { bindings } => bindings.iter().any(|name| name == word),
            // Card `LOOP-V1`: `for NAME in 0..BOUND`. The index is a definition
            // in the body's scope, not a reference to an outer one, and `in` is
            // a **contextual** clause word skipped only at the one position a
            // `for` header puts it — index 2, immediately after `for NAME`.
            //
            // `in` is deliberately **not** in `is_reserved_source_word`, and the
            // constraint-14 census is why: it occurs as a *reference* inside the
            // pinned contract corpus — `ensures normal resource_borrowed(in);`
            // in `providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_helpers.qv`, whose
            // `extern fn generate_chunk_name(in: ptr<mut, u8>, …)` names a
            // parameter `in`. That is exactly the shape card `STRING-SLICE`
            // measured for `len`, and it gets exactly that card's answer: narrow
            // the rule rather than edit a pinned corpus this card may not touch.
            // The bound, if it is a name rather than a literal, is a genuine
            // reference and still resolves normally.
            StatementKind::For { binding, .. } => {
                index == 1 && word == binding || (index == 2 && word == "in")
            }
            StatementKind::ThreadScope { .. } => index == 2,
            StatementKind::WithLock { .. } => matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Word(keyword)) if keyword == "as"
            ),
            _ => false,
        };
        // Card `STRING-SLICE`: `len` is a **contextual** clause word, skipped
        // only where it is immediately followed by `(`. It is deliberately not
        // in `is_reserved_source_word`: the constraint-14 census found `len` as
        // a *reference* inside the pinned contract corpus
        // (`ensures normal resource_borrowed(len);` in
        // `providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv` and
        // `providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv`), so a global reservation
        // would change how a corpus this card may not touch resolves a name.
        // The `len(` shape occurs zero times in QCore source anywhere in the
        // repository, so the narrower rule collides with nothing — the same
        // "rename or narrow rather than edit the pinned surface" discipline that
        // gave `bytes` its spelling and `effort` its name.
        // Card `STRING-CONCAT` widens the same rule to `concat` and
        // `borrow_utf8`, each censused at zero occurrences of its `WORD(` shape
        // anywhere in the repository, and each therefore reserved **nowhere**.
        let length_query =
            crate::source_std_string::is_string_clause_word(word, next.map(|t| &t.kind));
        // Card `SLICE-PERMISSIONS-WRITE-LEND` widens the same rule to `lend`,
        // censused at zero occurrences of its `lend(` shape anywhere in the
        // repository and therefore reserved **nowhere** — the read card's own
        // census already recorded the bare word free in source position.
        let lend_clause =
            crate::source_std_buffer::is_lend_clause_word(word, next.map(|t| &t.kind));
        // Card `WRITE-LEND-TASKS` widens the same rule to the **capture**
        // position: `lend` as the first word of a spawn capture
        // (`spawn s w(lend PLACE LOW..HIGH)`). Three positive premises, so an
        // unrecognised `lend` is still collected: the statement spells `spawn`
        // before this token, the word stands immediately after `(` or `,`,
        // and a name follows it. The census found `lend` in that position in
        // no file of this repository — before this card it was a parse
        // failure — and the place name after it *is* a reference and still
        // resolves normally.
        // Card `W48-QR-CLOSURES` adds the closure's opening bar to the two
        // symbols a capture follows.
        let lend_capture = word == "lend"
            && matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Symbol(
                    Symbol::LeftParen | Symbol::Comma | Symbol::Pipe
                ))
            )
            && matches!(next.map(|token| &token.kind), Some(TokenKind::Word(_)))
            && tokens[..index]
                .iter()
                .any(|token| matches!(&token.kind, TokenKind::Word(w) if w == "spawn"));
        // Card `W48-QR-SCALAR-TYPES`: the conversion `EXPR as TYPE` names a
        // type, never a binding. Contextual on both sides — the already
        // reserved `as` before it and a scalar type name — so no word is newly
        // reserved; the constraint-14 census found no `as` followed by a
        // scalar type name in any tracked source file, and a `with lock m as
        // g` binder is a different word in the same position.
        // Card `W48-QR-FLOATING-POINT`: `f32`/`f64` after `as` too, on the
        // same contextual terms; the census found `as f32`/`as f64` in no
        // tracked file, and a route that does not admit the conversion refuses
        // it at its own reader.
        let conversion_type =
            matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Word(previous)) if previous == "as"
            ) && qcore_core::scalar_types::ScalarType::parse_admitting_floats(word).is_some();
        // Card `W48-QR-SCALAR-TYPES-R6`: `let NAME = protected TYPE;` names a
        // typed protected payload, never a binding. Contextual on both sides —
        // `protected` before it and a scalar type name — so no word is newly
        // reserved; before the card the shape was refused here (`Q013`), and
        // the constraint-14 census found `protected` followed by a scalar type
        // name in no tracked source file.
        // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `protected f32`/`protected f64`
        // on the same contextual terms.
        let protected_type =
            matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Word(previous)) if previous == "protected"
            ) && qcore_core::scalar_types::ScalarType::parse_admitting_floats(word).is_some();
        // Card `W48-QR-SCALAR-TYPES-R2`: `raise ArithmeticError::VARIANT;` names
        // the builtin typed arithmetic error, which no declaration introduces.
        // Contextual — the name immediately followed by `::` — so no word is
        // reserved; the constraint-14 census found `ArithmeticError` in no
        // tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` file.
        let builtin_error = word == qcore_core::scalar_types::ArithmeticError::NAME
            && matches!(
                next.map(|token| &token.kind),
                Some(TokenKind::Symbol(Symbol::ColonColon))
            );
        // Card `W48-QR-CLOSURES`: `as NAME` after a closure's capture names
        // the parameter the capture is bound to inside the body, a definition
        // there and not a reference here. Contextual — `as` inside a closure's
        // capture list — so no word is reserved; the census found `as` inside
        // no spawn's capture list.
        let closure_binder = matches!(
            previous.map(|token| &token.kind),
            Some(TokenKind::Word(previous)) if previous == "as"
        ) && program.closures().iter().any(|closure| {
            closure.captures_span.start <= token.span.start
                && token.span.end <= closure.captures_span.end
        });
        if body_binder
            || closure_binder
            || builtin_error
            || length_query
            || lend_clause
            || lend_capture
            || conversion_type
            || protected_type
            || is_reserved_source_word(word)
            || matches!(
                previous.map(|token| &token.kind),
                Some(TokenKind::Symbol(
                    Symbol::Dot | Symbol::ColonColon | Symbol::Apostrophe
                ))
            )
            || matches!(
                next.map(|token| &token.kind),
                Some(TokenKind::Symbol(Symbol::Colon))
            )
        {
            continue;
        }
        result.insert(word.clone());
    }
    // Card `W48-QR-CLOSURES`: a statement that spawns a closure references
    // its hidden worker, as its twin's statement references the named one.
    for closure in program.closures() {
        if statement.span.start <= closure.spawn_keyword.start
            && closure.spawn_keyword.end <= statement.span.end
            && !program.outside_closure_window(closure.spawn_keyword, statement.span)
        {
            result.insert(closure.hidden.clone());
        }
    }
    result
}

fn is_reserved_source_word(word: &str) -> bool {
    matches!(
        word,
        "let"
            | "idx"
            // Card `LOOP-V1`: the bounded counted loop's own statement head
            // (`for NAME in 0..BOUND { … }`). Reserved for the same reason as
            // every profile keyword below: it is a statement's own keyword,
            // never a reference to a binding. The constraint-14 census found
            // **zero** occurrences outside `//` comments across all 418
            // `.qcore`/`.qc` files, so the reservation collides with nothing.
            //
            // `in` is **not** reserved and is handled contextually in
            // `referenced_names` instead — see the comment there for the
            // census site that forbids a global reservation.
            | "for"
            | "drop"
            | "move"
            | "discharge"
            | "guard"
            | "else"
            | "raise"
            | "if"
            | "match"
            | "while"
            | "invariant"
            | "quant"
            | "bound"
            | "return"
            | "break"
            | "continue"
            | "defer"
            | "error"
            | "region"
            | "catch"
            | "open"
            | "as"
            | "unsafe"
            | "thread"
            | "scope"
            | "footprint"
            | "range"
            | "with"
            | "lock"
            // Checked mutex-guard profile keywords. Reserving them keeps
            // `mutex`, `guards`, `protected`, `close` and `borrow` out of name
            // resolution, exactly as `lock`, `open`, `read` and `write` already
            // are.
            | "mutex"
            | "guards"
            | "protected"
            | "close"
            | "borrow"
            // Card T03-F's guard-profile completions: the checked
            // no-write invariant witness (`close g unchanged;`), guard-scoped
            // refinement facts (`let f = fact r;`, `assert f with g;`,
            // `use f;`) and an announced acquisition (`wait m;`). Reserved for
            // the same reason as the words above: they are profile keywords,
            // not references to bindings.
            | "unchanged"
            | "fact"
            | "assert"
            | "use"
            | "wait"
            // Card T03-GUARD-JOIN's guard-local child header
            // (`thread scope c under g { … }`). Reserved for the same reason as
            // every profile keyword above: it is a clause word in a thread-scope
            // header, never a reference to a binding. The guard name that
            // follows it *is* a reference and still resolves normally.
            | "under"
            // Card CT33-RANKS's ranked mutex declaration
            // (`let m = mutex rank 1 guards r;`). Reserved for the same reason
            // as every profile keyword above: it is a clause word in a
            // declaration, never a reference to a binding. Measured
            // load-bearing: without it the ranked declaration collects `rank`
            // as a referenced name and refuses with `Q013`.
            | "rank"
            // Card CT34-GROWTH's protection growth
            // (`grow GUARD guards RESOURCE;`). Reserved for the same reason as
            // every profile keyword above: it is the statement's own keyword,
            // never a reference to a binding. The guard and resource names that
            // follow it *are* references and still resolve normally. Measured
            // load-bearing: without it the statement collects `grow` as a
            // referenced name and refuses with `Q013`.
            | "grow"
            | "spawn"
            | "join"
            | "copy"
            | "read"
            | "write"
            | "share"
            | "mut"
            | "const"
            | "true"
            | "false"
            | "zero"
            | "one"
            | "affine"
            | "relevant"
            | "many"
            | "inf"
            | "FULL"
            | "qcore_buffer_new"
            | "qcore_buffer_read"
            | "qcore_buffer_write"
            // Card T06's payload intrinsic. Reserving it keeps a local binding
            // from shadowing the take form, exactly as the buffer intrinsics
            // above are protected.
            | "qcore_error_payload"
            | "qcore_acquire"
            | "qcore_check"
            // Card `source.extern-call.v1`: the literal outcome control of the
            // nullable-owned-return adapter. Reserved so a local binding named
            // `succeeds` cannot shadow the adapter form, exactly as the
            // mutex-guard keywords above are protected.
            | "succeeds"
            // Card EXTERN-F1: the declared out-slot marker in an extern call's
            // argument list (`CALLEE(out, …) succeeds …`). It names a
            // *parameter position*, never a binding, so reserving it keeps a
            // local named `out` from being read as the slot — and keeps the
            // slot from being read as a reference to a local.
            | "out"
            // Card T08-EPOCH-SRC's flat-epoch accounting profile keywords
            // (`source.epoch.bound.v1`): the scope marker, the declared
            // overhead header and its measured-basis suffix, the per-task
            // quantities and the work-as-time claim. Reserved for the same
            // reason as every profile keyword above: they are clause words in
            // an epoch declaration, never references to a binding. `share`,
            // `bound`, `read`, `write`, `join`, `move`, `drop`, `close`,
            // `thread`, `scope` and `as` are already reserved and are reused.
            | "epoch"
            | "overhead"
            | "peak"
            | "effort"
            | "credit"
            | "retained"
            | "latency"
            | "measured"
            // Card `STD-MODULES`: the `std.io`/`std.logger` statement keywords
            // and the `std.fmt` rendering and level words. Reserved for the
            // same reason as every profile keyword above: they are clause words
            // in a statement, never references to a binding. Measured
            // load-bearing exactly as `grow` and `rank` are — without this the
            // statement collects `print`/`log`/`dec`/… as referenced names and
            // refuses with `Q013`.
            //
            // `error` is already reserved as the error-type declaration word
            // and is interpreted contextually as a level after `log`. The
            // amendment census found no maintained source binding named
            // `trace` or `debug`; reserve them for the current resolver until
            // statement-clause words become contextual throughout.
            | "print"
            | "log"
            | "log_level"
            | "flush"
            | "dec"
            | "hex"
            | "trace"
            | "debug"
            | "info"
            | "warn"
            | "fatal"
            // Card `STD-BUFFER`: `source.buffer.v1`'s acquisition and extent
            // keywords. Reserved for the same reason as every profile keyword
            // above: they are clause words in a `let` initialiser, never
            // references to a binding. Measured load-bearing exactly as `grow`
            // and `rank` are — name resolution collects the tokens after a
            // `let`'s `=`, so without this `let b = bytes(8);` refuses at
            // `Q013` with `bytes` reported as an unknown name.
            //
            // The acquisition is spelled `bytes` and deliberately **not**
            // `buffer`: `buffer` occurs 555 times across the source-side
            // corpus and, unlike a parameter name (which the reference filter
            // above already shields, being followed by `:`), it occurs in
            // *binding* position in shipped fixtures —
            // `let buffer = qcore_buffer_new(1)?;` and `drop buffer;` in
            // `examples/threads/shared_readers.qr`. Reserving it would
            // refuse those files, so this card's syntax was renamed rather
            // than the pinned surface edited, exactly as card T08-EPOCH-SRC
            // renamed `work` to `effort` and card `STD-MODULES` spelled a
            // severity `fatal` rather than `error`. The *module* is still
            // named `std.buffer`: a module name is a dotted path in an
            // `import` declaration, never a reserved word.
            //
            // The release is the **existing** `drop` and the byte accessors
            // are the **existing** `[`/`]`, so this card adds two words and
            // not five.
            | "bytes"
            | "extent"
            // Card `STD-EVENT`: the `std.event` statement keywords
            // (`source.event.v1`). Reserved for the same reason as every
            // profile keyword above: they are clause words in a statement,
            // never references to a binding, and without this the statements
            // collect `dispatch`/`ready`/`idle`/… as referenced names and
            // refuse with `Q013`.
            //
            // `open`, `close`, `read` and `write` are already reserved and are
            // reused rather than respelled, which is why the context pair is
            // spelled `event open;` / `event close;` — two fewer words than a
            // dedicated pair would have cost. The constraint-14 census over
            // every `.qcore`/`.qc` fixture and every inline source string found
            // no collision: the corpus occurrences of `event` are ghost-field
            // names inside `contract { … }` clauses and prose comments, never a
            // statement head and never a binding.
            | "event"
            | "endpoints"
            | "watch"
            | "unwatch"
            | "wake"
            | "dispatch"
            | "ready"
            | "idle"
            // Card `QUIRIN-SOCKET-V1`: the `std.socket` statement keywords
            // (`source.socket.v1`). Reserved for the same reason as every
            // profile keyword above: they are clause words in a statement or in
            // a `let` initialiser, never references to a binding, and without
            // this `let s = tcp;` and `bind s;` collect `tcp`/`bind` as
            // referenced names and refuse with `Q013`.
            //
            // `let`, `close`, `return` and the standard-IO line words are
            // already reserved and are reused rather than respelled, which is
            // why the release is spelled `close NAME;`. The constraint-14
            // census ran over every `.qcore`/`.qc` fixture in the tree (412
            // files) and every Rust string literal (41,529): none of the ten
            // occurs in any QCore source position, the only occurrences at all
            // being `send` and `bind` inside `//` comments of the contract
            // corpus and Rust-side diagnostic text. `stream` was rejected as a
            // candidate spelling by that same census, occurring as a contract
            // `extern fn` parameter name.
            | "tcp"
            | "udp"
            | "bind"
            | "listen"
            | "connect"
            | "accept"
            | "send"
            | "recv"
            | "sendto"
            | "recvfrom"
            // Card `CT42-CELL-ROUTE`: the disjointness question `disjoint NAME
            // NAME;` of `source.socket.v1`. Reserved for the same reason as the
            // ten above: a clause word at a statement head, never a reference,
            // and without this the statement collects `disjoint` as a
            // referenced name and refuses at `Q013`. The constraint-14 census
            // ran over 525 `.qcore`/`.qc` files and every Rust string literal:
            // zero occurrences in any QCore source position (eleven inside
            // `//` comments), thirteen Rust literals that are test names,
            // `expect` messages or prose. The already-reserved `assert` and
            // `footprint` were considered and not taken (see
            // `source_socket.rs`).
            | "disjoint"
            // Card `HTTP-QUIRIN-V1`: the `std.http` statement keywords
            // (`source.http.v1`). Reserved for the same reason as every profile
            // keyword above: they are clause words at a statement head or in a
            // `let` initialiser, never references to a binding, and without this
            // `let r = request "GET" "/" "HTTP/1.1";` and `header r "Host" "x";`
            // collect `request`/`header` as referenced names and refuse at
            // `Q013`.
            //
            // `send`, `recv`, `let`, `close` and the standard-IO line words are
            // already reserved and are reused rather than respelled: a message
            // goes on a wire through the socket vocabulary that already exists,
            // so this card adds **four** words and not six.
            //
            // The constraint-14 census ran over 416 `.qcore`/`.qc` fixtures and
            // 43,239 Rust string literals. `request`, `response`, `header` and
            // `content` occur **zero** times in any QCore source position.
            // `body` — the obvious word for a message body — was **rejected**:
            // it occurs 8 times in source position as a *binding name* in two
            // shipped fixtures, referenced by `print body;` and `len(body)`, so
            // reserving it would refuse programs that ship today. RFC 9110 §6.4
            // renamed that concept **content**, so the word the census admits is
            // also the specification's current one. `message` was rejected the
            // same way (a contract `extern fn` parameter, referenced in an
            // `ensures` clause) and `payload` at 49 occurrences.
            | "request"
            | "response"
            | "header"
            | "content"
    )
}

fn statement_metadata(statement: &StatementKind) -> (Option<&str>, Option<&str>, SemanticPass) {
    match statement {
        StatementKind::Let { binding } => (binding.as_deref(), None, SemanticPass::OrdinaryTypes),
        StatementKind::LetIndex { binding } => (Some(binding), None, SemanticPass::Index),
        StatementKind::Assignment { place } => (None, place.as_deref(), SemanticPass::Ownership),
        StatementKind::Expression => (None, None, SemanticPass::Usage),
        StatementKind::Drop { place } => (None, place.as_deref(), SemanticPass::ResourceFlow),
        StatementKind::Move { place } => (None, place.as_deref(), SemanticPass::Ownership),
        StatementKind::Discharge { binding } => {
            (None, binding.as_deref(), SemanticPass::ErrorRegion)
        }
        StatementKind::Guard => (None, None, SemanticPass::Index),
        // Card `IF-STATEMENT`: `if` is the branch pass's own statement. `match`
        // and `while` are untouched and keep `Usage`, because nothing admits
        // them and this card does not claim them.
        StatementKind::If { .. } => (None, None, SemanticPass::Branch),
        StatementKind::Match { .. } | StatementKind::While { .. } => {
            (None, None, SemanticPass::Usage)
        }
        // Card `LOOP-V1`: `for` is the loop pass's own statement, on the
        // `IF-STATEMENT` precedent that gave `if` the branch pass. The binding
        // is introduced into the body's scope by `elaborate_block`, which reads
        // it off the variant.
        StatementKind::For { .. } => (None, None, SemanticPass::Loop),
        // Card `LOOP-UNROLL-BUDGET`: the unrolling attribute is the loop pass's
        // statement too — it licenses a loop and nothing else, and its refusals
        // are conditions of that pass's existing `Q097` and `Q098`.
        StatementKind::LoopAttribute => (None, None, SemanticPass::Loop),
        StatementKind::Return => (None, None, SemanticPass::OrdinaryTypes),
        StatementKind::Raise => (None, None, SemanticPass::TypedErrors),
        StatementKind::Break | StatementKind::Continue => (None, None, SemanticPass::ResourceFlow),
        StatementKind::Defer => (None, None, SemanticPass::CleanupLowering),
        StatementKind::ErrorRegion { .. } => (None, None, SemanticPass::ErrorRegion),
        StatementKind::Unsafe { .. } => (None, None, SemanticPass::CEmission),
        StatementKind::ThreadScope { .. }
        | StatementKind::Footprint { .. }
        | StatementKind::WithLock { .. } => (None, None, SemanticPass::Concurrency),
        // The bindings are introduced by `elaborate_block`, which reads them
        // off the variant: this tuple carries at most one, and an endpoint pair
        // introduces two.
        StatementKind::Event { .. } => (None, None, SemanticPass::Event),
    }
}

/// Converts an error from any established semantic API into a stable,
/// source-located diagnostic.  The individual passes retain their detailed
/// error types; this is the shared Cycle 10 presentation boundary.
pub fn source_diagnostic(
    pass: SemanticPass,
    span: Span,
    error: &(impl std::error::Error + ?Sized),
) -> SourceDiagnostic {
    let code = match pass {
        SemanticPass::NameResolution => "Q010",
        SemanticPass::OrdinaryTypes => "Q020",
        SemanticPass::Index => "Q001",
        SemanticPass::Usage => "Q100",
        SemanticPass::Ownership => "Q103",
        SemanticPass::ResourceFlow => "Q200",
        SemanticPass::TypedErrors => "Q300",
        SemanticPass::ErrorRegion => "Q302",
        SemanticPass::CleanupLowering => "Q202",
        SemanticPass::CEmission => "Q500",
        SemanticPass::Concurrency => "Q600",
        // The generic presentation code of the extern-call pass. The specific
        // conditions Q023-Q027 are issued at their own emission sites.
        SemanticPass::ExternCall => "Q022",
        // The generic presentation code of the standard-IO pass. The specific
        // conditions Q028-Q031 are issued at their own emission sites.
        SemanticPass::StandardIo => "Q030",
        // The generic presentation code of the unchecked-linkage pass. The
        // specific conditions Q032, Q034 and Q035 are issued at their own
        // emission sites; Q033 is the table refusal this code names.
        SemanticPass::UncheckedLinkage => "Q033",
        // The generic presentation code of the pipe pass. The specific
        // conditions Q037-Q038 are issued at their own emission sites.
        SemanticPass::Pipe => "Q036",
        // The generic presentation code of the native-library pass. The
        // specific conditions Q040-Q042 are issued at their own emission
        // sites; Q039 is the manifest load rejection this code names.
        SemanticPass::NativeLibrary => "Q039",
        // The generic presentation code of the branch pass, and the code of
        // every shape refusal in it. The specific conditions Q045 (a condition
        // outside the decidable tier) and Q046 (the arms disagree at the join)
        // are issued at their own emission sites.
        SemanticPass::Branch => "Q044",
        // The one code of the composed-route pass. It has no band: every other
        // refusal a composed module receives is one of the two composed routes'
        // own.
        SemanticPass::ComposedStdIo => "Q043",
        // The one code of the composed task pass. It has no band either: a
        // composed task module's other refusals are the task route's and the
        // standard-IO route's own.
        SemanticPass::ComposedTasks => "Q050",
        // The generic presentation code of the buffer pass, and the code of
        // every shape refusal in it. The specific conditions Q052 (an operand
        // that is not a check-time constant), Q053 (a bounds, byte-range or
        // extent-range violation), Q054 (a refusal the inherited ownership
        // checker reported), Q055 (a buffer at a position it may not cross) and
        // Q056 (a failed certificate rederivation) and Q057 (an undeclared
        // derived effect) are issued at their own emission sites.

        // `Q057` is deliberately this pass's own rather than a reuse of
        // `StandardIo`'s `Q029`: §L21 forbids a second pass reusing a code, and
        // the conditions differ besides — `Q029` reports an undeclared `Io` on a
        // route that derives nothing else, while `Q057` reports an undeclared
        // `Alloc` or `Mut` too.
        SemanticPass::StandardBuffer => "Q051",
        // The generic presentation code of the event pass, and the code of
        // every shape refusal in it. The specific conditions Q059 (a name that
        // is not a live endpoint or registration), Q060 (the reactor model's
        // own refusal), Q061 (the determinism boundary), Q062 (the effect
        // row) and Q074 (the composed line and value vocabulary's own
        // refusals, card `COMPOSE-EVENT-LINES`) are issued at their own
        // emission sites.
        SemanticPass::Event => "Q058",
        // The generic presentation code of the string pass, and the code of
        // every shape refusal in it. The specific conditions Q069 (a bound that
        // is not a check-time constant, or a view position naming no string),
        // Q070 (a quoted token that does not decode to a well-formed UTF-8
        // payload), Q071 (a slice outside `0 <= low <= high <= len` or off a
        // code-point boundary), Q072 (a string at a position it may not cross)
        // Q073 (an undeclared derived effect) and the second band's Q075 (the
        // shape of a rendered-text line) and Q076 (a text operand naming no
        // string this entry bound, card `STRING-SLICE-IO`) are issued at their
        // own emission sites, and Q068 is also the code of this route's failed
        // certificate rederivation.
        SemanticPass::StandardString => "Q068",
        // The generic presentation code of the socket pass, and the code of
        // every shape refusal in it. The specific conditions Q081 (an operand
        // that is not a check-time constant, or a custody position naming no
        // socket), Q082 (the transport state machine's own refusal), Q083 (a
        // refusal the inherited ownership checker reported), Q084 (a byte,
        // payload length or receive count out of range), Q085 (the determinism
        // boundary — a statement this program's own committed history does not
        // satisfy), Q086 (a socket at a position it may not cross), Q087 (an
        // undeclared derived effect), Q088 (a failed certificate rederivation)
        // and Q089 (a backend refusal) are issued at their own emission sites.
        SemanticPass::StandardSocket => "Q080",
        // The generic presentation code of the HTTP pass, and the code of every
        // shape refusal in it — including a statement spelling neither this
        // route nor its transport models, and a quoted token that does not
        // decode. The specific conditions Q091 (a start-line component RFC 9112
        // does not admit), Q092 (a header field, including the required `Host`
        // and the connection options v1 honours), Q093 (the framing rule of
        // RFC 9112 §6, of which the `Content-Length`/content-length equality is
        // the card's signature check), Q094 (a name at a message position that
        // is not one), Q095 (a refusal the inherited, unedited
        // `source.socket.v1` checker reported) and Q096 (an undeclared derived
        // effect) are issued at their own emission sites.
        SemanticPass::StandardHttp => "Q090",
        // The generic presentation code of the loop pass, and the code of every
        // shape refusal in it — a `for` header this construct does not model, a
        // body statement outside the admitted subset (including `break` and
        // `continue`), and a body binding that shadows one the entry already
        // made. The specific conditions Q098 (the bound — not a check-time
        // constant, zero, above the iteration maximum, or above the unrolled
        // budget) and Q099 (one iteration's net custody effect is not the
        // identity) are issued at their own emission sites.
        SemanticPass::Loop => "Q097",
        SemanticPass::PluginVerification => "Q700",
    };
    SourceDiagnostic {
        code,
        pass,
        span,
        message: error.to_string(),
    }
}

pub fn covered_semantic_passes(program: &TypedSourceProgram) -> BTreeSet<SemanticPass> {
    fn collect(statement: &TypedStatement, result: &mut BTreeSet<SemanticPass>) {
        result.insert(statement.pass);
        for nested in &statement.nested {
            collect(nested, result);
        }
    }
    let mut result = BTreeSet::new();
    for declaration in &program.declarations {
        result.insert(SemanticPass::NameResolution);
        for statement in &declaration.statements {
            collect(statement, &mut result);
        }
    }
    result
}

#[cfg(test)]
#[path = "../tests/unit/frontend.rs"]
mod tests;
