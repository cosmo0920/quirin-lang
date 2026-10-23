//! Card W48-BU-SINGLE-LEAF-BODIES (rule 35, arm 1): the decision table, its
//! Lean binding and the token reading.

use super::*;

fn follow_of(store: bool) -> Follow {
    if store { Follow::Store } else { Follow::Read }
}

#[test]
fn the_decision_vectors_hold() {
    for &(literal, store, expected) in DECISION_VECTORS {
        assert_eq!(
            decide(literal, follow_of(store)).code(),
            expected,
            "{literal} {store}"
        );
    }
}

#[test]
fn the_lean_decision_vectors_are_the_rust_table() {
    let lean =
        include_str!("../../../../proof/QCore/LiteralDecayReadings.lean").replace("\r\n", "\n");
    let block = lean
        .split("def literalVectors ")
        .nth(1)
        .expect("literalVectors in the Lean model");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    let rows = block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let fields = row
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 3, "{row}");
            (
                fields[0] == "true",
                fields[1] == "true",
                match fields[2] {
                    ".literal" => "literal",
                    ".store" => "store",
                    ".notLiteral" => "not-literal",
                    other => panic!("unknown decision {other}"),
                },
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, DECISION_VECTORS.to_vec());
}

fn texts(line: &str) -> Vec<String> {
    crate::legacy_c::tokenize(line)
        .into_iter()
        .map(|token| token.text)
        .collect()
}

fn star_exempt(line: &str) -> Vec<bool> {
    let owned = texts(line);
    let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
    texts
        .iter()
        .enumerate()
        .filter(|(_, text)| **text == "*")
        .map(|(index, _)| exempt(&texts, index))
        .collect()
}

#[test]
fn a_read_of_a_literal_is_exempt_and_every_other_star_is_not() {
    assert_eq!(star_exempt("!*\"fd not found\""), [true]);
    assert_eq!(star_exempt("c = *\"a\" \"b\" + 1"), [true]);
    assert_eq!(star_exempt("*\"a\" == 'a'"), [true]);
    assert_eq!(star_exempt("!*p"), [false]);
    assert_eq!(star_exempt("*(char *)\"a\""), [false, false]);
    assert_eq!(star_exempt("**\"a\""), [false, true]);
}

#[test]
fn a_store_or_increment_of_a_literal_is_not_exempt() {
    assert_eq!(star_exempt("*\"a\" = 1"), [false]);
    assert_eq!(star_exempt("*\"a\" += 1"), [false]);
    assert_eq!(star_exempt("*\"a\" <<= 1"), [false]);
    assert_eq!(star_exempt("*\"a\"++"), [false]);
    assert_eq!(star_exempt("*\"a\" == 1"), [true]);
}
