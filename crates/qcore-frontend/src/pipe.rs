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
//!             | "drop"                       -- the ownership IR's own release
//!             | NAME [ "(" [arg {"," arg}] ")" ] [ ADAPTER ]   -- a G02d call
//! ADAPTER   ::= "succeeds" BOOL "else" {disposition} "raise" NAME
//! ```
//!
//! | Written | Desugars to |
//! |---|---|
//! | `total \|> print dec;` | `print dec total;` |
//! | `total \|> log fatal dec;` | `log fatal dec total;` |
//! | `b \|> drop;` | `drop b;` |
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
//!   `share N`, and the mutex and epoch clause words — is a statement whose
//!   operands are not a parenthesized argument list. A call stage only ever
//!   emits `HEAD ( … )`, so none of them is reachable by *that* route, and a
//!   keyword-shaped head reaches the pipe only by being named in the closed
//!   statement table.
//!
//! # The releasing stage (card `PIPE-DROP-STAGE`)
//!
//! `drop` is the one keyword-shaped head that card added, and it is the only
//! one: see [`SINK_STAGES`] for the census and the criterion. What makes it a
//! *different kind of entry* from `print` and `log` is the argument for
//! admitting it, not the desugaring — both are `HEAD SELECTOR* OPERAND ";"`,
//! and both produce no result, so a release is last-stage-only by exactly the
//! rule that makes a sink last-stage-only. That coincidence is load-bearing and
//! is stated rather than relied on: a released value cannot feed a later stage
//! **because there is no value to feed**, and the pipe needs no rule about
//! custody to say so. What a *later statement* does with a released binding is
//! the checker's business, unchanged: `b |> drop; b |> drop;` is the ordinary
//! double release, refused where it always was.
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
//! * **A sink cannot feed a later stage.** `print`, `log`, `drop` and `flush`
//!   are last-stage-only and refuse at `Q037` naming the stage.
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
//! (Card BITWISE-CONDITIONS spent the three in an `if` condition; card
//! CELL-GUARD-RESULT-RANGE-CHECKED admits them as the set operators of an
//! `ensures normal result_set(SET);` contract clause, G05j, and nowhere else.)
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
use crate::frontend::{
    SemanticPass, SourceBlock, SourceContractClauseKind, SourceDiagnostic, SourceProgram,
    StatementKind,
};
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

/// What a statement-stage entry does with the value piped into it.
///
/// Both roles **emit identically** — the head, its closed selector words, then
/// the operand — and neither produces a result, so both are last-stage-only by
/// one rule. The role decides only what a refusal *says*, and records which
/// argument admitted the entry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StatementRole {
    /// Renders the value and leaves custody exactly as it found it.
    Renders,
    /// Releases the value's custody (card `PIPE-DROP-STAGE`).
    Releases,
}

/// The statement heads that are **statement stages**, with the number of closed
/// selector words each takes before its operand. Anything not in this table is
/// a call stage or a refusal; the table is closed on purpose, so a new
/// statement form does not become pipeable by accident.
///
/// # Which keyword-shaped heads are in it, and why the rest are not
///
/// Card `PIPE-INTRINSIC-STAGE` measured that a *call*-shaped model intrinsic
/// needs no entry — it is a call stage already — and deliberately withheld
/// `drop`, the entry a reader reaches for first, because a releasing entry is a
/// different kind of entry from a rendering one. Card `PIPE-DROP-STAGE` took
/// that decision, admitted `drop` and **no other head**, on one criterion
/// applied to the whole keyword-shaped census:
///
/// 1. its statement form is exactly `HEAD SELECTOR* OPERAND ";"`, where every
///    selector is a word from a closed, route-independent set; and
/// 2. a stage can actually *feed* it something no reader can name — that is,
///    the route it belongs to has a call-shaped intrinsic whose result is the
///    kind of thing the head takes. Otherwise the entry buys nothing but a
///    second spelling of a statement over a name the reader already wrote.
///
/// `drop` passes both: it is `drop NAME ";"` with no selectors, and the task
/// routes' `qcore_buffer_new` and `qcore_error_payload` — and, measured by this
/// card, `source.buffer.v1`'s own `bytes(N)` — bind owned resources to hygienic
/// temporaries that had, until this entry, no release at all
/// (`PIPE-INTRINSIC-STAGE` §3.4). One entry reaches every route that releases,
/// because they all spell their release with the same reserved `drop`: that word
/// is [`qcore_core::ownership::OwnershipOp::Drop`] itself, not a per-route
/// synonym. The rest of the census fails one condition or the other, each for a
/// reason of its own:
///
/// * `spawn S W(ARGS)?` and `join move H?` fail (1): their operands are neither
///   a parenthesized argument list nor a bare name.
/// * `move NAME;` passes (1) and is refused anyway: `move` is already this
///   grammar's own stage prefix for consuming an incoming temporary, so an
///   entry would give one spelling two meanings.
/// * `read N;`, `write N;`, `wait N;`, `use N;`, `open N;`, `close N;` pass (1)
///   and fail (2): they belong to `source.mutex.guard.v1`, whose model has **no
///   call-shaped intrinsic at all**, so a stage can never produce their
///   operand. Their in-section forms — `read N with G;`, `close N unchanged;`,
///   `grow N guards M;` — also fail (1), since a guard name is an open name
///   rather than a closed selector word.
/// * The epoch plan clauses (`bound`, `overhead`, `peak`, `effort`, `credit`,
///   `retained`) fail (2) twice over: their operands must be check-time
///   constants, which no stage result is.
/// * `return` passes (1) and fails (2) by the *second* half of its condition:
///   the value a chain produces is already nameable by the reader, because a
///   bound pipeline (`let NAME = … ;`) binds exactly it. A `return` entry would
///   be a control-flow entry — a third kind — bought for a spelling the
///   language already has.
///
/// # Why the selector count is a range (card `STRING-SLICE-IO`)
///
/// Each entry carries the **inclusive range** of closed selector words its head
/// may take, because a sink's selector set is the sink's own grammar and one of
/// them now has two arities: `source.string.v1` renders a string's bytes with no
/// `std.fmt` rendering at all, so `s |> print;` is `print s;` and
/// `s |> log info;` is `log info s;`, beside the existing `n |> print dec;` and
/// `n |> log info dec;`.
///
/// The pipe decides **nothing** about which arity a route admits. It emits the
/// head, the selector words the reader wrote and then the operand, and the
/// route's own reader refuses a shape it does not model at the route's own code
/// and span — `x |> print;` on `source.std-io.v1` refuses at `Q030` naming the
/// missing `dec`/`hex` rather than at `Q036` naming the pipe, which is the
/// refusal a reader of that module needs. Widening a range therefore moves a
/// refusal from the pipe to the route and never turns one into an acceptance;
/// every pipeline this table accepted before this card desugars to exactly the
/// same tokens, because every one of them wrote the maximum.
const SINK_STAGES: &[(&str, usize, usize, StatementRole)] = &[
    ("print", 0, 1, StatementRole::Renders),
    ("log", 1, 2, StatementRole::Renders),
    ("drop", 0, 0, StatementRole::Releases),
];

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
    /// acquisition keyword. A call stage produces a value; a statement stage
    /// (`print`, `log`, `drop`) and an operandless statement (`flush`) do not,
    /// and that is a property of the closed statement table, not of any
    /// declaration. Card `PIPE-DROP-STAGE`'s releasing entry inherits its
    /// terminality here rather than adding a rule: a released value cannot feed
    /// a later stage because there is no value to feed.
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
        let kind = classify(stage)?;
        if index != last && !stage.produces_a_result() {
            let name = word(&stage.tokens[0]).unwrap_or("this stage");
            // A releasing stage is terminal for the same *reason* a rendering
            // one is — it binds no result — and the message says the thing a
            // reader needs to act on, which is not the same sentence.
            let message = if let StageKind::Sink {
                role: StatementRole::Releases,
                ..
            } = kind
            {
                format!(
                    "`{name}` releases the value piped into it and binds nothing, so it cannot \
                     feed a later stage: a released value is gone. Make it the pipeline's last \
                     stage"
                )
            } else {
                format!(
                    "`{name}` produces no value, so it cannot feed a later stage: it is a sink, \
                     and only a call stage binds a result. Make it the pipeline's last stage"
                )
            };
            return Err(refusal("Q037", stage.tokens[0].span, message));
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
    /// operand, and the role that admitted it to [`SINK_STAGES`].
    Sink {
        selectors: usize,
        role: StatementRole,
    },
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
    if let Some((_, least, most, role)) = SINK_STAGES.iter().find(|(head, _, _, _)| *head == name) {
        if let Some(token) = stage.consumes {
            return Err(refusal(
                "Q036",
                token.span,
                match role {
                    StatementRole::Renders => {
                        format!("`{name}` renders a scalar; it takes no `move`")
                    }
                    // The release *is* the consumption, and `drop NAME;` is how
                    // the language spells it. One spelling per meaning.
                    StatementRole::Releases => format!(
                        "`{name}` releases the value piped into it, which is what consuming it \
                         means here; it takes no `move`"
                    ),
                },
            ));
        }
        let selectors = stage.tokens.len() - 1;
        if selectors < *least || selectors > *most {
            return Err(refusal(
                "Q036",
                head.span,
                match role {
                    StatementRole::Renders if least == most => format!(
                        "`{name}` names {least} selector word(s) before its operand; this \
                         stage names {selectors}"
                    ),
                    StatementRole::Renders => format!(
                        "`{name}` names {least} or {most} selector word(s) before its operand; \
                         this stage names {selectors}"
                    ),
                    StatementRole::Releases => format!(
                        "`{name}` takes the value piped into it and nothing else; this stage \
                         names {} further word(s)",
                        stage.tokens.len() - 1
                    ),
                },
            ));
        }
        return Ok(StageKind::Sink {
            // The count the reader wrote, not a table constant: the operand
            // follows every selector word this stage actually names.
            selectors,
            role: *role,
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
        StageKind::Sink { selectors, .. } => {
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

/// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the predicate spans of every
/// `quant checks P, Idx #i: PREDICATE;` clause (G05c). Its `&&` and `||` are
/// the predicate's own connectives, read literally by the bounds reader
/// (`qcore_contract::bounds_quant`), not expressions of this tier: the clause
/// has no exit selector, nothing lowers it, and no other consumer reads it.
fn quant_check_spans(program: &SourceProgram) -> Vec<Span> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| declaration.value.function_contract.as_ref())
        .flat_map(|contract| &contract.clauses)
        .filter(|clause| {
            matches!(
                clause.value.kind,
                SourceContractClauseKind::QuantCheck
                    | SourceContractClauseKind::QuantProposition
                    | SourceContractClauseKind::QuantCondition(_)
                    | SourceContractClauseKind::QuantEffect(_)
                    | SourceContractClauseKind::QuantTheorem
            )
        })
        .map(|clause| clause.value.expression_span)
        .collect()
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4,
/// grammar G05j): the expression spans of every `ensures normal
/// result_set(SET);` clause. Its `|`, `&` and `^` are the set operators of
/// G05j (union, intersection, symmetric difference), read literally by the
/// bounds reader (`qcore_contract::bounds_contract`), not integer expressions
/// of this tier: nothing lowers the clause and no other consumer reads it.
fn result_set_spans(program: &SourceProgram) -> Vec<Span> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| declaration.value.function_contract.as_ref())
        .flat_map(|contract| &contract.clauses)
        .filter(|clause| {
            matches!(
                clause.value.kind,
                SourceContractClauseKind::Ensures(
                    crate::frontend::SourceContractExitSelector::Normal
                )
            ) && clause
                .value
                .expression_source
                .trim_start()
                .strip_prefix("result_set")
                .is_some_and(|rest| rest.trim_start().starts_with('('))
        })
        .map(|clause| clause.value.expression_span)
        .collect()
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
    let result_sets = result_set_spans(program);
    let in_result_set = |span: Span| {
        result_sets
            .iter()
            .any(|extent| extent.start <= span.start && span.end <= extent.end)
    };
    let quant_checks = quant_check_spans(program);
    let in_quant_check = |span: Span| {
        quant_checks
            .iter()
            .any(|extent| extent.start <= span.start && span.end <= extent.end)
    };

    // Card `W48-QR-CLOSURES`: the two bars of a closure's capture list (the
    // one `||` of an empty list) are the closure's, directly after
    // `spawn SCOPE`. Every other bar keeps its reading; one that opens or
    // closes a capture-list shape anywhere else is named as that.
    let closure_bar = |span: Span| {
        program
            .closures()
            .iter()
            .any(|closure| closure.is_bar(span))
    };
    let capture_list_shape = |index: usize| {
        let next = tokens.get(index + 1).map(|token| &token.kind);
        matches!(next, Some(TokenKind::Symbol(Symbol::Arrow)))
            || (matches!(tokens[index].kind, TokenKind::Symbol(Symbol::Pipe))
                && matches!(
                    next,
                    Some(TokenKind::Word(word))
                        if matches!(word.as_str(), "copy" | "move" | "read" | "write" | "lend")
                ))
    };
    for (index, token) in tokens.iter().enumerate() {
        let message = match &token.kind {
            // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: a quant check's own
            // connectives (G05c).
            TokenKind::Symbol(Symbol::PipePipe | Symbol::AndAnd) if in_quant_check(token.span) => {
                continue;
            }
            TokenKind::Symbol(Symbol::Pipe | Symbol::PipePipe) if closure_bar(token.span) => {
                continue;
            }
            // Card CELL-GUARD-RESULT-RANGE-CHECKED: a result set's own
            // operators (G05j).
            TokenKind::Symbol(Symbol::Pipe | Symbol::Ampersand | Symbol::Caret)
                if in_result_set(token.span) =>
            {
                continue;
            }
            TokenKind::Symbol(Symbol::Pipe | Symbol::PipePipe)
                if !in_condition(token.span) && capture_list_shape(index) =>
            {
                "a capture list `|…|` belongs to a closure, and a closure is a spawn target \
                 only: `spawn SCOPE |CAPTURES| -> TYPE { … }`. A closure stored, passed or \
                 called as a value is not admitted (W48-QR-CLOSURE-VALUES)"
                    .to_owned()
            }
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
