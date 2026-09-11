//! The pipe operator `|>` (card `PIPE-OPERATOR`) and the per-file operator
//! gate (retabled by card `OR-AND-LOWERING`).
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
//! statement ::= [ "let" NAME "=" ] OPERAND ("|>" STAGE)+ ";"
//! OPERAND   ::= token+          -- passed through verbatim, e.g. `x`, `-7`, `move b`
//! STAGE     ::= [ "move" ] STAGE_BODY
//! STAGE_BODY::= "print" WORD                 -- a `source.std-io.v1` sink
//!             | "log" WORD WORD              -- a `source.std-io.v1` sink
//!             | NAME [ "(" [arg {"," arg}] ")" ] [ ADAPTER ]   -- a G02d call
//! ADAPTER   ::= "succeeds" BOOL "else" {disposition} "raise" NAME
//! ```
//!
//! | Written | Desugars to |
//! |---|---|
//! | `total \|> print dec;` | `print dec total;` |
//! | `total \|> log fatal dec;` | `log fatal dec total;` |
//! | `x \|> f;` | `f(x);` |
//! | `x \|> f(a, b);` | `f(x, a, b);` |
//! | `move b \|> release;` | `release(move b);` |
//! | `8 \|> alloc succeeds true else raise E \|> move free;` | `let ⟨t⟩ = alloc(8) succeeds true else raise E; free(move ⟨t⟩);` |
//! | `let r = x \|> f succeeds true else raise E;` | `let r = f(x) succeeds true else raise E;` |
//!
//! # Chains (card `PIPE-CHAIN`)
//!
//! A pipeline of `n` stages desugars to a **sequence of `n` statements**. Each
//! non-final stage binds its result to a hygienic temporary the next stage
//! receives; the final stage keeps the statement form the reader wrote around
//! the whole pipeline, which is either a bare statement or a `let NAME =`
//! binding. Nothing about custody is re-implemented: by the time any checker
//! runs, a temporary is an ordinary binding, so linearity, leak accounting and
//! use-after-move are inherited. A chain whose result is never consumed leaks
//! and refuses at the pre-existing `Q202`.
//!
//! Wave 18 recorded two blockers for this. The first — a call stage's result
//! has no name — is answered by *giving* it one. The second — that a statement
//! sequence would need statement windows this frontend keys by byte span —
//! turned out to rest on the assumption that the statements would **share** the
//! operand token. With a fresh temporary per stage they share nothing, and
//! their byte spans are disjoint by construction: statement `k` runs from its
//! own written tokens to the *start* of the `|>` that ends it, and statement
//! `k+1`'s synthesized tokens sit at that same `|>`'s *end*, two bytes later.
//!
//! "Produces a value" is decided syntactically, with no contract knowledge, by
//! **stage kind**: a call stage produces a value and a sink (`print`, `log`) or
//! an operandless statement (`flush`) does not. Card `PIPE-CHAIN` decided it by
//! the presence of `succeeds`, because the acquisition adapter was then the only
//! `let`-with-call the language admitted; card `PIPE-SCALAR-RESULT` adds the
//! other one, so the keyword no longer separates them. Whether a particular
//! callee's declared result is bindable is the *route's* decision and is
//! reported by the route's own diagnostic at that stage's own span — a
//! `unit`-returning callee in a non-final position still refuses, at `Q024`
//! naming its result rather than at `Q037` naming the pipe.
//!
//! # Model intrinsics as stages (card `PIPE-INTRINSIC-STAGE`)
//!
//! Card `COMPOSE-CONCURRENCY` filed a follow-on saying the brief's literal
//! `buffer |> read |> render |> log` needed "a pipeline stage for a task
//! intrinsic, which is a pipe-grammar change". The census measured for that card
//! says it does not, and the reason is worth stating here so a later card does
//! not add a table entry for it:
//!
//! * A **call-shaped** model intrinsic is spelled `NAME(ARGS)` and its statement
//!   form is `let NAME = INTRINSIC(ARGS);` — `qcore_buffer_new`,
//!   `qcore_buffer_read`, `qcore_buffer_write` and `qcore_error_payload` on the
//!   task routes, `qcore_acquire` and `qcore_check` on the sequential one. None
//!   is in [`SINK_STAGES`] or [`OPERANDLESS_STAGES`], so each is a **call
//!   stage** by the classification below and each therefore already desugars
//!   onto the statement form its route already lowers, with the hygienic
//!   temporary. `buffer |> qcore_buffer_read |> log info dec;` is
//!   `let ⟨t0⟩ = qcore_buffer_read(buffer); log info dec ⟨t0⟩;`.
//! * A **keyword-shaped** intrinsic — `spawn S W(ARGS)?`, `join move H?`,
//!   `share N`, `drop N`, and the mutex and epoch clause words — is a statement
//!   whose operands are not a parenthesized argument list. A call stage only
//!   ever emits `HEAD ( … )`, so none of them is reachable, and the closed sink
//!   table deliberately does not name them. Adding `drop` in particular would
//!   make a *custody-releasing* statement pipeable, which is a decision of its
//!   own: follow-on `PIPE-DROP-STAGE`.
//!
//! Whether the route can bind a particular intrinsic's result stays the route's
//! question, reported at that stage's span — `qcore_buffer_write` returns `unit`
//! and refuses mid-chain at `Q050`, not at `Q037` — which is exactly the rule
//! card `PIPE-SCALAR-RESULT` set for a `unit`-returning extern callee.
//!
//! Two spellings exist for the value a previous stage produced, because a
//! hygienic temporary has no name a reader could write:
//!
//! * `|> move STAGE` consumes it, mirroring `move OPERAND |> STAGE` for the
//!   first stage, where the reader's own operand already has a spelling (so
//!   `move` is refused there — one spelling per meaning); and
//! * a **bare `move`** inside a stage's adapter tail names it, so an
//!   acquisition's error arm can dispose of the value piped into it. That arm
//!   is mandatory whenever another identity is live (`Q025`), so without this a
//!   chain could hold at most one acquisition.
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
//! refusal inside a desugared statement therefore points at the byte range the
//! reader wrote — and in a chain, at *that stage's* bytes rather than at the
//! pipeline's first token. Only `let`, `=`, `;`, `(`, `)`, `,` and the
//! temporaries themselves are synthesized, each anchored zero-width at the `|>`
//! it came from, so even a fault that lands on synthesized punctuation names
//! the pipe rather than an invented position.
//!
//! One case cannot keep the reader's bytes and is recorded rather than hidden:
//! in a **multi-stage** `let NAME = …` pipeline the binder is written to the
//! left of every stage, so keeping its spans would make the last statement's
//! byte span swallow the statements before it. The binder is re-anchored at the
//! final `|>`, with the reader's own name preserved. A one-stage bound pipeline
//! keeps its binder tokens exactly where they were.
//!
//! # What this profile does not do, and why
//!
//! * **A sink cannot feed a later stage.** `print`, `log` and `flush` are
//!   last-stage-only and refuse at `Q037` naming the stage.
//! * **An assignment pipeline** — `x = y |> f;` — refuses at `Q036`; follow-on
//!   `PIPE-ASSIGN`.
//! * **A chain still cannot cross a route.** Card `PIPE-SCALAR-RESULT` closes
//!   the first half of `value |> render |> log`: a call whose declared result is
//!   a custody-free scalar now binds it, so a `render` stage exists and
//!   `x |> render |> scale` is expressible. The second half is unchanged — a
//!   module that both calls an extern function and prints stays on the extern
//!   route, which refuses `print` — and remains the already-recorded
//!   `STD-IO-COMPOSE`.
//!
//! # The operator gate
//!
//! This module also carries the per-file **operator gate**
//! ([`check_source_operators`]), which decides where each operator token is
//! allowed to appear at all. Card `OR-AND-LOWERING` retabled it:
//!
//! | Spelling | Token | Status |
//! |---|---|---|
//! | `[\|` / `\|]` | `QuantOpen` / `QuantClose` | unchanged — the quant-index alias |
//! | `\|>` | `PipeInto` | the pipe operator, desugared away before parsing |
//! | `\|\|` | `PipePipe` | **logical or**, admitted in an `if` condition and lowered there |
//! | `&&` | `AndAnd` | **logical and**, same, binding tighter than `\|\|` |
//! | `\|` / `&` / `^` | `Pipe` / `Ampersand` / `Caret` | reserved for bitwise or/and/xor; `Q038` everywhere |
//!
//! This **supersedes** card `PIPE-OPERATOR`'s table, in which the single `|`
//! was logical or and `||` was refused as a misspelling of it, by explicit user
//! direction. The single characters are freed so a future `BITWISE-OPERATORS`
//! wave has all three, which also resolves that card's open note that bitwise
//! or had no spelling left.
//!
//! The connectives are admitted **only** in an `if` condition, because that is
//! the only position where there is something to lower onto: `source.branch.v1`
//! (card `IF-STATEMENT`) turns them into nested branches, and there is still no
//! Boolean expression tier anywhere else to hold the value.
use crate::frontend::{SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, StatementKind};
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
///
/// Card `PIPE-INTRINSIC-STAGE` measured what that exclusion covers and left it
/// unchanged: every keyword-shaped model intrinsic (`spawn`, `join`, `share`,
/// `drop`, `grow`, `open`, `close`, and the epoch plan clauses) is outside it,
/// and a call-shaped one (`qcore_buffer_read` and its five siblings) does not
/// need to be inside it, being a call stage already. `drop` is the entry a
/// reader will reach for first and the one deliberately withheld: it takes a
/// single operand exactly as `print` does, but it *releases custody*, and a
/// releasing table entry is a different kind of entry from a rendering one.
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

/// The reserved prefix of a hygienic pipeline temporary (card `PIPE-CHAIN`).
///
/// A temporary is a synthesized [`TokenKind::Word`], so it never passes through
/// the lexer and could carry bytes no source can spell — but it *does* reach
/// name resolution and every diagnostic that names a binding, so it has to be a
/// legible identifier. Hygiene is therefore enforced by refusal rather than by
/// an unspellable name: if a file that needs temporaries also writes a word
/// beginning with this prefix, the whole pipeline is refused. Silent
/// α-renaming is deliberately not done — a renamed binding would make a later
/// diagnostic name a binding the reader never wrote.
const TEMPORARY_PREFIX: &str = "qcore__pipe";

/// The stage prefix that consumes the incoming temporary (card `PIPE-CHAIN`).
const MOVE_KEYWORD: &str = "move";

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
    let collision = tokens
        .iter()
        .find(|token| word(token).is_some_and(|word| word.starts_with(TEMPORARY_PREFIX)))
        .map(|token| token.span);

    let mut output: Vec<Token> = Vec::with_capacity(tokens.len() + 8 * runs.len());
    let mut cursor = 0usize;
    let mut counter = 0usize;
    for run in runs {
        output.extend_from_slice(&tokens[cursor..run.start]);
        match rewrite(
            &tokens[run.start..run.end],
            run.pipe,
            collision,
            &mut counter,
        ) {
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
                        "Q036",
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
            "Q036",
            tokens[at].span,
            "a pipeline is one statement ending in `;`; this one has no `;`",
        ));
    }
    runs
}

/// A synthesized word, anchored zero-width at `at`.
fn synthesized_word(name: String, at: usize) -> Token {
    Token {
        kind: TokenKind::Word(name),
        span: Span { start: at, end: at },
    }
}

/// A synthesized symbol, anchored zero-width at `at`.
fn synthesized_at(symbol: Symbol, at: usize) -> Token {
    Token {
        kind: TokenKind::Symbol(symbol),
        span: Span { start: at, end: at },
    }
}

/// One stage of a pipeline, as written, with its feeding `|>`.
struct Stage<'a> {
    /// The stage's own tokens, with any `move` prefix already stripped.
    tokens: &'a [Token],
    /// The `move` prefix, when the stage consumes its incoming value.
    consumes: Option<&'a Token>,
    /// The `|>` that feeds this stage.
    pipe: Span,
}

impl Stage<'_> {
    /// The byte offset at which this stage's synthesized tokens are anchored.
    ///
    /// The first stage anchors at the *start* of its `|>`, which is where card
    /// `PIPE-OPERATOR` put the synthesized delimiters of a one-stage pipeline
    /// and is inside the statement either way, since the operand it receives
    /// was written further left. Every later stage anchors at its `|>`'s
    /// **end**, which is what keeps consecutive desugared statements' byte
    /// spans strictly disjoint even where a `|>` abuts its neighbours: the
    /// previous statement's synthesized `;` sits at the same pipe's start, and
    /// `|>` is two bytes wide.
    fn anchor(&self, first: bool) -> usize {
        if first {
            self.pipe.start
        } else {
            self.pipe.end
        }
    }

    /// Whether this stage produces a value a later stage could receive.
    ///
    /// Decided syntactically, with no contract knowledge — which is what keeps
    /// the pipe a pure token pass — but by **stage kind** rather than by the
    /// acquisition keyword. A call stage produces a value; a sink (`print`,
    /// `log`) and an operandless statement (`flush`) do not, and that is a
    /// property of the closed statement table, not of any declaration.
    ///
    /// Card `PIPE-CHAIN` decided it by the presence of `succeeds`, because at
    /// that time the acquisition adapter was the only `let`-with-call the source
    /// language admitted. Card `PIPE-SCALAR-RESULT` adds the other one — a
    /// non-acquiring call's scalar result — so the keyword no longer separates
    /// the two. What separates a stage that can feed a later one from a stage
    /// that cannot is now exactly whether it is a call, and *whether the call's
    /// declared result is bindable* is the route's decision, reported by the
    /// route's own diagnostic at that stage's own span. A `unit`-returning
    /// callee therefore still refuses in a non-final position; it refuses at
    /// `Q024` naming its declared result rather than at `Q037` naming the pipe.
    fn produces_a_result(&self) -> bool {
        matches!(classify(self), Ok(StageKind::Call { .. }))
    }
}

/// Rewrite one run `[ "let" NAME "=" ] OPERAND { "|>" STAGE } ";"` into the
/// tokens of the statement sequence a reader would have written by hand.
fn rewrite(
    run: &[Token],
    first_pipe: Span,
    collision: Option<Span>,
    counter: &mut usize,
) -> Result<Vec<Token>, PipeRefusal> {
    let semicolon = run.last().expect("a run always ends in its `;`").clone();
    let mut body = &run[..run.len() - 1];

    // An optional leading binder. Card `PIPE-CHAIN` closes `PIPE-LET`: the
    // final stage keeps the statement form the reader wrote around the whole
    // pipeline, so `let r = x |> f succeeds …;` binds `r` to the last stage's
    // result exactly as the hand-written statement does.
    let mut binder: Option<[Token; 3]> = None;
    if word(&body[0]) == Some("let") {
        if body.len() < 3 || word(&body[1]).is_none() || !is(&body[2], Symbol::Equal) {
            return Err(refusal(
                "Q036",
                body[0].span,
                "a bound pipeline reads `let NAME = OPERAND |> STAGE …;`",
            ));
        }
        binder = Some([body[0].clone(), body[1].clone(), body[2].clone()]);
        body = &body[3..];
    }
    if let Some(equal) = body
        .iter()
        .position(|token| is(token, Symbol::Equal) || is(token, Symbol::Define))
    {
        return Err(refusal(
            "Q036",
            body[equal].span,
            "`source.pipe.v1` admits a pipeline as a whole statement or on the right of a leading \
             `let NAME =`; an assignment pipeline is follow-on `PIPE-ASSIGN`",
        ));
    }

    let mut segments: Vec<&[Token]> = Vec::new();
    let mut pipes: Vec<Span> = Vec::new();
    let mut segment_start = 0usize;
    let mut depth = 0i32;
    for (index, token) in body.iter().enumerate() {
        match &token.kind {
            TokenKind::Symbol(Symbol::LeftParen | Symbol::LeftBracket) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen | Symbol::RightBracket) => depth -= 1,
            TokenKind::Symbol(Symbol::PipeInto) if depth == 0 => {
                segments.push(&body[segment_start..index]);
                pipes.push(token.span);
                segment_start = index + 1;
            }
            _ => {}
        }
    }
    segments.push(&body[segment_start..]);

    let operand = segments[0];
    if operand.is_empty() {
        return Err(refusal(
            "Q036",
            first_pipe,
            "`|>` needs a value on its left; this pipeline pipes nothing",
        ));
    }

    let mut stages: Vec<Stage> = Vec::with_capacity(segments.len() - 1);
    for (index, segment) in segments[1..].iter().enumerate() {
        let pipe = pipes[index];
        if segment.is_empty() {
            return Err(refusal(
                "Q036",
                pipe,
                "`|>` needs a stage on its right; this pipeline pipes into nothing",
            ));
        }
        // `move` in front of a stage consumes the value the previous stage
        // produced. It is the only way to say it for a synthesized temporary,
        // and it is deliberately refused on the first stage, where the reader
        // already has the language's own spelling: `move OPERAND |> STAGE`.
        let consumes =
            (word(&segment[0]) == Some(MOVE_KEYWORD) && segment.len() > 1).then(|| &segment[0]);
        if consumes.is_some() && index == 0 {
            return Err(refusal(
                "Q036",
                segment[0].span,
                "`move` consumes the value a previous stage produced; the first stage's operand \
                 is written by hand, so spell it `move OPERAND |> STAGE`",
            ));
        }
        stages.push(Stage {
            tokens: if consumes.is_some() {
                &segment[1..]
            } else {
                &segment[..]
            },
            consumes,
            pipe,
        });
    }

    let last = stages.len() - 1;
    for (index, stage) in stages.iter().enumerate() {
        classify(stage)?;
        if index != last && !stage.produces_a_result() {
            let name = word(&stage.tokens[0]).unwrap_or("this stage");
            return Err(refusal(
                "Q037",
                stage.tokens[0].span,
                format!(
                    "`{name}` produces no value, so it cannot feed a later stage: it is a sink, \
                     and only a call stage binds a result. Make it the pipeline's last stage"
                ),
            ));
        }
    }
    if binder.is_some() && !stages[last].produces_a_result() {
        let name = word(&stages[last].tokens[0]).unwrap_or("this stage");
        return Err(refusal(
            "Q036",
            stages[last].tokens[0].span,
            format!("`{name}` produces no value, so a bound pipeline cannot end in it"),
        ));
    }
    if let (true, Some(span)) = (stages.len() > 1, collision) {
        return Err(refusal(
            "Q036",
            span,
            format!(
                "a multi-stage pipeline binds hygienic temporaries named `{TEMPORARY_PREFIX}N`, \
                 and this file writes that prefix itself; rename the binding rather than letting \
                 the desugaring rename it"
            ),
        ));
    }

    let mut out = Vec::with_capacity(run.len() + 8 * stages.len());
    let mut incoming: Vec<Token> = operand.to_vec();
    for (index, stage) in stages.iter().enumerate() {
        let first = index == 0;
        let anchor = stage.anchor(first);
        let final_stage = index == last;
        if final_stage {
            if let Some(binder) = &binder {
                // A one-stage bound pipeline keeps the reader's own `let NAME =`
                // tokens in place. A multi-stage one cannot: those bytes sit to
                // the left of every earlier stage, so keeping them would make
                // this statement's byte span swallow the statements before it
                // and every route's span-keyed token window would return the
                // wrong tokens. The binder is re-anchored at the last `|>`
                // instead, with the reader's own name preserved.
                if stages.len() == 1 {
                    out.extend(binder.iter().cloned());
                } else {
                    out.push(synthesized_word("let".to_owned(), anchor));
                    out.push(synthesized_word(
                        word(&binder[1])
                            .expect("the binder name is a word")
                            .to_owned(),
                        anchor,
                    ));
                    out.push(synthesized_at(Symbol::Equal, anchor));
                }
            }
        } else {
            out.push(synthesized_word("let".to_owned(), anchor));
            out.push(synthesized_word(temporary(*counter), anchor));
            out.push(synthesized_at(Symbol::Equal, anchor));
        }
        emit_stage(&mut out, stage, &incoming, anchor)?;
        if final_stage {
            out.push(semicolon.clone());
        } else {
            // The terminator anchors at the *start* of the next stage's `|>`,
            // and that stage's own synthesized tokens anchor at its end, so the
            // two statements' spans cannot touch.
            out.push(synthesized_at(
                Symbol::Semicolon,
                stages[index + 1].pipe.start,
            ));
            incoming = Vec::new();
            if let Some(token) = stages[index + 1].consumes {
                incoming.push((*token).clone());
            }
            incoming.push(synthesized_word(
                temporary(*counter),
                stages[index + 1].anchor(false),
            ));
            *counter += 1;
        }
    }
    Ok(out)
}

fn temporary(index: usize) -> String {
    format!("{TEMPORARY_PREFIX}{index}")
}

/// What kind of statement one stage is, refusing every shape outside the closed
/// stage table.
enum StageKind {
    /// A statement head with `selectors` closed selector words before its
    /// operand.
    Sink { selectors: usize },
    /// A callee name, an optional parenthesized argument list ending at
    /// `arguments_end`, and an optional adapter tail after it.
    Call { arguments_end: Option<usize> },
}

fn classify(stage: &Stage<'_>) -> Result<StageKind, PipeRefusal> {
    let head = &stage.tokens[0];
    let Some(name) = word(head) else {
        return Err(refusal(
            "Q036",
            head.span,
            "a pipeline stage is a statement head or a callee name",
        ));
    };
    if OPERANDLESS_STAGES.contains(&name) {
        return Err(refusal(
            "Q037",
            head.span,
            format!("`{name}` takes no operand, so nothing can be piped into it"),
        ));
    }
    if let Some((_, selectors)) = SINK_STAGES.iter().find(|(head, _)| *head == name) {
        if let Some(token) = stage.consumes {
            return Err(refusal(
                "Q036",
                token.span,
                format!("`{name}` renders a scalar; it takes no `move`"),
            ));
        }
        if stage.tokens.len() != 1 + selectors {
            return Err(refusal(
                "Q036",
                head.span,
                format!(
                    "`{name}` names {selectors} selector word(s) before its operand; this stage \
                     names {}",
                    stage.tokens.len() - 1
                ),
            ));
        }
        return Ok(StageKind::Sink {
            selectors: *selectors,
        });
    }
    if stage.tokens.len() == 1 {
        return Ok(StageKind::Call {
            arguments_end: None,
        });
    }
    if !is(&stage.tokens[1], Symbol::LeftParen) {
        // No argument list: everything after the callee name is the adapter
        // tail, which the route's own grammar checks.
        return Ok(StageKind::Call {
            arguments_end: None,
        });
    }
    let mut depth = 0i32;
    for (index, token) in stage.tokens.iter().enumerate().skip(1) {
        match &token.kind {
            TokenKind::Symbol(Symbol::LeftParen) => depth += 1,
            TokenKind::Symbol(Symbol::RightParen) => {
                depth -= 1;
                if depth == 0 {
                    return Ok(StageKind::Call {
                        arguments_end: Some(index),
                    });
                }
            }
            _ => {}
        }
    }
    Err(refusal(
        "Q036",
        stage.tokens[1].span,
        "a call stage is a callee name, optionally followed by a parenthesized argument list",
    ))
}

/// Emit one stage's statement body — everything between its binder, if it has
/// one, and its `;`.
fn emit_stage(
    out: &mut Vec<Token>,
    stage: &Stage<'_>,
    incoming: &[Token],
    anchor: usize,
) -> Result<(), PipeRefusal> {
    match classify(stage)? {
        StageKind::Sink { selectors } => {
            // A statement stage: the head and its closed selector words are not
            // arguments, so the operand follows them. Arity beyond the selector
            // count is left to the route's own grammar, which already refuses a
            // wrong selector or a wrong operand with its own diagnostic at the
            // right span.
            out.extend_from_slice(&stage.tokens[..1 + selectors]);
            out.extend_from_slice(incoming);
        }
        StageKind::Call { arguments_end } => {
            // A call stage. `f` and `f()` both take the incoming value as the
            // sole argument; `f(a, b)` takes it as the first of three. Anything
            // after the argument list is the acquisition adapter, carried
            // through verbatim.
            let (existing, tail): (&[Token], &[Token]) = match arguments_end {
                Some(end) => (&stage.tokens[2..end], &stage.tokens[end + 1..]),
                None => (&[], &stage.tokens[1..]),
            };
            out.push(stage.tokens[0].clone());
            out.push(synthesized_at(Symbol::LeftParen, anchor));
            out.extend_from_slice(incoming);
            if !existing.is_empty() {
                out.push(synthesized_at(Symbol::Comma, anchor));
                out.extend_from_slice(existing);
            }
            out.push(synthesized_at(Symbol::RightParen, anchor));
            emit_tail(out, tail, incoming)?;
        }
    }
    Ok(())
}

/// Emit a call stage's adapter tail, resolving every bare `move`.
///
/// A `move` with no name after it — admitted nowhere else in the source
/// language — denotes *the value piped into this stage*. It exists because a
/// hygienic temporary has no name the reader could write, and an acquisition's
/// error arm must name every live identity it disposes of (`Q025`). Resolving
/// it here keeps the pipe a pure token substitution: what reaches the extern
/// route is the ordinary `CALLEE(move NAME)` disposition it already checks.
fn emit_tail(out: &mut Vec<Token>, tail: &[Token], incoming: &[Token]) -> Result<(), PipeRefusal> {
    // The name of the incoming value, when it has one: `NAME` or `move NAME`.
    let piped = incoming
        .last()
        .filter(|token| word(token).is_some())
        .filter(|_| incoming.len() == 1 || word(&incoming[0]) == Some(MOVE_KEYWORD));
    for (index, token) in tail.iter().enumerate() {
        out.push(token.clone());
        if word(token) != Some(MOVE_KEYWORD) {
            continue;
        }
        let bare = tail
            .get(index + 1)
            .is_none_or(|next| is(next, Symbol::RightParen) || is(next, Symbol::Comma));
        if !bare {
            continue;
        }
        let Some(piped) = piped else {
            return Err(refusal(
                "Q036",
                token.span,
                "a bare `move` names the value piped into this stage, and this stage's incoming \
                 value is not a binding",
            ));
        };
        out.push(piped.clone());
    }
    Ok(())
}

/// The byte extents of every `if` condition in one file — the one position
/// where a logical connective is admitted (card `OR-AND-LOWERING`).
///
/// Collected from the parsed statements rather than from a token scan, so
/// "condition position" means *the parser decided this is a condition*, not
/// "there is an `if` somewhere to the left". Whether the connective inside is
/// well formed is `source.branch.v1`'s business (`Q045`); the gate only
/// declines to refuse it for being a connective.
fn condition_spans(program: &SourceProgram) -> Vec<Span> {
    fn walk(block: &SourceBlock, out: &mut Vec<Span>) {
        for statement in &block.statements {
            if let StatementKind::If { conditions, .. } = &statement.value {
                out.extend(conditions.iter().copied());
            }
            for nested in statement.value.nested_blocks() {
                walk(nested, out);
            }
        }
    }
    let mut spans = Vec::new();
    for declaration in &program.declarations {
        if let Some(body) = declaration.value.body.as_ref() {
            walk(body, &mut spans);
        }
    }
    spans
}

/// Raise the operator refusals recorded for one input file.
///
/// Called per input file before name/type checking, beside
/// [`crate::module_imports::check_source_imports`]. Two families:
///
/// * the desugarer's own refusals, carried on the program; and
/// * the operator tokens this revision gives no meaning **here**: the three
///   single-byte bitwise reservations `|`, `&` and `^` (which have no meaning
///   anywhere), and the two logical connectives `||` and `&&` outside an `if`
///   condition. All refuse at `Q038`.
///
/// Card `OR-AND-LOWERING` restated `Q038`'s condition rather than adding a
/// code. Under card `PIPE-OPERATOR` it read "a claimed-but-unlowered logical
/// operator"; the connectives are now lowered, so it reads "an operator token
/// used where this revision gives it no meaning", which is the condition this
/// gate has always decided. No code moved and no refusal was added or removed
/// at the *program* level: every byte that refused before still refuses.
pub fn check_source_operators(program: &SourceProgram) -> Result<(), Vec<SourceDiagnostic>> {
    let tokens = program.lexical_tokens();
    let mut diagnostics: Vec<SourceDiagnostic> = program
        .pipe_refusals()
        .iter()
        .map(|refusal| SourceDiagnostic {
            code: refusal.code,
            pass: SemanticPass::Pipe,
            span: refusal.span,
            message: refusal.message.clone(),
        })
        .collect();

    let conditions = condition_spans(program);
    let in_condition = |span: Span| {
        conditions
            .iter()
            .any(|extent| extent.start <= span.start && span.end <= extent.end)
    };

    for token in tokens {
        let message = match &token.kind {
            // Card `BITWISE-CONDITIONS` spends the three reservations card
            // `OR-AND-LOWERING` held: `|`, `^` and `&` are the bitwise
            // operators of `source.branch.v1`'s integer tier, and that tier
            // lives in **one** position — a comparison operand inside an `if`
            // condition. Everywhere else there is no integer tier to hold their
            // value, so the gate keeps refusing them, exactly as it refuses the
            // two logical connectives outside a condition.
            TokenKind::Symbol(Symbol::Pipe) if !in_condition(token.span) => {
                "`|` is the bitwise-or operator. It is admitted only inside an `if` \
                 condition, as a parenthesized comparison operand; there is no integer \
                 expression tier to hold its value anywhere else"
                    .to_owned()
            }
            TokenKind::Symbol(Symbol::Ampersand) if !in_condition(token.span) => {
                "`&` is the bitwise-and operator. It is admitted only inside an `if` \
                 condition, as a parenthesized comparison operand; there is no integer \
                 expression tier to hold its value anywhere else. QCore spells logical \
                 and as `&&`"
                    .to_owned()
            }
            TokenKind::Symbol(Symbol::Caret) if !in_condition(token.span) => {
                "`^` is the bitwise-xor operator. It is admitted only inside an `if` \
                 condition, as a parenthesized comparison operand; there is no integer \
                 expression tier to hold its value anywhere else"
                    .to_owned()
            }
            // The connectives are admitted only where `source.branch.v1` can
            // lower them: an `if` condition. Elsewhere there is no branch to
            // lower onto and no expression tier to hold a Boolean value.
            TokenKind::Symbol(Symbol::PipePipe) if !in_condition(token.span) => {
                "`||` is the logical-or operator. It is admitted only in an `if` condition, where \
                 `source.branch.v1` lowers it to a right-nested branch; there is no Boolean \
                 expression tier to hold its value anywhere else"
                    .to_owned()
            }
            TokenKind::Symbol(Symbol::AndAnd) if !in_condition(token.span) => {
                "`&&` is the logical-and operator. It is admitted only in an `if` condition, where \
                 `source.branch.v1` lowers it to a nested branch; there is no Boolean expression \
                 tier to hold its value anywhere else"
                    .to_owned()
            }
            // A `|>` that survived desugaring belongs to a pipeline the
            // rewriter refused; that refusal is already in `diagnostics` and
            // this residual would only duplicate it. A connective or a bitwise
            // operator inside a condition is the branch pass's to accept or
            // refuse at `Q045`.
            _ => continue,
        };
        diagnostics.push(SourceDiagnostic {
            code: "Q038",
            pass: SemanticPass::Pipe,
            span: token.span,
            message,
        });
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
        Err(diagnostics)
    }
}
