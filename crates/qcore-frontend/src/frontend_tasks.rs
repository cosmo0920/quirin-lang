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
    let (kind, mut next) = if parser.word(start, "spawn") {
        let scope = identifier(parser, start + 1)?;
        let worker = identifier(parser, start + 2)?;
        if !parser.symbol(start + 3, Symbol::LeftParen) {
            return Err(parser.error(
                start + 3,
                "spawn requires a parenthesized ordered capture list",
            ));
        }
        let close = parser.close(start + 3)?;
        let mut cursor = start + 4;
        let mut captures = Vec::new();
        while cursor < close {
            let begin = cursor;
            let mode = parser.word_value(cursor).unwrap_or("");
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
                        return Err(parser
                            .error(cursor, "read capture requires an explicit share expression"));
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
            captures.push(Located {
                span: parser.range_span(begin, cursor),
                value: SourceTaskCapture { mode, place },
            });
            if cursor == close {
                break;
            }
            if !parser.symbol(cursor, Symbol::Comma) {
                return Err(parser.error(
                    cursor,
                    "capture must be a place, followed by comma or closing parenthesis",
                ));
            }
            cursor += 1;
            if cursor == close {
                return Err(parser.error(cursor, "missing capture after comma"));
            }
        }
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
