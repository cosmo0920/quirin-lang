//! Exact source lowering for `source.record.v1` — the first source-level
//! aggregate type (card `AGGREGATES-V1`).
//!
//! ```text
//! decl    ::= "type" NAME "{" FIELD ":" FTYPE { "," FIELD ":" FTYPE } [","] "}"
//! FTYPE   ::= "i32" | "bytes"                -- `bytes`: card AGGREGATE-OWNED-FIELD
//! stmt    ::= "let" NAME "=" TYPE "{" FIELD ":" OPERAND
//!                                     { "," FIELD ":" OPERAND } [","] "}" ";"
//!           | "let" NAME "=" NAME "." FIELD ";"            -- field read, or move out
//!           | "let" NAME "=" NAME "." FIELD "[" OPERAND "]" ";"   -- load through a field
//!           | NAME "." FIELD "[" OPERAND "]" "=" OPERAND ";"      -- store through a field
//!           | "drop" NAME "." FIELD ";"                            -- release one field
//! OPERAND ::= ["-"] INTEGER | NAME          -- `source.buffer.v1`'s own
//! ```
//!
//! The three field-path statement forms and the `bytes` field type are card
//! `AGGREGATE-OWNED-FIELD`'s (wave 39). They reserve nothing: `bytes` is the
//! acquisition's own reserved word (`STD-BUFFER`), the `.` is the `Dot` this
//! reader has always consumed, and `drop`, `[` and `=` are what they were. A
//! construction supplying an acquired buffer's name at a `bytes` field moves
//! it in; `let NAME = RECORD.FIELD;` at such a field moves it out. Which of
//! the two a `let NAME = RECORD.FIELD;` is, the reader does not decide — the
//! spelling is one statement and the declared type decides it in the model.
//!
//! # Zero reserved words, and the census that decided the spelling
//!
//! **This card reserves nothing.** The declaration head is the already-reserved
//! `type`, claimed at the one shape a `{` after the name gives it; the
//! construction and the field read are ordinary `let` statements; and the field
//! separator, the `:` and the `.` are symbols the lexer has always produced.
//!
//! `record` was the obvious spelling and the constraint-14 census **rejected**
//! it. `docs/reports/remaining-v07/probes/d02_boundary_record.qcore` is the
//! single line `record R { }`, and two archived reports
//! (`remaining-v07`, `remaining-d01`) pin that file to
//! `d02_boundary_record.qcore:1:1 Q000`. Claiming `record` at declaration
//! position would make that file parse and falsify a frozen archive's own table,
//! which standing constraint 21 forbids repairing by editing the archive. The
//! syntax was renamed rather than the pinned surface edited — the
//! `buffer` → `bytes`, `work` → `effort` and `stream`-rejection precedent.
//!
//! # Name resolution needed no new rule, and that is a measurement
//!
//! Both positions this card introduces are **already drained** by rules
//! `frontend::lexical_references` has carried since long before it:
//!
//! * a word immediately **preceded by a `Dot`** is skipped, so the `x` of
//!   `let a = p.x;` is not collected — the same rule that already lets
//!   `cost.total` and `std.buffer` through;
//! * a word immediately **followed by a `Colon`** is skipped, so the `x` and `y`
//!   of `Point { x: 1, y: 2 }` are not collected — the same rule that shields
//!   every `extern fn` parameter name.
//!
//! What *is* collected is the type name at a construction (`Point`), and it
//! resolves because every declaration's name is a global — so a construction
//! naming a type this module does not declare refuses at `Q013` before this
//! reader ever sees it, and one naming a type it does declare resolves like any
//! other reference. **No line of `lexical_references` or
//! `is_reserved_source_word` changed for this card.**
//!
//! # The `.` adjacency, pinned rather than assumed
//!
//! Card `SOCKET-INET-ADDRESSES` measured that `127.0.0.1` lexes as
//! `Float Dot Float`, because a `.` between two digit runs is part of a float
//! spelling. A field access is not that shape and cannot become it: the lexer
//! starts a `Float` only at a **digit**, so `p.x` is `Word Dot Word` and `p.0`
//! is `Word Dot Integer` — never one token. The one place the float rule does
//! reach this construct is a *numeric* field path like `p.1.2`, which lexes
//! `Word Dot Float`; this reader requires a `Word` after the `Dot` and refuses
//! it by name. Both adjacencies are pinned by tests.
//!
//! # What this lowering does not establish
//!
//! No field write (`AGGREGATE-FIELD-WRITE`), no view field
//! (`AGGREGATE-VIEW-FIELD`), no nested record (`AGGREGATE-NESTED`), no
//! record-valued parameter or result (`AGGREGATE-ACROSS-CALL`), and no field
//! type but `i32` and `bytes` (`AGGREGATE-FIELD-TYPES`). Each is a named
//! follow-on in `docs/reports/aggregates-v1/README.md` §12 — and, for the
//! owned field's own boundaries, `docs/reports/aggregate-owned-field/README.md`
//! — with its missing capability stated.

use crate::frontend::{DeclarationKind, SourceBlock, SourceProgram};
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::source_record::{FieldType, RecordField, RecordType};

/// The declaration head. The **existing** reserved word, claimed at one shape.
pub const TYPE_KEYWORD: &str = "type";

/// One record construction or field read, read out of a statement's tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordStatement {
    /// `let NAME = TYPE { FIELD: OPERAND, … };`
    ///
    /// The field **values** are carried as the half-open index range of their
    /// tokens **inside the statement's own window**, not as parsed values: each
    /// is a `source.buffer.v1` operand, read by that route's own reader, so a
    /// value form one route admits and the other refuses is not expressible.
    Construct {
        binding: String,
        type_name: String,
        fields: Vec<(String, (usize, usize))>,
    },
    /// `let NAME = NAME . FIELD ;`
    Field {
        binding: String,
        record: String,
        field: String,
    },
    /// `let NAME = NAME . FIELD [ OPERAND ] ;` — a byte read through an owned
    /// field (card `AGGREGATE-OWNED-FIELD`). The index is carried as the
    /// half-open token range of its operand inside the statement's window,
    /// read by `source.buffer.v1`'s own operand reader, exactly as a
    /// construction's field values are.
    FieldLoad {
        binding: String,
        record: String,
        field: String,
        index: (usize, usize),
    },
}

/// A field-path statement that is not a `let` (card `AGGREGATE-OWNED-FIELD`):
/// a store through an owned field, or a release of one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldPathStatement {
    /// `NAME . FIELD [ OPERAND ] = OPERAND ;`, with the index and the value as
    /// token ranges inside the window.
    Store {
        record: String,
        field: String,
        index: (usize, usize),
        value: (usize, usize),
    },
    /// `drop NAME . FIELD ;`
    Release { record: String, field: String },
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

/// The tokens of `span`, in source order.
#[must_use]
pub fn window(tokens: &[Token], span: Span) -> Vec<&Token> {
    tokens
        .iter()
        .filter(|token| span.start <= token.span.start && token.span.end <= span.end)
        .collect()
}

/// Which of this card's constructs a statement uses, if any.
///
/// The **one** place that decides whether a statement is this construct's, so
/// the lowering and any future selector cannot disagree about it. Deliberately
/// shape-based and keyword-free: both forms begin `let NAME =`, and what tells
/// them apart from every other `let` is the token at index 4 — a `{` for a
/// construction, a `.` for a field read.
#[must_use]
pub fn record_statement_shape(window: &[&Token]) -> Option<RecordShape> {
    if word(window, 0) != Some("let") || !symbol(window, 2, Symbol::Equal) {
        return None;
    }
    word(window, 3)?;
    if symbol(window, 4, Symbol::LeftBrace) {
        return Some(RecordShape::Construct);
    }
    if symbol(window, 4, Symbol::Dot) {
        return Some(RecordShape::Field);
    }
    None
}

/// Which of the two forms a statement spells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecordShape {
    Construct,
    Field,
}

/// Read one statement as a record construction or field read.
///
/// Returns `None` when the statement is not one this construct spells at all,
/// and `Some(Err(message))` when it is one and is malformed — the
/// `source_std_buffer::buffer_statement` shape, so the caller attributes the
/// message to its own diagnostic code. The code is the caller's because §L21
/// gives each semantic pass its own range and this construct allocates none;
/// the message is this construct's, because the fault is its to describe.
#[must_use]
pub fn read_record_statement(window: &[&Token]) -> Option<Result<RecordStatement, String>> {
    let shape = record_statement_shape(window)?;
    let binding = word(window, 1)?.to_owned();
    let head = word(window, 3)?.to_owned();
    match shape {
        RecordShape::Field => {
            let Some(field) = word(window, 5) else {
                return Some(Err(format!(
                    "`{head}.` is not a field read; `source.record.v1` reads `NAME.FIELD`, where \
                     the field is a declared name — a numeric component is not one, and \
                     `{head}.1.2` is a single floating-point token besides"
                )));
            };
            // Card `AGGREGATE-OWNED-FIELD`: `let X = p.f[IDX];` — a byte read
            // through an owned field. Told from the field read by the `[`
            // after the field, exactly as a byte read is told from a literal
            // bind by the `[` after the name.
            if symbol(window, 6, Symbol::LeftBracket) {
                let close = window.len().checked_sub(2)?;
                if close <= 7
                    || !symbol(window, close, Symbol::RightBracket)
                    || !symbol(window, close + 1, Symbol::Semicolon)
                {
                    return Some(Err(format!(
                        "`{head}.{field}[…]` is not closed; a byte read through a field is \
                         `let NAME = RECORD.FIELD[INDEX];` with one operand as the index"
                    )));
                }
                return Some(Ok(RecordStatement::FieldLoad {
                    binding,
                    record: head,
                    field: field.to_owned(),
                    index: (7, close),
                }));
            }
            if window.len() != 7 || !symbol(window, 6, Symbol::Semicolon) {
                return Some(Err(format!(
                    "`{head}.{field}` is not closed; `source.record.v1` reads exactly one field \
                     per statement and carries no arithmetic on the result"
                )));
            }
            Some(Ok(RecordStatement::Field {
                binding,
                record: head,
                field: field.to_owned(),
            }))
        }
        RecordShape::Construct => {
            // `let NAME = TYPE { FIELD : … , FIELD : … } ;`
            //
            // The field **values** are not read here: each is a
            // `source.buffer.v1` operand, read by that route's own reader out of
            // its own token span, so a value form one route admits and the other
            // refuses is not expressible. What this reader returns is the field
            // name and the span of the value's tokens.
            let close = window.len().checked_sub(2)?;
            if !symbol(window, close, Symbol::RightBrace)
                || !symbol(window, close + 1, Symbol::Semicolon)
            {
                return Some(Err(format!(
                    "this `{head}` construction is not closed; `source.record.v1` constructs \
                     `TYPE {{ FIELD: VALUE, … }}` and ends the statement with `;`"
                )));
            }
            let mut fields = Vec::new();
            let mut cursor = 5;
            if cursor == close {
                return Some(Err(format!(
                    "this `{head}` construction supplies no field; `source.record.v1` constructs a \
                     record only from a complete field list"
                )));
            }
            while cursor < close {
                let Some(field) = word(window, cursor) else {
                    return Some(Err(format!(
                        "this `{head}` construction expects a field name; `source.record.v1` \
                         supplies every field by name, never by position"
                    )));
                };
                if !symbol(window, cursor + 1, Symbol::Colon) {
                    return Some(Err(format!(
                        "`{field}` is written without its `:`; `source.record.v1` supplies a field \
                         as `FIELD: VALUE`"
                    )));
                }
                let value_start = cursor + 2;
                let mut value_end = value_start;
                while value_end < close && !symbol(window, value_end, Symbol::Comma) {
                    value_end += 1;
                }
                if value_end == value_start {
                    return Some(Err(format!(
                        "`{field}` is supplied no value; `source.record.v1` supplies a field as \
                         `FIELD: VALUE`"
                    )));
                }
                fields.push((field.to_owned(), (value_start, value_end)));
                cursor = if value_end < close {
                    value_end + 1
                } else {
                    value_end
                };
                // A trailing comma leaves the cursor at the closing brace, which
                // ends the walk: `Point { x: 1, }` is admitted exactly as
                // `Point { x: 1 }` is, because the two differ in no token that
                // means anything.
            }
            Some(Ok(RecordStatement::Construct {
                binding,
                type_name: head,
                fields,
            }))
        }
    }
}

/// Whether a statement is one of the two field-path forms that are not a
/// `let` (card `AGGREGATE-OWNED-FIELD`): `NAME . FIELD [` at its head, or
/// `drop NAME . FIELD`. The one place that decides it, so the lowering and
/// [`block_record_span`] cannot disagree.
#[must_use]
pub fn field_path_statement_shape(window: &[&Token]) -> bool {
    if word(window, 0) == Some("drop") {
        return word(window, 1).is_some() && symbol(window, 2, Symbol::Dot);
    }
    word(window, 0).is_some()
        && word(window, 0) != Some("let")
        && symbol(window, 1, Symbol::Dot)
        && word(window, 2).is_some()
        && symbol(window, 3, Symbol::LeftBracket)
}

/// Read one store through a field or one field release, on the
/// [`read_record_statement`] contract: `None` when the statement is not one
/// of the two shapes, `Some(Err(message))` when it is one and is malformed.
#[must_use]
pub fn read_field_path_statement(window: &[&Token]) -> Option<Result<FieldPathStatement, String>> {
    if !field_path_statement_shape(window) {
        return None;
    }
    if word(window, 0) == Some("drop") {
        let record = word(window, 1)?.to_owned();
        let Some(field) = word(window, 3) else {
            return Some(Err(format!(
                "`drop {record}.` names no field; a field release is `drop RECORD.FIELD;`"
            )));
        };
        if window.len() != 5 || !symbol(window, 4, Symbol::Semicolon) {
            return Some(Err(format!(
                "`drop {record}.{field}` is not closed; a field release is `drop RECORD.FIELD;`"
            )));
        }
        return Some(Ok(FieldPathStatement::Release {
            record,
            field: field.to_owned(),
        }));
    }
    let record = word(window, 0)?.to_owned();
    let field = word(window, 2)?.to_owned();
    // `RECORD . FIELD [ INDEX ] = VALUE ;`
    let Some(close) = (4..window.len()).find(|at| symbol(window, *at, Symbol::RightBracket)) else {
        return Some(Err(format!(
            "`{record}.{field}[` is not closed; a store through a field is \
             `RECORD.FIELD[INDEX] = VALUE;`"
        )));
    };
    if close == 4 {
        return Some(Err(format!(
            "`{record}.{field}[]` indexes with nothing; a store through a field is \
             `RECORD.FIELD[INDEX] = VALUE;` with one operand as the index"
        )));
    }
    if !symbol(window, close + 1, Symbol::Equal) {
        return Some(Err(format!(
            "`{record}.{field}[…]` is followed by something other than `=`; a store through a \
             field is `RECORD.FIELD[INDEX] = VALUE;`"
        )));
    }
    let end = window.len().checked_sub(1)?;
    if !symbol(window, end, Symbol::Semicolon) || end <= close + 2 {
        return Some(Err(format!(
            "`{record}.{field}[…] =` stores nothing, or is not closed; a store through a field \
             is `RECORD.FIELD[INDEX] = VALUE;`"
        )));
    }
    Some(Ok(FieldPathStatement::Store {
        record,
        field,
        index: (4, close),
        value: (close + 2, end),
    }))
}

/// Read every record **type declaration** of this module.
///
/// Returns the declared types in declaration order, or the first malformed
/// declaration's name and message. The field list's shape is decided here and
/// nowhere else; whether the *type* is one the model admits — nonempty, no
/// repeated field, inside the field bound — is
/// [`qcore_core::source_record::admit_type`]'s, so the two cannot disagree.
pub fn record_declarations(program: &SourceProgram) -> Result<Vec<RecordType>, (Span, String)> {
    let tokens = program.lexical_tokens();
    let mut declared = Vec::new();
    for declaration in &program.declarations {
        if declaration.value.kind != DeclarationKind::Record {
            continue;
        }
        let span = declaration.span;
        let window = window(tokens, span);
        // `type NAME { … }` — the head was validated by the parser branch that
        // selected this kind, so the reader starts inside the brace.
        let Some(close) = window
            .iter()
            .rposition(|token| token.kind == TokenKind::Symbol(Symbol::RightBrace))
        else {
            return Err((span, "this record type is not closed".to_owned()));
        };
        let name = declaration.value.name.clone();
        let mut fields = Vec::new();
        let mut cursor = 3;
        while cursor < close {
            let Some(field) = word(&window, cursor) else {
                return Err((
                    span,
                    format!(
                        "`{name}` expects a field name; `source.record.v1` declares a record as \
                         `type {name} {{ FIELD: TYPE, … }}`"
                    ),
                ));
            };
            if !symbol(&window, cursor + 1, Symbol::Colon) {
                return Err((
                    span,
                    format!(
                        "`{name}.{field}` is declared without its `:`; a field is declared as \
                         `FIELD: TYPE`"
                    ),
                ));
            }
            let Some(spelling) = word(&window, cursor + 2) else {
                return Err((span, format!("`{name}.{field}` is declared with no type")));
            };
            let Some(field_type) = FieldType::from_spelling(spelling) else {
                // The refusal itself is the model's, and it is reported with
                // the model's own message so there is one description of the
                // rule rather than two.
                return Err((
                    span,
                    qcore_core::source_record::RecordRefusal::UnknownFieldType {
                        type_name: name.clone(),
                        field: field.to_owned(),
                        spelling: spelling.to_owned(),
                    }
                    .to_string(),
                ));
            };
            fields.push(RecordField {
                name: field.to_owned(),
                field_type,
            });
            cursor += 3;
            if symbol(&window, cursor, Symbol::Comma) {
                cursor += 1;
            } else if cursor < close {
                return Err((
                    span,
                    format!(
                        "`{name}` separates its fields with `,`; this one is followed by something \
                         else"
                    ),
                ));
            }
        }
        declared.push(RecordType { name, fields });
    }
    Ok(declared)
}

/// The span of this module's first record type declaration, if it has one.
#[must_use]
pub fn record_declaration_span(program: &SourceProgram) -> Option<Span> {
    program
        .declarations
        .iter()
        .find(|declaration| declaration.value.kind == DeclarationKind::Record)
        .map(|declaration| declaration.span)
}

/// Whether this module is a `source.record.v1` module.
///
/// **The condition is a record type declaration and nothing else.** A module
/// that constructs a value of a type it does not declare fails this guard and
/// keeps the route it had — which is what makes this card's route-selection
/// change one new door opened only by one new spelling, on the `STRING-CONCAT`
/// precedent. The construction then refuses at `Q013`, because the type name is
/// a reference that resolves to nothing.
#[must_use]
pub fn has_source_record(program: &SourceProgram) -> bool {
    record_declaration_span(program).is_some()
}

/// The span of the first record statement inside `block`, at any depth.
///
/// Used to refuse a record construct in a function that is not the entry, the
/// way `source_loop::block_loop_span` refuses a loop there.
#[must_use]
pub fn block_record_span(block: &SourceBlock, tokens: &[Token]) -> Option<Span> {
    for statement in &block.statements {
        let window = window(tokens, statement.span);
        if record_statement_shape(&window).is_some() || field_path_statement_shape(&window) {
            return Some(statement.span);
        }
        for nested in statement.value.nested_blocks() {
            if let Some(span) = block_record_span(nested, tokens) {
                return Some(span);
            }
        }
    }
    None
}
