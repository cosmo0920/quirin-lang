//! The pipe operator `|>` and the claimed logical-operator tokens (card
//! `PIPE-OPERATOR`).
//!
//! # What a pipe is here
//!
//! `|>` is a **pure token-stream desugaring that runs before parsing**. It
//! permutes the tokens of one statement into the tokens the same program would
//! have if it had been written without a pipe, and every later phase — name
//! resolution, the route selectors, custody, effects, linearity, concurrency,
//! whole-program checking, reference execution and C emission — sees an
//! ordinary statement. There is no pipe node, no pipe rule and no pipe pass
//! downstream: [`SourceProgram::syntax_equivalent`] holds between a piped
//! program and its hand-written form, which is the strongest available
//! statement of "the desugaring adds no semantics".
//!
//! ```text
//! statement ::= OPERAND "|>" STAGE ";"
//! OPERAND   ::= token+          -- passed through verbatim, e.g. `x`, `-7`, `move b`
//! STAGE     ::= "print" WORD                 -- a `source.std-io.v1` sink
//!             | "log" WORD WORD              -- a `source.std-io.v1` sink
//!             | NAME [ "(" [arg {"," arg}] ")" ]   -- a call stage
//! ```
//!
//! | Written | Desugars to |
//! |---|---|
//! | `total \|> print dec;` | `print dec total;` |
//! | `total \|> log fatal dec;` | `log fatal dec total;` |
//! | `x \|> f;` | `f(x);` |
//! | `x \|> f(a, b);` | `f(x, a, b);` |
//! | `move b \|> release;` | `release(move b);` |
//!
//! # Insertion position
//!
//! The piped value occupies the stage's **first operand position**. For a
//! parenthesized call that is immediately after `(` — Elixir-style
//! first-argument insertion, chosen because QCore is subject-first and has no
//! currying, so there is no partial application for a last-argument rule to
//! compose with. For a statement stage the head and its closed selector words
//! (`print dec`, `log fatal dec`) are not arguments, so the first operand
//! position is after them; both v1 statement stages take exactly one operand,
//! so "first operand position" and "append" coincide and no selector-arity
//! table is needed.
//!
//! # Attribution
//!
//! Every token that came from the source keeps its own span, unmoved. A
//! refusal inside the desugared statement therefore points at the byte range
//! the reader wrote: an operand fault at the operand, a stage fault at the
//! stage. Only `(`, `)` and `,` are synthesized, and each is anchored
//! zero-width at the `|>` it came from, so even a fault that lands on
//! punctuation names the pipe rather than an invented position.
//!
//! # What v1 does not do, and why
//!
//! * **A pipeline has exactly one stage.** `x |> f |> g` refuses at `Q033`.
//!   Two independent blockers, both measured: a call stage's result has no
//!   name, because QCore source has no expression tier at all
//!   (`StatementKind::Expression` is payload-free and an expression statement
//!   refuses at `Q500`); and expanding one pipeline into a statement
//!   *sequence* is incompatible with the byte-span statement windows every
//!   route uses to read its own tokens, since the shared operand would sit
//!   inside two overlapping statement spans. Named follow-on `PIPE-CHAIN`.
//! * **A pipeline is a whole statement.** `let r = x |> f succeeds …;` refuses
//!   at `Q032`. Named follow-on `PIPE-LET`.
//! * **`flush` takes no operand**, so piping into it refuses at `Q033`.
//!
//! # The claimed logical operators
//!
//! `|` (logical or) and `&&` (logical and) are lexed as tokens by this card so
//! that a later wave inherits them without a collision, and both refuse at
//! `Q034` with an actionable message. Their short-circuit lowering is *not*
//! implemented, on a measured blocker rather than a scheduling one: the
//! executable subset has no branch construct to lower onto. `if` itself
//! refuses at `Q500` ("structured control flow is not in the Cycle 12
//! executable subset"), the lowering IR
//! [`qcore_core::cleanup_lowering::SourceOp`] has no branch constructor, and
//! there are zero `if` statements in the 294 source-side `.qcore` files.
//! Follow-ons `OR-LOWERING` and `AND-LOWERING`.
use crate::frontend::{SemanticPass, SourceDiagnostic};
use crate::lexer::{Span, Symbol, Token, TokenKind};

/// A refusal recorded while desugaring. Carried on the parsed program and
/// raised as a [`SourceDiagnostic`] by [`check_source_operators`], so a
/// refusal is attributed like every other frontend diagnostic rather than
/// collapsing into an unattributed parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipeRefusal {
    pub code: &'static str,
    pub span: Span,
    pub message: String,
}

/// The statement heads that are sink stages, with the number of closed
/// selector words each takes before its operand. Anything not in this table is
/// a call stage or a refusal; the table is closed on purpose, so a new
/// statement form does not become pipeable by accident.
const SINK_STAGES: &[(&str, usize)] = &[("print", 1), ("log", 2)];

/// Statement heads that take no operand at all. Piping into one is refused
/// rather than silently dropping the value.
const OPERANDLESS_STAGES: &[&str] = &["flush"];

fn refusal(code: &'static str, span: Span, message: impl Into<String>) -> PipeRefusal {
    PipeRefusal {
        code,
        span,
        message: message.into(),
    }
}

fn is(token: &Token, symbol: Symbol) -> bool {
    token.kind == TokenKind::Symbol(symbol)
}

fn word(token: &Token) -> Option<&str> {
    match &token.kind {
        TokenKind::Word(word) => Some(word.as_str()),
        _ => None,
    }
}

/// A synthesized delimiter, anchored zero-width at the `|>` it came from.
fn synthesized(symbol: Symbol, pipe: Span) -> Token {
    Token {
        kind: TokenKind::Symbol(symbol),
        span: Span {
            start: pipe.start,
            end: pipe.start,
        },
    }
}

/// Rewrite every well-formed pipeline in `tokens` and record a refusal for
/// every ill-formed one.
///
/// The function is total: an ill-formed pipeline is left **untouched** rather
/// than half-rewritten, so its residual `|>` also refuses downstream and the
/// result is fail-closed twice over. `tokens` is expected without its trailing
/// [`TokenKind::Eof`].
#[must_use]
pub fn desugar_pipes(tokens: Vec<Token>) -> (Vec<Token>, Vec<PipeRefusal>) {
    let mut refusals = Vec::new();
    let runs = pipeline_runs(&tokens, &mut refusals);
    if runs.is_empty() {
        return (tokens, refusals);
    }

    let mut output: Vec<Token> = Vec::with_capacity(tokens.len() + 2 * runs.len());
    let mut cursor = 0usize;
    for run in runs {
        output.extend_from_slice(&tokens[cursor..run.start]);
        match rewrite(&tokens[run.start..run.end], run.pipe) {
            Ok(rewritten) => output.extend(rewritten),
            Err(error) => {
                refusals.push(error);
                output.extend_from_slice(&tokens[run.start..run.end]);
            }
        }
        cursor = run.end;
    }
    output.extend_from_slice(&tokens[cursor..]);
    (output, refusals)
}

/// One statement-shaped token run containing at least one top-level `|>`.
struct Run {
    /// First token of the run, inclusive.
    start: usize,
    /// One past the run's last token, which is always its `;`.
    end: usize,
    /// Span of the run's first `|>`, used to anchor synthesized delimiters.
    pipe: Span,
}

/// Locate the statement each top-level `|>` belongs to. A pipeline runs from
/// the token after the previous statement boundary (`;`, `{` or `}`) to its own
/// `;`, and may not cross a block.
fn pipeline_runs(tokens: &[Token], refusals: &mut Vec<PipeRefusal>) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut boundary = 0usize;
    let mut depth = 0i32;
    let mut pipe: Option<usize> = None;

    for (index, token) in tokens.iter().enumerate() {
        match &token.kind {
            TokenKind::Symbol(Symbol::LeftParen | Symbol::LeftBracket) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen | Symbol::RightBracket) => depth -= 1,
            TokenKind::Symbol(Symbol::PipeInto) if depth == 0 && pipe.is_none() => {
                pipe = Some(index);
            }
            TokenKind::Symbol(Symbol::LeftBrace | Symbol::RightBrace) if depth == 0 => {
                if let Some(at) = pipe.take() {
                    refusals.push(refusal(
                        "Q032",
                        tokens[at].span,
                        "a pipeline is one statement ending in `;`; this one reaches a block \
                         boundary first",
                    ));
                }
                boundary = index + 1;
            }
            TokenKind::Symbol(Symbol::Semicolon) if depth == 0 => {
                if let Some(at) = pipe.take() {
                    runs.push(Run {
                        start: boundary,
                        end: index + 1,
                        pipe: tokens[at].span,
                    });
                }
                boundary = index + 1;
            }
            _ => {}
        }
    }
    if let Some(at) = pipe {
        refusals.push(refusal(
            "Q032",
            tokens[at].span,
            "a pipeline is one statement ending in `;`; this one has no `;`",
        ));
    }
    runs
}

/// Rewrite one run `OPERAND |> STAGE ;` into the tokens of the equivalent
/// unpiped statement.
fn rewrite(run: &[Token], pipe: Span) -> Result<Vec<Token>, PipeRefusal> {
    let semicolon = run.last().expect("a run always ends in its `;`").clone();
    let body = &run[..run.len() - 1];

    // `let`/assignment pipelines are follow-on `PIPE-LET`: this profile admits
    // a pipeline only as a whole statement, so a binder would have to be
    // reassembled around the rewritten call.
    if let Some(equal) = body
        .iter()
        .position(|token| is(token, Symbol::Equal) || is(token, Symbol::Define))
    {
        return Err(refusal(
            "Q032",
            body[equal].span,
            "`source.pipe.v1` admits a pipeline as a whole statement; a pipeline on the right of \
             a binding is follow-on `PIPE-LET`",
        ));
    }

    let mut segments: Vec<&[Token]> = Vec::new();
    let mut segment_start = 0usize;
    let mut depth = 0i32;
    for (index, token) in body.iter().enumerate() {
        match &token.kind {
            TokenKind::Symbol(Symbol::LeftParen | Symbol::LeftBracket) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen | Symbol::RightBracket) => depth -= 1,
            TokenKind::Symbol(Symbol::PipeInto) if depth == 0 => {
                segments.push(&body[segment_start..index]);
                segment_start = index + 1;
            }
            _ => {}
        }
    }
    segments.push(&body[segment_start..]);

    let operand = segments[0];
    if operand.is_empty() {
        return Err(refusal(
            "Q032",
            pipe,
            "`|>` needs a value on its left; this pipeline pipes nothing",
        ));
    }
    if segments.len() > 2 {
        return Err(refusal(
            "Q033",
            segments[2].first().map_or(pipe, |token| token.span),
            "`source.pipe.v1` admits one stage per pipeline. A call stage's result has no name — \
             QCore source has no expression tier — and a chain of statement stages would need \
             statement windows this frontend keys by byte span. Follow-on `PIPE-CHAIN`; bind the \
             intermediate value instead",
        ));
    }
    let stage = segments[1];
    let Some(head) = stage.first() else {
        return Err(refusal(
            "Q032",
            pipe,
            "`|>` needs a stage on its right; this pipeline pipes into nothing",
        ));
    };

    let Some(name) = word(head) else {
        return Err(refusal(
            "Q032",
            head.span,
            "a pipeline stage is a statement head or a callee name",
        ));
    };

    if OPERANDLESS_STAGES.contains(&name) {
        return Err(refusal(
            "Q033",
            head.span,
            format!("`{name}` takes no operand, so nothing can be piped into it"),
        ));
    }

    let mut out = Vec::with_capacity(run.len() + 2);
    if let Some((_, selectors)) = SINK_STAGES.iter().find(|(head, _)| *head == name) {
        // A statement stage: the head and its closed selector words are not
        // arguments, so the operand follows them. Arity is left to the route's
        // own grammar, which already refuses a wrong selector or a wrong
        // operand count with its own diagnostic at the right span.
        if stage.len() != 1 + selectors {
            return Err(refusal(
                "Q032",
                head.span,
                format!(
                    "`{name}` names {selectors} selector word(s) before its operand; this stage \
                     names {}",
                    stage.len() - 1
                ),
            ));
        }
        out.extend_from_slice(stage);
        out.extend_from_slice(operand);
        out.push(semicolon);
        return Ok(out);
    }

    // A call stage. `f` and `f()` both take the operand as the sole argument;
    // `f(a, b)` takes it as the first of three.
    let existing: &[Token] = match stage.len() {
        1 => &[],
        _ => {
            if !is(&stage[1], Symbol::LeftParen) || !is(&stage[stage.len() - 1], Symbol::RightParen)
            {
                return Err(refusal(
                    "Q032",
                    stage[1].span,
                    "a call stage is a callee name, optionally followed by a parenthesized \
                     argument list",
                ));
            }
            &stage[2..stage.len() - 1]
        }
    };
    out.push(head.clone());
    out.push(synthesized(Symbol::LeftParen, pipe));
    out.extend_from_slice(operand);
    if !existing.is_empty() {
        out.push(synthesized(Symbol::Comma, pipe));
        out.extend_from_slice(existing);
    }
    out.push(synthesized(Symbol::RightParen, pipe));
    out.push(semicolon);
    Ok(out)
}

/// Raise the operator refusals recorded for one input file.
///
/// Called per input file before name/type checking, beside
/// [`crate::module_imports::check_source_imports`]. Two families:
///
/// * the desugarer's own refusals, carried on the program; and
/// * the claimed-but-unlowered logical operators, whose every occurrence
///   refuses here at `Q034`.
pub fn check_source_operators(
    refusals: &[PipeRefusal],
    tokens: &[Token],
) -> Result<(), Vec<SourceDiagnostic>> {
    let mut diagnostics: Vec<SourceDiagnostic> = refusals
        .iter()
        .map(|refusal| SourceDiagnostic {
            code: refusal.code,
            pass: SemanticPass::Pipe,
            span: refusal.span,
            message: refusal.message.clone(),
        })
        .collect();

    let mut index = 0usize;
    while index < tokens.len() {
        let token = &tokens[index];
        let message = match &token.kind {
            // `||` lexes as two `Pipe` tokens by the width-two rule. A
            // C-family programmer who typed it is redirected to the single-`|`
            // spelling, and the second `|` is consumed so one mistake produces
            // one diagnostic.
            TokenKind::Symbol(Symbol::Pipe)
                if tokens
                    .get(index + 1)
                    .is_some_and(|next| is(next, Symbol::Pipe)) =>
            {
                index += 1;
                "QCore spells logical or as a single `|`, not `||`. Its short-circuit lowering is \
                 follow-on `OR-LOWERING`: the executable subset has no branch construct to lower \
                 onto, and `if` itself refuses at `Q500`"
                    .to_owned()
            }
            TokenKind::Symbol(Symbol::Pipe) => "`|` is the logical-or operator. Its short-circuit \
                 lowering is follow-on `OR-LOWERING`: the executable subset has no branch \
                 construct to lower onto, and `if` itself refuses at `Q500`"
                .to_owned(),
            TokenKind::Symbol(Symbol::AndAnd) => "`&&` is the logical-and operator. Its \
                 short-circuit lowering is follow-on `AND-LOWERING`: the executable subset has no \
                 branch construct to lower onto, and `if` itself refuses at `Q500`. A single `&` \
                 is deliberately reserved for a future binary operation and is not logical and"
                .to_owned(),
            // A `|>` that survived desugaring belongs to a pipeline the
            // rewriter refused; that refusal is already in `diagnostics` and
            // this residual would only duplicate it.
            _ => {
                index += 1;
                continue;
            }
        };
        diagnostics.push(SourceDiagnostic {
            code: "Q034",
            pass: SemanticPass::Pipe,
            span: token.span,
            message,
        });
        index += 1;
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
        Err(diagnostics)
    }
}
