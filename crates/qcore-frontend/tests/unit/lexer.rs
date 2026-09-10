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
    // `||` is two `Pipe` tokens the grammar refuses with an actionable
    // message; it is deliberately never a lexer-level refusal, so the
    // diagnostic can name the single-`|` spelling.
    assert_eq!(
        kinds("a || b"),
        vec![
            TokenKind::Word("a".into()),
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Symbol(Symbol::Pipe),
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

#[test]
fn lexes_the_ampersand_family_with_the_single_byte_left_reserved() {
    let kinds = |source: &str| {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>()
    };
    // `&&` is atomic by longest match; a single `&` keeps exactly the meaning
    // it had before card `PIPE-OPERATOR` — none — and is held for a future
    // binary/bitwise operation.
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
