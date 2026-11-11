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

//! Structured task syntax and role-specific name resolution only. These records
//! are not task typing, custody, Send/Sync, scheduling or executable evidence.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourcePlace {
    Name(Located<String>),
    Dereference(Box<Located<SourcePlace>>),
    Field {
        base: Box<Located<SourcePlace>>,
        field: Located<String>,
    },
    Index {
        base: Box<Located<SourcePlace>>,
        index: SourceIndexExpression,
    },
}

impl SourcePlace {
    pub fn root(&self) -> &Located<String> {
        match self {
            Self::Name(name) => name,
            Self::Dereference(base) | Self::Field { base, .. } | Self::Index { base, .. } => {
                base.value.root()
            }
        }
    }
}

/// An index expression is retained structurally, not evaluated or proved in
/// bounds. The enclosing place tree never treats its tokens as an identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIndexExpression {
    pub span: Span,
    pub source: String,
    pub tokens: Vec<TokenKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceShareExpression {
    Full,
    Integer(String),
    Name(String),
    Add(
        Box<Located<SourceShareExpression>>,
        Box<Located<SourceShareExpression>>,
    ),
    Parenthesized(Box<Located<SourceShareExpression>>),
}

/// One bound of a range write lend's `LOW..HIGH` (card `WRITE-LEND-TASKS`).
/// Retained structurally: an integer literal is the only bound the lowering
/// admits, and a name is kept so the boundary can refuse it as a varying
/// bound by its own spelling rather than as a parse failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceLeaseBound {
    Integer(String),
    Name(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceCaptureMode {
    Copy,
    Move,
    Read {
        share: Located<SourceShareExpression>,
    },
    Write,
    /// `lend PLACE LOW..HIGH` (card `WRITE-LEND-TASKS`): lend write authority
    /// over the payload bytes `[LOW, HIGH)` to the worker for spawn..join.
    /// `lend` is contextual — claimed only as the first word of a capture,
    /// a position that was a parse failure before this card.
    LendWrite {
        low: Located<SourceLeaseBound>,
        high: Located<SourceLeaseBound>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTaskCapture {
    pub mode: SourceCaptureMode,
    pub place: Located<SourcePlace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceTaskExpressionKind {
    Spawn {
        scope: Located<String>,
        worker: Located<String>,
        captures: Vec<Located<SourceTaskCapture>>,
    },
    Join {
        handle: Located<SourcePlace>,
    },
}

/// A closure spawn target (card `W48-QR-CLOSURES`):
/// `spawn SCOPE |CAPTURES| -> TYPE [effects { … }] { BODY }`.
///
/// The parser lifts it into a **hidden named worker**: a function declaration
/// named [`Self::hidden`], placed immediately before the declaration that
/// spells the spawn, with one parameter per capture ([`Self::binders`]), the
/// written result and effect row, and the closure's body as its body. The spawn
/// itself is an ordinary spawn of that worker with the written captures, so
/// every later phase meets a named worker. The closure's tokens stay where the
/// reader wrote them; a window that contains a closure without lying inside it
/// skips [`Self::lifted`], so the enclosing statement reads as its hand-written
/// twin `spawn SCOPE worker(CAPTURES)` does. Nothing here decides a type, a
/// capture's admission or a name's resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceClosure {
    /// `ENCLOSING#closureN`: the enclosing function's name and the closure's
    /// ordinal in it, in source order. `#` is not an identifier character, so
    /// no source declaration can have this name.
    pub hidden: String,
    /// The function whose body spells the spawn (a hidden name for a closure
    /// written inside another closure).
    pub enclosing: String,
    /// The `spawn` keyword.
    pub spawn_keyword: Span,
    /// The opening bar through the body's closing brace: the hidden worker's
    /// own extent, and the span its refusals name.
    pub span: Span,
    /// The capture list, both bars included (`||` for the empty list).
    pub captures_span: Span,
    /// `->` through the body's closing brace: the hidden worker's header and
    /// body, read by the hidden worker alone.
    pub lifted: Span,
    pub captures: Vec<Located<SourceTaskCapture>>,
    /// One per capture, in order: the name the capture is bound to inside the
    /// body — the place's root name, a field capture's last segment, or an
    /// explicit `as NAME`.
    pub binders: Vec<Located<String>>,
}

impl SourceClosure {
    /// Whether `token` is one of this closure's two capture-list bars (the one
    /// `||` token of an empty list).
    pub fn is_bar(&self, token: Span) -> bool {
        token.start == self.captures_span.start || token.end == self.captures_span.end
    }
}

/// Token positions of one closure, for the parser's own passes.
#[derive(Clone, Debug)]
pub(super) struct ClosureSyntax {
    pub(super) open: usize,
    pub(super) close: usize,
    pub(super) arrow: usize,
    pub(super) result_end: usize,
    pub(super) body_open: usize,
    pub(super) body_close: usize,
    pub(super) captures: Vec<Located<SourceTaskCapture>>,
    pub(super) binders: Vec<Located<String>>,
}

/// A lifted closure: its public record and the positions that produced it.
#[derive(Clone, Debug)]
pub(super) struct LiftedClosure {
    pub(super) closure: SourceClosure,
    pub(super) syntax: ClosureSyntax,
}

/// Whether the tokens at `start` open a closure spawn: `spawn NAME` followed by
/// `|`, `||`, or the bracket-adjacent `[|` the closure parser refuses by name.
pub(super) fn is_closure_head(parser: &SourceParser<'_>, start: usize) -> bool {
    parser.word(start, "spawn")
        && !(start > 0
            && matches!(
                parser.tokens[start - 1].kind,
                TokenKind::Symbol(Symbol::Dot | Symbol::ColonColon)
            ))
        && parser.word_value(start + 1).is_some()
        && (parser.symbol(start + 2, Symbol::Pipe)
            || parser.symbol(start + 2, Symbol::PipePipe)
            || parser.symbol(start + 2, Symbol::QuantOpen))
}

/// Parse the closure whose `spawn` keyword is at `start` (card
/// `W48-QR-CLOSURES`). The capture list uses exactly the capture spellings of a
/// named worker's parenthesized list, each optionally followed by `as NAME`.
pub(super) fn parse_closure(
    parser: &SourceParser<'_>,
    start: usize,
) -> Result<ClosureSyntax, SourceParseError> {
    const BRACKETS: &str = "a closure's capture list is written between single bars, \
         `spawn SCOPE |copy x, …| -> TYPE { … }`, and never touches `[` or `]`: `[|` and `|]` \
         are the quant delimiters";
    identifier(parser, start + 1)?;
    let open = start + 2;
    if parser.symbol(open, Symbol::QuantOpen) {
        return Err(parser.error(open, BRACKETS));
    }
    let (close, captures) = if parser.symbol(open, Symbol::PipePipe) {
        (open, Vec::new())
    } else {
        let mut cursor = open + 1;
        loop {
            match parser.tokens.get(cursor).map(|token| &token.kind) {
                None => return Err(parser.error(open, "a closure's capture list ends at `|`")),
                Some(TokenKind::Symbol(Symbol::Pipe)) => break,
                Some(TokenKind::Symbol(Symbol::QuantOpen | Symbol::QuantClose)) => {
                    return Err(parser.error(cursor, BRACKETS));
                }
                Some(TokenKind::Symbol(
                    Symbol::LeftBrace
                    | Symbol::RightBrace
                    | Symbol::Semicolon
                    | Symbol::PipePipe
                    | Symbol::PipeInto
                    | Symbol::Arrow,
                )) => {
                    return Err(parser.error(cursor, "a closure's capture list ends at `|`"));
                }
                _ => cursor = parser.skip_group(cursor),
            }
        }
        if cursor == open + 1 {
            return Err(parser.error(
                cursor,
                "an empty capture list is spelled `||`, one token, as a named worker's is `()`",
            ));
        }
        (cursor, parse_captures(parser, open + 1, cursor, true)?)
    };
    let arrow = close + 1;
    if !parser.symbol(arrow, Symbol::Arrow) {
        return Err(parser.error(
            arrow,
            "a closure declares its result type after its capture list: `|…| -> TYPE { … }`",
        ));
    }
    let mut cursor = arrow + 1;
    let mut angle_depth = 0_u32;
    while cursor < parser.tokens.len() {
        if angle_depth == 0
            && (parser.word(cursor, "effects") || parser.symbol(cursor, Symbol::LeftBrace))
        {
            break;
        }
        if parser.symbol(cursor, Symbol::Less) {
            angle_depth += 1;
        } else if parser.symbol(cursor, Symbol::Greater) {
            angle_depth = angle_depth.saturating_sub(1);
        }
        cursor = parser.skip_group(cursor);
    }
    if cursor == arrow + 1 {
        return Err(parser.error(cursor, "a closure requires a nonempty result type"));
    }
    let result_end = cursor;
    let body_open = if parser.word(cursor, "effects") {
        if !parser.symbol(cursor + 1, Symbol::LeftBrace) {
            return Err(parser.error(cursor + 1, "a closure's effect row is `effects { … }`"));
        }
        parser.close(cursor + 1)? + 1
    } else {
        cursor
    };
    if !parser.symbol(body_open, Symbol::LeftBrace) {
        return Err(parser.error(body_open, "a closure's body is a block `{ … }`"));
    }
    let body_close = parser.close(body_open)?;
    let (captures, binders) = captures.into_iter().unzip();
    Ok(ClosureSyntax {
        open,
        close,
        arrow,
        result_end,
        body_open,
        body_close,
        captures,
        binders,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceTaskExpression {
    /// Expression including an immediately following `?`, when present.
    pub span: Span,
    pub statement_span: Span,
    pub kind: SourceTaskExpressionKind,
    /// Retained syntax only: no failure edge is discharged by this flag.
    pub propagates_error: bool,
}

/// A resolved root is not evidence of a stable address, a disjoint footprint,
/// valid indexing, live custody, or authority to access the place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTaskPlace {
    pub syntax: Located<SourcePlace>,
    pub root_symbol: SymbolId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTaskCapture {
    pub span: Span,
    pub mode: SourceCaptureMode,
    pub place: ResolvedTaskPlace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedTaskExpressionKind {
    Spawn {
        scope: SymbolId,
        worker: SymbolId,
        captures: Vec<ResolvedTaskCapture>,
    },
    /// Name resolution does not establish that this place contains a task,
    /// its creator, its outcome, or its outstanding join obligation.
    Join { handle: ResolvedTaskPlace },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTaskExpression {
    pub syntax: SourceTaskExpression,
    pub kind: ResolvedTaskExpressionKind,
}

pub(super) fn parse_inventory(
    parser: &SourceParser<'_>,
    declarations: &[Located<SourceDeclaration>],
) -> Result<Vec<SourceTaskExpression>, SourceParseError> {
    fn block(
        parser: &SourceParser<'_>,
        body: &SourceBlock,
        result: &mut Vec<SourceTaskExpression>,
    ) -> Result<(), SourceParseError> {
        for statement in &body.statements {
            let children = statement.value.nested_blocks();
            for (i, token) in parser.tokens.iter().enumerate() {
                if token.span.start < statement.span.start
                    || token.span.end > statement.span.end
                    || children.iter().any(|child| {
                        token.span.start >= child.span.start && token.span.end <= child.span.end
                    })
                    // Card `W48-QR-CLOSURES`: a closure's body belongs to its
                    // hidden worker, whose own statements are scanned there.
                    || parser.outside_closure_window(token.span, statement.span)
                {
                    continue;
                }
                if i > 0
                    && matches!(
                        parser.tokens[i - 1].kind,
                        TokenKind::Symbol(Symbol::Dot | Symbol::ColonColon)
                    )
                {
                    continue;
                }
                if parser.word(i, "spawn")
                    || (parser.word(i, "join") && !parser.symbol(i + 1, Symbol::LeftParen))
                {
                    result.push(parse_task(parser, i, statement.span)?);
                }
            }
            for child in children {
                block(parser, child, result)?;
            }
        }
        Ok(())
    }
    let mut result = Vec::new();
    for declaration in declarations {
        if let Some(body) = &declaration.value.body {
            block(parser, body, &mut result)?;
        }
    }
    result.sort_by_key(|task| task.span.start);
    Ok(result)
}

/// Find and lift every closure in `body`, which `enclosing` declares (card
/// `W48-QR-CLOSURES`). Each closure yields its hidden worker declaration into
/// `lifted` — a closure written inside another closure first, so each hidden
/// worker precedes the function that spawns it, as a hand-written twin's
/// declarations would — and its record into `closures`, keyed by the index of
/// its `spawn` keyword.
pub(super) fn find_closures(
    parser: &SourceParser<'_>,
    enclosing: &str,
    body: &SourceBlock,
    lifted: &mut Vec<Located<SourceDeclaration>>,
    closures: &mut BTreeMap<usize, LiftedClosure>,
) -> Result<(), SourceParseError> {
    fn walk(
        parser: &SourceParser<'_>,
        enclosing: &str,
        block: &SourceBlock,
        ordinal: &mut usize,
        lifted: &mut Vec<Located<SourceDeclaration>>,
        closures: &mut BTreeMap<usize, LiftedClosure>,
    ) -> Result<(), SourceParseError> {
        for statement in &block.statements {
            let children = statement.value.nested_blocks();
            let mut taken: Vec<Span> = Vec::new();
            for index in 0..parser.tokens.len() {
                let span = parser.tokens[index].span;
                if span.start < statement.span.start
                    || span.end > statement.span.end
                    || children
                        .iter()
                        .any(|child| span.start >= child.span.start && span.end <= child.span.end)
                    || taken
                        .iter()
                        .any(|closure| span.start >= closure.start && span.end <= closure.end)
                    || !is_closure_head(parser, index)
                {
                    continue;
                }
                let syntax = parse_closure(parser, index)?;
                *ordinal += 1;
                let hidden = format!("{enclosing}#closure{ordinal}");
                let body = parser.block(syntax.body_open)?;
                find_closures(parser, &hidden, &body, lifted, closures)?;
                let extent = parser.range_span(syntax.open, syntax.body_close + 1);
                let result_span = parser.range_span(syntax.arrow + 1, syntax.result_end);
                let parameters = syntax
                    .captures
                    .iter()
                    .zip(&syntax.binders)
                    .map(|(capture, binder)| SourceParameter {
                        name: binder.clone(),
                        // The type is not written: the task route decides it
                        // from the captured place at the spawn site.
                        type_kind: SourceTypeKind::Named,
                        type_span: capture.span,
                        type_source: String::new(),
                        // A `move` hands the worker custody of what it moves,
                        // which a hand-written twin spells `use one`.
                        usage: matches!(capture.value.mode, SourceCaptureMode::Move).then(|| {
                            Located {
                                span: capture.span,
                                value: Usage::ONE,
                            }
                        }),
                    })
                    .collect();
                lifted.push(Located {
                    span: extent,
                    value: SourceDeclaration {
                        kind: DeclarationKind::Function,
                        name: hidden.clone(),
                        parameters,
                        result_type: Some(SourceResultType {
                            kind: parser.type_kind(syntax.arrow + 1),
                            span: result_span,
                            source: parser.source[result_span.start..result_span.end].to_owned(),
                            tokens: parser.tokens[syntax.arrow + 1..syntax.result_end]
                                .iter()
                                .map(|token| token.kind.clone())
                                .collect(),
                        }),
                        quant_index_types: Vec::new(),
                        body: Some(body),
                        function_contract: None,
                        verifier_scope: None,
                        tokens: parser.tokens[syntax.open..=syntax.body_close]
                            .iter()
                            .map(|token| token.kind.clone())
                            .collect(),
                        c_ffi: None,
                    },
                });
                taken.push(extent);
                closures.insert(
                    index,
                    LiftedClosure {
                        closure: SourceClosure {
                            hidden,
                            enclosing: enclosing.to_owned(),
                            spawn_keyword: span,
                            span: extent,
                            captures_span: parser.range_span(syntax.open, syntax.close + 1),
                            lifted: parser.range_span(syntax.arrow, syntax.body_close + 1),
                            captures: syntax.captures.clone(),
                            binders: syntax.binders.clone(),
                        },
                        syntax,
                    },
                );
            }
            for child in children {
                walk(parser, enclosing, child, ordinal, lifted, closures)?;
            }
        }
        Ok(())
    }
    let mut ordinal = 0;
    walk(parser, enclosing, body, &mut ordinal, lifted, closures)
}

fn identifier(
    parser: &SourceParser<'_>,
    cursor: usize,
) -> Result<Located<String>, SourceParseError> {
    let name = parser.identifier(cursor)?;
    if is_reserved_source_word(name) {
        return Err(parser.error(cursor, "expected a place or task identifier"));
    }
    Ok(Located {
        span: parser.tokens[cursor].span,
        value: name.to_owned(),
    })
}

fn parse_task(
    parser: &SourceParser<'_>,
    start: usize,
    statement_span: Span,
) -> Result<SourceTaskExpression, SourceParseError> {
    let (kind, mut next) = if parser.word(start, "spawn")
        && let Some(lifted) = parser.closures.get(&start)
    {
        // Card `W48-QR-CLOSURES`: the closure's hidden worker, spawned with the
        // written captures. The expression runs through the body's closing
        // brace, so a following `?` is the spawn's, exactly as after `)`.
        (
            SourceTaskExpressionKind::Spawn {
                scope: identifier(parser, start + 1)?,
                worker: Located {
                    span: lifted.closure.captures_span,
                    value: lifted.closure.hidden.clone(),
                },
                captures: lifted.closure.captures.clone(),
            },
            lifted.syntax.body_close + 1,
        )
    } else if parser.word(start, "spawn") {
        let scope = identifier(parser, start + 1)?;
        let worker = identifier(parser, start + 2)?;
        if !parser.symbol(start + 3, Symbol::LeftParen) {
            return Err(parser.error(
                start + 3,
                "spawn requires a parenthesized ordered capture list",
            ));
        }
        let close = parser.close(start + 3)?;
        let captures = parse_captures(parser, start + 4, close, false)?
            .into_iter()
            .map(|(capture, _)| capture)
            .collect();
        (
            SourceTaskExpressionKind::Spawn {
                scope,
                worker,
                captures,
            },
            close + 1,
        )
    } else {
        if !parser.word(start + 1, "move") {
            return Err(parser.error(start + 1, "join requires `move` and a handle place"));
        }
        let limit = parser
            .tokens
            .iter()
            .position(|t| t.span.start >= statement_span.end)
            .unwrap_or(parser.tokens.len());
        let (handle, next) = parse_place(parser, start + 2, limit)?;
        (SourceTaskExpressionKind::Join { handle }, next)
    };
    let propagates_error = parser.symbol(next, Symbol::Question);
    if propagates_error {
        next += 1;
    }
    let span = parser.range_span(start, next);
    if span.end > statement_span.end {
        return Err(parser.error(start, "task expression crosses its statement boundary"));
    }
    Ok(SourceTaskExpression {
        span,
        statement_span,
        kind,
        propagates_error,
    })
}

/// One parsed capture and the name it is bound to inside a closure's body.
type BoundCapture = (Located<SourceTaskCapture>, Located<String>);

/// The captures between `start` and `close` (exclusive): a named worker's
/// parenthesized list, or — card `W48-QR-CLOSURES`, `closure` — a closure's
/// capture list, where each capture may end in `as NAME`, the name it is bound
/// to inside the body. Each capture carries that binder, the place's root name
/// or a field capture's last segment when none is written.
fn parse_captures(
    parser: &SourceParser<'_>,
    start: usize,
    close: usize,
    closure: bool,
) -> Result<Vec<BoundCapture>, SourceParseError> {
    let mut cursor = start;
    let mut captures = Vec::new();
    while cursor < close {
        let begin = cursor;
        let mode = parser.word_value(cursor).unwrap_or("");
        // Card `W48-QR-CLOSURES`: a closure passed to a named worker is a
        // closure as a value, which is not admitted.
        if !closure
            && (parser.symbol(cursor, Symbol::Pipe) || parser.symbol(cursor, Symbol::PipePipe))
        {
            return Err(parser.error(
                cursor,
                "a closure is a spawn target only, `spawn SCOPE |CAPTURES| -> TYPE { … }`; \
                 passing one to a worker is passing it as a value, which is not admitted \
                 (W48-QR-CLOSURE-VALUES)",
            ));
        }
        if !matches!(mode, "copy" | "move" | "read" | "write" | "lend") {
            return Err(parser.error(
                cursor,
                "capture requires copy, move, read ... share, lend ... LOW..HIGH, or write",
            ));
        }
        let (place, end) = parse_place(parser, cursor + 1, close)?;
        cursor = end;
        let mode = match mode {
            "copy" => SourceCaptureMode::Copy,
            "move" => SourceCaptureMode::Move,
            "write" => SourceCaptureMode::Write,
            "read" => {
                if !parser.word(cursor, "share") {
                    return Err(
                        parser.error(cursor, "read capture requires an explicit share expression")
                    );
                }
                let (share, end) = parse_share(parser, cursor + 1, close)?;
                cursor = end;
                SourceCaptureMode::Read { share }
            }
            "lend" => {
                // The range spelling is the loop's: `LOW..HIGH`, the `..`
                // being the two existing `Dot` tokens (card `LOOP-V1`'s
                // precedent; no new token).
                let (low, next) = lease_bound(parser, cursor, close)?;
                if !parser.symbol(next, Symbol::Dot) || !parser.symbol(next + 1, Symbol::Dot) {
                    return Err(parser.error(next, "a write lend spells its range `LOW..HIGH`"));
                }
                let (high, next) = lease_bound(parser, next + 2, close)?;
                cursor = next;
                SourceCaptureMode::LendWrite { low, high }
            }
            _ => unreachable!(),
        };
        let span = parser.range_span(begin, cursor);
        let binder = if closure && parser.word(cursor, "as") {
            let binder = identifier(parser, cursor + 1)?;
            cursor += 2;
            binder
        } else {
            match &place.value {
                SourcePlace::Field { field, .. } => field.clone(),
                other => other.root().clone(),
            }
        };
        captures.push((
            Located {
                span,
                value: SourceTaskCapture { mode, place },
            },
            binder,
        ));
        if cursor == close {
            break;
        }
        if !parser.symbol(cursor, Symbol::Comma) {
            return Err(parser.error(
                cursor,
                if closure {
                    "capture must be a place, optionally `as NAME`, followed by comma or the \
                     closing `|`"
                } else {
                    "capture must be a place, followed by comma or closing parenthesis"
                },
            ));
        }
        cursor += 1;
        if cursor == close {
            return Err(parser.error(cursor, "missing capture after comma"));
        }
    }
    Ok(captures)
}

fn parse_place(
    parser: &SourceParser<'_>,
    start: usize,
    limit: usize,
) -> Result<(Located<SourcePlace>, usize), SourceParseError> {
    if start >= limit {
        return Err(parser.error(start, "expected a capture or handle place"));
    }
    if parser.symbol(start, Symbol::Star) {
        let (base, next) = parse_place(parser, start + 1, limit)?;
        return Ok((
            Located {
                span: parser.range_span(start, next),
                value: SourcePlace::Dereference(Box::new(base)),
            },
            next,
        ));
    }
    let name = identifier(parser, start)?;
    let mut place = Located {
        span: name.span,
        value: SourcePlace::Name(name),
    };
    let mut next = start + 1;
    while next < limit {
        if parser.symbol(next, Symbol::Dot) {
            let field = identifier(parser, next + 1)?;
            next += 2;
            place = Located {
                span: parser.range_span(start, next),
                value: SourcePlace::Field {
                    base: Box::new(place),
                    field,
                },
            };
        } else if parser.symbol(next, Symbol::LeftBracket) {
            let close = parser.close(next)?;
            if close >= limit || close == next + 1 {
                return Err(parser.error(next, "place index requires a nonempty expression"));
            }
            let span = parser.range_span(next + 1, close);
            let index = SourceIndexExpression {
                span,
                source: parser.source[span.start..span.end].to_owned(),
                tokens: parser.tokens[next + 1..close]
                    .iter()
                    .map(|t| t.kind.clone())
                    .collect(),
            };
            next = close + 1;
            place = Located {
                span: parser.range_span(start, next),
                value: SourcePlace::Index {
                    base: Box::new(place),
                    index,
                },
            };
        } else {
            break;
        }
    }
    Ok((place, next))
}

/// One bound of a `lend PLACE LOW..HIGH` capture: an integer literal, or a
/// name retained for the boundary's varying-bound refusal.
fn lease_bound(
    parser: &SourceParser<'_>,
    start: usize,
    limit: usize,
) -> Result<(Located<SourceLeaseBound>, usize), SourceParseError> {
    if start >= limit {
        return Err(parser.error(start, "expected a write lend bound"));
    }
    let value = match &parser.tokens[start].kind {
        TokenKind::Integer(value) => SourceLeaseBound::Integer(value.clone()),
        TokenKind::Word(_) => SourceLeaseBound::Name(identifier(parser, start)?.value),
        _ => {
            return Err(parser.error(start, "a write lend bound is an integer literal"));
        }
    };
    Ok((
        Located {
            span: parser.tokens[start].span,
            value,
        },
        start + 1,
    ))
}

fn parse_share(
    parser: &SourceParser<'_>,
    start: usize,
    limit: usize,
) -> Result<(Located<SourceShareExpression>, usize), SourceParseError> {
    let (mut left, mut next) = share_atom(parser, start, limit)?;
    while next < limit && parser.symbol(next, Symbol::Plus) {
        let (right, end) = share_atom(parser, next + 1, limit)?;
        left = Located {
            span: parser.range_span(start, end),
            value: SourceShareExpression::Add(Box::new(left), Box::new(right)),
        };
        next = end;
    }
    Ok((left, next))
}

fn share_atom(
    parser: &SourceParser<'_>,
    start: usize,
    limit: usize,
) -> Result<(Located<SourceShareExpression>, usize), SourceParseError> {
    if start >= limit {
        return Err(parser.error(start, "expected a share expression"));
    }
    let value = match &parser.tokens[start].kind {
        TokenKind::Word(word) if word == "FULL" => SourceShareExpression::Full,
        TokenKind::Word(_) => SourceShareExpression::Name(identifier(parser, start)?.value),
        TokenKind::Integer(value) => SourceShareExpression::Integer(value.clone()),
        TokenKind::Symbol(Symbol::LeftParen) => {
            let close = parser.close(start)?;
            if close >= limit {
                return Err(parser.error(start, "share expression crosses its capture boundary"));
            }
            let (inner, next) = parse_share(parser, start + 1, close)?;
            if next != close {
                return Err(parser.error(next, "unsupported share expression"));
            }
            return Ok((
                Located {
                    span: parser.range_span(start, close + 1),
                    value: SourceShareExpression::Parenthesized(Box::new(inner)),
                },
                close + 1,
            ));
        }
        _ => {
            return Err(parser.error(
                start,
                "expected FULL, an integer, a name, or a parenthesized share sum",
            ));
        }
    };
    Ok((
        Located {
            span: parser.tokens[start].span,
            value,
        },
        start + 1,
    ))
}

pub(super) struct ResolutionContext<'a> {
    pub(super) program: &'a SourceProgram,
    pub(super) live_scopes: BTreeSet<SymbolId>,
    /// Card `W48-QR-CLOSURES`: while a closure's hidden worker is resolved,
    /// the names its enclosing scopes hold at the spawn site. None of them is
    /// in the worker's scope; they only let a refusal say why a name that the
    /// reader can see around the closure does not resolve inside it.
    pub(super) closure_outer: Option<BTreeSet<String>>,
}

/// A capture's place as the reader wrote it, for naming a duplicate.
pub(super) fn place_key(place: &SourcePlace) -> String {
    match place {
        SourcePlace::Name(name) => name.value.clone(),
        SourcePlace::Dereference(base) => format!("*{}", place_key(&base.value)),
        SourcePlace::Field { base, field } => format!("{}.{}", place_key(&base.value), field.value),
        SourcePlace::Index { base, index } => {
            format!("{}[{}]", place_key(&base.value), index.source)
        }
    }
}

pub(super) fn resolve_tasks(
    statement: Span,
    scopes: &[BTreeMap<String, SymbolId>],
    globals: &BTreeMap<String, SymbolId>,
    context: &ResolutionContext<'_>,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Vec<ResolvedTaskExpression> {
    let lookup = |name: &str| {
        scopes
            .iter()
            .rev()
            .find_map(|s| s.get(name))
            .or_else(|| globals.get(name))
            .copied()
    };
    let mut output = Vec::new();
    for syntax in context
        .program
        .task_expressions()
        .iter()
        .filter(|task| task.statement_span == statement)
    {
        let mut error = |span: Span, message: String| {
            diagnostics.push(SourceDiagnostic {
                code: "Q013",
                pass: SemanticPass::NameResolution,
                span,
                message,
            })
        };
        let mut resolve_place = |place: &Located<SourcePlace>| -> Option<ResolvedTaskPlace> {
            let root = place.value.root();
            let Some(root_symbol) = lookup(&root.value) else {
                error(
                    root.span,
                    format!("unknown task place root `{}`", root.value),
                );
                return None;
            };
            Some(ResolvedTaskPlace {
                syntax: place.clone(),
                root_symbol,
            })
        };
        let kind = match &syntax.kind {
            SourceTaskExpressionKind::Spawn {
                scope,
                worker,
                captures,
            } => {
                let Some(scope_symbol) =
                    lookup(&scope.value).filter(|s| context.live_scopes.contains(s))
                else {
                    error(
                        scope.span,
                        format!(
                            "spawn scope `{}` is not a live enclosing thread scope",
                            scope.value
                        ),
                    );
                    continue;
                };
                let declaration = context.program.declarations.iter().find(|d| {
                    d.value.name == worker.value && d.value.kind == DeclarationKind::Function
                });
                let Some((worker_symbol, declaration)) = lookup(&worker.value)
                    .zip(declaration)
                    .filter(|(symbol, _)| globals.get(&worker.value) == Some(symbol))
                else {
                    error(
                        worker.span,
                        format!(
                            "spawn worker `{}` must resolve to a first-order source function",
                            worker.value
                        ),
                    );
                    continue;
                };
                if declaration.value.parameters.len() != captures.len() {
                    error(
                        worker.span,
                        format!(
                            "spawn worker `{}` expects {} captures, received {}",
                            worker.value,
                            declaration.value.parameters.len(),
                            captures.len()
                        ),
                    );
                    continue;
                }
                let resolved = captures
                    .iter()
                    .map(|capture| {
                        Some(ResolvedTaskCapture {
                            span: capture.span,
                            mode: capture.value.mode.clone(),
                            place: resolve_place(&capture.value.place)?,
                        })
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(captures) = resolved else {
                    continue;
                };
                ResolvedTaskExpressionKind::Spawn {
                    scope: scope_symbol,
                    worker: worker_symbol,
                    captures,
                }
            }
            SourceTaskExpressionKind::Join { handle } => {
                let Some(handle) = resolve_place(handle) else {
                    continue;
                };
                ResolvedTaskExpressionKind::Join { handle }
            }
        };
        output.push(ResolvedTaskExpression {
            syntax: syntax.clone(),
            kind,
        });
    }
    output
}
