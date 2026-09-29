//! Exact source lowering for bounded copy-scalar and owned-buffer scoped tasks.
//!
//! Structural task records are not admitted on their own: this pass consumes
//! complete statements, checks every declaration and resolves immutable slots.
//! The returned core IR still requires independent `check_task_module` validation.
use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::composed_tasks::TaskIoSite;

/// Detect source task forms in every definition, including empty/nested scopes
/// and uncalled bodies. This selects a checker, never grants execution support.
pub fn has_source_tasks(source: &SourceProgram) -> bool {
    fn has_scope(block: &SourceBlock) -> bool {
        block.statements.iter().any(|s| {
            matches!(s.value, StatementKind::ThreadScope { .. })
                || s.value.nested_blocks().iter().any(|b| has_scope(b))
        })
    }
    // The `in_library_binding` exclusion is card
    // `NATIVE-LIBRARY-SOURCE-BINDING`'s constraint-14 census outcome: this is
    // one of the two whole-token-stream route selectors, so a library named
    // `qcore_buffer_new` would otherwise select the task route from its own
    // spelling in an `extern library NAME;` declaration.
    source.lexical_tokens().iter().any(|token| !source.in_library_binding(token.span) && matches!(&token.kind, TokenKind::Word(word) if word == "qcore_buffer_new" || word == "qcore_buffer_read" || word == "qcore_buffer_write"))
        || !source.task_expressions().is_empty()
        || source
            .declarations
            .iter()
            .filter_map(|d| d.value.body.as_ref())
            .any(has_scope)
}

fn failure(span: Span, pass: SemanticPass, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: match pass {
            SemanticPass::NameResolution => "Q013",
            SemanticPass::OrdinaryTypes => "Q020",
            SemanticPass::TypedErrors => "Q300",
            SemanticPass::Concurrency => "Q600",
            SemanticPass::ComposedTasks => "Q050",
            _ => "Q500",
        },
        pass,
        span,
        message: message.into(),
    }
}

fn tokens_for(source: &SourceProgram, span: Span) -> Vec<Token> {
    source
        .lexical_tokens()
        .iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .cloned()
        .collect()
}

/// The top-level comma positions of a call's argument list, which is what
/// selects between an arity this route already had and one card
/// `WRITE-LEND-TASKS-EXERCISE` adds. Nesting is tracked so a comma inside a
/// parenthesized operand cannot be mistaken for an argument separator.
fn argument_commas(tokens: &[Token]) -> Vec<usize> {
    let Some(close) = matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen) else {
        return Vec::new();
    };
    let mut depth = 0usize;
    let mut commas = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(close).skip(2) {
        if symbol(Some(token), Symbol::LeftParen) {
            depth += 1;
        } else if symbol(Some(token), Symbol::RightParen) {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && symbol(Some(token), Symbol::Comma) {
            commas.push(index);
        }
    }
    commas
}

fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Word(w)) if w == expected)
}

fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == expected)
}

fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(w) => Some(w),
        _ => None,
    }
}

fn matching(tokens: &[Token], start: usize, open: Symbol, close: Symbol) -> Option<usize> {
    if !symbol(tokens.get(start), open) {
        return None;
    }
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if symbol(Some(token), open) {
            depth += 1;
        }
        if symbol(Some(token), close) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[derive(Clone, Debug, Default)]
struct ParsedEffects {
    alloc: bool,
    spawn: bool,
    join: bool,
    may_block: bool,
    /// Card `COMPOSE-CONCURRENCY`. Admitted **only** when this module is on the
    /// composed route; on the task route alone `io` is still the unsupported
    /// effect it has always been, refused at `Q020`, so a module that spawns and
    /// prints without composing keeps its exact refusal.
    io: bool,
    errors: BTreeSet<String>,
    /// Card T06. The declared subset of `errors` whose variants carry exactly
    /// one `own<Buffer>`. It is derived from the error type declaration, never
    /// from a use site, so one tag cannot carry a payload in one function and
    /// not in another.
    error_payloads: BTreeSet<String>,
}

/// A declared nominal error type: its complete variant set and the subset that
/// carries one owned Buffer.
#[derive(Clone, Debug, Default)]
struct ErrorType {
    variants: BTreeSet<String>,
    payloads: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scalar {
    OwnedBuffer,
    /// `borrow<Buffer>`: a scoped read loan. Only a worker parameter can have
    /// this type, and only a `read … share FULL` capture can produce one.
    ReadLoan,
    /// `lend<Buffer>`: a scoped range write lease (card `WRITE-LEND-TASKS`).
    /// Only a worker parameter can have this type, and only a
    /// `lend PLACE LOW..HIGH` capture can produce one. `lend` stays
    /// contextual: it is recognised only immediately before `<Buffer>` in a
    /// task parameter type, a spelling that refused before this card.
    WriteLease,
    I32,
    /// Every other type of the §G02t fixed-width table (card
    /// `W48-QR-SCALAR-TYPES-R4`); `i32` keeps its own variant.
    Fixed(FixedType),
    Unit,
}

use qcore_core::scalar_types::{RecordedTarget, ScalarType as FixedType};

/// A §G02t type name as a task scalar: `i32` is [`Scalar::I32`], every other
/// name of the table [`Scalar::Fixed`].
fn table_scalar(name: &str) -> Option<Scalar> {
    FixedType::parse(name).map(|ty| {
        if ty == FixedType::I32 {
            Scalar::I32
        } else {
            Scalar::Fixed(ty)
        }
    })
}

fn scalar(tokens: &[Token]) -> Option<Scalar> {
    match tokens {
        [own, lt, buffer, gt]
            if word(Some(own), "own")
                && symbol(Some(lt), Symbol::Less)
                && word(Some(buffer), "Buffer")
                && symbol(Some(gt), Symbol::Greater) =>
        {
            Some(Scalar::OwnedBuffer)
        }
        [borrow, lt, buffer, gt]
            if word(Some(borrow), "borrow")
                && symbol(Some(lt), Symbol::Less)
                && word(Some(buffer), "Buffer")
                && symbol(Some(gt), Symbol::Greater) =>
        {
            Some(Scalar::ReadLoan)
        }
        [lend, lt, buffer, gt]
            if word(Some(lend), "lend")
                && symbol(Some(lt), Symbol::Less)
                && word(Some(buffer), "Buffer")
                && symbol(Some(gt), Symbol::Greater) =>
        {
            Some(Scalar::WriteLease)
        }
        [t] if word(Some(t), "unit") => Some(Scalar::Unit),
        [t] => name(Some(t)).and_then(table_scalar),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct Signature {
    id: SymbolId,
    name: String,
    parameters: Vec<(SymbolId, String, Scalar)>,
    result: Scalar,
    effects: ParsedEffects,
}

fn error_declarations(
    source: &SourceProgram,
) -> Result<BTreeMap<String, ErrorType>, SourceDiagnostic> {
    let builtin = |tag: &str| ErrorType {
        variants: BTreeSet::from([tag.to_owned()]),
        // Both are raised by the runtime after a failure that acquired
        // nothing, so neither can ever hold a payload.
        payloads: BTreeSet::new(),
    };
    let mut errors = BTreeMap::from([
        ("AllocError".to_owned(), builtin("AllocError")),
        ("SpawnError".to_owned(), builtin("SpawnError")),
        // Card `W48-QR-SCALAR-TYPES-R4`: the §G02t builtin, payload-free, one
        // tag per variant — what a value statement raises on a fault.
        (
            qcore_core::scalar_types::ArithmeticError::NAME.to_owned(),
            ErrorType {
                variants: qcore_core::scalar_types::ArithmeticError::ALL
                    .iter()
                    .map(|error| error.to_string())
                    .collect(),
                payloads: BTreeSet::new(),
            },
        ),
    ]);
    for declaration in &source.declarations {
        if declaration.value.kind != DeclarationKind::ErrorType {
            continue;
        }
        let tokens = tokens_for(source, declaration.span);
        let bad = || {
            failure(
                declaration.span,
                SemanticPass::TypedErrors,
                "task execution supports nominal error variants that are payload-free or carry exactly one own<Buffer>",
            )
        };
        if tokens.len() < 6
            || !word(tokens.first(), "error")
            || !symbol(tokens.get(2), Symbol::LeftBrace)
            || !symbol(tokens.last(), Symbol::RightBrace)
            || declaration.value.name == "SpawnError"
        {
            return Err(bad());
        }
        let mut declared = ErrorType::default();
        let inner = &tokens[3..tokens.len() - 1];
        let mut cursor = 0;
        while cursor < inner.len() {
            let Some(variant) = name(inner.get(cursor)) else {
                return Err(bad());
            };
            let tag = format!("{}::{variant}", declaration.value.name);
            // `V;` is payload-free; `V(own<Buffer>);` carries exactly one owned
            // Buffer. Nothing else is admitted, so a scalar, a borrow, several
            // fields and an empty list each reject rather than degrade.
            let payload = if symbol(inner.get(cursor + 1), Symbol::Semicolon) {
                cursor += 2;
                false
            } else if symbol(inner.get(cursor + 1), Symbol::LeftParen)
                && inner.len() > cursor + 7
                && scalar(&inner[cursor + 2..cursor + 6]) == Some(Scalar::OwnedBuffer)
                && symbol(inner.get(cursor + 6), Symbol::RightParen)
                && symbol(inner.get(cursor + 7), Symbol::Semicolon)
            {
                cursor += 8;
                true
            } else {
                return Err(bad());
            };
            if !declared.variants.insert(tag.clone()) {
                return Err(bad());
            }
            if payload {
                declared.payloads.insert(tag);
            }
        }
        if declared.variants.is_empty()
            || errors
                .insert(declaration.value.name.clone(), declared)
                .is_some()
        {
            return Err(bad());
        }
    }
    Ok(errors)
}

fn signature(
    source: &SourceProgram,
    declaration: &Located<SourceDeclaration>,
    typed: &TypedSourceDeclaration,
    errors: &BTreeMap<String, ErrorType>,
    composing: bool,
) -> Result<Signature, SourceDiagnostic> {
    let bad = |message| failure(declaration.span, SemanticPass::OrdinaryTypes, message);
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| bad("task functions require source bodies"))?;
    if declaration.value.function_contract.is_some() {
        return Err(bad(
            "source contracts require an implemented consumer for task execution",
        ));
    }
    let header_span = Span {
        start: declaration.span.start,
        end: body.span.start,
    };
    let tokens = tokens_for(source, header_span);
    if !word(tokens.first(), "fn")
        || name(tokens.get(1)) != Some(declaration.value.name.as_str())
        || !symbol(tokens.get(2), Symbol::LeftParen)
    {
        return Err(bad(
            "generic/unsupported function headers are outside task execution",
        ));
    }
    let close = matching(&tokens, 2, Symbol::LeftParen, Symbol::RightParen)
        .ok_or_else(|| bad("invalid task parameter list"))?;
    let groups = if close == 3 {
        Vec::new()
    } else {
        tokens[3..close]
            .split(|token| symbol(Some(token), Symbol::Comma))
            .collect::<Vec<_>>()
    };
    if groups.len() != typed.parameters.len() {
        return Err(bad("task parameter metadata disagrees with the source"));
    }
    let mut parameters = Vec::new();
    for (group, parameter) in groups.iter().zip(&typed.parameters) {
        if name(group.first()) != Some(parameter.name.as_str())
            || !symbol(group.get(1), Symbol::Colon)
        {
            return Err(bad("task parameters require explicit types"));
        }
        let kind = if group.len() == 3
            && let Some(kind) = name(group.get(2)).and_then(table_scalar)
            && parameter.usage.is_none()
        {
            // Card `W48-QR-SCALAR-TYPES-R4`: every §G02t type, `i32` as before.
            kind
        } else if group.len() == 8
            && scalar(&group[2..6]) == Some(Scalar::OwnedBuffer)
            && word(group.get(6), "use")
            && word(group.get(7), "one")
            && parameter
                .usage
                .as_ref()
                .is_some_and(|usage| usage.value == qcore_core::usage::Usage::ONE)
        {
            Scalar::OwnedBuffer
        } else if group.len() == 6
            && scalar(&group[2..6]) == Some(Scalar::ReadLoan)
            && parameter.usage.is_none()
        {
            // A read loan carries no usage grade in this profile: the borrower
            // may read it any number of times and may do nothing else with it.
            // Grading loan reads is deliberately left open.
            Scalar::ReadLoan
        } else if group.len() == 6
            && scalar(&group[2..6]) == Some(Scalar::WriteLease)
            && parameter.usage.is_none()
        {
            // A write lease carries no usage grade either: in this first
            // slice the holder performs no operation on it at all — the
            // authority is held and enforced against the owner, and the
            // worker-side range write is the named residual
            // `WRITE-LEND-TASKS-EXERCISE`.
            Scalar::WriteLease
        } else {
            return Err(bad(
                "task parameters require i32, own<Buffer> use one, an ungraded borrow<Buffer>, \
                 or an ungraded lend<Buffer>",
            ));
        };
        parameters.push((parameter.symbol, parameter.name.clone(), kind));
    }
    if !symbol(tokens.get(close + 1), Symbol::Arrow) {
        return Err(bad("task functions require an explicit result type"));
    }
    let result_end = tokens
        .iter()
        .enumerate()
        .skip(close + 2)
        .find_map(|(i, t)| word(Some(t), "effects").then_some(i))
        .unwrap_or(tokens.len());
    let result = scalar(&tokens[close + 2..result_end])
        .filter(|kind| *kind != Scalar::ReadLoan && *kind != Scalar::WriteLease)
        .ok_or_else(|| bad("task functions support i32, unit and own<Buffer> results"))?;
    let mut cursor = result_end;
    let mut effects = ParsedEffects::default();
    if cursor < tokens.len() {
        if !word(tokens.get(cursor), "effects")
            || !symbol(tokens.get(cursor + 1), Symbol::LeftBrace)
        {
            return Err(bad("unsupported function annotation in task execution"));
        }
        let last = matching(&tokens, cursor + 1, Symbol::LeftBrace, Symbol::RightBrace)
            .ok_or_else(|| bad("invalid task effect row"))?;
        if last + 1 != tokens.len() {
            return Err(bad(
                "trailing function annotations are not supported by task execution",
            ));
        }
        cursor += 2;
        let mut seen = BTreeSet::new();
        while cursor < last {
            let effect = name(tokens.get(cursor)).ok_or_else(|| bad("invalid task effect"))?;
            let key;
            match effect {
                "alloc" => {
                    effects.alloc = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "spawn" => {
                    effects.spawn = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "join" => {
                    effects.join = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "may_block" => {
                    effects.may_block = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                // The one widening card `COMPOSE-CONCURRENCY` makes to this
                // grammar, and it is gated: off the composed route the arm is
                // not taken and `io` falls to the refusal below, byte for byte.
                "io" if composing => {
                    effects.io = true;
                    key = effect.to_owned();
                    cursor += 1;
                }
                "throws" => {
                    if !symbol(tokens.get(cursor + 1), Symbol::Less)
                        || !symbol(tokens.get(cursor + 3), Symbol::Greater)
                    {
                        return Err(bad("throws requires one declared nominal error type"));
                    }
                    let error = name(tokens.get(cursor + 2))
                        .ok_or_else(|| bad("throws requires a nominal error type"))?;
                    let declared = errors
                        .get(error)
                        .ok_or_else(|| bad("throws references an unknown error type"))?;
                    effects.errors.extend(declared.variants.iter().cloned());
                    effects
                        .error_payloads
                        .extend(declared.payloads.iter().cloned());
                    key = format!("throws<{error}>");
                    cursor += 4;
                }
                _ => return Err(bad("unsupported effect in task execution")),
            }
            if !seen.insert(key) {
                return Err(bad("duplicate task effect"));
            }
            if cursor == last {
                break;
            }
            if !symbol(tokens.get(cursor), Symbol::Comma) {
                return Err(bad("task effects must be separated by commas"));
            }
            cursor += 1;
            if cursor == last {
                return Err(bad("missing effect after comma"));
            }
        }
    }
    // Card T06's bound on the *declared row*, not on a path. When some declared
    // variant carries a payload, every one of them must, so a joined error
    // always holds exactly one Buffer and the caller's take is total. A mixed
    // row would make the payload's presence depend on which variant occurred,
    // which this profile does not represent. It is a refusal, not a silent
    // widening: the core would still reject the take on the payload-free path.
    if !effects.error_payloads.is_empty() && effects.error_payloads != effects.errors {
        return Err(failure(
            declaration.span,
            SemanticPass::TypedErrors,
            "a declared error row must carry an owned payload on every variant or on none",
        ));
    }
    Ok(Signature {
        id: typed.symbol,
        name: typed.name.clone(),
        parameters,
        result,
        effects,
    })
}

use qcore_core::prelude::concurrency::task_execution::{
    ErrorOperand, FULL_READ_AUTHORITY, ScalarOperand, ScalarType, TaskBlock, TaskEffects,
    TaskFunction, TaskInstruction, TaskInstructionKind, TaskModule, TaskSpan, TaskTerminator,
    TaskTerminatorKind,
};

impl Scalar {
    fn core(self) -> ScalarType {
        match self {
            Self::OwnedBuffer => ScalarType::OwnedBuffer,
            Self::ReadLoan => ScalarType::ReadLoan,
            Self::WriteLease => ScalarType::WriteLease,
            Self::I32 => ScalarType::I32,
            Self::Fixed(ty) => ScalarType::Fixed(ty),
            Self::Unit => ScalarType::Unit,
        }
    }
}

/// Evaluate a source read share into the ledger's authority unit.
///
/// `FULL` is the whole read authority; a decimal literal and a sum of admitted
/// shares are the fractions of it. Every other form is a represented refusal:
/// a named share needs a declared share constant, which this profile does not
/// have. A share of zero grants nothing and a share above the full authority is
/// not a fraction of it, so both are refused here rather than clamped — the
/// ledger repeats both checks, so this is the first of two gates, not the only
/// one.
fn read_share(share: &SourceShareExpression, span: Span) -> Result<u32, SourceDiagnostic> {
    let value = match share {
        SourceShareExpression::Full => FULL_READ_AUTHORITY,
        SourceShareExpression::Parenthesized(inner) => read_share(&inner.value, inner.span)?,
        SourceShareExpression::Add(left, right) => read_share(&left.value, left.span)?
            .checked_add(read_share(&right.value, right.span)?)
            .ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::Concurrency,
                    "read share sum is outside the representable authority range",
                )
            })?,
        SourceShareExpression::Integer(digits) => digits.parse::<u32>().map_err(|_| {
            failure(
                span,
                SemanticPass::Concurrency,
                "a read share literal must be a decimal fraction of the full authority",
            )
        })?,
        SourceShareExpression::Name(_) => {
            return Err(failure(
                span,
                SemanticPass::Concurrency,
                "named read shares require a declared share constant, which is outside this profile",
            ));
        }
    };
    if value == 0 || value > FULL_READ_AUTHORITY {
        return Err(failure(
            span,
            SemanticPass::Concurrency,
            format!(
                "a read share must be positive and at most FULL ({}); this capture requests {value}",
                FULL_READ_AUTHORITY
            ),
        ));
    }
    Ok(value)
}

fn core_span(span: Span) -> TaskSpan {
    TaskSpan {
        start: span.start,
        end: span.end,
    }
}

/// Evaluate one bound of a `lend PLACE LOW..HIGH` capture (card
/// `WRITE-LEND-TASKS`). The bound tier is deliberately the narrowest of the
/// family: an integer literal only, because the crossing's custody projection
/// must *state* the lease's footprint and the disjointness question must be
/// decided at check time. A name is refused here as a **varying bound** — by
/// its own spelling, at the boundary, not as a parse failure — and the
/// widening to a constant-binding tier is the named residual
/// `WRITE-LEND-TASKS-VARYING-BOUND`.
fn lease_bound_value(bound: &Located<SourceLeaseBound>) -> Result<u64, SourceDiagnostic> {
    match &bound.value {
        SourceLeaseBound::Integer(digits) => digits.parse::<u64>().map_err(|_| {
            failure(
                bound.span,
                SemanticPass::Concurrency,
                "a write lend bound must be a decimal byte offset",
            )
        }),
        SourceLeaseBound::Name(name) => Err(failure(
            bound.span,
            SemanticPass::Concurrency,
            format!(
                "`{name}` is not a bound this crossing can decide: a write lend's range is \
                 decided at check time, from integer literals, so the disjointness of two \
                 workers' ranges is a decision rather than a premise; a varying bound is \
                 the named residual `WRITE-LEND-TASKS-VARYING-BOUND`"
            ),
        )),
    }
}

// ---------------------------------------------------------------------------
// Card CT46-SENDSYNC: the transferability judgment, consulted redundantly.
//
// `qcore_core::transfer` derives which kinds may cross a task-creation
// boundary from the kind itself. It is consulted here *beside* the existing
// enumeration in `spawn_inputs`, and the two must agree; see that function.
// Neither side is preferred, so this cannot widen admission — a disagreement
// in either direction is a refusal.
// ---------------------------------------------------------------------------

/// The elaborated scalar grammar, as a transfer kind.
///
/// This is total over `Scalar`, so a new executable scalar spelling must be
/// classified before it can reach a boundary.
fn transfer_kind(scalar: Scalar) -> qcore_core::transfer::TransferKind {
    use qcore_core::transfer::TransferKind;
    match scalar {
        Scalar::I32 => TransferKind::I32,
        Scalar::Fixed(_) => TransferKind::FixedScalar,
        Scalar::Unit => TransferKind::Unit,
        Scalar::OwnedBuffer => TransferKind::OwnedBuffer,
        Scalar::ReadLoan => TransferKind::ReadLoan,
        Scalar::WriteLease => TransferKind::WriteLease,
    }
}

/// The lowering's binding grammar, as a transfer kind. Total over
/// [`ValueType`].
///
/// Only the scalar cases can reach the capture match, because a non-scalar
/// place is already refused by the place/parameter type check. The other arms
/// exist so the guard in `spawn_inputs` can state — and a test can check —
/// that no non-scalar binding is ever sendable.
fn binding_transfer_kind(kind: &ValueType) -> qcore_core::transfer::TransferKind {
    use qcore_core::transfer::TransferKind;
    match kind {
        ValueType::Scalar(scalar) => transfer_kind(*scalar),
        ValueType::Handle(_) => TransferKind::TaskHandle,
        ValueType::Result { .. } => TransferKind::TaskResult,
        ValueType::Scope => TransferKind::ThreadScope,
        ValueType::KnownError(_) => TransferKind::KnownErrorTag,
        ValueType::Error { .. } => TransferKind::ErrorValue,
    }
}

/// The capture keywords, as transfer modes. Total over
/// [`SourceCaptureMode`], `Write` included: the keyword parses and has never
/// had a lowering, and the judgment refuses it by name rather than by
/// falling off the end of a match.
fn transfer_mode(mode: &SourceCaptureMode) -> qcore_core::transfer::TransferMode {
    use qcore_core::transfer::TransferMode;
    match mode {
        SourceCaptureMode::Copy => TransferMode::Copy,
        SourceCaptureMode::Move => TransferMode::Move,
        SourceCaptureMode::Read { .. } => TransferMode::Lend,
        SourceCaptureMode::Write => TransferMode::Write,
        SourceCaptureMode::LendWrite { .. } => TransferMode::LendWrite,
    }
}

// ---------------------------------------------------------------------------
// Card CT47-COPY-CAPTURE: the judgment's refusal, spelled at the boundary.
//
// CT46-SENDSYNC wired the judgment in beside the enumeration and deliberately
// moved nothing: a capture both sides refused still fell through to the
// enumeration's executable-subset catch-all in `spawn_inputs`, attributed to
// `CEmission` (`Q500`) — a statement about what the backend can encode, not
// about why the value may not cross. This card moves that attribution: where
// the judgment and the enumeration agree the capture is refused, the refusal
// is now the judgment's own, attributed to the crossing construct at
// `Concurrency` (`Q600`; a condition widening stated in spec §L21), with a
// message naming the missing transfer/share contract and the contracts the
// kind does carry. The grammar gate (`Q000`, a capture with no declared mode)
// and the catch-all itself are untouched: the catch-all stays as the
// fail-closed backstop for anything the judgment does not decide first.
// ---------------------------------------------------------------------------

/// The source spelling of an executable scalar, for the judgment's refusal
/// messages. Total over `Scalar`.
fn scalar_spelling(scalar: Scalar) -> &'static str {
    match scalar {
        Scalar::I32 => "`i32`",
        Scalar::Fixed(ty) => fixed_spelling(ty),
        Scalar::Unit => "`unit`",
        Scalar::OwnedBuffer => "`own<Buffer>`",
        Scalar::ReadLoan => "`borrow<Buffer>`",
        Scalar::WriteLease => "`lend<Buffer>`",
    }
}

/// The source spelling of a fixed-width type, for the judgment's refusal
/// messages.
fn fixed_spelling(ty: FixedType) -> &'static str {
    match ty {
        FixedType::Bool => "`bool`",
        FixedType::I8 => "`i8`",
        FixedType::I16 => "`i16`",
        FixedType::I32 => "`i32`",
        FixedType::I64 => "`i64`",
        FixedType::Isize => "`isize`",
        FixedType::U8 => "`u8`",
        FixedType::U16 => "`u16`",
        FixedType::U32 => "`u32`",
        FixedType::U64 => "`u64`",
        FixedType::Usize => "`usize`",
    }
}

/// The source spelling of a capture mode, for the judgment's refusal
/// messages. Total over `SourceCaptureMode`.
fn mode_spelling(mode: &SourceCaptureMode) -> &'static str {
    match mode {
        SourceCaptureMode::Copy => "`copy`",
        SourceCaptureMode::Move => "`move`",
        SourceCaptureMode::Read { .. } => "`read ... share ...`",
        SourceCaptureMode::Write => "`write`",
        SourceCaptureMode::LendWrite { .. } => "`lend ... LOW..HIGH`",
    }
}

/// The transfer/share contracts a kind does carry, read from the judgment's
/// own classification so a refusal names what would cross rather than only
/// what does not.
fn admitted_contracts(scalar: Scalar) -> &'static str {
    use qcore_core::transfer::{ShareProtocol, TransferClass};
    match qcore_core::transfer::transferable(transfer_kind(scalar)) {
        TransferClass::Duplicated => "`copy` (an independent duplicate; no custody crosses)",
        TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
            mutably_shareable_under: Some(ShareProtocol::DisjointWriteLeases),
        } => {
            "`move` (custody transferred to the worker), `read PLACE share N` \
             (a counted read share), or `lend PLACE LOW..HIGH` (a disjoint \
             range write lease)"
        }
        TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
            mutably_shareable_under: _,
        } => {
            "`move` (custody transferred to the worker) or `read PLACE share N` \
             (a counted read share)"
        }
        TransferClass::MovedOnly { .. } => "`move` (custody transferred to the worker)",
        TransferClass::Never(_) => "no transfer or share contract at all",
    }
}

/// Card CT47-COPY-CAPTURE: one refusal message per [`TransferRefusal`]
/// variant. Each names the judgment, the capture it refuses, and the missing
/// transfer/share contract — which is what makes the boundary's refusal
/// attributable to the crossing construct rather than to the backend.
fn transfer_refusal_message(
    place: Scalar,
    mode: &SourceCaptureMode,
    parameter: Scalar,
    refusal: qcore_core::transfer::TransferRefusal,
) -> String {
    use qcore_core::transfer::{TransferKind, TransferRefusal};
    let kind = scalar_spelling(place);
    let asked = mode_spelling(mode);
    let carries = admitted_contracts(place);
    match refusal {
        TransferRefusal::ModeNotAdmittedForKind => format!(
            "the transferability judgment refuses this capture: no transfer or share \
             contract admits {asked} of {kind} across a spawn boundary; {kind} crosses \
             only under {carries}"
        ),
        TransferRefusal::WriteCaptureUnimplemented => format!(
            "the transferability judgment refuses this capture: a {asked} capture names \
             no transfer or share contract — no kind crosses a spawn boundary under it; \
             {kind} crosses only under {carries}"
        ),
        TransferRefusal::ReceivedKindMismatch => {
            let declared = scalar_spelling(parameter);
            // A mismatch presupposes an admitted transfer; re-derive what it
            // delivers. The `Err` arm is unreachable and refuses generically
            // rather than panicking.
            let delivered =
                match qcore_core::transfer::admits(transfer_kind(place), transfer_mode(mode)) {
                    Ok(admitted) => {
                        match qcore_core::transfer::received_kind(transfer_kind(place), admitted) {
                            TransferKind::ReadLoan => "`borrow<Buffer>`",
                            TransferKind::WriteLease => "`lend<Buffer>`",
                            _ => kind,
                        }
                    }
                    Err(_) => kind,
                };
            format!(
                "the transferability judgment refuses this capture: {asked} of {kind} is \
                 an admitted transfer, but it delivers {delivered} and the worker \
                 declares it receives {declared}"
            )
        }
        TransferRefusal::KindNotTransferable(_) => format!(
            "the transferability judgment refuses this capture: {kind} carries no \
             transfer or share contract, so it does not cross a spawn boundary in any \
             capture mode"
        ),
        TransferRefusal::ProtocolUnavailable(_) => format!(
            "the transferability judgment refuses this capture: sharing {kind} needs a \
             protocol no capture mode of this route can request"
        ),
    }
}

/// Which of the three admitted capture forms the existing enumeration picked,
/// as a value rather than as an early return, so that the derived judgment can
/// be compared against a refusal as well as against an admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnumeratedCapture {
    Copy,
    Move,
    Lend,
    /// A range write lend (card `WRITE-LEND-TASKS`).
    LendWrite,
}

impl EnumeratedCapture {
    /// The derived transfer this enumerated form claims to be, for the
    /// agreement check.
    fn as_admitted(self) -> qcore_core::transfer::AdmittedTransfer {
        use qcore_core::transfer::{AdmittedTransfer, ShareProtocol};
        match self {
            EnumeratedCapture::Copy => AdmittedTransfer::Duplicate,
            EnumeratedCapture::Move => AdmittedTransfer::MoveCustody,
            EnumeratedCapture::Lend => {
                AdmittedTransfer::LendShare(ShareProtocol::CountedReadShares)
            }
            EnumeratedCapture::LendWrite => {
                AdmittedTransfer::LendWriteRange(ShareProtocol::DisjointWriteLeases)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Scalar(Scalar),
    Handle(SymbolId),
    Result {
        value: Scalar,
        errors: BTreeSet<String>,
        /// True when some variant this Result can carry declares an owned
        /// Buffer payload. Such a Result cannot be propagated with `?`: its
        /// payload has to be taken on an explicit error arm.
        error_payload: bool,
    },
    Scope,
    KnownError(String),
    Error {
        errors: BTreeSet<String>,
        /// True until `qcore_error_payload` takes the Buffer out. While it is
        /// true the binding cannot be re-raised or silently dropped.
        payload: bool,
    },
}

/// Lower complete source syntax into task execution IR. This checks the source
/// import/type boundary; the resulting IR requires `check_task_module` before
/// emission or a source-execution claim.
pub fn lower_source_tasks(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<TaskModule, Vec<SourceDiagnostic>> {
    lower_source_tasks_for_target(source, typed, entry, None)
}

/// [`lower_source_tasks`] under the builder's recorded target (card
/// `W48-QR-SCALAR-TYPES-R4`), which alone gives `isize`/`usize` a width.
pub fn lower_source_tasks_for_target(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&RecordedTarget>,
) -> Result<TaskModule, Vec<SourceDiagnostic>> {
    lower(source, typed, entry, false, target)
        .map(|lowered| lowered.tasks)
        .map_err(|error| vec![error])
}

/// A task module and the `source.std-io.v1` events placed in its entry
/// (card `COMPOSE-CONCURRENCY`).
#[derive(Clone, Debug)]
pub struct LoweredComposedTaskModule {
    pub tasks: TaskModule,
    pub sites: Vec<TaskIoSite>,
    /// Whether the entry declared `io`.
    pub declares_io: bool,
    /// The non-entry functions that declared `io` in their own row (card
    /// `OBS-REORDERING`). A worker that writes must say so itself.
    pub worker_io: std::collections::BTreeSet<usize>,
}

/// [`lower_source_tasks`] with the line-producing statements of `std.io`,
/// `std.logger` and `std.fmt` admitted in the **parent's** body.
///
/// `lower_source_tasks` is this function with `composing = false`, so a module
/// on the task route alone travels the identical code path: `io` is still an
/// unsupported effect there, a `print` is still an unsupported expression
/// statement there, and the lowered module is byte-identical. There is no second
/// lowering to drift.
pub fn lower_source_composed_tasks(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredComposedTaskModule, Vec<SourceDiagnostic>> {
    lower_source_composed_tasks_for_target(source, typed, entry, None)
}

/// [`lower_source_composed_tasks`] under the builder's recorded target.
pub fn lower_source_composed_tasks_for_target(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    target: Option<&RecordedTarget>,
) -> Result<LoweredComposedTaskModule, Vec<SourceDiagnostic>> {
    lower(source, typed, entry, true, target).map_err(|error| vec![error])
}

fn lower(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
    composing: bool,
    target: Option<&RecordedTarget>,
) -> Result<LoweredComposedTaskModule, SourceDiagnostic> {
    let span = Span {
        start: 0,
        end: source.source().len(),
    };
    if source.mode != SourceModuleMode::Executable {
        return Err(failure(
            span,
            SemanticPass::CEmission,
            "entry-free task execution checking is not implemented",
        ));
    }
    let reparsed = parse_source_program(source.source())
        .map_err(|error| failure(error.span, SemanticPass::NameResolution, error.message))?;
    if reparsed != *source {
        return Err(failure(
            span,
            SemanticPass::NameResolution,
            "task source metadata differs from independent parsing",
        ));
    }
    let retyped = elaborate_source(source).map_err(|errors| {
        errors
            .into_iter()
            .next()
            .expect("elaboration rejection has diagnostics")
    })?;
    if retyped != *typed {
        return Err(failure(
            span,
            SemanticPass::NameResolution,
            "task symbol metadata differs from independent elaboration",
        ));
    }
    if source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::ConcurrencyProfile)
        .count()
        != 1
    {
        return Err(failure(
            span,
            SemanticPass::Concurrency,
            "task execution requires exactly one `concurrency checked;` marker",
        ));
    }
    for declaration in &source.declarations {
        if matches!(
            declaration.value.name.as_str(),
            "qcore_buffer_new"
                | "qcore_buffer_read"
                | "qcore_buffer_write"
                | "qcore_error_payload"
                | "AllocError"
                // Card `W48-QR-SCALAR-TYPES-R4`: the builtin §G02t error type.
                | "ArithmeticError"
        ) {
            return Err(failure(
                declaration.span,
                SemanticPass::NameResolution,
                "reserved owned-buffer intrinsic or error name",
            ));
        }
        // Card `OWNED-FIELD-ROUTE-REACH` (wave 41): a **record type** in a
        // checked-concurrency module refuses here already — this is the arm it
        // reaches — but it refused with the generic declaration-kind sentence,
        // which named no boundary. `AGGREGATE-TASK-CAPTURE` now names itself,
        // at the **same code, the same span and over the same refused set**:
        // no condition is widened and no code is allocated (the
        // `CT24-CLEANUP-PREEMPTED` shape). It matters because the record route
        // is never *selected* for a module that spawns — route dispatch takes
        // the task route first — so this is the only place in the tree where a
        // reader meets the boundary, and before the card nothing there said so.
        if declaration.value.kind == DeclarationKind::Record {
            return Err(failure(
                declaration.span,
                SemanticPass::CEmission,
                "a `source.record.v1` record type is outside the copy-scalar task execution \
                 profile: the transferability judgment has no aggregate kind and the scoped-task \
                 ledger is keyed by plain name, so a record — owning or not — can neither be \
                 captured by a spawn nor held across a join (`AGGREGATE-TASK-CAPTURE`)",
            ));
        }
        if !matches!(
            declaration.value.kind,
            DeclarationKind::ConcurrencyProfile
                | DeclarationKind::Function
                | DeclarationKind::ErrorType
        ) {
            return Err(failure(
                declaration.span,
                SemanticPass::CEmission,
                "declaration is outside the copy-scalar task execution profile",
            ));
        }
    }
    let errors = error_declarations(source)?;
    let mut signatures = BTreeMap::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::Function)
    {
        let resolved = typed
            .declarations
            .iter()
            .find(|d| d.name == declaration.value.name)
            .expect("independent elaboration preserved functions");
        let signature = signature(source, declaration, resolved, &errors, composing)?;
        signatures.insert(signature.id, signature);
    }
    let entry = typed
        .globals
        .get(entry)
        .copied()
        .filter(|symbol| signatures.contains_key(symbol))
        .ok_or_else(|| {
            failure(
                span,
                SemanticPass::NameResolution,
                "task entry must name a source function",
            )
        })?;
    if !signatures[&entry].parameters.is_empty() {
        return Err(failure(
            span,
            SemanticPass::OrdinaryTypes,
            "task execution entry must have no parameters",
        ));
    }
    // Card `W48-QR-SCALAR-TYPES-R4`: the entry's result is published, never a
    // value a caller receives, so it stays `i32`/`unit` (card 0's rule); a
    // fixed-width value is a worker's result, collected by `join`.
    if let Scalar::Fixed(ty) = signatures[&entry].result {
        return Err(failure(
            span,
            SemanticPass::OrdinaryTypes,
            format!(
                "a task entry returns i32 or unit; `{ty}` is a worker's result, collected by join"
            ),
        ));
    }
    let mut functions = BTreeMap::new();
    let mut sites = Vec::new();
    for declaration in source
        .declarations
        .iter()
        .filter(|d| d.value.kind == DeclarationKind::Function)
    {
        let resolved = typed
            .declarations
            .iter()
            .find(|d| d.name == declaration.value.name)
            .unwrap();
        let signature = &signatures[&resolved.symbol];
        let body = declaration.value.body.as_ref().unwrap();
        let mut builder = Builder::new(
            source,
            &signatures,
            signature,
            &errors,
            next_temporary(resolved)?,
        );
        builder.composing = composing;
        builder.function = resolved.symbol.0 as usize;
        builder.target = target;
        if !builder.block(body, &resolved.statements)? {
            return Err(failure(
                body.span,
                SemanticPass::OrdinaryTypes,
                "task functions require an explicit return or raise on every path",
            ));
        }
        functions.insert(
            resolved.symbol.0 as usize,
            TaskFunction {
                name: signature.name.clone(),
                parameters: signature
                    .parameters
                    .iter()
                    .map(|(symbol, _, kind)| (symbol.0 as usize, kind.core()))
                    .collect(),
                result: signature.result.core(),
                errors: signature.effects.errors.clone(),
                error_payloads: signature.effects.error_payloads.clone(),
                declared_effects: TaskEffects {
                    alloc: signature.effects.alloc,
                    spawn: signature.effects.spawn,
                    join: signature.effects.join,
                    may_block: signature.effects.may_block,
                },
                entry: 0,
                blocks: builder.blocks,
                span: core_span(declaration.span),
            },
        );
        sites.extend(builder.sites);
    }
    // A worker cannot recursively or transitively create tasks in this first
    // executable profile. All source functions were still lowered above.
    let workers = functions
        .values()
        .flat_map(|function| {
            function
                .blocks
                .values()
                .filter_map(|block| match block.terminator.kind {
                    TaskTerminatorKind::TrySpawn { worker, .. } => Some(worker),
                    _ => None,
                })
        })
        .collect::<BTreeSet<_>>();
    for worker in &workers {
        if functions[worker]
            .blocks
            .values()
            .any(|block| matches!(block.terminator.kind, TaskTerminatorKind::TrySpawn { .. }))
        {
            let span = functions[worker].span;
            return Err(failure(
                Span {
                    start: span.start,
                    end: span.end,
                },
                SemanticPass::Concurrency,
                "spawned workers must be first-order leaf functions in this task profile",
            ));
        }
    }
    // Card `OBS-REORDERING`. A line stands in the entry or in a function the
    // entry spawns; a line anywhere else has no thread to attribute it to, so
    // there is no emitter and no window and the comparison would have nothing to
    // compare. The spawn set is only known here, after every terminator exists,
    // which is why this is a pass rather than a guard inside the builder.
    let admissible: BTreeSet<usize> = std::iter::once(entry.0 as usize)
        .chain(workers.iter().copied())
        .collect();
    if let Some(site) = sites
        .iter()
        .find(|site| !admissible.contains(&site.function))
    {
        return Err(failure(
            Span {
                start: site.span.start,
                end: site.span.end,
            },
            SemanticPass::ComposedTasks,
            "`source.std-io-tasks.v1` writes a line from the parent's own body or from the body \
             of a worker this entry spawns, and from nowhere else: a line in a function this \
             program never runs has no thread to attribute it to and no window to place it in",
        ));
    }
    let module = TaskModule {
        entry: entry.0 as usize,
        functions,
        source_len: source.source().len(),
        target: target.cloned(),
    };
    // Card `W48-QR-SCALAR-TYPES-R4`: `isize`/`usize` take their width from the
    // recorded target, and a function naming either with none is refused —
    // the width is never guessed.
    if target.is_none() {
        for function in module.functions.values() {
            if let Some(ty) = qcore_core::task_execution::function_fixed_types(function)
                .into_iter()
                .find(|ty| ty.is_target_width())
            {
                return Err(failure(
                    Span {
                        start: function.span.start,
                        end: function.span.end,
                    },
                    SemanticPass::OrdinaryTypes,
                    qcore_core::scalar_values::ValueRefusal::NoRecordedTarget { ty }.to_string(),
                ));
            }
        }
    }
    Ok(LoweredComposedTaskModule {
        declares_io: signatures[&entry].effects.io,
        worker_io: signatures
            .values()
            .filter(|signature| signature.id != entry && signature.effects.io)
            .map(|signature| signature.id.0 as usize)
            .collect(),
        tasks: module,
        sites,
    })
}

fn next_temporary(function: &TypedSourceDeclaration) -> Result<usize, SourceDiagnostic> {
    fn statement_max(statements: &[TypedStatement]) -> usize {
        statements
            .iter()
            .map(|s| {
                s.binding
                    .into_iter()
                    .chain(s.body_binding)
                    .chain(
                        s.result_match
                            .iter()
                            .flat_map(|m| m.arms.iter().filter_map(|a| a.binding)),
                    )
                    .map(|id| id.0 as usize)
                    .chain(std::iter::once(statement_max(&s.nested)))
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }
    function
        .parameters
        .iter()
        .map(|p| p.symbol.0 as usize)
        .chain(std::iter::once(statement_max(&function.statements)))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| {
            failure(
                function.span,
                SemanticPass::CEmission,
                "task slot identity space exhausted",
            )
        })
}

struct Builder<'a> {
    source: &'a SourceProgram,
    signatures: &'a BTreeMap<SymbolId, Signature>,
    signature: &'a Signature,
    errors: &'a BTreeMap<String, ErrorType>,
    environment: Vec<BTreeMap<String, (SymbolId, ValueType)>>,
    scopes: Vec<u64>,
    blocks: BTreeMap<usize, TaskBlock>,
    current: usize,
    next_block: usize,
    next_temporary: usize,
    instructions: Vec<TaskInstruction>,
    /// Card `COMPOSE-CONCURRENCY`; `false` on the task route alone.
    composing: bool,
    /// The function this builder is lowering, which is the function a line it
    /// records stands in (card `OBS-REORDERING`). Before that card only the
    /// entry could carry a line, and the flag was a Boolean.
    function: usize,
    sites: Vec<TaskIoSite>,
    /// The builder's recorded target (card `W48-QR-SCALAR-TYPES-R4`).
    target: Option<&'a RecordedTarget>,
    /// The number of every binding known at check time (card
    /// `W48-QR-SCALAR-TYPES-R4`): a literal, a copy of a known binding, a value
    /// statement over known operands. A binding identity is unique in its
    /// function, so this map needs no scoping. It decides, with the shared
    /// decision, whether a value statement folds or raises — the core decides
    /// the same thing again, independently.
    known: BTreeMap<SymbolId, i128>,
}

impl<'a> Builder<'a> {
    fn new(
        source: &'a SourceProgram,
        signatures: &'a BTreeMap<SymbolId, Signature>,
        signature: &'a Signature,
        errors: &'a BTreeMap<String, ErrorType>,
        next_temporary: usize,
    ) -> Self {
        Self {
            source,
            signatures,
            signature,
            errors,
            environment: vec![
                signature
                    .parameters
                    .iter()
                    .map(|(symbol, name, kind)| (name.clone(), (*symbol, ValueType::Scalar(*kind))))
                    .collect(),
            ],
            scopes: Vec::new(),
            blocks: BTreeMap::new(),
            current: 0,
            next_block: 1,
            next_temporary,
            instructions: Vec::new(),
            composing: false,
            function: 0,
            sites: Vec::new(),
            target: None,
            known: BTreeMap::new(),
        }
    }

    /// Whether this function's row permits raising `ArithmeticError`.
    fn raises_arithmetic(&self) -> bool {
        qcore_core::scalar_types::ArithmeticError::ALL
            .iter()
            .all(|error| self.signature.effects.errors.contains(&error.to_string()))
    }

    /// A literal written where a fixed-width type `ty` is expected — a typed
    /// `let` or a worker's `return` (card `W48-QR-SCALAR-TYPES-R4`): `INTEGER`,
    /// `- INTEGER`, `true` or `false`, range-checked against `ty`. `None` when
    /// the tokens are not a literal.
    fn fixed_literal(
        &self,
        tokens: &[Token],
        ty: FixedType,
        span: Span,
    ) -> Option<Result<ScalarOperand, SourceDiagnostic>> {
        let value = match tokens {
            [
                Token {
                    kind: TokenKind::Integer(text),
                    ..
                },
            ] => text.parse::<i128>().ok()?,
            [
                Token {
                    kind: TokenKind::Symbol(Symbol::Minus),
                    ..
                },
                Token {
                    kind: TokenKind::Integer(text),
                    ..
                },
            ] => -text.parse::<i128>().ok()?,
            [t] if word(Some(t), "true") || word(Some(t), "false") => {
                let value = i128::from(word(Some(t), "true"));
                return Some(if ty == FixedType::Bool {
                    Ok(ScalarOperand::Fixed { ty, value })
                } else {
                    Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        format!("`{ty}` is not `bool`; there is no conversion from bool"),
                    ))
                });
            }
            _ => return None,
        };
        let refused = |refusal: qcore_core::scalar_values::ValueRefusal| {
            failure(span, SemanticPass::OrdinaryTypes, refusal.to_string())
        };
        Some(
            qcore_core::scalar_values::Decider::default()
                .literal_at(value, ty, self.target)
                .map(|value| ScalarOperand::Fixed { ty, value })
                .map_err(refused),
        )
    }

    /// `let NAME = VALUE_EXPRESSION;` (card `W48-QR-SCALAR-TYPES-R4`): one
    /// §G02t operation, decided by the shared decision. A statement that can
    /// raise at run time lowers to the checked terminator whose error edge is
    /// this route's ordinary error exit; any other to one instruction.
    fn value_statement(
        &mut self,
        syntax: crate::source_value_statement::ValueExpressionSyntax<(SymbolId, Scalar)>,
        span: Span,
        binding: SymbolId,
    ) -> Result<ValueType, SourceDiagnostic> {
        use crate::source_value_statement::{
            ValueExpressionSyntax as Syntax, ValueOperandSyntax as Operand,
        };
        use qcore_core::prelude::concurrency::task_execution::{
            TaskValueOperand, TaskValueOperation,
        };
        use qcore_core::scalar_values::{self, Decider, ValueRefusal};
        let shared = |operand: Operand<(SymbolId, Scalar)>, known: &BTreeMap<SymbolId, i128>| {
            match operand {
                Operand::Value((symbol, scalar)) => scalar_values::Operand::Typed {
                    ty: match scalar {
                        Scalar::Fixed(ty) => ty,
                        _ => FixedType::I32,
                    },
                    known: known.get(&symbol).copied(),
                },
                Operand::Integer(literal) => scalar_values::Operand::Integer(literal),
                Operand::Bool(value) => scalar_values::Operand::Bool(value),
            }
        };
        let operation = match syntax {
            Syntax::Binary(op, a, b) => {
                scalar_values::Operation::Binary(op, shared(a, &self.known), shared(b, &self.known))
            }
            Syntax::Compare(cmp, a, b) => scalar_values::Operation::Compare(
                cmp,
                shared(a, &self.known),
                shared(b, &self.known),
            ),
            Syntax::Negate(a) => scalar_values::Operation::Negate(shared(a, &self.known)),
            Syntax::Convert(a, to) => scalar_values::Operation::Convert(shared(a, &self.known), to),
        };
        let decision = Decider::default()
            .decide(operation, self.raises_arithmetic(), self.target)
            .map_err(|refusal| match refusal {
                ValueRefusal::NeedsPermission { ty } => failure(
                    span,
                    SemanticPass::TypedErrors,
                    format!(
                        "this `{ty}` operation reads a value known only at run time and can \
                         fault; the function's row must declare `throws<ArithmeticError>` for \
                         the fault to be raised as a typed error, and without it the operation \
                         is refused"
                    ),
                ),
                other => failure(span, SemanticPass::OrdinaryTypes, other.to_string()),
            })?;
        // A literal takes the operation's own type, as the decision typed it.
        let lower_operand = |operand: Operand<(SymbolId, Scalar)>| match operand {
            Operand::Value((symbol, _)) => TaskValueOperand::Slot(symbol.0 as usize),
            Operand::Integer(value) => TaskValueOperand::Literal {
                ty: decision.operand_ty,
                value,
            },
            Operand::Bool(value) => TaskValueOperand::Literal {
                ty: FixedType::Bool,
                value: i128::from(value),
            },
        };
        let operation = match syntax {
            Syntax::Binary(op, a, b) => {
                TaskValueOperation::Binary(op, lower_operand(a), lower_operand(b))
            }
            Syntax::Compare(cmp, a, b) => {
                TaskValueOperation::Compare(cmp, lower_operand(a), lower_operand(b))
            }
            Syntax::Negate(a) => TaskValueOperation::Negate(lower_operand(a)),
            Syntax::Convert(a, to) => TaskValueOperation::Convert(lower_operand(a), to),
        };
        let slot = binding.0 as usize;
        if decision.raises {
            let success = self.block_id(span)?;
            let error_slot = self.temporary(span)?;
            let error = self.error_block(span, ErrorOperand::Slot(error_slot))?;
            self.finish(
                span,
                TaskTerminatorKind::TryCompute {
                    slot,
                    operation,
                    error_slot,
                    success,
                    error,
                },
            );
            self.current = success;
        } else {
            self.instruction(span, TaskInstructionKind::Compute { slot, operation });
        }
        if let Some(known) = decision.known {
            self.known.insert(binding, known);
        }
        Ok(ValueType::Scalar(if decision.ty == FixedType::I32 {
            Scalar::I32
        } else {
            Scalar::Fixed(decision.ty)
        }))
    }

    /// Record one `source.std-io.v1` event at the position the emitted program
    /// will perform it: immediately before the next instruction of the block
    /// being built.
    ///
    /// The operand is resolved here, in the **task** model's own environment, so
    /// a rendering names a task slot or nothing at all. Whether that slot may be
    /// *read* there is not decided here: `check_composed_task_program` asks the
    /// task checker, by copying the scalar at this exact position.
    fn source_line(
        &mut self,
        span: Span,
        event: qcore_core::std_io::StdIoEvent,
    ) -> Result<(), SourceDiagnostic> {
        use qcore_core::std_io::{StdIoEvent, StdIoOperand};
        // Which function may carry a line is decided once, after every function
        // is lowered, by `admissible_line_sites` below: it is the entry and the
        // functions the entry spawns, and the spawn set is not known until the
        // terminators exist. Card `OBS-REORDERING`.
        let operand = match &event {
            StdIoEvent::LogLevel { .. } | StdIoEvent::Flush | StdIoEvent::Return => None,
            StdIoEvent::Print { operand, .. } | StdIoEvent::Log { operand, .. } => match operand {
                StdIoOperand::Literal(_) => None,
                StdIoOperand::Borrowed(name) => {
                    return Err(failure(
                        span,
                        SemanticPass::ComposedTasks,
                        format!(
                            "`borrow {name}` renders the model identity a checked custody model \
                             published for a resource, and `source.std-io-tasks.v1` publishes \
                             none: the task ledger's parent/child custody edges, read shares and \
                             join discipline are a different question from the extern route's \
                             straight-line ledger, and a borrowed render across a task boundary \
                             needs its own custody argument"
                        ),
                    ));
                }
                StdIoOperand::Scalar(name) => {
                    let (id, kind) = self.lookup(name, span)?;
                    // Card `W48-QR-SCALAR-TYPES-R4`: a value of every §G02t
                    // type renders at its type; `bool` has no `hex`.
                    if let ValueType::Scalar(Scalar::Fixed(ty)) = kind {
                        let hex = matches!(
                            &event,
                            StdIoEvent::Print {
                                rendering: qcore_core::std_io::Rendering::Hex,
                                ..
                            } | StdIoEvent::Log {
                                rendering: qcore_core::std_io::Rendering::Hex,
                                ..
                            }
                        );
                        if hex && ty == FixedType::Bool {
                            return Err(failure(
                                span,
                                SemanticPass::ComposedTasks,
                                format!(
                                    "`{name}` is a `bool`, which renders `true`/`false` under \
                                     `dec` and has no `hex` rendering"
                                ),
                            ));
                        }
                    } else if kind != ValueType::Scalar(Scalar::I32) {
                        return Err(failure(
                            span,
                            SemanticPass::ComposedTasks,
                            format!(
                                "`{name}` is not an `i32` this entry bound; \
                                 `source.std-io-tasks.v1` renders integer literals and the `i32` \
                                 values the checked task model computes — a copied parameter, a \
                                 `let`-bound literal, a `qcore_buffer_read` of a live buffer, or \
                                 a collected join outcome"
                            ),
                        ));
                    }
                    Some(id.0 as usize)
                }
            },
        };
        self.sites.push(TaskIoSite {
            function: self.function,
            block: self.current,
            index: self.instructions.len(),
            event,
            operand,
            span: core_span(span),
        });
        Ok(())
    }

    fn bad(&self, span: Span, message: impl Into<String>) -> SourceDiagnostic {
        failure(span, SemanticPass::CEmission, message)
    }
    fn temporary(&mut self, span: Span) -> Result<usize, SourceDiagnostic> {
        let slot = self.next_temporary;
        self.next_temporary = slot
            .checked_add(1)
            .ok_or_else(|| self.bad(span, "task slot identity space exhausted"))?;
        Ok(slot)
    }
    fn block_id(&mut self, span: Span) -> Result<usize, SourceDiagnostic> {
        let block = self.next_block;
        self.next_block = block
            .checked_add(1)
            .ok_or_else(|| self.bad(span, "task block identity space exhausted"))?;
        Ok(block)
    }
    fn instruction(&mut self, span: Span, kind: TaskInstructionKind) {
        self.instructions.push(TaskInstruction {
            span: core_span(span),
            kind,
        });
    }
    fn finish(&mut self, span: Span, kind: TaskTerminatorKind) {
        self.blocks.insert(
            self.current,
            TaskBlock {
                instructions: std::mem::take(&mut self.instructions),
                terminator: TaskTerminator {
                    span: core_span(span),
                    kind,
                },
            },
        );
    }
    fn exit_instructions(&self, span: Span) -> Vec<TaskInstruction> {
        self.scopes
            .iter()
            .rev()
            .map(|scope| TaskInstruction {
                span: core_span(span),
                kind: TaskInstructionKind::ExitScope { scope: *scope },
            })
            .collect()
    }
    fn error_block(&mut self, span: Span, error: ErrorOperand) -> Result<usize, SourceDiagnostic> {
        let block = self.block_id(span)?;
        self.blocks.insert(
            block,
            TaskBlock {
                instructions: self.exit_instructions(span),
                terminator: TaskTerminator {
                    span: core_span(span),
                    // Every synthesized raise is payload-free: the two builtin
                    // failures hold nothing, and `?` is refused for a Result
                    // whose error can carry a payload.
                    kind: TaskTerminatorKind::Raise {
                        error,
                        payload: None,
                    },
                },
            },
        );
        Ok(block)
    }
    fn lookup(&self, name: &str, span: Span) -> Result<(SymbolId, ValueType), SourceDiagnostic> {
        self.environment
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .ok_or_else(|| {
                failure(
                    span,
                    SemanticPass::NameResolution,
                    "value is not a live local or parameter in this task function",
                )
            })
    }
    fn lookup_place(
        &self,
        place: &ResolvedTaskPlace,
    ) -> Result<(SymbolId, ValueType), SourceDiagnostic> {
        let SourcePlace::Name(name) = &place.syntax.value else {
            return Err(self.bad(
                place.syntax.span,
                "task execution supports direct local/parameter places only",
            ));
        };
        let found = self.lookup(&name.value, name.span)?;
        if found.0 != place.root_symbol {
            return Err(self.bad(
                name.span,
                "task place identity disagrees with the lexical value binding",
            ));
        }
        Ok(found)
    }

    fn allocation_input(
        &self,
        tokens: &[Token],
        span: Span,
    ) -> Result<Option<ScalarOperand>, SourceDiagnostic> {
        if !word(tokens.first(), "qcore_buffer_new") {
            return Ok(None);
        }
        if !self.signature.effects.alloc {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "buffer allocation requires alloc effect",
            ));
        }
        if !symbol(tokens.get(1), Symbol::LeftParen)
            || matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
                != Some(tokens.len().saturating_sub(1))
        {
            return Err(self.bad(
                span,
                "buffer constructor requires one complete i32 argument",
            ));
        }
        let (initial, kind) = self.scalar_operand(&tokens[2..tokens.len() - 1], span)?;
        if kind != Scalar::I32 {
            return Err(self.bad(span, "buffer constructor requires i32"));
        }
        Ok(Some(initial))
    }

    fn buffer_initializer(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        if word(tokens.first(), "qcore_buffer_new") {
            if !symbol(tokens.last(), Symbol::Question) {
                return Err(self.bad(
                    span,
                    "buffer allocation requires ? or explicit Result match",
                ));
            }
            let initial = self
                .allocation_input(&tokens[..tokens.len() - 1], span)?
                .unwrap();
            if !self.signature.effects.errors.contains("AllocError") {
                return Err(failure(
                    span,
                    SemanticPass::TypedErrors,
                    "allocation propagation requires throws<AllocError>",
                ));
            }
            let success = self.block_id(span)?;
            let allocation_error =
                self.error_block(span, ErrorOperand::Tag("AllocError".into()))?;
            self.finish(
                span,
                TaskTerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot: slot,
                    success,
                    allocation_error,
                },
            );
            self.current = success;
            return Ok(Some(ValueType::Scalar(Scalar::OwnedBuffer)));
        }
        if word(tokens.first(), "qcore_buffer_write") {
            // `qcore_buffer_write(BUFFER, VALUE)` evaluates to `unit`. Only an
            // owner can name the buffer: a read loan is not an `own<Buffer>`
            // binding, so a borrower cannot reach this form at all, and the
            // ledger separately refuses the write while any share is out.
            //
            // Card `WRITE-LEND-TASKS-EXERCISE` adds the **three**-argument
            // spelling `qcore_buffer_write(LEASE, INDEX, VALUE)`, the
            // `borrow_utf8` arity precedent: one byte of the payload, through
            // a range write lease, at an index relative to that lease's own
            // low. The arity is what selects the form, so no word is
            // reserved and the two-argument owner write is untouched.
            let commas = argument_commas(tokens);
            if commas.len() == 2 {
                return self.lease_byte_write(tokens, span, slot, &commas);
            }
            let comma = tokens
                .iter()
                .position(|token| symbol(Some(token), Symbol::Comma))
                .ok_or_else(|| {
                    self.bad(
                        span,
                        "buffer write requires one owned-buffer name and one i32 value",
                    )
                })?;
            if comma < 3
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
                    != Some(tokens.len().saturating_sub(1))
            {
                return Err(self.bad(
                    span,
                    "buffer write requires one owned-buffer name and one i32 value",
                ));
            }
            let target = name(tokens.get(2))
                .filter(|_| comma == 3)
                .ok_or_else(|| self.bad(span, "buffer write requires an exact buffer name"))?;
            let (id, kind) = self.lookup(target, span)?;
            if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                return Err(self.bad(
                    span,
                    "buffer write requires own<Buffer>; a read loan grants no write authority",
                ));
            }
            let (value, value_kind) =
                self.scalar_operand(&tokens[comma + 1..tokens.len() - 1], span)?;
            if value_kind != Scalar::I32 {
                return Err(self.bad(span, "buffer write requires an i32 value"));
            }
            self.instruction(
                span,
                TaskInstructionKind::WriteBuffer {
                    buffer: id.0 as usize,
                    value,
                },
            );
            self.instruction(
                span,
                TaskInstructionKind::BindScalar {
                    slot,
                    value: ScalarOperand::Unit,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::Unit)));
        }
        if word(tokens.first(), "qcore_buffer_read") {
            // The two-argument spelling `qcore_buffer_read(NAME, INDEX)` reads
            // one byte (card `WRITE-LEND-TASKS-EXERCISE`) — through a lease,
            // relative to its low, or through an owner, where the base is 0
            // and the whole-payload read's own authority gate applies.
            let commas = argument_commas(tokens);
            if commas.len() == 1 {
                return self.byte_read(tokens, span, slot, commas[0]);
            }
            if tokens.len() != 4
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
            {
                return Err(self.bad(span, "buffer read requires one exact owned-buffer name"));
            }
            let buffer =
                name(tokens.get(2)).ok_or_else(|| self.bad(span, "buffer read requires a name"))?;
            let (id, kind) = self.lookup(buffer, span)?;
            // A read is the one operation a loan admits, and the one operation
            // an owner keeps while its buffer is lent out.
            if !matches!(
                kind,
                ValueType::Scalar(Scalar::OwnedBuffer | Scalar::ReadLoan)
            ) {
                return Err(self.bad(span, "buffer read requires own<Buffer> or borrow<Buffer>"));
            }
            self.instruction(
                span,
                TaskInstructionKind::ReadBuffer {
                    buffer: id.0 as usize,
                    value_slot: slot,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::I32)));
        }
        // Card T06. `qcore_error_payload(e)` moves the owned Buffer out of a
        // bound typed error and into a new owned binding. It is the only way a
        // payload leaves an error, it works at most once per binding, and the
        // error keeps its tag afterwards so it can still be inspected.
        if word(tokens.first(), "qcore_error_payload") {
            if tokens.len() != 4
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
            {
                return Err(self.bad(
                    span,
                    "taking an error payload requires one exact bound error name",
                ));
            }
            let binding = name(tokens.get(2))
                .ok_or_else(|| self.bad(span, "taking an error payload requires a name"))?;
            let (id, kind) = self.lookup(binding, span)?;
            let ValueType::Error {
                errors,
                payload: true,
            } = kind
            else {
                return Err(self.bad(
                    span,
                    "qcore_error_payload requires a bound error that still holds an owned payload",
                ));
            };
            self.instruction(
                span,
                TaskInstructionKind::TakeErrorPayload {
                    error_slot: id.0 as usize,
                    buffer_slot: slot,
                },
            );
            // The binding now names a payload-free error. Rebinding it here is
            // what makes a second take, and a silent re-raise, impossible.
            self.rebind(
                binding,
                ValueType::Error {
                    errors,
                    payload: false,
                },
            );
            return Ok(Some(ValueType::Scalar(Scalar::OwnedBuffer)));
        }
        Ok(None)
    }

    /// Replace the innermost binding of `name` in place. Only the payload
    /// state of a typed error changes this way, and only from held to taken.
    fn rebind(&mut self, name: &str, kind: ValueType) {
        for scope in self.environment.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                entry.1 = kind;
                return;
            }
        }
        unreachable!("rebinding follows a successful lookup of the same name")
    }

    /// `qcore_buffer_write(LEASE, INDEX, VALUE)` — one byte of the payload,
    /// through a range write lease. The lease is the only authority this form
    /// admits: the owner's whole-payload write remains its only write
    /// footprint, so wave 31's "the owner holds no whole-payload authority
    /// while a lease is out" rule is inherited unchanged rather than restated
    /// against a second owner-side footprint.
    fn lease_byte_write(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
        commas: &[usize],
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let close = matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
            .filter(|close| *close == tokens.len().saturating_sub(1))
            .ok_or_else(|| {
                self.bad(
                    span,
                    "a byte write requires one lend<Buffer> name, one byte index and one \
                     i32 value",
                )
            })?;
        let target = name(tokens.get(2))
            .filter(|_| commas[0] == 3)
            .ok_or_else(|| self.bad(span, "a byte write requires an exact lease name"))?;
        let (id, kind) = self.lookup(target, span)?;
        if kind != ValueType::Scalar(Scalar::WriteLease) {
            return Err(self.bad(
                span,
                "a byte write requires lend<Buffer>: the owner's write footprint is the \
                 whole payload, and a read share grants no write authority",
            ));
        }
        let (index, index_kind) = self.scalar_operand(&tokens[commas[0] + 1..commas[1]], span)?;
        if index_kind != Scalar::I32 {
            return Err(self.bad(span, "a byte index must be an i32"));
        }
        let (value, value_kind) = self.scalar_operand(&tokens[commas[1] + 1..close], span)?;
        if value_kind != Scalar::I32 {
            return Err(self.bad(span, "a byte write requires an i32 value"));
        }
        self.instruction(
            span,
            TaskInstructionKind::WriteBufferByte {
                authority: id.0 as usize,
                index,
                value,
            },
        );
        self.instruction(
            span,
            TaskInstructionKind::BindScalar {
                slot,
                value: ScalarOperand::Unit,
            },
        );
        Ok(Some(ValueType::Scalar(Scalar::Unit)))
    }

    /// `qcore_buffer_read(NAME, INDEX)` — one byte, through a lease or through
    /// the owner. A read share is refused by name: giving a loan a byte view
    /// is the named residual `WRITE-LEND-EXERCISE-LOAN-BYTE-READ`.
    fn byte_read(
        &mut self,
        tokens: &[Token],
        span: Span,
        slot: usize,
        comma: usize,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let close = matching(tokens, 1, Symbol::LeftParen, Symbol::RightParen)
            .filter(|close| *close == tokens.len().saturating_sub(1))
            .ok_or_else(|| {
                self.bad(
                    span,
                    "a byte read requires one exact name and one byte index",
                )
            })?;
        let target = name(tokens.get(2))
            .filter(|_| comma == 3)
            .ok_or_else(|| self.bad(span, "a byte read requires an exact name"))?;
        let (id, kind) = self.lookup(target, span)?;
        if !matches!(
            kind,
            ValueType::Scalar(Scalar::OwnedBuffer | Scalar::WriteLease)
        ) {
            return Err(self.bad(
                span,
                "a byte read requires own<Buffer> or lend<Buffer>; a read share admits the \
                 whole-payload read only",
            ));
        }
        let (index, index_kind) = self.scalar_operand(&tokens[comma + 1..close], span)?;
        if index_kind != Scalar::I32 {
            return Err(self.bad(span, "a byte index must be an i32"));
        }
        self.instruction(
            span,
            TaskInstructionKind::ReadBufferByte {
                authority: id.0 as usize,
                index,
                value_slot: slot,
            },
        );
        Ok(Some(ValueType::Scalar(Scalar::I32)))
    }

    fn scalar_operand(
        &self,
        tokens: &[Token],
        span: Span,
    ) -> Result<(ScalarOperand, Scalar), SourceDiagnostic> {
        match tokens {
            [
                Token {
                    kind: TokenKind::Integer(value),
                    ..
                },
            ] => value
                .parse::<i32>()
                .map(|value| (ScalarOperand::I32(value), Scalar::I32))
                .map_err(|_| self.bad(span, "integer literal is outside i32")),
            [
                Token {
                    kind: TokenKind::Symbol(Symbol::Minus),
                    ..
                },
                Token {
                    kind: TokenKind::Integer(value),
                    ..
                },
            ] => {
                let magnitude = value
                    .parse::<u64>()
                    .ok()
                    .filter(|n| *n <= 2147483648)
                    .ok_or_else(|| self.bad(span, "negative integer literal is outside i32"))?;
                Ok((
                    ScalarOperand::I32((-(magnitude as i64)) as i32),
                    Scalar::I32,
                ))
            }
            // Card `W48-QR-SCALAR-TYPES-R4`: `true`/`false` are `bool` literals.
            [token] if word(Some(token), "true") || word(Some(token), "false") => Ok((
                ScalarOperand::Fixed {
                    ty: FixedType::Bool,
                    value: i128::from(word(Some(token), "true")),
                },
                Scalar::Fixed(FixedType::Bool),
            )),
            [
                Token {
                    kind: TokenKind::Word(name),
                    ..
                },
            ] => {
                let (symbol, kind) = self.lookup(name, span)?;
                let ValueType::Scalar(scalar) = kind else {
                    return Err(self.bad(
                        span,
                        "a scalar expression cannot copy a handle, result, or scope",
                    ));
                };
                if scalar == Scalar::OwnedBuffer {
                    return Err(self.bad(
                        span,
                        "owned buffers require explicit move or a checked buffer read",
                    ));
                }
                if scalar == Scalar::ReadLoan {
                    return Err(self.bad(
                        span,
                        "a read loan cannot be copied; it admits only a checked buffer read",
                    ));
                }
                Ok((ScalarOperand::Slot(symbol.0 as usize), scalar))
            }
            _ => Err(self.bad(
                span,
                "task scalar expressions require an i32 literal or immutable scalar binding",
            )),
        }
    }

    fn propagate(
        &mut self,
        span: Span,
        result_slot: usize,
        value_slot: usize,
        value: Scalar,
        errors: &BTreeSet<String>,
        error_payload: bool,
    ) -> Result<ValueType, SourceDiagnostic> {
        if !errors.is_subset(&self.signature.effects.errors) {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "propagated task errors are absent from the function's throws effects",
            ));
        }
        // `?` has no place to name the payload, so it could only discard or
        // silently forward it. Both are refused: the payload has to be taken
        // on an explicit error arm.
        if error_payload {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "an error that carries an owned payload requires an explicit Result match, not `?`",
            ));
        }
        if errors.is_empty() {
            self.instruction(
                span,
                TaskInstructionKind::UnwrapInfallible {
                    result_slot,
                    value_slot,
                },
            );
            return Ok(ValueType::Scalar(value));
        }
        let success = self.block_id(span)?;
        let error_slot = self.temporary(span)?;
        let error = self.error_block(span, ErrorOperand::Slot(error_slot))?;
        self.finish(
            span,
            TaskTerminatorKind::TryOutcome {
                result_slot,
                value_slot,
                error_slot,
                success,
                error,
            },
        );
        self.current = success;
        Ok(ValueType::Scalar(value))
    }

    fn spawn_inputs(
        &self,
        span: Span,
        scope: SymbolId,
        worker: SymbolId,
        captures: &[ResolvedTaskCapture],
    ) -> Result<(u64, Vec<ScalarOperand>), SourceDiagnostic> {
        if !self.signature.effects.spawn {
            return Err(failure(
                span,
                SemanticPass::TypedErrors,
                "spawn requires its declared effect",
            ));
        }
        let scope_id = u64::from(scope.0);
        if self.scopes.last() != Some(&scope_id) {
            return Err(self.bad(span, "spawn requires the innermost active thread scope"));
        }
        let signature = self
            .signatures
            .get(&worker)
            .ok_or_else(|| self.bad(span, "unknown source worker"))?;
        if captures.len() != signature.parameters.len() {
            return Err(self.bad(span, "worker capture arity mismatch"));
        }
        let mut arguments = Vec::new();
        let mut loans = 0usize;
        let mut leases = 0usize;
        for (capture, (_, _, parameter_type)) in captures.iter().zip(&signature.parameters) {
            let (symbol, kind) = self.lookup_place(&capture.place)?;
            // A read loan is the one capture whose place type and parameter
            // type differ on purpose: the lender keeps `own<Buffer>` and the
            // borrower receives `borrow<Buffer>`. A write lease (card
            // `WRITE-LEND-TASKS`) is the second: the lessor keeps
            // `own<Buffer>` and the worker receives `lend<Buffer>`.
            let place_type =
                if *parameter_type == Scalar::ReadLoan || *parameter_type == Scalar::WriteLease {
                    Scalar::OwnedBuffer
                } else {
                    *parameter_type
                };
            // Card CT46-SENDSYNC. A non-scalar binding has no transferable
            // kind, and the check just below already refuses it. This guard
            // states that agreement rather than assuming it: if the judgment
            // ever called a handle, scope, Result or error value sendable,
            // this refuses instead of letting the type check be the only
            // thing standing between it and a boundary.
            if !matches!(kind, ValueType::Scalar(_))
                && qcore_core::transfer::transferable(binding_transfer_kind(&kind)).sendable()
            {
                return Err(self.bad(
                    capture.span,
                    "the transferability judgment admits a binding that is not a value",
                ));
            }
            if kind != ValueType::Scalar(place_type) {
                return Err(failure(
                    capture.span,
                    SemanticPass::OrdinaryTypes,
                    "capture type does not match worker parameter",
                ));
            }
            // Card CT46-SENDSYNC: the derived judgment, over what the parent
            // holds (`place_type`), the mode asked for, and what the worker
            // declared it receives (`parameter_type`).
            let derived = qcore_core::transfer::admits_capture(
                transfer_kind(place_type),
                transfer_mode(&capture.mode),
                transfer_kind(*parameter_type),
            );
            // The existing enumeration, as a decision rather than an early
            // return, so that the two can be compared on refusals too.
            let enumerated = match (&capture.mode, parameter_type) {
                (SourceCaptureMode::Copy, Scalar::I32 | Scalar::Fixed(_)) => {
                    Some(EnumeratedCapture::Copy)
                }
                (SourceCaptureMode::Move, Scalar::OwnedBuffer) => Some(EnumeratedCapture::Move),
                (SourceCaptureMode::Read { .. }, Scalar::ReadLoan) => Some(EnumeratedCapture::Lend),
                (SourceCaptureMode::LendWrite { .. }, Scalar::WriteLease) => {
                    Some(EnumeratedCapture::LendWrite)
                }
                _ => None,
            };
            // Both must agree, on the verdict and on which transfer it is.
            // A disagreement in either direction refuses: the judgment cannot
            // admit what the enumeration refuses, and it cannot silently
            // withdraw what the enumeration admits either.
            let agrees = match (&derived, enumerated) {
                (Ok(admitted), Some(form)) => *admitted == form.as_admitted(),
                (Err(_), None) => true,
                _ => false,
            };
            if !agrees {
                return Err(self.bad(
                    capture.span,
                    "the derived transferability judgment and the capture enumeration disagree \
                     about this capture",
                ));
            }
            // Card CT47-COPY-CAPTURE. Where both sides agree the capture is
            // refused, the refusal is the judgment's own, attributed to this
            // capture at the crossing construct with the missing contract
            // named (`Q600`; condition widening stated in spec §L21). Before
            // this card the same shapes fell through to the executable-subset
            // catch-all below — a `CEmission` (`Q500`) statement about the
            // backend rather than about the boundary. The catch-all is kept,
            // unedited, as the fail-closed backstop.
            if let Err(refusal) = derived {
                return Err(failure(
                    capture.span,
                    SemanticPass::Concurrency,
                    transfer_refusal_message(place_type, &capture.mode, *parameter_type, refusal),
                ));
            }
            let operand = match enumerated {
                Some(EnumeratedCapture::Copy) => ScalarOperand::Slot(symbol.0 as usize),
                Some(EnumeratedCapture::Move) => ScalarOperand::Move(symbol.0 as usize),
                Some(EnumeratedCapture::Lend) => {
                    // The share the *borrower* receives. Whether the resource
                    // has that much authority left to give is the ledger's
                    // decision, on the path that reaches this creation.
                    let SourceCaptureMode::Read { share } = &capture.mode else {
                        // Unreachable: only a `read` capture enumerates as a
                        // lend. Refused rather than panicked on.
                        return Err(self.bad(capture.span, "a lend capture without a share"));
                    };
                    loans += 1;
                    ScalarOperand::Loan {
                        slot: symbol.0 as usize,
                        share: read_share(&share.value, share.span)?,
                    }
                }
                Some(EnumeratedCapture::LendWrite) => {
                    // The range the worker receives. Whether it is disjoint
                    // from every outstanding lease is the ledger's decision,
                    // on the path that reaches this creation, by the wave-29
                    // overlap rule; the two decisions made *here* are the
                    // check-time ones — the bounds are literals, the range is
                    // nonempty, and it names bytes the payload has.
                    let SourceCaptureMode::LendWrite { low, high } = &capture.mode else {
                        // Unreachable: only a `lend` capture enumerates as a
                        // write lend. Refused rather than panicked on.
                        return Err(self.bad(capture.span, "a write lend capture without bounds"));
                    };
                    let low_value = lease_bound_value(low)?;
                    let high_value = lease_bound_value(high)?;
                    if low_value >= high_value {
                        return Err(failure(
                            capture.span,
                            SemanticPass::Concurrency,
                            format!(
                                "a write lend must cover at least one byte: \
                                 [{low_value}, {high_value}) is empty"
                            ),
                        ));
                    }
                    if high_value > qcore_core::task_execution::BUFFER_PAYLOAD_BYTES {
                        return Err(failure(
                            capture.span,
                            SemanticPass::Concurrency,
                            format!(
                                "a write lend must stay inside the payload's {} bytes: \
                                 [{low_value}, {high_value}) ends past it",
                                qcore_core::task_execution::BUFFER_PAYLOAD_BYTES
                            ),
                        ));
                    }
                    leases += 1;
                    ScalarOperand::Lease {
                        slot: symbol.0 as usize,
                        low: low_value,
                        high: high_value,
                    }
                }
                None => {
                    return Err(self.bad(
                        capture.span,
                        "only copy i32, move own<Buffer>, read own<Buffer> share FULL and lend own<Buffer> LOW..HIGH captures are executable",
                    ));
                }
            };
            arguments.push(operand);
        }
        if loans > 1 {
            return Err(self.bad(
                span,
                "this profile admits at most one read loan per task creation",
            ));
        }
        if leases > 1 {
            return Err(self.bad(
                span,
                "this profile admits at most one write lease per task creation",
            ));
        }
        if loans != 0 && leases != 0 {
            return Err(self.bad(
                span,
                "one creation cannot both lend a read share and lease a write range",
            ));
        }
        Ok((scope_id, arguments))
    }

    fn result_match(
        &mut self,
        statement: &TypedStatement,
        tokens: &[Token],
    ) -> Result<bool, SourceDiagnostic> {
        let span = statement.span;
        let resolved = statement
            .result_match
            .as_ref()
            .ok_or_else(|| self.bad(span, "missing resolved Result match metadata"))?;
        let syntax = &resolved.syntax;
        if syntax.span != span
            || resolved.arms.len() != 2
            || syntax.arms.len() != 2
            || !word(tokens.first(), "match")
            || tokens.get(1).map(|t| t.span.start) != Some(syntax.scrutinee_span.start)
        {
            return Err(self.bad(
                span,
                "Result match requires one complete scrutinee and exactly two canonical arms",
            ));
        }
        let open = tokens
            .iter()
            .position(|t| t.span.start >= syntax.scrutinee_span.end)
            .ok_or_else(|| self.bad(span, "missing Result match body"))?;
        if open < 2
            || tokens[open - 1].span.end != syntax.scrutinee_span.end
            || matching(tokens, open, Symbol::LeftBrace, Symbol::RightBrace)
                != Some(tokens.len() - 1)
        {
            return Err(self.bad(
                span,
                "Result match must consume its complete expression and outer body",
            ));
        }
        let mut cursor = open + 1;
        let mut ok = None;
        let mut err = None;
        for (index, arm) in resolved.arms.iter().enumerate() {
            if arm.syntax != syntax.arms[index] {
                return Err(self.bad(span, "Result arm metadata disagrees with source"));
            }
            let pattern = arm.syntax.pattern.as_ref().ok_or_else(|| self.bad(arm.syntax.pattern_span, "only exact Result::Ok { value: name } and Result::Err { error: name } patterns are supported"))?;
            let binder = arm.binding.ok_or_else(|| {
                self.bad(
                    arm.syntax.pattern_span,
                    "missing resolved Result arm binder",
                )
            })?;
            let end = tokens
                .iter()
                .position(|t| t.span.end == arm.syntax.pattern_span.end)
                .ok_or_else(|| self.bad(span, "Result pattern boundary is unavailable"))?;
            if tokens.get(cursor).map(|t| t.span.start) != Some(arm.syntax.pattern_span.start)
                || end < cursor
                || !symbol(tokens.get(end + 1), Symbol::FatArrow)
                || tokens.get(end + 2).map(|t| t.span.start) != Some(arm.syntax.body.span.start)
            {
                return Err(self.bad(span, "unsupported tokens around Result match arm"));
            }
            let body_end = matching(tokens, end + 2, Symbol::LeftBrace, Symbol::RightBrace)
                .ok_or_else(|| self.bad(span, "invalid Result arm body"))?;
            if tokens[body_end].span.end != arm.syntax.body.span.end {
                return Err(self.bad(span, "Result arm body boundary mismatch"));
            }
            cursor = body_end + 1;
            let slot = match pattern.kind {
                ResultPatternKind::Ok => &mut ok,
                ResultPatternKind::Err => &mut err,
            };
            if slot.replace((index, binder)).is_some() {
                return Err(self.bad(arm.syntax.pattern_span, "duplicate Result match variant"));
            }
        }
        if cursor != tokens.len() - 1 {
            return Err(self.bad(span, "unsupported trailing Result match tokens"));
        }
        let (ok_index, ok_binder) =
            ok.ok_or_else(|| self.bad(span, "Result match lacks Ok arm"))?;
        let (err_index, err_binder) =
            err.ok_or_else(|| self.bad(span, "Result match lacks Err arm"))?;
        let success = self.block_id(span)?;
        let error = self.block_id(span)?;
        let (ok_type, err_type) = match resolved.scrutinee.as_ref() {
            Some(ResolvedResultMatchScrutinee::Task(task)) => {
                if task.syntax.propagates_error || task.syntax.span != syntax.scrutinee_span {
                    return Err(self.bad(
                        span,
                        "Result match requires the whole unpropagated task expression",
                    ));
                }
                match &task.kind {
                    ResolvedTaskExpressionKind::Spawn {
                        scope,
                        worker,
                        captures,
                    } => {
                        let (scope, arguments) =
                            self.spawn_inputs(span, *scope, *worker, captures)?;
                        self.finish(
                            span,
                            TaskTerminatorKind::TrySpawn {
                                scope,
                                worker: worker.0 as usize,
                                arguments,
                                handle_slot: ok_binder.0 as usize,
                                success,
                                spawn_error: error,
                            },
                        );
                        (
                            ValueType::Handle(*worker),
                            ValueType::KnownError("SpawnError".into()),
                        )
                    }
                    ResolvedTaskExpressionKind::Join { .. } => {
                        let result_slot = self.temporary(span)?;
                        let mut scrutinee_statement = statement.clone();
                        scrutinee_statement.task_expressions = vec![(**task).clone()];
                        let kind = self
                            .task_expression(
                                &scrutinee_statement,
                                &tokens[1..open],
                                Some(result_slot),
                            )?
                            .ok_or_else(|| self.bad(span, "join match requires a Result value"))?;
                        let ValueType::Result {
                            value,
                            errors,
                            error_payload,
                        } = kind
                        else {
                            return Err(self.bad(span, "join match scrutinee is not a Result"));
                        };
                        self.finish(
                            span,
                            TaskTerminatorKind::TryOutcome {
                                result_slot,
                                value_slot: ok_binder.0 as usize,
                                error_slot: err_binder.0 as usize,
                                success,
                                error,
                            },
                        );
                        (
                            ValueType::Scalar(value),
                            ValueType::Error {
                                errors,
                                payload: error_payload,
                            },
                        )
                    }
                }
            }
            Some(ResolvedResultMatchScrutinee::Value(symbol_id)) => {
                let [token] = &tokens[1..open] else {
                    return Err(
                        self.bad(span, "Result match requires one immutable Result binding")
                    );
                };
                let spelling =
                    name(Some(token)).ok_or_else(|| self.bad(span, "invalid Result binding"))?;
                let (found, kind) = self.lookup(spelling, syntax.scrutinee_span)?;
                if found != *symbol_id {
                    return Err(self.bad(span, "Result binding identity mismatch"));
                }
                let ValueType::Result {
                    value,
                    errors,
                    error_payload,
                } = kind
                else {
                    return Err(self.bad(span, "match requires an initialized task Result"));
                };
                self.finish(
                    span,
                    TaskTerminatorKind::TryOutcome {
                        result_slot: found.0 as usize,
                        value_slot: ok_binder.0 as usize,
                        error_slot: err_binder.0 as usize,
                        success,
                        error,
                    },
                );
                (
                    ValueType::Scalar(value),
                    ValueType::Error {
                        errors,
                        payload: error_payload,
                    },
                )
            }
            None => {
                let scrutinee = tokens_for(self.source, syntax.scrutinee_span);
                let initial = self
                    .allocation_input(&scrutinee, span)?
                    .ok_or_else(|| self.bad(span, "unsupported Result match scrutinee"))?;
                self.finish(
                    span,
                    TaskTerminatorKind::TryAllocateBuffer {
                        initial,
                        value_slot: ok_binder.0 as usize,
                        success,
                        allocation_error: error,
                    },
                );
                (
                    ValueType::Scalar(Scalar::OwnedBuffer),
                    ValueType::KnownError("AllocError".into()),
                )
            }
        };
        let outer_environment = self.environment.clone();
        let outer_scopes = self.scopes.clone();
        let mut continuation = None;
        for (index, arm) in resolved.arms.iter().enumerate() {
            let (target, binder, kind) = if index == ok_index {
                (success, ok_binder, ok_type.clone())
            } else {
                debug_assert_eq!(index, err_index);
                (error, err_binder, err_type.clone())
            };
            self.current = target;
            self.environment = outer_environment.clone();
            self.scopes = outer_scopes.clone();
            self.environment.push(BTreeMap::from([(
                arm.syntax.pattern.as_ref().unwrap().binding.value.clone(),
                (binder, kind),
            )]));
            let arm_statements = statement
                .nested
                .get(arm.statement_range.clone())
                .ok_or_else(|| self.bad(span, "Result arm statement range is invalid"))?;
            let terminated = self.block(&arm.syntax.body, arm_statements)?;
            self.environment.pop();
            if !terminated {
                let join = match continuation {
                    Some(id) => id,
                    None => {
                        let id = self.block_id(span)?;
                        continuation = Some(id);
                        id
                    }
                };
                self.finish(span, TaskTerminatorKind::Goto { target: join });
            }
        }
        self.environment = outer_environment;
        self.scopes = outer_scopes;
        if let Some(block) = continuation {
            self.current = block;
        }
        Ok(continuation.is_none())
    }

    fn task_expression(
        &mut self,
        statement: &TypedStatement,
        rhs: &[Token],
        slot: Option<usize>,
    ) -> Result<Option<ValueType>, SourceDiagnostic> {
        let span = statement.span;
        if statement.task_expressions.len() != 1 {
            return Err(self.bad(
                span,
                "task execution requires exactly one complete task expression",
            ));
        }
        let task = &statement.task_expressions[0];
        if rhs.first().map(|t| t.span.start) != Some(task.syntax.span.start)
            || rhs.last().map(|t| t.span.end) != Some(task.syntax.span.end)
        {
            return Err(self.bad(span, "task expression must consume its complete statement or initializer; suffixes and surrounding operations are unsupported"));
        }
        match &task.kind {
            ResolvedTaskExpressionKind::Spawn {
                scope,
                worker,
                captures,
            } => {
                let Some(slot) = slot else {
                    return Err(self.bad(span, "spawn requires a fresh handle binding"));
                };
                if !task.syntax.propagates_error {
                    return Err(self.bad(
                        span,
                        "spawn requires immediate error discrimination with `?` in this profile",
                    ));
                }
                if !self.signature.effects.spawn
                    || !self.signature.effects.errors.contains("SpawnError")
                {
                    return Err(failure(
                        span,
                        SemanticPass::TypedErrors,
                        "spawn requires spawn and throws<SpawnError> effects",
                    ));
                }
                let (scope_id, arguments) = self.spawn_inputs(span, *scope, *worker, captures)?;
                let success = self.block_id(span)?;
                let spawn_error = self.error_block(span, ErrorOperand::Tag("SpawnError".into()))?;
                self.finish(
                    span,
                    TaskTerminatorKind::TrySpawn {
                        scope: scope_id,
                        worker: worker.0 as usize,
                        arguments,
                        handle_slot: slot,
                        success,
                        spawn_error,
                    },
                );
                self.current = success;
                Ok(Some(ValueType::Handle(*worker)))
            }
            ResolvedTaskExpressionKind::Join { handle } => {
                if !self.signature.effects.join || !self.signature.effects.may_block {
                    return Err(failure(
                        span,
                        SemanticPass::TypedErrors,
                        "join requires join and may_block effects",
                    ));
                }
                let (handle, kind) = self.lookup_place(handle)?;
                let ValueType::Handle(worker) = kind else {
                    return Err(failure(
                        span,
                        SemanticPass::OrdinaryTypes,
                        "join requires a task handle, not a scalar or undiscriminated result",
                    ));
                };
                let worker = &self.signatures[&worker];
                let value = worker.result;
                let errors = worker.effects.errors.clone();
                // The worker's own declared payload row, not the caller's: a
                // caller cannot decide that a joined error holds nothing.
                let error_payload = !worker.effects.error_payloads.is_empty();
                if task.syntax.propagates_error {
                    let result_slot = self.temporary(span)?;
                    let value_slot = match slot {
                        Some(slot) => slot,
                        None => self.temporary(span)?,
                    };
                    self.instruction(
                        span,
                        TaskInstructionKind::Join {
                            handle: handle.0 as usize,
                            outcome_slot: Some(result_slot),
                        },
                    );
                    let kind = self.propagate(
                        span,
                        result_slot,
                        value_slot,
                        value,
                        &errors,
                        error_payload,
                    )?;
                    if slot.is_none() && value != Scalar::Unit {
                        return Err(self.bad(span, "discarded join result must be unit"));
                    }
                    Ok(slot.map(|_| kind))
                } else {
                    if slot.is_none() && (value != Scalar::Unit || !errors.is_empty()) {
                        return Err(self.bad(span, "discarded join result must be infallible unit"));
                    }
                    self.instruction(
                        span,
                        TaskInstructionKind::Join {
                            handle: handle.0 as usize,
                            outcome_slot: slot,
                        },
                    );
                    Ok(slot.map(|_| ValueType::Result {
                        value,
                        errors,
                        error_payload,
                    }))
                }
            }
        }
    }

    fn block(
        &mut self,
        block: &SourceBlock,
        typed: &[TypedStatement],
    ) -> Result<bool, SourceDiagnostic> {
        if block.statements.len() != typed.len() {
            return Err(self.bad(block.span, "task block metadata mismatch"));
        }
        let mut terminated = false;
        for (statement, typed) in block.statements.iter().zip(typed) {
            if terminated {
                return Err(self.bad(
                    statement.span,
                    "unreachable statements after return/raise are not admitted by task execution",
                ));
            }
            if statement.span != typed.span {
                return Err(self.bad(statement.span, "task statement metadata mismatch"));
            }
            let tokens = tokens_for(self.source, statement.span);
            match &statement.value {
                StatementKind::Match { .. } => {
                    terminated = self.result_match(typed, &tokens)?;
                }
                StatementKind::ThreadScope { name, body } => {
                    if tokens.len() < 5
                        || !word(tokens.first(), "thread")
                        || !word(tokens.get(1), "scope")
                        || name != super_name(tokens.get(2)).unwrap_or("")
                        || !symbol(tokens.get(3), Symbol::LeftBrace)
                        || tokens[3].span.start != body.span.start
                        || matching(&tokens, 3, Symbol::LeftBrace, Symbol::RightBrace)
                            != Some(tokens.len() - 1)
                        || tokens.last().map(|token| token.span.end) != Some(body.span.end)
                        || !typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(
                            statement.span,
                            "thread scope header must be consumed exactly",
                        ));
                    }
                    let scope = typed.body_binding.ok_or_else(|| {
                        self.bad(statement.span, "missing typed thread-scope binder")
                    })?;
                    self.instruction(
                        statement.span,
                        TaskInstructionKind::EnterScope {
                            scope: u64::from(scope.0),
                        },
                    );
                    self.scopes.push(u64::from(scope.0));
                    self.environment
                        .push(BTreeMap::from([(name.clone(), (scope, ValueType::Scope))]));
                    terminated = self.block(body, &typed.nested)?;
                    self.environment.pop();
                    self.scopes.pop();
                    if !terminated {
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::ExitScope {
                                scope: u64::from(scope.0),
                            },
                        );
                    }
                }
                StatementKind::Let {
                    binding: Some(binding),
                } => {
                    if !word(tokens.first(), "let")
                        || name(tokens.get(1)) != Some(binding.as_str())
                        || !symbol(tokens.last(), Symbol::Semicolon)
                    {
                        return Err(self.bad(
                            statement.span,
                            "only a single immutable name binding is supported",
                        ));
                    }
                    let equal = tokens
                        .iter()
                        .position(|token| symbol(Some(token), Symbol::Equal))
                        .ok_or_else(|| {
                            self.bad(statement.span, "task locals require initialized values")
                        })?;
                    let annotation = if equal == 2 {
                        None
                    } else if equal > 3 && symbol(tokens.get(2), Symbol::Colon) {
                        Some(scalar(&tokens[3..equal]).ok_or_else(|| {
                            self.bad(statement.span, "unsupported task local type")
                        })?)
                    } else {
                        return Err(self.bad(
                            statement.span,
                            "unsupported binding pattern/type annotation",
                        ));
                    };
                    let rhs = &tokens[equal + 1..tokens.len() - 1];
                    let id = typed
                        .binding
                        .ok_or_else(|| self.bad(statement.span, "missing typed local binder"))?;
                    let kind = if !typed.task_expressions.is_empty() {
                        self.task_expression(typed, rhs, Some(id.0 as usize))?
                            .unwrap()
                    } else if let Some(kind) =
                        self.buffer_initializer(rhs, statement.span, id.0 as usize)?
                    {
                        kind
                    } else if rhs.len() == 2 && symbol(rhs.get(1), Symbol::Question) {
                        let source_name = name(rhs.first()).ok_or_else(|| {
                            self.bad(
                                statement.span,
                                "result propagation requires an immutable result binding",
                            )
                        })?;
                        let (result, kind) = self.lookup(source_name, statement.span)?;
                        let ValueType::Result {
                            value,
                            errors,
                            error_payload,
                        } = kind
                        else {
                            return Err(
                                self.bad(statement.span, "`?` requires a collected task result")
                            );
                        };
                        self.propagate(
                            statement.span,
                            result.0 as usize,
                            id.0 as usize,
                            value,
                            &errors,
                            error_payload,
                        )?
                    } else if let Some(Scalar::Fixed(ty)) = annotation
                        && let Some(literal) = self.fixed_literal(rhs, ty, statement.span)
                    {
                        // Card `W48-QR-SCALAR-TYPES-R4`: a literal takes the
                        // annotation's type, range-checked against it.
                        let value = literal?;
                        if let ScalarOperand::Fixed { value, .. } = value {
                            self.known.insert(id, value);
                        }
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::BindScalar {
                                slot: id.0 as usize,
                                value,
                            },
                        );
                        ValueType::Scalar(Scalar::Fixed(ty))
                    } else if let Some(parsed) = {
                        let environment = &self.environment;
                        crate::source_value_statement::parse_value_expression(
                            &tokens[equal + 1..],
                            |name| {
                                environment
                                    .iter()
                                    .rev()
                                    .find_map(|scope| scope.get(name))
                                    .and_then(|(symbol, kind)| match kind {
                                        ValueType::Scalar(
                                            scalar @ (Scalar::I32 | Scalar::Fixed(_)),
                                        ) => Some((*symbol, *scalar)),
                                        _ => None,
                                    })
                            },
                        )
                    } {
                        // Card `W48-QR-SCALAR-TYPES-R4`: a value statement. A
                        // shape it cannot read is this route's shape refusal.
                        let syntax = parsed.map_err(|message| self.bad(statement.span, message))?;
                        self.value_statement(syntax, statement.span, id)?
                    } else {
                        let (value, scalar) = self.scalar_operand(rhs, statement.span)?;
                        // A copy of a known binding is known.
                        let known = match &value {
                            ScalarOperand::I32(value) => Some(i128::from(*value)),
                            ScalarOperand::Fixed { value, .. } => Some(*value),
                            ScalarOperand::Slot(slot) => self
                                .known
                                .iter()
                                .find(|(symbol, _)| symbol.0 as usize == *slot)
                                .map(|(_, value)| *value),
                            _ => None,
                        };
                        if let Some(known) = known {
                            self.known.insert(id, known);
                        }
                        self.instruction(
                            statement.span,
                            TaskInstructionKind::BindScalar {
                                slot: id.0 as usize,
                                value,
                            },
                        );
                        ValueType::Scalar(scalar)
                    };
                    if annotation.is_some_and(|annotation| kind != ValueType::Scalar(annotation)) {
                        return Err(failure(
                            statement.span,
                            SemanticPass::OrdinaryTypes,
                            "local result does not match its declared scalar type",
                        ));
                    }
                    self.environment
                        .last_mut()
                        .unwrap()
                        .insert(binding.clone(), (id, kind));
                }
                StatementKind::Drop { place: Some(place) } => {
                    if tokens.len() != 3
                        || !word(tokens.first(), "drop")
                        || name(tokens.get(1)) != Some(place.as_str())
                        || !symbol(tokens.get(2), Symbol::Semicolon)
                    {
                        return Err(self.bad(statement.span, "buffer drop requires one exact name"));
                    }
                    let (id, kind) = self.lookup(place, statement.span)?;
                    if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                        return Err(self.bad(statement.span, "drop requires an owned buffer"));
                    }
                    self.instruction(
                        statement.span,
                        TaskInstructionKind::DropBuffer {
                            buffer: id.0 as usize,
                        },
                    );
                }
                StatementKind::Expression => {
                    // Card `COMPOSE-CONCURRENCY`. Tried first and only when
                    // composing, so off the composed route this statement is the
                    // unsupported expression statement it has always been.
                    if self.composing
                        && let Some(parsed) =
                            crate::source_std_io::std_io_statement(self.source, statement.span)
                    {
                        match parsed {
                            Ok(event) => {
                                self.source_line(statement.span, event)?;
                                continue;
                            }
                            Err(message) => {
                                // The shape refusal is `source.std-io.v1`'s own,
                                // under that route's own code: the composition
                                // states no grammar of its own.
                                return Err(SourceDiagnostic {
                                    code: "Q030",
                                    pass: SemanticPass::StandardIo,
                                    span: statement.span,
                                    message: message.to_owned(),
                                });
                            }
                        }
                    }
                    if !symbol(tokens.last(), Symbol::Semicolon)
                        || typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(
                            statement.span,
                            "unsupported expression statement in task execution",
                        ));
                    }
                    self.task_expression(typed, &tokens[..tokens.len() - 1], None)?;
                }
                StatementKind::Return => {
                    if !symbol(tokens.last(), Symbol::Semicolon)
                        || !word(tokens.first(), "return")
                        || !typed.task_expressions.is_empty()
                    {
                        return Err(self.bad(statement.span, "unsupported task return expression"));
                    }
                    let (value, kind) = if tokens.len() == 4 && word(tokens.get(1), "move") {
                        let buffer = name(tokens.get(2)).ok_or_else(|| {
                            self.bad(statement.span, "owned return requires a name")
                        })?;
                        let (id, kind) = self.lookup(buffer, statement.span)?;
                        if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                            return Err(
                                self.bad(statement.span, "move return requires own<Buffer>")
                            );
                        }
                        (ScalarOperand::Move(id.0 as usize), Scalar::OwnedBuffer)
                    } else if tokens.len() == 2 {
                        (ScalarOperand::Unit, Scalar::Unit)
                    } else if let Scalar::Fixed(ty) = self.signature.result
                        && let Some(literal) =
                            self.fixed_literal(&tokens[1..tokens.len() - 1], ty, statement.span)
                    {
                        // Card `W48-QR-SCALAR-TYPES-R4`: a returned literal
                        // takes the declared result's type.
                        (literal?, Scalar::Fixed(ty))
                    } else {
                        self.scalar_operand(&tokens[1..tokens.len() - 1], statement.span)?
                    };
                    if kind != self.signature.result {
                        return Err(failure(
                            statement.span,
                            SemanticPass::OrdinaryTypes,
                            "return value does not match the declared task function result",
                        ));
                    }
                    self.instructions
                        .extend(self.exit_instructions(statement.span));
                    self.finish(statement.span, TaskTerminatorKind::Return { value });
                    terminated = true;
                }
                StatementKind::Raise => {
                    let (error, payload) = if tokens.len() == 3
                        && word(tokens.first(), "raise")
                        && symbol(tokens.get(2), Symbol::Semicolon)
                    {
                        let binding = name(tokens.get(1)).ok_or_else(|| {
                            self.bad(statement.span, "raise requires a bound typed error")
                        })?;
                        let (id, kind) = self.lookup(binding, statement.span)?;
                        let error = match kind {
                            ValueType::KnownError(tag) => {
                                if !self.signature.effects.errors.contains(&tag) {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "bound error is absent from throws effects",
                                    ));
                                }
                                ErrorOperand::Tag(tag)
                            }
                            ValueType::Error { errors, payload } => {
                                if !errors.is_subset(&self.signature.effects.errors) {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "bound error variants are absent from throws effects",
                                    ));
                                }
                                // Re-raising a bound error keeps its tag, and a
                                // tag alone cannot carry a Buffer onward: take
                                // the payload first, then decide what to do with
                                // it and which tag to raise.
                                if payload {
                                    return Err(failure(
                                        statement.span,
                                        SemanticPass::TypedErrors,
                                        "take the owned payload with qcore_error_payload before re-raising this error",
                                    ));
                                }
                                ErrorOperand::Slot(id.0 as usize)
                            }
                            _ => {
                                return Err(self.bad(
                                    statement.span,
                                    "raise requires a typed error arm binding",
                                ));
                            }
                        };
                        (error, None)
                    } else {
                        // `raise E::V;` is payload-free; `raise E::V(b);` moves
                        // exactly one owned Buffer into the raised error. The
                        // declaration decides which spelling is required.
                        let carried = tokens.len() == 8
                            && symbol(tokens.get(4), Symbol::LeftParen)
                            && symbol(tokens.get(6), Symbol::RightParen);
                        if (tokens.len() != 5 && !carried)
                            || !word(tokens.first(), "raise")
                            || !symbol(tokens.get(2), Symbol::ColonColon)
                            || !symbol(tokens.last(), Symbol::Semicolon)
                        {
                            return Err(self.bad(
                                statement.span,
                                "task raise requires a bound error, Error::Variant, or Error::Variant(buffer)",
                            ));
                        }
                        let error_type = name(tokens.get(1))
                            .ok_or_else(|| self.bad(statement.span, "invalid error type"))?;
                        let variant = name(tokens.get(3))
                            .ok_or_else(|| self.bad(statement.span, "invalid error variant"))?;
                        let tag = format!("{error_type}::{variant}");
                        if !self
                            .errors
                            .get(error_type)
                            .is_some_and(|declared| declared.variants.contains(&tag))
                            || !self.signature.effects.errors.contains(&tag)
                        {
                            return Err(failure(
                                statement.span,
                                SemanticPass::TypedErrors,
                                "raised variant must be declared and covered by throws effects",
                            ));
                        }
                        if self.signature.effects.error_payloads.contains(&tag) != carried {
                            return Err(failure(
                                statement.span,
                                SemanticPass::TypedErrors,
                                "raised payload disagrees with the declared error variant",
                            ));
                        }
                        let payload = if carried {
                            let buffer = name(tokens.get(5)).ok_or_else(|| {
                                self.bad(statement.span, "an error payload requires one name")
                            })?;
                            let (id, kind) = self.lookup(buffer, statement.span)?;
                            // A loan, a scalar and a Result each fail here: the
                            // payload has to be an owned Buffer this frame may
                            // give away, and the raise consumes it.
                            if kind != ValueType::Scalar(Scalar::OwnedBuffer) {
                                return Err(self
                                    .bad(statement.span, "an error payload requires own<Buffer>"));
                            }
                            Some(ScalarOperand::Move(id.0 as usize))
                        } else {
                            None
                        };
                        (ErrorOperand::Tag(tag), payload)
                    };
                    self.instructions
                        .extend(self.exit_instructions(statement.span));
                    self.finish(statement.span, TaskTerminatorKind::Raise { error, payload });
                    terminated = true;
                }
                _ => {
                    return Err(self.bad(
                        statement.span,
                        "statement is outside the copy-scalar task execution profile",
                    ));
                }
            }
        }
        Ok(terminated)
    }
}

// Avoid a pattern binding named `name` obscuring the token helper in scope arms.
fn super_name(token: Option<&Token>) -> Option<&str> {
    name(token)
}
