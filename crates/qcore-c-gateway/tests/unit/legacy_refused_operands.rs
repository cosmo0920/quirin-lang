//! Card W44-LEAK-PIN-AUDIT: the hand-on reader of the refused-escape rule.

use super::*;

fn places(statement: &str) -> Vec<(String, HandOn)> {
    let tokens = statement.split_whitespace().collect::<Vec<_>>();
    handed_on(&tokens).into_iter().collect()
}

fn escapes(statement: &str) -> Vec<String> {
    places(statement)
        .into_iter()
        .filter(|(_, how)| *how == HandOn::Escape)
        .map(|(place, _)| place)
        .collect()
}

fn calls(statement: &str) -> Vec<String> {
    places(statement)
        .into_iter()
        .filter(|(_, how)| *how == HandOn::Call)
        .map(|(place, _)| place)
        .collect()
}

#[test]
fn a_call_argument_is_the_callees_business() {
    assert_eq!(calls("destroy_metric ( metric )"), ["metric"]);
    assert!(escapes("destroy_metric ( metric )").is_empty());
}

#[test]
fn a_cast_value_stored_through_a_member_escapes() {
    // `map->parent = (void *) counter;`: the destination chain is written, the
    // cast operand is the stored value.
    assert_eq!(
        escapes("map - > parent = ( void * ) counter"),
        ["counter", "void"]
    );
}

#[test]
fn a_returned_value_escapes() {
    assert_eq!(
        escapes("return ( const char * * ) arr"),
        ["arr", "char", "const"]
    );
    assert_eq!(calls("return f ( p )"), ["p"]);
    assert!(escapes("return f ( p )").is_empty());
}

#[test]
fn an_address_inside_the_object_hands_on_every_prefix() {
    let passed = calls("cfl_list_add ( & counter - > _head , & cmt - > counters )");
    for place in ["counter", "counter->_head", "cmt", "cmt->counters"] {
        assert!(passed.contains(&place.to_owned()), "{place} in {passed:?}");
    }
    assert_eq!(escapes("x - > h = & p - > head"), ["p", "p->head"]);
}

#[test]
fn a_member_value_read_does_not_hand_on_the_base() {
    // `ret = mk_net_fd_connect(conn->fd, addr, port);`: an integer member is
    // read through `conn`; `conn` itself is not handed on.
    let passed = calls("ret = mk_net_fd_connect ( conn - > fd , addr , port )");
    assert!(passed.contains(&"conn->fd".to_owned()));
    assert!(!passed.contains(&"conn".to_owned()));
    assert!(escapes("x = p - > buf").contains(&"p->buf".to_owned()));
    assert!(!escapes("x = p - > buf").contains(&"p".to_owned()));
}

#[test]
fn a_subscript_or_call_base_is_not_handed_on() {
    let read = escapes("x = p - > items [ i ] + q - > fn ( y )");
    assert!(!read.contains(&"p->items".to_owned()));
    assert!(!read.contains(&"q->fn".to_owned()));
    assert!(read.contains(&"i".to_owned()));
    assert_eq!(calls("x = p - > items [ i ] + q - > fn ( y )"), ["y"]);
}

#[test]
fn a_dereferenced_name_hands_on_its_slot_place() {
    assert_eq!(escapes("x = * out"), ["*out"]);
    // A store through the slot writes it and stores the value.
    assert_eq!(escapes("* out = buf"), ["buf"]);
}

#[test]
fn a_statement_with_no_value_side_hands_on_nothing_but_call_arguments() {
    assert!(escapes("if ( ! p ) x ( )").is_empty());
    assert!(places("p - > len ++").is_empty());
}

#[test]
fn an_unevaluated_operand_hands_on_nothing() {
    assert!(places("n = sizeof ( * p )").is_empty());
}

#[test]
fn binary_operators_are_not_unary() {
    assert_eq!(escapes("n = a * b & c"), ["a", "b", "c"]);
}

#[test]
fn line_reads_key_every_line_a_statement_spans() {
    // `x = f(a,` on line 10, `b) + c;` on line 11, `y = d;` on line 12.
    let texts = [
        "x", "=", "f", "(", "a", ",", "b", ")", "+", "c", ";", "y", "=", "d", ";",
    ];
    let lines = [10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 12, 12, 12, 12];
    let reads = line_reads(&texts, &lines);
    assert_eq!(reads[&10].get("c"), Some(&HandOn::Escape));
    assert_eq!(reads[&11].get("a"), Some(&HandOn::Call));
    assert!(!reads[&12].contains_key("a"));
    assert_eq!(reads[&12].get("d"), Some(&HandOn::Escape));
}
