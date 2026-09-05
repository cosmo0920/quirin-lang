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
