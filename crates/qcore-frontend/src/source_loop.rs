//! Exact source lowering for `source.loop.v1` — the checked bounded counted
//! loop (card `LOOP-V1`).
//!
//! ```text
//! stmt  ::= "for" NAME "in" "0" ".." BOUND "{" stmt* "}"
//! BOUND ::= INTEGER | NAME
//! ```
//!
//! # One reserved word, zero new tokens, and one contextual clause word
//!
//! **`for`** is the only word this card reserves. The constraint-14 census over
//! every `.qcore`/`.qc` file in the tree — 418 of them — found `for` **zero**
//! times outside a `//` comment, so `for x …` at a statement head was a `Q000`
//! parse failure before this card and the reservation collides with nothing.
//!
//! **`in` is deliberately not reserved.** The same census found it six times in
//! code position: four are parameter names that the referenced-names filter
//! already shields (a word followed by `:`), but two are genuine **references**
//! inside the pinned contract corpus —
//! `ensures normal resource_borrowed(in);` in
//! `contract/qcore/fluent_bit/input_chunk_helpers.qcore` and in a gateway test
//! fixture, both naming the `in` parameter of
//! `extern fn generate_chunk_name(in: ptr<mut, u8>, …)`. That is exactly the
//! shape card `STRING-SLICE` measured for `len`, and it takes exactly that
//! card's answer: **narrow the rule rather than edit a pinned corpus**. `in` is
//! skipped by `frontend::referenced_names` at the one position a `for` header
//! puts it — immediately after `for NAME` — and is an ordinary word everywhere
//! else.
//!
//! **`..`** is two existing `Symbol::Dot` tokens read positionally, on the
//! precedent that `||` was two `Symbol::Pipe` tokens before card
//! `OR-AND-LOWERING` gave it a token of its own. The lexer is untouched.
//!
//! # `0..BOUND` and not `LOW..HIGH`
//!
//! The low end is the literal `0`, written out rather than implied. Spelling it
//! keeps the form readable as a range while admitting exactly one range per
//! bound, so the iteration values are `0, 1, …, BOUND-1` with nothing to derive
//! — and a reader who writes `1..n` gets a refusal that names the rule instead
//! of a loop that runs a different number of times than it looks like it does.
//! A general low end is the follow-on `LOOP-RANGE-LOW`.
//!
//! # What this module does not decide
//!
//! It reads a header and nothing else. How many times the body runs, what one
//! iteration may do to custody, and what the body's statements *are* belong to
//! [`qcore_core::source_loop`] and to the admitting route respectively — a loop
//! body's statements are the route's own statements, read by the route's own
//! reader, so a shape one admits and the other refuses is not expressible.

use std::collections::BTreeMap;

use qcore_core::source_loop::{BoundRefusal, LoopRefusal, MAX_ITERATIONS};

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, StatementKind,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};

/// The statement head. The one word this card reserves.
pub const FOR_KEYWORD: &str = "for";

/// The range clause word. **Contextual**, never globally reserved — see the
/// module header for the census site that forbids a global reservation.
pub const IN_KEYWORD: &str = "in";

/// A read `for NAME in 0..BOUND` header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopHeader {
    pub variable: String,
    pub count: u32,
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::Loop,
        span,
        message: message.into(),
    }
}

/// Present a [`LoopRefusal`] as this pass's diagnostic, at its own code.
#[must_use]
pub fn refusal_diagnostic(refusal: &LoopRefusal, span: Span) -> SourceDiagnostic {
    diagnostic(refusal.code(), span, refusal.to_string())
}

/// The header tokens of a `for` statement: everything before its body's `{`.
#[must_use]
pub fn header_window(tokens: &[Token], span: Span) -> Vec<&Token> {
    let mut window: Vec<&Token> = tokens
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect();
    if let Some(brace) = window
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        window.truncate(brace);
    }
    window
}

fn word<'a>(window: &'a [&Token], cursor: usize) -> Option<&'a str> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn symbol(window: &[&Token], cursor: usize, expected: Symbol) -> bool {
    matches!(
        window.get(cursor).map(|token| &token.kind),
        Some(TokenKind::Symbol(symbol)) if *symbol == expected
    )
}

fn integer(window: &[&Token], cursor: usize) -> Option<i64> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => text.parse::<i64>().ok(),
        _ => None,
    }
}

/// Read one `for NAME in 0..BOUND` header.
///
/// `constants` is the admitting route's check-time-constant environment: the
/// names it has bound to integers at this point in the entry. A bound that is a
/// name resolves through it, which is the same decidable tier
/// `source.branch.v1` established for a condition and `source.buffer.v1` for an
/// index — and it is what `STD-BUFFER-COPY` needs, because a copy's length is
/// usually a name.
pub fn read_loop_header(
    window: &[&Token],
    span: Span,
    constants: &BTreeMap<String, i32>,
) -> Result<LoopHeader, SourceDiagnostic> {
    if word(window, 0) != Some(FOR_KEYWORD) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` reads `for NAME in 0..BOUND { … }`",
        ));
    }
    let Some(variable) = word(window, 1) else {
        return Err(diagnostic(
            "Q097",
            span,
            "`for` binds one index name; `source.loop.v1` reads `for NAME in 0..BOUND { … }`",
        ));
    };
    if word(window, 2) != Some(IN_KEYWORD) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` reads `for NAME in 0..BOUND { … }`; the range is introduced by `in`",
        ));
    }
    if integer(window, 3) != Some(0) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` counts from 0, written out: `for NAME in 0..BOUND { … }`. A range \
             with another low end is refused rather than silently re-based",
        ));
    }
    if !symbol(window, 4, Symbol::Dot) || !symbol(window, 5, Symbol::Dot) {
        return Err(diagnostic(
            "Q097",
            span,
            "`source.loop.v1` spells its range `0..BOUND`",
        ));
    }
    // `BOUND` is a literal or a name this entry bound to a check-time constant.
    let bound = match window.get(6).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => text.parse::<i64>().map_err(|_| {
            refusal_diagnostic(
                &LoopRefusal::Bound(BoundRefusal::NotConstant {
                    spelling: text.clone(),
                }),
                span,
            )
        })?,
        Some(TokenKind::Word(name)) => match constants.get(name) {
            Some(value) => i64::from(*value),
            None => {
                return Err(refusal_diagnostic(
                    &LoopRefusal::Bound(BoundRefusal::NotConstant {
                        spelling: name.clone(),
                    }),
                    span,
                ));
            }
        },
        // A negative literal lexes as `Minus` then `Integer`, so this is where
        // `for i in 0..-1` arrives.
        Some(TokenKind::Symbol(Symbol::Minus)) => match integer(window, 7) {
            Some(value) => -value,
            None => {
                return Err(diagnostic(
                    "Q097",
                    span,
                    "`source.loop.v1` reads `for NAME in 0..BOUND { … }`",
                ));
            }
        },
        _ => {
            return Err(diagnostic(
                "Q097",
                span,
                "`source.loop.v1` reads `for NAME in 0..BOUND { … }`",
            ));
        }
    };

    let refusal = if bound < 0 {
        Some(BoundRefusal::Negative {
            value: i32::try_from(bound).unwrap_or(i32::MIN),
        })
    } else if bound == 0 {
        Some(BoundRefusal::Zero)
    } else if bound > i64::from(MAX_ITERATIONS) {
        Some(BoundRefusal::TooLarge { value: bound })
    } else {
        None
    };
    if let Some(refusal) = refusal {
        return Err(refusal_diagnostic(&LoopRefusal::Bound(refusal), span));
    }

    Ok(LoopHeader {
        variable: variable.to_owned(),
        count: u32::try_from(bound).expect("the bound is inside 1..=MAX_ITERATIONS"),
    })
}

/// Whether this program writes a `for` loop anywhere, and where the first one
/// is.
///
/// Used to pin the measured refusal of every route that does **not** admit the
/// construct, so "correctly refused" is a statement a test can make.
#[must_use]
pub fn loop_span(program: &SourceProgram) -> Option<Span> {
    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body
            && let Some(span) = block_loop_span(body)
        {
            return Some(span);
        }
    }
    None
}

/// The span of the first `for` statement in one block, at any depth.
#[must_use]
pub fn block_loop_span(block: &SourceBlock) -> Option<Span> {
    for statement in &block.statements {
        if matches!(statement.value, StatementKind::For { .. }) {
            return Some(statement.span);
        }
        for nested in statement.value.nested_blocks() {
            if let Some(span) = block_loop_span(nested) {
                return Some(span);
            }
        }
    }
    None
}

/// Whether this program writes a `for` loop anywhere.
#[must_use]
pub fn has_source_loop(program: &SourceProgram) -> bool {
    loop_span(program).is_some()
}
