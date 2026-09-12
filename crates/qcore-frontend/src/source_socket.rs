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
//!             | "event" "open" ";"                        -- std.event, the one reactor context
//!             | "event" "close" ";"
//!             | "watch" NAME NAME INTEREST ";"            -- register a socket: a LOAN of it
//!             | "unwatch" NAME ";"                        -- end the loan, retire the registration
//!             | "dispatch" "ready" NAME ";"               -- one turn, declared and verified
//!             | "dispatch" "idle" ";"
//!             | "print" RENDERING OPERAND ";"             -- std.io + std.fmt
//!             | "log" LEVEL RENDERING OPERAND ";"         -- std.logger + std.fmt
//!             | "flush" ";"                               -- std.logger
//!             | "return" ";"
//! PAYLOAD   ::= "[" OPERAND {"," OPERAND} "]"
//! OPERAND   ::= ["-"] INTEGER | NAME
//! INTEREST  ::= "read" | "write"
//! EFFECT    ::= "io" | "alloc" | "mut"
//! ```
//!
//! # The reactor composition adds **no** reserved word
//!
//! `event`, `open`, `close`, `watch`, `unwatch`, `dispatch`, `ready`, `idle`,
//! `read` and `write` are every word of the composition, and `STD-EVENT` and the
//! profiles before it reserved all ten. Card `SOCKET-EVENT-COMPOSE` therefore
//! runs the constraint-14 census and adds nothing to it — the composition is
//! spelled entirely in the two routes' existing vocabularies, which is what a
//! composition should look like. `wake` and `endpoints` are deliberately
//! **absent**: this route's readiness is induced by `connect`, `send` and
//! `sendto`, and its descriptors come from `tcp`, `udp` and `accept`, so a
//! module spelling either is refused as a statement this route does not model.
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
use qcore_core::std_event::Interest;
use qcore_core::std_socket::{
    MAX_PAYLOAD, SocketKind, SocketOperand, SocketPayload, SocketProgram, SocketStatement,
    TextPayload,
};
use qcore_core::std_string::decode_string_literal;
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

/// The reactor context keyword (card `SOCKET-EVENT-COMPOSE`). Every word of the
/// composition — `event`, `open`, `close`, `watch`, `unwatch`, `dispatch`,
/// `ready`, `idle`, `read`, `write` — is **already reserved**, by `STD-EVENT`
/// and by the profiles before it, so this card adds **none**.
pub const EVENT_KEYWORD: &str = "event";
pub const WATCH_KEYWORD: &str = "watch";
pub const UNWATCH_KEYWORD: &str = "unwatch";
pub const DISPATCH_KEYWORD: &str = "dispatch";

/// The statement heads of the reactor composition.
///
/// Deliberately **not** part of [`STATEMENT_HEADS`]: they do not select this
/// route. A module spelling only these is a `source.event.v1` module and stays
/// one; the composed route is reached only when *both* vocabularies are present,
/// which the driver decides with the two routes' own existing detectors.
const REACTOR_HEADS: [&str; 4] = [
    EVENT_KEYWORD,
    WATCH_KEYWORD,
    UNWATCH_KEYWORD,
    DISPATCH_KEYWORD,
];

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

/// The raw content of a quoted token at `cursor`, if there is one.
fn literal_text<'a>(window: &'a [&Token], cursor: usize) -> Option<&'a str> {
    match window.get(cursor).map(|token| &token.kind) {
        Some(TokenKind::StringLiteral(text)) => Some(text.as_str()),
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
    /// A transfer whose payload is a **quoted literal** rather than a bracketed
    /// byte list — `send C "…";`, `recv S "…";` and the datagram pair (card
    /// `HTTP-QUIRIN-V1`, the socket card's named follow-on
    /// `SOCKET-STRING-PAYLOAD`).
    ///
    /// A separate use from [`SocketUse::Transfer`] because it is gated by a
    /// **second** module: the payload is a string view's own bytes, which is
    /// `std.string`'s value and not `std.socket`'s, exactly as a rendered text
    /// line needs both `std.string` and an output module (`STRING-SLICE-IO`).
    TextPayload,
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

/// Whether this statement's payload position holds a **quoted literal**.
///
/// The one place that decides it, so the `std.string` import gate and the
/// lowering below cannot disagree about which statements need the text module.
fn carries_text_payload(window: &[&Token]) -> bool {
    let cursor = match word(window, 0) {
        Some(SEND_KEYWORD | RECV_KEYWORD | RECVFROM_KEYWORD) => 2,
        Some(SENDTO_KEYWORD) => 3,
        _ => return false,
    };
    literal_text(window, cursor).is_some()
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
        if carries_text_payload(&window) {
            found
                .entry(SocketUse::TextPayload)
                .or_insert(statement.span);
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

/// The span of the first reactor statement anywhere in `block`, if any
/// (card `SOCKET-EVENT-COMPOSE`).
///
/// Used only by [`lower_source_socket`]'s helper scan, so a `dispatch` in a
/// function this route never lowers is refused rather than silently unmodelled —
/// the hole `COMPOSE-EVENT-LINES` closed for its own lines. It is deliberately
/// **not** part of [`socket_uses`]: route selection and the `std.socket` import
/// gate both read that map, and a pure `source.event.v1` module must select
/// neither.
fn reactor_span(block: &SourceBlock, tokens: &[Token]) -> Option<Span> {
    for statement in &block.statements {
        let window = window(tokens, statement.span);
        if word(&window, 0).is_some_and(|head| REACTOR_HEADS.contains(&head)) {
            return Some(statement.span);
        }
        for nested in statement.value.nested_blocks() {
            if let Some(span) = reactor_span(nested, tokens) {
                return Some(span);
            }
        }
    }
    None
}

/// The span of the first statement needing `import std.socket;`.
///
/// A text payload is deliberately **excluded**: it is not a socket capability
/// and it is gated separately (see [`text_payload_span`]). A statement that
/// carries one is a `send`/`recv`, so the transfer's own use is already in the
/// map and the socket gate still sees it.
#[must_use]
pub fn socket_span(program: &SourceProgram) -> Option<Span> {
    socket_uses(program)
        .into_iter()
        .find(|(kind, _)| *kind != SocketUse::TextPayload)
        .map(|(_, span)| span)
}

/// The span of the first transfer whose payload is a string view, which is what
/// needs `import std.string;` **in addition to** `import std.socket;`.
#[must_use]
pub fn text_payload_span(program: &SourceProgram) -> Option<Span> {
    socket_uses(program).get(&SocketUse::TextPayload).copied()
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

/// One refusal a statement reader produces: the code it belongs to and what it
/// says. The **span** is the caller's, and so is the pass: a composing route
/// re-attributes these to its own band exactly as the socket route
/// re-attributes `source.std-io.v1`'s line messages to `Q080`.
pub type Refusals = Vec<(&'static str, String)>;

fn refuse(refusals: &mut Refusals, code: &'static str, message: impl Into<String>) {
    refusals.push((code, message.into()));
}

/// What one statement reader decided about a token window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketLowering {
    /// A statement of this route.
    Lowered(Box<SocketStatement>),
    /// A statement of this route, spelled wrongly.
    Refused(Refusals),
    /// Not a statement this route spells at all. The **caller** decides what
    /// that means: the socket route refuses it, and a composing route tries its
    /// own vocabulary first and refuses only if neither claims it.
    Unknown,
}

/// Lower one `let` statement. Five shapes share the head, so they are
/// distinguished here and in one place.
fn lower_let(window: &[&Token], refusals: &mut Refusals) -> Option<SocketStatement> {
    let Some(binding) = word(window, 1) else {
        refuse(refusals, "Q080", "this `let` binds no name");
        return None;
    };
    if !symbol(window, 2, Symbol::Equal) {
        refuse(
            refusals,
            "Q080",
            "`source.socket.v1` binds `let NAME = …;` only",
        );
        return None;
    }
    // `let NAME = tcp ;` / `let NAME = udp ;`
    if let Some(head @ (TCP_KEYWORD | UDP_KEYWORD)) = word(window, 3) {
        if !ends_with_semicolon(window, 4) {
            refuse(
                refusals,
                "Q080",
                format!("`source.socket.v1` creates a socket with `let NAME = {head};`"),
            );
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
            refuse(
                refusals,
                "Q080",
                "`accept NAME` names the listening socket it takes a connection from",
            );
            return None;
        };
        if !ends_with_semicolon(window, 5) {
            refuse(refusals, "Q080", "this `accept` is not closed");
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
            refuse(
                refusals,
                "Q080",
                format!("`{head} NAME` names the socket it receives from"),
            );
            return None;
        };
        if !ends_with_semicolon(window, 5) {
            refuse(
                refusals,
                "Q080",
                format!(
                    "the value form of `{head}` takes exactly one byte and binds it; a count \
                     belongs on the statement form `{head} NAME COUNT;`"
                ),
            );
            return None;
        }
        let binding = Some(binding.to_owned());
        let name = name.to_owned();
        return Some(if head == RECV_KEYWORD {
            SocketStatement::Recv {
                binding,
                name,
                count: 1,
                expect: None,
            }
        } else {
            SocketStatement::RecvFrom {
                binding,
                name,
                count: 1,
                expect: None,
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
            refuse(
                refusals,
                "Q080",
                "`source.socket.v1` binds an integer literal, `tcp`, `udp`, `accept NAME`, \
                 `recv NAME` or `recvfrom NAME`; nothing else",
            );
            None
        }
    }
}

/// Decode a quoted payload at `cursor`, or report why it is not one.
///
/// The decoder is `source.string.v1`'s own ([`decode_string_literal`]) and not a
/// second one: a payload is a string view's bytes, so the escapes it admits, the
/// UTF-8 well-formedness it requires and the length it bounds are that route's
/// rules, read here rather than restated. `\r` is one of those escapes, which is
/// what makes a CRLF spellable in a payload at all.
///
/// The refusal is reported at **this** route's `Q084` rather than at the string
/// route's `Q070`: §L21 forbids a second pass reusing a code, and the condition a
/// reader of a socket module meets is "these bytes are not a payload this
/// transfer can carry".
fn lower_text_payload(
    window: &[&Token],
    cursor: usize,
    refusals: &mut Refusals,
) -> Option<(TextPayload, usize)> {
    let raw = literal_text(window, cursor)?;
    match decode_string_literal(raw) {
        Ok(bytes) => Some((TextPayload::inline(bytes), cursor + 1)),
        Err(refusal) => {
            refuse(refusals, "Q084", refusal.to_string());
            None
        }
    }
}

/// Read a receive count: `recv NAME COUNT;`, or a declared payload:
/// `recv NAME "…";`.
fn lower_receive(
    window: &[&Token],
    head: &str,
    refusals: &mut Refusals,
) -> Option<SocketStatement> {
    // The declared-payload form, card `HTTP-QUIRIN-V1`. The count is the
    // declaration's own length, so the two can never disagree: there is one
    // number and it is read from the bytes.
    if literal_text(window, 2).is_some() {
        let Some(name) = word(window, 1) else {
            refuse(
                refusals,
                "Q080",
                format!("`{head} NAME \"…\";` names the socket it receives from"),
            );
            return None;
        };
        let (expect, next) = lower_text_payload(window, 2, refusals)?;
        if !ends_with_semicolon(window, next) {
            refuse(
                refusals,
                "Q080",
                format!(
                    "`{head} NAME \"…\";` declares the bytes it takes as one quoted token and \
                     nothing else"
                ),
            );
            return None;
        }
        let Ok(count) = u32::try_from(expect.bytes.len()) else {
            refuse(
                refusals,
                "Q084",
                "this declared payload is longer than any receive `source.socket.v1` models",
            );
            return None;
        };
        let name = name.to_owned();
        let expect = Some(expect);
        return Some(if head == RECV_KEYWORD {
            SocketStatement::Recv {
                binding: None,
                name,
                count,
                expect,
            }
        } else {
            SocketStatement::RecvFrom {
                binding: None,
                name,
                count,
                expect,
            }
        });
    }
    let Some(name) = word(window, 1) else {
        refuse(
            refusals,
            "Q080",
            format!("`{head} NAME COUNT;` names the socket it receives from"),
        );
        return None;
    };
    let Some((count, next)) = integer(window, 2) else {
        refuse(
            refusals,
            "Q080",
            format!(
                "`{head} NAME COUNT;` takes one integer literal count; a computed count is not a \
                 check-time constant and `source.socket.v1` decides every received byte before \
                 emission"
            ),
        );
        return None;
    };
    if !ends_with_semicolon(window, next) {
        refuse(refusals, "Q080", format!("this `{head}` is not closed"));
        return None;
    }
    let Ok(count) = u32::try_from(count) else {
        // The range refusal itself is the core checker's (`Q084`); this is the
        // narrower question of whether the literal is a count at all.
        refuse(
            refusals,
            "Q084",
            format!(
                "{count} is not a receive count `source.socket.v1` models; a receive takes 1 to \
                 {MAX_PAYLOAD} bytes"
            ),
        );
        return None;
    };
    let binding = None;
    let name = name.to_owned();
    Some(if head == RECV_KEYWORD {
        SocketStatement::Recv {
            binding,
            name,
            count,
            expect: None,
        }
    } else {
        SocketStatement::RecvFrom {
            binding,
            name,
            count,
            expect: None,
        }
    })
}

/// Read one statement window as a `source.socket.v1` statement.
///
/// **The one place this route's statement grammar lives**, published so a
/// *composing* route reads a socket statement with this reader rather than with
/// a second one that might disagree — the discipline
/// `source.std-io.v1::std_io_statement` established and this route has followed
/// for its own line statements since card `QUIRIN-SOCKET-V1`.
///
/// The refusals it returns carry their own **codes** and no span and no pass: the
/// caller owns attribution, so a composed route re-attributes them to its own
/// band without this route's grammar having to know who is reading it.
#[must_use]
pub fn socket_statement(parsed: &SourceProgram, window: &[&Token], span: Span) -> SocketLowering {
    let mut refusals: Refusals = Vec::new();
    let head = word(window, 0).unwrap_or_default();
    let lowered = match head {
        "let" => lower_let(window, &mut refusals),
        BIND_KEYWORD | LISTEN_KEYWORD | CLOSE_KEYWORD => {
            let Some(name) = word(window, 1) else {
                refuse(
                    &mut refusals,
                    "Q080",
                    format!("`{head} NAME;` names one socket"),
                );
                return SocketLowering::Refused(refusals);
            };
            if !ends_with_semicolon(window, 2) {
                refuse(
                    &mut refusals,
                    "Q080",
                    format!("`source.socket.v1` spells this statement `{head} NAME;`"),
                );
                return SocketLowering::Refused(refusals);
            }
            let name = name.to_owned();
            Some(match head {
                BIND_KEYWORD => SocketStatement::Bind { name },
                LISTEN_KEYWORD => SocketStatement::Listen { name },
                _ => SocketStatement::Close { name },
            })
        }
        CONNECT_KEYWORD => match (word(window, 1), word(window, 2)) {
            (Some(client), Some(listener)) if ends_with_semicolon(window, 3) => {
                Some(SocketStatement::Connect {
                    client: client.to_owned(),
                    listener: listener.to_owned(),
                })
            }
            _ => {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`connect NAME NAME;` names the connecting socket and a listening \
                     socket of this same program; `source.socket.v1` has no spelled \
                     address, which is what makes the connection's completion decidable \
                     (`SOCKET-EXTERNAL-PEER`)",
                );
                None
            }
        },
        ACCEPT_KEYWORD => {
            refuse(
                &mut refusals,
                "Q080",
                "`accept` acquires a socket and so must bind it: `let NAME = accept NAME;`",
            );
            None
        }
        SEND_KEYWORD => {
            let Some(name) = word(window, 1) else {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`send NAME [BYTE, …];` names the connected socket it sends from",
                );
                return SocketLowering::Refused(refusals);
            };
            if literal_text(window, 2).is_some() {
                let Some((text, next)) = lower_text_payload(window, 2, &mut refusals) else {
                    return SocketLowering::Refused(refusals);
                };
                if !ends_with_semicolon(window, next) {
                    refuse(
                        &mut refusals,
                        "Q080",
                        "`send NAME \"…\";` carries one quoted token and nothing else",
                    );
                    return SocketLowering::Refused(refusals);
                }
                Some(SocketStatement::Send {
                    name: name.to_owned(),
                    payload: SocketPayload::Text(text),
                })
            } else {
                match payload(window, 2) {
                    Some((bytes, next)) if ends_with_semicolon(window, next) => {
                        Some(SocketStatement::Send {
                            name: name.to_owned(),
                            payload: SocketPayload::Bytes(bytes),
                        })
                    }
                    _ => {
                        refuse(
                            &mut refusals,
                            "Q080",
                            "`send NAME [BYTE, …];` carries a bracketed list of integer \
                             literals or bound integer names, or `send NAME \"…\";` one \
                             quoted payload",
                        );
                        None
                    }
                }
            }
        }
        SENDTO_KEYWORD => match (word(window, 1), word(window, 2)) {
            (Some(name), Some(target)) if literal_text(window, 3).is_some() => {
                let Some((text, next)) = lower_text_payload(window, 3, &mut refusals) else {
                    return SocketLowering::Refused(refusals);
                };
                if !ends_with_semicolon(window, next) {
                    refuse(
                        &mut refusals,
                        "Q080",
                        "`sendto NAME NAME \"…\";` carries one quoted token and nothing else",
                    );
                    return SocketLowering::Refused(refusals);
                }
                Some(SocketStatement::SendTo {
                    name: name.to_owned(),
                    target: target.to_owned(),
                    payload: SocketPayload::Text(text),
                })
            }
            (Some(name), Some(target)) => match payload(window, 3) {
                Some((bytes, next)) if ends_with_semicolon(window, next) => {
                    Some(SocketStatement::SendTo {
                        name: name.to_owned(),
                        target: target.to_owned(),
                        payload: SocketPayload::Bytes(bytes),
                    })
                }
                _ => {
                    refuse(
                        &mut refusals,
                        "Q080",
                        "`sendto NAME NAME [BYTE, …];` carries a bracketed list of \
                         integer literals or bound integer names, or \
                         `sendto NAME NAME \"…\";` one quoted payload",
                    );
                    None
                }
            },
            _ => {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`sendto NAME NAME [BYTE, …];` names the sending socket and a bound \
                     udp socket of this same program",
                );
                None
            }
        },
        RECV_KEYWORD | RECVFROM_KEYWORD => lower_receive(window, head, &mut refusals),
        // The reactor composition (card `SOCKET-EVENT-COMPOSE`). Every word is
        // already reserved; the composition adds none.
        EVENT_KEYWORD => match word(window, 1) {
            Some("open") if ends_with_semicolon(window, 2) => Some(SocketStatement::ReactorOpen),
            Some(CLOSE_KEYWORD) if ends_with_semicolon(window, 2) => {
                Some(SocketStatement::ReactorClose)
            }
            _ => {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`source.socket.v1` opens and closes its one reactor context with \
                     `event open;` and `event close;`",
                );
                None
            }
        },
        WATCH_KEYWORD => match (
            word(window, 1),
            word(window, 2),
            word(window, 3).and_then(Interest::parse),
        ) {
            (Some(registration), Some(socket), Some(interest))
                if ends_with_semicolon(window, 4) =>
            {
                Some(SocketStatement::Watch {
                    registration: registration.to_owned(),
                    socket: socket.to_owned(),
                    interest,
                })
            }
            _ => {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`watch NAME NAME INTEREST;` registers a socket of this entry with its \
                     reactor, where INTEREST is `read` or `write`; the registration is a loan of \
                     the socket and not a second owner of it",
                );
                None
            }
        },
        UNWATCH_KEYWORD => match word(window, 1) {
            Some(registration) if ends_with_semicolon(window, 2) => {
                Some(SocketStatement::Unwatch {
                    registration: registration.to_owned(),
                })
            }
            _ => {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`unwatch NAME;` names the one registration it retires",
                );
                None
            }
        },
        DISPATCH_KEYWORD => match word(window, 1) {
            Some("ready") => match word(window, 2) {
                Some(registration) if ends_with_semicolon(window, 3) => {
                    Some(SocketStatement::Dispatch {
                        ready: Some(registration.to_owned()),
                    })
                }
                _ => {
                    refuse(
                        &mut refusals,
                        "Q080",
                        "`dispatch ready NAME;` names the one registration this turn reports",
                    );
                    None
                }
            },
            Some("idle") if ends_with_semicolon(window, 2) => {
                Some(SocketStatement::Dispatch { ready: None })
            }
            _ => {
                refuse(
                    &mut refusals,
                    "Q080",
                    "a dispatch turn is `dispatch ready NAME;` or `dispatch idle;`: it declares \
                     what the turn reports, and the emitted program checks the declaration rather \
                     than trusting it",
                );
                None
            }
        },
        PRINT_KEYWORD | LOG_KEYWORD | FLUSH_KEYWORD => {
            // The line grammar is `source.std-io.v1`'s, read by its own
            // reader, so a shape one route accepts and another refuses is
            // not expressible.
            match crate::source_std_io::std_io_statement(parsed, span) {
                Some(Ok(event)) => Some(SocketStatement::Line(event)),
                Some(Err(message)) => {
                    refuse(&mut refusals, "Q080", message);
                    None
                }
                None => {
                    refuse(
                        &mut refusals,
                        "Q080",
                        "this line-producing statement is not one `source.std-io.v1` models",
                    );
                    None
                }
            }
        }
        "return" => {
            if ends_with_semicolon(window, 1) {
                Some(SocketStatement::Return)
            } else {
                refuse(
                    &mut refusals,
                    "Q080",
                    "`source.socket.v1` models `return;` from a `unit` entry",
                );
                None
            }
        }
        _ => return SocketLowering::Unknown,
    };
    match lowered {
        Some(lowered) => SocketLowering::Lowered(Box::new(lowered)),
        None => SocketLowering::Refused(refusals),
    }
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
            // A reactor statement in a helper is refused for the same reason a
            // socket statement is: this route models the entry, and admitting a
            // construct without extending the helper scan would leave it
            // neither emitted nor refused.
            let span = found
                .values()
                .next()
                .copied()
                .or_else(|| reactor_span(body, tokens));
            if let Some(span) = span {
                diagnostics.push(diagnostic(
                    "Q080",
                    span,
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
        match socket_statement(parsed, &window, span) {
            SocketLowering::Lowered(lowered) => {
                statements.push(*lowered);
                event_spans.push(span);
            }
            SocketLowering::Refused(refusals) => diagnostics.extend(
                refusals
                    .into_iter()
                    .map(|(code, message)| diagnostic(code, span, message)),
            ),
            SocketLowering::Unknown => diagnostics.push(diagnostic(
                "Q080",
                span,
                format!(
                    "`{}` is not a statement `source.socket.v1` models",
                    word(&window, 0).unwrap_or_default()
                ),
            )),
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
