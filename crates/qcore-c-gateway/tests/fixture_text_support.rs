#[path = "support/text.rs"]
mod text;

use text::normalize_newlines;

#[test]
fn newline_normalization_preserves_lf_syntax() {
    let source = "module contract;\ncontract {\n};\n";
    assert_eq!(normalize_newlines(source), source);
}

#[test]
fn newline_normalization_maps_crlf_and_mixed_syntax_to_lf() {
    assert_eq!(
        normalize_newlines("module contract;\r\ncontract {\n};\r\n"),
        "module contract;\ncontract {\n};\n"
    );
}

#[test]
fn newline_normalization_is_idempotent() {
    let once = normalize_newlines("a\r\nb\nc\r\n");
    assert_eq!(normalize_newlines(&once), once);
}
