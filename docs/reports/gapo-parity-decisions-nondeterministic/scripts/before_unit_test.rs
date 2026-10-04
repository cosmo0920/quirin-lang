// GAPO-PARITY-DECISIONS-NONDETERMINISTIC: the minimised BEFORE test, appended
// to crates/qcore-c-gateway/tests/unit/typed_place.rs at the base (0ebe77f86)
// by scripts/before-unit.sh and run with QCORE_TYPED_PATH_PARITY set (the
// BEFORE code keeps decisions only on parity runs). Two consultations on one
// thread: a position reader whose consumer records nothing, then a statement
// consultation whose node does not match. The second consultation's record
// (what `record_decisions` would write as its `typed-place` row) must hold
// none of the first's decisions; at the base it holds one, deterministically.

#[test]
fn gapo_nd_before_a_declined_consultation_holds_no_leftover() {
    assert!(
        crate::typed_access_path::parity_enabled(),
        "run with QCORE_TYPED_PATH_PARITY set"
    );
    if crate::clang_command().is_err() {
        return;
    }
    let source = "struct I { int x; }; struct S { struct I in; struct S *next; int v; };\n\
int f(struct S *s) { struct S o; int y = s->next->in.x;\n\
o.in.x = 3;\n\
s->next = s;\n\
return y + o.in.x; }";
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    // The first consultation: rule 10k's condition reader, unrecorded (as
    // the parity-only `record_typed_position_legacy` asks it).
    let spelled = |text: &str| {
        tokenize(text)
            .into_iter()
            .map(|token| token.text)
            .collect::<Vec<_>>()
    };
    let operand = spelled("s->next->in.x");
    let operand = operand.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(index.condition(&operand, 2, |_| false).is_some());
    // The second: a statement consultation whose node does not match.
    let declined = spelled("s->v = 9");
    let declined = declined.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(
        index
            .statement_with(&declined, 4, &|| None, |_| false)
            .is_none()
    );
    let held = DECISIONS.with(|decisions| decisions.borrow().clone());
    assert!(
        held.is_empty(),
        "the declined consultation's typed-place row would carry {} leftover decision(s): {held:?}",
        held.len()
    );
}
