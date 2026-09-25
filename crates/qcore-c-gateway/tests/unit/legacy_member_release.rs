//! Card W45-CALLEE-SUMMARY-CASCADE: the member-release rules, end to end
//! through the unit analysis.

use super::super::{
    AnalysisStatus, LegacyProgramSummary, analyze_legacy_c_translation_unit, build_function_cfg,
    tokenize_with_conditionals,
};
use super::guarded_callees;
use qcore_schemas::contract::ResourceContracts;

fn contracts() -> ResourceContracts {
    let sources = [
        (
            "q_base.qcore".to_owned(),
            include_str!("../fixtures/contracts/q_base.qcore").to_owned(),
        ),
        (
            "q_nullable.qcore".to_owned(),
            include_str!("../fixtures/contracts/q_nullable.qcore").to_owned(),
        ),
    ]
    .into_iter()
    .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

const RECORD: &str = "struct inner { int n; };\n\
     struct node { void *buf; struct node *next; struct inner in; int n; };\n";

fn analyze(body: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{RECORD}{body}"), "subject", &contracts()).unwrap()
}

fn has(summary: &LegacyProgramSummary, function: &str, needle: &str) -> bool {
    summary.functions[function]
        .diagnostics
        .iter()
        .any(|diagnostic| format!("{:?}", diagnostic.kind).contains(needle))
}

fn status(summary: &LegacyProgramSummary, function: &str) -> AnalysisStatus {
    summary.functions[function].status
}

const DESTROY: &str = "static void node_destroy(struct node *n) {\n\
     if (n->buf) q_nullable_free(n->buf);\n\
     q_nullable_free(n);\n}\n";

#[test]
fn a_destructor_body_validates_and_its_caller_certifies() {
    let summary = analyze(&format!(
        "{DESTROY}void subject(void) {{ struct node *n = q_alloc(); node_destroy(n); }}\n"
    ));
    assert_eq!(
        status(&summary, "node_destroy"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["node_destroy"].diagnostics
    );
    assert_eq!(
        status(&summary, "subject"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn without_the_release_the_caller_leaks() {
    // Not weaker than the body: a destructor that forgets its formal gives the
    // caller's acquisition nowhere to go.
    let summary = analyze(
        "static void node_clear(struct node *n) { q_nullable_free(n->buf); }\n\
         void subject(void) { struct node *n = q_alloc(); node_clear(n); }\n",
    );
    assert_eq!(status(&summary, "node_clear"), AnalysisStatus::Safe);
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Unsafe);
    assert!(has(&summary, "subject", "Leak"));
}

#[test]
fn the_released_formal_is_still_released_exactly_once() {
    // The member release touches no tracked identity: releasing the formal
    // twice is still a violation.
    let summary = analyze(
        "void subject(void) { struct node *n = q_alloc(); q_nullable_free(n->buf); \
         q_nullable_free(n); q_nullable_free(n); }\n",
    );
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Unsafe);
}

#[test]
fn a_member_read_through_a_released_base_is_refused() {
    let summary = analyze(
        "void subject(void) { struct node *n = q_alloc(); q_nullable_free(n); \
         q_nullable_free(n->buf); }\n",
    );
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn a_non_null_release_keeps_the_refusal() {
    let summary = analyze("void subject(struct node *n) { q_free(n->buf); q_nullable_free(n); }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(has(&summary, "subject", "indirect memory access"));
}

#[test]
fn a_member_address_is_never_released() {
    let summary =
        analyze("void subject(struct node *n) { q_nullable_free(&n->in); q_nullable_free(n); }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn a_consuming_position_keeps_the_refusal() {
    let summary = analyze("void subject(struct node *n) { q_transfer(n->buf); }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(has(&summary, "subject", "indirect memory access"));
}

#[test]
fn a_released_member_of_a_linked_owner_is_refused() {
    // The body links a tracked child at `n->buf`, so the read may name it.
    let summary = analyze(
        "void subject(void) { struct node *n = q_alloc(); n->buf = q_alloc(); \
         q_nullable_free(n->buf); q_nullable_free(n); }\n",
    );
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn the_guard_refuses_a_caller_whose_actual_owns_sub_custody() {
    // The destructor releases whatever `n->buf` holds; a caller that linked a
    // tracked child there may not read the call as releasing `n` alone.
    let summary = analyze(&format!(
        "{DESTROY}void subject(void) {{ struct node *n = q_alloc(); n->buf = q_alloc(); \
         node_destroy(n); }}\n"
    ));
    assert_eq!(status(&summary, "node_destroy"), AnalysisStatus::Safe);
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(
        has(&summary, "subject", "UnknownAlias"),
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn the_guard_precedes_only_calls_to_functions_this_unit_defines() {
    let source = format!(
        "{RECORD}{DESTROY}void q_free(void *p) {{ }}\n\
         void subject(void) {{ struct node *n = q_alloc(); \
         node_destroy(n); void *p = q_alloc(); q_free(p); }}\n"
    );
    // `q_free` is defined here too, but declared: its declaration governs.
    let (tokens, _) = tokenize_with_conditionals(&source);
    // The environment the caller is analysed in once the destructor's summary
    // is installed.
    let mut environment = contracts();
    let released = environment.contract("q_nullable_free").unwrap().clone();
    environment.insert_contract("node_destroy", released);
    let cfg = build_function_cfg(&tokens, "subject", &environment).unwrap();
    assert_eq!(
        guarded_callees(&cfg).into_iter().collect::<Vec<_>>(),
        vec!["node_destroy".to_owned()]
    );
}

#[test]
fn a_member_null_test_is_a_typed_read_of_its_base() {
    let summary = analyze(
        "void subject(struct node *n) { if (n->buf == NULL) return; \
         if (!(n->next)) return; while (n->buf != ((void *)0)) { return; } }\n",
    );
    assert_eq!(
        status(&summary, "subject"),
        AnalysisStatus::Safe,
        "{:?}",
        summary.functions["subject"].diagnostics
    );
}

#[test]
fn a_member_null_test_reads_only_single_pointer_members() {
    for condition in ["n->next->buf", "n->buf && n->n", "n->buf == n->next"] {
        let summary = analyze(&format!(
            "void subject(struct node *n) {{ if ({condition}) return; }}\n"
        ));
        assert!(
            has(&summary, "subject", "branch condition"),
            "{condition}: {:?}",
            summary.functions["subject"].diagnostics
        );
    }
}

#[test]
fn a_member_null_test_over_a_released_base_is_refused() {
    // Over a formal: a body-acquired base's member test is the pre-existing
    // aggregate null condition (card O04-B08), not this rule.
    let summary =
        analyze("void subject(struct node *n) { q_nullable_free(n); if (n->buf) return; }\n");
    assert_ne!(status(&summary, "subject"), AnalysisStatus::Safe);
    assert!(has(&summary, "subject", "UnknownAlias"));
}
