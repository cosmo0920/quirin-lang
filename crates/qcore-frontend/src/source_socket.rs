//! Exact source lowering for `source.socket.v1` — the `std.socket` bundled
//! module (card `QUIRIN-SOCKET-V1`).
//!
//! ```text
//! module    ::= import* (error-decl | entry)*
//! import    ::= "import" ("std.socket" | "std.fmt" | "std.io" | "std.logger") ";"
//! entry     ::= "fn" NAME "(" ")" "->" "unit"
//!               ["effects" "{" EFFECT {"," EFFECT} "}"] "{" stmt* "}"
//! stmt      ::= "let" NAME "=" ["-"] INTEGER ";"
//!             | "let" NAME "=" ("tcp" | "udp") ";"        -- create, acquiring custody
//!             | "bind" NAME ";"                           -- loopback, kernel-chosen port
//!             | "listen" NAME ";"
//!             | "connect" NAME NAME ";"                   -- to a listener of THIS program
//!             | "let" NAME "=" "accept" NAME ";"          -- acquire the accepted socket
//!             | "send" NAME PAYLOAD ";"
//!             | "recv" NAME INTEGER ";"
//!             | "let" NAME "=" "recv" NAME ";"            -- one byte, bound
//!             | "sendto" NAME NAME PAYLOAD ";"
//!             | "recvfrom" NAME INTEGER ";"
//!             | "let" NAME "=" "recvfrom" NAME ";"        -- one byte, bound
//!             | "close" NAME ";"                          -- release
//!             | "print" RENDERING OPERAND ";"             -- std.io + std.fmt
//!             | "log" LEVEL RENDERING OPERAND ";"         -- std.logger + std.fmt
//!             | "flush" ";"                               -- std.logger
//!             | "return" ";"
//! PAYLOAD   ::= "[" OPERAND {"," OPERAND} "]"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # Ten reserved words, and a census that found no collision
//!
//! `tcp`, `udp`, `bind`, `listen`, `connect`, `accept`, `send`, `recv`,
//! `sendto` and `recvfrom`. Every one is a clause word in a statement, never a
//! reference to a binding, and every one is load-bearing in the measured sense
//! `grow`, `rank`, `bytes` and `extent` are: name resolution collects the tokens
//! after a `let`'s `=` and the words of a bare expression statement as
//! *referenced* names, so without reservation `let s = tcp;` and `bind s;`
//! refuse at `Q013` with the keyword reported as an unknown name.
//!
//! The constraint-14 census ran over every `.qcore`/`.qc` fixture in the tree
//! (412 files) and every string literal in every Rust source file (41,529
//! literals). None of the ten occurs in any QCore source position: the only
//! occurrences at all are `send` and `bind` inside `//` comments of the contract
//! corpus, and Rust-side diagnostic message text. `stream` and `socket` were
//! **rejected** as candidate spellings by the same census — `stream` occurs as a
//! contract `extern fn` **parameter name**, which the reference filter shields
//! only because it is followed by `:`, and that is too thin a margin to build a
//! keyword on.
//!
//! Four existing reserved words are reused rather than respelled: `let`,
//! `close`, `return` and the `source.std-io.v1` line words. The release is
//! `close NAME;` and deliberately **not** a new word — `close` is already
//! reserved, `std.event` and the checked mutex-guard profile already use it at a
//! statement head, and the operation it names on a socket is the one C spells
//! the same way. It is not `drop` either, even though the underlying operation
//! *is* [`qcore_core::ownership::OwnershipOp::Drop`]: `drop` is the language's
//! word for relinquishing a value, and a descriptor close is a system call whose
//! failure the OS-endpoint relation has documented answers for. Naming them the
//! same would hide that.
//!
//! # The module is `std.socket` and not `std.net`
//!
//! Both words census clean, so the choice is argued rather than forced. A module
//! name in this language claims exactly what it grants — `std.buffer` is not
//! `std.memory`, `std.string` is not `std.text` — and what this one grants is
//! the BSD socket vocabulary over loopback: create, bind, listen, connect,
//! accept, send, receive, close. `std.net` would name a capability space
//! (addressing, name resolution, interfaces, TLS) of which v1 implements none,
//! and a later card supplying any of those would have to either overload the
//! name or take a second one. `std.socket` leaves `std.net` available for
//! exactly that.
//!
//! # The operand rule is half the effect discipline
//!
//! An `OPERAND` is an integer literal or a name this entry bound to a
//! check-time-constant integer. Neither form can denote a socket, so a payload
//! byte can never be one. A socket is named only at a custody position, and each
//! of those must name one. That is why every byte this program puts on the wire
//! is decided before emission — which is what lets the checker predict what each
//! receive takes and the emitted program verify it.
//!
//! # What this lowering does not establish
//!
//! It decides no custody, no transport state and no determinism question: it
//! produces a [`SocketProgram`] and stops. Every refusal a reader sees at
//! `Q081`–`Q087` is [`qcore_core::std_socket`]'s, attributed to the statement
//! whose event it named.

use std::collections::BTreeMap;

use crate::frontend::{
    DeclarationKind, SemanticPass, SourceBlock, SourceDiagnostic, SourceProgram, TypedSourceProgram,
};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use crate::source_std_io::{FLUSH_KEYWORD, LOG_KEYWORD, PRINT_KEYWORD};
use qcore_core::std_socket::{
    MAX_PAYLOAD, SocketKind, SocketOperand, SocketProgram, SocketStatement,
};
use qcore_core::typed_errors::{Effect, EffectRow};

/// The TCP creation keyword.
pub const TCP_KEYWORD: &str = "tcp";
/// The UDP creation keyword.
pub const UDP_KEYWORD: &str = "udp";
pub const BIND_KEYWORD: &str = "bind";
pub const LISTEN_KEYWORD: &str = "listen";
pub const CONNECT_KEYWORD: &str = "connect";
pub const ACCEPT_KEYWORD: &str = "accept";
pub const SEND_KEYWORD: &str = "send";
pub const RECV_KEYWORD: &str = "recv";
pub const SENDTO_KEYWORD: &str = "sendto";
pub const RECVFROM_KEYWORD: &str = "recvfrom";
/// The release keyword. Deliberately the **existing** reserved `close`.
pub const CLOSE_KEYWORD: &str = "close";

/// The statement heads this route owns. A file containing any of them at a
/// statement head is a `source.socket.v1` module.
///
/// `close` is deliberately **absent**: it is an established statement head of
/// two other profiles, and a module containing only `close` is not a socket
/// module. The same reasoning `STD-BUFFER` applied to `drop`.
const STATEMENT_HEADS: [&str; 8] = [
    BIND_KEYWORD,
    LISTEN_KEYWORD,
    CONNECT_KEYWORD,
    SEND_KEYWORD,
    RECV_KEYWORD,
    SENDTO_KEYWORD,
    RECVFROM_KEYWORD,
    ACCEPT_KEYWORD,
];

/// A complete lowering of one entry. The core checker still has to validate the
/// program; this type is not evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredSocketModule {
    pub entry: String,
    pub program: SocketProgram,
    /// Source span of the statement at the same index. Every statement has one.
    pub event_spans: Vec<Span>,
    /// Fallback span for a refusal that names no statement.
    pub scope_span: Span,
}

fn window(tokens: &[Token], span: Span) -> Vec<&Token> {
    tokens
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect()
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

fn ends_with_semicolon(window: &[&Token], cursor: usize) -> bool {
    symbol(window, cursor, Symbol::Semicolon) && window.len() == cursor + 1
}

/// Read `["-"] INTEGER` starting at `cursor`. Out-of-range literals are refused,
/// never wrapped.
fn integer(window: &[&Token], cursor: usize) -> Option<(i32, usize)> {
    let (negative, cursor) = match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Symbol(Symbol::Minus)) => (true, cursor + 1),
        _ => (false, cursor),
    };
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Integer(text)) => {
            let value = text.parse::<i64>().ok()?;
            let value = if negative { -value } else { value };
            i32::try_from(value).ok().map(|value| (value, cursor + 1))
        }
        _ => None,
    }
}

fn operand(window: &[&Token], cursor: usize) -> Option<(SocketOperand, usize)> {
    if let Some((value, next)) = integer(window, cursor) {
        return Some((SocketOperand::Literal(value), next));
    }
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::Word(name)) => Some((SocketOperand::Scalar(name.clone()), cursor + 1)),
        _ => None,
    }
}

/// Read `"[" OPERAND {"," OPERAND} "]"` starting at `cursor`.
fn payload(window: &[&Token], cursor: usize) -> Option<(Vec<SocketOperand>, usize)> {
    if !symbol(window, cursor, Symbol::LeftBracket) {
        return None;
    }
    let mut at = cursor + 1;
    let mut bytes = Vec::new();
    loop {
        if symbol(window, at, Symbol::RightBracket) {
            // An empty list is lowered and refused by the model's own length
            // rule (`Q084`), so the two spellings of "no bytes" report the same
            // thing.
            return Some((bytes, at + 1));
        }
        let (byte, next) = operand(window, at)?;
        bytes.push(byte);
        at = next;
        if symbol(window, at, Symbol::Comma) {
            at += 1;
        } else if !symbol(window, at, Symbol::RightBracket) {
            return None;
        }
        if bytes.len() > MAX_PAYLOAD {
            // Stop rather than build an unbounded vector from a malformed
            // window; the model refuses the length it does see.
            return Some((bytes, at));
        }
    }
}

/// Which of this card's constructs a statement uses, if any. Used by the import
/// gate and by route selection, so a file can never be routed here without
/// having needed the import, or need the import without being routed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SocketUse {
    /// `let NAME = tcp;` / `let NAME = udp;`
    Create,
    /// `bind NAME;`, `listen NAME;`, `connect NAME NAME;`
    Configure,
    /// `let NAME = accept NAME;`
    Accept,
    /// `send`, `recv`, `sendto`, `recvfrom`
    Transfer,
}

/// Classify one statement's token window as a socket construct.
///
/// The **one** place that decides whether a statement is this route's, so route
/// selection, the import gate and the lowering below cannot disagree about it.
fn socket_use(window: &[&Token]) -> Option<SocketUse> {
    if word(window, 0) == Some("let") {
        if !symbol(window, 2, Symbol::Equal) {
            return None;
        }
        return match word(window, 3) {
            Some(TCP_KEYWORD | UDP_KEYWORD) => Some(SocketUse::Create),
            Some(ACCEPT_KEYWORD) => Some(SocketUse::Accept),
            Some(RECV_KEYWORD | RECVFROM_KEYWORD) => Some(SocketUse::Transfer),
            _ => None,
        };
    }
    match word(window, 0) {
        Some(BIND_KEYWORD | LISTEN_KEYWORD | CONNECT_KEYWORD) => Some(SocketUse::Configure),
        Some(SEND_KEYWORD | RECV_KEYWORD | SENDTO_KEYWORD | RECVFROM_KEYWORD) => {
            Some(SocketUse::Transfer)
        }
        Some(ACCEPT_KEYWORD) => Some(SocketUse::Accept),
        _ => None,
    }
}

/// Every use of this card's vocabulary in the program, with the span of the
/// first occurrence of each kind.
#[must_use]
pub fn socket_uses(program: &SourceProgram) -> BTreeMap<SocketUse, Span> {
    let tokens = program.lexical_tokens();
    let mut found: BTreeMap<SocketUse, Span> = BTreeMap::new();
    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Function {
            continue;
        }
        if let Some(body) = &declaration.value.body {
            scan(body, tokens, &mut found);
        }
    }
    found
}

fn scan(block: &SourceBlock, tokens: &[Token], found: &mut BTreeMap<SocketUse, Span>) {
    for statement in &block.statements {
        let window = window(tokens, statement.span);
        if let Some(kind) = socket_use(&window) {
            found.entry(kind).or_insert(statement.span);
        }
        for nested in statement.value.nested_blocks() {
            scan(nested, tokens, found);
        }
    }
}

/// Whether this module is a `source.socket.v1` module.
#[must_use]
pub fn has_source_socket(program: &SourceProgram) -> bool {
    !socket_uses(program).is_empty()
}

/// The span of the first statement needing `import std.socket;`.
#[must_use]
pub fn socket_span(program: &SourceProgram) -> Option<Span> {
    socket_uses(program).values().next().copied()
}

fn diagnostic(code: &'static str, span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code,
        pass: SemanticPass::StandardSocket,
        span,
        message: message.into(),
    }
}

/// The entry's declared `effects { … }` row. An absent suffix is the empty row,
/// which admits nothing, so the opt-in stays visible.
fn declared_effect_row(tokens: &[TokenKind], span: Span) -> Result<EffectRow, SourceDiagnostic> {
    let Some(index) = tokens
        .iter()
        .position(|token| matches!(token, TokenKind::Word(word) if word == "effects"))
    else {
        return Ok(EffectRow::default());
    };
    if !matches!(
        tokens.get(index + 1),
        Some(TokenKind::Symbol(Symbol::LeftBrace))
    ) {
        return Err(diagnostic(
            "Q080",
            span,
            "an `effects` row requires a block",
        ));
    }
    let mut effects = Vec::new();
    let mut cursor = index + 2;
    loop {
        match tokens.get(cursor) {
            Some(TokenKind::Symbol(Symbol::RightBrace)) => break,
            Some(TokenKind::Word(name)) => {
                let effect = match name.as_str() {
                    "io" => Effect::Io,
                    "alloc" => Effect::Alloc,
                    "mut" => Effect::Mut,
                    other => {
                        return Err(diagnostic(
                            "Q080",
                            span,
                            format!(
                                "`{other}` is not an effect `source.socket.v1` models; use io, \
                                 alloc or mut"
                            ),
                        ));
                    }
                };
                effects.push(effect);
                cursor += 1;
                if matches!(tokens.get(cursor), Some(TokenKind::Symbol(Symbol::Comma))) {
                    cursor += 1;
                }
            }
            _ => {
                return Err(diagnostic("Q080", span, "this `effects` row is not closed"));
            }
        }
    }
    Ok(EffectRow::new(effects))
}

/// Lower one `let` statement. Five shapes share the head, so they are
/// distinguished here and in one place.
fn lower_let(
    window: &[&Token],
    span: Span,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<SocketStatement> {
    let Some(binding) = word(window, 1) else {
        diagnostics.push(diagnostic("Q080", span, "this `let` binds no name"));
        return None;
    };
    if !symbol(window, 2, Symbol::Equal) {
        diagnostics.push(diagnostic(
            "Q080",
            span,
            "`source.socket.v1` binds `let NAME = …;` only",
        ));
        return None;
    }
    // `let NAME = tcp ;` / `let NAME = udp ;`
    if let Some(head @ (TCP_KEYWORD | UDP_KEYWORD)) = word(window, 3) {
        if !ends_with_semicolon(window, 4) {
            diagnostics.push(diagnostic(
                "Q080",
                span,
                format!("`source.socket.v1` creates a socket with `let NAME = {head};`"),
            ));
            return None;
        }
        return Some(SocketStatement::Create {
            name: binding.to_owned(),
            kind: if head == TCP_KEYWORD {
                SocketKind::Tcp
            } else {
                SocketKind::Udp
            },
        });
    }
    // `let NAME = accept NAME ;`
    if word(window, 3) == Some(ACCEPT_KEYWORD) {
        let Some(listener) = word(window, 4) else {
            diagnostics.push(diagnostic(
                "Q080",
                span,
                "`accept NAME` names the listening socket it takes a connection from",
            ));
            return None;
        };
        if !ends_with_semicolon(window, 5) {
            diagnostics.push(diagnostic("Q080", span, "this `accept` is not closed"));
            return None;
        }
        return Some(SocketStatement::Accept {
            binding: binding.to_owned(),
            listener: listener.to_owned(),
        });
    }
    // `let NAME = recv NAME ;` / `let NAME = recvfrom NAME ;`
    if let Some(head @ (RECV_KEYWORD | RECVFROM_KEYWORD)) = word(window, 3) {
        let Some(name) = word(window, 4) else {
            diagnostics.push(diagnostic(
                "Q080",
                span,
                format!("`{head} NAME` names the socket it receives from"),
            ));
            return None;
        };
        if !ends_with_semicolon(window, 5) {
            diagnostics.push(diagnostic(
                "Q080",
                span,
                format!(
                    "the value form of `{head}` takes exactly one byte and binds it; a count \
                     belongs on the statement form `{head} NAME COUNT;`"
                ),
            ));
            return None;
        }
        let binding = Some(binding.to_owned());
        let name = name.to_owned();
        return Some(if head == RECV_KEYWORD {
            SocketStatement::Recv {
                binding,
                name,
                count: 1,
            }
        } else {
            SocketStatement::RecvFrom {
                binding,
                name,
                count: 1,
            }
        });
    }
    // `let NAME = ["-"] INTEGER ;`
    match integer(window, 3) {
        Some((value, next)) if ends_with_semicolon(window, next) => Some(SocketStatement::Let {
            binding: binding.to_owned(),
            value,
        }),
        _ => {
            diagnostics.push(diagnostic(
                "Q080",
                span,
                "`source.socket.v1` binds an integer literal, `tcp`, `udp`, `accept NAME`, \
                 `recv NAME` or `recvfrom NAME`; nothing else",
            ));
            None
        }
    }
}

/// Read a receive count: `recv NAME COUNT;`.
fn lower_receive(
    window: &[&Token],
    span: Span,
    head: &str,
    diagnostics: &mut Vec<SourceDiagnostic>,
) -> Option<SocketStatement> {
    let Some(name) = word(window, 1) else {
        diagnostics.push(diagnostic(
            "Q080",
            span,
            format!("`{head} NAME COUNT;` names the socket it receives from"),
        ));
        return None;
    };
    let Some((count, next)) = integer(window, 2) else {
        diagnostics.push(diagnostic(
            "Q080",
            span,
            format!(
                "`{head} NAME COUNT;` takes one integer literal count; a computed count is not a \
                 check-time constant and `source.socket.v1` decides every received byte before \
                 emission"
            ),
        ));
        return None;
    };
    if !ends_with_semicolon(window, next) {
        diagnostics.push(diagnostic(
            "Q080",
            span,
            format!("this `{head}` is not closed"),
        ));
        return None;
    }
    let Ok(count) = u32::try_from(count) else {
        // The range refusal itself is the core checker's (`Q084`); this is the
        // narrower question of whether the literal is a count at all.
        diagnostics.push(diagnostic(
            "Q084",
            span,
            format!(
                "{count} is not a receive count `source.socket.v1` models; a receive takes 1 to \
                 {MAX_PAYLOAD} bytes"
            ),
        ));
        return None;
    };
    let binding = None;
    let name = name.to_owned();
    Some(if head == RECV_KEYWORD {
        SocketStatement::Recv {
            binding,
            name,
            count,
        }
    } else {
        SocketStatement::RecvFrom {
            binding,
            name,
            count,
        }
    })
}

/// Lower one entry into a [`SocketProgram`]. The module is either entirely
/// inside the grammar above or refused with a precise diagnostic; there is no
/// partially lowered middle state.
///
/// # Errors
///
/// Every refusal the module earns, in source order.
pub fn lower_source_socket(
    parsed: &SourceProgram,
    _typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredSocketModule, Vec<SourceDiagnostic>> {
    let tokens = parsed.lexical_tokens();
    let mut diagnostics = Vec::new();

    let Some(declaration) = parsed.declarations.iter().find(|declaration| {
        declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
    }) else {
        return Err(vec![diagnostic(
            "Q080",
            Span { start: 0, end: 0 },
            format!("`{entry}` is not a function declaration in this module"),
        )]);
    };
    let scope_span = declaration.span;

    // Every socket construct must be in the entry. A helper function carrying
    // one is refused rather than silently unmodeled.
    for other in &parsed.declarations {
        if other.value.kind != DeclarationKind::Function || other.value.name == entry {
            continue;
        }
        if let Some(body) = &other.value.body {
            let mut found = BTreeMap::new();
            scan(body, tokens, &mut found);
            if let Some(span) = found.values().next() {
                diagnostics.push(diagnostic(
                    "Q080",
                    *span,
                    format!(
                        "`source.socket.v1` models the entry only; `{}` is not the entry `{entry}`",
                        other.value.name
                    ),
                ));
            }
        }
    }

    if !declaration.value.parameters.is_empty() {
        diagnostics.push(diagnostic(
            "Q080",
            scope_span,
            "`source.socket.v1` models an entry with no parameters",
        ));
    }

    let declared_effects = match declared_effect_row(&declaration.value.tokens, scope_span) {
        Ok(row) => row,
        Err(error) => {
            diagnostics.push(error);
            EffectRow::default()
        }
    };

    let Some(body) = &declaration.value.body else {
        diagnostics.push(diagnostic("Q080", scope_span, "this entry has no body"));
        return Err(diagnostics);
    };

    let mut statements = Vec::new();
    let mut event_spans = Vec::new();

    for statement in &body.statements {
        let span = statement.span;
        if !statement.value.nested_blocks().is_empty() {
            diagnostics.push(diagnostic(
                "Q080",
                span,
                "`source.socket.v1` models one flat statement sequence; this statement opens a \
                 nested block",
            ));
            continue;
        }
        let window = window(tokens, span);
        let head = word(&window, 0).unwrap_or_default();
        let lowered = match head {
            "let" => lower_let(&window, span, &mut diagnostics),
            BIND_KEYWORD | LISTEN_KEYWORD | CLOSE_KEYWORD => {
                let Some(name) = word(&window, 1) else {
                    diagnostics.push(diagnostic(
                        "Q080",
                        span,
                        format!("`{head} NAME;` names one socket"),
                    ));
                    continue;
                };
                if !ends_with_semicolon(&window, 2) {
                    diagnostics.push(diagnostic(
                        "Q080",
                        span,
                        format!("`source.socket.v1` spells this statement `{head} NAME;`"),
                    ));
                    continue;
                }
                let name = name.to_owned();
                Some(match head {
                    BIND_KEYWORD => SocketStatement::Bind { name },
                    LISTEN_KEYWORD => SocketStatement::Listen { name },
                    _ => SocketStatement::Close { name },
                })
            }
            CONNECT_KEYWORD => match (word(&window, 1), word(&window, 2)) {
                (Some(client), Some(listener)) if ends_with_semicolon(&window, 3) => {
                    Some(SocketStatement::Connect {
                        client: client.to_owned(),
                        listener: listener.to_owned(),
                    })
                }
                _ => {
                    diagnostics.push(diagnostic(
                        "Q080",
                        span,
                        "`connect NAME NAME;` names the connecting socket and a listening \
                             socket of this same program; `source.socket.v1` has no spelled \
                             address, which is what makes the connection's completion decidable \
                             (`SOCKET-EXTERNAL-PEER`)",
                    ));
                    None
                }
            },
            ACCEPT_KEYWORD => {
                diagnostics.push(diagnostic(
                    "Q080",
                    span,
                    "`accept` acquires a socket and so must bind it: `let NAME = accept NAME;`",
                ));
                None
            }
            SEND_KEYWORD => {
                let Some(name) = word(&window, 1) else {
                    diagnostics.push(diagnostic(
                        "Q080",
                        span,
                        "`send NAME [BYTE, …];` names the connected socket it sends from",
                    ));
                    continue;
                };
                match payload(&window, 2) {
                    Some((bytes, next)) if ends_with_semicolon(&window, next) => {
                        Some(SocketStatement::Send {
                            name: name.to_owned(),
                            payload: bytes,
                        })
                    }
                    _ => {
                        diagnostics.push(diagnostic(
                            "Q080",
                            span,
                            "`send NAME [BYTE, …];` carries a bracketed list of integer literals \
                             or bound integer names",
                        ));
                        None
                    }
                }
            }
            SENDTO_KEYWORD => match (word(&window, 1), word(&window, 2)) {
                (Some(name), Some(target)) => match payload(&window, 3) {
                    Some((bytes, next)) if ends_with_semicolon(&window, next) => {
                        Some(SocketStatement::SendTo {
                            name: name.to_owned(),
                            target: target.to_owned(),
                            payload: bytes,
                        })
                    }
                    _ => {
                        diagnostics.push(diagnostic(
                            "Q080",
                            span,
                            "`sendto NAME NAME [BYTE, …];` carries a bracketed list of \
                                 integer literals or bound integer names",
                        ));
                        None
                    }
                },
                _ => {
                    diagnostics.push(diagnostic(
                        "Q080",
                        span,
                        "`sendto NAME NAME [BYTE, …];` names the sending socket and a bound \
                             udp socket of this same program",
                    ));
                    None
                }
            },
            RECV_KEYWORD | RECVFROM_KEYWORD => lower_receive(&window, span, head, &mut diagnostics),
            PRINT_KEYWORD | LOG_KEYWORD | FLUSH_KEYWORD => {
                // The line grammar is `source.std-io.v1`'s, read by its own
                // reader, so a shape one route accepts and another refuses is
                // not expressible.
                match crate::source_std_io::std_io_statement(parsed, span) {
                    Some(Ok(event)) => Some(SocketStatement::Line(event)),
                    Some(Err(message)) => {
                        diagnostics.push(diagnostic("Q080", span, message));
                        None
                    }
                    None => {
                        diagnostics.push(diagnostic(
                            "Q080",
                            span,
                            "this line-producing statement is not one `source.std-io.v1` models",
                        ));
                        None
                    }
                }
            }
            "return" => {
                if ends_with_semicolon(&window, 1) {
                    Some(SocketStatement::Return)
                } else {
                    diagnostics.push(diagnostic(
                        "Q080",
                        span,
                        "`source.socket.v1` models `return;` from a `unit` entry",
                    ));
                    None
                }
            }
            other => {
                diagnostics.push(diagnostic(
                    "Q080",
                    span,
                    format!("`{other}` is not a statement `source.socket.v1` models"),
                ));
                None
            }
        };
        if let Some(lowered) = lowered {
            statements.push(lowered);
            event_spans.push(span);
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(LoweredSocketModule {
        entry: entry.to_owned(),
        program: SocketProgram {
            entry: entry.to_owned(),
            declared_effects,
            statements,
        },
        event_spans,
        scope_span,
    })
}

/// Every statement head this route reserves, for the constraint-14 record and
/// for tests that assert the census stays true.
#[must_use]
pub fn statement_heads() -> [&'static str; 8] {
    STATEMENT_HEADS
}
