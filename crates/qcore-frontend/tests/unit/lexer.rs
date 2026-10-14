use super::*;

#[test]
fn tokenizes_literals_comments_and_compound_symbols() {
    let tokens = lex("foo 12 3.5 \"x\\\"y\" 'z' // c\n := :: -> => == != <= >=").unwrap();
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Word("foo".into()),
            TokenKind::Integer("12".into()),
            TokenKind::Float("3.5".into()),
            TokenKind::StringLiteral("x\\\"y".into()),
            TokenKind::CharLiteral("z".into()),
            TokenKind::Symbol(Symbol::Define),
            TokenKind::Symbol(Symbol::ColonColon),
            TokenKind::Symbol(Symbol::Arrow),
            TokenKind::Symbol(Symbol::FatArrow),
            TokenKind::Symbol(Symbol::EqualEqual),
            TokenKind::Symbol(Symbol::NotEqual),
            TokenKind::Symbol(Symbol::LessEqual),
            TokenKind::Symbol(Symbol::GreaterEqual),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn tokenizes_quant_delimiters_as_single_symbols_beside_plain_brackets() {
    let tokens = lex("quant[|n|] [x]").unwrap();
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Word("quant".into()),
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Word("n".into()),
            TokenKind::Symbol(Symbol::QuantClose),
            TokenKind::Symbol(Symbol::LeftBracket),
            TokenKind::Word("x".into()),
            TokenKind::Symbol(Symbol::RightBracket),
            TokenKind::Eof,
        ]
    );
}

/// Card `PIPE-OPERATOR` claims `|` and `&&` as tokens and adds `|>`. A bare
/// `|` was a lex error before that card; the *refusal* is preserved and moves
/// one layer later, where it can carry an actionable message
/// (`crates/qcore-frontend/src/pipe.rs`, `Q038`). This is a deliberate pin
/// update: the load-bearing half — that a bare `|` is not admitted as a quant
/// delimiter and no program containing one checks — is re-pinned at its new
/// layer in `compiler/tests/quant_index_contracts.rs` and
/// `compiler/tests/pipe_operator.rs`.
///
/// **Card `OR-AND-LOWERING` retables the family** by user direction: `||` is
/// now one `PipePipe` token (logical or) rather than two `Pipe` tokens, and
/// the single `|` is reserved for bitwise or rather than claimed as logical
/// or. Only the `a || b` row of this test moves; every other row — including
/// both quant-delimiter rows and the `|>` longest-match row — is asserted
/// byte-unchanged, which is what makes the retable safe for the shipped
/// `quant[|…|]` alias.
#[test]
fn lexes_the_vertical_bar_family_by_longest_match() {
    let kinds = |source: &str| {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    // The two-byte spellings win over their one-byte prefixes, and `[|` is
    // matched at `[`, so `[|>` can never become `[` `|>`.
    assert_eq!(
        kinds("[|>"),
        vec![
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Symbol(Symbol::Greater),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("[|>|]"),
        vec![
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Symbol(Symbol::Greater),
            TokenKind::Symbol(Symbol::QuantClose),
            TokenKind::Eof,
        ]
    );
    // `|>` wins over `|` then `>`. This is the resolution a future or-operator
    // wave must live with: `a | > b` is the spaced spelling.
    assert_eq!(
        kinds("a |> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipeInto),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a | > b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Symbol(Symbol::Greater),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // `||` is **one** `PipePipe` token: the logical-or operator, atomic by the
    // same width-two rule that decides `|>`. Card `PIPE-OPERATOR` lexed it as
    // two `Pipe` tokens because the single bar was then the or-spelling; card
    // `OR-AND-LOWERING` inverts that, and this row is the deliberate move.
    assert_eq!(
        kinds("a || b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipePipe),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // Three bars are decided by width and not by a longest-possible-run rule:
    // `||` then the reserved `|`, which refuses.
    assert_eq!(
        kinds("a ||| b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipePipe),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // `||>` is the or-operator then a comparison, never `|` then `|>`.
    assert_eq!(
        kinds("a ||> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipePipe),
            TokenKind::Symbol(Symbol::Greater),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("|>|"),
        vec![
            TokenKind::Symbol(Symbol::PipeInto),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Eof,
        ]
    );
    // `[||]` is matched at `[`, so it is the two quant delimiters and never an
    // or-operator: the retable cannot reach inside a quant-index type.
    assert_eq!(
        kinds("[||]"),
        vec![
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Symbol(Symbol::QuantClose),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a [ | ] b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::LeftBracket),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Symbol(Symbol::RightBracket),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
}

/// The three single-byte reservations of card `OR-AND-LOWERING`. `^` was an
/// `unexpected character` lex error before this card and is now a token; the
/// set of accepted programs is unchanged, because the operator gate refuses
/// every occurrence (`compiler/tests/logical_connectives.rs`).
#[test]
fn lexes_the_three_reserved_bitwise_characters() {
    let kinds = |source: &str| {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds("a | b & c ^ d"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Word("b".into()),
            TokenKind::Symbol(Symbol::Ampersand),
            TokenKind::Word("c".into()),
            TokenKind::Symbol(Symbol::Caret),
            TokenKind::Word("d".into()),
            TokenKind::Eof,
        ]
    );
    // `^` has no two-byte spelling in the table, so it never absorbs a
    // neighbour the way `|` and `&` do.
    assert_eq!(
        kinds("^^"),
        vec![
            TokenKind::Symbol(Symbol::Caret),
            TokenKind::Symbol(Symbol::Caret),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_the_ampersand_family_with_the_single_byte_left_reserved() {
    let kinds = |source: &str| {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    // `&&` is atomic by longest match and is **the** logical-and operator
    // after card `OR-AND-LOWERING`; a single `&` keeps exactly the meaning it
    // had before card `PIPE-OPERATOR` — none — and is reserved for bitwise
    // and. This test is byte-unchanged by the retable: `&&` was already the
    // and-spelling, so only its status moved, from claimed-and-refused to
    // admitted-in-a-condition.
    assert_eq!(
        kinds("a && b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::AndAnd),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a & & b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Ampersand),
            TokenKind::Symbol(Symbol::Ampersand),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("&&&"),
        vec![
            TokenKind::Symbol(Symbol::AndAnd),
            TokenKind::Symbol(Symbol::Ampersand),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("&Buffer"),
        vec![
            TokenKind::Symbol(Symbol::Ampersand),
            TokenKind::Word("Buffer".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_unterminated_comment() {
    let error = lex("/* no end").unwrap_err();
    assert_eq!(error.message, "unterminated block comment");
}

#[test]
fn distinguishes_lifetimes_from_character_literals() {
    let tokens = lex("'scope 'x' 'static").unwrap();
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Symbol(Symbol::Apostrophe),
            TokenKind::Word("scope".into()),
            TokenKind::CharLiteral("x".into()),
            TokenKind::Symbol(Symbol::Apostrophe),
            TokenKind::Word("static".into()),
            TokenKind::Eof,
        ]
    );
}

/// Card OP-CONSTRUCTOR-DECLARATIONS (USER DECISION 2026-10-14): the maps-to
/// token [`MAPS_TO`] is one three-byte token, told apart from the pipe `|>`,
/// the logical or `||` and the arrow `->`.
#[test]
fn lexes_maps_to_apart_from_the_pipe_the_or_and_the_arrow() {
    let kinds = |source: &str| {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    let maps = format!("rd_kafka_op_s.rko_op_cb {MAPS_TO} absent");
    assert_eq!(
        kinds(&maps),
        vec![
            TokenKind::Word("rd_kafka_op_s".into()),
            TokenKind::Symbol(Symbol::Dot),
            TokenKind::Word("rko_op_cb".into()),
            TokenKind::Symbol(Symbol::MapsTo),
            TokenKind::Word("absent".into()),
            TokenKind::Eof,
        ]
    );
    // The pipe keeps its two bytes.
    assert_eq!(
        kinds("a |> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipeInto),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // `||` keeps its two bytes, and `||->` is the or then an arrow, never a
    // bar then the maps-to.
    assert_eq!(
        kinds("a || b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipePipe),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a ||-> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::PipePipe),
            TokenKind::Symbol(Symbol::Arrow),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // A bare arrow is the arrow, and a spaced bar is a bar then an arrow.
    assert_eq!(
        kinds("a -> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Arrow),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("a | -> b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Symbol(Symbol::Arrow),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
    // `|-` short of the third byte is a bar then a minus.
    assert_eq!(
        kinds("a |- b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Symbol(Symbol::Minus),
            TokenKind::Word("b".into()),
            TokenKind::Eof,
        ]
    );
}
